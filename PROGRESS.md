# Salvo Compiler — Progress & Plan

Status snapshot as of 2026-08-31. This document is the handoff point for
continuing development: it records what is built, the key design decisions,
known limitations, and a detailed plan for the remaining milestones.

## How to build and test

```bash
cargo build                 # workspace build, no warnings
cargo test                  # 25 tests; includes a kotlinc compile+run test
                            # (skipped gracefully if kotlinc is not on PATH)
INSTA_UPDATE=always cargo test   # accept/update insta snapshots after intended changes

# End-to-end:
cargo run -- compile --backend kotlin --src ./some_dir --target ./out
cargo run -- compile --backend kotlin --src ./some_dir --target ./out --emit-ast  # debug AST dump

# Verify generated Kotlin manually:
kotlinc out/main.kt out/core/console.kt -d classes && kotlin -cp classes salvo.MainKt
```

## Workspace layout

```
crates/
├── salvo-cli/            # binary "salvo": clap CLI, backend registry, embeds std/ via include_dir
├── salvo-syntax/         # lexer, parser, AST, spans, diagnostics (no deps)
│   └── tests/corpus/     # README-example .sv files + insta snapshots
├── salvo-core/           # SourceSet (file discovery/classification), Program, Symbols
├── salvo-backend/        # Backend trait, BackendRegistry, BackendError
└── salvo-backend-kotlin/ # Kotlin emitter (emit.rs) + golden/kotlinc tests
std/core/                 # stdlib: basic.sv, string.sv, list.sv, console.sv (+ .kotlin.sv defines)
```

Adding a Rust backend later = new crate implementing `salvo_backend::Backend`
(name `"rust"`), register it in `salvo-cli/src/main.rs`, and write
`*.rust.sv` define files next to the std modules. Std embedding already
filters define files per backend at load time (`SourceSet::classify`).

## Completed milestones

### M0+M1 — CLI skeleton + full parser

- Hand-written lexer + recursive-descent parser covering the entire README
  grammar (deliberately hand-written: newline-terminated statements,
  template/interpolation lexer modes, struct-literal-vs-block ambiguity, and
  speculative parses for generic calls / paren lambdas make grammar
  generators a poor fit).
- Key parser mechanics:
  - Tokens carry `newline_before`; infix/postfix continuation across a
    newline only inside groups (`group_depth`). Blocks reset the depth.
  - `no_struct` flag disables struct-literal speculation in condition
    position (`if x is Person { ... }`).
  - `is` checks use a case heuristic: uppercase idents are type refs, a
    trailing lowercase ident is the binding (`is Str s`).
  - Snapshot/rollback backtracking for `f<T>(...)` vs comparison, `(a,b) ->`
    lambdas, brace lambdas `{ i: Int -> 0 }`, and `Int[5] { ... }` ArrayInit.
  - String interpolation: lexer captures `${...}` raw source + offset; parser
    re-lexes/parses fragments with spans shifted back into the file.
  - `` `` templates `` in define blocks lex as raw `Template` tokens, dedented.
- Diagnostics render with file:line:col and a caret underline; parser
  recovers at item/statement level, so all errors in a file are reported.
- Fixed inconsistencies in README + std (typos, `Iterator<T>`→`Iter<T>`,
  `String`→`Str`, Kotlin `.size`→`.length` for strings, `getOrNull`, added
  `Byte`/`Any`/`Nothing`/`Iter<T>` internal types to `std/core/basic.sv`,
  `kotlin.io.print` qualification in the StdOutConsole define to avoid
  self-recursion).

### M2 — Kotlin codegen, end-to-end verified

`salvo compile --backend kotlin` emits working Kotlin (verified: kotlinc
compiles it and a test asserts the exact runtime stdout —
`kotlinc_compiles_and_runs_demo` in `salvo-backend-kotlin/tests/codegen_tests.rs`).

Supported and emitted:

