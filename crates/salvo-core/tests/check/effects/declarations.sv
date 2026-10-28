// Effect lists, `use` and handler members the checker refuses: a `use`
// without the `use` effect, an effect listed twice or not declared, a call
// two instances of a generic effect could answer, and a handler member
// declaring effects of its own.

//--- use_requires_the_use_effect
// [effect-use-requires-use]
fn setup() -> None {
    //~ error[effect-use-requires-use:missing]
    use StdConsole
}

//--- duplicate_effect_in_list_is_rejected
// [effect-no-dup]
//~ error[effect-no-dup:duplicate]
fn f() [Console, Console] -> None {
}

//--- unknown_effect_is_rejected
// [effect-fn-deps]
//~ error[name-resolve:unknown-effect]
fn f() [Consle] -> None {
}

//--- ambiguous_generic_effect_call_is_rejected
// [effect-disambiguation]
export effect Random<T> {
    fn next_random() -> T
}

export fn f() [Random<Int>, Random<Double>] -> None {
    //~ error[effect-disambiguation:ambiguous]
    let _x = next_random()
}

//--- two_registered_instances_are_still_ambiguous
// [effect-disambiguation] [effect-use-no-dup] Shadowing collapses repeats of *one*
// instance, and must not collapse two *different* instances of a generic
// effect: those are still ambiguous, registered by `use` as much as by an
// effect list.
export effect Random<T> {
    fn next_random() -> T
}

export handler Fixed<T>(value: T) of Random<T> {
    fn next_random() -> T {
        //~ error[effect-state-store:moved-out]
        return value
    }
}

export fn main() [use] -> None {
    use Fixed(1)
    use Fixed("a")
    //~ error[effect-disambiguation:ambiguous]
    let _x = next_random()
}

//--- handler_member_effects_are_rejected
// [effect-member-no-effects]
export effect Ping {
    fn ping()
}

export handler LoudPing of Ping {
    //~ error[effect-member-no-effects:declared]
    fn ping() [Console] {
        println("ping")
    }
}

//--- effect_member_generics_bind_per_call
// [effect-member-generics] The member's own generics bind per call: the
// checker knows `pick(1, 2)` is `Int`, not an unbound `T`.
export effect Stash {
    fn pick<T>(a: T, b: T) -> T => !a, !b
}

export handler FirstStash of Stash {
    //~ error[deduce-default:kept-by-default-moved]
    fn pick<T>(a: T, b: T) -> T {
        return a
    }
}

export fn main() [use] -> None {
    use StdConsole
    use FirstStash
    //~ error[var-let-infer:annotation-mismatch]
    let bad: Str = pick(1, 2)
    println(bad)
}
