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

Inside an actor, `replyto` more often targets one of the actor's **own** members — usually a private one. A `send fn` that no face declares is a private member: nothing outside the handler can send to it, and the only ways in are `k@self(…)` from a sibling member and `replyto k(…)`. Its parameters are consumed like any message's, so its clause is written out, and its trailing parameter is the answer the continuation carries:

```
handler Gathering() of Gather {
    mailbox { capacity: 16 }
    total: Int = 0
    send fn scatter(word: Str, members: List<Addr<Search>>, out: Reply<Int>) => !word, !members, !out {
        for m in members { m.query(copy(word), replyto partial()) }   // one continuation per member
        …
    }
    send fn partial(n: Int) => !n { total = total + n … }              // private: no face declares it
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

## Across machines

Everything above runs in one process. The same actors run across a network, and the design has one line: **the network enters at the actor group, never at the spawn.** Every actor is spawned by the node that hosts it, on that node's own pools. What crosses the wire is addresses and messages — never construction — and a function declaring `[E]` never learns whether `E` is one local handler, an actor on another machine, or a group of a hundred across a fleet.

The layers, bottom up: a transport that moves bytes; a wire format the compiler owns; addresses that route; a group of nodes; a group of actors across those nodes; and a way to bind such a group where a program expects one effect. Each layer is ordinary Salvo in module `net`, and each runs unchanged over an in-memory transport, which is how a two-node program runs in one process and prints the same thing on both backends.

### The wire

At the bottom is module `net`'s transport: bytes between two **endpoints**.

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

The two halves are deliberately asymmetric. **Outbound** is a plain effect, `Transport`: `deliver(to, frame)` is a call on the sender's thread that answers whether the frame was handed to the wire — `Ok` means accepted, never that the far side has it. **Inbound** is an *actor* effect: bytes arrive on a thread the host owns, and the only way work from a foreign thread enters the scheduler is a send. So a node that wants to receive spawns an actor serving `Inbound` and registers its addr with `listen`, and the transport sends every arriving frame to it. Delivery is at most once and in order per pair of endpoints, and nothing more; anything stronger is a protocol over this.

The transport is the one piece of the network the platform owns. `HostTcpTransport` is the machine's, a **`threadsafe` platform handler** ([Backends](Backends.md)): it is reached from every pool at once, so the host synchronizes itself and the compiler shares it with no lock. Its listener also tells the scheduler that an outside source of work is open, so a program waiting for a frame is neither idle nor deadlocked while the wire may still deliver one.

`MemTransport` is the in-memory transport: one `MemNetwork` actor that many virtual nodes share — with `partition`, `heal` and `kill` as its members, so a test scripts the faults — and one `MemTransport(me, net)` per node. Everything above the wire runs on it exactly as on TCP, in one process.

### The wire format, and `noremote`

What crosses the wire is bytes in one **canonical encoding** the compiler owns, produced and read identically by both backends, so a Kotlin node and a Rust node can be members of one group. Every type has a wire form **by default**: scalars, strings, `Bytes`, lists, tuples, optionals, unions (one tag byte holding the arm's declared index) and structs (fields in declaration order). The opt-out is a word on the declaration:

```
noremote struct Canvas { surface: GpuSurface }    // process-local: no wire form
struct Frame { title: Str, canvas: Canvas }       // therefore none either — transitively
```

`encode(value)` answers the bytes and `decode<Point>(data)` reads them back (`None` when the bytes are not a well-formed `Point`); both are refused at the call for a type with no wire form, naming the field or declaration that stops it. std's process-local handles — `Pool`, the file streams — are `noremote` by declaration; a function value and a `proj` view are, by construction. Keyed containers (`Set`, `Map`) have no wire form yet: send a `List` of the entries.

Every actor protocol whose payloads all have a wire form also has a **canonical hash** — over its members' shape, with structs expanded to their fields, so a renamed struct is the same protocol and a reordered field is not. Two nodes may talk on a protocol exactly when their hashes agree, and that is settled when the nodes meet, never at decode: an exhaustive `when` has no arm for a message it has never heard of, so the language cannot decode leniently and refuses early instead.

### Addresses across machines

An `Addr<E>` that crosses the wire arrives as a **proxy**: the same type, the same `send`, and a mailbox on another node behind it. Nothing in the code that holds it changes — locality is a fact the runtime reads at the send. A `Reply<T>` crosses the same way, and the answer comes back over the wire into whatever was waiting: `main`'s `waitfor`, an actor's parked continuation, a task.

What keeps it honest across a machine is what keeps it honest across a thread. The mailbox's `capacity` is enforced by **credits**: a proxy sends only when the far mailbox has granted room, blocks at zero exactly as a local send blocks on a full queue, and is granted a credit back as each message is dequeued. And an address carries **unguessable bits** minted at the spawn and checked on delivery, so an addr is a capability on the wire as it is in memory — the administrative face of a two-face handler cannot be reached by whoever holds only the public one.

A process is one node; it may also **host virtual nodes** (`new_node()`, `pool_at(node, n)`), which is how a program runs two nodes in one process over `MemTransport` and exercises every remote path — proxies, credits, frames, remote replies — with no socket.

### Node groups

Which nodes exist is a **membership** question, and the answer is an actor effect, `NodeGroup`, whose handlers are the mechanisms. A node joins by spawning one on its own pool — `StaticNodeGroup("demo", me, all)` when the fleet is fixed, `GossipNodeGroup("mesh", me, seeds)` when it is not — and asks it `members(out)`, or `subscribe(w)` to hear `NodeChanges.joined` and `left` as messages.

```
let group = start_group(spawn GossipNodeGroup("mesh", copy(me), seeds) on p)
group.subscribe(spawn Announcing() on p)                    // "+ b:1", "- b:1 (left)"
let peers = waitfor out: Reply<List<Node>> { group.members(out) }
```

The handshake underneath is the runtime's, common to every mechanism: the group's name (so two deployments on one network refuse each other by name), the node's endpoint, and its protocol table — which is what lets a node refuse to talk to a peer whose version of a protocol differs before a single message is decoded.

### Actor groups

Nodes are where actors live; the set of actors a program spreads across them is an **actor group**, `ActorGroup<E>` — an actor effect over the protocol `E`. `attach(protocol<Ping>(), nodes)` starts one on the current node and publishes it by name, so a node that attaches the same protocol to the same node group gets a replica of the same group: replicas exchange their members as nodes meet, admit each remote member once, and withdraw every member hosted on a node that leaves. `join(group, addr)` adds a member, `members(out)` answers the union as seen from here, and `subscribe(w)` delivers `ActorChanges<Ping>.joined` and `left` as ordinary messages, each carrying an `Addr<Ping>` that works wherever it arrives.

```
let pings = attach(protocol<Ping>(), nodes)             // one replica per node
join(pings, spawn Pinging("a") on p)
pings.subscribe(spawn Noticing("a") on p)               // "a: + a Ping (local: false)"
```

`protocol<E>()` is where a protocol that cannot cross a node boundary is refused: if any `send fn` of `E` carries a `noremote` payload, a group of `E` cannot span nodes, and the compiler says so at that call rather than at the first remote send. The type parameter of `ActorGroup<E>` is only ever an effect, so it costs nothing at runtime — `Addr<E>` is one handle whatever `E` is, and a generic that is only ever an effect is erased from the generated code.

Three small readers go with a group: `pending(addr)` answers how much a member has in front of it — its mailbox depth if it is local, the in-flight count if it is a proxy; `node_of(addr)` names the host; and two `Addr<E>` compare with `eq`.

### One member of many: `any`

A function written against one actor — `bump(2); bump(3); total(out)` — relies on its sends arriving in order at one instance. A group cannot promise that, so a function that is happy with *any* member says so in its effect list, and a handler that forwards to many members says so in its `of` clause:

```
fn thumbnail(img: Image) [any Resizer] -> Image { … }    // any Resizer will do: one send, one reply
fn tally(items: List<Int>) [Resizer] -> Int { … }         // needs ONE Resizer, in order

