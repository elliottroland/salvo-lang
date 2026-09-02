# Salvo Compiler — Progress & Plan

Status snapshot as of 2026-09-02: both backends (Kotlin, Rust) work
end-to-end; the post-M8 phase added developer tooling (`salvo analyze`,
the `salvo lsp` language server, a VS Code extension) and a flow-sensitive
ownership analysis (use-after-consume from declared *and* inferred
deductions, uniform across types, branch- and loop-aware). The whole
shared-fate arc is complete — **S1 (strict links + poison), L2
(remaining consuming sites), S2 (move-mode bindings → real moves), L3
(same-call ordering + capped check/infer fixpoint), L4 (lambda captures
under shared fate), and S3 (borrow emission)**: move-mode bindings emit
real moves, borrow-mode bindings and loops emit real borrows (`&T`
locals, by-reference iteration), and read-only pipelines over kept
parameters are clone-free end to end. This document is the handoff
point for continuing development: it records what is built, the key
design decisions, known limitations, and the plan for what's next —
the remaining linear-types phases (L5 field precision if ever needed,
L6 must-use linearity, L7 parameterized compiler qualifiers).

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
cargo test                  # 170 tests; includes nine kotlinc and nine rustc
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
  language-level qualifier any type opts into with `with Mut`
  [type-with-mut]; backends map it per type (`Mut inline:` define
  sections). Unions are generated enums; effects are traits with
  `&mut dyn` threading; iterators are *eager* (`Iter<T>` = `Vec<T>`,
  documented divergence [rs-iter-vec]); `WrapOption` coercion added
  because optionals are physical in Rust and transparent in Kotlin
  [type-nullable]. Crate layout: main-declaring module is the crate root
  with `#[path]` mounts [rs-crate].

### Post-M8 — tooling and flow analysis (decision log)

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

### L6 — Must-use: true linearity

