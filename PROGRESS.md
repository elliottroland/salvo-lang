# Salvo Compiler — Progress & Plan

Status snapshot as of 2026-09-01 (morning, after M5). This document is
the handoff point for continuing development: it records what is built, the
key design decisions, known limitations, and a detailed plan for the
remaining milestones.

Companion documents: LANGUAGE.md is the narrative spec (source of truth);
LANGUAGE_SPEC.md states every feature as a labeled rule (`[qual-erasure]`
style) with the compiler decisions under it; BACKEND_SPEC.<backend>.md
(currently `BACKEND_SPEC.kotlin.md`) repeats rules with backend
interpretation details and adds backend-prefixed rules (`kt-…`) — load it
only when working on that backend. Labels are referenced from compiler
code and tests (`grep -rn '\[rule-name\]'`); backend-prefixed labels may
only be referenced from that backend's crate. Keep all of these in sync
when adding or changing features.

## How to build and test

```bash
cargo build                 # workspace build, no warnings
cargo test                  # 56 tests; includes four kotlinc compile+run tests
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
| `external handler` + `define handler` | `class H(ctor params) : Effect` with template-inlined bodies |
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
loop-as-value, `break <value>`, multi-spread struct literals, early `return`
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
    (M5), predicate qualifiers callable since M4.
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
construction via `ok()`/`err()` constructor fns (M4 syntax), `when` value +
statement forms, precise `is Err Str` on a 3-union with elif/else exclusion
narrowing — compiled by kotlinc and exact stdout asserted.

### M4 — Qualifiers with semantics

Design change (user decision, replacing the old spec): the `as` *effect* and
value-level `as` *expressions* are gone from the language. Constructive
qualifiers are built exclusively through **constructor functions** marked
with `-> T as Qualifier` in the return position: every return point returns
a plain `T` (the qualifier is applied *by construction*), callers see
`Qualifier T`. Union tagging now goes through generic constructors
(`fn ok<T>(value: T) -> T as Ok { return value }` … `return ok(input)`).
LANGUAGE.md + README were rewritten accordingly.

- **Constructive qualifiers** (bodiless decls): constructor fns must be in
  the same file as the qualifier; predicate qualifiers cannot have
  constructors; constructor return types must be simple (no union/tuple);
  the constructed qualifier must satisfy the `of` type. Since plain values
  never subtype `Qual T`, constructors really are the only way in.
- **Predicate qualifiers** (decls with a body): `is Positive` on a non-union
  subject records a `predicate_tests` entry; Kotlin emits the qualifier's
  `qualifies` fn as a top-level `fun Positive_qualifies(...)` and the check
  becomes a call (multiple quals `&&`-chain; `qualifies` effects are threaded
  as leading handler args). Narrowing adds the qualifiers to the subject's
  type in the then-branch (no else information). `qualifies` signatures are
  validated (1 param accepting the `of` type, returns `Bool`, no
  deductions).
- **Struct-field overrides** (`qualifier Surname of Person { surname: Str …}`):
  field accesses through a qualified base type get the override type and a
  `field_casts` entry; Kotlin emits `(person.surname as String)`
  (cast + assert per spec). Overrides must refine real fields (subtype
  check). Struct *destructuring* deliberately uses declared field types.
- **Type-annotation validation** (`validate_type` at declaration sites: fn
  signatures, `let` annotations, struct fields, qualifier decls): duplicate
  qualifier application, `of`-type applicability (via `unify`), pairwise
  `with` compatibility. `internal` qualifiers (`Mut`) compose with
  everything; `Mut` on a struct is checked against `with Mut`
  auto-qualifiers. Unresolvable qualifier names are skipped (lenient).
- **Qualified union groups** `Ok (A | B)` (user request): lowered as
  `Ty::Qualified { quals, base: Union }`. They wrap as a whole when they are
  themselves an arm of an expected union (`maybe_coerce` tries whole-group
  arm equality first), and otherwise coerce as the bare inner union
  (physically identical after erasure). Subtyping got a dedicated rule
  (group may drop its quals, tried after exact-arm equality). `Display`
  parenthesizes union bases.
- **Overload mangling under erasure**: qualifiers erase in Kotlin, so
  `full_name(Person)` vs `full_name(Surname Person)` would collide. When
  two overloads have the same *emitted* parameter signature, the qualified
  one gets a deterministic `__Qual` suffix (`full_name__Surname`), applied
  consistently at both the declaration and checker-resolved call sites.
- **M3 leftover fixed**: `while x is T (name)?` now emits (test in the loop
  condition, binding re-declared per iteration at the top of the body).
- Syntax: `FnDecl.constructs: Option<TypeRef>`; `EffectRef::As` and
  `Expr::As` removed from the AST/parser.

Verified end-to-end (`kotlinc_compiles_and_runs_qualifiers`): predicate
checks + narrowing, field-override casts, mangled qualifier overloads,
`while x is Int c` countdown, and a nested `Ok (Ok Str | Err Int) | Err Bool`
round-trip — compiled by kotlinc and exact stdout asserted.

### M5 — Effects, properly

The checker now owns effect semantics; the emitter consumes its tables.
Effect errors are spanned checker diagnostics instead of codegen-time
strings.

- **Checker effect environment** (`Checker.effect_env: Vec<Ty>` +
  `can_use`): seeded from the fn's declared effect list, grown by `use`
  statements, truncated at block boundaries (mirroring the emitter's
  scoping). Validated at fn declarations: unknown effect names, wrong
  type-argument counts, and duplicate instances (`[Console, Console]`) are
  errors; same effect with different generics (`[Random<Int>,
  Random<Double>]`) is fine per the spec.
- **`use` validation** (`check_use`): requires the `use` effect in the
  current fn's list; the handler must resolve; constructor args are typed
  and the handler's generics are *inferred from them by unification* — so
  `use CyclicRandom(list(1,2,3))` registers concrete `Random<Int>`, not
  `Random<T>` (which previously leaked into emitted Kotlin as an unresolved
  `T`). Registering a second handler for the same instance is an error.
- **Effect member calls** (`check_effect_call`): the providing instance
  must be in the environment ("no handler for effect `X` in scope").
  Generic disambiguation, in order: explicit type args
  (`next_random<Int>()`), argument types, then the *expected type*
  (`let int: Int = next_random()` picks `Random<Int>`) — `expected` is now
  threaded through `check_call`/`resolve_named_call` for this. Multiple
  survivors → "ambiguous effect call" error, zero → "no handler" error.
- **Fn call sites** (`check_callee_effects`): each callee effect dependency
  (with the call's generic substitution applied) must match an instance in
  the caller's environment — exact match first, then a unique
  unify-compatible match. Predicate-qualifier `is` checks validate the
  `qualifies` fn's effects the same way (by base name).
- Effect/handler *member* fns cannot declare effect dependencies (error
  "… cannot declare effect dependencies yet"): dispatch call sites go
  through the handler instance and cannot thread extra handler args.
- **New `Checked` tables** consumed by the emitter (all keyed by
  `(file_idx, span)`): `use_effects` (concrete instance per `use` stmt),
  `effect_calls` (instance an effect-member call dispatches through),
  `call_effects` (instances threaded as leading handler args per fn call,
  in the callee's declaration order).

**Emitter fallback mechanism (deliberate, revisit later).** The emitter's
effect environment is still string-keyed (`Vec<(kotlin-type-string,
kotlin-expr)>`), built from the declared effect refs at fn entry and from
`use` statements. What changed: at each site the emitter first consults the
checker table and renders the recorded `Ty` through a new `kotlin_ty(Ty)`
(which must agree with `emit_type` on the same source type — that agreement
is what makes exact env-key hits work), and only falls back to the old
string/base-name matching (`lookup_effect_handler*`) when the table has no
entry or the recorded type contains `Unknown` (`ty_is_concrete` guard).
The fallback keeps the lenient-checker contract: unchecked contexts
(arity-resolved calls, Kotlin-interop pass-through) still emit like before,
and a checker regression degrades to the old string matching rather than
wrong code (codegen errors still fire if that also fails). The cost is
double bookkeeping — two environments that must stay consistent — and the
subtle `kotlin_ty`/`emit_type` agreement requirement. When the emitter
eventually keys its environment by checker `Ty` directly (or the typed IR
lands), the string env and `lookup_effect_handler*` can be deleted.

- **std**: `List<T>` gained `size` (user request) — the `size(Str)` vs
  `size(List<T>)` overloads share a define name, so `define_for_decl` now
  prefers templates whose parameter *base types* match the resolved
  declaration before falling back to arity/shape (`type_base_name`).
- LANGUAGE.md fix: the effects examples used Kotlin's `val` instead of
  `let` (`random_numbers` example).

Verified end-to-end (`kotlinc_compiles_and_runs_effects`): two
`CyclicRandom` instances (`Random<Int>` + `Random<Str>`) registered via
generic inference at `use` sites, expected-type disambiguation inside a fn
declaring both, explicit `next_random<Int>()`, and handler threading through
call sites — compiled by kotlinc and exact stdout asserted.

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
- The effect environment in the emitter is string-keyed; effect resolution
  prefers the checker's `use_effects`/`effect_calls`/`call_effects` tables
  and falls back to string matching only in unchecked contexts (see the M5
  fallback-mechanism note above).

## Remaining milestones

### M3–M5 leftovers (small, do alongside M6)

- Struct-field subjects of union type in `is`/`when` (only ident subjects
  get union-test lowering; `T?` fields work via Kotlin smart casts).
- Struct destructuring ignores predicate-qualifier field overrides
  (deliberate: bindings get the declared type; direct accesses get the
  override + cast).
- `Ty::Var` bounds/occurs checks in `unify` are loose (first-binding wins);
  fine for the std surface, revisit with real generic libraries.
- Non-fn name collisions across visible modules silently last-win in
  `resolve.rs` (only imports get ambiguity errors).
- Coercion of union values inside arrays/tuples/lambda returns is not
  recorded (only direct boundary positions).
- Overload mangling only fires for checker-resolved call sites; unchecked
  (arity-fallback) calls to a mangled overload would emit the base name.
  Same class of gap: `Symbols::resolve_define_fn` (arity fallback) can pick
  the wrong same-name define (`size`) in unchecked contexts.
- Constructing a *nested* qualified union group in one expression
  (`ok(ok("yes"))` into `Ok (Ok Str | Err Int) | …`) needs an annotated
  intermediate `let`; single-level coercion only (errors, never mis-emits).
- Retire the emitter's string-keyed effect environment in favor of
  checker-`Ty` keys (see the M5 fallback-mechanism note).
- The LANGUAGE.md `CyclicRandom` example calls `values.size()` on a `T[]`;
  std only defines `size` for `Str` and `List<T>` — either add an array
  `size` or move the example to `List<T>`.
- Effect member fns with their *own* generics are lowered but never
  substituted per-call (only the effect's generics are).

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

## Test inventory (all green: 56)

- `salvo-core`: 8 unit tests (file classification; `types.rs` union
  normalization, subtyping, display, wrapper detection).
- `salvo-syntax`: 17 — std + LANGUAGE.md-corpus parse-clean assertions with insta
  AST snapshots (`tests/corpus/*.sv`), error-reporting tests.
- `salvo-backend-kotlin`: 31 — golden snapshots of the M2 demo, the M3
  unions demo, the M4 qualifiers demo, and the M5 effects demo;
  wrapper/wrap/`is`-lowering assertions (`unions_emit_sealed_wrappers`),
  predicate/mangling/field-cast assertions
  (`qualifiers_lower_to_predicates_and_mangled_overloads`), checker-driven
  effect-resolution assertions (`effects_resolve_through_checker_tables`);
  negative tests (non-exhaustive `when`, non-union `when` subject,
  no-matching-arm wrap, missing effect handler at a fn call site and at an
  effect-member call site, `use` without the `use` effect, duplicate effect
  in an effect list, duplicate `use` registration, unknown effect,
  ambiguous generic effect call, handler-member effect deps, duplicate
  qualifier, incompatible qualifiers, `of`-type mismatch, constructor
  same-file rule, predicate-constructor rejection, non-simple constructor
  return, `is` on constructive qualifiers, `qualifies` signature,
  constructive values only from constructors); and four kotlinc compile+run
  tests with exact stdout assertions.

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
- Subtype-rule *order* matters for qualified union groups: the
  `(_, Union)` any-arm rule would otherwise compare `Ok (A | B)` against
  single arms and always fail; the group rule (exact-arm equality, then
  drop-quals) must come before it. Symmetrically, `maybe_coerce` must try
  wrapping the group as a whole arm *before* stripping its qualifiers.
- `substitute_vars` must re-normalize `Ty::Qualified` through `qualify()`:
  a generic constructor's `Ok T` with `T = Ok Str` would otherwise nest
  `Qualified` inside `Qualified` (breaking the type invariant).
- Qualifier validation happens at *declaration sites* (`validate_type` in
  check_fn / let / struct fields), not inside `lower_type` — lowering runs
  repeatedly (e.g. per overload candidate), which would duplicate errors.
- Overload mangling compares *emitted* Kotlin parameter strings, so the
  `Mut List` → `MutableList` mapping naturally avoids false collisions.
- Predicate `is` on a subject already narrowed out of a wrapper union works
  because `emit_expr_base` (the unwrap) is used for the `qualifies` call
  argument.
- All handlers — external ones included — emit as Kotlin *classes* and are
  instantiated at their `use` site (user decision: `object` was an artifact
  of StdOutConsole being stateless). Bare `use Handler` is sugar for
  `use Handler()`; external handlers support constructor params like any
  other handler.
- `kotlin_ty(Ty)` (checker type → Kotlin) must agree with `emit_type`
  (AST type → Kotlin) on the same source type: checker-resolved effect
  types are looked up in an effect environment keyed by `emit_type`
  renderings. Both expand type aliases and map internal names through
  `emit_named_parts`, which is what keeps them aligned. If they drift, the
  lookup degrades to base-name fallback matching (or a codegen error) —
  never silently wrong dispatch, but worth knowing when adding type forms.
- The checker types effect-member call *args* only when it must
  disambiguate between multiple instances (multi-candidate path types them
  with `None` expected, then records coercions afterwards); the
  single-candidate path checks args directly against the substituted param
  types, preserving expected-type-driven inference for lambda/struct-lit
  arguments. Keep the two paths in sync when touching `check_effect_call`.
- `Stmt::Use` no longer runs `check_expr` on the whole handler expression
  (ctor args are typed individually in `check_use`), so the handler `Call`
  expr itself has no `expr_ty` entry — the emitter doesn't need one, but
  don't add a table lookup keyed on it.
- Same-name defines for overloaded externals (`size(Str)` vs
  `size(List<T>)`) are disambiguated in `define_for_decl` by comparing
  parameter *base type names* against the checker-resolved declaration —
  the arity-only fallback (`resolve_define_fn`) can still pick the wrong
  one in unchecked contexts.
- The `use` duplicate-registration check compares checker `Ty`s, so it
  catches `Random<Int>` twice while allowing `Random<Int>` +
  `Random<Str>`; the effect-list duplicate check is separate
  (`check_effect_list`) because declared effects never pass through
  `check_use`.