| Salvo | Kotlin |
|---|---|
| `struct` (+defaults, `with Mut`) | `data class` (`val`/`var` fields, `= null` defaults) |
| struct spread `P {...p, f: v}` | `p.copy(f = v)` |
| tuple/struct destructuring `let` | Kotlin destructuring / `__destructured` temps |
| `T?`, `is None`, `x!` | `T?`, `== null`, `!!` |
| `is Str s` binding | `val s = subj as String` at branch top (relies on subject purity) |
| `effect` | `interface` |
| `handler` (with state/ctor params) | `class H(private val ...) : Effect { private var state ... override fun }` |
| `external handler` + `define handler` | `object H : Effect` with template-inlined bodies |
| fn effect deps `[Console, Random<Int>]` | leading params `console: Console, random_int: Random<Int>`, threaded through call sites |
| `use Handler(...)` | `val console: Console = Handler(...)` + effect-env registration for rest of scope |
| iterator fns (`yield`) | `return Iterable<T> { iterator { ... yield(x) ... } }`; bare `return` → `return@iterator` |
| `define fn` / `define type` | inline expansion at call/type sites, `${arg}` & `${...variadic}` substitution, `imports:` hoisted per file |
| interpolation `"${a.b}"` | Kotlin templates (short `$name` form when simple) |
| dot-notation `x.f(y)` | normalized to `f(x, y)` when `f` resolves to fn/define/effect member, else kept as method call (Kotlin interop) |
| `let` | `val`, or `var` when the name is assigned or `++`-incremented anywhere in the fn (mutation pre-scan) |
| variadics `...xs: T[]` / spread arg `...xs` | `vararg xs: T` / `*xs` |
| `main() [use]` | `fun main()` (package `salvo`, entry `salvo.MainKt`) |

Deliberate M2 cuts — reported as codegen **errors** (never silent bad code):
general unions (only `T | None`), `when`, loop-as-value, `break <value>`,
`while x is T`, qualifier `is`-checks with qualifiers, multi-spread struct
literals, early `return` inside lambdas, tuples beyond Pair/Triple,
struct-literal without inferable type.

### Current architectural facts worth knowing

- **Flat namespace**: `Symbols::collect` merges all modules; `import` is
  parsed but ignored. Overload resolution is arity-based only
  (`Symbols::resolve_fn`) — e.g. `size(Str)` vs a hypothetical `size(List)`
  collide; the test suite works around it with `list_size`.
- **No typechecker yet.** The emitter works syntax-directed. Everything in
  M3+ hinges on adding one.
- Modules are emitted to `<module/path>.kt`, all in the single Kotlin package
  `salvo` (collisions possible; acceptable until the resolver lands).
- A std module is emitted only if it produces code (struct/effect/handler/fn
  with body) — currently just `core/console.kt`. "Only used modules" per the
  README is not yet enforced (unused std modules would be emitted too).
- Deductions (`-> [list: Mut] T`) are parsed and preserved in the AST but
  ignored by the Kotlin backend (they matter for the Rust backend).
- The effect environment is a `Vec<(canonical-type-string, kotlin-expr)>`;
  matching is by emitted type string with a same-base-name fallback for
  generic callee effects. Good enough until the typechecker owns this.

## Remaining milestones

### M3 — Typechecker + unions/when (the big one; do first)

Everything else is blocked on this. Suggested new crate module:
`salvo-core/src/{resolve,types,check}.rs`.

1. **Name resolution honoring modules/imports**: per-module scope; `core.*`
   implicit; `import a.b.c` / `as` aliases; ambiguity diagnostics. Replaces
   the flat `Symbols` (keep `Symbols` as the resolver's output, but keyed per
   module).
2. **Type representation** (`types.rs`): interned `Ty` with `Named`,
   `Union` (normalized: flattened, deduped, `T? ≡ T | None`), qualifier sets
   on named types (`Ok Str`), generics, arrays, fn types, tuples, `Any`,
   `Nothing`. Union simplification and subtype relation
   (`Nothing <: T <: Any`; `Qual T <: T`; arm-wise union subtyping).
3. **Checker** (`check.rs`): infer/check every expression, producing a
   side-table `ExprId -> Ty` (add stable IDs or use spans as keys). Flow
   narrowing: `is` checks narrow the subject binding in then/else branches
   and while bodies, per the README rules (including qualifier-subset
   narrowing and `elif` exclusion). `when` exhaustiveness over union arms.
   Overload resolution by parameter types (fixes the `size` collision).
   Variable "type can never widen" rule + no-shadowing rule.
4. **Union codegen (Kotlin)**: per README — sealed interface wrappers sized
   1..N. Sketch: generate once per compilation into `salvo/unions.kt`:
   `sealed interface Union2<A, B>; data class U2A<A, B>(val value: A) : Union2<A, B>; ...`
   With the type table, insert wrap at assignment/argument/return boundaries
   and unwrap at `is` branches (`when (u) { is U2A -> ... }`). Qualifier tags
   (`Ok Str | Err Str`) need the qualifier to be part of the arm identity —
   encode arms positionally (arm index = position in the *declared* union
   type), not structurally.
