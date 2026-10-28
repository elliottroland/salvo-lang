// mutable element handles [ref-handle] [rs-elem-mut]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [cmp-carry] [rs-borrows] A declared identity over a **Copy scalar** takes its
// arguments by value, while the host's `eq` parameter hands references — so
// the adapter derefs [backend-never-wrong].
//--- a_scalar_identitys_marker_derefs_its_arguments
export fn all_same(a: Int, b: Int) -> Bool => a, b {
    return true
}
fn main() [use] -> None {
    use StdConsole()
    let s: Mut Set<Int> = Mut Set(hash = hash, eq = all_same)
    add(s, 1)
    println("${size(s)}")
}
//= stdout
//= 1

// [cmp-carry] A **written lambda** as a container's `hash`: lent for each
// call like any implicit, so it needs nothing of its own (ROADMAP §0j step
// 7; it used to be moved into an owned `Arc`).
//--- a_written_lambda_at_a_kept_position_moves
struct Point : Hashed<self> by auto {
    x: Int,
    y: Int
}
fn collect(a: Point, ?Hashed<Point>) -> Set<Point> => !a {
    let s: Mut Set<Point> = Mut Set()
    add(s, a)
    return s
}
fn main() [use] -> None {
    use StdConsole()
    let s = collect(Point {x: 1, y: 1}, hash = (p: Point) -> to_long(p.x), eq = eq)
    println("${size(s)}")
}
//= stdout
//= 1

//--- a_borrowed_scalar_comparison

fn main() [use] {
    use StdConsole()
    let xs = List(10, 20, 30)
    let i = 1
    if i is Idx(xs) {
        // A borrowed Copy scalar in a *comparison*: the operand is copied out.
        if get(xs, i) == 20 { println("eq") }
        if get(xs, i) < 30 { println("lt") }
        // Arithmetic and interpolation need no deref (Rust implements
        // `&i32 + i32`), and must not grow one.
        println("${get(xs, i) + 1}")
    }
}
//= stdout
//= eq
//= lt
//= 21

//--- a_contents_mutation_handle

struct Ring { power: Int }
struct Entity canbe Mut {
    hp: Int,
    rings: Mut List<Ring>
}

fn add_ring(e: Mut Entity) -> None => e.rings: Mut {
    add(e.rings, Ring { power: 7 })
}

fn main() [use] {
    use StdConsole()
    let e = Mut Entity { hp: 20, rings: Mut List(Ring { power: 3 }) }
    let rings = e.rings
    add_ring(e)
    // The handle names the very list that grew: 2 on both backends.
    println("${size(rings)} ${size(e.rings)}")
}
//= stdout
//= 2 2

//--- a_copy_of_a_pick_over_a_temporary

struct Box {
    items: List<Str>
}

fn make() [] -> Box {
    return Box { items: List("a", "b") }
}

fn show(s: Str) [] -> Str => !s {
    return s
}

fn main() [use] {
    use StdConsole()
    println(show(copy(get(make().items, 1) ?: unreachable!("empty"))))
}
//= stdout
//= b

//--- elem_mut_handles

struct Counter canbe Mut {
    n: Int
}

fn bump(c: Mut Counter) -> None => c: Mut {
    c.n = c.n + 1
}

fn poke(xs: List<Mut Counter>) -> None {
    let h = at(xs, 1) ?: unreachable!()
    h.n = h.n + 10
    bump(h)
}

fn main() [use] {
    use StdConsole()
    let xs: List<Mut Counter> = List(Mut Counter { n: 1 }, Mut Counter { n: 2 })
    bump(at(xs, 0) ?: unreachable!())
    poke(xs)
    let a = get(xs, 0) ?: unreachable!()
    let b = get(xs, 1) ?: unreachable!()
    println("${a.n} ${b.n}")
}
//= stdout
//= 2 13

//--- a_field_granular_mutation

struct Ring { power: Int }
struct Entity canbe Mut {
    hp: Int,
    rings: Mut List<Ring>
}

fn damage(e: Mut Entity, n: Int) -> None => e.hp: Mut, n {
    e.hp = e.hp - n
}

fn main() [use] {
    use StdConsole()
    let e = Mut Entity { hp: 20, rings: Mut List(Ring { power: 3 }) }
    let rings = e.rings
    damage(e, 5)
    println("${size(rings)} ${e.hp}")
}
//= stdout
//= 1 15

