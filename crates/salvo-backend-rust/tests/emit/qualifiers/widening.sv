// The widening check `^` [qual-lift].

//--- widening_materializes_the_peel
// [qual-lift] `^` tests the arm *and* removes the claim, so a branch can
// `when` the union inside a qualified one — the shape `try` outcomes produce.
//
// [qual-lift] The peel is *materialized*: the widened value is bound to a
// shadowing local, so a nested `when` scrutinizes the inner union rather
// than the wrapper it came out of. Without it the inner match reads the
// outer arm and the second branch is dead code — which is what this program
// caught while `^` was being built.
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
    // The widened value peeled into a local, by reference; the nested `when`
    // then tests the peeled (inner) value.
    //~ emits(main.rs):     if matches!(nested, crate::unions::Union2::U1(_)) {
    //~|         let mut nested_1 = match &nested { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    //~|         if matches!(nested_1, crate::unions::Union2::U1(_)) {
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
