// [ast-print] [syntax-lowered] One use of each piece of sugar the parser
// keeps as written, in each position a formatter must print it back from:
// a statement, a condition, a call argument, a lambda body. Parsed only
// (`print_tests.rs`), never checked.

/// A doc comment survives the printing.
fn as_qual(x: Int, xs: List<Int>) -> None {
    let a = x as! Positive
    let b = x as? Positive
    if x as? Positive is Int {
        print(x as! Positive)
    }
    let f = (y) -> y as! Positive
    xs.each((y) -> {
        let c = y as? Positive NonZero
    })
}
