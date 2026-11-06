// [fn-overload] [actor-send-fn] Every send and mint target is chosen by the
// call's rule (the pipeline record B3.7, user decisions 2026-10-28).
//
// Compile-and-run data ([test-run-data]).

//--- an_overloaded_member_whose_parameters_extend_another
// [effect-member-overload] [kt-fn-mangling] `put(a: Int)` is a prefix of
// `put(a: Int, b: Int)`: each handler member takes its own overload's name
// (Kotlin gave the shorter the longer's, so the class implemented one and
// overrode nothing with the other). A plain effect's members may still be
// overloaded; an actor effect's send members may not [actor-message].

effect Sink {
    fn put(a: Int) -> Str
    fn put(a: Int, b: Int) -> Str
}

handler Joining() for Sink {
    fn put(a: Int) -> Str { return "one ${a}" }
    fn put(a: Int, b: Int) -> Str { return "two ${a} ${b}" }
}

fn main() [use] {
    use StdConsole()
    use Joining()
    println(put(1))
    println(put(1, 2))
}
//= stdout
//= one 1
//= two 1 2

//--- a_mint_ranked_by_its_captures
// [task-mint] D18 (a): two free `send fn report`s told apart by the captures.

actor effect Counter {
    send fn total(out: Reply<Int>) => !out
}

handler Counting() for Ordered Counter {
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

handler Stepping(seed: Int) for Random {
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
