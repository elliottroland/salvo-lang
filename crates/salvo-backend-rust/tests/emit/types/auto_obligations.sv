// Obligations met `by auto` [obligation-by] [cmp-auto].

//--- default_obligations_lower_to_salvo_fns
// [cmp-auto] `: Ordered<self> by auto` and friends: the compiler writes the
// structural implementations, `@`-scoped to the type, and lowers them to the
// derive the `default` clause asks for — so the generated
// member and the type's own `Ord`/`Hash` cannot disagree (user decision
// 2026-09-21, lowering split by author).
//
// Source and expected stdout are **verbatim** the Kotlin backend's
// `kotlinc_compiles_and_runs_auto_members`.
//
// [obligation-by] The lowering: a stamped member is an **ordinary Rust fn**
// whose body is the unrolled Salvo — a `cmp` per field, in declaration order —
// and the struct derives nothing beyond `Clone`, `Debug` and `PartialEq`: a
// container over a `List<Point>` goes through Salvo's list `eq`/`hash`/`cmp`
// [col-hashed-ordered].
//
// No structural member is lowered to the host's derived operation any more.
//~ !emits(main.rs): (Ord::cmp(a, b) as i32)
// Ordering is lexicographic by field declaration order, which is the language's
// rule on both backends.
//~ emits(main.rs): #[derive(Clone, Debug, PartialEq)]
//~| pub struct Point
// The stamped body is field-wise Salvo: one per-field `cmp`, then `0`.
//~ emits(main.rs): let mut c__c1: i32 = (Ord::cmp(&(a.x), &(b.x)) as i32);
//~ emits(main.rs): return 0i32;
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

// The hand-written generic `eq` takes its implicit.
//~ emits(main.rs): pub fn eq__
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
