// the widening check `^` [qual-lift]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- widening

import std.throw

fn wrapped(n: Int) [Throw<Str>] -> Ok Int | Err Str {
    if n < 0 {
        throw("negative")
    }
    if n == 0 {
        return err("zero")
    }
    return ok(n)
}

fn limit(n: Int) [Throw<Int>] -> Int {
    if n > 4 {
        throw(n)
    }
    return n
}

fn describe(n: Int) [Console] -> None {
    let nested = try { wrapped(n) }
    when nested {
        is ^Ok {
            when nested {
                is Ok {
                    println("value ${nested}")
                }
                is Err {
                    println("error ${nested}")
                }
            }
        }
        is Thrown {
            println("thrown ${nested}")
        }
    }
}

fn main() [use] -> None {
    use StdConsole
    describe(7)
    describe(0)
    describe(-1)
    let mixed = try {
        let a = wrapped(1)
        limit(9)
    }
    when mixed {
        is Ok {
            println("mixed ok ${mixed}")
        }
        is ^Thrown {
            when mixed {
                is Str {
                    println("message text ${mixed}")
                }
                is Int {
                    println("message number ${mixed}")
                }
            }
        }
    }
}
//= stdout
//= value 7
//= error zero
//= thrown negative
//= message number 9

//--- lifting_mut_converts_the_builder
// [qual-lift] [str-drop-mut] `^Mut` is the one lift that changes a
// representation: Kotlin's `Mut Str` is a `StringBuilder`. A read of the
// lifted subject, or a lifted binding, is the plain `Str` by a `Mut` drop
// (the pipeline record D2 `Test`, 2026-10-30). Before, the `if` form cast the
// builder to `String` (a `ClassCastException`) and the `when` form passed
// the builder where a `String` was wanted (refused by kotlinc).

fn size_of(s: Str) [] -> Int {
    return size(s)
}

fn main() [use] -> None {
    use StdConsole()
    let m: Mut Str | Int = Mut Str("hi")
    if m is ^Mut {
        println("if ${size_of(m)} ${m == "hi"}")
        let copied = m
        println(copied)
    }
    when m {
        is ^Mut { println("when ${size_of(m)} ${m == "hi"}") }
        is Int { println("int") }
    }
    when m {
        is ^Mut s { println("bound ${size_of(s)} ${s == "hi"}") }
        is Int { println("int") }
    }
    if m is ^Mut t {
        println("if bound ${size_of(t)}")
    }
}
//= stdout
//= if 2 true
//= hi
//= when 2 true
//= bound 2 true
//= if bound 2
