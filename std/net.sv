// `net`: actors across machines.
//
// The design has one line: **the network enters at the actor group, never at
// the spawn.** Every actor is spawned by the node that hosts it; what crosses
// the wire is addresses and messages, never construction; and a function
// declaring `[E]` never learns whether `E` is one local handler, an actor on
// another machine, or a group of a hundred across a fleet.
//
// This module is that stack, bottom up. Each layer is ordinary Salvo over the
// one below, and a program touches only the top three:
//
//   1. **The transport** — `Transport` (outbound: `deliver`, `listen`) and
//      `Inbound` (arriving frames as messages). Bytes between endpoints, at
//      most once and in order per pair, nothing more. `HostTcpTransport` is
//      the machine's, the one piece the platform owns; `MemTransport` over a
//      `MemNetwork` runs any number of virtual nodes in one process.
//   2. **The wire format** — `encode`/`decode`, the canonical encoding every
//      type has unless declared `noremote`, and the per-protocol hash two
//      nodes compare before they speak. Fixed in the compiler, identical on
//      both backends, so a Kotlin node and a Rust node are peers.
//   3. **Routing** — an `Addr<E>` that crosses the wire arrives as a proxy
//      with the same `send`; replies come back over the wire; a mailbox's
//      `capacity` holds across machines through credits. `NodeId` names a
//      node; `this_node`, `new_node`, `pool_at` host virtual nodes.
//      `connect(me)` binds the scheduler to the transport — its outbound
//      actor, its inbound actor, its own route — and a node group's `init`
//      calls it when nothing has. Everything the runtime needs underneath
//      (`add_route`, `route_frames`, `deliver_frame`, the handshake frames)
//      is private to this file: a program never calls it, and the comments
//      on each say what it does.
//   4. **Node groups** — `NodeGroup`, the membership of *machines*: a
//      mechanism (`StaticNodeGroup`, `GossipNodeGroup`) spawned on a node
//      with a `Transport` in scope, answering `members` and telling every
//      `NodeGroupWatcher` who subscribed. The handshake (group name,
//      endpoint, protocol table) is the runtime's and common to every
//      mechanism.
//   5. **Actor groups** — `ActorGroup<E>`, the membership of *actors* of one
//      protocol across those nodes: `actor_group<E>(nodes)` starts this
//      node's replica and publishes it by name, `spawn H() on p in group`
//      joins a member, `members`/`subscribe` read the set. `[any E]` is the
//      claim a function makes when any member will do.
//   6. **Picks** — `use route(group)` binds `any E` to whichever member a
//      `Pick<E>` policy chooses per send: `LeastLoaded`, `Sharded` by a
//      `Key`-marked argument, `Elected` behind a `Leader`.
//
// Rules: [net-transport] [net-host] [net-mem] [wire-format] [noremote]
// [protocol-hash] [addr-routable] [addr-capability] [remote-backpressure]
// [net-connect] [node-group] [actor-group] [effect-any] [route-stub].
//
// Two halves of the transport, deliberately asymmetric:
//
// * **Outbound** is a plain effect, [Transport]: `deliver(to, frame)` is a
//   call the sender makes on its own thread, and it answers whether the frame
//   was handed to the wire.
// * **Inbound** is an *actor* effect, [Inbound]: bytes arrive on a thread the
//   host owns (a socket reader), and the only way work from a foreign thread
//   enters the scheduler is a send. So a node that wants to receive spawns an
//   actor serving `Inbound` and registers its addr with `listen`; the
//   transport sends every arriving frame to it. That is the same host→runtime
//   upcall the timer makes when a deadline fires [time-timer].
//
// The platform owns the transport only: [HostTcpTransport] is the host
// class, declared `threadsafe` because a transport is called from every pool
// [threadsafe-platform]; [MemTransport] is the pure-Salvo double every test
// runs on — N virtual endpoints in one process, with the faults a test wants
// to script.

import time.Duration

// Where a transport can dial: known **before** any contact, which is what
// makes it the currency of node discovery. For TCP it is a host and a port;
// for [MemTransport] it is a virtual node's name. Printable, hashable and
// orderable, so it keys maps and sorts in a membership list.
//
// Distinct from a `Node`, which the network sequence's step ④ mints *after*
// a handshake and which carries an identity and a build — an endpoint says
// nothing about who answers there.
export struct NodeEndpoint : Ordered<self> by auto, Hashed<self> by auto {
    host: Str,
    port: Int
}

export fn to_str(e: NodeEndpoint) [] -> Str => e {
    return "${e.host}:${e.port}"
}

// Why a frame did not go: the destination is not reachable from here (no
// route, connection refused, or — under [MemTransport] — partitioned or
// killed), or the wire failed after a connection existed.
export struct Unreachable { to: NodeEndpoint }
export struct WireFailed { to: NodeEndpoint, reason: Str }
export type NetError = Unreachable | WireFailed

export fn to_str(e: NetError) [] -> Str => e {
    when e {
        is Unreachable { return "unreachable: ${to_str(e.to)}" }
        is WireFailed { return "wire failed to ${to_str(e.to)}: ${e.reason}" }
    }
}

// What a receiver serves: one member, one frame at a time, from whoever sent
// it. An **actor** effect, because frames arrive from a thread the scheduler
// does not own and a send is the only door in [actor-effect-kind].
//
// The frame is opaque `Bytes`: this layer does not know what a protocol is.
// The codecs (step ②) decode it one level up, in the actor that registered.
export actor effect Inbound {
    send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame
}

