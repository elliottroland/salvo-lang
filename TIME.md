# Time — Timer, Clock, and deterministic tests (working document)

Status: **DECIDED** (user decisions 2026-09-17, all five sections in one
sitting — the confirmation list is in COMPLETED.md's log, and the
implementation order is ROADMAP.md's **"The second sequence"**): T-1(b) the
intrinsic Timer (with the **time types** — `Instant`, `Duration` — designed
first, a DECISION row in ROADMAP due at step 5); T-2 time-as-data as the std
coupling stance; T-3(a) `on_idle` in the token form; T-4(a) multi-effect
handlers (same-named members legal when overloading distinguishes or one
method implements both; intersection types recorded in ROADMAP as the
unscheduled future alternative); T-5(c) `[waitfor]` as a placement-gated
capability effect with the uniform pump-tasks wait semantics, the `on`
clause **consuming** a `Dedicated Pool`. The sections below keep the
argument trails; this document is **deleted when the second sequence's
relevant steps land** (the FILE_SYSTEM.md life cycle).

Written 2026-09-16 by a read-only session, out of a design conversation that
started at bounded-vs-unbounded mailboxes, passed through the timeout pattern
(two racing `replyto` continuations over a pending map), and arrived at the
gap the pattern exposed: **Salvo has no way to sleep or read time.** It lays
out the design in DESIGN_DOC.md's shape — stated intent, fixed points, survey,
then numbered decision sections with options, trade-offs, and a
recommendation — and the calls are the user's (AGENTS.md's first invariant).

**Swept and propagated 2026-09-17.** Phase 5 closed 2026-09-16 (after this
document was written): the working documents it cited (CONCURRENCY.md,
SUPERVISION.md, the examples files, LINEARITY_COLLECTIONS.md,
EFFECT_UNIFICATION.md) are retired into COMPLETED.md's log and the
`[actor-*]` rules; `async effect` became **`actor effect`**; the spawn line
lost its `capacity` clause (**the mailbox is declared on the handler**,
[actor-mailbox]); and the carried IO-actor question was **closed** — a plain
effect is never actor-backed ([actor-effect-kind]) — which overtakes this
document's T-2 option 3 as originally worded (revised below). ROADMAP.md's
**"The second sequence"** now holds the implementation plan for this
document's decided outcomes. A sibling working document,
**free concurrency** (2026-09-17; its document retired into COMPLETED.md's
log when steps 1–2 landed) proposed free `send fn`s and
main-as-a-pool; its FC-4 reframes T-5 (noted there). Delete-when-decided
charter unchanged: this document goes when its outcomes live in the log and
the specs.

