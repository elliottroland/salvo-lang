# Concurrency examples — run-to-completion processes (working document)

Companion to CONCURRENCY.md, specifically to **C-5(d)** (run-to-completion
processes on a scheduler library) and **C-9** (awaiting as sugar over that
model). Written 2026-09-14, same session. **Nothing here is decided**: the
syntax is hypothetical throughout — `Pid<T>`, `Reply<T>`, `send`, `fulfil`,
`await`, `self`, and the `[Send]` effect are illustrative spellings, not
proposals. This file rides with CONCURRENCY.md's charter: when the C-5/C-9
calls are made, these examples either graduate (rewritten in real syntax, into
`examples/` and the specs) or are deleted with the working document.

## The model, in one paragraph — and against its neighbours

A process is an **explicit state struct plus handler functions**, one handler
per message type it accepts. There is no `receive` statement and no loop: a
`send` *is* scheduling a handler invocation, and a small scheduler library — a
work queue of (process, message) pairs over a thread pool — runs each handler
**to completion**. One message at a time per process, so handlers are the only
code that touches the state and each handler is atomic with respect to it: the
scheduler's serialization is the mutual exclusion, with no lock and no `Mutex`
in the language.

Against the neighbours:

- **vs. the classic OTP model (Erlang, and C-5(a)/(b)):** an Erlang process is
  a thread of control that *blocks* in `receive`; a waiting process costs a
  (green) stack, and the BEAM's runtime is what makes that cheap. Here a
  waiting process costs nothing — it *is* its state struct, sitting in a table
  until a message schedules a handler. Erlang's own `gen_server` API
  (`handle_call`/`handle_cast` over explicit state) is this surface already,
  with the loop hidden inside the behaviour; this model keeps the surface and
  deletes the hidden loop. What is given up: blocking mid-handler (Erlang's
  selective receive), which is what C-9's sugar reconstructs.
- **vs. coroutines / async-await (Kotlin `suspend`, Rust `async`, C-5(c)):**
  those compile suspension into compiler-generated state machines and need an
  executor/runtime shipped with the program; the suspension points colour
  function types. Here the "state machine" is the user's own state struct —
  ordinary, nameable, hand-writable — and each step is a plain function call
  run by a plain pool. This is the same move `iter fn` made against `yield fn`:
  the state stops being a compiler artifact and becomes a declared struct.
  What is given up: writing a multi-step conversation as one straight-line
  body — which C-9 restores as sugar *over* this model, without colouring and
  without a runtime.
- **vs. threads + locks:** the serialization guarantee replaces the lock, and
  the checker sees it statically: a handler's `Mut` access to its process state
  is exclusive by construction, not by discipline.

The three examples are chosen to span the shapes:

| Example | Shape | Await sugar? |
|---|---|---|
| `UserServer` | request-shaped: one logical operation spans two messages | Yes — it is what the sugar is *for* |
| `Topic` | event-shaped: every message is a complete, independent transition | No — sugar would obscure it |
| `NoticeFetcher` | hybrid: makes requests but must stay reactive while they are outstanding | No — and *why not* is the instructive part |

The rule of thumb: **await earns its keep when a process makes requests and
wants their answers in sequence; plain handlers are the natural form when a
process reacts.**

Example 4 is different in kind: not a shape but a failure mode — the await
cycle that deadlocks under non-reentrancy — and the options for catching it
statically.

**Sibling file:** CONCURRENCY_EXAMPLES.effects.md re-expresses these programs
under C-10's hypothesis (a process is an effect handler bound asynchronously),
where the message structs, reply fields, and dispatch below are generated
rather than written. Reading the two side by side is C-10's decision test.

## Example 1 — `UserServer`: request-shaped, where await sugar wins

A `UserServer` answers `GetUser` requests. To answer one it must query a `Db`
process — a wait in the middle of a logical operation, which run-to-completion
forbids expressing as a block.

