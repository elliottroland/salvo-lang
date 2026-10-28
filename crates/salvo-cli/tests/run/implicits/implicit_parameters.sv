// [implicit-param] [implicit-group] implicit parameters
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- auto_members

// Ordering is lexicographic by field declaration order, which is the language's
// rule on both backends.
struct Point : Ordered<self> by auto, Hashed<self> by auto {
    x: Int,
    y: Int
}

// [fn-by] The **function-level** form, with no obligation clause at all: one
// member stamped from `std.core.auto`, declared on the type by being inside it.
struct Tag {
    label: Str

    fn eq(a: Tag, b: Tag) [] -> Bool => a, b by auto
}

// A generic struct is not stamped at yet (ROADMAP §2c): its `eq` is
// written by hand, with the implicit the copy would have needed.
struct Box<T> : Eq<self> {
    item: T
}

fn eq<T>(a: Box<T>, b: Box<T>, ?Eq<T>) [] -> Bool => a, b {
    return eq(a.item, b.item)
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn sign(n: Int) [] -> Str {
    if n < 0 { return "<" }
    if n > 0 { return ">" }
    return "="
}

fn main() [use] {
    use StdConsole()
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 1, y: 9 }
    let r = Point { x: 1, y: 2 }
    println("cmp ${sign(cmp(p, q))}${sign(cmp(p, r))}${sign(cmp(q, p))}")
    println("eq ${eq(p, q)} ${eq(p, r)}")
    // [cmp-hash-values] The agreement with `eq`, never the value.
    println("hash agrees ${eq(hash(p), hash(r))}")
    let tags = eq(Tag { label: "a" }, Tag { label: "a" })
    let boxes = eq(Box<Int> { item: 1 }, Box<Int> { item: 2 })
    println("others ${tags} ${boxes}")
    // The capability through an implicit, filled with the generated canonical.
    let smaller = min_of(p, q)
    println("smaller ${smaller.y}")
}
//= stdout
//= cmp <=>
//= eq false true
//= hash agrees true
//= others true false
//= smaller 2

//--- a_canonical_implementation
// FILE: people.sv

export struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}

// The canonical ordering for `Person`: an ordinary overload that travels with
// the type, exported to match it.
// FILE: main.sv

// Only the *type* is imported; its canonical rides along [fn-attached].
import people.Person

