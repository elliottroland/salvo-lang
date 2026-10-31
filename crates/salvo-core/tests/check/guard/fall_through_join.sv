// [is-narrow-guard] After an `if` or a `when`, a place every path that
// falls through narrows is narrowed to the join of what they narrow it to
// (user decision 2026-10-30, generalizing "every branch leaves", so a
// leaving `?:` right side [op-elvis-guard] is the same rule).
struct A {
    n: Int
}

// The then branch falls through, the else leaves: `x` is an `A` below.
fn then_falls(x: A | Str) [] -> Int {
    if x is A {
        let _seen = 1
    } else {
        return 0
    }
    return x.n
}

// Two branches fall through, each narrowing `x`: below it is their join.
fn two_fall(x: A | Str | Int) [] -> Str {
    if x is A {
        let _a = 1
    } elif x is Int {
        let _i = 2
    } else {
        return "str"
    }
    return when x {
        is A { "a" }
        is Int { "int" }
    }
}

// A `when` on a variable: the subject is one of the arms whose branches
// fall through.
fn when_falls(v: A | Str | Int) [] -> Int {
    when v {
        is A | Int { }
        is Str { return -1 }
    }
    return when v {
        is A { v.n }
        is Int { v }
    }
}

// A path that does not narrow the place leaves it unnarrowed below.
fn one_path_plain(x: A | Str, flag: Bool) [] -> Int {
    if flag {
        let _f = 1
    } elif x is Str {
        return 0
    }
    //~ error[struct-field-resolve:not-a-struct]
    return x.n
}

// Nothing leaves: the paths cover the subject and nothing is narrowed.
fn nothing_leaves(x: A | Str) [] -> Int {
    if x is A {
        let _a = 1
    } else {
        let _s = 2
    }
    //~ error[struct-field-resolve:not-a-struct]
    return x.n
}

// An assignment on a path that falls through still ends the narrowing
// [is-narrow-assign-reset]; one on a path that leaves does not.
fn reassigned(x: A | Str) [] -> Int {
    let y: A | Str = x
    if y is A {
        y = "again"
    } else {
        return 0
    }
    //~ error[struct-field-resolve:not-a-struct]
    return y.n
}
