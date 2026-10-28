// Qualifier subjects [qual-subject]: state claims describe contents and a
// mutating call may strip them; provenance claims describe where the
// handle came from, survive every call, compose without `with`, and are
// mint-only.

struct Store<T> canbe Mut {
    value: T
}

// `Stocked` was `NonEmpty`, which std declares over its collections.
qualifier Stocked<T> of Store<T>
provenance qualifier Authenticated<T> of Store<T>

fn stocked<T>(s: Store<T>) -> +Stocked Store<T>
=> !s {
    return s
}

fn authenticate<T>(s: Store<T>) -> +Authenticated Store<T>
=> !s {
    return s
}

fn touch<T>(s: Mut Store<T>) [] -> None => s: Mut {}

fn needs_stocked<T>(s: Stocked Store<T>) -> None {
}

fn needs_auth<T>(s: Authenticated Store<T>) -> None {
}

// [qual-subject] [deduce-syntax] A mutating call strips a *state* claim:
// the stripping rule exists because mutation can invalidate a claim about
// contents.
fn strips(s: Mut Stocked Store<Int>) -> None {
    touch(s)
    //~ error[call-resolve:no-overload]
    needs_stocked(s)
}

// [qual-subject] Provenance survives it: mutating the contents cannot
// change where the handle came from. This is the payoff of the subject
// axis — without it, D3 refinements would be the only escape.
fn survives(s: Mut Authenticated Store<Int>) -> None {
    touch(s)
    needs_auth(s)
}

// [qual-subject] [qual-no-with] Qualifiers compose freely now that `with`
// is retired — provenance with state, provenance with provenance, and two
// state claims alike, with no declared compatibility.
struct Request { body: Str }
qualifier Validated of Request
qualifier Checked of Request
provenance qualifier FromCache of Request
provenance qualifier Signed of Request

fn compose_a(r: Signed Validated Request) -> Str { return r.body }
fn compose_b(r: Signed FromCache Request) -> Str { return r.body }

// Two *state* qualifiers also compose now — no `with` needed.
fn compose_two_state(r: Validated Checked Request) -> Str { return r.body }

// [qual-subject] Provenance is mint-only: nothing in the bits establishes
// where a handle came from, so a body (a `qualifies` predicate or field
// overrides, which are claims about contents) is a contradiction.
//~ error[qual-subject:provenance-body]
provenance qualifier Positive of Int {
    fn qualifies(v: Int) -> Bool { return v > 0 }
}

// [qual-subject] [qual-constructive] `is` on a non-union provenance value
// is an error — it falls out of provenance being constructive, and is the
// same message constructive qualifiers already give.
fn tested(r: Request) -> Bool {
    //~ error[qual-constructive:is-test]
    return r is Signed
}

// [qual-subject] Provenance is droppable (forgetting an origin is safe)
// and survives being stored into another value.
struct Wrapper { req: Signed Request }

fn sign(r: Request) -> +Signed Request
=> !r {
    return r
}

fn plain(r: Request) -> Str { return r.body }

fn stored(r: Request) -> Str
=> !r {
    let a = sign(r)
    let w = Wrapper {req: a}
    return plain(w.req)
}

// [qual-subject] The protocol tags are provenance (user decision
// 2026-09-23): std's `Ok` stacks with a state claim without any `with`
// declaration, because an origin is orthogonal to every claim about
// contents.
struct Box canbe Mut { n: Int }
qualifier Filled of Box
fn fill() -> +Filled Box { return Box { n: 1 } }
fn tagged() -> Ok Filled Box {
    return ok(fill())
}

// [qual-subject] A handler's constructor argument has to *fit* the parameter
// it is stored in, exactly as a call's argument does. A handler has no overload
// set, so nothing else reported it: a parameter demanding a claim
// (`numbers: Stocked Store<Int>`) silently accepted a value that does not
// carry one, and the mismatch surfaced — if at all — inside the member bodies.
effect Pick {
    fn pick() -> None
}

handler First(s: Stocked Store<Int>) of Pick {
    fn pick() -> None {
    }
}

fn unclaimed(plain: Store<Int>) [use] -> None
=> !plain {
    //~ error[effect-handler-generics:arg-mismatch]
    use First(plain)
}

// [qual-subject] And it is satisfied by a value that does carry the claim —
// the check is the ordinary fit, not a refusal of claims at constructors.
fn claimed(plain: Store<Int>) [use] -> None
=> !plain {
    use First(stocked(plain))
}
