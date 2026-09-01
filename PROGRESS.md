# Salvo Compiler — Progress & Plan

Status snapshot as of 2026-09-01 (after M8: the Rust backend, plus the
post-M8 tooling: `salvo analyze`, structured diagnostics, and the
`salvo lsp` language server). This document is
the handoff point for continuing development: it records what is built, the
key design decisions, known limitations, and a detailed plan for the
remaining milestones.

Companion documents: LANGUAGE.md is the narrative spec (source of truth);
LANGUAGE_SPEC.md states every feature as a labeled rule (`[qual-erasure]`
style) with the compiler decisions under it; BACKEND_SPEC.<backend>.md
(`BACKEND_SPEC.kotlin.md`, `BACKEND_SPEC.rust.md`) repeats rules with
backend interpretation details and adds backend-prefixed rules (`kt-…`,
`rs-…`) — load it only when working on that backend. Labels are referenced
from compiler code and tests (`grep -rn '\[rule-name\]'`); backend-prefixed
labels may only be referenced from that backend's crate. Keep all of these
in sync when adding or changing features.

## How to build and test

```bash
cargo build                 # workspace build, no warnings
cargo test                  # 124 tests; includes six kotlinc and six rustc
                            # compile+run tests (skipped gracefully when the
                            # toolchain is not on PATH)
INSTA_UPDATE=always cargo test   # accept/update insta snapshots after intended changes

# End-to-end:
cargo run -- compile --src ./some_dir --target ./out        # --backend defaults to kotlin
cargo run -- compile --backend rust --src ./some_dir --target ./out_rs
cargo run -- compile --src ./some_dir --target ./out --emit-ast             # user-module AST dump
cargo run -- compile --src ./some_dir --target ./out --emit-ast=core.list   # one module's AST

# Type-check without generating code [cli-analyze]:
cargo run -- analyze --src ./some_dir                       # text diagnostics, exit 1 on errors
cargo run -- analyze --src ./some_dir --format json         # machine-readable diagnostics
cargo run -- analyze --src ./some_dir --backend kotlin      # also parse kotlin define files

# Language server over stdio [cli-lsp] (point your editor's LSP client at it):
cargo run -- lsp

# Regenerate the VS Code extension's TextMate grammar [cli-lang]
# (a test fails if the checked-in copy is stale):
cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json

# Verify generated Kotlin manually (the CLI prints the entry point):
kotlinc $(find out -name '*.kt') -d classes && kotlin -cp classes salvo.main.MainKt
# Verify generated Rust manually (the CLI prints the exact command):
rustc --edition 2021 out_rs/main.rs -o program && ./program
```

## Workspace layout

```
crates/
├── salvo-cli/            # binary "salvo": clap CLI, backend registry, embeds std/ via include_dir
├── salvo-syntax/         # lexer, parser, AST, spans, diagnostics (no deps)
│   └── tests/corpus/     # LANGUAGE.md-example .sv files + insta snapshots
├── salvo-core/           # SourceSet, Program, Symbols + resolve.rs/types.rs/check.rs (M3)
├── salvo-backend/        # Backend trait, BackendRegistry, BackendError
├── salvo-backend-kotlin/ # Kotlin emitter (emit.rs) + golden/kotlinc tests
└── salvo-backend-rust/   # Rust emitter (emit.rs) + golden/rustc tests (M8)
std/core/                 # stdlib: basic.sv, string.sv, list.sv, console.sv (+ .kotlin.sv/.rust.sv defines)
```

Adding another backend = new crate implementing `salvo_backend::Backend`,
register it in `salvo-cli/src/main.rs`, write `*.<name>.sv` define files
next to the std modules, and add a `BACKEND_SPEC.<name>.md`. Std embedding
already filters define files per backend at load time
(`SourceSet::classify`).

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
| `main() [use]` | `fun main()` (entry `salvo.<module>.MainKt` since M7) |

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

### M6 — Deductions + loops-as-values

**Loops as values [while-value].** `while`/`for` are now expressions in
both checker and Kotlin emitter.

- Checker: the loop's value type joins the body's tail type, every
  `break value` type, and the `else` tail type — or `None` when there is
  no `else` (the loop may never run). A bare `break` or a `continue` also
  joins `None` (deliberate, conservative: an iteration may end without
  producing a value — at runtime a bare `break` keeps the *previous*
  iteration's tail value, which the optional type soundly covers; `!`
  recovers the non-optional type). Tails and break values coerce to the
  join exactly like `if` branch values. New `Checker.loop_stack`
  (`LoopCtx { breaks, may_skip_value }`) attributes `break`/`continue` to
  the innermost loop; lambda bodies are a barrier; `break`/`continue`
  outside a loop are now errors. `while`-`else` blocks are checked under
  the condition's else-narrows (they run only if the first evaluation
  failed).
