# IR_CHECKER.md — moving the "real" checking onto the IR

Working document for merging the checker and the IR builder, so that the
syntactic sugar we keep adding is desugared **once** (during lowering) and the
semantic checks (linear obligations, deductions, consume/fate, totality) run
**on the IR** rather than on the AST. Decisions here are the user's to make
([Non-negotiable invariants] in AGENTS.md); this doc is the options + the plan,
not a licence to implement.

Status: **design signed off (user, 2026-10-12); Phase 0 next.** The plan in §4
is being executed in order, one commit per phase, no stop between phases.

---

## 1. Why — the problem this solves

The immediate trigger is noticeboard TODO #2: a `read_value<Str>() ?: ...` that
discards an `Err Checked<StreamError>` is **not** flagged, even though reading a
`Checked` value and dropping it is exactly what the `Checked` obligation exists
to catch. It slips through because:

- The **checker** (`check.rs` + `deduce.rs`) walks the **AST**. It decides what
  each sugar means (an elvis pick, a terminal scope, a `when`) and enforces the
  obligations as it goes.
- The **IR builder** (`salvo-ir`, `build_program`) walks the **AST again** and
  *re-derives* the same sugar's meaning to lower it, copying the checker's
  span-keyed answers (`ty_of`, `coerce`, `linear_moves`, `deductions`, …).

So **every new piece of sugar is implemented twice**: once in the checker (to
check it) and once in the builder (to lower it), against two different mental
models of what the construct desugars to. #2 is the case where the two models
disagree about whether a value is consumed — the builder lowers the pick fine,
but the checker's obligation walk never sees the discarded arm as a *read of a
`Checked`* and so never fires.

The fix the user proposed: **lower once, check on the lowered form.** The IR
becomes the single desugared representation; the obligation/deduction checks
read the IR, where a pick is already a `Branch`/`Switch` with explicit `Read {
consume }` nodes and there is nothing left to re-interpret.

## 2. The target architecture (user-stated)

