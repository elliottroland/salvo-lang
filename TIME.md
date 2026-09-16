# Time — Timer, Clock, and deterministic tests (working document)

Status: **OPEN**, with recorded user leanings (2026-09-16, this session — to be
confirmed before implementation, and to be folded into COMPLETED.md as user
decisions when they are): the intrinsic-Timer direction (T-1(b)), support for
the quiescence hook (T-3), support for handlers of multiple effects (T-4), and
willingness to *consider* lifting `waitfor`'s main-only restriction (T-5).
Nothing here is implemented; the phase-5 first pass this document extends is
itself not yet built.

Written 2026-09-16 by a read-only session (another agent was working in the
repository; this file is the session's only write). The document grew out of a
design conversation that started at bounded-vs-unbounded mailboxes, passed
through the timeout pattern (two racing `replyto` continuations over a pending
map), and arrived at the gap the pattern exposed: **Salvo has no way to sleep
or read time.** It lays out the design in DESIGN_DOC.md's shape — stated
intent, fixed points, survey, then numbered decision sections with options,
trade-offs, and a recommendation — and the calls are the user's (AGENTS.md's
first invariant).

**Propagation owed.** Written read-only, so nothing has propagated: ROADMAP.md
does not yet name a `core.time` item, the T-3 hook under the scheduler work,
the T-4 feature, or the T-5 question; COMPLETED.md has no log entry for the
leanings above; no LANGUAGE_SPEC.md rules or labels exist yet. A write session
must land those, and this document is **deleted** once its outcomes live in the
decision log and the specs — the OBLIGATIONS.md / FILE_SYSTEM.md charter.

Sources: CONCURRENCY.md ("The direction", "The first pass" — the kernel
spellings `send`/`replyto`/`replyto!`/`waitfor`, the spawn syntax, C-3-reshaped,
C-4=(a), C-8, and the named question carried to the call-sugar pass: may an
ordinary member be process-backed — the "IO actor" pattern); SUPERVISION.md
(S-2 `watch(addr, on_exit: Reply<Exit>)`, S-3's idle-with-parked-gates report
— "cheap, the scheduler already knows"); CONCURRENCY_EXAMPLES.md (Example 4(d),
the timeout fallback `Reply | TimedOut`, of which Timer is the prerequisite);
LINEARITY_COLLECTIONS.md (linear tokens in collections, which the pending-map
pattern and ManualTime's heap consume); `crates/salvo-core/src/deadlock.rs`
[async-deadlock-cycle]; rule labels [linear-obligation], [effect-handler-deps],
[intrinsic-std-only], [platform-effect], [linear-static]; and the user's
stated intent (below).

## 0. The stated intent

The user's direction from the 2026-09-16 session, numbered so the decisions
below can be judged against it. Points 1–3 are direction; 4–6 are leanings
recorded at the user's request, each still owed a confirmed decision.

1. **Timer support is needed.** Today there is no way to sleep or track time;
   the timeout pattern (racing a reply against a deadline) has nothing to race
   against. The implementation must not keep threads busy sleeping — no
   thread-per-timer.
2. **The direction is an intrinsic Timer effect** (T-1(b)): an ordinary effect
   in std, default handler intrinsic-backed in the scheduler runtime library —
   chosen especially because the effect surface makes a pure-Salvo test fake
   (`ManualTimer`) possible, the `MemFs` move applied to time.
3. **Timer and Clock are designed together**, and the tension is that asking
   the time *now* is naturally synchronous while the timer is inherently
   asynchronous.
4. **Consider lifting `waitfor`'s main-only restriction** (T-5), so a test
   `Clock` handler can be written that simply waits on a `Timer` for the
   virtual time — while production keeps the real synchronous intrinsic.
5. **Inclined to support a quiescence hook** (T-3) — `on_idle`, shaped like
   `watch` — so tests can sequence "everything has drained" before advancing
   virtual time, without moving time into the runtime.
6. **Inclined to support handlers of multiple effects** (T-4) — one state,
   several typed faces — which removes the forwarding boilerplate between a
   production protocol and its test-control protocol.

Point 1 meets Example 4(d)'s recorded timeout fallback (Timer is its missing
prerequisite). Point 3 meets the IO-actor named question head-on — this
session arrived at that deferred question three separate times (unified
virtual time, the multi-effect admin face, the process-backed test clock),
which is recorded here as evidence it should come early in the call-sugar
pass. Point 4 collides with a first-pass decision (the `waitfor` main-only
rule) — that collision is T-5. Point 6 collides with the one-effect-per-
handler shape — that collision is T-4.

