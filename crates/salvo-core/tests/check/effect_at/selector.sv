// [effect-member-overload] [effect-at] Two effects may declare the same
// member name (user decision 2026-09-14, lifting the 2026-09-05 ban): a
// bare call disambiguates by which effect has a handler in scope, and the
// `member@Effect(…)` selector picks explicitly — including a generic
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

// With both effects in scope the bare call is ambiguous, and the error
// names the selector form ("`close` is a member of `Fs`, `Net`", `close@Fs`).
fn both(h: Int) [Fs, Net] -> Str {
    //~ error[effect-at:ambiguous-member]
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
