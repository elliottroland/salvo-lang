// The parentheses written around an `is` head's types are kept, on one line
// and broken; none are added where none were written and the parser does
// not need any.
fn tests(m: Int | Str | Bool) -> Bool {
    let bare = m is Int | Str
    let one = m is (Int | Str)
    let single = m is (Int)
    let not_one = m !is (Int | Str)
    let not_single = m !is (Int)
    let doubled = m is ((Int | Str))
    let doubled_single = m !is ((Int))
    if m is (Int n) {
        return n > 0
    }
    // Broken inside its parentheses: one test per line, `|` leading, the
    // `)` on a line of its own, however the author broke it.
    let broken = m is (Int
        | Str)
    let trailing = m !is (Int |
        Str | Bool)
    let doubled_broken = m is ((Int
        | Str))
    // Inside brackets a head needs no parentheses to break, and gets none.
    let inside = keep(m is Int
        | Str)
    let inside_written = keep(m !is (Int
        | Str))
    when m {
        is (Int | Str) { return true }
        is ((Bool)) { return false }
    }
    return bare && one && single && not_one && not_single && doubled && doubled_single
}

fn arms(m: Int | Str | Bool) -> Int {
    when m {
        is Int { return 0 }
        is (Str
            | Bool) { return 1 }
    }
}

fn keep(b: Bool) -> Bool {
    return b
}