## 1. Fixed points — already decided, inherited here

- **Run-to-completion: a handler cannot block mid-body** (C-5(d), the
  direction). Sleeping can therefore only mean *parking a continuation to be
  fulfilled later* — a blocking `sleep(millis)` member is anti-model, and this
  constraint dictates the Timer surface (`after` takes a `Reply`). §0 point 4
  knowingly reopens one corner of this; flagged there and worked in T-5.
- **Reply sends never block** (reserved reply capacity, C-3): a timer *firing*
  is `done.send(Fired {…})` — an enqueue — so whatever fires deadlines can
  never be wedged by a full mailbox.
- **The first-pass surface is frozen**: `send fn` members only, explicit
  `replyto`/`replyto!`, `r.send(v)`, `Addr<T>`, `use addr`, spawn-site `use`
  clause, `on POOL` (the mailbox is the handler's `mailbox { capacity: … }`
  slot since 2026-09-16), and `waitfor` **legal only in `main`** —
  main's sole token source. T-5 is the proposal to amend the last item.
- **A handler implements one effect** (the shape today). The pure-Salvo test
  timer therefore needs a forwarding handler between its `Timer` face and its
  control protocol — T-4 is the proposal to remove that.
- **The IO-actor named question is deferred to the call-sugar pass**
  (CONCURRENCY.md, "Carried to the call-sugar pass"): may an ordinary
  (non-`send`) member be process-backed, so a synchronous-looking call parks?
  The unified test clock is its first concrete customer (T-2, T-5).
- **The scheduler already commits to quiescence detection** (SUPERVISION.md
  S-3(i)): the idle-with-parked-gates report — all queues empty, no activation
  running — ships as a first-pass runtime diagnostic. T-3 exposes the same
  fact as a subscribable event; the detection machinery is not new.
- **`watch` is the precedent shape for runtime-delivered events** (S-2):
  a one-shot linear `Reply<Exit>` token the runtime consumes when the event
  happens; dropping the registration is a leak diagnostic [linear-obligation].
- **Linear tokens live in collections** (LINEARITY_COLLECTIONS.md): the
  pending-reply map of the timeout pattern and ManualTime's deadline list are
  this design's customers — a `Reply` inside a struct inside a `Mut List` /
  `Mut Map`, moved out by linear extraction.
- **The program ends when `main` returns** (the main-boundary decision):
  pending timers die with the program; no ambient keep-alive, consistent with
  no-run-to-quiescence. Tokens whose target died are silent no-ops (S-3).
- **Interop is unchanged** [intrinsic-std-only] / [platform-effect]: std's
  timer reaches the platform as an `intrinsic`; nothing here adds an interop
  path. Scheduler-adjacent machinery lives in **backend runtime files**, and
  anything needing compiler-specific cooperation is flagged to the user first
  (the implementation-placement decision).
- **Sendability is C-4 = (a)**: tokens and message payloads must be
  structurally sendable; `Reply<Fired>` crossing into the timer is ordinary.
- **The deadlock statics are the effect-graph cycle check**
  [async-deadlock-cycle]: gate cycles are errors, back-pressure cycles are
  warnings. T-5, if taken, adds a third edge kind (a true blocking wait).
