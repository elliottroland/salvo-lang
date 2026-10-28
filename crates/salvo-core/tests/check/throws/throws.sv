// Non-resumption: `throw` and the intrinsic `try` [throw] [throw-try].
//
// `throw(message)` returns `Never`, so the code between it and its delimiter
// does not run; a function that may throw says so with `[Throw<M>]` and
// returns its *own* type. The delimiter is `try { ... }`, a compiler intrinsic
// whose value is `Ok T | Thrown M` — an ordinary union, so
// `is`/`when`/exhaustiveness need no new rules. `M` is the union of the
// message types the body performs (user decision 2026-09-04).

import std.throw

linear struct Handle {
    fd: Int
}

fn close(x: Handle) -> None => !x { discard(x) }

fn open_handle(fd: Int) [] -> Handle {
    return Handle { fd: fd }
}

fn close_handle(h: Handle) [] -> None => !h {
    close(h)
}

fn note(text: Str) [] -> None => text {}

// A pair of throwing functions with *different* message types.
fn parse(line: Str) [Throw<Str>] -> Int => !line {
    if flagged(line) {
        throw("bad line")
    }
    return 1
}

fn limit(n: Int) [Throw<Int>] -> Int {
    if n > 3 {
        throw(n)
    }
    return n
}

fn flagged(line: Str) [] -> Bool => line {
    return true
}

// ===== the outcome type [throw-try] =====

// [throw-try] The delimiter's value is `Ok T | Thrown M` — read off a
// deliberate annotation mismatch (the message says it found `Ok Int | Thrown
// Str`), which is the cheapest way to see the type the checker computed.
fn try_yields_ok_and_thrown_arms() [] -> None {
    //~ error[var-let-infer:annotation-mismatch]
    let _outcome: Bool = try {
        parse("x")
    }
}

// [throw-try] `M` is the *union* of the message types performed in the body
// (user decision 2026-09-04, for consistency with `if`/`when` branch types) —
// a single type stays bare, several form a union: `Ok Int | Thrown (Str |
// Int)`.
fn several_message_types_union_together() [] -> None {
    //~ error[var-let-infer:annotation-mismatch]
    let _outcome: Bool = try {
        let n = parse("x")
        limit(n)
    }
}

// [throw-try] A body whose every path leaves still has an `Ok` arm: `Ok None`.
fn an_always_throwing_body_still_has_an_ok_arm() [] -> None {
    //~ error[var-let-infer:annotation-mismatch]
    let _outcome: Bool = try {
        parse("x")
        note("unreachable")
    }
}

// [throw-try] Nothing in the body can throw, so the delimiter has no outcome
// to produce (user decision 2026-09-04: an error, not `Thrown None`).
fn a_try_that_cannot_throw_is_rejected() [] -> None {
    //~ error[throw-try:nothing-throws]
    let _outcome = try {
        note("nothing to throw here")
    }
}

// ===== propagation [throw] =====

// [throw] A throw needs somewhere to land: a `try` or the enclosing fn's
// declared effect. The diagnostic names both remedies.
fn a_throw_with_nowhere_to_land_is_rejected() [] -> None {
    //~ error[throw:no-receiver]
    let n = parse("x")
    note("${n}")
}

// [throw] Declaring the effect passes it on: no `try` needed, and the fn's own
// return type is unchanged (no `Result` plumbing).
fn declaring_the_effect_propagates(line: Str) [Throw<Str>] -> Int => !line {
    return parse(line)
}

// [throw] The message must fit what the landing site carries.
fn a_message_the_target_cannot_carry_is_rejected(line: Str) [Throw<Int>] -> Int => !line {
    //~ error[throw-try:message-mismatch]
    return parse(line)
}

// [throw-not-main] The entry point has no caller to receive a throw, and no
// Rust lowering (`main` cannot return `ControlFlow`).
//~ error[throw-not-main:declared]
fn main() [Throw<Str>] -> None {
    note("hi")
}

// [throw] There is no handler for throwing: a handler would have to *resume*,
// which `Never` forbids.
// (Its member's body falls through, which a `-> Never` member may not.)
//~ error[throw:no-handlers]
handler Swallow of Throw<Str> {
    //~ error[fn-must-return:missing-return]
    fn throw(message: Str) -> Never => !message {
        note(message)
    }
}

// ===== divergence [type-any-never] [fn-must-return] =====

// [fn-must-return] `throw` returns `Never`, so a branch ending in one
// satisfies "every path returns" — the path ends there.
fn a_throwing_branch_counts_as_returning(n: Int) [Throw<Str>] -> Int {
    if n > 0 {
        return n
    } else {
        throw("not positive")
    }
}

// [type-any-never] The same rule at the level of flow analysis: a branch that
// throws never falls through, so a value it consumed is still live afterwards.
fn a_throwing_branch_does_not_leak_its_consumption(text: Str) [Throw<Str>] -> None => !text {
    if flagged(text) {
        throw(text)
    }
    note(text)
}

