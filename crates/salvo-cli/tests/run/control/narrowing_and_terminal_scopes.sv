// an `else` inside a `when` arm, and terminal scopes
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [ir-narrow] An `else` reads a name its branch's test narrowed as what
// remains, even inside an enclosing `when` arm: the else of `if one is A`
// in an `is Errs` arm used that arm's index (a `ClassCastException` on
// Kotlin, a type error on Rust). And a kept lambda's capture of a borrowed
// parameter is an owned clone, passed on borrowed (a type error on Rust).
//--- an_else_narrows_by_its_own_test_inside_a_when_arm
struct A { x: Str }
struct B { y: Str }
struct C { z: Str }
type U = A | B | C

fn msg(u: U) [] -> Str => u {
    when u {
        is A { return "a ${u.x}" }
        is B { return "b ${u.y}" }
        is C { return "c ${u.z}" }
    }
}

fn make() [] -> Ok Int | Errs NonEmpty List<U> {
    let b: U = B { y: "1" }
    let c: U = C { z: "2" }
    let a: U = A { x: "3" }
    let all: Mut List<U> = Mut List<U>()
    add(all, b)
    add(all, c)
    add(all, a)
    if all is NonEmpty {
        return Errs(all)
    }
    return Ok(1)
}

struct Found { values: List<Str> }

fn eval<Q>(f: once () -> Q) [] -> Q => !f {
    return f()
}

fn pick(found: Found, i: Int) [] -> Str => found {
    return copy(get(found.values, i) ?: unreachable!())
}

fn both(found: Found) [] -> Str => found {
    let a: Str = eval(() -> {
        return pick(found, 0)
    })
    let b: Str = eval(() -> {
        return pick(found, 1)
    })
    return "${a}${b}"
}

fn main() [use] -> None {
    use StdConsole()
    let r = make()
    when r {
        is Ok { println("ok") }
        is Errs {
            for e in r {
                let one = copy(e)
                if one is A {
                    println(one.x)
                } else {
                    println(msg(one))
                }
            }
        }
    }
    println(both(Found { values: List("x", "y") }))
}
//= stdout
//= b 1
//= c 2
//= 3
//= xy

// [expr-terminal-scope] A trailing `...` makes the rest of the enclosing
// block the construct's body — `if` (with an optional inline `else`) and
// loops.
//--- terminal_scopes
fn label(x: Str?) [Console] -> None {
    if x is Str ... else {
        println("absent")
    }
    println("present ${x}")
}
fn count(xs: List<Int>) [Console] -> None {
    let total = 0
    for n in xs ...
    total = total + n
    println("running ${total}")
}
fn main() [use] {
    use StdConsole()
    label("here")
    label(None)
    count(List(1, 2, 3))
}
//= stdout
//= present here
//= absent
//= running 1
//= running 3
//= running 6

// [is-narrow-guard] After an `if` or a `when`, the join of what the paths
// that fall through narrow a place to holds (user decision 2026-10-30): the
// then branch alone falls through, two branches do, and a `when` arm leaves.
//--- narrowing_after_the_paths_that_fall_through
struct A { n: Int }

fn then_falls(x: A | Str) [] -> Int {
    if x is A {
        let _seen = 1
    } else {
        return 0
    }
    return x.n
}

fn two_fall(x: A | Str | Int) [] -> Str {
    if x is A {
        let _a = 1
    } elif x is Int {
        let _i = 2
    } else {
        return "str"
    }
    return when x {
        is A { "a ${x.n}" }
        is Int { "int ${x}" }
    }
}

fn when_falls(v: A | Str | Int) [] -> Int {
    when v {
        is A | Int { }
        is Str { return -1 }
    }
    return when v {
        is A { v.n }
        is Int { v }
    }
}

fn main() [use] -> None {
    use StdConsole()
    println("${then_falls(A { n: 4 })} ${then_falls("s")}")
    println("${two_fall(A { n: 1 })} ${two_fall(5)} ${two_fall("q")}")
    println("${when_falls(A { n: 2 })} ${when_falls(7)} ${when_falls("q")}")
}
//= stdout
//= 4 0
//= a 1 int 5 str
//= 2 7 -1
