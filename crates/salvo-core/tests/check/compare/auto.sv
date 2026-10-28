// [cmp-auto] [obligation-by] The generated structural implementations:
// `: Ordered<self> by auto` stamps `std.core.auto`'s `comptime fn`s at the
// type, and `fn cmp(…) by auto` [fn-by] stamps one member.

// `: Ordered<self> by auto` writes the members: `cmp` resolves at the type,
// fills a `?Ordered<T>` position, and is reached by the canonical selector
// like a hand-written one [fn-attached].
struct Point : Ordered<self> by auto, Hashed<self> by auto {
    x: Int,
    y: Int
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn go(p: Point, q: Point) [] -> Bool => p, q {
    let order = cmp(p, q)
    let _same = eq(p, q)
    let _digest = hash(p)
    let selected = cmp@Point(p, q)
    return eq(order, selected)
}

// `min_of` answers one of its arguments, so it **moves** them [deduce-infer]:
// filling its `?cmp` with the canonical is the point here.
fn smaller(p: Point, q: Point) [] -> Point => !p, !q {
    return min_of(p, q, cmp = cmp@Point)
}

// [cmp-auto] **`Hashed<self> by auto` brings `eq` with it, and `Ordered<self>
// by auto` does not** (user decision 2026-09-22, revising decision 7): a hash
// container buckets by `hash` and confirms by `eq`, so the pair is the unit —
// while no sorted container consults equality at all.
struct Key : Hashed<self> by auto {
    x: Int
}

fn same_key(a: Key, b: Key) [] -> Bool => a, b {
    return eq(a, b)
}

// `Ordered<self> by auto` generates only `cmp`, so equality is a separate ask.
struct OrderedOnly : Ordered<self> by auto {
    x: Int
}

fn same_ordered(a: OrderedOnly, b: OrderedOnly) [] -> Bool => a, b {
    //~ error[call-resolve:no-overload]
    return eq(a, b)
}

// …and asking for it is one line, which is what the function-level form is
// for: some members generated, others not.
struct FnForm : Ordered<self>, Eq<self> {
    x: Int

    fn eq(a: FnForm, b: FnForm) [] -> Bool => a, b by auto

    fn cmp(a: FnForm, b: FnForm) [] -> Int => a, b by auto
}

fn same_fn_form(a: FnForm, b: FnForm) [] -> Bool => a, b {
    return eq(a, b) && cmp(a, b) == 0
}

// (`by auto` on a group with no generator, a group argument that is not
// `self`, and the fields a stamp validates are `compare_tests.rs`'s: each
// reports two diagnostics for its one mistake.)

// [cmp-auto] The **function-level** form, and the reason it exists (user
// decision 2026-09-22): a type generates the members it wants generated and
// writes the one it wants written. Here `Person` is ordered by name, hashed
// structurally, and equal exactly when it ties — which no single obligation
// clause can say.
struct Person : Ordered<self>, Hashed<self> {
    name: Str,
    age: Int

    fn eq(a: Person, b: Person) [] -> Bool => a, b {
        return cmp(a, b) == 0
    }

    fn hash(value: Person) [] -> Long => value by auto

    fn cmp(a: Person, b: Person) [] -> Int => a, b by auto
}

fn probe_person(a: Person, b: Person) [] -> Bool => a, b {
    return a == b && a < b && hash(a) == hash(b)
}

// [cmp-auto] `Group<self> by auto` is sugar for exactly those declarations, so
// the two spellings are interchangeable.
struct A : Hashed<self> by auto {
    x: Int
}

struct B {
    x: Int

    fn eq(a: B, b: B) [] -> Bool => a, b by auto

    fn hash(value: B) [] -> Long => value by auto
}

fn probe_ab(p: A, q: A, r: B, s: B) [] -> Bool => p, q, r, s {
    return eq(p, q) && eq(r, s) && hash(p) == hash(r)
}

// [fn-by] What a `by` declaration must be: stamped at a struct or a declared
// union (the first parameter's type), from a scope holding a `comptime fn` of
// that name, bodiless, and of the instantiation's own shape. Each refusal
// names the remedy rather than leaving a body unwritten. (Not at a struct: the
// `by auto` program below.)
//
// A member `auto` has no comptime fn for.
struct Described {
    x: Int

    //~ error[obligation-by:module-lacks-member]
    fn describe(p: Described) [] -> Str => p by auto
}

// The wrong shape for the comptime fn it names: its arity, and its return type.
struct Shaped {
    x: Int

