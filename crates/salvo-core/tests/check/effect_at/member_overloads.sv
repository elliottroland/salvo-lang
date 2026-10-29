// ===== overloading *within* one effect [effect-member-overload] =====

// [effect-member-overload] Within one effect a repeated name is an
// **overload** (user decision 2026-09-14, §5.10.2 sub-question A): phase 4's
// `Fs` declares `close` for each stream type, so the parameter types are
// what tell them apart. (`Pipe` was a second `Fs`.)
effect Pipe {
    fn close(handle: Int) -> Str
    fn close(handle: Str) -> Str => handle
}

// [effect-member-unique] What stays an error is a *duplicate*: the same name
// taking the same types, which no call could ever tell apart ("effect
// `Twice` already declares a member named `close` with these parameter
// types"). (`Twice` was a third `Fs`.)
effect Twice {
    fn close(handle: Int) -> Str
    //~ error[effect-member-unique:duplicate]
    fn close(handle: Int) -> Int
}

// [effect-member-overload] The phase-4 shape: one effect, one member name,
// two parameter types — `Fs` declares `close` for each stream token
// (FILE_SYSTEM.md §5.10.2 sub-question A). The call picks by argument type,
// exactly as a function overload does.
struct InFile { id: Int }
struct OutFile { id: Int }

effect Fs {
    fn close(f: InFile) -> Str => !f
    fn close(f: OutFile) -> Int => !f
}

fn shut(a: InFile, b: OutFile) [Fs] -> Str => !a, !b {
    let text = close(a)
    let code = close(b)
    return "${text}${code}"
}

// [effect-member-overload] [effect-at] The selector picks the *effect*; the
// argument types still pick the overload inside it.
effect Net {
    fn close(socket: Int) -> Str
}

fn shut_both(a: OutFile) [Fs, Net] -> Str => !a {
    let code = close@Fs(a)
    let msg = close@Net(7)
    return "${code}${msg}"
}

// [effect-member-overload] An argument no overload takes is the call's error,
// naming what was passed ("no matching overload for `close(Int)`") — not a
// silent pick of the first one [backend-never-wrong].
fn shut_nothing() [Fs] -> Str {
    //~ error[call-resolve:no-overload]
    return "${close(7)}"
}

// [effect-member-overload] Overloading by *arity* alone works too, and needs
// no argument typing at all — the shape `position(s)` / `position(s, base)`
// would take.
effect Clock {
    fn now() -> Int
    fn now(offset: Int) -> Int
}

fn stamp() [Clock] -> Int {
    return now() + now(5)
}
