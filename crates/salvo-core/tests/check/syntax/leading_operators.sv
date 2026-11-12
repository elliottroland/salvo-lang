// [expr-wrap] [expr-infix] The operators in three tiers by where a line may
// break (user decision 2026-11-12). Tier 1 (`|>`, `?>`, `?:`) and tier 2
// (`||`, `&&`) may begin a continuation line or end the line they continue;
// tier 3 (equality, comparison, arithmetic, `**`, `is`, `!is`, `as!`,
// `as?`, an `is` head's `|`) and a union type's `|` do neither outside
// brackets, and inside `(…)` break anywhere. The values are the run data's,
// `run/operators/a_line_led_by_a_binary_operator.sv`.

//--- tiers_one_and_two_lead_or_trail_a_line
fn ops(a: Int, b: Int, ok: Bool, done: Bool, n: Int?) [] -> Int {
    let _led = ok
        && done
        || a > b
    let _trailed = ok &&
        done ||
        a > b
    //~ type[Bool]
    type?(_led)
    //~ type[Bool]
    type?(_trailed)
    let picked = n
        ?: 0
    let picked_trailing = n ?:
        0
    let piped = a
        |> _ + 1
    let piped_trailing = a |>
        _ + 1
    let _safe = n
        ?> _ + 1
    //~ type[Int?]
    type?(_safe)
    if ok
        && done {
        return picked + picked_trailing
    }
    while ok &&
        done {
        return piped + piped_trailing
    }
    return a
}

//--- the_pick_binds_between_the_pipe_and_or
// `?:` is looser than `||` and every tighter operator, so its right side
// takes a comparison, and a pick after `&&` picks from the whole `&&`.
fn picks(n: Int?, m: Bool?, ok: Bool) [] -> Int {
    let _compared = n ?: 0 > 5
    //~ type[Int | Bool]
    type?(_compared)
    let _grouped = (n ?: 0) > 5
    //~ type[Bool]
    type?(_grouped)
    let summed = n ?: 1 + 1
    //~ type[Int]
    type?(summed)
    let _either = m ?: ok || false
    //~ type[Bool]
    type?(_either)
    // `||` on the line after a pick is in its right side.
    let _wrapped = m ?: ok
        || false
    //~ type[Bool]
    type?(_wrapped)
    // A pick leading its line takes everything before it, a pipe included.
    let _after_pipe = n ?> _ + 1
        ?: 0
    //~ type[Int]
    type?(_after_pipe)
    return summed
}

//--- tier_three_continues_no_line
// A line that begins with a tier-3 operator after an expression is refused
// (it would begin a statement of its own); its operand is read as one.
fn sums(a: Int, b: Int, v: Int | Str) [] -> Int {
    let s = a
    //~ error[expr-wrap:tier3-leading]
    + b
    let p = a
    //~ error[expr-wrap:tier3-leading]
    * b
    let q = a
    //~ error[expr-wrap:tier3-leading]
    ** 2
    let e = a
    //~ error[expr-wrap:tier3-leading]
    == b
    let l = a
    //~ error[expr-wrap:tier3-leading]
    < b
    let _t = v
    //~ error[expr-wrap:tier3-leading]
    is Int
    let _u = v
    //~ error[expr-wrap:tier3-leading]
    !is Str
    return s + p + q + e + l
}

//--- a_trailing_tier_three_operator_is_refused
// One that ends its line is refused, and its operand read from the next.
fn trailing(a: Int, b: Int, v: Int | Str, xs: List<Int>) [] -> Int {
    //~ error[expr-wrap:tier3-trailing]
    let s = a +
        b
    //~ error[expr-wrap:tier3-trailing]
    let _e = a ==
        b
    //~ error[expr-wrap:tier3-trailing]
    let _p = a **
        2
    //~ error[expr-wrap:tier3-trailing]
    let _t = v is
        Int
    //~ error[expr-wrap:tier3-trailing]
    let _u = v is Int |
        Str
    //~ error[expr-wrap:tier3-trailing]
    let _q = xs as?
        NonEmpty
    return s
}

//--- a_minus_line_is_a_statement_of_its_own
// `-` is a negation where a line begins with it, so `a` then `-1` is two
// statements, and a block's last line `-1` is its value.
fn signs(ok: Bool, a: Int) [] -> Int {
    let n = 0
    let k = a
    -1
    //~ type[Int]
    type?(k)
    let value = if ok {
        n += 1
        -1
    } else {
        1
    }
    //~ type[Int]
    type?(value)
    return value + k
}

//--- an_is_head_s_bar_does_not_begin_a_line
// Outside brackets the head's `|` stays on its line: one beginning the
// next is refused, and the head read on.
fn heads(v: Int | Str | Bool, w: Int | Str | Bool) [] -> Int {
    if v is Int
        //~ error[expr-wrap:tier3-leading]
        | Str {
        return 1
    }
    when w {
        is Int
            //~ error[expr-wrap:tier3-leading]
            | Str {
            return 2
        }
        is Bool { }
    }
    return 0
}

