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

_Nothing started yet. Next action: Phase 0 — the builder-facing `node_facts`
helper + pick the `facts.rs` home, then migrate the **call** cluster._

### Log
- 2026-10-10 — Doc created. Finding recorded: backends already read only the
  IR; this is a checker→builder handoff refactor. Option 2 skipped. Clusters
  drafted. Not started in code.
