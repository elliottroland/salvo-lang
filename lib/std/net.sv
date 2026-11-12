// Actors across machines: a program's actors run on several **nodes**
// (processes, or virtual nodes inside one process) and reach each other by
// address, with the same `send`, `waitfor` and `Reply` they use in one
// process.
//
// The network enters at the **actor group**, never at the spawn. Every actor
// is spawned by the node that hosts it, on that node's own pools; what
// crosses the wire is addresses and messages, never construction. A function
// that declares a bare `[E]` cannot tell whether its sends go to a local actor,
// to an actor on another machine, or to one of a hundred spread over a
// fleet. A program touches four layers, bottom up:
//
// - **A transport** moves bytes between [NodeEndpoint]s. [HostTcpTransport]
//   is the real one, over TCP; [MemTransport] over a [MemNetwork] runs many
//   nodes in one process, with the faults a test wants to script.
// - **A node group** ([NodeGroup]) is the membership of machines.
//   [StaticNodeGroup] takes a fixed list of endpoints, [GossipNodeGroup] a
//   few seeds. Spawning one connects the node to the wire and introduces it
//   to its peers.
// - **An actor group** ([ActorGroup]) is the set of actors of one protocol
//   across those nodes. [actor_group] opens it on this node, and
//   `spawn H() on p in group` adds a member.
// - **A router** binds a group where a function expects one actor: a
//   handler of the protocol that sends each message to one member —
//   [LeastLoaded], [Sharded] by a key (`std.net.shard`), or [Elected]
//   behind a [Leader].
//
// A server and a client, each the `main` of its own process. The protocol
// is an ordinary actor effect, and `hello` is written against it alone:
//
// ```
// import std.time
//
// actor effect Greeter {
//     send fn greet(name: Str, out: Reply<Str>) => !name, !out
// }
//
// handler Greeting(who: Str) for Ordered Greeter {
//     mailbox { capacity: 16 }
//     send fn greet(name: Str, out: Reply<Str>) => !name, !out {
//         out.send("hello ${name}, from ${who}")
//     }
// }
//
// // Any greeter will do: nothing here names a node.
// fn hello(name: Str) [Greeter] -> Str => !name {
//     return waitfor out: Reply<Str> { greet(name, out) }
// }
//
// fn fleet() [] -> List<NodeEndpoint> {
//     return List(NodeEndpoint { host: "10.0.0.1", port: 7000 }, NodeEndpoint { host: "10.0.0.2", port: 7000 })
// }
//
// // The server joins the node group, hosts one greeter and serves for an
// // hour: the program ends when `main` returns.
// fn server_main() [use, spawn] {
//     let me = NodeEndpoint { host: "10.0.0.1", port: 7000 }
//     use HostTcpTransport(copy(me))
//     let p = pool(2)
//     let nodes = spawn StaticNodeGroup("demo", fleet()) on p
//     let greeters = actor_group<Greeter>(nodes)
//     spawn Greeting(to_str(me)) on p in greeters
//     let timer = spawn DefaultTimer() on pool(1)
//     let _done = waitfor f: Reply<Fired> { timer.after(hours(1), f) }
// }
//
// // The client joins the same node group and opens the same actor group,
// // where the server's greeter appears. Its first send waits until it has.
// fn client_main() [use, spawn] {
//     use StdConsole()
//     use HostTcpTransport(NodeEndpoint { host: "10.0.0.2", port: 7000 })
//     let nodes = spawn StaticNodeGroup("demo", fleet()) on pool(1)
//     let greeters = actor_group<Greeter>(nodes)
//     use LeastLoaded(greeters, true)
//     println(hello("ann"))                   // hello ann, from 10.0.0.1:7000
// }
// ```
//
// Either process may start first: each node group greets every listed
// endpoint as it starts, and whichever node comes up second completes the
// handshake.
//
// **What can cross.** A message crosses the wire in the canonical encoding
// of [std.codec], which both backends share, so a Kotlin node and a Rust node
// can be peers. Every type has a wire form unless it is declared `noremote`
// or holds something that is. [actor_group] refuses, at compile time, a
// protocol whose messages carry such a payload. Two nodes compare a hash of
// each protocol when they meet, and a peer whose version of a protocol
// differs never sees this node's members of it.
//
// **Addresses across machines.** An `eff[E]` that crosses the wire arrives
// as a **proxy**: the same type and the same `send`, with a mailbox on
// another node behind it. A `Reply` crosses the same way, and its answer
// comes back over the wire. A proxy honours its mailbox's `capacity`: a send
// blocks when the far mailbox is full, as a local send does. An address
// carries unguessable bits checked on delivery, so holding an address is
// still what entitles you to send to it.
//
// **Delivery.** Frames between two nodes arrive at most once and in order,
// and nothing more: a frame the wire loses is lost, and only a failure the
// transport can see is reported (a [NetError] from `deliver`). A program
// that needs more builds it as a protocol of its own.
//
// **Nodes leaving.** A node that calls `leave()` on its node group tells
// every peer. The peers drop it from the node group, withdraw its members
// from every actor group, and kill their proxies of its actors: a send to
// one does nothing, and a `watch` on one fires. A node that stops without
// leaving is not noticed yet; detecting it is future work.
//
// **One process, many nodes.** [new_node] hosts a virtual node in this
// process and [pool_at] gives it pools; an address of an actor spawned
// there reaches other nodes as a proxy, as from another machine. Over
// [MemTransport], a whole cluster
// runs in one process with no socket, which is how its tests and
// `examples/cluster` run. An actor test (see the Testing topic) must use
// [MemTransport]: a host transport's threads are outside its virtual clock.
//
// The Concurrency topic, section "Across machines", tells the same story at
// more length.
// @compilerrules [net-transport, net-host, net-mem, net-wire-format, net-noremote, actor-protocol-hash, net-addr-routable, net-addr-capability, net-remote-backpressure, net-connect, node-group, actor-group, effect-claim, actor-router, time-timer, platform-threadsafe]

import std.codec.decode
import std.codec.encode
import std.runtime.addr_index
import std.runtime.addr_of
import std.runtime.pool_of
import std.runtime.routing
import std.time.Duration

// An address a transport can reach: a host and a port.
//
// An endpoint is known before any contact, which is why node groups are
// configured with them. For [HostTcpTransport] it is a TCP host name or IP
// address and a port; for [MemTransport] it is any name you choose for a
// virtual node. It says nothing about who answers there: that is a [Node],
// learnt when two nodes meet. Endpoints compare, hash and order by their
// fields, so they key maps and sort.
//
// ```
// let e = NodeEndpoint { host: "10.0.0.5", port: 7000 }
// println(to_str(e))                                   // 10.0.0.5:7000
// ```
export struct NodeEndpoint : Ordered<self> by auto, Hashed<self> by auto {
    host: Str
    port: Int
}

// An endpoint as `host:port`.
export fn to_str(e: NodeEndpoint) [] -> Str => e {
    return "${e.host}:${e.port}"
}

// Nobody could be reached at the endpoint.
export struct Unreachable {
    to: NodeEndpoint
}

// The connection to the endpoint failed: [reason] is the transport's account.
export struct WireFailed {
    to: NodeEndpoint
    reason: Str
}

// Why a transport could not take a frame, or could not listen.
//
// [Unreachable] means nobody could be reached at the endpoint: over TCP, the
// connection was refused or could not be made; over [MemTransport], the
// endpoint is partitioned from this one, killed, or has no listener.
// [WireFailed] means a connection failed: over TCP, either `listen` could
// not bind its port, or a write on an open connection failed, in which case
// the connection is dropped and the next `deliver` to that endpoint
// connects afresh. Neither is thrown: a [Transport] call answers it as an
// `Err`.
//
// ```
// fn report(outcome: Ok None | Err NetError) [Console] -> None => outcome {
//     when outcome {
//         is Ok { println("sent") }
//         is Err {
//             let e: NetError = outcome
//             println(to_str(e))                       // unreachable: b:1
//         }
//     }
// }
// ```
export type NetError = Unreachable | WireFailed

// A failure as one line of text: `unreachable: b:1`, or
// `wire failed to b:1: ` and the transport's reason.
export fn to_str(e: NetError) [] -> Str => e {
    when e {
        is Unreachable {
            return "unreachable: ${to_str(e.to)}"
        }
        is WireFailed {
            return "wire failed to ${to_str(e.to)}: ${e.reason}"
        }
    }
}

// The receiving side of a transport: an actor that is sent each frame that
// arrives at an endpoint, with the endpoint it came from.
//
// Frames arrive on threads the transport owns (for TCP, one reader thread
// per connection), and a message is the only way work from such a thread
// enters the scheduler. So receiving is an **actor** effect: a node spawns
// an actor serving `Inbound` and registers its address with
// `Transport.listen`, and the transport sends it one `receive_frame` per
// frame, in the order each sender sent them. The frame is opaque `Bytes`;
// what it means is up to the actor that registered.
//
// A program rarely serves this itself: [connect] registers the runtime's
// own receiver, which routes each frame to the actor, reply or proxy it is
// for. Serving it is how you use a transport as a plain message pipe:
//
// ```
// actor effect Tally {
//     send fn seen(out: Reply<Int>) => !out
// }
//
// // Counts the frames that arrive, and answers how many on request.
// handler Tallying() for Ordered Inbound, Ordered Tally {
//     mailbox { capacity: 16 }
//     state {
//         n: Int = 0
//     }
//     send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame {
//         n = n + 1
//     }
//     send fn seen(out: Reply<Int>) => !out {
//         out.send(n)
//     }
// }
// ```
// @compilerrules [actor-effect-kind]
export actor effect Inbound {
    send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame
}

