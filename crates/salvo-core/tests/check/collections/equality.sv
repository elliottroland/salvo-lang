// [col-equality] [col-hashed-ordered] [op-equality] [op-order] Equality,
// ordering and the key opt-ins: a struct compares, orders or hashes by having
// an `eq`, a `cmp` or a `hash` — hand-written, or stamped by `by auto`.

// [op-equality] Equality **ignores qualifiers** — it is about the data at the
// moment of the check, so `Tagged Point == Point` resolves the same
// `eq(Point, Point)`. (Since 2026-09-21 the struct has to *have* one:
// equality is opt-in, and `Eq<self> by auto` is the one-token way to ask.)
qualifier Tagged of Point

struct Point : Eq<self> by auto {
    x: Int,
    y: Int
}

fn tagged(p: Point) -> +Tagged Point
=> !p {
    return p
}

fn equality_ignores_qualifiers(a: Point, b: Point) -> Bool => !a, b {
    let t = tagged(a)
    return t == b
}

// [op-equality] …and a struct with no `eq` cannot be compared at all, which
// is what "equality is opt-in" means (user decision 2026-09-21, overturning
// [col-equality]'s every-struct rule). The diagnostic names both ways to get
// one.
struct Plain {
    x: Int
}

fn no_eq(a: Plain, b: Plain) -> Bool => a, b {
    //~ error[call-resolve:no-overload]
    return a == b
}

// [col-equality] [op-equality] Comparing two *different* struct types with
// no `eq` taking the pair is an error, not a constant `false`: the operator
// is the call `eq(A, B)`, which nothing declares (R10 D6 dropped the
// same-type rule; the call's refusal is what remains).
struct A {
    v: Int
}

struct B {
    v: Int
}

fn different_types(a: A, b: B) -> Bool => a, b {
    //~ error[call-resolve:no-overload]
    return a == b
}

// [obligation-by] [comptime-instantiate] A fn-typed field bars the stamped
// `eq`: the copy for that field finds no `eq` over a function value, and the
// error lands at the field, prefixed with what was being stamped. It does not
// bar the *struct* from equality: declare an `eq` that ignores the field, and
// the type is comparable (and hashable, with a `hash` to match).
struct FnHolder : Eq<self> by auto {
    //~ error[call-resolve:no-overload]
    f: (Int) -> Int
}

// [fn-attached] Declared *inside* the body: the hand-written `eq` is on the
// type, and ignores the field the structural one could not compare.
struct Holder {
    f: (Int) -> Int,
    tag: Int

    fn eq(a: Holder, b: Holder) [] -> Bool => a, b {
        return eq(a.tag, b.tag)
    }
}

fn compares_by_hand(a: Holder, b: Holder) -> Bool => a, b {
    return a == b
}

// [op-order] Ordering needs a `cmp` — generated or hand-written — and says so
// when there is none.
struct Ord1 : Ordered<self> by auto {
    x: Int
}

fn generated_cmp(a: Ord1, b: Ord1) -> Bool => a, b {
    return a < b
}

// [fn-attached] Hand-written, declared inside the body.
struct Ord2 {
    x: Int

    fn cmp(a: Ord2, b: Ord2) [] -> Int => a, b {
        return cmp(a.x, b.x)
    }
}

fn hand_written_cmp(a: Ord2, b: Ord2) -> Bool => a, b {
    return a < b
}

// And with neither, the diagnostic names both ways to declare one.
struct Ord3 {
    x: Int
}

fn no_cmp(a: Ord3, b: Ord3) -> Bool => a, b {
    //~ error[call-resolve:no-overload]
    return a < b
}

// [obligation-by] [comptime-refuse] The `by auto` clauses are validated where
// they are written: `std.core.auto`'s `hash` **refuses** a `canbe Mut` struct
// (a key that can change under the collection holding it), and every field
// has to resolve its own `hash`/`cmp` — the rules `canbe hashed`/`canbe
// ordered` used to carry, now stated in std rather than in the compiler.
//
// A `canbe Mut` struct could change under the collection holding it. The
// refusal is one error; the obligation left unfulfilled by it is another, at
// the clause.
//~ error[group-obligation:no-member]
//~ error[comptime-refuse:refused]
struct KMut : Hashed<self> by auto canbe Mut {
    v: Int
}

// A float field has no `hash` (by neither backend) — reported at the field,
// in the stamped copy's terms.
struct KFloatHash : Hashed<self> by auto {
    //~ error[call-resolve:no-overload]
    v: Double
}

// ... nor a `cmp`.
struct KFloatOrd : Ordered<self> by auto {
    //~ error[call-resolve:no-overload]
    v: Double
}

// A float field is fine for **equality**, which Salvo owns [kt-float-eq].
struct KFloatEq : Eq<self> by auto {
    v: Double
}

// A nested struct must itself be hashable — which now means *having* a
// `hash`, not declaring an opt-in.
struct Inner {
    v: Int
}

struct KNested : Hashed<self> by auto {
    //~ error[call-resolve:no-overload]
    //~ error[call-resolve:no-overload]
    i: Inner
}

// Lists and tuples qualify when their elements do (user, 2026-09-12).
struct KContainers : Hashed<self> by auto, Ordered<self> by auto {
    parts: List<Int>,
    pair: (Str, Int)
}

// [qual-canbe-optin] The `canbe` list is closed — and since 2026-09-21 it
// holds only `Mut` and `once`: `hashed`/`ordered` were deleted, with a message
// naming the obligation that replaced them [cmp-auto].
//~ error[qual-canbe-optin:unknown]
struct KSorted canbe Sorted {
    v: Int
}

//~ error[obligation-by:canbe-hashed]
struct KHashed canbe hashed {
    v: Int
}

// [type-canbe-mut] A struct literal's written `Mut` is validated like any
// written type: `Mut Cell { … }` for a struct without `canbe Mut` is refused
// (ROADMAP §0g: it used to pass the checker, run on Rust and fail in kotlinc).
struct Cell { v: Int }

fn mut_literal() -> Int {
    //~ error[struct-mut:not-canbe-mut]
    let c = Mut Cell { v: 1 }
    return c.v
}
