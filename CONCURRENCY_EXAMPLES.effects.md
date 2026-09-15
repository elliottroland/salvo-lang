# Concurrency examples, effect-surface parallels (working document)

Companion to CONCURRENCY.md **C-10** (processes as effect handlers) and the
sibling of CONCURRENCY_EXAMPLES.md: the same programs, re-expressed under the
hypothesis that **a process is an effect handler bound asynchronously** — the
protocol is an `effect` declaration, a process is a `handler` of it brought to
life with `spawn` instead of `use`, and the message structs / reply handles /
dispatch of the sibling file are *generated*, not written. Written 2026-09-14,
same session; **the direction was taken 2026-09-15** ("The kernel, and the
sugar tower", below, and CONCURRENCY.md "The direction") — so this file now
both runs C-10's readability test (answered) and records the direction's
surface. The syntax remains hypothetical in its spellings; the *shape* is the
direction. The surface forms are tagged with C-10's sub-decisions:

**Spelling note (2026-09-15, updated):** the kernel spellings were since
decided — `tell` → `send` (`send fn` members), `mint` → **`replyto`** (renamed
from `reply` for the "reply goes *to* k" reading), the gated mint →
**`replyto!`**, `fulfil(r, v)` → **`r.send(v)`** (consumption discharges the
obligation; no special verb), the `[Spawn]` effect is lowercase **`[spawn]`**
like `use`, and spawn takes constructor params + a spawn-site `use` clause +
`on pool(n)`; `then`/`defer`/`-> T`/call syntax are deferred to later sugar
passes (CONCURRENCY.md, "The first pass"). Examples 1–5 retain the exploration
spellings until the owed one-sweep rewrite — read `tell fn` as `send fn`,
`then k(c)` as a `replyto k(c)` argument, `then only` as `replyto!`,
`fulfil(r, v)` as `r.send(v)`. **Example 6 is written in the frozen grammar.**

- **(i)** `tell fn` — a member whose call returns at *enqueue*, not at reply
  (the call/tell split).
- **(ii)** `Addr<Protocol>` as an ordinary sendable token; `addr.member(args)`
  dot-call as scoped binding through it (effects stay non-values; addresses
  become values).
- **(iii)** `defer reply` — a member that names its reified linear continuation
  instead of auto-responding; `call(...) then member` — performing a dependency
  call without parking, routing its reply to a named continuation-member; and
  the gate modifier `call(...) then only member` — the same routing with the
  mailbox *gated* to that reply, which is the desugaring target of a plain
  parking call (user direction 2026-09-14: adopt the modifier). What a
  `Reply<T>` *is* — an addressed one-shot linear token; fulfil-as-enqueue;
  reserved reply capacity — is defined in CONCURRENCY_EXAMPLES.md, "What a
  `Reply<T>` is".
- **(iv)** serialization/reentrancy — where C-9(i) resurfaces in effect
  vocabulary.

This file rides with CONCURRENCY.md's charter: graduated or deleted when the
C-10/C-2 call is made.

## Example 1 — `UserServer`: the clean win

The protocol is the effect declaration — compare the sibling file's `GetUser`
and `DbReply` structs, which are this, defunctionalized by hand:

```
effect UserApi {
    fn get_user(id: Int) -> User | Err Str
}

effect DbApi {
    fn fetch_row(id: Int) -> Str?
}
```

The process is a handler. Its downstream dependency is a *handler dependency*
([effect-handler-deps] — the rule already exists), supplied at spawn, which is
CONCURRENCY.md C-1 answered by an existing mechanism:

```
handler UserServer() of UserApi [DbApi] {
    fn get_user(id: Int) -> User | Err Str {
        let row = fetch_row(id)      // ← the seam — an ordinary effect call
        when row {
            is Str  { return parse_user(row) }
            is None { return err("no such user") }
        }
    }
}
```

Wiring — `spawn` is `use`'s asynchronous sibling; an `Addr` is the transferable
token (ii), and `use addr` binds the effect in scope to the running process:

```
fn main() [use, Spawn] {
    let db = spawn DbStore()                     // db: Addr<DbApi>
    let users = spawn UserServer() [DbApi: db]   // handler dep bound at spawn
    use users                                    // UserApi in scope, process-backed
    let u = get_user(7)                          // send + park + resume, implicit
    ...
}
```

What disappeared relative to the sibling file's Example 1: both message
structs; the `pending` map and its correlation id; the explicit `reply` field
and its `fulfil`; the second handler. `get_user`'s body *is* the sugared form
of C-9, with nothing to name it — the await is just a call whose handler
happens to be scheduled. What the checker must still own is unchanged
(C-9(ii)/(iii) under the surface): `row` and anything else live across the seam
is stored and sendability-checked; the implicit continuation is one-shot
because it is linear.

Note the caller's side of `let u = get_user(7)`: `main` parks at *its* seam
until the reply arrives, exactly as any effect call waits for its handler —
the chain of seams is an ordinary call stack that happens to be asynchronous
underneath. Non-reentrancy of that chain is sub-decision (iv).

### Example 1, desugared — the plain call spelled with `defer`, `then`, and `only`

The plain-call seam above is sugar with a **complete desugaring in this same
surface** — the iter-fn property, demonstrated: `defer` names the continuation,
`then` routes a reply to a member, and the gate modifier **`only`** (user
direction 2026-09-14: adopt the modifier) carries the non-reentrancy gate that
a bare `then` deliberately lacks.

```
handler UserServer() of UserApi [DbApi] {
    fn get_user(id: Int) -> User | Err Str defer reply {
        fetch_row(id) then only row_arrived(reply)  // gated: only this reply
    }                                               // gets through until resume

    // captured arguments first, the reply value as the trailing parameter
    then fn row_arrived(reply: Reply<User | Err Str>, row: Str?) {
        when row {
            is Str  { fulfil(reply, parse_user(row)) }
            is None { fulfil(reply, err("no such user")) }
        }
    }
}
```

The correspondence, piece by piece:

- The member's implicit continuation → `defer reply` names it.
- The body *after* the seam → the continuation-member `row_arrived`, receiving
  the reply value (`row`) as its trailing parameter.
- Locals alive across the seam → **captured arguments** on the `then` (here
  only `reply`; anything computed before the call and used after it would ride
  alongside). This is where the C-9(ii) storability/sendability check lands.
- The sibling file's `pending` map → gone even in the desugared form: a parked
  continuation *instance*, with its captures, **is** the pending entry —
  correlation included. An explicit map reappears only when a program parks
  many continuations under its own keying, as `NoticeFetcher.waiting` does.
- `reply` *moves* into the capture, so the obligation travels
  [linear-obligation]: `row_arrived` must fulfil it exactly once, with the
  usual leak and double-use diagnostics.

**The gate, now a surface form.** A plain call is `then only` plus the
mechanical split: *implicit `defer`, cut at the seam, seam-crossing locals
become captures, route with `then only`*. Every step is expressible above, so
the desugaring is complete. The two routing forms differ in one bit of
scheduler policy: while a **gated** continuation (`then only`, or the plain
call that desugars to it) is outstanding, the process's mailbox admits *only
the awaited reply* — everything else queues; a **bare `then`** leaves the
mailbox open. Both end the activation and store a continuation —
run-to-completion is never violated. Consequences worth pinning:

- **The primitive is strictly more expressive than the sugar.** `then only`
  admits code after the call (finish this activation's bookkeeping *now*, then
  sleep for everything but the reply) and gated routing to a named member with
  captures — neither writable with a plain call, whose entire remainder lands
  on the far side of the reply.
- **The deadlock-edge rule refines** (sibling file, Example 4): *gated*
  continuations — plain calls and `then only` — contribute wait-for edges;
  bare `then` contributes none.
- **What the gate is:** a bounded, compiler-controlled *selective receive* —
  the feature Gleam dropped for typing tractability, readmitted in the one
  shape where the awaited type is statically known. Its latitude: the modifier
  generalizes naturally to a member *set* later ("serve this reply and
  `shutdown`, queue the rest") if a customer demands it — typed selective
  receive in full, deliberately not proposed now.
- **The decomposition asymmetry.** `only` *names* the gate; it does not
  decompose it. The gate's true decomposition is the stash (Akka's pattern:
  buffer invocations while busy, replay on resume), and writing that by hand
  requires reifying calls as values — the message structs this surface
  deleted. So under C-10(a) (dual surface) the gate can be genuinely
  decomposed one level down; under C-10(c) (effects-only) `then only` is a
  primitive, full stop. Either is coherent; they differ in what "desugars"
  bottoms out to.

**Code after a `then` call — the ordering rule.** A `then` call does not need
tail position. It registers the continuation and the activation simply
continues: statements after it run *immediately* — and are **guaranteed** to
run before the continuation-member, because the reply queues like any other
message and the process runs one activation at a time. So the two spellings
differ in *which side of the reply* the following code lands on:

```
let row = fetch_row(id)     // code below runs LATER, after the reply
fetch_row(id) then k(...)   // code below runs NOW, before k — guaranteed
```

Two consequences. One activation may issue several `then` calls before ending —
scatter–gather falls out for free (fire N requests, each routing to a member
that gathers into state until a count is met). And the caveat that keeps the
rule honest: "before the continuation" is not "immediately before" — between
this activation's end and the continuation-member's turn, *other messages may
be served* (sub-decision (iv) again), which is why a `then`-member's first act
is re-checking state, never assuming it.

## Example 2 — `Topic`: `tell` members, and Pids in state

Event-shaped logic needs fire-and-forget, which a synchronous effect call
cannot mean — so every member here is a `tell fn` (i): the call returns when
the message is enqueued, and a `-> T` return type is illegal on a `tell`.

```
effect Subscriber<E> {
    tell fn deliver(events: List<E>)
}

effect TopicApi<E> {
    tell fn subscribe(who: Addr<Subscriber<E>>)
    tell fn unsubscribe(who: Addr<Subscriber<E>>)
    tell fn publish(event: E)
    tell fn tick()
}

handler Topic<E>(max_batch: Int) of TopicApi<E> {
    subscribers: Mut Set<Addr<Subscriber<E>>> = mut_set_of()
    buffer: Mut List<E> = mut_list_of()

    tell fn subscribe(who: Addr<Subscriber<E>>) {
        subscribers.add(who)
        if buffer.size() > 0 {
            who.deliver(copy buffer)     // (ii): dot-call through the token
        }
    }

    tell fn unsubscribe(who: Addr<Subscriber<E>>) {
        subscribers.remove(who)
    }

    tell fn publish(event: E) {
        buffer.add(event)
        if buffer.size() >= max_batch { flush() }
    }

    tell fn tick() { flush() }

    // A private fn of the handler — not a member of TopicApi, not callable
    // from outside; the moral equivalent of the sibling file's free `flush`.
    fn flush() {
        if buffer.size() == 0 { return }
        for sub in subscribers {
            sub.deliver(copy buffer)
        }
        buffer = mut_list_of()
    }
}
```

What this example forces onto the surface: the `tell` marker (i) — `Topic` is
*all* tells, which is exactly what "event-shaped" means under the unification
(the sibling file's rule of thumb restates as: **all-`tell` protocols are the
reactive processes; protocols with call members are the ones awaiting serves**).
And Pids stored in fields and sent in messages (ii) — `subscribers` is a set of
tokens, `who` travels inside a message — which is the place the "effects are
not values" rule must bend to an "addresses are values" rule without breaking.

## Example 3 — `NoticeFetcher`: deferred replies and continuation-members

The hybrid: it *implements* call members whose replies it parks, and it *makes*
a downstream call it must not park on. Both needs are (iii).

```
effect NoticeSource {
    fn next_notice() -> Notice | Err Str
    fn shutdown() -> Done
}

effect DbApi {
    fn fetch_batch(count: Int) -> List<Notice>
    tell fn return_notices(notices: List<Notice>)
}

effect Timer {
    fn after(delay: Duration)    // -> None; the reply IS the firing — route it
}                                //    with `then`, or park on it with a plain call

handler NoticeFetcher(batch_size: Int, deadline: Duration)
        of NoticeSource [DbApi, Timer] {
    waiting: Mut List<Reply<Notice | Err Str>> = mut_list_of()
    stock: Mut List<Notice> = mut_list_of()
    stock_gen: Int = 0
    fetch_in_flight: Bool = false
    stopping: Bool = false
    shutdown_done: Reply<Done>? = None

    // `defer reply` (iii): do NOT auto-respond when the body returns; instead
    // the reified continuation is in scope as `reply` — linear, parkable,
    // exactly the sibling file's Reply field, now opt-in per member.
    fn next_notice() -> Notice | Err Str defer reply {
        if stopping { fulfil(reply, err("shutting down")) return }
        if stock.size() > 0 { fulfil(reply, stock.take_first()) return }
        waiting.add(reply)
        ensure_fetching()
    }

    fn shutdown() -> Done defer done {
        stopping = true
        for r in waiting.drain() { fulfil(r, err("shutting down")) }
        if stock.size() > 0 { return_notices(stock.drain()) }
        if fetch_in_flight { shutdown_done = done }   // park own completion
        else { fulfil(done, Done {}) }
    }

    fn ensure_fetching() {
        if fetch_in_flight { return }
        fetch_in_flight = true
        // (iii): perform the DbApi call WITHOUT parking this handler — route
        // its reply to a continuation-member. This is Example 3b's reentrant
        // shape, kept visible: the handler stays reactive while it waits.
        fetch_batch(batch_size) then batch_arrived
    }

    // A continuation-member: private, invoked when the routed reply arrives.
    then fn batch_arrived(notices: List<Notice>) {
        fetch_in_flight = false
        if stopping {
            return_notices(notices)
            if shutdown_done is Reply<Done> { fulfil(shutdown_done, Done {}) }
            return
        }
        for notice in notices {
            if waiting.size() > 0 { fulfil(waiting.take_first(), notice) }
            else { stock.add(notice) }
        }
        if stock.size() > 0 {
            stock_gen = stock_gen + 1
            // captured args ride in the parked continuation — the Tick's
            // generation tag travels with the `then`, not in a message struct
            after(deadline) then deadline_passed(stock_gen)
        }
        if waiting.size() > 0 { ensure_fetching() }
    }

    then fn deadline_passed(gen: Int) {
        if gen != stock_gen { return }     // stale deadline: stock turned over
        if stock.size() > 0 { return_notices(stock.drain()) }
    }
}
```

Readings against the sibling file's Example 3:

- **The coordination state is identical** — `waiting`, `stock`, `stock_gen`,
  the flags. As Example 3b concluded, no surface removes coordination; this one
  just stops making you *also* write the transport (message structs, dispatch,
  correlation).
- **`defer reply` is the honest form of "the handler receives the
  continuation".** It is the literature's `k`, made a linear Salvo value: park
  it, forward it, fulfil it exactly once — with the leak diagnostic forcing the
  shutdown drain, as before.
- **`then` is hand-written reentrancy, named at the call site.** `fetch_batch(n)
  then batch_arrived` says *this handler keeps serving while the call is out* —
  the fetcher's essential property — whereas Example 1's plain `fetch_row(id)`
  says *this member's activation parks*. The (iv) reentrancy decision becomes
  local and legible: parking and non-parking calls are different spellings, not
  different semantics hidden behind one keyword.
- **The generation counter rides in the continuation.** `then
  deadline_passed(stock_gen)` captures the tag; there is no `Tick` struct to
  carry it. Same logic, one less nameable thing.

## Example 4 — supervision as interception, across the scheduler

New in this file, because it is the unification's unique payoff: Salvo's
existing interception mechanism — a handler that depends on the effect it
implements, binding strictly outward — now composes *across the process
boundary*, giving retry/timeout/restart policy with no new construct. A
synchronous policy handler wraps an asynchronous process transparently:

```
// An ordinary interceptor, exactly as the effect chapter writes them today:
// depends on the NoticeSource already registered — here, a process.
handler RetryOnce() of NoticeSource [NoticeSource] {
    fn next_notice() -> Notice | Err Str {
        let first = next_notice()        // the intercepted, process-backed one
        when first {
            is Notice { return first }
            is Err    { return next_notice() }   // one retry, then pass through
        }
    }
    fn shutdown() -> Done { return shutdown() }
}

fn main() [use, Spawn] {
    let db      = spawn DdbStore()
    let clock   = spawn Ticker()
    let fetcher = spawn NoticeFetcher(25, seconds(5)) [DbApi: db, Timer: clock]

    use fetcher          // NoticeSource in scope, backed by the process
    use RetryOnce()      // …now wrapped by a synchronous policy, in scope

    // workers see one NoticeSource; retries and the scheduler are invisible
    let notice = next_notice()
    ...
}
```

Two things to notice. The policy handler is *synchronous and local* — it runs
on the caller's thread, and only its inner call crosses the scheduler; nothing
about `RetryOnce` knows or cares that its dependency is a process. And the same
shape scales to the supervisor fixed point: a restart policy is an interceptor
whose `[Spawn]` dependency lets it respawn the `Addr` it delegates to — the
"supervisor is a handler" line of ROADMAP.md, now with the mechanism visible.

## Example 5 — scatter–gather: many instances of one handler

The ordering rule promised scatter–gather "for free"; here it is — and it is
also the example that answers *how multiple instances of the same handler
work*, because a scatter needs N running copies of one protocol and a central
place that addresses them individually.

```
effect ShardApi {
    fn query(term: Str) -> List<Hit>
}

effect SearchApi {
    fn search(term: Str) -> List<Hit>
}

// One gather in flight, keyed by id; the parked reply lives inside it
struct Gather {
    reply: Reply<List<Hit>>,
    hits: Mut List<Hit>,
    outstanding: Int
}

handler Shard(index: Int) of ShardApi {
    data: Mut Map<Str, List<Hit>> = mut_map_of()
    fn query(term: Str) -> List<Hit> {
        return data.get_or(term, list_of())
    }
}

handler Searcher(shards: List<Addr<ShardApi>>, deadline: Duration)
        of SearchApi [Timer] {
    gathers: Mut Map<Int, Gather> = mut_map_of()
    next_id: Int = 0

    fn search(term: Str) -> List<Hit> defer reply {
        next_id = next_id + 1
        let id = next_id
        gathers.put(id, Gather {reply: reply,
                                hits: mut_list_of(),
                                outstanding: shards.size()})
        for shard in shards {
            shard.query(term) then one_arrived(id)   // scatter: N bare `then`s
        }
        after(deadline) then times_up(id)            // the partial-results clock
    }

    then fn one_arrived(id: Int, hits: List<Hit>) {
        let g = gathers.take(id)                 // Gather? — None if deadline won
        when g {
            is None { return }                   // late reply: dropped, harmless
            is Gather {
                g.hits.add_all(hits)
                if g.outstanding == 1 {
                    fulfil(g.reply, g.hits)      // last one in: complete
                } else {
                    gathers.put(id, Gather {...g, outstanding: g.outstanding - 1})
                }
            }
        }
    }

    then fn times_up(id: Int) {
        let g = gathers.take(id)
        when g {
            is None { return }                   // gather completed: stale clock
            is Gather { fulfil(g.reply, g.hits) }  // partial results ARE the answer
        }
    }
}

fn main() [use, Spawn] {
    let shards = mut_list_of()
    for i in range(0, 8) {
        shards.add(spawn Shard(i))       // 8 instances of the SAME handler:
    }                                    // 8 distinct Pids, one protocol
    let searcher = spawn Searcher(shards, millis(200)) [Timer: spawn Ticker()]
    use searcher
    let hits = search("salvo")           // main parks until gathered
    ...
}
```

What this example establishes:

- **Instances are Pids; `use` cannot fan out.** `spawn Shard(i)` eight times
  yields eight distinct tokens of one protocol, held in an ordinary
  `List<Addr<ShardApi>>`. This is where sub-decision (ii)'s dot-call stops being
  a convenience and becomes the *only* form that works: scoped binding is
  singular — `use` binds **one** handler per effect per scope (a later `use`
  shadows, it does not add) — so N same-protocol instances are unreachable
  through scope and must be addressed through their tokens: `shard.query(term)`.
  Scatter is a `for` over tokens, full stop.
- **The scatter must be bare `then`, by necessity, not preference.** The gate
  admits exactly one awaited reply — so a gated scatter is a contradiction, and
  a rule falls out that C-10(iv) should carry: **issuing a gated continuation
  while another gated continuation is outstanding is an error** (the plain-call
  sugar can never trip it — one seam at a time — only hand-written `then only`
  can). Eight replies interleaving back into the mailbox is the point; the
  ungated form is the correct spelling and the deadlock graph gains no edges.
- **The ordering rule earns its keep.** The whole scatter loop *and* the
  deadline registration run before any reply can be served — the activation
  completes first, guaranteed — so `gathers` is fully initialized before the
  first `one_arrived` fires. No half-built-state race is possible, by
  construction rather than by care.
- **The gather map is the "own keying" case, as predicted.** Example 1's note
  said an explicit map reappears when a program parks many continuations under
  its own keying: a concurrent scatter–gather is exactly that — one `Gather`
  per in-flight `search`, each holding a parked linear `reply` (the
  collections-of-linears fence, one more time), with the captured `id` on every
  `then` doing the correlation.
- **`take` + `None` replaces the generation counter.** Both races (late shard
  reply after the deadline fired; stale deadline after the gather completed)
  collapse into the same shape: `take` removes the entry, the loser finds
  `None` and returns. No generation tag needed, because ids are never reused —
  compare `NoticeFetcher`, whose *reusable* stock slot is why it needed `gen`.

## Example 6 — the binding swap: process ⇄ local handler (frozen grammar)

Added 2026-09-15, after the first-pass grammar froze; the first example in
the **decided spellings** (`send fn`, `replyto`, `r.send(v)`, spawn-site
`use`, `on pool(n)`, `waitfor`). It demonstrates the shared-vocabulary bet
paying off: because dependencies are declared on the handler and supplied at
the spawn site — and the spawn-`use` clause accepts *both* a handler
construction and an `Addr` — a process and a local synchronous handler swap
without touching a character of the consuming code.

The shared contract, an all-`send` protocol (so both bindings are first-pass
legal):

```
effect DbApi {
    send fn fetch_batch(count: Int, out: Reply<List<Notice>>)
    send fn return_notices(notices: List<Notice>)
}

handler NoticeFetcher(batch_size: Int, deadline: Duration)
        of NoticeSource [DbApi, Timer] {
    waiting: Mut List<Reply<Notice | Err Str>> = mut_list_of()
    fetch_in_flight: Bool = false
    ...

    send fn ensure_fetching() {
        if fetch_in_flight { return }
        fetch_in_flight = true
        fetch_batch(batch_size, replyto batch_arrived())  // unqualified: DbApi,
    }                                                     //  whatever it's bound to
    send fn batch_arrived(notices: List<Notice>) { ... }
}
```

**Direction 1 — production process, local handler in the test:**

```
// Production: DbApi is a shared process — one DDB client on four IO threads,
// serving every fetcher. Binding an effect to a Addr in the spawn clause:
fn main() [use, spawn] {
    let ddb = spawn DdbClient(config) on pool(4)
    let fetcher = spawn NoticeFetcher(25, seconds(5))
                      use ddb, SystemTimer() on pool(1)
    ...
}

// Test: DbApi is a plain handler, constructed ON the fetcher — its member
// bodies run inline on the fetcher's thread. Deterministic; no second process.
handler ScriptedDb(script: List<List<Notice>>) of DbApi {
    served: Mut Int = 0
    returned: Mut List<Notice> = mut_list_of()

    send fn fetch_batch(count: Int, out: Reply<List<Notice>>) {
        served = served + 1
        out.send(script.get_or(served - 1, list_of()))   // replies immediately
    }
    send fn return_notices(notices: List<Notice>) {
        returned.add_all(notices)                        // captured for asserting
    }
}

fn main() [use, spawn] {   // the test's main
    let fetcher = spawn NoticeFetcher(25, seconds(5))
                      use ScriptedDb(fixtures), InstantTimer()
    let notice = waitfor out: Reply<Notice | Err Str> {
        fetcher.next_notice(out)
    }
    ...assert...
}
```

**Direction 2 — a local handler, promoted to a process.** The motivating case
is the one per-child construction cannot do: *sharing*. N workers each
constructing `use LocalStats()` hold N disjoint stat sets; when one aggregated
view is needed, the collector becomes a process and the binding becomes a Addr
— no change to any `Worker` member:

```
spawn Worker(x) use LocalStats(), ...     // N workers → N disjoint stat sets

let stats = spawn StatsHub() on pool(1)   // one instance, scheduler-serialized
spawn Worker(x) use stats, ...            // N workers → one shared, race-free hub
```

(The same promotion covers a test that wants several fetchers to see *one*
scripted database: spawn `ScriptedDb` as a process there too.)

Three notes that keep the symmetry honest:

- **A local binding of a `send` protocol runs the member body inline** at the
  call site, inside the caller's activation — and `out.send(v)` inside it
  still *enqueues* (fulfil-is-enqueue is unconditional), so `batch_arrived`
  runs as its own later activation under both bindings. Run-to-completion and
  the protocol's causal order are binding-independent.
- **The swap changes timing tightness, not shape.** Locally, the reply is
  already queued when the activation ends — no round-trip window for other
  messages to slip into. Tighter timing is what a test wants, and the
  difference is the same kind `MemFs`-for-`DefaultFs` always had.
- **The deadlock graph keys on the binding, as promised**: `ScriptedDb`
  contributes no wait-for edges; `use ddb` is where the fetcher→ddb edge
  appears. Same code, different edges — the analysis reads bindings, not
  bodies.

## The kernel, and the sugar tower — the direction (user, 2026-09-15)

The exploration of 2026-09-14/15 (recorded as the C-9/C-10 refinements in
CONCURRENCY.md) bottomed out in a four-piece kernel, with every surface form
above it a strict tower of sugar. The user took this as **the direction**
2026-09-15. Its inherited name is **asynchronous effect handlers**, after its
closest formal relative — Ahman & Pretnar's *asynchronous effects* (Æff),
whose signal/interrupt/interrupt-handler decoupling matches the kernel term
for term (CONCURRENCY.md §2 "Æff" and "The direction" carry the comparison
and the full lineage). Two collapses got it there:

**Collapse 1 — a return type is a Reply parameter.** A member's `-> T` is
sugar for an implicit trailing token:

```
fn get_user(id: Int) -> User | Err Str        tell fn get_user(id: Int,
    { ... return v ... }               ≡          reply: Reply<User | Err Str>)
                                                  { ... fulfil(reply, v) ... }
```

So the only primitive member kind is `tell`; a "call member" is a tell member
whose message carries a token, and a plain body is fulfil-at-every-return.

**Collapse 2 — `defer` is not an operation.** A deferred member *is* the
right-hand side above: an ordinary tell member taking a `Reply<T>` argument
and treating it as the value it is (park, forward, fulfil). `defer` survives
only as signature reconciliation — implementing a declared `-> T` member at
its desugared level — plus a one-word warning to readers ("does not respond
before returning"). Keeping the keyword vs. letting handlers write the
desugared signature directly is an open readability call (CONCURRENCY.md,
"Decisions pending").

**The kernel** that remains:

1. **Process state** — a handler's fields; exclusive by scheduler
   serialization.
2. **`tell`** — enqueue a member invocation. The only send.
3. **`mint k(captures)`** — allocate a parked one-shot continuation targeting
   member `k`, yielding its linear `Reply<T>`. Independently meaningful, not
   just `then`'s internals: members with *multiple* explicit token parameters
   (`tell fn split(job: Job, done: Reply<Summary>, failed: Reply<Err Str>)` —
   separate success/failure routes) can only be called with explicit mints,
   and registration-style tokens (`watcher.on_ready(mint ready(ctx))`) put a
   token in a *payload* position, not a reply slot. Call syntax is legal only
   for members with a single trailing token — a multi-token member has no
   implicit slot for the auto-mint to fill, a rule that falls out rather than
   being made.
4. **The gate bit** (`only`) — bounded selective receive; named, not
   decomposed; at most one outstanding per process.

**The tower**, each layer expressible in the one below:

| Form | Is |
|---|---|
| `fulfil(r, v)` | `tell` to a token (a `Reply` is a one-shot Addr with one tell member) |
| `a(...) then k(c)` | `a(..., mint k(c))` — the mint *is* the second argument, placed in the reply slot |
| `a(...) then only k(c)` | the same, plus the gate bit |
| member `-> T` | implicit trailing `reply: Reply<T>` parameter (collapse 1) |
| plain member body | `fulfil` at every return (collapse 1) |
| `defer` | a marker; otherwise nothing (collapse 2) |
| `let x = m(a)` | auto-mint an anonymous resume continuation (captures = the seam-crossing locals) + the gate |
| `k@self(c, reply e1, reply e2)` | the merge/join: multi-mint into generated gather state |

**The merge/join row, expanded** (raised by the user 2026-09-15): the form
mints one pending invocation of `k` with captured `c` and one slot per
`reply e` argument; each `e` is performed with a mint targeting its slot; the
activation runs when the last slot fills. `then` is its one-slot special case
(`a() then k(c)` ≡ `k@self(c, reply a())`). It is deliberately **sugar, not
primitive**: its desugaring is Example 5's `Gather` pattern (generated pending
struct, per-slot filler members, dispatch-when-complete), fully
surface-expressible — and the policies a join immediately wants (deadline,
partial results, race/first-of-N) are gather-state *programs*, which a fixed
scheduler primitive could not express without growing new forms. Join slots
are necessarily ungated (the one-outstanding-gate rule); a *gated* join is
what the member-set gate generalization would mean, if ever demanded — the two
open latitudes are the same latitude.

## The test this file exists to run

*(Run and answered: the user took the direction 2026-09-15 — see the kernel
section above and CONCURRENCY.md, "The direction".)*

C-10's recommendation says the unification stands or falls on whether these
read *simpler* than their message-passing siblings. The scorecard so far:

| | Sibling (messages) | This file (effects) |
|---|---|---|
| Example 1 | 2 structs, 2 handlers, pending map, correlation | 1 effect, 1 handler, 1 ordinary call |
| Example 2 | 4 structs + a `Batch`, 4 handlers | 1 effect (+1 for subscribers), all `tell` |
| Example 3 | 5 structs, 5 handlers, hand dispatch | 3 effects, same state, `defer`/`then` forms |
| Example 4 | (not expressible without new machinery) | existing interception, unchanged |
| Example 5 | (no sibling written: same shape + N message structs + hand correlation) | a `for` of `then`s over Pids, captured-id correlation |

The honest cost column: three new surface forms — `tell` (i), `Addr` tokens with
dot-call (ii), `defer`/`then` (iii) — and the (iv) reentrancy vocabulary. The
sibling file's approach needs *none* of those but pays with hand-written
transport in every program. That trade is the C-10/C-2 call.