// A second ordering is named for what it does and passed explicitly: a second
// `cmp(Person, Person)` here would be refused, since `cmp` on `Person` is
// declared only in `Person`'s file [op-unique-shape].
fn by_name(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn main() [use] {
    use StdConsole()
    let ada = Person { name: "ada", age: 36 }
    let bob = Person { name: "bob", age: 5 }
    // Both spellings of the selector: at a call, and as a value filling an
    // implicit parameter [implicit-override].
    println("by age ${cmp@Person(ada, bob)}")
    println("by name ${by_name(ada, bob)}")
    let younger = min_of(ada, bob, cmp = cmp@Person)
    println("younger ${younger.name}")
}
//= stdout
//= by age 1
//= by name -1
//= younger bob

//--- a_carried_ordering

// The claim a ranked list carries: the ordering it is ranked by, named in the
// qualifier's own **fn slot**.
qualifier Ranked<T>(?cmp: (T, T) -> Int) of List<T>

// [fn-attached] The obligation names the capability; this file's `cmp` fulfils
// it, so `cmp` is declared on `Person` and travels with it.
struct Person : Ordered<self> { name: Str, age: Int }

fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

// A second ordering a ranked list may be built with instead.
fn by_name(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

// [cmp-binder] Here the binder is an implicit parameter the fn declares, so
// resolution fills it and the result type publishes what it chose.
fn empty_ranked<T>(?cmp: (T, T) -> Int) -> +Ranked<T>(?cmp) Mut List<T> {
    return Mut List()
}

// [cmp-binder] And here it is captured from the argument's type: nothing is
// written but `?cmp`, and the slot it fills states its type. The claim is
// re-minted on the way out, which is what a constructor fn may do today —
// keeping it across a `Mut` parameter is ROADMAP's D2.
fn rank_add<T>(r: Ranked<T>(?cmp) Mut List<T>, elem: T) -> +Ranked<T>(?cmp) Mut List<T> => !r, !elem {
    add(r, elem)
    return r
}

// One body, two answers: it compares with whatever ordering its argument was
// built with.
fn least_index<T>(r: Ranked<T>(?cmp) List<T>) -> Int => r {
    let best = 0
    let i = 1
    while i < size(r) {
        if cmp(get(r, i) ?: unreachable!(), get(r, best) ?: unreachable!()) < 0 {
            best = copy(i)
        }
        i = i + 1
    }
    return best
}

// A body that never needs the identity takes the claim bare.
fn ranked_size<T>(r: Ranked List<T>) -> Int => r {
    return size(r)
}

fn main() [use] {
    use StdConsole()

    // Built with the canonical ordering: the youngest ranks first.
    let by_age = rank_add(
        rank_add(
            rank_add(empty_ranked<Person>(), Person {name: "Ada", age: 36}),
            Person {name: "Bob", age: 24}
        ),
        Person {name: "Cyd", age: 31}
    )
    let youngest = get(by_age, least_index(by_age)) ?: unreachable!()
    println("by age: ${youngest.name} of ${ranked_size(by_age)}")

    // Built with another ordering: the same code, a different answer.
    let alpha = rank_add(
        rank_add(
            rank_add(empty_ranked<Person>(cmp = by_name), Person {name: "Cyd", age: 31}),
            Person {name: "Bob", age: 24}
        ),
        Person {name: "Ada", age: 36}
    )
    let first = get(alpha, least_index(alpha)) ?: unreachable!()
    println("by name: ${first.name} of ${ranked_size(alpha)}")
}
//= stdout
//= by age: Bob of 3
//= by name: Ada of 3

// Under rustc, with the stdout the Kotlin backend asserts byte for byte.
//--- the_comparison_groups

// [cmp-groups] The three capabilities, asked for by name. Nothing about a `T`
// is knowable, so an ordering arrives as an implicit parameter — and the call
// site fills it with the canonical overload for the type it instantiates.
fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn alike<T>(a: T, b: T, ?Eq<T>) [] -> Bool => a, b {
    return eq(a, b)
}

// [cmp-hash-values] A hash value is never printed: it differs between the
// backends by design. What holds on both is that equal values hash equal —
// which is what this answers.
fn digest_agrees<T>(a: T, b: T, ?Eq<T>, ?Hashed<T>) [] -> Bool => a, b {
    if eq(a, b) {
        return hash(a) == hash(b)
    }
    return true
}

fn sign(n: Int) [] -> Str {
    if n < 0 { return "<" }
    if n > 0 { return ">" }
    return "="
}

fn main() [use] {
    use StdConsole()
    let ab = "ab"
    let b = "b"
    let same_ab = "ab"
    // The canonical `cmp` at each intrinsic type, called directly.
    println("int ${sign(cmp(1, 2))}${sign(cmp(2, 2))}${sign(cmp(3, 2))}")
    println("long ${sign(cmp(to_long(9), to_long(4)))}")
    println("byte ${sign(cmp(to_byte(200), to_byte(3)))}")
    println("char ${sign(cmp('a', 'b'))}")
    println("bool ${sign(cmp(false, true))}")
    // `Str` compares by **code point**, which is what the Kotlin backend has
    // to arrange deliberately: "ab" < "b" because 'a' < 'b'.
    println("str ${sign(cmp(ab, b))}${sign(cmp(b, b))}${sign(cmp(b, ab))}")
    // Equality covers the float widths, where no total order exists.
    println("eq ${eq(1, 1)} ${eq(1.5, 2.5)} ${eq(to_float(1.0), to_float(1.0))} ${eq(ab, b)}")
    println("same ${alike(7, 7)} ${alike(ab, b)} ${alike(ab, same_ab)}")
    println("digest ${digest_agrees(ab, same_ab)} ${digest_agrees(3, 3)}")
    // `min_of` answers one of its arguments, so it **moves** them: the
    // deduction is declared `=> !a, !b` [deduce-default], and these two are
    // the last use of their values.
    println("min ${min_of(4, 2)} ${min_of(ab, b)}")
}
//= stdout
//= int <=>
//= long >
//= byte >
//= char <
//= bool <
//= str <=>
//= eq true false true false
//= same true false true
//= digest true true
//= min 2 ab

//--- comptime

struct Point : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto {
    x: Int,
    y: Int
}

struct Manual { note: Str }
struct Imported { feed: Str, n: Int }
type Source = Manual | Imported | None : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto

fn cmp(a: Manual, b: Manual) -> Int => a, b { return cmp(a.note, b.note) }
fn eq(a: Manual, b: Manual) -> Bool => a, b { return eq(a.note, b.note) }
fn hash(v: Manual) -> Long => v { return hash(v.note) }
fn to_str(v: Manual) -> Str => v { return "Manual(${v.note})" }

struct Imported2 : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto { feed: Str, n: Int }
fn cmp(a: Imported, b: Imported) -> Int => a, b { return cmp(a.n, b.n) }
fn eq(a: Imported, b: Imported) -> Bool => a, b { return eq(a.n, b.n) }
fn hash(v: Imported) -> Long => v { return hash(v.n) }
fn to_str(v: Imported) -> Str => v { return "Imported(${v.feed}, ${v.n})" }

struct Reading : Hashed<self> {
    sensor: Str,
    value: Int

    fn hash(r: Reading) -> Long by auto
    fn eq(a: Reading, b: Reading) -> Bool => a, b { return a.sensor == b.sensor }
}

fn main() [use] {
    use StdConsole()
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 1, y: 3 }
    println("${p}")
    println("cmp: ${cmp(p, q)} ${cmp(q, p)} ${cmp(p, p)}")
    println("eq: ${p == q} ${p == Point { x: 1, y: 2 }}")
    println("hash agrees: ${hash(p) == hash(Point { x: 1, y: 2 })}")
    let s: Set<Point> = Set(p, q, Point { x: 1, y: 2 })
    println("set size: ${size(s)}")
    let a: Source = Manual { note: "m" }
    let b: Source = Imported { feed: "f", n: 3 }
    let c: Source = None
    println("${to_str(a)} ${to_str(b)} ${to_str(c)}")
    println("union cmp: ${cmp(a, b)} ${cmp(b, a)} ${cmp(a, a)} ${cmp(c, a)}")
    println("union eq: ${eq(a, b)} ${eq(a, Manual { note: "m" })} ${eq(c, c)}")
    println("union hash: ${hash(a) == hash(Manual { note: "m" })}")
    println("${Imported2 { feed: "x", n: 1 }}")
    println("reading: ${hash(Reading { sensor: "s", value: 1 }) == hash(Reading { sensor: "s", value: 1 })}")
}
//= stdout
//= Point { x: 1, y: 2 }
//= cmp: -1 1 0
//= eq: false true
//= hash agrees: true
//= set size: 2
//= Manual(m) Imported(f, 3) None
//= union cmp: -1 1 0 1
//= union eq: false true true
//= union hash: true
//= Imported2 { feed: x, n: 1 }
//= reading: true

//--- the_elvis_operator

struct Person {
    name: Str,
    nickname: Str? = None
}

fn shout(p: Person) -> Str? {
    let nick: Str = p.nickname ?: return None
    return "${nick}!"
}

fn greet(p: Person) -> Str {
    let nick: Str = p.nickname ?: "friend"
    return "hello ${nick}"
}

fn pick(n: Int) -> Str | Int | None {
    if n == 0 {
        return None
    }
    if n == 1 {
        return "one"
    }
    return n
}

fn main() [use] {
    use StdConsole()
    let named = Person {name: "Ada", nickname: "Addy"}
    let plain = Person {name: "Bob"}
    let s = shout(named)
    if s is Str {
        println("shout ${s}")
    }
    if shout(plain) is None {
        println("shout none")
    }
    println(greet(named))
    println(greet(plain))

    // [op-elvis] `?:` picks the first arm (`Str`); the rest (`Int | None`) go
    // to `_`, which the default here ignores.
    let v: Str | Int | None = pick(2) ?: "none"
    when v {
        is Str { println("str ${v}") }
        is Int { println("int ${v}") }
        is None { println("none") }
    }
    let n: Int? = 5
    if n ?: 0 > 3 {
        println("bigger")
    }
    println("sum ${n ?: 1 + 1}")
    let a: Int? = None
    let b: Int? = 7
    println("chain ${a ?: b ?: 0}")
}
//= stdout
//= shout Addy!
//= shout none
//= hello Addy
//= hello friend
//= str none
//= bigger
//= sum 5
//= chain 7

//--- escapes_in_expression_position

fn twice(n: Int) -> Int {
    return n * 2
}

fn guarded(n: Int) -> Int {
    if n < 0 {
        return twice(return 0)
    }
    return n
}

fn first_big(limit: Int) -> Int {
    let total: Int = 0
    let i: Int = 0
    while true {
        let step: Int = if i < limit { i } else { break }
        total = total + step
        i = i + 1
    }
    return total
}

fn main() [use] {
    use StdConsole()
    println("guarded(-1) ${guarded(-1)}")
    println("guarded(7) ${guarded(7)}")
    println("first_big(4) ${first_big(4)}")
}
//= stdout
//= guarded(-1) 0
//= guarded(7) 7
//= first_big(4) 6

// Under rustc, with the stdout the Kotlin backend asserts byte for byte.
//--- a_composed_pass_with_a_stored_callback

struct Countdown : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

struct Doubling : Next<self, Int> canbe Mut {
    src: Mut Countdown,
    f: (Int) -> Int
}

fn next(d: Mut Doubling) -> Emitted Int | Finished => d: Mut {
    let step = next(d.src)
    when step {
        is Emitted {
            let g = d.f
            return emitted(g(step))
        }
        is Finished {
            return finished()
        }
    }
}

fn twice(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdConsole()
    let doubled = Mut Doubling { src: Mut Countdown { at: 3 }, f: twice }
    for n in doubled {
        println("n ${n}")
    }
    println("done")
}
//= stdout
//= n 6
//= n 4
//= n 2
//= done

// [comptime-generic] [implicit-recursive] A generic struct stamped `by auto`
// keys a `Set`: the stamp's `?Hashed<T>` is filled at each instantiation
// (found 2026-10-20; the slot fit used to refuse it). The Kotlin suite runs
// the same program (`generic-auto-key`).
//--- compiles_and_runs_a_generic_auto_key

struct W<T> : Hashed<self> by auto { v: T }

fn main() [use] {
    use StdConsole()
    let s: Mut Set<W<Int>> = Mut Set()
    add(s, W { v: 1 })
    add(s, W { v: 1 })
    add(s, W { v: 2 })
    let t = Mut Set(W { v: "a" }, W { v: "b" }, W { v: "a" })
    let u = Mut Set<(Int, Str)>()
    add(u, (1, "a"))
    add(u, (1, "a"))
    println("${size(s)} ${size(t)} ${contains(t, W { v: "b" })} ${contains(s, W { v: 3 })} ${size(u)}")
}
//= stdout
//= 2 2 true false 1

// [rs-wire] [net-wire-format] A generic struct's `T?` field round-trips, with the
// bytes the Kotlin suite's `generic-opt-codec` case asserts too (the Kotlin
// codec did not compile until 2026-10-20).
//--- compiles_and_runs_a_generic_optional_codec
import std.codec

struct Arg<T> {
    default: T? = None
    n: Int? = None
}

fn main() [use] {
    use StdConsole()
    let a = encode(Arg<Int> { default: 3 })
    println(to_hex(a))
    let b = decode<Arg<Int>>(a)
    if b is None { println("none") } else { println("${b.default ?: 0}") }
    let c = encode(Arg<Str> { n: 4 })
    println(to_hex(c))
    let d = decode<Arg<Str>>(c)
    if d is None { println("none") } else { println("${d.n ?: 0} ${d.default ?: "-"}") }
}
//= stdout
//= 010000000300
//= 3
//= 000100000004
//= 4 -

// [rs-collections] A generic function over a **hash** container that actually
// hashes. This did not compile before the keyed containers moved behind a store
// (2026-09-22): the emitter writes `<T: Clone>` while the runtime's hashing
// operations needed `T: Hash + Eq`, so `contains` inside generic code was a
// rustc error in emitted output — a program the checker accepted and the backend
// could not build. Every operation is bound-free now, which removed the
// requirement rather than adding one. Since ROADMAP §0j step 7 a generic fn
// that looks an element up captures the set's identity from its type
// (`Set<T>(?hash, ?eq)`), as a fn over a `Heap` captures its `?cmp`.
//--- a_generic_over_a_hash_container

fn has<T>(s: Set<T>(?hash, ?eq), e: T) -> Bool => s, e {
    return contains(s, e)
}

fn tally<K, V>(m: Map<K, V>) -> Int => m {
    return size(m)
}

fn main() [use] {
    use StdConsole()
    let seen = Set(1, 2, 3)
    let ages: Map<Str, Int> = Map("ada": 36, "bob": 24)
    println("has ${has(seen, 2)} ${has(seen, 9)} of ${tally(ages)}")
    println("map ${to_str(ages)}")
}
//= stdout
//= has true false of 2
//= map {ada: 36, bob: 24}

//--- a_hand_written_pass

struct Zip : Next<self, (Str, Int)> canbe Mut {
    left: List<Str>,
    right: List<Int>,
    at: Int
}

fn next(z: Mut Zip) -> Emitted (Str, Int) | Finished => z: Mut {
    let l = get(z.left, z.at)
    let r = get(z.right, z.at)
    if l is Str && r is Int {
        z.at = z.at + 1
        return emitted((copy(l), r))
    }
    return finished()
}

fn zip(left: List<Str>, right: List<Int>) -> Zip => !left, !right {
    return Zip { left: left, right: right, at: 0 }
}

struct Countdown : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn countdown(from: Int) -> Countdown {
    return Countdown { at: from }
}

fn main() [use] {
    use StdConsole()
    for n in countdown(3) {
        println("n ${n}")
    }
    let names = List("ada", "grace", "alan")
    let ages = List(36, 45)
    for pair in zip(names, ages) {
        println("${pair.0} is ${pair.1}")
    }
    println("done")
}
//= stdout
//= n 3
//= n 2
//= n 1
//= ada is 36
//= grace is 45
//= done

// Under rustc, with the stdout the Kotlin backend asserts byte for byte.
//--- implicit_parameters

fns Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn add(a: Int, b: Int) -> Int { return a + b }
fn zero() -> Int { return 0 }
// [op-unique-shape] Not `times`: that is the `*` operator on `Int`, declared
// only in `Int`'s own file; an override is named for what it does.
fn multiply(a: Int, b: Int) -> Int { return a * b }
fn one() -> Int { return 1 }

fn total<T>(xs: List<T>, ?Field<T>) -> T => xs {
    let acc = zero()
    for x in xs {
        acc = add(acc, x)
    }
    return acc
}

fn total_all<T>(rows: List<List<T>>, ?Field<T>) -> T => rows {
    let acc = zero()
    for row in rows {
        acc = add(acc, total(row))
    }
    return acc
}

fn sum_pair<T>(a: T, b: T, ?add: (T, T) -> T) -> T {
    return add(a, b)
}

fn main() [use] {
    use StdConsole()
    println("total=${total(List(1, 2, 3))}")
    println("product=${total(List(2, 3, 4), add = multiply, zero = one)}")
    // [col-of-nonempty] Literals rather than constructor calls: the element
    // constructors claim `NonEmpty`, and a `List<NonEmpty List<Int>>` does not
    // fit a `List<List<Int>>` position (type arguments are invariant). A literal
    // claims nothing, which is what a nested plain list needs.
    println("nested=${total_all(List<List<Int>>(List(1, 2), List(3)))}")
    println("pair=${sum_pair(20, 22)}")
    println("lambda=${sum_pair(2, 3, add = (a: Int, b: Int) -> a * b)}")
}
//= stdout
//= total=6
//= product=24
//= nested=6
//= pair=42
//= lambda=6

//--- a_keyed_hash_pair

struct Person : Hashed<self> by auto { name: Str, age: Int }

// A pair that keys by age alone: two people of an age are one member.
fn age_hash(p: Person) -> Long => p {
    return to_long(p.age)
}

fn same_age(a: Person, b: Person) -> Bool => a, b {
    return a.age == b.age
}

// One body, either keying: the pair is the container's, not the caller's.
fn tally<T>(s: Set<T>) -> Int => s {
    return size(s)
}

fn main() [use] {
    use StdConsole()
    let all: Mut Set<Person> = Mut Set()
    // [cmp-binder] The pair is **passed**, never inferred from the annotation:
    // what fills the binder decides what the container is keyed by (user
    // decision 2026-09-26 — one way to choose an identity).
    let byage: Mut Set<Person>(age_hash, same_age) = Mut Set(hash = age_hash, eq = same_age)
    let bob = Person {name: "Bob", age: 24}
    add(all, copy(bob))
    add(byage, bob)
    let eve = Person {name: "Eve", age: 24}
    let fresh = add(all, copy(eve))
    let twin = add(byage, eve)
    println("added ${fresh} ${twin}")
    println("sizes ${tally(all)} ${tally(byage)}")
}
//= stdout
//= added true false
//= sizes 2 1

//--- a_keyed_container_ordering

struct Person : Ordered<self> by auto, Eq<self> by auto { name: Str, age: Int }

fn by_age(a: Person, b: Person) -> Int => a, b {
    return cmp(a.age, b.age)
}

// One body, either ordering: the identity is the container's, not the caller's.
fn lowest<T>(s: SortedSet<T>) -> T? => s {
    return min(s)
}

fn main() [use] {
    use StdConsole()
    let byname = SortedSet(
        Person {name: "Cyd", age: 31},
        Person {name: "Ada", age: 36},
        Person {name: "Bob", age: 24}
    )
    // [cmp-binder] The ordering is **passed**: what fills the binder decides
    // what the container carries (user decision 2026-09-26).
    let byage: SortedSet<Person>(by_age) = SortedSet(
        Person {name: "Cyd", age: 31},
        Person {name: "Ada", age: 36},
        Person {name: "Bob", age: 24},
        cmp = by_age
    )
    println("byname ${(lowest(byname) ?: unreachable!()).name} of ${size(byname)}")
    println("byage ${(lowest(byage) ?: unreachable!()).name} of ${size(byage)}")

    // `cmp`-distinct membership: a second 24-year-old is the same member under
    // `by_age`, and its own member under the canonical ordering.
    let twin = Person {name: "Eve", age: 24}
    let grown_age: Mut SortedSet<Person>(by_age) = Mut SortedSet(cmp = by_age)
    let grown_name: Mut SortedSet<Person> = Mut SortedSet()
    add(grown_age, Person {name: "Bob", age: 24})
    add(grown_name, Person {name: "Bob", age: 24})
    println("twin ${add(grown_age, copy(twin))} ${add(grown_name, twin)}")
}
//= stdout
//= byname Ada of 3
//= byage Bob of 3
//= twin false true

//--- a_lift_binding

fn classify(n: Int) -> Ok Int | Err Str {
    if n == 0 {
        return err("zero")
    }
    return ok(n)
}

fn describe(n: Int) [Console] -> None {
    let outcome = classify(n)
    if outcome is ^Ok value {
        println("ok ${value}")
    }
    if outcome is Err {
        println("err ${outcome}")
    }
}

fn read_only(list: Mut List<Int>) [Console] -> None => list: Mut {
    if list is ^Mut plain {
        println("size ${size(plain)}")
    }
}

fn main() [use] {
    use StdConsole()
    describe(7)
    describe(0)
    read_only(Mut List(1, 2, 3))
}
//= stdout
//= ok 7
//= err zero
//= size 3

//--- effect_member_implicits

effect Show {
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v
}

handler Angle of Show {
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v {
        return "<${fmt(v)}>"
    }
}

fn fmt(n: Int) -> Str => n {
    return "n=${n}"
}

fn loud(n: Int) -> Str => n {
    return "N=${n}!"
}

fn main() [use] {
    use StdConsole()
    use Angle()
    println(show(7))
    println(show(7, fmt = loud))
}
//= stdout
//= <n=7>
//= <N=7!>

//--- mutation_through_narrowed_places

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
    if p is Mut ListNext<Int> {
        show(next(p))
        show(next(p))
        show(next(p))
    }
    let qs = List(7, 8)
    let q: Mut ListNext<Int> | Int = iter(qs)
    if q is Mut ListNext<Int> {
        show(next(q))
        show(next(q))
    }
    let rs = List(1, 2, 3)
    let r: Mut ListNext<Int>? = iter(rs)
    if r is Mut ListNext<Int> {
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
//= stdout
//= got 1
//= got 2
//= end
//= got 7
//= got 8
//= got 3
//= n 1
//= n 2
//= n 3
//= n 4
//= n 5

//--- mutation_through_a_narrowed_mut_arm

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
    if a is Mut { add(a, 9) }
    if a is Mut { println("var ${to_str(a)}") }

    let b = Mut Shelf { items: Mut List(1) }
    if b.items is Mut { add(b.items, 9) }
    if b.items is Mut { println("field ${to_str(b.items)}") }

    let c: Ok Mut List<Int> | Err Str = ok(Mut List(1))
    if c is ^Ok { add(c, 9) }
    if c is ^Ok { println("widen ${to_str(c)}") }

    let d: Ok Mut List<Int> | Err Str = ok(Mut List(1))
    when d { is Ok { add(d, 9) } is Err { } }
    if d is ^Ok { println("when ${to_str(d)}") }

    let e: Ok Mut List<Int> | Err Str | None = ok(Mut List(1))
    if e is ^Ok { add(e, 9) }
    if e is ^Ok { println("nullable ${to_str(e)}") }

    println("moved ${eat(ok(Mut List(1)))}")
    stash(5)
    println("state ${dump()}")
}
//= stdout
//= var [1, 9]
//= field [1, 9]
//= widen [1, 9]
//= when [1, 9]
//= nullable [1, 9]
//= moved [1, 7]
//= state [0, 5]

//--- a_combinator_over_a_next_spread

struct Slice<T> : Next<self, proj T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> Mut Slice<T> => items {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> Emitted (proj(p) T) | Finished => p: Mut {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(e)
}

fn map2<It, T, U>(it: Mut It, mapper: (T) -> U, ?Next<It, T>) -> Mut List<U> => it: Mut, mapper {
    let out = Mut List<U>()
    let going = true
    while going {
        let step = next(it)
        when step {
            is Emitted {
                add(out, mapper(step))
            }
            is Finished {
                going = false
            }
        }
    }
    return out
}

fn double(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
    let doubled = map2(slice(xs), double)
    for d in doubled {
        println("d ${d}")
    }
}
//= stdout
//= d 2
//= d 4
//= d 6

//--- operators_through_the_groups

struct Point : Ordered<self> by auto, Eq<self> by auto {
    x: Int,
    y: Int
}

// A hand-written canonical, which the operators reach exactly as they reach a
// generated one [fn-attached].
struct Age {
    years: Int,
    label: Str

    fn eq(a: Age, b: Age) [] -> Bool => a, b {
        return eq(a.years, b.years)
    }

    fn cmp(a: Age, b: Age) [] -> Int => a, b {
        return cmp(a.years, b.years)
    }
}

// [implicit-forward] Generic code publishes the capability in its signature,
// and the operators inside go through the parameter.
fn larger_of<T>(a: T, b: T, ?Ordered<T>) [] -> T => !a, !b {
    if a > b {
        return a
    }
    return b
}

fn same_of<T>(a: T, b: T, ?Eq<T>) [] -> Bool => a, b {
    return a == b
}

fn main() [use] {
    use StdConsole()
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 1, y: 9 }
    println("struct ${p < q} ${p == q} ${p != q} ${q >= p}")
    let young = Age { years: 5, label: "bob" }
    let old = Age { years: 36, label: "ada" }
    println("by hand ${young < old} ${young == old}")
    // `Str` ordering: "ab" < "b" by code point, on both backends.
    let ab = "ab"
    let b = "b"
    println("strs ${ab < b} ${ab == b} ${same_of(ab, b)}")
    println("generic ${larger_of(3, 9)} ${larger_of(p, q).y} ${larger_of(young, old).label}")
    // [type-basic] The basic types compare with the host's operator, `Bool`
    // and `Char` included.
    println("basic ${true > false} ${false >= true} ${'a' < 'b'} ${3 != 4} ${2.5 == 1.5}")
}
//= stdout
//= struct true false true true
//= by hand true false
//= strs true false false
//= generic 9 9 ada
//= basic true false true true false

// [rs-narrow-mut] [rs-opt-borrow] A union-typed parameter whose arm is `Mut`
// is lent mutably, arm by arm (`Union2<&mut Vec<i32>, &String>`), so peeling
// the `Mut` arm out of it and mutating reaches the caller's value — as on
// Kotlin. (Until 2026-10-10 the frame received it read-only, and this was a
// reported error.)
//--- peeling_a_mut_arm_out_of_a_borrowed_parameter

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
//= stdout
//= 3

//--- a_positional_pick

fn parse(text: Str) -> Ok Int | Err Str {
    if size(text) == 0 {
        return err("empty")
    }
    return ok(size(text))
}

fn doubled(text: Str) -> Ok Int | Err Str {
    let got: Ok Int = parse(text) ?: return _
    if got is ^Ok n {
        return ok(n * 2)
    }
    return got
}

fn show(label: Str, r: Ok Int | Err Str) [Console] -> None {
    when r {
        is Ok { println("${label} ok ${r}") }
        is Err { println("${label} err ${r}") }
    }
}

// [op-elvis-guard] After a guarding pick the subject reads as the matched *arm*:
// `r` is an `Ok Int` below, so it needs no further check.
fn guarded(text: Str) [Console] -> None {
    let r = parse(text)
    let n: Ok Int = r ?: return
    let same: Ok Int = r
    println("guarded ${n} ${same}")
}

fn main() [use] {
    use StdConsole()
    show("doubled", doubled("abc"))
    show("doubled", doubled(""))
    guarded("hello")
    guarded("")
}
//= stdout
//= doubled ok 6
//= doubled err empty
//= guarded 5 5

//--- the_pipe_operator

fn double(n: Int) -> Int {
    return n * 2
}

fn add(a: Int, b: Int) -> Int {
    return a + b
}

fn main() [use] {
    use StdConsole()
    double(21) |> println("single ${_}")
    let chained = 5
        |> double(_)
        |> add(_, 100)
    println("chained ${chained}")
    let prec = 2 + 3 |> double(_)
    println("prec ${prec}")
}
//= stdout
//= single 42
//= chained 110
//= prec 10

//--- a_group_over_plain_arms

fn step(n: Int) -> Emitted (Str | Int) | Finished {
    if n == 1 {
        return emitted("one")
    }
    if n == 2 {
        return emitted(2)
    }
    return finished()
}

fn show(n: Int) [Console] {
    let r = step(n)
    when r {
        is ^Emitted {
            when r {
                is Str { println("str ${r}") }
                is Int { println("int ${r}") }
            }
        }
        is Finished { println("finished") }
    }
}

fn main() [use] {
    use StdConsole()
    show(1)
    show(2)
    show(3)
}
//= stdout
//= str one
//= int 2
//= finished

//--- a_released_raw_pass

linear struct Ticks : Next<self, Int> canbe Mut {
    at: Int
}

fn next(l: Mut Ticks) -> Emitted Int | Finished => l: Mut {
    if l.at <= 0 {
        return finished()
    }
    let v = copy(l.at)
    l.at = l.at - 1
    return emitted(v)
}

fn close(l: Ticks) [Console] -> None => !l {
    discard(l)
    println("closed")
}

fn drained() [Console] -> None {
    let lines = Mut Ticks { at: 2 }
    for n in lines {
        println("n ${n}")
    }
    close(lines)
    println("after drain")
}

fn abandoned() [Console] -> None {
    let lines = Mut Ticks { at: 5 }
    for n in lines {
        println("m ${n}")
        break
    }
    close(lines)
    println("after break")
}

fn main() [use] {
    use StdConsole()
    drained()
    abandoned()
}
//= stdout
//= n 2
//= n 1
//= closed
//= after drain
//= m 5
//= closed
//= after break

//--- a_sub_union_rewrap

import std.throw

fn classify(n: Int) -> Ok Int | Ok Str | Err Str {
    if n == 0 {
        return err("zero")
    }
    if n > 0 {
        return ok(n)
    }
    return ok("negative")
}

fn attempt(text: Str) -> Ok Int | Err Str | Thrown Str {
    if size(text) == 0 {
        return err("empty")
    }
    if size(text) > 5 {
        return thrown("too long")
    }
    return ok(size(text))
}

fn show(n: Int) [Console] -> None {
    let o = classify(n)
    if o is ^Ok value {
        when value {
            is Int { println("int ${value}") }
            is Str { println("str ${value}") }
        }
    }
    if o is Err {
        println("err ${o}")
    }
}

fn run(text: Str) -> Err Str | Thrown Str {
    let got: Ok Int = attempt(text) ?: return _
    if got is ^Ok n {
        return err("len ${n}")
    }
    return err("unreachable")
}

fn report(text: Str) [Console] -> None {
    let r = run(text)
    when r {
        is Err { println("err ${r}") }
        is Thrown { println("thrown ${r}") }
    }
}

fn main() [use] {
    use StdConsole()
    show(3)
    show(-1)
    show(0)
    report("abc")
    report("")
    report("abcdefgh")
}
//= stdout
//= int 3
//= str negative
//= err zero
//= err len 3
//= err empty
//= thrown too long

//--- the_safe_pipe_operator

fn maybe(present: Bool) -> Int? {
    if present {
        return 42
    }
    return None
}

fn double(x: Int) -> Int {
    return x * 2
}

fn half_if_even(x: Int) -> Int? {
    if x % 2 == 0 {
        return x / 2
    }
    return None
}

fn show(label: Str, v: Int?) [Console] -> None {
    when v {
        is Int { println("${label} present ${v}") }
        is None { println("${label} absent") }
    }
}

fn main() [use] {
    use StdConsole()
    show("present", maybe(true) ?> double(_))
    show("absent", maybe(false) ?> double(_))
    show("flat", maybe(true) ?> half_if_even(_))
    show("flat-none", maybe(true) ?> half_if_even(_ + 1))
    show("chain", maybe(true) ?> double(_) ?> double(_))
}
//= stdout
//= present present 84
//= absent absent
//= flat present 21
//= flat-none absent
//= chain present 168

// [cmp-carry] [cmp-binder] An identity is a **declaration**: two modules'
// `my_hash` key two sets, and a binder filled from one calls that module's —
// a private one, from a file that imports neither (found 2026-10-20). The
// Kotlin suite runs the same program (`two-identities`).
//--- compiles_and_runs_two_modules_same_named_identities
// FILE: p.sv
export struct P : Hashed<self> by auto { x: Int }
// FILE: a.sv
import p.P
fn my_hash(v: P) -> Long { return hash(v.x) }
fn my_eq(l: P, r: P) -> Bool { return l.x == r.x }
export fn make_a() -> Mut Set<P>(my_hash, my_eq) {
    return Mut Set<P>(hash = my_hash, eq = my_eq)
}
// FILE: b.sv
import p.P
export fn my_hash(v: P) -> Long { return 7 }
export fn my_eq(l: P, r: P) -> Bool { return true }
export fn make_b() -> Mut Set<P>(my_hash, my_eq) {
    return Mut Set<P>(hash = my_hash, eq = my_eq)
}
// FILE: main.sv
import p.P
import a.make_a
import b.make_b
fn one(x: Mut Set<P>(?hash, ?eq), v: P) -> Bool => x, v { return contains(x, v) }
fn main() [use] {
    use StdConsole()
    let x = make_a()
    let y = make_b()
    add(x, P { x: 1 })
    add(y, P { x: 1 })
    add(y, P { x: 2 })
    println("${one(x, P { x: 2 })} ${one(y, P { x: 5 })} ${size(y)}")
}
//= stdout
//= false true 1
