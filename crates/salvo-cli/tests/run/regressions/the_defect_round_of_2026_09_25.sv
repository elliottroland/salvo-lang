// the defect round of 2026-09-25
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- arithmetic_as_the_calls_it_names

struct V {
    x: Int,
    y: Int
}

fn plus(a: V, b: V) [] -> V => a, b {
    return V { x: a.x + b.x, y: a.y + b.y }
}

fn minus(a: V, b: V) [] -> V => a, b {
    return V { x: a.x - b.x, y: a.y - b.y }
}

fn times(a: V, k: Int) [] -> V => a {
    return V { x: a.x * k, y: a.y * k }
}

fn divide(a: V, k: Int) [] -> V => a {
    return V { x: a.x / k, y: a.y / k }
}

fn show(v: V) [] -> Str => v {
    return "(${v.x}, ${v.y})"
}

// The brief's `Person`: `**` is the `pow` in its body [fn-attached].
struct Person {
    name: Str,
    age: Int

    fn pow(p: Person, power: Int) [] -> Person => p {
        return Person { name: copy(p.name), age: p.age ** power }
    }
}

// R10 D3: the available effect's member is the `times` the operator calls.
struct M {
    k: Int
}

effect Scale {
    fn times(a: M, b: M) -> M => a, b
}

handler Doubling of Scale {
    fn times(a: M, b: M) -> M {
        return M { k: a.k * b.k * 2 }
    }
}

fn scaled(a: M, b: M) [Scale] -> Int => a, b {
    let m = a * b
    return m.k
}

// R10 D1: the enclosing `?plus` takes `+` on `Int`s, as it takes the call.
fn add_with(a: Int, b: Int, ?plus: (x: Int, y: Int) -> Int) [] -> Int => a, b {
    return a + b
}

fn digits(x: Int, y: Int) [] -> Int => x, y {
    return x * 10 + y
}

// At a generic `T`, `+` is the `?plus` the caller fills.
fn sum3<T>(a: T, b: T, c: T, ?plus: (x: T, y: T) -> T) [] -> T => a, b, c {
    return a + b + c
}

fn main() [use] {
    use StdConsole()
    let a = V { x: 1, y: 2 }
    let b = V { x: 3, y: 4 }
    println("${show(a + b)} ${show(b - a)} ${show(a * 3)} ${show((a + b) / 2)}")
    println("${add_with(1, 2, plus = digits)} ${add_with(1, 2)} ${sum3(1, 2, 3)} ${show(sum3(a, b, a))}")
    use Doubling()
    println("${scaled(M { k: 3 }, M { k: 4 })}")
    let i = 7
    let l = 3000000000L
    println("${i + l} ${l - i} ${-7 / 2} ${2.5 * 2.0} ${1.0f + 0.5f}")
    println("${2 ** 10} ${-2 ** 2} ${2 ** 3 ** 2} ${1 + 2 * 3 ** 2} ${3L ** 3} ${2L ** 40}")
    println("${2.0 ** 3} ${2.0 ** -2} ${2.0 ** 0.5} ${9.0 ** 0.5}")
    let p = Person { name: "Ann", age: 3 }
    let q = p ** 2
    println("${q.name} ${q.age}")
    println("${2147483647 + 1} ${2 ** 31} ${3 ** 40} ${-2147483647 - 2}")
}
//= stdout
//= (4, 6) (2, 2) (3, 6) (2, 3)
//= 12 3 6 (5, 8)
//= 24
//= 3000000007 2999999993 -3 5.0 1.5
//= 1024 -4 512 19 27 1099511627776
//= 8.0 0.25 1.4142135623730951 3.0
//= Ann 9
//= -2147483648 -2147483648 689956897 2147483647

//--- bare_literal_arguments

fn main() [use] -> None {
    use StdConsole()
    println("A ${size(to_set(List(1, 2)))}")
    println("B ${size(to_list(Set(3, 4)))}")
    let words = to_set(List("a", "b", "a"))
    println("C ${size(words)}")
}
//= stdout
//= A 2
//= B 2
//= C 2

//--- a_bound_accessor_handle

struct Entity canbe Mut { hp: Int }

