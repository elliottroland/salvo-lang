# Shareable handlers — monitors, mixed handlers, and the future of `[waitfor]` (working document)

Status: **OPEN** — every decision below is the user's (AGENTS.md's first
invariant). Written 2026-09-17, the evening the second sequence's steps 1–2
landed, out of a conversation that began at "how do I implement `Random` when
its state must live behind a thread boundary" and worked through four designs,
two of which survive below. The document follows DESIGN_DOC.md's shape —
options, trade-offs, recommendations — and carries the delete-when-built
charter of TIME.md / FREE_CONCURRENCY.md: when its outcomes are decided and
built, the decisions go to COMPLETED.md's log, the rules to the specs, and
this file is deleted.

Sources: LANGUAGE_SPEC.md's actor chapter — [actor-kind], [actor-effect-kind]
(the mixed-kind refusal this document proposes to amend), [actor-sendable],
[actor-replyto], [actor-use-addr] (the forwarding stub the façade builds on),
[actor-deadlock-cycle] (the graph this document extends), [waitfor-effect] /
[waitfor-dedicated] / [waitfor-pump] / [main-pool] (step 1), [free-send-fn] /
[task-mint] / [task-pool-inherit] / [pool-fault-sink] (step 2); LANGUAGE.md's
"Where work runs"; and the runtime mechanics in both backends'
`runtime/scheduler.{rs,kt}`, which §2 walks through because every deadlock
below is a fact about them.

## 0. The stated intent

The user's direction from the 2026-09-17 conversation, numbered so the
decisions below can be judged against it.

1. **Handler state should be confinable behind `send fn` members**, so that a
   handler's mutable state is touched only by serialized activations — and
   the handler may then also have ordinary (sync) members, which run on the
   caller's thread as all sync functions do, and *wait* on the send members'
   answers. The motivating case: `CyclicRandom` implements the plain effect
   `Random` — whose `next()` declares no `[waitfor]` — even though its
   changing cursor must live behind a thread boundary.
2. **Hidden occupancy is acceptable** (user, in session): "the general rule
   of a function call is that it will occupy the thread until it returns."
   Occupancy is the norm of synchronous calls; what is new is only that the
   occupied thread's pool keeps progressing. So neither the plain effect's
   member, nor the handler, nor its callers should have to declare
   `[waitfor]` for this to work.
3. **Deadlock detection is wanted** — the user is "all for" it — but the
   argument "handlers would need to be each other's dependencies, which is
   impossible" was tested in session and does not hold (§3.1: request
   topology is not dependency topology, because addrs travel as values).
4. **A second mechanism surfaced during the analysis** and was accepted as
   the likely better fit for the motivating case: *synchronized* members —
   a lock, not a servant — restricted to pure state transformation (§4).
5. The conversation also put two adjacent questions on the table, recorded
   here because their answers shape the design: whether `Addr` should
   survive at all once handlers are shareable (§5 — it should, and the
   façade *is* its generalization), and whether `[waitfor]` should remain a
   declared effect (§6 — the load-bearing open question).

## 1. Fixed points — already decided, inherited here

- **Run-to-completion; an activation cannot be suspended mid-body**
  [actor-kind]. Everything below composes waits by *nesting on a stack*,
  never by suspending.
- **The wait rule** [waitfor-pump]: *a `waitfor` serves its own pool's tasks,
  and its pool's activations except those of the actor doing the waiting.*
  Blocking is the empty-queue degenerate case. §2 spells out the mechanism,
  because each deadlock is a corollary of one clause of this rule.
- **The mixed-kind refusal** [actor-effect-kind]: `actor effect` members are
  all send-kind; a sync member beside them was refused because "the same
  handler state would be reachable from two threads, and a synchronous reader
  can observe state mid-activation". §3's confinement rule removes exactly
  that reason, which is why amending the refusal is coherent rather than a
  carve-out. The refusal of **full** unification (every member a `send fn`)
  stays: that was a different question.
- **Addrs are first-class values** [actor-use-addr] [actor-types]: freely
  copied, stored in state and collections, sent in messages. This is what
  makes §3.1's deadlock constructible with an acyclic dependency graph — and
  what most of the language's topology (meshes, routers, supervision,
  sharding, `watch`) is built on.
- **The deadlock graph** [actor-deadlock-cycle]: nodes are actor protocols;
  a gate is a wait-for edge (cycles are errors), an ordinary send a
  back-pressure edge (cycles are warnings), a declared `[waitfor]` a block
  edge (errors; step 1). Sound, not precise, over types not instances.
- **Tasks** [free-send-fn] [task-mint]: a task belongs to no actor, so a wait
  inside a task body excludes nothing; a task's captures are checked sendable
  at the mint. The façade value below must satisfy the same sendability rule.
- **The whole-program stance**: "Salvo assumes it can see everything." Every
  inference this document leans on (occupancy through façades, §3.3) is
  ordinary under that stance; nothing here requires modular analysis.

## 2. The scheduler mechanics that decide every case below

Everything in §3 is a corollary of five concrete facts about
`runtime/scheduler.rs` / `.kt`. Read this section once and each deadlock
walkthrough becomes an exercise in pointing at the fact that pins each thread.

