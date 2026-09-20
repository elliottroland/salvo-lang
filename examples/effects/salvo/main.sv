// Effects: the capabilities a function declares, the handlers that implement
// them, and what happens when several are in play at once.
//
// An effect is a set of functions that only code with a *handler* for them may
// call. A function declares the effects it needs in `[...]` before its arrow,
// and `use` registers a handler for the rest of the scope. Nothing is ambient:
// a function that does not declare an effect cannot perform it, and a caller
// that cannot supply one cannot call it.
//
// What this program is chosen to show, in the order it bites:
//
//   1. an effect, a handler with state, and `use`
//   2. several effects in one signature — and a callee that needs fewer
//   3. a handler that depends on another effect
//   4. interception: a handler that depends on the effect it implements
//   5. shadowing, and the block a registration lives in
//   6. two effects that share a member name
//   7. two instances of one generic effect
//
// The shape to notice across all of it: only `main` ever names a handler.
// Everything else names capabilities, which is what makes the composition
// swappable.

// ===== 1. an effect, a handler, and `use` =====
//
// The effect declares what can be called. `Clock` is one function.
effect Clock {
    fn now() -> Int
}

// A handler implements every member, and may keep **state** for its lifetime —
// declared with an initializer, and private to it. This one is a fake clock:
// each reading is five ticks after the last, so the program's output does not
// depend on when it ran.
handler TickingClock of Clock {
    tick: Int = 0

    fn now() -> Int {
        tick = tick + 5
        return tick
    }
}

// ===== 2. several effects in one signature =====
//
// Two effects, and the list is the whole story: `stamp` may read the clock and
// may print, and may do nothing else. `Console` is std's, and `println` is one
// of its members — the effect the other examples in this tree use without ever
// remarking on it.
fn stamp(label: Str) [Clock, Console] -> None => label {
    let t = now()
    banner("${label} at t=${t}")
}

// A callee needing *fewer* effects than its caller. Nothing is passed at the
// call site: the caller's set covers the callee's, and the compiler checks
// exactly that.
fn banner(text: Str) [Console] -> None => text {
    println("   ${text}")
}

// ===== 3. a handler that depends on another effect =====
//
// A `Logger` has to print, but an effect *member* may not declare effects of
// its own — the interface would then differ per implementation. So the
// dependency belongs to the handler, and it is written the way a function
// writes one: an effect list on the declaration. Two consequences worth
// seeing:
//
//   * the `use` site writes `use PlainLogger` — never the console;
//   * callers of `log` declare `[Logger]` and nothing else, so swapping in a
//     logger that needs a *different* effect changes no signature anywhere.
//
// The dependencies have no names, because nothing could refer to them: code
// inside a handler reaches an effect the way all Salvo code does, by calling
// its members (`println` below).
effect Logger {
    fn log(message: Str) -> None => message
}

handler PlainLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("   ${message}")
    }
}

// A handler that throws its input away — used in section 5. It needs nothing at
// all, which is the point: `Logger` says nothing about how logging happens, so
// "not at all" is a legitimate implementation.
handler QuietLogger of Logger {
    fn log(message: Str) -> None => message {
    }
}

// ===== 4. interception =====
//
// A handler may depend on **the effect it implements**. The dependency binds
// *outward* — to whatever was registered before this one — so the handler
// wraps it rather than replacing it, and a call to `log` inside the body goes
// one layer out instead of recursing.
//
// `Stamped` depends on two effects at once, one of them its own: it needs the
// clock for the timestamp and the wrapped logger to hand the line to. The
// list reads like a fn's, and means the same thing — "this needs these to
// run" — with the compiler supplying them where the handler is registered.
// `local` because the registration below sits in a fn that *received* these
// effects through its signature: a plain `use` captures its dependencies as
// owned handles, which needs a binding in the same function, so wiring that
// works over signature-supplied effects stays scope-local and says so.
handler Stamped [local Logger, local Clock] of Logger {
    fn log(message: Str) -> None => message {
        log("[t=${now()}] ${message}")
    }
}

// A second one, with state of its own. Interceptors stack: registering this
// over `Stamped` numbers the line the stamping logger will then print.
handler Numbered [local Logger] of Logger {
    seen: Int = 0

    fn log(message: Str) -> None => message {
        seen = seen + 1
        log("#${seen} ${message}")
    }
}

