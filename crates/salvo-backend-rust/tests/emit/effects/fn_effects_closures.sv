// Effects threaded into fn values [fn-effects].

//--- fn_type_effects_thread_into_closures
// The same program the Kotlin suite runs: an effect-using fn value passed
// to a callee that needs one too (what the fusion cut rejected), a named fn
// whose effect list matches, and a *pure* named fn in the same position.
// Identical stdout on both backends is what parity means here.
//
// [fn-effects] [rs-handle] The effect is a leading `&E` handle
// parameter of the closure type, so nothing is captured — which is what
// lets the value cross a call that borrows the same effect value. The
// lambda takes it as a typed parameter and the named-fn adapter forwards
// the reborrow to a declaration that needs it (`shout`).
effect Logger {
    fn log(message: Str) -> None => message
}

handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")
    }
}

fn shout(s: Str) [Logger] -> Str => s {
    log("shouting ${s}")
    return "${s}!"
}

fn plain(s: Str) -> Str => s {
    return "${s}."
}

// No effect list of its own: `run_it` *inherits* `[Logger]` from `f`.
//~ emits(main.rs): f: &mut dyn FnMut(&crate::Logger, &String) -> String
fn run_it(f: (s: Str) [Logger] -> Str, value: Str) -> Str =>[f] s => value {
    return f(value)
}

fn demo() [Console, Logger] -> None {
    //~ emits(main.rs): &mut |mut __leff0, mut s| -> String {
    //~|         __leff0.log(
    println(run_it(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    }, "x"))
    //~ emits(main.rs): |__e0: &crate::Logger, __a0: &String| crate::shout(__e0, __a0)
    println(run_it(shout, "one"))
    //~ emits(main.rs): |mut __fx0, mut __a0| -> String {
    //~|         crate::plain(__a0)
    println(run_it(plain, "two"))
}

fn main() [use] -> None {
    use StdConsole
    use ConsoleLogger()
    demo()
}