### Desugared — the hand-written form (and what the sugar compiles to)

```
// The message protocol — the process's send-surface is the union
// GetUser | DbReply (CONCURRENCY.md C-2(a))
struct GetUser { id: Int, reply: Reply<User | Err Str> }   // Reply<T> is linear
struct DbReply { id: Int, row: Str? }

// The process is just a struct. `pending` is the continuation state —
// the thing a coroutine compiler would hide inside a state machine.
struct UserServer {
    db: Pid<DbQuery>,
    pending: Mut Map<Int, Reply<User | Err Str>>
}

// One handler per mailbox type. The scheduler invokes these directly:
// a send IS scheduling one of these calls. No receive loop exists.
fn handle(s: Mut UserServer, msg: GetUser) [Send] {
    // Can't block. Park the linear reply handle, fire the request, return.
    // [linear-obligation] does the bookkeeping: the obligation to fulfil
    // msg.reply moved INTO the map — dropping it there would be a leak error.
    s.pending.put(msg.id, msg.reply)
    send(s.db, DbQuery {id: msg.id, reply_to: self})
}

fn handle(s: Mut UserServer, msg: DbReply) [Send] {
    // The manual resume: correlate, take the obligation back out, discharge it.
    let reply = s.pending.take(msg.id)
    when msg.row {
        is Str  { fulfil(reply, parse_user(msg.row)) }
        is None { fulfil(reply, err("no such user")) }
    }
}
```

### Sugared — one handler, one seam

```
fn handle(s: Mut UserServer, msg: GetUser) [Send] {
    let row = await request(s.db, DbQuery {id: msg.id})   // ← the seam
    when row {
        is Str  { fulfil(msg.reply, parse_user(row)) }
        is None { fulfil(msg.reply, err("no such user")) }
    }
}
```

What the compiler writes, mechanically — the analogue of `iter fn` writing the
pass struct:

1. **The parked state**: every local alive across the seam becomes a field of a
   generated pending entry — where C-9(ii)'s sendability/storability check must
   run.
2. **The resume handler**: everything after the seam, as a generated
   `DbReply`-shaped handler, plus the correlation key (`request` = `send` + a
   fresh id + registering the continuation — Erlang's `{pid, make_ref()}` as a
   compiler artifact).
3. **The linearity plumbing**: the continuation rides in a linear reply handle,
   so one-shot is the existing obligation analysis (C-9(iii)) — resume-twice is
   use-after-discharge, resume-never is the leak diagnostic reading "this
   request can never be answered".

*Known caveat:* `pending.take` assumes a collection can hold and yield linear
values by move — the L8/composite territory phase 3 fenced deliberately. The
generated storage may need its own story rather than a literal `Map`; flagged
in C-9(ii).

### What a `Reply<T>` is

Defined once, here, where it first appears (the effects file and CONCURRENCY.md
C-6/C-10 reference this block). A `Reply<T>` is the **reified continuation as
an address**: a one-shot, typed, linear send-capability pointing back at a
parked continuation. The type carries only `T` because that is the *contract*
— what the holder must provide; the routing lives in the **value**:

- the requester's Pid (which process to wake), and
- a continuation-slot id (*which* parked continuation in it — the pending
  entry with its captures).

Nearest relatives: `tokio::oneshot::Sender<T>`, Erlang's `From = {pid, ref}`,
and — closest in spirit — a one-shot, linear Gleam `Subject(t)`. It is also
the design's second token, symmetric with the first: `Pid<Protocol>` is a
many-shot address typed by a protocol; `Reply<T>` is a one-shot address typed
by a single value — the degenerate session "send me one `T` and we are done."

The lifecycle, and the invariant it protects:

1. **Minting.** The *requester's* process allocates the continuation slot
   (resume point + captured locals) when it parks, mints the token pointing at
   (own pid, slot id), and sends it inside the request.
