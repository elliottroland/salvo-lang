// [when-union-head] A `when` arm head may be a union, `is A | B` (user
// decision 2026-10-30, option 1a): the arm covers every alternative, and the
// subject is narrowed to their union inside it, as `x is A | B` narrows in a
// condition [is-union].
struct A {
    n: Int
}

struct B {
    s: Str
}

fn inner(v: B | Int) [] -> Int {
    return when v {
        is B { 1 }
        is Int { v }
    }
}

fn covered(v: A | B | Int) [] -> Int {
    return when v {
        is A { v.n }
        is B | Int { inner(v) }
    }
}

fn levels(s: "debug" | "info" | "warn") [] -> Int {
    return when s {
        is "debug" | "info" { 0 }
        is "warn" { 1 }
    }
}

// A head covers what earlier arms left: an alternative the arms before it
// took is no error, but a head none of whose alternatives is left is dead.
fn dead(v: A | B | Int) [] -> Int {
    return when v {
        is A | B { 0 }
        //~ error[when-exhaustive:dead-branch]
        is A | B { 1 }
        is Int { 2 }
    }
}

// A union head has nothing single to bind, and no lift.
fn bound(v: A | B | Int) [] -> Int {
    return when v {
        //~ error[is-union:binding]
        is A | B x { 0 }
        is Int { 2 }
    }
}

// Exhaustiveness still holds: `Int` is left.
fn missing(v: A | B | Int) [] -> Int {
    //~ error[when-exhaustive:missing-arms]
    return when v {
        is A | B { 0 }
    }
}
