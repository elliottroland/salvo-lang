// Effects on fn types [fn-effects].
//
// An effect list on a fn type is a *requirement the caller of the value
// supplies*, not a capability the value carries: `(s: Str) [Logger] -> Str`
// means "call me with Logger available". So a lambda body performs only the
// effects its type declares, a higher-order fn *inherits* its fn-typed
// parameters' effects (user decision 2026-09-04 — the only reason to take
// `f` is to call it), and a fn value can be stored or passed freely because
// it holds no handler: only *calling* it needs one.
//
// The old tests each declared their own `run`; here each shape has a name
// (`run_logged`, `run_pure`, …) and each case its own caller.

effect Logger {
    fn log(message: Str) -> None => message
}

effect Counter {
    fn bump() -> None
}

handler QuietLogger of Logger {
    fn log(message: Str) -> None => message {}
}
handler ZeroCounter of Counter {
    fn bump() -> None {}
}

fn note(text: Str) [] -> None => text {}

// A higher-order fn whose parameter declares `[Logger]`, and one whose
// parameter declares nothing.
fn run_logged(f: (s: Str) [Logger] -> Str) -> None =>[f] s {
    note(f("x"))
}

fn run_pure(f: (s: Str) -> Str) -> None =>[f] s {
    note(f("x"))
}

// ===== a lambda body performs what its type declares =====

// [fn-effects] A lambda passed where a fn type declares the effect may
// perform it — the call site supplies it.
fn declared_in_the_body() [Logger] -> None {
    run_logged(s -> {
        log("in lambda ${s}")
        return "seen ${s}"
    })
}

// [fn-effects] The dual: a lambda checked against a fn type that declares
// *nothing* may not perform an effect ("no handler for effect `Logger`"),
// even though the enclosing scope has one. Lexical leakage is what made a fn
// value's capabilities invisible in its type.
fn undeclared_in_the_body() [Logger] -> None {
    run_pure(s -> {
        //~ error[effect-available:no-handler]
        log("in lambda ${s}")
        return "seen ${s}"
    })
}

// ===== inheritance (user decision 2026-09-04) =====

// [fn-effects] A fn taking an effectful fn value *inherits* the effect: it
// needs no list of its own to call `f`, and its callers supply it.
fn supplies_the_inherited() [Logger] -> None {
    run_logged(s -> "${s}")
}

// [fn-effects] The other half of inheritance: a caller that *cannot*
// supply the inherited effect is rejected ("no handler for effect `Logger`"
// at the call of `run_logged`), since that is where the value comes from at
// run time.
fn cannot_supply_the_inherited() [] -> None {
    //~ error[effect-available:callee-no-handler]
    run_logged(s -> "${s}")
}

// [fn-effects] Inheritance reaches through a qualifier: `once` fn types
// carry effects the same way.
fn run_once(f: once () [Logger] -> None) -> None => !f {
    f()
}

fn through_a_qualifier() [Logger] -> None {
    run_once(() -> { log("once") })
}

// [fn-effects] A written list is the whole set, `[]` included (user
// decision 2026-10-20): a fn that writes one inherits nothing, so calling
// an effectful parameter it did not list is refused at the call, and its
// callers need not supply the effect.
fn run_listing_nothing(f: (s: Str) [Logger] -> Str) [] -> None =>[f] s {
    //~ error[effect-available:fn-value]
    note(f("x"))
}

// Listing the effect is the way to pass it on.
fn run_listing_logger(f: (s: Str) [Logger] -> Str) [Logger] -> None =>[f] s {
    note(f("x"))
}

fn calls_the_listing_one() [Logger] -> None {
    run_listing_logger(s -> "${s}")
}

// ===== variance =====

// [fn-effects] A value performing *fewer* effects fits where more are
// expected: the caller supplies what it declared and the value ignores it.
fn pure_fn(s: Str) [] -> Str => s {
    return "plain ${s}"
}

fn fewer_fit() [Logger] -> None {
    run_logged(pure_fn)
    run_logged(s -> "${s}")
}

// [fn-effects] Never the reverse: a value that performs an effect its
// position does not declare would reach a call site that cannot supply it.
fn shout(s: Str) [Logger] -> Str => s {
    log(s)
    return "loud ${s}"
}

fn more_do_not_fit() [Logger] -> None {
    //~ error[call-resolve:no-overload]
    run_pure(shout)
}

// ===== inference =====

// [fn-effects] An un-annotated lambda's effect set is *inferred* from its
// body (inference from a visible body is what [decl-explicit] permits), so
// it does not silently fit a pure position.
fn inferred() [Logger] -> None {
    let f = (s: Str) -> {
        log(s)
        return "logged ${s}"
    }
    //~ error[call-resolve:no-overload]
    run_pure(f)
}

// ===== a fn value carries no capability =====

// [fn-effects] Storing an effectful fn value is fine — it holds no handler.
// What needs the effect is *calling* it, which is why "forbid escape" was
// dropped from the design (user decision 2026-09-04). `g` declares no
// effects, so the `f(s)` inside it has no Logger ("no handler for effect
// `Logger`", "function value").
fn holding_is_fine() [Logger] -> None {
    let f = (s: Str) -> {
        log(s)
        return "logged ${s}"
    }
    let g: (s: Str) -> Str = (s: Str) -> {
        //~ error[effect-available:fn-value]
        return f(s)
    }
    note(g("x"))
}

// [fn-effects] Two effects on one fn type both have to be supplied ("no
// handler for effect `Counter`").
fn run_two(f: (s: Str) [Logger, Counter] -> Str) -> None =>[f] s {
    note(f("x"))
}

fn supplies_one_of_two() [Logger] -> None {
    //~ error[effect-available:callee-no-handler]
    run_two(s -> "${s}")
}

// ===== [fn-effects] a pass performs its effects in its `next` =====
//
// A producer used to be a *type* (`Logger Iter<Int>`) whose driving performed
// the claim, so its effects were written in qualifier position on the return
// type (D8, 2026-09-07). With the reduction to `next` a producer is a struct
// and its `next` is an ordinary function, so there is nothing special left:
// `[fn-effects]` says it all, and the old spellings are refused.

// An effect name in qualifier position is refused, naming the replacement
// ("is an effect, not a qualifier", "yield fn next"): the effect list of the
// `next` that performs it.
//~ error[fn-effects:effect-as-qualifier]
fn in_qualifier_position(n: Logger Int) -> None => !n {
    return None
}

// A fn type is redirected to its own bracket list ("declares its effects in
// its own list"), as it always was: two spellings of one thing is how they
// drift.
//~ error[fn-effects:in-qualifier-position]
fn on_a_fn_type(g: Logger (Int) -> Int) -> None => !g {
    return None
}

// The restriction is on *producing*, not consuming: a `for` loop sits in an
// ordinary fn and may perform whatever that fn declares.
fn report(xs: Int[]) [Logger] -> None => xs {
    for _v in xs {
        log("one")
    }
}