2. **`fulfil` enqueues; it never executes.** `fulfil(reply, v)` constructs a
   resume message addressed by the token and queues it — no user code runs. If
   it invoked the continuation inline, requester code would execute on the
   fulfiller's thread mid-activation, breaking per-process serialization — the
   invariant the whole model rests on. Fulfil-as-enqueue is what keeps "one
   activation at a time" unconditionally true.
3. **The continuation runs as an ordinary activation of the requester's
   process**, on the requester's pool (C-5(b)). A gated requester admits it
   immediately (it is the one thing the gate lets through); an ungated one
   takes it in queue order.

Two rules that fall out:

- **`fulfil` can never block — capacity was reserved at request time.** The
  parked slot *is* the reply's landing place, allocated when the request was
  made, so a reply never competes for bounded-mailbox capacity (C-3) and
  `fulfil` contributes **no wait-for edge** to Example 4's deadlock graph.
  Without this exemption, replies through bounded mailboxes would create
  load-dependent deadlock cycles — the nastiest class.
- **Value-ness is what makes delegation work.** The token is ordinary sendable
  data: parked in a list (`NoticeFetcher.waiting`), moved into a map
  (`Searcher`'s `Gather`), captured in a `then` — or forwarded to a *third*
  process that fulfils on the original requester's behalf (Erlang's
  `gen_server:reply`-from-elsewhere). Linearity rides through every move
  [linear-obligation], so exactly-once holds however far the token travels.

Backends, briefly: on Rust, morally a `oneshot::Sender<T>` (queue handle +
slot index; `Send`, consumed by `fulfil`); on Kotlin, a
`CompletableDeferred<T>`-shaped pair. Neither needs runtime one-shot
enforcement — the static linearity check *is* the enforcement, which is the
"statically linear continuations" advantage over OCaml recorded in C-10.

