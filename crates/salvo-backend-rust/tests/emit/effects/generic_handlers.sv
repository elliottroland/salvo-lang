// [effect-handler-generics] A `use` gives its handler type arguments.

//--- a_generic_handler_is_constructed_at_its_type
// A stateless generic handler: `use Plain<Int>()` is the *only* thing that
// can say what `T` is, since there is no constructor argument to infer it
// from. The written arguments used to be discarded by the checker and both
// emitters, so this program type-checked and then failed in rustc
// (`E0283`, plus `E0392` for the handler struct) — a [backend-never-wrong]
// violation, fixed 2026-09-06.
effect Show<T> {
    fn show(v: T) -> Str => v
}

// Stateless *and* generic: nothing but the `use` site can say what `T` is.
// A type parameter no field mentions needs `PhantomData`, or the
// struct itself does not compile (`E0392`).
//~ emits: __phantom_T: std::marker::PhantomData<T>,
//~ emits: __phantom_T: std::marker::PhantomData
//~| 
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
    // The instance's annotation, from the written type argument alone...
    //~ emits: let mut __use_3: crate::Plain<i32> = crate::Plain::new();
    use Plain<Int>()
    // ...and where a constructor argument could also have bound it.
    //~ emits: let mut __use_5: crate::Prefixed<i32> = crate::Prefixed::new(String::from("p"));
    use Prefixed<Int>("p")
    println(show(7))
    println(tagged(7))
}
