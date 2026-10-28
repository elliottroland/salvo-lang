// Effects: the handler instances a `use` registers, the handles they are
// bound behind, and how effect dependencies are threaded through calls.

//--- effects_resolve_through_checker_tables
// [effect-use] [effect-disambiguation] [kt-effect-params]
// The M5 demo (`EFFECTS_DEMO` in codegen_tests.rs): generic effects with
// multiple instances in scope, checker-driven disambiguation (explicit type
// args, expected type), handler generic inference at `use` sites, and
// handler threading through fn call sites — all resolved from the checker's
// effect tables.
effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: List<T>, ?copy: (v: T) -> T) of Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        let value = get(values, i % values.size()) ?: unreachable!()
        i = i + 1
        return copy(value)
    }
}

fn draw() [Random<Int>, Random<Str>, Console] {
    // Expected-type disambiguation picks the right handler per call: the
    // two `Random` instances are two parameters.
    //~ emits: val n: Int = random.nextRandom()
    let n: Int = next_random()
    //~ emits: val s: String = random__1.nextRandom()
    let s: Str = next_random()
    println("${s}: ${n}")
}

fn lucky_number() [Random<Int>] -> Int {
    return next_random()
}

fn main() [use] {
    use StdConsole
    // `use` registers the *concrete* effect instance: the handler's generics
    // come from the written type arguments or from the constructor
    // arguments, and are written out at the constructor either way
    // [effect-handler-generics] — the emitter does not reason about what
    // Kotlin could have inferred, and a stateless generic handler leaves it
    // nothing to infer from.
    //~ emits: : CyclicRandom<Int> = CyclicRandom<Int>(values = listOf<Int>(10, 20, 30), copy = { __a0 -> __a0 })
    use CyclicRandom(List(10, 20, 30))
    //~ emits: : CyclicRandom<String> = CyclicRandom<String>(values = listOf<String>("a", "b"), copy = { __a0 -> __a0 })
    use CyclicRandom(List("a", "b"))
    // Each instance is bound behind the effect's handle [effect-handle].
    //~ emits: : Random<Int> = __Mon_Random<Int>(__use_
    //~ emits: : Random<String> = __Mon_Random<String>(__use_
    // Callee effect dependencies are threaded in declaration order.
    //~ emits: draw(__handle_4, __handle_6, __handle_2)
    draw()
    draw()
    println("lucky: ${next_random<Int>()}")
    //~ emits: luckyNumber(__handle_4)
    println("again: ${lucky_number()}")
}

//--- a_use_may_shadow_an_earlier_registration
// [effect-use-no-dup] [effect-intercept]
// Until 2026-09-14 this was "a handler for `Console` is already registered
// in this scope"; shadowing is now how interception is written, so a second
// registration is legal and the innermost wins. Both registrations are
// emitted.
fn main() [use] -> None {
    //~ emits 2: StdConsole()
    use StdConsole
    use StdConsole
}

//--- a_generic_dependent_handler_holds_a_typed_dependency_field
// [kt-handle] [effect-handler-deps] A **generic dependent handler**
// (`Twice<T> [Store<T>]`) is an ordinary class with a `__dep_Store: Store<T>`
// field: nothing generated has to name the handler's generics, so the cut
// the fused emission stated ("an effect set fused here is still generic")
// is gone (2026-09-28). Same program and output on the Rust backend.
effect Store<T> {
    fn keep(value: T) -> Str => value
}

handler MemStore<T> of Store<T> {
    fn keep(value: T) -> Str => value {
        return "kept"
    }
}

//~ emits: class Twice<T>(private val __dep0: Store<T>) : Store<T>
handler Twice<T> [Store<T>] of Store<T> {
    fn keep(value: T) -> Str => value {
        return "${keep(value)} twice"
    }
}

fn main() [use] -> None {
    use StdConsole()
    use MemStore<Int>()
    // [effect-handle] The `use` passes the scope's handle of `Store<Int>`.
    //~ emits: : Twice<Int> = Twice<Int>(__handle_
    use Twice<Int>()
    println(keep(3))
}

//--- effect_params_avoid_user_names
// [kt-effect-params] Effect parameters avoid user parameter names.
// The generated effect parameter picks a fresh name.
//~ emits: fun shadowed(console__1: salvo.std.core.console.Console, console: String)
export fn shadowed(console: Str) [Console] -> None => !console {
    //~ emits: println(console__1, "param: ${console}")
    println("param: ${console}")
}

export fn main() [use] -> None {
    use StdConsole
    shadowed("value")
}

//--- effect_member_generics_render_on_the_interface
// [effect-member-generics] Member generics render on the interface
// member and bind per call from argument types.
// (`MEMBER_GENERICS_DEMO` in codegen_tests.rs.)
effect Stash {
    //~ emits: fun<T> pick(a: T, b: T): T
    fn pick<T>(a: T, b: T) -> T => !a, !b
}

handler FirstStash of Stash {
    fn pick<T>(a: T, b: T) -> T
    => !a, !b {
        return a
    }
}

fn main() [use] -> None {
    use StdConsole
    use FirstStash
    let x = pick(7, 2)
    let s = pick("l", "r")
    println("${x} ${s}")
}

//--- aliased_effect_types_resolve_to_the_same_handler
// [effect-disambiguation] [kt-effect-params] The emitter's effect
// environment is keyed by the checker's lowered effect types
// (`Checked::fn_effects` / `use_effects` / `call_effects`), not by type
// renderings: an effect written through a type alias resolves to the same
// instance a `use` registered under the canonical type.
// (`ALIASED_EFFECT_DEMO` in codegen_tests.rs.)
type Count = Int

effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: List<T>, ?copy: (v: T) -> T) of Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        let value = get(values, i % values.size()) ?: unreachable!()
        i = i + 1
        return copy(value)
    }
}

fn roll() [Random<Count>] -> Count {
    return next_random()
}

fn main() [use] -> None {
    use StdConsole
    // The `use` registers `Random<Int>`; `roll`'s `Random<Count>`
    // parameter and its call site must agree with it.
    //~ emits: : CyclicRandom<Int> = CyclicRandom<Int>(values = listOf<Int>(7, 8), copy = { __a0 -> __a0 })
    //~ emits: : Random<Int> = __Mon_Random<Int>(
    use CyclicRandom(List(7, 8))
    //~ emits: roll(__handle_
    println("${roll()} ${roll()}")
}

//--- fn_type_effects_thread_into_lambdas
// [fn-effects] [kt-handle] Kotlin threads the effect as a leading closure
// parameter too, rather than capturing it — one mechanism on both backends.
// A named fn declaring exactly the type's effects passes as a reference;
// a pure one gets an adapter that ignores the threaded effect (the
// variance rule). (`FN_EFFECTS_DEMO` in codegen_tests.rs.)
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

// No effect list of its own: `run_it` *inherits* `[Logger]` from `f`, since
// calling `f` is the only reason it takes it.
// The inherited effect is `run_it`'s own leading parameter; the fn *type*
// carries the per-effect parameter too.
//~ emits: runIt(logger: Logger, f: (Logger, String) -> String
fn run_it(f: (s: Str) [Logger] -> Str, value: Str) -> Str =>[f] s => value {
    return f(value)
}

fn demo() [Console, Logger] -> None {
    // The lambda takes the effect rather than capturing it.
    //~ emits: fun(__leff0: Logger, s: String): String {
    println(run_it(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    }, "x"))
    // Named fns: the effectful one passes as a reference, the pure one is
    // adapted to drop the threaded effect.
    //~ emits: runIt(logger, ::shout,
    println(run_it(shout, "one"))
    //~ emits: plain(__a0)
    println(run_it(plain, "two"))
}

fn main() [use] -> None {
    use StdConsole
    use ConsoleLogger()
    demo()
}

//--- a_generic_handler_is_constructed_at_its_type
// [effect-handler-generics] A `use` gives its handler type arguments.
// The same source and stdout as the Rust backend's
// `rustc_compiles_and_runs_a_generic_handler`. Kotlin needed the fix too:
// erasure removes a type argument from the JVM but not from the *source*, so
// `val h: Show<Int> = Plain()` is a kotlinc error ("cannot infer type for
// type parameter 'T'") — which the spec used to claim erasure made
// impossible. (`HANDLER_GENERICS_DEMO` in codegen_tests.rs.)
effect Show<T> {
    fn show(v: T) -> Str => v
}

// Stateless *and* generic: nothing but the `use` site can say what `T` is.
handler Plain<T> of Show<T> {
    fn show(v: T) -> Str => v {
        return "plain"
    }
}

effect Tag<T> {
    fn tagged(v: T) -> Str => v
}

// Generic with state: the constructor argument used to be the only thing
// that could bind `T`, and still works.
handler Prefixed<T>(prefix: Str) of Tag<T> {
    fn tagged(v: T) -> Str => v {
        return "${prefix}!"
    }
}

fn main() [use] {
    use StdConsole()
    //~ emits: : Plain<Int> = Plain<Int>()
    //~ emits: : Show<Int> = __Mon_Show<Int>(
    use Plain<Int>()
    //~ emits: : Prefixed<Int> = Prefixed<Int>(prefix = "p")
    use Prefixed<Int>("p")
    println(show(7))
    println(tagged(7))
}

//--- effect_member_overloads_get_distinct_names
// [effect-member-overload] The interface declares the overloads under
// distinct names (the first keeps the plain one), the handler overrides those
// same names, and each call site emits the one the *checker* resolved.
// [effect-member-overload] Verbatim the Rust backend's `MEMBER_OVERLOADS`:
// one effect declaring `close` per token type, which is phase 4's `Fs`
// (FILE_SYSTEM.md §5.10.2 sub-question A). Kotlin *could* overload here, and
// that is the hazard — it would resolve by Kotlin's type lattice rather than
// Salvo's [kt-fn-mangling] — so the names are made distinct by the same
// `salvo_core` rule the Rust backend uses.
struct InFile { id: Int }
struct OutFile { id: Int }

//~ emits: fun close__InFile(f: InFile): String
//~ emits: fun close__OutFile(f: OutFile): String
//~ emits: fun describe(f: InFile): String
effect Vault {
    fn close(f: InFile) -> Str => !f
    fn close(f: OutFile) -> Str => !f
    fn describe(f: InFile) -> Str => f
}

//~ emits: override fun close__InFile(f: InFile): String {
//~ emits: override fun close__OutFile(f: OutFile): String {
handler Files of Vault {
    fn close(f: InFile) -> Str => !f {
        return "closed in ${f.id}"
    }
    fn close(f: OutFile) -> Str => !f {
        return "closed out ${f.id}"
    }
    fn describe(f: InFile) -> Str => f {
        return "file ${f.id}"
    }
}

fn main() [use] {
    use StdConsole()
    use Files()
    println(describe(InFile { id: 3 }))
    //~ emits: .close__InFile(InFile(
    println(close(InFile { id: 1 }))
    //~ emits: .close__OutFile(OutFile(
    println(close@Vault(OutFile { id: 2 }))
}