1. **An actor's activation holds its `running` flag for the whole body** —
   including across a nested wait. `run_job` sets `running = true`, releases
   the scheduler lock, and runs the member body on the worker's stack; if
   that body waits, the wait happens *inside* the still-running activation.
   `deliverable()` answers `None` for a running actor, so **an occupied
   actor's mailbox is served by nobody** — not by other workers, not by other
   waiters' pumps, not by its own wait (which additionally excludes its own
   actor explicitly). "The mailbox is stalled" is this flag, nothing subtler.
2. **A wait is a pump loop, not a park.** `salvo_wait` repeatedly: checks its
   token; runs a queued **task** of its own pool; else runs a deliverable
   **activation** of its own pool, excluding the waiting actor's own; else
   checks `idle()`; else parks on the condvar until anything changes. So a
   waiting thread is a worker for its pool — with the one exclusion — for as
   long as it waits. `main` is the degenerate case: not an actor, so its wait
   excludes nothing, and its pool has no other worker [main-pool].
3. **Waits pump; sends and locks do not.** A sender blocked on a full mailbox
   sits in `salvo_send`'s condvar loop serving nothing. A thread blocked on a
   host mutex (§4) is inside the OS, invisible to the scheduler entirely.
   Only `salvo_wait` serves while waiting.
4. **Two kinds of token, two delivery paths.** A `waitfor` token targets a
   **waiter slot** tied to the parked thread — the wait is on the *thread*,
   not on any mailbox, even when it is nested inside an activation.
   Fulfilling it writes the value into the slot and signals the condvar;
   nothing queues and nothing is scheduled, because *the waiting thread is
   its own pickup*: its pump loop re-checks its slot at the top of every
   iteration — **before** serving a task, before serving an activation — so
   fulfilment preempts all queued work and the waiter resumes at its next
   pass. A `replyto` continuation targets the minting actor's **mailbox**:
   it is delivered as a reply *entry* and runs later as an ordinary
   activation, in arrival order (under a gate, it becomes the only
   deliverable entry). "Reserved capacity" means only that a reply entry
   never blocks its sender and never counts against the bound — not that it
   skips the queue. §3.1's deadlock lives exactly in this split: the peer's
   own wait would resume instantly if its token were fulfilled (path one),
   but the fulfilment sits inside a continuation that must be *delivered as
   an activation* of the occupied peer (path two), and path two is what the
   `running` flag stalls.