5. **`when` expressions** → Kotlin `when` over the sealed wrapper, plus
   if-chains for qualifier predicates.
6. Rewire the emitter to consult the type table instead of its syntactic
   heuristics (bare struct literals, effect matching, overloads, `Mut`).

Definition of done: README's `Result`-style examples (`Ok Str | Err Int`,
`when` with exhaustiveness, precise `is Err Str` checks) compile and run
under kotlinc, with tests like `kotlinc_compiles_and_runs_demo`.

### M4 — Qualifiers with semantics

- Predicate qualifiers: `is Positive` calls `qualifies()` at runtime;
  narrowing tracked by the checker. Struct-field overrides (`Surname.surname:
  Str`) cast+assert on access.
- Constructive qualifiers: `as` effect verified by the checker (only in the
  qualifier's own file); erased in Kotlin except where a define/internal
  mapping says otherwise (`Mut List` → `MutableList` already exists as the
  model).
- `with` compatibility checking; duplicate-qualifier rejection.
- Kotlin: qualifiers are type-level only (erased), except their effect on
  union arm identity (from M3) and predicate calls.

### M5 — Effects, properly

- Checker validates: every call's effects are either in the caller's list or
  `use`d; `use` only in `[use]` fns; duplicate-effect-type rejection;
  generic effect disambiguation (`next_random<Int>()`) via types instead of
  the current string matching.
- Diagnostics move from codegen-time strings to spanned diagnostics.

### M6 — Deductions + loops-as-values

- Deduction inference (strictest deduction over all uses, per README) and
  validation against explicit annotations. Kotlin ignores them; they are the
  Rust backend's ownership/borrow contract, so compute + store them in the
  typed IR now.
- Loop values: `while`/`for` as expressions with `break value` and `else`
  blocks. Kotlin lowering sketch: `run { ... }` block with a labeled loop,
  assigning to a local before `break`.

### M7 — Polish + README compliance

- "Only used modules are transpiled": reachability from `main` (or all user
  fns) over the resolved call graph.
- Per-module Kotlin packages + generated imports (replace the single
  `package salvo`).
- Define coverage check at compile time: every reachable `external` item
  must have a define for the selected backend (currently only surfaces when
  a call site fails to resolve).
- Companion-file copying (`complicated.kt` support from the README).
- `T[]` may want `IntArray`/`DoubleArray` specializations.
- Effect-param name collision handling; `__destructured` temp uniquing (two
  struct-destructuring `let`s in one block currently collide).
- CLI: `--backend` default?, `clean` of stale target files, better
  `--emit-ast` filtering.

### M8 — Rust backend

Only start after M3/M6 (needs the typed IR + deductions). Reuse the
`Backend` trait; unions → enums; effects → trait objects or generics;
deductions decide `&`/`&mut`/move; `define` files `*.rust.sv` (std needs
them written); `Mut` → `mut`/`&mut`.

## Test inventory (all green)

- `salvo-core`: 4 unit tests (file classification).
- `salvo-syntax`: 17 — std + README-corpus parse-clean assertions with insta
  AST snapshots (`tests/corpus/*.sv`), error-reporting tests.
- `salvo-backend-kotlin`: 4 — golden snapshot of generated Kotlin for the
  demo, two negative tests (missing effect handler, general-union rejection),
  and the kotlinc compile+run test with exact stdout assertion.

When intentionally changing std, the parser AST, or the emitter output, rerun
with `INSTA_UPDATE=always` and review the snapshot diffs.

## Gotchas / lessons learned

- Kotlin smart casts make some emitted `as` casts redundant (kotlinc warns
  "no cast needed") — harmless, but the typechecker could skip emitting the
  binding cast when the subject is a stable val.
- `define` templates that call something with the same name as the effect
  member they implement must qualify it (hence `kotlin.io.print`).
- Overload resolution pitfalls surface as *Kotlin* compile errors today
  (e.g. `size`); after M3 they must be Salvo-side diagnostics.
- insta snapshot tests fail on first run by design; accept with
  `INSTA_UPDATE=always`.
- The parser's `Number` type alias in std is a general union — fine, because
  type aliases are only expanded on use, and nothing uses `Number` yet.
