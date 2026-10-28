// Expressions and statements the parser refuses. One program per error,
// since a parse error stops a program.

//--- an_unclosed_parameter_list
//~ error[syntax-expected:identifier]
fn broken( {
}

//--- try_without_a_block_is_an_error
// [throw-try] A bodyless `try` is a parse error naming the form.
fn parse(line: Str) [] -> Int => line {
    return 1
}

fn f(line: Str) {
    //~ error[throw-try:missing-block]
    let outcome = try parse(line)
}

// ===== The subject-less `when` [when-condition] =====

//--- subjectless_when_without_an_else_is_an_error
// [when-condition] `when` is always exhaustive: with no subject there are
// no arms to be exhaustive over, so the `else` is required.
fn f(n: Int) -> Str {
    //~ error[when-condition:missing-else]
    return when {
        n < 0 { "neg" }
    }
}

//--- when_with_only_an_else_is_an_error
// [when-condition] An `else` with nothing above it decides nothing.
fn f() -> Str {
    //~ error[when-condition:only-else]
    return when {
        else { "x" }
    }
}

//--- a_branch_after_the_else_is_an_error
// [when-condition] The `else` is the fall-back, so nothing follows it.
fn f(n: Int) -> Str {
    return when {
        n < 0 { "neg" }
        //~ error[when-condition:else-not-last]
        else { "other" }
        n > 0 { "pos" }
    }
}

//--- an_else_in_the_subject_form_names_the_other_form
// [when-union-subject] The subject form takes no `else` — it is exhaustive
// over the union's arms. The diagnostic names the subject-less form instead
// of reporting a missing `is`, which is what the reader actually needs.
fn f(result: Ok Int | Err Str) -> Str {
    //~ error[when-exhaustive:missing-arms]
    return when result {
        is Ok { "ok" }
        //~ error[when-union-subject:else]
        else { "err" }
    }
}

// ===== Call arguments =====

//--- mixing_a_pair_and_a_positional_argument_is_rejected
// [col-pairs] Pairs are all-or-nothing: a positional argument cannot be
// mixed with `k: v` pairs in one call.
fn f() {
    //~ error[name-ident-resolve:unknown]
    //~ error[col-pairs:positional-after-pair]
    let _m = Mut Map("a": 1, 99)
}

//--- a_positional_argument_after_a_named_one_is_rejected
// [implicit-override] A positional argument after a named one has no
// meaning.
fn f() {
    //~ error[implicit-override:positional-after-named]
    let x = total(add = times, xs)
}

// ===== Selectors [fn-overload-at] [effect-at] =====

//--- a_scope_selector_needs_a_name
// [fn-overload-at] The selector belongs to a *name*, so writing it after
// anything else is a parse error naming both places it may go.
fn probe() -> None {
    //~ error[fn-overload-at:misplaced]
    let _n = (1 + 2)@core.list
}

//--- an_effect_selector_needs_a_name
// [effect-at] The effect selector follows a name too.
fn probe() -> None {
    //~ error[effect-at:misplaced]
    let _n = (1 + 2)@Fs
}

//--- the_receiver_form_is_not_the_self_send
// [actor-self-send] `self.step(n)` is the retired spelling: a parse error
// naming `k@self(…)`.
handler Working() of Work {
    mailbox { capacity: 1 }

    send fn start(n: Int) {
        //~ error[fn-dot:unknown-fn]
        //~ error[actor-self-send:self-dot-call]
        self.step(n)
    }
}

// ===== Tests and checks [assert-op] [is-not] [is-union] =====

//--- a_postfix_bang_is_refused
// [assert-op] The postfix `!` assertion is gone (2026-10-20): `first(xs)!`
// is a parse error that names the replacements.
fn f(xs: List<Int>) -> None {
    //~ error[op-no-none:optional-operand]
    //~ error[assert-op:postfix-bang]
    let _x = first(xs)! + 1
}

//--- not_is_refuses_a_binding
// [is-not] A negated test narrows nothing on the branch it guards, so there
// is no value for a binding to name.
fn f(s: Str?) -> Str {
    //~ error[is-not:binding]
    if s !is Str text {
        return "no"
    }
    return "yes"
}

//--- not_is_refuses_a_widening
// [is-not] …and `^` has nothing to widen either [qual-lift].
fn f(o: Ok Int | Err Str) -> Int {
    //~ error[is-not:lift]
    if o !is ^Ok {
        return 0
    }
    return 1
}

//--- a_union_is_binds_nothing
// [is-union] A binding on a union `is` is refused…
fn f(x: Int | Str | Bool) {
    //~ error[is-union:binding]
    let _a = x is Int | Str v
}

//--- a_union_is_tests_a_place
// …and so is a subject that is not a place, since the subject is read once
// per test.
fn g() [] -> Str {
    return "json"
}

fn f() {
    //~ error[is-union:subject-not-place]
    let _a = g() is "json" | "text"
}

// ===== Line breaks [op-compound] [expr-infix] =====

//--- compound_assignment_does_not_cross_a_line
// [op-compound] A compound assignment is a **statement**, so the operator
// does not chain and does not cross a line break.
fn f() -> None {
    let n = 1
    n
    //~ error[syntax-expected:expression]
    += 2
}

//--- comparison_does_not_wrap_across_lines
// [expr-infix] A newline before a comparison ends the statement: the
// continuation line `> 3` cannot start a statement, so this is a parse error.
fn f(a: Int) {
    let _x = a
    //~ error[syntax-expected:expression]
    > 3
}

// ===== [expr-replyto-flat] =====

//--- flattened_replyto_after_a_non_call_is_an_error
// [expr-replyto-flat] A trailing `replyto` after a non-call is an error (no
// argument slot).
fn f(y: Int) {
    //~ error[actor-replyto:not-after-call]
    let _x = y replyto k()
}

// ===== Literals [col-no-literal] [type-literal] =====

//--- a_bare_brace_is_not_a_literal
// [col-no-literal] [struct-lit-shorthand] Without a type name, a bare
// `{ x }` is neither a struct literal nor a lambda: it was a set literal,
// and collection literals are retired.
fn f() {
    //~ error[col-no-literal:brace-literal]
    let s = { x }
}

//--- a_float_cannot_be_a_literal_type
// [type-literal] A float is refused as a literal type.
//~ error[type-literal:float]
type F = 1.5 | 2

// ===== Terminal `...` scopes [expr-terminal-scope] =====

//--- a_terminal_scope_as_a_let_value
// [expr-terminal-scope] It is statement-only: a `...` in a value position (a
// `let` value, a call argument, an operand) is a parse error naming the
// braced form.
fn f(x: Str?) {
    //~ error[expr-terminal-scope:nested-if]
    let v = if x is Str ...
}

//--- a_terminal_scope_as_an_argument
fn f(x: Str?) {
    //~ error[expr-terminal-scope:nested-if]
    g(if x is Str ...)
}

//--- a_terminal_scope_as_an_operand
fn f(x: Str?) {
    //~ error[expr-terminal-scope:nested-if]
    let v = 1 + if x is Str ...
}

//--- a_terminal_if_refuses_elif
// [expr-terminal-scope] A terminal `if` takes only an optional braced
// `else` — no `elif`, no `else ...`.
fn f(x: Str?) {
    if x is Str ...
    //~ error[syntax-expected:expression]
    //~ error[expr-terminal-scope:terminal-if-elif]
    //~ error[col-no-literal:brace-literal]
    elif false {
    }
}

//--- a_terminal_if_refuses_else_dots
fn f(x: Str?) {
    //~ error[expr-terminal-scope:terminal-if-else]
    if x is Str ... else ...
}
