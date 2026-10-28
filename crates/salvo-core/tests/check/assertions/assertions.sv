// Assertions [assert-op] [assert-fn] [assert-narrow] (user decisions
// 2026-09-23, ASSERTIONS.md's A-1…A-7).
//
// Two forms, one idea: a place where the program states a fact it cannot
// prove, and fails if it is wrong (the postfix `!` presence assertion was
// removed 2026-10-14; presence is `?: unreachable!(…)`). `assert!(cond)`
// asserts a condition — and **narrows** when the condition is an `is` test,
// which is what makes it more than a check — and `unreachable!()` asserts that
// a path is not taken.

// ===== [assert-fn] `todo!()` and dead code =====

// `todo!()` warns at every site, with or without a message, and does not
// reject the program.
fn stub() -> Int {
    //~ warning[assert-fn:todo]
    todo!()
}

fn stub_msg() -> Int {
    //~ warning[assert-fn:todo]
    todo!("later")
}

// A statement after a diverging one (`return`, `unreachable!()`, `todo!()`)
// is unreachable: a warning on the first such statement, not an error.
fn after_return() -> Int {
    return 1
    //~ warning[expr-dead-code:unreachable]
    let _x = 2
}

// After `unreachable!()` too.
fn after_unreachable() -> Int {
    unreachable!()
    //~ warning[expr-dead-code:unreachable]
    let _y = 2
}

// Reachable code does not warn.
fn live() -> Int {
    let z = 1
    return z
}

// ===== [loop-never] `while true` with no break =====

// A `while true {}` with no `break` targeting it never terminates, so it is
// `Never`: a fn whose body ends in one needs no return…
fn spins() -> Int {
    while true {
    }
}

// …and code after it is unreachable: a dead-code warning (not an error).
fn after_spinning() -> Int {
    while true {
    }
    //~ warning[expr-dead-code:unreachable]
    let _x = 3
}

// A `break` targeting the `while true` lets control out, so it is **not**
// `Never`: a `-> Int` fn whose only tail is that loop is then missing a
// return.
//~ error[fn-must-return:missing-return]
fn breaks(n: Int) -> Int {
    while true {
        if n > 0 { break }
    }
}

// A `break` in a *nested* loop targets the inner loop, so the outer `while
// true` still never terminates — `Never`, no return needed.
fn nested(flag: Bool) -> Int {
    while true {
        while flag {
            break
        }
    }
}

// [assert-op] The replacement for a value, `?: unreachable!(…)`, still needs
// something that can be absent: on a never-absent value the pick keeps its only
// arm, refused like any pointless `?:`.
fn pick_on_a_never_absent_value() -> Int {
    let n = 3
    //~ error[op-elvis:single-arm]
    return n ?: unreachable!()
}

// The form it exists for works: an optional operand.
fn pick_on_an_optional(v: Int?) -> Int => v {
    return v ?: unreachable!()
}

// ===== [assert-fn] [assert-narrow] `assert!` =====

// [assert-narrow] The point of the form: an `is` test asserted at run time
// narrows for the rest of the scope, so the fact reaches the *type*.
fn an_assert_narrows_a_union(v: Int | Str) -> Int => !v {
    assert!(v is Int, "expected an Int")
    return v
}

// The control: without the assertion the same body is rejected, so the
// acceptance above is the narrowing and not something else.
fn without_the_assert_the_union_does_not_narrow(v: Int | Str) -> Int => !v {
    //~ error[fn-syntax:return-mismatch]
    return v
}

// [assert-fn] The message is optional, and it is an ordinary expression — so
// interpolation works, and (being compiler-owned) it is evaluated only on
// failure.
fn bare_message(n: Int) -> None => n {
    assert!(n > 0)
}

fn interpolated_message(n: Int) -> None => n {
    assert!(n > 0, "n must be positive, was ${n}")
}

// A message that is not a `Str` is refused, since it is what the trap prints.
fn a_non_str_message_is_refused(n: Int) -> None => n {
    //~ error[assert-fn:message-not-str]
    assert!(n > 0, n)
}

// ===== [assert-fn] `unreachable!` =====

// [assert-fn] `unreachable!()` is typed `Never`, so it stands where a value is
// expected — which is what a `when`'s impossible arm needs.
fn unreachable_stands_in_for_a_value(n: Int) -> Str => n {
    return when {
        n > 0 { "positive" }
        n <= 0 { "other" }
        else { unreachable!("an Int compares one way or the other") }
    }
}

// …and it ends a path, so a function whose tail is one needs no return value
// [fn-must-return].
fn unreachable_ends_a_path(n: Int) -> Str => n {
    unreachable!()
}

// Both names stay ordinary identifiers: only `assert!(` and `unreachable!(`
// are the forms.
fn the_names_are_contextual(assert: Int, unreachable: Int) -> Int => assert, unreachable {
    return assert + unreachable
}

// [qual-whynot] `whynot?(x is Q)` is a `Str?` — usable where one is expected
// and refused where a `Str` is — and explains a predicate test only: a binding
// and a union arm test are refused.
qualifier Even of Int {
    // @whynot [n] is odd
    fn qualifies(n: Int) -> Bool {
        return true
    }
}

fn whynot_answers_a_str_option(n: Int) [] -> Str? => n {
    return whynot?(n is Even Int)
}

fn whynot_is_not_a_str(n: Int) [] -> Str => n {
    //~ error[fn-syntax:return-mismatch]
    return whynot?(n is Even Int)
}

fn whynot_explains_a_predicate_test_only(n: Int, u: Int | Str) [] -> None => n, u {
    //~ error[qual-whynot:binding]
    let _a = whynot?(n is Even Int e)
    //~ error[qual-whynot:union-test]
    let _b = whynot?(u is Int)
}

// [doc-tags] The doc tags are checked where they are written: a tag this
// version does not know, an `@param` naming no parameter (or nothing, or on a
// declaration that is not a fn), and an `@whynot` that is not on a qualifier's
// `qualifies` warn, at the documented name; so does an `@compilerrules` with no
// bracketed list [doc-compilerrules], while one with a list is silent. Tags
// never reject a program.

// A point.
//
// @param x  nothing
// @frob something
// @compilerrules doc-tags
// @compilerrules [doc-tags] fine
//~ warning[doc-compilerrules:no-rules]
//~ warning[doc-tags:param-not-fn]
//~ warning[doc-tags:unknown-tag]
struct P {
    x: Int
}

// Adds.
//
// @param a  the first
// @param c  no such
// @param
// @whynot nope
// @returns  the sum,
//           of both
//~ warning[doc-tags:param-unknown]
//~ warning[doc-tags:param-unnamed]
//~ warning[qual-whynot:misplaced-tag]
fn add(a: Int, b: Int) [] -> Int => a, b {
    return a + b
}