fn front(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    return at(es, 0)
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    let boss = front(es) ?: unreachable!()
    boss.hp = boss.hp + 100
    let n = size(es)
    boss.hp = boss.hp + n
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
//= stdout
//= 107 7

//--- a_fn_value_by_name_ranked_as_a_call

import lib.tag
fn tag<T>(v: T) [] -> Str => v { return "any" }
fn apply(f: (x: Int) -> Str, x: Int) [] -> Str => f { return f(x) }
fn main() [use] {
    use StdConsole()
    println(tag(1))
    println(apply(tag, 1))
    println(apply(tag@main, 1))
}
// FILE: lib.sv
export fn tag(v: Int) [] -> Str => v { return "int" }
//= stdout
//= int
//= int
//= any

//--- comparisons_as_the_calls_they_name

import bag.Bag
import bag.pack
import bag.eq

struct Meters {
    v: Int
}

struct Feet {
    v: Int
}

// R10 D6: a mixed `eq`, declared with its left operand's type.
fn eq(a: Meters, b: Feet) [] -> Bool => a, b {
    return a.v * 328 == b.v * 100
}

struct Point {
    x: Int,
    y: Int
}

effect Same {
    fn eq(a: Point, b: Point) -> Bool => a, b
}

handler ByX of Same {
    fn eq(a: Point, b: Point) -> Bool {
        return a.x == b.x
    }
}

fn same_len(a: Str, b: Str) [] -> Bool => a, b {
    return size(a) == size(b)
}

fn by_len(a: Str, b: Str) [] -> Int => a, b {
    return cmp(size(a), size(b))
}

fn backwards(a: Double, b: Double) [] -> Int => a, b {
    if a < b {
        return 1
    }
    if a > b {
        return -1
    }
    return 0
}

// R10 D1: the enclosing implicits take `==` and `<` on `Str`, as they take the calls.
fn check(a: Str, b: Str, ?eq: (a: Str, b: Str) -> Bool, ?cmp: (a: Str, b: Str) -> Int) [] -> Str => a, b {
    return "${a == b} ${a < b}"
}

// …and `<` on a `Double`, which has no `cmp` of its own.
fn float_lt(a: Double, b: Double, ?cmp: (a: Double, b: Double) -> Int) [] -> Bool => a, b {
    return a < b
}

// R10 D3: the available effect's member is the `eq` the operator calls.
fn points(a: Point, b: Point) [Same] -> Str => a, b {
    return "${a == b} ${a != b}"
}

fn main() [use] {
    use StdConsole()
    println(check("ab", "cd", eq = same_len, cmp = by_len))
    println("${float_lt(1.0, 2.0, cmp = backwards)} ${1.0 < 2.0}")
    let xs = pack(Bag { items: List(1, 2) })
    let ys = pack(Bag { items: List(3, 4) })
    println("${xs == ys}")
    let m = Meters { v: 100 }
    let f = Feet { v: 328 }
    println("${m == f}")
    use ByX()
    println(points(Point { x: 1, y: 2 }, Point { x: 1, y: 3 }))
    let n = -1L
    let x: Long? = n
    let y: Long? = None
    println("${x == -1L} ${y == -1L}")
    let i = 3
    println("${i < 4L} ${2.5 > 1.0f} ${i == 3L}")
}
// FILE: bag.sv

// The two overloads of `eq` on `Bag`, in `Bag`'s own file [op-unique-shape].
export struct Bag<T> {
    items: List<T>
}

export qualifier Packed of Bag<Int>

export fn eq<T>(a: Bag<T>, b: Bag<T>, ?eq: (x: T, y: T) -> Bool) [] -> Bool => a, b {
    return false
}

export fn eq(a: Packed Bag<Int>, b: Packed Bag<Int>) [] -> Bool => a, b {
    return true
}

export fn pack(b: Bag<Int>) -> +Packed Bag<Int> => !b {
    return b
}
//= stdout
//= true false
//= false true
//= true
//= true
//= true false
//= true false
//= true true true

//--- deque_handles_and_the_total_map_at

struct Item canbe Mut { n: Int }

fn bump(i: Mut Item) -> None => i: Mut {
    i.n = i.n + 1
}

fn main() [use] {
    use StdConsole()
    let d: Mut Deque<Mut Item> = mut_deque_of()
    add_first(d, Mut Item { n: 1 })
    add_last(d, Mut Item { n: 2 })
    let h = at(d, 1) ?: unreachable!()
    h.n = h.n + 10
    println("${size(d)} ${h.n}")
    bump(at(d, 0) ?: unreachable!())
    let i = 0
    if i is Idx(d) {
        bump(at(d, i))
    }
    println("${at(d, 7) is None} ${(get(d, 0) ?: unreachable!()).n} ${(get(d, 1) ?: unreachable!()).n}")
    let m: Mut Map<Str, Mut Item> = Mut Map()
    put(m, "a", Mut Item { n: 5 })
    let k = "a"
    if k is KeyOf(m) {
        let v = at(m, k)
        v.n = v.n * 3
        bump(at(m, k))
    }
    println("${(get(m, "a") ?: unreachable!()).n}")
}
//= stdout
//= 2 12
//= true 3 12
//= 16

//--- distinct_pair_calls

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
    if d is NotSame(a) {
        attack(a, d)
        a.hp = a.hp + 1
        d.hp = d.hp + 1
        attack(a, d)
    }
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 0) ?: unreachable!()).energy} ${(get(es, 1) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).energy}")
}
//= stdout
//= 11 3 17 8

//--- handles_into_element_containers

struct Track canbe Mut { plays: Int }
struct Album canbe Mut { name: Str, tracks: List<Mut Track> }
struct Library canbe Mut { albums: List<Mut Album> }

// A mint reaching two levels into the library.
fn track(lib: Library, i: Int, j: Int) -> ref(lib) Mut Track? {
    let al = at(lib.albums, i)
    if al is Album {
        return at(al.tracks, j)
    }
    return None
}

// Handles anywhere inside `lib`.
fn swap_plays(lib: Library, a: ref(lib) Mut Track, b: ref(lib) Mut Track) -> None {
    let p = a.plays
    a.plays = b.plays
    b.plays = p
}

// A handle parameter anchored at another handle parameter: positions in
// `al`, which is a position in `lib`.
fn swap_in(lib: Library, al: ref(lib) Mut Album, a: ref(al) Mut Track, b: ref(al) Mut Track) -> None {
    let p = a.plays
    a.plays = b.plays
    b.plays = p
    al.name = "${al.name}*"
}

fn main() [use] {
    use StdConsole()
    let lib = Mut Library { albums: List(
        Mut Album { name: "x", tracks: List(Mut Track { plays: 1 }, Mut Track { plays: 2 }) },
        Mut Album { name: "y", tracks: List(Mut Track { plays: 3 }) }) }
    // Through a mint, inline nested `at`s, and a bound handle.
    swap_plays(lib, track(lib, 0, 1) ?: unreachable!(), track(lib, 1, 0) ?: unreachable!())
    swap_plays(lib, at((at(lib.albums, 0) ?: unreachable!()).tracks, 0) ?: unreachable!(), at((at(lib.albums, 1) ?: unreachable!()).tracks, 0) ?: unreachable!())
    let al = at(lib.albums, 0) ?: unreachable!()
    swap_plays(lib, at(al.tracks, 0) ?: unreachable!(), track(lib, 1, 0) ?: unreachable!())
    // Self-strike is fine: one container, two positions that coincide.
    swap_in(lib, al, at(al.tracks, 0) ?: unreachable!(), at(al.tracks, 1) ?: unreachable!())
    swap_in(lib, al, at(al.tracks, 1) ?: unreachable!(), at(al.tracks, 1) ?: unreachable!())
    let a0 = get(lib.albums, 0) ?: unreachable!()
    let a1 = get(lib.albums, 1) ?: unreachable!()
    println("${a0.name} ${(get(a0.tracks, 0) ?: unreachable!()).plays} ${(get(a0.tracks, 1) ?: unreachable!()).plays} ${a1.name} ${(get(a1.tracks, 0) ?: unreachable!()).plays}")
}
//= stdout
//= x** 3 1 y 2

