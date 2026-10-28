// [actor-watch] The monitor surface, and [actor-spawn-effect]'s capability
// on what runs processes.

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


// [actor-watch] The whole of it: a token minted like any other, handed to
// `watch`, and answered by the scheduler when the process dies. `main`'s
// token comes from `waitfor`, so a program can wait for a child's death
// without a handler of its own.
fn watch_takes_an_addr_and_a_reply_token() [use, spawn] {
    let c = spawn Counting() with Printing() on pool(1)
    let e = waitfor out: Reply<Exit> {
        watch(c, out)
    }
    discard(e.reason)
}

// [actor-watch] [linear-obligation] The registration *is* the obligation:
// the token is consumed by `watch`, so a `waitfor` block that mints one and
// registers nothing is the ordinary leak, which is how "you cannot silently
// forget you were watching" is enforced, with no rule of its own.
fn a_watch_token_that_is_never_registered_leaks() [use, spawn] {
    let c = spawn Counting() with Printing() on pool(1)
    //~ error[ir-linear-check:leak-at-exit]
    let e = waitfor out: Reply<Exit> {
        discard(c)
    }
    discard(e.reason)
}

// [actor-watch] [actor-spawn-effect] Watching is a `spawn`-capability
// operation: a monitor is part of running processes, so a function that has
// not been given the capability cannot register one.
fn observe(c: eff[Counter], out: Reply<Exit>) [] -> None => c, !out {
    //~ error[actor-spawn-effect:callee-needs-spawn]
    watch(c, out)
}

// [actor-spawn-effect] And the capability **propagates**, like any effect: a
// helper that creates a pool needs `[spawn]`, and so does everyone who calls
// it.
fn make() -> Pool {
    //~ error[actor-spawn-effect:callee-needs-spawn]
    return pool(1)
}
