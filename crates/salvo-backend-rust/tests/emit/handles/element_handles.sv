// Mutable element handles render as positions into their container
// [ref-handle] [rs-elem-mut] [rs-path] [rs-loc].

//--- elem_mut_handles_render_as_get_mut_and_captured_indices
// [ref-handle] Mutation through element handles of a `List<Mut T>`: a
// statement-scoped one (`bump(at(xs, 0)!)` → an `Option<&mut T>` from `at`), a bound
// one in a callee (a captured-index virtual binding), and reads afterwards
// observing both mutations — the aliasing semantics Kotlin gets natively.
//
// [rs-elem-mut] [rs-path] The three renderings: the container parameter
// arrives `&mut` (element-level `Mut` lends), the statement-scoped handle is
// `at`'s path materialized as `&mut xs[pos]` in place, and the bound handle
// is a captured index re-materialized at every use — never a bound `&mut`.
struct Counter canbe Mut {
    n: Int
}

fn bump(c: Mut Counter) -> None => c: Mut {
    c.n = c.n + 1
}

//~ emits: pub fn poke(xs: &mut Vec<crate::Counter>)
fn poke(xs: List<Mut Counter>) -> None {
    //~ emits: let __h1: usize = crate::std_core_list::at_platform(&*xs, 1i32).expect(
    let h = at(xs, 1) ?: unreachable!()
    //~ emits: xs[__h1].n = i32::wrapping_add(xs[__h1].n, 10i32);
    h.n = h.n + 10
    //~ emits: crate::bump(&mut xs[__h1]);
    bump(h)
}

fn main() [use] {
    use StdConsole()
    let xs: List<Mut Counter> = List(Mut Counter { n: 1 }, Mut Counter { n: 2 })
    // A temporary handle is `at`'s position, read from a shared borrow and
    // only then materialized as the element borrowed `&mut` in place.
    //~ emits: let __h1: Option<usize> = crate::std_core_list::at_platform(&xs, 0i32);
    //~ emits: let __h2 = __h1.expect("salvo: value is absent");
    //~|             &mut xs[__h2]
    bump(at(xs, 0) ?: unreachable!())
    poke(xs)
    let a = get(xs, 0) ?: unreachable!()
    let b = get(xs, 1) ?: unreachable!()
    println("${a.n} ${b.n}")
}

//--- a_search_loop_returns_the_found_position
// [rs-loc] ④a slice 5 — a **search loop** whose element binding is
// returned: in the minting fn's path rendering the `for` over a list becomes
// an *indexed* loop and the binding a captured-index handle, so the found
// **position** travels out [rs-path]. Rust can write this function (NLL accepts an
// `iter_mut` search returning `Option<&mut T>`); what it cannot do is read
// the container while the result is live, which is what the position
// rendering buys — see `examples/borrowing/README.md`.
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

//~ emits: pub fn wounded(es: &Vec<crate::Entity>) -> Option<usize>
// The position fn is the only rendering.
//~ !emits: Option<&mut crate::Entity>
fn wounded(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    //~ emits: for __li1 in 0..es.len()
    for e in es {
        if e.hp < 10 {
            //~ emits: return Some(__li1);
            return e
        }
    }
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 50 }, Mut Entity { hp: 3 })
    // …and the caller materializes it.
    //~ emits: let __h1: Option<usize> = crate::wounded(&es);
    heal(wounded(es) ?: unreachable!())
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}

//--- a_mint_into_a_nested_field_paths_through_the_fields
// [rs-path] A user mint whose container is a **nested field** of structs
// that have no accessor of their own: the path fn takes the outer struct
// by shared borrow and answers the list position, and every use indexes the
// same field path (`a.b.c[pos]`) — a statement-scoped handle and a bound
// one alike.
struct Entity canbe Mut { hp: Int }

struct B canbe Mut {
    c: List<Mut Entity>
}

struct A canbe Mut {
    b: B
}