// Moves frames of bytes between endpoints: the bottom layer of the network,
// which the runtime uses to carry messages between nodes.
//
// Sending is a plain call on the caller's thread: `deliver` answers `Ok`
// when the transport has taken the frame, or a [NetError] when it could
// not. Receiving goes to an [Inbound] actor registered with `listen`.
//
// **Delivery is at most once and in order per pair of endpoints**, and
// nothing more. `Ok` means the transport accepted the frame (for TCP, that
// it was written to a connected socket), not that the far side has it. A
// frame lost after that is lost silently. Anything stronger is a protocol
// over this one.
//
// std has two handlers: [HostTcpTransport], over TCP, and [MemTransport],
// over a [MemNetwork] in this process. A node binds one with `use`, and
// everything that needs the wire (its node group, [connect]) finds it there.
// Over [MemTransport], with a test's faults:
//
// ```
// fn main() [use, spawn] {
//     use StdConsole()
//     let a = NodeEndpoint { host: "a", port: 1 }
//     let b = NodeEndpoint { host: "b", port: 1 }
//     let net = spawn MemNetwork() on pool(1)
//     use MemTransport(copy(a), copy(net))
//     let sink = spawn Printing() on pool(1)
//     let _l = listen(copy(b), sink)            // b's listener (node b's job, normally)
//     let sent = deliver(copy(b), to_bytes("one"))
//     println("${sent is Ok}")                  // true
//     net.partition(copy(a), copy(b))
//     let lost = deliver(copy(b), to_bytes("two"))
//     println("${lost is Ok}")                  // false: unreachable
// }
//
// handler Printing() for Ordered Inbound {
//     mailbox { capacity: 16 }
//     send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame {}
// }
// ```
export effect Transport {
    // Starts receiving at [at]: every frame that arrives there is sent to
    // [sink]. A second `listen` at the same endpoint replaces the sink. Over
    // TCP, `Err` when the port cannot be bound; and while a TCP listener is
    // open the program is not considered idle, since a frame may arrive.
    fn listen(at: NodeEndpoint, sink: eff[Inbound]) -> Ok None | Err NetError => at, !sink
    // Stops receiving at [at]. Frames still on their way there are lost.
    fn unlisten(at: NodeEndpoint) -> None => at
    // Hands [frame] to the wire for [to]: `Ok` when the transport took it.
    // Over TCP the first frame to an endpoint opens the connection, and the
    // caller's thread waits while it connects.
    fn deliver(to: NodeEndpoint, frame: Bytes) -> Ok None | Err NetError => to, !frame
    // The endpoint this node's frames carry as their `from`: the one the
    // handler was made with.
    fn local_endpoint() -> NodeEndpoint
}

// -------------------------------------------------------------- routing ----

// The identity of a node: a process, or a virtual node a process hosts.
//
// The runtime gives each process a random identity when it starts, and
// [new_node] makes more. Every actor carries the node it was spawned on
// ([node_of]), an address that crosses the wire carries it, and a [Node]
// in a node group pairs it with the endpoint it answers at. Two ids compare
// equal exactly when they name the same node.
// @compilerrules [net-addr-routable]
export struct NodeId : Hashed<self> by auto {
    value: Long
}

// The node this code runs on: the node of the pool the current actor (or
// `main`) runs on. Compare it with [node_of] to tell whether an actor is
// local.
// @compilerrules [net-addr-routable]
export fn this_node() [] -> NodeId {
    return NodeId { value: here_node() }
}

// Hosts a fresh **virtual node** in this process, and answers its id.
//
// A virtual node stands in for another machine. Pools made for it with
// [pool_at] belong to it, and actors spawned on those pools carry it, so an
// address of one of them that crosses the wire to another node arrives as a
// proxy, and its messages travel through the transport as frames. Together
// with [MemTransport], this runs a whole cluster in one process. A virtual
// node is on the wire once something running on it calls [connect] or
// spawns a node group, with that node's own transport in scope.
//
// ```
// fn main() [use, spawn] {
//     use StdConsole()
//     let other = new_node()
//     let there = spawn Counting() on pool_at(other, 1)
//     println("${eq(node_of(there), other)}")          // true
//     println("${eq(node_of(there), this_node())}")    // false
// }
//
// actor effect Counter {
//     send fn bump(n: Int)
// }
//
// handler Counting() for Ordered Counter {
//     mailbox { capacity: 8 }
//     send fn bump(n: Int) {}
// }
// ```
// @compilerrules [net-addr-routable]
export fn new_node() [spawn] -> NodeId {
    return NodeId { value: new_node@runtime.routing() }
}

// A pool of [size] worker threads belonging to [node]: what runs that
// node's actors. `this_node()` inside them answers [node].
// @compilerrules [net-addr-routable]
export fn pool_at(node: NodeId, size: Int) [spawn] -> Pool => node, size {
    return pool_of(pool_at(node.value, size))
}

// ---- the runtime's side of the wire (private: `connect` is the program's call)
//
// The scheduler does not know how bytes move. It keeps a **routing table**
// (node → endpoint), and when it has a frame for another node — a message to
// a proxy, a reply, a credit, a handshake — it looks the node up and hands
// (endpoint, frame) to whatever the node bound as its **outbound**. When the
// transport delivers a frame that arrived, the **inbound** side hands the
// bytes back and the runtime finds the actor, waiter or proxy they are for.
// These three intrinsics are those bindings; `connect` makes all of them.

// Records where frames for [node] go. The self-route is
// written by `connect`; routes to peers are learnt in the node group's
// handshake. A frame for a node with no route yet waits until one arrives.
// @compilerrules [net-addr-routable]
fn add_route(node: NodeId, at: NodeEndpoint) [] -> None => node, at {
    route(node.value, encode(copy(at)))
}

// Binds the current node's outbound side: every frame the
// runtime wants sent is queued to [out], whose activation calls
// `Transport.deliver` — with the scheduler's lock released, which is why it
// is an actor and not a direct call.
// @compilerrules [net-addr-routable]
fn route_frames(out: eff[Outbound]) [] -> None => !out {
    bind_outbound(here_node(), addr_index(out), forward_frame)
    outbound_bound()
}

// The outbound hook: called by the host,
// on whichever thread has a frame for the wire, with the `Outbound` actor's
// index, the endpoint's canonical bytes and the frame. An endpoint that does
// not decode is dropped, never sent wrong.
// @compilerrules [net-addr-routable, platform-fn-value]
fn forward_frame(out: Int, to: Bytes, frame: Bytes) [] -> None => out, to, !frame {
    if decode<NodeEndpoint>(to) is NodeEndpoint ep {
        let sending: eff[Outbound] = addr_of(out)
        sending.send_frame(ep, frame)
    }
}

// Records [hook] as node [node]'s outbound hook, called
// with [out]: the host keeps the named fn and calls it for each frame.
// @compilerrules [platform-fn-value]
platform fn bind_outbound(node: Long, out: Int, hook: (out: Int, to: Bytes, frame: Bytes) -> None) [] -> None
=> node, out, !hook =>[hook] !frame

// Hands one frame that arrived on the wire to the runtime:
// a message into an actor's mailbox, a reply to its waiter, a credit to a
// proxy, a handshake to the node group. Answers whether it was delivered; a
// frame for an unknown target, with mismatched bits or a malformed payload is
// dropped, never delivered wrong.
// @compilerrules [net-addr-routable]
fn deliver_frame(data: Bytes) [] -> Bool => data {
    return deliver(data)
}

// How many more messages a proxy may send before it waits for room in the
// far mailbox; `None` for an actor on this node.
//
// A proxy is the address of an actor on another node. A mailbox's
// `capacity` holds across the wire through **credits**: the far node grants
// a proxy room, each send spends a credit, and each message the far actor
// takes off its queue grants one back. A send through a proxy with no
// credits blocks until one arrives, as a send to a full local mailbox does.
// This is the number a route selector can read as a remote member's load.
//
// ```
// fn main() [use, spawn] {
//     use StdConsole()
//     let here = spawn Counting() on pool(1)
//     println("${credits(here) is None}")              // true: a local actor
// }
//
// actor effect Counter {
//     send fn bump(n: Int)
// }
//
// handler Counting() for Ordered Counter {
//     mailbox { capacity: 8 }
//     send fn bump(n: Int) {}
// }
// ```
// @compilerrules [net-remote-backpressure]
export fn credits<E>(a: eff[E]) [] -> Int? => a {
    let c = credits(addr_index(a))
    if c is None {
        return None
    }
    if c < 0 {
        return 0
    }
    return c
}

// The runtime's outgoing frames: the protocol of the actor that puts them
// on the wire.
//
// Every frame the runtime wants sent (a message to a proxy, a reply, a
// credit, a group's handshake) is sent to this actor as `send_frame`, and the
// actor calls `Transport.deliver` for it. Sending is an actor's job so that
// the transport's call, which may wait on a socket, never runs while the
// scheduler holds its lock. [connect] spawns the one handler, [Sending]; a
// program does not send to it.
// @compilerrules [net-addr-routable]
export actor effect Outbound {
    send fn send_frame(to: NodeEndpoint, frame: Bytes) => !to, !frame
}

// The runtime's outgoing side: hands each frame to the [Transport] it was
// spawned with. A frame the transport refuses is dropped, as the transport's
// at-most-once delivery allows. [connect] spawns it, with room for 256
// frames in its mailbox.
export handler Sending [Transport] for Ordered Outbound {
    mailbox { capacity: 256 }

    send fn send_frame(to: NodeEndpoint, frame: Bytes) => !to, !frame {
        // A frame the wire refuses is lost: at-most-once, as promised.
        let _sent = deliver(to, frame)
    }
}

// The runtime's incoming side: an [Inbound] actor that hands each arriving
// frame to the runtime, which finds the actor, reply or proxy it is for. A
// frame for an actor that does not exist, or with an address whose
// capability bits do not match, is dropped. [connect] spawns it and
// registers it with `Transport.listen`, with room for 256 frames in its
// mailbox.
// @compilerrules [net-addr-routable]
export handler Receiving for Ordered Inbound {
    mailbox { capacity: 256 }

    send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame {
        let _delivered = deliver_frame(frame)
    }
}

