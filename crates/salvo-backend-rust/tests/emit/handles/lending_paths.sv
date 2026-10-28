// Handles on Rust: distinct-pair splits, locator-specialized lending (path
// fns, locator closures, lending effect members), the total `at`, handles
// into generic containers, and `NotSame` through custom accessors.

//--- a_distinct_pair_call_splits_the_container_once
// [fate-elem-distinct] [ref-notsame] [rs-elem-mut] The distinct-pair shapes: a
// `NotSame`-proven pair of bound handles in one call, interleaved mutation
// through both, and the pair in a call again — the checker spares the
// sibling, the emitter splits the container once per pair call.
//
// [rs-elem-mut] A proven pair call renders as one `salvo_pair_mut`
// preamble (a `split_at_mut`, no aliasing) whose two `&mut` halves are the
// call's arguments, guarded by the two positions differing
// [rs-runtime-source].
struct Entity canbe Mut {
    hp: Int,
    energy: Int
}

fn attack(a: Mut Entity, d: Mut Entity) -> None => a: Mut, d: Mut {
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
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    //~ emits: crate::std_core_ref::NotSame_qualifies(&es[__h2], &es[__h1])
    if d is NotSame(a) {
        // One preamble per pair call.
        //~ emits 2: salvo_pair_mut(&mut es[..],
        //~ emits: if __h1 != __h2 { let (__u3, __w4) = crate::seq::salvo_pair_mut(&mut es[..], __h1, __h2)
        attack(a, d)
        a.hp = a.hp + 1
        d.hp = d.hp + 1
        //~ emits: crate::attack(__pm5, __pm6)
        attack(a, d)
    }
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 0) ?: unreachable!()).energy} ${(get(es, 1) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).energy}")
}

//--- a_mut_used_lender_renders_as_its_path_fn
// [rs-loc] [rs-path] Locator-specialized lending (④a slice 1, re-founding
// ③'s mechanism): a user-written accessor that mints a mutable handle
// renders only as its path fn, under its own name, and the C family is
// ordinary std Salvo riding the same mechanism.
//
// [rs-path] The minting fn has one emission — the path fn answering
// position data from a *read-mode* search, under the plain name, with no
// `&mut`-returning face beside it — and every call site takes it: the
// mutable use materializes `&mut es[pos]` in a block whose read borrow
// ends before the write borrow begins, the read use indexes the container.
struct Entity canbe Mut { hp: Int }

// A minting fn keeps no `&mut` face.
//~ emits: pub fn front(es: &Vec<crate::Entity>) -> Option<usize>
//~ !emits: Option<&mut crate::Entity>
//~ !emits: __loc
fn front(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    return at(es, 0)
}

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    // The mutable use materializes the position where it stands.
    //~ emits: crate::heal({
    //~|         let __h1: Option<usize> = crate::front(&es);
    //~ emits: &mut es[__h2]
    heal(front(es) ?: unreachable!())
    // A handle read later is a position: the read borrow ends at once.
    //~ emits: let __h3: usize = crate::front(&es).expect(
    //~ emits: es[__h3].hp
    let hp = (front(es) ?: unreachable!()).hp
    println("${hp} ${(get(es, 1) ?: unreachable!()).hp}")
}

//--- a_bound_accessor_handle_is_a_captured_locator
// [rs-loc] ④a slice 2 — the bound-mint lift: a handle from a user
// accessor binds (①'s "direct `get` only" cut widened), re-materializes
// per use, and tolerates container *reads* between uses — the exact
// shape a bound `&mut` could never survive (E0502).
struct Entity canbe Mut { hp: Int }

fn front(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    return at(es, 0)
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    //~ emits: let __h1: usize = crate::front(&es).expect(
    let boss = front(es) ?: unreachable!()
    //~ emits: es[__h1].hp = i32::wrapping_add(es[__h1].hp
    boss.hp = boss.hp + 100
    let n = size(es)
    boss.hp = boss.hp + n
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}

//--- a_lending_fn_value_renders_as_a_locator_closure
// [rs-loc] ④a slice 3 — a **lending fn value** serves a `Mut`
// position: the closure answers a locator (position data crosses the
// boundary where a `&mut` could not), and the use site materializes. The
// `?at` idiom — a generic accessor supplied by the caller, the house
// pattern that pierces opacity — is the acceptance test.
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

