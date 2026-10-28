// the network sequence, step ③: routable addrs across two nodes
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- two_nodes_over_mem_transport
import std.net
import std.codec
import std.time

actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 2 }
    state {
        sum: Int = 0
    }
    send fn bump(n: Int) => !n { sum = sum + n }
    send fn total(out: Reply<Int>) => !out { out.send(sum) }
}

// Brings the node this runs on up over the shared in-process network: its
// transport, its outbound and inbound actors, and its route. Placement decides
// the node: called from `main` it sets up `main`'s node; run inside an actor
// spawned on another node's pool, it sets up that node.
fn bring_up(at: NodeEndpoint, net: eff[MemNet]) [Transport, spawn] -> None => at, net {
    let p = pool(1)                         // inherits this node
    let _connected = connect(copy(at), copy(p))
}

actor effect Boot {
    send fn boot(at: NodeEndpoint, net: eff[MemNet], done: Reply<Bool>) => !at, !net, !done
}

handler Booting() [Transport, spawn] of Boot {
    mailbox { capacity: 1 }
    send fn boot(at: NodeEndpoint, net: eff[MemNet], done: Reply<Bool>) => !at, !net, !done {
        bring_up(at, net)
        done.send(true)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let a = NodeEndpoint { host: "a", port: 1 }
    let b = NodeEndpoint { host: "b", port: 1 }
    let net = spawn MemNetwork() on pool(1)

    // Node A is this one; node B is hosted beside it, and booted from a pool
    // of its own so `this_node()` there answers B.
    use MemTransport(copy(a), copy(net))
    bring_up(copy(a), copy(net))
    let pb = pool_at(new_node(), 1)
    let booter = spawn Booting() with MemTransport(copy(b), copy(net)) on pb
    let _up = waitfor done: Reply<Bool> { booter.boot(copy(b), copy(net), done) }

    // A counter living on node B, reached from node A through the wire: the
    // addr crosses as bytes and comes back as a proxy.
    let there = spawn Counting() on pb
    let far = decode<eff[Counter]>(encode(copy(there)))
    if far is None {
        println("addr did not decode")
        return
    }
    println("is proxy: ${!(credits(far) is None)}")
    let timer = spawn DefaultTimer() on pool(1)
    let _t = waitfor f: Reply<Fired> { timer.after(millis(300), f) }
    println("credits after a beat: ${credits(far) ?: -1}")
    for i in range(0, 5) {
        far.bump(i)                         // 0+1+2+3+4 = 10, through a capacity-2 mailbox
    }
    let sum = waitfor out: Reply<Int> { far.total(out) }
    println("total over the wire: ${sum}")
    let direct = waitfor out: Reply<Int> { there.total(out) }
    println("direct: ${direct}")
}
//= stdout
//= is proxy: true
//= credits after a beat: 2
//= total over the wire: 10
//= direct: 10
