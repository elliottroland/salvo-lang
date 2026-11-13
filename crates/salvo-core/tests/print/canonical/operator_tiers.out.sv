// [fmt-width] [fmt-author-breaks] [expr-wrap] Where each tier of operators
// breaks. The pipes and the pick (tier 1) and `||`/`&&` (tier 2) begin their
// continuation line: an author's trailing ones print in the leading form,
// a tier-2 run inside a tier-2 run one level further in. After a `}`
// outside brackets a tier-2 operator ends its line, as a line it began
// there would not continue. The tighter operators (tier 3) and a union's
// `|` break nowhere outside brackets: a line too wide with one stays long,
// and inside parentheses the author's breaks are kept, the operator
// leading (`operator_tiers.out.sv`). Parsed only (`print_tests.rs`), never
// checked.
type Arms = FirstArm | SecondArm | ThirdArm

type Json = (
    Null
    | Flag
    | Number
    | Text
    | Items
)

fn f(a: Int, b: Int, ok: Bool, done: Bool, m: Int? | Str, xs: List<Int>) -> Int {
    let both = ok
        && done
    let either = ok
        || done
    let picked = m
        ?: a
    let piped = a
        |> plus(_, 1)
        ?> twice(_)
    if ok
            && done
        || a > b {
        return a
    }
    // A run of `&&` inside a run of `||`, both broken by the author: the
    // inner run sits one level further in.
    let mixed = ok
            && done
        || a == b
    // The pick binds looser than `||`, so `||` on the line after it is in
    // its right side, and a pick after `||` takes the whole `||`.
    let wider = m ?: done
        || ok
    let narrower = done || ok
        ?: false
    // Tier 3 inside parentheses: the author's breaks, the operator leading.
    let sum = (a
        + b
        - 1)
    let tested = (m is Int
        | Str)
    let long = first_summand_of_the_total(a) * second_summand_of_the_total(b) + third_summand_of_the_total(a) * fourth(b)
    let deep = check(
        first_condition_of_the_test(a) && second_condition_of_the_test(b) && third_condition_of_the_test(ok)
    )
    let compared = (m ?: 0) > 5
    // An `is` head the author broke in its parentheses prints one test per
    // line, and a `when` arm's the same; on one line its parentheses stay.
    let head = m is (
        Int
        | Str
    )
    let not_head = m !is (
        Int
        | Str
    )
    let flat_head = m is (Int | Str)
    when m {
        is (
            Int
            | Str
        ) {
            return 1
        }
    }
    // A pick leading its line takes the pipe before it; one in a pipe's
    // right side stays on the pipe's line.
    let after = a |> plus(_, 1)
        ?: 0
    let inside = a |> (plus(_, 1) ?: 0)
    // Too wide, the pipe breaks; the pick in its right side does not.
    let wide = first_operand_of_the_pipe(a, b)
        |> the_long_named_step_of_the_pipe(_, a) ?: the_long_named_fallback_of_the_pick(b, a)
    // A struct literal's fields are read as a bracket's contents.
    let point = Point { x: a
        + b }
    // A trailing `(x) -> ...` argument's body starts on the next line, so
    // the operator ends its line, as after a `}`.
    let counted = any(map(xs, (e) -> ...)) ||
        ok
    let d = e
    // A block inside a broken chain sits one level in with the chain
    // (its arms print as blocks, as everywhere).
    return when m {
            is Int {
                true
            }
            is Str {
                false
            }
        } ||
        ok
}
