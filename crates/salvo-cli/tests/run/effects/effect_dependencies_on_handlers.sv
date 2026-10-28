// effect dependencies on handlers [effect-handler-deps]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- handler_dependencies

effect Logger {
    fn log(message: Str) -> None => message
}

handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")
    }
}

fn work() [Logger] -> None {
    log("from work")
}

fn main() [use] -> None {
    use StdConsole
    use ConsoleLogger()
    work()
    log("from main")
}
//= stdout
//= LOG: from work
//= LOG: from main
