// [fn-member-call] [fn-uniform] `x.name(args)` is a **member call** when
// `x`'s type has a member `name` — a struct's field, or an `eff[E]`'s member —
// and otherwise a **uniform call**, the plain call `name(x, args)` exactly
// (user decision 2026-10-28).

struct Adder {
    f: (n: Int) -> Int
    n: Int
}

fn f(a: Adder, n: Int) [] -> Int => a, n { return n }

fn bump(a: Adder) [] -> Int => a { return a.n + 1 }

// [fn-member-call] A fn-typed field is called through its value.
fn through_a_field(a: Adder) -> Int => a {
    return (a.f)(1) + a.n
}

// [fn-member-call] Any member shadows: a field that is not a function makes
// the call an error that names the uniform spelling.
fn a_field_that_is_not_a_fn(a: Adder) -> Int => a {
    //~ error[fn-member-call:not-callable]
    return a.n()
}

// [fn-member-call] A member that hides a function the uniform call would have
// reached is called, with a warning naming both spellings…
fn a_member_shadows_a_fn(a: Adder) -> Int => a {
    //~ warning[fn-member-call:shadows]
    return a.f(2)
}

// …and the grouped form says the member is meant, so it does not warn.
fn the_grouped_form_is_explicit(a: Adder) -> Int => a {
    return (a.f)(2)
}

// [fn-uniform] No member of the name: the plain call `bump(a)`.
fn a_uniform_call(a: Adder) -> Int => a {
    return a.bump()
}

// [fn-uniform] A uniform call is the plain call exactly, so a fn-typed local
// of the name takes it, as it takes `twice(n)` (R10 D12, D13).
fn a_uniform_call_through_a_local(n: Int) -> Int => n {
    let twice = (k: Int) -> k * 2
    return n.twice()
}

// [fn-uniform] …and so does an implicit of the enclosing fn.
fn a_uniform_call_through_an_implicit(n: Int, ?scale: (k: Int) -> Int) -> Int => n, scale {
    return n.scale()
}

// [fn-uniform] With neither a member nor a function of the name, the call
// names both places it looked.
fn neither(a: Adder) -> Int => a {
    //~ error[fn-uniform:unknown-fn]
    return a.missing()
}

// [fn-member-call] A grouped callee is a field read, whatever is in scope.
fn a_grouped_missing_member(a: Adder) -> Int => a {
    //~ error[struct-field-resolve:no-field]
    return (a.bump)()
}

actor effect Counter {
    send fn add(n: Int)
}

fn describe(c: eff[Counter]) [] -> Str => c { return "a counter" }

// [fn-member-call] [fn-uniform] An `eff[E]` has E's members: `c.add(1)` is a
// send, and a function taking `eff[E]` first is reached by a uniform call.
fn sends_and_uniform_calls(c: eff[Counter]) [] -> Str => c {
    c.add(1)
    return c.describe()
}