// Whether the current node is on the wire: whether [connect] has run on
// it, directly or from a node group.
// @compilerrules [net-connect]
export fn connected() [] -> Bool {
    return connected@runtime.routing()
}

// Connects the current node to the wire at endpoint [me], over the
// [Transport] in scope, and answers whether this call did it.
//
// A node must be connected before it can reach another or be reached. The
// call spawns two actors, a [Sending] that puts the runtime's frames on the
// wire and a [Receiving] that listens at [me] and hands arriving frames back,
// and records that frames for this node go to [me]. Where other nodes are
// is learnt when they meet, through a node group.
//
// A program seldom needs it: a node group connects its node as it starts, at
// the transport's own `local_endpoint`, when nothing has. Call it to bring
// the wire up before any group exists, or to choose the pool the two actors
// run on (the overload with [on]; this one uses a new `pool(1)`).
//
// It is idempotent: on a node already connected it answers `false` and
// spawns and binds nothing.
//
// ```
// fn main() [use, spawn] {
//     use StdConsole()
//     let me = NodeEndpoint { host: "a", port: 1 }
//     let net = spawn MemNetwork() on pool(1)
//     use MemTransport(copy(me), net)
//     println("${connected()}")                        // false
//     println("${connect(copy(me))}")                  // true: this call connected
//     println("${connect(copy(me))}")                  // false: already connected
//     println("${connected()}")                        // true
// }
// ```
//
// @param me  this node's endpoint: where it listens, and where its peers
//            send its frames
// @returns   `true` when this call connected the node
// @compilerrules [net-connect]
export fn connect(me: NodeEndpoint) [Transport, spawn] -> Bool => !me {
    return connect(me, pool(1))
}

// [connect] with the two wire actors on the pool [on].
export fn connect(me: NodeEndpoint, on: Pool) [Transport, spawn] -> Bool => !me, !on {
    if connected@net() {
        return false
    }
    let sending = spawn Sending() on on
    let receiving = spawn Receiving() on on
    route_frames(sending)
    let _listening = listen(copy(me), receiving)
    add_route(this_node(), me)
    return true
}

// ----------------------------------------------------------- node group ----

// A member of a node group: a node's identity, and the endpoint it answers
// at. Where a [NodeEndpoint] is known before contact, a `Node` is what two
// nodes learn of each other when they meet.
// @compilerrules [node-group]
export struct Node : Hashed<self> by auto {
    id: NodeId
    at: NodeEndpoint
}

// The membership of **machines**: which nodes are in the cluster, as this
// node knows it.
//
// A node group is an actor, one per node, spawned with the node's
// [Transport] in scope. Its handlers are the ways of finding peers:
// [StaticNodeGroup] for a fixed list of endpoints, [GossipNodeGroup] for a
// few seeds that introduce the rest. Each starts itself as it is spawned:
// it connects the node to the wire if nothing has ([connect]) and greets the
// endpoints it knows. When two nodes meet they exchange the group's name
// (nodes of differently named groups ignore each other), their endpoints,
// and a hash of every actor protocol they speak ([peer_protocol]).
//
// A node group is what [actor_group] needs to find the other nodes'
// members. Ask it `members` for the nodes it knows now, or `subscribe` an
// actor serving [NodeGroupWatcher] to hear of each node as it arrives and
// goes:
//
// ```
// handler Announcing() [Console] for Ordered NodeGroupWatcher {
//     mailbox { capacity: 32 }
//     send fn joined(n: Node) => !n {
//         println("+ ${to_str(n.at)}")
//     }
//     send fn left(n: Node, why: Str) => !n, !why {
//         println("- ${to_str(n.at)} (${why})")
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let me = NodeEndpoint { host: "10.0.0.1", port: 7000 }
//     let seeds = List(NodeEndpoint { host: "10.0.0.9", port: 7000 })
//     use HostTcpTransport(copy(me))
//     let p = pool(1)
//     let nodes = spawn GossipNodeGroup("shop", seeds) on p
//     nodes.subscribe(spawn Announcing() on p)         // + 10.0.0.9:7000, …
//     let peers = waitfor out: Reply<List<Node>> { nodes.members(out) }
//     println("${size(peers)} peers so far")
// }
// ```
//
// A node is seen to go when it sends `leave`: its peers drop it, and every
// proxy of an actor on it dies. A node that stops without leaving stays a
// member; noticing it is future work.
// @compilerrules [node-group]
export actor effect NodeGroup {
    // Answers every node this node knows of, itself excluded.
    send fn members(out: Reply<List<Node>>) => !out
    // Subscribes [w]: it first hears `joined` for every node already known,
    // then hears of every arrival and departure.
    send fn subscribe(w: eff[NodeGroupWatcher]) => !w
    // Tells every known peer this node is leaving.
    send fn leave()
}

// What a subscriber to a [NodeGroup] hears: a node joining, and a node
// leaving.
// @compilerrules [node-group]
export actor effect NodeGroupWatcher {
    // [n] has joined, or was already known when the subscriber subscribed.
    send fn joined(n: Node) => !n
    // [n] has gone; [why] is `"left"` when it announced it with `leave`.
    send fn left(n: Node, why: Str) => !n, !why
}

// ---- the handshake (private: a mechanism's tools)
//
// Two nodes meet by exchanging a [Hello] and an [Ack]: each carries the
// group's name, the sender's endpoint and its **protocol table** (every actor
// effect with a wire form, and its hash). A [Leaving] announces departure and
// kills every proxy of an actor on that node; an [Intro] carries endpoints
// one node tells another about, which is how gossip converges on a full mesh.
//
// The protocol is std's, written here once for both backends: the runtime
// carries it as **control frames** and nothing more. A control frame names a
// channel, and the actor listening on that channel at the node it reaches
// (`watch_control`) receives its payload as a private `control(from, data)`
// message: a node group's mechanism on the channel `""`, an actor group's
// replica on its group's name. The payloads are the canonical encodings of
// the types below. Control frames travel through the runtime rather than as
// an actor protocol, so a mechanism never sends the protocol it serves,
// which keeps gossip out of the deadlock graph's send-cycle warning.
// @compilerrules [actor-private-send, net-wire-format, actor-deadlock-cycle]

struct Hello {
    group: Str
    at: NodeEndpoint
    protocols: List<(Str, Str)>
}

struct Ack {
    group: Str
    at: NodeEndpoint
    protocols: List<(Str, Str)>
}

struct Leaving {}

struct Intro {
    peers: List<NodeEndpoint>
}

type Handshake = Hello | Ack | Leaving | Intro

// The current node's control frames on [channel]
// go to [sink], as the private `control(from, data)` message of the handler
// whose `init` calls it — `self@E` there. Every registration is the node's,
// so several hosted nodes in one process each hear their own.
// @compilerrules [node-group, actor-group]
intrinsic fn watch_control<E>(channel: Str, sink: eff[E]) [] -> None => channel, !sink

// Routes [payload] to the actor listening on
// [channel] at node [to]: a control frame, parked until [to] has a route.
// @compilerrules [node-group, actor-group]
fn send_control(to: NodeId, channel: Str, payload: Bytes) [] -> None => to, channel, !payload {
    send_control(to.value, copy(channel), payload)
}

// `control_frame`: a control frame on a channel for **whichever node
// receives it**: what a HELLO is, since before the handshake the peer's
// identity is not known and it has no route, so it is handed to the
// transport directly. (Now `runtime.routing`'s own, called directly.)
// @compilerrules [node-group, runtime-handles]

// Peer [node] has left: the runtime forgets its route
// and protocol table, and every proxy of an actor on it dies — a send to one
// is the silent no-op, and a `watch` on one fires.
// @compilerrules [node-group, node-exit]
fn node_left(node: NodeId) [] -> None => node {
    node_left(node.value)
}

// `local_protocols`: this program's protocol table, which every [Hello]
// and [Ack] carries. (Now `runtime.routing`'s own, called directly.)
// @compilerrules [actor-protocol-hash, runtime-handles]

// Records peer [node]'s protocol table, as its handshake
// carried it: what [peer_protocol] answers from.
// @compilerrules [actor-protocol-hash]
fn set_peer_protocols(node: NodeId, table: List<(Str, Str)>) [] -> None => node, table {
    set_peer_protocols(node.value, copy(table))
}

// A peer node's hash for the actor protocol named [protocol]; `None` for a
// node not met, or one that does not speak the protocol.
//
// The hash is the one the peer sent when the two nodes met.
//
// Every actor effect whose messages can cross the wire has a hash computed
// by the compiler from its shape: the same on both backends, and different
// once a message is added, removed or changed. Two nodes whose hashes for a
// protocol differ cannot exchange its messages, and an [ActorGroup] ignores
// the other's members. This program's own hash for a protocol `P` is
// `protocol<P>().hash`.
//
// ```
// fn speaks_greeter(n: Node) [] -> Bool => n {
//     return !(peer_protocol(n.id, "Greeter") is None)
// }
// ```
// @compilerrules [actor-protocol-hash]
export fn peer_protocol(node: NodeId, protocol: Str) [] -> Str? => node, protocol {
    return peer_protocol(node.value, protocol)
}

// The HELLO for group [group], to be handed to the transport
// directly: a node with no route yet cannot be sent to any other way.
// @compilerrules [node-group]
fn hello_frame(group: Str) [Transport] -> Bytes => group {
    let hello: Handshake = Hello { group: copy(group), at: local_endpoint(), protocols: local_protocols() }
    return control_frame("", encode(hello))
}

// What a mechanism learns from one handshake message.
// @compilerrules [node-group]
struct PeerHello {
    node: NodeId
    at: NodeEndpoint
}

struct PeerGone {
    node: NodeId
}

