// --- Qualifier refinements [qual-refn] ---

//--- a_refinement_recovers_a_qualifier_stripped_by_a_mutating_call
// The refined program over the *real* std:
// `add`'s exhaustive `[list: Mut]` drops `Filled` [deduce-syntax], and
// `Filled`'s own refinement puts it back, so the `Filled` overload still
// resolves after the mutation.
qualifier Filled<T> for List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }

    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +Filled
}

fn count<T canbe linear>(list: Filled List<T>) -> Int => list {
    return size(list)
}

fn main() [use] {
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    let _n = count(xs)
}

//--- without_the_refinement_std_add_strips_the_claim
// The control: without the refinement, std's `add` strips the claim.
qualifier Filled<T> for List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }
}

fn count<T canbe linear>(list: Filled List<T>) -> Int => list {
    return size(list)
}

fn main() [use] {
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    //~ error[implicit-fit:no-fill]
    let _n = count(xs)
}
