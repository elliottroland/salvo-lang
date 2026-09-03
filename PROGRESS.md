# Salvo Compiler — Progress & Plan

Status snapshot as of 2026-09-02: both backends (Kotlin, Rust) work
end-to-end; the post-M8 phase added developer tooling (`salvo analyze`,
the `salvo lsp` language server, a VS Code extension) and a flow-sensitive
ownership analysis (use-after-consume from declared *and* inferred
deductions, uniform across types, branch- and loop-aware). The
shared-fate arc (S1, L2, S2, L3, L4, S3) **and L6 must-use linearity**
are complete: move-mode bindings emit real moves, borrow-mode bindings
and loops emit real borrows, read-only pipelines over kept parameters
are clone-free — and `canbe Linear` types carry a use obligation
(forgetting to close/commit is a compile error, `discard` is the
explicit escape hatch, enforcement is purely static and identical on
both backends). This document is the handoff point for continuing
development: it records what is built, the key design decisions, known
limitations, and the plan for what's next. **The L7 milestone is
complete** — L7a (`<T canbe Linear>`), L7b (`Once` fn types), L7c
(derived returns `ReadOnly[from: p]`), and L7d (fn-type contracts:
named fn-type parameters with standard deduction lists, applied at
fn-value calls, inherited by lambdas, carried by named fns, emitted as
real Rust modes). Small recorded remainders: fn-type *effect* lists
parse but are not yet enforced, `Once` inference, the internal
qualifier unification (nothing forces it), and L5 field precision only
if whole-variable granularity proves too coarse.

**General-leftover sweep (2026-09-02, this session).** Eleven of the
non-ownership leftovers were worked through; four turned out to be
*stale* (the capability was already there, just untested — now pinned
with tests), the rest were implemented. Landed: precedence-aware binary
rendering in the Kotlin emitter; iterator-body `return` retargeting
through value-position lowerings (`StmtCtx` is now emitter state, with
lambda bodies as the barrier); aliased imports of mangled qualified
overloads; the new `[mod-collision]` rule (non-fn name collisions are
errors, not last-win); binding widening in `unify` (with the deliberate
no-occurs-check rationale recorded); type-directed dispatch for
unchecked call sites, ambiguity now a codegen error; the new
`[effect-member-generics]` rule (member generics bind per call; Kotlin
renders them on the interface, Rust rejects them loudly); effect
environments keyed by checker `Ty` on both backends (new
`Checked::fn_effects`); and the new `[lsp-definition]` rule
(go-to-definition via `ModuleScope::def_sites` + `Checked::def_refs`).
Two real bugs fell out of the probes: a stray `eprintln!` debug print in
the checker, and the Rust emitter rendering fn-type `let` annotations as
`impl FnMut(...)` (invalid Rust). Four items were escalated as
**DECISION**s for the user; three came back decided and are now
implemented (std array functions; optionals rejected at operators and in
interpolation), leaving `when` on non-identifier subjects and the wider
operator-typing rules open.

**Next arc: place-based flow analysis (P1).** A new roadmap section
tracks widening flow-state keys from variables to *places*: **P1** is
field smart-casting (`h.field is Str` narrowing reads of `h.field`),
with **P2** `when` on field subjects riding on it, and L5's
field-disjoint ownership sharing the same substrate. Recommended
sequencing is P1 before L5 — P1 has a live customer and is monotone in
acceptance, so the shared substrate gets validated under the lower-risk
feature. Two P1 design points (which projections narrow; whether
narrowing survives a kept-immutable call) are marked **DECISION**.

**Bodyless declarations are explicit (user decision 2026-09-03).** No
inference for anything the compiler cannot see: `external`/`internal` fns
must declare effects, deductions, *and* return type; effect members must
declare return type and deductions; and every `define fn` must match an
external one-to-one, taking its signature from that external
[decl-explicit]. This removed the last inference-from-nothing guess (and
with it the `Mut`-parameter proxy D1 needed), and fixed a real ownership
bug it had been hiding: std's `add` did not consume its element, so
`add(xs, h)` then using `h` compiled on Kotlin and was rejected by rustc.
New roadmap sections: **E1** effect-to-effect dependencies (the effect
declares them, handlers mirror exactly) and **E2** heuristics for
validating external declarations against their define templates.

**Backwards compatibility is not a requirement (user decision
2026-09-03).** The language is experimental and its features are still
being worked out, so compatibility machinery would only get in the way:
syntax and rules change outright, with no deprecation periods, no
grammars that accept both spellings, and no version gates. The
obligation that replaces it is a *sweep*: every affected example gets
rewritten in the same change (`std/`, test corpora, inline `.sv` sources
in Rust tests, the spec documents, README, and the syntax references in
this file — it is a handoff document, not an archive, so stale syntax in
prose is a bug and the change belongs in this decision log instead).
Anything that cannot be rewritten confidently — ambiguous intent under
the new rules, `experiments/` sketches of unimplemented features, or a
site that merely *looks* like the changed construct — is flagged for the
user to update by hand. A transitional *error* naming the replacement is
permitted but not expected (it is a diagnostic, not compatibility);
still accepting the old form never is — and the `canbe` rename's own
transitional error was removed on the user's call, so a plain parse
error is the default. Recorded as an invariant in AGENTS.md with a step
in the task workflow.

**Dot-names landed 2026-09-03 (roadmap N1).** Structs and qualifiers can
be declared `Ns.Name` where `Ns` is a struct in the same file
([name-dot]), giving the Kotlin wrapper-type idiom (`Environment.Id`)
without nested declarations — Kotlin emits a nested class, Rust flattens
to `EnvironmentId`. Casing became a language rule ([name-casing]): types
uppercase, values lowercase, module paths lowercase (so an uppercase
`.sv` file or directory name is a compile-time error). That rule is what
makes `Environment.Id { … }` decidable against `person.name`, and it
retired the parser's old "lowercase after `is` means a binding"
heuristic. Nothing visible may carry the concatenated spelling, checked
scope-wide because Rust flattening and overload mangling share it. No
existing Salvo source violated the casing rule.

**`canbe` replaces `with` at opt-in sites (user decision 2026-09-03).**
Auto-qualifiers on struct and type declarations and the per-type-parameter
linear opt-in are now spelled `canbe` (`struct Person canbe Mut`,
`external type List<T> canbe Mut`, `struct FileHandle canbe Linear`,
`fn hold<T canbe Linear>`), which states the optionality the clause
actually carries [canbe-optin]. `with` keeps exactly one job: qualifier
*compatibility* (`qualifier Old of Person with Surname` [qual-with]) —
co-application of two qualifiers, which `canbe` would misdescribe; no
better word was found, so the overload is gone but `with` stays. The two
clauses are simply independent syntax now — a transitional rename
diagnostic (and the test pinning it) was implemented and then removed on
the user's call, since nothing outside this repository writes Salvo yet.
Labels renamed with the
syntax: `[type-with-mut]` → `[type-canbe-mut]`, `[linear-with]` →
`[linear-canbe]`; AST field `FnDecl.generic_with` → `generic_canbe`
(uniform snapshot churn). The same analysis produced roadmap **D4**:
`is` on a union subject always means arm identity, so a *predicate*
qualifier can never be tested against a union-typed value — fixing that
needs qualifiers over unions, and `is` itself stays as-is (user decision
2026-09-03).

**D1 landed 2026-09-02: deductions are exhaustive by default.** A
confirmed unsoundness — qualifiers surviving calls that invalidate them,
reproduced with a `clear` that emptied a list while the caller kept
believing `NonEmpty` — is fixed. `[list: NonEmpty Mut]` now means *only*
those apply afterwards (including dropping qualifiers the callee never
declared), `[list: -Q]` is the delta form for "everything else
preserved", `[list: Nothing]` is moved, and a parameter the body mutates
may use neither keep-all nor a delta. D2 holds `+Q` asserts and their
possible unity with constructive qualifiers; D3 covers refinements — the
general answer to D1's accepted over-strictness, after analysis showed
blanket qualifier *polymorphism* cannot be sound.

Companion documents: LANGUAGE.md is the narrative spec (source of truth);
LANGUAGE_SPEC.md states every feature as a labeled rule (`[qual-erasure]`
style) with the compiler decisions under it; BACKEND_SPEC.<backend>.md
(`BACKEND_SPEC.kotlin.md`, `BACKEND_SPEC.rust.md`) repeats rules with
backend interpretation details and adds backend-prefixed rules (`kt-…`,
`rs-…`) — load it only when working on that backend. Labels are referenced
from compiler code and tests (`grep -rn '\[rule-name\]'`); backend-prefixed
labels may only be referenced from that backend's crate. Keep all of these
in sync when adding or changing features. Detailed feature mechanics live
in those specs; this file keeps the decision log, the plan, and the
hard-won operational knowledge.

## How to build and test

```bash
cargo build                 # workspace build, no warnings
cargo test                  # 244 tests; includes twenty-three kotlinc and nineteen rustc
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
├── salvo-cli/            # binary "salvo": clap CLI, backend registry, embeds std/ via include_dir,
│                         #   analysis pipeline (analysis.rs), LSP server (lsp.rs), tm-grammar (lang.rs)
├── salvo-syntax/         # lexer, parser, AST, spans, diagnostics (no deps)
│   └── tests/corpus/     # LANGUAGE.md-example .sv files + insta snapshots
├── salvo-core/           # SourceSet, Program, Symbols + resolve.rs/types.rs/check.rs/deduce.rs/reach.rs
├── salvo-backend/        # Backend trait, BackendRegistry, BackendError
├── salvo-backend-kotlin/ # Kotlin emitter (emit.rs) + golden/kotlinc tests
└── salvo-backend-rust/   # Rust emitter (emit.rs) + golden/rustc tests
std/                      # stdlib: core/ (basic, string, list, console) + random.sv
                          #   (+ .kotlin.sv/.rust.sv defines next to each module)
vscode/                   # VS Code extension: LSP client + generated TextMate grammar
```

Adding another backend = new crate implementing `salvo_backend::Backend`,
register it in `salvo-cli/src/main.rs`, write `*.<name>.sv` define files
next to the std modules, and add a `BACKEND_SPEC.<name>.md`. Std embedding
already filters define files per backend at load time
(`SourceSet::classify`).

## History (condensed)

The milestone-by-milestone detail that used to live here has been folded
into the spec documents; what follows is the decision log — the choices
that still shape the code, and where to look for the mechanics.

- **M0+M1 — CLI + parser.** Hand-written lexer + recursive-descent parser
  (deliberate: newline-terminated statements, template/interpolation
  lexer modes, struct-literal-vs-block ambiguity, and speculative parses
  make grammar generators a poor fit). Tokens carry `newline_before`;
  parser recovers at item/statement level. String interpolation re-lexes
  `${...}` fragments with spans shifted back into the file.
- **M2 — Kotlin codegen, end-to-end verified** (kotlinc compiles the
  output; tests assert exact stdout). Founding invariant
  [backend-never-wrong]: unsupported constructs are codegen *errors*,
  never silently wrong code. Remaining deliberate cuts: multi-spread
  struct literals, early `return` inside lambdas, tuples beyond
  Pair/Triple.
- **M3 — Typechecker + unions.** `Ty` model (`Qualified` with sorted qual
  sets, `Union` flattened/deduped in declaration order), per-file scopes,
  and the side-table architecture (`Checked`: `expr_ty`, `repr_ty`,
  `coerce`, `is_tests`, `call_fn`, …) the emitters consult. Two founding
  decisions: the checker is *lenient* (anything untypable is
  `Ty::Unknown` and passes through — Kotlin interop), and union arm
  identity is *positional over the declared type's non-`None` arms*
  [union-arm-identity]. Kotlin unions lower to generated sealed wrappers
  (`UnionN`), `None` arms to outer nullability.
- **M4 — Qualifiers.** User decision replacing the old spec: the `as`
  effect and value-level `as` expressions were removed; constructive
  qualifiers are built exclusively through constructor functions
  (`fn ok<T>(value: T) -> T as Ok`), same-file rule, simple return types
  [qual-ctor-fn] [qual-ctor-same-file] [qual-ctor-simple]. Predicate
  qualifiers lower to `{Q}_qualifies` calls at `is` sites; struct-field
  overrides cast+assert. Overloads identical after erasure get
  deterministic `__Qual` name mangling. Qualified union groups
  (`Ok (A | B)`) wrap whole-arm first, then coerce as the bare union.
- **M5 — Effects.** The checker owns effect semantics: per-fn effect
  environments seeded from declared lists, grown by `use`, truncated at
  block boundaries; handler generics *inferred by unification* from `use`
  constructor args; effect-member disambiguation via explicit type args →
  argument types → expected type. The emitters prefer the checker's
  effect tables and fall back to string-keyed environments only in
  unchecked contexts (see "Current architectural facts").
- **M6 — Deductions + loops-as-values.** Loops are expressions
  [while-value] (body tail / `break value` / `else` join; no `else` means
  optional). Deductions (`deduce.rs`): user decision — a deduction list
  is interpreted *relative to the callee's declared parameter qualifiers*
  (a call removes exactly `declared − kept`); inference is a
  whole-program fixpoint from an optimistic start (facts only removed →
  terminates); written lists may be stricter than the body but never
  looser [deduce-syntax] [deduce-infer].
- **M7 — Polish.** Only reachable modules are emitted [mod-used-only]
  (name-usage reachability, deliberately conservative); per-module Kotlin
  packages + generated imports [kt-package] [kt-imports]; define coverage
  checked upfront for `core.*` and at reference sites
  [backend-external]; backend-native companion files copy verbatim
  [backend-companion]. Decision under [type-array]: `T[]` stays
  `Array<T>` in Kotlin (no primitive-array specialization without
  profiling data).
- **M8 — Rust backend.** The point of the design: deductions drive
  ownership [rs-borrows] — omitted parameter = moved (by value), kept =
  borrowed (`&mut` when `Mut`); expressions emit owned by default
  (borrowed reads clone); no emitted signature returns a reference, so no
  lifetimes exist anywhere. User decision: `Mut` generalized to a
  language-level qualifier any type opts into with `canbe Mut`
  [type-canbe-mut]; backends map it per type (`Mut inline:` define
  sections). Unions are generated enums; effects are traits with
  `&mut dyn` threading; iterators are *eager* (`Iter<T>` = `Vec<T>`,
  documented divergence [rs-iter-vec]); `WrapOption` coercion added
  because optionals are physical in Rust and transparent in Kotlin
  [type-nullable]. Crate layout: main-declaring module is the crate root
  with `#[path]` mounts [rs-crate].

### Post-M8 — tooling and flow analysis (decision log)

- **Optionals never reach operators or interpolation (user decisions
  2026-09-02)** — `[op-no-none]`, `[interp-no-none]`. Both were parity
  holes, not just strictness gaps: Kotlin compares against `null` and
  prints `null`, Rust rejects the `Option` (`Display` unimplemented, type
  mismatch on comparison). The checker now errors on a possibly-`None`
  operand of `+ - * / %` and `< > <= >= == !=`, and on interpolating a
  possibly-`None` value; `None` itself is rejected in both positions.
  Remedy is narrowing (`is` / `when`, taking the binding for a
  non-variable place) or `!`. Deliberately *not* covered: `&&`/`||`,
  because operand typing beyond `None` (numeric towers, promotion, `Bool`
  requirements) is a separate open decision, and value-position `&&` goes
  through a different path than `analyze_cond`. Two pieces of evidence
  that the leniency was costing us: the loops demo's Kotlin and Rust
  sources had silently diverged (`${capped}` vs `${capped!}`) because
  only Rust complained, and three LANGUAGE.md nullability examples
  interpolated `${person.surname}` after `person.surname is Str`,
  relying on field narrowing Salvo does not do — all now use the
  spec's own `is Str surname` binding idiom.
- **Arrays get a std function surface (user decision 2026-09-02)** —
  `[type-array]`. Arrays already had literals, indexing, and native
  `for` iteration on both backends; only functions were missing, which
  is why LANGUAGE.md's `CyclicRandom` example (`values.size()` on a
  `T[]`) did not compile. New `core.array` module mirrors `core.list`
  minus construction (literals are the constructor) and mutation
  (fixed size): `size`, `get`, `first`, `iter`, with the same
  `<T canbe Linear>` opt-in pattern (measuring/iterating a linear array
  is fine; taking an element out is not). The example now compiles and
  runs verbatim on both backends.

- **Structured diagnostics [diag-structured] + `salvo analyze`
  [cli-analyze]**: errors are `FileDiagnostic` (file index, span,
  severity, message) rendered only at the consuming boundary; `analyze`
  runs the front half of the pipeline with text or JSON output. Analysis
  is *backend-neutral* (`--backend` only opts define files into parsing).
  Parse-broken files participate with recovered ASTs but contribute only
  their parse diagnostics — one broken file never suppresses diagnostics
  elsewhere.
