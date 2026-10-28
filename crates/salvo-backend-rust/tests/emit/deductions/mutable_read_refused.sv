// A mutable read ahead of a mutation in one call's arguments.

//--- a_mutable_read_before_a_mutation_is_refused
// [rs-mut-arg-hoist] …and the shape the hoist cannot take: the read is
// **mutable data**, so copying it out of the way would be a snapshot on Rust
// and a live handle on Kotlin. Reported with the two remedies the program
// can choose between, never silently either way [backend-never-wrong].
struct Holder canbe Mut { items: Mut List<Int>, n: Int }

fn grow(h: Mut Holder) -> Int => h: Mut {
    add(h.items, 9)
    return size(h.items)
}

fn pair(xs: List<Int>, n: Int) -> Str => xs, n {
    return "${to_str(xs)}/${n}"
}

fn main() [use] {
    use StdConsole()
    let h = Mut Holder { items: Mut List(1, 2), n: 0 }
    //~ refused[rs-mut-arg-hoist:read-then-mutated]
    println("${pair(h.items, grow(h))}")
}
