// [fn-once] `once` as the *use*-multiplicity qualifier, and specifically its
// generalization beyond function types (user decision 2026-09-07): written on
// function types, on a type of one's own that says `canbe once`, and (D6, user
// decision 2026-09-12) on any type — an upper bound the holder imposes on
// itself, which never drops.

// [fn-once] D6: the fn-only/`canbe once` position gate is gone — `once` is
// valid on any type.
fn on_a_scalar(x: once Int) -> None => !x {}

// [qual-canbe-optin] A type of one's own reaches the same place by opting in,
// the way it opts into mutability and linearity — the author declares that
// using the value uses it up, rather than the compiler inferring an
// obligation (user decision 2026-09-07).
struct Ticket canbe once {
    id: Int
}

fn on_an_opted_in_type(t: once Ticket) -> None => !t {}

// Only the opt-in qualifiers can be written after `canbe`.
//~ error[qual-canbe-optin:unknown]
struct NotATicket canbe Ok {
    id: Int
}

fn on_a_fn_type(f: once () -> None) -> None => !f { f() }

// [fn-once] D6: `once` on a *data* type — the upper bound is the holder's
// self-restriction, enforcement is ordinary consumption, and `once T` drops to
// `T` at a call (a plain-typed holder consumes at most once anyway). On fn
// types nothing changes: dropping `once` there stays refused.
struct Pass {
    id: Int
}

fn redeem(t: Pass) -> None => !t {}

fn spend(t: once Pass) -> None => !t {
    redeem(t)
}

fn spend_twice(t: once Pass) -> None => !t {
    redeem(t)
    //~ error[flow-use-after-move:consumed]
    redeem(t)
}