Everything through L5 is affine ("at most once"). The linear half ("at
least once") makes dropping a value an error — the payoff for resource
types (file handles, transactions, sockets): forgetting to close/commit
becomes a compile error. Needs: an opt-in marker on types, an
obligation check at scope exit on every path (the branch-merge machinery
provides the paths), and a blessed set of consuming operations.

- **DECISION L6a — the marker.** How does a type opt in? `with Linear`
  auto-qualifier (parallel to `with Mut`, backend-neutral, fits the
  existing `type … with` syntax) vs a `Linear` qualifier applied at use
  sites vs a distinct declaration keyword. Recommendation: `with
  Linear` on the type declaration.
- **DECISION L6b — what consumes.** Any move (passing to a consuming
  call, returning, storing)? Or only designated consumers (fns marked
  somehow, e.g. by taking the parameter unlisted in deductions — which
  is exactly "moves it")? Recommendation: consumption = any move; the
  deduction system already defines it.
- **DECISION L6c — escape hatches and failure paths.** Is there a
  `discard(x)` in std for deliberately dropping a linear value? What
  happens on early-`return` paths (obligation still checked — the
  merge machinery handles it) and on future panic/abort semantics
  (out of scope until Salvo has them)?
- **DECISION L6d — linearity in composite types.** Is a
  `List<FileHandle>` linear? A union with one linear arm? An optional?
  Simplest sound rule: a composite containing a linear component is
  itself linear. Generics: forbid instantiating an unconstrained `T`
  with a linear type initially (a `where T: Linear`-style opt-in can
  come later).
- **DECISION L6e — Kotlin backend stance.** Linearity is enforced
  purely statically; on the JVM nothing physically prevents reuse, and
  there are no destructors either way. Recommendation: document that
  linear types are a *protocol* checker feature, identical on both
  backends, with no runtime component.

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
- Additional option for the same milestone (user suggestion
  2026-09-02): a **call-multiplicity compiler qualifier** on fn-typed
  parameters, surfaced in deduction lists — e.g. a fn whose deductions
  mark a lambda parameter `Once` guarantees it calls the lambda at most
  once. That would let a lambda *consume* its captures when passed to
  such a fn (today always an error [fate-lambda], because multiplicity
  is untracked) — the Rust backend would emit `FnOnce`. Fits the same
  distinct-class rules (compiler-inserted, inferred from the callee's
  body, never affecting overloads).

Sequencing note: L1 lands in stages (S1 strict checker-only → S2
move-mode bindings → S3 borrow emission); S1+L2 closed real
rustc-rejection gaps; L3 and L4 are done (2026-09-02);
L5 is largely subsumed by shared fate (field-disjoint precision only);
L6 is the only phase introducing new language surface and should get a
LANGUAGE.md section of its own before implementation. Per AGENTS.md,
each phase lands with LANGUAGE_SPEC.md rules (extending
[deduce-consume], new [linear-*] labels for L6) and tests at every
affected layer.

## Remaining leftovers (small; no milestone claims them)

- `salvo lsp` go-to-definition: `Resolution`/`Symbols` know the declaring
  items but no def-site *spans* are recorded; add ident spans to the
  declaration tables and a `textDocument/definition` handler. Also worth
  considering: incremental analysis if workspaces outgrow
  re-check-everything-per-keystroke, and a `positionEncoding` negotiation
  for UTF-8-native clients. Signature hover covers fn decls only —
  effect members and define fns have no `FnKey`.
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
  checker-`Ty` keys (see the fallback note under architectural facts).
- The LANGUAGE.md `CyclicRandom` example calls `values.size()` on a `T[]`;
  std only defines `size` for `Str` and `List<T>` — either add an array
  `size` or move the example to `List<T>`.
- Effect member fns with their *own* generics are lowered but never
  substituted per-call (only the effect's generics are).
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
- An aliased import of a *mangled* qualified overload maps to the
  unmangled name in the generated Kotlin alias import ([kt-imports];
  same class as the unchecked-context mangling gap).
- Module reachability is name-based and conservative: a local variable
  shadowing a std fn name still pulls that std module in (harmless
  extra output, never a missing module).

## Test inventory (all green: 170)

- `salvo-core`: 25 - 8 unit tests (file classification; `types.rs` union
  normalization, subtyping, display, wrapper detection) + 2 source
  discovery tests (`tests/source_tests.rs` [mod-ignore]: `.svignore`
  skips listed files/subtrees; hidden and `CACHEDIR.TAG` directories
  skipped with the root exempt) + 13 deduction
  tests (`tests/deduce_tests.rs`: removal-set subtraction, undeclared
  qualifiers passing through calls, move inference, call-graph fixpoint
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
  multi-file programs index the declaring file [diag-structured]).
- `salvo-cli`: 41 - 34 `analyze` integration tests running the built
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
  `--backend` opting
  define files into the analysis, unknown backend rejected) + 2 UTF-16
  position-mapping unit tests (`src/lsp.rs` [cli-lsp]: multi-byte and
  supplementary-plane round-trips, clamping) + 2 LSP integration tests
  (`tests/lsp_tests.rs` [cli-lsp]: speaks framed JSON-RPC to the binary —
  initialize, didOpen of an unsaved broken buffer -> publishDiagnostics
  with UTF-16 range, didChange fix -> clearing publish, hover -> checked
  type, fn-name hover -> full signature with inferred deductions at both
  the declaration and a call site [fn-ref-table], derived-variable
  hover -> bare `ReadOnly T` type line with root/binding-site detail
  below [fate-link],
  shutdown/exit -> clean process exit; codeAction import quickfix
  round-trip [diag-import-suggest]) + 3 grammar tests
  (`src/lang.rs` [cli-lang]: highlighting categories exactly partition
  the lexer's keyword table, generated grammar is valid JSON containing
  every keyword, checked-in VS Code grammar matches the generated one).
- `salvo-syntax`: 20 - std + LANGUAGE.md-corpus parse-clean assertions with
  insta AST snapshots (`tests/corpus/*.sv`), error-reporting tests, and
  lexer unit tests for numeric literal suffixes [lit-numeric] (`1L`,
  `1.2f`, invalid suffix/juxtaposition errors, `1.size()` stays an int).
- `salvo-backend-kotlin`: 55 - golden snapshots of the M2 demo, the M3
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
  mutability is a codegen error); and nine kotlinc compile+run tests
  with exact stdout assertions (including the M7 multi-module program
  with packages, generated imports, and a companion file, the S1
  copy demo, and the S2/S3 move-mode and borrow demos — emission
  unchanged, stdout identical to the Rust runs [fate-move-mode]
  [fate-link]).
- `salvo-backend-rust`: 29 - golden snapshots of the same five demos
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
  alias of an owned local, read-only pipeline clone-free); and nine
  rustc compile+run tests with exact stdout assertions mirroring the
  kotlinc set (demo, unions, qualifiers, effects, loops, multi-module,
  copy, the S2 zero-clone move-mode demo, and the S3 borrow demo).

When intentionally changing std, the parser AST, the checker's lowering, or
the emitter output, rerun with `INSTA_UPDATE=always` and review the
snapshot diffs.

## Gotchas / lessons learned

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
