// the interceptor chain [effect-handle]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [effect-handle] [effect-handler-deps] [rs-handle] The chain end to end:
// `MemCounter` behind its lock, `PlainLogger` bare with a captured console
// handle, `Shout` wrapping the `Logger` registered before it — and the
// count proving both `work` calls went through the interceptor and the
// monitor. Byte-identical stdout on Kotlin.
//--- a_shareable_interceptor_chain

effect Counter {
    fn bump() -> None
    fn total() -> Int
}

handler MemCounter of Counter {
    state {
        n: Int = 0
    }
    fn bump() { n = n + 1 }
    fn total() -> Int { return n }
}

effect Logger {
    fn log(m: Str) -> None => m
}

handler PlainLogger [Console] of Logger {
    fn log(m: Str) -> None => m {
        println("log: ${m}")
    }
}

handler Shout [Logger, Counter] of Logger {
    fn log(m: Str) -> None => m {
        bump()
        log("${m}!")
    }
}

fn work(step: Str) [Logger] -> None => step {
    log(step)
}

fn audit() [Counter, Console] -> None {
    println("shouted ${total()}")
}

fn main() [use] -> None {
    use StdConsole
    use MemCounter()
    use PlainLogger()
    work("plain")
    use Shout()
    work("loud")
    work("louder")
    audit()
}
//= stdout
//= log: plain
//= log: loud!
//= log: louder!
//= shouted 2
