// emitter gaps: names that captured generated calls, and ROADMAP 0c
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [kt-stdlib-qualified] A program may name its own fns and types after the
// Kotlin stdlib functions the emitter calls — `run` (every block in value
// position), `error`, `check`, `Pair` (tuples) — without the generated code
// calling the program's instead: each is written qualified. Before
// 2026-10-20 the elvis below recursed into the program's `run` and
// overflowed the stack on Kotlin.
//--- kotlin_stdlib_names_do_not_capture_generated_calls
struct Pair {
    a: Int
}

struct Triple {
    b: Int
}

fn run(f: (v: Int) -> Str?) [] -> Str => f {
    return f(1) ?: "none"
}

fn error(s: Str) [] -> Str => s {
    return "mine: ${s}"
}

fn check(b: Bool) [] -> Bool {
    return !b
}

fn main() [use] -> None {
    use StdConsole()
    println(run((v: Int) -> None))
    println(error("x"))
    println("${check(true)}")
    let (n, s) = (Pair { a: 1 }.a, "two")
    let (x, y, z) = (Triple { b: 3 }.b, 4, 5)
    println("${n} ${s} ${x + y + z}")
}
//= stdout
//= none
//= mine: x
//= false
//= 1 two 12

// The emitter gaps of ROADMAP 0c, closed 2026-10-04: one program for every
// item, each line numbered after its item: (1) a second `let` of a name
// after a loop that declared it, (2) a union argument to a `send fn`
// through an addr, (3) an effect named `Box`, (4) `f(f(x))`, (6) a `return`
// in a lambda inside a throwing fn [kt-lambda-return], (9) an `is` binding
// read later in its `&&` chain [is-and-chain], (11) a fn field with a named
// parameter, (12) a field default naming a type the literal's file does not
// import [rs-default-path], (13) a handler named like another module's type
// [type-identity], (14) writing a field of a `for` element
// [iter-for-elem-write], (16) a fn-typed local named like a top-level fn
// [call-resolve].
//--- the_roadmap_0c_emitter_gaps
// FILE: lib/conf.sv
import std.time.Duration

// [rs-default-path] A default naming a type the user's file does not import.
export struct Conf { d: Duration = Duration { nanos: 5L } }

// [type-identity] A handler named like a type of another module.
export effect Table {
    fn size() -> Int
}
export handler Circle() of Table {
    fn size() -> Int { return 3 }
}
// FILE: lib/shapes.sv
export struct Circle { r: Int }
// FILE: main.sv
import lib.conf
import lib.shapes
import std.throw

struct Node { id: Int, label: Str }
struct A { x: Int }
struct B { y: Int }
struct Rec canbe Mut { dead: Bool }
struct H<T> { skip: (x: T) -> Bool }
struct Tok { f: (x: Int) -> Int }

effect Box {
    fn get() -> Int
}
handler Boxing() of Box {
    fn get() -> Int { return 7 }
}

actor effect Bin {
    send fn go(v: A | B | None, done: Reply<Int>) => !v, !done
}
handler Binning() of Bin {
    mailbox { capacity: 4 }
    send fn go(v: A | B | None, done: Reply<Int>) {
        if v is A {
            done.send(v.x)
        } else {
            done.send(0)
        }
    }
}

fn skip(s: Str) [] -> Str => !s { return s }

// 0c item 1: a second `let n` after a loop that declared one.
fn rebind(m: Mut Map<Int, Node>, k: Int) [] -> Int => m: Mut {
    let seen = 0
    for id in keys(m) {
        let n = get(m, id)
        if !(n is None) { seen = seen + size(n.label) }
    }
    let n = remove(m, k)
    if n is None { return -1 }
    for _i in List(1, 2) { seen = seen + copy(n).id }
    return seen
}

// 0c item 4: a fn value called on its own result.
fn twice(f: (n: Int) -> Int, x: Int) [] -> Int => f {
    return f(f(x))
}

// 0c item 6: a `return` inside a lambda in a fn that may throw.
fn may(n: Int) [Throw<Str>] -> Int {
    if n < 0 { throw("neg") }
    let f = (x: Int) -> {
        if x > 1 { return x * 2 }
        return x
    }
    return f(n)
}

// 0c item 9: an `is` binding read later in its own `&&` chain.
fn same(b: Long?, x: Long) [] -> Bool {
    if b is Long known && known == x { return true }
    return false
}

// 0c item 16: a fn-typed local named like a top-level fn.
fn use_it<T>(h: H<T>, v: T) [] -> Bool => h, v {
    let skip = h.skip
    return skip(v)
}

fn main() [use, spawn] {
    use StdConsole()
    let m: Mut Map<Int, Node> = Mut Map()
    put(m, 1, Node { id: 1, label: "ab" })
    println("1: ${rebind(m, 1)}")
    let b = spawn Binning() on pool(1)
    let got = waitfor out: Reply<Int> { b.go(A { x: 9 }, out) }
    println("2: ${got}")
    use Boxing()
    println("3: ${get()}")
    println("4: ${twice((n: Int) -> n + 1, 1)}")
    let r = try { may(2) }
    when r {
        is Ok { println("6: ${r}") }
        is Thrown { println("6: thrown") }
    }
    println("9: ${same(3L, 3L)} ${same(None, 3L)}")
    let t = Tok { f: x -> x + 1 }
    let tf = t.f
    println("11: ${tf(2)}")
    let c = Conf {}
    println("12: ${c.d.nanos}")
    use Circle()
    let shape = Circle { r: 2 }
    println("13: ${size()} ${shape.r}")
    let xs: Mut List<Mut Rec> = Mut List(Mut Rec { dead: false })
    for a in xs {
        if !a.dead { a.dead = true }
    }
    println("14: ${(get(xs, 0) ?: unreachable!()).dead}")
    println("16: ${use_it(H<Int> { skip: (x: Int) -> x > 2 }, 3)} ${skip("s")}")
}
//= stdout
//= 1: 4
//= 2: 9
//= 3: 7
//= 4: 3
//= 6: 4
//= 9: true false
//= 11: 3
//= 12: 5
//= 13: 3 2
//= 14: true
//= 16: true s
