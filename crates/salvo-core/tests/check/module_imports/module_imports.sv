// [mod-import-module] The whole-module import: `import chrono` brings every
// name in a module into scope, at a rung below a named import and above
// implicit `std.core.*` visibility (user decision 2026-09-18).
//
// The rule's point is that a bulk import never *fights* anything: it loses
// silently to an own declaration and to a named import, functions resolve
// through the ordinary ladder with `@module` as the tie-break
// [fn-overload-scope], and only two bulk imports carrying one *type* name
// produce a diagnostic — a warning, since there is no `@` form for a type.
//
// (The old tests' module was `time` and its type `Span`; std has a
// `std.time` and std.core a `Span`, so here they are `chrono` and
// `Interval`.)

// [mod-import-module] One line brings every name in the module: a struct, an
// effect and a free fn, none of them named in the import.

// FILE: chrono.sv
export struct Interval {
    nanos: Long
}

export effect Ticking {
    fn tick() -> Long
}

export fn millis(n: Long) [] -> Interval {
    return Interval {nanos: n}
}

// FILE: main.sv
import chrono

fn use_it() [Ticking] -> Interval {
    let now = tick()
    return millis(now)
}

//--- a module import names one module, not a tree
// [mod-import-module] A whole-module import names **one** module, not the
// tree under it (user decision 2026-09-26): importing less is what an import
// is for, and importing more is one more statement. A path that is only a
// *prefix* says so and lists the modules it could have meant.

// FILE: chrono/types.sv
export struct Interval {
    nanos: Int
}

// FILE: chrono/clock.sv
export fn seconds(n: Int) [] -> Int {
    return n * 1000
}

// FILE: main.sv
//~ error[mod-import-module:path-prefix]
import chrono

fn f() [] -> Int {
    //~ error[call-resolve:unknown-fn]
    return seconds(2)
}

//--- a submodule is imported by naming it
// [mod-import-module] And naming the submodule works — one line per module,
// so a file states exactly which of a tree's modules it depends on.

// FILE: chrono/types.sv
export struct Interval {
    nanos: Int
}

// FILE: chrono/clock.sv
export fn seconds(n: Int) [] -> Int {
    return n * 1000
}

// FILE: main.sv
import chrono.types
import chrono.clock

fn f() [] -> Interval {
    return Interval {nanos: seconds(2)}
}

//--- an own declaration beats a module import
// [mod-import-module] An own declaration wins over a bulk import, silently:
// a convenience import must not break a file that declares its own
// `Interval`.

// FILE: chrono.sv
export struct Interval {
    nanos: Long
}

// FILE: main.sv
import chrono

struct Interval {
    label: Str
}

fn f() [] -> Interval {
    return Interval {label: "mine"}
}

//--- a named import beats a module import
// [mod-import-module] A *named* import outranks a bulk one, also silently —
// which is how a program picks a specific module's type when two carry the
// name.

// FILE: chrono.sv
export struct Interval {
    nanos: Long
}

// FILE: other.sv
export struct Interval {
    label: Str
}

// FILE: main.sv
import chrono
import other.Interval

fn f() [] -> Interval {
    return Interval {label: "b"}
}

//--- two module imports of one type name
// [mod-import-module] Two bulk imports carrying one type name: the first
// wins and the second warns, because a type has no `@module` form to
// disambiguate with — the remedy the message names is a named import.

// FILE: chrono.sv
export struct Interval {
    nanos: Long
}

// FILE: other.sv
export struct Interval {
    label: Str
}

// FILE: main.sv
import chrono
//~ warning[mod-import-module:two-bulk-imports]
import other

fn f() [] -> Long {
    return 1
}

//--- a module import competes with the own module
// [mod-import-module] [fn-overload-ambiguous] A bulk-imported overload does
// **not** rank below this module's: both fit, nothing separates them by
// signature, so the call names the place it means (user decision
// 2026-09-26). Either selector settles it.

// FILE: chrono.sv
export fn label(n: Int) [] -> Str {
    return "lib"
}

// FILE: main.sv
import chrono

fn label(n: Int) [] -> Str {
    return "mine"
}

fn pick() [] -> Str {
    //~ error[fn-overload-ambiguous:call]
    return label(1)
}

fn pick_mine() [] -> Str {
    return label@main(1)
}

fn pick_theirs() [] -> Str {
    return label@chrono(1)
}

//--- a module import has no alias
// [mod-import-module] A module import has no alias: there is nothing to
// qualify a renamed module with.

// FILE: chrono.sv
export struct Interval {
    nanos: Long
}

// FILE: main.sv
//~ error[mod-import-module:alias]
import chrono as t

fn f() [] -> Long {
    return 1
}

//--- importing core is redundant
// [mod-import-module] [mod-visibility] Importing `std.core` is redundant, and
// saying so is better than adding every core name at a second rung (which
// would double every core overload).
//~ warning[mod-visibility:redundant-import]
import std.core

fn f() [] -> Long {
    return 1
}

//--- an unknown module names the candidates
// [mod-import-module] An unknown module says so, and lists what there is —
// rather than reporting the old "import path must be `module.item`" shape
// error, which described a form the writer did not intend.

// FILE: chrono.sv
export struct Interval {
    nanos: Long
}

// FILE: main.sv
//~ error[mod-import-module:unknown-module]
import chrnoo

fn f() [] -> Long {
    return 1
}

//--- the name form is unaffected
// [mod-import-module] The name form still works, and still takes precedence
// as a *reading*: `import chrono.Interval` is one name, not the module —
// and it brings *only* that name: `Hidden` stays invisible (std.core has
// a qualifier `Other`, the old tests' name).

// FILE: chrono.sv
export struct Interval {
    nanos: Long
}

export struct Hidden {
    v: Long
}

// FILE: main.sv
import chrono.Interval

fn f() [] -> Interval {
    return Interval {nanos: 1}
}

//~ error[name-resolve:unknown-type]
fn g() [] -> Hidden {
    //~ error[name-resolve:unknown-type]
    return Hidden {v: 1}
}