//~ emits: pub fn at(a: &crate::A, mut index: i32) -> Option<usize>
fn at(a: A, index: Int) -> (ref(a) Mut Entity)? {
    return a.b.c.at(index)
}

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn main() [use] {
    use StdConsole()
    let a = Mut A { b: Mut B { c: List(Mut Entity { hp: 1 }, Mut Entity { hp: 2 }) } }
    //~ emits: &mut a.b.c[__h2]
    heal(at(a, 1) ?: unreachable!())
    //~ emits: let __h3: usize = crate::at(&a, 0i32).expect(
    let h = at(a, 0) ?: unreachable!()
    //~ emits: a.b.c[__h3].hp = i32::wrapping_add(a.b.c[__h3].hp, 100i32);
    h.hp = h.hp + 100
    println("${(get(a.b.c, 0) ?: unreachable!()).hp} ${(get(a.b.c, 1) ?: unreachable!()).hp}")
}

//--- ref_handles_render_as_their_container_and_positions
// [ref-anchor] [rs-loc] ④b — a callee whose two handle parameters are
// `ref(c)` of one container parameter takes two element handles that **may
// be the same element**: the handles render as the container once plus a
// position each (two `&mut` into one container cannot coexist), and the
// same-call rule stands down without any disjointness proof.
struct Entity canbe Mut { hp: Int, energy: Int }

//~ emits: pub fn attack(c: &mut Vec<crate::Entity>, __c1: usize, __c2: usize)
fn attack(c: List<Mut Entity>, a: ref(c) Mut Entity, d: ref(c) Mut Entity) -> None {
    //~ emits: c[__c1].energy = i32::wrapping_sub(c[__c1].energy, 1i32);
    a.energy = a.energy - 1
    d.hp = d.hp - 2
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(
        Mut Entity { hp: 10, energy: 5 },
        Mut Entity { hp: 20, energy: 8 })
    let i = 0
    let j = 1
    // Each handle is minted as a position before the container is lent.
    //~ emits: let __q1 = crate::std_core_list::at_platform(&es, i).expect(
    //~ emits: let __c2 = __q1;
    //~ emits: crate::attack(&mut es, __c2, __c4)
    attack(es, at(es, i) ?: unreachable!(), at(es, j) ?: unreachable!())
    // …and the aliasing case `ref(c)` exists for: one element, both
    // handles.
    attack(es, at(es, i) ?: unreachable!(), at(es, i) ?: unreachable!())
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 0) ?: unreachable!()).energy} ${(get(es, 1) ?: unreachable!()).hp}")
}

//--- a_ref_anchored_handle_indexes_its_container_parameter
// [ref-anchor] [rs-loc] A handle container that is a **field path** at the
// call: the callee's container parameter is the `&mut` the call lends
// (`&mut squad.members`) and the handles add nothing but an index each — no
// second borrow of the container is synthesized, which made every such call
// E0499 before the defect closed 2026-09-25.
struct Fighter canbe Mut { name: Str, hp: Int, energy: Int }

struct Squad canbe Mut {
    banner: Str,
    members: List<Mut Fighter>
}

// One borrow of the container, two positions — and no `__anchor`, since
// the container is a parameter the callee already has.
//~ emits: pub fn trade(members: &mut Vec<crate::Fighter>, __c1: usize, __c2: usize)
//~ !emits: __anchor
fn trade(members: List<Mut Fighter>, one: ref(members) Mut Fighter, other: ref(members) Mut Fighter) -> None {
    //~ emits: members[__c1].energy = i32::wrapping_sub(members[__c1].energy, 1i32);
    one.energy = one.energy - 1
    other.energy = other.energy + 1
    return None
}

fn main() [use] {
    use StdConsole()
    let squad = Mut Squad { banner: "red", members: List(
        Mut Fighter { name: "Ada", hp: 30, energy: 4 },
        Mut Fighter { name: "Bo", hp: 8, energy: 9 }) }
    let i = 0
    let j = 1
    //~ emits: crate::trade(&mut squad.members, __c2, __c4) };
    //~ emits: let __q1 = crate::std_core_list::at_platform(&squad.members, i).expect(
    trade(squad.members, at(squad.members, i) ?: unreachable!(), at(squad.members, j) ?: unreachable!())
    // …and the aliasing case `ref(c)` exists for: one fighter, both roles.
    trade(squad.members, at(squad.members, i) ?: unreachable!(), at(squad.members, i) ?: unreachable!())
    println("${(get(squad.members, 0) ?: unreachable!()).energy} ${(get(squad.members, 1) ?: unreachable!()).energy}")
}
