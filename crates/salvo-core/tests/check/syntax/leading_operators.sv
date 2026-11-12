// [expr-wrap] A binary operator may begin a continuation line, or end the
// line it continues: both are layouts of one expression (user decision
// 2026-11-12). It binds at its own precedence with the expression so far,
// as on one line (the values are the run data's,
// `run/operators/a_line_led_by_a_binary_operator.sv`).

//--- every_binary_operator_may_lead_a_line
struct P {
    x: Int
}

type Shape = Int
    | Str
    | Bool

fn ops(a: Int, b: Int, ok: Bool, v: Int | Str | Bool, p: P?) [] -> Int {
    let sum = a
        + b
        - 1
        * 2
        / 3
        % 4
    let power = a
        ** 2
    let _test = a
        < b
        == ok
        != false
        || a
        <= b
        && a
        >= b
        || a
        > b
    //~ type[Bool]
    type?(_test)
    let tested = v
        is Int
    let untested = v
        !is Str
    let union = v
        is Int
        | Str
    let trailing = v is Int |
        Str
    let xs = List(a, b)
    let _qualified = xs
        as? NonEmpty
    //~ type[(NonEmpty List<Int>)?]
    type?(_qualified)
    let shape: Shape = v
    let _kept: Int
        | Str = 1
    if tested
        && untested
        || union
        && trailing {
        return sum
    }
    while ok
        && a
        > b {
        return power
    }
    when shape {
        is Int
            | Str {
            return 0
        }
        is Bool { }
    }
    // A `}` inside brackets is no line's end.
    let grouped = (if ok { 1 } else { 2 })
        + 3
    let called = apply((x) -> { x }, a)
        + 1
    let px = (P { x: 1 }).x
        + 1
    return a
        + grouped
        + called
        + px
        + (p?.x ?: 0)
}

fn apply(f: (Int) -> Int, x: Int) [] -> Int {
    return f(x)
}

//--- a_prefix_step_starts_its_own_statement
// `--` and `++` are one token each: a line beginning with one is a prefix
// step, its own statement, where `- -` continues the line above.
fn steps(n: Int) [] -> Int {
    let i = n
    let j = i
    --i
    ++i
    let k = j
        - -i
    //~ type[Int]
    type?(k)
    return k
}

//--- a_line_after_a_wrapped_link_continues_the_whole
// A line-leading `.` brackets everything before it, and an operator leading
// the line after it continues that whole, `(xs.size()) - 1`.
fn last(xs: List<Int>) [] -> Int {
    let n = xs
        .size()
        - 1
    //~ type[Int]
    type?(n)
    return n
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

//--- a_negation_as_a_block_s_value_needs_parentheses
// A block's value is its last expression, so a negation there, after another
// statement, continues that statement unless it is parenthesized: here the
// first `if`'s branch is `n += 1 - 1`, of no value.
fn signs(ok: Bool) [] -> Int {
    let n = 0
    let _continued = if ok {
        n += 1
        -1
    } else {
        1
    }
    //~ type[Int?]
    type?(_continued)
    let negated = if ok {
        n += 1
        (-1)
    } else {
        1
    }
    //~ type[Int]
    type?(negated)
    return negated
}

//--- a_trailing_lambda_s_body_starts_its_own_line
// After a `(x) -> ...` argument the next line is the lambda body's first
// statement, not a continuation, `-e` included.
fn apply(x: Int, f: (Int) -> Int) [] -> Int {
    return f(x)
}

fn negated(n: Int) [] -> Int {
    return apply(n, (e) -> ...)
    -e
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

//--- a_line_after_a_brace_does_not_continue
// After a `}` — a block's, a `when`'s, a struct literal's, an `if` used
// as a value — a line that begins with a binary operator would continue a
// statement that may be over: it is refused, `-` included, rather than
// read as a new statement (a negation) or as a continuation.
struct P {
    x: Int
}

fn after(ok: Bool, n: Int, v: Int | Str) [] -> Int {
    if ok {
        return 1
    }
    //~ error[expr-wrap:after-brace]
    && ok
    let w = when v {
        is Int { 1 }
        is Str { 2 }
    }
    //~ error[expr-wrap:after-brace]
    - 1
    let s = if ok { 1 } else { 2 }
    //~ error[expr-wrap:after-brace]
    + n
    let p = P { x: 1 }
    //~ error[expr-wrap:after-brace]
    is P
    for i in List(1, 2) {
        let _i = i
    }
    //~ error[expr-wrap:after-brace]
    -n
    return w + s + p.x
}
