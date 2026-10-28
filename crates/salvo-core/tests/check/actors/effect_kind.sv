// [actor-effect-kind] [actor-sendable] The effect kind is declared, not
// diagnosed (user decision 2026-09-15, EU-5): an author choosing between
// `effect` and `actor effect` is deciding whether the protocol crosses
// threads.

actor effect Counter {
    send fn bump(n: Int)
    send fn total(out: Reply<Int>) => !out
}

actor effect Log {
    send fn note(what: Str) => !what
}

handler Printing() of Log {
    mailbox { capacity: 4 }

    send fn note(what: Str) {}
}

handler Counting() [Log] of Counter {
    mailbox { capacity: 16 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
        note("bumped")
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}


// `send fn` needs the actor kind, and the diagnostic names the marker.
effect Plain {
    //~ error[actor-send-fn:plain-effect]
    send fn nope(n: Int)
}

// And inside an `actor effect`, everything a seam cannot carry is refused
// **at the declaration**, where the choice is being made: a kept parameter, a
// `Mut` parameter, and a non-sendable payload, each naming the law rather
// than the symptom.
struct Job {
    name: Str,
    run: () -> Int
}

actor effect Bad {
    //~ error[actor-sendable:kept-payload]
    send fn keeps(s: Str) => s
    //~ error[actor-sendable:mut-payload]
    send fn mutates(xs: Mut List<Int>) => !xs
    //~ error[actor-sendable:unsendable-payload]
    send fn unsendable(j: Job) => !j
}

// [actor-sendable] The other half of C-4(a): a `proj` view borrows the
// sender's value, so it cannot cross either.
struct Window {
    over: proj List<Int>
}

actor effect Peek {
    //~ error[actor-sendable:unsendable-payload]
    send fn look(w: Window) => !w
}

// [actor-effect-kind] [actor-monitor-handler] The binding gate, amended by
// SH-3 (user decision 2026-09-19): a plain effect is still never
// actor-backed (no mailbox, no messages), but `spawn`, `use addr` and naming
// `eff[E]` accept it as the **monitor** kind: one shared instance behind a
// lock, its members on the callers' threads.
effect Pinged {
    fn ping() -> Int
}

handler Pinging() of Pinged {
    fn ping() -> Int {
        return 1
    }
}

fn hold(a: eff[Pinged]) -> Int => a {
    return 1
}

fn a_plain_effect_spawns_as_a_monitor_not_an_actor() [use, spawn] {
    let p = spawn Pinging()
    let _held = hold(p)
    use p
    let _pinged = ping()
}

// And the two shapes a monitor cannot wear, each named: a mailbox (there is
// no queue to bound) and a placement (there is nothing to place).
effect Boxed {
    fn boxed() -> Int
}

handler Boxing() of Boxed {
    //~ error[actor-mailbox:nothing-to-enqueue]
    mailbox { capacity: 1 }

    fn boxed() -> Int {
        return 1
    }
}

fn a_monitor_refuses_a_pool() [use, spawn] {
    //~ error[actor-monitor-handler:on-pool]
    let _p = spawn Pinging() on pool(1)
}

// A **plain** handler binding of an actor effect stays legal: that is
// Example 6's binding swap, and the reason the kind sits on the effect rather
// than on the handler.
fn an_actor_effect_can_still_be_used_synchronously() [use, spawn] {
    use Printing()
    note("inline")
}