//--- a_written_fn_value_across_erased_qualifiers

fn any_text<T>(x: T) [] -> Str => x { return "?" }
fn render_row(xs: List<Int>) [] -> Str => xs { return to_str(xs, to_str = any_text) }
fn apply(f: (xs: NonEmpty List<Int>) -> Str, xs: NonEmpty List<Int>) [] -> Str => f, xs { return f(xs) }
fn shout(s: Str) [] -> Str => s { return "${s}!" }
fn apply_ok(f: (s: Ok Str) -> Str, s: Ok Str) [] -> Str => f, s { return f(s) }
fn main() [use] {
    use StdConsole()
    let xss = List(List(1, 2), List(3))
    println(to_str(xss, to_str = render_row))
    println(apply(render_row, List(1, 2)))
    println(apply_ok(shout, ok("hi")))
}
//= stdout
//= [[?, ?], [?]]
//= [?, ?]
//= hi!

//--- salvos_float_text

struct Point : ToStr<self> by auto {
    x: Double,
    ratio: Float
}

fn main() [use] -> None {
    use StdConsole()
    println("${2.0} ${8.25} ${0.0 - 2.0} ${0.0} ${0.001} ${0.0001}")
    println("${1234567.0} ${12345678.0} ${100000000000000000000.0} ${0.00000000000001}")
    let zero = 0.0
    let one = 1.0
    println("${zero / zero} ${one / zero} ${0.0 - one / zero}")
    println("${Point { x: 2.0, ratio: 3.0 }}")
    let xs: List<Double> = List(2.0, 0.5)
    println("${xs} ${to_str(xs)} ${List(4.0)}")
    let m: Map<Str, Double> = Map("a": 2.0)
    println("${m}")
}
//= stdout
//= 2.0 8.25 -2.0 0.0 0.001 1.0E-4
//= 1234567.0 1.2345678E7 1.0E20 1.0E-14
//= NaN Infinity -Infinity
//= Point { x: 2.0, ratio: 3.0 }
//= [2.0, 0.5] [2.0, 0.5] [4.0]
//= {a: 2.0}

//--- what_the_flow_pass_now_accepts

struct Rec {
    name: Str,
    tags: Mut List<Str>
}

struct Nest {
    a: Rec,
    other: Mut List<Str>
}

fn eat(s: Str) -> None => !s {
}

fn eatl(xs: Mut List<Str>) -> Int => !xs {
    return size(xs)
}

fn look(s: Str) -> Int {
    return size(s)
}

fn take<T>(v: T) -> None => !v {
}

// A body that always leaves has no back edge.
fn drain(c: Bool, s: Str) -> Int => !s {
    let n = 0
    while c {
        n = look(s)
        eat(s)
        break
    }
    return n
}

// A union narrowed to a Copy scalar moves for free.
fn count(v: Int | Bool) -> Int => !v {
    let n = 0
    if v is Int {
        take(v)
        take(v)
        n = v
    }
    take(v)
    return n
}

// A sibling of a moved field, under the same parent, is still there.
fn sibling(n: Nest) -> Int => !n {
    let k = eatl(n.a.tags)
    return k + look(n.a.name)
}

// A top-level `break` of a local moves it out of a body that always leaves.
fn first(c: Bool) -> Int {
    let s = "ab${1}"
    let r = while c {
        break s
    }
    if r is Str {
        return look(r)
    }
    return 0
}

fn main() [use] {
    use StdConsole()
    println("${drain(true, "abc${1}")} ${drain(false, "x")}")
    println("${count(7)} ${count(true)}")
    println("${first(true)} ${first(false)}")
    let t = Mut List<Str>()
    add(t, "a")
    add(t, "b")
    println("${sibling(Nest { a: Rec { name: "four", tags: t }, other: Mut List<Str>() })}")
}
//= stdout
//= 4 0
//= 7 0
//= 3 0
//= 6

//--- a_for_as_the_calls_it_names

// R10 D8: the most specific `next` drives `for`, as it takes the call.
// R10 D10: `Ctr` declares no `: Next<self, Int>`.
struct Ctr<T> canbe Mut {
    n: Int,
    tag: T
}

fn next<T>(c: Mut Ctr<T>) [] -> Emitted Int | Finished => c: Mut {
    if c.n >= 2 {
        return finished()
    }
    c.n = c.n + 1
    return emitted(c.n)
}

fn next(c: Mut Ctr<Str>) [] -> Emitted Int | Finished => c: Mut {
    if c.n >= 2 {
        return finished()
    }
    c.n = c.n + 1
    return emitted(c.n * 100)
}

// R10 D9: a renamed `next` answers to its new name only, so `for` mints
// with the `iter` instead.
struct Steps canbe Mut {
    n: Int,
    items: List<Int>
}

fn next(s: Mut Steps) [] -> Emitted Int | Finished => s: Mut {
    if s.n >= 2 {
        return finished()
    }
    s.n = s.n + 1
    return emitted(s.n)
}

rename fn advance = next(s: Mut Steps)

fn iter(s: Steps) [] -> Mut ListNext<Int> => s {
    return iter(s.items)
}

// R10 D11: the enclosing fn's own `?next` drives, as it takes the call.
struct Tick canbe Mut {
    n: Int
}

fn next(t: Mut Tick) [] -> Emitted Int | Finished => t: Mut {
    if t.n >= 2 {
        return finished()
    }
    t.n = t.n + 1
    return emitted(t.n)
}

fn by_tens(t: Mut Tick) [] -> Emitted Int | Finished => t: Mut {
    if t.n >= 2 {
        return finished()
    }
    t.n = t.n + 1
    return emitted(t.n * 10)
}

fn walk(t: Mut Tick, ?next: (t: Mut Tick) -> Emitted Int | Finished) [] -> Str => t: Mut {
    let out = Mut List<Int>()
    for x in t {
        add(out, x)
    }
    return "${out}"
}