struct PeerIntro {
    peers: List<NodeEndpoint>
}

// The handshake, common to every mechanism: a [Hello] of the
// same group records the peer's route and protocol table and is answered with
// an [Ack], which records the same without an answer; a group of another name
// is refused by its name; a [Leaving] forgets the peer and kills its proxies.
// Answers what the mechanism should know, or `None` for a message to ignore
// (malformed, or of another group).
// @compilerrules [node-group]
fn handshake(group: Str, from: NodeId, data: Bytes) [Transport] -> PeerHello | PeerGone | PeerIntro | None
=> group, !from, !data {
    let msg = decode<Handshake>(data)
    when msg {
        is Hello {
            if !eq(msg.group, group) {
                return None
            }
            add_route(copy(from), copy(msg.at))
            set_peer_protocols(copy(from), copy(msg.protocols))
            let ack: Handshake = Ack { group: copy(group), at: local_endpoint(), protocols: local_protocols() }
            send_control(copy(from), "", encode(ack))
            return PeerHello { node: from, at: copy(msg.at) }
        }
        is Ack {
            if !eq(msg.group, group) {
                return None
            }
            add_route(copy(from), copy(msg.at))
            set_peer_protocols(copy(from), copy(msg.protocols))
            return PeerHello { node: from, at: copy(msg.at) }
        }
        is Leaving {
            node_left(copy(from))
            return PeerGone { node: from }
        }
        is Intro {
            return PeerIntro { peers: copy(msg.peers) }
        }
        is None {
            return None
        }
    }
}

// Tells peer [node] about [peers]: an [Intro].
// @compilerrules [node-group]
fn introduce(node: NodeId, peers: List<NodeEndpoint>) [] -> None => node, peers {
    let intro: Handshake = Intro { peers: copy(peers) }
    send_control(copy(node), "", encode(intro))
}

// A [Leaving] to every peer in [known].
// @compilerrules [node-group]
fn leave_group(known: Map<NodeId, Node>) [] -> None => known {
    for id in keys(known) {
        let leaving: Handshake = Leaving {}
        send_control(copy(id), "", encode(leaving))
    }
}

// A [NodeGroup] over a fixed list of endpoints: every node of the cluster is
// known up front.
//
// As it starts, it connects the node if nothing has, then greets every
// endpoint in [all] other than its own, once. A node that is not up yet is
// met later, when it starts and greets this one, so the nodes may start in
// any order.
// Every node that answers to the same [name] becomes a member. It notices a
// departure only when the node sends `leave`.
//
// ```
// fn main() [use, spawn] {
//     let me = NodeEndpoint { host: "10.0.0.1", port: 7000 }
//     let fleet = List(
//         copy(me),
//         NodeEndpoint { host: "10.0.0.2", port: 7000 },
//         NodeEndpoint { host: "10.0.0.3", port: 7000 }
//     )
//     use HostTcpTransport(copy(me))
//     let nodes = spawn StaticNodeGroup("demo", fleet) on pool(1)
//     let _pings = actor_group<Ping>(nodes)
// }
//
// actor effect Ping {
//     send fn ping(out: Reply<Str>) => !out
// }
// ```
//
// [name] is the group's name: nodes of differently named groups ignore each
// other. [all] lists every node's endpoint; this node's own may be in it,
// and is skipped.
// @compilerrules [node-group]
export handler StaticNodeGroup(name: Str, all: List<NodeEndpoint>) [Transport, spawn] for NodeGroup {
    mailbox { capacity: 64 }

    state {
        known: Mut Map<NodeId, Node> = Mut Map()
        watchers: Mut List<eff[NodeGroupWatcher]> = Mut List()
    }

    // The mechanism starts itself: `init` is the first activation, and
    // `self@NodeGroup` is where the control frames of its handshake go.
    // It connects the node if nothing has yet — the transport it
    // depends on is a handle the two wire actors inherit — at the transport's
    // own endpoint: one source of truth for "where this node is".
    // @compilerrules [net-connect]
    init {
        let me = local_endpoint()
        let _connected = connect(copy(me))
        watch_control("", self@NodeGroup)
        for e in all {
            if !eq(e, me) {
                let _sent = deliver(copy(e), hello_frame(name))
            }
        }
    }

    send fn members(out: Reply<List<Node>>) => !out {
        out.send(known_nodes(known))
    }

    // A subscriber hears first of every node already known, then of every
    // arrival and departure.
    send fn subscribe(w: eff[NodeGroupWatcher]) => !w {
        for id in keys(known) {
            let n = get(known, id)
            if !(n is None) {
                w.joined(copy(n))
            }
        }
        add(watchers, w)
    }

    send fn leave() {
        leave_group(known)
    }

    send fn control(from: NodeId, data: Bytes) => !from, !data {
        let event = handshake(name, from, data)
        when event {
            is PeerHello {
                if contains_key(known, event.node) {
                    return
                }
                let n = Node { id: copy(event.node), at: copy(event.at) }
                put(known, copy(event.node), copy(n))
                for w in watchers {
                    w.joined(copy(n))
                }
            }
            is PeerGone {
                let n = remove(known, event.node)
                if n is None {
                    return
                }
                for w in watchers {
                    w.left(copy(n), "left")
                }
            }
            // A static group knows its list already; an introduction is news
            // of nothing.
            is PeerIntro {}
            is None {}
        }
    }
}

// Every node in [known], in the map's order.
fn known_nodes(known: Map<NodeId, Node>) [] -> List<Node> => known {
    let all_known: Mut List<Node> = Mut List()
    for id in keys(known) {
        let n = get(known, id)
        if !(n is None) {
            add(all_known, copy(n))
        }
    }
    return all_known
}

// A [NodeGroup] that starts from a few known nodes and learns the rest:
// for a cluster whose nodes are not all known in advance.
//
// As it starts, it connects the node if nothing has and greets each of
// [seeds]. When it meets a new node, it tells that node of every node it
// already knows, and tells each of those of the newcomer; a node told of an
// endpoint greets it in turn. So from any seeds that connect the cluster,
// every node comes to know every other. A greeting that cannot be delivered
// (the node is not up yet) is retried the next time that endpoint is
// introduced. It notices a departure only when the node sends `leave`, and
// it has no policy yet for a network partition.
//
// ```
// fn main() [use, spawn] {
//     let me = NodeEndpoint { host: "10.0.0.7", port: 7000 }
//     use HostTcpTransport(copy(me))
//     let seeds = List(NodeEndpoint { host: "10.0.0.1", port: 7000 })
//     let nodes = spawn GossipNodeGroup("shop", seeds) on pool(1)
//     let _ids = actor_group<Ping>(nodes)
// }
//
// actor effect Ping {
//     send fn ping(out: Reply<Str>) => !out
// }
// ```
//
// [name] is the group's name: nodes of differently named groups ignore each
// other. [seeds] are some nodes to start from; this node's own endpoint may
// be among them, and is skipped.
// @compilerrules [node-group]
export handler GossipNodeGroup(name: Str, seeds: List<NodeEndpoint>) [Transport, spawn] for NodeGroup {
    mailbox { capacity: 64 }

    state {
        known: Mut Map<NodeId, Node> = Mut Map()
        dialed: Mut Set<Str> = Mut Set()
        watchers: Mut List<eff[NodeGroupWatcher]> = Mut List()
    }

    init {
        let me = local_endpoint()
        let _connected = connect(copy(me))
        watch_control("", self@NodeGroup)
        for e in seeds {
            dial(dialed, name, copy(e))
        }
    }

    send fn members(out: Reply<List<Node>>) => !out {
        out.send(known_nodes(known))
    }

    // A subscriber hears first of every node already known, then of every
    // arrival and departure.
    send fn subscribe(w: eff[NodeGroupWatcher]) => !w {
        for id in keys(known) {
            let n = get(known, id)
            if !(n is None) {
                w.joined(copy(n))
            }
        }
        add(watchers, w)
    }

    send fn leave() {
        leave_group(known)
    }

    send fn control(from: NodeId, data: Bytes) => !from, !data {
        let event = handshake(name, from, data)
        when event {
            is PeerHello {
                if contains_key(known, event.node) {
                    return
                }
                // Everyone already known learns of the newcomer, and the
                // newcomer learns of everyone already known.
                let others: Mut List<NodeEndpoint> = Mut List()
                for id in keys(known) {
                    let n = get(known, id)
                    if !(n is None) {
                        add(others, copy(n.at))
                    }
                    introduce(copy(id), List(copy(event.at)))
                }
                introduce(copy(event.node), others)
                add(dialed, to_str(event.at))
                let n = Node { id: copy(event.node), at: copy(event.at) }
                put(known, copy(event.node), copy(n))
                for w in watchers {
                    w.joined(copy(n))
                }
            }
            is PeerGone {
                let n = remove(known, event.node)
                if n is None {
                    return
                }
                for w in watchers {
                    w.left(copy(n), "left")
                }
            }
            is PeerIntro {
                for e in event.peers {
                    dial(dialed, name, copy(e))
                }
            }
            is None {}
        }
    }
}

// Says HELLO to an endpoint once; the handshake does the rest. A HELLO the
// transport could not deliver — the peer's node not on the wire yet — is
// forgotten, so the next introduction of that endpoint dials it again.
fn dial(dialed: Mut Set<Str>, group: Str, e: NodeEndpoint) [Transport] -> None
=> dialed: Mut, group, !e {
    if eq(e, local_endpoint()) || contains(dialed, to_str(e)) {
        return
    }
    add(dialed, to_str(e))
    let sent = deliver(copy(e), hello_frame(group))
    if sent is Err {
        let _forgot = remove(dialed, to_str(e))
    }
}

// ---------------------------------------------------------- actor group ----

// (Maintainer note.) `E` types the value and is erased in the output; the
// value carries what erasure drops, which is how a generic fn
// (`actor_group<E>`) gets to hold the name and hash of an `E` it cannot see.