- **Example 4(d)'s timeout fallback** (`await_within` → `Reply | TimedOut`)
  is recorded but unbuilt; it presumes a timer. The hand-written form —
  two bare `replyto` mints racing into a pending map, loser finds `None` —
  was worked in this session and needs only T-1 to become writable. A
  gated timeout needs the gate member-set generalization (a recorded later
  sugar pass); the ungated form does not.

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

§0 points 1–2; the prerequisite for Example 4(d)'s timeout fallback.

The surface, dictated by run-to-completion (a fixed point, not a choice):

```
// std, core.time
struct Fired { at: Int }                 // fires carry the fire-time — see T-2
effect Timer {
    // Consume `done` when at least `millis` have passed.
    send fn after(millis: Int, done: Reply<Fired>)
}
```

No cancellation in the first cut: a "cancelled" timer fires into a
continuation whose pending-map `take` finds `None` — one no-op activation, the
same discipline as a lost race. A cancel handle is a later addition if the
no-op cost ever measures.

What backs `after`:

- **(a) Time-gated replies in the scheduler kernel** — "enqueue this at time
  T" as a scheduler primitive. *Cost:* the kernel grows a fifth piece; a
  delayed message breaks the one-arrival-order-queue rule (needing a separate
  deadline structure internally anyway); and it escapes the effect discipline
  — code could schedule delayed work with nothing in any signature, the
  hidden dependency the language exists to forbid. Unfakeable from Salvo.
- **(b) An intrinsic Timer effect in std, backed by the scheduler runtime
  library** — the deadline heap + one waiting thread (Rust) or a
  `ScheduledThreadPoolExecutor` (JVM), per §2. The effect surface means a
  **pure-Salvo fake** (`ManualTime`, § Worked setups) costs nothing.
  *Cost:* one runtime thread on Rust; the timer becomes the first intrinsic
  handler *for an async effect*, so it is the test case for how intrinsics
  meet the scheduler runtime — flag per the implementation-placement rule.
- **(c) A platform effect** — customers declare their own. Wrong division of
  labor: time is as std-worthy as the filesystem, and std's primitives are
  `intrinsic` by rule. Listed to rule out.

**Recommendation (user leaning 2026-09-16: (b)):** (b). The fake-for-free
argument decided it in session; recorded here with (a)'s costs spelled so the
choice is auditable.

## T-2. Clock, and how it agrees with Timer in tests

§0 point 3. The natures differ, so the split is two effects with different
bindings — this half is not really open:

```
effect Clock {
    fn now() -> Int        // monotonic millis; ordinary member, ordinary return
}
```

`Clock` is a synchronous effect, `use`-bound, running on the caller's thread —
mechanically supported today. `Timer` is async (T-1). Production coupling is
trivial: `DefaultClock` and `DefaultTimer` intrinsics read the same OS
monotonic source. Wall-clock time (dates, zones) is a separate, larger std
design and out of scope here.

The open question is **test coupling**: one virtual time, two effects, and no
shared mutable state to point them at. Options:

- **(1) Time as data.** `Fired {at}` carries the fire-time; requests are
  stamped at the system's edge and timestamps travel in messages. Most tested
  code then needs no `Clock` at all, and `ManualTime` alone determinizes it.
  *Cost:* an architectural stance, not a mechanism — code that wants ambient
  `now()` mid-handler must be restructured. *Benefit:* zero machinery, and
  the actor model quietly prefers it anyway (time observed mid-activation is
  a race with your own mailbox).
- **(2) Scripted per-process `TestClock`.** Constructor takes the readings.
  Works today; fine for code that measures locally; diverges from
  `ManualTime`'s virtual time, acceptable when a test does not cross both.