5. **The idle report has a hole for occupied waiters** (reproduced
   2026-09-17; the repro is in ROADMAP's open defects). `idle()` requires
   `active == 0`, and an activation parked in a nested wait still counts in
   `active` — so any deadlock involving an occupied actor hangs silently
   instead of producing the named report. Every hard case below currently
   dies as a hang, not a diagnostic. Fixing this (count activations parked in
   `salvo_wait` out of `active`, or track them separately) is a
   **prerequisite** of everything in this document: with hidden waits, the
   occupied-waiter deadlock becomes the common failure shape, and the runtime
   net must fire for it.

### 2.1 Considered: one mailbox for everything (waits unified into deliveries)

Asked by the user at this document's revision: should `waitfor` and `replyto`
be unified — always a mailbox, with `main` owning an unbounded one? Worked
through and **rejected as semantics, recorded as available (and pointless) as
representation**; kept here because the analysis sharpens why fact 4's split
exists.

- **The split is two control-flow shapes, not two delivery mechanisms.** A
  `replyto` continuation is deliverable as a mailbox entry *because its frame
  is gone* — the activation ended at the mint, and the answer starts a fresh
  one. A `waitfor`'s frame is **still alive** on a thread's stack; nothing can
  be "delivered into" a live frame. Any unification must therefore keep the
  pickup discipline "the waiting thread notices its own answer" — the slot is
  that discipline with the scan optimized away.
- **The naive version deadlocks every occupied wait by construction.** While
  an actor's activation waits, its `running` flag stalls its mailbox (fact 1).
  Route the wait's own answer through that mailbox and no wait inside any
  actor can ever complete — the answer sits in a queue that is stalled
  *because* the waiter is waiting. To recover, one must add "a waiting actor's
  thread may take exactly the awaited entry from its own stalled mailbox" —
  which re-derives the waiter slot as a mailbox mode, with a queue scan where
  a slot read was. The bypass is not an optimization; it is what makes an
  occupied wait completable at all.
- **What the coherent version would buy: nothing observable.** Main as an
  actor with an unbounded mailbox permanently gate-scanned by its innermost
  wait is semantics-preserving relabeling: fulfilment is already non-blocking
  on every path (slots, reserved reply capacity, task queues), nested waits
  already compose (each checks its own slot; an outer answer waits in its slot
  until the inner completes), and `main` still has no members, so its mailbox
  could hold nothing but reply entries — a bag of slots, renamed, plus scan
  machinery. The unbounded part changes no blocking behaviour anywhere, and
  bounded *user* mailboxes stay load-bearing (back-pressure is the designed
  behaviour, §3.4).
- **The genuine unification runs the other way** and is already the recorded
  endpoint: the sugar pass's **parking call member** — `replyto` everywhere,
  waits respelled as parked continuations, at which point the waiter machinery
  shrinks toward `main`, dedicated threads and FFI bridges. Unify by making
  waits into parks (frames end), not by making parks into deliveries (frames
  would have to survive delivery, which is the suspension machinery the
  `yield fn` deletion deliberately removed).

## 3. Design A — the mixed handler (active object: confinement + façade)

The user's design (§0.1), amended in session on one point and extended on two.

**The confinement rule, corrected.** Handler state is accessible **only from
`send fn` members** — and sync members touch *no* state, not merely no
mutable state: every handler field is assignable inside members, and even an
immutable read from another thread races with an activation that assigns it.
So a sync member's environment is: its own parameters, the handler's
**constructor parameters** (immutable after construction, so freely
copyable into the façade), and sends to its own servant.

**What a mixed handler is.** One declaration; the send members and the state
form the **servant** (an ordinary actor: mailbox, arrival order,
serialization); the sync members form the **façade**, running on the caller's
thread. A sync member that needs an answer sends to its own servant and waits:

```
effect Random {                     // plain effect — no [waitfor] anywhere
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    cursor: Int = 0                          // servant-only, by the rule

    send fn advance(out: Reply<Int>) {       // the servant: owns the state
        cursor = (cursor * 1103515245 + seed) % 2147483648
        out.send(cursor)
    }

    fn next() -> Int {                       // the façade: caller's thread
        return waitfor got: Reply<Int> {
            advance(got)                     // a send to *own* servant
        }
    }
}
```

**The façade value.** What a consumer holds — and what may cross seams — is
the servant's addr plus the constructor parameters plus dispatch to the sync
bodies: copyable and sendable by construction (an addr is an index; ctor
params are checked sendable at spawn). This is `Addr<E>` *generalized*, not a
new kind of value (§5). No `Arc`, no lock, no shared mutation anywhere: the
state never leaves the servant.

**Consequences accepted in session:**

- A mixed handler is **spawn-only**: bound with `use`, its sync members would
  mint toward an inline servant with no mailbox — the existing
  parking-handler refusal, verbatim.
- **A getter costs a round trip.** A sync member reading one field must send
  and wait; there is no peeking. (This is what makes Design B worth having.)
- **Two façade calls are two activations** — no atomicity across them; a
  read-modify-write is one send member.
- A sync member's own arguments never cross a seam (it runs locally), so it
  may take `Mut` parameters and non-sendable values; only what it *forwards*
  to the servant must be sendable. Strictly more expressive than a send-only
  protocol.

### 3.1 Deadlock walkthrough I — the upcall (the case that decides the design)

The user's soundness argument was: a cycle would require two handlers to be
each other's *dependencies*, which construction order makes impossible. The
argument is right about dependencies and wrong about deadlocks, because
**request topology is not dependency topology**: an `Addr` travels in a
message after construction is done, and the waits-for graph is drawn by
whoever holds whose addr at run time.

The program (sketch): a registry façade whose servant *validates each answer
with a peer* before giving it out — and the peer, an ordinary actor, uses the
registry through the plain effect.

```
effect Registry { fn lookup(id: Int) -> Int }        // plain; no [waitfor]

actor effect RegState {
    send fn get(id: Int, out: Reply<Int>) => !id, !out
    send fn meet(peer: Addr<PeerApi>) => !peer       // the addr arrives here
}
// the servant's get(id, out): peer.validate(id, replyto checked(out))
//   — parks a continuation, activation ends, servant is FREE throughout

actor effect PeerApi {
    send fn validate(id: Int, ok: Reply<Bool>) => !id, !ok
}
// the peer's handler declares [Registry]; its validate calls lookup(id)
```

Construction is acyclic and unremarkable: spawn the servant; spawn the peer
with `Registry` bound to the façade; send `meet(peer)`. No declaration refers
to the other. Now the run, thread by thread. Call the threads **T-main**,
**T-peer** (the peer's pool worker), **T-serv** (the servant's).

| step | thread | what happens | why the scheduler allows it |
|---|---|---|---|
| 1 | T-main | `lookup(7)` → façade sends `get(7, got)` to servant, waits on `got` | main's wait pumps main's pool (empty) and parks — fact 2 |
| 2 | T-serv | runs `get(7)`: sends `validate(7, …)` to peer, **parks a continuation** `checked(got)`, activation **ends** | a bare `replyto` leaves the servant free — fact 4: the continuation is a mailbox entry, delivered later as an activation |
| 3 | T-peer | runs `validate(7)`: calls `lookup(7)` → the façade sends `get(7, got2)` to the servant and **waits on `got2`** | the sync member runs on the caller's thread; the peer's activation is now **occupied** — its `running` flag is held across the nested wait (fact 1) |
| 4 | T-serv | runs the second `get(7)`: sends `validate(7, …)` **to the peer**, parks `checked(got2)`, ends | the servant is still free; the send succeeds — the peer's mailbox has room |
| 5 | — | the second `validate` sits in the peer's mailbox **forever** | the peer is running (fact 1): `deliverable()` skips it for every worker and every pump. T-peer's own wait serves its pool *except its own actor* — its own mailbox is the one thing it will never serve (fact 2) |

Nothing progresses: `got2` is fulfilled by `checked(got2)`, which runs on the
servant **when the peer answers the second validate** — which requires the
peer to serve a mailbox entry — which requires the peer's activation to end —
which requires `got2`. T-serv idles (genuinely free, nothing deliverable),
T-peer pumps an empty pool inside `validate`, T-main pumps an empty pool
inside `lookup`. And by fact 5, `active ≥ 1` (the peer's occupied
activation), so the idle report never fires: **a silent hang on both
backends**, today.

Three things this case establishes:

- **Where the lock occurs**: nowhere exotic. It is the `running` flag of the
  peer's activation, held across the nested wait, making the peer's mailbox
  undeliverable — combined with the servant's answer being a *continuation*
  that must route through that mailbox. Every other thread is innocently
  idle.
- **Pumping does not help and cannot**: the peer's wait is forbidden from
  serving its own actor, and relaxing that would let a nested `validate`
  observe the handler mid-`validate` — the exact re-entrancy serialization
  exists to prevent. The exclusion is load-bearing; the deadlock is its
  corollary.
- **Hiding is not the culprit** (the user's later question, answered): write
  `[waitfor]` on every signature in this program and it deadlocks
  identically. Occupancy cycles are a property of *waits on cyclic request
  topology* — gates already produce this class fully declared. What hiding
  changes is only that the **graph** must infer the occupancy edge instead of
  reading it (§3.3), and that a human reading `PeerApi`'s handler cannot see
  it may stall. The user accepts the second (§0.2).

### 3.2 Deadlock walkthrough II — the shapes around it

**II-a. The one-hop self-deadlock.** An actor A calls a façade whose servant
sends back to A itself (A's addr reached the servant's state earlier, as a
payload). Step 3 and step 5 of §3.1 collapse into one actor: A is occupied in
the façade's wait; the servant needs A to serve a message before it can
fulfil A's token; A's mailbox is stalled by A's own `running` flag. Same
mechanism, minimal form. (A cannot reach its *own* façade lexically — a
handler calling a member of the effect it implements is refused today, with
`total@self` named as the remedy, verified in session — so this needs the
addr to travel; but addrs travel.)

**II-b. Two waiting handlers calling each other's façades.** Mixed handlers
H1 and H2; H1's sync member is called from inside one of H2's *send* members
and vice versa (each holds the other's façade, arrived as values). T1 runs
H2's send member → calls H1's façade → sends to H1's servant and waits,
occupying **H2**. T2 symmetrically occupies **H1**. Now H1's servant cannot
run H1's send members? — it can, H1's *servant* is a different actor from
H2's activation… the deadlock needs the answering path to route through the
occupied one: H1's servant's answer to T1's request requires (by its body) a
message served by H2 — which is occupied. Two occupied activations, each
stalling the mailbox the other's answer routes through. This is §3.1 with
the roles symmetric, and it is what a **wait-edge cycle** in the graph looks
like when both edges are inferred occupancy. Step 1's `[waitfor]`-block
cycle test is this shape with declarations; the inference (§3.3) must
classify it identically without them.

**II-c. The gate cycle (existing, for contrast).** Two actors `replyto!`
each other — the phase-5 committed example. A gated actor is *less* stalled
than an occupied one: it still serves its awaited reply (the gate filters the
queue rather than skipping the actor), so the deadlock needs the requests to
*cross*. An occupied actor serves nothing of its mailbox, so §3.1 deadlocks
on every run once the topology exists. This case is here to show the class
predates hidden waits — and that the existing "one ungated side downgrades
the cycle to a warning" rule must **not** be extended to occupancy: the
downgrade's argument (the ungated side keeps serving) is exactly what fact 1
removes.

**II-d. The main-pool variants (adjacent, not new).** An occupied actor *on
the main pool* is served only inside `main`'s waits, so a façade call from a
main-pool actor whose answer arrives after `main`'s last wait simply never
completes — the silent-death case already recorded under
[task-pool-inherit], wearing a new coat. And `main` itself calling a façade
is fine (main is not an actor; its wait excludes nothing) — unless the
servant's answer routes back through a *main-pool actor's* mailbox while that
actor is occupied, which is §3.1 with T-peer = a main-pool actor and T-main
its only server. Nothing new to detect; the same edges.

### 3.3 What the statics must do (Design A's checker obligations)

1. **Infer the occupancy edge.** A call to a sync member of a mixed handler
   is a wait on its servant's protocol: edge from the enclosing actor's
   protocol (or `main`/task, which contribute no node) to the servant's, at
   the call site, resolved through the binding in force there. Whole-program,
   so no declaration is needed — this is the user's §0.2 position, adopted.
2. **Classify occupancy like a gate, not like a send**: any cycle containing
   an occupancy edge is an **error** (see II-c for why the warning downgrade
   must not apply). Anchor the report at the sync-member call (the seam),
   naming the binding that chose the mixed handler, the cycle as a path, and
   the remedies: answer without consulting the peer; respell the sync call
   as a send-plus-continuation; bind a non-mixed handler.
3. **Keep the stated imprecision honest.** Over types, not instances; over
   member bodies' possible sends, not their data flow — a program that never
   sends the addr still gets the error (the servant's *body* can send to
   `PeerApi`; whether `meet` ever runs is invisible). Same trade the graph
   already documents, same remedy style.
4. **The runtime net under it**: fix fact 5's hole (the prerequisite), so
   that whatever the graph misses hangs loudly — the report should name the
   occupied actor and the member it is parked in.
5. **What stays invisible, stated**: stack depth (nested pumped waits pin
   frames; a deep façade chain across actors sharing a pool nests on one
   stack, and nothing bounds it), and senders blocked on full mailboxes
   (fact 3: they serve nothing while they wait — under load, the casualties
   queue up *behind* an occupied actor). Record both; neither has a cheap
   static answer.

### 3.4 Can deadlocks be made impossible, rather than detectable?

Asked by the user at this document's revision. Yes — deadlock freedom is a
well-studied guarantee — but every known form of it buys the guarantee by
making the waits-for relation **structurally well-founded**, and each
structure costs something Salvo currently sells. The honest menu:

- **Fragments Salvo already guarantees (or will, with this design).**
  *Monitors* (§4) are deadlock-free by construction: the restriction makes a
  lock innermost, so no thread ever holds one while wanting anything.
  And the **default actor fragment** — bare `replyto`, ordinary sends, no
  gates, no occupancy waits — has no unconditional deadlock at all: a bare
  mint leaves the minter free (§3.1's servant never blocks), so the only
  remaining cycle class is **back-pressure under load**, which is not a
  defect but the designed behaviour of a bounded mailbox (the "fix",
  unbounded queues, only moves the failure to memory). A program that stays
  in this fragment cannot write §3.1; occupancy is what the mixed handler
  *adds*, and it is added deliberately.
- **The full guarantee exists**, and has a Salvo-shaped form already parked
  in ROADMAP: **stratification** — a tier on addrs, with every *blocking*
  operation pointing strictly down the strata. Well-foundedness then makes a
  waits-for cycle unwritable, the same way the literature's session-typed
  calculi get it (tree-shaped communication; Kobayashi-style priorities are
  the generalization). To be a *full* guarantee it must discipline all three
  blocking primitives — the gate, the occupancy wait, and the send into a
  bounded mailbox — which means upward sends must be nonblocking (unbounded,
  or an overflow policy). The costs are exactly the language's selling
  points: peer meshes are same-tier cycles, supervision loops cross tiers in
  both directions, addr mobility must be level-checked, and the annotation
  burden lands on code that is currently free. Verdict: worth having as an
  **opt-in discipline** for programs that want the guarantee (the tier
  qualifier composes with everything here), wrong as the default.
- **Salvo's stance is already stronger than "detect".** Detection implies
  finding the deadlock when it happens; the graph *refuses at compile time*
  every unconditional cycle the type-level abstraction can see (gates,
  declared blocks, and — with SH-4 — inferred occupancy). That is a
  conservative guarantee over the type graph: what compiles has no
  type-level wait cycle, at the price of false refusals on instance-level
  patterns (the stated imprecision, for which stratification doubles as the
  escape hatch). Under it: warnings for the load-conditioned class, and the
  runtime net for what the abstraction cannot see (the attribution gaps
  around `main` and parked obligations; SH-8's report; the parked
  `Reply | TimedOut` timeout form as the OTP-style last resort).

So the answer to "is this always a risk": within the monitor and
bare-mint fragments, no — those are guarantees. Once occupancy or gates
enter, a global guarantee requires stratification's trade, and short of
adopting it the design's position is conservative refusal of everything
statically visible plus a runtime net that must actually fire (SH-8). That
is not "detection instead of prevention"; it is prevention over an
abstraction, with detection for the abstraction's stated blind spots.

### 3.5 The restricted façade — direct-answer servants (deadlock-free by construction)

Asked by the user at this document's revision: is there a version of the
monitor's trick for the façade? There is, and it slots in as a middle rung.
The monitor's restriction makes the lock **innermost** (holding it, you can
reach nothing that blocks); the façade's analogue makes the occupancy
**terminal**: the caller's wait depends on nothing but one activation of the
servant itself.

**The rule.** A mixed handler is *direct-answer* when its send members:

- **discharge every `Reply` parameter before the activation ends** — fulfil
  it (`out.send(v)`); never park a continuation toward it, never store it in
  state, never forward it as a payload, never capture it in a mint. All four
  escapes are already distinguishable to the checker (fulfilment is the
  intrinsic `send`; the other three are stores/sends/captures of a
  `Reply`-typed value, which linearity tracks today);
- **never occupy**: no `waitfor`, no calls to sync members of mixed handlers.
  (Fire-and-forget sends and token-free `k@self` stay legal — they defer
  *work*, not answers.)

**Why this is a guarantee.** A façade call's dependency chain is then depth
one and terminal: caller → one straight-line servant activation → token
fulfilled → waiter resumes (§2 fact 4: the token goes to the waiter slot
directly). The servant can always *run*: it is never occupied (rule two), a
different-pool servant has its own workers, and a same-pool servant is served
by the caller's own pump (the wait excludes only the caller's actor). In
graph terms: occupancy edges from direct façades point only at nodes with no
outgoing wait or continuation edges, so no cycle can contain one —
well-founded by construction, no analysis needed. §3.1 is unwritable: the
servant's `get` cannot park `checked(out)` to consult the peer, because
parking toward its `Reply` parameter is exactly what the rule refuses.

**The honest residue** (shared with the bare fragment, §3.4, and absent from
monitors): back-pressure. The façade's *request* is an ordinary send into a
bounded mailbox, and a full servant queue blocks the caller in `salvo_send`,
which does not pump (fact 3) — on a shared single-threaded pool that is the
same wedge shape the main-pool report already names. Under the restriction
the servant drains promptly, so the bound is a burst limiter rather than a
deadlock ingredient; the optional upgrade that closes even this is a
**pumping request send** for façade calls (serve your own pool while waiting
for queue room), a runtime-only change, or mint-time capacity reservation in
the servant's queue (the recorded remote-mint semantics). Decide with SH-9.

**What rung two adds over the monitor**, since both are
deadlock-free-by-construction and the monitor is cheaper: send members. A
direct-answer handler is an ordinary actor protocol *plus* safe sync getters
— fire-and-forget mutation (`record(x)` enqueues and returns), arrival-order
processing, a mailbox that other actors reach through effect lists, and one
`snapshot()` a caller can read synchronously. A monitor has no mailbox, no
send members, and blocks the caller for every operation. `CyclicRandom` fits
either; an aggregator with async writes and sync reads fits only this rung.

**The ladder, complete:**

| rung | mechanism | guarantee | fits |
|---|---|---|---|
| 1 | monitor (§4) | deadlock-free, no residue | passive state, short members |
| 2 | direct-answer façade (§3.5) | no occupancy cycles by construction; back-pressure residue | actors wanting sync getters; async writes + sync reads |
| 3 | full mixed handler (§3) | graph-checked (SH-4): inferred occupancy edges, cycles are errors | servants that aggregate — ask another actor, hold obligations, park |

The compiler can name the next rung down in each refusal: a monitor member
that needs a mailbox is told about rung 2; a direct member that must park is
told about rung 3; rung 3's price is the graph.

### 3.6 What rung 3 is for — answers that depend on future events

Asked by the user: what does the full mixed handler give that monitors and
direct-answer façades cannot? The dividing line is rung 2's rule itself: a
direct servant fulfils the token **in the activation that received it**, so
rung 3 is needed exactly when the answer depends on a *future event* — then
someone must hold the caller's token across activations, and storing or
parking a `Reply` is what rung 2 refuses. The recurring shapes:

| pattern | why rung 2 cannot | the future event |
|---|---|---|
| **`TestClock of Clock`** (TIME.md's customer) | `ManualTime.after` *stores* the token in `scheduled`; `advance()` drains the due ones — deferral is its semantics | virtual time reaching the deadline |
| **multiplexed connection** (`fn query(sql) -> Rows`) | the token parks in a `pending` map keyed by request id; a *different* activation (`frame_arrived`) answers | the response frame on the wire |
| **blocking acquire** (`fn acquire() -> Conn`), rate limiter, semaphore | when nothing is free the caller's token joins a waiter queue, drained by `release` | another caller releasing |
| **request coalescing** (`fn get(k) -> V`) | in-flight fetch → the token joins the crowd in `Mut Map<K, Mut List<Reply<V>>>`, all drained when the one fetch lands | the shared fetch completing |
| **barrier / latch / shutdown drain** | the token waits on a predicate moved by other send members | the condition becoming true |

**The "other way" exists and is not one**: every row has a plain-actor twin
in continuation style — `Desk` in `examples/actors/` *is* blocking-acquire —
but the twin makes the **caller** an actor passing tokens explicitly, which
colors the whole call chain, and a plain effect can never be its interface
(an ordinary member with a return type cannot be actor-backed — the decided
"forbid"). Monitors reach none of it: no effects, no waits, no mailbox. So
rung 3's unique purchase is **a synchronous signature over an
event-dependent answer** — `now() -> Int`, `query(sql) -> Rows`,
`acquire() -> Conn` as plain effects callable from ordinary code, the
asynchrony confined inside the handler instead of propagated to every
caller. Which is also why rung 3 and the graph are inseparable: an answer
deferred to a future event is an occupancy of unknowable duration — exactly
the edge §3.3 checks.

## 4. Design B — the monitor (synchronized members, restricted)

The second mechanism, surfaced when the getter round trip (§3, consequences)
met the motivating case: `CyclicRandom` does not need a servant, a mailbox,
or a thread boundary at all. It needs mutual exclusion around twenty
instructions.

**The rule.** A handler may declare **synchronized members** (spelling open —
SH-6): a synchronized member may touch state and may perform **no effect and
no wait** — it is a pure state transformation over its arguments and the
handler's state. Sibling calls compile within the one acquisition.

**Why the restriction is the whole design.** It makes the lock *innermost by
construction* — no thread ever holds it while acquiring anything else or
while parked — which eliminates every lock pathology **without any
analysis**:

- **Lock-order deadlock (the classic).** Unrestricted: T1 runs a member of
  A (holds A's lock), which calls an effect bound to B (wants B's lock);
  T2 symmetrically holds B and wants A. Neither can proceed; the cycle is
  over locks, invisible in signatures, and needs the same whole-program
  cycle analysis §3.3 builds — nothing would be saved over Design A.
  Restricted: a member performs no effect, so no thread holds one handler's
  lock while wanting another's. The interleaving cannot be written.
- **Lock-across-pump self-deadlock.** Unrestricted: an activation takes the
  lock, then waits; the wait *pumps* (fact 2); the pumped work — a task or
  another activation on the same pool, same thread — calls the same
  handler and wants the same lock. One thread, deadlocked against itself,
  and no other thread can help (the lock is held by a frame buried under
  the pump on this very stack). Restricted: no wait while holding, so a
  pump never runs under a held lock.
- **The lock wait that serves nothing.** Unrestricted, no pump involved: a
  thread blocked on a contended mutex is inside the OS (fact 3) — it is
  not a `salvo_wait`, so it pumps nothing. If the lock holder needs that
  blocked thread's pool to progress before it releases (say the holder
  waits on a task queued on `main`'s pool while `main` is the thread
  blocked on the mutex), the block is total. Restricted: a holder never
  waits, so a lock is only ever held for a bounded straight-line body, and
  a blocked acquirer waits O(one member body).
- **The re-entrancy parity trap.** JVM monitors are re-entrant; Rust's
  `Mutex` is not. An unrestricted member re-entering its own handler
  (via an effect that routes back) would deadlock on Rust and silently
  proceed on Kotlin — observing its own mid-mutation state, the exact
  thing serialization exists to prevent, and a [backend-never-wrong]-grade
  divergence. Restricted: no member can reach another handler, so no path
  routes back; re-entrancy is unwritable and the two backends' lock
  semantics never diverge observably.

**What it costs**: expressiveness, deliberately. The moment a member needs
to perform an effect, wait, park a continuation, hold an obligation, or
watch — it is not a monitor, and the diagnostic names Design A (or a plain
actor). The boundary is a refusal list at the declaration, checkable with
what the checker already has.

**Representation**: `Arc<Mutex<H>>` on Rust (the deferred C-4(c)
"Arc-where-sent" growth point, arriving with a design that actually needs
it), `synchronized`/`ReentrantLock` on Kotlin (re-entrancy unobservable
under the restriction). State must be sendable-at-rest (`Send` on the field
types); linear values in monitor state follow [linear-state]'s existing
rules. The monitor's *handle* is copyable and sendable like the façade's.

**Fit**: passive shared state — counters, caches, config, cursors, `Random`.
`CyclicRandom.next()` under Design B is: lock, advance, unlock, return — no
servant, no round trip, no `[waitfor]`, no occupancy, no graph node.

## 5. `Addr`, `spawn`, and `use H() on POOL`

Asked in session: once handlers are shareable, why keep `Addr` — why not
`use CyclicRandom() on pool` as *the* form? Answered, and recorded as the
design's shape:

- **The façade value is `Addr<E>` generalized**, not its replacement: an
  addr is the degenerate façade of a send-only protocol with no constructor
  parameters. One value kind, more members reachable through it. `Addr`
  keeps its name and its type argument (the effect — interchangeability at
  binding sites is the point).
- **`use H() on POOL` is wanted as sugar**, ≡ `let __a = spawn H() on POOL;
  use __a` — the dominant case (one shared instance serving an effect in a
  scope) in one line, and the natural spelling of "shareable handler".
- **The value-producing form cannot go.** A `use` binding is lexical and
  anonymous; dropping the handle-as-value would cost: multiple instances of
  one protocol (sharding over `List<Addr<ShardApi>>`), dynamic topologies
  (an actor per connection), handles in messages and state (meshes, routers,
  registries, supervisors), `watch` (needs a target identity), task captures
  (a task cannot capture a lexical binding), and later host bridging. A
  use-only world is structured concurrency with nothing first-class — a
  coherent language, not this one; three of `examples/actors/`' eight
  sections already lean on addrs.

## 6. Does `[waitfor]` survive? — the open question this design forces

Step 1 made `[waitfor]` a declared, propagating capability validated by
placement. Design A deliberately does **not** put it on the plain effect's
member, the mixed handler, or the callers — the user's §0.2 position — and
the pump rule (fact 2) already removed most of what dedicated placement was
protecting against (an occupied worker no longer starves its pool). What is
left of the declaration, case by case:

- **Not safety.** The upcall (§3.1) deadlocks with or without declarations;
  what prevents it is the graph, and the graph can *infer* occupancy
  whole-program (§3.3) — the declaration was a spelling of the edge, not its
  source.
- **Not the placement gate, mostly.** "May occupy this thread" needed a
  dedicated thread when a wait blocked; a wait that serves its pool is not
  the hazard the gate priced in. The genuine residue: a wait still pins a
  *stack frame* (depth is the budget), and a wait still stalls the waiting
  actor's own mailbox (the §3.1 mechanism) — but neither is addressed by
  placement, so the gate does not earn its cost against them.
- **Human visibility** — the one thing inference cannot give. The user's
  position: a call occupying its thread is what a call *is*; the language
  does not annotate ordinary blocking, so it should not annotate this. The
  counter-position (Locality): "this call may park your activation and stall
  your mailbox until another actor answers" is a bigger fact than "this call
  takes a while", and it is precisely the fact §3.1's author needed to see.
- **Three places it remains load-bearing regardless**: `main`'s own
  signature is harmless either way (its thread is dedicated by nature);
  **host/FFI bridges** (FC-7, parked) want a signature-level "this export
  blocks the calling thread", and a host thread cannot be inferred *into*;
  and **`thread()` + `Dedicated Pool`** keep their meaning — "exactly one
  occupant, by linearity" — for the cases that genuinely want a thread of
  their own (blocking-IO wrappers around host APIs), independent of any
  grant check.

**Options:**

- **(a) Keep step 1 whole**: `[waitfor]` declared and propagating; mixed
  handlers carry it; callers declare it; spawn sites need `thread()`.
  *Cost*: kills §0.1 — `Random`'s member would need `[waitfor]`, so a plain
  effect could never be backed by confined state; a shared façade costs a
  dedicated thread per consumer. Recorded as rejected by the intent unless
  the user reverses.
- **(b) Demote to inference + keep the vocabulary** *(recommended)*: drop
  the propagation requirement and the placement gate for occupancy; the
  graph infers occupancy edges (§3.3) and classifies them as errors in
  cycles; `[waitfor]` remains as an **optional, checked annotation** — a
  function or effect member *may* declare it, the checker verifies it
  against the inferred truth (declaring it falsely is a warning, hiding it
  is legal), FFI exports and `main` keep it, and `thread()`/`Dedicated
  Pool` stay for placement that is wanted rather than required. The
  deadlock net moves fully onto the graph + the fixed runtime report.
- **(c) Delete the effect**: pure inference, no vocabulary. *Cost*: FC-7's
  bridge story loses its spelling; TestClock-style handlers lose the
  documented hazard marker; and un-deciding a same-day decision without
  keeping even the optional form throws away a word the language will want
  at the FFI boundary. Not recommended.

The step-1 machinery is **not** wasted under (b): the edge kind, the pump,
the main pool, `thread()`, and the consumption rule all survive; what is
dropped is the *mandatory* grant check, which §2's facts show was pricing
the wrong hazard.

## 7. Decision surface

| # | Question | Options | Recommendation |
|---|---|---|---|
| SH-1 | Amend [actor-effect-kind]: mixed handlers (send members + state confined to them; sync members touch no state) | yes / no | **yes**, as stated in §3 |
| SH-2 | The façade value: `Addr<E>` generalized (stub + ctor params + sync dispatch), sendable; mixed handlers spawn-only | as stated / variants | **as stated** (§3, §5) |
| SH-3 | Monitors: synchronized members restricted to state + pure computation — no effects, no waits | yes / no / unrestricted-with-analysis | **yes, restricted**; unrestricted is rejected (§4: it re-imports every pathology plus a parity trap) |
| SH-4 | Occupancy in the graph: inferred through façades; cycles containing an occupancy edge are **errors** (no back-pressure downgrade) | as stated / declarations required | **as stated** (§3.3); §3.4 records why full deadlock *freedom* is not the goal, and stratification as the opt-in that would provide it |
| SH-5 | `[waitfor]`: (a) keep whole / (b) demote to optional-checked, drop propagation + placement gate / (c) delete | — | **(b)** (§6) |
| SH-9 | Direct-answer façades (§3.5) as a declared rung: the two-part restriction, its checkability, and whether the request send pumps (or reserves at mint) to close the back-pressure residue | as stated / collapse into SH-1 (all mixed handlers start restricted, graph unlocks rung 3) / omit | **as stated**, and consider *defaulting* to it: a mixed handler is direct-answer unless its body needs rung 3, so the guarantee is what you get unless you ask for the analysis |
| SH-6 | Spelling for monitor members (`sync fn`? `locked fn`? bare `fn` + state access implies?) | — | needs a round of its own; implicit-by-state-access is the ergonomic option and the least visible |
| SH-7 | `use H() on POOL` sugar for `spawn` + `use addr` | yes / no | **yes**, low-cost (§5) |
| SH-8 | Prerequisite defect: the idle report's `active` hole (fact 5) | — | fix **first**, independent of every other row |

Sequencing, if decided: SH-8 immediately (it is a defect regardless); the
rest after the second sequence's steps 3–6, since T-4 (multi-effect
handlers) and `core.time`'s `TestClock` interact with SH-1/SH-5 — a
TestClock under SH-5(b) needs no `[waitfor]` declaration and no dedicated
thread, which simplifies TIME.md's Test B and should be decided in sight of
it.