// An actor protocol as a value: its name and the hash of its shape, which
// two nodes compare to tell whether they speak the same version of it.
//
// [protocol] makes one. [actor_group] takes one as an implicit parameter,
// filled at the call, so a program seldom handles it; a function of your
// own that is generic over a protocol takes the same implicit and passes it
// on.
// @compilerrules [actor-protocol-hash, effect-generic-decl]
// @compilerknown
export struct Protocol<E> {
    name: Str
    hash: Str
}

// (Maintainer note.) The one intrinsic that reads its type argument: an
// effect-only generic is erased in the output, so the name and hash of `E`
// exist only where `E` is written concretely. The checker finds it for an
// implicit `?protocol: () -> Protocol<E>` once the call's `E` is known, and
// passes it as an adapter closure whose body is the literal, as std's `iter`
// fills an `?Iterable`.

// The actor protocol [E] as a value: its name, as written, and the hash of
// its shape.
//
// This is what fills an implicit `?protocol: () -> Protocol<E>` once the
// call's `E` is known, so `actor_group<Ping>(nodes)` needs nothing more. A
// protocol whose messages carry a `noremote` payload has no wire form and no
// hash, and asking for one is a compile-time error.
//
// ```
// actor effect Ping {
//     send fn ping(out: Reply<Str>) => !out
// }
//
// fn ping_name() [] -> Str {
//     let p = protocol<Ping>()
//     return copy(p.name)                              // Ping
// }
// ```
// @compilerrules [actor-protocol-hash, implicit-intrinsic, implicit-resolve, net-noremote]
export intrinsic fn protocol<E>() [] -> Protocol<E>

// The set of actors serving protocol `E` across a cluster's nodes: what a
// program sends to when any one of several actors will do.
//
// Every node that opens the group with [actor_group] runs a **replica** of
// it, and the replicas of one name find each other through the node group
// and exchange their members. Each replica answers the members it knows of,
// on every node, as addresses that work wherever they arrive: a member on
// another node is reached through a proxy. A replica learns of another
// node's members a moment after the nodes meet, not at once.
//
// Members are added with `spawn H() on p in group`, or with [join] for an
// actor already running. A member whose node leaves is withdrawn from every
// replica, and subscribers hear of it. The group's type says which protocol
// it holds, so an `ActorGroup<Ping>` only ever has `eff[Ping]` members.
//
// To send to *a* member rather than list them, bind a router over the
// group: [LeastLoaded], [Sharded] or [Elected].
//
// ```
// handler Noticing() [Console] for Ordered ActorGroupWatcher<Ping> {
//     mailbox { capacity: 16 }
//     send fn joined(member: eff[Ping]) => !member {
//         println("+ a Ping on this node: ${eq(node_of(member), this_node())}")
//     }
//     send fn left(member: eff[Ping]) => !member {
//         println("- a Ping")
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let me = NodeEndpoint { host: "a", port: 1 }
//     let net = spawn MemNetwork() on pool(1)
//     use MemTransport(copy(me), net)
//     let p = pool(1)
//     let nodes = spawn StaticNodeGroup("demo", List(copy(me))) on p
//     let pings = actor_group<Ping>(nodes)
//     pings.subscribe(spawn Noticing() on p)
//     spawn Pinging("one") on p in pings               // + a Ping on this node: true
//     let all = waitfor out: Reply<List<eff[Ping]>> { pings.members(out) }
//     for m in all {
//         println(waitfor r: Reply<Str> { m.ping(r) }) // pong from one
//     }
// }
//
// actor effect Ping {
//     send fn ping(out: Reply<Str>) => !out
// }
//
// handler Pinging(who: Str) for Ordered Ping {
//     mailbox { capacity: 8 }
//     send fn ping(out: Reply<Str>) => !out { out.send("pong from ${who}") }
// }
// ```
// @compilerrules [actor-group, net-addr-routable, effect-generic-decl]
// @compilerknown
export actor effect ActorGroup<E> {
    // Adds [member]; every replica of the group comes to know it.
    send fn join(member: eff[E]) => !member
    // Withdraws [member] from this replica. The other nodes' replicas keep
    // it until its node leaves.
    send fn leave(member: eff[E]) => !member
    // Answers every member this replica knows of, on every node.
    send fn members(out: Reply<List<eff[E]>>) => !out
    // Subscribes [w] to hear of members joining and leaving from now on.
    send fn subscribe(w: eff[ActorGroupWatcher<E>]) => !w
    // Tells every route over this group that something its selector reads
    // outside the members has changed (an election's answer), so the
    // selector's `changed` runs again before the next pick.
    // @compilerrules [actor-router]
    send fn refresh()
}

// What a subscriber to an [ActorGroup] hears: a member joining, and a
// member leaving.
// @compilerrules [actor-group]
export actor effect ActorGroupWatcher<E> {
    // [member] has joined the group, on this node or another.
    send fn joined(member: eff[E]) => !member
    // [member] has been withdrawn, or its node has left.
    send fn left(member: eff[E]) => !member
}

// ---- the replica's tools (private)
//
// An actor group has no home: every node that opens it runs a **replica**,
// and replicas find each other by **name** — each listens for control frames
// on its group's name, and shares its member set, with its hash of the
// protocol, with the same-named replica of every node its node group tells it
// of. A replica hearing from a new peer shares back, so two replicas converge
// whichever opened first. Members travel as addresses; nothing is copied or
// moved.

// Shares [members] with the replica named [name] on peer
// [node], stamped with this side's [hash] of the protocol.
// @compilerrules [actor-group]
fn share_members<E>(name: Str, hash: Str, node: NodeId, members: List<eff[E]>) [] -> None
=> name, hash, node, members {
    send_control(copy(node), copy(name), encode((copy(hash), copy(members))))
}

// How many messages are waiting for an actor, as seen from this node.
//
// For an actor on this node it is the length of its queue; for a proxy, the
// messages sent through it that the far node has not yet taken off the
// queue. It is a moment's reading: it changes with every send. [LeastLoaded] reads
// it per send to pick the lightest member.
//
// ```
// fn lighter(a: eff[Ping], b: eff[Ping]) [] -> eff[Ping] => a, b {
//     if pending(a) <= pending(b) {
//         return copy(a)
//     }
//     return copy(b)
// }
//
// actor effect Ping {
//     send fn ping(out: Reply<Str>) => !out
// }
// ```
// @compilerrules [actor-group, net-remote-backpressure]
export fn pending<E>(a: eff[E]) [] -> Int => a {
    return pending(addr_index(a))
}

// Opens the [ActorGroup] of protocol `E` on this node, over the node group
// [nodes], and answers its replica.
//
// Written with the protocol as a type argument: `actor_group<Ping>(nodes)`.
// The group is named after the protocol, and the replicas that other nodes
// of the node group open under the same name are the same group: they
// exchange members as the nodes meet. A node whose version of the protocol
// differs (its hash does not match) is ignored, so its members never appear
// here. The replica runs on a pool of its own.
//
// A program has one node group, about machines, and opens one actor group
// per protocol whose actors it spreads across them. A protocol whose
// messages carry a `noremote` payload cannot cross a node boundary, and
// opening a group of it is a compile-time error.
//
// ```
// fn open(nodes: eff[NodeGroup]) [spawn] -> eff[ActorGroup<Ping>] => !nodes {
//     let pings = actor_group<Ping>(nodes)
//     spawn Pinging() on pool(1) in pings
//     return pings
// }
//
// actor effect Ping {
//     send fn ping(out: Reply<Str>) => !out
// }
//
// handler Pinging() for Ordered Ping {
//     mailbox { capacity: 8 }
//     send fn ping(out: Reply<Str>) => !out { out.send("pong") }
// }
// ```
//
// @param nodes     this node's node group, through which the replicas meet
// @param protocol  found at the call site: the protocol's name and hash
// @compilerrules [actor-group, net-noremote, effect-generic-decl, actor-protocol-hash, implicit-resolve, implicit-forward]
export fn actor_group<E>(nodes: eff[NodeGroup], ?protocol: () -> Protocol<E>) [spawn] -> eff[ActorGroup<E>]
=> !nodes {
    // An ordinary fn: the protocol's name and hash are constants of the
    // *written* `E`, and an effect-only generic is erased in the output, so
    // this body cannot produce them. The implicit `protocol` is filled at the
    // call, where `E` is concrete, with the one intrinsic that reads its type
    // argument; a generic caller forwards its own `?protocol`, as with any
    // implicit.
    let proto = protocol()
    let name = copy(proto.name)
    return actor_group<E>(name, nodes)
}

// [actor_group] under a [name] of your own, for several groups of one
// protocol: `actor_group<Ping>("east", nodes)` and
// `actor_group<Ping>("west", nodes)` are two groups.
// @compilerrules [actor-group, actor-deadlock-cycle]
export fn actor_group<E>(name: Str, nodes: eff[NodeGroup], ?protocol: () -> Protocol<E>) [spawn] -> eff[ActorGroup<E>]
=> !name, !nodes {
    // The subscription to the node group is sent from here rather than from
    // the replica's `init`: sent by the replica it would be a
    // `NodeGroupWatcher -> NodeGroup -> NodeGroupWatcher` edge in the deadlock
    // graph (a real one, since the node group sends `joined` back), and sent
    // from the opener's frame it is not. [actor-deadlock-cycle]
    let (group, watcher) = spawn ActorGrouping<E>(name, protocol()) on pool(1)
    nodes.subscribe(watcher)
    return group
}