//--- inside_parentheses_a_line_breaks_anywhere
fn grouped(a: Int, b: Int, v: Int | Str | Bool) [] -> Int {
    let s = (a
        + b
        * 2)
    let trailing = (a +
        b)
    let tested = (v is Int
        | Str)
    let tested_trailing = (v is Int |
        Str)
    let compared = (a
        == b)
    // A struct literal's fields are read as a bracket's contents.
    let p = P { x: a
        + b }
    if tested && tested_trailing && compared {
        return s + p.x
    }
    return trailing
}

struct P {
    x: Int
}

//--- a_union_type_breaks_inside_parentheses
// A union written across lines is in parentheses, in every type position.
struct Holder {
    held: (Int
        | Str),
    other: (
        Bool
        | Str
    )
}

type Shape = (
    Int
    | Str
)

type Mark = (
    Holder
    | Bool
) : ToStr<self> by auto

type Body = (Int
    | Bool) {
    fn size(b: Body) [] -> Int {
        return 1
    }
}

effect Store {
    fn get(key: (Str
        | Int)) -> (Int
        | None)
    => key
}

fn takes(v: (Int
    | Str
    | Bool), xs: List<(Int
    | Str)>) [] -> (Int
    | Str) {
    let _shape: Shape = 1
    let _kept: (Int
        | Str) = 1
    let _listed: List<(
        Int
        | Bool
    )> = List()
    let tested = v is (Int
        | Str)
    let _untested = v !is (
        Int
        | Bool
    )
    when v {
        is (Int
            | Str) {
            return 0
        }
        is Bool { }
    }
    if tested {
        return xs.size()
    }
    return "s"
}

//--- a_union_type_does_not_break_outside_parentheses
type Led = Int
    //~ error[expr-wrap:union-type-line]
    | Str

//~ error[expr-wrap:union-type-line]
type Trailing = Int |
    Str

//~ error[expr-wrap:union-type-line]
fn f(v: Int | Str) [] -> Int |
    Str {
    return 1
}

//--- a_line_after_a_wrapped_link_continues_the_whole
// A line-leading `.` brackets everything before it, and a tier-2 operator
// leading the line after it continues that whole, `(xs.is_empty()) || ok`.
fn last(xs: List<Int>, ok: Bool) [] -> Bool {
    let empty = xs
        .is_empty()
        || ok
    //~ type[Bool]
    type?(empty)
    return empty
}

//--- an_escape_without_a_value_is_not_continued
// A `return`, `break` or `continue` with nothing after it on its line ends
// there: the next line is a statement of its own.
fn early(ok: Bool, n: Int) [] -> None {
    for i in List(1, 2) {
        if ok {
            continue
        }
        let _i = i
    }
    while ok {
        break
        //~ warning[expr-dead-code:unreachable]
        -n
    }
    for _j in List(1) {
        continue
        //~ warning[expr-dead-code:unreachable]
        -n
    }
    return
    //~ warning[expr-dead-code:unreachable]
    -n
}

//--- a_trailing_lambda_s_body_starts_its_own_line
// After a `(x) -> ...` argument the next line is the lambda body's first
// statement, not a continuation.
fn apply(x: Bool, f: (Bool) -> Bool) [] -> Bool {
    return f(x)
}

fn negated(ok: Bool) [] -> Bool {
    return apply(ok, (e) -> ...)
    !e
}

//--- a_subject_less_when_s_arms_keep_their_conditions
// An arm's condition after the arm before it follows that arm's `}`, so
// it starts an arm, `-x` included.
fn sign(x: Int) [] -> Int {
    return when {
        x > 0 { 1 }
        -x > 0 { -1 }
        else { 0 }
    }
}

//--- after_a_brace
// After a `}` — a block's, a `when`'s, a struct literal's, an `if` used as
// a value — a line that begins with `||` or `&&` would continue a statement
// that may be over: it is refused. A tier-1 operator continues there (the
// value's), a `-` begins a negation, and a tier-3 one is refused as
// anywhere.
struct P {
    x: Int
}

fn after(ok: Bool, n: Int, v: Int | Str, m: Int?) [] -> Int {
    if ok {
        return 1
    }
    //~ error[expr-wrap:after-brace]
    && ok
    let w = when v {
        is Int { m }
        is Str { m }
    }
        ?: 0
    //~ type[Int]
    type?(w)
    let s = if ok { 1 } else { 2 }
    //~ error[expr-wrap:tier3-leading]
    + n
    let p = P { x: 1 }
    //~ error[expr-wrap:tier3-leading]
    is P
    for i in List(1, 2) {
        let _i = i
    }
    -n
    let b = if ok { true } else { false }
    //~ error[expr-wrap:after-brace]
    || ok
    return w + s + p.x + (if b { 1 } else { 0 })
}
