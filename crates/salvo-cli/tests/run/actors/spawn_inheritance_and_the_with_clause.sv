// spawn-inheritance and the `with` clause [actor-spawn-inherit]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [effect-handle] [effect-handler-deps] A **generic** dependent handler works
// in the owned-handle form: `Relay<T>`'s dependency is a handle field, so
// no generated trait has to name the handler's generics — the cut that
// still stands for the fusion form (`generic_dependent_handler_is_a_
// codegen_error`) does not bind here.
//--- a_generic_handler_with_a_handle_dep

effect Sink<T> {
    fn accept(value: T) -> Str => value
}

handler Relay<T> [Console] of Sink<T> {
    fn accept(value: T) -> Str => value {
        println("relayed")
        return "ok"
    }
}

fn main() [use] -> None {
    use StdConsole
    use Relay<Int>()
    println(accept(1))
}
//= stdout
//= relayed
//= ok

//--- flattened_replyto

actor effect Counter {
    send fn bump(by: Int) => !by
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 4 }
    state {
        sum: Int = 0
    }
    send fn bump(by: Int) {
        sum = sum + by
    }
    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

actor effect Ledger {
    send fn report(out: Reply<Int>) => !out
    send fn reported(out: Reply<Int>, total: Int) => !out, !total
}

handler Bookkeeping() [Counter] of Ledger {
    mailbox { capacity: 4 }
    send fn report(out: Reply<Int>) {
        total()
            replyto reported(out)
    }
    send fn reported(out: Reply<Int>, total: Int) {
        out.send(total)
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let counter = spawn Counting() on pool(2)
    counter.bump(2)
    counter.bump(3)
    let ledger = spawn Bookkeeping() with counter on pool(2)
    let line = waitfor out: Reply<Int> {
        ledger.report(out)
    }
    println("ledger total ${line}")
}
//= stdout
//= ledger total 5

// [actor-spawn-inherit] [effect-with-clause] [rs-handle] The arc end to end:
// a signature-supplied effect captured into a handler (the hidden `__Hs_N`
// bundle), a spawn inheriting its dependency from the scope, and a `with`
// clause overriding the self-dependency with a private instance.
// Byte-identical stdout on Kotlin.
//--- spawn_inheritance

effect Clock {
    fn now() -> Int
}

handler TickingClock of Clock {
    state {
        t: Int = 0
    }
    fn now() -> Int { t = t + 1 return t }
}

effect Logger {
    fn log(m: Str) -> None => m
}

handler PlainLogger [Console] of Logger {
    fn log(m: Str) -> None => m { println("log: ${m}") }
}

handler Stamped [Logger, Clock] of Logger {
    fn log(m: Str) -> None => m {
        log("[t=${now()}] ${m}")
    }
}

handler FixedClock of Clock {
    fn now() -> Int { return 99 }
}

actor effect Reporter {
    send fn report(what: Str, done: Reply<Int>) => !what, !done
}

handler Reporting() [Logger] of Reporter {
    mailbox { capacity: 4 }

    send fn report(what: Str, done: Reply<Int>) {
        log("reported ${what}")
        done.send(1)
    }
}

fn work(step: Str) [Logger] -> None => step {
    log(step)
}

fn interception() [Logger, Clock, use] -> None {
    work("plain")
    use Stamped
    work("stamped")
}

fn main() [use, spawn] -> None {
    use StdConsole
    use TickingClock()
    use PlainLogger()
    interception()
    let r = spawn Reporting() on pool(1)
    let acked = waitfor done: Reply<Int> {
        r.report("inherited", done)
    }
    let sink = acked
    use Stamped with FixedClock()
    work("overridden")
}
//= stdout
//= log: plain
//= log: [t=1] stamped
//= log: reported inherited
//= log: [t=99] overridden