- Kotlin lowering [kt-loop-value]: a value-position loop becomes a
  `run {}` block with a `var __loopN` result local — nullable temp
  initialized `null`, ended with `__loopN!!` when the join has no `None`
  arm (`Any?` pass-through for `None`-typed/unchecked joins). The body's
  tail expression assigns the local; `break value` assigns then breaks
  (routed via an emitter `loop_results` stack mirroring the checker's);
  with `else`, a `__loopN_ran` flag guards an `if (!__loopN_ran)` block.
  `None`-typed tails/breaks stay statements and assign `null`;
  `Nothing`-typed tails stay statements. Statement-position loops keep
  plain Kotlin loops (an `else` needs only the ran-flag; a discarded
  `break value` evaluates its operand for side effects).
  `emit_value_block` routes a *trailing* loop through the value lowering
  (a Kotlin block would otherwise value the loop statement as `Unit`).

**Deductions [deduce-syntax] [deduce-infer].** New `salvo-core/deduce.rs`
post-pass (runs at the end of `check_program`); results stored in the
typed IR as `Checked::deductions: HashMap<FnKey, Vec<ParamDeduction>>`
(`{ param, kept, quals }`). Kotlin ignores them; they are the Rust
backend's ownership/borrow contract.

- **Interpretation (user decision):** a deduction is relative to the
  qualifiers *declared on the callee's parameter* — a call removes
  exactly the set `declared − kept` from the argument's known
  qualifiers. Qualifiers beyond the declared ones pass through
  (`fn f(list: A B List<T>) -> [list: B]` on an `A B C List<T>` argument
  leaves `B C List<T>`).
- Inference (unwritten lists): whole-program fixpoint from an optimistic
  start (everything kept with declared quals); constraints only remove
  facts (monotone → terminates). Moves: bare parameter passed to a call
  whose deduction omits it, bound by `let`/assignment, returned,
  `break`/`yield`-ed, stored in a struct/array/tuple literal, or passed
  to a `use` handler constructor. Calls resolve through the checker's
  `call_fn` table (dot-notation receiver = argument 0; trailing args bind
  the variadic param). Lenient: unresolved callees (interop, effect
  members) borrow and preserve everything; bare-parameter value flow out
  of branch/loop tails is not tracked as a move yet.
- Written lists are shape-checked (unknown/duplicate parameter, keeping a
  qualifier not declared on the parameter) and validated against the body
  facts: stricter-than-body is fine; promising a parameter back that the
  body moves, or a qualifier the body may remove, is an error.
- Only top-level `fn`s (the ones with `FnKey`s) participate; handler and
  qualifier member fns are outside `call_fn` resolution anyway.

Verified end-to-end (`kotlinc_compiles_and_runs_loops`): last-evaluated
value with `else` (ran and never-ran), `break value` out of a `for` over
an iterator with `T?` + `is` narrowing, statement-position `for`-`else`,
bare `break` keeping the previous value, and a union-typed loop value
re-wrapped to the declared arm order — compiled by kotlinc and exact
stdout asserted. Deduction semantics covered by 8 new salvo-core tests
(removal-set subtraction, pass-through of undeclared qualifiers, move
inference, call-graph fixpoint transitivity, lenient interop, validation
errors).

### M7 — Polish + LANGUAGE.md compliance

**Only used modules [mod-used-only].** New `salvo-core/reach.rs`: roots
are the user modules declaring `fn main` (all user modules when none —
library compile); reachability follows *name usage* — every identifier
and type name a file mentions is looked up in its resolved scope and
each declaring module becomes reachable. `ModuleScope` gained
`name_origins: name -> declaring modules` (populated in `add_items`,
aliases recorded under the alias). Deliberately conservative (shadowed/
overloaded names pull in every declaring module) — never drops a module
emitted code could reference. Unreachable modules are not emitted;
`unions.kt` is now generated from the *emitters'* tracked sizes only.

**Per-module packages + generated imports [kt-package] [kt-imports].**
Each module emits into `salvo.<module.path>` (was: single `package
salvo`), killing cross-module collisions; `unions.kt` stays in root
`salvo`. Files get generated imports: a wildcard `import salvo.<mod>.*`
per foreign *emitted* module whose names they use (computed from the
same `used_names` + `name_origins` machinery), `import salvo.*` when the
file uses union wrappers, plus template `imports:` as before. Aliased
Salvo imports of Kotlin-visible items (fn with body, struct, effect,
handler) emit `import salvo.<mod>.<name> as <alias>` and call sites keep
the alias (`emit_fn_call` uses the source name when it differs from the
decl); inlined externals/type aliases/qualifiers get no alias import.
Entry point is now `salvo.<module>.MainKt` (CLI prints it).

**Define coverage [backend-external].** Compile-time checks: (a) upfront
— every external fn/type/handler in `core.*` must have a define for the
backend (core is implicitly imported); (b) at reference sites — calls to
external fns without a define error in both the checker-resolved and
arity-fallback paths, references to external types without a `define
type` error instead of passing through, and external handlers without a
`define handler` already errored at emission.

**Companion files [backend-companion].** `SourceSet`/`Program` carry
`CompanionFile`s (backend-native extension, e.g. `.kt`, discovered by
`add_dir`; module = directory + stem). The backend copies them verbatim
when their module is reachable; a companion colliding with a generated
file is an error (companion modules should be externals-only, per the
LANGUAGE.md `complicated.kt` pattern). `Backend` trait gained
`file_extension()`.

**Emitter fixes.** Effect parameters and `use` variables avoid user
names ([kt-effect-params]: pre-scan of params + declared locals via
`collect_declared`; collisions get `console2`-style suffixes).
`__destructuredN` temps are unique per fn ([let-destructure]). Array
specialization decision recorded under [type-array]: `T[]` stays
`Array<T>` — `IntArray`/`DoubleArray` are unrelated types in Kotlin and
would fracture generics/varargs/interop; revisit only with profiling
data.

**CLI.** `--backend` defaults to `kotlin`; stale `*.<ext>` files in the
target that this compile didn't write are deleted (only
backend-extension files are touched); `--emit-ast` now prints user
modules only, `--emit-ast=<module>` one module (std included); the entry
point class is printed after compiling.

