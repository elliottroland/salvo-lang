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
