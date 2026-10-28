// Effects lower to traits; handlers to structs behind handles.

//--- effects_lower_to_traits_and_mut_dyn_params
// [rs-effects] Effects are traits, deps are `&mut dyn` params, `use`
// locals thread as `&mut local`.
//~ emits: pub trait __Stateless_Random<T>: Send + Sync {
//~ emits: fn next_random(&self) -> T;
effect Random<T> {
    fn next_random() -> T
}

//~ emits: impl<T: Clone + Send + Sync + 'static> crate::__Stateful_Random<T> for CyclicRandom<T>
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

// [rs-handle] Effect deps as leading `&E` handle parameters, named for
// the effect (a second instance of one effect numbered).
//~ emits: pub fn draw(random: &crate::Random<i32>, random__1: &crate::Random<String>, console: &crate::std_core_console::Console)
fn draw() [Random<Int>, Random<Str>, Console] {
    // Threading and expected-type disambiguation.
    //~ emits: let mut n: i32 = random.next_random();
    let n: Int = next_random()
    //~ emits: let mut s: String = random__1.next_random();
    let s: Str = next_random()
    println("${s}: ${n}")
}

fn lucky_number() [Random<Int>] -> Int {
    return next_random()
}

fn main() [use] {
    // `use` instantiates handlers into locals holding the handle: a stateless
    // one `shared`, a stateful one `locked` [rs-handle].
    //~ emits: let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    use StdConsole
    // [effect-handler-generics] The handler is constructed *at* a type: the
    // local's annotation says which.
    //~ emits: let mut __use_3: crate::CyclicRandom<i32> = crate::CyclicRandom::new(vec![10i32, 20i32, 30i32], move |__a0: &i32| *__a0);
    //~ emits: let __handle_4 = crate::Random::locked(__use_3);
    use CyclicRandom(List(10, 20, 30))
    use CyclicRandom(List("a", "b"))
    //~ emits: crate::draw(&__handle_4, &__handle_6, &__handle_2);
    draw()
    draw()
    println("lucky: ${next_random<Int>()}")
    println("again: ${lucky_number()}")
}

//--- generic_effect_members_are_rejected
// [rs-effects] [backend-never-wrong]
//~ refused[effect-member-generics:no-dynamic-dispatch]
export effect Weird {
    fn pick<T>(value: T) -> T => !value
}

export handler PassThrough of Weird {
    fn pick<T>(value: T) -> T
    => !value {
        return value
    }
}

export fn main() [use] -> None {
    use PassThrough
}

//--- effect_member_generics_are_rejected_loudly
// [effect-member-generics] [rs-effects] `dyn` traits cannot have generic
// methods: the rust backend rejects them loudly instead of emitting
// invalid code.
//~ refused[effect-member-generics:no-dynamic-dispatch]
export effect Stash {
    fn pick<T>(a: T, b: T) -> T => !a, !b
}

export handler FirstStash of Stash {
    fn pick<T>(a: T, b: T) -> T
    => !a, !b {
        return a
    }
}

export fn main() [use] -> None {
    use StdConsole
    use FirstStash
    println("${pick(7, 2)}")
}

//--- aliased_effect_types_resolve_to_the_same_handler
// [effect-disambiguation] [rs-effects] The emitter's effect environment is
// keyed by the checker's lowered effect types (`Checked::fn_effects` /
// `use_effects` / `call_effects`), not by type renderings: an effect
// written through a type alias resolves to the same instance a `use`
// registered under the canonical type.
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
    use CyclicRandom(List(7, 8))
    //~ emits: crate::roll(&__handle_4), crate::roll(&__handle_4)
    println("${roll()} ${roll()}")
}