    //~ error[fn-by:param-count]
    fn hash(a: Shaped, b: Shaped) [] -> Long => a, b by auto
}

struct Returned {
    x: Int

    //~ error[fn-by:return-type]
    fn cmp(a: Returned, b: Returned) [] -> Bool => a, b by auto
}

// [comptime-generic] A generic struct **is** stamped at (ROADMAP §0j 6i): the
// stamped `eq` is generic and asks for `?Eq<T>`, since a field of type `T`
// has nothing to compare it by but what the caller passes. A concrete field
// beside it still reaches its own `eq` — the implicit joins resolution
// rather than shadowing [implicit-resolve-body]. Calls and operators fill
// the stamp's implicits.
struct Box<T> : Eq<self> by auto, Hashed<self> by auto {
    item: T,
    n: Int
}

fn same_box(a: Box<Str>, b: Box<Str>) [] -> Bool => a, b {
    return eq(a, b) && a == b && hash(a) == hash(b)
}

// [comptime-generic] [implicit-recursive] A generic stamp **fills an implicit
// position** like std's `hash(List<T>, ?hash)` does: its `?Hashed<T>` spread
// is filled in turn, so `?hash: (W<Int>) -> Long` takes it.
struct W<T> : Hashed<self> by auto {
    v: T
}

fn digest(x: W<Int>, ?hash: (v: W<Int>) -> Long, ?eq: (a: W<Int>, b: W<Int>) -> Bool) [] -> Long => x {
    return hash(x)
}

fn probe_digest() [] -> Long {
    return digest(W { v: 1 })
}

// The obligation is still checked at the struct [group-obligation]: `by auto`
// satisfies it by generating, and a bare `Eq<self>` with nothing to satisfy it
// still fails — the two spellings answer the same question.
struct Generated : Eq<self> by auto {
    x: Int
}

//~ error[group-obligation:no-member]
struct Bare : Eq<self> {
    x: Int
}

//--- a by fn not at a struct
// [fn-by] Not at a struct or union: there are no fields or arms to walk.
// (`hash`, not `cmp`: a `cmp` on `Int` outside `Int`'s file is refused for its
// placement first [op-unique-shape].) Its own program: a `hash(Int)` beside
// std's makes every other `hash` at `Int` ambiguous.
//~ error[fn-by:target-not-struct-or-union]
fn hash(value: Int) [] -> Long => value by auto

// The same, at a type with no fields to read: an intrinsic is not a target.
//~ error[fn-by:target-not-struct-or-union]
fn hash(value: Str) [] -> Long => value by auto

//--- a stamp passed by name
// [comptime-generic] [fn-value] Passing a stamp by name fills its implicits
// exactly as an implicit is filled (refused until 2026-10-20, when the two
// rules were made one).
struct Box<T> : Eq<self> by auto {
    item: T
}

fn takes(f: (Box<Str>, Box<Str>) -> Bool) [] -> Bool => f {
    return true
}

fn pass() [] -> Bool {
    return takes(eq)
}

//--- the clause and the fn form on one member
// [cmp-auto] Mixing the two spellings on one member is the ordinary duplicate.
//~ error[obligation-by:stamped-and-written]
struct A : Hashed<self> by auto {
    x: Int

    fn hash(value: A) [] -> Long => value by auto
}

//--- a hand-written member beside a generated one
// A hand-written member of the same shape beside a generated one is the
// ordinary duplicate, with the message naming the half to delete.
//~ error[obligation-by:stamped-and-written]
struct Point : Eq<self> by auto {
    x: Int

    fn eq(a: Point, b: Point) [] -> Bool => a, b {
        return eq(a.x, b.x)
    }
}

//--- a by fn with a body
// [fn-by] A body, which the stamping was going to write: a parse error.
struct Point {
    x: Int

    //~ error[fn-by:body]
    fn cmp(a: Point, b: Point) [] -> Int => a, b by auto {
        return 0
    }
}

//--- canbe hashed is gone
// [cmp-auto] `canbe hashed` and `canbe ordered` are **gone**, and the
// diagnostic names what replaced them rather than reporting an unknown opt-in.
//~ error[obligation-by:canbe-hashed]
struct Point canbe hashed {
    x: Int
}

//--- canbe ordered is gone
//~ error[obligation-by:canbe-hashed]
struct Point canbe ordered {
    x: Int
}