// Adds [member] to [group]; every replica of the group comes to know it.
// `spawn Pinging() on p in pings` is the same, written at the spawn.
//
// ```
// fn add(pings: eff[ActorGroup<Ping>], p: Pool) [spawn] -> None => pings, !p {
//     let one = spawn Pinging() on p
//     join(pings, one)
// }
//
// actor effect Ping {
//     send fn ping(out: Reply<Str>) => !out
// }
//
// handler Pinging() for Ordered Ping {
//     mailbox { capacity: 8 }
//     send fn ping(out: Reply<Str>) => !out { out.send("pong") }
// }
// ```
// @compilerrules [actor-group]
export fn join<E>(group: eff[ActorGroup<E>], member: eff[E]) [] -> None => group, !member {
    (group.join)(member)
}

// The replica of an actor group that [actor_group] spawns: one per node and
// group. Its [NodeGroupWatcher] face hears the node group, so it shares its
// members with each node that arrives and withdraws the members of each
// node that leaves. A program opens groups with [actor_group] rather than
// spawning this itself.
// @compilerrules [actor-group, actor-router]
export handler ActorGrouping<E>(name: Str, proto: Protocol<E>) [spawn] for ActorGroup<E>, NodeGroupWatcher {
    // State: members as a list of addrs (an addr has no `hash` yet, so
    // membership is checked by `eq`), the peer replicas, the subscribers. A
    // node's arrival is where the replica first shares with that node's
    // replica. After every change it mirrors the member set into the runtime
    // (`view_set`), which is what a router reads on the sender's
    // thread: the replica is behind the view by one message, never in the
    // send path.
    mailbox { capacity: 64 }

    state {
        all: Mut List<eff[E]> = Mut List()
        peers: Mut List<NodeId> = Mut List()
        watchers: Mut List<eff[ActorGroupWatcher<E>]> = Mut List()
    }

    // The replica starts itself: it listens on its group's name, so the
    // replicas other nodes open under the same name reach it.
    init {
        watch_control(copy(name), self@ActorGroup<E>)
    }

    send fn join(member: eff[E]) => !member {
        if !admit(all, copy(member)) {
            return
        }
        mirror(self@ActorGroup<E>, all)
        for w in watchers {
            w.joined(copy(member))
        }
        for p in peers {
            share_members(name, proto.hash, copy(p), List(copy(member)))
        }
    }

    send fn leave(member: eff[E]) => !member {
        if !withdraw(all, copy(member)) {
            return
        }
        mirror(self@ActorGroup<E>, all)
        for w in watchers {
            w.left(copy(member))
        }
    }

    // A node of the group: its replica of this name, if it has
    // one, hears ours — and answers with its own when we are new to it.
    // @compilerrules [actor-group]
    send fn joined(n: Node) => !n {
        share_members(name, proto.hash, n.id, all)
    }

    // A node that left took its members with it: withdraw
    // every addr it hosted, telling the subscribers. The runtime has already
    // killed the proxies, so a send to one is the silent no-op.
    // @compilerrules [actor-group]
    send fn left(n: Node, why: Str) => !n, !why {
        let gone: Mut List<eff[E]> = Mut List()
        for m in all {
            if eq(node_of(m), n.id) {
                add(gone, copy(m))
            }
        }
        for m in gone {
            if withdraw(all, copy(m)) {
                for w in watchers {
                    w.left(copy(m))
                }
            }
        }
        if size(gone) > 0 {
            mirror(self@ActorGroup<E>, all)
        }
    }

    send fn members(out: Reply<List<eff[E]>>) => !out {
        out.send(copy(all))
    }

    send fn refresh() {
        view_refresh(self@ActorGroup<E>)
    }

    send fn subscribe(w: eff[ActorGroupWatcher<E>]) => !w {
        add(watchers, w)
    }

    // A peer replica's members. A peer whose
    // version of the protocol differs is invisible: its members are never
    // merged, and it never hears ours.
    // @compilerrules [actor-group, actor-protocol-hash]
    send fn control(from: NodeId, data: Bytes) => !from, !data {
        let got = decode<(Str, List<eff[E]>)>(data)
        if got is None {
            return
        }
        if !eq(got.0, proto.hash) {
            return
        }
        if !contains_node(peers, copy(from)) {
            add(peers, copy(from))
            share_members(name, proto.hash, copy(from), all)
        }
        let changed = false
        for m in got.1 {
            if admit(all, copy(m)) {
                changed = true
                for w in watchers {
                    w.joined(copy(m))
                }
            }
        }
        if changed {
            mirror(self@ActorGroup<E>, all)
        }
    }
}

// Mirrors [members] into the runtime's view of [group].
// @compilerrules [actor-router]
fn mirror<E>(group: eff[ActorGroup<E>], members: List<eff[E]>) [] -> None => group, members {
    view_set(copy(group), copy(members))
}

// Adds an addr to a list unless it is already there; answers whether it was
// added. Linear scan: a group's member set is small.
fn admit<E>(list: Mut List<eff[E]>, a: eff[E]) [] -> Bool => list: Mut, !a {
    for x in list {
        if eq(x, a) {
            return false
        }
    }
    add(list, a)
    return true
}

fn contains_node(list: List<NodeId>, n: NodeId) [] -> Bool => list, n {
    for x in list {
        if eq(x, n) {
            return true
        }
    }
    return false
}

fn withdraw<E>(list: Mut List<eff[E]>, a: eff[E]) [] -> Bool => list: Mut, !a {
    let mut_index: Int? = None
    let i = 0
    for x in list {
        if eq(x, a) {
            mut_index = i
        }
        i = i + 1
    }
    if mut_index is None {
        return false
    }
    let _removed = remove_at(list, mut_index)
    return true
}

// The node an actor lives on: the node of the pool it was spawned on, also
// for a proxy of an actor on another node.
//
// ```
// fn is_local(a: eff[Ping]) [] -> Bool => a {
//     return eq(node_of(a), this_node())
// }
//
// actor effect Ping {
//     send fn ping(out: Reply<Str>) => !out
// }
// ```
// @compilerrules [net-addr-routable]
export fn node_of<E>(a: eff[E]) [] -> NodeId => a {
    return NodeId { value: identity(addr_index(a)).node }
}

// ---------------------------------------------------------- the pick kit ----

// A pick runs on the sender's thread at every send, so it cannot *ask* the
// replica (a hop per send is what the stub exists to avoid). It reads a
// **mirror** the replica keeps in the runtime, written after every change:
// behind the replica by one message, never in the send path.

// (private) The mirror: written by the replica, read by a router per send.
// A group with no replica on this node has an empty view, so a router over
// it waits. Every write moves the view's **version**, and so does
// `refresh`; a waiting send wakes when it moves.
// @compilerrules [actor-router]
fn view_set<E>(group: eff[ActorGroup<E>], members: List<eff[E]>) [] -> None
=> !group, !members {
    let ixs = Mut List<Int>()
    for m in members {
        add(ixs, addr_index(m))
    }
    view_set(addr_index(group), ixs)
}

fn view_members<E>(group: eff[ActorGroup<E>]) [] -> List<eff[E]> => group {
    let out: Mut List<eff[E]> = Mut List()
    for ix in view_members(addr_index(group)) {
        add(out, addr_of<E>(ix))
    }
    return out
}

fn view_version<E>(group: eff[ActorGroup<E>]) [] -> Long => group {
    return view_version(addr_index(group))
}

fn view_refresh<E>(group: eff[ActorGroup<E>]) [] -> None => group {
    view_refresh(addr_index(group))
}

fn view_wait<E>(group: eff[ActorGroup<E>], seen: Long, nanos: Long) [] -> None
=> group, !seen, !nanos {
    view_wait(addr_index(group), seen, nanos)
}

// One member of a group as a router sees it.
//
// Its load is not here, since it changes with every send: read it when it is
// wanted, with [pending].
// @compilerrules [actor-router]
export struct RouteMember<E> {
    addr: eff[E]
    local: Bool
}

// The members a router chooses from: every member this node's replica of
// the group knows, in an order that is the same on every node (by node,
// then by actor), and the [version] of the group they are at.
//
// The version moves whenever the members change or the group is refreshed
// (`ActorGroup.refresh`), so a router that derives something from the
// members (a leader, a ring) keeps the version it derived it at and works
// it out again when the version differs.
// @compilerrules [actor-router]
export struct RouteView<E> {
    members: List<RouteMember<E>>
    version: Long
}

// The members of [group] a router chooses from now, read on the sender's
// thread from a copy the replica keeps: no message to the replica.
//
// ```
// fn first<E>(group: eff[ActorGroup<E>]) [] -> eff[E]? => group {
//     let view = route_view(group)
//     let m = get(view.members, 0) ?: return None
//     return copy(m.addr)
// }
// ```
// @compilerrules [actor-router]
export fn route_view<E>(group: eff[ActorGroup<E>]) [] -> RouteView<E> => group {
    let version = view_version(copy(group))
    let members: Mut List<RouteMember<E>> = Mut List()
    for m in view_members(copy(group)) {
        add(members, RouteMember { addr: copy(m), local: eq(node_of(m), this_node()) })
    }
    return RouteView { members: copy(members), version: version }
}

// Waits while a router has no member to send to: until [group]'s members
// move past [view]'s version, or [wait] passes. Answers the wait for next
// time, [wait] doubled, up to [config]'s `max_wait`.
//
// A router loops on it, so a send to an empty group, or to a group with no
// leader, waits for a member rather than failing:
//
// ```
// fn pick<E>(group: eff[ActorGroup<E>], config: RouteConfig) [] -> eff[E] => group, config {
//     let wait = copy(config.first_wait)
//     while true {
//         let view = route_view(group)
//         let m = get(view.members, 0)
//         if !(m is None) {
//             return copy(m.addr)
//         }
//         wait = route_wait(group, view, wait, config)
//     }
//     return pick(group, config)
// }
// ```
// @compilerrules [actor-router]
export fn route_wait<E>(
    group: eff[ActorGroup<E>],
    view: RouteView<E>,
    wait: Duration,
    config: RouteConfig
) [] -> Duration
=> group, view, !wait, config {
    view_wait(copy(group), copy(view.version), copy(wait.nanos))
    let next = wait.nanos * 2L
    if next > config.max_wait.nanos {
        return copy(config.max_wait)
    }
    return Duration { nanos: next }
}