handler RoundRobin(members: List<Addr<Resizer>>) of any Resizer { … }   // forwards; promises no order
```

Bare `[E]` keeps the strong meaning, so binding a group where `[Resizer]` is required is an error at the call, and a program written before groups existed cannot be broken by binding one under it. `[any E]` accepts every binding — a single instance is a group of one — and is viral downward like `local`. The full rule is in [Effects and handlers](Effects-and-Handlers.md#many-instances-any-e-and-of-any-e).

### Routing: `use route(group)` and `Pick<E>`

A program that wants *a* member rather than the set binds the group as the effect itself. `use route(stock)` binds `any Inventory` — each send goes to whichever member a **policy** picks — and the policy is an ordinary handler bound just before it:

```
actor effect Inventory {
    send fn reserve(sku: Key Str, qty: Int, out: Reply<Str>) => !sku, !qty, !out
}

fn checkout(skus: List<Str>) [any Inventory, Console] -> None { … }   // `any`: shards are unordered

let stock = attach(protocol<Inventory>(), nodes)
join(stock, spawn Stocking("s1") on p)
use Sharded<Inventory>()          // the policy: the member that owns the key…
use route(stock)                  // …and the stub, binding `any Inventory`
checkout(["apple", "pear", "apple"])   // "apple" lands on the same shard both times
```

`route` is a handler the compiler writes for you: one `of any Inventory` whose every member asks `Pick<Inventory>` which member takes this send and forwards to it. `Pick<E>` is a plain effect — `fn choose(view: ActorGroupView<E>) -> Addr<E>?` — over a **view** of the group: every member the local replica knows, with `pending` and `local` beside each, and the hash of the send's `Key` argument when the protocol marks one. `None` parks the send until the answer changes, so an empty group, a full group and a group without a leader all wait rather than fail. The view is a mirror the replica keeps in the runtime, read on the sender's thread: no hop to the replica, and the replica is never in the send path.

std ships three policies. **`LeastLoaded<E>(prefer_local)`** takes the lightest queue, local members first when asked. **`Sharded<E>()`** takes the member that owns the key — `Key` on a parameter says which argument decides, and is erased from the type, so callers pass a plain value — and two sends for the same key land on the same member in order, a stronger guarantee than `any` claims. **`Elected<E>()`** takes the member on the leader's node, where the leader is whatever `Leader.leader()` answers: a Salvo election, a platform handler over a lease store, or `StaticLeader(node)` for a test. A policy of your own is a handler of `Pick<E>`.

A policy that must read the message — scatter a query to every member and merge the answers, race two members and take the first, retry on a member's death — is not a pick: it is a handler `of any E` you write for that protocol, forwarding as `RoundRobin` does and receiving through an actor of its own. `examples/cluster/` has one of each, beside a singleton behind `Elected`, shards under `Sharded`, and a node leaving the group with its members withdrawn and the election following.