// The function doing the work knows none of this. It declares `[Logger]`, and
// what that means is decided entirely by the `use` above it.
fn work(step: Str) [local Logger] -> None => step {
    log(step)
}

// Note where the *dependencies* appear: `interception` registers `Stamped`,
// so `interception` is what needs a `Clock` available — its callers do not.
// Composition is where the wiring lives.
fn interception() [local Logger, local Clock, use] -> None {
    work("4. plain")
    use local Stamped
    work("4. stamped")
    use local Numbered
    work("4. numbered, then stamped")
    work("4. and again")
}

// ===== 5. shadowing, and the block a registration lives in =====
//
// A `use` for an effect already in scope is not an error: it takes over for
// the rest of the block, and what it shadowed comes back at the closing brace.
// `QuietLogger` is not an interceptor — it has no dependency and swallows what
// it is given — so this is shadowing on its own, without wrapping.
fn scoping() [Logger, use] -> None {
    work("5. before the block")
    if true {
        use QuietLogger
        work("5. this line is swallowed")
    }
    work("5. after the block, logging again")
}

// ===== 6. two effects that share a member name =====
//
// `record` on two effects is the natural spelling, not a collision. A bare
// call resolves through whichever effect actually has a handler here; where
// both do, `@` picks — the same selector that picks a module's overload.
effect Audit {
    fn record(what: Str) -> None => what
}

effect Metrics {
    fn record(what: Str) -> None => what
}

handler ConsoleAudit [Console] of Audit {
    fn record(what: Str) -> None => what {
        println("   audit: ${what}")
    }
}

handler ConsoleMetrics [Console] of Metrics {
    fn record(what: Str) -> None => what {
        println("   metric: ${what}")
    }
}

// Only `Audit` is available, so the bare name is unambiguous.
fn audit_only(what: Str) [Audit] -> None => what {
    record(what)
}

// Both are, so both calls say which.
fn audit_and_measure(what: Str) [Audit, Metrics] -> None => what {
    record@Audit(what)
    record@Metrics(what)
}

// ===== 7. two instances of one generic effect =====
//
// A generic effect is a family, and `Setting<Int>` and `Setting<Str>` are two
// separate capabilities: both can be in scope at once, and a call picks its
// instance by the expected type or by writing the type argument.
effect Setting<T> {
    // `?copy` is an implicit parameter: the handler stores its value for
    // every call, so handing one out needs an independent copy — and *how*
    // to copy a `T` is the caller's knowledge, not the handler's, so the
    // call site fills it in [copy-implicit] [effect-state-store].
    fn setting(?copy: (v: T) -> T) -> T
}

handler Fixed<T>(value: T) of Setting<T> {
    // The handler keeps its constructor argument for its whole life, so a
    // member cannot hand the stored value out — every call would be moving
    // the same one. `copy` is the opt-in that says "an independent value"
    // [effect-state-store].
    fn setting(?copy: (v: T) -> T) -> T {
        return copy(value)
    }
}

fn settings() [Setting<Int>, Setting<Str>, Console] -> None {
    let retries: Int = setting()
    let region = setting<Str>()
    println("   retries=${retries} region=${region}")
}

// ===== the composition root =====
//
// The only place in the program that names a handler. Reading it top to bottom
// is reading the whole configuration — and a handler registered here is
// reached by every function below it that declares the effect, without being
// threaded through the calls in between.
fn main() [use] -> None {
    use StdOutConsole
    use TickingClock

    println("1. the clock reads ${now()}, then ${now()}")

    println("2. two effects in one signature:")
    stamp("2. a labelled moment")

    println("3. a logger whose handler needs the console:")
    use PlainLogger
    work("3. logged through the console")

    println("4. interception — each `use` wraps the one before it:")
    interception()

    println("5. shadowing is not wrapping:")
    scoping()

    println("6. two effects, one member name:")
    use ConsoleAudit
    audit_only("6. audited only")
    use ConsoleMetrics
    audit_and_measure("6. audited and measured")

    println("7. two instances of one generic effect:")
    use Fixed(3)
    use Fixed("eu-west-1")
    settings()
}
