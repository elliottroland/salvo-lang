// [col-sorted] [cmp-carry] Sorted collections: a key must be **orderable**,
// a different bar from hashable, and a keyed container names the ordering or
// the hash it keeps its keys by.

// [col-sorted] A sorted collection's key must be **orderable**, which is a
// different bar from hashable — and the diagnostic says which one it is.
struct P : Ordered<self> by auto {
    x: Int
}

fn orderable_keys(a: SortedSet<Str>, b: SortedMap<Int, Str>, c: SortedSet<P>) -> Int => a, b, c {
    return size(a) + size(b) + size(c)
}

// A struct with only the hashing pair is a `Set` key but not a `SortedSet`
// one: the two axes are separate.
struct H : Hashed<self> by auto {
    x: Int
}

//~ error[col-sorted:key]
fn hashed_only(a: SortedSet<H>) -> Int => a {
    return size(a)
}

// [col-sorted] A **union** can be hashed but never ordered: comparing values
// of different types has no obvious meaning (user decision 2026-09-12).
//~ error[col-sorted:key]
fn union_key(a: SortedSet<Int | Str>) -> Int => a {
    return size(a)
}

// [col-sorted] Floats are out of both: no total order on Rust's `f64`.
//~ error[col-sorted:key]
fn float_key(a: SortedSet<Double>) -> Int => a {
    return size(a)
}

// [col-sorted-list] A `Sorted` claim is about the element order, so the
// elements must be orderable — refused where the claim is *written*.
//~ error[col-sorted-list:element]
fn sorted_doubles(xs: Sorted List<Double>) [] -> None => xs {
}

// [cmp-carry] The keyed containers **name** the ordering or the hash they
// keep their keys by, defaulted to the canonical one (user decision
// 2026-09-22). The default is not materialized, so `Set<Str>` is the type it
// always was — which is what keeps this out of everything that reads a
// container's arguments.
fn canonical_identity(s: Set<Int>, o: SortedSet<Int>) -> Int => s, o {
    return size(s) + size(o)
}

// [cmp-carry] A signature may **omit** a keyed container's identity
// arguments, and then it says nothing about them: `size(set: SortedSet<T>)`
// takes a set however it is ordered, which is what keeps std's own surface
// generic over the ordering without writing a binder in thirty signatures.
// Unstated is unconstrained, the same rule an unwritten slot has.
fn count<T>(s: SortedSet<T>) -> Int => s {
    return size(s)
}

fn omitted_identity(a: SortedSet<Int>, b: SortedSet<Str>) -> Int => a, b {
    return count(a) + count(b)
}

// …and a **named** one is accepted: the ordering is part of the type, and
// what a backend does with it is the backend's business — the Rust emitter
// keeps the collection by it, and the Kotlin emitter refuses until its
// runtime can [backend-never-wrong].
fn by_size(a: Int, b: Int) -> Int => a, b {
    return 0
}

fn named_ordering(s: SortedSet<Int>(by_size)) -> Int => s {
    return size(s)
}

// …and an identity that does not *fit* the slot is the ordinary mismatch,
// named at the type that wrote it.
fn wrong(a: Int) -> Int => a {
    return a
}

//~ error[cmp-carry:slot-mismatch]
fn misfit_identity(s: SortedSet<Int>(wrong)) -> Int => s {
    return size(s)
}
