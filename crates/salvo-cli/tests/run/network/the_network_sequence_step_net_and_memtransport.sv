// the network sequence, step ①: `net` and `MemTransport`
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// The `MemTransport` double end to end under rustc.
//--- the_mem_transport
import std.net

actor effect Counting {
    send fn total(out: Reply<Str>) => !out
}

// The receiver records rather than prints: a `deliver` answers as soon as
// the frame is handed over, so a print here would race main's own.
handler Tallying(label: Str) of Inbound, Counting {
    mailbox { capacity: 16 }
    state {
        seen: Mut List<Str> = Mut List<Str>()
    }

    send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame {
        seen.add("${label} <- ${to_str(from)}: ${str_of_bytes(frame) ?: "?"}")
    }

    send fn total(out: Reply<Str>) => !out {
        out.send(join(seen, ", "))
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
    let a = NodeEndpoint { host: "a", port: 1 }
    let b = NodeEndpoint { host: "b", port: 1 }
    let c = NodeEndpoint { host: "c", port: 9 }
    let workers = pool(2)
    let net = spawn MemNetwork() on workers
    use MemTransport(copy(a), copy(net))

    let (in_a, _count_a) = spawn Tallying("a") on workers
    let (in_b, count_b) = spawn Tallying("b") on workers
    report("listen a", listen(copy(a), in_a))
    report("listen b", listen(copy(b), in_b))

    report("hello", deliver(copy(b), to_bytes("hello")))
    net.partition(copy(a), copy(b))
    report("lost", deliver(copy(b), to_bytes("lost")))
    net.heal(copy(a), copy(b))
    report("again", deliver(copy(b), to_bytes("again")))
    report("void", deliver(copy(c), to_bytes("void")))
    net.kill(copy(b))
    report("dead", deliver(copy(b), to_bytes("dead")))

    let got = waitfor out: Reply<Str> { count_b.total(out) }
    let routed = waitfor out: Reply<Int> { net.delivered(out) }
    println("b saw [${got}], routed ${routed}")
}
//= stdout
//= listen a: sent
//= listen b: sent
//= hello: sent
//= lost: unreachable: b:1
//= again: sent
//= void: unreachable: c:9
//= dead: unreachable: b:1
//= b saw [b <- a:1: hello, b <- a:1: again], routed 2

//--- the_tcp_transport_on_localhost
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
//= stdout
//= listen: sent
//= void: unreachable: 127.0.0.1:47312
//= hello: sent
//= me <- 127.0.0.1:47311: hello over tcp
//= local 127.0.0.1:47311
