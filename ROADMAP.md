# Salvo Compiler — Roadmap

What is **left** to build. Everything finished — the milestones, the four
completed sequences, the options explored and abandoned, and every closed defect
with its repro — is in [COMPLETED.md](COMPLETED.md); this document assumes it and
points into it rather than repeating it.

Consolidated 2026-09-26 (user request): the finished sections that used to fill
this file are gone, their reasoning already recorded in COMPLETED.md's decision
log, and what remains is **one sequence**.

Companion documents: [docs/language/](docs/language/) is the narrative spec
(source of truth); [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md) states every feature as a
labeled rule (`[qual-erasure]` style) with the compiler decisions under it;
`BACKEND_SPEC.<backend>.md` ([kotlin](BACKEND_SPEC.kotlin.md),
[rust](BACKEND_SPEC.rust.md)) repeats rules with backend interpretation and adds
backend-prefixed rules — load one only when working on that backend. Labels are
referenced from code and tests (`grep -rn '[rule-name]'`).

## How to read this

- **The sequence is the order of work.** Take the next unblocked item; nothing
  below step 1 is blocked on anything above it except where it says so.
- **A `DECISION` needs a language-design call before implementation**, and that
  call is the user's: state the options, the trade-offs and a recommendation,
  then wait (AGENTS.md's first invariant).
- **Standing constraints on every item.** Unsupported constructs are *errors*,
  never wrong output [backend-never-wrong]. The two backends must agree
  observably; a divergence is closed by restriction or by faithful emission.
  Backwards compatibility is not a requirement, so a rule change is a sweep of
  every example rather than a shim.
- **Before starting anything**: read COMPLETED.md's "Gotchas / lessons learned"
  for traps in the area you are touching, and its decision log for whether the
  question was already answered.

## Where we are

Everything the four sequences covered is built and running on **both backends
with identical output**: shared fate and borrow emission, linearity with a
designated `close`, effects through handler dependencies, `throw`/`try`, places
and field narrowing, deductions with refinements, the iterator reduction to
`next` (with any-name `iter fn`, `Iter` for sources and the `iter T`
placeholder — the 2026-09-27 redesign), the collections, the filesystem, actors (spawn, send, park, watch,
bridge), time, free concurrency, shareable-by-default handlers, refinement types,
group borrowing, the testing framework, the comparison/hashing capabilities, and
the wire under actors across machines (`net`, step ① of the network sequence).
Ten worked examples in `examples/` carry the checked-in generated code for both
targets and the output they print. 1615 tests green.

## The sequence

### 1 — ✅ The std reorganisation (complete 2026-09-26)

One theme: std's shape. Step (a) landed with the sitting that scheduled the rest,
and each remaining step is mechanical but wide — the sweeps are the cost, not the
design.

- **(a) ✅ `Checked<T>` in `core`, and a list write answers one** — built
  2026-09-26 [checked-type] [col-bounds] (COMPLETED.md's log). `take` is
  `detach`; the fallible `swap` hands back a `Checked<Bool>`.
- **(b) ✅ The filesystem on `Checked<T>`** — built 2026-09-26 [fs-surface]
  [checked-type] (COMPLETED.md's log). `Err Checked<FsError>` throughout; the
  bespoke wrapper, its `ignore`/`detach` and its `to_str` are gone.
- **(c) ✅ The filesystem out of `core`** — built 2026-09-26 [fs-module]
  [fs-host-split] (COMPLETED.md's log). Modules `fs`, `fs.host`, `fs.mem`,
  `fs.restricted`, unprefixed (user decision: (B1), with step 8's `std.` prefix
  left as its own slice). `DefaultFs` stays in `fs.host` rather than the surface
  module — the fusion gate reads reachable modules, so a `fs.mem` test would
  otherwise be fused by a handler it never registers.
- **(d) ✅ `core.nonempty` dissolves into the collections** — built 2026-09-26
  [qual-overload] (COMPLETED.md's log). Each `NonEmpty` sits beside its
  container; the four ordered accessors delegate through a `rename fn`, which is
  what let the module go. The recorded "`NonEmpty` constructor convention — the
  siblings" item is now *unblocked* rather than closed: `set_of(first, ...rest)
  -> +NonEmpty Set<T>` siblings are writable where they were not, and whether
  std should have them is an API question.
- **(e) ✅ `throw` out of `core`** — built 2026-09-26 [throw]
  [test-implicit-import] (COMPLETED.md's log). `import throw`, and a test annex
  gets it implicitly because the harness puts the effect there. Two defects fell
  out of the module's name: Kotlin keyword package segments now mangle
  [kt-package-keyword], and a duplicate emitted path is refused rather than
  clobbered [backend-companion].

**Step 1 is complete.**

### 2 — ✅ Actors across machines: the network sequence (complete 2026-09-27)

Eight steps, sixteen user decisions (2026-09-26), all built: COMPLETED.md's log
has one entry per step ("The network sequence, step ①" … "step ⑧") and the
round's entry ("Actors across machines — the network round"). What the
sequence left behind, by step — each a leftover, none a blocker:

- **① wire/transport**: `delay(node, d)` on `MemNetwork`; a `.test.sv` annex
  for `net` (test bodies have no `spawn`); the multi-face stateful monitor gap,
  which is why the double is two pieces.
- **② codecs**: keyed containers have no wire form; a generic `T` is refused at
  `encode`; Kotlin tuples past `Triple` have no codec.
- **③ routable addrs**: several remote senders may over-subscribe a mailbox by
  one each (the initial grant is "at least one"); a malformed remote answer is
  dropped silently.
- **④ node groups**: the partition/unreachable policy for gossip (a failed
  `deliver` → `left(n, "unreachable")`); the shared secret/TLS half of N-6 at
  the handshake; `HeartbeatNodeGroup` over a `Ddb` platform effect as the
  interop example.
- **⑥ `any`**: an `Addr<E>` answered by *spawning* a router binds as an
  ordinary `E` under `use addr` (the addr type has no room for the claim); a
  lambda's body is checked under strong availabilities; `local`/`any` on a fn
  type is reported once per lowering of the type (three times for a
  parameter), a pre-existing duplication.
- **⑦ picks**: consistent hashing for `Sharded` (today `key mod n` over the
  ordered view, so a join reshuffles most keys); a `[keyed E]` claim for a
  program relying on `Sharded`'s per-key ordering; a stub for a group whose
  replica is on another node (empty view, the stub parks); the stub is
  generated only where the module spells the protocol; two instances of an
  erased effect (`Pick<A>`, `Pick<B>`) cannot share a scope — a function per
  policy is the pattern, and lifting it means keeping a phantom on the erased
  trait plus a marker type per effect on the Rust side.
- **the tidy-up (2026-09-27)**: a node group connecting the node itself
  (the mechanism asserts `connected()` instead) waits on an actor member
  being able to hand an inherited effect to a spawn on the Rust backend
  [rs-handle-bundle]; the in-process double's second node spawns its wire
  actors `with` the transport meanwhile. A mixed handler has no `init`
  block yet. The mixed servant / private-member unification
  (below) is unchanged.
- **⑧ the example**: `examples/cluster/` ships an election by host order behind
  `Leader`, not Raft. **Raft as a flagship example is still to be written** —
  terms, votes, heartbeats over `Timer`, a replicated log — and wants a
  deterministic clock across nodes first (virtual time is per process today,
  and messages between virtual nodes run on other threads). N-10's user-facing
  half — the manifest's `version` plus a lock file, so a protocol change
  without a bump fails the build — **waits on section 4's manifest DECISION**
  (that section notes it as its second customer).

Both groups stay **actors until the sugar pass** (section 13), which thereby
gains two concrete targets: `members()` as a plain read (the answering stub)
and `replyto` onto another actor's member (the remote mint). Recorded gap: the
deadlock graph is per program, so a wait cycle closing through a handler in
*another* program of the same node group is invisible — the same shape as the
"over types, not instances" gap, same deferred remedy.

### 2b — One shape for effects: handles everywhere, fusion removed, `local` removed (user decisions 2026-09-28; **next**)

**The decision** (user, 2026-09-28, after reading EFFECT_FUSION.md): while
the language is still being designed, a *uniform* emission with fewer edge
cases is worth a performance hit; smarter per-case emissions are a later
optimisation pass. Three calls, all the user's:

1. **Every binding is a handle** (EFFECT_FUSION.md §4, Option 1). A `use`
   makes one value, `__Handle_E`, and everything downstream — a fn's effect
   parameter, a dependent handler's dependency, a spawn's or task's capture,
   a fn value's effect — is that value or a clone of it. The three Rust
   shapes for a dependent handler (owned handles / `__Impl_H` fusion form /
   actor provider) become one: the handler holds a `__Handle_E` field per
   dependency, whether it is `use`d or spawned.
2. **No fusion, on either backend.** A fn declaring `[A, B]` takes one
   parameter **per effect, in declaration order** — the "plain mode" shape
   Rust already has, typed by handles — not one fused value. Fusion's two
   reasons (a dependent handler's members had no room for their dependencies;
   two faces of one handler bound to a fn declaring both is E0499) are both
   answered by handles (a field; two clones of one `Arc`). The user likes
   the fused syntax but chose the simpler, more legible output.
3. **`local` is removed** — `use local H()` and `[local E]` — in the same
   sequence. With one shape it selects nothing: every binding is shareable
   and every dependency is captured. A lock-free scope-local binding returns
   as a ROADMAP item when the shapes have settled (see "Recorded, not
   scheduled").

Two consequences the user accepted as decisions: a **stateful handler
captured by two spawns is now shared state** (an `Arc<Mutex<H>>` cloned
twice — Kotlin's behaviour already, since references alias; today the
checker refuses the capture of a local binding, and that refusal goes); and
**interception** (`handler Stamped [Logger] of Logger`) holds a handle to
the previous Logger rather than borrowing the scope — same behaviour, one
mechanism.

**The handle**, per backend. Chosen by the handler's *statefulness*, which
is visible in its declaration (`state` fields; a platform handler without
`threadsafe` counts as stateful):

| handler | Rust `__Handle_E` holds | Kotlin |
|---|---|---|
| stateless, or `threadsafe platform` | `Arc<H>` behind `Box<dyn __Share_E>`; members `&self` for a threadsafe host, else a clone per call is *not* done — the handler is called through `&mut` on a `Mutex`-free path only when stateless is provable: **decide** whether stateless handlers use `Arc<H>` + interior-mutability-free `&mut` via `Arc::get_mut` fallback, or simply share the `Mutex` path too (simplest; uncontended lock) | the instance itself |
| stateful | `Arc<Mutex<H>>` (today's `__Lock_E`) | `__Mon_E` (`synchronized`) |

**Decided (user, 2026-09-28)**: one implementation — `Arc<Mutex<H>>` for
*every* handler, stateless included (an uncontended lock per member call),
with the lock-free stateless path deferred to the optimisation pass. So
`__Handle_E` is one struct per effect with one impl; `__Share_E`, `__Lock_E`
and `__Arc_H` are all deleted. Kotlin does the same for parity: every `use`
wraps in `__Mon_E` (`synchronized`), stateless or not — harmless, and it
keeps "one shape" true on both backends. Two consequences of "one impl":

- **`threadsafe platform handler`** becomes emission-neutral (the host is
  behind the same mutex as everything else; the `&self` twin trait
  `__Shared_H` and the `__Arc_H` adapter go). **Decided (user, 2026-09-28)**:
  keep the word as the declared contract `salvo platform generate` prints,
  emit nothing for it; it earns its keep in the lock-free pass.
- **A handler holding a function value** (`handler Derived(step: (n: Int) ->
  Int) of Random`) is today refused for a shareable binding — a Rust
  `Box<dyn FnMut>` is not `Send` — and is `use local`-only. With `local`
  gone it could not be bound at all. **Decided (user, 2026-09-28)**: fn-typed
  constructor parameters and state fields emit as `Box<dyn FnMut(…) + Send>`
  (Salvo lambdas capture by value, so the bound holds), and the "holds a
  function value" arm of `unsendable_reason` is deleted. Stored lambdas then
  work everywhere, on both backends.

**The one thing a handle cannot do**: a member that lends `&mut` into the
handler's state ([rs-loc] "wholesale-lending", `fn_type_lends_mut`) cannot
be behind a `Mutex` guard. Today such a handler is `use local`-only; with
`local` gone it is a **checker error** at the declaration ("a member lends
into the handler's state, which a handle cannot give — answer a copy, or
take a closure"). Surveyed 2026-09-28: **no std effect has such a member**
(`Fs`, `Console`, `Clock`, `Ticker`, `Random`, `Transport`, `Pick`, `Leader`,
`RawFs`, `Throw` — none lends), so this only ever affects user code, and the
rustc E0515 the emitter special-cased becomes a Salvo diagnostic.

**The sequence** (commit after each step, short messages; full
`cargo test` warm and `SALVO_E2E_FRESH=1 cargo nextest run` before each
commit; regenerate `examples/*/{rust,kotlin}` whenever emission changes):

① **Remove `local`.** Parser: `use local` and `[local E]` become parse
   errors naming the removal (no dual acceptance — the language just stops
   having it); `EffectRef::LocalEffect` deleted; `Stmt::Use.local` deleted;
   the `local`/`any` "do not combine" check goes with it; tm-grammar entries
   in `crates/salvo-cli/src/lang.rs` removed and regenerated. Checker:
   `UseKind` collapses to one (or is deleted: every `use` is a handle),
   `classify_shareable_use` and its blockers deleted except the new
   lending-member refusal, `EffectAvail.local` deleted, the call-site
   "requires a shareable E" rule deleted, the spawn-capture-of-local refusal
   deleted, `handler_handle_deps` deleted (every dependent handler holds
   handles), `check_local_send_member`'s "pins the fusion form" comments
   cleaned; `require_handle`/`Checked.handle_requirements` (the checker half
   of handle bundles, 12 sites) deleted. **Order caution**: `salvo_core::
   handler_handle_deps` is called by *both* emitters (Kotlin ×4) — delete it
   with its last caller in step ③, or delete every caller in this step.
   Parser AST snapshots change wholesale (`Stmt::Use.local` is gone). std: the 16 `[local E]` / `use local` sites (`fs.sv` ×14,
   `console.sv`, `time.sv`) become bare. Tests: ~90 fixture sites
   (`codegen_tests.rs` ×60, `monitor_tests.rs` ×18, others); the
   `a_local_binding_satisfies_only_local_requirements` family and the
   "cannot be bound shareable" tests are deleted, not rewritten.
   `examples/linearity/` has two sites. Docs: `Effects-and-Handlers.md`
   "Shareable by default" section rewritten as "Every binding is a handle";
   `[effect-local]`/`[use-local]` in LANGUAGE_SPEC.md become one paragraph
   pointing at the record; the label references in code (`grep -rn
   '\[use-local\]\|\[effect-local\]'` — ~60) retargeted to the new rule
   `[effect-handle]`. *This step can land with the emitters unchanged*: the
   Rust emitter already handles `UseKind::Bare`/`Monitor`; the only emission
   that disappears is `Local`'s.

② **Rust: handles everywhere, fusion deleted.** In `crates/salvo-backend-
   rust/src/emit.rs`: delete `program_needs_fusion` and the `fusion` flag —
   the plain-mode code path becomes the only one, with `&mut dyn E`
   parameters replaced by `&mut __Handle_E`; delete `emit_fusion_inner`,
   `emit_fusion_instance`, `emit_forward_impl`, `emit_deps_adapter`,
   `emit_dependent_members` (`__Impl_H`), `__Deps_H`, `prov_trait`,
   `__FxDyn`, `handle_bundle_param`/`handle_bundle_arg` and
   `Checked.handle_requirements`, the actor `__Prov_H` provider (the actor
   body holds `handler: H` whose fields are the handles — `__dispatch`
   calls `self.handler.k(args)` for every handler, dependent or not), the
   `__Has_E` traits and `emit_has_impl`. A dependent handler is emitted as
   today's shape A for every handler: `struct H { __dep_E: __Handle_E, … }`,
   `new(…, __dep_E)`, members read `self.__dep_E.member(…)`. A `use` is
   `let mut e = __Handle_E::new(H::new(args, deps…))` with the deps cloned
   from the handles in scope. A fn declaring `[A, B]` is `fn f(a: &mut
   __Handle_A, b: &mut __Handle_B, …)`; a call passes `&mut a, &mut b`
   (locals) or `&mut self.__dep_A` (inside a handler). A fn value with
   effects is `impl FnMut(&mut __Handle_A, …)`. A spawn clones the handles
   into `H::new`; a task's closure captures clones; `replyto` unchanged.
   Goldens: all five Rust goldens change; accept after reading the diff.
   `BACKEND_SPEC.rust.md`: `[rs-effect-fusion]`, `[rs-monitor]`,
   `[rs-handle-bundle]` replaced by one `[rs-handle]` section. **Also in
   this step**: `Addr<E>` for a *plain* effect (a monitor spawn
   [monitor-handler]) *is* `__Handle_E` — the spawn just answers the
   handle, and `use addr` binds it; the `[rs-loc]` locator faces on
   *effect members* (`__loc` twins, `mark_loc_forwards` for members) go
   with the lending-member refusal — the fn-level locator variant stays;
   `[effect-intercept]` gains the emission-order sentence (the previous
   registration's handle is minted *before* the interceptor is constructed,
   which is "binds strictly outward" in emission). **Re-examine, possibly
   lift**: the "one scope, one instance of an erased effect" refusal
   (`Pick<A>` beside `Pick<B>`, [effect-generic-decl]) existed because two
   `__Has_Pick` impls collided on one fusion struct; with per-effect
   parameters they are two locals of one type, resolved by instance, so the
   refusal is probably unnecessary — test it, and if it lifts, restore
   `examples/cluster/`'s `two_ids`/`shop` to inline blocks and delete the
   `two_erased_instances_cannot_share_a_scope` test.

③ **Kotlin: fusion deleted, per-effect parameters.** The gate and
   `fx.kt` go; a fn declaring `[A, B]` is `fun f(a: A, b: B, …)` (the JVM
   reference is the handle; `__Mon_E` wraps a stateful one exactly as now);
   a dependent handler takes one constructor argument per dependency typed
   by the effect interface (`class Stamped(private val __dep_Logger: Logger,
   …)`), the `<__Fx>` carrier type parameter goes, `__Actor_H` is no longer
   generic; every `use` wraps in `__Mon_E`; `handler_handle_deps` and its
   callers deleted here if not in ①. `[kt-effect-fusion]` replaced by
   `[kt-handle]`. Both backends'
   output for `EFFECT_FUSION.md`'s examples should now read the same modulo
   syntax.

④ **Records.** COMPLETED.md: the log entry (this decision, what it took,
   the lending-member refusal, the shared-state consequence), the test
   count, gotchas (the ones that fall out of deleting ~3,000 lines will be
   about what depended on fusion unexpectedly). `EFFECT_FUSION.md` rewritten
   from "three shapes and four options" to "the one shape", keeping §4 as a
   short history of why. LANGUAGE_SPEC.md: `[effect-handle]` stated once,
   `[effect-handler-deps]`/`[spawn-inherit]`/`[with-clause]`/
   `[monitor-handler]` sub-bullets that mention capture rules simplified.
   `docs/language/Effects-and-Handlers.md` and `Concurrency.md` swept for
   `local`, "fusion", "monitor spawn" wording. README feature bullet.
   `tools/sync-wiki.sh`. Delete this section; add to "Recorded, not
   scheduled": *lock-free scope-local bindings* (the optimisation pass:
   `Rc<RefCell<H>>` or a borrow for a binding no spawn captures — the
   checker already knows which bindings are captured, `handle_captures`),
   and *lock-free stateless handlers* if the recommendation above was taken.

**Size**: two to three days. Step ① is the largest sweep but mechanical;
step ② is the deletion of most of the fusion machinery and the rewrite of
`emit_use`/`emit_spawn`/fn signatures around one handle type; step ③ is
small. The tests that matter most are the compile-and-run cases on both
backends (`kotlinc_compiles_and_runs_every_case` and its Rust twins), which
exercise every shape: interception (`effects/` example), multi-face
handlers, monitors, dependent actors (`Counting [Clock]`), tasks capturing
effects, `init` on a dependent handler, the whole of `net`.

**What a new agent should read first**: this section; EFFECT_FUSION.md
(the shapes being removed, with real output); COMPLETED.md's 2026-09-20
"shareable by default" entry and its gotchas (what `local` was for, so its
removal is understood); `[rs-effect-fusion]` and `[rs-monitor]` in
BACKEND_SPEC.rust.md; `[kt-effect-fusion]` in BACKEND_SPEC.kotlin.md.

### 3 — Recorded: the restrictive reading of a fn-typed slot's lend

Both defects of the 2026-09-25 round are closed — a bare generic struct literal
now determines its type arguments [struct-literal-arg], and a fn-typed slot whose
return is opaque now **infers** its lend [proj-infer-fn-type] (both 2026-09-26,
COMPLETED.md's log).

What the second one left open, deliberately, is the reading it chose *against*:

- **A slot could state that its fn does *not* lend.** `(c: C) -> Mut It` would
  then mean "the iterator must not borrow the container", and implicit resolution
  would refuse `core.list`'s borrowing `iter` for it — naming the candidate and
  `holds proj(c)` — while accepting `core.set`'s snapshotting one. The machinery
  is already there: resolution **already** compares a candidate's contract against
  a slot's and refuses a mismatch (probed 2026-09-26: a keep/consume mismatch is
  reported with both remedies named), so adding the lend to that comparison is
  small.
- **Why it was not taken**: the restrictive reading buys nothing observable
  today. The two things it would protect — mutating or consuming the container
  while the iterator is live, and the iterator outliving the container — are both accepted
  under *either* reading (probed), and in a generic body there is nothing an
  opaque `C` can be done to anyway. So the restriction would have cost an
  annotation on the language's most ordinary generic-iteration signature to
  express a property nothing yet depends on.
- **The trigger for revisiting**: a program that *needs* an iterator independent of
  its container — one that stores it, returns it past the container's life, or
  sends it to another thread ([actor-sendable] refuses a borrowing iterator, so a
  task or actor taking an iterator is the likely first customer). When that appears,
  the restrictive slot becomes worth stating, and the shape is above. Note the
  matching must be **directional**: a lending slot still accepts a non-lending
  candidate, so it is subtyping on the contract rather than equality.
- ✅ The spare clause **warns** (user decision 2026-09-26, built): writing
  `holds proj(c)` on an opaque-returning slot says what the signature infers, so
  it is reported the way a no-op `@place` selector is [fn-overload-at] — one
  diagnostic per clause, naming every source, and silent where the return is
  concrete.

### 4 — Project manifest and LSP source-root discovery (DECISION, then build)

**The defect**: editing std with the *repository root* as the editor's workspace
folder produces ~750 lines of spurious diagnostics, because the LSP takes the
client's `rootUri` as the analysis root and the repo's independent trees (`std/`,
`examples/*/salvo/`, `demo/`, test corpora) are then analyzed as one program. The
reported symptom was at `std/core/list.sv:86` — a `preserve Idx` promise
apparently ignored — and it is collateral: under the repo root the on-disk file
classifies as module `std.core.list` while the embedded copy is `core.list`, so
[std-shadow] misses, both copies load, and the duplicated `swap` makes the
refinements refuse to attach. Repro without an editor:

```bash
cargo run -- analyze --src .      # from the repo root: errors in std
cargo run -- analyze --src std    # the correct root: clean
```

The **workaround** meanwhile: open `std/` as its own workspace folder.

**The decided direction** (user, 2026-09-24): per-document source-root discovery
in the LSP, anchored by a **project manifest** — option (b) of that round, chosen
over widening [std-shadow] to strip a leading `std/` (fixes only this case, and
silently re-classifies a user's own `std/` tree) and over documenting the
workaround alone.

**DECISION — the manifest's shape**: what the file is called, what it may state
(source root certainly; backend, main and target dir are the obvious candidates,
each a CLI flag today), whether `run`/`compile`/`test` read it too (they should,
or the LSP and the CLI disagree about what a project is), and what root discovery
does with no manifest in sight (fall back to `rootUri`, today's behaviour).
**A second customer since 2026-09-26**: the network sequence's version label
(section 2, step 8) wants a `version` field in the manifest plus a lock file the
build maintains — effect → (declared version, protocol hash) — so a protocol
change without a version bump fails the build. The manifest's shape should be
decided with that in view.

### 5 — Consistency passes the 2026-09-26 ambiguity round left

Three narrower questions, all downstream of "refuse to choose" (COMPLETED.md's
log for the round itself).

- **✅ Identical-signature shadowing stays an ambiguity** — confirmed
  2026-09-26 (user decision, COMPLETED.md's log). Nothing to build. What came
  out of confirming it is a **small new question**: the remedy is
  `f@<this module's path>(…)`, and there is no shorthand — `@mod` does not
  exist, and `@mod`/`@import` as *rung* keywords were rejected 2026-09-07
  because they ask the reader to know which rung a name arrived on. A `@mod`
  (or `@self`) meaning **"this file's module"** is narrower than what was
  rejected: one fixed meaning, no rung to know, and it stays short where a
  module path does not (`size@orders.pricing(xs)`). Options: leave it (the path
  is explicit and greppable, and a call needing the selector is rare by
  design); add `@mod`; or add `@self`, which already means "this handler" for a
  send [actor-self-send] and would then mean two things. Recommendation: leave
  it until a real program reads badly — the trigger is narrow enough that the
  evidence may never arrive.
- **Implicit resolution still resolves by rung**, deliberately: an implicit has no
  written call site to annotate, and two same-named types have no distinguishing
  selector at all. If that is to change it needs a spelling first.
- **The rest of the resolution-by-position rules.** The round covered calls and
  refinements. A pass over the spec should list every remaining
  resolution-by-position rule and decide each — separating *silent winners* from
  *designed shadowing* (a later `use` shadowing an earlier handler is interception
  semantics [effect-intercept], and a fn-typed local shadowing a name outright is
  the caller's explicit choice).

### 6 — Recursive implicit resolution, so a tuple can have a `cmp`

[col-hashed-ordered] says "a `List` or a tuple qualifies exactly when its elements
do, comparing lexicographically", and that is true of the two *backends* rather
than of the language: `core.compare` declares `cmp` for the intrinsic scalars
only. It cannot declare one for a tuple, because
`cmp<A, B>(a: (A, B), b: (A, B)) -> Int` needs `?Ordered<A>, ?Ordered<B>` and
[implicit-resolve] **skips a candidate that itself needs implicits**.

This is the accepted limitation behind the keyed containers: a keyed container over
a **tuple or list** is refused by name (user decision 2026-09-26 — "I'm ok with the
limitation today"). Lifting it has two halves, and the second is the larger:

1. **Resolution** — `resolve_implicit_fn_at`'s one-line skip becomes a recursive
   resolution with a depth cap and a cycle refusal. Contained.
2. **Emission** — a filled implicit that *itself* needs implicits has to be handed
   its own, so `ImplicitArg::Resolved` needs nested arguments and both backends'
   adapter closures have to pass them (`cmp((A, B))` calling `cmp(A)`/`cmp(B)`).
   `implicit_args` records no nesting today, so this is where the work is.

The cheaper alternative, with its cost stated: declare the tuple and `List<T>`
`cmp`/`eq`/`hash` as **intrinsics**, which is what the backends already do
structurally. No recursion needed, and it *documents* the status quo — but it
freezes it: an element type's own declared identity would be ignored inside a
tuple or list key. That is already true; declaring it makes it look intended.

Two recorded items wait on the same lift: `expect_eq` on a generic container
cannot resolve a `to_str` [interp-to-str], and property testing's `?generate`
(step 8) needs it.

### 7 — Qualifiers are droppable, then variance

- **Qualifiers are droppable on assignment** (user decision 2026-09-23, not
  built). A variable's type may never *widen*, but a qualifier is by definition
  something that optionally applies, so dropping one is always legal: assigning a
  plain `List<Int>` to a variable inferred as `NonEmpty List<Int>` must be
  accepted, and the variable simply stops being `NonEmpty` (the flow state already
  models exactly this — a mutating call drops claims the same way). Provenance
  qualifiers need thought: dropping one is harmless, re-*gaining* it must stay
  impossible. Where it shows up today: `analyze_tests`'
  `fate_links_merge_across_branches` and `inferred_moves_consume_arguments` had to
  annotate their `let`s when the constructors started claiming, and those
  annotations come back out.
- **An inferred type argument is not widened by the expected type** — found
  while building step 1(b), and the same invariance in a narrower place.
  `checked<T>(value: T)` binds `T` from the argument, so
  `checked(NotFound { … })` is a `Checked<NotFound>` and the expected
  `Checked<FsError>` does not widen it — not through a `let` annotation either
  (probed). std therefore names the argument at all 35 construction sites,
  `checked<FsError>(NotFound { … })`, which is explicit and costs a word. The
  declared-position widening that the old `FsError { kind: … }` relied on still
  works; what does not is *seeding a type variable* from the expectation. The
  keyed-container work seeds a callee's type parameters from the expected type
  already, so the machinery is there — the question is whether an expectation
  may pick a **supertype** for an inferred argument, which is the same question
  variance asks and should be answered with it. **A small DECISION**: widen
  (unify against the expectation's arms, so `checked(NotFound { … })` at a
  `Checked<FsError>` position infers `FsError`), or leave the explicit form as
  the one way and keep this recorded as intended.
- **DECISION — variance on generic parameters** (user direction 2026-09-23). A
  `List<NonEmpty List<Int>>` is not a `List<List<Int>>` today, so a constructor
  call cannot fill a plain type-argument position. The direction is `in`/`out`
  declaration-site variance as C# and Kotlin have it. What has to be decided with
  it: declaration-site or use-site (recommendation: declaration-site first,
  because std's containers are where it pays); what `Mut` does to it (a
  `Mut List<T>` cannot be covariant in `T` — the classic array-store hole — so the
  natural rule is that covariance holds only while the value is not `Mut`);
  whether qualifiers on a type argument are a **separate, narrower** rule worth
  pricing first (the case that raised this needs only `NonEmpty List<Int>` →
  `List<Int>` *inside* a type argument, which is qualifier-dropping at depth); and
  that the emitted Rust must not depend on it, since Rust has no variance.

### 8 — Mutating through a union arm (DECISION)

One shape is **refused on Rust and accepted on Kotlin**, which is a divergence
closed by restriction on one side and therefore a decision rather than a resting
place:

```
fn bump(o: Ok Mut List<Int> | Err Str) [] -> None {
    if o is ^Ok {
        add(o, 7)          // Kotlin: mutates the caller's list
    }                      // Rust: reported — `o` is read-only here
}
```

The parameter renders `&Union2<…>` because an arm's `Mut` is not a claim about
`o`, and **nothing can ask for the `&mut`**: a written `=> o: Mut` is refused
("a deduction may preserve or drop qualifiers, not add them"). So the question is
what a `Mut` arm means for the *parameter* that carries it.

- **(a) The arm's `Mut` makes the parameter mutable.** Cost: the Rust signature
  then disagrees with the checker's contract, which still says the parameter is a
  kept read — and two arguments naming the same place, which the checker permits
  as two reads, become two `&mut` borrows and an E0499 on a program Salvo
  accepted. Teaching the fate analysis about that is the real scope.
- **(b) Admit the deduction.** Relax "may not add qualifiers" so `=> o: Mut` is
  legal when `Mut` is present on an arm, and let inference write it from the body.
  Cost: a written deduction now means "through an arm", a new reading of the
  clause; benefit: the contract stays visible in the signature, which is what
  deductions are for.
- **(c) Keep the refusal, and refuse it in the *checker*** so both backends say
  the same thing. Cost: a Kotlin program stops compiling; benefit: one story, and
  the remedy (take the payload as its own `Mut List<T>` parameter) is one line.

**Recommendation: (b)**, with (c) as the fallback. (a) is the one to avoid: it
makes the two sides of the compiler disagree about what a signature means, which
is how the original defect happened.

### 9 — Testing, beyond the MVP (decided 2026-09-23, not built)

The framework's core is built and `salvo test` runs std's own suite. What was
deliberately cut, in the order the decisions put it:

- **`context` scopes** — decided in shape (TF-9): nesting plus per-test re-run
  initialization (`use`s and `let`s that run again for every test, so no
  cross-test state is expressible), with the test's id joining the context names.
  Three sub-calls ride the implementation: statements **preamble-only**
  (recommended, error otherwise), a preamble may do **anything a test body may**,
  and a **childless context warns**. The lowering is per-test inlining, which
  makes isolation true by construction.
- **Property testing** (TF-5): `check(runs, property)` with the generator arriving
  as the implicit **`?generate`**, so the *qualifier on the parameter type picks
  the generator* (`(text: ValidDate Str) -> …`); randomness threaded as a
  `Mut Rng` **value**, which the effect-free-resolution rule turns into a
  determinism guarantee; shrinking by replaying the generator over a shrunken draw
  stream. Two prerequisites: std needs wrapping/bit `Long` intrinsics for a
  pure-Salvo splitmix64 (parity by construction — a recommendation, not yet
  decided), and step 5's implicit lift.
- **Actor testing helpers** (TF-6): `settle(p)`/`expect_settled(p)` over
  [actor-on-idle], a recording `Probe<M>` handler, `expect_fault(target, body)`
  over [actor-watch]. Needs `spawn` added to [test-body]'s implicit powers, and
  the harness's default fault sink to record rather than print. No test scheduler
  (rejected as out of scale).
- **The blackbox tier** (TF-2's `tests/` tree): ordinary modules that `import`
  what they test and see only exports.
- **`--isolate`, `--timeout`, crash recovery** (TF-4): one process per test, a
  per-test wall-clock kill, automatic re-run of a crashed batch's remainder. The
  MVP runs everything in one process and reports a test that died (`DIED`).
- **The `std.` import prefix** (TF-8, decided 2026-09-19): `import std.time`
  rather than `import time`, `std` a reserved root, the bare spelling refused with
  the corrected path named, and the library tree moving to `lib/std`. Sequenced
  after the MVP deliberately. Step 1(c) shipped **without** the prefix (user
  decision 2026-09-26, option B1), so the filesystem's modules are `fs`,
  `fs.host`, `fs.mem`, `fs.restricted` and this step renames every top-level std
  module at once. What it drags, and why it is its own slice: module paths derive
  from the embed root, so moving the tree to `lib/std` renames `core.*` to
  `std.core.*` — five hardcoded `"core"` sites in `resolve.rs` deciding implicit
  visibility, `[std-shadow]`'s classification, the CLI embed root, and the
  emitted path of every std module in every golden and every example. **Whether
  `core` becomes `std.core` at all is the decision to take first**; the
  alternative examined in that round was a reserved `std` root mapping to the
  embedded library whatever the on-disk layout, leaving `core` unprefixed as a
  stated exception.
- **Smaller leftovers**: the annex's one-way visibility holds by construction but
  is not *checked*; the report has no `--format json` for editors; nothing
  migrates the compiler's own e2e suite onto `salvo test` (user decision: leave
  it).

### 10 — The assertion trap policy (A-6, decided 2026-09-23, not built)

Three failure classes still take the hosts' behaviour:

- **Subscript out of range** → trap with *our* message (index and length),
  replacing the hosts' two different texts. Needs care on the *place* path
  (`arr[i] = x`), where a block expression cannot stand.
- **Division by zero** → trap with our message; both hosts already trap, only the
  text differs.
- **Integer overflow** → **wrapping**, stated in the spec, with `checked_*` /
  `saturating_*` std functions for the cases that care. This is the JVM's
  behaviour today and the cheap one on Rust. **It is the only row that changes
  what existing programs compute**, so it wants its own slice and a parity test:
  today Kotlin wraps silently while Rust refuses a constant fold and panics in
  debug.

### 11 — One read, one mode: the rendering that reports a reference

Three slices landed 2026-09-23 [rs-read-mode]; what is left is the **refactor the
section is named after**. The slices work because their sites know the shape they
will get and can ask a predicate first (`owned_optional_local`,
`narrowed_borrow`). The general case cannot: a site that wants `&T` writes
`&{code}`, and if `{code}` is already a `&T` the result is `&&T`. The durable
answer is for the rendering to answer *what it produced* (`Rendered { code,
is_ref }`) so a site can decide whether to add the `&`.

The site waiting on it is **interpolation**, the most common read position in the
tree, which still clones every narrowed or `!`-ed value:

```rust
// std/test.sv's expect_trap_with, where the same `trap` two lines up borrows
format!("… got `{}`: {}", trap.as_ref().unwrap().clone(), label.clone())
```

`emit_interp_value` cannot simply switch to `emit_read`: its native branch would
take the reference happily (`format!("{}", &String)` displays), but the two
`to_str` branches in the same function write `to_str(&{arg})` and
`{place}.{field}` — three questions to the predicate in one function, which is
precisely the shape that wants the rendering to answer for itself.

Worth finishing because it is the last **systematic** copy nobody wrote,
`[copy-opt-in]` is a stated principle rather than an aspiration, and it is
measurable: `examples/*/rust/**` carries the clones, so the diff *is* the
benefit. Two recorded items are the same missing information in other clothes:
the temporary-subject `for` loop, and the copy an adapter closure makes of a
returned projection.

### 12 — Effect transformers (E3 step 4)

The last rung of the handler-control arc. A **transformer** is an effect member
that runs a fn-typed parameter with *additional* effects available — its body
having registered handlers for them. `Retry`, `Timeout` and async are all that
shape, and `try` could be re-expressed as a library transformer if it reads
better than the intrinsic.

- **The gate is already built**: step 3 made fn-type effect lists real and threads
  effects *into* a fn value instead of capturing them [fn-effects]. What remains is
  the surface.
- **No silent colouring** (user decision 2026-09-04): the ability to not resume is
  declared on the effect member, never discovered from the handler.
- **Async is explicitly not part of this arc**: it arrives later as an explicit
  effect, likely a compiler intrinsic, not as an `async`/`suspend` transform of
  the whole program.
- **Multi-shot resumption is closed on principle**: resuming twice duplicates a
  use obligation, so it cannot coexist with `Linear`. (Neither target offers it
  either.)

Two related recorded items: a **handler cannot dispatch to itself** — inside a
member the bare member name is already taken (it means the handler registered
*before* this one [effect-intercept]), so self-dispatch needs a different
spelling, and that spelling is a language call (`self.bump()`, `bump@self()`, …);
the workaround (a free fn both members call) is what std does and has cost
nothing. And **two cuts inside the effect fusion** stay *reported* rather than
mis-emitted [rs-effect-fusion]: a dependent handler using its own generic
parameters in a member signature, and a `use` whose effect instance is still
generic.

### 13 — The sugar pass (after the explicit surface, decided 2026-09-15)

Phase 5 delivered the **explicit** actor surface (tokens and reply parameters
written out); every layer of sugar above it is a later item with its own decision
surface. What is already decided, so the pass starts from a plan:

- **Per-kind `-> T`** (EU-6): plain effects unchanged forever; inside an
  `actor effect`, `fn m(a) -> T` means an implicit trailing `Reply<T>`,
  fulfil-at-every-return, and caller-side call syntax = auto-mint + gate. This is
  why `send fn` stayed explicit: the non-send forms are stated *against* it.
- **The generalized mint** (EU-7b): `replyto k(c)` resolves lexically against the
  enclosing handler, else through the effect list as a *remote* mint — a curried,
  capacity-reserved, one-shot send, with reservation at mint time so discharge
  never blocks, and mint sites contributing deadlock edges like sends.
- **Two stub readings appear here, and only here.** An answering member cannot
  have one implementation for both bindings: from an actor the call parks, from
  synchronous code it must block.
- Then the rest of the tower, each its own call: `then`/`then!`, `defer`,
  merge/join, the gate's member-set generalization.
- [fate-lambda] belongs here — no first-pass form crosses a closure. The recorded
  refinement is `move`-closure emission with hoisted clones, plus a treatment for
  captured effect-handler locals.

### 14 — Shared mutable state: `Cell` (DECISION)

Deferred until after actors deliberately, because the OTP answer is that actors
own their state and message-pass — which may remove the motivation. The full
design (a capability *qualifier* rather than a container type, the representation
per backend, why it cannot panic, the rules it drags in, and the producer case
that is its hardest customer) is recorded in COMPLETED.md's "Shared mutable state
(`Cell`)" section, moved there with this consolidation. The question to answer
first is whether shared mutable state joins the language at all.

When it is taken, it should be decided **on one table** with the rest of the
sharing story: shared-immutable versus shared-mutable, invalidation-checked
versus unchecked (today's fate links, frozen `Reg`, `canbe` groups, `Cell`) —
with the observation that `proj` is the degenerate group (a read-only member of a
singleton group under maximal invalidation sensitivity), so the two are points on
one dial rather than two features.

### 15 — Regions (designed 2026-09-10, unbuilt)

Fully designed and recorded: `effect Region` with an intrinsic handler, `Reg` as
an intrinsic provenance qualifier, regional-by-birth defaults, `reg`/`unreg`, the
freeze (dropping `Mut` makes handles duplicable, kills fate links and makes state
qualifiers permanent), and the escape rule. Kotlin erases it entirely; Rust stages
it — v1 `Rc<T>` [rs-region-rc], v2 a real arena with one mechanical lifetime per
delimiter [rs-region-arena]. The design, the R2 rejection (regions manage memory
and lifetime, never obligations) and what it retires for frozen values are in
COMPLETED.md's "Regions" section, moved there with this consolidation.

Still open when it is picked up: the exact freeze spelling (`^Mut` as an
expression, freeze-by-position, or both); cross-region operations (out of v1);
`unreg` of a deeply regional structure copying deeply; and folding D7's
`Local`/`Escaping` watch-list entry into the design.

### 16 — Composing iterators: stages over a generic source (after the redesign)

The iterator redesign (COMPLETED.md, 2026-09-27) settled §16's *spelling*: a
stage is an `iter fn`. Over a **concrete** source it works today —

```
iter fn evens(xs: List<Int>) -> Emitted Int | Finished {
    state { inner: Mut ListYield<Int> = iter(xs) }
    …
}
```

— nothing is minted until the call, per drive, so replay is free, and
`for n in doubled(evens(xs))` chains. What is **not built** is the generic
stage, in either of its two forms, both refused today at the `iter fn`:

- **over a generic source**: `iter fn evens<C>(c: C, ?Iter<C, Int>)` with
  `state { inner: iter Int = iter(c) }`;
- **over an iterator**: `iter fn doubled(it: iter Int)`, the position
  *consumed* into the hidden struct (today every `iter fn` parameter is
  borrowed and read-only).

Both need the same two things (ITER_REDUX.md, round 6): the generated `next`
carries the **forwarded implicits** (`?Yield<It, Int>` on the hidden `next`,
forwarded by whoever drives it), and the [group-obligation] match **ignores
trailing implicit parameters** when checking the hidden struct's `: Yield<self,
T>` — implicits are the callee's business, resolved at the call, not part of
the member's shape. The alternative is storing the inner `next` as a fn-valued
field, which [rs-stored-implicit] already lowers for keyed containers; the
implicit-on-`next` form is cleaner. A hand-written generic stage (`struct
MapIter<It, T, U> : Yield<self, U>` with `next(p, ?Yield<It, T>)`) hits the same
obligation gap today, so this is a prerequisite rather than a cost of the sugar.

What falls out once it lands: pipelines as values (`let pipeline = c ->
doubled(evens(c))`, passed as `total(xs, iter = pipeline)`); `zip`, `chain`,
`take` as `iter fn`s with iterator parameters and no hand-written struct; the
two-sources case (`?Iter<A, T>, ?Iter<B, T>` — two implicits named `iter` and
two named `next` at different types) as the first thing to test.

What stays parked: a stage over a **linear** iterator (`Lines`). The hidden
struct would have to be `linear` with a discharger, and the user decided
(2026-09-27) no discharger is generated — a discharger would have to be a
member of the group to be consistent — so that iterator is written by hand,
and generic code that may receive one takes the discharger as a callback
[linear-generics]. Sendability of a composed iterator is unchanged.

**Cleanup riding along**: the internal names still say "pass" (`PassDriver`,
`PassMember`, `pass_elem_ty`, `pass_driver_of`, `for_drivers`' doc) and the
dead residue of the deleted `yield fn` origin machinery is still in the tree
(`PassDriver.origin`, always `false`; `mint_machines`, `ORIGIN_PASS_PREFIX`,
`pass_or_origin_elem_ty`'s origin branch, the emitters' `if driver.origin`
arms). Rename and delete in one change, cutting by function rather than by
region (the 2026-09-10 lesson).

### 17 — Recursive types (DECISION, end of the queue)

Investigated 2026-09-12; nothing needs it, and List-mediated recursion covers its
customers (trees, ASTs, JSON) meanwhile. Where it stands: nothing rejects a
recursive type, so `struct Node { value: Int, next: Node | None }` passes the
checker, runs on Kotlin and dies at rustc with E0072 — an accept/reject
divergence. Recursion **through `List<T>` already works end to end on both
backends**.

- **Step 1, the diagnostic** (a defect fix, independent of the feature): an SCC
  walk over the type graph (struct fields, union arms, alias expansions;
  `List`/array/fn-typed edges do **not** count — they indirect already) and an
  error at the declaration naming the field that closes the cycle, with the
  `List<T>` encoding as the named remedy. [type-no-cycle] landed the declaration
  refusal for the *direct* case 2026-09-25; the union-arm case is what is left.
- **Step 2, the feature**: a boxing rule for the Rust backend — where the box goes
  (minimal-edge boxing, the presumption), transparency at every use site (literals
  wrap, reads autoderef, matches need explicit derefs since box patterns are not
  stable, partial moves keep working, `Mut` paths get `&mut` via `DerefMut`,
  `copy` deep-clones), and **non-regular (polymorphic) recursion refused in
  Salvo** — a type may recurse only at its own instantiation, or Rust
  monomorphizes forever while Kotlin's erasure accepts it.
- **The semantic edges**, each a small language call: **constructibility**
  (`struct A { a: A }` has no base case — refuse cycles with no optional/union
  escape arm, recommended); **depth, not cycles** (values are acyclic, so
  `to_str`/equality/drop terminate, but each recurses per node and a 100k chain
  overflows generated `toString`/derived `Drop` — accept-and-document for v1);
  **linearity stays out** (refuse recursion + linearity in one declaration for
  v1); and an `iter fn` over a recursive subject snapshots per field, which is a
  deep copy.

## Recorded, not scheduled

Each was considered and deliberately parked. Nothing here is blocking, and
several are "revisit only if a customer appears".

- **Unify the mixed servant with private send members** (2026-09-27). A mixed
  handler's servant emits a handler-keyed `__Msg_H` and calls its members
  directly from `resume`; a private send member of an actor handler now does
  the same through `__Priv_H` [actor-private-send]. They are one mechanism
  in two emitter paths (`emit_mixed_actor_parts` beside `emit_actor_body`);
  re-expressing the servant as "a handler with no actor faces and only private
  members" would delete the first. Churn in two emitters, three goldens and
  `examples/`, for no behaviour — do it when one of the paths next needs a
  change.

- **Pick chains** (`^Ok?: Err?: err(_)`) — recorded rather than scheduled at the
  user's call: the same program is expressible today with one pick plus a `when`,
  so a chain buys brevity and the evidence that would settle it is *a real program
  in `examples/` or `std/` that reads worse without one*. Decided and still
  standing if built: picks consume arms **in order**, and the empty pick may appear
  anywhere in the chain. Cost: `Expr::Elvis` needs `picks: Vec<ElvisPick>`, the
  checker consumes arms pick by pick, both emitters emit an `if`/`else if` chain;
  the parser's lookahead already exists.
- **Value-level parity for hash and random.** Callable `hash` lowers to each
  backend's **native** hashing, so hash *values* diverge across backends —
  accepted deliberately, on the analogy of random numbers. The shape and the
  high-level guarantees are identical (`eq(a,b)` ⇒ `hash(a) == hash(b)` within one
  execution). The possible future reversal, for hash and random *together*:
  language-defined algorithms implemented identically in both runtimes (the
  rejected sketch: FNV-1a 64 over a canonical byte encoding). Until then a program
  must not print or persist a hash value and expect cross-backend identity.
- **Platform-handler thread-safety contract — decided 2026-09-26**, as
  `threadsafe platform handler`; built as step ① of the network sequence
  (section 2). What used to be the DECISION here is in COMPLETED.md's log.
- **`on_idle`'s predicate (DECISION).** The hook fires on the strict quiescence
  condition while the deadlock report fires on a weaker one, so a program stuck
  *with a parked frame* gets the report and exit 1 where the relaxed reading would
  let it react. One line in each runtime either way; the argument for relaxing is
  consistency, the argument against is that `on_idle`'s meaning drifts toward
  "nobody can move", which is the report's job.
- **`size(Str)` outside ASCII (DECISION).** Kotlin lowers it to `String.length`
  (UTF-16 code units), Rust to `chars().count()` (code points), so
  `println("${size("a😀b")}")` prints 3 on Rust and 4 on Kotlin. Needs a decision
  about what a `Str` index *means*, then one lowering per backend. `byte_size` is
  parity-safe by construction and is what the filesystem uses.
- **Intersection types (DECISION)** — whether `Addr<A & B>`-style types join the
  language; recorded 2026-09-17 when the tuple form shipped instead.
- **`platform type`** — deferred by decision; `platform effect` and
  `platform handler` are the whole interop surface until a need arises.
- **`const` bindings** — announced (user intent 2026-09-19), not designed. Its
  first customer is recorded: the shareable-handler taxonomy's rung 1 keys on "no
  mutable state", which today means "no fields, no `Mut` constructor parameters";
  `const` immutable fields would join the allowance.
- **`Deque<T>`** — the honest replacement for a linked list, and the next
  collection when a customer appears: one intrinsic type, six functions, no new
  concepts. (A representation qualifier `Linked List<T>` was examined and
  rejected: it would make the shared `List` surface worse, and Rust's
  `LinkedList` has no stable cursor API.)
- **`entries`/`values` iterators over a Map** — deferred: an entries iterator needs an
  owned `(K, V)` and Kotlin cannot copy a generic `V`, so identity-sharing would
  alias mutable values. The answer is probably the snapshot shape the key iterator
  uses, over a `List<(K, V)>`.
- **Test-suite speed** — the stamp key stays **keyed on the generated sources**
  (user decision 2026-09-25): it cannot go stale, and that is worth more than the
  seconds a cheaper key would save. If it ever bites, the options in order:
  a source-keyed stamp plus an emitter-version token; caching the emission beside
  the verdict; emitting `std` once per source and running many; shrinking the
  registry. Measure first: how much of the Kotlin binary's ~15s is `std`
  re-emission versus per-case work.
- **Locators through opaque anchors, branded tokens, and the bounds-check
  mitigation ladder** — the group-borrowing ladder's recorded refinements, with
  GhostCell declined on the record (a brand is a scope-bound *lifetime* and actor
  state escapes every scope) and raw pointers / `RefCell` rejected. In
  COMPLETED.md's log for 2026-09-24/25.
- **In-place writes during iteration** — refused by the driven-origins rule
  [iter-fn]. The contents-versus-replacement distinction [deduce-field] is what
  would license it, and the locator model already makes the shape renderable.
- **Field-set inference for [deduce-field]**, and qualifiers on struct fields:
  v1/v2 are written-only, so an unannotated fn keeps the conservative whole-value
  event.
- **`once` inference**, **returning/storing capture-carrying closures**,
  **exactly-once closures consuming a linear capture**, **reassignable borrowed
  locals** (accumulator bodies under a projected return), **same-call borrow/move
  (E0505 shape)**, **the internal qualifier unification**, **L5 field-disjoint
  precision**, **`NotEq` over any `?eq`-capable subject**, **link parameters**
  (with the nested-`proj` source they would make real), and **type-mention tracing
  for instantiation links** — all unforced, each recorded with its shape where its
  rule lives.
- **The linear instantiation ban misses generic effect members** — a hole in what
  shipped: `[linear-generics]` is checked in `resolve_named_call`, and an effect
  member's own generics are not covered, so a linear value can be smuggled
  through one.
- **Nested patterns** (`let ((a, b), c) = …`) are refused in a `let` as in a
  loop, and **struct patterns bind by field name only** (no `..` rest, no nested
  field pattern, no binding of a projection).
- **A generic `List<T>` cannot be interpolated** [interp-to-str] — waits on
  step 5's lift plus `implicit_args` being keyed by something an interpolation
  has.
- **Emission marks everything public**, by decision: [mod-export] is a checker
  rule. Narrowing generated visibility would buy dead-code warnings the suite
  already tolerates and needs the reachability pass to agree.
- **No re-export**, so a facade module declares wrappers; the spelling would be
  `export import a.B`, currently a targeted parse error. And **no example shows
  `export`**, because every program in `examples/` is a single file — a two-file
  example would fix that and would be the tree's first multi-module one.
- **Where dot-notation is normalized** [fn-dot] — the receiver-as-argument-0
  rewrite happens twice and never in the AST, so anything inspecting the *written*
  expression must re-derive the shift (one defect came from exactly that). Two
  shapes when it is picked up: record the normalized argument list per call span
  in a side table (cheap), or a real normalization pass after types are known
  (removes the duplication). It cannot move into `salvo-syntax`: the name must
  resolve, and an `Addr<E>` receiver is not argument 0.
- **Actor-surface prose is owed in `docs/language/`.** Every actor rule is in
  LANGUAGE_SPEC.md, and docs/language/ has the "Where work runs" and "Time"
  chapters — both of which assume vocabulary the document never introduces
  (`actor effect`, `send fn`, `spawn`, `Addr`, `replyto`/`waitfor`, `watch`).
- **Actor leftovers, each with a named trigger**: a self-send into a full own
  mailbox wedges and `k@self` deliberately contributes no deadlock edge
  (**DECISION** when it matters); the deadlock graph is over actor *types*, not
  instances, so a chain of same-protocol workers reads as a self-loop
  (stratification and the timeout form wait for observed false positives —
  writable now that `Timer` exists); the report names actors by index rather than
  by handler and member; a one-thread pool whose occupant sends into a full
  mailbox on that same pool still hangs (only the `main` case is caught); a `use`
  site does not check the `[spawn]` capability; a main-pool task whose answer
  arrives after `main`'s last wait never runs, silently; an effectful discharger
  cannot `drain` a container; there is no positional list write; `on_idle`'s
  refinements (per-pool firing, naming who is parked, a many-shot form); FC-7 host
  bridging; and the `[waitfor]` spawn-placement diagnostic still states a hazard
  the pump rule removed.
- **Multi-effect and mixed-handler leftovers**: a same-named member across two
  faces still needs `@Effect` at the call even where the parameters distinguish it
  ([effect-at] keys on the name); a dependent multi-face handler is untested;
  mixed handlers with dependencies, overloaded servant members and several faces
  were first-slice cuts; one lock behind several faces has no backend
  representation; generic-instance dependencies stay fusion-pinned (the
  representation now exists, so lifting the `handler_handle_deps` exclusion is
  engineering); a `with` item may not have dependencies of its own, and `main`'s
  platform-effect parameters cannot be captured as handles.
- **`local` inference** — the checker writing `local` for you, lifting the
  virality down local-trafficking call chains (the deduction pattern: written
  validates, unwritten infers). **Note (user, 2026-09-26): the meaning of `local`
  is itself being revisited** — a round of questions about `println`'s
  `local Console` dependency was deferred with "I might have misunderstood what
  `local` means", so re-read [use-local] and [effect-local] with the user before
  building anything here.
- **`fn qualifies@Positive`** — migrating qualifier bodies' `fn qualifies` to the
  `@`-scoped shape canonicals used. Note that the shape it would migrate *to*
  changed on 2026-09-26 [fn-attached], so this is now "should a qualifier's
  `qualifies` be declared on its subject type?" and wants re-deciding rather than
  implementing. Handler members stay put: they interact with handler state.
- **Constants: literal establishment and constant subtyping** — the refinement
  round's two remainders. Literal establishment is `listen(8080)` proving itself
  (compile-time evaluation of `qualifies`); constant subtyping is
  `InRange(10, 20)` fitting an `InRange(0, 100)` position, which needs
  per-qualifier semantics for what the constants *mean* — **DECISION**-shaped
  when it is wanted.
- **`enumerate`'s claimed `index` field and a claimed `keys` iterator** — left out of
  claim minting by design: a dependent claim on a struct field names a value the
  struct does not contain, and the map iterator walks a key snapshot. The snapshot
  form (`keys -> List<KeyOf(map) K>`) waits for step 6's variance round.
- **The `Bytes` span twin** needs same-name-different-subject value slots.
- **Binding a view of a temporary** is refused for now (user, 2026-09-11); the
  possible automation is hoisting the temporary into a fresh local, which is what
  the user writes today — not free of judgement, since the temporary then lives to
  the end of the block and a hoist inside a loop changes how often it is built.
- **A runtime file name colliding with an emitted std module** — the *silence*
  is closed (2026-09-26): emission refuses at the end when two files claim one
  path [backend-companion], which is how it was caught the second time (`throw`
  left `core`, and the runtime's `throw.kt` had been winning that path). The
  workaround is still a rename each time — `hosttime.{rs,kt}`, now
  `throwsignal.kt`. The durable fix, unscheduled: namespace the runtime under
  `salvo_rt/`, which touches every golden, every example and the documented
  `kotlinc`/`rustc` invocations, so it wants its own slice.
- **`to_str(Duration)` stops at seconds**, and there is no `to_str` for `Instant`
  or `Tick`: a wall-clock text form is a date (the calendar layer's), and a
  monotonic reading has no rendering beyond its number. **Cancellation is not in
  the timer surface** (recorded by decision), and **the wall-clock layer is
  designed but unbuilt** — `DateTime` as the calendar view of an `Instant` in a
  zone, `Period`, the two bridges, and a `WallClock` effect whose member must
  **not** be named `now`.
- **The unified test clock fakes `Ticker`; the `Clock` face is untested**, and a
  handler that waits on a *positive* deadline still wedges a `ManualTime` test —
  though it now says so, via the deadlock report. Every clock reading through the
  unified form is a round trip, which is the stance's remaining cost; the recorded
  upgrade is scheduler-owned virtual time.
- **The parked-obligation gap in the deadlock graph**: an actor gated on a token a
  *task* must discharge has a wait-for edge pointing at no effect node.
- **Array elements never narrow**, **deduction inference does not track
  bare-parameter value flow out of branch/loop tails as a move**, **module
  reachability is conservative for non-fn names**, **a private type in an
  exported signature is an opaque type and nothing checks the author meant it**,
  **struct destructuring ignores predicate-qualifier field overrides**,
  **repeating the *same* qualifier in a nested group needs an annotated
  intermediate `let`**, **`Byte` is not operator-numeric**, and **generic
  (`Ty::Var`) operands stay lenient** — each documented where its rule lives.
- **LSP**: `[symbol]` resolution is name-based over the AST rather than
  import-visibility-exact [doc-symbol-ref]; no incremental analysis; no
  `positionEncoding` negotiation for UTF-8-native clients; signature *hover*
  covers fn decls only (effect members and define fns have no `FnKey`); the
  def-site table is name-keyed, so go-to-definition on a shared effect-member
  name lands on one declaration; and hovering a fate *root* to see what derives
  from it is not built.
- **Fresh-suite speed, remaining steps toward ~10–15s**: the CLI suites (each
  spawns `salvo run`/`compile` and pays its own kotlinc), the rust codegen suite
  (a shared-runtime batch or precompiled `libcore`), and running the batched
  Kotlin programs in one JVM instead of one `kotlin` launch each. Past those the
  floor is the matrix size itself.
