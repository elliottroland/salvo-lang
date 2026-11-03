// [fn-lambda-sig] What a lambda's written effect list and deduction clause
// may say: the clause names its own parameters, consuming (`!p`) or keeping
// (`p`) them, and nothing else; a kept parameter stays the caller's; a
// consumed one may not be where the expected fn type keeps it; the list is
// what the body may perform, and no more than the expected type's.

//--- clause_entries
struct Box { v: Str }

fn eat(b: Box) -> Str => !b {
    return b.v
}

fn apply(f: (b: Box) -> Str, b: Box) -> Str => f {
    return f(b)
}

fn main() -> None {
    let y = Box { v: "kept" }
    //~ error[fn-lambda-sig:consumes-kept]
    let _a = apply((b) => !b -> eat(b), y)
    //~ error[fn-contract:consume-kept]
    let _g = (b: Box) => b -> eat(b)
    //~ error[fn-lambda-sig:unknown-param]
    let _p = (b: Box) => c -> b.v
    //~ error[fn-lambda-sig:entry-kind]
    let _q = (b: Box) => b: Mut -> b.v
    let _ok = (b: Box) => !b -> eat(b)
}

//--- effect_lists
effect Logger {
    fn log(s: Str) -> None => s
}

fn run(f: () [Logger] -> None) [Logger] => f {
    f()
}

fn quiet(f: () [Logger] -> None) [Logger] => f {
    f()
}

fn body() [Logger, Console] -> None {
    //~ error[fn-lambda-sig:effect-not-expected]
    run(() [Console] -> println("x"))
    quiet(() [] -> {})
    //~ error[effect-available:callee-no-handler]
    let _r = () [] -> println("pure?")
    //~ error[fn-effects:use]
    let _s = () [use] -> 1
    let _t = () [Logger] -> log("fine")
}