//--- a_generic_keyed_container

struct Member {
    id: Int,
    name: Str
}

// A declared identity that is *not* structural: two members with one id are the
// same member. A generic collector must key by *this*, not by the host's.
fn hash(m: Member) -> Long => m {
    return to_long(m.id)
}

fn eq(a: Member, b: Member) -> Bool => a, b {
    return a.id == b.id
}

fn cmp(a: Member, b: Member) -> Int => a, b {
    return a.id - b.id
}

// A generic body constructing each keyed family. The implicits are forwarded:
// the enclosing fn declares the capability, and the constructor's own
// `?Hashed<T>`/`?Ordered<T>` is filled from it.
fn gather<T>(a: T, b: T, ?Hashed<T>) -> Mut Set<T> => !a, !b {
    let s: Mut Set<T> = Mut Set()
    add(s, a)
    add(s, b)
    return s
}

// …and one that forwards the capability on rather than using it directly, which
// is what makes the demand transitive.
fn gather_three<T>(a: T, b: T, c: T, ?Hashed<T>) -> Set<T> => !a, !b, !c {
    let s: Mut Set<T> = gather(a, b)
    add(s, c)
    return s
}

fn tally<T>(a: T, b: T, ?Hashed<T>) -> Map<T, Int> => !a, !b {
    let m: Mut Map<T, Int> = Mut Map()
    put(m, a, 1)
    put(m, b, 2)
    return m
}

fn ranked<T>(a: T, b: T, ?Ordered<T>) -> SortedSet<T> => !a, !b {
    let s: Mut SortedSet<T> = Mut SortedSet()
    add(s, a)
    add(s, b)
    return s
}

fn scored<T>(a: T, b: T, ?Ordered<T>) -> SortedMap<T, Int> => !a, !b {
    let m: Mut SortedMap<T, Int> = Mut SortedMap()
    put(m, a, 1)
    put(m, b, 2)
    return m
}

fn main() [use] -> None {
    use StdConsole()
    // Fresh values per call: the collectors consume their elements.
    let one = size(gather(Member { id: 1, name: "ann" }, Member { id: 1, name: "bob" }))
    let two = size(gather(Member { id: 1, name: "ann" }, Member { id: 2, name: "cyd" }))
    let three = gather_three(Member { id: 1, name: "ann" }, Member { id: 1, name: "bob" }, Member { id: 2, name: "cyd" })
    let tallied = tally(Member { id: 1, name: "ann" }, Member { id: 2, name: "cyd" })
    // One element: the declared `eq` collapses them.
    println("${one} ${two} ${size(three)} ${size(tallied)}")
    // Ordered by the declared `cmp`, not by the host's.
    let order = ranked(Member { id: 2, name: "cyd" }, Member { id: 1, name: "ann" })
    for m in to_list(order) {
        println(m.name)
    }
    let scores = scored(Member { id: 2, name: "cyd" }, Member { id: 1, name: "ann" })
    // A primitive key still reaches the host's own identity through the same
    // generic body.
    let ints = gather_three(3, 3, 4)
    let sorted = ranked(9, 2)
    println("${size(scores)} ${to_str(ints)} ${to_str(sorted)}")
}
//= stdout
//= 1 2 2 2
//= ann
//= cyd
//= 2 {3, 4} {2, 9}

//--- a_generic_written_identity

// Generic identities, both written at the constructor — one source, so the rule
// allows it. The subject is a type *variable*, which no marker type can name.
export fn wide_hash<T>(a: T) -> Long => a {
    return 1
}

export fn all_same<T>(a: T, b: T) -> Bool => a, b {
    return true
}

fn gather<T>(a: T, b: T, ?Hashed<T>) -> Set<T> => !a, !b {
    let s: Mut Set<T> = Mut Set(hash = wide_hash, eq = all_same)
    add(s, a)
    add(s, b)
    return s
}

fn main() [use] -> None {
    use StdConsole()
    println("${size(gather(1, 2))}")
}
//= stdout
//= 1

//--- a_declared_identity

struct Member {
    id: Int,
    name: Str
}

// A declared identity that is *not* structural: two members with one id are
// the same member, whatever the name says.
fn hash(m: Member) -> Long => m {
    return to_long(m.id)
}

fn eq(a: Member, b: Member) -> Bool => a, b {
    return a.id == b.id
}

