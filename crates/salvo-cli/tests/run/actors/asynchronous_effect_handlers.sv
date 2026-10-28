// [rs-actor] asynchronous effect handlers
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- an_actor

actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 8 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let counter = spawn Counting() on pool(1)
    counter.bump(2)
    counter.bump(3)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    println("sum ${sum}")
}
//= stdout
//= sum 5

//--- a_deferring_mixed_handler

effect Random {
    fn next() -> Int
}

actor effect Oracle {
    send fn divine(out: Reply<Int>) => !out
}

handler Delphi() of Oracle {
    mailbox { capacity: 4 }
    state {
        n: Int = 0
    }

    send fn divine(out: Reply<Int>) {
        n = n + 7
        send(out, n)
    }
}

handler Forwarding(oracle: eff[Oracle]) of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        oracle.divine(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let oracle = spawn Delphi() on pool(1)
    use Forwarding(oracle) on pool(1)
    println("first ${next()}")
    println("second ${next()}")
}
//= stdout
//= first 7
//= second 14

//--- a_dependent_spawn

actor effect Log {
    send fn note(what: Str) => !what
    send fn dump(out: Reply<Str>) => !out
}

actor effect Tally {
    send fn tick(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

actor effect Counter {
    send fn bump(n: Int) => !n
    send fn report(out: Reply<Str>) => !out
}

handler Recording() of Log {
    mailbox { capacity: 1 }

    state {
        last: Str = "none"
    }

    send fn note(what: Str) {
        last = what
    }

    send fn dump(out: Reply<Str>) {
        out.send(copy(last))
    }
}

handler Summing() of Tally {
    mailbox { capacity: 8 }

    state {
        sum: Int = 0
    }

    send fn tick(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

handler Counting() [Log, Tally] of Counter {
    mailbox { capacity: 8 }

    send fn bump(n: Int) {
        note("bumped ${n}")
        tick(n)
    }

    send fn report(out: Reply<Str>) {
        dump(out)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let tally = spawn Summing() on pool(1)
    let counter = spawn Counting() with Recording(), tally on pool(1)
    counter.bump(2)
    counter.bump(3)
    let last = waitfor out: Reply<Str> {
        counter.report(out)
    }
    println("last ${last}")
    let sum = waitfor out: Reply<Int> {
        tally.total(out)
    }
    println("sum ${sum}")
}
//= stdout
//= last bumped 3
//= sum 5

//--- a_fn_value_declared_later

fn apply(f: (s: Str) [Console] -> None) [Console] -> None {
    f("hi")
}

fn main() [use] {
    use StdConsole()
    apply(shout)
}

fn shout(s: Str) [Console] -> None {
    println("${s}!")
}
//= stdout
//= hi!

//--- a_mixed_handler

effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        cursor = (cursor * 31 + seed) % 100000
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng on pool(1)
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    println("actor drew ${drawn}")
    println("main drew ${next()}")
}
//= stdout
//= main drew 12345
//= actor drew 95040
//= main drew 58585

//--- a_mixed_handler_with_dependencies

effect Random {
    fn next() -> Int
}

effect Label {
    fn label(n: Int) -> Str
}

handler Hash() of Label {
    fn label(n: Int) -> Str {
        return "#${n}"
    }
}

handler Counted(seed: Int) [Console, Label] of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        cursor = cursor + seed
        println("servant at ${label(cursor)}")
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let rng = spawn Counted(7) with Hash()
    use rng
    println("main drew ${next()}")
    println("main drew ${next()}")
}
//= stdout
//= servant at #7
//= main drew 7
//= servant at #14
//= main drew 14

//--- a_monitor

effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    state {
        cursor: Int = 0
    }

    fn next() -> Int {
        cursor = (cursor * 31 + seed) % 100000
        return cursor
    }
}

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng on pool(1)
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    println("actor drew ${drawn}")
    println("main drew ${next()}")
}
//= stdout
//= main drew 12345
//= actor drew 95040
//= main drew 58585

//--- a_parking_mixed_handler

effect Random {
    fn next() -> Int
}

actor effect Oracle {
    send fn divine(out: Reply<Int>) => !out
}

handler Delphi() of Oracle {
    mailbox { capacity: 4 }
    state {
        n: Int = 0
    }

    send fn divine(out: Reply<Int>) {
        n = n + 7
        send(out, n)
    }
}

handler Parking(oracle: eff[Oracle]) of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        oracle.divine(replyto settled(out))
    }

    send fn settled(out: Reply<Int>, drawn: Int) => !out, !drawn {
        send(out, drawn + tally)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let oracle = spawn Delphi() on pool(1)
    use Parking(oracle) on pool(1)
    println("first ${next()}")
    println("second ${next()}")
}
//= stdout
//= first 8
//= second 16

//--- a_servant_sibling_chain

effect Random {
    fn next() -> Int
}

handler Chain() of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        relay(out)
    }

    send fn relay(out: Reply<Int>) => defer out {
        deliver@self(out)
    }

    send fn deliver(out: Reply<Int>) => !out {
        send(out, tally * 10)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance@self(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    use Chain() on pool(1)
    println("first ${next()}")
    println("second ${next()}")
}
//= stdout
//= first 10
//= second 20
