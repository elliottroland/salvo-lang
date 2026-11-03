// [call-type-args] [fn-variadic] [effect-member-generics] A member's own type
// parameter bound by a variadic tail is known at the call: `count(1, 2, 3)`
// is `count<Int>`, and its tail an `Int[]` (ROADMAP 0at: the tail lowered at
// the bare `T`, which kotlinc refused).
//
// Compile-and-run data ([test-run-data]).

//--- a_generic_variadic_member
// BACKENDS: kotlin
// (The Rust backend refuses a member with generics of its own: it cannot
// dispatch one dynamically yet.)
effect Bag {
    fn count<T>(...xs: T[]) -> Int => xs
}

handler Counting() of Bag {
    fn count<T>(...xs: T[]) -> Int => xs {
        return size(xs)
    }
}

fn main() [use] {
    use StdConsole()
    use Counting()
    println("${count(1, 2, 3)} ${count("a")}")
}
//= stdout
//= 3 1