//~ emits: -> Option<usize>
fn bump_at(es: List<Mut Entity>, i: Int,
           at: (c: List<Mut Entity>, k: Int) -> ref(c) Mut Entity?) -> None {
    //~ emits: let __h1: Option<usize> = at(&*es, i);
    //~ emits: &mut es[__h2]
    heal(at(es, i) ?: unreachable!())
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    //~ emits: &mut |mut c, mut k| -> Option<usize> {
    //~|         crate::std_core_list::at_platform(c, k)
    //~|
    bump_at(es, 1, (c: List<Mut Entity>, k: Int) -> at(c, k))
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}

//--- a_locate_implicit_renders_as_a_locator_closure
// [col-locate] [rs-loc] ④a slice 3′ — std's `Ref` bundle: a generic
// algorithm hands out mutable handles because the caller supplies `at`,
// and the implicit renders as a **locator** closure (position data out,
// read-mode parameters) with the handle materialized at the use site.
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

//~ emits: at: &mut dyn FnMut(&Vec<crate::Entity>, &L) -> Option<usize>
fn heal_at<L>(c: List<Mut Entity>, l: L, ?Ref<List<Mut Entity>, L, Entity>) -> None {
    heal(at(c, l) ?: unreachable!())
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    //~ emits: crate::std_core_list::at_platform(__a0, *__a1)
    heal_at(es, 1)
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}

//--- a_lending_effect_member_lends_through_the_trait
// [rs-path] An **effect member** that mints a handle: the trait carries its
// path face only, under the member's own name, every handler and adapter
// implements it, and a use routes to it.
//
// [rs-path] A lending effect member renders as its path face only, under its
// own name: the trait answers where the element is, and the caller walks it.
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

effect Lender {
    //~ emits: fn lease(&mut self, es: &Vec<crate::Entity>) -> Option<usize>;
    //~ !emits: lease_loc
    fn lease(es: List<Mut Entity>) -> (ref(es) Mut Entity)? => es
}

handler FirstLender of Lender {
    fn lease(es: List<Mut Entity>) -> (ref(es) Mut Entity)? => es {
        return at(es, 0)
    }
}

fn run(es: List<Mut Entity>) [Lender] -> None {
    heal(lease(es) ?: unreachable!())
    return None
}

fn main() [use] {
    use StdConsole()
    use FirstLender()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    run(es)
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}

//--- a_distinct_pair_call_in_a_value_position_splits_in_place
// [rs-elem-mut] A proven pair call in a *value* position: the split is a
// block expression around the call, so the position needs no statement of
// its own.
struct Entity canbe Mut { hp: Int }

fn poke(a: Mut Entity, d: Mut Entity) -> Int => a: Mut, d: Mut {
    a.hp = a.hp + 1
    d.hp = d.hp + 1
    return a.hp
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 1 }, Mut Entity { hp: 2 })
    let i = 0
    let j = 1
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    if d is NotSame(a) {
        //~ emits: let mut x: i32 = { let (__pm5, __pm6) = if __h1 != __h2 { 
        //~ emits: crate::seq::salvo_pair_mut(&mut es[..], __h1, __h2)
        //~ emits: crate::poke(__pm5, __pm6) };
        let x = poke(a, d)
        println("${x}")
    }
}

//--- a_lending_member_through_a_struct_field
// [rs-path] [ref-handle] A lending effect member is its path face only: a
// handler delegating to a mint answers a path through a struct field, two
// handles from it stay live across a write to the container's other field,
// and one is passed on as a `Mut` argument.
struct Entity canbe Mut { hp: Int }
struct Team canbe Mut { name: Str, members: List<Mut Entity> }

fn pick(t: Team, i: Int) -> (ref(t) Mut Entity)? => t {
    return at(t.members, i)
}

effect Roster {
    //~ emits: fn member(&self, t: &crate::Team, i: i32) -> Option<usize>;
    fn member(t: Team, i: Int) -> (ref(t) Mut Entity)? => t
}

