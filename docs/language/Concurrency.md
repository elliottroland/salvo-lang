# Where work runs

Concurrency in Salvo is built out of two things that run on **pools**. A pool is a set of worker threads (`pool(4)`), or exactly one (`thread()`), and it is an ordinary value: one pool can host many actors.

An actor first, since everything else on this page is about *where* its work
runs. An `actor effect` declares the protocol — `send fn` members, which
enqueue and answer nothing — a handler of it is ordinary Salvo, and `spawn`
gives it a mailbox:

```
actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 8 }

    sum: Int = 0

    send fn bump(n: Int) {
        sum = sum + n          // no lock: activations run one at a time
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)          // `Reply` is linear — answered exactly once
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let workers = pool(2)
    let counter = spawn Counting() on workers

    counter.bump(2)            // enqueues; returns immediately
    counter.bump(3)

    let sum = waitfor answer: Reply<Int> { counter.total(answer) }
    println("${sum}")          // 5
}
```

The state is the actor's alone and its members run one at a time, so the
serialization *is* the mutual exclusion. `waitfor` is the bridge for a frame
that does mean to wait — no capability, no declaration.

The two kinds of work are:

* an **activation** — one member invocation of an actor, which is a handler bound with `spawn` instead of `use`. An actor's activations run one at a time, in the order their invocations arrived, on some worker of the actor's pool. That serialization *is* its mutual exclusion.
* a **task** — the body of a free `send fn`, scheduled rather than called. A task has no mailbox, no state and no identity; it exists because `replyto` may target a free `send fn`, so an ordinary synchronous function can wire future work and return:

```
send fn finish(label: Str, out: Reply<Str>, row: Int) => !label, !out, !row {
    out.send("${label}=${row}")
}

fn fetch(id: Int, out: Reply<Str>) [Db] -> None => !out {
    db.query(id, replyto finish("row", out))   // wires the work, then returns
}
```

**A mint schedules nothing.** `replyto finish(…)` allocates a token and decides *where* the continuation will run; the body runs only when someone sends to that token. Sending to it queues the task on that pool, and a worker of that pool runs it.

**Placement is inherited unless you write it.** `on POOL` is optional at a mint, and omitted means the pool current where the mint was written — inside an actor's member, the actor's own pool; in `main`, main's own pool. Whoever creates work pays for it, so a client cannot spend a shared service's threads by accident, and an actor's continuations stay on the threads its author budgeted. Write `on p` when the work belongs somewhere else.

**One serving rule, and one ordering.** A worker serves its own pool. It takes a queued task **before** a pending activation, and each queue is served in arrival order. Both backends do exactly this, so an interleaving does not depend on which one you compiled for.

That leaves one question — *which* thread serves the pool, and what else that thread might be doing. The cases:

* **A pool with several workers.** The task runs as soon as any worker is free. Nothing more to know.
* **A one-thread pool whose thread is idle.** The task runs immediately.
* **A one-thread pool whose thread is inside a blocked activation.** A `waitfor` **serves its own pool while it waits**, so the task still runs — nested on that thread, while the waiting actor's mailbox stays stalled. This is not an optimisation, it is what makes the inherit-by-default rule safe: an actor that mints a task on its own pool and then waits for that task's answer would deadlock against itself if a wait merely blocked.
* **The main pool.** `main` is the single worker of a pool of its own, and it never gets a thread besides its own — so main-pool work runs **only while `main` waits**, on main's own thread, nested inside the `waitfor`. This is what makes `main` and an actor indistinguishable to a function that mints: a mint from `main` has somewhere to land.

The main pool has one consequence worth stating on its own. **If the answer arrives after `main`'s last `waitfor`, the task never runs.** It is not lost work that will be picked up later: nothing else serves that pool, and everything still queued dies when `main` returns. From `main`, a task is *reached* by a subsequent wait, so "wire it and forget it" is the one shape that quietly does nothing. Place the work `on` a pool with workers of its own if it must proceed regardless of what `main` does next.

What a wait serves is worth being precise about, because it is asymmetric:

* An **actor's** wait serves its pool's tasks and other actors' activations, but never its own — re-entering an actor mid-activation is exactly what serialization exists to prevent. On a dedicated thread (`thread()`), which by linearity has exactly one occupant, that means it serves tasks only.
* A **task's** wait excludes nothing, because a task belongs to no actor. It cannot re-enter a running actor regardless: an actor with an activation in progress has nothing deliverable.
* Blocking is just the degenerate case of an empty queue. There is no separate "blocking" and "pumping" semantics to reason about.

Two more cases complete the picture:

* **A token that is never sent to.** The task never runs — but you cannot get there by forgetting, because a `Reply<T>` is linear: whoever holds it must send to it or pass it on. What can still happen is that its holder *dies* first (a faulted actor loses what it owed), and then the runtime's idle report names the waiter that can no longer be answered instead of hanging.
* **A task that faults.** It has no identity, so there is nothing to `watch`. The fault goes to its pool's **fault sink** if the pool was given one — `pool(4, sink)`, where `sink` is the addr of an actor serving `Faults` — and otherwise is named on stderr. Either way the program carries on: a task's death is not the program's.

