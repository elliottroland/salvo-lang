# Salvo Compiler — Progress & Plan

Status snapshot as of 2026-09-01: both backends (Kotlin, Rust) work
end-to-end; the post-M8 phase added developer tooling (`salvo analyze`,
the `salvo lsp` language server, a VS Code extension) and a flow-sensitive
ownership analysis (use-after-consume from declared *and* inferred
deductions, uniform across types, branch- and loop-aware). This document
is the handoff point for continuing development: it records what is
built, the key design decisions, known limitations, and the plan for
what's next — chiefly the roadmap toward full linear types.

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
cargo test                  # 142 tests; includes six kotlinc and six rustc
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

### Current architectural facts worth knowing

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

Where we are: an *affine* analysis ("use at most once") that watches one
event — a bare identifier passed at a call site whose deduction contract
moves or weakens it — with solid underpinnings: interprocedural contracts
(inferred + validated deductions), `Nothing`-narrowing with revival, and
branch-/loop-aware state merging. The deduction *inference* already knows
every way a value escapes a function; the *local flow analysis* only
reacts to calls. Closing the gap is mostly feeding more events into the
same lattice, plus two genuinely new mechanisms (places, must-use).
Meanwhile the safety net holds: any hole surfaces as a rustc error on the
generated code (loud, never silently wrong); Kotlin is unaffected.

Each phase below is independently shippable, in rough dependency order.
Items marked **DECISION** need a language-design call before or during
implementation — everything else is analysis engineering under decisions
already made (uniform-across-types consumption, `Nothing` as the marker,
maybe-moved-is-unusable).

### L1 — Aliasing bindings are moves

`let m = n` and `x = n` (bare non-copy… no — *uniformly*, per the
uniform-consumption decision) consume `n`: narrow it to `Nothing` exactly
like a consuming call. This closes the biggest soundness hole (today the
alias silently duplicates ownership; the Rust backend emits a real move
and rustc rejects later uses loudly) and aligns local flow with what
deduction inference already assumes (`let`-binding a parameter counts as
a move of that parameter).

