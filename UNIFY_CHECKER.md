# Unifying the checker's side tables → attributed IR (Option 4)

Working document for ROADMAP §13(b): the ~50 span-keyed `Checked` tables that
exist only to hand facts from the checker to the IR builder. Goal (user,
2026-10-10): **Option 4 — attributed IR**, reached *through* Option 3
(cluster the tables by consumer). This doc records the plan and work in
progress so the context can be cleared and resumed from here.

When the work lands, this doc is retired into COMPLETED.md's decision log and
ROADMAP §13(b) is closed (the option-space design-doc convention).

## The finding that shapes everything (verified 2026-10-10)

- **The backends read ONLY the IR**, never `Checked` (grep of
  `salvo-backend-{rust,kotlin}/src` is clean). So "Option 4 for the backends"
  is already done — the IR is attributed: `Expr` carries `ty`/`span` inline,
  `ExprKind::Call` carries resolved `target`/`type_args`/`args`, `MakeUnion`
  carries `arm`, `Spawn`/`ReplyTo::Task` carry `effects`, etc. The checker's
  span-keyed facts are **consumed by the builder and baked into node fields**;
  they do not survive into the IR.
- **So this is entirely a checker→IR-builder handoff problem.** The ~50
  `HashMap<Key, V>` tables (`Key = (file, span)`) exist so
  `crates/salvo-ir/src/build/body/exprs.rs` can populate node fields. The
  builder reads ~50 of them, each as `self.ctx.checked.<table>.get(&self.key(span))`
  — one map lookup per fact per node, the "check multiple places" friction.
- **`Checked` field census** (≈90 fields): ~75 `Key`-keyed (per-expression),
  ~7 `FnKey`-keyed (per-fn contracts — the std-cache set, leave alone), ~8
  oddballs (whole-program sets `parking_handlers`/`platform_closure_all`; the
  deadlock edge-lists `actor_sends`/`waitfor_sites`/… which are `Vec<(String,
  Key)>` and consumed by `deadlock::check`, *not* the IR builder — leave
  alone). **Only the per-expression, IR-builder-consumed tables are in scope.**

## Why we skip Option 2 (recorded so it is not re-litigated)

Option 2 was "a read-side `view(span)` helper over the unchanged tables." It
gives the builder one call instead of N `.get`s, but it does **not** move facts
toward node attribution — the facts still live in N scattered maps written at
~200 sites, which is exactly what Option 4 must dissolve. On the path to 4 it is
throwaway. User decision: skip it, go straight to Option 3's clustering, which
*is* a step toward 4 (cohesive records the builder consumes, then inlines into
nodes).

## Correction (2026-10-10, after Phase 0): the goal is view accessors, not storage-collapse

Phase 1 was planned as "collapse each cluster's tables into one
`HashMap<Key, ClusterFacts>`". Investigating the writers/readers showed this is
the **wrong move**, and the plan is corrected:

- **The tables are not IR-builder-exclusive.** `call_fn` has 13 readers across
  the checker (overload resolution, `deduce.rs`, …); `effect_calls`/`may_throw`
  are read by the checker's own hot paths; `self_sends`/`facade_sends`/
  `addr_calls` are also read by `features.rs` (`.keys().any`). Collapsing their
  *storage* would force rewriting every checker reader and complicate the
  checker's own logic — wide, risky, and low value.
- **Option 4's backend-visible goal is ALREADY met.** The backends read only
  the IR; nothing after `build` reads `Checked` (refs pass, dump, both
  emitters are clean). The IR nodes are self-contained. The span tables are
  **builder-private handoff state** the backends never see.
- **The only remaining "Option 4" would be merging the checker and IR builder
  into one pass** so facts never need a span-keyed intermediary. That is a
  massive rewrite with **no** backend-visible benefit (the tables are already
  invisible downstream). Not worth it; recorded as rejected.

