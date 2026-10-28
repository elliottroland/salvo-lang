// The subject-less `when` [when-condition] and the boolean-condition rule
// [expr-cond-bool].
//
// `when` is always exhaustive, in one of two ways. With a subject it is
// exhaustive over the union's arms and takes no `else`
// [when-union-subject]; without one it is a condition chain whose `else` is
// mandatory — which is exactly what separates it from `if`/`elif`/`else`,
// since an `if` without an `else` folds `None` into its value
// [expr-if-else-none]. Conditions on both forms (and on `if`/`elif`/`while`) are
// `Bool` and nothing else: Salvo has no truthiness.

fn note(text: Str) [] -> None => text {}

// ===== [when-condition] =====

// The plain form: bare boolean branch heads, a mandatory `else`.
fn classify(n: Int) [] -> Str {
    return when {
        n < 0 { "negative" }
        n == 0 { "zero" }
        else { "positive" }
    }
}

// [when-value] The mandatory `else` is the point: every path produces a
// value, so the type is the join of the branches with no `None` folded in —
// unlike an `if` chain without an `else` [expr-if-else-none]. A `Str` return
// type accepting the chain is the observable proof.
fn classify_value(n: Int) [] -> Str {
    let label = when {
        n < 0 { "negative" }
        else { "other" }
    }
    return label
}

// The same chain written as an `if` is `Str?`, and returning it fails.
fn classify_if(n: Int) [] -> Str {
    let label = if n < 0 {
        "negative"
    }
    //~ error[fn-syntax:return-mismatch]
    return label
}

// Branch heads are ordinary boolean expressions, so `is` works in them and
// narrows its branch — and the negative narrowing accumulates into the
// later branches exactly as in an `if`/`elif` chain [is-narrowing].
fn describe(value: Str | Int) [] -> None => !value {
    when {
        value is Str { note(value) }
        else { note("an int") }
    }
}

// The `else` has the arm removed: `note` takes a `Str`, and there the
// value is an `Int`.
fn describe_else(value: Str | Int) [] -> None => !value {
    when {
        value is Str { note("a str") }
        //~ error[call-resolve:no-overload]
        else { note(value) }
    }
}

// [fn-must-return] A total chain returns on every path, so no fall-through
// return is needed after it.
fn classify_returns(n: Int) [] -> Str {
    when {
        n < 0 { return "negative" }
        else { return "other" }
    }
}

// ===== [expr-cond-bool] =====

// Conditions are `Bool`. There is no truthiness rule that could turn an
// `Int` into a decision, so the non-boolean condition is an error on every
// construct that takes one.
fn cond_if(n: Int) [] -> None {
    //~ error[expr-cond-bool:not-bool]
    if n {
        note("x")
    }
}

fn cond_elif(n: Int) [] -> None {
    if n > 0 {
        note("x")
    //~ error[expr-cond-bool:not-bool]
    } elif n {
        note("y")
    }
}

fn cond_while(n: Int) [] -> None {
    //~ error[expr-cond-bool:not-bool]
    while n {
        note("x")
    }
}

fn cond_when(n: Int) [] -> None {
    when {
        //~ error[expr-cond-bool:not-bool]
        n { note("x") }
        else { note("y") }
    }
}

// The check lands on the *leaf* that has the wrong type, so a compound
// condition reports the operand rather than the whole expression.
fn cond_leaf(n: Int) [] -> None {
    //~ error[expr-cond-bool:not-bool]
    if n > 0 && n {
        note("x")
    }
}

// A possibly-absent boolean is not a boolean: which way `None` should
// decide is exactly what the rule refuses to guess. `!` (or an `is` test)
// is the way through.
fn cond_optional(b: Bool?) [] -> None => !b {
    //~ error[expr-cond-bool:not-bool]
    if b {
        note("x")
    }
}

fn cond_optional_unwrapped(b: Bool?) [] -> None => !b {
    if (b ?: unreachable!()) {
        note("x")
    }
}

// [type-unknown-lenient] A type the checker could not infer produces one
// diagnostic, not two: the condition rule stays quiet on `Unknown`.
fn cond_unknown() [] -> None {
    //~ error[call-resolve:unknown-fn]
    if unknown_thing() {
        note("x")
    }
}

//--- a subject-less when requires an else
// Without an `else` there is nothing to make the chain exhaustive, and
// `when` is always exhaustive.
fn classify(n: Int) [] -> Str {
    //~ error[when-condition:missing-else]
    return when {
        n < 0 { "negative" }
    }
}

//--- a when with only an else
// An `else` with no condition above it decides nothing.
fn classify(n: Int) [] -> Str {
    //~ error[when-condition:only-else]
    return when {
        else { "whatever" }
    }
}

//--- else is the last branch
// The `else` closes the chain: branches after it would be dead.
fn classify(n: Int) [] -> Str {
    return when {
        n < 0 { "negative" }
        //~ error[when-condition:else-not-last]
        else { "other" }
        n > 0 { "positive" }
    }
}

//--- the subject form takes no else
// [when-union-subject] The subject form is exhaustive over the arms, so an
// `else` there is a category error — and the diagnostic names the other
// form (`when { cond { … } }`) rather than reporting a missing `is`.
// (Both arms are covered here: with only `is Ok`, the refused `else` also
// leaves `Err Str` unhandled, `when-exhaustive:missing-arms`.)
fn outcome() [] -> Ok Int | Err Str { return ok(0) }
fn probe() [] -> Str {
    let o = outcome()
    return when o {
        is Ok { "ok" }
        is Err { "err" }
        //~ error[when-union-subject:else]
        else { "other" }
    }
}
