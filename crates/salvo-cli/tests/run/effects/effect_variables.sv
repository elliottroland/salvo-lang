// effect variables
//
// [effect-var] A generic written `[E]` ranges over a set of effects: a call
// binds it to what the fn value it is handed performs, and the callee passes
// the pack of those instances on without knowing what is in it. Each program
// runs on both backends with the same output.

//--- std_callbacks_perform_effects
// std's callback-taking functions take an effect variable, so a callback may
// perform effects: none (the empty pack), one, several, or a named fn's.
effect Tally {
    fn bump() -> None
    fn total() -> Int
}

handler Counting() of Tally {
    state {
        n: Int = 0
    }
    fn bump() { n = n + 1 }
    fn total() -> Int { return n }
}

fn shout(x: Int) [Console] {
    println("shout ${x}")
}

fn loud(x: Int) [Console] -> Int {
    println("map ${x}")
    return x * 10
}

fn noisy(x: Int) [Console, Tally] {
    bump()
    println("noisy ${x}")
}

// A pure fn parameter forwarded into an effect-polymorphic one: adapted to
// take the empty pack.
fn doubled(xs: List<Int>, f: (x: Int) -> Int) -> Mut List<Int> => xs, f {
    return map(xs, f)
}

fn main() [use] -> None {
    use StdConsole()
    use Counting()
    let xs = List(1, 2, 3)
    let ys = map(xs, (x) -> loud(x))
    println("mapped ${ys}")
    let some = any(xs, (x) -> {
        bump()
        return x > 1
    })
    println("any ${some}")
    println("count ${count(xs, (x) -> x > 1)}")
    let sum = reduce(xs, 0, (a, x) -> {
        bump()
        return a + x
    })
    println("sum ${sum}")
    drain(List(4, 5), shout)
    drain(List(6), noisy)
    let both: (x: Int) [Console, Tally] -> None = noisy
    drain(List(7), both)
    println("doubled ${doubled(xs, (x) -> x * 2)}")
    println("bumps ${total()}")
}
//= stdout
//= map 1
//= map 2
//= map 3
//= mapped [10, 20, 30]
//= any true
//= count 2
//= sum 6
//= shout 4
//= shout 5
//= noisy 6
//= noisy 7
//= doubled [2, 4, 6]
//= bumps 7

//--- polymorphic_functions
// A function of one's own: the variable passed on, beside named effects the
// callee supplies itself, through recursion, and as a handle.
import std.throw

effect Say {
    fn say(s: Str) -> None => s
}

handler Printing() [Console] of Say {
    fn say(s: Str) { println(s) }
}

effect Tally {
    fn bump() -> None
    fn total() -> Int
}

handler Counting() of Tally {
    state {
        n: Int = 0
    }
    fn bump() { n = n + 1 }
    fn total() -> Int { return n }
}

effect Bumps {
    fn bump2() -> None
}

handler Counting2() [Console] of Bumps {
    fn bump2() { println("bump2") }
}

fn each<T, [E]>(xs: List<T>, f: (x: T) [E] -> None) => xs, f {
    for x in xs {
        f(x)
    }
}

// The callee performs `Say` itself and hands `E` on.
fn announced<T, [E]>(xs: List<T>, f: (x: T) [E, Say] -> None) [E, Say] => xs, f {
    say("start")
    each(xs, (x) -> f(x))
    say("end")
}

fn countdown<[E]>(n: Int, f: (x: Int) [E] -> None) => f {
    if n > 0 {
        f(n)
        countdown(n - 1, f)
    }
}

// Two parameters share `E`: it binds what either performs.
fn both<[E]>(f: () [E] -> None, g: () [E] -> None) => f, g {
    f()
    g()
}

// A handle over an effect variable, bound in the body.
fn later<[E]>(fx: eff[E], f: () [E] -> None) [use] => fx, f {
    use fx
    f()
}