// ===== linear obligations across a throw [linear-obligation] =====

// [throw] [linear-obligation] The code after a may-throw call does not run on
// the throw path, so a linear value live across it would leak. Since `defer`
// was removed (2026-09-10) there is no construct that discharges on a path the
// author does not write, so the diagnostic names the two remedies that remain:
// release before the call, or move the value onward.
fn a_linear_value_across_a_may_throw_call_is_rejected(line: Str) [Throw<Str>] -> Int => !line {
    //~ error[ir-linear-check:leak-across-throw]
    let h = open_handle(1)
    let n = parse(line)
    close_handle(h)
    return n
}

// The remedy that remains: release *before* the call, so the throw path owes
// nothing. This is what removing `defer` costs — the release is written where
// it happens rather than registered once — and what it buys: the obligation is
// checked rather than delegated to a construct.
fn releasing_before_the_call_discharges_it(line: Str) [Throw<Str>] -> Int => !line {
    let h = open_handle(1)
    close_handle(h)
    return parse(line)
}

// ===== nesting [throw-try-innermost] =====

// [throw-try-innermost] A throw lands in the innermost delimiter, so an inner
// `try` does not need the outer one's message type — and the outer fn's
// `[Throw<M>]` is unaffected by what the inner one catches. The inner
// delimiter's outcome carries `Int` (from `limit`, `Ok Int | Thrown Int`), *not*
// the `Str` the enclosing fn propagates; the annotation error is the only one.
fn an_inner_try_takes_only_its_own_throws(line: Str) [Throw<Str>] -> Int => !line {
    //~ error[var-let-infer:annotation-mismatch]
    let _inner: Bool = try {
        limit(2)
    }
    return parse(line)
}

// ===== nested qualification and union messages [throw-try] =====
// The two shapes the design flagged as wanting a test rather than an
// assumption. Both are reachable: a *qualified union* is a claim about a
// union, so the inner arms are matched by binding at the inner type — the
// droppable-qualifier rule (`Qual T <: T`) does the unwrapping, which is also
// why `once` (never droppable) needs no special case here.

// [throw-try] A body that already returns a result yields `Ok (Ok Int | Err
// Str) | Thrown M`, and the inner result is reachable.
fn wrapped(n: Int) [Throw<Str>] -> Ok Int | Err Str {
    if flagged("x") {
        throw("bad")
    }
    return ok(n)
}

fn a_nested_result_outcome_can_be_taken_apart() [] -> None {
    let outcome = try { wrapped(1) }
    when outcome {
        is Ok {
            let inner: Ok Int | Err Str = outcome
            when inner {
                is Ok { note("value") }
                is Err { note("error") }
            }
        }
        is Thrown {
            note("thrown")
        }
    }
}

// [throw-try] The same for a *union message*: `Thrown (Str | Int)`'s arms are
// reachable by binding at the message type.
fn a_union_message_can_be_taken_apart() [] -> None {
    let outcome = try {
        let n = parse("x")
        limit(n)
    }
    when outcome {
        is Ok {
            note("ok")
        }
        is Thrown {
            let message: Str | Int = outcome
            when message {
                is Str {
                    note("text")
                }
                is Int {
                    note("number")
                }
            }
        }
    }
}

// [when-union-subject] Matching the qualified union *directly* is rejected
// with a diagnostic naming that remedy (bind the inner union to a local) — the
// same qualifier name can appear at both levels (`Ok (Ok Int | …)`), so the
// binding is what makes which level is meant visible.
fn matching_a_qualified_union_directly_names_the_remedy() [] -> None {
    let outcome = try {
        let n = parse("x")
        limit(n)
    }
    when outcome {
        is Ok {
            note("ok")
        }
        is Thrown {
            //~ error[when-union-subject:qualified-union]
            when outcome {
                is Str {
                    note("text")
                }
                is Int {
                    note("number")
                }
            }
        }
    }
}

// ===== a `try` body is ordinary code to every traversal =====

// [throw-try] [is-narrow-assign-reset] An assignment inside a `try` body resets
// an earlier `is` narrowing, like an assignment anywhere else. (Flow-sensitive
// checking of the body is what achieves this, not the syntactic assigned-name
// scan. Kept as a behavioural regression: the body is code, and `try` must not
// become a narrowing barrier.)
// (A parameter is a constant binding [fn-param-const], so the narrowed place
// is a local.)
fn an_assignment_inside_a_try_body_resets_narrowing(value: Str | Int) [] -> None => !value {
    let v = value
    if v is Str {
        let _outcome = try {
            v = 7
            parse("x")
        }
        //~ error[call-resolve:no-overload]
        note(v)
    }
}

// The control: without the assignment the narrowing stands and the read is
// fine, so the rejection above means what it says.
fn without_the_assignment_the_narrowing_stands(value: Str | Int) [] -> None => !value {
    let v = value
    if v is Str {
        let _outcome = try {
            parse("x")
        }
        note(v)
    }
}
