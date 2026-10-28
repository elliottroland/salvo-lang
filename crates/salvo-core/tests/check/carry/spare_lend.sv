// [proj-infer-fn-type] A fn-typed slot whose return is a bare type parameter
// lends its named kept positions by inference (2026-09-26), so writing the
// clause there says nothing — and an annotation that changes nothing reads as
// evidence that something needs saying, the same reason a no-op `@place`
// selector warns [fn-overload-at]. A **warning**, because the program is
// correct and the clause is merely spare; and silent where the return is
// concrete, since a named type says for itself whether it borrows.

//--- a_spare_lend_on_an_opaque_slot_warns_once
// Written where it is inferred: one diagnostic, naming the clause.
//~ warning[proj-infer-fn-type:redundant-holds]
fn total<C, It, T>(c: C, ?iter: (c: C) -> Mut It holds proj(c), ?Next<It, T>) -> Int
=> c {
    let n = 0
    for _ in iter(c) {
        n = n + 1
    }
    return n
}

fn main() [use] -> None {
    use StdConsole()
    println("${total(List("a", "b"))}")
}

//--- without_the_spare_lend_there_is_nothing_to_say
// Dropping it is what the warning asks for, and then there is nothing to say.
fn total<C, It, T>(c: C, ?iter: (c: C) -> Mut It, ?Next<It, T>) -> Int
=> c {
    let n = 0
    for _ in iter(c) {
        n = n + 1
    }
    return n
}

fn main() [use] -> None {
    use StdConsole()
    println("${total(List("a", "b"))}")
}

//--- a_concrete_return_needs_its_lend
// A **concrete** return still needs the clause, so it stays silent.
fn walk<T>(xs: List<T>, ?iter: (c: List<T>) -> Mut ListNext<T> holds proj(c)) -> Int
=> xs {
    let n = 0
    for _ in iter(xs) {
        n = n + 1
    }
    return n
}

fn main() [use] -> None {
    use StdConsole()
    println("${walk(List(1, 2, 3))}")
}