*Deferred edge:* linearity guarantees no *code path* drops a `Reply`, but a
process that **dies** takes its parked obligations with it and the requester
hangs. That is not a token flaw; it is the supervision/monitor question
(Erlang's links exist for exactly this), owned by the supervisor-as-handler
story — flagged here, not solved here.

## Example 2 — `Topic`: event-shaped, where no sugar is needed

A pub-sub topic with batched delivery. Every message is a complete,
self-contained state transition; no message is "the middle of" anything.

```
// Four mailbox types: Subscribe | Unsubscribe | Publish | Tick
struct Subscribe   { who: Pid<Batch> }
struct Unsubscribe { who: Pid<Batch> }
struct Publish     { event: Event }
struct Tick {}                          // from a timer process

struct Batch { events: List<Event> }    // what subscribers receive

// The state is the whole story
struct Topic {
    subscribers: Mut Set<Pid<Batch>>,
    buffer: Mut List<Event>,
    max_batch: Int
}

fn handle(t: Mut Topic, msg: Subscribe) [Send] {
    t.subscribers.add(msg.who)
    if t.buffer.size() > 0 {
        // late joiner catches up immediately
        send(msg.who, Batch {events: copy t.buffer})
    }
}

fn handle(t: Mut Topic, msg: Unsubscribe) {
    t.subscribers.remove(msg.who)
}

fn handle(t: Mut Topic, msg: Publish) [Send] {
    t.buffer.add(msg.event)
    if t.buffer.size() >= t.max_batch {
        flush(t)                        // batch full → deliver now
    }
}

fn handle(t: Mut Topic, msg: Tick) [Send] {
    flush(t)                            // deadline → deliver whatever we have
}

// Handlers are just functions, so logic factors normally
fn flush(t: Mut Topic) [Send] {
    if t.buffer.size() == 0 { return }
    for sub in t.subscribers {
        send(sub, Batch {events: copy t.buffer})
    }
    t.buffer = mut_list_of()
}
```

Why await has nothing to offer here:

1. **No message has a continuation.** Every handler runs to its genuine end;
   after `Publish`, this process is not waiting to hear back about anything —
   the fan-out `send` is fire-and-forget.
2. **No `pending` map — and that's the tell.** Continuation-as-state appears
   exactly when a later message must be *correlated* with an earlier one.
   `Topic` correlates nothing: `Tick` doesn't answer `Publish`; it's an
   independent event touching the same buffer. The state is a domain model,
   not a parking lot for suspended control flow.
3. **Interleaving is the point, not a hazard.** A `Subscribe` between two
   `Publish`es is meaningful behaviour (the late joiner catches up), not a
   reentrancy bug. C-9(i) doesn't arise: no handler has a seam to interleave
   into.
4. **Run-to-completion is the correctness argument.** `flush` cannot observe a
   half-added subscriber; nothing arrives "during" a buffer swap. Per process,
   the code reads like single-threaded code because it is.

*C-2 note:* `Tick` in the same union as `Publish` means per-type mailboxes give
it no ordering relative to publishes — a tick can jump ahead of buffered
publishes. For a flush deadline that is harmless (arguably desirable), but it is
the cross-type-ordering caveat of C-2(a) showing up in a concrete program.

## Example 3 — `NoticeFetcher`: the hybrid, where hand-written continuation state is right

From the user's own experience (2026-09-14). The original, thread-based
implementation: any number of concurrent callers ask for a notice to work on. A
**background thread** blocks until at least one requester exists; then it
fetches a batch from DynamoDB — possibly *more* items than there are waiting
requesters; it assigns items to requesters, and whatever it cannot assign
within a time limit it **returns to DynamoDB**.

The run-to-completion translation:

```
// Protocol: Request | FetchDone | Tick | Shutdown
struct Request   { reply: Reply<Notice | Err Str> }  // Err: refused (shutdown)
struct FetchDone { notices: List<Notice> }           // the Db process's reply
struct Tick      { gen: Int }                        // a return-deadline, tagged
struct Shutdown  { done: Reply<Done> }               // graceful-stop request
struct Done {}

struct NoticeFetcher {
    waiting: Mut List<Reply<Notice | Err Str>>, // parked requesters — obligations
    stock: Mut List<Notice>,            // fetched, not yet assigned — also owed!
    stock_gen: Int,                     // which batch the pending Tick is for
    fetch_in_flight: Bool,
    stopping: Bool,
    shutdown_done: Reply<Done>?,        // parked when stopping with fetch in flight
    db: Pid<DbFetch | ReturnNotices>,
    timer: Pid<After>,
    batch_size: Int,
    return_deadline: Duration
}

fn handle(f: Mut NoticeFetcher, msg: Request) [Send] {
    if f.stopping {
        fulfil(msg.reply, err("shutting down"))    // refused, but answered
        return
    }
    if f.stock.size() > 0 {
        fulfil(msg.reply, f.stock.take_first())    // assign from stock, done
        return
    }
    f.waiting.add(msg.reply)                       // park the obligation
    ensure_fetching(f)
}

fn ensure_fetching(f: Mut NoticeFetcher) [Send] {
    if f.fetch_in_flight { return }
    f.fetch_in_flight = true
    send(f.db, DbFetch {count: f.batch_size, reply_to: self})
}

fn handle(f: Mut NoticeFetcher, msg: FetchDone) [Send] {
    f.fetch_in_flight = false
    if f.stopping {
        // Shutdown ran while the fetch was in flight: nothing may be assigned
        // any more — the whole batch goes straight back...
        send(f.db, ReturnNotices {notices: msg.notices})
        // ...and the shutdown that was waiting on this fetch completes now.
        if f.shutdown_done is Reply<Done> {
            fulfil(f.shutdown_done, Done {})
        }
        return
    }
    for notice in msg.notices {
        if f.waiting.size() > 0 {
            fulfil(f.waiting.take_first(), notice) // assign: obligation moves
        } else {
            f.stock.add(notice)                    // hold, under deadline
        }
    }
    if f.stock.size() > 0 {
        f.stock_gen = f.stock_gen + 1              // this batch's deadline...
        send(f.timer, After {delay: f.return_deadline,
                             then: Tick {gen: f.stock_gen}, to: self})
    }
    if f.waiting.size() > 0 {
        ensure_fetching(f)                         // demand still unmet → refetch
    }
}

fn handle(f: Mut NoticeFetcher, msg: Tick) [Send] {
    if msg.gen != f.stock_gen { return }   // stale deadline: stock has turned over
    if f.stock.size() > 0 {
        // couldn't assign in time → give them back
        send(f.db, ReturnNotices {notices: f.stock.drain()})
    }
}

fn handle(f: Mut NoticeFetcher, msg: Shutdown) [Send] {
    f.stopping = true
    for r in f.waiting.drain() {
        fulfil(r, err("shutting down"))    // every parked requester is answered —
    }                                      // the leak diagnostic forces this loop
    if f.stock.size() > 0 {
        send(f.db, ReturnNotices {notices: f.stock.drain()})
    }
    if f.fetch_in_flight {
        f.shutdown_done = msg.done         // park: finish when FetchDone lands
    } else {
        fulfil(msg.done, Done {})          // nothing outstanding: done now
    }
}
```

What the translation teaches — four observations, each generalising past this
example:

1. **The background thread dissolves.** Its entire job was *coordination*:
   block until a requester exists, wake, loop. Here that machinery — the
   thread, the condition variable, the wait/notify handshake — becomes nothing
   at all: the arrival of a `Request` message *is* the wake-up, and
   `ensure_fetching`'s flag is the only residue. The original design's hardest
   part (getting the blocking/waking right) has no analogue because there is
   nothing to block.
2. **This is the case where await sugar would be *wrong* — the reentrancy call
   made concrete (C-9(i)).** One might try `let batch = await request(f.db,
   DbFetch {…})` inside the `Request` handler. Under C-9's recommended
   *non-reentrant* default, the process would then accept no further `Request`s
   while the fetch is outstanding — but accumulating requesters *during* the
   fetch is the point of the design (fetch once, assign to many). The
   `fetch_in_flight` flag plus the `FetchDone` handler is exactly a *reentrant*
   await, written by hand, precisely where reentrancy is wanted. The rule of
   thumb sharpens: await suits a process whose requests are *per-message*
   sequential; a process that must keep reacting while its request is in
   flight wants the explicit form — and having both in one model, chosen per
   handler, is something neither Orleans (non-reentrant default) nor Swift
   (reentrant always) offers so plainly.
3. **"Assign or return" is a linear obligation with two dischargers.** The
   original design's subtlest invariant — a fetched notice must be either
   assigned to a requester or returned to DynamoDB, never dropped — is exactly
   the shape `linear struct` + a discharge set already expresses
   ([linear-obligation]; LANGUAGE.md's discharge sets, e.g. `stop` *or* `join`
   for a thread handle). Make `Notice` linear with dischargers
   `fulfil`-assignment and `ReturnNotices`, and losing a notice becomes a
   compile-time leak diagnostic instead of a production incident and an
   at-least-once redelivery hope. The `stock` of linear values meets the same
   collections-of-linears caveat as Example 1's `pending`.
4. **Both waits in the original become messages, but differently.** The
   requester's wait ("call and block until a notice arrives") is C-9 sugar on
   the *requester's* side: `let notice = await request(fetcher, Request {})`.
   The time limit ("assign within N seconds or return") is a `Tick` from a
   timer process — the same move as `Topic`'s flush deadline. One wait was a
   continuation; one was an event; the model files each where it belongs.

*On the two hardening details, both now in the code above:*

- **The generation counter** closes the `Tick`/`FetchDone` race: a deadline is
  for *a particular batch* of stock, so `Tick` carries the `stock_gen` it was
  scheduled against and a stale tick (stock drained and refilled since) is
  ignored. Three lines of ordinary state — no cancellation API, no timer
  handle: the timer still fires, and the *receiver* decides the event no longer
  means anything. That is the idiomatic run-to-completion answer to
  cancellation generally: don't retract the message, make it ignorable.
- **The shutdown path** is forced, not optional: every `Reply` parked in
  `waiting` is a linear obligation, so a `Shutdown` handler that forgot the
  drain loop would be a leak diagnostic at compile time. Note that shutdown
  itself uses continuation-as-state — "wait for the in-flight fetch to land"
  parks `msg.done` in `shutdown_done`, exactly the pattern of Example 1, three
  fields big. Graceful stop is two-phase by nature (`stopping` flag, then
  completion when the last outstanding request returns), and the model makes
  the phases explicit messages. The `shutdown_done: Reply<Done>?` field is a
  linear value in a union arm — legal per phase 3's O-C2 (owes while
  un-narrowed, narrowing discharges) — but *taking* it out of a `Mut` struct
  field to fulfil it brushes the same fence as the collections-of-linears
  caveat; real syntax for that move is design work C-9(ii) owns.