// R10 D3: an available effect's member `next` drives.
struct Gauge canbe Mut {
    n: Int
}

effect Gen {
    fn next(g: Mut Gauge) -> Emitted Int | Finished => g: Mut
}

handler ByThrees of Gen {
    fn next(g: Mut Gauge) -> Emitted Int | Finished => g: Mut {
        if g.n >= 2 {
            return finished()
        }
        g.n = g.n + 1
        return emitted(g.n * 3)
    }
}

fn gauge(g: Mut Gauge) [Gen] -> Str => g: Mut {
    let out = Mut List<Int>()
    for x in g {
        add(out, x)
    }
    return "${out}"
}

// A type with both a `next` and an `iter` is driven by its `next` (P15).
struct Both canbe Mut {
    n: Int,
    items: List<Int>
}

fn next(b: Mut Both) [] -> Emitted Int | Finished => b: Mut {
    if b.n >= 2 {
        return finished()
    }
    b.n = b.n + 1
    return emitted(b.n)
}

fn iter(b: Both) [] -> Mut ListNext<Int> => b {
    return iter(b.items)
}

fn main() [use] {
    use StdConsole()
    let c = Mut Ctr { n: 0, tag: "s" }
    let cs = Mut List<Int>()
    for x in c {
        add(cs, x)
    }
    println("${cs}")
    let s = Mut Steps { n: 0, items: List(7, 8, 9) }
    let ss = Mut List<Int>()
    for x in s {
        add(ss, x)
    }
    let a = advance(s)
    println("${ss} ${a is Emitted Int}")
    println(walk(Mut Tick { n: 0 }, next = by_tens))
    println(walk(Mut Tick { n: 0 }))
    use ByThrees()
    println(gauge(Mut Gauge { n: 0 }))
    let b = Mut Both { n: 0, items: List(7, 8, 9) }
    let bs = Mut List<Int>()
    for x in b {
        add(bs, x)
    }
    println("${bs}")
}
//= stdout
//= [100, 200]
//= [7, 8, 9] true
//= [10, 20]
//= [1, 2]
//= [3, 6]
//= [1, 2]

//--- handles_into_generic_containers

struct Player canbe Mut { name: Str, goals: Int }
struct Team canbe Mut { coach: Str, roster: List<Mut Player> }
struct League canbe Mut {
    teams: Map<Str, Mut Team>,
    bench: List<Mut Player>
}

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
fn bump<C>(c: C, a: ref(c) Mut Player) -> None {
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
//= stdout
//= 1009 6 2000
//= 14 103

//--- handle_reads

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
    maybe_print(x)
    maybe_print(z)
    let owned = keep(copy(y))
    let w = y
    w.text = "changed"
    println("${owned.text} ${(get(a.b.c, 0) ?: unreachable!()).text}")
}
//= stdout
//= something
//= two
//= something changed

//--- handler_parameter_defaults

effect Tell {
    fn tell() -> Int
}

handler Teller(n: Int = 7) of Tell {
    fn tell() -> Int {
        return n
    }
}

effect Spanned {
    fn width() -> Int
    fn stride() -> Int
}

handler Span(lo: Int, hi: Int = lo + 10, step: Int = 1) of Spanned {
    fn width() -> Int {
        return hi - lo
    }
    fn stride() -> Int {
        return step
    }
}

effect Counted {
    fn count() -> Int
}

handler Bag(xs: List<Int>, n: Int = size(xs)) of Counted {
    fn count() -> Int {
        return n + size(xs)
    }
}

effect Sum {
    fn sum() -> Int
}

handler Plus [Tell] of Sum {
    fn sum() -> Int {
        return tell() + 1
    }
}

actor effect Desk {
    send fn ask(out: Reply<Int>) => !out
}

handler Desking(room: Int = 4, capacity: Int = 2) of Desk {
    mailbox { capacity: capacity }

    send fn ask(out: Reply<Int>) {
        send(out, room)
    }
}

handler Telling() [Tell] of Desk {
    mailbox { capacity: 2 }

    send fn ask(out: Reply<Int>) {
        send(out, tell())
    }
}

effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int = 5, mult: Int = 31) of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 1
    }

    send fn advance(out: Reply<Int>) => !out {
        cursor = (cursor * mult + seed) % 1000
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn show() [use, Console] {
    use Teller(n = 3)
    println("${tell()}")
}

fn spans() [use, Console] {
    use Span(5)
    println("${width()} ${stride()}")
    use Span(5, step = 2)
    println("${width()} ${stride()}")
    let lo = 100
    use Span(1, step = lo)
    println("${width()} ${stride()} ${lo}")
    use Span(1, 4)
    println("${width()} ${stride()}")
}

fn sums() [use, Console] {
    use Plus with Teller(n = 40)
    println("${sum()}")
    use Plus with Teller()
    println("${sum()}")
}

fn main() [use, spawn] {
    use StdConsole()
    use Teller()
    println("${tell()}")
    show()
    spans()
    sums()
    use Bag(List(1, 2, 3))
    println("count ${count()}")
    let desk = spawn Desking() on pool(1)
    let room = waitfor got: Reply<Int> {
        desk.ask(got)
    }
    println("room ${room}")
    let teller = spawn Telling() with Teller(n = 11) on pool(1)
    let told = waitfor got: Reply<Int> {
        teller.ask(got)
    }
    println("told ${told}")
    let rng = spawn CyclicRandom(mult = 2)
    use rng
    println("drew ${next()} ${next()}")
}
//= stdout
//= 7
//= 3
//= 10 1
//= 10 2
//= 10 100 100
//= 3 1
//= 41
//= 8
//= count 6
//= room 4
//= told 11
//= drew 7 19

//--- implicits_chosen_as_calls_are

fn any_text<T>(x: T) [] -> Str => x { return "?" }

struct Foo { n: Int }

fn to_str(f: Foo) [] -> Str => f { return "?" }

fn hex(n: Int) [] -> Str => n { return "#${n}" }

fn show(xsss: NonEmpty List<NonEmpty List<NonEmpty List<Int>>>, ?to_str: (x: Int) -> Str) [Console] -> None => xsss {
    println("${xsss}")
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2)
    println("${List(1, 2)}")
    println(to_str(xs))
    println("${List(Foo { n: 1 })}")
    println("${to_str(xs, to_str = any_text)}")
    let xsss = List(List(List(1, 2), List(3)), List(List(4)))
    show(xsss, to_str = hex)
}
//= stdout
//= [1, 2]
//= [1, 2]
//= [?]
//= [?, ?]
//= [[[#1, #2], [#3]], [[#4]]]

