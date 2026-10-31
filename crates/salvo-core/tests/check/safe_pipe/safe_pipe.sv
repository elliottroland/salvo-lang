// [expr-safe-pipe] [when-lowered] The null-safe pipe `?>` is a `when`: its
// operator's own diagnostics survive the lowering.

fn double(n: Int) [] -> Int {
    return n * 2
}

fn maybe(b: Bool) [] -> Int? {
    if b {
        return 3
    }
    return None
}

// The ordinary shapes: an optional result, a general union passing its
// other arms through, and a right side that leaves.
fn shapes(r: Ok Int | Err Str) [] -> Int? {
    let a: Int? = maybe(true) ?> double(_)
    let _b: Int | Err Str = r ?> double(_)
    let c: None = maybe(false) ?> return a
    return c
}

// A right side that reads no `_` is dead scaffolding.
fn unused() [] -> Int? {
    //~ error[expr-safe-pipe:unused-placeholder]
    return maybe(true) ?> 5
}

// A value with one arm leaves nothing to pass through.
fn one_arm() [] -> Int {
    //~ error[expr-safe-pipe:single-arm]
    return 4 ?> double(_)
}

fn main() [] -> None {
}