### Example 3b — the same fetcher, if await is allowed to be reentrant

C-9(i) recommends non-reentrant as the default. Suppose instead the sugar is
**reentrant**: while an activation is parked at a seam, the process keeps
handling other messages. Could `NoticeFetcher` then be written with `await` and
lose its `FetchDone` handler? Yes — and seeing exactly what changes (and what
does not) is the best argument in this document for treating reentrancy as the
load-bearing call.

```
fn handle(f: Mut NoticeFetcher, msg: Request) [Send] {
    if f.stopping { fulfil(msg.reply, err("shutting down")) return }
    if f.stock.size() > 0 { fulfil(msg.reply, f.stock.take_first()) return }
    f.waiting.add(msg.reply)

    if f.fetch_in_flight { return }        // another activation is already fetching
    f.fetch_in_flight = true
    let batch = await request(f.db, DbFetch {count: f.batch_size})  // ← seam
    f.fetch_in_flight = false

    // Everything below runs LATER. Every claim established above is stale:
    // Requests were parked at the seam, Ticks fired, Shutdown may have run.
    if f.stopping {                        // ← mandatory re-check, easy to forget
        send(f.db, ReturnNotices {notices: batch})
        if f.shutdown_done is Reply<Done> { fulfil(f.shutdown_done, Done {}) }
        return
    }
    for notice in batch {
        if f.waiting.size() > 0 { fulfil(f.waiting.take_first(), notice) }
        else { f.stock.add(notice) }
    }
    if f.stock.size() > 0 {
        f.stock_gen = f.stock_gen + 1
        send(f.timer, After {delay: f.return_deadline,
                             then: Tick {gen: f.stock_gen}, to: self})
    }
    if f.waiting.size() > 0 { ensure_fetching_sugared(f) }  // refetch tail
}
```

