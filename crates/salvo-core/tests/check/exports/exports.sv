// [mod-export] Module-private by default, `export` to let a declaration out
// (user decision 2026-09-18).
//
// The rule is one line — a declaration is visible outside its own module
// only if it says `export` — and what these cases pin is the surface around
// it: every declaration kind takes the modifier, the *diagnostics* say
// "declared but not exported" rather than "no such name" (the difference
// between a rule and a mystery), an import of a private name is refused at
// the import, and `export` stays an ordinary identifier everywhere else.

// [mod-export] An exported declaration crosses; the module's own code sees
// everything either way: `both` calling `hidden` is what "module-private"
// means.

// FILE: lib.sv
export fn shown() [] -> Int {
    return 1
}

fn hidden() [] -> Int {
    return 2
}

export fn both() [] -> Int {
    return shown() + hidden()
}

// FILE: main.sv
import lib.shown
import lib.both

fn use_them() [] -> Int {
    return shown() + both()
}

//--- a private name is reported as private
// [mod-export] The diagnostic is the feature: a name that exists and is
// private says so, and names the fix — and it does not suggest an import
// that could not work.

// FILE: lib.sv
fn hidden() [] -> Int {
    return 2
}

// FILE: main.sv
fn f() [] -> Int {
    //~ error[call-resolve:unknown-fn]
    return hidden()
}

//--- importing a private name
// [mod-export] An import of a private name is refused **at the import**,
// which is where the reader is looking, rather than silently importing
// nothing and failing at every use.

// FILE: lib.sv
struct Point {
    x: Int
}

// FILE: main.sv
//~ error[mod-export:not-exported]
import lib.Point

fn f() [] -> Int {
    return 1
}

//--- a wrong-kind name in scope
// [mod-export] A name that *is* in scope under another kind has a different
// problem, and must not be told about exports — it would send the reader to
// the wrong file. `Tag` is a qualifier here, used where a type belongs;
// another module declares a private `Tag` too, which must not colour the
// diagnostic.

// FILE: other.sv
struct Tag {
    v: Int
}

// FILE: main.sv
qualifier Tag of Int

//~ error[name-resolve:unknown-type]
fn f(x: Tag) [] -> Int {
    return 1
}

//--- every declaration kind takes the modifier
// [mod-export] Every declaration kind takes the modifier, and it comes
// *first* — before `intrinsic` / `linear` / `actor` / `platform` /
// `provenance`.

// FILE: lib.sv
export struct Shape {
    side: Int
}

export linear struct Ticket {
    id: Int
}

export type Alias = Int

export qualifier Tagged of Int

export provenance qualifier Vetted of Shape

export effect Logger {
    fn log(m: Str) -> None => !m
}

export actor effect Mailer {
    send fn mail(m: Str) => !m
}

export effect Host {
    fn tick() [] -> Int
}

export platform handler HostTick of Host

export handler Loud of Logger {
    fn log(m: Str) -> None {}
}

export fns ToText<T> {
    fn text(v: T) -> Str => v
}

export fn plain() [] -> Int {
    return 1
}

export fn close(t: Ticket) [] -> None => !t {
    discard(t)
}

// FILE: main.sv
import lib.Shape
import lib.Ticket
import lib.Alias
import lib.Tagged
import lib.Vetted
import lib.Logger
import lib.Mailer
import lib.Host
import lib.HostTick
import lib.Loud
import lib.ToText
import lib.plain

fn f() [] -> Int {
    return plain()
}

//--- an iter fn's pass type is exported with it
// [mod-export] An `iter fn`'s generated pass type inherits the function's
// visibility: a `for` over the pass needs the *type* in scope, so exporting
// the function while hiding its pass would make the iterator undrivable from
// another module — importing `lib.CountdownIter` would be refused as
// private. [iter-for] [fn-attached] And the pass's `next` travels with the
// pass type (B3.3d), so the imported pass type is what drives the loop.
// (The old test declared its own step protocol; std's is
// `std.core.iterator`'s.)

// FILE: lib.sv
export struct Countdown {
    from: Int
}

export iter fn iter(c: Countdown) [] -> Emitted Int | Finished {
    state CountdownIter {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

// FILE: main.sv
import lib.Countdown
import lib.iter
import lib.CountdownIter

fn total() [] -> Int {
    let sum = 0
    let c = Countdown {from: 3}
    for x in iter(c) {
        sum = sum + x
    }
    return sum
}

//--- a pass whose next is not in scope names the import
// [iter-for] [fn-attached] [backend-never-wrong] A `for` over an `iter fn`'s
// pass from another module calls the pass's `next`, which travels with the
// pass *type* (B3.3d): without the type imported no `next` takes the value,
// and the loop says which import brings it. Until B3.3d the loop answered
// the `: Next` clause's element with no `next` to call and emitted a native
// loop over the struct, which both target compilers refused.

// FILE: lib.sv
export struct Countdown {
    from: Int
}

export iter fn iter(c: Countdown) [] -> Emitted Int | Finished {
    state CountdownIter {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

// FILE: main.sv
import lib.Countdown
import lib.iter

fn total() [] -> Int {
    let sum = 0
    let c = Countdown {from: 3}
    //~ error[iter-resolve:not-iterable]
    for x in iter(c) {
        sum = sum + x
    }
    return sum
}
