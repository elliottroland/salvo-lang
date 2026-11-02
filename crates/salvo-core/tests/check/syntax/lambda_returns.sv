// [fn-lambda] A lambda's parameters are always parenthesized, and a block
// body answers its written return type, else the expected fn type's, else
// `None` — as a fn with no `->` does (user decision 2026-10-30).

//--- a_block_body_answers_its_written_return_type
struct Point {
    x: Int
}

struct Finished {}

fn apply(f: (n: Int) -> Int, v: Int) -> Int {
    return f(v)
}

fn main() [use] -> None {
    use StdConsole
    // Written: the call site sees it.
    let double = (n: Int) -> Int { return n * 2 }
    let _four: Int = double(2)
    // Expected: the fn type at the position says it.
    let _six = apply((n) -> { return n * 3 }, 2)
    // An expression body answers its expression; a type before a struct
    // literal is still the literal's.
    let _p = (n: Int) -> Point { x: n }
    // A fieldless struct literal is the body when parenthesized.
    let _done = () -> (Finished {})
    // No `return` value, no annotation: `None`.
    let say = (s: Str) -> { println(s) }
    say("hi")
}

//--- an_unannotated_block_body_answers_none
fn main() -> None {
    let _double = (n: Int) -> {
        //~ error[fn-syntax:return-mismatch]
        return n * 2
    }
}

//--- a_written_return_type_is_checked_at_the_call
fn main() -> None {
    let double = (n: Int) -> Int { return n * 2 }
    //~ error[var-let-infer:annotation-mismatch]
    let _s: Str = double(4)
}

//--- the_single_parameter_shorthand_is_gone
fn main() -> None {
    //~ error[syntax-expected:expression]
    //~ error[name-ident-resolve:unknown]
    let _f = x -> x + 1
}

//--- the_brace_lambda_is_gone
fn main() -> None {
    // Read as a struct literal, which `->` cannot continue.
    //~ error[syntax-expected:token]
    let _f = { x: Int -> x + 1 }
//~ error[syntax-expected:item]
}