fn main() [use] -> None {
    use StdConsole()
    let s: Mut Set<Member> = Mut Set()
    add(s, Member { id: 1, name: "ann" })
    add(s, Member { id: 1, name: "bob" })
    let m: Mut Map<Member, Int> = Mut Map()
    put(m, Member { id: 2, name: "cyd" }, 1)
    put(m, Member { id: 2, name: "dee" }, 2)
    // …and a container of a primitive still keys by the host's own identity.
    let words: Mut Set<Str> = Mut Set()
    add(words, "a")
    add(words, "a")
    add(words, "b")
    println("${size(s)} ${size(m)} ${size(words)}")
}
//= stdout
//= 1 1 2

//--- a_held_identity

struct Point : Hashed<self> by auto {
    x: Int,
    y: Int
}

export fn by_x(p: Point) -> Long => p {
    return to_long(p.x)
}

// Declares only `?hash`, individually — no group spread anywhere. Inside, the
// container's `hash` is *forwarded* from this parameter while its `eq` is
// *resolved* to Point's generated one. Nobody writes anything at the
// constructor.
// Holding the *pair* rather than half of it: both members are then forwarded,
// which is one source.
fn collect(a: Point, b: Point, ?Hashed<Point>) -> Set<Point> => !a, !b {
    let s: Mut Set<Point> = Mut Set()
    add(s, a)
    add(s, b)
    return s
}

fn main() [use] -> None {
    use StdConsole()
    let s = collect(Point {x: 1, y: 1}, Point {x: 1, y: 2}, hash = by_x, eq = eq)
    println("${size(s)}")
}
//= stdout
//= 2

//--- a_mixed_identity

// `Hashed<self> by auto` generates `hash` and `eq`; the constructor keeps the
// generated equality and writes a coarser hash of its own.
struct Point : Hashed<self> by auto {
    x: Int,
    y: Int
}

export fn by_x(p: Point) -> Long => p {
    return to_long(p.x)
}

fn main() [use] -> None {
    use StdConsole()
    let s: Mut Set<Point> = Mut Set(hash = by_x, eq = eq)
    add(s, Point {x: 1, y: 1})
    add(s, Point {x: 1, y: 2})
    add(s, Point {x: 1, y: 1})
    let m: Mut Map<Point, Int> = Mut Map(hash = by_x, eq = eq)
    put(m, Point {x: 5, y: 0}, 1)
    put(m, Point {x: 5, y: 0}, 2)
    let v = get(m, Point {x: 5, y: 0})
    when v {
        is Int { println("${size(s)} ${size(m)} ${v}") }
        is None { println("${size(s)} ${size(m)} missing") }
    }
}
//= stdout
//= 2 1 2

//--- a_mint_into_a_nested_field_paths_through_the_fields

struct Entity canbe Mut { hp: Int }

struct B canbe Mut {
    c: List<Mut Entity>
}

struct A canbe Mut {
    b: B
}

fn at(a: A, index: Int) -> (ref(a) Mut Entity)? {
    return a.b.c.at(index)
}

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn main() [use] {
    use StdConsole()
    let a = Mut A { b: Mut B { c: List(Mut Entity { hp: 1 }, Mut Entity { hp: 2 }) } }
    heal(at(a, 1) ?: unreachable!())
    let h = at(a, 0) ?: unreachable!()
    h.hp = h.hp + 100
    println("${(get(a.b.c, 0) ?: unreachable!()).hp} ${(get(a.b.c, 1) ?: unreachable!()).hp}")
}
//= stdout
//= 101 12

//--- a_ref_anchored_call

struct Entity canbe Mut { hp: Int, energy: Int }

fn attack(c: List<Mut Entity>, a: ref(c) Mut Entity, d: ref(c) Mut Entity) -> None {
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
    attack(es, at(es, i) ?: unreachable!(), at(es, j) ?: unreachable!())
    // …and the aliasing case `ref(c)` exists for: one element, both
    // handles.
    attack(es, at(es, i) ?: unreachable!(), at(es, i) ?: unreachable!())
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 0) ?: unreachable!()).energy} ${(get(es, 1) ?: unreachable!()).hp}")
}
//= stdout
//= 8 3 18

//--- a_search_loop_lender

struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn wounded(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    for e in es {
        if e.hp < 10 {
            return e
        }
    }
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 50 }, Mut Entity { hp: 3 })
    heal(wounded(es) ?: unreachable!())
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
//= stdout
//= 50 13
