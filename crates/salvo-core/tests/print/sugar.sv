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

fn compound(p: Mut Point, xs: List<Int>) -> None {
    let n = 1
    n += 2
    n -= 1 + 2
    n *= -3
    n /= n as! Positive
    n **= 2
    p.x += 1
    if n > 0 {
        p.y -= n
    }
    xs.each((y) -> {
        n += y
    })
}

fn is_forms(s: Str, x: Int | Str | None, xs: List<Int | Str>) -> Bool {
    let a = x !is Str
    let b = s is "json" | "text"
    let c = x !is Int | Str
    if x !is None && s is "a" | "b" | "c" {
        print(x !is Int, s !is "a" | "b")
    }
    while x !is Int {
        return false
    }
    let f = (y) -> y !is Int | Str
    assert!(x as? NonNegative !is None)
    return !(x !is Str) || x is Int | None
}