handler Plain of Roster {
    fn member(t: Team, i: Int) -> (ref(t) Mut Entity)? => t {
        return pick(t, i)
    }
}

fn heal(e: Mut Entity, by: Int) -> None => e: Mut {
    e.hp = e.hp + by
    return None
}

fn run(t: Mut Team) [Roster] -> None => t: Mut {
    let a = member(t, 0) ?: unreachable!()
    let b = member(t, 1) ?: unreachable!()
    t.name = "renamed"
    if b is NotSame(a) {
        a.hp = a.hp + b.hp
        heal(b, 100)
    }
    heal(member(t, 0) ?: unreachable!(), 1000)
    return None
}

fn main() [use] {
    use StdConsole()
    use Plain()
    let t = Mut Team { name: "t", members: List(Mut Entity { hp: 1 }, Mut Entity { hp: 2 }) }
    run(t)
    println("${t.name} ${(get(t.members, 0) ?: unreachable!()).hp} ${(get(t.members, 1) ?: unreachable!()).hp}")
}

//--- handles_into_generic_containers
// [rs-path] Handles into a container that is a type parameter: the
// handle's path is the fn's own type parameter `P: Walk<C, E>`, walked at
// each use — `ref(c)` parameters, a pair proven apart and split by
// `walk_pair`, a handle minted through `?Ref`, and a delegating mint.
struct Player canbe Mut { name: Str, goals: Int }
struct Team canbe Mut { coach: Str, roster: List<Mut Player> }
struct League canbe Mut {
    teams: Map<Str, Mut Team>,
    bench: List<Mut Player>
}

//~ emits: impl crate::seq::Walk<crate::League, crate::Player> for crate::__Path_League__Player
fn at(l: League, team: Str, n: Int) [] -> ref(l) Mut Player? => l, team, n {
    if n < 0 {
        return at(l.bench, -n - 1)
    }
    let t = at(l.teams, team)
    if t is None {
        return None
    }
    return at(t.roster, n)
}

// Generic over the container: the handle's path is opaque here.
//~ emits: pub fn bump<C: Clone, __P0: crate::seq::Walk<C, crate::Player> + Copy + PartialEq>(c: &mut C, __c1: __P0)
fn bump<C>(c: C, a: ref(c) Mut Player) -> None {
    //~ emits: (*__c1.walk(&mut *c)).goals
    a.goals = a.goals + 1
}

fn swap_goals<C>(c: C, a: ref(c) Mut Player, b: ref(c) Mut Player) -> None {
    let g = a.goals
    a.goals = b.goals
    b.goals = g
}

// A delegating mint: the accessor comes in through `?Ref`.
fn borrow_via<C, L>(c: C, l: L, ?Ref<C, L, Player>) [] -> ref(c) Mut Player? => c, l {
    return at(c, l)
}

fn bump_at<C, L>(c: C, l: L, ?Ref<C, L, Player>) -> None {
    let h = at(c, l) ?: unreachable!()
    h.goals = h.goals + 10
    h.goals = h.goals + 1
}

fn score(a: Mut Player, b: Mut Player) -> None => a: Mut, b: Mut {
    a.goals = a.goals + 1000
    b.goals = b.goals + 2000
}

// Two handles into a generic container, proven apart: one split, walked.
fn score_both<C>(c: C, a: ref(c) Mut Player, b: ref(c) Mut Player) -> None {
    if b is NotSame(a) {
        //~ emits: .walk_pair(&
        score(a, b)
    }
}