//--- implicits_selected_as_calls_select

import lib.by_len

struct Point { x: Int, y: Int }

effect Fmt {
    fn to_str(p: Point) -> Str => p
    fn tag() -> Str
}

handler Brackets of Fmt {
    fn to_str(p: Point) -> Str { return "<${p.x}, ${p.y}>" }
    fn tag() -> Str { return "fmt" }
}

fn render(p: Point, ?to_str: (p: Point) -> Str) [] -> Str => p { return to_str(p) }

fn tagged(p: Point, ?to_str: (p: Point) -> Str) [Fmt] -> Str => p { return "${tag()} ${to_str(p)}" }

fn show(p: Point) [Fmt, Console] -> None => p {
    println(render(p))
    println(render(p, to_str = to_str@Fmt))
}

fn f(x: Int) [] -> Str => x { return "int" }
fn f<T>(x: T) [] -> Str => x { return "any" }
fn conv<B>(x: Int, ?f: (x: Int) -> B) [] -> B => x { return f(x) }

fn tag(x: Int) [] -> Str => x { return "int" }
fn tag(x: Int | Str) [] -> Str => x { return "any" }
fn both(n: Int, ?tag: (x: Int) -> Str, ?tag: (x: Int | Str) -> Str) [] -> Str => n { return tag(n) }

fn by_len(a: Str, b: Str) [] -> Int => a, b { return size(a) - size(b) }

fn main() [use] {
    use StdConsole()
    use Brackets()
    show(Point { x: 1, y: 2 })
    println(render(Point { x: 3, y: 4 }))
    println(tagged(Point { x: 5, y: 6 }))
    let s = conv(1)
    println(s)
    println(both(1))
    let m: Mut SortedSet<Str>(by_len@main) = Mut SortedSet("ccc", "a", "bb", cmp = by_len@main)
    println("${m}")
}
// FILE: lib.sv
export fn by_len(a: Str, b: Str) [] -> Int => a, b { return size(b) - size(a) }
//= stdout
//= <1, 2>
//= <1, 2>
//= <3, 4>
//= fmt <5, 6>
//= int
//= int
//= {a, bb, ccc}

//--- a_to_str_from_another_module
// FILE: pair.sv

export struct Pair {
    a: Int,
    b: Int

    fn to_str(p: Pair) -> Str => p {
        return "(${p.a}, ${p.b})"
    }
}
// FILE: main.sv

import pair.Pair

fn main() [use] -> None {
    use StdConsole()
    let p = Pair { a: 1, b: 2 }
    println("${p}")
}
//= stdout
//= (1, 2)

//--- an_unannotated_lambda_takes_its_handler_constructor_parameters_type

effect Calc { fn calc(n: Int) -> Int }
handler Derived(step: (n: Int) -> Int) of Calc {
    fn calc(n: Int) -> Int { return step(n) }
}
fn main() [use] {
    use StdConsole()
    use Derived(n -> n + 1)
    println("${calc(1)}")
}
//= stdout
//= 2

//--- a_handle_from_a_lending_member_survives_a_read_of_its_container

struct Entity canbe Mut { hp: Int }

effect Lender {
    fn lease(es: List<Mut Entity>) -> (ref(es) Mut Entity)? => es
}

handler FirstLender of Lender {
    fn lease(es: List<Mut Entity>) -> (ref(es) Mut Entity)? => es {
        return at(es, 0)
    }
}

fn run(es: List<Mut Entity>) [Lender] -> None {
    let e = lease(es) ?: unreachable!()
    let n = size(es)
    e.hp = e.hp + n
    return None
}

