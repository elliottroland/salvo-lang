// throwing fn values and union-typed throws
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [fn-throw-value] [fn-effects] A fn value that may throw throws to the
// `try` around its call: a named fn and a lambda passed into a throwing
// slot, a function that delimits it writing `[]`, one passing it on
// declaring the throw, one writing no list inheriting it, and an implicit
// with a throwing type (the shape `args`' validation hook needs).
//--- a_throwing_fn_value_is_delimited
import std.throw

fn check(n: Int) [Throw<Str>] -> Int {
    if n < 0 {
        throw("negative")
    }
    return n
}

// Delimits the throw itself, so says `[]`.
fn run<T>(n: Int, f: (n: Int) [Throw<Str>] -> T) [] -> Str => f {
    let r = try {
        f(n)
    }
    when r {
        is Ok { return "ok" }
        is Thrown { return "thrown: ${r}" }
    }
}

// Passes the throw on, so says so.
fn pass_on(n: Int, f: (n: Int) [Throw<Str>] -> Int) [Throw<Str>] -> Int => f {
    return f(n) + 1
}

// Writes no list, so inherits `f`'s.
fn inherit(n: Int, f: (n: Int) [Throw<Str>] -> Int) -> Int => f {
    return f(n) * 10
}

// The implicit form the args hook needs.
fn by_implicit<V>(n: Int, ?check: (n: Int) [Throw<Str>] -> V) [] -> Str {
    let r = try {
        check(n)
    }
    when r {
        is Ok { return "implicit ok" }
        is Thrown { return "implicit thrown: ${r}" }
    }
}

fn main() [use] -> None {
    use StdConsole()
    println(run(3, check))
    println(run(-1, check))
    println(run(5, (n: Int) -> {
        if n > 3 {
            throw("big")
        }
        return n + 0
    }))
    let a = try {
        pass_on(-2, check)
    }
    when a {
        is Ok { println("passed ${a}") }
        is Thrown { println("passed on: ${a}") }
    }
    let b = try {
        inherit(4, check)
    }
    when b {
        is Ok { println("inherited ${b}") }
        is Thrown { println("inherited: ${b}") }
    }
    let i: Str = by_implicit<Int>(-7)
    println(i)
}
//= stdout
//= ok
//= thrown: negative
//= thrown: big
//= passed on: negative
//= inherited 40
//= implicit thrown: negative

// [kt-throw-tag] [rs-throw-controlflow] A union-typed message is caught by
// a `try` whose message is a wider union: on Kotlin it escaped, since the
// signal was tagged with the sub-union's name, which matched no case.
//--- a_sub_union_throw_is_caught
import std.throw

struct A { x: Int }
struct B { y: Int }
struct C { z: Int }

fn pick(n: Int) [] -> A | B {
    if n > 0 {
        return A { x: n }
    }
    return B { y: n }
}

fn f(n: Int) [Throw<A | B | C>] -> Int {
    if n == 0 {
        throw(C { z: 0 })
    }
    let e: A | B = pick(n)
    throw(e)
}

fn main() [use] -> None {
    use StdConsole()
    let r = try {
        f(1)
    }
    when r {
        is Ok { println("ok") }
        is Thrown { println("caught") }
    }
}
//= stdout
//= caught
