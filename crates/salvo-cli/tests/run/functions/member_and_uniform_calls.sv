// [fn-member-call] [fn-uniform] `x.name(args)`: a member call when `x`'s type
// has a member `name`, otherwise the plain call `name(x, args)` (user
// decision 2026-10-28).
//
// Compile-and-run data ([test-run-data]).

//--- member_calls_through_fields

struct Adder {
    f: (n: Int) -> Int
    by: Int
}

struct Hook {
    run: (s: Str) [Console] -> None
}

fn make(by: Int) [] -> Adder => by { return Adder { f: (n: Int) -> n + by, by: by } }

fn twice(a: Adder, n: Int) [] -> Int => a, n { return n * 2 }

fn main() [use] {
    use StdConsole()
    let a = make(10)
    println("member: ${a.f(3)}")
    println("grouped: ${(a.f)(4)}")
    println("temporary: ${make(100).f(5)}")
    println("uniform: ${a.twice(6)}")
    let h = Hook { run: (s: Str) -> println("hook ${s}") }
    h.run("ran")
    let maybe: Adder? = a
    println("safe: ${maybe?.f(7) ?: 0}")
    let none: Adder? = None
    println("safe none: ${none?.f(7) ?: 0}")
}
//= stdout
//= member: 13
//= grouped: 14
//= temporary: 105
//= uniform: 12
//= hook ran
//= safe: 17
//= safe none: 0

//--- uniform_calls_are_plain_calls

fn twice(n: Int) [] -> Int => n { return n * 2 }

fn scaled(n: Int, ?scale: (k: Int) -> Int) [] -> Int => n, scale { return n.scale() }

fn main() [use] {
    use StdConsole()
    println("declared: ${5.twice()}")
    let twice = (k: Int) -> k * 3
    println("local: ${5.twice()}")
    let m: Int? = 7
    println("safe local: ${m?.twice() ?: 0}")
    println("implicit: ${scaled(1, scale = (k: Int) -> k + 1000)}")
}
//= stdout
//= declared: 10
//= local: 15
//= safe local: 21
//= implicit: 1001

//--- sends_and_uniform_calls_on_an_addr

actor effect Counter {
    send fn bump(by: Int) => !by
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 8 }
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

fn describe(c: eff[Counter]) [] -> Str => c { return "a counter" }

fn bump(c: eff[Counter], by: Int) [] -> None => c, by {
    (c.bump)(by * 100)
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let counter = spawn Counting() on pool(2)
    println(counter.describe())
    (counter.bump)(1)
    bump(counter, 2)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    println("total ${sum}")
}
//= stdout
//= a counter
//= total 201
