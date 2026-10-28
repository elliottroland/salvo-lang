// Comparison, equality and hashing — the three capabilities, declared the
// way iteration is declared: as **fns groups** [implicit-group], not as
// traits (user decisions 2026-09-21, the ordering round).
//
// [cmp-groups] A capability is "a function of this shape is in scope", and
// nothing more: `cmp` orders, `eq` decides equality, `hash` digests. Each is
// an ordinary overloadable fn, so a type joins in by declaring one — and a
// generic fn reaches the capability by asking for it, `?Ordered<T>` at the
// signature, the caller's own resolution filling it [implicit-resolve].
// There is no interface, no dispatch table and no runtime representation on
// either backend.
//
// The contracts, which the language trusts rather than checks:
//
//   * `cmp(a, b)` is negative when `a` sorts before `b`, zero when they tie
//     and positive when `a` sorts after — a **total order**: consistent
//     (`cmp(a, b) == -cmp(b, a)`), transitive, and tying exactly when `eq`
//     says equal. `Double`/`Float` therefore have no `cmp` (`NaN` ties with
//     nothing, so no total order exists), which is also why a sorted
//     collection of them stays refused [col-sorted].
//   * `eq(a, b)` is an equivalence: reflexive, symmetric, transitive.
//   * `hash(a)` agrees with `eq`: `eq(a, b)` implies `hash(a) == hash(b)`
//     [cmp-hash-values]. The converse is not promised — a collision is
//     ordinary — and neither is the *value*, which differs between backends
//     by design.

// The ordering capability: one function, answering the sign of `a` against
// `b`. A structure that *holds* an ordering (a `SortedSet`, a heap) binds one
// at construction rather than resolving one per operation.
export fns Ordered<T> {
    fn cmp(a: T, b: T) -> Int
}

// The equality capability: one function, deciding whether two values are the
// same. A struct with a fn-typed field — which nothing can compare
// structurally — becomes comparable by declaring an `eq` that ignores it.
export fns Eq<T> {
    fn eq(a: T, b: T) -> Bool
}

// What a `Set` element or a `Map` key needs, which is a **pair**: a hash
// container buckets by `hash` and confirms the bucket hit by `eq`, so a `hash`
// without its `eq` is useless and a pair that disagrees is a silently broken
// container (user decision 2026-09-22). `Long` rather than `Int` because both
// hosts hand back a wide digest and narrowing it would throw away bits for
// nothing.
//
// `Ordered` deliberately does *not* bundle `eq`: no sorted container consults
// equality — both hosts collapse by the comparator — so an `eq` there would be a
// member nothing reads, and keeping the two apart is what lets one program hold
// a `Set<Person>` by all fields beside a `SortedSet<Person>(by_age)` by rank
// without either being a lie.
// [implicit-with] The two are filled **together**: a hash and an equality are
// only meaningful as a pair, and a container looks up by the hash first, so a
// custom equality beside a resolved hash quietly does nothing rather than
// failing. `=> eq with hash` makes a call state the pair or leave it alone —
// write both (`Mut Set(hash = by_x, eq = eq)`) when one of them differs
// (user decision 2026-09-26).
//
// `Ordered` needs no such clause: one member cannot disagree with itself.
export fns Hashed<T> => eq with hash {
    fn hash(value: T) -> Long
    fn eq(a: T, b: T) -> Bool
}

// ===== where the implementations are =====
//
// [unique-shape] `cmp` and `eq` are operators (`<` is `cmp(a, b) < 0`, `==` is
// `eq(a, b)`), so each one on a type is declared in the type's own file: the
// scalars' and the tuples' in `std.core.basic`, `Str`'s in `std.core.string`,
// `List`'s in `std.core.list` (user decision O1-D, 2026-10-27). `hash` is not
// an operator, so the hashes stay here, with the fold they share.

// [cmp-groups] The canonical `hash` for the intrinsic types, `intrinsic` for
// the same reason every other primitive operation is: the digest is the
// host's, and each backend lowers it [backend-intrinsic].
//
// [cmp-hash-values] A hash is a value **within one execution** and nothing
// more: each backend hashes with its own host algorithm, so the same value
// digests differently on Kotlin and on Rust — deliberately, on the analogy of
// the two backends' random numbers (user decision 2026-09-21). What holds
// everywhere is the agreement with `eq`. So a program must not print, persist
// or compare-across-runs a hash value; it is a bucket, not data.
//
// `Double`/`Float` have no `hash`: Rust's `f64` is not `Hash`, and a key that
// is not orderable and hashes `NaN` unpredictably has no business in a
// keyed collection.
export intrinsic fn hash(value: Int) [] -> Long => value
export intrinsic fn hash(value: Long) [] -> Long => value
export intrinsic fn hash(value: Byte) [] -> Long => value
export intrinsic fn hash(value: Char) [] -> Long => value
export intrinsic fn hash(value: Bool) [] -> Long => value
export intrinsic fn hash(value: Str) [] -> Long => value

// [col-hashed-ordered] [implicit-recursive] A `List` and a tuple hash
// **element by element, through the elements' own identities**, as they
// compare (their `eq` and `cmp` beside their types): a `List<Person>` hashes
// by `Person`'s declared `hash`. Ordinary Salvo over the implicits of the
// element types (ROADMAP §0j step 8).
export fn hash<T>(value: List<T>, ?hash: (x: T) -> Long) [] -> Long => value {
    let h = 7L
    for x in value {
        h = mix_hash(h, hash(x))
    }
    return h
}

export fn hash<A, B>(value: (A, B), ?hash: (x: A) -> Long, ?hash: (x: B) -> Long) [] -> Long => value {
    return mix_hash(mix_hash(7L, hash(value.0)), hash(value.1))
}

export fn hash<A, B, C>(value: (A, B, C), ?hash: (x: A) -> Long, ?hash: (x: B) -> Long, ?hash: (x: C) -> Long) [] -> Long => value {
    return mix_hash(mix_hash(mix_hash(7L, hash(value.0)), hash(value.1)), hash(value.2))
}

// The fold a structural `hash` combines its fields' digests with: `seed * 31 +
// value`, which wraps on both backends because integer arithmetic does
// [op-wrap]. What `std.core.auto`'s `hash` calls per field; a hand-written `hash`
// may use it too. Ordinary Salvo since 2026-10-05 (ROADMAP §0j step 2).
export fn mix_hash(seed: Long, value: Long) [] -> Long => seed, value {
    return seed * 31L + value
}
