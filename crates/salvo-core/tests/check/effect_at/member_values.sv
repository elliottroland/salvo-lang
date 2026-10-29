// [effect-member-overload] [fn-value-select] An available effect's member is
// a value by its bare name, as a fn is (R10 D3, PIPELINE.md B3.6): at a
// fn-typed position its members compete with the fns of the name, and the
// winner is a closure over the instance in scope. Before, only `name@Effect`
// made a member a value.

struct Point { x: Int }

effect Fmt {
    fn show(p: Point) -> Str => p
}

fn show(n: Int) -> Str => n { return "${n}" }

fn on_point(f: (p: Point) -> Str, p: Point) -> Str => f, p { return f(p) }

fn on_int(f: (n: Int) -> Str, n: Int) -> Str => f, n { return f(n) }

// The position's type chooses: the member for a `Point`, the fn for an `Int`.
fn chosen(p: Point) [Fmt] -> Str => p {
    return "${on_point(show, p)}${on_int(show, 1)}"
}

// [effect-available] With no handler in scope the member is no candidate, and
// the fn alone is: its value does not fit the `Point` position.
fn no_handler(p: Point) -> Str => p {
    //~ error[call-resolve:no-overload]
    return on_point(show, p)
}

effect Logs {
    fn note(s: Str) -> Str => s
}

// [effect-at] Detached from a fn-typed position a member names no closure,
// as `note@Logs` does not.
fn detached() [Logs] -> None {
    //~ error[fn-value-select:member-not-expected]
    let _v = note
}

fn on_str(f: (s: Str) -> Str) -> Str => f { return f("a") }

fn passed() [Logs] -> Str {
    return on_str(note)
}

//--- a member and a fn of one type
// [fn-overload-ambiguous] A member and a fn fitting the position alike are the
// ordinary ambiguity, naming the selector forms (`note@Logs`, `note@main`).
effect Logs {
    fn note(s: Str) -> Str => s
}

fn note(s: Str) -> Str => s { return "fn" }

fn on_str(f: (s: Str) -> Str) -> Str => f { return f("a") }

fn passed() [Logs] -> Str {
    //~ error[fn-value-select:ambiguous]
    return on_str(note)
}

fn picked() [Logs] -> Str {
    return "${on_str(note@Logs)}${on_str(note@main)}"
}