// The wire. A plain effect — its members are calls on the caller's thread —
// with the delivery side handed to an [Inbound] actor at `listen`.
//
// **Delivery is at most once and in order per (sender, receiver) pair**, and
// nothing more: a frame that is not delivered is *lost*, and the caller learns
// so only when the transport can tell (an `Err` from `deliver`), never when it
// cannot (the wire dropped it after accepting). Anything stronger is a
// protocol over this, not a promise of it.
export effect Transport {
    // Start receiving at [at]: every frame that arrives there is sent to
    // [sink]. One listener per endpoint; a second `listen` at the same
    // endpoint replaces the first.
    fn listen(at: NodeEndpoint, sink: Addr<Inbound>) -> Ok None | Err NetError => at, !sink
    // Stop receiving at [at]. Frames in flight to it are lost.
    fn unlisten(at: NodeEndpoint) -> None => at
    // Hand [frame] to the wire for [to]. `Ok` means the transport accepted
    // it — for TCP, that it was written to a connected socket — not that
    // the far side has it.
    fn deliver(to: NodeEndpoint, frame: Bytes) -> Ok None | Err NetError => to, !frame
    // The endpoint frames sent by this node carry as their `from`. Set once
    // by the constructor of the host handler; a test double answers the
    // virtual node it stands for.
    fn local_endpoint() -> NodeEndpoint
}

// ----------------------------------------------------------------- wire ----

// [wire-format] The canonical encoding, as a value: every type has a wire
// form unless it is (or holds) something declared `noremote` [noremote], and
// the compiler generates the codec for both backends — so the bytes `encode`
// answers on a Kotlin node are the bytes a Rust node's `decode` reads. What
// the frames of step ③ carry.
//
// Refused at the call, naming the field or declaration that stops it, for a
// type with no wire form; a generic `T` is refused too until the
// instantiation is known.
export intrinsic fn encode<T>(value: T) [] -> Bytes => !value

// [wire-format] The inverse: `None` when the bytes are not a well-formed
// encoding of `T` — truncated, or a union tag out of range. Written as
// `decode<Point>(data)`, since nothing but the type argument says what to
// read.
export intrinsic fn decode<T>(data: Bytes) [] -> T? => data

// -------------------------------------------------------------- routing ----

// [addr-routable] The identity of a node: a process, or one of the virtual
// nodes a process hosts for the in-process double. Minted by the runtime at
// start (`this_node`) or on demand (`new_node`), unique across a node group
// with overwhelming probability, and compared everywhere — a proxy's home,
// a `Node` in a group, the leader an election answers — so it has a wire
// form: an `Addr` that crosses the wire is `(node, actor, bits)`.
export struct NodeId : Hashed<self> by auto {
    id: Long
}

// [addr-routable] The identity of the node this code runs on: every actor
// carries the node it was spawned on.
export intrinsic fn this_node() [] -> NodeId

// [addr-routable] Hosts a fresh **virtual node** in this process: the
// in-process double of another machine. Pools made with [pool_at] belong to
// it, actors spawned on them carry it, and an addr of one that crosses to
// another node — virtual or not — is reached through a proxy, so every remote
// path runs without a socket.
export intrinsic fn new_node() [spawn] -> NodeId

// [addr-routable] A pool of [size] workers belonging to [node].
export intrinsic fn pool_at(node: NodeId, size: Int) [spawn] -> Pool => node, size

// ---- the runtime's side of the wire (private: `connect` is the program's call)
//
// The scheduler does not know how bytes move. It keeps a **routing table**
// (node → endpoint), and when it has a frame for another node — a message to
// a proxy, a reply, a credit, a handshake — it looks the node up and hands
// (endpoint, frame) to whatever the node bound as its **outbound**. When the
// transport delivers a frame that arrived, the **inbound** side hands the
// bytes back and the runtime finds the actor, waiter or proxy they are for.
// These three intrinsics are those bindings; `connect` makes all of them.

// [addr-routable] Records where frames for [node] go. The self-route is
// written by `connect`; routes to peers are learnt in the node group's
// handshake. A frame for a node with no route yet waits until one arrives.
intrinsic fn add_route(node: NodeId, at: NodeEndpoint) [] -> None => node, at

// [addr-routable] Binds the current node's outbound side: every frame the
// runtime wants sent is queued to [out], whose activation calls
// `Transport.deliver` — with the scheduler's lock released, which is why it
// is an actor and not a direct call.
intrinsic fn route_frames(out: Addr<Outbound>) [] -> None => !out

// [addr-routable] Hands one frame that arrived on the wire to the runtime:
// a message into an actor's mailbox, a reply to its waiter, a credit to a
// proxy, a handshake to the node group. Answers whether it was delivered; a
// frame for an unknown target, with mismatched bits or a malformed payload is
// dropped, never delivered wrong.
intrinsic fn deliver_frame(data: Bytes) [] -> Bool => data

// [remote-backpressure] For an addr that crossed the wire, how many more
// messages its mailbox has granted room for; `None` for a local actor. What a
// pick reads as a remote member's load (step ⑤).
export intrinsic fn credits<E>(a: Addr<E>) [] -> Int? => a

// [addr-routable] The outbound protocol: the runtime sends every frame it
// wants on the wire here, and the one handler of it hands them to the
// `Transport` in scope.
export actor effect Outbound {
    send fn send_frame(to: NodeEndpoint, frame: Bytes) => !to, !frame
}

