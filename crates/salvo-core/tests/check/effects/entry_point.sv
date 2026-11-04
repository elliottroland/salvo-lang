// [main-effects] An entry point's effect list holds only what the host gives
// it, `use` and `spawn`: nothing calls `main`, so nothing could hand it a
// handler (user decision 2026-10-30, ROADMAP 0bd: `main() [Console]` checked
// clean, and neither target ran it — Rust's `main` had the wrong type, and
// the JVM found no entry point).

//--- an_effect_on_main_is_refused
effect Log {
    fn log(s: Str) -> None => s
}

handler Quiet() for Log {
    fn log(s: Str) -> None => s {}
}

//~ error[main-effects:declared]
//~ error[main-effects:declared]
fn main() [Console, Log] -> None {
    println("hi")
    log("there")
}

//--- use_and_spawn_are_the_hosts
fn main() [use, spawn] -> None {
    use StdConsole()
    println("hi")
}

//--- another_fn_may_declare_one
// Only `main` is the entry point: a fn it calls takes the handler it
// registers.
fn greet() [Console] -> None {
    println("hi")
}

fn main() [use] -> None {
    use StdConsole()
    greet()
}
