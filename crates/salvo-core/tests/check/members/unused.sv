// [var-unused] Unused variables: a local that is never read warns, and says
// how to opt out.

fn an_unused_local() -> Int {
    //~ warning[var-unused:never-used]
    let spare = 1
    return 1
}

// [var-unused] A leading `_` is the opt-out, and a read is a use. `echo`
// itself is never read, so it *does* warn — the rule is uniform.
fn an_underscore_and_a_read() -> Int {
    let _spare = 1
    let used = 2
    //~ warning[var-unused:never-used]
    let echo = used
    return 1
}

// [var-unused] Assignment is not a use: a variable only ever written to has
// no reader, which is exactly the mistake worth reporting.
fn only_assigned() -> Int {
    //~ warning[var-unused:never-used]
    let counter = 1
    counter = 2
    return 1
}

// [var-unused] A *parameter* never warns: a signature often dictates it — an
// effect or handler member implementing a declared interface cannot drop one —
// so the warning would fire where nothing can be done about it.
fn takes(a: Int, b: Int) -> Int {
    return a
}