// A throw the parameter's type names is the callee's to catch.
fn guarded<[E]>(f: () [E, Throw<Str>] -> None) [E] -> Str => f {
    let r = try { f() }
    when r {
        is Ok { return "ok" }
        is Thrown { return "caught ${r}" }
    }
}

fn main() [use] -> None {
    use StdConsole()
    let s = use Printing()
    use Counting()
    announced(List(1, 2), (x) -> {
        bump()
        say("item ${x}")
    })
    countdown(2, (x) -> say("down ${x}"))
    both(() -> bump(), () -> say("both"))
    later(s, () -> say("later"))
    later(eff[], () -> {})
    // A handle of two effects, in another order than the variable's pack
    // keeps them: reordered into it.
    let t = use Counting2()
    later(eff[s, t], () -> {
        say("both")
        bump2()
    })
    println(guarded(() -> say("fine")))
    println(guarded(() -> throw("no")))
    println("bumps ${total()}")
}
//= stdout
//= start
//= item 1
//= item 2
//= end
//= down 2
//= down 1
//= both
//= later
//= both
//= bump2
//= fine
//= ok
//= caught no
//= bumps 3

//--- structs_and_lazy_adaptors
// A struct holds an effect variable as a fn field's effects, and its type
// names what it is bound to (`Later<[]>`). std's lazy adaptors are such
// structs: their callbacks run as the adaptor is driven, by a `for` or by a
// function filling `?Next` with an adaptor that performs nothing.
effect Tally {
    fn bump() -> None
    fn total() -> Int
}

handler Counting() of Tally {
    state {
        n: Int = 0
    }
    fn bump() { n = n + 1 }
    fn total() -> Int { return n }
}

struct Later<[E]> {
    n: Int,
    run: (x: Int) [E] -> Int
}

fn later<[E]>(n: Int, f: (x: Int) [E] -> Int) -> Later<[E]> => !f {
    return Later { n: n, run: f }
}

fn fire<[E]>(l: Later<[E]>) [E] -> Int => l {
    let f = l.run
    return f(l.n)
}

fn tens(x: Int) [Tally] -> Int {
    bump()
    return x * 10
}

fn main() [use] -> None {
    use StdConsole()
    use Counting()
    let a = later(3, (x) -> {
        bump()
        println("a ${x}")
        return x * 2
    })
    println("fired ${fire(a)}")
    let b: Later<[]> = later(4, (x) -> x + 1)
    println("fired ${fire(b)}")
    let xs = List(1, 2, 3, 4)
    for y in mapping(iter(xs), (x) -> tens(x)) {
        println("y ${y}")
    }
    for y in filtering(iter(xs), (x) -> x > 2) {
        println("big ${y}")
    }
    for y in taking_while(iter(xs), (x) -> {
        println("check ${x}")
        return x < 3
    }) {
        println("took ${y}")
    }
    for y in skipping_while(iter(xs), (x) -> {
        bump()
        return x < 3
    }) {
        println("from ${y}")
    }
    let m = mapping(iter(xs), (x) -> x + 1)
    println("list ${size(to_list(m))}")
    println("total ${total()}")
}
//= stdout
//= a 3
//= fired 6
//= fired 5
//= y 10
//= y 20
//= y 30
//= y 40
//= big 3
//= big 4
//= check 1
//= took 1
//= check 2
//= took 2
//= check 3
//= from 3
//= from 4
//= list 4
//= total 8

//--- a_polymorphic_fn_as_a_value
// A struct literal's fn field binds the struct's effect variable, and a
// function with an effect variable of its own, passed as a value, takes the
// pack the position hands it — of two effects, and of none.
effect Tally {
    fn bump() -> None
    fn total() -> Int
}

handler Counting() of Tally {
    state {
        n: Int = 0
    }
    fn bump() { n = n + 1 }
    fn total() -> Int { return n }
}

