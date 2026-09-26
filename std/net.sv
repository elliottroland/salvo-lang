// `net`: the wire under actors across machines — step ① of the network
// sequence (ROADMAP.md section 2; user decisions 2026-09-26). Rules:
// [net-transport] (the effect and `Inbound`), [net-host] (`HostTcpTransport`),
// [net-mem] (`MemNet`/`MemNetwork`/`MemTransport`), [wire-format] and
// [noremote] (`encode`/`decode`; the encoding itself is fixed in salvo-core's
// `wire.rs` and implemented in each backend's `wire` runtime), [addr-routable]
// [addr-capability] [remote-backpressure] (the routing surface: `this_node`,
// `new_node`, `pool_at`, `add_route`, `route_frames`, `deliver_frame`,
// `credits`, `Outbound`/`Sending`, `Receiving`).
//
// The design's one-line shape is that **the network enters at the actor
// group, never at the spawn**: every actor is spawned by the node that hosts
// it, and what crosses the wire is addresses and messages. This module is the
// bottom of that stack and knows nothing about groups, nodes or protocols. It
// moves **frames** — opaque `Bytes` — between **endpoints**, and delivers what
// arrives into the scheduler as an ordinary message. Everything above it
// (`NodeGroup`, `ActorGroup<E>`, the codecs) is Salvo written over this.
//
// Two halves, deliberately asymmetric:
//
// * **Outbound** is a plain effect, [Transport]: `deliver(to, frame)` is a
//   call the sender makes on its own thread, and it answers whether the frame
//   was handed to the wire.
// * **Inbound** is an *actor* effect, [Inbound]: bytes arrive on a thread the
//   host owns (a socket reader), and the only way work from a foreign thread
//   enters the scheduler is a send. So a node that wants to receive spawns an
//   actor serving `Inbound` and registers its addr with `listen`; the
//   transport sends every arriving frame to it. That is the same host→runtime
//   upcall the timer makes when a deadline fires [time-timer], and it needs
//   nothing the language does not already have: the host holds an
//   `Addr<Inbound>` and calls the generated forwarding stub on it.
//
// The platform owns the transport only (user decision 2026-09-26, N-4):
// [HostTcpTransport] is the host class, declared `threadsafe` because a
// transport is called from every pool [threadsafe-platform]; [MemTransport] is
// the pure-Salvo double every later step of the sequence tests on — N virtual
// endpoints in one process, with the faults a test wants to script.

import time.Duration

// Where a transport can dial: known **before** any contact, which is what
// makes it the currency of node discovery. For TCP it is a host and a port;
// for [MemTransport] it is a virtual node's name. Printable, hashable and
// orderable, so it keys maps and sorts in a membership list.
//
// Distinct from a `Node`, which the network sequence's step ④ mints *after*
// a handshake and which carries an identity and a build — an endpoint says
// nothing about who answers there.
export struct NodeEndpoint : auto Ordered<self>, auto Hashed<self> {
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
    send fn frame(from: NodeEndpoint, data: Bytes) => !from, !data
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

// [addr-routable] The identity of the node this code runs on. A node is a
// process, or one of the virtual nodes a process hosts for the in-process
// double; every actor carries the node it was spawned on, and an `Addr` that
// crosses the wire is `(node, actor, bits)`.
export intrinsic fn this_node() [] -> Long

// [addr-routable] Hosts a fresh **virtual node** in this process: the
// in-process double of another machine. Pools made with [pool_at] belong to
// it, actors spawned on them carry it, and an addr of one that crosses to
// another node — virtual or not — is reached through a proxy, so every remote
// path runs without a socket.
export intrinsic fn new_node() [spawn] -> Long

// [addr-routable] A pool of [size] workers belonging to [node].
export intrinsic fn pool_at(node: Long, size: Int) [spawn] -> Pool => node, size

// [addr-routable] Where frames for [node] go. Learnt in the handshake by a
// `NodeGroup` (step ④); until a node has a route its frames wait.
export intrinsic fn add_route(node: Long, at: NodeEndpoint) [] -> None => node, at

// [addr-routable] Binds the current node's outbound side: every frame the
// runtime sends is delivered to [out], which puts it on the wire.
export intrinsic fn route_frames(out: Addr<Outbound>) [] -> None => !out

// [addr-routable] Hands one frame that arrived on the wire to the runtime:
// a message into an actor's mailbox, a reply to its waiter, a credit to a
// proxy. Answers whether it was delivered; a frame for an unknown target, with
// mismatched bits or a malformed payload is dropped, never delivered wrong.
export intrinsic fn deliver_frame(data: Bytes) [] -> Bool => data

// [remote-backpressure] For an addr that crossed the wire, how many more
// messages its mailbox has granted room for; `None` for a local actor. What a
// pick reads as a remote member's load (step ⑤).
export intrinsic fn credits<E>(a: Addr<E>) [] -> Int? => a

// [addr-routable] The outbound protocol: the runtime sends every frame it
// wants on the wire here, and the one handler of it hands them to the
// `Transport` in scope.
export actor effect Outbound {
    send fn frame(to: NodeEndpoint, data: Bytes) => !to, !data
}

export handler Sending() [Transport] of Outbound {
    mailbox { capacity: 256 }

    send fn frame(to: NodeEndpoint, data: Bytes) => !to, !data {
        // A frame the wire refuses is lost: at-most-once, as promised.
        let _sent = deliver(to, data)
    }
}

// [addr-routable] The inbound side: the `Inbound` handler that hands every
// frame to the runtime. `listen(at, spawn Receiving())` is a node's whole
// receive path.
export handler Receiving() of Inbound {
    mailbox { capacity: 256 }

    send fn frame(from: NodeEndpoint, data: Bytes) => !from, !data {
        let _delivered = deliver_frame(data)
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
// crosses into any actor that needs the wire [use-local].
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
        sink.frame(copy(me), frame)
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