export handler Sending() [Transport] of Outbound {
    mailbox { capacity: 256 }

    send fn send_frame(to: NodeEndpoint, frame: Bytes) => !to, !frame {
        // A frame the wire refuses is lost: at-most-once, as promised.
        let _sent = deliver(to, frame)
    }
}

// [addr-routable] The inbound side: the `Inbound` handler that hands every
// frame to the runtime. `listen(at, spawn Receiving())` is a node's whole
// receive path.
export handler Receiving() of Inbound {
    mailbox { capacity: 256 }

    send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame {
        let _delivered = deliver_frame(frame)
    }
}

// [net-connect] Whether the current node is connected to the wire: its
// outbound side bound by [connect]. What makes `connect` idempotent, and what
// a program asks before it decides to.
export intrinsic fn connected() [] -> Bool

// [net-connect] Connects the current node to the wire, at endpoint [me]:
// the three bindings between the scheduler and the `Transport` in scope,
// which every node needs exactly once before it can reach or be reached by
// another —
//
// 1. **outbound**: a `Sending` actor is spawned and bound with
//    `route_frames`; every frame the runtime wants on the wire (a message to
//    a proxy, a reply, a credit, a HELLO) is queued to it, and its
//    activation calls `Transport.deliver` with the scheduler's lock released;
// 2. **inbound**: a `Receiving` actor is spawned and registered with
//    `listen(me, …)`; the transport sends each frame that arrives at [me] to
//    it, and its activation hands the bytes back to the runtime with
//    `deliver_frame` — a send being the only way work from a host thread
//    enters the scheduler;
// 3. **the self-route**: `add_route(this_node(), me)` records that frames
//    for *this* node go to [me] — routes to peers are learnt in the node
//    group's handshake, but a node's own endpoint is something only it knows.
//
// Answers whether **this call** connected the node: `false` when it was
// already connected, in which case nothing is rebound and no actor is
// spawned. The two actors run on `pool(1)`, or on [on] when given. A program
// calls it when it wants the wire up before any group exists, or wants to
// place the wire actors itself; otherwise a node group's `init` connects the
// node at the transport's own endpoint when nothing has (user decision
// 2026-09-28), so a fresh node needs only its transport and its group.
export fn connect(me: NodeEndpoint) [Transport, spawn] -> Bool => !me {
    return connect(me, pool(1))
}