fn main() [use] {
    use StdConsole()
    use FirstLender()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    run(es)
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
//= stdout
//= 7 7

//--- a_lending_member_through_a_struct_field

struct Entity canbe Mut { hp: Int }
struct Team canbe Mut { name: Str, members: List<Mut Entity> }

fn pick(t: Team, i: Int) -> (ref(t) Mut Entity)? => t {
    return at(t.members, i)
}

effect Roster {
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
//= stdout
//= renamed 1003 102

//--- a_mut_lending_accessor

struct Entity canbe Mut { hp: Int }

fn front(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    return at(es, 0)
}

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    heal(front(es) ?: unreachable!())
    let hp = (front(es) ?: unreachable!()).hp
    println("${hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
//= stdout
//= 15 7

//--- a_lending_fn_value

struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn bump_at(es: List<Mut Entity>, i: Int,
           at: (c: List<Mut Entity>, k: Int) -> ref(c) Mut Entity?) -> None {
    heal(at(es, i) ?: unreachable!())
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    bump_at(es, 1, (c: List<Mut Entity>, k: Int) -> at(c, k))
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
//= stdout
//= 5 17

//--- a_lending_effect_member

struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

effect Lender {
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
//= stdout
//= 15 7

//--- a_lifted_claim_on_a_field_narrows_what_follows

import std.throw
struct H { result: Ok (Ok Int | Err Str) | Thrown Str }

fn wrapped(n: Int) [Throw<Str>] -> Ok Int | Err Str {
    if n < 0 { throw("neg") }
    if n == 7 { return err("bad") }
    return ok(n)
}

fn main() [use] {
    use StdConsole()
    let h = H { result: try { wrapped(7) } }
    if h.result is ^Ok {
        if h.result is Ok { println("ok ${h.result}") } else { println("not ok") }
        if h.result is Err { println("err ${h.result}") } else { println("not err") }
    }
}
//= stdout
//= not ok
//= err bad

//--- the_locate_bundle

struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn heal_at<L>(c: List<Mut Entity>, l: L, ?Ref<List<Mut Entity>, L, Entity>) -> None {
    heal(at(c, l) ?: unreachable!())
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    heal_at(es, 1)
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
//= stdout
//= 5 17

//--- a_breaking_value_loop_and_ordered_arguments

fn first_big(xs: List<Int>) -> Int? => xs {
    return for x in xs {
        if x > 2 { break x }
    }
}

fn noisy(n: Int) [Console] -> Int {
    println("noisy ${n}")
    return n
}

fn both(f: (a: Int, b: Int) [Console] -> Int, x: Int) [Console] -> Int {
    return f(noisy(1), f(noisy(2), x))
}

fn main() [use] {
    use StdConsole()
    println("${first_big(List(1, 5)) ?: 0} ${first_big(List(1)) ?: 0}")
    println("${both((a, b) -> { println("f ${a} ${b}") return a + b }, 10)}")
}
//= stdout
//= 5 0
//= noisy 1
//= noisy 2
//= f 2 10
//= f 1 12
//= 13

//--- an_infinite_loop_as_never

fn pick(n: Int) -> Int {
    if n > 0 { return n }
    while true {
    }
}

fn main() [use] {
    use StdConsole()
    println("${pick(7)}")
}
//= stdout
//= 7

//--- a_loop_over_a_temporary

fn main() [use] -> None {
    use StdConsole()
    for x in iter(List(1, 2)) {
        println("a ${x}")
    }
    for x in filter(iter(List(3, 4, 5)), (n: Int) -> { return n > 3 }) {
        println("b ${x}")
    }
    let xs: List<Int> = List(9)
    for x in xs {
        println("c ${x}")
    }
}
//= stdout
//= a 1
//= a 2
//= b 4
//= b 5
//= c 9

//--- a_mutable_handle_chained_through_a_map_at

struct Item canbe Mut { tag: Str }

export fn main() [use] -> None {
    use StdConsole()
    let m: Mut Map<Str, Mut List<Mut Item>> = Mut Map()
    let l: Mut List<Mut Item> = Mut List(Mut Item { tag: "x" }, Mut Item { tag: "y" })
    put(m, "a", l)
    let it = at(at(m, "a") ?: unreachable!(), 1) ?: unreachable!()
    it.tag = "z"
    let v = at(m, "a") ?: unreachable!()
    add(v, Mut Item { tag: "w" })
    println("${(get(get(m, "a") ?: unreachable!(), 1) ?: unreachable!()).tag} ${size(get(m, "a") ?: unreachable!())}")
}
//= stdout
//= z 3

//--- mut_optional_slots

struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity?) -> Int => e: Mut {
    if e is Entity {
        e.hp = e.hp + 1
        return e.hp
    }
    return -1
}

fn heal_twice(e: Mut Entity?) -> None => e: Mut {
    heal(e)
    heal(e)
}

fn shout(s: Mut Str?) -> None => s: Mut {
    if s is Mut Str {
        append(s, "!")
    }
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 3 }, Mut Entity { hp: 10 })
    println("${heal(at(es, 0))} ${heal(at(es, 9))}")
    let h = at(es, 1)
    heal(h)
    heal_twice(h)
    heal_twice(at(es, 0))
    let own: Mut Entity? = Mut Entity { hp: 100 }
    heal(own)
    heal(None)
    let ws: List<Mut Str> = List(Mut Str("hey"))
    shout(at(ws, 0))
    shout(at(ws, 0))
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp} ${(own ?: unreachable!()).hp} ${get(ws, 0) ?: unreachable!()}")
}
//= stdout
//= 4 -1
//= 6 13 101 hey!!

//--- none_typed_return_into_optional

fn log_it() -> None {
    return None
}

fn pick(which: Int) -> Str? {
    if which == 0 { return log_it() }
    return "value"
}

fn show(r: Str?, tag: Str) [Console] -> None {
    when r {
        is Str { println("${tag}: ${r}") }
        is None { println("${tag}: none") }
    }
}

fn main() [use] {
    use StdConsole()
    show(pick(0), "zero")
    show(pick(1), "one")
}
//= stdout
//= zero: none
//= one: value

//--- notsame_handles

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
//= stdout
//= bench 1
//= Cy 7
//= same element
//= 18 110
//= 1 3
//= 7 10
//= same cell
//= 102 212

//--- borrowed_optionals_in_owned_optional_slots

struct Entity canbe Mut { name: Str, hp: Int }

fn inner(s: Str?) -> Str {
    return s ?: "-"
}

fn outer(s: Str?) -> Str {
    let t = s
    let u = inner(t)
    let n = if s is Str v { to_upper(v) } else { "?" }
    let c = copy(s)
    let none = if s is None { "absent" } else { "present" }
    return "${u} ${n} ${inner(c)} ${none}"
}

fn named(e: Entity?) -> Str {
    if e is Entity f {
        return "${f.name}:${f.hp}"
    }
    return "nobody"
}

fn view(s: Str?) -> (proj(s) Str)? => s {
    return s
}

effect Shower {
    fn show(s: Str?) -> None => s
}

handler PrintShower [Console] of Shower {
    fn show(s: Str?) -> None => s {
        println("shown ${inner(s)}")
    }
}

fn main() [use] {
    use StdConsole()
    use PrintShower()
    let xs: List<Str> = List("abc", "de")
    println(outer(get(xs, 0)))
    println(outer(get(xs, 9)))
    let o: Str? = "own"
    println(outer(o))
    println(outer("lit"))
    println(outer(None))
    let ws: List<Mut Str> = List(Mut Str("w"))
    println(outer(get(ws, 0)))
    println(outer(at(ws, 0)))
    let cw = copy(get(ws, 0))
    println(outer(cw))
    let es: List<Mut Entity> = List(Mut Entity { name: "e", hp: 3 })
    println("${named(get(es, 0))} ${named(at(es, 0))} ${named(get(es, 4))} ${named(None)}")
    let h = at(es, 0)
    println(named(h))
    let ce = copy(get(es, 0))
    let hh = h ?: unreachable!()
    hh.hp = 40
    println("${named(ce)} ${named(get(es, 0))}")
    println(inner(view(get(xs, 1))))
    show(get(xs, 0))
    show(o)
}
//= stdout
//= abc ABC abc present
//= - ? - absent
//= own OWN own present
//= lit LIT lit present
//= - ? - absent
//= w W w present
//= w W w present
//= w W w present
//= e:3 e:3 nobody nobody
//= e:3
//= e:3 e:40
//= de
//= shown abc
//= shown own

//--- a_disjunction_and_a_negation_narrow_nothing_after_them

fn pick(n: Int) -> Int | Str | Bool {
    if n == 1 { return "one" }
    if n == 2 { return true }
    return n
}

fn main() [use] {
    use StdConsole()
    for k in List(0, 1, 2) {
        let v = pick(k)
        if v is Int || v is Str { println("int or str") }
        if v is Int n || v is Str s { println("bound") }
        if !(v is Int) { println("not int") } else { println("int") }
        if v is Bool b {
            if b { println("true") }
        }
    }
}
//= stdout
//= int or str
//= bound
//= int
//= int or str
//= bound
//= not int
//= not int
//= true

// [rs-elem-mut] A proven pair call in a *value* position: the split is a
// block expression around the call, so the position needs no statement of
// its own.
//--- a_distinct_pair_call_in_a_value_position_splits_in_place

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
        let x = poke(a, d)
        println("${x}")
    }
}
//= stdout
//= 2

