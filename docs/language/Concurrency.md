# Concurrency

## Overview

Concurrency in Salvo is built from **actors**: handlers that run on their
own, one message at a time, on a **pool** of threads. If you know Erlang
processes or Akka actors, the model is the same: an actor owns its state,
other code reaches it only by sending it messages, and because it handles one
message at a time its state needs no lock. A pool is close to a Java
executor: a set of worker threads (`pool(4)`), or exactly one (`thread()`),
and an ordinary value, so one pool can host many actors.

An actor's protocol is an `actor effect`, whose members are `send fn`s: a
send enqueues a message and answers nothing. A handler of it is ordinary
Salvo ([Effects and handlers](Effects-and-Handlers.md)), and `spawn` gives it
a mailbox:

```salvo
actor effect Counter {
    send fn bump(n: Int)
    send fn total(out: Reply<Int>) => !out
}

handler Counting() for Counter {
    mailbox { capacity: 8 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n          // no lock: activations run one at a time
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)          // `Reply` is linear: answered exactly once
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let workers = pool(2)
    let counter = spawn Counting() on workers

    counter.bump(2)            // enqueues; returns immediately
    counter.bump(3)

    let sum = waitfor answer: Reply<Int> { counter.total(answer) }
    println("${sum}")          // 5
}
```

The pieces:

- **`spawn H() on p`** starts the actor on pool `p` and answers its address,
  an `eff[E]` ([`Addr`](../../lib/std/core/actor.sv#type-addr)).
  `addr.member(args)` sends to it. A function may declare `[spawn]`, as
  `main` does, to be allowed to spawn.
- **The `mailbox`** bounds how many messages may queue
  ([`Mailbox`](../../lib/std/core/actor.sv#struct-mailbox)); a send to a full
  mailbox waits.
- **The state** is the actor's alone and its members run one at a time, so
  the serialization *is* the mutual exclusion.
- **A [`Reply<T>`](../../lib/std/core/actor.sv#type-reply)** is how a member
  answers. It is [linear](Linear-Types.md): every path must
  [`send`](../../lib/std/core/actor.sv#fn-send) to it exactly once or hand
  it on, so "every request gets one answer" is checked by the compiler.
- **`waitfor`** is the bridge for a frame that does mean to wait: it mints a
  reply, runs the block, and yields the answer. It needs no capability and no
  declaration.

The module [`std.core.actor`](../../lib/std/core/actor.sv) documents the
types and their guarantees (arrival order, when a send blocks, what happens
on failure and at shutdown, what things cost), with a supervisor as a worked
example. The scheduler underneath is [`std.runtime`](../../lib/std/runtime.sv),
written in Salvo. This page explains the model: what work there is, where it
runs, and how the same actors run across machines.

## Two kinds of work

Everything an actor program runs is one of two kinds of work:

* an **activation**: one member invocation of an actor, which is a handler
  bound with `spawn` instead of `use`. An actor's activations run one at a
  time, in the order their invocations arrived, on some worker of the
  actor's pool. That serialization *is* its mutual exclusion.
* a **task**: the body of a free `send fn`, scheduled rather than called. A
  task has no mailbox, no state and no identity. It exists because `replyto`
  may target a free `send fn`, so an ordinary synchronous function can wire
  future work and return.

## Waiting for an answer: `waitfor`

`waitfor out: Reply<T> { … }` mints a reply token, requires the block to
consume it, occupies the thread until the token is sent to, and yields what
was sent. The binder's type may be left out where it can be read off the
send the block makes: `waitfor out { counter.total(out) }` is a wait for an
`Int`, because `total` declares `out: Reply<Int>`. Where several members of
that name would make it ambiguous, the error names the written form.

A wait does not idle its thread: while it waits, the thread serves other
work of its pool (see [Where work runs](#where-work-runs)). A `waitfor` is
legal anywhere except inside a lambda, and needs no declaration on the
function that waits or on any caller.

## Continuations: `replyto`

Inside a handler, or in an ordinary function, a reply can be minted for work
that runs *later* instead of for a frame that waits. `replyto k(captures)`
makes a reply whose answer runs `k` with the captures and the answer as its
last argument. When `k` is a free `send fn`, that run is a task:

```salvo
send fn finish(label: Str, out: Reply<Str>, row: Int) => !label, !out {
    out.send("${label}=${row}")
}

fn fetch(id: Int, out: Reply<Str>) [Db] -> None => !out {
    db.query(id, replyto finish("row", out))   // wires the work, then returns
}
```

**Choosing the target.** Where several `send fn`s of the name are visible,
the mint picks among them as a call picks among overloads: by the captures it
is given, and by the answer the position wants. When two fit equally it is an
error, and `replyto finish@reports("row", out)` names the module whose
`send fn` is meant.

**Private members.** Inside an actor, `replyto` more often targets one of the
actor's **own** members, usually a private one. A `send fn` that no face
declares is a private member: nothing outside the handler can send to it,
and the only ways in are `k@self(…)` from a sibling member and
`replyto k(…)`. Its parameters are consumed like any message's, so its
clause is written out, and its trailing parameter is the answer the
continuation carries:

```salvo
handler Gathering() for Gather {
    mailbox { capacity: 16 }
    state {
        total: Int = 0
    }
    send fn scatter(word: Str, members: List<eff[Search]>, out: Reply<Int>) => !word, !members, !out {
        for m in members { m.query(copy(word), replyto partial()) }   // one continuation per member
        …
    }
    send fn partial(n: Int) { total = total + n … }              // private: no face declares it
}
```

An answer to an actor's continuation joins the actor's queue like any
message. A **gated** mint, `replyto! k(…)`, closes the actor's queue to
everything but that answer until it arrives (see
[`std.core.actor`](../../lib/std/core/actor.sv)).

**A mint schedules nothing.** `replyto finish(…)` allocates a token and
decides *where* the continuation will run; the body runs only when someone
sends to that token. Sending to it queues the task on that pool, and a worker
of that pool runs it.

**A `replyto` can be written after the call it answers.** The `Reply<T>` a
send takes is its last argument, so `db.query(id, replyto finish("row", out))`
nests the mint inside the call. The same thing reads flatter as a trailing
operator, where `replyto` after the call fills that final argument:

```salvo
db.query(id)
    replyto finish("row", out)
```

is exactly `db.query(id, replyto finish("row", out))`. A chain nests
rightward, each further `replyto` filling the previous mint's own answer
slot, so `a(x) replyto b(y) replyto c(z)` is
`a(x, replyto b(y, replyto c(z)))`: the shape where one send's answer is
itself computed by another. It is a flattening operator, so a `replyto` on
its own line brackets the call before it; written on one line it reads left
to right. The spelling is pure sugar: it is the same mint, filling the same
parameter.

**Placement is inherited unless you write it.** `on POOL` is optional at a
mint, and omitted means the pool current where the mint was written: inside
an actor's member, the actor's own pool; in `main`, main's own pool. Whoever
creates work pays for it, so a client cannot spend a shared service's threads
by accident, and an actor's continuations stay on the threads its author
budgeted. Write `on p` when the work belongs somewhere else.

## Where work runs

**One serving rule, and one ordering.** A worker serves its own pool. It
takes a queued task **before** a pending activation, and each queue is served
in arrival order. Both backends do exactly this, so an interleaving does not
depend on which one you compiled for.

That leaves one question: *which* thread serves the pool, and what else that
thread might be doing. The cases:

* **A pool with several workers.** The task runs as soon as any worker is
  free. Nothing more to know.
* **A one-thread pool whose thread is idle.** The task runs immediately.
* **A one-thread pool whose thread is inside a blocked activation.** A
  `waitfor` **serves its own pool while it waits**, so the task still runs,
  nested on that thread, while the waiting actor's mailbox stays stalled.
  This is not an optimisation: it is what makes the inherit-by-default rule
  safe. An actor that mints a task on its own pool and then waits for that
  task's answer would deadlock against itself if a wait merely blocked.
* **The main pool.** `main` is the single worker of a pool of its own, and it
  never gets a thread besides its own, so main-pool work runs **only while
  `main` waits**, on main's own thread, nested inside the `waitfor`. This is
  what makes `main` and an actor indistinguishable to a function that mints:
  a mint from `main` has somewhere to land.

**Work for `main` needs `main` to wait.** If the answer arrives after
`main`'s last `waitfor`, the task never runs. It is not lost work that will
be picked up later: nothing else serves that pool, and everything still
queued dies when `main` returns. From `main`, a task is *reached* by a
subsequent wait, so "wire it and forget it" is the one shape that quietly
does nothing. Place the work `on` a pool with workers of its own if it must
proceed regardless of what `main` does next. (The program ends when `main`
returns, whatever else is still running; see Shutdown in
[`std.core.actor`](../../lib/std/core/actor.sv).)

**What a wait serves** is worth being precise about, because it is
asymmetric:

* An **actor's** wait serves its pool's tasks and other actors' activations,
  but never its own: re-entering an actor mid-activation is exactly what
  serialization exists to prevent. On a dedicated thread
  ([`thread()`](../../lib/std/core/actor.sv#fn-thread)), which by linearity
  has exactly one occupant, that means it serves tasks only.
* A **task's** wait excludes nothing, because a task belongs to no actor. It
  cannot re-enter a running actor regardless: an actor with an activation in
  progress has nothing deliverable.
* Blocking is just the degenerate case of an empty queue. There is no
  separate "blocking" and "pumping" semantics to reason about.

## When work cannot finish

* **A token that is never sent to.** The task never runs. You cannot get
  there by forgetting, because a `Reply<T>` is linear: whoever holds it must
  send to it or pass it on. What can still happen is that its holder *dies*
  first (a faulted actor loses what it owed), and then the runtime's idle
  report names the waiter that can no longer be answered instead of hanging.
* **A task that faults.** It has no identity, so there is nothing to
  [`watch`](../../lib/std/core/actor.sv#fn-watch). The fault goes to its
  pool's **fault sink** if the pool was given one
  ([`pool(4, sink)`](../../lib/std/core/actor.sv#fn-pool), where `sink` is
  the addr of an actor serving
  [`Faults`](../../lib/std/core/actor.sv#effect-faults)), and otherwise is
  named on stderr. Either way the program carries on: a task's death is not
  the program's.

An actor that faults dies, and whoever watches it is sent an
[`Exit`](../../lib/std/core/actor.sv#struct-exit); supervision is written in
Salvo on top of `watch` (the Failure section of
[`std.core.actor`](../../lib/std/core/actor.sv) has an example).

## Knowing when the work is done: `on_idle`

A program can ask to be told when the work is finished:
[`on_idle(p, notify)`](../../lib/std/core/actor.sv#fn-on_idle) registers a
one-shot for the moment nothing anywhere can run, and answers an
[`Idle`](../../lib/std/core/actor.sv#struct-idle) saying what pool `p` is
still owed:

- `parked_gates`, the actors placed there whose mailbox is gated on a reply;
- `parked_tokens`, the reply tokens aimed at work there that nobody has
  discharged.

Both zero means the program is *finished*, not merely quiet.

```salvo
let p = pool(2)
counter.bump(2)                                  // … place work on p …
let settled = waitfor i: Reply<Idle> { on_idle(p, i) }
println("gates ${settled.parked_gates}, tokens ${settled.parked_tokens}")
```

- **The token is consumed by the registration**, and minted like any other,
  so a hook you forget to register is the ordinary linearity error rather
  than a request that quietly never answers.
- **The answer is edge-triggered and one-shot**, because delivering it is
  itself work and ends the idleness that produced it: hearing about the next
  one means registering again.
- **It holds only while nothing outside the scheduler injects work.** A
  platform handler with a thread of its own can make "idle" stale.
- **It is near the runtime's stuck-detection, not the same.** The hook reads
  the stricter condition: it does not fire while any frame is parked in a
  wait, where the stuck report counts a parked frame out of the running ones,
  since a parked frame cannot get anywhere on its own.

## What a task may do

A task body is ordinary Salvo, with one restriction: it declares no effects.
A task runs detached from the frame that minted it (that frame may have
returned by the time it runs), so there is no scope left to supply its
handlers from. Reaching an actor needs no effect declaration, so the way to
give a task a capability is to hand it an `eff[E]` handle as a capture and
send to it; anything else belongs in the function that mints. A task may wait
(`waitfor` needs no declaration anywhere), and a wait serves the pool it runs
on.

## Across machines

Everything above runs in one process. The same actors run across a network,
and the design has one line: **the network enters at the actor group, never
at the spawn.** Every actor is spawned by the node that hosts it, on that
node's own pools. What crosses the wire is addresses and messages, never
construction, and a function declaring `[E]` never learns whether `E` is one
local handler, an actor on another machine, or a group of a hundred across a
fleet.

The layers, bottom up:

1. a transport that moves bytes;
2. a wire format the compiler owns;
3. addresses that route;
4. a group of nodes;
5. a group of actors across those nodes;
6. a way to bind such a group where a program expects one effect.

Each layer is ordinary Salvo in module [`std.net`](../../lib/std/net.sv),
whose docs have a complete server and client, and each runs unchanged over
an in-memory transport, which is how a two-node program runs in one process
and prints the same thing on both backends.

### The wire

At the bottom is module `net`'s transport: bytes between two **endpoints**
([`NodeEndpoint`](../../lib/std/net.sv#struct-nodeendpoint)).

```salvo
import std.net

handler Receiving() [Console] for Inbound {           // frames arrive as messages
    mailbox { capacity: 16 }
    send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame {
        println("${to_str(from)}: ${str_of_bytes(frame) ?: "?"}")
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let me = NodeEndpoint { host: "127.0.0.1", port: 7000 }
    use HostTcpTransport(copy(me))                    // the wire: a host class
    listen(copy(me), spawn Receiving() on pool(1))    // receive here
    deliver(copy(me), to_bytes("hello"))              // Ok: accepted by the wire
}
```

The two halves are deliberately asymmetric.

- **Outbound** is a plain effect,
  [`Transport`](../../lib/std/net.sv#effect-transport): `deliver(to, frame)`
  is a call on the sender's thread that answers whether the frame was handed
  to the wire. `Ok` means accepted, never that the far side has it.
- **Inbound** is an *actor* effect,
  [`Inbound`](../../lib/std/net.sv#effect-inbound): bytes arrive on a thread
  the host owns, and the only way work from a foreign thread enters the
  scheduler is a send. So a node that wants to receive spawns an actor
  serving `Inbound` and registers its addr with `listen`, and the transport
  sends every arriving frame to it.

Delivery is at most once and in order per pair of endpoints, and nothing
more; anything stronger is a protocol over this.

The transport is the one piece of the network the platform owns.
[`HostTcpTransport`](../../lib/std/net.sv#handler-hosttcptransport) is the
machine's, a **`threadsafe` platform handler** ([Backends](Backends.md)): it
is reached from every pool at once, so the host synchronizes itself and the
compiler shares it with no lock. Its listener also tells the scheduler that
an outside source of work is open, so a program waiting for a frame is
neither idle nor deadlocked while the wire may still deliver one.

[`MemTransport`](../../lib/std/net.sv#handler-memtransport) is the in-memory
transport: one [`MemNetwork`](../../lib/std/net.sv#handler-memnetwork) actor
that many virtual nodes share (with `partition`, `heal` and `kill` as its
members, so a test scripts the faults) and one `MemTransport(me, net)` per
node. Everything above the wire runs on it exactly as on TCP, in one process.

### The wire format, and `noremote`

What crosses the wire is bytes in one **canonical encoding** the compiler
owns, produced and read identically by both backends, so a Kotlin node and a
Rust node can be members of one group. Every type has a wire form **by
default**: scalars, strings, `Bytes`, lists, tuples, optionals, unions (one
tag byte holding the arm's declared index) and structs (fields in
declaration order). The opt-out is a word on the declaration:

```salvo
noremote struct Canvas { surface: GpuSurface }    // process-local: no wire form
struct Frame { title: Str, canvas: Canvas }       // therefore none either, transitively
```

- [`encode(value)`](../../lib/std/codec.sv#fn-encode) answers the bytes and
  [`decode<Point>(data)`](../../lib/std/codec.sv#fn-decode) reads them back
  (both from `import std.codec`), `None` when the bytes are not a well-formed
  `Point`. Both are refused at the call for a type with no wire form, naming
  the field or declaration that stops it.
- std's process-local handles, `Pool` and the file streams, are `noremote` by
  declaration; a function value and a `proj` view are, by construction.
- Keyed containers (`Set`, `Map`) have no wire form yet: send a `List` of the
  entries.

**Protocol hashes.** Every actor protocol whose payloads all have a wire form
also has a **canonical hash**, over its members' shape, with structs expanded
to their fields, so a renamed struct is the same protocol and a reordered
field is not. Two nodes may talk on a protocol exactly when their hashes
agree, and that is settled when the nodes meet, never at decode: an
exhaustive `when` has no arm for a message it has never heard of, so the
language cannot decode leniently and refuses early instead.

### Addresses across machines

An `eff[E]` that crosses the wire arrives as a **proxy**: the same type, the
same `send`, and a mailbox on another node behind it. Nothing in the code
that holds it changes; locality is a fact the runtime reads at the send. A
`Reply<T>` crosses the same way, and the answer comes back over the wire into
whatever was waiting: `main`'s `waitfor`, an actor's parked continuation, a
task.

What keeps it honest across a machine is what keeps it honest across a
thread:

- **Credits.** The mailbox's `capacity` is enforced by credits: a proxy sends
  only when the far mailbox has granted room, blocks at zero exactly as a
  local send blocks on a full queue, and is granted a credit back as each
  message is dequeued.
- **Unguessable bits.** An address carries bits minted at the spawn and
  checked on delivery, so an addr is a capability on the wire as it is in
  memory: the administrative face of a two-face handler cannot be reached by
  whoever holds only the public one.

A process is one node; it may also **host virtual nodes**
([`new_node()`](../../lib/std/net.sv#fn-new_node),
[`pool_at(node, n)`](../../lib/std/net.sv#fn-pool_at)), which is how a
program runs two nodes in one process over `MemTransport` and exercises every
remote path (proxies, credits, frames, remote replies) with no socket.

### Node groups

Which nodes exist is a **membership** question, and the answer is an actor
effect, [`NodeGroup`](../../lib/std/net.sv#effect-nodegroup), whose handlers
are the mechanisms. A node binds a transport and spawns a mechanism on its
own pool:
[`StaticNodeGroup("demo", all)`](../../lib/std/net.sv#handler-staticnodegroup)
when the fleet is fixed,
[`GossipNodeGroup("mesh", seeds)`](../../lib/std/net.sv#handler-gossipnodegroup)
when it is not. It then asks the group `members(out)`, or `subscribe(w)` to
hear `joined` and `left` as messages. `w` is the address of an actor serving
[`NodeGroupWatcher`](../../lib/std/net.sv#effect-nodegroupwatcher), the
*listener's* protocol: the group is the publisher, and whoever wants to hear
implements the watcher (an actor group does, to withdraw the members of a
node that left).

```salvo
use HostTcpTransport(copy(me))
let group = spawn GossipNodeGroup("mesh", seeds) on p       // starts itself in its `init`, connecting the node
group.subscribe(spawn Announcing() on p)                    // "+ b:1", "- b:1 (left)"
let peers = waitfor out: Reply<List<Node>> { group.members(out) }
```

**Connecting.** The mechanism **connects** the node as it starts:
[`connect(me)`](../../lib/std/net.sv#fn-connect) binds the scheduler to the
transport in scope (the actor that puts the runtime's frames on the wire, the
actor that hands arriving frames back, and the route to the node's own
endpoint), and the mechanism calls it, at the transport's own endpoint, if
nothing has. `connect` is also a program's call, to bring the wire up before
any group exists, or to place the two wire actors on a pool of its choosing.
It is idempotent: [`connected()`](../../lib/std/net.sv#fn-connected) says
whether the node is on the wire, and a second `connect` answers `false` and
binds nothing.

**The handshake** underneath is std's, written once in `net` and common to
every mechanism. It carries the group's name (so two deployments on one
network refuse each other by name), the node's endpoint, and its protocol
table, which is what lets a node refuse to talk to a peer whose version of a
protocol differs before a single message is decoded. The runtime only
carries it, as control frames that arrive at the mechanism as a private
member of its own, registered in its `init` with
`watch_control("", self@NodeGroup)`.

A watcher that subscribes to a node group hears first of every node already
known, then of every arrival and departure. A node that calls `leave()` on
its node group tells every peer; a node that stops without leaving is not
noticed yet (see Nodes leaving in [`std.net`](../../lib/std/net.sv)).

### Actor groups

Nodes are where actors live; the set of actors a program spreads across them
is an **actor group**,
[`ActorGroup<E>`](../../lib/std/net.sv#effect-actorgroup), an actor effect
over the protocol `E`.
[`actor_group<Ping>(nodes)`](../../lib/std/net.sv#fn-actor_group) starts one
on the current node and publishes it by name, so a node that opens the same
protocol to the same node group gets a replica of the same group. Replicas
exchange their members as nodes meet, admit each remote member once, and
withdraw every member hosted on a node that leaves.

- [`join(group, addr)`](../../lib/std/net.sv#fn-join) adds a member;
- `members(out)` answers the union as seen from here;
- `subscribe(w)` delivers
  [`ActorGroupWatcher<Ping>`](../../lib/std/net.sv#effect-actorgroupwatcher)`.joined`
  and `left` as ordinary messages, each carrying an `eff[Ping]` that works
  wherever it arrives.

```salvo
let pings = actor_group<Ping>(nodes)             // one replica per node
spawn Pinging("a") on p in pings
pings.subscribe(spawn Noticing("a") on p)               // "a: + a Ping (local: false)"
```

`spawn Pinging("a") on p in pings` is a local spawn with one more message:
`join(pings, addr)`. The actor runs here, on `p` (there is no remote spawn),
and `in` says where it can be found; with a handler of several faces, the
face the group is of is the one that joins. `join(group, addr)` is the same
thing written after the fact.

**Where a protocol is refused.** `actor_group<E>(…)` is where a protocol that
cannot cross a node boundary is refused: if any `send fn` of `E` carries a
`noremote` payload, a group of `E` cannot span nodes, and the compiler says
so at that call rather than at the first remote send. The type parameter of
`ActorGroup<E>` is only ever an effect, so it costs nothing at runtime:
`eff[E]` is one handle whatever `E` is, and a generic that is only ever an
effect is erased from the generated code.

**Why `actor_group` is an ordinary function.** That erasure is also why
`actor_group` is an ordinary function and not a special one. Its body spawns
the replica and hands it the protocol's name and hash, but a body generic
over `E` cannot know either. So the signature asks for them as an
**implicit**,
`fn actor_group<E>(nodes: eff[NodeGroup], ?protocol: () -> Protocol<E>)`,
and the call fills it, exactly as `sort(xs)` fills its `?cmp` (see
[Functions](Functions.md#implicit-parameters)). The function that fills it is
std's [`protocol<E>()`](../../lib/std/net.sv#fn-protocol), the one intrinsic
whose meaning depends on the type argument it is written with, and a program
never writes it: `actor_group<Ping>(nodes)` is the whole spelling. A function
of your own that is generic over a protocol takes the same `?protocol` and
forwards it, and a builder that opens groups in some other way is a plain
function over a `Protocol<E>` value.

Three small readers go with a group:
[`pending(addr)`](../../lib/std/net.sv#fn-pending) answers how much a member
has in front of it (its mailbox depth if it is local, the in-flight count if
it is a proxy); [`node_of(addr)`](../../lib/std/net.sv#fn-node_of) names the
host; and two `eff[E]` compare with
[`eq`](../../lib/std/core/actor.sv#fn-eq).

### One member of many: `any`

A function written against one actor (`bump(2); bump(3); total(out)`) relies
on its sends arriving in order at one instance. A group cannot promise that,
so a function that is happy with *any* member says so in its effect list, and
a handler that forwards to many members says so in its `for` clause:

```salvo
fn thumbnail(img: Image) [any Resizer] -> Image { … }    // any Resizer will do: one send, one reply
fn tally(items: List<Int>) [Resizer] -> Int { … }         // needs ONE Resizer, in order

handler RoundRobin(members: List<eff[Resizer]>) for any Resizer { … }   // forwards; promises no order
```

Bare `[E]` keeps the strong meaning, so binding a group where `[Resizer]` is
required is an error at the call, and a program written before groups
existed cannot be broken by binding one under it. `[any E]` accepts every
binding (a single instance is a group of one) and is viral downward. The full
rule is in [Effects and
handlers](Effects-and-Handlers.md#many-instances-any-e-and-for-any-e).

### Routing: `use any group` and `RouteSelector<E>`

A program that wants *a* member rather than the set binds the group as the
effect itself. `use any stock` binds `any Inventory`: each send goes to
whichever member a **selector** chooses, and the selector is an ordinary
handler bound just before it:

```salvo
actor effect Inventory {
    send fn reserve(sku: Key Str, qty: Int, out: Reply<Str>) => !sku, !out
}

fn checkout(skus: List<Str>) [any Inventory, Console] -> None { … }   // `any`: shards are unordered

let stock = actor_group<Inventory>(nodes)
spawn Stocking("s1") on p in stock
use Sharded<Inventory>()          // the selector: the member that owns the key…
use any stock                     // …and the route, binding `any Inventory`
checkout(List("apple", "pear", "apple"))   // "apple" lands on the same shard both times
```

The route is a handler the compiler derives for you, once per protocol: one
`for any Inventory` whose every member asks
[`RouteSelector<Inventory>`](../../lib/std/net.sv#effect-routeselector)
which member takes this send and forwards to it. A selector works in two
phases over a **view** of the group
([`RouteView`](../../lib/std/net.sv#struct-routeview)): every member the
local replica knows, with `local` beside each, in the same order on every
node.

```salvo
effect RouteSelector<E> {
    fn changed(view: RouteView<E>) -> None          // the view moved: derive what you choose by
    fn select(view: RouteView<E>, key: Long?) -> eff[E]?   // per send
}
```

- **`changed`** runs when the view has moved since the selector last saw it
  (a member joined or left, or the group was **refreshed**), and the
  selector keeps what it derives in its own state: a probability per member,
  a leader, a ring.
- **`select`** runs per send, with the same view and the hash of the send's
  `Key` argument when the protocol marks one. A selector that wants load
  reads it then, with `pending(addr)`, since load changes with every send.
- **`None` makes the send wait**, so an empty group and a group without a
  leader wait rather than fail: it waits 1 ms, doubling up to 5 s, and wakes
  at once whenever the view moves. The waits are the selector's: std's
  selectors take an optional `config`
  (`use Sharded<Inventory>(config = RouteConfig { first_wait: …, max_wait: … })`,
  see [`RouteConfig`](../../lib/std/net.sv#struct-routeconfig)), and a
  selector of your own answers `config()`.

The view is a mirror the replica keeps in the runtime, read on the sender's
thread: no hop to the replica, and the replica is never in the send path.

Something a selector reads outside the view, such as an election's answer,
reaches it the same way: whoever decides calls `refresh()` on the group when
the answer changes, the view's version moves, and every route over it runs
`changed` again. So a membership change and a leader change are one signal.

std ships three selectors:

- **[`LeastLoaded<E>(prefer_local)`](../../lib/std/net.sv#handler-leastloaded)**
  takes the lightest queue, local members first when asked.
- **[`Sharded<E>()`](../../lib/std/net.sv#handler-sharded)** takes the member
  that owns the key. [`Key`](../../lib/std/net.sv#qualifier-key) on a
  parameter says which argument decides, and is erased from the type, so
  callers pass a plain value. Two sends for the same key land on the same
  member in order, a stronger guarantee than `any` claims.
- **[`Elected<E>()`](../../lib/std/net.sv#handler-elected)** takes the member
  on the leader's node, asking
  [`Leader`](../../lib/std/net.sv#effect-leader)`.leader()` when the view
  changes: a Salvo election, a platform handler over a lease store, or
  [`StaticLeader(node)`](../../lib/std/net.sv#handler-staticleader) for a
  test.

A selector of your own is a handler of `RouteSelector<E>`.

A policy that must read the message (scatter a query to every member and
merge the answers, race two members and take the first, retry on a member's
death) is not a selector: it is a handler `for any E` you write for that
protocol, forwarding as `RoundRobin` does and receiving through an actor of
its own. `examples/cluster/` has one of each, beside a singleton behind
`Elected`, shards under `Sharded`, and a node leaving the group with its
members withdrawn and the election following.
