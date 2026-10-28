// [implicit-param] An effect member is an ordinary signature, so it may
// declare implicit parameters (user decision 2026-09-06). They resolve at
// the *call*, and the handler receives them like any other argument.
fn fmt(n: Int) -> Str => n { return "" }
fn loud(n: Int) -> Str => n { return "" }

effect Show {
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v
}

handler Angle of Show {
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v { return fmt(v) }
}

fn member_takes_implicits() [use] -> Str {
    use Angle()
    return show(7)
}

// And the call site can override one, exactly as for a fn.
fn member_call_overrides() [use] -> Str {
    use Angle()
    return show(7, fmt = loud)
}

// [copy-implicit] A handler *constructor* may take implicit parameters
// (lifted 2026-09-11): the `use` site is where the handler's type arguments
// are known, so it resolves them as a call resolves a fn's. The forcing case
// is a generic handler that must `copy` a `T` it cannot see through.
// (`Display`/`Bracket` were a second `Show`/`Angle`.)
effect Display {
    fn display(v: Int) -> Str => v
}

handler Bracket(?fmt: (n: Int) -> Str) of Display {
    fn display(v: Int) -> Str => v { return fmt(v) }
}

fn constructor_resolves_at_use() [use] {
    use Bracket()
    let _s = display(1)
}

//--- handler type arguments
// ===== [effect-handler-generics] a `use` may write its handler's types =====
//
// A handler with no constructor argument has nothing to infer its generics
// from, so the `use` site's written type arguments are the only source. They
// used to be discarded silently, which both backends then turned into invalid
// target code — see COMPLETED.md's closed-defect entry, fixed 2026-09-06.
effect Show<T> {
    fn show(v: T) -> Str => v
}

handler Plain<T> of Show<T> {
    fn show(v: T) -> Str => v { return "plain" }
}

// [effect-state-store] The member does not *return* `prefix`: a handler's own
// storage outlives every member call, so handing it out is `copy(prefix)`.
// What this is about is the type argument a constructor argument binds, so a
// literal answer says the same thing.
handler Prefixed<T>(prefix: Str) of Show<T> {
    fn show(v: T) -> Str => v { return "p" }
}

// The written arguments make the instance concrete, which is what a member
// call resolves against.
fn written() [use] -> Str {
    use Plain<Int>()
    return show(1)
}

// A constructor argument still binds them, as it always did.
fn by_the_constructor() [use] -> Str {
    use Prefixed<Int>("p")
    return show(1)
}

// Written arguments that disagree with what an argument implies
// (`use Echo<Str>(1)`) stay a Rust test, `implicit_tests.rs`: they report two
// errors for the one mistake.

// And the wrong *number* of them is reported against the handler.
fn wrong_count() [use] -> Str {
    //~ error[effect-handler-generics:type-arg-count]
    use Plain<Int, Str>()
    return show(1)
}

//--- an effect member fills an implicit
// The member is `describe` (the old prelude's `to_str`): std declares
// `to_str` for every scalar, and an unfilled `?to_str` would be the near
// miss against them rather than "no `to_str` for this call".
struct Point { x: Int }

effect Fmt {
    fn describe(p: Point) -> Str => p
}

handler Plain of Fmt {
    fn describe(p: Point) -> Str { return "p" }
}

fn render(p: Point, ?describe: (p: Point) -> Str) [] -> Str => p { return describe(p) }

// [effect-member-overload] [effect-available] An available effect's member
// fills an implicit position (R10 D3: an effect's members fill every
// position a fn does), passed as a value that carries its handler — so the
// pure `render` takes it. Before, "no `describe` fits".
fn show(p: Point) [Fmt] -> Str => p { return render(p) }

fn probe() [use] -> Str {
    use Plain()
    return render(Point { x: 1 })
}

// [effect-available] With no instance in scope the member is no candidate
// ("no `describe` for this call").
fn unavailable() [] -> Str {
    //~ error[implicit-resolve:none]
    return render(Point { x: 1 })
}

// [effect-member-overload] [implicit-infer] A member teaches as it fills:
// `?f: (x: Int) -> B` filled by an available member `f(x: Int) -> Str`
// infers `B = Str`.
effect Conv {
    fn f(x: Int) -> Str => x
}

fn conv<B>(x: Int, ?f: (x: Int) -> B) [] -> B => x { return f(x) }
fn takes(s: Str) [] -> Str => !s { return s }

fn inference_reads_a_member() [Conv] -> Str {
    let s = conv(1)
    return takes(s)
}

//--- a member and a fn tie at an implicit
// [effect-member-overload] [fn-overload-ambiguous] A member and a fn of one
// signature tie, and the remedies name both — the member as a value,
// `describe@Fmt`, included ("`describe = describe@Fmt` or `describe = describe@main`")
// — which then settle it. A program of its own: the fn `describe(Point)` would
// tie every other fill of `?describe`.
struct Point { x: Int }

effect Fmt {
    fn describe(p: Point) -> Str => p
}

fn render(p: Point, ?describe: (p: Point) -> Str) [] -> Str => p { return describe(p) }

fn describe(p: Point) [] -> Str => p { return "fn" }

fn show(p: Point) [Fmt] -> Str => p {
    //~ error[fn-overload-ambiguous:implicit]
    return render(p)
}

fn show_settled(p: Point) [Fmt] -> Str => p {
    let _a = render(p, describe = describe@Fmt)
    return render(p, describe = describe@main)
}