export fn main() [use] -> None {
    use StdConsole()
    let teams: Mut Map<Str, Mut Team> = Mut Map()
    put(teams, "red", Mut Team { coach: "Ada", roster: List(Mut Player { name: "Bo", goals: 0 }, Mut Player { name: "Cy", goals: 5 }) })
    let league = Mut League { teams: teams, bench: List(Mut Player { name: "Flo", goals: 9 }) }
    let squad: List<Mut Player> = List(Mut Player { name: "Ed", goals: 2 }, Mut Player { name: "Di", goals: 3 })

    bump(league, at(league, "red", 1) ?: unreachable!())
    bump(squad, at(squad, 0) ?: unreachable!())
    swap_goals(league, at(league, "red", 0) ?: unreachable!(), at(league, "", -1) ?: unreachable!())
    swap_goals(squad, at(squad, 0) ?: unreachable!(), at(squad, 1) ?: unreachable!())
    let h = borrow_via(squad, 1) ?: unreachable!()
    h.goals = h.goals + 100
    bump_at(squad, 0)
    score_both(league, at(league, "red", 0) ?: unreachable!(), at(league, "", -1) ?: unreachable!())
    score_both(squad, at(squad, 0) ?: unreachable!(), at(squad, 0) ?: unreachable!())
    let red = get(league.teams, "red") ?: unreachable!()
    println("${(get(red.roster, 0) ?: unreachable!()).goals} ${(get(red.roster, 1) ?: unreachable!()).goals} ${(get(league.bench, 0) ?: unreachable!()).goals}")
    println("${(get(squad, 0) ?: unreachable!()).goals} ${(get(squad, 1) ?: unreachable!()).goals}")
}

//--- the_total_at
// [col-idx] [ref-handle] The total `at`: at an `Idx` the mint answers the
// handle itself, so a bound handle is a bare position, `update` materializes
// it with no `Option` match, and two bound total handles proven apart by
// `NotSame` [ref-notsame] split through their two paths.
struct E canbe Mut { hp: Int }

fn bump(e: Mut E) -> None => e: Mut {
    e.hp = e.hp + 1
}

fn hit(c: List<Mut E>, a: ref(c) Mut E, b: ref(c) Mut E) -> None {
    a.hp = a.hp - 1
    b.hp = b.hp - 2
}

fn merge(p: Mut E, q: Mut E) -> None => p: Mut, q: Mut {
    p.hp = p.hp + q.hp
}

export fn main() [use] -> None {
    use StdConsole()
    let xs: List<Mut E> = List(Mut E { hp: 10 }, Mut E { hp: 20 })
    let i = 0
    let j = 1
    if i is Idx(xs) {
        if j is Idx(xs) {
            //~ emits: let __h1: usize = crate::std_core_list::at(&xs, i);
            let a = at(xs, i)
            a.hp = a.hp + 5
            //~ emits: crate::bump({ let __l2 = crate::std_core_list::at(&xs, j); &mut xs[__l2] });
            bump(at(xs, j))
            hit(xs, at(xs, i), at(xs, i))
            // `update` materializes the total handle with no `Option` match.
            //~ emits(std/core/list.rs): f({ let __l1 = crate::std_core_list::at(&*list, index); &mut list[__l1] });
            update(xs, i, (e: Mut E) -> { e.hp = e.hp * 2 })
            let p = at(xs, i)
            //~ emits: let __h8: usize = crate::std_core_list::at(&xs, j);
            let q = at(xs, j)
            if q is NotSame(p) {
                //~ emits: crate::seq::salvo_pair_mut(&mut xs[..], __h7, __h8)
                //~ emits: crate::merge(__pm11, __pm12)
                merge(p, q)
            }
        }
    }
    println("${(get(xs, i) ?: unreachable!()).hp} ${(get(xs, j) ?: unreachable!()).hp}")
}
//--- notsame_handles
// [ref-notsame] [fate-elem-distinct] [rs-path] Two handles proven apart by
// `NotSame`, through a custom container's own accessor: the split lands
// where the handles' paths diverge — two map slots (`pair_mut` on the map),
// two positions of one slot's roster, a roster and the bench (different
// fields) — and through nested structs with no accessor of their own
// (`a.b.c`). Two handles to one element make the test false at run time
// (`std::ptr::eq`). Source and expected stdout are verbatim the Kotlin
// backend's `kotlinc_compiles_and_runs_notsame_handles`.
// A player lives on a team's roster (a map slot's list) or on the bench.
struct Player canbe Mut { name: Str, goals: Int }
struct Team canbe Mut { coach: Str, roster: List<Mut Player> }
struct League canbe Mut {
    teams: Map<Str, Mut Team>,
    bench: List<Mut Player>
}

