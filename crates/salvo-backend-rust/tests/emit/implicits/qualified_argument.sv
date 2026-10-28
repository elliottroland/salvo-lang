// A qualified argument still binds an implicit's type variable
// [implicit-infer].

//--- a_qualified_argument_still_binds_an_implicits_type_variable
// [implicit-infer] A **qualified argument** through an implicit fn position:
// `total(List(1, 2, 3))`'s argument is `NonEmpty List<Int>`
// [col-of-nonempty], and matching the candidate `iter` strictly against that
// claim taught the call nothing — so `It` stayed unbound and the `?next`
// beside it resolved by *rung*, picking a sibling pass's `next` and emitting
// a call the target compiler rejected. The unqualified literal always worked,
// which is what pointed at the qualifier.
struct Sib : Next<self, Int> canbe Mut {
    at: Int
}

fn next(p: Mut Sib) -> Emitted Int | Finished => p: Mut {
    if p.at > 2 {
        return finished()
    }
    p.at = p.at + 1
    return emitted(p.at)
}

fn total<C, It>(c: C, ?iter: (c: C) -> Mut It holds proj(c), ?Next<It, Int>) -> Int => c {
    let t = 0
    for x in iter(c) {
        t = t + x
    }
    return t
}

fn main() [use] -> None {
    use StdConsole()
    // Both calls fill `?next` from `std.core.list`, not from the sibling `Sib`.
    //~ emits 2: next__ListNext(
    println("${total(List(1, 2, 3))}")
    println("${total(List(1, 2, 3))}")
}
