// [actor-self-send] `k@self(args)`: an unqualified call would run `k` inside
// this activation, and a self-send runs it as its own later one, which is how
// "finish this, then continue with `k`" is written.

actor effect Counter {
    send fn bump(n: Int)
    send fn total(out: Reply<Int>) => !out
}

actor effect Log {
    send fn note(what: Str) => !what
}

// Legal in a member body, with the arguments checked like any send's.
actor effect Work {
    send fn start(n: Int)
    send fn step(n: Int)
}

handler Working() of Work {
    mailbox { capacity: 1 }

    state {
        done: Int = 0
    }

    send fn start(n: Int) {
        step@self(n)
    }

    send fn step(n: Int) {
        done = done + n
    }
}

// The target must exist and be a `send fn`: a member that answers would have
// to wait for itself.
handler Later() of Log {
    mailbox { capacity: 1 }

    send fn note(what: Str) {
        //~ error[actor-self-send:no-member]
        later@self(what)
    }
}

actor effect Ready {
    send fn start(n: Int)
    fn ready() -> Bool
}

handler Readying() of Ready {
    mailbox { capacity: 1 }

    send fn start(n: Int) {
        //~ error[actor-self-send:not-send]
        ready@self()
    }

    fn ready() -> Bool {
        return true
    }
}

// `self` names the handler a member belongs to, so the form needs one: in a
// plain function it is an error saying what `self` means.
fn a_self_send_is_illegal_outside_a_handler_member() [use, spawn] {
    //~ error[actor-self-send:outside-member]
    bump@self(1)
}

// Its arguments are typed against the member's parameters, and a payload is
// *consumed*: the message outlives this activation even though it never
// leaves the process.
actor effect Typed {
    send fn start(n: Int)
    send fn step(n: Int)
}

handler Typing() of Typed {
    mailbox { capacity: 1 }

    send fn start(n: Int) {
        //~ error[call-resolve:no-overload]
        step@self("one")
    }

    send fn step(n: Int) {}
}

// [actor-self-send] `self` is **contextual**, not reserved: it means the
// enclosing handler only immediately after `@`, so it stays an ordinary name
// elsewhere, and a local of that name can never shadow the form, which is
// what the selector spelling buys over the `self.k(…)` receiver it replaced.
handler Selfish() of Log {
    mailbox { capacity: 1 }

    send fn note(what: Str) {
        let self = 1
        discard(self)
        note@self(what)
    }
}

fn plain() -> Int {
    let self = 2
    return self
}

// The self-dispatch diagnostic names the remedy that exists: calling one's
// own effect member unqualified is still refused, but for a `send fn` the
// message points at `k@self(…)`.
handler Counting2() of Counter {
    mailbox { capacity: 1 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        //~ error[effect-available:own-effect]
        bump(n)
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

// [actor-private-send] A `send fn` no face declares is a **private** member:
// reachable by `k@self(…)` and `replyto k(…)`, refused to a bare call (with
// the selector named), and carrying the free send fn's written, all-consumed
// clause [actor-free-send-fn] since no effect declaration mirrors it. A
// private member consuming a linear parameter discharges it by its own
// clause. (The deadlock graph's nodes are effects, so forwarding to another
// `Counter` is a `Counter → Counter` edge, and the back-pressure warning is
// what it reports.)
handler Sneaky(inner: eff[Counter]) of Counter {
    mailbox { capacity: 4 }
    state {
        sum: Int = 0
    }
    send fn bump(n: Int) { helper@self(n) }
    //~ warning[actor-deadlock-cycle:send-cycle]
    send fn total(out: Reply<Int>) { inner.total(replyto answered(out)) }
    send fn helper(n: Int) { sum = sum + n }
    send fn answered(out: Reply<Int>, n: Int) => !out { out.send(n + sum) }
}

// [copy-scalar-free] The clause names what is consumed, so the requirement is
// about a non-scalar parameter.
handler Careless() of Counter {
    mailbox { capacity: 4 }
    state {
        sum: Int = 0
    }
    //~ error[call-resolve:unknown-fn]
    send fn bump(n: Int) { helper("x") }
    send fn total(out: Reply<Int>) { out.send(sum) }
    //~ error[actor-mixed-handler:send-clause]
    send fn helper(s: Str) { sum = sum + 1 }
}

// [actor-mixed-handler] A mixed handler's servant keeps its handler-local
// send members under the same rule: a written clause, reached by `@self`.
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }
    send fn advance(out: Reply<Int>) => !out {
        cursor = cursor + seed
        out.send(cursor)
    }
    fn next() -> Int {
        return waitfor out: Reply<Int> { advance@self(out) }
    }
}

// [handler-init] `init { … }` is checked as a parameterless private member;
// `self@Face` is the handler's own address as one of its actor faces.
actor effect Registry { send fn register(who: eff[Counter]) => !who }
handler Registering() of Registry {
    mailbox { capacity: 4 }
    send fn register(who: eff[Counter]) => !who {}
}
handler Announcing(reg: eff[Registry]) of Counter {
    mailbox { capacity: 4 }
    state {
        sum: Int = 0
    }
    init {
        sum = 1
        reg.register(self@Counter)
    }
    send fn bump(n: Int) { sum = sum + n }
    send fn total(out: Reply<Int>) { out.send(sum) }
}
fn init_and_self_addr_check_clean() [use, spawn] {
    let p = pool(1)
    let reg = spawn Registering() on p
    let _a = spawn Announcing(reg) on p
}

// …refused outside a handler, for a face the handler lacks or a plain one,
// and a handler using it is spawn-only.
handler Twice() of Counter {
    mailbox { capacity: 4 }
    init {}
    //~ error[handler-init:member-named-init]
    send fn init() {}
    //~ error[actor-self-send:no-face]
    send fn bump(n: Int) { let _me = self@Log }
    send fn total(out: Reply<Int>) { out.send(0) }
}
handler Named() of Counter {
    mailbox { capacity: 4 }
    send fn bump(n: Int) { let _me = self@Counter }
    send fn total(out: Reply<Int>) { out.send(0) }
}
//~ error[actor-self-send:address-outside-member]
fn free() -> None { let _x = self@Counter }
fn binds_a_self_naming_handler() [use] {
    //~ error[actor-replyto:use-bound]
    use Named()
}
