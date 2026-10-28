// [rs-narrow-mut] Mutating through a narrowed place on the Rust backend: every
// mutable use borrows into the storage, never into a clone of it.

//--- a_mutable_use_of_a_narrowed_place_borrows_the_storage
// [rs-narrow-mut] Mutating through a **narrowed** place. Every shape the
// unwrap can take is here, because the read form of each was silently wrong:
// `&mut` of `p.as_ref().unwrap().clone()` compiles and mutates the clone, so
// a pass driven through a narrowed handle re-emitted its first element for
// ever while Kotlin — whose smart cast *is* the storage — advanced.
//
// - a narrowed optional passed to a `Mut` parameter (`Option::as_mut`);
// - a narrowed union arm, same thing through `u1_mut()`;
// - an assignment whose *base* is narrowed (`r.at = 2`), which reached for a
//   field of the `Option` and was an E0609 rather than a silent wrong answer;
// - a narrowed `state` slot of an `iter fn`, which is the shape that found it:
//   a lazy flatten holding the inner pass. That one also needed the
//   desugaring to stop giving the synthesized `__p` base the read's span.
//
// [rs-narrow-mut] The same thing read off the generated source, so the
// regression is caught with no toolchain on PATH: a mutable use borrows into
// the storage, and the tell-tale `&mut` of a clone appears nowhere.
struct Flat {
    rows: List<List<Int>>
}

fn show(step: Emitted Int | Finished) [Console] {
    when step {
        is ^Emitted { println("got ${step}") }
        is Finished { println("end") }
    }
}

iter fn iter(f: Flat) -> Emitted Int | Finished {
    state FlatIter { at: Int = 0, inner: Mut ListNext<Int>? = None }
    while true {
        // The `state` slot inside the generated pass, narrowed into a `&mut`
        // binding of the storage.
        //~ emits: let mut __narrowed_1 = __p.inner.as_mut().unwrap();
        if inner is Mut ListNext<Int> {
            let step = next(inner)
            when step {
                is ^Emitted { return emitted(step) }
                is Finished { inner = None }
            }
        }
        let row = get(f.rows, at)
        if row is None {
            return finished()
        }
        at = at + 1
        inner = iter(row)
    }
    return finished()
}

fn main() [use] {
    use StdConsole()
    let base = List(1, 2)
    let p: Mut ListNext<Int>? = iter(base)
    // The optional, narrowed into a `&mut` binding of the storage.
    //~ emits: let mut p_3 = p.as_mut().unwrap();
    //~ !emits: &mut (p.as_ref
    if p is Mut ListNext<Int> {
        //~ emits: next__ListNext::<i32>(&mut *p_3)
        show(next(p))
        show(next(p))
        show(next(p))
    }
    let qs = List(7, 8)
    let q: Mut ListNext<Int> | Int = iter(qs)
    // The union arm.
    //~ emits: let mut q_4 = match &mut q { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    if q is Mut ListNext<Int> {
        show(next(q))
        show(next(q))
    }
    let rs = List(1, 2, 3)
    let r: Mut ListNext<Int>? = iter(rs)
    if r is Mut ListNext<Int> {
        // The assignment base.
        //~ emits: let mut r_5 = r.as_mut().unwrap();
        //~|         r_5.at = 2i32;
        r.at = 2
        show(next(r))
    }
    // Literals, not constructor calls: the element constructors claim
    // `NonEmpty` [col-of-nonempty], which a plain `List<List<Int>>` field
    // refuses (type arguments are invariant).
    let all = Flat { rows: List<List<Int>>(List(1, 2), List(3, 4, 5)) }
    for n in iter(all) {
        println("n ${n}")
    }
}

//--- a_mut_payload_is_peeled_by_borrowing_the_storage
// [rs-narrow-mut] The other two halves of the same rule, found 2026-09-20
// while designing `?.` and silently wrong until then: the **intrinsic**
// argument path and the `^` **branch shadow** [rs-widen-shadow].
//
// The 2026-09-10 fix covered declared calls (`ParamMode::RefMut` →
// `borrowed_mut_arg`) and assignment bases. It missed:
//
// - an **intrinsic** whose parameter is `Mut` — `intrinsic_arg_code` rendered
//   every place with `emit_place`, so `add(a, 9)` on a narrowed
//   `Mut List<Int>?` emitted `a.as_ref().unwrap().clone().push(9)`: compiles,
//   warns about nothing, drops the element. Every read cloned afresh, so the
//   mutation was gone on the next line;
// - the `^` branch shadow, which bound an owned clone, so a mutation *held*
//   inside the branch and vanished when it ended.
//
// Both disagreed with Kotlin, which casts the storage and mutates the real
// value — the repro and the reasoning are in COMPLETED.md. The shapes here are
// the ones that were wrong, each with a different storage: a variable and a
// struct field over the nullable representation, a wrapper arm through `^` and
// through `when`/`is`, a wrapper behind an `Option`, a **moved** union
// parameter (whose binder needs `mut`), and a handler **state field**
// (`self.held`).
//
// [rs-narrow-mut] The same regression caught off the generated source, with no
// toolchain on PATH. The negative assertions are the sharp ones: the two
// clone-then-mutate shapes that were the defect must appear nowhere.
struct Shelf canbe Mut {
    items: Mut List<Int>? = None
}