**So the valuable, safe work — and what this doc now plans — is the ergonomic
win: one view accessor per builder read-cluster** (like Phase 0's `call_info`),
so the builder reads one struct per concern instead of N scattered
`.get(&self.key(span))`s. The tables stay as sound checker state. This fully
answers the stated pain ("check multiple places when building the IR") without
destabilising the checker. Each accessor is a pure refactor gated on the IR
goldens.

Steps are therefore: a `*_info(key)` view accessor for each cluster the builder
reads together, and switch the builder's lowering for that construct to use it.
No `Checked` field is removed; no writer site changes.

## Constraints any change must respect

1. **Writers set one fact at a time.** ~200 checker sites do
   `self.out.<table>.insert(key, v)`. A cluster must still let a site set its
   one field without constructing the whole record (`entry(k).or_default().f = v`).
2. **Absent = no fact.** Most expressions have no entry in most tables; keep
   storage sparse (per-`Key` cluster structs, not one giant per-expression
   struct — that was Option 1, rejected for sparsity).
3. **`Checked: Clone`** (the std-cache wraps a whole `Checked`). Clusters stay
   `Clone`.
4. **The IR goldens are the regression gate.** `salvo-ir`'s golden/corpus tests
   compare emitted code; after each cluster migration they must be unchanged
   (`cargo test -p salvo-ir`, and the kotlinc/rustc corpus tests). Any diff is a
   bug in the migration, not an intended change.
5. **No backend or language-surface change.** This is internal; not a DECISION.
   Keep each step a pure refactor with goldens green.

## The clusters (draft — refine as we go)

Grouped by how the builder reads them together. Exact membership is settled
when each cluster is migrated (read the builder's call/use/etc. lowering and
take what it consults at one node). First-cut:

- **call**: `call_fn`, `call_type_args`, `call_effects`, `may_throw`,
  `lending_calls`, `derived_calls`, `distinct_pairs`, `temp_views`,
  `mut_lend_calls`, `fn_value_calls`, `implicit_args`, `moved_projections`,
  `local_calls`, `local_call_names`, `member_calls`, `effect_member_calls`,
  `effect_calls`, `dot_calls`. (The densest, most-read — do it first.)
- **expr-type**: `expr_ty`, `repr_ty`, `promotions`, `coerce`, `field_casts`,
  `widen_targets`. (Type/representation of an expression.)
- **test/narrow**: `is_tests`, `predicate_tests`, `comparisons`, `pick_left`,
  `safe_strip`, `elvis_picks`, `for_drivers`.
- **interp**: `interp_to_str`, `interp_union`, `interp_implicit`.
- **use/spawn**: `use_effects`, `use_deps`, `use_with_items`, `use_addrs`,
  `use_handler_args`, `spawn_effects`, `spawn_deps`, `spawn_dep_items`,
  `addr_calls`, `route_stubs`, `facade_sends`, `self_sends`, `replyto_members`,
  `replyto_tasks`, `task_mint_effects`.
- **binding/place**: `binding_modes`, `handle_muts`, `virtual_place_binds`,
  `linear_moves`, `state_takes`, `fate_reads`.
- **refs**: `fn_refs`, `def_refs`, `field_refs`, `module_refs` (also read by the
  LSP — check consumers before moving).
- **lambda**: `lambda_effects`, `lambda_captures`, `lambda_contracts`.

Not clustered / out of scope: `FnKey`-keyed (`fn_effects`, `deductions`,
`fn_lends`, `param_mutations`, `iter_returns`, `implicit_params`,
`established_quals`, `carried_identities`), whole-program sets, deadlock
edge-lists, `errors`, `refinements`, `protocol_hashes`, the `written_types`/
`type_ref_*`/`visible_*` type-identity maps (keyed by address or name, read
widely incl. LSP — treat separately/last).

## Plan (sequence of commits, goldens green at each)

**Phase 0 — groundwork.**
- [ ] Add a `node_facts` read helper on the IR builder that, given a span,
      returns the cluster record(s) it needs — but implement it over the
      *existing* tables first, so the builder's call sites change shape once
      and the storage migrates underneath them without re-touching the builder.
      (This is the one useful sliver of Option 2: a stable builder-facing
      surface, introduced only because it makes Phases 1–N touch the builder
      once instead of per-cluster.)
- [ ] Decide the cluster struct home: `crates/salvo-core/src/check.rs` beside
      `Checked`, or a new `check/facts.rs` module. Lean: a `facts.rs`.

**Phases 1–N — migrate one cluster at a time (call cluster first).**
For each cluster C:
- [ ] Define `struct CFacts { field: Option<...>, … }` (`Clone, Default`).
- [ ] Replace the N `Checked` fields with one `HashMap<Key, CFacts>` (or keep
      `HashSet`-like facts as a `bool` field).
- [ ] Update the ~writer sites: `out.<x>.insert(k, v)` →
      `out.c.entry(k).or_default().<x> = Some(v)`. Grep each table name.
- [ ] Update the builder reads to go through the Phase-0 helper / read `CFacts`.
- [ ] `cargo test -p salvo-ir` + `SALVO_SKIP_E2E=1 cargo test -p salvo-cli`
      green; then the corpus/golden e2e. Commit.

**Phase F — inline into nodes (the Option-4 finish).**
Once the handoff is clustered, fold each cluster into the IR node it attributes,
so the builder writes the fact onto the node directly and the cluster table
disappears. Many node fields already exist (`Call.target`, `MakeUnion.arm`);
this phase adds the ones still looked up adjacent to emission and removes the
intermediate record. Reassess scope here — some facts may be fine to leave as a
small builder-local handoff if inlining them bloats every node.

## Work in progress

**Phase 0 done (2026-10-10): the call-cluster read surface.** `Checked` now has
`call_info(key: Key) -> CallInfo<'_>` (`crates/salvo-core/src/check.rs`), a
borrowing view over the call-cluster tables, and the IR builder's `call` method
(`crates/salvo-ir/src/build/body/exprs.rs`) reads it in **one** lookup instead
of ~13 scattered `.get(&self.key(span))`s. Pure refactor: IR goldens + both
codegen corpus tests unchanged, all salvo-core tests green. This establishes the
stable builder-facing surface so the storage migration (Phase 1) can collapse
the tables underneath `call_info` without re-touching the builder.

**Next action: Phase 1 — migrate call-cluster STORAGE.** Replace the ~15
separate `Checked` fields the call cluster covers (`call_fn`, `call_type_args`,
`call_effects`, `may_throw`, `addr_calls`, `self_sends`, `facade_sends`,
`dot_calls`, `member_calls`, `effect_calls`, `local_calls`,
`local_call_names`, `implicit_args`?, `fn_value_calls`, `moved_projections`)
with a single `HashMap<Key, CallFacts>` (+ a `Clone, Default` `CallFacts`
struct of `Option`/`bool` fields). Rewrite `call_info` to read the one map, and
update every checker writer site (grep each field name for `.insert`/direct
assignment) to `out.call_facts.entry(k).or_default().<f> = …`. Watch:
- `implicit_args` and `effect_calls` are read by *other* lowerings too (member
  calls, set/map literals, struct lits) and by non-call checker sites — decide
  whether they belong in the call cluster or a shared cluster; may be cleaner to
  leave `implicit_args` out of CallFacts for now (it is read at many spans).
- `moved_projections`/`dot_calls`/`local_calls` are `HashSet<Key>` → become
  `bool` fields.
- After the storage swap, `cargo test -p salvo-ir` (goldens) + the corpus e2e
  must still be unchanged.

Then repeat per cluster (expr-type, test/narrow, interp, use/spawn,
binding/place, refs, lambda), then Phase F (inline into IR nodes).

### Log
- 2026-10-10 — Doc created. Finding recorded: backends already read only the
  IR; this is a checker→builder handoff refactor. Option 2 skipped. Clusters
  drafted. Not started in code.
- 2026-10-10 — Phase 0 done: `Checked::call_info` + `CallInfo` view, builder's
  `call` reads it in one lookup. Goldens/corpus/core tests green. Committed.
  `[unify-checker]` is the label (implementation, not a language rule).
- 2026-10-10 — Correction recorded (see section above): storage-collapse is the
  wrong move; the work is view accessors per builder read-cluster, tables left
  as checker state.
- 2026-10-10 — `use_info` + `spawn_info` views added; builder's `use_stmt`,
  `Spawn` and `ReplyTo` lowering read them in one lookup each. IR corpus green.
  Committed.
- 2026-10-10 — `pick_info` (qualifier-pick/elvis: `is_tests`/`elvis_picks`/
  `pick_left`/`predicate_tests`) and `interp_info` (`interp_implicit`/
  `interp_union`/`interp_to_str`) views added; builder reads each in one lookup.
  Corpus + core green. Committed. **This completes the view-accessor work: every
  builder site that read ≥3 checker tables at one span now reads one view
  (call, use, spawn/reply, pick, interp).**

### Status: DONE (view-accessor form)

The valuable, safe unification is complete. Five view accessors on `Checked`
(`call_info`, `use_info`, `spawn_info`, `pick_info`, `interp_info`) replace the
multi-table lookups the IR builder did per node. The remaining `checked.X.get`
reads in the builder are **single-table per site** (one construct reads one
table — `comparisons` at an operator, `for_drivers` at a `for`, `promotions` at
a widen, `is_tests` alone at an `is`/`when`, `expr_ty`/`repr_ty` via the
existing `ty_of`/`repr_of` accessors, the binding/place sets); a view there
would not reduce "multiple places" because there is only one. No `Checked`
field was removed and no checker writer site changed — the tables remain sound
checker state; only the builder's read surface was unified.

**Not pursued (recorded):** collapsing the tables' storage into cluster maps
(fights the checker's own many readers of `call_fn` etc.) and merging the
checker+builder into one pass to eliminate the span-keyed handoff entirely (a
massive rewrite with no backend-visible benefit, since the IR is already
self-contained and the tables are builder-private). See the "Correction"
section.

**When retiring this doc:** the five accessors are the whole deliverable; fold
this plan's reasoning into COMPLETED.md's decision log and delete the file, per
the design-doc convention. ROADMAP §13(b) can then note the builder reads
clustered views and the deeper table-elimination was judged not worth it.
