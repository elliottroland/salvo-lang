// dependent handlers in anger [rs-handle]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [fn-effects] The program the fusion used to lose to Kotlin: a function
// *value* that uses an effect, passed to a callee that needs one too.
// Capturing made the closure hold a borrow that overlapped the call's own
// (`E0499`), and hoisting could not separate them. Threading the effect
// *into* the value lifts it: the closure captures nothing, so the two
// borrows are of different things.
//--- effect_using_fn_value

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
//= stdout
//= LOG: in lambda x
//= done x

//--- fusion

effect Random<T> {
    fn next_random() -> T
}

effect Counter {
    fn bump() -> None
    fn total() -> Int
}

effect Logger {
    fn log(message: Str) -> None => message
}

effect Audit {
    fn note(message: Str) -> None => message
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}

handler MemCounter of Counter {
    state {
        n: Int = 0
    }
    fn bump() -> None { n = n + 1 }
    fn total() -> Int { return n }
}

handler ConsoleLogger [Console] of Logger {
    state {
        seen: Int = 0
    }
    fn log(message: Str) -> None => message {
        seen = seen + 1
        println("LOG ${seen}: ${message}")
    }
}

handler CountingAudit [Console, Counter] of Audit {
    fn note(message: Str) -> None => message {
        bump()
        println("[${total()}] ${message}")
    }
}

fn shout(message: Str) [Console] -> None => message {
    println("!! ${message}")
}

fn banner() [Console, Logger] -> None {
    log("banner")
    shout("done")
}

fn draw() [Console, Random<Int>, use] -> None {
    use MemCounter
    bump()
    println("drew ${next_random<Int>()} at ${total()}")
}

fn main() [use] -> None {
    use StdConsole
    use ConsoleLogger()
    banner()
    if true {
        use MemCounter
        use CountingAudit()
        note("inner")
        banner()
    }
    log("outer again")
    use CyclicRandom(array_of(10, 20, 30))
    draw()
    draw()
}
//= stdout
//= LOG 1: banner
//= !! done
//= [1] inner
//= LOG 2: banner
//= !! done
//= LOG 3: outer again
//= drew 20 at 1
//= drew 30 at 1

//--- fusion_chain

effect Logger {
    fn log(message: Str) -> None => message
}

effect Audit {
    fn note(message: Str) -> None => message
}

effect Tally {
    fn add_up(n: Int) -> None => n
    fn tally() -> Int
}

handler MemTally of Tally {
    state {
        sum: Int = 0
    }
    fn add_up(n: Int) -> None => n { sum = sum + n }
    fn tally() -> Int { return sum }
}

handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")
    }
}

handler LoggingAudit [Logger] of Audit {
    state {
        count: Int = 0
    }
    fn note(message: Str) -> None => message {
        count = count + 1
        log("note ${count}: ${message}")
    }
}

qualifier Loud of Str {
    fn qualifies(text: Str) [Console] -> Bool {
        println("checking ${text}")
        return text.size() > 3
    }
}

fn label(n: Int) [Console] -> Str {
    println("labelling ${n}")
    return "n=${n}"
}

fn main() [use] -> None {
    use StdConsole
    use ConsoleLogger()
    use LoggingAudit()
    note("first")
    for i in List(1, 2) {
        use MemTally
        add_up(i)
        log("loop ${i} tally ${tally()}")
    }
    log(label(7))
    let text = "hello"
    if text is Loud {
        note("loud")
    }
}
//= stdout
//= LOG: note 1: first
//= LOG: loop 1 tally 1
//= LOG: loop 2 tally 2
//= labelling 7
//= LOG: n=7
//= checking hello
//= LOG: note 2: loud

//--- cross_module_fusion
// FILE: logging.sv

export effect Logger {
    fn log(message: Str) -> None => message
}

export handler ConsoleLogger [Console] of Logger {
    state {
        tag: Str = "M"
    }
    fn log(message: Str) -> None => message {
        println("${tag}: ${message}")
    }
}

export fn work() [Logger] -> None {
    log("from work")
}
// FILE: main.sv

import logging.work
import logging.Logger
import logging.ConsoleLogger

fn main() [use] -> None {
    use StdConsole
    use ConsoleLogger()
    work()
    log("from main")
}
//= stdout
//= M: from work
//= M: from main

//--- fusion_mixed

effect Logger {
    fn log(message: Str) -> None => message
}

effect Sink {
    fn keep(items: Mut List<Int>) -> None => !items
    fn kept() -> Int
}

effect Counter {
    fn bump() -> None
    fn total() -> Int
}

handler PrefixLogger(prefix: Str, level: Int) [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("${prefix}[${level}] ${message}")
        tallied(message)
    }
}

handler MemSink of Sink {
    state {
        held: Mut List<Int> = Mut List()
    }
    fn keep(items: Mut List<Int>) -> None => !items { held = items }
    fn kept() -> Int { return size(held) }
}

handler MemCounter of Counter {
    state {
        n: Int = 0
    }
    fn bump() -> None { n = n + 1 }
    fn total() -> Int { return n }
}

fn tallied(text: Str) [Console, use] -> None => text {
    use MemSink
    let xs: Mut List<Int> = Mut List()
    add(xs, size(text))
    keep(xs)
    println("  tallied ${kept()}")
}

fn twice(f: (s: Str) [Logger] -> Str) -> Str =>[f] s {
    return f("a")
}

fn report(label: Str) [Console, Logger, Counter] -> None => label {
    bump()
    log("${label} #${total()}")
}

fn main() [use] -> None {
    use StdConsole
    use PrefixLogger("L", 3)
    use MemCounter
    log(twice(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    }))
    report("one")
    let i = 0
    while i < 2 {
        use MemSink
        let ys: Mut List<Int> = Mut List()
        add(ys, copy(i))
        keep(ys)
        report("loop ${kept()}")
        i = i + 1
    }
}
//= stdout
//= L[3] in lambda a
//=   tallied 1
//= L[3] done a
//=   tallied 1
//= L[3] one #1
//=   tallied 1
//= L[3] loop 1 #2
//=   tallied 1
//= L[3] loop 1 #3
//=   tallied 1

//--- a_generic_dependent_handler
effect Store<T> {
    fn keep(value: T) -> Str => value
}

handler MemStore<T> of Store<T> {
    fn keep(value: T) -> Str => value {
        return "kept"
    }
}

handler Twice<T> [Store<T>] of Store<T> {
    fn keep(value: T) -> Str => value {
        return "${keep(value)} twice"
    }
}

fn main() [use] -> None {
    use StdConsole()
    use MemStore<Int>()
    use Twice<Int>()
    println(keep(3))
}
//= stdout
//= kept twice
