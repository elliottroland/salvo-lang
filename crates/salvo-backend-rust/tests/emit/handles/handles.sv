// Effects travel as handles: one `&E` parameter per declared effect, and a
// dependent handler captures clones of the handles it depends on.

//--- a_fn_takes_one_handle_per_declared_effect
// A handler constructor parameter of effect type is a dependency: the
// member body may use that effect, the `use` site supplies it from scope,
// and callers of the outer effect never mention it.
// [rs-handle] One parameter per declared effect, in declaration order,
// typed by the effect's handle — the only shape there is.
// No fusion items are generated.
//~ !emits: __Fx_
//~ !emits: __Prov_
//~ !emits: __Has_
effect Logger {
    fn log(message: Str) -> None => message
}

handler PlainLogger of Logger {
    fn log(message: Str) -> None => message { }
}

//~ emits: pub fn work(logger: &crate::Logger, console: &crate::std_core_console::Console)
fn work() [Logger, Console] -> None {
    log("hi")
    println("there")
}

fn main() [use] -> None {
    use StdConsole
    use PlainLogger
    work()
}

//--- signature_supplied_effects_are_handles
// The arc's shape, with nothing written where nothing needs to be: the
// interception wiring lives in a fn that *received* the effects it wires (the
// lexical cut is lifted — Rust threads the handles in one hidden bundle), a
// spawned actor inherits its dependency from the spawning scope with no
// clause at all, and a `with` clause overrides one — a private instance,
// including for the self-dependency.
// [rs-handle] [actor-spawn-inherit] The one shape (user decision 2026-09-28): a fn
// declaring `[Logger, Clock]` takes one `&E` handle per effect, a
// dependent handler captures clones of them (`Stamped::new(logger.clone(),
// clock.clone())`) whether the effect arrived through the signature or a
// `use`, a `with` item is a private instance behind its own handle, and the
// caller passes `&mut` of its own handles — no bundles, no fused value.
// No bundle, fused value or accessor trait remains.
//~ !emits: __Hs_
//~ !emits: __Fx
//~ !emits: __Has_
effect Clock {
    fn now() -> Int
}

handler TickingClock of Clock {
    state {
        t: Int = 0
    }
    fn now() -> Int { t = t + 1 return t }
}

effect Logger {
    fn log(m: Str) -> None => m
}

handler PlainLogger [Console] of Logger {
    fn log(m: Str) -> None => m { println("log: ${m}") }
}

handler Stamped [Logger, Clock] of Logger {
    fn log(m: Str) -> None => m {
        log("[t=${now()}] ${m}")
    }
}

handler FixedClock of Clock {
    fn now() -> Int { return 99 }
}

actor effect Reporter {
    send fn report(what: Str, done: Reply<Int>) => !what, !done
}

handler Reporting() [Logger] of Reporter {
    mailbox { capacity: 4 }

    send fn report(what: Str, done: Reply<Int>) {
        log("reported ${what}")
        done.send(1)
    }
}

fn work(step: Str) [Logger] -> None => step {
    log(step)
}

// One handle parameter per declared effect.
//~ emits: pub fn interception(logger: &crate::Logger, clock: &crate::Clock)
fn interception() [Logger, Clock, use] -> None {
    work("plain")
    // The capture clones the handles in scope.
    //~ emits: crate::Stamped::new(logger.clone(), clock.clone());
    //~|     let __handle_2 = crate::Logger::shared(__use_1);
    use Stamped
    work("stamped")
}

fn main() [use, spawn] -> None {
    use StdConsole
    use TickingClock()
    use PlainLogger()
    // The caller passes its handles.
    //~ emits: crate::interception(&__handle_6, &__handle_4);
    interception()
    let r = spawn Reporting() on pool(1)
    let acked = waitfor done: Reply<Int> {
        r.report("inherited", done)
    }
    let sink = acked
    // A `with` item is a private instance behind its own handle.
    //~ emits: crate::Stamped::new(__handle_6.clone(), crate::Clock::shared(crate::FixedClock::new()))
    use Stamped with FixedClock()
    work("overridden")
}

//--- a_dependent_handler_is_one_struct_however_often_it_is_used
// [rs-handle] A dependent handler is one struct with a handle field per
// dependency; a `use` of it anywhere is `E::shared(H::new(deps…))`
// — nothing per site is generated.
// Nothing per `use` site is generated.
//~ !emits: __Fx_
export effect Logger {
    fn log(m: Str) -> None => m
}

// One struct for the handler; the dependency is a handle field.
//~ emits 1: pub struct PlainLogger {
//~ emits: __dep0: crate::std_core_console::Console,
export handler PlainLogger [Console] of Logger {
    fn log(m: Str) -> None => m {
        println("log: ${m}")
    }
}

// Both fns construct the handler from their own handle.
//~ emits 2: crate::PlainLogger::new(console.clone());
//~|     let __handle_2 = crate::Logger::shared(__use_1);
export fn first() [Console, use] -> None {
    use PlainLogger()
    log("first")
}

export fn second() [Console, use] -> None {
    use PlainLogger()
    log("second")
}

export fn main() [use] -> None {
    use StdConsole
    first()
    second()
}

//--- effect_using_fn_value
// [fn-effects] The program the fusion used to lose to Kotlin: a function
// *value* that uses an effect, passed to a callee that needs one too.
// Capturing made the closure hold a borrow that overlapped the call's own
// (`E0499`), and hoisting could not separate them. Threading the effect
// *into* the value lifts it: the closure captures nothing, so the two
// borrows are of different things.
export effect Logger {
    fn log(message: Str) -> None => message
}

export handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")
    }
}

export fn run_it(f: (s: Str) [Logger] -> Str) [Console, Logger] -> None =>[f] s {
    println(f("x"))
}

export fn demo() [Console, Logger] -> None {
    // The effect arrives as a parameter of the closure, not a capture: the
    // effect's handle, typed on the lambda's leading parameter [rs-handle].
    //~ emits: : &crate::Logger, 
    run_it(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    })
}

export fn main() [use] -> None {
    use StdConsole
    use ConsoleLogger()
    demo()
}
