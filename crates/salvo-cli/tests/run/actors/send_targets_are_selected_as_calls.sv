// [fn-overload] [actor-send-fn] Every send and mint target is chosen by the
// call's rule (PIPELINE.md B3.7, user decisions 2026-10-28).
//
// Compile-and-run data ([test-run-data]).

//--- an_overloaded_send_member_chosen_by_type
// [effect-member-overload] D24 (a): `s.put(1)` and `s.put("one")` reach the
// overloads by their argument types, and each lowers to its own message.

actor effect Sink {
    send fn put(a: Int)
    send fn put(a: Str) => !a
    send fn done(out: Reply<Str>) => !out
}

handler Collecting() of Sink {
    mailbox { capacity: 8 }
    state {
        seen: Str = ""
    }

    send fn put(a: Int) {
        seen = "${seen}int ${a}; "
    }

    send fn put(a: Str) {
        seen = "${seen}str ${a}; "
    }

    send fn done(out: Reply<Str>) {
        out.send(copy(seen))
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let s = spawn Collecting() on pool(1)
    s.put(1)
    s.put("one")
    let got = waitfor out: Reply<Str> {
        s.done(out)
    }
    println(got)
}
//= stdout
//= int 1; str one; 

//--- a_mint_ranked_by_its_captures
// [task-mint] D18 (a): two free `send fn report`s told apart by the captures.

actor effect Counter {
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 4 }

    send fn total(out: Reply<Int>) {
        out.send(5)
    }
}

send fn report(label: Str, done: Reply<Str>, n: Int) [Console] => !label, !done {
    println("${label}: ${n}")
    done.send("ok")
}

send fn report(code: Int, done: Reply<Str>, n: Int) [Console] => !done {
    println("code ${code}: ${n}")
    done.send("ok")
}

fn main() [use, spawn] {
    use StdConsole()
    let c = spawn Counting() on pool(1)
    let a = waitfor d: Reply<Str> {
        c.total(replyto report("sum", d))
    }
    let b = waitfor d: Reply<Str> {
        c.total(replyto report(7, d))
    }
    println("${a} ${b}")
}
//= stdout
//= sum: 5
//= code 7: 5
//= ok ok

//--- a_uniform_call_reaches_the_handlers_own_send
// [fn-uniform] [actor-mixed-handler] B3.7 question 3 (a): `2.advance(got)` is
// `advance(2, got)`, the servant send.

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

fn main() [use, spawn] {
    use StdConsole()
    let rng = spawn Stepping(10) on pool(1)
    use rng
    let a = next()
    let b = next()
    println("${a} ${b}")
}
//= stdout
//= 12 24