//--- parameter_defaults

fn greet(name: Str, greeting: Str = "Hello", punct: Str = "!") -> Str {
    return "${greeting}, ${name}${punct}"
}

fn span(lo: Int, hi: Int = lo + 10) -> Int {
    return hi - lo
}

fn describe(xs: List<Int>, count: Int = size(xs)) -> Str {
    return "size is ${count}"
}

struct Point { x: Int, y: Int }

fn locate(p: Point, label: Str = "${p.x},${p.y}") -> Str => !label {
    return label
}

fn main() [use] {
    use StdConsole()
    println(greet("Ada", "Hi", "."))
    println(greet("Ada", "Hi"))
    println(greet("Ada"))
    println(greet("Ada", punct = "?"))
    println("${span(5)}")
    println("${span(5, 20)}")
    println(describe(List(1, 2, 3)))
    println(locate(Point { x: 3, y: 4 }))
}
//= stdout
//= Hi, Ada.
//= Hi, Ada!
//= Hello, Ada!
//= Hello, Ada?
//= 10
//= 15
//= size is 3
//= 3,4

//--- placeholder_loops

fn main() [use] {
    use StdConsole()
    let hits: Mut List<Int> = Mut List()
    // [expr-placeholder] Two loops that bind nothing, in one function: neither
    // introduces a name, so neither can collide with the other.
    for _ in range(0, 3) {
        add(hits, 1)
    }
    for _ in range(0, 2) {
        add(hits, 2)
    }
    // A native `for` over a list, and a real binder beside a placeholder.
    for _ in List(7, 8) {
        add(hits, 3)
    }
    for i in range(0, 3) {
        for _ in range(0, i) {
            add(hits, 4)
        }
    }
    println("${size(hits)}")
}
//= stdout
//= 10

//--- a_qualified_argument_through_an_implicit

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
    println("${total(List(1, 2, 3))}")
    println("${total(List(1, 2, 3))}")
}
//= stdout
//= 6
//= 6

//--- after_the_reachability_narrowing

fn main() [use] -> None {
    use StdConsole()
    let xs: List<Int> = List(1, 2, 3)
    for x in xs {
        println("${x}")
    }
    let total = reduce(iter(xs), 0, (a: Int, b: Int) -> { return a + b })
    println("${total}")
}
//= stdout
//= 1
//= 2
//= 3
//= 6

//--- after_the_reachability_narrowing_2

fn main() [use] -> None {
    use StdConsole()
    for i in range(1, 4) {
        println("${i}")
    }
}
//= stdout
//= 1
//= 2
//= 3

//--- a_read_before_a_mutation

struct Box canbe Mut { n: Int, tag: Str }

fn bumped(b: Mut Box) -> Int => b: Mut {
    b.n = b.n + 1
    return b.n
}

fn label(s: Str, n: Int) -> Str => s, n {
    return "${s}/${n}"
}

fn main() [use] {
    use StdConsole()
    let b = Mut Box { n: 1, tag: "t" }
    // An interpolated place read, then a call that mutates it.
    println("1. ${b.n} ${bumped(b)}")
    // The same across a call's arguments, where the read is a borrow.
    println("2. ${label(b.tag, bumped(b))}")
    // The other order needs nothing: the mutation is over before the read.
    println("3. ${bumped(b)} ${b.n}")
}
//= stdout
//= 1. 1 2
//= 2. t/3
//= 3. 4 4

//--- one_rendering_for_a_ref_result
struct Counter canbe Mut { n: Int }

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
//= stdout
//= 11 4
//= a b a
//= a 111

//--- a_ref_anchored_field_call

struct Fighter canbe Mut { name: Str, hp: Int, energy: Int }

struct Squad canbe Mut {
    banner: Str,
    members: List<Mut Fighter>
}

fn trade(members: List<Mut Fighter>, one: ref(members) Mut Fighter, other: ref(members) Mut Fighter) -> None {
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
    trade(squad.members, at(squad.members, i) ?: unreachable!(), at(squad.members, j) ?: unreachable!())
    // …and the aliasing case `ref(c)` exists for: one fighter, both roles.
    trade(squad.members, at(squad.members, i) ?: unreachable!(), at(squad.members, i) ?: unreachable!())
    println("${(get(squad.members, 0) ?: unreachable!()).energy} ${(get(squad.members, 1) ?: unreachable!()).energy}")
}
//= stdout
//= 3 10

//--- a_user_struct_named_like_stds

struct Range {
    start: Int,
    end: Int
}