// (Maintainer note.) A fn rather than a field default: a field default is
// inlined where the literal is written, which would need `Duration`
// imported there.

// The default waits of a router: 1 ms at first, doubling up to 5 s. The
// default of the `config` parameter of [LeastLoaded], [Sharded] and
// [Elected].
// @compilerrules [actor-router]
export fn default_route_config() [] -> RouteConfig {
    return RouteConfig {}
}

// How long a router waits between looks while it has no member to send to:
// [first_wait] at first, doubling each time up to [max_wait].
//
// A waiting send also wakes at once whenever the group's members change.
// There is no time limit: the send waits until there is a member, which is
// how a send to an empty group, or to a group without a leader, waits for
// one rather than failing.
//
// ```
// import std.time
//
// fn quick() [] -> RouteConfig {
//     return RouteConfig { first_wait: millis(10), max_wait: seconds(1) }
// }
// ```
// @compilerrules [actor-router]
export struct RouteConfig {
    first_wait: Duration = Duration { nanos: 1000000L }
    max_wait: Duration = Duration { nanos: 5000000000L }
}

// The hash of [k]'s wire encoding, the same on both backends and every
// node: what [Sharded] reduces to a member, through a `key` fn
// `std.net.shard` stamps.
// @compilerrules [actor-router]
export intrinsic fn key_hash<T>(k: T) [] -> Long => k

// (Maintainer note.) Exported because a router's instantiation lives in the
// module of the protocol it routes, which calls it from there.

// The member of [view] with the fewest messages waiting ([pending]), local
// members first when [prefer_local] asks; `None` when there are none. What
// [LeastLoaded] picks.
// @compilerrules [actor-router]
export fn least_loaded<E>(view: RouteView<E>, prefer_local: Bool) [] -> eff[E]? => view, !prefer_local {
    let best: RouteMember<E>? = None
    let best_pending = 0
    for a in view.members {
        let load = pending(copy(a.addr))
        if best is None {
            best = copy(a)
            best_pending = load
        } else {
            let b: RouteMember<E> = best
            // A local member beats a remote one when locality is preferred;
            // otherwise, or between two of the same locality, the lighter
            // queue wins.
            let take = when {
                prefer_local && a.local && !b.local {
                    true
                }
                prefer_local && !a.local && b.local {
                    false
                }
                else {
                    load < best_pending
                }
            }
            if take {
                best = copy(a)
                best_pending = load
            }
        }
    }
    let chosen: RouteMember<E> = best ?: (return None)
    return copy(chosen.addr)
}

// The router that sends each message to the member of [group] with the
// fewest messages waiting ([pending]), read at every send.
//
// A router is a handler of the protocol it routes: `use LeastLoaded(group,
// prefer_local)` binds `E`, and each send through it goes to one member.
// With [prefer_local], a member on this node is taken over any member on
// another, since a message within the node does not cross the wire; the
// load decides among members on the same side. With no members the send
// waits for one, as [config] says.
//
// ```
// fn fresh_id(seq: eff[ActorGroup<Sequencer>]) [use] -> Str => !seq {
//     use LeastLoaded(seq, true)
//     return waitfor out: Reply<Str> { next(out) }
// }
//
// actor effect Sequencer {
//     send fn next(out: Reply<Str>) => !out
// }
// ```
// @compilerrules [actor-router, actor-sent]
export handler LeastLoaded<E>(
    group: eff[ActorGroup<E>],
    prefer_local: Bool,
    config: RouteConfig = default_route_config()
) for E {
    mailbox { capacity: 16 }

    send fn sent(m: message[E]) => !m {
        let wait = copy(config.first_wait)
        while true {
            let view = route_view(group)
            let best = least_loaded(view, copy(prefer_local))
            if !(best is None) {
                send(best, m)
                return
            }
            wait = route_wait(group, view, wait, config)
        }
    }
}

// The router that sends each message to the member of [group] that owns
// its key: two sends with the same key go to the same member, in order. It
// claims that ([ShardKeyOrdered]), and not [Ordered]: two keys may reach two
// members.
//
// The key is what the `key` fn in scope answers for the message; `std.net.
// shard` stamps one from a `ShardKey` mark on a send fn's parameter, and a
// message with no key goes to the first member. The member is the key's
// hash modulo the number of members, over a view whose order is the same on
// every node, so every node sends a key to the same member. When a member
// joins or leaves, many keys move to another member (there is no consistent
// hashing yet). With no members the send waits for one, as [config] says.
//
// ```
// import std.net.shard
//
// actor effect Inventory {
//     send fn reserve(ShardKey sku: Str, qty: Int, out: Reply<Str>) => !sku, !out
// }
//
// fn key(m: message[Inventory]) [] -> Long? => m by shard
//
// fn shop(stock: eff[ActorGroup<Inventory>]) [use, Console] -> None => !stock {
//     use Sharded(stock)
//     println(waitfor out: Reply<Str> { reserve("apple", 1, out) })
//     println(waitfor out: Reply<Str> { reserve("apple", 2, out) })    // the same member
// }
// ```
// @param key found at the use: the message's key, or `None`.
// @compilerrules [actor-router, actor-sent, effect-claim]
export handler Sharded<E>(
    group: eff[ActorGroup<E>],
    config: RouteConfig = default_route_config(),
    ?key: (m: message[E]) -> Long?
) for ShardKeyOrdered E {
    mailbox { capacity: 16 }

    send fn sent(m: message[E]) => !m {
        let k = key(m) ?: 0L
        let magnitude = if k < 0L {
            0L - k
        } else {
            k
        }
        let wait = copy(config.first_wait)
        while true {
            let view = route_view(group)
            let n = size(view.members)
            if n > 0 {
                let picked = get(view.members, to_int(magnitude % to_long(n))) ?: unreachable!()
                send(copy(picked.addr), m)
                return
            }
            wait = route_wait(group, view, wait, config)
        }
    }
}

// Answers which node leads, for [Elected]: `None` while there is no leader
// (an election is under way).
//
// Whoever serves it decides: an election written in Salvo, a platform
// handler over a lease store, or [StaticLeader] for a fixed one. [Elected]
// asks it only when the group's members change, not at every send, so when
// the answer changes on its own, whatever decides it calls `refresh()` on
// every group routed by it.
//
// ```
// // The node whose host name sorts last leads.
// handler LastHost(nodes: eff[NodeGroup], me: NodeEndpoint) for Leader {
//     fn leader() -> NodeId? {
//         let peers = waitfor out: Reply<List<Node>> { nodes.members(out) }
//         let best_host = copy(me.host)
//         let best = this_node()
//         for n in peers {
//             if n.at.host > best_host {
//                 best_host = copy(n.at.host)
//                 best = copy(n.id)
//             }
//         }
//         return best
//     }
// }
// ```
// @compilerrules [actor-router]
export effect Leader {
    // Answers the node that leads now, or `None` while there is none.
    fn leader() -> NodeId?
}

// A [Leader] that always answers [node]: for a test, or for a deployment
// whose primary node is fixed by configuration.
// @compilerrules [actor-router]
export handler StaticLeader(node: NodeId) for Leader {
    fn leader() -> NodeId? {
        return copy(node)
    }
}

// (Maintainer note.) Exported for the reason [least_loaded] is.

// The member of [view] on the node [Leader] answers, or `None`. What
// [Elected] picks.
// @compilerrules [actor-router]
export fn on_leader<E>(view: RouteView<E>) [Leader] -> eff[E]? => view {
    let l = leader() ?: (return None)
    for a in view.members {
        if eq(node_of(copy(a.addr)), l) {
            return copy(a.addr)
        }
    }
    return None
}

// The router that sends every message to the member of [group] on the
// leader's node, as the [Leader] in scope answers it.
//
// It asks the leader when the group's members change (or the group is
// refreshed) and keeps the answer. While there is no leader, or the leader's
// node hosts no member of the group, sends wait, as [config] says. When the
// leader's node leaves, its members are withdrawn, and the next send asks
// the leader again: this is how a singleton fails over.
//
// ```
// import std.time
//
// actor effect Sequencer {
//     send fn next(out: Reply<Str>) => !out
// }
//
// fn ids(seq: eff[ActorGroup<Sequencer>], primary: NodeId) [use, Console] -> None => !seq, !primary {
//     use StaticLeader(primary)
//     use Elected(seq, config = RouteConfig { first_wait: millis(10), max_wait: seconds(1) })
//     let first = waitfor out: Reply<Str> { next(out) }
//     let second = waitfor out: Reply<Str> { next(out) }
//     println("${first} ${second}")                    // both from the primary's member
// }
// ```
// @compilerrules [actor-router, actor-sent]
export handler Elected<E>(group: eff[ActorGroup<E>], config: RouteConfig = default_route_config()) [Leader] for E {
    mailbox { capacity: 16 }

    state {
        seen: Long = -1L
        chosen: eff[E]? = None
    }

    send fn sent(m: message[E]) => !m {
        let wait = copy(config.first_wait)
        while true {
            let view = route_view(group)
            if view.version != seen {
                chosen = on_leader(view)
                seen = copy(view.version)
            }
            let to = copy(chosen)
            if !(to is None) {
                send(to, m)
                return
            }
            wait = route_wait(group, view, wait, config)
        }
    }
}

// ---------------------------------------------------------------- host ----

// (Maintainer note.) The host class lives in `lib/platform/std/net.{rs,kt}`,
// one per backend, both signing the `threadsafe` contract: a transport is
// reached from every pool of the program at once, so the host synchronizes
// its own connection table and the compiler shares the instance with no
// lock.

