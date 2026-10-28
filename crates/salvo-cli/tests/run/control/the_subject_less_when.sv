// the subject-less `when` [when-condition] [rs-when-cond]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- when_cond

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
//= stdout
//= negative
//= zero
//= positive
//= sign -1 0 1
//= str hi
//= int 42
//= small
//=   medium branch
//= medium
//= large
//= positive ok 3
//= err bad