struct Later<[E]> {
    n: Int,
    run: (x: Int) [E] -> Int
}

fn fire<[E]>(l: Later<[E]>) [E] -> Int => l {
    let f = l.run
    return f(l.n)
}

fn main() [use] -> None {
    use StdConsole()
    use Counting()
    let loud: List<Later<[Console, Tally]>> = List(Later { n: 1, run: (x) -> {
        bump()
        println("loud ${x}")
        return x
    } })
    println("${map(loud, fire)}")
    let quiet: List<Later<[]>> = List(Later { n: 2, run: (x) -> x * 3 })
    println("${map(quiet, fire)}")
    println("total ${total()}")
}
//= stdout
//= loud 1
//= [1]
//= [6]
//= total 1

//--- an_actor_effect_through_a_handle
// An actor's address passed as `eff[E]` is made its instance, the pack's
// member: alone, and beside a plain effect's handle.
actor effect Counter {
    send fn bump(n: Int)
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

effect Say {
    fn say(s: Str) -> None => s
}

handler Printing() [Console] of Say {
    fn say(s: Str) { println(s) }
}

// A handle over an actor's effect, bound in a body generic over `E`.
fn later<[E]>(fx: eff[E], f: () [E] -> None) [use] => fx, f {
    use fx
    f()
}

fn main() [use, spawn] {
    use StdConsole()
    let s = use Printing()
    let counter = spawn Counting() on pool(1)
    later(counter, () -> bump(2))
    later(eff[s, counter], () -> {
        say("both")
        bump(3)
    })
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    println("sum ${sum}")
}
//= stdout
//= both
//= sum 5

//--- a_generic_throw_beside_a_variable
// A parameter's `Throw<X>` with `X` generic: a lambda's throws bind `X`, and a
// lambda that throws nothing binds it to the bottom type. Kotlin cannot infer
// such an `X` from the arguments, so the call writes it.
import std.throw

fn guarded<[E], X>(f: () [E, Throw<X>] -> None) [E] -> Str => f {
    let r = try { f() }
    when r {
        is Ok { return "ok" }
        is Thrown { return "caught" }
    }
}

fn attempt<X>(f: () [Throw<X>] -> None) [] -> Str => f {
    let r = try { f() }
    when r {
        is Ok { return "ok" }
        is Thrown { return "caught" }
    }
}

fn main() [use] -> None {
    use StdConsole()
    println(guarded(() -> println("fine")))
    println(guarded(() -> throw("no")))
    println(guarded(() -> throw(3)))
    println(attempt(() -> throw("no")))
}
//= stdout
//= fine
//= ok
//= caught
//= caught
//= caught

//--- platform_functions_take_effectful_callbacks
// std's platform functions with callbacks take an effect variable too: the
// host takes the callback pure, and the wrapper hands it a closure over the pack.
effect Tally {
    fn bump() -> None
    fn total() -> Int
}

handler Counting() of Tally {
    state {
        n: Int = 0
    }
    fn bump() { n = n + 1 }
    fn total() -> Int { return n }
}

fn entry(i: Int) [Tally] -> (Int, Int) {
    bump()
    return (i, i * 10)
}

fn shown(i: Int) [Console] -> Int {
    println("set ${i}")
    return i
}

fn main() [use] -> None {
    use StdConsole()
    use Counting()
    let xs = List(1, 2, 3, 4)
    let big = filter(xs, (x) -> {
        bump()
        return x > 2
    })
    println("big ${size(big)}")
    let m = map_by(3, (i) -> entry(i))
    println("map ${size(m)}")
    let s = set_by(2, (i) -> shown(i))
    println("set ${size(s)}")
    let pure = filter(xs, (x) -> x > 1)
    println("pure ${size(pure)}")
    println("total ${total()}")
}
//= stdout
//= big 2
//= map 3
//= set 0
//= set 1
//= set 2
//= pure 3
//= total 7