What the comparison shows:

- **What disappears: only the correlation plumbing.** One handler instead of
  two; `batch` is a local instead of a message field; no compiler-generated (or
  hand-written) pending entry for the fetch. That is the whole win.
- **What does not disappear: every piece of coordination state.** `waiting`,
  `fetch_in_flight`, `stopping`, `shutdown_done`, `stock_gen` all survive,
  because interleaved activations coordinate *through* them — the requester
  parked at line 4 is served by a *different* activation's post-seam code.
  Reentrancy inlines the continuation; it does not remove the coordination.
  Line for line, 3b is barely shorter than 3.
- **What gets worse: claims must be re-established after every seam.** The
  `f.stopping` re-check after the await is mandatory and is exactly the bug
  class Swift actors are known for: the code *visually continues*, inviting the
  reader (and writer) to carry pre-seam facts across. In the split version this
  protection comes free from structure — `handle(f, msg: FetchDone)` is a fresh
  function, so "start by checking state" is its obvious first line. The
  qualifier-invalidation idea in C-9(i) — a claim invalidated by an await, the
  way claims are invalidated by mutation — is what would make the reentrant
  form mechanically safe rather than convention-safe; without it, non-reentrant
  or hand-written are the forms whose safety comes from shape.
- **A shape oddity worth naming: one requester's activation becomes the
  coordinator.** The activation that happened to trigger the fetch does the
  assignment loop *for everyone* — its own `msg.reply` was parked long ago and
  may not even be the first one served. Harmless, but the handler has outlived
  its own message's relevance, which is a sign the logic was never really
  per-message.

