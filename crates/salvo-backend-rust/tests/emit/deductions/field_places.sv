// Field-precise borrows and moves, and optional borrows, on the Rust backend.

//--- field_disjoint_access
// [fate-field-disjoint] [rs-borrow-locals] L5's payoff on this backend, and
// the reason it needs no new emission: a borrow-mode binding from `p.name`
// already emits a real borrow (`let mut n = &p.name;`), and rustc allows it
// to live across a `&mut` of a *disjoint field* of the same local. So
// Salvo's field precision and rustc's coincide — these programs were
// checker-rejected before L5 (the remedy was `copy`, a real clone), and they
// compile clone-free now.
//
// Shared with the Kotlin backend, byte for byte.
struct Person canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

struct Pair {
    left: Mut List<Str>,
    right: Mut List<Str>
}

fn touch(list: Mut List<Str>) -> None => list: Mut {
    add(list, "t")
    return None
}

fn main() [use] {
    use StdConsole()
    // read one field, mutate another
    let p = Person { name: "ann", tags: Mut List("x") }
    // The borrow is real and lives across the disjoint mutation: that is the
    // shape rustc has to accept for L5 to be clone-free here.
    //~ emits: let mut n = &p.name;
    let n = p.name
    //~ emits: add_platform::<String>(&mut p.tags, 
    add(p.tags, "y")
    println("A ${n} ${size(p.tags)}")

    // two disjoint mutable fields
    let q = Pair { left: Mut List("l"), right: Mut List("r") }
    let l = q.left
    add(q.right, "r2")
    println("C ${size(l)} ${size(q.right)}")

    // read a field, hand a disjoint field to a mutating fn
    let p2 = Person { name: "dee", tags: Mut List("x") }
    let n2 = p2.name
    touch(p2.tags)
    println("D ${n2} ${size(p2.tags)}")

    // assignment to a disjoint field
    let p3 = Mut Person { name: "eve", tags: Mut List("x") }
    let n3 = p3.name
    p3.tags = Mut List("q", "r")
    println("E ${n3} ${size(p3.tags)}")
}

//--- a_partial_move
// [fate-partial-move] L5's move half end to end: a field handed to a
// consuming fn is a **real partial move** in the emitted Rust
// (`crate::eat(p.tags);` then a read of `p.name`), and reassigning the moved field
// is rustc's reinitialization — both accepted by borrowck. These programs
// were checker-rejected before the move half landed.
//
// Shared with the Kotlin backend, byte for byte.
struct Person canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

fn eat(list: Mut List<Str>) -> None => !list {
    add(list, "eaten")
    return None
}

fn main() [use] {
    use StdConsole()
    // hand one field away, keep reading the other
    let p = Person { name: "ann", tags: Mut List("x") }
    // The field moves out as a raw place (no clone), and the sibling is
    // still read afterwards: a genuine partial move.
    //~ emits: crate::eat(p.tags);
    //~|     crate::std_core_console::println(&__handle_2, &format!("1 {}", p.name));
    eat(p.tags)
    println("1 ${p.name}")

    // the same through a move-mode binding
    let q = Person { name: "bob", tags: Mut List("y") }
    //~ emits: let mut t: Vec<String> = q.tags;
    let t = q.tags
    add(t, "z")
    println("2 ${q.name} ${size(t)}")

    // put the field back, and the whole value works again
    let u = Mut Person { name: "eve", tags: Mut List("t") }
    eat(u.tags)
    u.tags = Mut List("new", "pair")
    println("3 ${u.name} ${size(u.tags)}")
}

//--- an_optional_borrow_unwrap
// [rs-opt-borrow] A local bound from a derived-return call with an optional
// result holds `Option<&T>`. Narrowing it and using the value where an owned
// `T` is expected used to emit `.as_ref().unwrap().clone()` — a clone of the
// *reference* (`&&T` → `&T`), so rustc rejected checker-clean code. The
// unwrap now yields the reference and clones through it.
struct Person { name: Str }

fn take(p: Person) -> Str => !p { return p.name }

fn main() [use] {
    use StdConsole()
    // Annotated to a plain list: the element constructor claims `NonEmpty`
    // [col-of-nonempty], and on a non-empty list `first` answers an element
    // rather than the optional this case is about.
    let people: List<Person> = List(Person { name: "ann" }, Person { name: "bob" })
    let head = first(people)
    if head is None {
        return
    }
    //~ emits: let mut head_3 = head.unwrap();
    //~|     crate::std_core_console::println(&__handle_2, &format!("{}", crate::take((head_3).clone())));
    //~ !emits: head.as_ref().unwrap()
    println("${take(copy(head))}")
    println("${head.name}")
}