effect Bag {
    fn stash(n: Int) -> None
    fn dump() -> Str
}

handler Holder of Bag {
    state {
        held: Ok Mut List<Int> | Err Str = ok(Mut List(0))
    }

    fn stash(n: Int) -> None {
        // A handler state field peels through `self`.
        //~ emits: let mut held_1 = match &mut self.held { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        if held is ^Ok {
            add(held, n)
        }
    }

    fn dump() -> Str {
        if held is ^Ok {
            return to_str(held)
        }
        return "err"
    }
}

// A moved union parameter binds `mut`, or the peel cannot borrow it.
//~ emits: pub fn eat(mut o: 
fn eat(o: Ok Mut List<Int> | Err Str) [] -> Str => !o {
    if o is ^Ok {
        add(o, 7)
        return to_str(o)
    }
    return "err"
}

fn main() [use] {
    use StdConsole()
    use Holder()

    let a: Mut List<Int>? = Mut List(1)
    // A variable and a field over the nullable repr, each narrowed into
    // a `&mut` binding of the storage.
    //~ emits: let mut a_5 = a.as_mut().unwrap();
    //~|         crate::std_core_list::add_platform::<i32>(&mut *a_5, 9i32);
    //~ !emits: .as_ref().unwrap().clone().push(
    if a is Mut { add(a, 9) }
    if a is Mut { println("var ${to_str(a)}") }

    let b = Mut Shelf { items: Mut List(1) }
    //~ emits: let mut __narrowed_7 = b.items.as_mut().unwrap();
    if b.items is Mut { add(b.items, 9) }
    if b.items is Mut { println("field ${to_str(b.items)}") }

    let c: Ok Mut List<Int> | Err Str = ok(Mut List(1))
    // The `^` shadow: a borrow into the storage.
    //~ emits: let mut c_9 = match &mut c { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    //~ !emits: .u1().clone().push(
    //~ !emits: let mut c = c.u1().clone();
    if c is ^Ok { add(c, 9) }
    if c is ^Ok { println("widen ${to_str(c)}") }

    let d: Ok Mut List<Int> | Err Str = ok(Mut List(1))
    // A wrapper arm, narrowed by `when`/`is`.
    //~ emits: let mut d_11 = match &mut d { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    when d { is Ok { add(d, 9) } is Err { } }
    if d is ^Ok { println("when ${to_str(d)}") }

    let e: Ok Mut List<Int> | Err Str | None = ok(Mut List(1))
    //~ emits: let mut e_14 = match &mut e { Some(crate::unions::Union2::U1(__v)) => __v, _ => unreachable!() };
    if e is ^Ok { add(e, 9) }
    if e is ^Ok { println("nullable ${to_str(e)}") }

    println("moved ${eat(ok(Mut List(1)))}")
    stash(5)
    println("state ${dump()}")
}

//--- peeling_a_mut_arm_out_of_a_borrowed_parameter
// [rs-narrow-mut] [rs-opt-borrow] A union-typed parameter whose arm is `Mut`
// is lent mutably, arm by arm (`Union2<&mut Vec<i32>, &String>`), so peeling
// the `Mut` arm out of it and mutating reaches the caller's value — as on
// Kotlin. (Until 2026-10-10 the frame received it read-only, and this was a
// reported error.)
//~ emits: o: crate::unions::Union2<&mut Vec<i32>, &String>
fn bump(o: Ok Mut List<Int> | Err Str) [] -> None {
    if o is ^Ok {
        add(o, 7)
    }
}

fn main() [use] {
    use StdConsole()
    let r: Ok Mut List<Int> | Err Str = ok(Mut List(1))
    bump(r)
    bump(r)
    if r is ^Ok {
        println("${size(r)}")
    }
}