> the (today's) checker used as a sort of 'working memory' used when lowering
> the AST down to the IR. The IR itself should encode the results of the
> checker, as well as serve as the basis on which to do the 'real' checking and
> validation (such as linear obligations, checking deductions, etc.). … the IR
> [must] store the source spans … so that any checker errors can be surfaced …
> This will also allow me to generate an IR of something which doesn't check.
> It possibly also means that we might need extra directives in the IR which
> encode when certain obligations are met.

Decomposed into four properties the end state must have:

- **(A) Checker as working memory.** Type inference, overload resolution, trait
  obligation solving, flow narrowing — the *inference* that figures out what
  each AST node means — stays. Its output drives lowering. What changes is that
  its results are consumed to **build the IR**, and the IR, not the AST, is what
  the *semantic* checks then run against.
- **(B) IR encodes the checker's results.** Already largely true: every `Expr`
  carries its `Ty`, every narrowing is a `Narrow` with a `Justification`, every
  coercion is an explicit node, every call names the resolved `FnRef`. The gap
  is the *obligation* side (below).
- **(C) IR is the substrate for real validation.** The linear-obligation walk,
  the deduction validation, consume/fate, totality — move from AST walks in
  `check.rs` to IR walks in (a new crate or module). This is the point.
- **(D) IR can represent a program that does not check.** Lowering must not
  assume a clean check; it must produce an IR even where an obligation is
  violated, so the IR-level checker can *find* the violation and point at it.

### What already lines up

The IR is closer to this than it looks. The builder's own header says it
"copie[s] every answer the checker recorded … Nothing here infers." And the IR
**already encodes proof**: a `Stmt::Narrow` carries a `Justification` —
`Test`/`Arm`/`Cond`/`After`/`LoopValue`/`Claim` — saying *why* the narrowing is
sound. That is precisely the shape property (C) and the "obligation discharge"
directives need: a node that records *that a check was discharged, and by what*.
`ExprKind::Read { place, consume }` already marks where a value's life ends. So
the move node and the proof node both exist; what is missing is the obligation
bookkeeping and the spans to report on.

### The gaps (what must change in the IR)

1. **Spans are incomplete.** `Expr` and the top-level decls carry a `Span`.
   `Stmt`, `Block`, `Place`, `Step`, `SwitchArm`, `ArmTest`, `Param`, `Field`,
   `Variant` do **not**. A linear error ("this `Ticket` is dropped here") is
   about a *statement* or a *scope exit*, not an expression, so the IR cannot
   yet point at it. Stmts do carry a `NodeId`, but that is an internal handle,
   not a source location.
2. **No obligation state in the IR.** There is no node or flag that says "a
   linear obligation is created here" / "discharged here" / "must be discharged
   by the end of this scope." Today that lives only in `check.rs`'s transient
   `linear_settled` flags and frame walks.
3. **Lowering is gated behind a clean check.** `build_program` is only ever
   called on a program the checker passed (gated in `salvo-backend/driver.rs`
   `check_for_emission`, `salvo_ir::dump_program`, the `salvo ir` CLI, and the
   golden tests). `Unsupported`/`Diagnostics` exist for *un-lowerable* nodes,
   not for *semantically invalid* ones. Property (D) needs lowering to be
   total over checked-shape ASTs regardless of obligation violations.

## 3. Design decisions (DECIDED — user, 2026-10-12)

### D1. Where do the IR-level checks live? → **Merge `salvo-ir` into `salvo-core`.**

The IR moves into `salvo-core` as the `salvo_core::ir` module (a submodule tree
mirroring today's `salvo-ir/src/`: `ir/mod.rs` the node set, `ir/build/…` the
builder, `ir/dump.rs`, `ir/walk.rs`, `ir/check/…` the new validation).
`salvo-core` becomes **the one place checking is done** — AST inference *and* IR
validation under one roof, no cross-crate cycle (checking reads the IR; the IR
reads `Checked`; both now live together).

- **Rationale (user):** avoid crate proliferation; make core the single home of
  checking. The alternative (a `salvo-ir-check` crate) was rejected — it would
  force either a dependency cycle or a third crate for what is one pipeline.
- **Fallout:** `salvo-backend-kotlin` and `salvo-backend-rust` depend on
  `salvo-ir` today; they re-point `use salvo_ir::…` → `use salvo_core::ir::…`.
  The `salvo-ir` crate and its `Cargo.toml` entry are deleted from the
  workspace. `salvo ir` CLI, `dump_program`, golden tests follow the move.

### D2. What stays in `check.rs` as "working memory"? → **Inference stays; validation moves.**

- **Stays (inference / resolution):** type inference and `expr_ty`; overload and
  member resolution (`fn_refs`, `call_fn`); coercion synthesis (`coerce`,
  `widen_targets`, `field_casts`); narrowing/fate *derivation*; deduction
  *inference* (`deduce::infer` — the builder needs each fn's contract to lower a
  call, so inference cannot move).
- **Moves to the IR checker:** linear-obligation enforcement
  (`check_linear_frame_drop`, `check_linear_exit`, `linear_settled`); deduction
  *validation* (does the body honour its declared contract); consume-after-move
  and fate-conflict reporting; `when`/totality violation reporting.

### D3. How are obligations represented in the IR? → **One `Obligation` directive marking every change to the outstanding-obligation set.**

The IR carries **explicit directives, always** (no deriving from `consume` +
types): the dump becomes a running ledger of what a function currently owes, and
every mutation of that ledger is a visible line. One directive kind with a
transition variant marks the **four points** where the set changes:

1. **GainParams** — at the top of a fn body: the obligations entering from
   linear parameters (concrete linear) and `canbe linear` generic parameters.
2. **Gain** — after a binding (`Let`/`Alias`) whose value, or named arms of it,
   carries an obligation (this is the **return-value** case the user asked for:
   a value received from a call is linear, or has one or more linear arms, which
   are named).
3. **Release** — after a value is moved out: the obligations that are *no longer
   ours*. (A plain move is thus still the thing that transfers an obligation; the
   Release directive makes the transfer explicit rather than leaving it implied
   by a `Read { consume: true }`.)
4. **Discharge** — when an obligation is settled **without** moving (consumed in
   place).

Sketch (final shape settled in Phase 1):

```
Obligation { transition: GainParams | Gain | Release | Discharge,
             obligations: Vec<ObligationRef> }
ObligationRef { place: Place, arms: Option<Vec<usize>> }   // arms: for a union, which arms are linear
```

The IR-level checker is then a **balance check**: walk the directives
maintaining the outstanding set; at every scope exit the set must be empty — a
leftover is the violation. **`Checked<T>` needs nothing special**: it is a
linear struct, so a `Checked` value is Gained at its binding and the error for
#2 is simply "an obligation was neither Released nor Discharged by scope end."

Settled during Phase 1 implementation (will record the outcome here):
- Whether Release is a standalone directive or attached to the `Read`/`Stmt`
  that moves. Default per the "always explicit" rule: a standalone directive,
  with the `consume` flag as the *lowering input* that produces it.
- Conditional moves (moved on one branch, not another): a Release inside the arm
  that moves, so the per-path balance sees it.

### D4. Error reporting from the IR. → **Hard-fail the build; `salvo ir` dumps and annotates inline.**

- IR-check failures are **hard build errors**, same gate as type errors. The
  emission gate changes from "AST check clean" to "AST check **and** IR check
  clean." A dropped linear obligation is exactly what `[backend-never-wrong]`
  forbids, so it must block.
- `salvo ir` **dumps and reports**: it still lowers and prints an invalid
  program (the point of property D — see the IR even when it is wrong), with
  each error embedded **in the dump itself** as `!!! <error text>`, trailing the
  offending IR line (or on its own line where no single line fits). `salvo build`
  is what fails; `salvo ir` stays runnable (exit 0) as the debugging view.

## 4. Phased plan (each phase builds + warm `cargo nextest run` green, then a commit)

Each phase is one commit per AGENTS.md (feature + tests + spec + docs + the
COMPLETED.md decision-log entry land together). No stop between phases unless a
new decision is needed from the user. Spec labels to add as they land:
`[ir-obligation]` (the directive), `[ir-linear-check]` (the balance check).

- **Phase 0 — merge `salvo-ir` into `salvo-core`, then add spans.**
  1. Move `crates/salvo-ir/src/*` under `crates/salvo-core/src/ir/`; delete the
     `salvo-ir` crate and its workspace/`Cargo.toml` entries. Re-point every
     `use salvo_ir::…` (both backend crates, the CLI, tests) to
     `salvo_core::ir::…`. The `salvo ir` subcommand, `dump_program`, and golden
     tests move with it. No behaviour change — the suite is the proof.
  2. Add `span: Span` where a semantic error must point and no child `Expr`
     already carries it. Every `Expr` *already* has a span, and most `Stmt`
     variants wrap an `Expr`/`Block`, so the minimal, low-risk set is: **`Block`**
     (a scope-exit span, for "obligation outstanding at end of scope" — the #2
     case) and **`Param`** (for the GainParams directive). `Place`, `SwitchArm`,
     `ArmTest`, and a general per-`Stmt` span are **deferred**: their errors are
     reachable through a child `Expr`/`Block` span, and retrofitting all ~213
     `Stmt` sites + ~90 `Place` sites across both emitters is precision not yet
     needed (revisit if a ported check wants it). The obligation directives
     added in Phase 1 carry their own spans directly. Keep the default dump
     **byte-identical** (spans not shown) so goldens are untouched. Verifies:
     whole suite unchanged.

- **Phase 1 — the `Obligation` directive + the first IR check (#2).** Add the
  `Obligation` directive (§3 D3) and emit it at the four transition points
  during lowering, driven by the checker's working-memory linearity facts. Add
  the IR-level balance check (`ir/check/`), emitting a diagnostic at the span of
  any obligation outstanding at a scope exit. First real client: the linear
  balance that catches noticeboard #2 (a dropped `Err Checked<…>` arm). Add a
  focused corpus `.sv` reproducing it. Resolves noticeboard #2.

  **Soundness-before-gating (refinement, 2026-10-12).** The balance check
  hard-gates the build (D4a) — so a *false positive* would break a
  currently-valid program (std and every example carry correctly-discharged
  linear values). To de-risk, within Phase 1: (1) emit all four directives from
  the *same* checker signals that already make the AST check correct — Release
  from `consume` (`linear_moves ∪ state_takes ∪ moved_projections`), Discharge
  from narrow-to-non-linear (the `linear_settled` logic); (2) validate the
  balance check against the whole corpus (std + examples + the backend codegen
  cases) and iterate to **zero false positives** on known-good code *before* it
  gates; (3) only then wire the hard gate and add the #2 positive test. If a
  class of false positive cannot be resolved cleanly in Phase 1, the IR check
  ships **advisory (warn, no gate)** in Phase 1 and the hard gate moves to
  Phase 2, where the AST enforcement is deleted and the two can be diffed
  directly. The AST check stays live throughout Phase 1, so real leaks are still
  caught meanwhile.

- **Phase 2 — move linear-obligation enforcement onto the IR.** Make the balance
  check the *sole* enforcer of linearity; delete the AST-side
  `check_linear_frame_drop` / `check_linear_exit` / `linear_settled` in the
  **same commit** (no double diagnostics). Verifies: every existing linear test
  still fires (spans may improve).

- **Phase 3 — move deduction *validation* onto the IR.** `deduce::infer`
  (inference) stays in `check.rs`; the "body honours the declared contract"
  check moves to the IR, where consume/borrow are explicit directives.

- **Phase 4 — consolidate.** Revisit whether `check.rs` can shed the
  now-duplicated obligation bookkeeping, and whether any of the ~15 span-keyed
  `Checked` tables the builder reads can be dropped in favour of IR structure.
  (Ties into memory `checked-unify-options` / ROADMAP §13(b).)

## 5. Risks / open questions

- **Double diagnostics.** During phases 1–3 a check briefly exists in both
  places; each ported check must be deleted from `check.rs` in the *same* commit
  that adds it to the IR.
- **Span fidelity.** Phase 0 must thread *source* spans, not synthesised ones,
  or ported checks point at the wrong place. The AST nodes the builder lowers
  from still have their spans at lowering time; the work is plumbing.
- **Generated decls have no source span.** Actor dispatch fns, message enums,
  stubs (`actors::generate`) are built, not parsed. Their IR nodes need a
  sentinel span (or the enclosing decl's span) so the IR checker attributes or
  skips them sensibly.
- **Gate ordering.** The build gate becomes "AST clean AND IR-check clean."
  `salvo ir` must *not* gate (it reports inline and exits 0); the four gate sites
  (`driver.rs::check_for_emission`, `dump_program`, the CLI, golden tests) each
  need the right one.

---

## Decision log (this doc)

- **2026-10-12 — Phase 1 status: directives + per-path check built; the check
  is NOT yet sound enough to gate (needs the Phase 2 flow work).** The
  `Obligation` directive, the shared `crate::linear` predicate, the
  annotation pass (`ir::obligations`), and the per-path balance check
  (`ir::check`) are all built and compile. The ledger dumps correctly, and the
  #2 case shows exactly the intended shape — `gain [__pick~1.arms{1}]`,
  `discharge` on the kept (narrowed-to-`Int`) path, and nothing on the `else`
  path where `return 0` drops the `Problem` arm.

  **But validating the check against known-good code (std + examples) found
  ~4131 false positives**, so it CANNOT gate the build yet (per the
  Soundness-before-gating refinement, a false positive breaks a valid program).
  The false positives are **not** random — they are a few flow patterns the
  coarse directive placement does not yet model, the same patterns the AST
  checker's `linear_settled`/`with_narrows` flow analysis handles:
  - **Per-arm narrowing (`is`/`when`/`if … is Err`).** A `Ok A | Err Checked<E>`
    value handled by narrowing each arm and consuming/returning it per branch:
    the discharge happens *through* the narrow-and-consume in each arm, which
    the current `discharge_of` (only a `Narrow` that drops to a wholly
    non-linear type) and `releases_in` (only a direct `Read{consume}` in the
    same statement) do not jointly capture. Dominant class (`opened`, `closed`,
    `content`, `got.arms{…}`, `__subject~1`).
  - **A linear value returned by `return x`** is a release, now ordered before
    the leave — fixed — but a value consumed deeper (passed to a consuming
    call whose result is returned) still needs the release attributed to the
    leaving statement.
  - **Loop-carried and reassigned linear locals** (`out`, `done`, `m`): a
    value re-bound each iteration, or moved then rebound.

  **Resolution (chosen, within the pre-approved fallback):** Phase 1 ships the
  **directives + the ledger in the dump** (correct, and the point of D3's
  "make the IR reason visibly"), plus `ir::check` present but **not wired as a
  gate and not run by default**. The hard gate (D4a), the `salvo ir` `!!!`
  inline annotation (D4b), and driving the false-positive count to zero move to
  **Phase 2**, where the AST enforcement is deleted and the IR check is iterated
  against the AST checker's behaviour as the reference oracle — the natural
  place to get the flow analysis exactly right, rather than guessing twice. The
  AST checker stays the sole enforcer through Phase 1, so nothing regresses and
  real leaks (including the std ones, which are *not* real — they are false
  positives) stay correctly handled.

  **Open question deferred to Phase 2 (noticeboard #2):** the whole merge was
  motivated by #2, which the AST checker misses. Phase 1 makes #2 *visible in
  the dump* but does not yet *fail the build* on it. Catching #2 as an error
  requires the sound per-arm flow check — so #2's fix lands with Phase 2, not
  Phase 1. (If the user wants #2 enforced sooner, the alternative is a narrow,
  special-cased check for exactly "a linear union arm dropped by a pick with no
  narrow/consume on the dropped path" — smaller, but it is the thin end of the
  flow analysis Phase 2 does properly.)

- **2026-10-12 — Phase 1 progress: directives + per-path check built; false
  positives driven from 4131 → ~45 on std; check unwired pending the rest.**
  Built and committed as a checkpoint (user: committing mid-solution is fine):
  the `Obligation` directive, the shared `crate::linear` predicate, the
  annotation pass (`ir::obligations`), and the per-path balance check
  (`ir::check`). The ledger dumps correctly and the #2 case shows the intended
  shape (gain the `Problem` arm, discharge on the narrowed-to-`Int` path,
  nothing on the `else` path that drops it).

  **Key architecture fix: the ledger lives only on the dump/check path, never
  in the IR the backends emit.** Inserting directives into `build_program`'s
  output changed what the Rust emitter's handle analysis (`paths.rs`) saw and
  regressed `runtime.rs` codegen. So `obligations::annotate` now runs *only* in
  `dump_program` (for `salvo ir`) and before `ir::check` — the emitters get the
  un-annotated IR and generate byte-identical code. Golden snapshots updated to
  show the ledger; the Rust/Kotlin codegen goldens are unchanged.

  **Validating the check against known-good code (std + examples) drove a
  sequence of soundness fixes**, each grounded in a real flow pattern seen in
  the dump (4131 → 2071 → 180 → 147 → 101 → 64 → 17 → ~45):
  - **Narrow-transfer:** a `Narrow` of a linear source transfers its obligation
    to the narrowed binding (source discharged on the path, binding gains).
  - **Container decomposition:** a linear struct is settled once all its linear
    fields are moved out (`ObligationRef.container_fields`); a field-release
    `x.f` decrements the container's pending fields.
  - **Borrows owe nothing:** a `ref`/`proj`-qualified type (and borrowed union
    arms) carry no obligation.
  - **Negative/positive arm tests:** falling past `x is arm k` (or guarding an
    arm with `!(x is arm k)`) proves the arm absent and discharges it.
  - **Switch dispatches its subject:** the subject leaves the set on entry
    (distributed to per-arm narrows).
  - **Actor expression forms** (`Send`/`ReplyTo`/`SelfSend`/`HandlerCall`/
    `Spawn`) carry consuming reads that release — traversed now.
  - **Generic container fields:** `linear_fields` resolves a field's bare
    generic type through the struct's instantiation (fixed `detach`).

  **Remaining 7 are one coherent cluster** (down from ~45 after fixing a
  `bare_name` bug that misread any argument-less named type as a generic, and
  after making `break`/`continue` leave only the *loop* — reporting obligations
  gained inside the loop, not those inherited and settled after it). All 7 are
  in the actor runtime (`runtime::*`) and share a root cause: a **struct field
  whose type is a union with linear arms** (e.g. `Token { target: ToActor |
  ToWaiter | ToTask }` where `ToTask` is linear). The AST-based
  `linear::linear_fields`/`approx_ty` does not resolve such a field's union
  linearity, so the container's `container_fields` misses it and decomposition
  never settles the container. The fix is to compute a field's linearity from
  the IR builder's already-resolved `StructDecl.fields` types rather than
  re-approximating from the AST — a clean next step. Tracked by the harness.

- **2026-10-12 — Phase 1 implementation finding: the balance must be
  per-arm and path-sensitive, and Gain must fire on synthetic bindings.**
  Building the directive emission surfaced two things the flat model missed,
  and the second needs a user call on the obligation *semantics*:
  1. **Gain must fire on synthetic bindings, not just source `let`s.** The
     elvis `?:` pick desugars to a synthetic `let __pick~1: Int | Problem`
     built inside `exprs.rs`, bypassing `let_stmt` — so a Gain hook placed only
     in `let_stmt` misses exactly the #2 case. Fix (mechanical): emit Gain
     centrally for every `Stmt::Let` whose type is linear, wherever built. Not
     a semantics question.
  2. **The #2 leak is per-arm and path-sensitive — the flat per-scope balance
     cannot express it.** For `attempt() ?: return 0` where `attempt()` returns
     `Ok Int | Err Problem` (Problem linear): the pick binds `__pick~1`
     (gains the Problem-arm obligation), narrows to `Int` on the kept path
     (the Problem arm is proven *absent* there — so the obligation is
     discharged-by-narrowing on that path), and on the else path the value
     still holds the `Problem` arm and `return 0` drops it — the leak. A flat
     "gained once, released once per scope" counter cannot see this: the
     obligation is live on one path and gone on another, and the drop is at a
     path exit. The IR already carries what's needed — `Stmt::Narrow` with a
     `Justification` says which arm is live where, and `Switch`/`Branch` arms
     delimit the paths — so the balance check must walk the control-flow arms
     and track the outstanding set **per path**, discharging an arm-obligation
     when a narrow proves that arm absent and reporting it when a path exits
     (return/break/fall-through) with the arm still possible.

  **This is a refinement of the D3 obligation model; the user signed it off
  (2026-10-12).** The directive node already
  supports per-arm refs (`ObligationRef.arms`); the confirmed checker reasoning
  is: (a) a `Discharge` is emitted when a `Narrow` removes a
  linear arm from a value's live type (arm proven absent on that path); (b) the
  balance check is per-path over the `Branch`/`Switch` arm structure, and an
  obligation (or a still-possible linear arm) outstanding at a path exit is the
  leak.

- **2026-10-12 — all four design decisions made (user).** D1: merge `salvo-ir`
  into `salvo_core::ir`, making core the single home of checking (no new crate).
  D2: inference stays in `check.rs`/`deduce.rs`, validation moves onto the IR.
  D3: one `Obligation` directive, always explicit, marking the four transitions
  of a fn's outstanding-obligation set (GainParams / Gain / Release / Discharge),
  with per-arm granularity for unions; the IR checker is a balance check and
  `Checked<T>` needs no special case. D4: IR-check failures hard-fail the build;
  `salvo ir` dumps invalid programs and annotates errors inline as `!!! …`,
  exit 0.
