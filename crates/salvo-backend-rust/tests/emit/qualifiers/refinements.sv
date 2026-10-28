// [qual-refn] Qualifier refinements on the Rust backend.

//--- a_refined_program
// A refinement is *compile-time only*: `NonEmpty` is erased like every
// qualifier [qual-erasure], and a refinement adds no call, no check and no
// wrapper — it only decides which overload the checker picks. So the two
// backends run the same source to the same output with no refinement
// machinery anywhere in between.
//
// [qual-refn] [qual-erasure] The same source and the same expected stdout as
// the Kotlin backend asserts — which is the parity claim itself. The emitted
// Rust carries no trace of the refinement.
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

// A refinement is *trusted* like `-> T as Q` [qual-ctor-fn]: no runtime
// check is emitted where it applies.
//~ !emits: NonEmpty_qualifies
//~ !emits: crate::NonEmpty__List_qualifies
fn main() [use] {
    use StdConsole()
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    println("after add: ${count(xs)}")
    refill(xs, 2)
    println("after refill: ${count(xs)}")
}