fn range(start: Int, end: Int) -> Range {
    return Range { start: start, end: end }
}

iter fn iter(r: Range) -> Emitted Int | Finished {
    state RangeIter {
        i: Int = r.start
    }
    if i >= r.end {
        return finished()
    }
    let out = i.copy()
    i += 1
    return emitted(out)
}

fn main() [use] -> None {
    use StdConsole()
    // `std.core.range` declares a `range(Int, Int)` too, and nothing separates the
    // two by signature, so the call names the place [fn-overload-ambiguous].
    for i in range@main(1, 4) {
        println("${i}")
    }
}
//= stdout
//= 1
//= 2
//= 3

//--- the_total_at

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
            let a = at(xs, i)
            a.hp = a.hp + 5
            bump(at(xs, j))
            hit(xs, at(xs, i), at(xs, i))
            update(xs, i, (e: Mut E) -> { e.hp = e.hp * 2 })
            let p = at(xs, i)
            let q = at(xs, j)
            if q is NotSame(p) {
                merge(p, q)
            }
        }
    }
    println("${(get(xs, i) ?: unreachable!()).hp} ${(get(xs, j) ?: unreachable!()).hp}")
}
//= stdout
//= 45 21

//--- unions_of_borrows

struct A canbe Mut { n: Int }
struct B canbe Mut { s: Str }

fn describe(x: A | B) -> Str => x {
    if x is A {
        return "a${x.n}"
    }
    if x is B {
        return "b${x.s}"
    }
    return "?"
}

fn relay(x: A | B) -> Str => x {
    let y = x
    return "${describe(y)}/${describe(x)}"
}

fn maybe(x: (A | B)?) -> Str => x {
    if x is None {
        return "none"
    }
    if x is A {
        return "A${x.n}"
    }
    return "other"
}

fn bump(x: Mut A | Mut B) -> None => x: None {
    if x is A {
        x.n = x.n + 1
    }
    if x is B {
        x.s = "${x.s}+"
    }
}

fn pick(flag: Bool, xs: List<Mut A>, b: B) -> Str => xs, b {
    let v = if flag { get(xs, 0) ?: unreachable!() } else { b }
    return relay(v)
}

fn main() [use] {
    use StdConsole()
    let xs: List<Mut A> = List(Mut A { n: 1 })
    let b = B { s: "x" }
    println("${pick(true, xs, b)} ${pick(false, xs, b)}")
    let own: A | B = A { n: 7 }
    println("${describe(own)} ${relay(own)} ${maybe(own)} ${maybe(None)} ${maybe(get(xs, 0))} ${maybe(get(xs, 3))}")
    let m: Mut A | Mut B = Mut B { s: "q" }
    bump(m)
    bump(m)
    let h = at(xs, 0) ?: unreachable!()
    let w: Mut A | Mut B = Mut A { n: 0 }
    bump(w)
    println("${describe(m)} ${describe(w)} ${(get(xs, 0) ?: unreachable!()).n}")
    let c = copy(if true { get(xs, 0) ?: unreachable!() } else { b })
    println(describe(c))
    let _ = h
}
//= stdout
//= a1/a1 bx/bx
//= a7 a7/a7 A7 none A1 none
//= bq++ a1 1
//= a1

//--- a_union_interpolates_by_the_text_of_its_arm

struct P { x: Int }
fn to_str(p: P) -> Str { return "P(${p.x})" }
type Either = Int | Str | P

fn widen(x: Int | Str) -> Int | Str | Bool
=> !x { return x }
fn either(n: Int) -> Either {
    if n == 0 { return 5 }
    if n == 1 { return "s" }
    return P { x: n }
}

fn main() [use] {
    use StdConsole()
    let x: Int | Str = 3
    let y = widen(x)
    println("${y}")
    println("${widen("hi")}")
    for k in List(0, 1, 2) { println("e=${either(k)}") }
    let l: List<Int> | Str = List(1, 2)
    println("l=${l}")
}
//= stdout
//= 3
//= hi
//= e=5
//= e=s
//= e=P(2)
//= l=[1, 2]

//--- union_spanning_generic

struct Box2<C> { value: C }

fn detach<T, C>(value: ...T | Box2<C>) -> T | C => !value {
    when value {
        is Box2<C> { return value.value }
        is T { return value }
    }
}

fn show(v: Str | Int | Bool) [Console] -> None {
    when v {
        is Str { println("str ${v}") }
        is Int { println("int ${v}") }
        is Bool { println("bool ${v}") }
    }
}

fn main() [use] {
    use StdConsole()
    let a: Str | Int | Box2<Bool> = "hi"
    let b: Str | Int | Box2<Bool> = 7
    let c: Str | Int | Box2<Bool> = Box2<Bool> { value: true }
    let d: Str | Int | Box2<Bool> = "tf"
    show(detach(a))
    show(detach(b))
    show(detach(c))
    show(detach<Str | Int, Bool>(d))
}
//= stdout
//= str hi
//= int 7
//= bool true
//= str tf

//--- two_generic_union_arms

fn swap<T, S>(v: T | S) -> S | T => !v {
    when v {
        is T { return v }
        is S { return v }
    }
}

fn show(v: Int | Str) [Console] -> None {
    when v {
        is Int { println("int ${v}") }
        is Str { println("str ${v}") }
    }
}

fn main() [use] {
    use StdConsole()
    let a: Str | Int = "hi"
    let b: Int | Str = 7
    show(swap(a))
    show(swap(b))
}
//= stdout
//= str hi
//= int 7

//--- the_update_family

struct Entity canbe Mut { hp: Int }

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 10 }, Mut Entity { hp: 20 })
    let i = 0
    let j = 1
    if i is Idx(es) {
        if j is Idx(es) {
            update(es, i, (e: Mut Entity) -> { e.hp = e.hp + 1 })
            let a = at(es, i)
            let b = at(es, j)
            if b is NotSame(a) {
                a.hp = a.hp + 100
                b.hp = b.hp + 200
            }
            println("${get(es, i).hp} ${get(es, j).hp}")
        }
    }
}
//= stdout
//= 111 220
