// [expr-wrap] A binary operator may begin a continuation line, and binds at
// its own precedence with the expression so far, as on one line: the
// newline is layout. `--b` and `++b` on a line of their own are prefix
// steps (one token each), so they start statements; `- -b` continues.
//
// Compile-and-run data ([test-run-data]).

//--- a_leading_operator_binds_at_its_precedence
fn pick(x: Int?) -> Int {
    return x ?: 10
        + 1
}

fn main() [use] {
    use StdConsole()
    let a = 1
        + 2 * 3
    let b = 2
        * 3 + 1
    let c = 2 *
        3
    let d = 10
        - 2
        - 3
    let e = 2
        ** 3
    let f = a > 6
        && b == 7
        || false
    println("${a} ${b} ${c} ${d} ${e} ${f}")
    // `x ?: 10 + 1` is `x ?: (10 + 1)`, on one line or two.
    println("${pick(5)} ${pick(None)}")
    let n = 3
    let m = n
    --n
    let k = m
        - -n
    ++n
    println("${m} ${n} ${k}")
    let s: Int | Str = "s"
    let wide = s
        is Int
        || s
        is "s"
    println("${wide}")
}
//= stdout
//= 7 7 6 5 8 true
//= 5 11
//= 3 3 5
//= true