- **(3) One owner: `Clock` backed by the timer process.** `now()` looks
  synchronous but consults `ManualTime`. Two possible mechanisms, each a
  decision of its own: the *park* (the IO-actor named question — call-sugar
  pass) or the *block* (T-5's lifted `waitfor`). Perfect agreement, one
  virtual time.
- **(4) Scheduler-owned virtual time** (§2 kotlinx/Tokio; § Worked setups,
  Test C). Solves coupling *and* the advance race in one mechanism, rebinding
  nothing (the same `DefaultClock`/`DefaultTimer` intrinsics read the pool's
  clock). *Cost:* time moves into the runtime; the pure-Salvo fake is
  forfeited; quiescence-aware advance must be implemented identically on both
  backends.

**Recommendation (the user's call):** ship the split with (1) as the std
design stance (fires carry `at`; err toward delivering time as data), keep
(2) for local measurement, and treat (3) as the principled endpoint — now
with two candidate mechanisms, T-5 (near-term, user is open to it) and the
IO-actor park (call-sugar pass). Hold (4) as the upgrade path if
duration-measuring tests or advance ergonomics bite. Note T-3 removes (4)'s
*sequencing* advantage, narrowing what (4) uniquely buys to clock unification.

## T-3. The quiescence hook — `on_idle`

§0 point 5 (**user leaning: support**). The S-3(i) detection already exists in
the committed first-pass scheduler; this exposes it through S-2's `watch`
shape:

```
// One-shot: the runtime consumes `notify` when the pool reaches quiescence —
// no activation running, all queues empty, nothing in flight.
send fn on_idle(p: Pool, notify: Reply<Idle>)

struct Idle {
    parked_gates: Int,     // 0 = truly done; >0 = idle but waiting (S-3's data)
    parked_tokens: Int
}
```

Same properties as `watch`, for the same reasons: linear token (a dropped
registration is a leak diagnostic); edge-triggered one-shot (delivering to a
process member ends the idleness — correct); a token targeting `main`'s
`waitfor` wakes the real thread without perturbing the pool.

What it buys: the virtual-time **advance race** — `ctl.advance(…)` competing
in arrival order with an in-flight `after(…)` registration — dissolves into
two lines of test code (§ Worked setups, Test A), keeping time itself in pure
Salvo. Without the hook, the only pure-Salvo mitigation is counted advance
(`advance_when(scheduled_count, millis)` re-enqueuing itself until the count
is met), which encodes *how many* timers the code under test sets — brittle
in exactly the way quiescence is not.

- **(a) The token form** (above). Uniform with `watch`; usable by a
  supervisor process, not just `main`; composes with the later merge/join
  sugar. *Cost:* the first use of two outstanding main-bridge tokens — nested
  `waitfor` (outer token already moved into the system, inner blocking
  inline) must be blessed by the `waitfor` rules.
- **(b) A main-only blocking form**, `p.await_idle()`. Simpler; no nesting
  question. *Cost:* breaks token uniformity; unusable from processes; a
  second main-only construct beside `waitfor`.
- **(c) None** — counted advance as the test discipline. *Cost:* the
  brittleness above; and S-3's detection exists anyway, so the saving is only
  the registration API.

Caveats inherited from S-3, not new: idleness is only meaningful while `main`
is the sole external injector (a platform handler with a real thread — a
socket — can stale the answer); and a blocked/parked activation counts as
non-idle, so the hook and T-5's blocking waits interact benignly (idle simply
does not fire while a blocking call is in flight).

**Recommendation (the user's call, leaning recorded):** (a), resolving the
nested-`waitfor` question alongside it; (b) acceptable as a first cut if that
question wants deferring.

## T-4. Handlers of multiple effects

§0 point 6 (**user leaning: support**). Today's shape forces the forwarding
split: `ManualTimerCore of TimerCtl` owning state plus `ManualTimer of Timer
[TimerCtl]` forwarding into it — same runtime shape, pure boilerplate, and
the pattern generalizes ("a public face and an admin face": health checks,
draining, stats — the test control here is one instance).

```
handler ManualTime of Timer, TimerCtl { … }

let (timer, ctl) = spawn ManualTime() on pool(1)
//   ^Addr<Timer>  ^Addr<TimerCtl> — least authority falls out of the types:
//   production code holding `timer` cannot name `advance`.
```

- **(a) Multi-effect handlers, spawn yielding one addr per effect** (above).
  `use H(…)` binds all implemented effects in scope; per-effect shadowing
  already works. No new types: no intersection addrs, no subtyping. The
  deadlock graph [async-deadlock-cycle] keeps its effect-keyed nodes — two
  nodes that happen to be one process is the same conservative approximation
  the type-level graph already makes. *Cost:* grammar + resolver work;
  exhaustiveness accounting at `use`/spawn sites covers a set.
- **(b) Status quo: the forwarding pattern.** Works today (the forwarder is
  constructed inside the consuming process via the spawn-site `use` clause —
  not an extra process). *Cost:* boilerplate per protocol pair, forever; the
  admin-face pattern stays annoying.
- **(c) Intersection-typed addrs** (`Addr<Timer & TimerCtl>`). More
  expressive (one value, both faces) but imports intersection types into a
  language that has unions only. Rejected as over-machinery; (a)'s tuple
  gets the value without the type.

Precedent: Cloud Haskell's several-typed-ports-per-process — the C-2
alternative that lost to the union surface, readmitted here in the harmless
form (multiple *protocols*, still one arrival-order queue per process).

**What it does not buy** (recorded because the session established it): none
of option 4's benefits — the advance race is a property of asynchrony, not
handler shape, and clock coupling is a property of the sync/async boundary.

**Recommendation (the user's call, leaning recorded):** (a).

## T-5. Lifting `waitfor`'s main-only restriction

§0 point 4 (**user: would consider**). The collision: the first pass rules
`waitfor` legal only in `main`. The motivating customer is the test clock —
`TestClock of Clock [Timer]` whose `now()` waits for `after(0, …)`'s `Fired`
and returns `.at`: production binds the intrinsic `DefaultClock`, tests bind
this, and virtual time has one owner with no runtime machinery and no
call-sugar dependency.

**Why it is not the self-deadlock it first looks like.** A `replyto`-style
wait inside a handler *would* self-deadlock: the reply is a mailbox message,
and the blocked activation is exactly what stops the mailbox being served.
`waitfor` is different by construction — it is the main-boundary bridge, so
its token wakes the blocked **real thread directly**, out-of-band of any
mailbox (that is how `main`, which has no mailbox, receives at all). Inside a
handler it therefore *works* — and what it costs is precisely what
run-to-completion paid to avoid:

- **A pool thread is held for the wait's duration.** On a one-thread pool
  whose fulfiller shares the pool, that is a certain wedge; generally, N
  simultaneous waits on an N-thread pool wedge it. The fulfilling process
  must effectively live on a different pool (or capacity must be budgeted).
- **The waiting process serves nothing meanwhile** — a gate in effect, but
  without the gate's static visibility, unless surfaced (below).
- **The deadlock graph needs a third edge kind**: a true blocking wait, at
  least as strong as a gate edge (error class when cyclic), plus the
  pool-capacity hazard, which is not a graph property at all (pools are
  values; the same topology wedges or not depending on `pool(n)`).

Options:

- **(a) Keep main-only.** The test clock waits for the IO-actor park
  (call-sugar pass); tests meanwhile use time-as-data (T-2(1)). *Cost:* the
  unified pure-Salvo clock is deferred, possibly long.
- **(b) Lift it, surfaced as a handler dependency.** `waitfor` outside `main`
  is legal only in a handler that declares it — spelled like the lowercase
  effects (`handler TestClock of Clock [Timer, waitfor]`). Handler
  dependencies are invisible to callers by existing rule
  [effect-handler-deps], so the `Clock` *effect* and every call site stay
  clean, and the blocking hazard keys on the **binding site** — the same
  resolution sketched for the IO-actor named question. The deadlock checker
  reads the new edge from the declaration; the pool-wedging hazard is
  documented (and cheaply runtime-detected: all threads of a pool blocked in
  `waitfor` = a named runtime error, the S-3 report's sibling). *Cost:* the
  substrate's "no blocking mid-body" promise gains an explicit, grep-able
  exception; misuse on small pools is a runtime failure, not a static one.
- **(c) Lift it unrestricted** — `waitfor` anywhere, nothing declared.
  *Cost:* invisible blocking edges; the deadlock graph goes blind exactly
  where the language promises visibility. Rejected on the visibility
  principle (the same one that rejected silent async colouring).

**Recommendation (the user's call):** if the unified test clock is wanted
before the call-sugar pass, (b) — the declared form keeps every hazard at a
declaration the checker and the reader can see, and its machinery (the edge
kind, the pool-exhaustion report) is small. Otherwise (a): the IO-actor park
is the principled endpoint (it parks instead of holding a thread), and this
session's triple arrival at that question is itself an argument for just
prioritizing it. (b) and the park are not exclusive — (b) could ship for
tests and be quietly superseded when parking exists.

## Worked setups — production vs test

The options above, made concrete on the credit-check-with-timeout service
(the session's running example: `OrderService` races a `CreditCheck` reply
against a 2000ms deadline via two bare `replyto` mints over a pending map).

### Production

```
fn main() [use, spawn] {
    use DefaultClock()                          // intrinsic: OS monotonic
    let timer  = spawn DefaultTimer() on pool(1)
    //           intrinsic: deadline heap + one waiting thread (Rust),
    //           ScheduledThreadPoolExecutor (JVM) — §2
    let credit = spawn CreditBureau(...) on pool(4)
    let orders = spawn OrderService(credit) [timer] on pool(4)
    // ... serve; the program ends when main returns — pending timers die with it
}
```

### Test A — pure Salvo: `ManualTime` + `on_idle` (T-1(b) + T-3 + T-4)

Everything in Salvo except the committed S-3 idle detection. Deterministic
*ordering*; time never really passes.

```
handler ManualTime of Timer, TimerCtl {          // T-4: two faces, one state
    now: Mut Int = 0,
    scheduled: Mut List<Scheduled> = []          // linear tokens in a collection

    send fn after(millis: Int, done: Reply<Fired>) {
        scheduled.add(Scheduled {at: now + millis, done: done})
    }
    send fn advance(millis: Int) {
        let target = now + millis
        for s in scheduled.take_where((s: Scheduled) -> s.at <= target).sorted_on_at() {
            now = s.at
            s.done.send(Fired {at: s.at})        // an enqueue — never blocks
        }
        now = target
    }
}

fn main() [use, spawn] {
    let p = pool(1)
    let (timer, ctl) = spawn ManualTime() on p
    let credit       = spawn SilentCredit() on p    // never answers
    let orders       = spawn OrderService(credit) [timer] on p

    let outcome = waitfor result: Reply<Placed | Rejected Str> {
        orders.place(order(7), result)
        waitfor idle: Reply<Idle> { on_idle(p, idle) }   // T-3: drain first —
        ctl.advance(2000)                                 // the race cannot exist
    }
    // outcome == rejected("credit check timed out"), always
}
```

Without T-3, the `advance` races the in-flight `after` registration and the
test can hang; the pure-Salvo fallback is counted advance (`advance_when`),
recorded in T-3 as the brittle alternative.

### Test B — the unified clock via lifted `waitfor` (adds T-5(b))

For code that *measures* — `rejected("timed out after ${now() - started}ms")`
— clock and timer must agree. `TestClock` waits on `ManualTime` for the
virtual time; the declared `waitfor` dependency keeps the blocking visible at
the binding, and the timer lives on its own pool so the wait cannot wedge the
system under test.

```
handler TestClock of Clock [Timer, waitfor] {    // T-5(b): dependency, not colour
    fn now() -> Int {
        let f = waitfor fired: Reply<Fired> {
            after(0, fired)                       // "fire immediately" = ask the time
        }
        return f.at
    }
}

fn main() [use, spawn] {
    let tp = pool(1)                              // the timer's own pool — T-5's
    let p  = pool(1)                              //   wedge hazard, budgeted away
    let (timer, ctl) = spawn ManualTime() on tp
    let credit       = spawn SilentCredit() on p
    let orders       = spawn OrderService(credit) [use TestClock(), timer] on p

    let outcome = waitfor result: Reply<Placed | Rejected Str> {
        orders.place(order(7), result)
        waitfor idle: Reply<Idle> { on_idle(p, idle) }
        ctl.advance(2000)
    }
    // outcome == rejected("credit check timed out after 2000ms") — exactly:
    // `started` read virtual 0, the fire happened at virtual 2000, one clock.
}
```

The same test under (a)-not-lifted: replace `TestClock` with time-as-data
(stamp the request at ingress; the continuation has `Fired.at`) — T-2(1) —
or wait for the IO-actor park, under which `TestClock`'s `now()` parks
instead of blocking and the `[waitfor]` dependency disappears.

### Test C — the upgrade path: scheduler-owned virtual time (T-2(4), not taken now)

Recorded for contrast: `let vt = test_pool()` — the pool owns a virtual
clock; the *production* `DefaultTimer`/`DefaultClock` intrinsics read it, so
tests rebind nothing; `vt.advance(2000)` means "run to quiescence, then move
time", and an auto mode jumps idle+pending-deadline straight to the deadline
(the kotlinx/Tokio shape, §2). Subsumes Test A's hook *and* Test B's clock in
one mechanism — at the cost of moving time into the runtime on both backends
and forfeiting the pure-Salvo fake. The narrowing effect of T-3 is worth
restating: with `on_idle` shipped, (4)'s unique remainder is clock
unification, which T-5(b) or the IO-actor park also deliver.

## Decisions pending

| Label | Question | Answers | Recommendation / recorded leaning |
|---|---|---|---|
| **T-1** | What backs the Timer effect | §0.1–2; Example 4(d)'s prerequisite | **(b)** intrinsic effect over a runtime deadline heap — *user leaning 2026-09-16* |
| **T-2** | Clock surface + test coupling stance | §0.3; the IO-actor named question's first customer | Sync `Clock` effect; time-as-data as the std stance; unified clock via T-5 or the park; scheduler time held as upgrade path |
| **T-3** | The quiescence hook | §0.5; S-3(i) exposed via S-2's shape | **(a)** token form `on_idle(p, Reply<Idle>)` — *user leaning: support*; resolves nested-`waitfor` alongside |
| **T-4** | Handlers of multiple effects | §0.6 | **(a)** spawn yields one addr per effect; no new types — *user leaning: support* |
| **T-5** | Lifting `waitfor` main-only | §0.4; collides with a first-pass rule | **(b)** if wanted before call sugar: legal only under a declared handler dependency, new deadlock-edge kind, pool-exhaustion runtime report; else defer to the IO-actor park |

Load-bearing order: **T-1 first** (everything is downstream of a timer
existing); **T-3 next** (nearly free — the detection is committed — and it
de-risks every test story); **T-4** any time (independent); **T-5 before
T-2's coupling stance is finalized**, since it decides which unified-clock
mechanism is near-term. The IO-actor named question stays owned by the
call-sugar pass, with this document as three-time-arrived evidence for its
priority.

Also owed with implementation, whatever is decided: fresh rule labels
(suggested: `[time-timer]`, `[time-clock]`, `[sched-on-idle]`,
`[handler-multi-effect]`, `[waitfor-handler]`) in LANGUAGE_SPEC.md and the
backend specs; `core.time` in std; and this document's deletion once the
outcomes live in COMPLETED.md's decision log.
