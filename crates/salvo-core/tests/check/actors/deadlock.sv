// [actor-deadlock-cycle] The static deadlock baseline. The graph is
// program-wide, so every case here is a program of its own.

//--- gate-cycle
// [actor-deadlock-cycle] Example 4, the committed baseline: two processes
// that each park a **gated** continuation on the other's answer. Neither
// handler is wrong on its own: the bug is a property of the pair, and it is
// interleaving-dependent, so it is the possibility that is reported.
actor effect OrderApi {
    send fn place(id: Int, out: Reply<Str>) => !out
    send fn open_orders(id: Int, out: Reply<Int>) => !out
}

actor effect CreditApi {
    send fn credit(id: Int, out: Reply<Bool>) => !out
}

handler Orders(credit: eff[CreditApi]) of OrderApi {
    mailbox { capacity: 1 }

    send fn place(id: Int, out: Reply<Str>) {
        credit.credit(id, replyto! placed(out))
    }
    send fn placed(out: Reply<Str>, ok: Bool) => !out {
        out.send("placed")
    }
    send fn open_orders(id: Int, out: Reply<Int>) {
        out.send(0)
    }
}

handler Credit(orders: eff[OrderApi]) of CreditApi {
    mailbox { capacity: 1 }

    send fn credit(id: Int, out: Reply<Bool>) {
        //~ error[actor-deadlock-cycle:gate-cycle]
        orders.open_orders(id, replyto! counted(out))
    }
    send fn counted(out: Reply<Bool>, n: Int) => !out {
        out.send(true)
    }
}

//--- inferred-wait-cycle
// [actor-waitfor] [actor-deadlock-cycle] The block edge, **inferred** since
// SH-5(d) deleted the declaration: a handler whose member contains a
// `waitfor` serves no message while it waits, so a cycle through it
// deadlocks exactly as a gate cycle does, and a dedicated thread does not
// save it, because the fulfilment routes back through the stalled mailbox.
actor effect TimerApi {
    send fn after(ms: Int, out: Reply<Int>) => !out
}

actor effect ClockApi {
    send fn now(out: Reply<Int>) => !out
}

handler Timing(clock: eff[ClockApi]) of TimerApi {
    mailbox { capacity: 1 }

    send fn after(ms: Int, out: Reply<Int>) {
        clock.now(replyto! fired(out))
    }
    send fn fired(out: Reply<Int>, at: Int) => !out {
        out.send(at)
    }
}

handler Clocking() [TimerApi] of ClockApi {
    mailbox { capacity: 1 }

    send fn now(out: Reply<Int>) {
        //~ error[actor-deadlock-cycle:wait-cycle]
        let at = waitfor fired: Reply<Int> {
            after(0, fired)
        }
        out.send(at)
    }
}

//--- one-ungated-side
// [actor-deadlock-cycle] The rule the refusal keys on: a **bare** `replyto`
// leaves the mailbox open, so a cycle where one side keeps serving is not a
// deadlock. It is still the back-pressure class, so what is left is the
// warning: the program compiles.
actor effect OrderApi {
    send fn place(id: Int, out: Reply<Str>) => !out
    send fn open_orders(id: Int, out: Reply<Int>) => !out
}

actor effect CreditApi {
    send fn credit(id: Int, out: Reply<Bool>) => !out
}

handler Orders(credit: eff[CreditApi]) of OrderApi {
    mailbox { capacity: 1 }

    send fn place(id: Int, out: Reply<Str>) {
        credit.credit(id, replyto! placed(out))
    }
    send fn placed(out: Reply<Str>, ok: Bool) => !out {
        out.send("placed")
    }
    send fn open_orders(id: Int, out: Reply<Int>) {
        out.send(0)
    }
}

handler Credit(orders: eff[OrderApi]) of CreditApi {
    mailbox { capacity: 1 }

    send fn credit(id: Int, out: Reply<Bool>) {
        //~ warning[actor-deadlock-cycle:send-cycle]
        orders.open_orders(id, replyto counted(out))
    }
    send fn counted(out: Reply<Bool>, n: Int) => !out {
        out.send(true)
    }
}

//--- interception-is-exempt
// [actor-deadlock-cycle] **Interception is exempt**, and it has to be: a
// handler of `E` declaring `[E]` is the showcase pattern (a policy wrapper
// around the process already serving `E`) and it is an `E → E` edge by
// construction. The dependency binds strictly *outward*, so the chain ends at
// the innermost instance and no instance ever waits for itself. No error,
// and no warning either.
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
handler Caching() [Counter] of Counter {
    mailbox { capacity: 1 }

    send fn bump(n: Int) {
        bump(n)
    }
    send fn total(out: Reply<Int>) {
        total(replyto! answered(out))
    }
    send fn answered(out: Reply<Int>, n: Int) => !out {
        out.send(n)
    }
}

//--- one-way-pair
// [actor-deadlock-cycle] A one-directional pair is the common, correct
// topology and must stay silent: the requester gates, the answerer only ever
// fulfils tokens, and a fulfil contributes no edge at all.
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
actor effect Waiting {
    send fn ask()
    send fn got(n: Int)
}

handler Asking(c: eff[Counter]) of Waiting {
    mailbox { capacity: 1 }

    send fn ask() {
        c.total(replyto! got())
    }
    send fn got(n: Int) {}
}

//--- task-body-traced
// [task-mint] FC-6's conservative tracing: a task's sends are attributed to
// every actor whose mints reach it, so an actor that hands its work to a task
// which sends back closes the same cycle it would have closed directly.
// Without the tracing this program looks acyclic and the cycle ships. (The
// handler reads its constructor parameters with `copy`: a mint's capture
// moves, and the handler still owns them [effect-state-store].)
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
actor effect Peer {
    send fn ping(back: eff[Counter]) => !back
}

actor effect Ask {
    send fn ask(out: Reply<Int>) => !out
}

send fn answer(peer: eff[Peer], back: eff[Counter], n: Int) => !peer, !back {
    //~ warning[actor-deadlock-cycle:send-cycle]
    peer.ping(back)
    discard(n)
}

handler Peering(counter: eff[Counter]) of Peer {
    mailbox { capacity: 1 }

    send fn ping(back: eff[Counter]) {
        counter.bump(1)
        discard(back)
    }
}

handler Working(peer: eff[Peer], back: eff[Counter]) [Ask] of Counter {
    mailbox { capacity: 1 }

    send fn bump(n: Int) {
        discard(n)
    }
    send fn total(out: Reply<Int>) {
        ask(replyto answer(copy(peer), copy(back)))
        out.send(0)
    }
}

//--- task-body-untraced
// The same program without the mint has no cycle: the task's sends are
// attributed to the actors that mint it, and nothing else.
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
actor effect Peer {
    send fn ping(back: eff[Counter]) => !back
}

actor effect Ask {
    send fn ask(out: Reply<Int>) => !out
}

send fn answer(peer: eff[Peer], back: eff[Counter], n: Int) => !peer, !back {
    peer.ping(back)
    discard(n)
}

handler Peering(counter: eff[Counter]) of Peer {
    mailbox { capacity: 1 }

    send fn ping(back: eff[Counter]) {
        counter.bump(1)
        discard(back)
    }
}

handler Working(peer: eff[Peer], back: eff[Counter]) [Ask] of Counter {
    mailbox { capacity: 1 }

    send fn bump(n: Int) {
        discard(n)
    }
    send fn total(out: Reply<Int>) {
        discard(copy(peer))
        discard(copy(back))
        out.send(0)
    }
}