- **DECISION L1a — the copy escape hatch.** With `let m = n` consuming
  `n`, users need an explicit way to duplicate: a std
  `fn copy<T>(value: T) -> T` (define per backend: Kotlin identity /
  `.copy()` for structs, Rust `.clone()`)? A different name (`clone`,
  `dup`)? Or qualifier-driven (only `Mut`/owned things need copying)?
  Recommendation: `copy(n)` in `core`, defined for all types, so the
  consuming `let` has a one-word remedy in diagnostics ("use
  `copy(n)`").
- **DECISION L1b — do projections copy?** `let m = person.name` reads a
  field. Both emitters *clone* field/index reads today, so semantically
  this is a copy, not a partial move. Recommendation: keep reads-as-
  copies (no consumption), and defer real partial moves to L5. This
  should be stated as a spec rule either way.

### L2 — Remaining consuming sites

Feed the other escape routes into the same narrowing: storing a bare
identifier in a struct/array/tuple literal, `yield n`, `break n`,
`return n` (terminal, but matters inside branches), spread `...n`, and
`use Handler(n)` constructor arguments. Mirrors the move list in
[deduce-infer]; mostly mechanical. The `yield` case interacts with the
loop back-edge re-check (a yield in a loop body consumes every
iteration — the two-pass analysis already models it once the event is
tracked).

- **DECISION L2a — interpolation.** Is `"${n}"` a read or a move? Both
  emitters render interpolated values owned-by-clone, so it is
  physically a copy. Recommendation: reads never consume; spec it.

### L3 — Same-call and convergence tightening

Two known approximations in the current engine:

- `f(a, a)` where both parameters move: arguments are all typed before
  narrowing applies, so the double move within one call isn't caught.
  Fix: apply consumption between argument checks (or a post-check scan
  of the call's own args).
- Two-round checking doesn't iterate: round two's narrowing can change
  overload resolution whose re-inferred deductions never feed back.
  Fix: iterate check→infer to a fixpoint with a small round cap.
  - **DECISION L3a** — determinism/cost policy: fixed cap (e.g. 4
    rounds, error if still unstable — making instability *visible*) vs
    iterate-to-fixpoint (risk of oscillation between overload choices;
    needs a tie-breaker rule).

### L4 — Lambda captures

Captures are completely untracked (lambda bodies are a barrier). A
closure that captures `n` and is stored/returned carries `n` with it.
Prerequisite: audit what the Rust emitter actually does with captured
locals today (clone vs move) — the checker rule must match the emission
or change it.

- **DECISION L4a — capture semantics.** Options: (a) captures always
  *move* (creation consumes; strictest, simplest, matches "no hidden
  borrows"); (b) captures copy via the L1a `copy` semantics (never
  consume, costs clones — closest to current emission); (c) inferred
  per-lambda from usage, with fn-typed values carrying deduction-like
  capture contracts (most precise, most machinery). Recommendation:
  start with (b) to match today's emitters, leave (c) as the long-term
  design.

### L5 — Places and partial moves

Track paths (`x.field`, tuple/array elements), not just whole variables:
destructuring consumes its source; moving a field out leaves the struct
partially unusable. This is the largest analysis change (place lattice
instead of per-variable states).

- **DECISION L5a — allow partial moves at all?** Rust permits moving a
  field out of an owned struct (struct becomes partially moved);
  forbidding it (require destructuring or `copy`) keeps states simple
  and matches the emitters' clone-by-default projections.
  Recommendation: forbid projections-as-moves initially (L1b keeps them
  copies); revisit only with a concrete use case, because "reads are
  copies" may be the permanently right answer for a language without
  references.

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

Sequencing note: L1+L2 are small and high-value (they close real
rustc-rejection gaps); L3 is hygiene; L4 needs the emitter audit first;
L5 can be deferred indefinitely if L1b's "reads are copies" holds up;
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
- Rust: `let a = b` moves local `b`; a later Salvo use of `b` is legal in
  the checker today but fails rustc (loud). Roadmap phase L1 is the
  proper fix.
- An aliased import of a *mangled* qualified overload maps to the
  unmangled name in the generated Kotlin alias import ([kt-imports];
  same class as the unchecked-context mangling gap).
- Module reachability is name-based and conservative: a local variable
  shadowing a std fn name still pulls that std module in (harmless
  extra output, never a missing module).

## Test inventory (all green: 142)

- `salvo-core`: 21 - 8 unit tests (file classification; `types.rs` union
  normalization, subtyping, display, wrapper detection) + 2 source
  discovery tests (`tests/source_tests.rs` [mod-ignore]: `.svignore`
  skips listed files/subtrees; hidden and `CACHEDIR.TAG` directories
  skipped with the root exempt) + 9 deduction
  tests (`tests/deduce_tests.rs`: removal-set subtraction, undeclared
  qualifiers passing through calls, move inference, call-graph fixpoint
  transitivity, lenient interop borrows, written-list body validation,
  written-list shape validation, stricter-than-body lists) + 2
  structured-diagnostic tests (`tests/diag_tests.rs`: checker errors
  carry file index/span/severity and render with file:line:col + caret;
  multi-file programs index the declaring file [diag-structured]).
- `salvo-cli`: 28 - 21 `analyze` integration tests running the built
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
  `--backend` opting
  define files into the analysis, unknown backend rejected) + 2 UTF-16
  position-mapping unit tests (`src/lsp.rs` [cli-lsp]: multi-byte and
  supplementary-plane round-trips, clamping) + 2 LSP integration tests
  (`tests/lsp_tests.rs` [cli-lsp]: speaks framed JSON-RPC to the binary —
  initialize, didOpen of an unsaved broken buffer -> publishDiagnostics
  with UTF-16 range, didChange fix -> clearing publish, hover -> checked
  type, fn-name hover -> full signature with inferred deductions at both
  the declaration and a call site [fn-ref-table],
  shutdown/exit -> clean process exit; codeAction import quickfix
  round-trip [diag-import-suggest]) + 3 grammar tests
  (`src/lang.rs` [cli-lang]: highlighting categories exactly partition
  the lexer's keyword table, generated grammar is valid JSON containing
  every keyword, checked-in VS Code grammar matches the generated one).
- `salvo-syntax`: 20 - std + LANGUAGE.md-corpus parse-clean assertions with
  insta AST snapshots (`tests/corpus/*.sv`), error-reporting tests, and
  lexer unit tests for numeric literal suffixes [lit-numeric] (`1L`,
  `1.2f`, invalid suffix/juxtaposition errors, `1.size()` stays an int).
- `salvo-backend-kotlin`: 50 - golden snapshots of the M2 demo, the M3
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
  companion/generated-file collision); and six kotlinc compile+run tests
  with exact stdout assertions (including the M7 multi-module program
  with packages, generated imports, and a companion file).
- `salvo-backend-rust`: 23 - golden snapshots of the same five demos
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
  [rs-effects]); and six rustc compile+run tests with exact stdout
  assertions mirroring the kotlinc set (demo, unions, qualifiers,
  effects, loops, multi-module).

When intentionally changing std, the parser AST, the checker's lowering, or
the emitter output, rerun with `INSTA_UPDATE=always` and review the
snapshot diffs.

## Gotchas / lessons learned

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