Verified end-to-end (`kotlinc_compiles_and_runs_multi_module` + a manual
CLI run of the LANGUAGE.md companion scenario): multi-module program with
per-module packages, generated wildcard + alias imports, unused user
module dropped, companion `.kt` copied and called through its define
template, stale target file removed — kotlinc-compiled with exact stdout.

### M8 — Rust backend (+ `Mut` as a language feature)

`salvo compile --backend rust` emits a single-binary Rust crate, verified
end-to-end: six rustc compile+run tests assert exact stdout on the same
demo programs the Kotlin backend runs (structs/effects/iterators, unions,
qualifiers, generic effects, loops-as-values, multi-module). Full spec in
BACKEND_SPEC.rust.md; highlights and decisions:

- **`Mut` generalized (user decision, replacing `internal qualifier`)**
  [type-with-mut]: `Mut` is now a language-level qualifier any type
  declaration can opt into with `with Mut` (`external type List<T> with
  Mut` in std; structs unchanged). The checker validates `Mut` against
  the declaration's auto-qualifiers (error otherwise) and lets `Mut`
  compose with every other qualifier. Backends map it per type: a
  `define type` block may carry a **`Mut inline:`** section
  (`MutableList<${T}>` in `list.kotlin.sv`); without one `Mut` erases
  (the Rust defines map both `List<T>` and `Mut List<T>` to `Vec<T>` —
  mutability lives in bindings/references). Parser: `with` clause on
  `type` decls (`TypeDecl.auto_qualifiers`), `Mut inline:` define
  section (`DefineBody.mut_inline`). The Kotlin emitter's hardcoded
  `Mut List → MutableList` mapping was replaced by the template.
- **Deductions drive ownership [rs-borrows] (the point of the whole
  design):** a parameter *omitted* from a fn's deductions is **moved**
  (passed by value — Salvo guarantees the caller no longer touches it);
  a *kept* parameter is **borrowed**, `&mut T` when its declared type
  carries `Mut`, `&T` otherwise; Copy scalars and variadics always pass
  by value; fns outside the deduction tables (qualifies/handler/effect
  members) default to the kept rule. Call sites render arguments per the
  resolved callee's modes (`&x` / `&mut x` / move; parameter bindings
  reborrow implicitly). Expressions emit *owned* by default: borrowed
  idents and non-Copy field/index reads clone; owned locals move. Every
  local is `let mut` (crate-root `#![allow(unused_mut)]`); no emitted
  signature returns a reference, so no named lifetimes exist anywhere.
- **New backend-neutral coercion `WrapOption`** [type-nullable]:
  optionals are physical in Rust (`Some(...)`), transparent in Kotlin
  (no-op arm in `apply_coercion`). `maybe_coerce` records it at every
  `T → T?` boundary, and its "effective repr" rule now also treats a
  `T?`-repr ident narrowed to its value arm as unwrapped — the Rust
  emitter unwraps those uses physically (`x.unwrap()` /
  `x.as_ref().unwrap().clone()`) where Kotlin smart-casts.
- **Crate layout [rs-crate]:** the main-declaring module *is* the crate
  root (`main.sv` → `main.rs` with `#![allow(...)]` +
  `#[path = "..."] pub mod core_console;` mounts for every other emitted
  file; synthetic `lib.rs` for library compiles); modules glob-import
  each other via `use crate::<mangled>::*;` [rs-imports]; everything is
  `pub`. The CLI prints the exact `rustc --edition 2021 …` command as
  the entry point.
- **Unions [rs-union-enums]:** generated `unions.rs` enums
  (`pub enum Union2<T1,T2> { U1(T1), U2(T2) }`) with panicking per-arm
  accessors (`.u1()`) and a `Display` impl (still-union values
  interpolate directly — no `.value` dance). Wraps use turbofish
  (`Union2::<i32, String>::U1(x)`, `Some(...)` for nullable targets);
  `is` tests lower to `matches!` with `|` patterns; `when` lowers to
  `match` (an expression — no `run{}`-style lowering needed), with a
  `_ => unreachable!()` arm when narrowing left repr arms uncovered.