export fn connect(me: NodeEndpoint, on: Pool) [Transport, spawn] -> Bool => !me, !on {
    if connected() {
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

// [node-group] A member of a node group: the identity the handshake minted and
// the endpoint it answers at. What a mechanism knows *after* contact; a
// `NodeEndpoint` is what it knows before.
export struct Node : Hashed<self> by auto {
    id: NodeId,
    at: NodeEndpoint
}

// [node-group] Membership of **nodes**: one actor effect, whose handlers are
// the mechanisms — a fixed list, gossip over seeds, a heartbeat store. Every
// handler serves this face the same way; what differs is how it learns who is
// there and how it notices who has gone.
export actor effect NodeGroup {
    // Everyone this node currently knows, itself excluded.
    send fn members(out: Reply<List<Node>>) => !out
    // Hear about arrivals and departures.
    send fn subscribe(w: Addr<NodeGroupWatcher>) => !w
    // Tell every peer this node is going, and stop.
    send fn leave()
}

// [node-group] What a subscriber hears. [why] on a departure is `"left"` for
// an announced one, or the mechanism's account of an unannounced one.
export actor effect NodeGroupWatcher {
    send fn joined(n: Node) => !n
    send fn left(n: Node, why: Str) => !n, !why
}

// ---- the handshake (private: a mechanism's tools)
//
// Two nodes meet by exchanging HELLO frames: each carries the group's name,
// the sender's endpoint and its **protocol table** (every actor effect with a
// wire form, and its hash). The runtime answers a HELLO with an ACK, records
// the peer's route and table, and tells the node group's mechanism through
// the `PeerEvents` face it registered. A LEAVE announces departure and kills
// every proxy of an actor on that node; an INTRO carries endpoints one node
// tells another about, which is how gossip converges on a full mesh. A
// mechanism composes these; the frames themselves are the runtime's.

// [node-group] The current node joins group [name], answering at [at]: what
// its HELLO carries and what a peer's HELLO is checked against — two nodes in
// groups of different names refuse each other by name at the handshake.
intrinsic fn set_group(name: Str, at: NodeEndpoint) [] -> None => name, at

// [node-group] Where the current node's peer events go — the runtime's
// account of the handshake, delivered to the mechanism as messages to three
// **private** send members it declares [actor-private-send]: `hello(node, at,
// protocols)` when a peer completed a HELLO/ACK exchange, `gone(node)` when a
// peer sent LEAVE, `introduced(peers)` when a peer introduced others (an INTRO
// frame, sent with [introduce]). Introductions travel as frames rather than
// as an actor protocol so a mechanism never sends the protocol it serves —
// which keeps gossip out of the deadlock graph's send-cycle warning
// [actor-deadlock-cycle]. Called from the mechanism's `init` with
// `self@NodeGroup`; the emitters build the mechanism's own private messages.
intrinsic fn watch_peers(sink: Addr<NodeGroup>) [] -> None => !sink

// [node-group] Tells peer [node] about [peers]: an INTRO frame, which arrives
// there as `PeerEvents.introduced`.
intrinsic fn introduce(node: NodeId, peers: List<NodeEndpoint>) [] -> None => node, peers

// [node-group] The current node's HELLO frame, to be handed to the transport
// directly: a node with no route yet cannot be sent to any other way.
intrinsic fn hello_frame() [] -> Bytes

// [node-group] A LEAVE to every peer the current node knows.
intrinsic fn leave_group() [] -> None

// [protocol-hash] A peer's hash for the protocol named [protocol], as its
// handshake carried it; `None` for an unknown peer or one without the
// protocol. What a replica compares before merging a peer's members.
export intrinsic fn peer_protocol(node: NodeId, protocol: Str) [] -> Str? => node, protocol

// [node-group] Fixed membership: every endpoint is known up front, so the
// mechanism is "say HELLO to each, and report who answers". Death detection
// is the transport's: a LEAVE, or — once step ④'s follow-up lands — a failed
// delivery. The double every later step tests on, over `MemTransport`.
export handler StaticNodeGroup(name: Str, all: List<NodeEndpoint>) [Transport, spawn]
    of NodeGroup {
    mailbox { capacity: 64 }

    known: Mut Map<NodeId, Node> = mut_map_of()
    watchers: Mut List<Addr<NodeGroupWatcher>> = mut_list_of()

    // The mechanism starts itself: `init` is the first activation, and
    // `self@NodeGroup` is where the runtime's peer events go. [net-connect] It
    // connects the node if nothing has yet — the transport it depends on is a
    // handle the two wire actors inherit — at the transport's own endpoint:
    // one source of truth for "where this node is".
    init {
        let me = local_endpoint()
        let _connected = connect(copy(me))
        set_group(copy(name), copy(me))
        watch_peers(self@NodeGroup)
        for e in all {
            if !eq(e, me) {
                let _sent = deliver(copy(e), hello_frame())
            }
        }
    }

    send fn members(out: Reply<List<Node>>) => !out {
        let all_known: Mut List<Node> = mut_list_of()
        for id in keys(known) {
            let n = get(known, id)
            if !(n is None) {
                add(all_known, copy(n))
            }
        }
        out.send(all_known)
    }

    send fn subscribe(w: Addr<NodeGroupWatcher>) => !w {
        add(watchers, w)
    }

    send fn leave() {
        leave_group()
    }

    send fn hello(node: NodeId, at: NodeEndpoint, protocols: List<(Str, Str)>)
        => !node, !at, !protocols {
        if contains_key(known, node) {
            return
        }
        let n = Node { id: copy(node), at: at }
        put(known, node, copy(n))
        for w in watchers {
            w.joined(copy(n))
        }
    }

    send fn gone(node: NodeId) => !node {
        let n = remove(known, node)
        if n is None {
            return
        }
        for w in watchers {
            w.left(copy(n), "left")
        }
    }

    send fn introduced(peers: List<NodeEndpoint>) => !peers {
        // A static group knows its list already; an introduction is news of
        // nothing.
    }
}

// [node-group] Gossip membership: a node knows only some [seeds] to start
// with; every node it meets is told who else it knows, and tells everyone
// else about it, so the group converges on a full mesh from any connected set
// of seeds. Death is a LEAVE (or, once the failure follow-up lands, a failed
// delivery). The mechanism most deployments want when the fleet is not fixed;
// its partition policy is the recorded follow-up.
export handler GossipNodeGroup(name: Str, seeds: List<NodeEndpoint>) [Transport, spawn]
    of NodeGroup {
    mailbox { capacity: 64 }

    known: Mut Map<NodeId, Node> = mut_map_of()
    dialed: Mut Set<Str> = mut_set_of()
    watchers: Mut List<Addr<NodeGroupWatcher>> = mut_list_of()

    init {
        let me = local_endpoint()
        let _connected = connect(copy(me))
        set_group(copy(name), copy(me))
        watch_peers(self@NodeGroup)
        for e in seeds {
            dial(dialed, copy(e))
        }
    }

    send fn members(out: Reply<List<Node>>) => !out {
        let all_known: Mut List<Node> = mut_list_of()
        for id in keys(known) {
            let n = get(known, id)
            if !(n is None) {
                add(all_known, copy(n))
            }
        }
        out.send(all_known)
    }

    send fn subscribe(w: Addr<NodeGroupWatcher>) => !w {
        add(watchers, w)
    }

    send fn leave() {
        leave_group()
    }

    send fn hello(node: NodeId, at: NodeEndpoint, protocols: List<(Str, Str)>)
        => !node, !at, !protocols {
        if contains_key(known, node) {
            return
        }
        // Everyone already known learns of the newcomer, and the newcomer
        // learns of everyone already known.
        let others: Mut List<NodeEndpoint> = mut_list_of()
        for id in keys(known) {
            let n = get(known, id)
            if !(n is None) {
                add(others, copy(n.at))
            }
            introduce(copy(id), [copy(at)])
        }
        introduce(copy(node), others)
        add(dialed, to_str(at))
        let n = Node { id: copy(node), at: at }
        put(known, node, copy(n))
        for w in watchers {
            w.joined(copy(n))
        }
    }

    send fn gone(node: NodeId) => !node {
        let n = remove(known, node)
        if n is None {
            return
        }
        for w in watchers {
            w.left(copy(n), "left")
        }
    }

    send fn introduced(peers: List<NodeEndpoint>) => !peers {
        for e in peers {
            dial(dialed, copy(e))
        }
    }

}

// Says HELLO to an endpoint once; the runtime's handshake does the rest. A
// HELLO the transport could not deliver — the peer's node not on the wire yet —
// is forgotten, so the next introduction of that endpoint dials it again.
fn dial(dialed: Mut Set<Str>, e: NodeEndpoint) [Transport] -> None => dialed: Mut, !e {
    if eq(e, local_endpoint()) || contains(dialed, to_str(e)) {
        return
    }
    add(dialed, to_str(e))
    let sent = deliver(copy(e), hello_frame())
    if sent is Err {
        let _forgot = remove(dialed, to_str(e))
    }
}

// ---------------------------------------------------------- actor group ----

// [protocol-hash] A protocol named as a value: what a replica is told which
// effect a group is of, so a peer whose hash for it differs is refused before
// any message is exchanged. `E` types the value and is erased in the output
// [effect-generic-decl]; the value carries what erasure drops, which is how
// a generic fn (`actor_group<E>`) gets to hold the name and hash of an `E`
// it cannot see.
export struct Protocol<E> {
    name: Str,
    hash: Str
}

// [protocol-hash] [implicit-intrinsic] The protocol [E], as a value. The
// **one** intrinsic that reads its type argument: an effect-only generic is
// erased in the output, so the name and hash of `E` exist only where `E` is
// written concretely, and this is the function the checker finds for an
// implicit `?protocol: () -> Protocol<E>` once the call's `E` is known
// [implicit-resolve] — passed as an adapter closure whose body is the
// literal, exactly as std's `iter` fills an `?Iterable`. A program never
// needs to write `protocol<Ping>()` itself: it writes `actor_group<Ping>(…)`
// and the implicit is filled. Resolving it for an `E` whose payloads have no
// wire form is refused [noremote]: a group of `E` could not span nodes.
export intrinsic fn protocol<E>() [] -> Protocol<E>

// [actor-group] Membership of **actors of one protocol**, across every node
// of a group — the other membership level, beside `NodeGroup`. An
// `ActorGroup<E>` is a set of `Addr<E>`, one replica per node, the replicas
// finding each other by name through the node group's peers and exchanging
// their sets; a member may live on any node, and an addr answered by
// `members` routes there like any addr [addr-routable]. Generic over the
// *effect* [effect-generic-decl]: `ActorGroup<Counter>` and
// `ActorGroup<Ledger>` are distinct types, so a group can only ever hold the
// protocol it is of.
export actor effect ActorGroup<E> {
    // Register [member] here; every replica learns of it.
    send fn join(member: Addr<E>) => !member
    // Withdraw [member].
    send fn leave(member: Addr<E>) => !member
    // Every member known to this replica, across nodes.
    send fn members(out: Reply<List<Addr<E>>>) => !out
    // Hear about arrivals and departures.
    send fn subscribe(w: Addr<ActorGroupWatcher<E>>) => !w
}

// [actor-group] What a subscriber hears.
export actor effect ActorGroupWatcher<E> {
    send fn joined(member: Addr<E>) => !member
    send fn left(member: Addr<E>) => !member
}

// ---- the replica's tools (private)
//
// An actor group has no home: every node that opens it runs a **replica**,
// and replicas find each other by **name** — each publishes its addr under
// the group's name, the runtime sends the names it holds to every peer at the
// handshake (and to every peer when a name is published later), and a replica
// hearing of a same-named peer shares its member set with it. Members travel
// as addresses; nothing is copied or moved.

// [actor-group] Publishes [me] under [name] on the current node and asks to
// hear of every peer node that publishes the same name, and of what those
// peers share, as messages to two **private** members of the replica
// [actor-private-send]: `peer(node)` and `merged(from, found)`. Replicas talk
// through frames rather than through addrs of one another, so the group
// serves `ActorGroup` and never sends it — no send cycle for the deadlock
// graph to warn of. Called from the replica's `init` with `self@ActorGroup<E>`.
intrinsic fn publish_group<E>(name: Str, me: Addr<ActorGroup<E>>) [] -> None => name, !me

// [actor-group] Tells the replica named [name] on peer [node] about [members]:
// a MEMBERS frame, arriving there as the replica's private `merged`.
intrinsic fn share_members<E>(name: Str, node: NodeId, members: List<Addr<E>>) [] -> None
    => name, node, members

// [actor-group] [remote-backpressure] How loaded a member looks from here:
// the queue depth of a local actor, or the messages in flight to a remote one
// that its host has not yet dequeued. What a pick compares (step ⑦).
export intrinsic fn pending<E>(a: Addr<E>) [] -> Int => a

// [actor-group] Opens the group of protocol [E] over [nodes] on this node:
// one replica, spawned on the current pool, published under the group's name
// so the replicas other nodes open under the same name find it — the same
// name from two nodes is one group. Written with the protocol as a type
// argument, `actor_group<Ping>(nodes)`, which is also where a protocol that
// cannot cross a node boundary is refused [noremote]. A peer whose hash of
// the protocol differs is invisible: its members are never merged.
//
// A node group is about **machines** and a program has one; an actor group
// is about the **actors of one protocol** across them, and a program opens
// one per protocol it routes to or lists the members of.
//
// An ordinary fn: the protocol's name and hash are constants of the *written*
// `E`, and an effect-only generic is erased in the output
// [effect-generic-decl], so this body cannot produce them — the implicit
// [protocol] is filled at the call, where `E` is concrete, with the one
// intrinsic that reads its type argument [protocol-hash] [implicit-resolve].
// A generic caller forwards its own `?protocol` [implicit-forward], as with
// any implicit.
export fn actor_group<E>(nodes: Addr<NodeGroup>, ?protocol: () -> Protocol<E>) [spawn] -> Addr<ActorGroup<E>>
    => !nodes {
    let proto = protocol()
    let name = copy(proto.name)
    return actor_group<E>(name, nodes)
}

// [actor-group] Several groups of one protocol, told apart by [name]. The
// subscription to the node group is sent from here rather than from the
// replica's `init`: sent by the replica it would be a `NodeGroupWatcher →
// NodeGroup → NodeGroupWatcher` edge in the deadlock graph
// [actor-deadlock-cycle] — a real one, since the node group sends `joined`
// back — and sent from the opener's frame it is not.
export fn actor_group<E>(name: Str, nodes: Addr<NodeGroup>, ?protocol: () -> Protocol<E>) [spawn] -> Addr<ActorGroup<E>>
    => !name, !nodes {
    let (group, watcher) = spawn ActorGrouping<E>(name, protocol()) on pool(1)
    nodes.subscribe(watcher)
    return group
}

// [actor-group] Registers [member] in [group]. `spawn Pinging() on p in pings`
// is the same thing written at the spawn.
export fn join<E>(group: Addr<ActorGroup<E>>, member: Addr<E>) [] -> None => group, !member {
    group.join(member)
}

// [actor-group] The std replica. State: members as a list of addrs (an addr
// has no `hash` yet, so membership is checked by `eq`), the peer replicas, the
// subscribers. Its second face hears the node group: a node's departure withdraws every member it
// hosted. [route-stub] After every change it mirrors the member set into the
// runtime (`view_set`), which is what a `route(group)` stub reads on the
// sender's thread — the replica is behind the view by one message, never in
// the send path.
export handler ActorGrouping<E>(name: Str, proto: Protocol<E>) [spawn]
    of ActorGroup<E>, NodeGroupWatcher {
    mailbox { capacity: 64 }

    all: Mut List<Addr<E>> = mut_list_of()
    peers: Mut List<NodeId> = mut_list_of()
    watchers: Mut List<Addr<ActorGroupWatcher<E>>> = mut_list_of()

    // The replica starts itself: it publishes its `ActorGroup<E>` face by
    // name, so the replicas other nodes open under the same name find it.
    init {
        publish_group(copy(name), self@ActorGroup<E>)
    }

    send fn join(member: Addr<E>) => !member {
        if !admit(all, copy(member)) {
            return
        }
        mirror(self@ActorGroup<E>, all)
        for w in watchers {
            w.joined(copy(member))
        }
        for p in peers {
            share_members(copy(name), copy(p), [copy(member)])
        }
    }

    send fn leave(member: Addr<E>) => !member {
        if !withdraw(all, copy(member)) {
            return
        }
        mirror(self@ActorGroup<E>, all)
        for w in watchers {
            w.left(copy(member))
        }
    }

    send fn joined(n: Node) => !n {}

    // [actor-group] A node that left took its members with it: withdraw
    // every addr it hosted, telling the subscribers. The runtime has already
    // killed the proxies, so a send to one is the silent no-op.
    send fn left(n: Node, why: Str) => !n, !why {
        let gone: Mut List<Addr<E>> = mut_list_of()
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

    send fn members(out: Reply<List<Addr<E>>>) => !out {
        out.send(copy(all))
    }

    send fn subscribe(w: Addr<ActorGroupWatcher<E>>) => !w {
        add(watchers, w)
    }

    send fn peer(node: NodeId) => !node {
        // [protocol-hash] A peer whose version of the protocol differs is
        // invisible: its members are never merged, and it never hears ours.
        let theirs = peer_protocol(copy(node), copy(proto.name))
        if theirs is None || !eq(theirs, proto.hash) {
            return
        }
        if contains_node(peers, copy(node)) {
            return
        }
        add(peers, copy(node))
        share_members(copy(name), node, copy(all))
    }

    send fn merged(from: NodeId, found: List<Addr<E>>) => !from, !found {
        if !contains_node(peers, copy(from)) {
            let theirs = peer_protocol(copy(from), copy(proto.name))
            if theirs is None || !eq(theirs, proto.hash) {
                return
            }
            add(peers, from)
        }
        let changed = false
        for m in found {
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

// [route-stub] Mirrors [members] into the runtime's view of [group].
fn mirror<E>(group: Addr<ActorGroup<E>>, members: List<Addr<E>>) [] -> None => group, members {
    view_set(copy(group), copy(members))
}

// Adds an addr to a list unless it is already there; answers whether it was
// added. Linear scan: a group's member set is small.
fn admit<E>(list: Mut List<Addr<E>>, a: Addr<E>) [] -> Bool => list: Mut, !a {
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

fn withdraw<E>(list: Mut List<Addr<E>>, a: Addr<E>) [] -> Bool => list: Mut, !a {
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

// [addr-routable] The node an addr lives on — for a log line, and for the
// group's peer check.
export intrinsic fn node_of<E>(a: Addr<E>) [] -> NodeId => a

// ---------------------------------------------------------- the pick kit ----

// A pick runs on the sender's thread at every send, so it cannot *ask* the
// replica (a hop per send is what the stub exists to avoid). It reads a
// **mirror** the replica keeps in the runtime, written after every change:
// behind the replica by one message, never in the send path.

// [route-stub] (private) The mirror: written by the replica, read by a
// `route(group)` stub per send. A group with no replica on this node has an
// empty view, so its stub parks.
intrinsic fn view_set<E>(group: Addr<ActorGroup<E>>, members: List<Addr<E>>) [] -> None
    => !group, !members
intrinsic fn view_members<E>(group: Addr<ActorGroup<E>>) [] -> List<Addr<E>> => group

// [route-stub] One member as a pick sees it: where it is, how loaded it
// looks from here ([pending]), whether it is on this node.
export struct ActorView<E> {
    addr: Addr<E>,
    pending: Int,
    local: Bool
}

// [route-stub] What a pick chooses from: every member the local replica
// knows, in a stable order (by node, then actor — the same on every node),
// and the hash of the send's `Key` argument when the protocol marks one.
export struct ActorGroupView<E> {
    actors: List<ActorView<E>>,
    key: Long? = None
}

// [route-stub] The policy behind a `route(group)` stub: which member takes
// this send. `None` means nothing is eligible yet — the stub parks the send
// and asks again — which is how an empty group, a full group and a group
// with no leader all wait rather than fail. std ships [LeastLoaded],
// [Sharded] and [Elected]; a policy of your own is a handler of this.
export effect Pick<E> {
    fn choose(view: ActorGroupView<E>) -> Addr<E>? => !view
}

// [route-stub] Marks the parameter of a `send fn` whose value decides the
// member: `send fn reserve(sku: Key Str, ...)`. A claim about the handle,
// erased like any qualifier [qual-erasure]; the stub hashes the argument
// into `ActorGroupView.key`. At most one per member.
export provenance qualifier Key<T> of T

// [route-stub] The canonical hash of a key argument — over its wire encoding,
// so both backends agree on the member a key lands on. Exported for the
// generated `route` stubs, which call it from the program's own module; a
// program has no reason to.
export intrinsic fn key_hash<T>(k: T) [] -> Long => k

// [route-stub] (private) Yields the sender's thread briefly; what a stub does
// between two picks that answered `None`.
intrinsic fn park_briefly() [] -> None

// [route-stub] The stub's whole send path: build the view, ask the policy,
// park until it answers. Exported for the generated `route` stubs, which call
// one of these two per member from the program's own module.
export fn route_to<E>(group: Addr<ActorGroup<E>>) [Pick<E>] -> Addr<E> => group {
    return route_keyed(group, None)
}

export fn route_to<E>(group: Addr<ActorGroup<E>>, key: Long) [Pick<E>] -> Addr<E> => group, key {
    return route_keyed(group, key)
}

fn route_keyed<E>(group: Addr<ActorGroup<E>>, key: Long?) [Pick<E>] -> Addr<E> => group, key {
    while true {
        let members = view_members(copy(group))
        let actors: Mut List<ActorView<E>> = mut_list_of()
        for m in members {
            add(actors, ActorView { addr: copy(m), pending: pending(copy(m)), local: eq(node_of(m), this_node()) })
        }
        let picked = choose(ActorGroupView { actors: copy(actors), key: copy(key) })
        if !(picked is None) {
            return picked
        }
        park_briefly()
    }
    // Unreachable: the loop returns or parks.
    return route_keyed(group, key)
}

// [route-stub] The default policy: the least loaded member, local members
// first when [prefer_local] — a hop within the node is cheaper than one
// across the wire. `None` for an empty view.
export handler LeastLoaded<E>(prefer_local: Bool) of Pick<E> {
    fn choose(view: ActorGroupView<E>) -> Addr<E>? => !view {
        let best: ActorView<E>? = None
        for a in view.actors {
            if best is None {
                best = copy(a)
            } else {
                let b: ActorView<E> = best
                // A local member beats a remote one when locality is preferred;
                // otherwise, or between two of the same locality, the lighter
                // queue wins.
                let take = when {
                    prefer_local && a.local && !b.local { true }
                    prefer_local && !a.local && b.local { false }
                    else { a.pending < b.pending }
                }
                if take {
                    best = copy(a)
                }
            }
        }
        let chosen: ActorView<E> = best ?: return None
        return copy(chosen.addr)
    }
}

// [route-stub] The member that owns the send's key: the view's members are in
// a stable order on every node, so `key mod n` lands on the same member
// everywhere. A member joining reshuffles keys (consistent hashing is a
// recorded follow-up). A send with no key goes to the first member.
export handler Sharded<E>() of Pick<E> {
    fn choose(view: ActorGroupView<E>) -> Addr<E>? => !view {
        let n = size(view.actors)
        if n == 0 {
            return None
        }
        let k = view.key ?: 0L
        let magnitude = if k < 0L { 0L - k } else { k }
        let slot = to_int(magnitude % to_long(n))
        let picked = get(view.actors, slot) ?: return None
        return copy(picked.addr)
    }
}

// [route-stub] Who leads: `None` while an election is in progress. A Salvo
// election or a platform handler over a lease store serve it alike; std ships
// [StaticLeader] for a fixed one.
export effect Leader {
    fn leader() -> NodeId?
}

// [route-stub] The leader fixed by configuration — for a test, or a
// deployment where one node is the primary by decree.
export handler StaticLeader(node: NodeId) of Leader {
    fn leader() -> NodeId? {
        return copy(node)
    }
}

// [route-stub] The member the current leader hosts; parks while there is no
// leader or the leader hosts no member yet.
export handler Elected<E>() [Leader] of Pick<E> {
    fn choose(view: ActorGroupView<E>) -> Addr<E>? => !view {
        let l = leader() ?: return None
        for a in view.actors {
            if eq(node_of(a.addr), l) {
                return copy(a.addr)
            }
        }
        return None
    }
}

// ---------------------------------------------------------------- host ----

// The machine's transport: TCP, one connection per peer, length-prefixed
// frames. The host class lives in std's `platform/net/` tree, one file per
// backend [platform-tree].
//
// `threadsafe` [threadsafe-platform]: a transport is reached from every pool
// of the program at once, so the host synchronizes its own connection table
// and the compiler shares the instance with no lock. Signed in the host
// files. [bind] is the endpoint this node listens on and reports as its
// `local_endpoint`.
export threadsafe platform handler HostTcpTransport(bind: NodeEndpoint) of Transport

// ------------------------------------------------------------------ mem ----

// The in-process double, in two pieces that mirror what a real network is:
// **one network** many nodes share, and **one transport per node** over it.
//
// [MemNet] is the network — an actor holding the listener table and the
// faults a test scripts (a partition, a dead node), so the whole test has one
// place to say "a cannot reach b". [MemTransport] is a node's transport: a
// stateless handler holding the network's addr and the endpoint it stands
// for, whose `deliver` asks the network where the frame goes and sends it
// there. One `MemNetwork` is spawned per test; one `MemTransport` is bound
// per virtual node, exactly as one `HostTcpTransport` is bound per process.
//
// It **runs the same code** the real transport does above it: the same
// `Inbound` actors, the same frames, the same `Err`s. What it does not do is
// serialize — a frame is delivered as the `Bytes` value that was sent, so a
// codec bug is invisible here until step ② adds the encode/decode pair on
// both sides of `deliver`.
//
// (Why two pieces: a handler of *several plain effects* cannot yet be shared
// stateful — one lock behind several faces has no backend form — so the
// fault controls could not ride on `MemTransport` as a second face the way
// `ManualTime` carries `TimerCtl`. An actor face costs nothing, and the split
// reads better anyway: faults belong to the network, not to one node.)
export actor effect MemNet {
    // Register [sink] as the listener at [at], replacing any previous one
    // and reviving a killed node.
    send fn attach(at: NodeEndpoint, sink: Addr<Inbound>) => !at, !sink
    // Drop the listener at [at].
    send fn detach(at: NodeEndpoint) => !at
    // Where a frame from [from] to [to] goes: the listener's addr, or `None`
    // when [to] is unreachable from [from] — partitioned, killed, or nobody
    // listening. Answering `Some` counts the delivery.
    send fn route(from: NodeEndpoint, to: NodeEndpoint, out: Reply<Addr<Inbound>?>) => !from, !to, !out
    // Sever [a] from [b] in both directions until `heal`.
    send fn partition(a: NodeEndpoint, b: NodeEndpoint) => !a, !b
    // Undo `partition(a, b)`.
    send fn heal(a: NodeEndpoint, b: NodeEndpoint) => !a, !b
    // Take [node] off the network: its listener is dropped and every frame to
    // it is unreachable until something `attach`es there again.
    send fn kill(node: NodeEndpoint) => !node
    // How many frames have been routed so far — the assertion a test wants
    // after a partition ("nothing got through").
    send fn delivered(out: Reply<Int>) => !out
}

export handler MemNetwork() of MemNet {
    mailbox { capacity: 64 }

    listeners: Mut Map<NodeEndpoint, Addr<Inbound>> = mut_map_of()
    cuts: Mut Set<Str> = mut_set_of()
    dead: Mut Set<NodeEndpoint> = mut_set_of()
    count: Int = 0

    send fn attach(at: NodeEndpoint, sink: Addr<Inbound>) => !at, !sink {
        remove(dead, at)
        put(listeners, at, sink)
    }

    send fn detach(at: NodeEndpoint) => !at {
        remove(listeners, at)
    }

    send fn route(from: NodeEndpoint, to: NodeEndpoint, out: Reply<Addr<Inbound>?>) => !from, !to, !out {
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

// One virtual node's transport over a shared [MemNet]. Stateless — the
// network holds everything — so a `use` binds it bare and shareable, and it
// crosses into any actor that needs the wire [effect-handle].
//
// `deliver` waits on the network for the route: a `waitfor` on the caller's
// thread, which serves the caller's pool meanwhile [waitfor-pump], so a node
// delivering from inside an activation does not wedge its own pool.
export handler MemTransport(me: NodeEndpoint, net: Addr<MemNet>) of Transport {
    fn listen(at: NodeEndpoint, sink: Addr<Inbound>) -> Ok None | Err NetError => at, !sink {
        net.attach(copy(at), sink)
        return ok(None)
    }

    fn unlisten(at: NodeEndpoint) -> None => at {
        net.detach(copy(at))
    }

    fn deliver(to: NodeEndpoint, frame: Bytes) -> Ok None | Err NetError => to, !frame {
        let sink = waitfor out: Reply<Addr<Inbound>?> { net.route(copy(me), copy(to), out) }
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

