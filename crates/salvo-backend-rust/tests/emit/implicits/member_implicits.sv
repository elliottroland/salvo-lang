// [implicit-param] Implicit parameters of effect members on the Rust backend.

//--- an_effect_member_carries_its_implicits_into_the_trait
// [implicit-param] An effect member is an ordinary signature, so it may
// declare implicit parameters (user decision 2026-09-06): the trait method
// takes them, every handler's implementation takes them, and the call site
// fills them. They render as `dyn` here, not `impl` — an effect trait is
// used as `&mut dyn E` [rs-effects], and `impl Trait` in argument position
// would cost object safety.
effect Show {
    // The trait method, object-safe.
    //~ emits: fn show(&mut self, v: i32, fmt: &mut dyn FnMut(i32) -> String) -> String;
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v
}

handler Angle of Show {
    // The implementation has to match it exactly (stateless: `&self`).
    //~ emits: fn show(&self, v: i32, fmt: &mut dyn FnMut(i32) -> String) -> String {
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v {
        return "<${fmt(v)}>"
    }
}

fn fmt(n: Int) -> Str => n {
    return "n=${n}"
}

fn loud(n: Int) -> Str => n {
    return "N=${n}!"
}

fn main() [use] {
    use StdConsole()
    use Angle()
    // And the call site fills it.
    //~ emits: __handle_4.show(7i32, &mut |__a0: i32| crate::fmt(__a0))
    println(show(7))
    println(show(7, fmt = loud))
}
