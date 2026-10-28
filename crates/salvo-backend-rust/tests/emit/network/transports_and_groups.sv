// The network sequence: a host transport behind the effect's handle, and a
// node group's protocol table [platform-threadsafe] [node-group].

//--- the_tcp_transport_on_localhost
// [platform-threadsafe] The real wire, on localhost: `HostTcpTransport` is
// std's first `threadsafe` platform handler, so this is the `Arc`-shared,
// `&self`-membered host compiled and run under rustc — and the host→scheduler
// door for real: a frame read on a socket thread becomes an activation
// through `__Stub_Inbound`. One smoke test per backend; everything else about
// the network runs on `MemTransport`. The port is this backend's own so the
// two suites can run at once.
import std.net
import std.time

actor effect Seen {
    send fn wait_one(out: Reply<Str>) => !out
}

// Records the one frame it expects and answers whoever asked for it —
// ordering by data dependency, since the frame arrives on a socket thread.
handler Echoing() of Inbound, Seen {
    mailbox { capacity: 16 }
    state {
        got: Str = ""
        waiting: Mut List<Reply<Str>> = Mut List()
    }

    send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame {
        got = "${to_str(from)}: ${str_of_bytes(frame) ?: "?"}"
        let text = copy(got)
        drain(waiting, r -> send(r, copy(text)))
        waiting = Mut List()
    }

    send fn wait_one(out: Reply<Str>) => !out {
        if got == "" {
            add(waiting, out)
            return
        }
        send(out, copy(got))
    }
}

