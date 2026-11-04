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
//= fine
//= ok
//= caught no
//= bumps 3
