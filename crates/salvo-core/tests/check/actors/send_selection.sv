// [fn-overload] [actor-send-fn] Every send and mint target is chosen by the
// call's rule (PIPELINE.md B3.7, user decisions 2026-10-28): D18 (a), D24 (a),
// and a handler's own `send fn` reached by a uniform call.

actor effect Counter {
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 4 }

    send fn total(out: Reply<Int>) {
        out.send(1)
    }
}

// [task-mint] D18 (a): a mint ranks the visible `send fn`s of its name by the
// captures, as a call ranks overloads by its arguments.
send fn report(label: Str, n: Int) => !label {}

send fn report(code: Int, n: Int) {}

fn mints_rank_by_the_captures() [use, spawn] {
    let c = spawn Counting() on pool(1)
    c.total(replyto report("a"))
    c.total(replyto report(7))
}

// [actor-send-fn] A message carries its arguments as written: a defaulted
// parameter left out is refused until a send can fill it.
actor effect Log {
    send fn note(what: Str, level: Int = 1) => !what
}

handler Logging() of Log {
    mailbox { capacity: 4 }

    send fn note(what: Str, level: Int = 1) {}
}

fn a_send_with_a_default_left_out() [use, spawn] {
    let l = spawn Logging() on pool(1)
    l.note("all", 2)
    //~ error[actor-send-fn:message-shape]
    l.note("one")
}

//--- two send fns of one signature
// [task-mint] [fn-overload-ambiguous] An imported `send fn` beside this
// module's own of one signature is the ordinary ambiguity (census D18's
// `mint-rung`: before, this module's won in silence), naming
// `replyto formatted@main(…)` and `replyto formatted@lib(…)`.
// FILE: lib.sv
export send fn formatted(label: Str, n: Int) => !label {}
// FILE: main.sv
import lib.formatted

actor effect Counter {
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 4 }

    send fn total(out: Reply<Int>) {
        out.send(1)
    }
}

send fn formatted(label: Str, n: Int) => !label {}

fn go() [use, spawn] {
    let c = spawn Counting() on pool(1)
    //~ error[task-mint:ambiguous-send]
    c.total(replyto formatted("a"))
    c.total(replyto formatted@main("a"))
    c.total(replyto formatted@lib("a"))
}

//--- a mint takes a module selector only
// FILE: main.sv
actor effect Counter {
    send fn total(out: Reply<Int>) => !out
}

send fn report(label: Str, n: Int) => !label {}

fn go(c: eff[Counter]) [] => c {
    //~ error[actor-replyto:selector]
    c.total(replyto report@Counter("a"))
}

//--- a uniform call reaches the handler's own send fn
// [fn-uniform] [actor-mixed-handler] `x.k()` is `k(x)` exactly, and a
// handler's own `send fn` is in scope for it (B3.7 question 3 (a)).
effect Random {
    fn next() -> Int
}

handler Stepping(seed: Int) of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }

    send fn advance(by: Int, out: Reply<Int>) => !out {
        cursor = cursor + by + seed
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            2.advance(got)
        }
    }
}

//--- a send to an overloaded member of this actor
// [actor-send-fn] A self-send, servant send or member mint names its member,
// which cannot say which overload yet: refused, naming the addr send as the
// way (ROADMAP 0au).
actor effect Sink {
    send fn put(a: Int)
    send fn put(a: Str) => !a
}

handler Dropping() of Sink {
    mailbox { capacity: 4 }

    send fn put(a: Int) {
        //~ error[actor-send-fn:overloaded-own]
        put@self("again")
    }

    send fn put(a: Str) {}
}
