// [expr-wrap] [expr-infix] Where a line may break, by tier, and what the
// pick binds (user decision 2026-11-12): `|>`, `?>`, `?:`, `||` and `&&`
// may begin a continuation line, `||`/`&&` binding at their own precedence
// there; the tighter operators continue no line, so `a` then `-1` is two
// statements, and inside parentheses any line breaks. `?:` sits between
// the pipe and `||`: `n ?: 0 > 5` is `n ?: (0 > 5)`.
//
// Compile-and-run data ([test-run-data]).

//--- a_leading_operator_binds_at_its_precedence
fn pick(x: Int?) -> Int {
    return x ?: 10
        |> _ + 1
}

fn compared(n: Int?) -> Int | Bool {
    return n ?: 0 > 5
}

fn either(m: Bool?, ok: Bool) -> Bool {
    return m ?: ok
        || false
}

fn last(n: Int) -> Int {
    let k = n
    -1
    return k
}

fn main() [use] {
    use StdConsole()
    let a = (1
        + 2 * 3)
    let b = (2 *
        3 + 1)
    let c = 2 ** 3
    let f = a > 6
        && b == 7
        || false
    let g = false && true
        || true
    println("${a} ${b} ${c} ${f} ${g}")
    // `x ?: 10` then `|> _ + 1` pipes the whole pick.
    println("${pick(5)} ${pick(None)}")
    // The pick's right side is `0 > 5`; in parentheses the pick is compared.
    let n: Int? = None
    println("${compared(7)} ${compared(None)} ${(n ?: 9) > 5}")
    // `||` on the line after a pick is in its right side.
    println("${either(None, false)} ${either(false, true)} ${either(None, true)}")
    // `x ?: y || z` and `x ?: y + 1` take the whole right side.
    let p: Bool? = None
    println("${p ?: false || true} ${n ?: 1 + 1}")
    println("${last(4)}")
    let s: Int | Str = "s"
    let wide = (s
        is Int)
        || s is "s"
    println("${wide}")
}
//= stdout
//= 7 7 8 true true
//= 6 11
//= 7 false true
//= false false true
//= true 2
//= 4
//= true
