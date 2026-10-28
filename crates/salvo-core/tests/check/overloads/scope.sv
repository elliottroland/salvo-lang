// ===== [fn-overload-ambiguous] scope does not decide =====
//
// Over the real std: `List(1, 2)` builds the list (the old prelude's
// `of_list`), and `size` is std's own, `std.core.list`'s and
// `std.core.string`'s.
//
// A module's own overload does **not** beat `std.core`'s: two declarations that
// both fit and that the ranking cannot separate are an ambiguity, and the call
// names the place it means (user decision 2026-09-26, reversing the
// scope-precedence rule of 2026-09-07). The remedy is in the diagnostic:
// `size@std.core.list(...)`, `size@main(...)`.
fn size<T>(list: List<T>) [] -> Int => list { return 0 }

fn own_beside_core() -> None {
    let xs = List(1, 2)
    //~ error[fn-overload-ambiguous:call]
    let _picked: Int = size(xs)
}

//--- a different return type tells them apart
// [fn-overload-return] With a different return type the expected type
// tells them apart, from any module: overloading by return type is how
// each type's module can declare its own `from_str`.
fn size<T>(list: List<T>) [] -> Str => list { return "mine" }

fn by_return_type() -> None {
    let xs = List(1, 2)
    let _mine: Str = size(xs)
    let _core: Int = size(xs)
}

// [fn-overload-at] And naming the place resolves it — either place.
fn naming_the_place() -> None {
    let xs = List(1, 2)
    let _mine: Str = size@main(xs)
    let _core: Int = size@core.list(xs)
}

// It works in dot form too, since `@` attaches to the *name*.
fn in_dot_form() -> None {
    let xs = List(1, 2)
    let _mine: Str = xs.size@main()
    let _core: Int = xs.size@core.list()
}

//--- an import is the only candidate
// FILE: lib.sv
// An **import** is not a tiebreaker either: with core declaring nothing of
// the name the import simply is the only candidate…
export fn describe(v: Int) [] -> Bool { return true }
// FILE: main.sv
import lib.describe

fn probe() -> None {
    let _picked: Bool = describe(1)
}

//--- an import beside an own declaration
// FILE: lib.sv
// …but an own-module declaration beside it is an ambiguity like any other,
// naming `describe@lib(...)` and `describe@main(...)`.
export fn describe(v: Int) [] -> Bool { return true }
// FILE: main.sv
import lib.describe

fn describe(v: Int) [] -> Bool { return false }

fn probe() -> None {
    //~ error[fn-overload-ambiguous:call]
    let _picked: Bool = describe(1)
}

//--- a more specific signature from a farther scope
// A **more specific signature** from a farther scope is no longer discarded
// with a warning: the two compete on signature alone, so the precise one
// (std's `size(Str)`) wins outright, with no warning. This is the case the
// old rule handled by picking the nearer scope and warning about it.
fn size(v: Any) [] -> Str => !v { return "mine" }

fn more_specific_wins() -> None {
    let s = "abc"
    let _picked: Int = size(s)
}

// ===== [fn-overload-at] naming the module =====

// `@module` picks that module's overload. Std's `size(Str)` is what the bare
// name reaches too, so its `@core.string` is not needed (and warned about);
// `@main` is what reaches the `Any` one.
fn at_module_picks() -> None {
    let s = "abc"
    //~ warning[fn-overload-at:not-needed]
    let _core: Int = size@core.string(s)
    let _mine: Str = size@main(s)
}

//--- naming a module
// [mod-suffix] A module is named by any unambiguous **suffix** of its path
// (user decision 2026-09-29): `size@string` is `size@core.string` — which
// the bare name reaches as well, hence the warning.
fn by_a_suffix() -> None {
    let s = "abc"
    //~ warning[fn-overload-at:not-needed]
    let _core: Int = size@string(s)
}

// Naming a module that declares no fitting overload is an error listing the
// ones that do (`core.string`) — not a silent fallback — once.
fn no_fitting_overload_there() -> None {
    let s = "abc"
    //~ error[fn-overload-at:no-overload-in-module]
    let _n = size@core.list(s)
}

// `@` is the way out of a **shadowed** name: a local of the same name hides
// every function, and nothing else could reach one.
fn describe(v: Int) [] -> Str { return "fn" }

fn past_a_local() -> None {
    let describe = "a string"
    let _picked: Str = describe@main(1)
    let _local: Str = describe
}

// [fn-overload-at] The "`@` not needed" warning asks whether the bare name
// reaches the same function, and a local of that name takes a bare call
// first: beside a fn-typed local `show`, `show@main(5)` is the only way to
// the declaration and is not warned about (B1 census D13, 2026-10-27: it
// was, because the check ranked the declared fns alone). The dot form sees
// no locals [fn-dot], so `5.show@main()` is still redundant there; without
// the local, the plain form is too ("this `@main` is not needed").
fn show(n: Int) [] -> Str => n { return "decl" }

fn shadowed() -> None {
    let show = (n: Int) -> "local"
    let _l: Str = show(5)
    let _d: Str = show@main(5)
}

fn shadowed_dot() -> None {
    let show = (n: Int) -> "local"
    let _l: Str = show(5)
    //~ warning[fn-overload-at:not-needed]
    let _d: Str = 5.show@main()
}

fn plain() -> None {
    //~ warning[fn-overload-at:not-needed]
    let _d: Str = show@main(5)
}

//--- a suffix two modules end in
// FILE: shop/list.sv
// A suffix that fits two modules is refused, naming both (`std.core.list`
// and `shop.list`).
export fn size(v: Any) [] -> Str => !v { return "mine" }
// FILE: main.sv
import shop.list

fn probe() [] -> None {
    let xs: List<Int> = List(1)
    //~ error[fn-overload-at:ambiguous-suffix]
    let _n = size@list(xs)
}
