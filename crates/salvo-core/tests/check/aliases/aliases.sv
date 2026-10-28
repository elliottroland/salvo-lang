// [effect-alias] Effect-group aliases: `effects IO = [Console, Fs]` is
// shorthand for its member list anywhere an effect list appears. Expanded
// away before resolution, the same way prerequisites are (user decision
// 2026-10-14): the checker and both emitters see an ordinary program with no
// aliases in it.
//
// Over the real std, `Console` is std's (`std.core.console`), bound by
// `StdConsole`: the old prelude's `Console`/`Out` pair was a stand-in for it.

effect Clock {
    fn now() -> Int
}
handler Ticking() of Clock {
    state { tick: Int = 0 }
    fn now() -> Int {
        tick = tick + 1
        return tick
    }
}

// [effect-alias] An alias in a fn's effect list is its member list: a fn
// declaring `[App]` may call members of every effect in `App`, and nothing
// else. A transitive alias (`Big = [App, Clock]`) closes to the same set,
// deduped.
effects App = [Console, Clock]
effects Big = [App, Clock]

fn a() [App] -> None {
    print("x")
    let _ = now()
}

fn b() [Big] -> None {
    print("y")
    let _ = now()
}

fn main() [use] {
    use StdConsole()
    use Ticking()
    a()
    b()
}

// [effect-alias] A fn that declares `[ConsoleOnly]` may not perform an effect
// `ConsoleOnly` does not stand for — expansion adds nothing extra.
// (`ConsoleOnly` was a second `App`.)
effect Secret {
    fn leak() -> None
}
effects ConsoleOnly = [Console]

fn leaky() [ConsoleOnly] -> None {
    //~ error[effect-available:no-handler]
    leak()
}

// [effect-alias] An alias works in a handler's dependency list and in an
// effect's prerequisite list, not only in a fn's.
effect Logger {
    fn log(m: Str) -> None => m
}
handler StampLog() [App] of Logger {
    fn log(m: Str) -> None => m {
        print("${now()} ${m}")
    }
}

effect Reporter [App] {
    fn report(w: Str) -> None => w
}
handler Rep() of Reporter {
    fn report(w: Str) -> None => w {
        print("${now()} ${w}")
    }
}

fn do_log(m: Str) [Logger] -> None => m { log(m) }
fn do_rep(w: Str) [Reporter] -> None => w { report(w) }

fn in_deps_and_prereqs() [use] {
    use StdConsole()
    use Ticking()
    use StampLog()
    do_log("a")
    use Rep()
    do_rep("b")
}

// [effect-eff-handle] An alias flattens inside an `eff[…]` handle type too,
// not only in an effect list: `eff[App]` is `eff[Console, Clock]`, so a fn
// taking `eff[App]` accepts a handle carrying those effects — no unknown
// effect from an unexpanded `App` inside the handle type.
fn read(h: eff[App]) [use] -> None => h {
    use h
    print("x")
    let _ = now()
}

fn in_a_handle_type() [use] {
    let c = use StdConsole()
    let _ = c
}

// FILE: caps.sv
// [effect-alias] An exported alias is usable from another module: `import
// caps.App` brings the alias (`consumer.sv`), and expansion synthesizes
// imports for whichever members the consuming file cannot already name (a
// `std.core.*` effect — `Console` — needs none; a local effect of the
// alias's module — `Clock` — does).
export effect Clock {
    fn now() -> Int
}
export handler Ticking() of Clock {
    state { tick: Int = 0 }
    fn now() -> Int {
        tick = tick + 1
        return tick
    }
}
export effects App = [Console, Clock]

// FILE: consumer.sv
import caps.App
import caps.Ticking

fn report(label: Str) [App] -> None => label {
    print(label)
    let _ = now()
}

fn main() [use] {
    use StdConsole()
    use Ticking()
    report("x")
}

//--- malformed aliases
// [effect-alias] The declaration's own rules: only plain named effects as
// members — `use`, `spawn` ("`use` and `spawn`") and `any` ("`any` has no
// meaning in an effect alias") are refused where the alias is declared.
// (The names were both `A`.) The cycle half of the old test, and `any` on an
// alias at a use site, stay in `alias_tests.rs`: each reports more than its
// one mistake.
//~ error[effect-alias:capability]
effects WithUse = [Console, use]

//~ error[effect-alias:any]
effects WithAny = [any Console]
