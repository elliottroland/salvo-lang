// [type-literal] [type-union-arm-identity] Unions of literals in the checker:
// which values they accept, how `is` narrows them, and what is refused (user
// decisions 2026-09-30). The runtime tables a backend reads are
// `literal_tests.rs`'s.

type Class = "STANDARD" | "GLACIER" | Other Str

type Closed = "A" | "B"

fn describe(c: Class) [] -> Int => c {
    when c {
        is "STANDARD" { return 1 }
        is "GLACIER" { return 2 }
        is Other { return 3 }
    }
}

fn widen(s: Str) [] -> Str => !s { return s }

// Literals take their type from the expected type, and `is` narrows them.
fn uses() [] -> Int {
    let a: Class = "STANDARD"
    let later: Class? = None
    later = "GLACIER"
    let _untyped = "STANDARD"
    let _w = widen(a)
    if later is None {
        return 0
    }
    return describe(later) + describe("GLACIER") + describe(other("DEEP"))
}

// A literal the union does not list is refused by name, and the message
// offers `other("STANDRD")`.
fn misspelt() [] -> None {
    //~ error[type-literal:not-listed]
    let _c: Class = "STANDRD"
}

// Open or closed, a `Str` is not one of the literals until narrowed — only
// `other(…)` reaches the open arm (the user's rule): the open union's message
// names it, the closed one's does not.
fn plain_values(s: Str) [] -> None => s {
    //~ error[type-literal:needs-narrowing]
    let _open: Class = s
    //~ error[type-literal:needs-narrowing]
    let _closed: Closed = s
}

// A `when` over literals is exhaustive.
fn partial_when(c: Closed) [] -> Int => c {
    //~ error[when-exhaustive:missing-arms]
    when c {
        is "A" { return 1 }
    }
}

// A literal beside its bare base is a warning: the `Str` arm subsumes it.
//~ warning[type-literal:subsumed]
type Sub = "a" | Str

// Overloads on literals of one base are refused.
fn pick(x: "A") [] -> Int => x { return 1 }
//~ error[type-literal:erased-overload]
fn pick(x: "B") [] -> Int => x { return 2 }

// [type-literal-qualified] A literal listed under two qualifier sets is
// refused, aliases expanded, naming both arms.
export provenance qualifier Short of Str
type Port = "port"

//~ error[type-literal-qualified:listed-twice]
type Bad = Short "port" | "port"
//~ error[type-literal-qualified:listed-twice]
type BadAliased = Short Port | "port" | "host"