- **Effects [rs-effects]:** traits with `&mut self` methods; deps become
  leading `&mut dyn E` params; `use` emits `let mut h = H::new(args);`
  and threads `&mut h` (params thread as themselves — implicit
  reborrow). Handlers are struct + `new()` + trait impl; member bodies
  address ctor params/state as `self.x`. Effect members with their own
  generics are a codegen error (`dyn` incompatible). Same
  checker-table-first/string-fallback resolution as Kotlin.
- **Iterators are eager [rs-iter-vec] (deliberate divergence):**
  `Iter<T>` = `Vec<T>`; `yield` pushes into a `__yielded` vec, bare
  `return` returns it. Side-effect *timing* differs from Kotlin's lazy
  sequences; values match. Rust generators are unstable.
- **Loops as values [rs-loop-value]:** block expression + `Option`
  result local mirroring the Kotlin lowering (`.unwrap()` when the join
  has no `None` arm; optional joins assign coerced values directly).
  `i++` lowers to `({ let __t = i; i += 1; __t })` in value position,
  `i += 1;` as a statement [rs-postincrement].
- **No overloading in Rust [rs-fn-mangling]:** the Kotlin qualifier
  suffix rule applies first, then still-colliding bodied overloads get
  positional `__2`/`__3` suffixes.