// A custom container's own accessor: the handle's path runs through a map
// slot, or through a field.
fn at(l: League, team: Str, n: Int) [] -> ref(l) Mut Player? => l, team, n {
    if n < 0 {
        return at(l.bench, -n - 1)
    }
    let t = at(l.teams, team)
    if t is None {
        return None
    }
    return at(t.roster, n)
}

fn trade(l: League, a: ref(l) Mut Player, b: ref(l) Mut Player) -> None {
    let g = copy(a.goals)
    a.goals = b.goals
    b.goals = g
}

fn score(a: Mut Player, b: Mut Player) -> None => a: Mut, b: Mut {
    a.goals = a.goals + 1
    b.goals = b.goals + 1
}

fn league() [] -> Mut League {
    let teams: Mut Map<Str, Mut Team> = Mut Map()
    put(teams, "red", Mut Team { coach: "Ada", roster: List(Mut Player { name: "Bo", goals: 0 }, Mut Player { name: "Cy", goals: 5 }) })
    put(teams, "blue", Mut Team { coach: "Dee", roster: List(Mut Player { name: "Ed", goals: 2 }) })
    return Mut League { teams: teams, bench: List(Mut Player { name: "Flo", goals: 9 }) }
}

// Structs with no accessor of their own, nested: the handle's path is the
// field chain `a.b.c` plus a position.
struct Cell canbe Mut { v: Int }
struct Inner canbe Mut { c: List<Mut Cell> }
struct Outer canbe Mut { tag: Str, b: Inner }

fn at(a: Outer, index: Int) [] -> ref(a) Mut Cell? => a, index {
    return at(a.b.c, index)
}

fn bump(x: Mut Cell, y: Mut Cell) -> None => x: Mut, y: Mut {
    x.v = x.v + 1
    y.v = y.v + 10
}

fn shared_container() [Console] -> None {
    let l = league()
    let star = at(l, "red", 1) ?: unreachable!()
    star.goals = star.goals + 1
    println("bench ${size(l.bench)}")
    star.goals = star.goals + 1
    trade(l, at(l, "red", 0) ?: unreachable!(), at(l, "blue", 0) ?: unreachable!())
    //~ emits: panic!("salvo: two handles to one element")
    trade(l, star, star)
    println("${star.name} ${star.goals}")
    // The bench and a roster: the paths diverge at the first field.
    let sub = at(l, "", -1) ?: unreachable!()
    if sub is NotSame(star) {
        score(star, sub)
        star.goals = star.goals + 10
        sub.goals = sub.goals + 100
    }
    // Two handles to one element: the test answers false at run time.
    let same_one = at(l, "red", 1) ?: unreachable!()
    //~ emits(std/core/ref.rs): std::ptr::eq(a, b)
    if same_one is NotSame(star) {
        println("wrongly distinct")
    } else {
        println("same element")
    }
    println("${star.goals} ${(get(l.bench, 0) ?: unreachable!()).goals}")
}

fn map_slots() [Console] -> None {
    let l = league()
    // Two map slots: the paths diverge at the slot.
    let a = at(l, "red", 0) ?: unreachable!()
    let b = at(l, "blue", 0) ?: unreachable!()
    if b is NotSame(a) {
        //~ emits: l.teams.pair_mut(
        score(a, b)
        println("${a.goals} ${b.goals}")
    }
    // One slot, two positions in its roster.
    let c = at(l, "red", 1) ?: unreachable!()
    if c is NotSame(a) {
        score(a, c)
    }
    let d = at(l, "", -1) ?: unreachable!()
    if d is NotSame(c) {
        score(c, d)
    }
    println("${c.goals} ${d.goals}")
}

fn nested() [Console] -> None {
    let a = Mut Outer { tag: "t", b: Inner { c: List(Mut Cell { v: 1 }, Mut Cell { v: 2 }) } }
    let p = at(a, 0) ?: unreachable!()
    let q = at(a, 1) ?: unreachable!()
    if q is NotSame(p) {
        //~ emits: crate::seq::salvo_pair_mut(&mut a.b.c[..], __h1, __h2)
        bump(p, q)
        p.v = p.v + 100
        q.v = q.v + 200
    }
    let r = at(a, 0) ?: unreachable!()
    if r is NotSame(p) {
        println("wrongly distinct")
    } else {
        println("same cell")
    }
    println("${(get(a.b.c, 0) ?: unreachable!()).v} ${(get(a.b.c, 1) ?: unreachable!()).v}")
}

