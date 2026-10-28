// qualifier refinements [qual-refn]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [qual-refn] [qual-erasure] The same source and the same expected stdout as
// the Kotlin backend asserts — which is the parity claim itself. The emitted
// Rust carries no trace of the refinement.
//--- a_refined_program

qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return size(list) > 0
    }

    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}

// Only callable while the compiler still believes the list is non-empty.
fn count<T canbe linear>(list: NonEmpty List<T>) -> Int => list {
    return size(list)
}

fn refill(list: Mut NonEmpty List<Int>, value: Int) -> None => list: Mut NonEmpty {
    add(list, value)
}

fn main() [use] {
    use StdConsole()
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    println("after add: ${count(xs)}")
    refill(xs, 2)
    println("after refill: ${count(xs)}")
}
//= stdout
//= after add: 1
//= after refill: 2