- **`salvo lsp` [cli-lsp]**: LSP over stdio (`lsp-server`/`lsp-types`,
  sync); no incremental state — every document event re-runs
  whole-workspace analysis with open buffers as an overlay. Diagnostics
  (with clearing publishes), expression-type hover, fn-signature hover
  with effective (inferred) deductions [fn-ref-table], and import-fix
  code actions [diag-import-suggest].
- **VS Code extension + `salvo lang tm-grammar` [cli-lang]**: `vscode/`
  bundles a grammar *generated by the compiler* from the lexer's keyword
  table (tests fail if the checked-in grammar or keyword categories
  drift); `salvo.serverPath` points at a locally built binary.
- **Source discovery [mod-ignore]**: the walk skips hidden directories,
  `CACHEDIR.TAG` directories (Cargo's `target/`), and `.svignore`
  entries; the root itself is exempt.
- **Import suggestions [diag-import-suggest]**: unresolved
  handler/effect/import diagnostics carry `module.Item` suggestions from
  a whole-program declaration index; rendered as `help:` lines, JSON
  `imports`, and LSP quickfixes. std gained `random`
  (`DefaultRandom`), the first non-`core` module — and exposed
  [kt-handler-template-return]: value-returning handler-member templates
  emit `return run { … }`.
- **Numeric literal suffixes [lit-numeric]**: `1` Int, `1L` Long, `1.2`
  Double, `1.2f` Float; decision: `f` requires a decimal point (`1f` is
  a lex error). Kotlin renders native suffixes; Rust renders explicit
  types (`1i64`, `1.2f32`).
- **Missing-return [fn-must-return]**: non-`None` fns must return on
  every path (syntactic; loops never count; yield-fns exempt).
- **Predicate-qualifier constructors [qual-ctor-predicate]**: the
  constructive-only restriction was lifted; a predicate constructor
  asserts its predicate by construction.
- **Deduction entry forms [deduce-syntax]**: bare `[list]` keeps *all*
  declared qualifiers (semantics change from "keep none"),
  `[list: Mut]` keeps exactly the listed, `[list:]` strips all
  (`Deduction.explicit` flag).
- **Use-after-consume [deduce-consume]** — the largest post-M8 feature,
  built up across several user decisions:
  - Consumed values narrow to `Ty::Nothing` (decision: `Nothing` *is*
    the marker — "a value that no longer exists is an impossibility");
    referencing one is an error; assignment revives.
  - `check_program` runs *two rounds* so inferred deductions are
    enforced at call sites exactly like declared ones (round one checks
    + infers; round two re-checks with the inferred facts injected,
    then re-infers). Round one's diagnostics are discarded (checking is
    deterministic). Known non-convergence: round two's narrowing can
    change overload resolution, whose re-inferred deductions are not
    fed back again (no third round); acceptable at current scale.
  - Decision: consumption is *uniform across all types* (a rejected
    alternative exempted backend-copyable scalars; consistency of the
    abstract contract won). Made livable by a **branch-aware** analysis:
    per-branch snapshot/isolate/merge (`snapshot_narrows`/
    `restore_narrows`/`merge_fallthrough` + `block_always_exits`) —
    always-exiting branches contribute nothing, a value consumed on any
    fall-through path stays consumed (maybe-moved, as in Rust),
    disagreeing states keep only common qualifiers.
  - Kept parameters shed their removal set (declared − kept) from the
    argument's narrowed type, so a second `remove_first` after
    `[list: Mut]` stripped `NonEmpty` fails overload resolution.
  - Consumption survives `is`-narrowing restores (`Nothing` is skipped
    on restore), and loop bodies are re-checked once with their exit
    state as entry when the first pass changed anything
    (`check_loop_body`) — back-edge use-after-move surfaces like
    rustc's "moved in previous iteration" (second-pass duplicates
    deduplicated by file/span/message; value results discarded).
- **Union-arm arguments [type-union]**: fixed `unify`'s match-arm order
  so `describe(ok("x"))` resolves against `Ok Str | Err Str` (see
  Gotchas).

### S1 — shared fate, strict checker-only (completed 2026-09-01)

The first stage of the shared-fate roadmap (see the L1 section below for
the decided model). What landed:

- **`internal fn copy<T>(value: T) -> [value] T`** in `std/core/basic.sv`
  [internal-fn] [copy-fn]: parser already accepted `internal fn`
  (body-less like `external`); the declaration flows through
  Symbols/resolve/checker unchanged — the `[value]` deduction is the
  whole checker contract. Both emitters intercept
  `backing == Internal` before define-template lookup and lower the
  call from the checker's resolved argument type: Kotlin [kt-copy]
  identity for transitively immutable types, `.toMutableList()` /
  `.copy()` / `.copyOf()` for `Mut List` / `Mut` structs / arrays,
  codegen error for nested mutability and unknown/generic types; Rust
  [rs-copy] `.clone()` on the argument's place.
- **Fate links in the checker** [fate-link]: `LocalVar` gained
  `id`/`links`/`poison`; links are directed, flattened to roots at the
  binding, whole-variable. Creators: `let`/assignment from a bare
  identifier or projection chain, destructuring, `for` bindings,
  `is`/`when` bindings. Snapshot/restore/merge and the loop re-check
  carry the full `VarState`; links union across branch merges.
- **Poison rules** [fate-poison]: a `Mut`-kept call argument, projection
  assignment, or `++` on a root — and moves and whole-variable
  reassignment of it — poison its derived variables (`Nothing` + a
  recorded reason; the use-site error names the root, the event, and
  the `copy` remedy). Derived variables are read-only
  [fate-derived-readonly]: moving (consuming call, `return`, `break
  value`, `yield`) or mutating one errors at the site.
- **deduce.rs**: `let`/assignment of a bare parameter is no longer
  inferred as a move — it links; sound because every escape of the
  derived variable is a checker error until `copy` intervenes.
- **[struct-mut] is now enforced** at field-assignment sites (it was
  spec'd but unchecked, and became load-bearing: Kotlin's identity-copy
  is only correct if non-`Mut` values really are immutable). Arrays
  stay index-assignable without `Mut` (status quo; `copy` does a real
  array copy). Fixed a LANGUAGE.md spec bug the enforcement exposed:
  the `Mut` example mutated `person` instead of `mutable_person`.
- **Rust emission**: fate-linked bindings clone — `let`/assignment
  values and `for` iterables that are bare identifiers of owned
  non-Copy locals emit `.clone()` instead of moving (the checker keeps
  both sides readable). This also closed the old "`let a = b` moves
  local `b`" rustc-rejection leftover. Kotlin emission unchanged
  (aliasing is unobservable because mutation-after-link is rejected).
- Deliberately not tracked yet (later stages): non-identifier call
  arguments in *moved* positions (physically a clone today;
  kept-`Mut` positions *are* tracked since the 2026-09-02 parity fix —
  they mutate their provenance roots), lambda captures, and S2's
  move-mode relaxation. (Literal stores, spread, and `use`
  handler-constructor arguments landed in L2 — see the next section.)

### L2 — remaining consuming sites (completed 2026-09-02)

Every remaining move event now feeds the same consumption lattice as
call-site moves (mirroring the [deduce-infer] move list): storing a bare
identifier in a struct/array/tuple literal, spread `...n` (struct-literal
spreads and any `Expr::Spread`), `return n`, `break n`, `yield n`, and
`use Handler(n)` constructor arguments. One helper (`fate_move`) handles
all of them: a derived variable errors at the site
[fate-derived-readonly], a root is consumed (`Nothing`) and poisons its
derived variables [fate-poison], exactly like a call. What's worth
knowing:

- **Loop exits merge break-path states.** `LoopCtx` captures a
  `NarrowSnapshot` at every `break`; the `while`/`for` checking merges
  them with the fall-through exit state via `merge_fallthrough`. Without
  this, a `break s` inside an `if` was invisible after the loop (the
  always-exiting branch contributes nothing to the merge *inside* the
  body — correct there, but the loop exit is precisely where break-path
  state lands). Merge order puts the current (fall-through) snapshot
  first so the shorter frame stack drives the merge (break snapshots
  carry extra inner frames; for `for` loops the merge runs after the
  binding frame is popped).
- **Diagnostics name the event**: `LocalVar`/`VarState` carry
  `consumed_by: Option<&'static str>` ("a literal store", "a `...`
  spread", "a `break`", "a `yield`", "a `use` handler registration",
  "an earlier call"), threaded through snapshot/restore/merge like
  poison, cleared by reassignment revival.
- **`yield` + back edge works for free**: the existing loop re-check
  reports the second-iteration use at the `yield` itself.
- **L2a decided (user, 2026-09-02)**: string interpolation is a *read*
  — `"${n}"` never consumes. Spec'd under [type-str], cross-referenced
  from [deduce-consume]. Parity-sound because both emitters render the
  interpolated value as an owned copy purely for formatting.
- **Parity audit** (per the backend-parity principle): all new sites
  render through `emit_expr` = owned rendering in the Rust backend, and
  an owned non-Copy local emits as a bare place — a *physical move* —
  so bare-ident consumption is faithful-emission parity and closed real
  rustc-rejection gaps (`let t = (s, 1)` then `read(s)` was
  checker-clean but rustc-rejected before L2). Struct-literal spread was
  a latent *mutable-data* parity hole: Kotlin emits shallow `.copy()`
  (aliases `Mut` fields) while Rust deep-clones (`..base.clone()`) —
  `let p2 = Person {...p}; add(p.tags, 2)` would print different values
  per backend. Closed by restriction: the spread consumes `p`. The one
  finding left open — *projection* values in moved positions (literal
  stores `Box {item: h.tags}`, moved-position call arguments), where
  Rust cloned and Kotlin aliased, observable for mutable data and *not*
  caught by rustc — was accepted as a known live divergence (user
  decision 2026-09-02) and closed by S2 the same day [fate-move-mode].

### S2 — move-mode bindings (completed 2026-09-02)

The relaxation stage of shared fate: bindings have *modes* inferred
from downstream flow. What landed (rule [fate-move-mode]):

- **Mode inference rides the two-round architecture.** Round one is
  strict S1; `error_derived` — the single choke point for every derived
  move/mutation — records the *whole bind chain* as move-mode
  candidates (links now keep their original bind spans when flattened,
  so a variable's links describe the full derivation chain even after
  intermediate variables die) plus parameter *claims*. Round two
  applies the modes at every bind event (`declare_var` and assignment
  re-links): all live ancestors owned → consume them at the binding
  (poison names the binding, `FateEvent::BoundAway`), the binding
  carries no links, and the event is recorded in
  `Checked::binding_modes`.
- **Claims make the flagship work.** A move-mode binding reaching a
  parameter of an *inferable* fn claims it as moved;
  `deduce::infer(program, checked, claims)` seeds claims after every
  body inference (monotone). Written-kept parameters block claims: the
  binding stays borrow-mode and the S1 error stands at the move site.