**The general lesson, sharpening C-9(i):** reentrant await wins when awaiting
activations are *independent* — Example 1's `UserServer` under reentrancy loses
its `pending` map entirely *and* keeps serving concurrent `GetUser`s, each
activation owning its own locals; that is the clean win case. It wins little
when activations *coordinate through shared state* — `NoticeFetcher`'s state is
the coordination, so the sugar removes one seam's plumbing and imports a
re-validation obligation at that seam. Per-handler choice (await where
activations are independent, explicit handlers where they coordinate) is the
combination neither Orleans (non-reentrant only) nor Swift (reentrant always)
offers — and Salvo choosing *non-reentrant default, explicit form for
coordination* gets the same coverage with the smaller checker burden.

## Example 4 — the non-reentrant deadlock, and detecting it statically

C-9(i) recommends non-reentrant await as the default. This is its cost, in the
smallest realistic shape: **two processes that each await the other**. Neither
handler is wrong on its own; the bug is a property of the pair.

```
// Each service holds a Pid of the other — a mutually-consulting topology.
struct OrderService {
    customers: Pid<CreditCheck>,
    open: Mut Map<Int, List<Order>>
}
struct CustomerService {
    orders: Pid<OpenOrders>,
    accounts: Mut Map<Int, Account>
}

// Placing an order consults the customer's credit:
fn handle(o: Mut OrderService, msg: PlaceOrder) [Send] {
    let credit = await request(o.customers, CreditCheck {id: msg.customer_id})
    when credit {
        is Approved { o.open.get_or_new(msg.customer_id).add(msg.order) }
        is Denied   { fulfil(msg.reply, err("credit denied")) }
    }
}

// Deleting a customer consults their open orders:
fn handle(c: Mut CustomerService, msg: DeleteCustomer) [Send] {
    let open = await request(c.orders, OpenOrders {id: msg.id})
    if open.size() == 0 {
        c.accounts.remove(msg.id)
    }
    ...
}
```

The deadly interleaving, step by step:

1. `OrderService` begins `PlaceOrder` and parks at its seam — its `CreditCheck`
   request is now in `CustomerService`'s mailbox. Non-reentrant: `OrderService`
   handles **nothing** until the reply arrives.
2. Before picking up `CreditCheck`, `CustomerService` begins `DeleteCustomer`
   (it was already in its mailbox) and parks at *its* seam — its `OpenOrders`
   request lands in `OrderService`'s mailbox.
3. `CreditCheck` sits unprocessed in a parked process's mailbox. So does
   `OpenOrders`. Both processes wait forever on a reply the other can never
   produce; neither pool thread is blocked (the schedulers are idle and
   healthy), which makes it *quieter* than a thread deadlock — nothing is even
   spinning.

Note what makes this the worst failure class: it is **interleaving-dependent**.
If `CustomerService` happens to process `CreditCheck` before `DeleteCustomer`,
everything works. The system runs fine in every test and deadlocks in
production on the rare crossing — precisely the argument for catching the
*possibility* statically rather than the occurrence at runtime. (The degenerate
form — a process whose await targets, directly or through helpers, *itself* —
deadlocks unconditionally, and is the easy case every approach below catches.)

### Static approaches

