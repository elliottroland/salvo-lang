// ===== [fn-value-select] passing a function by name =====

// The **expected fn type** selects the overload — before this, an
// overloaded name resolved to whichever was declared first and then failed
// to match wherever it was going.
fn tag(v: Int) [] -> Str => v { return "int" }
fn tag(v: Str) [] -> Str => v { return "str" }
fn apply(f: (Str) -> Str, s: Str) [] -> Str => f, s { return f(s) }

fn selected_by_the_expected_type() -> None {
    let _out = apply(tag, "x")
}

// With nothing to fit, an overloaded name is an ambiguity naming both
// remedies ("`tag` is overloaded", `rename fn`).
fn no_expectation() -> None {
    //~ error[fn-value-select:overloaded]
    let _f = tag
}

//--- an equal signature beside an import
// FILE: lib.sv
// [fn-value-select] [fn-overload-scope] By name, as at a call, every visible
// overload competes whatever rung brought it in (ROADMAP 0w): an
// equal-signature overload of this module's beside an imported one is an
// ambiguity naming both candidates (`describe(Int)` from `lib` and from
// `main`), both `@` remedies and `rename`, where it used to take this
// module's by rung; `@` still selects either.
export fn describe(v: Int) [] -> Bool { return true }
// FILE: main.sv
import lib.describe

fn describe(v: Int) [] -> Bool { return false }
fn apply(f: (v: Int) -> Bool, v: Int) [] -> Bool => f { return f(v) }

fn by_name() -> None {
    //~ error[fn-value-select:ambiguous]
    let _b = apply(describe, 1)
}

fn at_lib() -> None {
    let _b = apply(describe@lib, 1)
}

fn at_main() -> None {
    let _b = apply(describe@main, 1)
}

//--- by name across scopes
// FILE: lib.sv
// [fn-value-select] [fn-overload-rank] By name, the more specific signature
// wins across scopes, as at a call: an imported `tag(Int)` over this
// module's `tag<T>` (the 0w repro; the e2e cases print which ran).
export fn tag(v: Int) [] -> Str => v { return "int" }
// FILE: main.sv
import lib.tag

fn tag<T>(v: T) [] -> Str => v { return "any" }
fn apply(f: (x: Int) -> Str, x: Int) [] -> Str => f { return f(x) }

fn probe() -> None {
    let _a = apply(tag, 1)
    let _b = apply(tag@main, 1)
}