- **Moved-position projections closed the accepted parity divergence**
  (the S2 obligation): a projection of *transitively mutable* data
  (`ty_transitively_mut`, following struct fields with a visited set)
  in a moved position consumes its owned roots
  (`Checked::moved_projections`; for-binding roots flip the loop to
  by-value) or errors for a written-kept parameter root ("cannot move
  mutable data out of `h`", remedy `copy`). The 2026-09-02 probe
  (`wrap(h.tags)` then `add(h.tags, 9)`) is now *rejected* — verified.
  Immutable projections stay untracked by design (unobservable).
- **Rust emission is faithful**: move-mode bind events emit raw places
  (real moves, partial for projections), move-mode loops iterate by
  value, tracked projections render without clones. The flagship
  pipeline (`longest_name` with inferred deductions) emits **zero
  clones**, rustc-compiles, and prints identically on both backends
  (verified end to end). Kotlin emission unchanged. `is`/`when`
  move-mode bindings still clone on Rust (restriction-valid).
- **Pre-existing false positive fixed**: a `for`-loop binding consumed
  in the body (`for s in xs { consume(s) }`) errored on the back-edge
  re-check — bindings now go through `check_loop_body`'s per-pass
  bindings channel (`pattern_bindings`), so each pass re-declares them
  fresh (an iteration binds a new element).

### L3 + L4 — convergence, same-call ordering, lambda captures (completed 2026-09-02)

- **L3 same-call ordering [deduce-same-call]:** within one call, a later
  argument may not mention a value an earlier argument consumed
  (`f(a, a)`, `f(a, size(a))`) — argument typing precedes contract
  enforcement, so the contract loop now tracks what this call consumed
  (`consumed_here`, fed by bare-ident moves and `projection_move`'s
  returned root names) and mention-checks every argument
  (`expr_mentions`, a full expression/block walk). Nested calls were
  already ordered (consumption applies during argument typing).
- **L3a decided (user, 2026-09-02): iterate to a capped fixpoint
  [deduce-fixpoint]** — option (iii): extra checking rounds run only
  when the driving facts changed (inferred deductions, move-mode
  candidates, claims), capped at four; stable programs stay at two
  rounds and identical cost; a program unstable at the cap gets a
  deterministic error naming the oscillating fns with the
  write-the-list remedy. Candidates/claims grow monotonically, so late
  discoveries converge — a move-mode candidate first seen under
  round-two narrowing is now *applied* in round three (the diagnostic
  moves from the raw derived-move error to the true site). The cap
  error is direct code but untested: constructing a genuine overload
  oscillator is an open exercise.
- **L4a decided (user, 2026-09-02): lambdas are ordinary values under
  shared fate [fate-lambda]** — superseding the recorded
  captures-copy recommendation after the emitter audit (plain borrow
  closures on Rust; Kotlin aliases; mutate-after-capture was a rustc
  E0502, not a silent divergence). Per-capture classification from the
  body, bound at creation: immutable reads free; mutable reads link
  the closure to the variable (root mutation poisons it — the E0502
  class becomes a Salvo diagnostic); mutated captures consumed at
  creation (kept-param → error, inferable param → claim); consuming a
  capture is always an error (multiplicity untracked). **No emitter
  changes**: borrow-captures alias on both backends, so parity is
  direct, and checker-legal programs pass NLL (verified end to end —
  identical stdout). Implementation rides the existing event
  machinery: a `lambda_ctx` boundary stack, capture recording in the
  Ident read arm, mutation marking in `fate_mutation`, consumption
  guards at the four consuming sites, and `finish_lambda_captures`
  applying the contract; lambda values get links via `links_for_value`
  and `Checked::lambda_captures` is exported for tooling/emitters.
  Discovered en route: PROGRESS previously overstated "captures are
  completely untracked" — body *consumption* already applied inline at
  creation; the new guard turns that into the multiplicity error.
- Known loud leftover [fate-lambda]: returning/storing a
  capture-carrying closure is a rustc lifetime error the checker does
  not reject; the recorded refinement is `move`-closure emission with
  hoisted clones, pending a treatment for captured effect-handler
  locals. Fn-type contracts (deductions/effects on `Ty::Fn`, the
  named-fn mode mismatch, closure double-use) remain deferred to the
  L7 parameterized-qualifier work.

### S3 — borrow emission (completed 2026-09-02)

The final shared-fate stage: borrow-mode bindings become real Rust
borrows [rs-borrow-locals]. What landed:

- **`&T` locals via `BindKind::Ref` reuse**: a borrow-mode `let` from a
  *pure place* (bare ident / field / index chain; no coercion,
  narrowing unwrap, or field cast; name never reassigned; bind event
  not move-mode) emits `let mut n = &person.name;` and registers as a
  reference binding — the entire existing kept-parameter rendering
  (owned reads clone, borrow positions pass bare, Copy derefs) then
  applies unchanged. Already-`&` roots pass the reference through;
  `&mut` roots reborrow (`&*x`).
- **By-reference loops**: a borrow-mode `for` over a pure-place
  iterable with a plain ident binding iterates without cloning the
  collection (`for person in persons` where `persons: &Vec<Person>`),
  the loop variable itself a reference binding. Guarded to concrete
  non-union element types — union/optional elements go through
  `matches!`/unwrap lowering that expects owned subjects and keep the
  clone path.
- **Decision S3a resolved as emission, not semantics**: a mixed join
  (linked on one path, independent on another) simply keeps today's
  owned/clone emission — since S1, links union across branches and
  poison covers every observation, so the clone is restriction-sound;
  forbidding would have added errors with no parity need, and `Cow`
  buys nothing. No program's legality changed anywhere in S3.
- **Borrowck alignment**: checker-legal programs pass NLL because
  poison forbids using a derived value after its root is mutated,
  moved, or reassigned — so every borrow's last use precedes the
  conflicting event. Known loud exception (documented, rare shape): a
  single call that passes a borrow-emitted local *and* moves its root
  (rustc E0505; the checker's left-to-right argument model accepts
  it). Loud, never wrong.
- **Exit criterion verified**: the moved-position parity probe is
  still rejected after the emission change, and the read-only pipeline
  demo compiles and prints identically on both backends with *zero*
  clones in the Rust output (`count_long`: borrowed param, borrowed
  loop, borrowed field binding).

### L6 — must-use linearity (completed 2026-09-02)

All five decisions (L6a–e) approved by the user as recommended; rules
[linear-canbe] [linear-obligation] [linear-discard] [linear-composite]
[linear-generics] [linear-lambda] [linear-static]. What landed:

- **`canbe Linear`** on type declarations (the auto-qualifier clause
  already parsed arbitrary auto-qualifiers; `has_auto_linear` mirrors
  `has_auto_mut`); `Linear` in a use-site type is an error — linearity
  is declared, not applied. `ty_transitively_linear` /
  `ast_type_linear` mirror the `Mut` transitive analysis (composites
  are contagious).
- **Obligation checks ride the existing flow machinery**:
  `owes_linear` (declared-linear + live + no links + owned-param rule
  via `own_contract`), scanned at frame pops (`check_linear_frame_drop`
  in `check_branch_block`/`check_fn`/`check_lambda`), at
  `return` (all frames) and `break`/`continue` (frames above the
  loop's `entry_depth`, new `LoopCtx` field) via `check_linear_exit`,
  at linear-typed expression statements, and at assignment over a live
  linear value. The all-paths rule lives in `merge_fallthrough` (which
  gained a `span` parameter): consumed on some fall-through paths but
  not all = error — the exact dual of maybe-moved. Reported variables
  are marked consumed (one error per obligation). Gated to round two+
  (`inferred.is_some()`), like the other contract-dependent checks.
- **`internal fn discard<T>(value: T) -> [] None`** in std; the `[]`
  deduction makes the discharge just another move. Rust lowers to
  `drop(value)`, Kotlin to `(value).let {}` [internal-fn]. Both
  verified end to end with identical stdout on the open/use/close
  resource demo.
- **Generic ban** in `resolve_named_call` on the resolved substitution:
  linear instantiation of an unconstrained `T` errors; `copy` refuses
  with its own message; `discard` (internal, by name) is blessed.
  Known leftover: effect members with their own generics are not
  covered by the ban.
- **Lambda guard**: capture-and-mutate of a linear value errors in
  `finish_lambda_captures` (the closure would swallow the obligation);
  read captures are aliases and fine.
- `LocalVar` gained `decl_span`, so obligation diagnostics point at the
  variable's declaration.
- Practical consequence (documented): a Salvo-bodied consumer
  (`fn close_file(h: FileHandle) -> []`) must itself end the chain with
  `discard(h)` — real resource release lives in external fns, which
  have no body to check. `List<FileHandle>` is expressible but not
  constructible until a generic opt-in exists (L7).

### L7a — generic linear opt-in `<T canbe Linear>` (completed 2026-09-02)

Syntax decision (user, 2026-09-02, option C of the explored set): the
opt-in reuses the `canbe Linear` phrase on *type parameters* — one
qualifier per `canbe`, comma separates parameters; struct-side syntax
deferred. (Both sites were spelled `with` until the 2026-09-03 rename
[canbe-optin].) What landed (rule [linear-generics] rewritten):

- **Parser**: `parse_generics_canbe` parses `<T canbe Q, U>` into
  `FnDecl.generic_canbe: Vec<(Ident, TypeRef)>` (fn declarations and
  define signatures); non-fn declarations report "`canbe` on a type
  parameter is only supported on functions". Uniform snapshot churn
  (new FnDecl field) accepted.
- **Checker**: `own_linear_generics` set per fn; `Ty::Var(name)` joined
  the transitive linearity analysis — so opted bodies are checked with
  `T` linear (a written-moved parameter the body drops is a leak), and
  forwarding an opted `T` to an unopted generic fails the ban
  compositionally. The ban lift replaced the discard-by-name blessing;
  `copy` keeps its dedicated refusal. Only `Linear` is accepted in the
  clause.
- **Variadic guard**: linear values are refused in variadic positions
  (untracked — the value would be physically moved but statically still
  owed); the audit found this while opting in `list`/`mutable_list`,
  whose variadic constructors would otherwise have leaked obligations.
  Empty construction + `add` is the supported pattern.
- **std audit**: `add`, `list`, `mutable_list`, `size` opted in;
  `discard` re-declared as
  `internal fn discard<T canbe Linear>(value: T) -> [] None`; `get`
  deliberately *not* opted (returns an alias of an element — a clone of
  a linear value would duplicate the obligation); `copy` refused.
- Verified end to end: the `List<FileHandle>` workflow (construct
  empty, `add` individually, `size`, `discard`) compiles and prints
  identically on both backends.

### L7b — `Once` fn types (completed 2026-09-02)

The call-multiplicity qualifier, landed as a *fn-type qualifier* rather
than the originally-sketched deduction-list surface (user decision
2026-09-02 — calling once is consuming, which contradicts a deduction
entry's kept-ness; the type-qualifier form rides the existing
machinery). Rule [once-fn]:

- **Enforcement is consumption**: calling a `Once` value consumes it —
  double calls, loop back-edge calls, and call-after-escape are the
  ordinary consumed-use errors; maybe-calls are conservative; zero
  calls fine. One fix en route: the Ident-callable path in `check_call`
  bypassed the standard consumed-read error (it never `check_expr`s the
  callee ident), so calling an already-consumed callable reported
  nothing — it now routes through the standard error.
- **Inverted subtyping, flagged for future review** (user request):
  plain fn <: `Once` fn, and `Once` may never be dropped — the
  opposite direction of every other qualifier, special-cased in
  `is_subtype` and `unify` with loud comments.
- **Consuming-capture lambdas legalized**: [fate-lambda]'s always-error
  became "legal but `Once`-typed" — the capture is consumed at
  creation, the closure fits only `Once` positions (boundary error
  otherwise), and every *enclosing* lambda is marked too (an outer
  closure re-creating an inner consuming one re-consumes per run).
  Linear captures still refuse (exactly-once closures are future
  work). The escape rule consumes a `Once` value passed as any
  argument (fn-value ownership is otherwise untracked).
- **Emission nearly free**: Rust emits `impl FnOnce(…)` for `Once`
  params (`QualifiedGroup` over `Fn` in `emit_type` + the
  fn-param-Owned mode extended to qualified groups); lambda emission
  unchanged (rustc's capture inference produces `FnOnce` closures
  itself). Kotlin erases `Once` entirely. Verified end to end with
  identical stdout.
- No inference in v1 (written `Once` only); the parser needed nothing —
  `Once () -> None` already parsed as a qualified group over a fn type.

### L7c — derived returns `ReadOnly[from: p]` (completed 2026-09-02)

The relaxation of S1a: zero-copy accessors across fn boundaries. Rule
[readonly-return]; surface decided by the user (square brackets — angle
reads as generics, round collides with qualified groups, square is
where annotations already name parameters). What landed:

- **Parser**: `parse_derived_return` recognizes
  `ReadOnly[from: param]` between the deduction list and the return
  type in both fn parse paths; stored as `FnDecl.derived_return`
  (never enters the type AST — mirroring the checker design, where the
  fact becomes links at the boundary and the result's *type* stays
  plain, so overloading is untouched).
- **Checker**: validation (parameter exists, *kept* — written or
  inferred), return-provenance validation (every returned value's link
  chain terminates at `p`, `None` free, forwarded derived calls
  validate through the same links), and returns in derived fns do not
  consume. Caller side: `Checked::derived_calls` (call span → argument
  index) makes `links_for_value` link results to arguments.
- **Borrowed links close the S2 interaction**: probing found move-mode
  would have "taken ownership" of a *physically borrowed* result
  (`take(h)` after narrowing `first(...)`'s result compiled to moving
  out of a `&`). `FateLink` gained a `borrowed` flag, set through
  derived calls and propagated through derivation chains;
  `apply_binding_mode` refuses move-mode over borrowed links, so the
  S1 error with the `copy` remedy stands.
- **Rust emission**: `&T` / `Option<&T>` returns; lifetime elision for
  a single reference parameter, mechanical `'a` generation onto the
  annotated parameter and return when there are more — the first
  deliberate exception to the no-lifetimes invariant. Return values
  render as borrows (`Some(&place)`, bare for `&` bindings,
  pass-through for forwarded derived calls). Caller-side narrowing
  works without emitter changes (`Option<&T>` is `Copy`; the
  `.clone()` on a `&&T` copies the reference — the
  `suspicious_double_ref_op` lint joined the generated allow list).
  std's `first` dropped its `.cloned()` — clone-free — and gained the
  annotation. Kotlin: zero changes.
- **v1 scope recorded**: plain `T` and `T?` shapes; fn declarations
  only; accumulator bodies (`best = person; …; return best`) rejected
  by the provenance validation — reassignable borrowed locals are the
  recorded refinement.

### L7d — fn-type contracts (completed 2026-09-02)

The user-directed redesign of the original "piece 3": instead of a
fixed all-moved convention, fn types carry *declared* contracts —
default keeps-everything, so nothing broke and the parity hole closed
by faithful emission. Rule [fn-contract]:

- **Surface**: fn-type parameters may be named and a standard deduction
  list may follow the arrow (`(v: List<Int>) -> [] Int`). `Type::Fn`
  gained `param_names`/`deductions`; `Ty::Fn` gained
  `contract: Option<Vec<FnParamContract>>` (a types.rs struct — kept
  out of `Display` to avoid message churn).
- **Checker**: `apply_fn_value_contract` mirrors the named-call
  contract loop (consumption, kept-`Mut` mutation events, qualifier
  shedding, same-call ordering, Once escape, capture/kept guards) at
  fn-value call sites; lambdas checked against a contract mark kept
  parameters `lambda_kept` (never consumable — guarded at all four
  consuming sites plus binding modes); named fns passed by value build
  their contract from written/inferred deductions; `contract_fits`
  (keeps <: consumes, inverted like [once-fn] and flagged with it)
  joined `is_subtype` and `unify`. One enabling fix: single-candidate
  named calls now type *lambda literal* arguments against the callee's
  parameter types upfront, so expected fn-type contracts actually
  reach `check_lambda` (multi-candidate calls keep the untyped probe).
- **deduce.rs**: calls through fn-typed *parameters* of the walking fn
  apply that parameter's written contract, so consuming contracts
  propagate interprocedurally (`caller_loses` sees its argument die).
- **Rust emission**: fn params render `&mut impl FnMut(…)` (closure
  double-use fixed by faithful emission; `FnMut` accepts
  handler-mutating closures; `Once` stays `impl FnOnce`), argument
  types per contract, call-site arguments per
  `Checked::fn_value_calls`, lambda bindings/annotations per
  `Checked::lambda_contracts`, and named fns wrap in mechanical
  adapter closures. **Kotlin**: contracts erase; named fns emit
  `::name` function references (that pass was silently broken on
  Kotlin too — `count` bare emitted a call-less identifier kotlinc
  rejects).
- Deferred, recorded: fn-type *effect* lists (parse, lexical env
  meanwhile), `Once` inference, per-parameter written contracts
  beyond kept/moved/quals.

### Current architectural facts worth knowing

- **`ReadOnly` presentation (landed 2026-09-02)**: reads of fate-linked
  variables record their links into `Checked::fate_reads` (root name +
  bind span per link [fate-link]); the LSP hover renders such a
  variable as `ReadOnly T` with the qualifier's parameters (roots,
  binding sites, `copy` remedy) as detail below the type line —
  progressive disclosure per user decision. Presentation-only:
  `ReadOnly` is not in the type system and cannot be written. It is
  phase 1 of the parameterized-compiler-qualifier design recorded
  under L7.
- **Resolution/checking pipeline**: `emit_program` runs
  `Symbols::collect` (flat, still used for define templates and arity
  fallbacks) → `salvo_core::resolve` (per-file scopes) →
  `salvo_core::check_program` (two rounds + deduction inference, see
  [deduce-consume]). Type errors are structured `FileDiagnostic`s
  [diag-structured]; they abort emission and are rendered at the backend
  boundary into `BackendError::Codegen` strings (the CLI `analyze`
  command consumes them structured instead).
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
  them (kept = borrow, omitted = move [rs-borrows]); the checker enforces
  them flow-sensitively at call sites [deduce-consume].
- **Emitter effect-environment fallback (deliberate, revisit later)**:
  the emitters' effect environments are string-keyed; at each site they
  first consult the checker's `use_effects`/`effect_calls`/`call_effects`
  tables (rendered through `kotlin_ty`, which must agree with `emit_type`
  on the same source type) and fall back to string/base-name matching
  only when the table has no entry or the type contains `Unknown`. The
  fallback keeps the lenient-checker contract: a checker regression
  degrades to string matching rather than wrong code. Cost: double
  bookkeeping. When the emitters key their environments by checker `Ty`
  directly, the string env can be deleted.
- The two emitters deliberately share their architecture (side-table
  access, `emit_expr` = base + coercion, fallback paths, is-binding and
  loop lowering shape). When a lowering rule changes, check both crates —
  and the checker, which must agree with them on the ident-unwrap
  predicates (`maybe_coerce`'s "effective repr").

## Roadmap: toward full linear types

Where we are: an *affine* analysis ("use at most once") with solid
underpinnings: interprocedural contracts (inferred + validated
deductions), `Nothing`-narrowing with revival, and branch-/loop-aware
state merging. Since L2, the local flow analysis watches *every* move
event the deduction inference knows — call-site moves, literal stores,
spread, `return`/`break`/`yield`, `use` constructor arguments — for bare
identifiers; the remaining gaps are projection values in moved positions
(a parity question, see the L2 audit), lambda captures (L4), and the two
genuinely new mechanisms (places, must-use). The safety net holds:
holes surface as rustc errors on the generated code (loud, never
silently wrong); the one known exception — moved-position projections
of mutable data — was closed by S2 (see the backend-parity principle
note).

Each phase below is independently shippable, in rough dependency order.
Items marked **DECISION** need a language-design call before or during
implementation — everything else is analysis engineering under decisions
already made (uniform-across-types consumption, `Nothing` as the marker,
maybe-moved-is-unusable).

### Backend-parity principle (user decision 2026-09-01)

The operational semantics must be the *same* on both backends; divergent
representations are acceptable only where the difference is
unobservable. This principle governs every move/copy/borrow decision in
the stages below:

- **Immutable data**: clone vs shared reference is purely a performance
  question — free to address later, in any direction, at any time.
- **Mutable data**: a clone where Kotlin shares a reference is a
  *semantics* change, never just a cost. Parity must come from one of
  exactly two strategies:
  1. **Restriction** — the checker rejects every program that could
     observe the difference. This is what deductions and shared fate do
     today: S1's clone-emission is sound *because*
     mutate-after-link and mutate-through-derived are compile errors.
  2. **Faithful emission** — the Rust backend works harder, including
     emitting code shaped *differently from the source* when a
     mechanical equivalence justifies it. Recorded technique, **read
     redirection**: after `let name = person.name`, a later read of
     `person.name` with no intervening mutation is guaranteed equal to
     `name`, so Rust may emit the binding as a real (partial) move and
     redirect subsequent reads of the moved path to the surviving
     variable — no clone, no borrow, no lifetime; the fate-link table
     already knows the equivalence. The same idea generalizes to any
     place the checker can prove holds the same data as a live local.
  Silent clones of mutable data are *not* a valid parity strategy.
- **Parity bug closed (verified 2026-09-01, fixed 2026-09-02):** a
  *projection* passed directly to a kept `Mut` parameter is a mutation
  of its provenance roots — before the fix, `let t = h.tags;
  add(h.tags, 2); size(t)` compiled clean and printed 2 on Kotlin
  (alias) but 1 on Rust (clone). The call-site contract loop now runs
  `fate_mutation` on the provenance roots of non-identifier arguments
  in kept-`Mut` positions (mirroring projection assignment), so the
  derived variable is poisoned and the program is rejected; `copy` at
  the binding is the remedy. Audit the remaining untracked events
  (literal stores, `use` ctor args, interpolation, *moved*-position
  projection args) against this principle when L2 lands them.
- **Moved-position projection divergence — CLOSED by S2 (2026-09-02):**
  a *projection* of transitively-`Mut` data in a *moved* position (a
  consuming call argument, a literal store, a `use` ctor argument)
  cloned in Rust but aliased in Kotlin, and rustc did not catch it (the
  clone is valid Rust) — verified live with `wrap(h.tags)` then
  `add(h.tags, 9)`: Kotlin printed 2, Rust printed 1. S2 closed it
  [fate-move-mode]: such projections now consume their owned roots (the
  probe program is *rejected* — re-verified) or error for written-kept
  parameter roots with the `copy` remedy; tracked projections also emit
  as real partial moves in Rust. Projections of immutable data remain
  deliberately untracked (clone-vs-alias unobservable).

### L1 — Shared fate: links, poison, and `copy` (user decisions 2026-09-01)

Redesigned: replaces the original "aliasing bindings are moves" +
"reads are copies" plan. Shared fate is a lifetime-free borrow
discipline: a variable bound to the value or projection of another
*links* to it, reads flow freely through links, and ownership-requiring
operations (moves, `Mut` ops) consume the rest of the link group. The
motivating example: `longest = person.name` inside a loop over a *kept*
parameter `persons`, then `return longest` — invalid (moving a value
derived from a borrow); remedy `return copy(longest)` — one copy at the
escape instead of a clone per iteration.

The decided model:

- **L1a — `internal fn copy` (decided).** New `internal` item keyword
  for compiler-intrinsic fns:
  `internal fn copy<T>(value: T) -> [value] T` is declared in std (the
  signature + `[value]` deduction are all the checker needs:
  non-consuming, result independent), has *no define files*, and each
  emitter lowers calls to it type-directedly — Kotlin: identity for
  immutable data, real copies for `Mut`-capable types; Rust:
  `.clone()` (scalar identity as a later refinement). Unsupported
  types are codegen errors [backend-never-wrong]. Rationale: a define
  template is type-blind text per signature and cannot dispatch on the
  instantiated `T`; an intrinsic sees the checker's resolved argument
  type at each call site.
- **Shared fate (decided; supersedes L1b).** Links are *directed*
  (derived → root), *transitive* (`longest` → `person` → `persons`),
  and at *whole-variable granularity* (no place lattice). Link
  creators: `let`/assignment from a bare identifier or a projection,
  loop bindings, destructuring.
  - Reads never consume and never poison, on any member, any time.
  - Mutation events are already defined by the deduction system:
    Mut-kept call args and projection assignments (effect-handler
    capture deferred to L4).
  - A `Mut` op or move on the *root* poisons every derived member
    (narrows to `Nothing`, error at later *use*, revival by
    reassignment — the existing possibly-consumed machinery). The
    root stays usable after mutation. Poison is not retroactive:
    derivatives created after a mutation are fine. This is NLL-like
    precision without a liveness analysis (no later use → nothing
    fires).
  - **Bindings have modes**, decided by downstream flow:
    *borrow-mode* (derived value only ever read; ancestors stay
    usable) vs *move-mode* (derived value eventually moved/mutated;
    the *binding itself* consumes the ancestors — Rust partial-move
    semantics, emission-faithful, no hidden clones). Poison-at-move
    was rejected: it cannot be emitted faithfully without a hidden
    clone.
  - Moving or mutating a value derived from a *kept* parameter is a
    deduction-contract violation regardless of mode (you cannot move
    out of a borrow); remedy `copy`.
  - Uniform across all types (standing decision). On Kotlin the whole
    discipline is a purely static protocol (no runtime component); it
    rejects some JVM-fine programs by design, remedy `copy`.

Stages (each independently shippable):

- **S1 — strict, checker-only. ✅ Done 2026-09-01** (see the S1 section
  in the decision log above). Links + poison rules with derived
  members *read-only* (any move or `Mut` op on a derived member is an
  error at that site; remedy `copy`). Emission unchanged
  (clone-by-default), so nothing can physically break — the protocol
  lands before the performance. Includes `internal fn copy`
  end-to-end and diagnostics naming the link and the event.
  - **S1a — the function boundary (decided 2026-09-01):** call
    results never link to arguments — returned values are always
    independent; a fn returning a projection of a kept param must
    `copy` internally; links stay intraprocedural. Derived-return
    annotations are reconsidered in a dedicated late milestone (L7).
- **S2 — move-mode bindings. ✅ Done 2026-09-02** (see the S2 section
  in the decision log above; rule [fate-move-mode]). Binding modes
  inferred from downstream flow; move-mode bindings consume their
  owned ancestors at the binding and emit as real moves (zero-clone
  pipelines verified end to end); parameters are claimed into the
  deduction fixpoint; written-kept parameters keep the S1 errors.
  - **Obligation discharged**: the moved-position projection
    divergence is closed — projections of transitively-`Mut` data in
    moved positions consume owned roots / error for written-kept
    parameter roots (`copy` remedy); the parity probe is rejected.
  - Refinement (recorded, optional, still open): *read redirection*
    (see the backend-parity principle) can soften poison-at-binding —
    a read of exactly the moved path (`person.name` after a move-mode
    `let name = person.name`, before any mutation) is provably equal
    to the surviving variable, so the checker may allow it and the
    Rust emitter substitutes `name`. Keeps more source shapes legal
    without clones.
- **S3 — borrow emission (Rust). ✅ Done 2026-09-02** (see the S3
  section in the decision log above; rule [rs-borrow-locals]).
  Borrow-mode bindings from pure places emit `&T` locals; borrow-mode
  loops iterate by reference; Kotlin unchanged; clone fallback
  everywhere else (restriction-sound).
  - **DECISION S3a — resolved 2026-09-02 as emission, not semantics:**
    mixed joins keep the owned/clone emission — link-union + poison
    already reject every observation, so no forbid and no `Cow`; no
    program's legality changed. Read redirection remains a recorded
    future refinement.
  - Exit criterion verified: the parity probe is still rejected after
    the emission change, and the borrow demo prints identically on
    both backends with zero clones in the Rust pipeline.

### L2 — Remaining consuming sites. ✅ Done 2026-09-02

(See the L2 section in the decision log above.) All root-consuming move
events are tracked: literal stores, spread, `return`/`break`/`yield`
(with break-path states merged into loop exits, and the yield/back-edge
interaction handled by the existing two-pass loop analysis), and `use`
constructor arguments. The parity audit found bare-ident consumption is
faithful-emission parity (Rust already moved at these sites) and closed
the struct-spread shallow-copy-vs-deep-clone hole by restriction.
Remaining audit finding: projection values in moved positions cloned
in Rust and aliased in Kotlin — observable for mutable data only, and
*not* caught by rustc (the clone is valid Rust). Accepted as a known
live divergence (user decision 2026-09-02) and closed by S2 the same
day [fate-move-mode].

- **Priority item — done 2026-09-02**: the verified parity bug (see the
  backend-parity principle above) is fixed — non-identifier arguments
  in kept-`Mut` positions run `fate_mutation` on their provenance
  roots.
- **DECISION L2a — interpolation (decided 2026-09-02).** `"${n}"` is a
  *read*: reads never consume. Both emitters render interpolated values
  owned-by-clone purely for formatting (no reference retained), so the
  decision is parity-sound. Spec'd under [type-str] and cross-referenced
  from [deduce-consume].

### L3 — Same-call and convergence tightening. ✅ Done 2026-09-02

(See the L3+L4 section in the decision log above.) Same-call argument
ordering enforced [deduce-same-call]; checking iterates to a capped
fixpoint [deduce-fixpoint].

- **DECISION L3a — decided (user, 2026-09-02):** option (iii) — iterate
  only while facts changed, cap four rounds, deterministic instability
  error with the write-the-list remedy.

### L4 — Lambda captures. ✅ Done 2026-09-02

(See the L3+L4 section in the decision log above; rule [fate-lambda].)

- **DECISION L4a — decided (user, 2026-09-02):** lambdas are ordinary
  values under shared fate — per-capture classification from the body
  (read/mutate/move), contract bound at creation; immutable reads
  free, mutable reads fate-link the closure, mutated captures consumed
  at creation, consuming a capture always an error. The emitter audit
  superseded the recorded (b) recommendation: plain borrow-closures
  already alias on both backends, so no emitter change was needed.
  Option (c) — full capture/deduction contracts on fn types — remains
  the long-term design, folded into the L7 parameterized-qualifier
  work.

### L5 — Places and partial moves

Track paths (`x.field`, tuple/array elements), not just whole variables:
destructuring consumes its source; moving a field out leaves the struct
partially unusable. This is the largest analysis change (place lattice
instead of per-variable states).

- **L5a — answered by shared fate (2026-09-01):** partial moves exist
  as *move-mode bindings* at whole-variable granularity (L1/S2); the
  question left for L5 is only *field-disjoint precision* (using one
  field while another is moved/borrowed), a refinement with no current
  use case. Revisit only if whole-variable poison proves too coarse in
  practice.
- **Not L5: field smart-casting.** Place-based *type narrowing* (reads
  of `h.field` narrowed by `h.field is T`) is a separate feature from
  place-based ownership — it is roadmap phase **P1** (see "Roadmap:
  place-based flow analysis"), which shares L5's `Place` substrate. The
  recommendation there is **P1 before L5**: P1 has a live customer
  (`when` on field subjects) and is monotone in acceptance, so the shared
  substrate gets designed and validated under the lower-risk feature.

### L6 — Must-use: true linearity. ✅ Done 2026-09-02

(See the L6 section in the decision log above; rules [linear-*].) All
five decisions approved by the user as recommended on 2026-09-02:

- **L6a**: `canbe Linear` on the type declaration; `Linear` is not
  writable at use sites (every value of the type is linear, always).
- **L6b**: consumption = any move, as the deduction system defines it;
  moves transfer the obligation (compositional across calls, returns,
  stores, and move-mode bindings).
- **L6c**: `internal fn discard<T>(value: T) -> [] None` is the
  explicit escape hatch; early-exit paths are checked by the existing
  path machinery; panic/abort semantics out of scope until they exist.
- **L6d**: composites containing linear components are linear
  (transitive); unconstrained generics refuse linear instantiation
  (`copy` refused with a dedicated message, `discard` blessed);
  `where T: Linear`-style opt-in deferred to the L7 qualifier work.
- **L6e**: purely static protocol, identical on both backends, no
  runtime component.

### L7 — Reconsider derived-return annotations

Revisit S1a's "returned values are always independent" rule once
shared fate (S1–S3) and linearity (L6) have real usage. The question:
should deductions grow a derived-return dimension ("the result is
derived from parameter X" — lifetime elision by another name), so
zero-copy accessors (`first(persons)` returning a linked element)
work across call boundaries? Adding it later is purely a relaxation
(more programs expressible, nothing breaks). Evaluate against real
std/user code: if the intra-function `copy` costs never show up in
practice, independent returns may be the permanently right answer.

- **DECISION L7a** — whether to add it at all, and the annotation
  surface if so (e.g. `-> [persons] persons.T`-style vs a marker on
  the return type); every std external returning a projection would
  need auditing.
- **Leading design for L7a (user decision 2026-09-02): parameterized
  compiler qualifiers.** Supersedes the `-> [persons] persons.T`
  strawman. Compiler-inserted qualifiers form a distinct class — never
  affecting overload resolution/`unify`/mangling/erasure, not testable
  with `is`, not constructible, strippable only by blessed fns
  (`copy`, later `discard`) — and may carry *parameters* the checker
  uses for checking and diagnostics. `ReadOnly(from: root)` is the
  fate link as a type; `-> ReadOnly(from: param) T` in a signature is
  the derived-return annotation, giving exact root-naming (the caller
  links the result to that argument; Rust lifetime annotations are
  generated mechanically from the parameter — elision already covers
  the single-kept-param case). The same vehicle can later carry
  deduction contracts on `Ty::Fn` values (closing the
  named-fn-as-lambda mode-mismatch leftover) and L4 capture contracts.
  Representational discipline: parameters are var identities in flow
  state and param *names* in signatures, substituted at call
  boundaries (same shape as generic substitution). Adoption is phased
  so each step pays for itself:
  1. **Presentation (✅ done 2026-09-02)**: derived variables hover as
     bare `ReadOnly T`, with the parameters (roots, binding sites,
     `copy` remedy) as on-request detail — progressive disclosure per
     user decision; `Checked::fate_reads` carries the data. No
     type-system change.
  2. **Internal unification (when L7 starts)**: fold
     links/poison/consumed_by into parameterized qualifiers on the
     narrowed type with per-qualifier join direction (`ReadOnly`
     params union across branches; user qualifiers intersect);
     diagnostics gain LSP related-information spans from the
     parameters. Same programs accepted/rejected — done at L7 to avoid
     refactoring twice.
  3. **Signature transport (L7 proper)**: `ReadOnly(from: param)` in
     return position, call-site substitution, mechanical Rust lifetime
     generation; std externals returning projections audited then.
  Open decision points for when L7 starts: the writability boundary
  (readable everywhere; writable only in return position first?),
  per-qualifier join declarations, and whether `Nothing` gains
  parameters (recommended: yes — strictly more informative, revival
  unchanged).
- **L7a delivered 2026-09-02**: the generic linear opt-in
  `<T canbe Linear>` (see the decision-log section; syntax option C —
  `where` clauses and qualifier-prefix forms were explored and
  declined; body-inference recorded as a possible later complement,
  mirroring the deduction precedent: written validates, unwritten
  infers).
- **L7c delivered 2026-09-02**: derived returns `ReadOnly[from: p]`
  (see the decision-log section) — the L7a-recorded bare-`ReadOnly`
  idea matured into the parameterized square-bracket surface; the
  internal qualifier unification was *not* needed (links carry the
  fact across the one boundary it crosses).
- **L7b delivered 2026-09-02**: the `Once` call-multiplicity qualifier
  (see the decision-log section) — landed on *fn types* rather than in
  deduction lists (the surface originally sketched here; user approved
  the change): `fn run(f: Once () -> None)`, enforcement by
  consumption, inverted subtyping (flagged for review),
  consuming-capture lambdas legal and `Once`-typed, Rust `impl FnOnce`.
  Inference (a callee auto-promoting a once-called fn param) remains
  open for the fn-type-contracts sub-phase.

Sequencing note: L1 lands in stages (S1 strict checker-only → S2
move-mode bindings → S3 borrow emission); S1+L2 closed real
rustc-rejection gaps; L3 and L4 are done (2026-09-02);
L5 is largely subsumed by shared fate (field-disjoint precision only);
L6 landed 2026-09-02 with its own LANGUAGE.md section and the
[linear-*] rule family. Per AGENTS.md, each phase lands with
LANGUAGE_SPEC.md rules and tests at every affected layer.

## Roadmap: effects

### E1 — Effect-to-effect dependencies (user decision 2026-09-03)

An effect may depend on another effect: the *effect* declares the
dependency and every handler must mirror the effect exactly. Today
[effect-member-no-effects] forbids members declaring effects at all,
because dispatch goes through the handler instance and the call site has
no way to thread extra handler arguments. Declaring the dependency on the
effect (not the member) fixes that: the set is known from the effect
declaration, so a handler's members can receive the dependencies the same
way ordinary fns do, and call sites thread them like any other effect
list.

- The mirroring principle generalizes: a handler must match its effect
  exactly, just as a `define fn` must match an external exactly (D-defines
  below). Anywhere the compiler cannot see an implementation, the
  declaration is the contract and the implementation is validated against
  it — one-to-one, no inference.
- Sequencing note: effect *member* deduction contracts (declared on the
  member, validated against each handler's body) are the smaller sibling
  of this work and land first — see the bodyless-explicitness rule
  [decl-explicit].

### E2 — Heuristics for validating external functions (user decision 2026-09-03)

An `external fn` is a trust boundary: its declared contract (effects,
deductions, return type, qualifiers) is taken on faith, and a wrong
declaration introduces a hole *accidentally* rather than maliciously.
Worth building cheap checks that catch the common mistakes by inspecting
the backend define template:

- A template that mutates its argument (`.clear()`, `.push(...)`,
  assignment) under a parameter the external declares as kept-immutable,
  or that keeps qualifiers a mutation would invalidate.
- A template that moves an argument (Rust: passing by value into a
  container) while the external's deduction keeps it — the `add`
  ownership bug this arc found, in reverse.
- A template performing I/O (`println`, file APIs) while the external
  declares `[]`.
- Necessarily heuristic and backend-specific (pattern matching on native
  source), so findings should be *warnings* with an opt-out, never hard
  errors — a false positive must not block a legitimate define.

## Roadmap: place-based flow analysis

A second flow-analysis arc, independent of linearity. Today every flow
fact is keyed by *variable* — narrowed type, poison, links, consumed
state all live on a `LocalVar` and are snapshotted/merged by name (see
the S1 gotcha: keep new facts inside `VarState` so
`snapshot_narrows`/`restore_narrows`/`merge_fallthrough` stay the single
source of truth). Two wanted features both need that key widened to a
*place* (`h`, `h.field`, `h.a.b`):

- **P1 — field smart-casting**: place-based *type* narrowing. After
  `h.field is Str`, reads of `h.field` have type `Str`.
- **L5 (field-disjoint precision)** — place-based *ownership*: using one
  field while another is moved or borrowed. Lives in the linear-types
  roadmap above; listed here because it shares P1's substrate.

### P1 — Field smart-casting (place-based narrowing)

Motivation, in order of weight:

1. **`when` on field subjects** becomes natural (user interest
   2026-09-02). Without narrowing, arms cannot read the subject at all
   except through a binding, and `when` bindings only work for
   single-type arms — so `when h.result { … }` would be a poor cousin of
   the variable form. With narrowing it is the same feature.
2. **The optional-strictness rules made the gap user-facing**
   (2026-09-02): `[interp-no-none]` / `[op-no-none]` mean a field check
   must use the `is T name` binding form before the value can be
   interpolated or used as an operand. Three LANGUAGE.md examples had to
   be rewritten for exactly this.
3. It is **monotone in acceptance**: currently-rejected reads become
   legal, no existing program changes meaning. Low blast radius.

Scope: a `Place` (root var id + projection path), prefix relations
(is-prefix-of / overlaps), place-keyed narrowing state through
snapshot/restore/merge, and invalidation on assignment to any prefix,
mutation through a `Mut`-keeping call on any prefix, and root
reassignment — the same event set the fate analysis already watches
(`fate_mutation`, [fate-poison]). Emitters already lower the *tests* for
any place; they need the narrowed *reads* unwrapped (Kotlin smart-casts
`T?` but needs `.value as T` for wrapper unions; Rust needs the `.uN()`
accessor), and checker/emitter must agree as ever.

- **DECISION P1a** — which projections narrow. Field chains are the
  clear case; array/tuple *elements* (`arr[i]`) are not statically
  identifiable in general, so narrowing them is either unsound or
  restricted to constant indices. Recommendation: field chains only at
  first, but define the projection enum with an element variant from the
  start so L5 can reuse it without a rework.
- **DECISION P1b** — whether narrowing survives a *call* that keeps the
  root (`f(h)` with `h` kept but not `Mut`): a kept-immutable parameter
  cannot mutate, so narrowing could survive. Conservative default:
  invalidate on any `Mut`-keeping call, survive pure reads.
- **P2 — `when` on field subjects** rides on P1 and is small once
  narrowing exists (the checker's `when` currently rejects non-ident
  subjects because `narrows` is name-keyed; emitters already handle
  field subjects for `is`). Also a **DECISION** in its own right — it is
  a language-surface change ([when-union-subject]).

### Sequencing with L5 (recommendation)

**P1 before L5.** Both widen the same key, so whichever lands first pays
for the substrate; the argument for P1 going first:

- P1 has a live customer (`when` on fields, plus the strictness
  regression); L5a records that field-disjoint ownership has *no current
  use case*. Designing the shared substrate under the feature that is
  actually wanted validates it with real code instead of speculation.
- P1 is monotone (more programs compile, nothing breaks). L5 changes
  rejection behavior in both directions — more precision accepts some
  programs while a place lattice tracks new events that can surface new
  errors. Smaller blast radius first.
- The integration with fate/poison happens once either way; doing it
  with the smaller feature means less code is at risk when the keying
  changes.

The one risk of P1-first is designing `Place` against the easier
consumer and having to generalize it for ownership (elements, not just
fields) — mitigated by DECISION P1a's recommendation to include the
element variant from the start. Neither phase hard-blocks the other:
they are coupled only through the substrate.

## Roadmap: deductions and qualifier reasoning

Motivated by a confirmed unsoundness (see "Remaining leftovers"): the
removal-set rule (`removal = declared − kept`) assumes a function can only
invalidate qualifiers it *declares*, which is false for any function that
mutates. Fixing it needs a way to say "and nothing else survives", which a
delta-only model cannot express.

### D1 — Exhaustive and delta qualifier deductions. ✅ Done 2026-09-02

Keep the `:` syntax; give the qualifier list a *polarity* per entry:

| Form | Meaning |
|---|---|
| `[list]` | keep the parameter, all qualifiers preserved (unchanged) |
| `[list:]` | keep it, strip every qualifier (unchanged) |
| `[list: NonEmpty Mut]` | **exhaustive**: afterwards *only* these apply |
| `[list: -NonEmpty]` | **delta**: drop `NonEmpty`, everything else preserved |
| `[list: Nothing]` | moved (equivalently: omit the entry) |
| `[]` | no promises about any parameter — everything moved |
| *(absent)* | inferred [deduce-infer] |

`+Q` (add a qualifier) is **not part of the language** — it is an assert,
see D2 (user decision 2026-09-02). The parser recognizes the `+` only to
report "adding qualifiers in a deduction (`+Qual`) is not supported yet"
instead of a bare parse error.

The plain (exhaustive) form is what closes the hole: `clear`'s
`[list: Mut]` now drops a caller's `NonEmpty` because it was not listed.

- **Soundness rule (the load-bearing part).** For a parameter the body
  *mutates*, only the exhaustive form (or `[list:]`) may be written:
  keep-all `[list]` and `-` deltas both claim "everything else survives",
  which is exactly the unsound claim. Inference emits the declared set
  for mutated parameters and keep-all for read-only ones — so pure reads
  still never strip a caller's qualifiers, which is the property the
  original removal-set rule existed to protect.
- **Is mutation the only invalidating operation?** (user question
  2026-09-02) Within the current language, yes — and it follows from the
  model rather than being a stipulation. Everything a callee can do to a
  *kept* parameter is: read it (projections, interpolation, passing it on
  to other readers) — observably state-preserving, so no predicate can
  break; mutate it — the invalidating case; or move it — after which the
  caller has no access, so preservation is moot. There is no way for a
  kept value to escape observably (fate links do not outlive the call;
  `use` constructor arguments are moves). So "mutation forces the
  exhaustive form" is the complete rule for today's language, and a new
  invalidating operation could only arrive with a new capability
  (out-parameters, escaping references).
- **But the *granularity* is wrong, and that is worth marking on
  qualifiers.** Qualifiers split into two kinds:
  * **State predicates** (`NonEmpty`, `Sorted`, `Validated`): claims
    about content. Mutation may falsify them.
  * **Capabilities** (`Mut`, and the usage disciplines `Linear`,
    `Once`): claims about what the *holder* may do. Mutation cannot
    falsify "you may mutate this".
  Only state predicates need dropping when a body mutates; capabilities
  survive trivially. Today this distinction is **degenerate**: every
  capability qualifier is built into the language and every *user*
  qualifier is a state predicate (or provenance, which behaves like one —
  `Validated` is falsified by mutation as surely as `NonEmpty`). So D1
  hard-codes the built-ins and does **not** add a marker; revisit if a
  user ever needs to declare a capability qualifier.
  * Consequence for D1's syntax: `Mut` is written explicitly in
    exhaustive lists (`[list: Mut]`), and `[list:]` keeps stripping
    everything including `Mut`, exactly as today. The alternative
    (auto-preserving capabilities) would silently change `[list:]`'s
    meaning for no present benefit.
- **Accepted cost (user decision 2026-09-02): over-strictness.** A
  mutating function cannot promise to preserve a qualifier it does not
  declare, so `add(list: Mut List<T>, elem: T)` drops a caller's
  `NonEmpty` even though appending cannot empty a list. Remedy is a
  re-test (`if list is NonEmpty`); the general answer is D3.
- **Mixing polarities is an error.** An entry is either all-plain
  (exhaustive) or all-delta (`-`, and later `+`): `[list: Mut -NonEmpty]`
  is redundant under the exhaustive reading and contradictory otherwise.
- **`-Q` may name a qualifier the parameter does not declare** (a
  function that knows it invalidates a specific property). It is a
  convenience only — the exhaustive form remains the sound default, and
  inference never relies on `-`.

**Implementation notes (2026-09-02).** `QualEffect { KeepAll,
Exhaustive(Vec<String>), Remove(Vec<String>) }` lives in `types.rs` and is
carried by both `ParamDeduction` and `FnParamContract`; all three forms
reduce to one operation, `removal_set(have)`, computed against the
qualifiers the *argument* carries (`have`) — that is where the soundness
comes from. Mutation is tracked like move-mode claims: `fate_mutation`
records the enclosing fn's parameter in a new `param_mutations` map (for
*written* lists too — that is what the validation reads), threaded through
`check_once` and the fixpoint's convergence check. Inference gained
`restrict_to` for the contagion rule, and the lattice `KeepAll` →
`Remove` (growing) → `Exhaustive` (shrinking) is what keeps the fixpoint
terminating; `bodyless_mutations` supplies the `Mut`-parameter proxy for
externals. Exactly one test had to change meaning:
`undeclared_qualifiers_pass_through_calls` *encoded* the unsoundness, and
is now `exhaustive_lists_drop_undeclared_qualifiers`, with
`delta_lists_pass_undeclared_qualifiers_through` covering the opt-in.
- **Type in the entry.** An entry's items are qualifiers plus *at most
  one* type — structurally identical to an `is` check pattern
  (`CheckPat { quals, base }`), so the parser and resolver can share that
  path: uppercase idents parse as type refs, and qualifier-vs-type is
  settled by resolution.
  * `[list: Nothing]` is sound *because `Nothing` is uninhabited*: it
    asserts nothing about runtime content, it only withdraws use. That is
    why "moved" falls out of the same rule rather than being a special
    case.
  * **Type narrowing beyond `Nothing`: its own step (user decision
    2026-09-02, following the recommendation).** `[x: Str]` on a `Str?`
    parameter is a *content* claim, so the callee must make it true.
    Out-parameters would do it, but Salvo has none (reassigning a
    parameter is a rebind, invisible to the caller). The implementable
    reading is a **verified postcondition**: "if this call returns
    normally, the parameter is a `Str`" — checked for Salvo bodies by
    requiring the promised narrowed type at every normal exit (the
    machinery `check_linear_exit` already walks), trusted for externals.
    D1 lands with `Nothing` only; this follows as **D1b**.

### D3 — Refinements (and why polymorphism is probably a dead end)

D1's over-strictness has two candidate general answers. Analysis
2026-09-02 says they are *not* both needed, and the more obvious one does
not work:

- **Polymorphism — "preserves whatever predicates it received" — is
  unsound as a blanket promise.** Different predicates react differently
  to the *same* mutation: `add(list, elem)` preserves `NonEmpty` but can
  break `Sorted`. So a mutating function cannot honestly promise to
  preserve an unknown set. Restricting it to *non*-mutating functions
  makes it vacuous (keep-all `[list]` already preserves everything
  there). The only sound version is an explicit per-qualifier set, which
  is refinements by another name.
- **Refinements — a qualifier annotates existing functions with
  additional deductions, without changing their implementation.** This is
  the same information as "which operations invalidate me", stated from
  the qualifier's side, and it is per-qualifier-per-function, which is the
  granularity soundness actually requires. It also inverts the dependency
  in the useful direction: a user's `NonEmpty` can declare that std's
  `add` preserves it, without std knowing the qualifier exists.
  * Open questions: where a refinement may be declared (the qualifier's
    own file, mirroring the constructor-fn rule?); whether it is trusted
    or checked; how refinements from several qualifiers on one function
    compose; and whether a refinement can *strengthen* a std function's
    contract for callers who do not import the qualifier (it must not).

### D2 — Qualifier asserts (`+Q`)

Deferred (user decision 2026-09-02: not even in the grammar for now).
`+Q` asserts that the body *establishes* `Q`, which is what `-> T as Q`
does for return values — the parameter-position analogue. A predicate
qualifier cannot be proven statically (that means reasoning about the
algorithm), so establishment is trust (like `as Q`) or a runtime
`qualifies` check.

- **DECISION D2a** — whether `+Q` and `as Q` unify into one notion of
  "this function establishes a qualifier", and whether establishment is
  trusted, runtime-checked, or restricted to fns declared in the
  qualifier's own file (as `as Q` is today).

### D4 — Predicate `is` on union subjects (and qualifiers over unions)

Motivated by an analysis of the `is` keyword (2026-09-03): `is` has one
grammar and two evidence sources — union-arm identity, statically known
[is-narrowing], and a runtime `qualifies` call [is-qualifies]. There is
no parse ambiguity (one `Expr::Is` node; both forms are
`subject is Qual* [Type] [binding]`), but `is_info` picks between them
by the *subject's shape*: a `Ty::Union` subject **always** takes the
arm-matching path. Consequence: a predicate qualifier can never be
tested against a union-typed value. With `let x: Int | Str`,
`x is Positive` matches no arm and reports "this check can never
succeed"; the workaround is to narrow first (`x is Int && x is Positive`
works, because the second test sees a non-union subject). The predicate
form is shadowed by the union form, and the shadow is invisible in the
surface syntax.

Fixing it is not a checker patch — the narrowed type it should produce
is a union whose arms carry a qualifier, so it needs qualifiers over
unions in general (today [qual-union-arm] binds a qualifier to a single
arm, and only an explicitly parenthesized group can be qualified
[qual-group]).

- **DECISION D4a — semantics of `x is Q` on a union subject.** Which
  arms participate (recommendation: those whose qualifier-stripped type
  satisfies `Q`'s `of` type), and what the check *is*: a conjunction of
  the arm/tag test and the `qualifies` call (recommended — it is what
  the two-step workaround does today), or `qualifies` alone.
  Then-type: the participating arms with `Q` added.
- **DECISION D4b — the else branch.** A failed predicate proves nothing
  ([is-qualifies] already records "no else information"), so the
  remaining set must *keep* the participating arms — unlike a pure arm
  test, which subtracts them. That asymmetry is the load-bearing
  difference and it propagates: a `when` whose arms are predicate checks
  can never be exhaustive. Decide whether predicate checks are allowed
  in `when` arms at all (recommendation: allow only when the arms are
  exhaustive on tags alone, otherwise reject with a message pointing at
  `if`/`elif`).
- **D4c — qualifiers over unions.** Decide the shape of the narrowed
  type: per-arm `Q A | Q B` (recommended — preserves [qual-union-arm]
  and existing arm identity) versus a qualified group `Q (A | B)`
  ([qual-group], which changes wrapper identity). Per-arm keeps the
  positional-arm invariant that the checker and both emitters share.
- **D4d — mixed checks.** `x is Positive Int` on a union: base-type arm
  test *plus* the `qualifies` call, one lowering.
- **Backend work.** `is_tests` and `predicate_tests` are separate side
  tables and each emitter lowers one of them; a union subject with a
  predicate needs a *combined* lowering (tag test `&&` qualifies call)
  in both, and checker and emitters must agree exactly, per the
  invariant. Effects on the `qualifies` fn stay subject to
  [is-qualifies-effects] at every such site.
- Sequencing: independent of D1–D3, but it shares the "what does a
  qualifier mean over a composite type" question with D3's refinements;
  do D4c's decision before either.
- Not in scope: renaming `is`. The two readings are opposites in
  *feel* — "already attached" versus "may be attached" — but both are
  "test whether this holds now, and refine if it does"; conferring a
  qualifier is what `-> T as Q` does [qual-ctor-fn]. User decision
  2026-09-03: keep one `is`, revisit only if D4's rules prove confusing
  in practice.

### D5 — Qualifier subjects: state vs provenance (user decisions 2026-09-03)

The predicate/constructive split [qual-predicate] [qual-constructive] is
an *evidence* axis — how a value acquires a fact. It says nothing about
what the fact is *about*, which is why `NonEmpty` is legitimately both
(one state claim, two evidence routes [qual-ctor-predicate]). Three
independent axes were identified:

| Axis | Values | Status |
|---|---|---|
| Evidence | predicate (runtime `qualifies`) / constructive (mint-only) | in the spec |
| **Subject** | **state (contents) / provenance (the handle)** | **D5** |
| Authority | user-declarable / compiler intrinsic | implicit |

One combination is impossible and explains the shape of the intrinsics:
a **predicate capability cannot exist** — no inspection of the bits
reveals whether you are *permitted* to write, so `Mut` has no
`qualifies` and never could. Capability/provenance implies constructive;
constructive does not imply capability (`Sorted` minted by `sort()` is
mint-only *state*).

Decided (user, 2026-09-03):

1. **Qualifier declarations gain a subject axis**: *state* (default,
   today's semantics) and *provenance*. Option B of the explored set.
2. **Provenance is a content-independent claim about where the handle
   came from** — mint-only (no `qualifies` body; `is Q` on a non-union
   subject stays a compile error, as [qual-constructive] already says).
   The payoff: a provenance tag **survives an unlisted mutation**.
   D1's stripping rule is sound only for claims about contents —
   [deduce-syntax] says so in as many words ("mutation can invalidate a
   caller's *state* predicates") — so exempting provenance needs no new
   soundness argument. Without this, `Authenticated Request` loses its
   tag to any logger declaring `[req: Mut]`, and D3 refinements would be
   the only escape: per-qualifier-per-function boilerplate for a fact
   that is blanket-true.
3. **Provenance is droppable** (forgetting provenance is safe) and
   **survives being stored into another value**.
4. **Provenance composes without a `with` declaration**, mirroring the
   `Mut` auto-qualifier [type-canbe-mut]: it is orthogonal to every
   claim about contents. Tags stack freely, including several over one
   base (`Authenticated EnvironmentId Str`). `with` [qual-with] keeps
   its original job — two *state* claims co-applying, where explicit
   compatibility is the whole point. Without this rule
   `Ok EnvironmentId Str` would be inexpressible (per-tag
   `qualifier Ok<T> of T with EnvironmentId` does not scale, and
   `Ok (EnvironmentId Str)` is the nested-qualified case [qual-generic]
   calls discouraged) — a newtype that cannot be returned in a `Result`
   is half a newtype.
5. **Hard capabilities stay compiler intrinsics** (`Mut`, `Linear`,
   `Once`, `ReadOnly[from: p]`). Each needs something no user can write:
   a per-backend representation choice (`Mut` is *not* erased —
   `MutableList`, `&mut`), a rule in the flow analysis, a non-standard
   subtyping direction (`Once` inverts it), or a restricted syntactic
   position (`Linear` never at a use site; `ReadOnly` return-only).
   User-authored versions would need a metalanguage for checker rules —
   rejected as out of proportion to Simplicity/Verifiability.
6. **Vocabulary.** What users declare is *state* or *provenance*. What
   the compiler owns are *permissions* and *obligations*, and the
   sub-rule is **permissions are droppable, obligations are not**:
   `Mut Person <: Person` (`map` returns `List<T>` while knowing
   `Mut List<T>` internally), while `Once` may never be dropped and
   `Linear` cannot be written at all.

**Motivating example (record this one — it is more legible than
`Authenticated`): the wrapper/newtype pattern.** The Kotlin habit of
`data class Environment(val id: Id)` with a nested
`data class Id(val value: String)` exists so that an incomplete refactor
is a type error instead of a silently mis-wired `String`. As a Salvo
provenance qualifier (`qualifier EnvironmentId of Str` plus a
constructor), it is content-independent (any string can be an id),
mint-only, and not runtime-testable — and it delivers the protection,
because a bare `Str` does not subtype `EnvironmentId Str`: swapping two
tagged arguments fails to compile in both positions. Only widening is
open, which is the approved droppability, and is the same hole Kotlin
has whenever the target parameter is `String`.

Two properties of the erased design worth knowing before revisiting it:

- **In union position the tag is reified.** `Ok Str | Err Str` already
  works — positional wrappers give arms a physical identity even when
  they erase to the same type, and `is Err Str` matches precisely
  [is-precise]. So `EnvironmentId Str | DeploymentId Str` discriminates
  at runtime; erasure applies to a value in a parameter, not to a value
  in a union.
- **What erasure actually costs** is identity, not usability: two tags
  over `"prod"` compare equal and collide as keys in one map. Display,
  interpolation, base-type operations and (static) overloading all work
  for free — the `toString` override the Kotlin pattern needs exists
  only to undo the boxing, which Salvo never does.

7. **Provenance stays erased** (user decision 2026-09-03, D5a settled):
   uniform with [qual-erasure], so the backends are untouched by D5.
   Reification (a wrapper type per tag per backend) was declined: it
   needs wrap/unwrap insertion at every widening site, double wrapping
   inside unions, and messier interop for std fns taking `Str`, while
   the only benefit — distinct equality and map keys — is already
   reachable with a one-field struct, which *is* the nominal flavor of
   this pattern and needs no new feature. The decisive cost is two
   lowering models for one concept, doubling the checker/emitter
   agreement surface. Kotlin's own tool for this job
   (`@JvmInline value class`) is likewise unboxed except in
   generic/nullable/collection positions, so the erased design is the
   idiomatic trade, not an exotic one.
Namespacing (the former D5b) outgrew this section and is now its own
roadmap item, **N1** — dot-names apply to structs as well as qualifiers,
so it is a naming feature independent of the subject axis.

Implementation sketch (no backend work — provenance is erased, so
emitters are untouched under D5a-as-recommended):

- `QualifierDecl` gains the subject kind (parser + AST); the checker's
  `validate_quals` enforces the provenance legality rules (no
  `qualifies` body, no `with` needed, `is` on a non-union subject
  rejected — the last one is already [qual-constructive] behavior).
- The removal-set computation in the deduction machinery skips
  provenance tags when a call strips state qualifiers; the *contagious*
  exhaustiveness rule [deduce-syntax] narrows accordingly.
- Stacking validation stops requiring pairwise `with` when either side
  is provenance.

**Future intrinsic capabilities (watch list, in order of how soon they
force themselves on us).** `Sendable`/thread-safety the moment
concurrency lands — structurally inferred, trusted-by-audit for
externals (the `canbe Linear` pattern), and asymmetric between backends,
so it cannot be user-authored. Then `Uniq`/`Shared` if sharing arrives
(`Uniq` is the precondition for in-place mutation of a shared type),
`Local`/`Escaping` (the conservative refusal of escaping closures
becomes a contract), `Init` for two-phase initialization
(`MaybeUninit`/`lateinit`), and `Const` if compile-time evaluation
lands. Note that `Uniq` and `Local` are facts the fate analysis
*already computes*: they are the user-facing side of the recorded
"internal qualifier unification" leftover, which is the strongest reason
to set the vocabulary up deliberately now.

### Related, independent: `!is` in expressions

Negated checks (`if x !is Str { … }`) — sugar over `!(x is Str)` with the
same fact propagation, and no binding form (a failed test binds nothing:
`x !is T name` is a parse error). Lexing (user decision 2026-09-02): `!`
binds *adjacently* — `x!` (assert) requires no space before `!`, `!is`
requires no space between, `!x` (not) requires no space after, and a
floating `!` (space on both sides) is a syntax error.

The disambiguating clause, refining "not preceded *and* followed by
alphanumerics": the left neighbour must count `)` and `]` as
value-endings, or `names.first()!is Str` stays ambiguous — and
`first()!` is real, common code (the M2 demo interpolates
`${names.first()!}`). So:

> `!` may not be *directly* preceded by a value-ending token
> (identifier, literal, `)`, `]`) **and** directly followed by an
> operand-starting token (identifier, literal, `(`, `[`) or the keyword
> `is`. Whitespace on one side resolves it.

This keeps `x!.field`, `x!)`, `x!,` legal (the following token cannot
start an operand), rejects `x!is T` and `a!b`, and leaves `x! is T`
(assert then test) and `x !is T` (negated test) as the two spellings.

## Roadmap: names and namespacing

### N1 — Dot-names for structs and qualifiers ✅ Done 2026-09-03

Motivated by the wrapper/newtype pattern (see D5): the Kotlin habit of
nesting `Id` inside `Environment` so signatures can demand
`Environment.Id`, without Salvo adopting nested *declarations* — the
user prefers flat code, so the namespace is in the *name*, not the
layout.

Proposed surface: a struct or qualifier may be declared as
`<ns>.<name>`, where `<ns>` names a struct in the same file.
`<ns>` must be a struct in that file, and no name in that file may be
`<ns><name>` when a dot-name with that `<ns>` exists (the Rust
flattening would collide). Backends: Kotlin nests the type inside
`<ns>`'s class; Rust concatenates (`<ns><name>`), since Rust modules and
structs do not relate the way Kotlin classes do. Qualifiers are erased,
so for them the dot is compile-time only on both backends — which is why
qualifiers are the easy half.

Import disambiguation rests on a new casing rule: modules always start
lowercase; types, structs and qualifiers always start uppercase. That
turns `import a.b.Environment.Id` into a decidable split (leading
lowercase segments are the module path, trailing uppercase segments the
item name) where today the split is purely positional — `resolve_import`
takes the *last* segment as the item name and everything before it as
the module prefix.

Decided in review (user decisions 2026-09-03):

- **N1a — the casing rule covers values too.** An uppercase-initial
  *head* identifier starts a *type path*; variables, parameters, fields
  and fn names start lowercase. Needed because in expression position
  `Environment.Id { value: "x" }` is otherwise indistinguishable from a
  field access on a variable named `Environment` (or a dot-notation
  call). Side benefit: `parse_is_check` currently guesses "lowercase
  means a binding" by convention (the `is Str surname` form) — the rule
  makes that principled.
- **N1b — uppercase module segments are a compile-time error, reported
  at source discovery.** Module paths *are* file paths [mod-file], so
  the rule reaches into the filesystem: `src/Utils.sv` stops being a
  legal program, and the diagnostic must name the file rather than
  surfacing later as a baffling unresolved import.
- **N1c — importing `Ns` brings `Ns.X` into scope.** Precedent:
  importing an *effect* brings its members (`ModuleScope` maps members
  to the owning effect). Dot-names carry their namespace at every use
  site, so auto-importing them cannot introduce an ambiguity — and
  without it every newtype costs an import line.
- The **collision ban is module-wide**, not file-wide: a module is all
  of its files, including backend define files (`list.sv` +
  `list.kotlin.sv`) [mod-visibility].
- The **collision ban also covers mangled names and imported names.**
  Overload mangling embeds qualifier names (`full_name__Surname`
  [kt-qual-mangling] [rs-fn-mangling]), so a dot-name must canonicalize
  its dot, and the resulting suffix can collide with a plain qualifier
  of the concatenated name imported from another module — which a
  file-scoped ban never sees. No encoding escapes this (Rust identifiers
  are `[A-Za-z0-9_]`, so every encoding is also a legal name), which is
  why a ban is the right mechanism; it just has to be scope-wide and
  applied to mangled forms.
- **Kotlin emits a *nested* class, never `inner`.** An `inner class`
  captures an outer instance and cannot be constructed without one.
  Consequence: a nested class of a *generic* struct cannot reference the
  outer type parameters (only `inner` can), so either `<ns>` must be
  non-generic or the member may not mention the outer generics.
- **Rust concatenates (`<ns><name>`) — the nested-module alternative is
  rejected.** Rust puts modules and structs in one *type namespace*, so
  `pub mod Environment` collides with `pub struct Environment`
  (E0428, verified with rustc: "`Environment` must be defined only once
  in the type namespace of this module") — and since `<ns>` is required
  to be a struct in the same file, that collision is guaranteed, not
  incidental. A mangled module (`Environment__ns::Id`) compiles but
  reads worse than the flat name; a lowercase module (`environment::Id`)
  also compiles but impersonates a Salvo module and adds a collision
  surface against real ones; Rust allows no item declarations in `impl`
  blocks and inherent associated types are unstable. Flattening is
  therefore the only clean rendering, which is what makes the collision
  ban load-bearing rather than a nuisance.
- Minor, but pin them: names cap at two segments (no `A.B.C`, and a
  dot-named struct may not itself be an `<ns>`); and `ty_base_name` /
  `type_base_name` plus the string-keyed define-template environments
  must agree on the canonical spelling of a dot-name — the documented
  "touch one, touch all three" trap.

Independent of D5: dot-names work the same for state qualifiers, and
apply to structs too, so this is a naming feature rather than part of the
subject axis.

What landed (rules [name-casing] [name-dot] [name-dot-import]
[kt-nested-dot-name]):

- **Parser**: `ident_decl_dotted` for struct/qualifier declarations and
  `type_ref_name` for every type position (annotations, `of` types, `is`
  checks, `as Q`, `canbe`, deduction lists, struct literals). A dot is
  only taken when *both* segments are uppercase, so `person.name` and
  `list.size()` are untouched. The name is carried as one dotted string —
  `Ident { name: "Environment.Id" }` — which is what keeps scope keys,
  checker types and the emitters' `ty_base_name`/`type_base_name`
  conventions in agreement without a second representation.
- **Casing rule**: `ident_type` / `ident_value` at every declaration site
  (structs, qualifiers, types, effects, handlers, generic parameters;
  fns, parameters, fields, `let`/`for`/destructuring bindings, lambda
  parameters). No existing Salvo source violated it — std and the
  corpora were already consistent.
- **Resolve**: casing-aware import splitting (trailing uppercase segments
  are the item, so `import env.types.Environment.Id` works while
  `import core.list.size` keeps its old positional meaning); an unaliased
  namespace import brings dot-named members (`want` matches the `Ns.`
  prefix); `ModuleItems::name_ref` exchanges a synthesized dotted lookup
  key for the declaration's own `&'p str`; `check_dot_names` enforces the
  same-file non-generic namespace, the scope-wide concatenation ban, and
  module-path casing.
- **Kotlin**: `emit_struct_with_members` nests members inside the
  namespace class (declared under their member segment, referenced with
  the dotted name); `qual_suffix` flattens dots so mangled overloads stay
  single identifiers (`label__EnvironmentTag`).
- **Rust**: `rs_ident` flattens dots — one funnel covers declarations,
  references and mangling, and no other Salvo name can carry a dot
  because dots are invalid in Rust identifiers.
- Verified end to end on both backends with two programs — a namespace
  struct with two members plus a dot-named qualifier driving an overload,
  and dot-named types as *union arms* (`when` narrowing, a dot-named
  qualifier in a nullable union, `canbe Mut` on a dot-named struct):
  identical output under `kotlinc` and `rustc`. Union arm identity needed
  no special handling — the wrapper/enum machinery keys off arm position,
  and the flattened Rust names fall out of `rs_ident`.



Ownership-arc remainders (each recorded in its milestone section and/or
spec rule; consolidated here for findability):

- **Fn-type effect lists** (`(v: T) [Console] -> ...`) parse but are not
  enforced as contracts — lambda bodies use the lexical effect
  environment [fn-contract].
- **`Once` inference**: a callee calling its fn param at most once does
  not auto-promote to `Once`; written only [once-fn]. Same
  written-validates/unwritten-infers pattern as deductions when taken.
- **Contracted lambdas passed to *overloaded* fns**: expected fn-type
  contracts only reach lambda literals for single-candidate callees
  (multi-candidate arg typing stays unbiased) [fn-contract]. Related:
  passing an *overloaded* fn by name resolves to the first overload
  (`entries[0]`) with no signature-based selection.
- **Returning/storing capture-carrying closures**: rustc lifetime error
  the checker does not reject; the recorded refinement is
  `move`-closure emission with hoisted clones, pending a treatment for
  captured effect-handler locals [fate-lambda].
- **Exactly-once closures**: a `Once` lambda may not consume a *linear*
  capture (the closure would inherit the obligation) [linear-lambda];
  supporting it means linear fn values.
- **Reassignable borrowed locals** (accumulator bodies in derived-return
  fns: `best = person; …; return best`) are rejected by provenance
  validation; supporting them is the recorded [readonly-return]
  refinement.
- **Effect members with their own generics** are not covered by the
  linear instantiation ban [linear-generics].
- **Same-call borrow/move (E0505 shape)**: one call that passes a
  borrow-emitted local *and* moves its root is checker-legal
  (left-to-right model) but rustc-rejected — loud, rare
  [rs-borrow-locals].
- **Internal qualifier unification** (folding links/poison/consumed_by
  into parameterized qualifiers with per-qualifier joins) remains
  unforced — L7c/L7d landed on links alone; revisit only when a
  customer needs it. L5 field-disjoint precision likewise only if
  whole-variable granularity pinches.

- `salvo lsp`: go-to-definition landed [lsp-definition]. Still open:
  incremental analysis if workspaces outgrow
  re-check-everything-per-keystroke, and a `positionEncoding` negotiation
  for UTF-8-native clients. Signature *hover* still covers fn decls only
  — effect members and define fns have no `FnKey` (go-to-definition does
  reach effect members, via `def_refs`).
- **Predicate `is` on a union subject** is now roadmap phase **D4**, not
  a leftover: a `Ty::Union` subject always takes the arm-matching path,
  so `x is Positive` on `Int | Str` errors ("this check can never
  succeed") instead of calling `qualifies` — narrow first
  (`x is Int && x is Positive`). Lifting it requires qualifiers over
  unions [is-qualifies] [qual-union-arm].
- Struct destructuring ignores predicate-qualifier field overrides
  (deliberate: bindings get the declared type; direct accesses get the
  override + cast).
- Constructing a *nested* qualified union group in one expression
  (`ok(ok("yes"))` into `Ok (Ok Str | Err Int) | …`) needs an annotated
  intermediate `let`; single-level coercion only (errors, never mis-emits).
- Deduction inference does not track bare-parameter value flow out of
  branch/loop tails as a move (documented leniency in [deduce-infer]).
- **Field smart-casting** is now roadmap phase **P1** (place-based flow
  analysis), not a leftover: struct-field subjects do not flow-narrow, so
  after `h.field is Str` a read of `h.field` still has the declared type
  and cannot be interpolated or used as an operand
  ([interp-no-none] [op-no-none]) — the `is Str name` binding form is the
  idiom until P1 lands.
- `yield` inside a *value-position* loop of an iterator body is a kotlinc
  error ("restricted suspending functions…"): the `run {}` value lowering
  is not an inline suspension scope. Loud, never silently wrong; the fix
  is a lowering that keeps the loop inside the `iterator {}` builder.
- Module reachability is name-based and conservative: a local variable
  shadowing a std fn name still pulls that std module in (harmless
  extra output, never a missing module).
- **DECISION (open, for the user):**
  - `when` on field subjects is roadmap phase **P2** (it rides on P1's
    narrowing); the language-surface change to [when-union-subject] is
    still a decision, as are P1a (which projections narrow) and P1b
    (whether narrowing survives a kept-immutable call).
  - Binary operators are typed only for `None` [op-no-none]: everything
    else is unchecked (`Str * Bool` passes, result typing is just the
    left operand's type). Decide the operator typing rules — legal
    operand types per operator, numeric promotion, `Bool` for `&&`/`||`.

## Test inventory (all green: 263)

- `salvo-core`: 47 - 8 unit tests (file classification; `types.rs` union
  normalization, subtyping, display, wrapper detection) + 2 source
  discovery tests (`tests/source_tests.rs` [mod-ignore]: `.svignore`
  skips listed files/subtrees; hidden and `CACHEDIR.TAG` directories
  skipped with the root exempt) + 16 deduction
  tests (`tests/deduce_tests.rs`: exhaustive lists dropping *undeclared*
  qualifiers and delta lists passing them through, mutating bodies
  requiring the exhaustive form, `Nothing` meaning moved
  [deduce-syntax], move inference, call-graph fixpoint
  transitivity, lenient interop borrows, written-list body validation,
  written-list shape validation, stricter-than-body lists,
  `let`-bindings linking instead of moving — the parameter stays kept,
  reads through the alias are free, `copy` severs [fate-link] — and
  move-mode bindings *claiming* parameters as moved, through binding
  chains and propagated through the call graph [fate-move-mode], a
  lambda capture-mutation claiming its parameter while a read capture
  keeps it [fate-lambda], and late move-mode candidates converging in a
  third round with the refined diagnostic superseding the raw
  derived-move error [deduce-fixpoint]) + 2
  structured-diagnostic tests (`tests/diag_tests.rs`: checker errors
  carry file index/span/severity and render with file:line:col + caret;
  multi-file programs index the declaring file [diag-structured]) + 5
  name-collision tests ([mod-collision]: conflicting imports, an import
  shadowing an own-module declaration, `as` resolving the conflict, a
  duplicate declaration within one module, and same-name fns staying
  overloads) + 3 generic-binding tests ([fn-overload]: bindings widen to
  the more general type in either argument order; unrelated bindings
  still reject) + 4 optional-strictness tests ([op-no-none]
  [interp-no-none]: possibly-`None` operands rejected for arithmetic,
  ordering, and equality — and `None` itself — while narrowed/asserted
  operands pass; interpolating an optional or a struct field rejected,
  the `is T name` binding and `!` forms accepted) + 7 name tests
  (`tests/name_tests.rs` [name-dot] [name-dot-import] [name-casing]:
  dot-names resolve, the namespace must be a same-file non-generic
  struct, the concatenated-name ban fires for own-module *and* imported
  names, importing a namespace brings its members and a member imports
  directly, an uppercase module path is an error naming the file).
- `salvo-cli`: 52 - 44 `analyze` integration tests running the built
  binary (`tests/analyze_tests.rs` [cli-analyze]: clean program exits 0,
  type errors render with location and exit 1, JSON diagnostics
  (populated + empty array), parse errors reported, a parse error in one
  file not suppressing checker diagnostics in others, `.svignore`
  exclusions [mod-ignore], import suggestions rendered as help lines +
  JSON `imports` for std and user modules [diag-import-suggest],
  missing-return analysis [fn-must-return], use-after-consume from
  declared *and inferred* deductions, uniform across types, incl.
  reassignment revival, consumption surviving `is`-narrowing restores,
  `while`/`for` loop back-edge re-checking with consume-then-revive
  staying clean, the `if`/`else` merge matrix (consumed on every
  fall-through path / on the only fall-through path / only on an
  always-exiting path), `when`-arm merging incl. subject consumption,
  partial qualifier removal joining conservatively, and call-site
  qualifier removal for kept params incl. `[list:]` [deduce-consume],
  bodyless-declaration explicitness ([decl-explicit]: an external missing
  all three parts, an effect member missing the two that apply to it and
  *not* asked for effects, an effect member's declared deductions enforced
  at the call site, and std's `add` consuming its element),
  the D1 deduction forms ([deduce-syntax]: the `clear`/`NonEmpty`
  unsoundness now rejected, a delta preserving an undeclared qualifier, a
  bodyless `Mut` parameter refusing the delta form, a mutating body
  refusing keep-all, mixed polarities, `+Qual`, and non-`Nothing` types),
  union-arm arguments resolving against union params [type-union],
  the shared-fate matrix (derived variables read-only for
  moves/mutations/returns with the `copy` remedy [fate-derived-readonly],
  root mutation/move/reassignment poisoning derived variables with
  use-site errors naming the event — and unobserved poison staying
  silent [fate-poison], links flowing through projections, `for` and
  `is` bindings transitively to the root, reassignment revival, links
  unioning across branch merges [fate-link], `copy` producing
  independent values that make the whole matrix pass [copy-fn],
  and the backend-parity fix: a projection argument in a kept-`Mut`
  position mutating its provenance roots, poisoning derived variables
  and rejecting mutation through derived ones),
  `[struct-mut]` field assignment requiring a `Mut`-qualified struct
  value,
  the L2 move-site matrix (struct/array/tuple literal stores, spread,
  `break` inside an always-exiting branch consuming after the loop,
  `yield`-in-loop back-edge, `use` constructor arguments — each with the
  event named in the diagnostic, derived variables rejected at the new
  sites, and the positive side: `copy` at every site, reassignment
  revival incl. ahead of the loop back edge, `break n` consuming only
  its operand, and in-branch `return` poison not leaking to the
  fall-through path [deduce-consume] [fate-derived-readonly]),
  the S2 move-mode matrix ([fate-move-mode]: the zero-clone pipeline
  and mutation-driven move-mode staying clean, per-iteration loop
  bindings consumable, immutable projections in moved positions free,
  `copy` keeping sources usable; and the negative side: ancestors
  consumed at the binding with the use-site error naming the binding
  through whole chains, the moved-position parity probe rejected,
  kept-parameter projections erroring with the `copy` remedy, and
  written-kept bindings keeping the S1 move-site error),
  same-call argument ordering ([deduce-same-call]: double moves and
  move-then-read within one call rejected at the later argument, `copy`
  at the consuming argument and kept-position reads clean),
  the L4 capture matrix ([fate-lambda]: immutable captures free,
  closure poisoned by a mutable read-capture's root mutation, mutated
  captures consumed at creation with claims reaching callers,
  written-kept parameter capture-mutation rejected, capture consumption
  always rejected, `copy` remedies clean),
  the L6 linearity matrix ([linear-obligation]: scope-exit leak,
  consumed-on-some-paths-only, dropped expression result, overwrite of
  a live value, return-while-owing, `copy` refused, lambda swallow
  rejected — with pass/discard/return/kept-borrow/alias/composite all
  clean — and the generic instantiation ban [linear-generics]),
  the L7a opt-in matrix ([linear-generics]: opted bodies checked with
  `T` linear, unopted forwarding rejected, non-`Linear` clauses
  rejected, variadic positions refusing linear values with their
  follow-on leaks, and the opted-std `List<FileHandle>` workflow
  clean),
  the L7b `Once` matrix ([once-fn]: double call and loop back-edge
  call consumed, `Once` lambdas rejected at plain-fn boundaries,
  escape-then-call rejected, `Once` on non-fn types rejected, with
  maybe-call and both subtyping directions' positives clean),
  the L7c derived-return matrix ([readonly-return]: independent
  returns rejected, moved/unknown parameters rejected, argument
  mutation poisoning the result, borrowed results immovable with the
  `copy` remedy clean, direct element returns and forwarded derived
  calls clean),
  the L7d contract matrix ([fn-contract]: double use through a
  consuming contract rejected, a consuming named fn rejected at a
  keeping boundary, kept lambda parameters unconsumable, consuming
  contracts propagating to callers, keeping contracts clean),
  `--backend` opting
  define files into the analysis, unknown backend rejected) + 2 UTF-16
  position-mapping unit tests (`src/lsp.rs` [cli-lsp]: multi-byte and
  supplementary-plane round-trips, clamping) + 3 LSP integration tests
  (`tests/lsp_tests.rs` [cli-lsp]: speaks framed JSON-RPC to the binary —
  initialize, didOpen of an unsaved broken buffer -> publishDiagnostics
  with UTF-16 range, didChange fix -> clearing publish, hover -> checked
  type, fn-name hover -> full signature with inferred deductions at both
  the declaration and a call site [fn-ref-table], derived-variable
  hover -> bare `ReadOnly T` type line with root/binding-site detail
  below [fate-link],
  shutdown/exit -> clean process exit; codeAction import quickfix
  round-trip [diag-import-suggest]; go-to-definition for a call-site
  callee, a cross-file struct, an effect member, a handler in `use`, and
  an effect in `of`, plus a no-name position yielding nothing
  [lsp-definition]) + 3 grammar tests
  (`src/lang.rs` [cli-lang]: highlighting categories exactly partition
  the lexer's keyword table, generated grammar is valid JSON containing
  every keyword, checked-in VS Code grammar matches the generated one).
- `salvo-syntax`: 28 - std + LANGUAGE.md-corpus parse-clean assertions with
  insta AST snapshots (`tests/corpus/*.sv`), error-reporting tests,
  lexer unit tests for numeric literal suffixes [lit-numeric] (`1L`,
  `1.2f`, invalid suffix/juxtaposition errors, `1.size()` stays an int),
  3 `canbe` opt-in tests ([canbe-optin]: `canbe Mut` on a struct and
  on an `external type`, `<T canbe Linear>` on a fn, and `canbe` on a
  non-fn type parameter rejected [linear-generics]), and 5 name tests
  ([name-dot] [name-casing]: dot-names in declarations and type
  positions, a dot-name struct literal distinguished from a field read
  and a dot-call, three-segment names rejected, the casing rule enforced
  across ten declaration forms, generic parameters uppercase).
- `salvo-backend-kotlin`: 84 - golden snapshots of the M2 demo, the M3
  unions demo, the M4 qualifiers demo, the M5 effects demo, and the M6
  loops demo;
  M7 assertions (only-used-modules + companion copying, per-module
  packages + generated imports, alias imports, effect-param collision
  avoidance, unique destructure temps);
  M8 `Mut` assertions (`Mut List<T>` maps through the `Mut inline:`
  template; `Mut` on a non-`canbe Mut` type is an error [type-canbe-mut]);
  wrapper/wrap/`is`-lowering assertions (`unions_emit_sealed_wrappers`),
  predicate/mangling/field-cast assertions
  (`qualifiers_lower_to_predicates_and_mangled_overloads`), checker-driven
  effect-resolution assertions (`effects_resolve_through_checker_tables`),
  loop-lowering assertions (`loops_lower_to_run_blocks`);
  union-arm argument wrapping at call sites [type-union]; numeric literal
  suffixes; handler-member template returns
  [kt-handler-template-return]; predicate-qualifier constructors
  [qual-ctor-predicate];
  negative tests (non-exhaustive `when`, non-union `when` subject,
  no-matching-arm wrap, missing effect handler at a fn call site and at an
  effect-member call site, `use` without the `use` effect, duplicate effect
  in an effect list, duplicate `use` registration, unknown effect,
  ambiguous generic effect call, handler-member effect deps, duplicate
  qualifier, incompatible qualifiers, `of`-type mismatch, constructor
  same-file rule, non-simple constructor
  return, `is` on constructive qualifiers, `qualifies` signature,
  constructive values only from constructors, `break` outside a loop,
  missing defines for used external fns/types, uncovered core externals,
  companion/generated-file collision); `copy` intrinsic lowering
  assertions (identity / `.toMutableList()` / `.copy()` / `.copyOf()`
  [kt-copy] [internal-fn]) and a negative test (`copy` of nested
  mutability is a codegen error);
  define/external pairing ([decl-explicit]: same-name defines dispatching
  by parameter base types, a define with no external rejected, two defines
  for one external rejected);
  general-sweep assertions (precedence-preserving binary rendering,
  iterator-body `return` retargeting through a value-position loop,
  alias imports of mangled qualified overloads keeping the `__Qual`
  suffix [kt-imports] [kt-qual-mangling], field-subject `is` lowering
  [is-narrowing] with `when` still rejecting field subjects
  [when-union-subject], union coercion inside array/tuple literals and
  lambda tail returns, type-directed dispatch for unchecked define
  overloads plus the ambiguity error [backend-never-wrong], effect
  member generics rendered on the interface and bound per call
  [effect-member-generics], and an aliased effect type resolving to the
  handler registered under the canonical type
  [effect-disambiguation], and the std array functions with the
  LANGUAGE.md `CyclicRandom` handler [type-array]);
  and twenty-three kotlinc compile+run tests
  with exact stdout assertions (including the M7 multi-module program
  with packages, generated imports, and a companion file, the S1
  copy demo, the S2/S3 move-mode and borrow demos, the L6 linear
  resource demo, and the L7a–L7d linear-generics, `Once`,
  derived-returns, and fn-contracts demos —
  emission aliases throughout, stdout identical to the Rust runs
  [fate-move-mode] [fate-link] [linear-static] [once-fn]).
- `salvo-backend-rust`: 52 - golden snapshots of the same five demos
  emitted as Rust; deduction-mode assertions
  (`deductions_drive_parameter_modes`: kept -> `&`, kept+Mut -> `&mut`,
  omitted -> move, matching call-site argument shapes [rs-borrows]);
  union-enum assertions (`unions_emit_enums`), predicate/mangling
  assertions (`qualifiers_lower_to_predicates_and_mangled_fns`),
  effect-trait assertions (`effects_lower_to_traits_and_mut_dyn_params`),
  loop-lowering assertions (`loops_lower_to_block_expressions`),
  crate-layout assertions (`crate_layout_mounts_only_used_modules`
  [rs-crate] [rs-imports]); numeric literal suffixes emit explicit types;
  predicate-qualifier constructors emit plain fns [qual-ctor-predicate];
  negative tests (missing defines for external
  fns/types, uncovered core externals, generic effect members
  [rs-effects]); `copy`-lowering assertions (`.clone()` on the
  argument's place, and fate-linked `let`s cloning instead of moving
  [rs-copy] [fate-link]); move-mode emission assertions
  (`move_mode_bindings_emit_real_moves` [fate-move-mode]: claimed
  parameter taken by value, loop by value, partial field move, move-mode
  `let` moving, pipeline clone-free); borrow emission assertions
  (`borrow_mode_bindings_emit_borrows` [rs-borrow-locals]: `&Vec`
  parameter iterated bare by reference, `&T` field binding, borrow
  alias of an owned local, read-only pipeline clone-free); `discard`
  lowering assertions (`drop(...)` [linear-discard]);
  general-sweep assertions (field-subject `is` lowering
  [is-narrowing], union coercion inside array/tuple literals and lambda
  tail returns with fn-type `let` annotations dropped [fn-contract],
  type-directed dispatch for unchecked define overloads plus the
  ambiguity error [backend-never-wrong], and an aliased effect type
  resolving to the handler registered under the canonical type
  [effect-disambiguation], and the std array functions with the
  LANGUAGE.md `CyclicRandom` handler [type-array]); and nineteen rustc
  compile+run tests with exact stdout assertions mirroring the kotlinc
  set (demo, unions, qualifiers, effects, loops, multi-module, copy,
  the S2 zero-clone move-mode demo, the S3 borrow demo, the L6 linear
  resource demo, the L7a linear-generics workflow demo, the L7b
  `Once` demo — with `once_fn_params_emit_fnonce` asserting the
  `impl FnOnce` rendering [once-fn] — the L7c derived-returns demo,
  with `derived_returns_emit_borrows` asserting the elided and
  generated-lifetime signatures and the borrow returns
  [readonly-return], and the L7d contracts demo, with
  `fn_type_contracts_emit_modes` asserting `&mut impl FnMut`
  signatures with contract-mode argument types and the named-fn
  adapter [fn-contract]).

When intentionally changing std, the parser AST, the checker's lowering, or
the emitter output, rerun with `INSTA_UPDATE=always` and review the
snapshot diffs.

## Gotchas / lessons learned

- (sweep) Four "known leftovers" were already fixed by earlier work and
  only *looked* open because nothing tested them (field-subject `is`
  lowering, union coercion inside arrays/tuples/lambda returns, and both
  halves of the unchecked-mangling item). Probe before implementing: the
  cheapest step in closing a leftover is a program that demonstrates it.
  Two of those probes instead found *new* bugs (a stray `eprintln!`
  debug print left in `check.rs`, and Rust rendering fn-type `let`
  annotations as `impl FnMut(...)` — invalid Rust).
- (sweep) Emitter *state* beats threaded parameters for context that must
  survive nested lowerings: `StmtCtx` was passed down through
  `emit_stmt`/`emit_block_stmts`, so every value-position path
  (`emit_value_block`, loop lowering, `when` expressions) silently reset
  it to `Normal` and iterator-body `return`s stopped retargeting. As a
  field with explicit save/restore at the two real boundaries (fn bodies,
  lambda bodies) the default becomes "inherit", which is what the
  language means.
- (sweep) Kotlin and Rust do *not* share an operator precedence table:
  Kotlin gives comparison a tighter level than equality, Rust puts them
  on one non-associative level. A ported precedence table must be
  re-derived per backend, or `(a == b) < c` re-renders flat and
  re-associates.
- (sweep) Emitter dispatch in unchecked contexts must be *arity plus
  types*: `resolve_define_fn`'s arity-only fallback silently picked the
  first same-arity template. The fix pattern is
  `<candidates> → filter by checked argument base names → exactly one or
  error` ([backend-never-wrong]); `ty_base_name` (checker types) and
  `type_base_name` (AST types) must agree on conventions (`T?` compares
  as its value arm, arrays as `[]`).
- (sweep) `HashMap` iteration order leaked into user-visible behavior:
  implicit `core.*` modules were added to each scope in hash order, which
  ordered overload candidates. Anything that feeds resolution or
  diagnostics must iterate deterministically (`core_modules` is sorted
  now).
- (sweep) Generic bindings in `unify` must *widen*: first-binding-wins
  made `pick(1, maybe_int)` bind `T = Int` and hide the optional. There
  is deliberately no occurs check — `Ty::Var` identity is name-scoped per
  side, so a callee's `T` legitimately binds to `List<caller-T>`; a
  name-based occurs check would reject generic forwarding.
- (sweep) Handler state fields are declared bare (`i: Int = 0`) — there
  is no `state` keyword, despite the prose in some notes.
- (D1) A test can *encode* the bug. `undeclared_qualifiers_pass_through_
  calls` asserted the exact behavior that made the checker unsound, with a
  comment explaining why it was right. When a soundness fix makes a test
  fail, read the test as evidence about the old model before assuming the
  new code is wrong — and rewrite it to state the new rule rather than
  deleting it (the delta form now carries the old behavior as an opt-in).
- (D1) The unsoundness survived because the *inference* direction hid it:
  facts only shrink, so "preserve everything not mentioned" looked like a
  safe optimistic start. Optimism is only safe if the pessimistic end of
  the lattice is reachable for every fn that needs it — here mutating fns
  could never reach it, because no constraint knew about qualifiers the
  signature never named.
- (decl-explicit) The `add` bug is the cautionary tale for
  inference-from-nothing: a bodyless fn has no body to constrain
  inference, so the "optimistic start" of the deduction fixpoint *was* the
  answer — keep everything, including an element the list had taken
  ownership of. It survived because the two backends disagreed quietly:
  Kotlin aliased and printed, rustc rejected with E0382. When a rule's
  inputs are absent, ask what the optimistic default *claims*, not whether
  the algorithm terminates.
- (decl-explicit) Trust boundaries should be one-directional. Once
  externals must declare their contracts, the checker must *stop*
  second-guessing them — the `Mut`-parameter proxy added hours earlier had
  to come back out, because inferring mutation from a declaration and
  overriding the author is the opposite of "the declaration is the
  contract". Catching wrong declarations belongs in a separate, opt-out
  validation pass (roadmap E2), not in the semantics.
- (decl-explicit) Requiring declarations exposed a second silent hole for
  free: effect-member deductions were *parsed* and ignored, so a member
  taking ownership never consumed its argument. The fix was to extract the
  flow half of the fn-value contract loop (`apply_call_contract`) and
  reuse it — when two call paths are documented as "keep the two in
  sync", that is a sign they should share code instead.
- (D1) Deriving a capability from a *declaration* rather than a body is
  the only option for `external`s, and it is often the right call anyway:
  `Mut` on a bodyless parameter means "I may mutate this", which is
  exactly the fact the soundness rule needs. Applying the same proxy to
  bodied fns would have been strictly worse (a non-`Mut` parameter can
  still have `Mut` *contents* mutated through it — the `h.tags` case the
  fate analysis already tracks).
- (std) Analyzing `--src std` double-loads the standard library (the CLI
  always loads its embedded copy) and trips every [mod-collision] check.
  Expected artifact of pointing the tool at its own std — use an ordinary
  project directory to see std diagnostics.
- (arrays) The CLI embeds std with `include_dir` at *build* time: adding a
  file under `std/` does not invalidate the crate, so the new module is
  silently invisible to `cargo run -- compile` (the tell is the file count
  in "parsed N file(s)"). Touch a `salvo-cli` source file to force the
  rebuild. The backend test crates read `std/` from disk and see new
  files immediately, which makes the discrepancy easy to misread.
- (optionals) A new strictness rule is also a *documentation* audit: the
  interpolation ban immediately flagged three LANGUAGE.md examples that
  read `${person.surname}` after `person.surname is Str` — the spec was
  quietly assuming Kotlin-style field smart-casts. Fix the examples to
  the binding form (`is Str surname`), which the spec already used
  elsewhere. Also worth reading twice: when two backend demo *sources*
  differ (here `${capped}` vs `${capped!}`), that divergence is usually
  evidence of a missing checker rule, not a backend quirk.

- (L7d) Expected types only reach lambda arguments when the arg-typing
  pass supplies them: named calls typed all arguments with `None`
  before overload matching, so fn-type contracts silently never
  arrived at `check_lambda`. Single-candidate callees now pre-lower
  their parameter types as expecteds for lambda literals; overloaded
  callees still probe untyped (an expected could bias resolution) —
  contracted lambdas passed to *overloaded* fns are a known gap.
- (L7d) The named-fn-by-value pass was broken on BOTH backends in
  different ways (Rust: signature-mode mismatch; Kotlin: a bare
  identifier where `::name` is required) — when a feature is
  "loud-but-unsupported", verify each backend's failure mode
  separately; they rarely match.
- (L7c) A borrow crossing a fn boundary interacts with *every* later
  relaxation: S2's move-mode would happily have "taken ownership" of a
  physically borrowed call result (moving out of a `&` — rustc
  rejects, checker accepted). Flow facts that change physical
  representation must be carried on the links themselves
  (`FateLink.borrowed`) so every downstream rule can refuse; when
  adding a new emission regime, probe its interaction with move-mode
  before shipping.
- (L7b) The Ident-callable path in `check_call` never `check_expr`s the
  callee identifier, so consumed-value reads did not error there —
  calling an already-consumed callable was silently accepted until the
  path got an explicit `Ty::Nothing` branch. When adding a consumption
  rule, audit every place an identifier is *used* without going
  through the standard read path.
- (L7b) `Once` is the first qualifier with *inverted* subtyping (a
  restriction, not a refinement): plain fn <: `Once` fn and `Once` may
  never drop. Both `is_subtype` and `unify` carry special cases —
  flagged for review; if a second restricting qualifier ever appears,
  generalize direction into the qualifier model instead of a third
  special case.
- (L7a) Opting a variadic constructor into linearity is a trap:
  variadic positions are untracked by the flow analysis, so a linear
  argument would be physically moved while statically still owed —
  contradictory requirements. The variadic guard refuses linear values
  there outright; collection construction with linear elements is
  empty-then-`add`. When opting in an external, audit *every* position,
  not just the contract shape.
- (L6) A Salvo-bodied consuming fn must end the obligation chain
  itself: `fn close(h: FileHandle) -> [] None {}` leaks `h` by its own
  rules — the body owns the moved-in value and must `discard` it (real
  release lives in external fns with no body to check). Tests and
  examples that stub consumers with empty bodies will all fail the
  frame-drop check; stub with `discard`.
- (L6) The obligation checks mark reported variables as consumed so
  each obligation errors exactly once (a `return`-site error would
  otherwise repeat at the frame pop). Any new exit-shaped check should
  follow the same report-then-consume pattern.
- (S3) Decide the borrow path *before* emitting the value: `emit_let`
  computed `value_code` eagerly, and the discarded emission would have
  recorded coercions/union sizes for code that never lands. Emission
  helpers are not side-effect free — order decisions first, render
  second.
- (S3) By-reference loops hand `&T` to every lowering that touches the
  loop variable: `matches!`/unwrap lowering for union and optional
  elements expects owned subjects and breaks on references — hence the
  concrete-non-union element guard. If later refinements widen the
  guard, extend the union-test rendering to ref patterns first.
- (S3) Borrowck alignment is a consequence of poison, not a separate
  analysis: a checker-legal program never uses a derived value after
  its root's mutation/move, so NLL sees every borrow die before the
  conflict. The one mismatch is *within a single call* (pass a
  borrowed local and move its root in one argument list — rustc E0505,
  checker-legal): loud, documented, rare.
- (L4) "Lambda bodies are a barrier" was only ever true of
  `loop_stack`: bodies are checked *inline*, so flow events (consuming
  calls, mutations) always fired against outer variables — captures
  were never fully untracked, they were tracked with the wrong
  multiplicity (once, at creation). The L4 model rides those existing
  events (a boundary stack + guards at the consuming sites) instead of
  adding a separate capture walk; when auditing "untracked" claims,
  check what the inline checking already does.
- (L3) Sibling arguments of one call are the *only* place where a read
  can see a value after its move without the standard consumed-read
  error firing: argument typing runs before contract enforcement, and
  nested calls consume during typing. Hence the dedicated
  mention-scan (`expr_mentions`) rather than a flow-state fix.
- (L3a) The fixpoint converges because move-mode candidates and claims
  only grow; inferred deductions are the one non-monotone axis
  (overload resolution can flip with narrowing), which is why the
  round cap exists. The instability error path is untested — nobody
  has constructed a genuine oscillator yet; if you find one, turn it
  into a test.
- (S2) Round one must see *every* fate event, or mode inference goes
  blind: kept-`Mut` mutation events only fired when a call contract
  existed, and round one had no inferred facts — so mutation-driven
  move-mode candidates were recorded one round too late. The fix:
  round one falls back to the *optimistic* contract
  (`deduce::optimistic`) for unwritten callees — it enforces no moves
  and strips nothing, so it is behavior-neutral except for surfacing
  the `Mut` mutation events.
- (S2) Flattened fate links now keep their *original* bind spans
  (only the direct source link carries the current event's span). This
  makes a variable's links describe its whole derivation chain, which
  move-mode candidate recording needs — intermediate variables of a
  chain (loop bindings especially) are often dead by the time the move
  is seen. If links are ever restamped again, zero-clone chains break
  silently (one clone reappears per dead intermediate).
- (S2) A `for`-loop binding must be re-declared *fresh on every
  checking pass* of the body (it binds a new element each iteration).
  Declaring it outside `check_loop_body` let its consumed state leak
  into the back-edge re-check — a live false positive
  (`for s in xs { consume(s) }` errored) that predated S2 and only
  surfaced when move-mode made consuming loop bindings routine. Loop
  bindings go through the per-pass `bindings` channel
  (`pattern_bindings`); `is`-bindings always did.
- (L2) An always-exiting branch contributes nothing to the merge after
  the construct — which is correct *inside* a loop body but silently
  drops `break`-path consumption for the code *after the loop*. The
  loop exit is a join point of its own: `LoopCtx` captures a state
  snapshot at every `break` and the loop merges them with the
  fall-through exit state. When adding a new control-flow event, ask
  *where its state lands*, not just whether the branch exits.
- (L2) When merging snapshots with different frame depths
  (`merge_fallthrough`), put the shallowest (current) snapshot first —
  it drives the key iteration, and deeper break-time frames align as a
  prefix. For `for` loops, merge only after the loop-binding frame is
  popped.
- (L2) Kotlin's `.copy()` for struct spread is *shallow* while Rust's
  `..base.clone()` is *deep* — a mutable-data parity divergence that
  had been sitting unobserved in [struct-spread] since M8. Consuming
  the spread base closed it by restriction. Same audit lens as the S1
  lesson: for every emission difference, ask "could the two backends
  disagree observably?" — the remaining known case is projection values
  in moved positions (documented in [fate-poison]).
- (S1) `[struct-mut]` ("only `Mut Name` values may have fields
  assigned") was in both specs but *unenforced* — and became
  load-bearing: Kotlin's identity lowering of `copy` is only correct if
  non-`Mut` values really are immutable. When a backend decision leans
  on a checker rule, verify the rule is actually enforced, not just
  written down. (Enforcing it also exposed a LANGUAGE.md example bug:
  the `Mut` struct example mutated `person` instead of
  `mutable_person`.)
- (S1) Poison rides the existing consumed-state machinery: a poisoned
  variable is just `narrowed = Nothing` plus a `Poison` reason on the
  `LocalVar` — branch merging, revival-by-reassignment, `is`-restore
  survival, and the loop re-check all carry it with no new lattice.
  Keep new flow facts inside `VarState` (narrowed/links/poison) so
  `snapshot_narrows`/`restore_narrows`/`merge_fallthrough` stay the
  single source of flow-state truth.
- (S1) Variable *names* are not stable identities across sibling scopes
  even with [var-no-shadow] (two sequential `for person in …` loops);
  fate links use per-binding numeric ids (`Checker.next_var_id`), and a
  link may outlive its root (the loop binding dies, the flattened link
  to the collection survives) — flatten to all transitive roots at the
  binding, not lazily.
- (S1) `is`/`when`/`for` bindings alias their subject, so they must
  inherit its fate links — the binding tuples are `(Ident, Ty,
  Vec<FateLink>)` threaded through `CondInfo`/`IsInfo`/
  `check_branch_block`. A new binding form must decide its provenance.
- (S1) The checker now keeps *both* sides of `let a = b` readable, so
  the Rust emitter had to stop moving bare-ident sources
  (`emit_linked_value`: owned non-Copy locals clone in `let`/assignment
  values and `for` iterables). Checker-permissiveness changes and
  emission must move together, or the gap surfaces as rustc errors on
  generated code.
- (S1) `i++` and whole-variable reassignment are *rebinds* (revival:
  sever the variable's own links, poison its previous derivatives), not
  mutations — only projection assignment and `Mut`-kept call arguments
  are mutation events. Getting this wrong makes ordinary loop counters
  (`let i = start; while … { i++ }`) uncompilable.
- (S1) Kotlin's `copy` immutability analysis must be *transitive* (a
  non-`Mut` struct with a `Mut List` field is not immutable — identity
  would alias the mutable part and diverge from Rust's deep clone), and
  generic struct fields must be checked under the instantiation's
  substitution (`approx_ty` bridges written field types to checker
  `Ty`s just far enough for that).
- (S1) Leniency gaps in the fate analysis are not always mere
  strictness gaps — some are *backend-parity* holes. Untracked
  mutation events let a Kotlin alias observe a change a Rust clone
  does not (`let t = h.tags; add(h.tags, 2); size(t)` prints 2 vs 1).
  When auditing an unhandled event, always ask "could the two backends
  disagree?", not just "should this be an error?" — the parity
  principle in the roadmap is the checklist.
- `unify` (overload resolution) is *order-sensitive across its match
  arms*: the argument-qualifier-stripping arm
  (`(_, Ty::Qualified { .. })`) must come *after* the union-parameter
  arm, or a qualified argument (`Ok Str`) loses its qualifier before the
  union's qualified arms are tried and `describe(ok("x"))` against
  `Ok Str | Err Str` never resolves. Found passing a union-arm value in
  argument position; `is_subtype` had the arm→union rule all along, but
  candidates were rejected by `unify` before the subtype check ran.
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
- (rename) A keyword rename touches more than the lexer: `KEYWORDS`
  feeds `salvo lang tm-grammar`, and a test compares the generated
  grammar against the checked-in `vscode/syntaxes/salvo.tmLanguage.json`
  byte for byte — add the word to a category list in `cli/src/lang.rs`
  (`keywords_are_fully_categorized` asserts the partition is exact) and
  regenerate the file in the same change. A longer keyword also shifts
  every span in the parser snapshots, so the insta diffs are large but
  should contain *only* span deltas and the renamed field.
- (rename) Sweeping a keyword with `sed` needs case sensitivity and a
  pass over the hits first: `with Linear` (syntax) and `with linear
  type` (an error message's prose) differ only in case, and one site
  that looked like the same clause — `qualifier NonEmpty<T> of List<T>
  with Mut<T>` in `experiments/` — was the *other* meaning of `with`
  ([qual-with]) and had to stay. Grep with context, exclude the
  exceptions explicitly, then re-grep for leftovers.
- (N1) **Rust puts modules and structs in one type namespace.** The
  attractive idea of emitting a dot-name as a real nested module
  (`pub mod Environment { pub struct Id }`) dies on E0428 the moment the
  namespace struct exists — which the rule requires. Verified with rustc
  before writing any emitter code; a lowercase module (`environment::Id`)
  does compile but impersonates a Salvo module. Flattening plus a
  language-level collision ban was the answer.
- (N1) **One name representation beats two.** Carrying a dot-name as a
  single dotted string in `Ident.name` meant scope keys, checker types,
  `ty_base_name`/`type_base_name` and define-template environments needed
  *no* changes — the "touch one, touch all three" trap never fired.
  Translation happens where a Salvo name becomes target syntax: Kotlin
  renders it verbatim (nested access is spelled the same), Rust flattens
  in `rs_ident`, which is the single funnel for all 56 identifier
  renderings. The safety argument for that: a dot is invalid in a Rust
  identifier, so any dotted string reaching `rs_ident` can only be a
  dot-name.
- (N1) **A casing rule pays for itself.** Making types-uppercase /
  values-lowercase normative was needed for dotted *expressions*
  (`Environment.Id { … }` vs `person.name`), but it also turned an
  existing heuristic into a consequence: `parse_is_check` had been
  guessing that a lowercase word after `is` was a binding. Rules that
  retire guesses are cheaper than they look — and no existing source
  violated it, so the sweep cost nothing.
- (N1) Import paths split *positionally* (last segment = item), so
  two-segment item names needed the split to become casing-aware:
  trailing uppercase segments are the item, and a lowercase last segment
  stays a value (`import core.list.size`). The synthesized dotted key is
  short-lived, so `ModuleItems::name_ref` exchanges it for the
  declaration's own `&'p str` before it enters the scope maps.
