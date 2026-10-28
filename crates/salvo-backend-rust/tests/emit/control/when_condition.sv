// The subject-less `when` [when-condition] [rs-when-cond].

//--- a_subjectless_when_emits_an_if_chain
// [when-condition] A condition chain with a mandatory `else`, in value and
// statement position, with `is` heads that narrow, a chain that returns from
// every branch, and one nested inside a subject `when`'s arm.
//
// [rs-when-cond] Rust has no subject-less `match`, so the chain lowers to
// `if`/`else if`/`else`. The mandatory `else` makes it total, so no
// `unreachable!()` filler and — in value position — no `else { None }`
// [expr-if-else-none].
// The whole of `classify`: an if/else-if chain, a plain `else`, no optional
// filler and no filler arm.
//~ emits(main.rs): pub fn classify(mut n: i32) -> String {
//~|     return if (n < 0i32) {
//~|         String::from("negative")
//~|     } else if (n == 0i32) {
//~|         String::from("zero")
//~|     } else {
//~|         String::from("positive")
//~|     };
//~| }
fn classify(n: Int) -> Str {
    return when {
        n < 0 { "negative" }
        n == 0 { "zero" }
        else { "positive" }
    }
}

fn sign(n: Int) -> Int {
    when {
        n < 0 { return -1 }
        n > 0 { return 1 }
        else { return 0 }
    }
}

fn describe(value: Str | Int) [Console] -> None => !value {
    when {
        // An `is` head still declares its binding at the top of the branch.
        //~ emits(main.rs): let mut s = match &value { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        value is Str s { println("str ${s}") }
        else { println("int ${value}") }
    }
}

fn label(n: Int) [Console] -> Str {
    let tag = when {
        n < 10 { "small" }
        n < 100 {
            println("  medium branch")
            "medium"
        }
        else { "large" }
    }
    return tag
}

fn nested(o: Ok Int | Err Str) [Console] -> None => !o {
    when o {
        is Ok {
            when {
                o > 0 { println("positive ok ${o}") }
                else { println("nonpositive ok ${o}") }
            }
        }
        is Err {
            println("err ${o}")
        }
    }
}

fn main() [use] -> None {
    use StdConsole
    println(classify(-5))
    println(classify(0))
    println(classify(7))
    println("sign ${sign(-3)} ${sign(0)} ${sign(9)}")
    describe("hi")
    describe(42)
    println(label(5))
    println(label(50))
    println(label(500))
    nested(ok(3))
    nested(err("bad"))
}
