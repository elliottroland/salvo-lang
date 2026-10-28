// E3 step 3: effects threaded into fn values [fn-effects]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- fn_type_effects

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
fn run_it(f: (s: Str) [Logger] -> Str, value: Str) -> Str =>[f] s => value {
    return f(value)
}

fn demo() [Console, Logger] -> None {
    println(run_it(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    }, "x"))
    println(run_it(shout, "one"))
    println(run_it(plain, "two"))
}

fn main() [use] -> None {
    use StdConsole
    use ConsoleLogger()
    demo()
}
//= stdout
//= LOG: in lambda x
//= done x
//= LOG: shouting one
//= one!
//= two.
