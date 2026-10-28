// [effect-any] The shared shape: a router `of any Counter` in front of two
// members. Forwarding to a member of the effect it implements is not
// self-dispatch (the receiver is an addr), and the deadlock graph gives the
// router a node of its own, so `Counter → Counter` is not reported: this
// program reports no cycle.

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
handler Spraying(a: eff[Counter], b: eff[Counter]) of any Counter {
    mailbox { capacity: 16 }
    state {
        flip: Bool = false
    }
    send fn bump(n: Int) {
        flip = !flip
        if flip { a.bump(n) } else { b.bump(n) }
    }
    send fn total(out: Reply<Int>) {
        a.total(out)
    }
}

fn tally() [Counter] -> Int {
    bump(1)
    bump(2)
    return waitfor out: Reply<Int> { total(out) }
}

fn poke() [any Counter] -> None {
    bump(3)
}

// [effect-any] The identity rule: a `use` of a router binds `any Counter`,
// which a bare `[Counter]` requirement refuses (one instance, ordered sends)
// and `[any Counter]` accepts (`poke` is legal). The message names both
// remedies.
fn a_router_binding_satisfies_only_any_requirements() [use, spawn] {
    let p = pool(2)
    let log = spawn Printing() on p
    use log
    let a = spawn Counting() on p
    let b = spawn Counting() on p
    use Spraying(a, b)
    poke()
    //~ error[effect-any:single-instance-callee]
    let _n = tally()
}

// [effect-any] Viral downward: a body holding `[any Counter]` may not call a
// callee that assumes one instance, and a body holding the strong
// `[Counter]` may call either.
fn weak() [any Counter] -> Int {
    poke()
    //~ error[effect-any:single-instance-callee]
    return tally()
}

fn strong() [Counter] -> Int {
    poke()
    return tally()
}