// The transport over TCP: what each node of a real cluster uses, one node
// per process.
//
// [bind] is this node's endpoint: `listen` opens a TCP listener there (a
// node group or [connect] does that for you), and it is the `from` of every
// frame this node sends. The first frame to a peer opens one TCP connection
// to it, which carries every later frame; the caller's thread waits while
// it connects, with no time limit of its own. A connection that cannot be
// made answers [Unreachable]; a write that fails answers [WireFailed] and
// drops the connection, and the next frame connects again. Both backends
// put the same bytes on the wire, so a Kotlin node and a Rust node can talk.
//
// Frames travel as plain TCP, neither encrypted nor authenticated: bind to
// an address only the cluster's own machines can reach. Each listener and
// each accepted connection has a thread of its own, and while a listener is
// open the program is not considered idle. An actor test cannot use it; it
// uses [MemTransport].
//
// ```
// fn main() [use, spawn] {
//     let me = NodeEndpoint { host: "10.0.0.1", port: 7000 }
//     use HostTcpTransport(copy(me))
//     let nodes = spawn StaticNodeGroup("demo", List(copy(me), NodeEndpoint { host: "10.0.0.2", port: 7000 })) on pool(1)
//     let peers = waitfor out: Reply<List<Node>> { nodes.members(out) }
// }
// ```
// @compilerrules [platform-tree, platform-threadsafe]
export threadsafe platform handler HostTcpTransport(bind: NodeEndpoint) for Transport

// ------------------------------------------------------------------ mem ----

// (Maintainer note.) Why two pieces: a handler of *several plain effects*
// cannot yet be shared stateful (one lock behind several faces has no
// backend form), so the fault controls could not ride on `MemTransport` as
// a second face the way `ManualTime` carries `TimerCtl`. An actor face costs
// nothing, and faults belong to the network, not to one node.

// An in-memory network that many virtual nodes share, with the faults a
// test wants to script: the protocol of [MemNetwork].
//
// The in-memory double of a real network comes in two pieces, as a real
// network does: **one network**, a [MemNetwork] actor spawned once, and
// **one transport per node**, a [MemTransport] for each node's endpoint. The
// network holds who listens where, and the faults: `partition` severs two
// endpoints, `heal` restores them, and `kill` takes a node off until
// something listens there again. Everything above the transport runs on it
// as it does over TCP: the same frames, the same actors, the same errors.
// What it does not do is serialize frames for a socket, so it cannot find a
// fault in a host transport.
//
// Two nodes in one process: `a` is `main`'s own node, and `b` is a virtual
// node, booted from an actor on its own pool with its own transport.
//
// ```
// import std.time
//
// actor effect Ping {
//     send fn ping(out: Reply<Str>) => !out
// }
//
// handler Pinging(who: Str) for Ordered Ping {
//     mailbox { capacity: 8 }
//     send fn ping(out: Reply<Str>) => !out { out.send("pong from ${who}") }
// }
//
// actor effect Boot {
//     send fn boot(done: Reply<Bool>) => !done
// }
//
// // Brings up a node from inside it: everything spawned here is on it.
// handler Booting(all: List<NodeEndpoint>) [Transport, spawn] for Ordered Boot {
//     mailbox { capacity: 1 }
//     send fn boot(done: Reply<Bool>) => !done {
//         let p = pool(1)
//         let nodes = spawn StaticNodeGroup("demo", copy(all)) on p
//         let pings = actor_group<Ping>(nodes)
//         spawn Pinging("b") on p in pings
//         done.send(true)
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let a = NodeEndpoint { host: "a", port: 1 }
//     let b = NodeEndpoint { host: "b", port: 1 }
//     let all = List(copy(a), copy(b))
//     let net = spawn MemNetwork() on pool(1)
//
//     use MemTransport(copy(a), copy(net))
//     let p = pool(1)
//     let nodes = spawn StaticNodeGroup("demo", copy(all)) on p
//     let pings = actor_group<Ping>(nodes)
//     spawn Pinging("a") on p in pings
//
//     let pb = pool_at(new_node(), 1)
//     let booter = spawn Booting(all) with MemTransport(copy(b), copy(net)) on pb
//     let _up = waitfor done: Reply<Bool> { booter.boot(done) }
//
//     // A moment for the nodes to meet.
//     let timer = spawn DefaultTimer() on pool(1)
//     let _t = waitfor f: Reply<Fired> { timer.after(millis(500), f) }
//     let seen = waitfor out: Reply<List<eff[Ping]>> { pings.members(out) }
//     println("${size(seen)} members")                 // 2 members
//     net.partition(copy(a), copy(b))                  // from here, a and b cannot talk
// }
// ```
export actor effect MemNet {
    // Registers [sink] as the listener at [at], replacing any previous one,
    // and brings a killed endpoint back. `MemTransport.listen` sends it.
    send fn attach(at: NodeEndpoint, sink: eff[Inbound]) => !at, !sink
    // Drops the listener at [at]. `MemTransport.unlisten` sends it.
    send fn detach(at: NodeEndpoint) => !at
    // Answers the listener a frame from [from] to [to] goes to, or `None`
    // when [to] is partitioned from [from], killed, or has no listener; an
    // answered listener counts as one delivered frame. `MemTransport.deliver`
    // asks it.
    send fn route(from: NodeEndpoint, to: NodeEndpoint, out: Reply<eff[Inbound]?>) => !from, !to, !out
    // Severs [a] from [b], in both directions, until `heal(a, b)`.
    send fn partition(a: NodeEndpoint, b: NodeEndpoint) => !a, !b
    // Undoes `partition(a, b)`.
    send fn heal(a: NodeEndpoint, b: NodeEndpoint) => !a, !b
    // Takes [node] off the network: its listener is dropped, and every frame
    // to it is unreachable until something listens there again.
    send fn kill(node: NodeEndpoint) => !node
    // Answers how many frames have been routed so far: what a test checks
    // after a partition to see that nothing got through.
    send fn delivered(out: Reply<Int>) => !out
}

// The in-memory network: spawn one per test (or per in-process cluster) and
// give its address to each node's [MemTransport].
//
// It is one actor, so every frame between its nodes passes through its
// mailbox (64 messages) to find its listener. A pair partitioned with
// `partition` stays so until `heal`, in both directions; a killed endpoint
// stays dead until something listens there again.
export handler MemNetwork for Ordered MemNet {
    mailbox { capacity: 64 }

    state {
        listeners: Mut Map<NodeEndpoint, eff[Inbound]> = Mut Map()
        cuts: Mut Set<Str> = Mut Set()
        dead: Mut Set<NodeEndpoint> = Mut Set()
        count: Int = 0
    }

    send fn attach(at: NodeEndpoint, sink: eff[Inbound]) => !at, !sink {
        remove(dead, at)
        put(listeners, at, sink)
    }

    send fn detach(at: NodeEndpoint) => !at {
        remove(listeners, at)
    }

    send fn route(from: NodeEndpoint, to: NodeEndpoint, out: Reply<eff[Inbound]?>) => !from, !to, !out {
        if contains(dead, to) || contains(cuts, cut_key(from, to)) {
            out.send(None)
            return
        }
        let sink = get(listeners, to)
        if sink is None {
            out.send(None)
            return
        }
        count = count + 1
        out.send(copy(sink))
    }

    send fn partition(a: NodeEndpoint, b: NodeEndpoint) => !a, !b {
        add(cuts, cut_key(a, b))
        add(cuts, cut_key(b, a))
    }

    send fn heal(a: NodeEndpoint, b: NodeEndpoint) => !a, !b {
        remove(cuts, cut_key(a, b))
        remove(cuts, cut_key(b, a))
    }

    send fn kill(node: NodeEndpoint) => !node {
        remove(listeners, node)
        add(dead, node)
    }

    send fn delivered(out: Reply<Int>) => !out {
        out.send(count)
    }
}

// One node's [Transport] over a shared [MemNetwork]: [me] is the endpoint
// it stands for, and [net] the network.
//
// It holds no state of its own, so it can be bound with `use` and handed to
// any actor. `listen` registers with the network and always answers `Ok`;
// `deliver` asks the network for the listener at the destination and sends
// the frame to it, answering [Unreachable] when the two endpoints are
// partitioned, the destination is killed, or nobody listens there. While it
// waits for the network's answer, the caller's thread serves its own pool,
// so delivering from inside an actor cannot stall that actor's pool.
//
// ```
// fn main() [use, spawn] {
//     let a = NodeEndpoint { host: "a", port: 1 }
//     let net = spawn MemNetwork() on pool(1)
//     use MemTransport(copy(a), net)
//     let nodes = spawn StaticNodeGroup("solo", List(copy(a))) on pool(1)
//     let peers = waitfor out: Reply<List<Node>> { nodes.members(out) }
// }
// ```
// @compilerrules [effect-handle, waitfor-pump]
export handler MemTransport(me: NodeEndpoint, net: eff[MemNet]) for Transport {
    fn listen(at: NodeEndpoint, sink: eff[Inbound]) -> Ok None | Err NetError => at, !sink {
        net.attach(copy(at), sink)
        return ok(None)
    }

    fn unlisten(at: NodeEndpoint) -> None => at {
        net.detach(copy(at))
    }

    fn deliver(to: NodeEndpoint, frame: Bytes) -> Ok None | Err NetError => to, !frame {
        let sink = waitfor out: Reply<eff[Inbound]?> {
            net.route(copy(me), copy(to), out)
        }
        if sink is None {
            return err(Unreachable { to: copy(to) })
        }
        sink.receive_frame(copy(me), frame)
        return ok(None)
    }

    fn local_endpoint() -> NodeEndpoint {
        return copy(me)
    }
}

// A partition is a directed pair; keyed as text so one set holds both
// directions without a tuple key.
fn cut_key(a: NodeEndpoint, b: NodeEndpoint) [] -> Str => a, b {
    return "${to_str(a)}>${to_str(b)}"
}