fn main() [use] {
    use StdConsole()
    shared_container()
    map_slots()
    nested()
}

//--- handle_reads
// [ref-handle] [rs-path] A handle read where a value is expected: unwrapped
// into a kept owned slot, passed whole (optional) to a borrowed optional
// slot (`ref(c) T <: proj T`, the optional walked if present), and copied
// (`copy` binds `T` to the element, not the handle) — the copy is a value
// of its own, so a later write through the handle leaves it alone.
struct Word canbe Mut { text: Str }
struct B canbe Mut { c: List<Mut Word> }
struct A canbe Mut { b: B, d: List<Mut Word> }

fn at(a: A, index: Int) [] -> ref(a) Mut Word? => a, index {
    if index < 0 {
        return a.d.at(-index)
    }
    return a.b.c.at(index)
}

fn print_one(w: Word) [Console] => w {
    println(w.text)
}

fn maybe_print(w: (proj Word)?) [Console] => w {
    if w is Word {
        println(w.text)
    }
}

fn keep(w: Word) [] -> Word => !w {
    return w
}

export fn main() [use] -> None {
    use StdConsole()
    let a = A { b: B { c: List(Mut Word { text: "something" }) }, d: List(Mut Word { text: "one" }, Mut Word { text: "two" }) }
    let x = at(a, -1)
    let y = at(a, 0)
    let z = at(a, 5)
    print_one(y ?: unreachable!())
    // The optional handle is walked when present.
    //~ emits: .map(|__x
    maybe_print(x)
    maybe_print(z)
    let owned = keep(copy(y))
    let w = y
    w.text = "changed"
    println("${owned.text} ${(get(a.b.c, 0) ?: unreachable!()).text}")
}

//--- one_rendering_for_a_ref_result
// [rs-path] [ref-handle] One rendering for a `ref` result, whatever `T` is:
// `head_of` at a `Mut` element and at `Str`, a `Pick` group through one and
// two implicit layers at both, and a read-only handle held across a write
// through another handle.
struct Counter canbe Mut { n: Int }

// The path function only: no borrow-answering face beside it.
//~ !emits: __loc
//~ emits: -> Option<usize>
fn head_of<T>(xs: List<T>) -> ref(xs) T? => xs {
    return at(xs, 0)
}

fns Pick<C, L, T> {
    fn pick(c: C, l: L) -> ref(c) T?
}

fn pick_via<C, L, T>(c: C, l: L, ?Pick<C, L, T>) -> ref(c) T? => c, l {
    return pick(c, l)
}

fn pick2<C, L, T>(c: C, l: L, ?Pick<C, L, T>) -> ref(c) T? => c, l {
    return pick_via(c, l)
}

fn pick<T>(c: List<T>, l: Int) -> ref(c) T? => c, l {
    return at(c, l)
}

fn bump(c: Mut Counter) -> None => c: Mut {
    c.n = c.n + 1
}

fn main() [use] {
    use StdConsole()
    let cs: Mut List<Mut Counter> = Mut List(Mut Counter { n: 1 }, Mut Counter { n: 2 })
    let h = head_of(cs) ?: unreachable!()
    h.n = h.n + 10
    bump(pick_via(cs, 1) ?: unreachable!())
    bump(pick2(cs, 1) ?: unreachable!())
    println("${(get(cs, 0) ?: unreachable!()).n} ${(get(cs, 1) ?: unreachable!()).n}")
    let ss: List<Str> = List("a", "b")
    println("${head_of(ss) ?: unreachable!()} ${pick_via(ss, 1) ?: unreachable!()} ${pick2(ss, 0) ?: unreachable!()}")
    // a read-only handle held across a write through another handle
    let r = head_of(ss) ?: unreachable!()
    let w = head_of(cs) ?: unreachable!()
    w.n = w.n + 100
    println("${r} ${(get(cs, 0) ?: unreachable!()).n}")
}

