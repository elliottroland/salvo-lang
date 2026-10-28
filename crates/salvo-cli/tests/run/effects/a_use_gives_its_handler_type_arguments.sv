// [effect-handler-generics] a `use` gives its handler type arguments
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_generic_handler

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
    use Plain<Int>()
    use Prefixed<Int>("p")
    println(show(7))
    println(tagged(7))
}
//= stdout
//= plain
//= p!