**Asking when the work is done.** A near relative of that detection is available to a program (near, not the same: the hook reads the stricter condition — it does not fire while any frame is parked in a wait, where the report counts a parked frame out of the running ones, since a parked frame cannot get anywhere on its own): `on_idle(p, notify)` registers a one-shot for the moment nothing anywhere can run, and answers an `Idle` saying what pool `p` is still owed — `parked_gates`, the actors placed there whose mailbox is gated on a reply, and `parked_tokens`, the reply tokens aimed at work there that nobody has discharged. Both zero means the program is *finished*, not merely quiet.

```
let p = pool(2)
counter.bump(2)                                  // … place work on p …
let settled = waitfor i: Reply<Idle> { on_idle(p, i) }
println("gates ${settled.parked_gates}, tokens ${settled.parked_tokens}")
```

The token is minted like any other and **consumed** by the registration, so a hook you forget to register is the ordinary linearity error rather than a request that quietly never answers. The answer is edge-triggered and one-shot, because delivering it is itself work and ends the idleness that produced it: hearing about the next one means registering again. And it says what it says only while nothing outside the scheduler injects work — a platform handler with a thread of its own can make "idle" stale.

Finally, what a task body may *do*. It is ordinary Salvo, with one restriction: it declares no effects. A task runs detached from the frame that minted it — that frame may have returned by the time it runs — so there is no scope left to supply its handlers from. Reaching an actor needs no effect declaration, so the way to give a task a capability is to hand it an `Addr` as a capture and send to it; anything else belongs in the function that mints. A task may wait (`waitfor` needs no declaration anywhere), and a wait serves the pool it runs on.

## Across machines — the wire (step ① of the network sequence)

Actors will run across a network, and the design that lands them (user decisions 2026-09-26; ROADMAP.md section 2) has one line: **the network enters at the actor group, never at the spawn.** Every actor is spawned by the node that hosts it, on its own pools; what crosses the wire is addresses and messages; a function declaring `[E]` never learns whether `E` is one local handler, an actor, or a group of a hundred across a fleet. The groups, the codecs and the routing arrive in later steps. What exists now is the bottom of the stack, module `net`: the wire.

```
import net

handler Receiving() [Console] of Inbound {           // frames arrive as messages
    mailbox { capacity: 16 }
    send fn frame(from: NodeEndpoint, data: Bytes) => !from, !data {
        println("${to_str(from)}: ${str_of_bytes(data) ?: "?"}")
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let me = NodeEndpoint { host: "127.0.0.1", port: 7000 }
    use HostTcpTransport(copy(me))                    // the wire: a host class
    listen(copy(me), spawn Receiving() on pool(1))    // receive here
    deliver(copy(me), to_bytes("hello"))              // Ok: accepted by the wire
}
```

Two halves, deliberately asymmetric. **Outbound** is a plain effect, `Transport`: `deliver(to, frame)` is a call on the sender's thread that answers whether the frame was handed to the wire — `Ok` means accepted, never that the far side has it. **Inbound** is an *actor* effect: bytes arrive on a thread the host owns, and the only way work from a foreign thread enters the scheduler is a send. So a node that wants to receive spawns an actor serving `Inbound` and registers its addr with `listen`, and the transport sends every arriving frame to it. Delivery is at most once and in order per pair, and nothing more; anything stronger is a protocol over this.

`HostTcpTransport` is the machine's transport and std's first **`threadsafe` platform handler** (Backends.md) — a transport is reached from every pool at once, so the host synchronizes itself and the compiler shares it with no lock. Its listener also tells the scheduler that an outside source of work is open, so a program waiting for a frame is neither idle nor deadlocked while the wire may still deliver one.

`MemTransport` is the double every later step tests on: one `MemNetwork` actor many virtual nodes share (with `partition`, `heal` and `kill` as its members, so a test scripts the faults), and one `MemTransport(me, net)` per node. It runs the same `Inbound` actors, frames and errors the real transport does, in one process, on both backends, with identical output.

### The wire format, and `noremote`

What crosses the wire is bytes in one **canonical encoding** the compiler owns — stated in one place, produced and read identically by both backends, so a Kotlin node and a Rust node are members of one group. Every type has a wire form **by default**: scalars, strings, `Bytes`, lists, tuples, optionals, unions (one tag byte holding the arm's declared index) and structs (fields in declaration order). The opt-out is a word on the declaration:

```
noremote struct Canvas { surface: GpuSurface }    // process-local: no wire form
struct Frame { title: Str, canvas: Canvas }       // therefore none either — transitively
```

`encode(value)` answers the bytes and `decode<Point>(data)` reads them back (`None` when the bytes are not a well-formed `Point`), and both are refused at the call for a type with no wire form, naming the field or declaration that stops it. std's process-local handles — `Pool`, the file streams — are `noremote` by declaration; a function value and a `proj` view are, by construction. Keyed containers (`Set`, `Map`) are the one recorded gap: their identity capabilities do not travel yet, so send a `List` of the entries.

Every actor protocol whose payloads all have a wire form also gets a **canonical hash** — over its members' shape, with structs expanded to their fields, so a renamed struct is the same protocol and a reordered field is not. Two nodes may talk on a protocol exactly when their hashes agree, and that is settled at the handshake, never at decode: an exhaustive `when` has no arm for a message it has never heard of, so the language cannot decode leniently and refuses early instead.
