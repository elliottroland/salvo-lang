// ===== [effect-available] one overload set: members and fns compete =====
//
// User decision 2026-09-14 (option (a) of three): a name that is both an
// available effect's member and an ordinary fn resolves as *one* set, ranked
// by signature specificity. std needs it — `close` is an `Fs` member per
// stream token *and* the `Lines` pass's discharger — and so does anyone who
// writes a linear pass while a filesystem is in scope.
//
// A `Store` with a `close` per handle type, and a module-level `close` of
// its own. The argument types tell all three apart.

struct Note { text: Str }

effect Store {
    fn close(handle: Int) -> Str
}

handler MemStore of Store {
    fn close(handle: Int) -> Str {
        return "store"
    }
}

fn close(n: Note) -> Str => !n {
    return n.text
}

// The member and the fn coexist: `close(1)` is the member, `close(note)` is
// the fn, in one scope with the handler registered.
fn work(n: Note) [Store] -> Str => !n {
    let a = close(1)
    return "${a}${close(n)}"
}

// The fn is reachable where it used to be shadowed outright: before the
// rule, an available `Store` claimed the name and the call was an error
// naming `Store.close`'s overloads.
fn work_fn_side(n: Note) [Store] -> Str => !n {
    return close(n)
}

// Neither side accepting the arguments is **one** diagnostic: to the caller
// they are one name, and one overload set ("no matching overload for
// `close(Str)`").
fn work_neither() [Store] -> Str {
    //~ error[call-resolve:no-overload]
    return close("nope")
}

// [effect-available] Availability still decides first: with no handler in
// scope the member set is not a candidate at all, so the fn answers.
fn work_no_handler(n: Note) -> Str => !n {
    return close(n)
}

// A **more specific fn** wins over a generic member: specificity decides,
// exactly as it does between two fn overloads [fn-overload-rank].
effect Sink {
    fn keep<T>(x: T) -> Str => x
}

handler Bin of Sink {
    fn keep<T>(x: T) -> Str => x {
        return "any"
    }
}

fn keep(x: Int) -> Str => x {
    return "int"
}

fn use_it() [Sink] -> Str {
    return keep(1)
}

//--- a member and a fn tie
// A genuine tie — same parameter type on both sides — is the ordinary
// ambiguity, naming both remedies rather than a silent preference
// ("ambiguous call to `close(Int)`", `close@Store(...)`, `close@main(...)`).
effect Store {
    fn close(handle: Int) -> Str
}

handler MemStore of Store {
    fn close(handle: Int) -> Str {
        return "store"
    }
}

fn close(handle: Int) -> Str => handle {
    return "fn"
}

fn work() [Store] -> Str {
    //~ error[fn-overload-ambiguous:call]
    return close(1)
}

// The selectors settle it by hand, each picking its own side: `@Effect` the
// member, `@module` the fn — and `@module` skips the member set whole, which
// is what makes a shadowed fn callable at all.
fn work_picked() [Store] -> Str {
    let a = close@Store(1)
    let b = close@main(1)
    return "${a}${b}"
}

//--- a lambda the candidates agree on
// [effect-available] [type-unknown-lenient] A lambda with an unwritten
// parameter type in a call where member and fn collide (B1 census D16): the
// member and the fn are one candidate set, so the lambda is typed as for any
// overloads — by the lead candidate, or, with none, by the type every
// candidate declares at its position (PIPELINE.md B3.5). When they disagree
// nothing can type it, and the call is refused — before 2026-10-27 the
// lambda's parameter stayed `?`, the analysis was clean, and rustc and
// kotlinc both refused the generated code.
effect Ops {
    fn apply(f: (x: Int) -> Int, n: Int) -> Int => f
}

handler OpsH of Ops {
    fn apply(f: (x: Int) -> Int, n: Int) -> Int { return f(n) }
}

fn apply(f: (x: Int) -> Int, s: Str) -> Int => f { return f(1) }

fn work() [Ops] -> Int {
    return apply(x -> x + 1, 20) + apply(x -> x * 2, "s")
}

//--- a lambda the candidates disagree on
// The refusal ("cannot type this lambda", `(Int) -> Int`, `(Str) -> Str`)
// is one diagnostic for one mistake; writing the parameter type is the
// remedy. A member leads on the footing of the module's own fns, so neither
// types the other's lambda by scope.
effect Ops {
    fn apply(f: (x: Int) -> Int, n: Int) -> Int => f
}

handler OpsH of Ops {
    fn apply(f: (x: Int) -> Int, n: Int) -> Int { return f(n) }
}

fn apply(f: (x: Str) -> Str, n: Int) -> Int => f { return 1 }

fn work() [Ops] -> Int {
    //~ error[fn-overload-rank:lambda-untyped]
    return apply(x -> x, 20)
}

fn work_written() [Ops] -> Int {
    return apply((x: Int) -> x, 20)
}