- Misc: struct literals inline declared defaults (no default args in
  Rust) and lower spread to functional update with a cloned base;
  string literals emit `.to_string()`, interpolation `format!` (braces
  escaped); binary operands re-parenthesize by operator precedence (the
  AST is right, flat re-rendering wouldn't be); define-template args
  splice as raw places (method-style templates borrow natively) except
  variadic parts, which splice owned into `vec![...]`; generic params
  get a blanket `Clone` bound and structs derive `Clone, Debug`.

Verified end-to-end (6 rustc tests, exact stdout): the M2 demo, unions,
qualifiers (incl. `full_name__Surname` mangling and field-cast unwraps),
generic effects with two `CyclicRandom` instances, loops-as-values
(incl. a union-typed loop join re-wrapped through `match`), and the
multi-module crate layout. Plus a manual CLI check of a `Mut` struct
mutated through a `&mut` parameter.

### Post-M8 — `salvo analyze`, structured diagnostics, `salvo lsp`

Editor/LSP support (user request), built in two steps.

- **Structured diagnostics [diag-structured]** (new
  `salvo-core/src/diag.rs`): `Resolution::errors` and `Checked::errors`
  are now `Vec<FileDiagnostic>` — `(file index, span, severity, message)`
  — instead of pre-rendered strings. The three error sinks
  (`Checker::error`, `resolve_import`, deduce's written-list validation)
  construct them directly; rendering moved to the consuming boundary
  (`FileDiagnostic::render(&program.files)` in both backends'
  `emit_program`, and in the CLI). `Checker` lost its now-unneeded
  `file_name`/`source` fields. Backend `emit` error signatures are
  unchanged (`Err(Vec<String>)`), so `BackendError` handling and all
  negative tests still work on rendered strings.
- **`salvo analyze` [cli-analyze]**: parse + resolve + check with no
  code generation. Text mode renders diagnostics to stderr with a
  summary line; `--format json` prints a
  `{file, line, col, start, end, severity, message}` array to stdout
  (hand-rolled JSON — no new dependencies). Exit code 1 iff any
  diagnostic is an error. Resolve/check always run, even with parse
  errors: parse-broken files participate with their recovered ASTs but
  contribute only their parse diagnostics (their resolution/checker
  diagnostics are dropped — recovered ASTs cascade nonsense), so a
  broken file never suppresses diagnostics in other files. Design
  decision: analysis is *backend-neutral* — the checker
  skips non-Language files and never consults defines, so `--backend`
  merely opts that backend's define files into loading (they get parse
  checking); without it, the load filter (empty backend name) matches no
  define suffix and only language files are analyzed.
- **`salvo lsp` [cli-lsp]** (`salvo-cli/src/lsp.rs`, in the CLI crate so
  it shares the embedded std and the pipeline — now factored into
  `salvo-cli/src/analysis.rs::analyze_sources`, used by both commands):
  a language server over stdio using `lsp-server` + `lsp-types` 0.95
  (sync, no tokio; the rust-analyzer stack). No incremental state:
  every document event re-runs the whole-workspace analysis with open
  buffers as a content overlay (`analyze_sources`' `overlay` parameter;
  unsaved files under the root are added to the source set).
  Implemented: publish-diagnostics on open/change/close/save — every
  open document gets a publish (empty = clean) and disappeared files
  get a clearing publish — and hover showing `Checked::expr_ty` for
  the smallest non-`Unknown` expression span under the cursor.
  Positions convert byte offset ↔ UTF-16 line/character (the LSP
  default encoding; `offset_to_position`/`position_to_offset` with
  unit tests). Go-to-definition needs def-site spans recorded in
  `Resolution`/`Symbols` — still a leftover.
- **VS Code extension + `salvo lang tm-grammar` [cli-lang]** (user
  request): `vscode/` holds a local extension bundling a TextMate
  grammar with an LSP client that spawns `salvo lsp` over stdio
  (`vscode-languageclient`, TypeScript). `salvo.serverPath` names the
  binary — relative paths resolve against the workspace folder, so
  `target/debug/salvo` picks up a freshly built compiler; a
  "Salvo: Restart Language Server" command (and automatic restart on
  settings changes) swaps binaries without reloading the window.
  `salvo.backend` forwards `--backend` to the server. Design decision:
  the grammar is *generated by the compiler* — `salvo lang tm-grammar
  [--out PATH]` (`salvo-cli/src/lang.rs`) derives keyword alternations
  from the lexer's keyword table, which was lifted into a shared
  `salvo_syntax::token::KEYWORDS` const that `TokenKind::keyword` now
  consults. Tests enforce sync in both directions: the highlighting
  categories must exactly partition `KEYWORDS`, and the checked-in
  `vscode/syntaxes/salvo.tmLanguage.json` must byte-equal the generated
  output (regenerate with the command above). Build the extension with
  `npm install && npm run compile` in `vscode/` (see `vscode/README.md`;
  `vscode/.npmrc` pins the public npm registry).
- **Source discovery hygiene + `.svignore` [mod-ignore], resilient
  checking** (user request, found analyzing the repo root): the source
  walk (`SourceSet::add_dir`) now skips hidden directories, cache
  directories carrying a `CACHEDIR.TAG` marker (Cargo writes one into
  `target/` — stale test fixtures under `target/tmp` were leaking into
  root-level analysis and their parse errors gated checking for the
  whole workspace), and entries listed in `<root>/.svignore` (one
  root-relative path per line, file or directory subtree; `#` comments).
  The root itself is exempt from the hidden/cache rules so `--src`
  pointed *at* such a directory still works. Alongside: resolve/check
  now always run, even with parse errors — parse-broken files
  participate with their recovered ASTs (their parsed declarations
  still resolve for other files) but their resolution/checker
  diagnostics are dropped, so one broken file no longer suppresses
  diagnostics elsewhere (and LSP hover keeps working in the rest of
  the workspace).
- **Import suggestions [diag-import-suggest] + `std/random`** (user
  request): diagnostics for unresolved names now carry structured
  import suggestions (`FileDiagnostic::suggested_imports`, populated
  from a whole-program declaration index `Resolution::declared_in`;
  effect members map to their owning effect; `core.*` is never
  suggested — it is implicitly visible). Attached at unknown handler in
  `use`, unknown effect in an effect list, and unresolved imports
  (which suggest the correct module path). Text mode renders
  ``help: add `import …` `` lines, JSON gains an `"imports"` array, and
  the LSP carries suggestions on `Diagnostic.data` and serves
  `textDocument/codeAction` quickfixes inserting the import line after
  the file's last import (the client echoes `data` back in the
  codeAction context, so no re-analysis). Alongside: a `random` std
  module (`effect Random { fn random() -> Float }`, `external handler
  DefaultRandom`) — the first std module outside `core`, so
  `use DefaultRandom` without an import exercises the suggestion
  end-to-end. Its Kotlin define exposed an emitter gap: handler members
  with return types dropped the template's value; they now emit
  `return run { … }` [kt-handler-template-return] (verified by
  compiling and running both backends' output).

### Current architectural facts worth knowing

- **Resolution/checking pipeline**: `emit_program` runs
  `Symbols::collect` (flat, still used for define templates and arity
  fallbacks) → `salvo_core::resolve` (per-file scopes) →
  `salvo_core::check_program`. Type errors are structured
  `FileDiagnostic`s [diag-structured]; they abort emission and are
  rendered at the backend boundary into `BackendError::Codegen` strings
  (the CLI `analyze` command consumes them structured instead).
- The checker is *lenient by design*: anything it cannot type is
  `Ty::Unknown` and emits like before (Kotlin interop pass-through).
  Coercions/unwraps only fire where the tables say so — the emitter's
  syntactic paths remain the fallback everywhere else.
- Wrapper-union arm identity is positional over the **declared** type's
  non-`None` arms; narrowing never re-wraps a variable in place (uses are
  unwrapped/re-wrapped at expression sites instead).
- Modules are emitted to `<module/path>.kt` / `<module/path>.rs`.
  `unions.kt` / `unions.rs` is emitted whenever any wrapper size is used
  by an *emitted* file.
- Only reachable modules that produce code are emitted [mod-used-only]
  (`reach.rs`: name-usage edges over `ModuleScope::name_origins`); each
  module gets its own Kotlin package `salvo.<module.path>` with generated
  imports [kt-package] [kt-imports]; companions copy verbatim
  [backend-companion].
- Deductions (`-> [list: Mut] T`) are inferred/validated by the
  `deduce.rs` post-pass and stored in `Checked::deductions`; the Kotlin
  backend ignores them, the Rust backend derives its parameter modes from
  them (kept = borrow, omitted = move [rs-borrows]).
- The effect environment in both emitters is string-keyed; effect
  resolution prefers the checker's
  `use_effects`/`effect_calls`/`call_effects` tables and falls back to
  string matching only in unchecked contexts (see the M5
  fallback-mechanism note above).
- The two emitters deliberately share their architecture (side-table
  access, `emit_expr` = base + coercion, fallback paths, is-binding and
  loop lowering shape). When a lowering rule changes, check both crates -
  and the checker, which must agree with them on the ident-unwrap
  predicates (`maybe_coerce`'s "effective repr").

## Remaining milestones

### Leftovers (small; no milestone currently claims them)

- `salvo lsp` go-to-definition: `Resolution`/`Symbols` know the declaring
  items but no def-site *spans* are recorded; add ident spans to the
  declaration tables and a `textDocument/definition` handler. Also worth
  considering: incremental analysis if workspaces outgrow
  re-check-everything-per-keystroke, and a `positionEncoding` negotiation
  for UTF-8-native clients.
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
- Caller-side qualifier narrowing from deductions (the LANGUAGE.md
  `remove_first` example: after the call, the local's `NonEmpty` is
  gone and a second `remove_first(list)` should not resolve) is not
  applied yet — deductions are computed and stored, but call sites do
  not consume them for flow narrowing. This is also what would make
  Rust-side moves of locals (`let a = b`, then using `b`) a Salvo
  error instead of a rustc error.
- Deduction inference does not track bare-parameter value flow out of
  branch/loop tails as a move (documented leniency in [deduce-infer]).
- Bare `return` inside a value-position loop (or any value block) in an
  iterator body is not re-targeted to `return@iterator`.
- The Kotlin emitter renders binary expressions flat, without
  re-parenthesizing by precedence: `(a - b) * c` would emit as
  `a - b * c` (latent, unexercised by std/demos; the Rust emitter got
  precedence-aware rendering in M8 - port it back).
- Rust: passing a named fn where a lambda is expected can mismatch
  parameter modes (`impl Fn(S) -> T` args are owned; a named fn's params
  may be borrows) - rustc rejects it loudly, never wrong output.
- Rust: interpolating a still-optional value (a `T?` never narrowed) is a
  rustc error (`Option` has no `Display`); Kotlin prints `null`. Narrow
  or `!` first.
- Rust: `let a = b` moves local `b`; a later Salvo use of `b` is legal in
  the checker today but fails rustc (loud). Caller-side move/narrowing
  enforcement in the checker (the `remove_first` leftover below) is the
  proper fix.
- An aliased import of a *mangled* qualified overload maps to the
  unmangled name in the generated Kotlin alias import ([kt-imports];
  same class as the unchecked-context mangling gap).
- Module reachability is name-based and conservative: a local variable
  shadowing a std fn name still pulls that std module in (harmless
  extra output, never a missing module).

## Test inventory (all green: 124)

- `salvo-core`: 20 - 8 unit tests (file classification; `types.rs` union
  normalization, subtyping, display, wrapper detection) + 2 source
  discovery tests (`tests/source_tests.rs` [mod-ignore]: `.svignore`
  skips listed files/subtrees; hidden and `CACHEDIR.TAG` directories
  skipped with the root exempt) + 8 deduction
  tests (`tests/deduce_tests.rs`: removal-set subtraction, undeclared
  qualifiers passing through calls, move inference, call-graph fixpoint
  transitivity, lenient interop borrows, written-list body validation,
  written-list shape validation, stricter-than-body lists) + 2
  structured-diagnostic tests (`tests/diag_tests.rs`: checker errors
  carry file index/span/severity and render with file:line:col + caret;
  multi-file programs index the declaring file [diag-structured]).
- `salvo-cli`: 18 - 11 `analyze` integration tests running the built
  binary (`tests/analyze_tests.rs` [cli-analyze]: clean program exits 0,
  type errors render with location and exit 1, JSON diagnostics
  (populated + empty array), parse errors reported, a parse error in one
  file not suppressing checker diagnostics in others, `.svignore`
  exclusions [mod-ignore], import suggestions rendered as help lines +
  JSON `imports` for std and user modules [diag-import-suggest],
  `--backend` opting
  define files into the analysis, unknown backend rejected) + 2 UTF-16
  position-mapping unit tests (`src/lsp.rs` [cli-lsp]: multi-byte and
  supplementary-plane round-trips, clamping) + 2 LSP integration tests
  (`tests/lsp_tests.rs` [cli-lsp]: speaks framed JSON-RPC to the binary —
  initialize, didOpen of an unsaved broken buffer -> publishDiagnostics
  with UTF-16 range, didChange fix -> clearing publish, hover -> checked
  type, shutdown/exit -> clean process exit; codeAction import quickfix
  round-trip [diag-import-suggest]) + 3 grammar tests
  (`src/lang.rs` [cli-lang]: highlighting categories exactly partition
  the lexer's keyword table, generated grammar is valid JSON containing
  every keyword, checked-in VS Code grammar matches the generated one).
- `salvo-syntax`: 17 - std + LANGUAGE.md-corpus parse-clean assertions with
  insta AST snapshots (`tests/corpus/*.sv`), error-reporting tests.
- `salvo-backend-kotlin`: 48 - golden snapshots of the M2 demo, the M3
  unions demo, the M4 qualifiers demo, the M5 effects demo, and the M6
  loops demo;
  M7 assertions (only-used-modules + companion copying, per-module
  packages + generated imports, alias imports, effect-param collision
  avoidance, unique destructure temps);
  M8 `Mut` assertions (`Mut List<T>` maps through the `Mut inline:`
  template; `Mut` on a non-`with Mut` type is an error [type-with-mut]);
  wrapper/wrap/`is`-lowering assertions (`unions_emit_sealed_wrappers`),
  predicate/mangling/field-cast assertions
  (`qualifiers_lower_to_predicates_and_mangled_overloads`), checker-driven
  effect-resolution assertions (`effects_resolve_through_checker_tables`),
  loop-lowering assertions (`loops_lower_to_run_blocks`);
  negative tests (non-exhaustive `when`, non-union `when` subject,
  no-matching-arm wrap, missing effect handler at a fn call site and at an
  effect-member call site, `use` without the `use` effect, duplicate effect
  in an effect list, duplicate `use` registration, unknown effect,
  ambiguous generic effect call, handler-member effect deps, duplicate
  qualifier, incompatible qualifiers, `of`-type mismatch, constructor
  same-file rule, predicate-constructor rejection, non-simple constructor
  return, `is` on constructive qualifiers, `qualifies` signature,
  constructive values only from constructors, `break` outside a loop,
  missing defines for used external fns/types, uncovered core externals,
  companion/generated-file collision); and six kotlinc compile+run tests
  with exact stdout assertions (including the M7 multi-module program
  with packages, generated imports, and a companion file).
- `salvo-backend-rust`: 21 - golden snapshots of the same five demos
  emitted as Rust; deduction-mode assertions
  (`deductions_drive_parameter_modes`: kept -> `&`, kept+Mut -> `&mut`,
  omitted -> move, matching call-site argument shapes [rs-borrows]);
  union-enum assertions (`unions_emit_enums`), predicate/mangling
  assertions (`qualifiers_lower_to_predicates_and_mangled_fns`),
  effect-trait assertions (`effects_lower_to_traits_and_mut_dyn_params`),
  loop-lowering assertions (`loops_lower_to_block_expressions`),
  crate-layout assertions (`crate_layout_mounts_only_used_modules`
  [rs-crate] [rs-imports]); negative tests (missing defines for external
  fns/types, uncovered core externals, generic effect members
  [rs-effects]); and six rustc compile+run tests with exact stdout
  assertions mirroring the kotlinc set (demo, unions, qualifiers,
  effects, loops, multi-module).

When intentionally changing std, the parser AST, the checker's lowering, or
the emitter output, rerun with `INSTA_UPDATE=always` and review the
snapshot diffs.

## Gotchas / lessons learned

- `SourceSet::classify` with a backend name that matches no define suffix
  (the CLI passes `""` for backend-neutral `analyze`) loads language
  files only — every `*.<something>.sv` is treated as another backend's
  define file and skipped. Cheap way to get a language-only load; don't
  name a real backend the empty string.
- CLI integration tests use `env!("CARGO_BIN_EXE_salvo")` +
  `env!("CARGO_TARGET_TMPDIR")` (both provided by Cargo for integration
  tests of a crate with a binary) — no `assert_cmd`/`tempfile`
  dependencies needed.
- (lsp) `lsp_server::Connection` must be *dropped before*
  `io_threads.join()`: the writer thread only exits when the
  connection's channel sender is dropped — joining first deadlocks the
  server on shutdown (symptom: clean shutdown/exit exchange, then the
  process never terminates).
- (lsp) `Path::canonicalize` fails for files that don't exist on disk
  (unsaved editor buffers): canonicalize the *parent* and re-append the
  file name, or symlinked roots (macOS `/tmp` -> `/private/tmp`) make
  overlay paths miss the workspace root and the buffer silently drops
  out of the analysis.
- (lsp) Publish diagnostics against the URI the client opened the
  document under, not one rebuilt from the canonicalized path — clients
  match URIs textually, and a `/var` vs `/private/var` rewrite makes
  them ignore the publish.
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
- Kotlin loops are *statements*: any block whose value is a trailing
  `while`/`for` (if-expr branches, `when` branches) must route the loop
  through `emit_loop_value` — a plain emission would silently value the
  block as `Unit`. `emit_value_block` special-cases the trailing loop.
- The loop result local must be a *nullable* temp (`var __loopN: T? =
  null`) even for non-optional joins: Kotlin cannot prove definite
  assignment through a loop, so the block ends `__loopN!!` instead. Only
  loops with `else` produce non-optional joins, and then every path
  assigns — except the spec-silent corner where every iteration
  `continue`s before the tail; the checker closes it by joining `None`
  into the type whenever a bare `break`/`continue` exists.
- `break`/`continue` attribution needs matching stacks on both sides
  (checker `loop_stack`, emitter `loop_results`), and both must treat
  lambda bodies as barriers and *pop before checking/emitting the `else`
  block* (a `break` in `else` belongs to the outer loop).
- Deduction inference must interpret a callee's deduction list relative
  to the callee's *declared* parameter qualifiers (removal set =
  declared − kept, subtracted from the argument), not as the absolute
  set of remaining qualifiers — otherwise qualifiers the callee never
  declared would be wrongly stripped from the caller's argument.
- Deduction fixpoint direction matters: start optimistic (all kept, all
  quals) and only remove facts — starting pessimistic would not converge
  to the least-strict sound answer and recursive fns would infer
  everything as moved.
- Kotlin wildcard imports of *nonexistent* packages are compile errors:
  generated `import salvo.<mod>.*` lines must be filtered to modules that
  are actually emitted (reachable *and* produce code) — hence
  `emitted_modules` is computed before any file is emitted.
- A companion file with the same stem as a code-producing module would
  silently overwrite the generated `.kt` at write time (both map to the
  same path); `emit_program` errors on the collision instead. The
  LANGUAGE.md pattern keeps companion modules externals-only.
- Aliased Salvo imports need Kotlin alias imports only for items that
  exist as Kotlin symbols; alias-importing an *inlined* external (define
  template) would reference a nonexistent symbol and fail kotlinc.
  `emit_fn_call` must emit the source (alias) name when it differs from
  the declaration name, or the alias import is dead and the call
  ambiguous.
- `unions.kt` is now driven by the emitters' tracked sizes only (the
  checker's `union_sizes` may include wraps in unreachable modules);
  every emission path that renders a wrapper inserts its size
  (`emit_ty`, `emit_union_type`, `apply_coercion`, union tests) — keep
  that invariant when adding forms.
- The `--emit-ast` flag takes an optional `=MODULE` value
  (`num_args 0..=1` + `require_equals` in clap); a bare flag must not
  swallow the next positional argument.
- (M8) `matches!(subj, Pat)` and `match subj` on a place do not move it
  when the patterns bind nothing - union tests and `when` lowering rely
  on this to test borrowed subjects without clones.
- (M8) Rust implicitly reborrows `&mut` *place* expressions at call
  sites, and coerces `&mut T` to `&T` - which is why parameter bindings
  thread through calls as bare names while locals need explicit
  `&`/`&mut`. Unsized coercion turns `&mut ConcreteHandler` into
  `&mut dyn Effect` at the argument position for free.
- (M8) The Rust ident-unwrap rule is a superset of Kotlin's: it also
  unwraps `T?`-repr idents narrowed to their value arm (Kotlin smart
  casts those). `maybe_coerce`'s "effective repr" was extended to match
  both - if you touch one of the three places, touch all three.
- (M8) `WrapOption` must be recorded even though Kotlin ignores it:
  the checker cannot know which backend will consume the tables.
  Backends must treat unknown-to-them coercions as no-ops only when the
  representation really is transparent (Kotlin nullability), never by
  default.
- (M8) Everything the Rust emitter renders in a *moved* position must be
  owned; the easy mistake is a field read (`person.name`) - moving out
  of a borrow is illegal, hence the clone-by-default owned rendering.
  The place/owned/raw rendering split (`emit_place`/`emit_owned`/
  `emit_raw`) exists to keep assign targets and `matches!` subjects
  clone-free.
- (M8) Define templates receive raw *places* for non-variadic args so
  method-style templates (`${list}.push(..)`) borrow natively; variadic
  parts splice owned because they land inside constructors
  (`vec![${...elems}]`). Getting this backwards either double-clones or
  moves out of borrows.
- (M8) rustc needs `match` exhaustiveness over the *representation*:
  a `when` over a narrowed subject covers only the logical arms, so the
  emitter appends `_ => unreachable!()` when repr arms are left over
  (the checker proved them impossible).
- (M8) Enum-variant wraps need turbofish (`Union2::<A, B>::U1(x)`): the
  other type parameters are not inferable from one arm's payload.
- (M8) Eager iterators change side-effect *timing* vs Kotlin's lazy
  `Iterable` (documented divergence [rs-iter-vec]); values are equal for
  finite iterators, and infinite ones would hang - revisit if generators
  stabilize.
- (M8) The crate root must carry `#![allow(...)]` *before* any item, and
  `#[path]` mounts resolve relative to the file containing them - the
  root-file header is prepended after all files are emitted, when the
  full mount list (unions, companions) is known.