fn report(label: Str, outcome: Ok None | Err NetError) [Console] -> None => label, outcome {
    when outcome {
        is Ok { println("${label}: sent") }
        is Err {
            let e: NetError = outcome
            println("${label}: ${to_str(e)}")
        }
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let me = NodeEndpoint { host: "127.0.0.1", port: 47311 }
    let nobody = NodeEndpoint { host: "127.0.0.1", port: 47312 }
    // The host behind the effect's handle.
    //~ emits(main.rs): = crate::std_net::__Platform_HostTcpTransport::new((me).clone());
    //~ emits(main.rs): = crate::std_net::Transport::shared(__use_
    use HostTcpTransport(copy(me))
    let workers = pool(2)
    let (echo, seen) = spawn Echoing() on workers
    report("listen", listen(copy(me), echo))
    report("void", deliver(copy(nobody), to_bytes("nobody home")))
    report("hello", deliver(copy(me), to_bytes("hello over tcp")))
    let frame = waitfor out: Reply<Str> { seen.wait_one(out) }
    println("me <- ${frame}")
    println("local ${to_str(local_endpoint())}")
}

//--- node_groups_over_mem_transport
// [node-group] [actor-protocol-hash] Two virtual nodes form a `StaticNodeGroup`
// over `MemTransport`: the HELLO/ACK handshake, the protocol table (B knows
// A's `Counter` hash), a subscriber hearing the join, a LEAVE heard as a
// departure. Then three fresh nodes on a second network, each seeded with
// the first only, converge on a full mesh through `GossipNodeGroup`'s
// introductions. Byte-identical on both backends.
import std.net
import std.time

handler Announcing(label: Str) [Console] of NodeGroupWatcher {
    mailbox { capacity: 32 }
    send fn joined(n: Node) => !n { println("${label}: + ${to_str(n.at)}") }
    send fn left(n: Node, why: Str) => !n, !why { println("${label}: - ${to_str(n.at)} (${why})") }
}

actor effect Boot {
    send fn boot(done: Reply<eff[NodeGroup]>) => !done
}

// Brings up node B from a pool of its own: transport, wire, listener, and its
// group over the same endpoint list.
handler Booting(at: NodeEndpoint, all: List<NodeEndpoint>, net: eff[MemNet], gossip: Bool) [Transport, spawn] of Boot {
    mailbox { capacity: 1 }
    send fn boot(done: Reply<eff[NodeGroup]>) => !done {
        let p = pool(1)
        if gossip {
            let group = spawn GossipNodeGroup("mesh", copy(all)) on p
            done.send(group)
            return
        }
        let group = spawn StaticNodeGroup("demo", copy(all)) on p
        done.send(group)
    }
}

// main registers the program's protocol table.
//~ emits(main.rs): crate::scheduler::salvo_set_protocols(vec![(
fn main() [use, spawn] {
    use StdConsole()
    let a = NodeEndpoint { host: "a", port: 1 }
    let b = NodeEndpoint { host: "b", port: 1 }
    let all = List(copy(a), copy(b))
    let net = spawn MemNetwork() on pool(1)

    // Node A: this one.
    use MemTransport(copy(a), copy(net))
    let pa = pool(1)
    let _connected = connect(copy(a), copy(pa))
    let group_a = spawn StaticNodeGroup("demo", copy(all)) on pa
    group_a.subscribe(spawn Announcing("a") on pa)

    // Node B: hosted beside it.
    let pb = pool_at(new_node(), 1)
    let booter = spawn Booting(copy(b), copy(all), copy(net), false) with MemTransport(copy(b), copy(net)) on pb
    let group_b = waitfor done: Reply<eff[NodeGroup]> { booter.boot(done) }

    // Give the handshake a moment, then read both views.
    let timer = spawn DefaultTimer() on pool(1)
    let _t = waitfor f: Reply<Fired> { timer.after(millis(400), f) }
    let seen_a = waitfor out: Reply<List<Node>> { group_a.members(out) }
    let seen_b = waitfor out: Reply<List<Node>> { group_b.members(out) }
    println("a sees ${size(seen_a)}: ${names_of(seen_a)}")
    println("b sees ${size(seen_b)}: ${names_of(seen_b)}")
    let first_b = get(seen_b, 0)
    if !(first_b is None) {
        println("b knows a's Counter protocol: ${!(peer_protocol(first_b.id, "Counter") is None)}")
    }

    // B leaves; A hears it.
    group_b.leave()
    let _t2 = waitfor f: Reply<Fired> { timer.after(millis(300), f) }
    let after = waitfor out: Reply<List<Node>> { group_a.members(out) }
    println("a sees ${size(after)} after b left")

    // Gossip: three fresh nodes on a second network, each seeded with the
    // first only, converge on a full mesh through introductions.
    let net2 = spawn MemNetwork() on pool(1)
    let x = NodeEndpoint { host: "x", port: 2 }
    let y = NodeEndpoint { host: "y", port: 2 }
    let z = NodeEndpoint { host: "z", port: 2 }
    let gx = boot_node(copy(x), List(copy(x)), copy(net2))
    let gy = boot_node(copy(y), List(copy(x)), copy(net2))
    let gz = boot_node(copy(z), List(copy(x)), copy(net2))
    // Converging takes a few gossip rounds, and how long those take depends on
    // the machine's load: poll, bounded, rather than sleep a fixed time (a
    // fixed 500ms failed under a fully loaded fresh run, 2026-09-29).
    let _t3 = waitfor f: Reply<Fired> { timer.after(millis(200), f) }
    let sx = waitfor out: Reply<List<Node>> { gx.members(out) }
    let sy = waitfor out: Reply<List<Node>> { gy.members(out) }
    let sz = waitfor out: Reply<List<Node>> { gz.members(out) }
    let tries = 0
    while (size(sx) < 2 || size(sy) < 2 || size(sz) < 2) && tries < 100 {
        let _t4 = waitfor f: Reply<Fired> { timer.after(millis(100), f) }
        sx = waitfor out: Reply<List<Node>> { gx.members(out) }
        sy = waitfor out: Reply<List<Node>> { gy.members(out) }
        sz = waitfor out: Reply<List<Node>> { gz.members(out) }
        tries = tries + 1
    }
    println("mesh: x sees ${size(sx)}, y sees ${size(sy)}, z sees ${size(sz)}")
}

fn boot_node(at: NodeEndpoint, seeds: List<NodeEndpoint>, net: eff[MemNet]) [spawn] -> eff[NodeGroup] => !at, !seeds, !net {
    let p = pool_at(new_node(), 1)
    let booter = spawn Booting(copy(at), seeds, copy(net), true) with MemTransport(at, net) on p
    let group = waitfor done: Reply<eff[NodeGroup]> { booter.boot(done) }
    // A group's `init` connects its node; `members` is answered after it, so
    // the next node's seeds are on the wire before it dials them.
    let _started = waitfor out: Reply<List<Node>> { group.members(out) }
    return group
}

actor effect Counter { send fn bump(n: Int) => !n }

fn names_of(nodes: List<Node>) -> Str => nodes {
    let out: Mut Str = Mut Str()
    for n in nodes {
        append(out, to_str(n.at))
    }
    return to_str(out)
}
