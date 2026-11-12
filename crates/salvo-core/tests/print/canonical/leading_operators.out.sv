// [fmt-width] [fmt-author-breaks] [expr-wrap] A broken binary operator
// begins its continuation line: an author's trailing operators print in the
// leading form, each operator family, and a chain inside a chain one level
// further in. After a `}` outside brackets the operator ends its line, as
// a line it began there would not continue (`leading_operators.out.sv`).
// Parsed only (`print_tests.rs`), never checked.
type Arms = FirstArm
    | SecondArm
    | ThirdArm

fn f(a: Int, b: Int, ok: Bool, done: Bool, m: Int | Str, xs: List<Int>) -> Int {
    let both = ok
        && done
    let either = ok
        || done
    let same = a
        == b
    let less = a
        < b
    let sum = a
        + b
        - 1
    let product = a
        * b
        / 2
        % 3
    let tested = m is Int
        | Str
    if ok
            && done
        || a > b {
        return a
    }
    while a
        < b {
        return b
    }
    // A chain of a looser level holds one of a tighter level, broken by its
    // author: the inner chain sits one level further in.
    let mixed = a
            + b
                * a
        || ok
    // The pick binds looser than `+`, so the sum is its right side.
    let picked = m ?: a
        + 1
    let held = Point { x: 1 } ==
        p
    let wrapped = (Point { x: 1 })
        == p
    let long = first_summand_of_the_total(a) * second_summand_of_the_total(b)
        + third_summand_of_the_total(a) * fourth(b)
    let deep = check(
        first_condition_of_the_test(a) && second_condition_of_the_test(b) && third_condition_of_the_test(ok)
    )
    // A trailing `(x) -> ...` argument's body starts on the next line, so
    // the operator ends its line, as after a `}`.
    let counted = count(map(xs, (e) -> ...)) +
        1
    let d = e
    // A block inside a broken chain sits one level in with the chain
    // (its arms print as blocks, as everywhere).
    return when m {
            is Int {
                1
            }
            is Str {
                2
            }
        } +
        a
}
