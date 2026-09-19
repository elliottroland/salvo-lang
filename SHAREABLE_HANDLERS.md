# Shareable handlers — monitors, mixed handlers, and the future of `[waitfor]` (working document)

Status: **DECIDED 2026-09-19, fully** — the decision round took every row.
SH-1, SH-2, SH-3, SH-4, SH-5(d), SH-7, SH-9's defaulting (pumping request send
deferred); SH-10: the word is `defer`, general per (c), **(b) upper bound at
both levels** (a declared `defer` need not be exercised; an effect member's
`defer` is an upper bound on handlers — and its absence forbids, or the
interface says nothing), the rung-4 opt-in is the **declared-`defer`
synthesis** (no kind word); SH-6 was decided `sync fn` and **revised the same
day to shape-based classification** (§3.12) — a lock and a mailbox are two
serialization mechanisms over one state, so the kind is a whole-handler fact
read off the members that appear (with rung 1 keyed on *mutable* state — a
handler carrying only immutable constructor parameters, later `const`
immutable fields, is shareable bare), and the design's entire syntax bill is one
contextual word (`defer`) plus one sugar (`use … on POOL`). What remains is
engineering (the SH-1 plan is §7, the build order at the end of §8) plus one inferred residue noted
for SH-4's build (§3.12 point 3). The document stays alive until built. Written 2026-09-17, the evening the second sequence's steps 1–2
landed, out of a conversation that began at "how do I implement `Random` when
its state must live behind a thread boundary" and worked through four designs,
two of which survive below. The document follows DESIGN_DOC.md's shape —
options, trade-offs, recommendations — and carries the delete-when-built
charter its two predecessors had (TIME.md and FREE_CONCURRENCY.md, both now
retired into COMPLETED.md's log): when its outcomes are decided and
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
   declared effect (§6 — unpacked job by job at the user's request 2026-09-18,
   with "occupancy" defined in §2.1; the optional-annotation middle ground was
   put up and **rejected** the same day, leaving §6.5's (d) as the recommendation:
   no internal spelling, the gate re-aimed at the host boundary).

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
5. **The idle report had a hole for occupied waiters — ✅ fixed 2026-09-18
   (SH-8, the one row here that needed no user call).** `idle()` required
   `active == 0`, and an activation parked in a nested wait still counts in
   `active`, so every deadlock involving an occupied actor hung *silently*
   instead of producing the named report. The runtime now books frames parked
   in `salvo_wait` (`parked_frames`) and `main`'s own waits (`main_waits`)
   separately, and the report reads `stuck()` — `active == parked_frames &&
   main_waits > 0 && quiet()` — where the `on_idle` hook keeps the stricter
   `idle()`. `main`'s clause is load-bearing (its thread runs program code
   without being a frame the scheduler counts, so an actor parked while `main`
   works is an ordinary program), and a fulfilled-but-not-yet-picked-up waiter
   slot counts as queued work, which closes the delivery/pickup race. The
   report now names the actors **parked in a wait** beside the **gated** ones.
   Two things stay open and are recorded in ROADMAP rather than here: whether
   the *hook* should read `stuck()` too (observable, so the user's call), and
   naming the handler and member rather than `actor 0`. So every hard case
   below now dies **loudly**, which is what this document's §3.3 point 4
   assumed.

### 2.1 Occupancy, precisely — the word, and what it costs

The word is used from §3 on, and it is not a synonym for "blocking".

**A frame occupies its thread when it is waiting for an answer only somebody
else can produce, and its own stack frame stays alive until that answer
arrives.** The unit is the *frame* — one `waitfor`'s place on one stack —
not the thread (which keeps working) and not the actor (which may have no
frame in flight at all).

Four neighbouring things, kept apart because Salvo's mechanisms differ exactly
there:

| | the frame | the thread | who may serve the frame's actor |
|---|---|---|---|
| **occupancy** — `waitfor` | alive, parked in the wait | *works*: pumps its own pool, minus the waiting actor's activations (fact 2) | nobody: the `running` flag is held for the whole body (fact 1) |
| **gating** — `replyto!` | **ended** at the mint | free | the actor itself, for the awaited reply only |
| **deferral** — bare `replyto`, `defer` (§3.9) | **ended** at the mint | free | anybody: the mailbox is open |
| **blocking** — a send into a full mailbox, a host mutex, an FFI call | alive | idle: serves nothing (fact 3) | nobody |

Occupancy costs exactly three things, and the ladder (§3.5) bounds them
differently, which is why they are worth naming separately:

1. **A stack frame is pinned.** Pumped work runs nested on that stack, and a
   pumped item that itself waits nests further; nothing bounds the depth
   ([waitfor-pump]'s stated cost).
2. **The waiting actor's mailbox is stalled** — and only this cost needs the
   frame to be an *activation*. Nothing of that mailbox is delivered by
   anybody, the waiting thread included: its pump excludes its own actor
   deliberately, because the alternative is re-entering an activation
   mid-body. `main` has no mailbox, so `main`'s occupancy costs 1 and 3 only,
   which is the whole reason `waitfor` was `main`-only before 2026-09-17.
3. **The duration belongs to somebody else.** It ends when a token is
   discharged: a lock body at rung 1–2, one straight-line activation plus
   queue drain at rung 3, and at rung 4 a future event that may never come
   (§3.6) — legitimately, as `acquire()` on an exhausted pool.

Two properties that shape every option in §6:

- **Occupancy is transitive, and invisible at the call.** A caller of a sync
  façade member occupies its frame although the *callee* is what waits, and
  the same holds through any helper. That is why SH-4's edge is inferred at
  the call site through the binding in force there, rather than read off the
  caller's own signature.
- **The language already has undeclared occupancy.** An ordinary send into a
  full mailbox holds the sender's frame for as long as the target takes to
  drain, stalls the *sender's* mailbox if the sender is an actor, and serves
  nothing while it waits (fact 3) — strictly worse than a wait on every axis
  — in the most common operation an actor program has, with no declaration
  anywhere and no placement check. `waitfor` is not the language's first
  occupancy; it is the first one anybody proposed to annotate.

### 2.2 Considered: one mailbox for everything (waits unified into deliveries)

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
inside `lookup`. Until 2026-09-18 that was a **silent hang on both backends**
— `active ≥ 1` (the peer's occupied activation) suppressed the report; with
fact 5's hole fixed the same program now dies with the named report, naming the
peer as parked in a wait. The deadlock is unchanged; only its failure mode is.

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
4. **The runtime net under it**: fact 5's hole is fixed (2026-09-18), so
   whatever the graph misses now fails loudly rather than hanging, and the
   report names the occupied actor. Naming the *member* it is parked in is the
   remaining half, recorded in ROADMAP.
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
| **a timer-backed clock** ([time-coupling]'s customer) | `ManualTime.after` *stores* a future deadline's token in `pending`; `advance()` drains the due ones — deferral is its semantics. (A deadline that is *already* due fires inside `after` as of 2026-09-18, so the zero-length reading a test clock uses is a rung-2 shape; anything with a real wait is not.) | virtual time reaching the deadline |
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

### 3.7 Are the rungs user-facing, and why they survive the graph (asked 2026-09-18)

The user's restatement widened the ladder to five rungs — the reasons a
handler can be shareable: **(1) stateless** — shareable with no
synchronization at all; **(2) monitor** (§4) — state, no send members;
**(3) direct-answer / "terminal"** (§3.5) — plain + send members, the plain
members never park; **(4) full mixed** (§3) — the graph; **(5) all send
members** — an ordinary actor, checked by the existing graph. Two questions
were put: (a) do users need to know the rungs, or do they only inform
compilation and diagnostics? (b) if the deadlock analysis must exist anyway
for rungs 4–5, what do rungs 2 and 3 buy by being separated out?

**(a) What is already visible, and what must be.** Rungs 1, 2 and 5 are
visible in the declaration's *shape*: no state / state with only plain
members / only send members. The member kinds and the presence of fields
carry the classification, so no new user-facing concept is needed there —
only the refusal list attached to each shape (§3.5: each refusal names the
next rung). The one invisible line is **3 vs 4**, because it hangs on body
facts: does a send member park, store, forward or capture a `Reply`; does a
sync member occupy. And that is exactly the line users must be able to see,
because it is the boundary between *guarantee by construction* and
*compiles only if the whole program's graph agrees*. Purely inferred, it is
unstable under maintenance: an edit that parks one reply silently
reclassifies the handler to rung 4 and detonates in some **consumer's**
build as a path-shaped cycle error, far from the edit, phrased over actors
the editor has never heard of. Declared and checked, the error is local and
lands at the handler: "this member parks a reply; this handler is
direct-answer."

Two audiences. The handler *author* needs the whole ladder — it is the
vocabulary for choosing a mechanism, and §3.6's table is a decision
procedure (answer depends on a future event → rung 4; async writes + sync
reads → rung 3; twenty instructions under a lock → rung 2). The *consumer*
mostly does not: the effect interface stays plain (`Random` never changes),
and interchangeability at binding sites is the point. What leaks through to
a consumer is exactly three things: spawn-only vs `use`-bindable, cost
(nothing vs a lock vs a round trip), and — rung 4 only — the possibility
that binding this handler makes the graph refuse the program. That last is
a genuine consumer-facing property, and the second reason the 3/4 line
deserves a spelling. Conclusion: the rungs mostly inform compilation and
diagnostics, with one load-bearing exception — the 3/4 boundary should be a
declared, checked contract, in the same spirit as SH-5(b)'s
optional-checked `[waitfor]`.

**(b) Why rungs 2 and 3 survive the graph.** Five benefits, none of which
the analysis substitutes for:

1. **They are different machines, not different analysis strengths.** A
   monitor is a lock: no mailbox, no round trip, sibling-call atomicity,
   `use`-bindable. A direct-answer façade is a servant: mailbox, arrival
   order, fire-and-forget writes, sync reads, back-pressure. Even with a
   perfect deadlock oracle both stay: users choose a rung for cost and
   semantics, and the deadlock properties are corollaries of the
   restrictions, not their purpose. `CyclicRandom` as a monitor is "lock,
   advance, unlock" with no servant anywhere; forcing it through rung 3
   buys a round trip per `next()` for nothing.
2. **A guarantee composes; an analysis verdict does not.** The graph is
   conservative over types, not instances (§3.3 point 3), so a rung-4
   handler exports *false-refusal risk*: whether it compiles depends on the
   consuming program's topology. A rung-2/3 handler contributes no
   occupancy edges and can never be the reason a consumer's build fails.
   For std and any reusable handler this is the difference between "drop it
   in" and "your program may be refused because my body *could* send
   somewhere".
3. **The rungs keep the graph small, which keeps it precise.** "The
   analysis exists anyway" is true for rung 5 — but that is the existing
   gate/send graph. What rung 4 *adds* is the inferred occupancy edges, and
   a conservative analysis's false positives scale with how much lives
   inside it. Every handler that stays at rung ≤ 3 is subtracted from the
   imprecise region, so the graph's verdicts concern only the handlers that
   genuinely need it.
4. **Stall bounds, which deadlock-freedom does not give.** The graph rules
   out cycles; it says nothing about durations. The ladder does: rung 2
   stalls a caller O(one member body); rung 3 stalls one straight-line
   activation plus queue drain (bounded, because the servant never
   occupies); rung 4 stalls until a *future event* that may never come —
   §3.6's defining property, and legitimate (`acquire()` on an exhausted
   pool). A caller reasoning about latency reads the rung, not the graph.
5. **Diagnostic locality.** Rung refusals are declaration-anchored, one
   line, with the next rung named as the remedy. Graph errors are
   whole-program paths anchored at a seam. Catching most mistakes with the
   first kind, and reserving the second for programs that opted into
   rung 4, is strictly better than routing everything through cycle
   reports.

Rungs 2 and 3 do not collapse into *each other* either: merging them loses
one side — either every counter pays a servant round trip (no monitors), or
every stateful handler gets lock semantics and loses async writes (no
terminal façades). The residue also differs: monitors have none; rung 3
keeps back-pressure.

Consequence for SH-9: this sharpens its "consider defaulting" into a
position — **rung 4 is the thing you ask for, and the price quoted is the
graph.**

### 3.8 Marking the rung — prior art and the options (asked 2026-09-18)

**How other languages mark this boundary.** The field splits into three
families:

- **Declared kinds.** *Ada* is the strongest precedent for rung 2: a
  `protected` object is a declared kind distinct from a `task`, and the
  language forbids "potentially blocking operations" inside protected
  actions (RM 9.5.1 — entry calls, delays, task interaction raise
  `Program_Error`; the Ravenscar profile tightens this toward static
  enforcement). That is §4's restriction — the kind in the declaration, the
  refusal list as the law of the kind — shipped in 1995. *Swift* declares
  everywhere the boundary is user-visible: `actor` is a declaration
  keyword, `nonisolated` marks per-member the members that touch no
  isolated state (our façade sync member, spelled), and `Sendable` is a
  declared-but-compiler-checked conformance with an `@unchecked` escape.
  Note Swift's deadlock answer is one Salvo cannot copy: actors are
  *re-entrant at suspension points*, so the §3.1 shape cannot lock — at the
  price of observing state mid-logical-operation, exactly what
  run-to-completion [actor-kind] exists to refuse. Having kept
  run-to-completion, Salvo needs the ladder and the graph where Swift needs
  neither. *Pony* declares reference capabilities per type
  (`iso`/`val`/`ref`/`box`/`tag`) and deletes the question instead: no
  blocking anywhere, behaviours are async-only, so every handler is rung 5
  and deadlock-free by construction — at the price of no synchronous getter
  existing at all. The cost of collapsing the ladder to one rung, made
  concrete.
- **Inferred, with declared assertions retrofitted.** *Rust*'s
  `Send`/`Sync` are auto traits — inferred from field composition, opt-out
  declarable. The instability §3.7 predicts is Rust's documented lived
  experience: a private field change silently drops `Send` from a public
  type and breaks downstream crates (a known semver hazard), and the
  ecosystem's remedy is optional checked declarations bolted on after the
  fact (`static_assertions::assert_impl_all!`, semver-check tooling).
  *Koka* infers effect rows and lets a signature declare them, checked
  against the inference — the exact SH-5(b) model. Both arrive at "infer
  the truth, let the author pin a contract"; Rust shows what happens when
  the pin is not offered from the start.
- **Unmarked, runtime nets.** *Erlang/OTP* distinguishes sync `call` from
  async `cast` at each call site by convention only; a cyclic `gen_server`
  call deadlocks at runtime and is caught by the default 5-second call
  timeout plus supervision restart — the runtime-net-only answer, and the
  precedent for the parked `Reply | TimedOut` last resort §3.4 records.
  *Java*'s `synchronized` is a declared but **unrestricted** monitor —
  every §4 pathology is writable — and the ecosystem grew opt-in checked
  annotations anyway (JCIP/ErrorProne `@GuardedBy`; Clang's C++ thread
  safety annotations `REQUIRES`/`EXCLUDES`), as lint rather than law.

The pattern across all three: contract-grade concurrency boundaries end up
*declared* in every mature design — either from the start (Ada, Swift) or
retrofitted as assertions after inference's instability bites (Rust, Java).
No surveyed language regrets a declared kind; two document the pain of its
absence.

**What must govern the spelling: multi-effect handlers (T-4).** The rung is
a property of the **handler**, never of the effect:

- One handler `of A, B` has one state, one servant, one mailbox — so one
  rung, decided by its weakest member. The declaration must cover all
  implemented effects at once; per-effect rungs on a shared state are
  incoherent.
- The effect *cannot* carry it, independently of T-4: one effect has
  handlers on different rungs — `Clock` alone spans the ladder end to end
  (`SystemClock` is rung 1, stateless; `TestClock` is rung 4, §3.6's first
  row). Marking the effect would either forbid one of them or lie about the
  other.
- Under T-4 a handler may implement a plain effect *and* an actor effect at
  once, so send members may be public protocol rather than private
  servants. The classification must therefore key on what members *do*
  (does a `Reply` escape its activation) — not on where the send members
  came from.
- Diagnostics must name the member **and** the effect it implements, since
  a rung violation in a multi-effect handler is otherwise ambiguous.

**The options:**

- **M-1: pure inference, no spelling.** Rejected by §3.7(a): the 3/4
  boundary becomes unstable under maintenance and its failures land in
  consumers' builds. Rust's retrofit history is the evidence.
- **M-2: handler-kind keywords.** `monitor CyclicRandom(seed: Int) of
  Random { … }` for rung 2; a rung-4 opt-in word on the handler (spelling
  open — `holding handler`? `parking handler`?). The Ada/Swift shape. One
  word regardless of how long the `of` list is, so T-4-proof by
  construction. Rungs 1 and 5 need no word (shape-evident); under SH-9's
  defaulting, rung 3 is the unmarked mixed default, so the only mandatory
  new vocabulary is rung 2's kind and rung 4's opt-in.
- **M-3: member-kind spellings only** (SH-6's implicit-by-state-access
  route, extended). The rung falls out of which member kinds appear plus
  body checks. Leaves the 3/4 line invisible — the one line §3.7 says must
  be visible. Rejected *alone*; survives as the mechanism under M-5.
- **M-4: deduction clauses on members.** Salvo already spells "what this
  function does to its parameters" in `=>` clauses, and actor-effect
  members already carry sendable deductions (`=> !id, !out`). Parking is a
  fact of the same kind — "the `Reply` escapes its activation" — so it can
  ride the same syntax: `send fn after(d: Duration, out: Reply<Fired>) =>
  !d, defer out` (spelling open), inferred from the body when unsaid,
  checked when said, exactly the deductions model. Per-member, so the
  diagnostic names the member and the effect; multi-effect-safe by
  construction. What it lacks alone is the handler-level summary a human
  reads first.
- **M-5: M-2 over M-4** — the handler-level kind word as the contract a
  reader and a consumer see; the member-level reply deduction as what the
  checker verifies and points at when the contract is violated. Under
  SH-9's defaulting, the mandatory vocabulary is minimal (rung 2's kind,
  rung 4's opt-in), and a rung-4 member's `defer` deduction doubles as the
  per-member documentation of *which* answer is deferred.

**Recommendation: M-5**, with SH-9's defaulting. This also bears on SH-6:
the locality argument weighs against implicit-by-state-access — the
prior-art norm is a declared kind, and "least visible" is now a cost, not
an economy.

### 3.9 `defer`, precisely — and the Koka model it borrows from (asked 2026-09-18)

**The semantics.** `Reply<T>` is already linear: [actor-replyto] tracks
every reply until it is discharged exactly once, and §3.5's rule already
enumerates the four ways one can escape an activation. `defer p` (spelling
open with SH-10) surfaces that already-computed fact as a declarable
contract, so it costs the checker nothing new:

- **Without `defer`** — the direct-answer default — the obligation carried
  by `p` is discharged *in-frame*: `p.send(v)` happens exactly once, on
  every control path, before the activation ends. Anywhere on the
  activation's stack counts (a helper the member calls may fulfil it); `p`
  never escapes.
- **With `defer p`** the obligation *may outlive the activation that
  received it*, by any of the four escape routes linearity already
  distinguishes: parked in a `replyto` continuation, stored in handler
  state, forwarded as a message payload, captured in a task mint.
  Linearity keeps tracking it wherever it went — exactly-once discharge
  still holds globally; `defer` only withdraws the promise that the
  discharge happens in *this* activation.

What each party reads off it: for the **caller and the graph**, a façade
wait on a `defer` member is a wait for a future event — occupancy of
unknowable duration, so the edge is non-terminal and participates in
cycles, while a non-`defer`, non-occupying member gives the terminal edge
§3.5's guarantee rests on. For the **handler kind** (M-5's other half), an
inferred `defer` inside a monitor or direct-answer handler is an error at the
escape site; inside a rung-4 handler it is legal, and writing it documents
*which* answers are deferred — §3.6's table becomes readable off
signatures. For **composition**, it rides the existing deduction
machinery: inferred from the body when unsaid, through callees (a helper
that parks its `Reply` parameter confers `defer` on whatever its callers
passed it), per-parameter (one member can defer one reply and discharge
another).

Two edges pinned: **forwarding counts as `defer` even when the forwardee
answers promptly** — the dependency chain now extends past one activation
of this servant, which is exactly what the terminal guarantee cannot
contain. And token-free `k@self` or fire-and-forget sends stay non-deferring
*unless they carry the reply* — they defer work, not answers, as §3.5
already rules.

**The Koka model, concretely.** Koka puts effects in a row on the function
type, inferred by default:

```koka
fun greet( name : string )
  println("Hi " ++ name)
// inferred: (name : string) -> console ()
```

Write no signature and the compiler knows the truth anyway. Write one and
it is *checked against* the inference, as an upper bound — over-approximate
freely, hide never:

```koka
fun greet2( name : string ) : io ()      // ok: console ⊑ io
  println("Hi " ++ name)

fun pure( name : string ) : total ()     // error: body has console,
  println("Hi " ++ name)                 // total is the empty row
```

(Higher-order functions are effect-polymorphic — `map`'s type carries its
callback's row — the discipline Salvo's effect lists already have for
callbacks.)

The core SH-5(b) shares with it: **inference is the source of truth; a
written annotation is checked against it, never trusted; the analysis
never depends on the annotation existing.** The instructive difference is
polarity. In Koka, *hiding is impossible* (the effect is part of the
propagating type and semantically load-bearing — a handler must be found
for it) while *over-claiming is legal* (signatures are upper bounds).
SH-5(b) flips both: hiding occupancy is legal (the graph infers the edge;
post-pump-rule, occupancy is a fact, not a capability needing a grant) and
declaring it falsely is a warning. Each polarity follows from what the
annotation is *for* in its language.

**The sub-decision this exposes (SH-10(b)): is a declared `defer` exact or
an upper bound?** Exact (SH-5(b)-style: a `defer` the body never exercises
warns) keeps documentation honest. Upper bound (Koka-style: a false `defer`
is legal) lets a rung-4 handler *reserve* deferral before its body needs
it — contract stability under evolution. An over-claim's cost is real but
self-inflicted and coherent: a spurious `defer` adds a spurious
non-terminal edge, so the author pays with graph precision for evolution
room. Leaning: **upper bound for `defer`** (it is a "may", and the handler
already paid rung 4's price by declaring the kind), keeping the warning
polarity for `[waitfor]` — the user's call, with SH-10.

**Generalization (asked 2026-09-18): is `defer` Reply-specific, or a fact
about linear parameters?** The fact is general; the load-bearing
consequence is (today) unique to `Reply`. A linear parameter's obligation
has exactly three dispositions relative to a call: **discharged in-frame**
(the stream closed, the reply fulfilled, on every path before the call's
synchronous extent ends), **returned** (the obligation threads back to the
caller), or **escaped** — stored, captured, forwarded. `defer` is the third
arm, and the compiler already computes the distinction for a different
reason: the Rust backend's ownership inference (an escaping parameter must
be moved; an in-frame-only one is borrowable). It is binary for send
members only because they answer nothing, so "returned" is unavailable.
What is unique to `Reply` is being Salvo's only *waiter-backed* linear
type — its discharge unblocks a specific parked party — which is what
turns the disposition into a progress fact, a graph input, and the
rung 3/4 line. For streams or `FsError` the same disposition is a
resource-lifetime fact (a held `OutStream` is a file open indefinitely):
worth reading off a signature, but nobody stalls on it. And even the
`Reply` case already exceeds the handler setting: any function with a
`Reply` parameter — helper, free `send fn`, task body — carries the
disposition and confers it up the call chain, and the graph traces task
bodies, so it needs `defer` everywhere a `Reply` travels; the handler
member is merely where the fact becomes a *contract* via the kind word.
**Leaning (SH-10(c))**: define `defer` generally — the escaped disposition
of a linear parameter, beside consumed/returned — with load-bearing
consequences only where a waiter exists (`Reply` now; the parked
`Reply | TimedOut` timeout form inherits it for free; other linear types
get checked documentation, lifetime lints a plausible future consumer).
Costs nothing now and avoids a second word later if another waiter-backed
type arrives.

### 3.10 Considered: timeouts or fallible waits by default (asked 2026-09-19)

The user's aim, stated with the question: make deadlock-risky code the *less
obvious* way to write Salvo, accepting it cannot be removed outright (any more
than infinite recursion can). Three proposals examined: a default timeout on
`send`/`waitfor`; a fallible result type (`T | TimedOut`, fed by wall time or
by runtime deadlock detection) with the infallible blocking form gated behind
an effect; and timeouts set on handlers rather than callers.

**Default timeouts: rejected, on four grounds.**

1. **Determinism.** A wall-clock timeout fires on a loaded machine and not on
   a fast one, so a program's *output* becomes timing-dependent — against both
   backend parity and the [time-coupling] posture. Worse, it tears the two
   timelines the time module keeps apart: under `ManualTime` nothing advances
   real time's proxy, so a default (real) timeout under a virtual-time test
   either never fires or fires meaninglessly. There is no timeline a *default*
   can safely live on.
2. **The tax lands on the common case.** Every wait's value becomes a union
   and every caller handles a `TimedOut` arm that almost never arrives.
   Erlang's precedent points the other way: its 5s call timeout *crashes* the
   caller into supervision rather than answering a value, precisely so that
   ordinary code does not branch on it.
3. **The mechanics weaken linearity's meaning.** A timed-out wait resumes, but
   the `Reply` it minted is still in someone's hands — linearity *requires*
   the holder to discharge it. So a late answer must become a silent no-op
   into a dead slot, and "exactly once" quietly becomes "at most once
   observed". (A timeout is also only observed between pumped items: a wait
   that is mid-way through a pumped activation checks its slot again when the
   activation ends, so a long pumped body delays the deadline it sits under.)
4. **The incentive inversion.** A recoverable deadlock is a *cheaper*
   deadlock: the named report (SH-8) is loud, attributable and terminal, where
   a `TimedOut` arm invites the retry loop — a livelock with worse
   diagnostics. Making the risky shape survivable makes it more attractive to
   write, which is the opposite of the stated aim.

**Fallibility fed by deadlock *detection* rather than time**: better —
`stuck()` is a logical condition, so it is deterministic and test-stable — but
it inherits ground 4 whole, and its coverage is thin: `stuck()` is global
quiescence, which a partially-live program never reaches, so the value would
arrive exactly when the whole program is wedged and almost never in a server
with live parts. Per-cycle runtime detection (a runtime waits-for graph over
tokens and frames) is real machinery the scheduler does not have, and it
still converts a defect into control flow.

**Timeouts on the handler: the salvageable piece — as an opt-in deadline on
`defer`.** The caller cannot set a meaningful budget (occupancy is transitive
and invisible at the call, §2.1); the handler author knows the semantics. And
the handler side already has the exact slot: a deferred answer is a declared
`defer` (§3.9, rung 4), so a deadline is a *parameter of the hold* — "held, and
answered or timed out within d" — checked and discharged by the runtime timer,
riding M-4/M-5's syntax. Where it makes no sense is §3.6's own table: the
rows whose *semantics* is indefinite deferral — `acquire()` on an exhausted
pool, a barrier, a shutdown drain, the timer itself — which is the proof it
cannot be a default even handler-side. Recorded as a refinement of the
`Reply | TimedOut` form (same machinery, better placement), unscheduled until
a rung-4 handler wants it.

**What actually does the job the aim names is already on the table.** The
unmarked idioms are deadlock-free by construction: a bare `replyto` leaves the
minter serving, rungs 1–3 contribute no occupancy edges, and the graph refuses
the statically visible cycles among what remains. SH-9's defaulting plus
SH-10's marking make the risky forms the *asked-for* forms — rung 4 is an
opt-in word, a gate is a distinct spelling (`replyto!`), and occupancy is
priced by the graph. That is the static version of "risky code is the less
obvious way", achieved with no runtime tax on the common case — and it is
stronger than the recursion analogy suggests, since recursion gets no static
help at all.

### 3.11 What sync members buy over caller-side waiting (asked 2026-09-19)

The question: with monitors covering the no-locking motivation, why not expose
only actor effects and let callers do their own waiting? Four answers, one of
them a concession.

1. **Monitors cover rung 2 only.** No effects, no waits, no mailbox: a monitor
   is pure state transformation under a lock. Every row of §3.6's table —
   answers that depend on a *future event*: the multiplexed connection, the
   blocking acquire, the coalesced fetch, the timer-backed clock — is out of
   its reach, and those are the cases the sync member exists for. The locking
   motivation is moot; the event-dependent-answer motivation is untouched.
2. **The interface is the real purchase.** Caller-side waiting requires the
   *effect* to be actor-kind, and that colours every consumer forever: each
   call site writes the token dance, and — by the decided "forbid" (an
   ordinary member with a return type cannot be actor-backed) — no plain
   effect can ever be served by confined state. `Clock` is the standing
   counter-example: `SystemClock` is a stateless rung-1 handler and
   `TestClock` a rung-4 servant behind one plain interface, interchangeable at
   the binding site. Declare `Clock` actor-kind for the fake's benefit and the
   real clock's every caller pays the token style; higher-order and generic
   code stops composing with it too, since combinators take functions, not
   protocols-plus-a-place-to-wait.
3. **The concession: the power is not new.** The two-piece form exists today —
   an actor protocol plus an adapter handler of the plain effect whose member
   waits on the addr. `TestTicker` *is* that adapter: `handler
   TestTicker(timer: Addr<Timer>) [waitfor] of Ticker`, a plain-effect handler
   parking its caller on a servant's answer. So SH-1 is honestly scoped: it
   does not add expressiveness, it makes the existing idiom first-class — one
   declaration instead of protocol + adapter, the confinement rule *checked*
   rather than idiomatic, the façade value that travels where two pieces
   cannot, spawn-only enforced, and the occupancy edge inferred through it
   (§3.3) instead of through an ad-hoc adapter the graph reads as a
   `[waitfor]` block today. (Under SH-5(d) the adapter also loses its
   `[waitfor]` declaration and its dedicated thread, so the fused and
   two-piece forms converge in cost; what remains different is the checking
   and the one-declaration surface.)
4. **The deadlock-minimisation lens cuts against hiding, and the ladder is the
   reconciliation.** A caller-side `waitfor` is lexically visible at every
   wait; a sync member hides the wait at the call — that is exactly the burden
   SH-4 carries. The resolution is not to forbid the hiding but to bound it:
   at rungs ≤ 3 the hidden wait is terminal and bounded (§3.5), so hiding is
   harmless; rung 4 — where hiding a wait means hiding an unbounded one — is
   the declared, graph-priced opt-in. SH-9's defaulting makes that the
   *shape of the feature*: what you get without asking cannot deadlock, and
   the form that can is the marked one.

### 3.12 The shape taxonomy (user decision 2026-09-19, revising SH-6)

The user's revision, after choosing `sync fn` earlier the same day: attaching
the monitor marker to a *member* suggests it can vary within one handler, and
it cannot — **a lock and a mailbox are two serialization mechanisms, and one
state can be under only one of them**. A monitor handler cannot be mixed. So
the kind is a whole-handler fact, and the members that appear are the
declaration of it; `sync fn` is retired, and no member-level keyword replaces
it.

| shape | kind | rung |
|---|---|---|
| no **mutable** state | **immutable** — shareable bare, nothing to serialize | 1 |
| mutable state, no `send fn`s | **monitor** — members run under the lock, restricted (§4: no effects, no waits) | 2 |
| mutable state + `send fn`s + bare `fn`s, no declared `defer` | **mixed, direct-answer** — state confined to send members; bare fns are the façade and touch no state | 3 |
| same, with a declared `defer` | **mixed, deferring** — the declared `defer` is the opt-in | 4 |
| mutable state + `send fn`s only | ordinary actor | 5 |

With this, the design's entire syntax bill is **one contextual word (`defer`)
and one sugar (`use H() on POOL`)**: no `sync fn`, no kind words, no
`[waitfor]` (SH-5(d)). The declaration a user already writes is the
classification.

Refinements recorded with it:

1. **Rung 1's criterion is *no mutable state*, not "no state" and not
   "pure"** (user refinement, 2026-09-19). A handler whose data cannot change
   needs no synchronization at all, and "cannot change" is two conditions,
   both required: **not reassignable** — today only constructor parameters
   qualify (immutable after construction; §3's confinement rule already lets
   façade members read them on exactly this ground), and later `const` fields,
   a binding form the user intends to add (recorded in ROADMAP) — and **an
   immutable type**: a `Mut List<Int>` constructor parameter is mutable state
   with no reassignment anywhere, since two threads mutating through it race
   regardless, so a `Mut`-typed anything disqualifies however it is bound.
   Today rung 1 therefore reads: no fields, no `Mut` constructor parameters.
   And rung 1 is *not* "pure": its members may perform effects and even
   occupy — `TestTicker` carries only an `Addr<Timer>` ctor param and parks
   its caller until a timer fires. Occupancy is orthogonal to the ladder
   (§6.6); rung 1 means *shareable without synchronization*, nothing more.
2. **The classification governs sharing, not existence.** "Mutable state + no
   send fns" also describes handlers that exist today and are neither monitors nor
   wrong: a stateful interceptor whose members call the effect it wraps is
   scope-local, single-threaded, lock-free, and effectful. Unshared, they stay
   exactly as they are. The monitor reading — and §4's restrictions — bind
   where the handler is *shared*: spawned, bound with `use … on POOL`, or its
   handle crossing into a spawn clause. Precedent: [actor-sendable] checks
   constructor params at the spawn, not the declaration. Whole-program lets
   the diagnostic do better than a spawn-site error: anchor it **at the
   offending member**, naming the sharing site as the reason monitor rules
   apply.
3. **The rung-3 guarantee keeps one inferred residue**, noted for SH-4's build
   rather than decided here: a send member that calls a **rung-4** handler's
   façade waits an unbounded time inside its activation, so its own answers
   are unbounded transitively — effectively rung 4 with no `defer` of its own.
   It cannot be refused outright (a send member calling `random()` backed by a
   mixed handler is a core use case) and cannot be declared (SH-5(d) deleted
   the vocabulary). The graph sees it — the edge is anchored at the *callee's*
   declared `defer` — so cycles are still errors; what is inferred is only the
   transitive unboundedness. Calls to rung ≤ 3 façades from send members are
   terminal and harmless.
4. **Bare fns mean two disciplines, disambiguated by shape**: in a mixed
   handler they are the façade and touch no state (§3's confinement rule); in
   a monitor they touch state and do nothing else (§4's restriction). Same
   spelling — which is exactly why the shape must be the classifier.

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
member, the mixed handler, or the callers — the user's §0.2 position. This
section unpacks what the declaration actually does today, what its price is,
which of its jobs survive its removal, and the four options, one of which
(§6.5 option (d)) was added at the user's prompting: *should it exist at all
apart from bridging a host boundary?*

### 6.1 What the declaration does today — the eight jobs, and where each lives

`[waitfor]` is one word doing eight separate jobs. They have to be listed
separately, because every option below keeps some and drops others.

1. **It gates the expression.** `waitfor out: Reply<T> { … }` is legal only in
   a frame whose effect list declares `[waitfor]` (`can_wait` in `check.rs`;
   `handler_waits` for the handler-dependency form). Without it: an error
   naming the capability.
2. **It propagates through calls**, like any effect: a function reaching a wait
   through a helper declares it too.
3. **It may sit on an effect declaration's *member*** — the single exception to
   [effect-member-no-effects], carved for it on 2026-09-17 — so a protocol can
   say "this member may occupy your thread" and have it reach callers by
   ordinary propagation.
4. **It is refused on a fn type**, and `waitfor` is refused inside a lambda
   [actor-no-closure]: a function value runs wherever it is called, and the
   capability is a claim about *where*.
5. **Grant check I — the spawn's placement** [waitfor-dedicated]: a handler
   declaring `[waitfor]` must be spawned `on thread()`, whose `Dedicated Pool`
   the `on` clause **consumes**, so a dedicated thread has exactly one occupant
   by linearity. An omitted `on` is refused with it.
6. **Grant check II — the `use` site**: binding such a handler is refused
   unless the binding scope declares `[waitfor]` too. This is the only way the
   hazard reaches a *synchronous* consumer, since a handler's dependencies are
   otherwise invisible to callers.
7. **Grant check III — the task mint**: `replyto` at a free `send fn` that
   declares `[waitfor]` needs `on thread()`, unless the minting frame itself
   declares `[waitfor]` — which proves its own pool is already dedicated.
8. **The deadlock graph's third edge kind** [actor-deadlock-cycle]: a handler
   declaring `[waitfor]` gets a **block** edge to every actor protocol it can
   reach — declared dependencies, addrs it sends through, and the sends of
   tasks it mints — with the *declaration* as the blamed site. Cycles through
   it are errors. This is the one job that is analysis rather than bookkeeping.

`main` participates as an ordinary holder (`fn main() [use, spawn, waitfor]`);
its thread is dedicated by nature, so no placement is written for it.

### 6.2 What it costs today, measured

The whole footprint of the feature in real Salvo is **three declarations**:

```
examples/time/salvo/main.sv:104  handler TestTicker(timer: Addr<Timer>) [waitfor] of Ticker
examples/time/salvo/main.sv:133  fn main() [use, spawn, waitfor] -> None
examples/actors/salvo/main.sv:180 fn main() [use, spawn, waitfor]
```

Nothing in `std` declares it. **No effect member anywhere declares it** — job 3,
the exception carved into [effect-member-no-effects] for it, has zero users. Of
the ~99 occurrences across the repository including inline test sources, almost
all are test `main`s.

So the one non-`main` customer is the unified test clock, and it is worth
reading what the declaration does to that program:

```
handler TestTicker(timer: Addr<Timer>) [waitfor] of Ticker {   // one zero-length
    fn tick() -> Tick { … waitfor … }                          // deadline read
}
…
let sleeper = spawn Napping() use timer, TestTicker(timer) on thread()
```

`Napping` is an ordinary actor with no wait of its own. It is forced onto a
**dedicated thread** because a handler it binds reads a clock — one virtual,
zero-length deadline that is fulfilled inside `after` at registration. That is
the shape [time-coupling] recommends, so the cost lands on the posture std
itself teaches. Generalize it to SH-1's motivating case and the price is the
design: a shared `Random` façade would cost its consumer a thread per binding
scope, and `Random.next()` — a plain member on a plain effect — could not be
declared at all without job 3.

### 6.3 What changed under it, the same day and the day after

- **The pump** [waitfor-pump]: a wait *serves its own pool* while it waits,
  minus the waiting actor's own activations. The hazard the placement gate was
  priced against — an occupied worker starves the pool it sits on — no longer
  exists for a Salvo wait.
  * Sharp enough to catch the diagnostic in the act: grant check I tells the
    user that "a shared `pool(n)` would let one wait stall every actor on it",
    which the pump rule made false in the same step. Under option (a) that
    message has to be rewritten; under the others it goes away.
- **SH-8** (fixed 2026-09-18): a wait that cannot end is now a named report
  rather than a silent hang, and it names the occupied actor. The runtime net
  the declaration was standing in for exists.
- **§3.1**: the upcall deadlocks *identically* with every signature declared.
  Occupancy cycles are a property of waits on cyclic request topology, not of
  hidden waits, so the declaration is a spelling of the edge, never its source.

### 6.4 Job by job: what survives removal, and what would have to replace it

| job | survives without the capability? |
|---|---|
| 1 — gates the expression | **No, and nothing is lost.** A wait in an ordinary function is exactly what §0.2 asks for; the placement it needs is no longer special. |
| 2 — propagates | **No.** Replaced by inference, which SH-4 must compute anyway. |
| 3 — on an effect member | **No.** Zero users today, and SH-1 wants its *absence* (a plain effect backed by confined state). Optional under (b) as documentation. |
| 4 — no fn types, no lambdas | **Yes, and it must be kept on its own footing.** A wait inside a function value would be occupancy at every call site of that value, and a fn value's call sites are not statically known — so the graph cannot place the edge. Keep it as a syntactic rule, independent of any capability. |
| 5 — spawn placement | **No**, for Salvo waits. Genuinely needed for *host* blocking, which does not pump (§6.5(d)). |
| 6 — the `use`-site grant | **No.** What it communicated — "binding this may park your frame" — is a documentation job, and §6.6 asks whether anything else can carry it. |
| 7 — task-mint placement | **No**, same reason as 5. |
| 8 — the graph's block edge | **Must be replaced, and the replacement is better.** The declaration draws edges from a handler to *everything it can reach*; SH-4's inference draws them at the sites that actually wait, resolved through the binding in force. An explicit `waitfor` block is the *easy* case for it — the block names the sends. Whole-program rather than modular, which Salvo has by decision. |

The load-bearing conclusion: **only jobs 4 and 8 carry weight, and neither
needs a capability effect** — job 4 is a syntactic refusal, job 8 is the
inference SH-4 builds regardless.

### 6.5 The options

- **(a) Keep step 1 whole.** `[waitfor]` declared and propagating; mixed
  handlers carry it; callers declare it; spawn sites need `thread()`.
  *Cost*: kills §0.1 — `Random`'s member would need job 3, so a plain effect
  could never be backed by confined state, and a shared façade costs a
  dedicated thread per consumer. Also inherits a diagnostic that now
  misstates its own reason (§6.3). Rejected by the stated intent unless the
  user reverses it.
- **(b) Demote to optional-checked** — *recommended until 2026-09-18, then
  **rejected by the user**, and the argument is decisive.* The proposal was:
  drop jobs 1, 2, 5, 6, 7, keep 4 as a syntactic rule, replace 8 with SH-4's
  inference, and let `[waitfor]` survive as an **optional, checked
  annotation** verified against the inferred truth.
  * **The user's question**: would a caller of a function that declares it
    have to declare it too? Both answers sink the option.
  * **If yes** — propagating whenever declared — it is a nuisance *and*
    incoherent: one author's documentation becomes every caller's obligation,
    while "optional" means the chain breaks at the first author who omits it.
    A propagation that may be abandoned anywhere carries no information; it
    only spreads noise from the sites that opted in.
  * **If no** — and this is what "optional" has to mean — the annotation has
    **no consequence anywhere**: not on callers, not on placement, not on the
    graph, which reads the inference either way. It reduces to local
    documentation, which is spotty by construction (a wrapper that waits says
    nothing unless its author felt like saying it) and needs a grammar slot, an
    `EffectRef` variant, a check and a warning class to carry.
  * **And the positive claim has no teeth to gain.** Salvo's other
    optional-but-checked annotation is the deduction clause, and writing one
    *constrains the body* — that is why it is worth writing. `[waitfor]` is a
    **may**: writing it forbids nothing, promises nothing, and rules out no
    program. §6.6 follows this where it leads.
- **(c) Delete the effect outright.** Pure inference, no vocabulary at all.
  *Cost*: nothing internal can document occupancy, and the word the FFI
  boundary will want has to be re-invented.
- **(d) Move it to the boundary** *(the user's question, and it sharpens the
  whole section)*. Internally the capability disappears, as in (c); the word
  survives — and the **mandatory grant check with it** — only where a host
  thread crosses in: FC-7's synchronous export bridge, and any `platform
  effect` member that blocks the calling thread inside the host.
  * **Why the boundary is where it belongs.** The check exists to price
    "occupies a thread that serves nothing". After [waitfor-pump] that
    describes *no* Salvo wait and *every* host block: an FFI call sits in the
    OS, invisible to the scheduler (fact 3), so it genuinely starves the pool
    its thread belongs to, and genuinely needs `on thread()`. The capability
    is not being deleted so much as **following its hazard**: same word, same
    placement gate, correctly targeted for the first time.
  * **A host thread cannot be inferred into**, so the boundary is exactly
    where a declaration is the only option — the mirror of why inference
    suffices inside.
  * **What it keeps**: `thread()`, `Dedicated Pool` and the `on` clause's
    consumption (now mandatory only at the boundary), the lambda barrier
    (job 4), and SH-4's inference (job 8).
  * **What it gives up**: any internal spelling of "this call may park your
    frame" — see §6.6, which is the one argument that keeps (b) alive.
  * **Cost of the interim**: FC-7 is deferred, so the word would have no user
    at all until a host caller exists. Under (b) the optional form keeps it
    in the language meanwhile.

### 6.6 Visibility without a word — and the claim that would have teeth

(b) existed for one job: letting a human see that a call may park their frame.
Two replacements cover it better, and one of them is already how Salvo treats
a fact of exactly this kind.

- **Inferred and *displayed*, the deduction precedent.** A deduction clause is
  inferred from the body when unwritten, and the language server renders a fn's
  hover with the **effective** list — `Checked::deductions` when available, else
  as declared (`fn_decl_signature`, [fn-ref-table]). Occupancy is the same shape
  of fact: SH-4 computes it for the graph regardless, so the hover can show "may
  occupy the calling frame" on *every* function, always current, with nothing to
  write and nothing to maintain. Annotation gives spotty coverage of the same
  fact and can go stale between edits; inference-plus-display cannot.
- **Named where it bites.** The graph's refusal already has to name the cycle,
  the seam and the binding that chose the handler (§3.3), and SH-8's report
  names the actor parked in a wait. A reader who needs the fact in order to act
  gets it there, in the one situation where acting is required.

**And the claim worth spelling is the negative one.** "This may park you" is
unfalsifiable, constrains nothing and refuses no program. "This will *not* park
you" constrains a body and everything it calls, is checkable, and is something a
caller can build on — a latency contract rather than a hazard label. Salvo
already has that claim in two places, which is the reason no third word is
needed:

- **the rung** (§3.5): a direct-answer handler's sync members provably do not
  occupy, by the restriction that defines the rung;
- **an effect-level promise**, if it is ever wanted: an `actor effect` or plain
  effect could declare that *every* handler of it answers without occupying,
  checked at each handler. Recorded as an option rather than proposed, because
  the cost is concrete and known: such a promise on `Ticker` would have refused
  `TestTicker`, and with it step 6's unified test clock. So it can only ever be
  opt-in per effect, never a default — and until something asks for it, the rung
  covers the same ground where it is actually needed.

The residual honest loss of carrying no word at all: a handler like
`TestTicker` — stateless, **rung 1**, no synchronization of any kind, and it
parks its caller until a timer fires — has nothing in its declaration that says
so. Occupancy is orthogonal to the ladder (the ladder grades state and
synchronization; occupancy is about waiting), so the rung cannot carry it. The
answer is the hover and the diagnostics, not a modifier: the fact is real, it is
computed, and it does not need an author to repeat it.

### 6.7 Recommendation: (d), with occupancy an inferred fact

**(d) — move it to the boundary** (user direction 2026-09-18, having rejected
(b); the call itself is still open):

- **Internally there is no spelling.** Jobs 1, 2, 3, 5, 6, 7 go; job 4 stays as
  a syntactic refusal on its own reason (a wait inside a function value would be
  occupancy at call sites the graph cannot enumerate); job 8 becomes SH-4's
  inferred occupancy edges, which are strictly more precise than the
  declaration-wide block edge.
- **Occupancy becomes an inferred fact with three consumers**: the deadlock
  graph (refusal), the language server (hover, on the deduction precedent), and
  the runtime report (SH-8). Nobody writes it; it cannot go stale.
- **At the host boundary the word and its mandatory gate survive**, because
  there the hazard is real and inference cannot reach: an FFI call blocks a
  thread that serves *nothing* (fact 3), so it starves its pool and genuinely
  needs `on thread()`. FC-7's synchronous export bridge and any blocking
  `platform effect` member are its only holders.
- **`thread()` / `Dedicated Pool` survive as placement you may want**, with the
  linear one-occupant meaning intact, required only at that boundary.

What that removes from today's language: the `waitfor` gate, the propagation,
the three grant checks, the fn-type refusal, the [effect-member-no-effects]
exception (zero users), and three declarations in real Salvo plus the test
`main`s. What it adds: nothing at the surface.

**Sequencing.** SH-4's occupancy inference must land *before or with* the
deletion, because job 8 is real coverage today: dropping the declaration first
would leave declared waits in handlers with no edge at all. The hover is a small
follow-on once the inference exists, and worth doing in the same breath — it is
what makes "no word" a visibility *improvement* rather than a trade.

## 7. The SH-1 build plan (established 2026-09-19; **checker half built the same day**)

Status: **built end to end, 2026-09-19** — the checker surface
(`mixed_tests.rs`, 8 tests) and the emission (findings 2–4 below, as
specified: the clone-box handle, `__Msg_H`/`__Actor_H`, `__Fac_H`), verified
on both backends with identical output. The findings below are the record of
what shaped it.

Findings from reading the machinery SH-1 extends, each of which shapes the
implementation; recorded so the build does not rediscover them.

1. **Sibling member calls do not resolve today** — a handler member cannot
   call another member of the same handler by bare name (verified: "no
   function named `advance` is in scope"). So the façade's core line,
   `advance(got)` inside `fn next()`, is *new resolution*, scoped narrowly:
   inside a **sync member of a mixed handler**, a bare call naming one of the
   handler's own `send fn` members resolves as a send to the servant — typed
   like an addr send (payload consumed, answers nothing), own-members-first
   in the scope ladder. Handler-local members are otherwise already legal
   (conformance skips members that implement no face).
2. **The actor body is face-keyed and needs a handler-keyed twin.** Message
   enums are per *effect* (`__Msg_E` over an actor effect's sends); a mixed
   handler's send members belong to no effect, so the servant needs
   `__Msg_H` + an `__Actor_H` dispatching it — the same machinery keyed on
   the handler. `replyto` already resolves against the enclosing handler's
   members, so parked continuations toward local send members should ride
   for free.
3. **The handle representation must unify monitors and façades.** Both are
   `Addr<E>` for a plain `E`, and the monitor slice lowered that to the lock
   wrapper — but a mixed handler's façade must NOT sit behind that lock (a
   façade member waits inside it; a second caller then blocks on the mutex
   *without pumping*, which wedges a shared single-threaded pool). So
   `__Mon_E` becomes the **clonable boxed handle** on Rust — a per-effect
   clone-box trait (`__Share_E: E + Send` with a blanket impl over
   `E + Clone + Send + 'static`), `__Mon_E { inner: Box<dyn __Share_E> }` —
   with the lock demoted into a per-effect generic adapter
   (`__Lock_E<H: E + Send>(Arc<Mutex<H>>)`) that monitor spawns wrap, and
   façades boxed directly. On Kotlin the unification is the interface
   itself: `Addr<plain E>` lowers to `E`, and both `__Mon_E` and `__Fac_H`
   implement it.
4. **The façade is a second generated type per mixed handler**:
   `__Fac_H { __addr, ctor params }` implementing each plain face, sync
   member bodies emitted on it (ctor params as self-fields, the existing
   handler-member binding machinery reused), own-send calls lowered to
   `salvo_send(self.__addr, __Msg_H::…)`. Constructor arguments are
   evaluated **once** and shared between the handler instance and the
   façade (hoisted locals; cloned on Rust) — sendability of ctor params is
   the declaration-level check that makes the copy legal.

Checker rules, beyond the resolution above: a handler is **mixed** when all
faces are plain and it has send members (the monitor path's send-member
refusal reroutes here); `mailbox` becomes required (the mailbox check keys on
"actor face OR local send members"); **confinement** — sync members bind no
state fields, and a read of one gets the confinement diagnostic by name, not
"unknown variable"; sync members also see **no handler dependencies** (deps
live in the servant; the façade has none); **spawn-only** (a `use` of a mixed
handler is refused naming spawn — the parking-handler refusal's reasoning,
verbatim); multi-face mixed refused like multi-face monitors.

Two interim gaps, deliberate and recorded:

* **The `waitfor` carve**: a sync member of a mixed handler may wait without
  any `[waitfor]` declaration — the user's §0.2 position and SH-5(d)'s down
  payment, since requiring the capability would force `thread()` placement
  machinery that fits nothing here (the façade does not run on the spawned
  thread at all).
* **Mixed handlers are unpriced until SH-4 and the `defer` build land**: no
  occupancy edges, no rung-3/4 boundary — a §3.1-shaped program compiles and
  dies at runtime with SH-8's named report rather than statically. The build
  order accepts this window; SH-4 is the next slice after SH-1 for exactly
  this reason.

## 8. Decision surface

| # | Question | Options | Recommendation |
|---|---|---|---|
| SH-1 | Amend [actor-effect-kind]: mixed handlers (send members + state confined to them; sync members touch no state) | yes / no | ✅ **decided yes and built** (2026-09-19): [mixed-handler], [rs-mixed], [kt-mixed] — checker surface *and* emission, verified end to end on both backends with identical output (COMPLETED.md's log). First-slice cuts recorded in ROADMAP: no deps, one face, no `replyto` (which enforces SH-9's direct-answer default by construction until `defer`) |
| SH-2 | The façade value: `Addr<E>` generalized (stub + ctor params + sync dispatch), sendable; mixed handlers spawn-only | as stated / variants | ✅ **decided as stated and built** (2026-09-19): Rust's clone-boxed `__Mon_E` handle carries monitors and façades alike; Kotlin's handle is the interface itself |
| SH-3 | Monitors: synchronized members restricted to state + pure computation — no effects, no waits | yes / no / unrestricted-with-analysis | ✅ **decided yes, restricted** (user, 2026-09-19) and ✅ **built the same day** — [monitor-handler], [rs-monitor], [kt-monitor]; the restriction is checked as "no dependencies", the lowering is a per-effect lock wrapper, verified end to end on both backends (COMPLETED.md's log) |
| SH-4 | Occupancy in the graph: inferred through façades; cycles containing an occupancy edge are **errors** (no back-pressure downgrade) | as stated / declarations required | ✅ **decided as stated and built** (2026-09-19): the occupancy edge inferred from dependency declarations, mixed handlers as servant nodes, no downgrade, the upcall cycle refused with §3.3's three remedies (COMPLETED.md's log) |
| SH-5 | `[waitfor]`: (a) keep step 1 whole / (b) optional **checked** annotation / (c) delete the word outright / (d) **move it to the boundary** | — | ✅ **decided (d)** (user, 2026-09-19, confirming the 2026-09-18 direction): no internal spelling; occupancy an inferred fact (graph, LSP hover, SH-8's report); the mandatory placement gate survives only at host bridges. (b) was rejected 2026-09-18: non-propagating has no consequence, propagating-when-declared is incoherent (§6.5–6.7) |
| SH-9 | Direct-answer façades (§3.5) as a declared rung, and the default | as stated / collapse into SH-1 / omit | ✅ **decided: the default** (user, 2026-09-19): a mixed handler is direct-answer unless it opts into rung 4 — the principle row (what you get without asking cannot deadlock; §3.10 examined and rejected default timeouts/fallible waits for the same job). The **pumping request send is deferred** (user, 2026-09-19): it bifurcates the send semantics (a second entry point, or all sends pump and §2 fact 3 changes globally), and the residue it closes is a burst limiter on a promptly-draining servant. Trigger to revisit: a reproduced wedge of a façade request into a full servant queue on a shared single-threaded pool. Mint-time reservation stays the recorded alternative |
| SH-6 | Spelling for monitor members | `sync fn` / `locked fn` / implicit-by-state-access | ✅ **decided `sync fn`, then revised the same day** (user, 2026-09-19) to **shape-based classification** (§3.12): the kind is a whole-handler fact — a lock and a mailbox cannot share one state — so no member keyword exists; "state + no send fns" *is* the monitor declaration, checked where the handler is shared |
| SH-10 | Marking the rung (§3.8) + the deferral deduction (§3.9) | M-1…M-5; (b) exact vs upper bound; (c) general vs Reply-specific | ✅ **decided** (user, 2026-09-19): the word is **`defer`** (the deleted 2026-09-10 statement leaves it free; contextual in deduction position); **(c) general** — any function may defer a linear obligation, the escaped disposition beside consumed/returned, inferred when unsaid and checked when said; **(b) upper bound, at both levels** — a declared `defer` need not be exercised, and an effect member's `defer` is an upper bound on its handlers (its absence forbids: hide never, over-approximate freely; handler-local servant members have no contract above them); **the rung-4 opt-in is the declared-`defer` synthesis** — a spawned handler's send member that defers must declare it, and the declaration is the opt-in; no kind word (§3.12 removed the last one) |
| SH-7 | `use H() on POOL` sugar for `spawn` + `use addr` | yes / no | ✅ **decided yes** (user, 2026-09-19) (§5) |
| SH-8 | Prerequisite defect: the idle report's `active` hole (fact 5) | — | ✅ **done 2026-09-18** — the report fires for an occupied waiter and names it; two sub-questions left in ROADMAP (whether the `on_idle` hook reads the same weaker condition; naming handlers and members instead of `actor 0`) |

Sequencing, now decided: SH-8 is **done** (2026-09-18 — it was a defect
regardless); the rest is unblocked, and lands after the second sequence's
steps 3–6, which are also complete, since T-4 (multi-effect
handlers) and `core.time`'s `TestClock` interact with SH-1/SH-5 — a
TestClock under SH-5(b) needs no `[waitfor]` declaration and no dedicated
thread, which would simplify the unified test clock [time-coupling] built at
step 6 — and should be decided in sight of it. SH-10's spelling should be decided together with SH-6 and after T-4's
surface exists, since the multi-effect `of` list is what the handler-level
kind word must sit beside, and `TestClock` is the first rung-4 handler that
would carry the opt-in.
