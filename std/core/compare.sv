// Comparison, equality and hashing — the three capabilities, declared the
// way iteration is declared: as **params groups** [implicit-group], not as
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
export params Ordered<T> {
    fn cmp(a: T, b: T) -> Int
}

// The equality capability: one function, deciding whether two values are the
// same. A struct with a fn-typed field — which nothing can compare
// structurally — becomes comparable by declaring an `eq` that ignores it.
export params Eq<T> {
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
// a `Set<Person>` by all fields beside a `SortedSet<Person, by_age>` by rank
// without either being a lie.
export params Hashed<T> {
    fn hash(value: T) -> Long
    fn eq(a: T, b: T) -> Bool
}

// ===== the canonical implementations for the intrinsic types =====
//
// [cmp-groups] These are what a `cmp`/`eq`/`hash` at a primitive resolves to,
// and they are `intrinsic` for the same reason every other primitive
// operation is: the comparison is the host's, and each backend lowers it
// [backend-intrinsic].
//
// `cmp(Str, Str)` keeps the code-point contract both backends already owe
// [col-sorted]: Rust's `String: Ord` is byte-wise UTF-8, which *is* code-point
// order, while the JVM's `String.compareTo` is UTF-16 code-unit order and has
// to be corrected [kt-ordered].

export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn cmp(a: Long, b: Long) [] -> Int => a, b
export intrinsic fn cmp(a: Byte, b: Byte) [] -> Int => a, b
export intrinsic fn cmp(a: Char, b: Char) [] -> Int => a, b
export intrinsic fn cmp(a: Bool, b: Bool) [] -> Int => a, b
export intrinsic fn cmp(a: Str, b: Str) [] -> Int => a, b

// Equality covers the float widths where ordering cannot: comparing two
// `Double`s is meaningful (and both backends agree on IEEE, `NaN` equalling
// nothing), ordering a set of them is not.
export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b
export intrinsic fn eq(a: Long, b: Long) [] -> Bool => a, b
export intrinsic fn eq(a: Double, b: Double) [] -> Bool => a, b
export intrinsic fn eq(a: Float, b: Float) [] -> Bool => a, b
export intrinsic fn eq(a: Byte, b: Byte) [] -> Bool => a, b
export intrinsic fn eq(a: Char, b: Char) [] -> Bool => a, b
export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b
export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b

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
