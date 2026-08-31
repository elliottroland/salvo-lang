# Salvo Compiler — Progress & Plan

Status snapshot as of 2026-08-31 (evening). This document is the handoff
point for continuing development: it records what is built, the key design
decisions, known limitations, and a detailed plan for the remaining
milestones.

## How to build and test

```bash
cargo build                 # workspace build, no warnings
cargo test                  # 34 tests; includes two kotlinc compile+run tests
                            # (skipped gracefully if kotlinc is not on PATH)
INSTA_UPDATE=always cargo test   # accept/update insta snapshots after intended changes

# End-to-end:
cargo run -- compile --backend kotlin --src ./some_dir --target ./out
cargo run -- compile --backend kotlin --src ./some_dir --target ./out --emit-ast  # debug AST dump

# Verify generated Kotlin manually:
kotlinc out/main.kt out/unions.kt out/core/console.kt -d classes && kotlin -cp classes salvo.MainKt
```

## Workspace layout

```
crates/
├── salvo-cli/            # binary "salvo": clap CLI, backend registry, embeds std/ via include_dir
├── salvo-syntax/         # lexer, parser, AST, spans, diagnostics (no deps)
│   └── tests/corpus/     # LANGUAGE.md-example .sv files + insta snapshots
├── salvo-core/           # SourceSet, Program, Symbols + resolve.rs/types.rs/check.rs (M3)
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

- Hand-written lexer + recursive-descent parser covering the entire LANGUAGE.md
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
- Fixed inconsistencies in LANGUAGE.md + std (typos, `Iterator<T>`→`Iter<T>`,
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

Deliberate cuts still reported as codegen **errors** (never silent bad code):
loop-as-value, `break <value>`, `while x is T`, predicate-qualifier `is`
checks on non-union values, multi-spread struct literals, early `return`
inside lambdas, tuples beyond Pair/Triple, struct-literal without inferable
type.

### M3 — Typechecker + unions/`when` (Kotlin end-to-end)

New `salvo-core` modules; the Kotlin emitter now consults the checker's
side tables instead of most syntactic heuristics.

- **`types.rs`** — semantic `Ty`: `Named`, `Qualified` (sorted qual set,
  e.g. `Ok Int`), `Union` (flattened, deduped, *declaration order preserved*
  — arm order is the wrapper arm identity), `Tuple`, `Array`, `Fn`,
  `Var` (generic), `Any`, `Nothing`, `Unknown`. Subtyping
  (`Nothing <: T <: Any`, `Qual T <: T`, arm-wise unions); `Unknown` is
  compatible both ways so unchecked code never cascades errors.
- **`resolve.rs`** — per-file `ModuleScope`: own module (all files of the
  module, including backend define files) + implicit `core.*` + `import`s
  with `as` aliases. Unresolved/ambiguous imports are rendered errors.
  Import prefixes match module paths exactly or as a leading path
  (`import core.Str` finds `core.string`).
- **`check.rs`** — checks every fn body (params, handler state/ctor params
  in scope). Output `Checked` tables keyed by `(file_idx, span)`:
  - `expr_ty`: logical type of each expression (post-narrowing);
  - `repr_ty`: declared (physical) type for narrowed ident uses;
  - `coerce`: `WrapUnion { target, arm }` / `Rewrap { from, to }` at
    boundaries (let/assign/return/args/branch values);
  - `is_tests`: `UnionTest { size, arms, nullable, match_none }` keyed by
    the `is`-expr or `when`-branch span (how the check lowers at runtime);
  - `call_fn`: type-resolved overload per call site (fixes the `size`
    Str-vs-List collision; scoring prefers exact matches and qualified
    params, per the LANGUAGE.md `full_name` example).
  - Flow narrowing: `is` narrows ident subjects in then/else, `elif`
    exclusion, `&&`/`||`/`!` propagation, binding declaration, narrowing
    reset for variables assigned inside branches. Rules enforced:
    no shadowing, no widening assignment, `when` needs a union-typed
    variable subject, `when` exhaustiveness (sequential arm consumption),
    branch-matches-nothing, `None` not an arm, ambiguous-arm wrap.
  - Deliberately lenient elsewhere: unknown names/fields/methods stay
    `Unknown` (Kotlin interop pass-through), effects are not yet validated
    (M5), predicate qualifiers not yet callable (M4).
- **Kotlin union encoding** — non-`None` arms become
  `UnionN<T1..TN>` (qualifiers erased, arm identity positional); a `None`
  arm becomes outer nullability (`Union2<..>?`); 1 non-`None` arm stays
  `T?`. `unions.kt` is generated with sealed wrappers for every size used:
  `sealed interface Union2<out T1, out T2> { val value: Any? }` +
  `data class U2_1/U2_2(override val value: Ti)`.
  - Wrap at boundaries: `U2_1<Int, String>(expr)`; re-wrap between union
    reprs via `expr.let { when (it) { is U3_2<*,*,*> -> U2_1<...>(it.value as ...) ... } }`.
  - `is` checks: `x is U3_2<*, *, *>` (multi-arm → `||` chain, all arms →
    `!= null`/`true`, `is None` → `== null`, `T?` repr → null tests).
  - Ident uses narrowed to a single arm unwrap in place:
    `(x.value as Int)`; interpolating a still-union value appends `.value`.
  - `when` → Kotlin `when (subj)` over the sealed wrappers (checker
    guarantees exhaustiveness; kotlinc re-proves it); `T?` subjects lower
    to a subject-less `when` whose last branch becomes `else`.
- Emitter restructuring: `emit_expr` = `emit_expr_base` (ident unwraps) +
  `apply_coercion`; raw variants for assign targets/`is` subjects/`++`.
  Call emission prefers `call_fn`-resolved declarations; external
  signatures route to their define template by shape. Generic type aliases
  now expand in the emitter too (`subst_ast_type`).

Verified end-to-end (`kotlinc_compiles_and_runs_unions`): `Ok Int | Err Str`
construction via `as Ok`/`as Err`, `when` value + statement forms, precise
`is Err Str` on a 3-union with elif/else exclusion narrowing — compiled by
kotlinc and exact stdout asserted.

### Current architectural facts worth knowing

- **Resolution/checking pipeline**: `emit_program` runs
  `Symbols::collect` (flat, still used for define templates and arity
  fallbacks) → `salvo_core::resolve` (per-file scopes) →
  `salvo_core::check_program`. Type errors abort emission and surface as
  `BackendError::Codegen` (still strings; spanned rendering happens inside
  the checker via `Diagnostic::render`).
- The checker is *lenient by design*: anything it cannot type is
  `Ty::Unknown` and emits like before (Kotlin interop pass-through).
  Coercions/unwraps only fire where the tables say so — the emitter's
  syntactic paths remain the fallback everywhere else.
- Wrapper-union arm identity is positional over the **declared** type's
  non-`None` arms; narrowing never re-wraps a variable in place (uses are
  unwrapped/re-wrapped at expression sites instead).
- Modules are emitted to `<module/path>.kt`, all in the single Kotlin
  package `salvo` (collisions possible). `unions.kt` is emitted whenever
  any wrapper size is used.
- A std module is emitted only if it produces code — currently just
  `core/console.kt`. "Only used modules" per LANGUAGE.md is not yet enforced.
- Deductions (`-> [list: Mut] T`) are parsed and preserved in the AST but
  ignored by the Kotlin backend (they matter for the Rust backend).
- The effect environment is still string-keyed
  (`Vec<(canonical-type-string, kotlin-expr)>`); the checker does not yet
  validate effects (M5).

## Remaining milestones

### M3 leftovers (small, do alongside M4)

- `while x is T` conditions (rebinding per iteration) — still a codegen
  error; the checker already narrows the body, only the emission is missing.
- Struct-field subjects of union type in `is`/`when` (only ident subjects
  get union-test lowering; `T?` fields work via Kotlin smart casts).
- `Ty::Var` bounds/occurs checks in `unify` are loose (first-binding wins);
  fine for the std surface, revisit with real generic libraries.
- Non-fn name collisions across visible modules silently last-win in
  `resolve.rs` (only imports get ambiguity errors).
- Coercion of union values inside arrays/tuples/lambda returns is not
  recorded (only direct boundary positions).

### M4 — Qualifiers with semantics (next)

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

- Deduction inference (strictest deduction over all uses, per LANGUAGE.md) and
  validation against explicit annotations. Kotlin ignores them; they are the
  Rust backend's ownership/borrow contract, so compute + store them in the
  typed IR now.
- Loop values: `while`/`for` as expressions with `break value` and `else`
  blocks. Kotlin lowering sketch: `run { ... }` block with a labeled loop,
  assigning to a local before `break`.

### M7 — Polish + LANGUAGE.md compliance

- "Only used modules are transpiled": reachability from `main` (or all user
  fns) over the resolved call graph.
- Per-module Kotlin packages + generated imports (replace the single
  `package salvo`).
- Define coverage check at compile time: every reachable `external` item
  must have a define for the selected backend (currently only surfaces when
  a call site fails to resolve).
- Companion-file copying (`complicated.kt` support from LANGUAGE.md).
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

## Test inventory (all green: 34)

- `salvo-core`: 8 unit tests (file classification; `types.rs` union
  normalization, subtyping, display, wrapper detection).
- `salvo-syntax`: 17 — std + LANGUAGE.md-corpus parse-clean assertions with insta
  AST snapshots (`tests/corpus/*.sv`), error-reporting tests.
- `salvo-backend-kotlin`: 9 — golden snapshots of the M2 demo and the M3
  unions demo, wrapper/wrap/`is`-lowering assertions
  (`unions_emit_sealed_wrappers`), three negative tests (non-exhaustive
  `when`, non-union `when` subject, no-matching-arm wrap), missing effect
  handler, and two kotlinc compile+run tests with exact stdout assertions.

When intentionally changing std, the parser AST, the checker's lowering, or
the emitter output, rerun with `INSTA_UPDATE=always` and review the
snapshot diffs.

## Gotchas / lessons learned

- Kotlin smart casts make some emitted `as` casts redundant (kotlinc warns
  "no cast needed") — harmless. The `(x.value as T)` unwrap casts are
  *required* though: narrowing may come from `elif` exclusion where Kotlin
  has no smart cast, and `value` is typed `Any?` on the sealed interface.
- `is UN_i` checks need star projections (`is U2_1<*, *>`) — kotlinc
  rejects bare generic classes in `is`. Sealed exhaustiveness still works.
- A subject-less Kotlin `when` used as an expression demands an `else`
  branch — the emitter converts the last branch of a `T?`-subject `when`
  to `else` (sound because the checker proved exhaustiveness).
- The checker and emitter must agree on the ident-unwrap rule
  (`repr is wrapper && logical is a single non-None arm`); `maybe_coerce`
  computes the "effective repr" with the same predicate the emitter uses.
- `define` templates that call something with the same name as the effect
  member they implement must qualify it (hence `kotlin.io.print`).
- Overload resolution now happens in the checker; `size(Str)` vs
  `size(List<T>)` style collisions are resolved by argument type. The
  emitter still falls back to arity in unchecked contexts.
- insta snapshot tests fail on first run by design; accept with
  `INSTA_UPDATE=always`.
- The `Number` type alias in std is a general union — fine: aliases expand
  on use (now with generic substitution in both checker and emitter), and
  nothing uses `Number` yet.
