// [effect-member-overload] [effect-at] Two effects may declare the same
// member name (user decision 2026-09-14, lifting the 2026-09-05 ban): a
// bare call sees the members of the effects with a handler in scope, ranked
// by the argument types like any overloads, and the `member@Effect(…)`
// selector picks explicitly — including a generic
// instance pinned by the call's type arguments
// (`next_random@Random<Int>()`).
//
// Two effects sharing `close`, plus handlers.

effect Fs {
    fn close(handle: Int) -> Str
}

effect Net {
    fn close(handle: Int) -> Str
}

handler MemFs of Fs {
    fn close(handle: Int) -> Str {
        return "fs"
    }
}

handler MemNet of Net {
    fn close(handle: Int) -> Str {
        return "net"
    }
}

// A bare call resolves through the one effect that is *available*: `shut`
// declares only `[Fs]`, so `close` is unambiguous there.
fn shut(h: Int) [Fs] -> Str {
    return close(h)
}

// With both effects in scope their members are ranked like overloads
// (PIPELINE.md B3.5, R10 P4): one signature on both sides is the ordinary
// ambiguity, naming the selector forms (`close@Fs(...)`, `close@Net(...)`).
fn both(h: Int) [Fs, Net] -> Str {
    //~ error[fn-overload-ambiguous:call]
    return close(h)
}

// [effect-at] The selector picks, both directions, in plain and dot form.
fn picked(h: Int) [Fs, Net] -> Str {
    let a = close@Fs(h)
    let b = close@Net(h)
    let c = h.close@Fs()
    return "${a}${b}${c}"
}

// [effect-at] The named effect must exist ("no effect named `Files`"), be in
// scope as an effect, and declare the member ("effect `Fs` has no member
// named `open`"); a selected member is a value only at a fn-typed position
// (R10 D3), and `let v = close@Fs` has none.
fn selector_errors(h: Int) [Fs] -> None {
    //~ error[effect-at:unknown-effect]
    close@Files(h)
    //~ error[effect-at:no-member]
    open@Fs(h)
    //~ error[effect-at:value-not-expected]
    let _v = close@Fs
}

// [effect-at] The selected effect still needs a handler in scope ("no
// handler for effect `Net`") — the selector picks the effect, not a handler
// out of thin air.
fn unavailable(h: Int) [Fs] -> None {
    //~ error[effect-available:no-handler]
    close@Net(h)
}

// [effect-at] [effect-disambiguation] The call's type arguments keep
// their meaning under the selector: they pin the generic effect's
// instance.
effect Random<T> {
    fn next_random() -> T
}

effect Dice {
    fn next_random() -> Int
}

fn draw() [Random<Int>, Random<Str>, Dice] -> Str {
    let n = next_random@Random<Int>()
    let s = next_random@Random<Str>()
    let d = next_random@Dice()
    return "${n}${s}${d}"
}

//--- two effects ranked by the arguments
// [effect-member-overload] [fn-overload-rank] Members of two available
// effects that take different types are one overload set (PIPELINE.md B3.5,
// census D15): `put(1)` is `Ints`', `put("s")` is `Strs`'. Before, two
// available owners were refused whatever the arguments.
effect Ints {
    fn put(x: Int) -> Str
}

effect Strs {
    fn put(x: Str) -> Str => x
}

fn both() [Ints, Strs] -> Str {
    return "${put(1)}${put("s")}"
}