**(a) The await graph over process types.** Build a graph at compile time: one
node per process type, an edge `P → Q` wherever a handler of `P` contains an
await whose request targets a `Pid` of `Q`'s protocol. A cycle is a potential
deadlock — report it with the participating handlers and seams named. Salvo is
unusually well-placed for this: the language already assumes it can see
everything ([call-resolve] and kin — no dynamic loading, no unknown callees),
and `Pid<Protocol>` is typed, so every await edge is statically knowable
whole-program. *The imprecision to be honest about:* the graph is over process
**types**, not instances. A chain of `Worker`s each awaiting the next is a
type-level self-loop but instance-level acyclic — a false positive. Sound
(catches every possible cycle), not precise.

**(b) Surface awaits as an effect, so the graph is modular.** If awaiting is an
effect parameterised by the target protocol — a handler that awaits `Q`
carries `[Await<Q>]`, and helper functions propagate it exactly as effects
already propagate — then the await graph falls out of *signatures* rather than
whole-program body analysis. Effects are already how Salvo makes dependencies
visible in signatures; this makes "who might this handler wait on" part of the
same discipline, the diagnostic composable ("this call adds the edge
`OrderService → CustomerService`; the cycle closes here"), and the graph
construction incremental. (a) and (b) are the same check; (b) is where it
lives.

**(c) Stratification, via qualifiers: make cycles unwritable.** The classic
lock-ordering discipline as a type rule: processes are assigned *tiers* (a
declared partial order), and an await edge must go **strictly downward** —
awaiting an equal or higher tier is an error at the seam. Acyclicity by
construction, no graph analysis, and instance-precision where (a) is blind:
the `Worker` chain stratifies by giving each spawn a descending tier. Salvo
has a natural home for the tier: a qualifier on the `Pid` (provenance-style —
a claim about where the handle sits in the topology, not about its contents).
And OTP practice suggests the annotation burden is low: supervision topologies
are overwhelmingly *trees*, which stratify trivially. What it costs: genuinely
peer-to-peer topologies cannot be stratified and would need the (d) fallbacks
— arguably a feature, since a peer-to-peer await mesh is exactly the design
that deserves friction.

**(d) The fallbacks, for what static analysis refuses.** Where a cycle is
reported but the developer knows the instance topology is acyclic (or accepts
the risk): a timeout form — `await_within(d, request(...))` returning
`Reply | TimedOut` — turns a would-be deadlock into a typed error path the
`when` must handle; a per-edge *reentrant* opt-in breaks the cycle by letting
the marked process keep serving while parked (importing Example 3b's
stale-claim burden, deliberately, at one named seam); and Orleans-style
runtime cycle detection can ride in debug builds as the diagnostic of last
resort. These are escape hatches, not the mechanism — the design goal is that
(a)/(b) report the cycle and (c) or a fallback is the *written, visible*
answer to it.

**One more edge set, owed to C-3.** Await seams are not the only wait-for
edges: a *blocking send* into a full bounded mailbox (C-3(a)) from inside a
handler blocks a pool thread and creates a `P → Q` edge that exists **only
under load** — a cycle through such edges deadlocks only when the mailboxes
involved are simultaneously full, which is rarer and nastier than the await
case. If sends from handlers keep blocking semantics under C-5(d), the same
graph must include them (with edges conditioned on boundedness); the
alternative — make `send` from inside a handler non-blocking-or-error, pushing
back-pressure handling to an explicit form — is a C-3/C-5(d) interaction the
substrate decision should settle explicitly.

**Recommendation (the user's call):** (a)+(b) as the baseline — the effect-
surfaced await graph is cheap, whole-program sound, and lands the diagnostic at
the seam that closes the cycle; (c) as the opt-in precision tool where the
type-level graph is too coarse, with tree-shaped supervision making it near
free in practice; (d) as explicitly-written escape hatches. This keeps the
promise pattern of the rest of the language: the default is checked and
conservative, and every loosening is visible in the source.