Sources: LANGUAGE_SPEC.md's actor chapter — [actor-kind] (run-to-completion,
no blocking mid-body), [actor-effect-kind] (the kind divide and its refusal
list; plain effects are never actor-backed), [actor-sendable],
[actor-replyto] (mint targets, trailing-token convention), [actor-waitfor]
(`main`'s bridge; the program ends when `main` returns), [actor-use-addr],
[actor-mailbox], [actor-watch] (`watch(p, on_exit: Reply<Exit>) [spawn]` —
the whole monitor surface), [actor-deadlock-cycle] (gate cycles error,
back-pressure cycles warn); [linear-obligation] and [linear-container]
(tokens in collections — the pending-map pattern's enabler);
COMPLETED.md's phase-5 log (the runtime idle report — "salvo: deadlock: all
actors idle while main waits" — and the timeout fallback
`Reply | TimedOut` recorded with the deadlock statics); ROADMAP.md ("Actors",
"The sugar pass — after phase 5"); `examples/actors/` (current syntax,
including nested `waitfor`); and the user's stated intent (below).

## 0. The stated intent

The user's direction from the 2026-09-16 session, numbered so the decisions
below can be judged against it. Points 1–3 are direction; 4–6 are leanings
recorded at the user's request, each still owed a confirmed decision.

1. **Timer support is needed.** There is no way to sleep or track time; the
   timeout pattern (racing a reply against a deadline) has nothing to race
   against. The implementation must not keep threads busy sleeping — no
   thread-per-timer.
2. **The direction is an intrinsic Timer effect** (T-1(b)): an ordinary
   `actor effect` in std, default handler intrinsic-backed in the scheduler
   runtime library — chosen especially because the effect surface makes a
   pure-Salvo test fake (`ManualTime`) possible, the `MemFs` move applied to
   time.
3. **Timer and Clock are designed together**, and the tension is that asking
   the time *now* is naturally synchronous while the timer is inherently
   asynchronous.
4. **Consider lifting `waitfor`'s main-only restriction** (T-5), so a test
   `Clock` handler can be written that simply waits on a `Timer` for the
   virtual time — while production keeps the real synchronous intrinsic.
   *(Upgraded 2026-09-17: the lift became `[waitfor]` as a placement-gated
   capability effect — see T-5.)*
5. **Inclined to support a quiescence hook** (T-3) — `on_idle`, shaped like
   `watch` — so tests can sequence "everything has drained" before advancing
   virtual time, without moving time into the runtime.
6. **Inclined to support handlers of multiple effects** (T-4) — one state,
   several typed faces — which removes the forwarding boilerplate between a
   production protocol and its test-control protocol.

Point 1 meets the recorded timeout fallback (`await_within` →
`Reply | TimedOut`, held with the deadlock statics' deferred work — ROADMAP
"Actors"): Timer is its missing prerequisite. Point 3 originally met the
carried IO-actor question, since **closed** by [actor-effect-kind]; the
surviving mechanisms are T-5 and the sugar pass (see T-2). Point 4 collides
with [actor-waitfor] ("legal only in `main`") — that collision is T-5, now
also reframed by FC-4 (built). Point 6 collides with the
one-effect-per-handler shape — that collision is T-4.

## 1. Fixed points — already decided, inherited here

- **Run-to-completion: a handler cannot block mid-body** [actor-kind].
  Sleeping can therefore only mean *parking a continuation to be fulfilled
  later* — a blocking `sleep(millis)` member is anti-model, and this
  constraint dictates the Timer surface (`after` takes a `Reply`). §0 point 4
  knowingly reopens one corner of this; flagged there and worked in T-5.
- **Reply discharge never blocks** (reserved capacity, [actor-replyto]): a
  timer *firing* is `done.send(Fired {…})` — so whatever fires deadlines can
  never be wedged by a full mailbox.
- **The landed surface**: `actor effect` members are `send fn`s
  [actor-effect-kind]; `replyto k(captures)` targets a `send fn` member of
  the enclosing handler [actor-replyto]; the handler declares its own queue
  depth (`mailbox { capacity: n }`, readable from constructor parameters)
  [actor-mailbox]; a spawn reads `spawn H(args) use deps on POOL`
  [actor-spawn-expr]; and `waitfor` is **legal only in `main`**
  [actor-waitfor] — `main`'s only token source. T-5 is the proposal to amend
  the last item.
- **Nested `waitfor` works** — demonstrated in `examples/actors/` (section 4
  nests one bridge inside another). The composition question this document
  originally flagged for T-3 is answered by running code.
- **A handler implements one effect** (the shape today). The pure-Salvo test
  timer therefore needs a forwarding handler between its `Timer` face and its
  control protocol — T-4 is the proposal to remove that.
- **A plain effect is never actor-backed** [actor-effect-kind] — the old
  IO-actor question, closed. A synchronous-*looking* call that parks exists
  only in the sugar pass's plan (an `actor effect` **call member**, EU-2's
  two stub readings: from an actor the call parks; from synchronous code it
  must block, which `main` may and an actor may not — ROADMAP "The sugar
  pass"). T-2's unified-clock option is respelled against this.
- **The runtime already detects quiescence**: the idle report shipped with
  phase 5 ("salvo: deadlock: all actors idle while main waits"). T-3 exposes
  the same fact as a subscribable event; the detection machinery is not new.
- **`watch` is the precedent shape for runtime-delivered events**
  [actor-watch]: `watch(p, on_exit: Reply<Exit>) [spawn]` — a one-shot
  linear token the runtime consumes when the event happens; dropping the
  registration is a leak diagnostic [linear-obligation].
- **Linear tokens live in collections** [linear-container]: the pending-reply
  map of the timeout pattern and ManualTime's deadline list are customers —
  a `Reply` inside a struct inside a `Mut List` / `Mut Map`, moved out by
  linear extraction (`remove_first`, `drain` — see `examples/actors/` and
  `examples/linearity/`).
- **The program ends when `main` returns** [actor-waitfor]: pending timers
  die with the program; no ambient keep-alive. Tokens whose target died are
  silent no-ops; sends to the dead likewise.
- **Interop is unchanged** [intrinsic-std-only] / [platform-effect]: std's
  timer reaches the platform as an `intrinsic`. Scheduler-adjacent machinery
  lives in **backend runtime files**; anything needing compiler-specific
  cooperation is flagged to the user first.
- **Sendability** [actor-sendable]: payloads and `replyto` captures are
  checked (declaration for member payloads, site for captures and spawn
  arguments); `Reply<Fired>` crossing into the timer is ordinary.
- **The deadlock statics** [actor-deadlock-cycle]: gate cycles are errors,
  back-pressure cycles warnings. The recorded fallbacks for reported cycles
  (a timeout form `Reply | TimedOut`, per-edge reentrant opt-in) live with
  the deferred checker work in ROADMAP "Actors" — this design's T-1 is their
  prerequisite. The hand-written timeout form — two bare `replyto` mints
  racing into a pending map, loser finds `None` — needs only T-1 to become
  writable; a *gated* timeout needs the gate member-set generalization (a
  recorded later sugar item).

## 2. What other languages teach

### Kotlin / kotlinx-coroutines — virtual time lives in the test scheduler

`runTest` runs coroutines under a `TestCoroutineScheduler` that owns virtual
time: `delay` registers against it, and when the dispatcher is idle the
scheduler advances time to the next scheduled resumption automatically
(`advanceUntilIdle` / `advanceTimeBy` for manual control). Production `delay`
is scheduled by the dispatcher — no thread sleeps per delay.

- Informs T-2/T-3 and the "option 4" upgrade path (§ Worked setups, Test C):
  scheduler-owned virtual time solves both the advance race (via
  run-until-idle) and clock/timer agreement — at the price of moving time into
  the runtime. Also the precedent for the classic virtual-clock race: a
  deadline registered *relative to current virtual time* after an advance has
  already run misses its window unless advance waits for quiescence first.

### Rust / Tokio — `pause()` and auto-advance

`tokio::time::pause()` freezes the runtime clock in tests; when the runtime
has no work left and the only pending items are timers, time auto-advances to
the next deadline. Production timers are a hierarchical timer wheel serviced
by the runtime — again no thread-per-timer.

- Same lesson as kotlinx, from the other backend's ecosystem: both targets'
  native test-time answers are scheduler-owned. Salvo choosing the pure-Salvo
  fake (T-1(b) + T-3) instead is a deliberate divergence, taken for the
  `MemFs`-style purity, with the scheduler-owned design held as upgrade path.

### Erlang / OTP — timers are messages

`erlang:send_after(Time, Dest, Msg)` delivers an ordinary message at deadline;
`receive … after Timeout` is the built-in selective-receive timeout. Time
arrives in the mailbox like everything else.

- Informs T-1 directly: "a timer fire is an ordinary message" is exactly
  `done.send(Fired {at})`, and validates carrying the fire-time *in* the
  message (time as data, T-2 option 1). `receive after` is the gated-timeout
  shape Salvo would only reach via the gate member-set generalization.

### Pony — timers as runtime-owned actors

Pony's `Timers` is an actor backed by a runtime timer wheel; user code holds a
`Timer` notify object and receives fires as behaviour invocations. No blocking
anywhere, matching Pony's no-blocking substrate — the same substrate Salvo
chose.

- Informs T-1(b): the "intrinsic handler over a runtime wheel/heap" shape,
  proven in the closest substrate relative.

### The two backends — where the intrinsic lands

- **JVM:** `ScheduledThreadPoolExecutor` / `DelayQueue` *are* the deadline
  heap; the fire action is the token send. Possibly zero extra threads.
- **Rust:** std has no timer wheel; the intrinsic is a `BinaryHeap` of
  `(deadline, token)` plus **one** dedicated thread in
  `Condvar::wait_timeout` until the earliest deadline (registering an earlier
  deadline notifies the condvar). ~50 lines, no busy-waiting, no
  thread-per-timer — meeting §0 point 1 on both targets.

## T-1. The Timer surface and what backs it

§0 points 1–2; the prerequisite for the recorded timeout fallback.

The surface, dictated by run-to-completion (a fixed point, not a choice):

```
// std, core.time
struct Fired { at: Instant }             // Instant: the time types, designed
                                         //   at step 5 before the payloads
actor effect Timer {
    // Consume `done` when at least `wait` has passed.
    send fn after(wait: Duration, done: Reply<Fired>) => !done
}
```

(The worked examples below keep provisional integer millis — the `Instant` /
`Duration` design is the step-5 DECISION, and the examples are respelled
when it lands.)

No cancellation in the first cut: a "cancelled" timer fires into a
continuation whose pending-map `take` finds `None` — one no-op activation, the
same discipline as a lost race. A cancel handle is a later addition if the
no-op cost ever measures.

What backs `after`:

- **(a) Time-gated replies in the scheduler kernel** — "enqueue this at time
  T" as a scheduler primitive. *Cost:* the kernel grows a piece; a delayed
  message breaks the arrival-order queue rule (needing a separate deadline
  structure internally anyway); and it escapes the effect discipline — code
  could schedule delayed work with nothing in any signature, the hidden
  dependency the language exists to forbid. Unfakeable from Salvo.
- **(b) An intrinsic Timer effect in std, backed by the scheduler runtime
  library** — the deadline heap + one waiting thread (Rust) or a
  `ScheduledThreadPoolExecutor` (JVM), per §2. The effect surface means a
  **pure-Salvo fake** (`ManualTime`, § Worked setups) costs nothing.
  *Cost:* one runtime thread on Rust; the timer becomes the first intrinsic
  handler *for an actor effect*, so it is the test case for how intrinsics
  meet the scheduler runtime — flag per the implementation-placement rule.
- **(c) A platform effect** — customers declare their own. Wrong division of
  labor: time is as std-worthy as the filesystem, and std's primitives are
  `intrinsic` by rule. Listed to rule out.

**Decided (user, 2026-09-17): (b)** — with the refinement that **the time
types come first**: `Instant`, `Duration` and kin get designed before the
payloads (ROADMAP DECISION row, due at step 5 of the second sequence), so
`Fired` carries an `Instant`, not a raw number. The fake-for-free
argument decided it in session; recorded here with (a)'s costs spelled so the
choice is auditable.

## T-2. Clock, and how it agrees with Timer in tests

§0 point 3. The natures differ, so the split is two effects with different
bindings — this half is not really open:

```
effect Clock {
    fn now() -> Instant    // monotonic; a plain synchronous effect
}
```

`Clock` is a synchronous effect, `use`-bound, running on the caller's thread —
mechanically supported today. `Timer` is an actor effect (T-1). Production
coupling is trivial: `DefaultClock` and `DefaultTimer` intrinsics read the
same OS monotonic source. Wall-clock time (dates, zones) is a separate,
larger std design and out of scope here.

The open question is **test coupling**: one virtual time, two effects, and no
shared mutable state to point them at. Options:

- **(1) Time as data.** `Fired {at}` carries the fire-time; requests are
  stamped at the system's edge and timestamps travel in messages. Most tested
  code then needs no `Clock` at all, and `ManualTime` alone determinizes it.
  *Cost:* an architectural stance, not a mechanism — code that wants ambient
  `now()` mid-handler must be restructured. *Benefit:* zero machinery, and
  the actor model quietly prefers it anyway (time observed mid-activation is
  a race with your own mailbox).
- **(2) Scripted per-actor `TestClock`.** Constructor takes the readings.
  Works today; fine for code that measures locally; diverges from
  `ManualTime`'s virtual time, acceptable when a test does not cross both.
- **(3) One owner: `Clock`'s reading backed by the timer actor.**
  *Revised 2026-09-17* — as originally worded ("the IO-actor pattern") this
  is **closed off**: a plain effect is never actor-backed
  [actor-effect-kind]. The two surviving mechanisms, each a decision of its
  own: **T-5(c)'s `[waitfor]` effect** (a `TestClock` handler that waits on
  the timer — the near-term form, worked in T-5), or the **sugar pass's call
  member** (`fn now() -> Int` inside an `actor effect` — parking from
  actors, blocking from `main`, per EU-2's two stub readings). Perfect
  agreement, one virtual time, either way.
- **(4) Scheduler-owned virtual time** (§2 kotlinx/Tokio; § Worked setups,
  Test C). Solves coupling *and* the advance race in one mechanism, rebinding
  nothing (the same `DefaultClock`/`DefaultTimer` intrinsics read the pool's
  clock). *Cost:* time moves into the runtime; the pure-Salvo fake is
  forfeited; quiescence-aware advance must be implemented identically on both
  backends.

**Decided (user, 2026-09-17):** ship the split with (1) as the std
design stance (fires carry `at`; err toward delivering time as data), keep
(2) for local measurement, and treat (3) as the principled endpoint — via
T-5 near-term or the call member later. Hold (4) as the upgrade path if
duration-measuring tests or advance ergonomics bite. Note T-3 removes (4)'s
*sequencing* advantage, narrowing what (4) uniquely buys to clock
unification.

## T-3. The quiescence hook — `on_idle` — ✅ built 2026-09-18

§0 point 5 (**user leaning: support**). The runtime's idle detection already
exists (the phase-5 report); this exposes it through [actor-watch]'s shape:

```
// One-shot: the runtime consumes `notify` when the pool reaches quiescence —
// no activation running, all queues empty, nothing in flight.
fn on_idle(p: Pool, notify: Reply<Idle>) [spawn]

struct Idle {
    parked_gates: Int,     // 0 = truly done; >0 = idle but waiting
    parked_tokens: Int
}
```

Same properties as `watch`, for the same reasons: linear token (a dropped
registration is a leak diagnostic); edge-triggered one-shot (delivering to an
actor ends the idleness — correct); a token targeting `main`'s `waitfor`
wakes the real thread without perturbing the pool.

What it buys: the virtual-time **advance race** — `ctl.advance(…)` competing
in arrival order with an in-flight `after(…)` registration — dissolves into
two lines of test code (§ Worked setups, Test A), keeping time itself in pure
Salvo. Without the hook, the only pure-Salvo mitigation is counted advance
(`advance_when(scheduled_count, millis)` re-enqueuing itself until the count
is met), which encodes *how many* timers the code under test sets — brittle
in exactly the way quiescence is not.

- **(a) The token form** (above). Uniform with `watch`; usable by a
  supervisor actor, not just `main`; composes with the later merge/join
  sugar. The composition it needs — two outstanding main-bridge tokens via
  nested `waitfor` — is **already demonstrated** (`examples/actors/`,
  section 4), so the question this document originally flagged here is
  answered.
- **(b) A main-only blocking form**, `p.await_idle()`. Simpler; no token.
  *Cost:* breaks token uniformity; unusable from actors; a second main-only
  construct beside `waitfor`.
- **(c) None** — counted advance as the test discipline. *Cost:* the
  brittleness above; and the runtime's detection exists anyway, so the
  saving is only the registration API.

Caveats inherited from the existing detection, not new: idleness is only
meaningful while `main` is the sole external injector (a platform handler
with a real thread — a socket — can stale the answer); and a blocked or
parked activation counts as non-idle, so the hook and T-5's blocking waits
interact benignly (idle simply does not fire while a blocking call is in
flight).

**Decided (user, 2026-09-17): (a)**, the token form.

## T-4. Handlers of multiple effects — ✅ built 2026-09-18

§0 point 6 (**user leaning: support**). Today's shape forces the forwarding
split: `ManualTimerCore of TimerCtl` owning state plus `ManualTimer of Timer
[TimerCtl]` forwarding into it — same runtime shape, pure boilerplate, and
the pattern generalizes ("a public face and an admin face": health checks,
draining, stats — the test control here is one instance).

```
handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 64 }
    …
}

// One actor, one mailbox, one owner of virtual time, two typed faces:
let (timer, ctl) = spawn ManualTime() on pool(1)
//   ^Addr<Timer>  ^Addr<TimerCtl> — least authority falls out of the types:
//   production code holding `timer` cannot name `advance`.
```

- **(a) Multi-effect handlers, spawn yielding one addr per effect** (above).
  `use H(…)` binds all implemented effects in scope; per-effect shadowing
  already works. No new types: no intersection addrs, no subtyping. The
  deadlock graph [actor-deadlock-cycle] keeps its effect-keyed nodes — two
  nodes that happen to be one actor is the same conservative approximation
  the type-level graph already makes. *Cost:* grammar + resolver work;
  exhaustiveness accounting at `use`/spawn sites covers a set; one
  `mailbox` slot serves both faces (arrival order across them, as across
  members today).
- **(b) Status quo: the forwarding pattern.** Works today (the forwarder is
  constructed inside the consuming actor via the spawn-site `use` clause —
  not an extra actor). *Cost:* boilerplate per protocol pair, forever; the
  admin-face pattern stays annoying.
- **(c) Intersection-typed addrs** (`Addr<Timer & TimerCtl>`). More
  expressive (one value, both faces) but imports intersection types into a
  language that has unions only. Rejected as over-machinery; (a)'s tuple
  gets the value without the type.

Precedent: Cloud Haskell's several-typed-ports-per-process — readmitted here
in the harmless form (multiple *protocols*, still one arrival-order queue per
actor).

**What it does not buy** (recorded because the session established it): the
advance race is a property of asynchrony, not handler shape, and clock
coupling is a property of the sync/async boundary.

**Decided (user, 2026-09-17): (a)**, with two refinements: **same-named
members across the implemented effects are legal** — either overloading
distinguishes them or one handler method implements both — and rejected only
where overloading cannot distinguish (same parameters, different return
type); and **intersection types** (option (c)'s shape) are recorded in
ROADMAP as an unscheduled future consideration rather than rejected
outright.

## T-5. `waitfor` as an effect, gated by placement

§0 point 4, **upgraded 2026-09-17** (user direction, in session): instead of
a handler-only carve-out, **`[waitfor]` becomes a capability effect** — any
function may declare it — **and the compiler validates that whatever carries
it runs on a dedicated thread**, checked where running-places are chosen. A
second requirement recorded with it: **uniform semantics** — whether a wait
*blocks* or *pumps* must not fork the meaning, and `[waitfor]` must pass
around as an effect identically under either execution strategy. The
motivating customer is unchanged: `TestClock of Clock [Timer, waitfor]`,
whose `now()` waits for `after(0, …)`'s `Fired` and returns `.at`.
(*Cross-reference:* FC-4 — its "driving main" and this
section's semantics are one rule; decide together.)

### The design

- **The effect.** `[waitfor]` is a lowercase capability effect on
  [actor-spawn-effect]'s exact precedent (`[spawn]` is already "a capability
  the context supplies, carried through effect lists"). It propagates like
  any effect; handler dependencies stay invisible to *effect callers* by
  existing rule, so a `Clock` bound to `TestClock [Timer, waitfor]` keeps
  clean call sites while the hazard keys on the binding.
- **The placement type.** The checker cannot evaluate `on` expressions, but
  it can read their types — and "a claim about where the handle came from"
  is a provenance qualifier. `thread()` (std) answers a **`Dedicated Pool`**
  — one fresh OS thread, owned by what is placed on it; `pool(n)` answers a
  plain `Pool`. "May block here" becomes an affordance in the type — and
  **the `on` clause consumes a `Dedicated Pool`** (user refinement,
  2026-09-17): reusing a thread is impossible by linearity, so exclusive
  ownership is enforced by the move, not by convention.
- **Granting rules — all binding-site checks:**
  * `spawn H(...) on <expr>`: if `H` carries `[waitfor]` (transitively,
    through ordinary propagation), the placement must type as
    `Dedicated Pool`. An actor on its own thread may block — it wedges only
    itself, which is its own business, like a gate.
  * `use H(...)` inside an actor: a `[waitfor]`-carrying handler makes the
    binding actor carry it, which flows to *its* spawn site.
  * A mint targeting a `[waitfor]`-carrying free `send fn`
    [task-mint]: requires dedicated placement — explicit
    `on thread()`, or inherited from a context that itself carries
    `[waitfor]`, whose ambient pool is thereby provably dedicated. The proof
    travels in the effect lists; no ambient-type problem.
  * `main` declares it like anything else: `fn main() [use, spawn, waitfor]`.
    A main that never blocks doesn't say it. This **deletes** the main-only
    rule rather than amending it — main was only ever special by being a
    dedicated thread.
- **What the statics see.** The deadlock graph gets its third edge kind
  *from signatures* — the modular form the retired survey predicted for
  awaits, arriving for blocks. That covers the hazard dedicated threads do
  not remove: a wait whose fulfilment routes back through the waiter's own
  mailbox is a self-deadlock, now a visible `[waitfor]`-edge cycle. The
  runtime pool-exhaustion report demotes to belt-and-braces. FFI sync
  bridges (FC-7, parked in ROADMAP) stop being a special case: a host
  thread is a dedicated thread, so the blocking shim carries `[waitfor]`
  and type-checks like everything else.

### Uniform wait semantics — blocking and pumping are one rule

The user's requirement (2026-09-17): no semantic fork. The resolution is one
rule everywhere: **a `waitfor` serves its own pool's detached tasks while it
waits, and never serves activations.**

- **For `main`** (under FC-4(a)): main's pool's work is all tasks — main has
  no mailbox — so this *is* the "driving main" of FC-4, full pumping. Not a
  special case: the general rule applied to a pool with no activations.
- **For a dedicated actor**: its mailbox stays stalled while it waits —
  serialization is preserved, and for *messages* the behaviour is observably
  identical to blocking. But its own minted tasks still progress — which
  dissolves a default-path trap: FC-3's inherit rule places a task on the
  waiter's own thread, and under pure blocking, waiting on that task's reply
  would be a guaranteed self-deadlock. Under task-pumping it completes.
- **Blocking is the degenerate case** (an empty task queue), so "blocking vs
  pumping" is not a fork at all — one semantics whose behaviour depends on
  what is queued, identical on both backends. `[waitfor]`'s signature-level
  meaning — "may occupy this thread until an answer arrives" — is the same
  under either, which is what makes the effect passable regardless.
- **Costs, stated:** pumped tasks run nested on the waiter's stack
  (recursion; a pumped task that itself waits nests further — Kotlin's
  nested `runBlocking` is the livable precedent, and stack depth is the
  budget); task side effects are observable *during* a wait (defined and
  deterministic given the queue, but code that assumed nothing happens
  during its wait must not — only its *mailbox* is quiet).
- **What stays hazardous:** activations are never pumped, so the
  own-mailbox self-cycle above still hangs — caught statically by the
  `[waitfor]` edges, and dynamically by the idle report.

### Options

- **(a) Keep main-only.** The test clock waits for the sugar pass's call
  member; tests meanwhile use time-as-data (T-2(1)). *Cost:* the unified
  pure-Salvo clock is deferred, possibly long.
- **(b) The 2026-09-16 form — a declared handler dependency, runtime
  enforcement.** Kept for the argument trail; **superseded by (c)**, which
  keeps its spelling (`[Timer, waitfor]`) and replaces its weakest point —
  the pool-wedging hazard as documentation plus a runtime report — with a
  static placement check.
- **(c) The effect + placement design above** (user direction 2026-09-17).
  *Cost:* a new provenance qualifier on pools (`Dedicated`), `thread()` in
  std, the granting checks, the new edge kind, and the uniform pump
  semantics in both runtimes. *What it buys:* any function may wait, the
  constraint moves from *who you are* to *where you run*, and every hazard
  is either a type error or a visible graph edge.

**Decided (user, 2026-09-17): (c)**, jointly with FC-4 — the pump rule and
the main-pool question are one surface, and both were confirmed the same
sitting. Blocking remains the wrong tool for high-traffic paths (a held
thread is a held thread); the sugar pass's parking call member stays the
endpoint for those, and (c) is the honest spelling for the cases that
genuinely want a thread — test clocks, FFI edges, dedicated blocking-IO
actors wrapping host APIs.

## Worked setups — production vs test

The options above, made concrete on the credit-check-with-timeout service
(the session's running example: `OrderService` races a `CreditCheck` reply
against a 2000ms deadline via two bare `replyto` mints over a pending map).
Syntax is current as of 2026-09-17 (`actor effect`, handler-declared
mailbox, `spawn H(args) use deps on POOL`).

### Production

```
fn main() [use, spawn] {
    use DefaultClock()                    // intrinsic: OS monotonic
    let timers = spawn DefaultTimer() on pool(1)
    //           intrinsic: deadline heap + one waiting thread (Rust),
    //           ScheduledThreadPoolExecutor (JVM) — §2; mailbox declared
    //           by the handler [actor-mailbox]
    let credit = spawn CreditBureau(...) on pool(4)
    let orders = spawn OrderService() use credit, timers on pool(4)
    // ... serve; the program ends when main returns — pending timers die with it
}
```

### Test A — pure Salvo: `ManualTime` + `on_idle` (T-1(b) + T-3 + T-4)

Everything in Salvo except the already-landed idle detection. Deterministic
*ordering*; time never really passes.

```
handler ManualTime() of Timer, TimerCtl {        // T-4: two faces, one state
    mailbox { capacity: 64 }
    now: Int = 0,
    scheduled: Mut List<Scheduled> = mut_list_of()   // linear tokens in a
                                                     // collection [linear-container]
    send fn after(millis: Int, done: Reply<Fired>) {
        add(scheduled, Scheduled {at: now + millis, done: done})
    }
    send fn advance(millis: Int) {
        let target = now + millis
        for s in take_due(scheduled, target) {   // linear extraction, deadline order
            now = s.at
            s.done.send(Fired {at: s.at})        // discharge — never blocks
        }
        now = target
    }
}

fn main() [use, spawn] {
    let p = pool(1)
    let (timer, ctl) = spawn ManualTime() on p
    let credit       = spawn SilentCredit() on p     // never answers
    let orders       = spawn OrderService() use credit, timer on p

    let outcome = waitfor result: Reply<Placed | Rejected Str> {
        orders.place(order(7), result)
        waitfor idle: Reply<Idle> {        // block main until the system drains:
            on_idle(p, idle)               //   place ran, credit.check delivered,
        }                                  //   the deadline is registered
        ctl.advance(2000)                  // the race is gone — nothing in flight
    }
    // outcome == rejected("credit check timed out"), always
}
```

Without T-3, the `advance` races the in-flight `after` registration and the
test can hang; the pure-Salvo fallback is counted advance (`advance_when`),
recorded in T-3 as the brittle alternative.

### Test B — the unified clock via the `waitfor` effect (adds T-5(c))

For code that *measures* — `rejected("timed out after ${now() - started}ms")`
— clock and timer must agree. `TestClock` waits on `ManualTime` for the
virtual time; the declared `waitfor` dependency keeps the blocking visible at
the binding, and the timer lives on its own pool so the wait cannot wedge the
system under test.

```
handler TestClock() of Clock [Timer, waitfor] {  // T-5(c): the capability effect
    fn now() -> Int {
        let f = waitfor fired: Reply<Fired> {
            after(0, fired)                       // "fire immediately" = ask the time
        }
        return f.at
    }
}

fn main() [use, spawn, waitfor] {
    let tp = pool(1)
    let (timer, ctl) = spawn ManualTime() on tp
    let credit       = spawn SilentCredit() on pool(1)
    // OrderService binds TestClock, so it carries [waitfor] — the placement
    // check requires a Dedicated Pool, and the wait can wedge only itself:
    let op     = thread()                         // Dedicated Pool
    let orders = spawn OrderService() use credit, TestClock(), timer on op

    let outcome = waitfor result: Reply<Placed | Rejected Str> {
        orders.place(order(7), result)
        waitfor idle: Reply<Idle> { on_idle(op, idle) }
        ctl.advance(2000)
    }
    // outcome == rejected("credit check timed out after 2000ms") — exactly:
    // `started` read virtual 0, the fire happened at virtual 2000, one clock.
}
```

The same test under (a)-not-lifted: replace `TestClock` with time-as-data
(stamp the request at ingress; the continuation has `Fired.at`) — T-2(1) —
or wait for the sugar pass's call member, under which `now()` parks from
actors instead of blocking and the `[waitfor]` dependency disappears.

### Test C — the upgrade path: scheduler-owned virtual time (T-2(4), not taken now)

Recorded for contrast: `let vt = test_pool()` — the pool owns a virtual
clock; the *production* `DefaultTimer`/`DefaultClock` intrinsics read it, so
tests rebind nothing; `vt.advance(2000)` means "run to quiescence, then move
time", and an auto mode jumps idle+pending-deadline straight to the deadline
(the kotlinx/Tokio shape, §2). Subsumes Test A's hook *and* Test B's clock in
one mechanism — at the cost of moving time into the runtime on both backends
and forfeiting the pure-Salvo fake. The narrowing effect of T-3 is worth
restating: with `on_idle` shipped, (4)'s unique remainder is clock
unification, which T-5(c) or the sugar pass's call member also deliver.

## Decided (user, 2026-09-17)

| Label | Decision | Refinements added at decision time |
|---|---|---|
| **T-1** | **(b)** — the intrinsic Timer as an actor effect over a runtime deadline heap | **The time types come first**: `Instant`, `Duration` and kin get designed before the payloads (DECISION row in ROADMAP, due at step 5); `Fired` carries an `Instant`, not a raw number |
| **T-2** | Sync `Clock` effect (`now() -> Instant`, monotonic); **time-as-data** as the std coupling stance; scheduler-owned virtual time recorded as the un-built upgrade path | Unified clock = `TestClock [Timer, waitfor]`, writable after steps 1 + 5 |
| **T-3** | **(a)** — `on_idle(p, Reply<Idle>)` in the token form | — |
| **T-4** | **(a)** — spawn yields one addr per implemented effect (the tuple form) | Same-named members across the effects are **legal** — overloading distinguishes, or one method implements both; rejected only where overloading cannot distinguish (same parameters, different return type). **Intersection types** added to ROADMAP as the unscheduled future alternative |
| **T-5** | **(c)** — `[waitfor]` as a capability effect, placement-gated, uniform pump-tasks semantics | The `on` clause **consumes** a `Dedicated Pool` — thread reuse impossible by linearity. Effect declaration members may carry `[waitfor]` |

Implementation order: **ROADMAP.md, "The second sequence"** — the `waitfor`
package first (jointly with FC-4), then the task kernel, `on_idle`,
multi-effect handlers, `core.time` (time types, then the intrinsics and
`ManualTime`), and the coupling stance last.

Owed with implementation: fresh rule labels (suggested: `[time-timer]`,
`[time-clock]`, `[time-types]`, `[sched-on-idle]`, `[handler-multi-effect]`,
`[waitfor-effect]`, `[pool-dedicated]`) in LANGUAGE_SPEC.md and the backend
specs; `core.time` in std; and this document's deletion once the relevant
second-sequence steps land.
