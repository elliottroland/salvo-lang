// Ordering, equality and hashing: what `<`, `==` and the keyed collections
// rely on.
//
// Each capability is a single overloadable function: `cmp` orders two
// values, `eq` decides whether two are equal, `hash` digests one. A type
// takes part by having the function, and nothing else: there is no
// interface to implement, no dispatch table and nothing at run time. The
// groups declared here, [Ordered], [Eq] and [Hashed], name those shapes so
// that a type can promise them and a generic function can ask for them.
//
// The operators are calls: `a < b` is `cmp(a, b) < 0`, and `a == b` is
// `eq(a, b)`. The basic types, strings, lists and tuples come with theirs. A
// struct of your own gets the usual field-by-field ones with `by auto`, or
// declares its own:
//
// ```
// struct Point : Ordered<self> by auto, Hashed<self> by auto {
//     x: Int,
//     y: Int
// }
//
// struct Person : Ordered<self> {
//     name: Str,
//     age: Int
// }
//
// // People order by age.
// export fn cmp(a: Person, b: Person) [] -> Int => a, b {
//     return cmp(a.age, b.age)
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     let p = Point { x: 1, y: 2 }
//     let q = Point { x: 1, y: 3 }
//     println("${p < q} ${p == q}")                    // true false
//     let seen: Mut Set<Point> = Mut Set()
//     add(seen, p)
//     println("${contains(seen, q)}")                  // false
//     let ann = Person { name: "ann", age: 30 }
//     let bob = Person { name: "bob", age: 25 }
//     println("${bob < ann}")                          // true
// }
// ```
//
// **Generic code asks.** Nothing is known about a type parameter, so a
// generic function that compares says so in its signature, and its caller's
// `cmp` for the actual type is passed in automatically:
//
// ```
// fn smaller<T>(a: T, b: T, ?Ordered<T>) [] -> T => !a, !b {
//     if a <= b {
//         return a
//     }
//     return b
// }
//
// fn fruit() [] -> Str {
//     return smaller("pear", "apple")                  // apple
// }
// ```
//
// **The contracts.** The language trusts these rather than checking them,
// and the collections misbehave when they do not hold:
//
// - `cmp(a, b)` is negative when `a` sorts before `b`, zero when they tie
//   and positive when `a` sorts after, and it is a **total order**:
//   `cmp(a, b)` has the opposite sign of `cmp(b, a)`, it is transitive, and
//   it ties exactly when `eq` says equal. The floats' `cmp` is IEEE
//   comparison, under which `NaN` ties with everything; that is not a total
//   order, so a sorted collection of floats is refused.
// - `eq` is an equivalence: reflexive, symmetric and transitive.
// - `hash` agrees with `eq`: equal values hash equal. Different values may
//   hash equal too; that is an ordinary collision.
//
// **A hash is a value within one run, and nothing more.** Each backend
// hashes with its host's own algorithm, so the same value digests
// differently on Kotlin and on Rust. Do not print a hash, store it or
// compare it across runs: it is a bucket, not data.
//
// **Another ordering is another function.** What `<` means on a type is
// fixed in the type's own file. To sort or key by something else, write a
// function named for what it does and pass it where it is wanted, as
// `sort(people, cmp = by_name)` or `Mut Set(hash = by_x, eq = same_x)`.
// The Comparison and hashing topic has the whole story.
// @compilerrules [implicit-group, cmp-groups, implicit-resolve, col-sorted, cmp-hash-values]

// The ordering capability: a `cmp` answering the sign of `a` against `b`.
//
// A type promises it with `: Ordered<self>` (checked at the declaration),
// and a generic function asks for it with `?Ordered<T>`, which makes `<`,
// `<=`, `>` and `>=` usable on a `T`. A structure that stays ordered, such
// as a `SortedSet` or a heap, takes its ordering once, when it is built,
// and records it in its type, so that two built under different orderings
// cannot be mixed.
//
// It does not include `eq`: no sorted structure consults equality, since
// ties under `cmp` decide membership there.
// @compilerknown
export fns Ordered<T> {
    // Negative when `a` sorts before `b`, zero when they tie, positive
    // otherwise.
    fn cmp(a: T, b: T) -> Int
}

// The equality capability: an `eq` deciding whether two values are the
// same, which is what `==` and `!=` call.
//
// A struct holding a function, which nothing can compare field by field,
// becomes comparable by declaring an `eq` that ignores that field.
// @compilerknown
export fns Eq<T> {
    // Whether `a` and `b` are equal.
    fn eq(a: T, b: T) -> Bool
}

// What a `Set` element or a `Map` key needs: a `hash` and an `eq` that
// agree.
//
// The two are a **pair**: a hash container finds the bucket by `hash` and
// confirms a match by `eq`, so a `hash` without its `eq` is useless, and a
// pair that disagrees is a container that loses values. A call therefore
// supplies both or neither: replacing just one, as in
// `Mut Set(eq = same_age)`, is refused, and the error asks for both
// (`Mut Set(hash = age_hash, eq = same_age)`).
//
// The hash is a `Long`, the width both hosts digest at. Keeping `Ordered`
// separate from this pair is what lets one program hold a `Set<Person>`
// keyed by every field beside a `SortedSet<Person>` ranked by age.
// @compilerrules [implicit-with]
// @compilerknown
export fns Hashed<T> => eq with hash {
    // The digest of a value; equal values must digest equal.
    fn hash(value: T) -> Long
    // Whether `a` and `b` are equal, as the hash container confirms a match.
    fn eq(a: T, b: T) -> Bool
}

// ===== where the implementations are =====
//
// `cmp` and `eq` are operators (`<` is `cmp(a, b) < 0`, `==` is `eq(a, b)`),
// so each one on a type is declared in the type's own file: the scalars' and
// the tuples' in [std.core.basic], `Str`'s in [std.core.string], `List`'s in
// [std.core.list]. `hash` is not an operator, so the hashes are here, with
// the fold they share.
// @compilerrules [op-unique-shape]

// The hash of a basic value or a string, computed by the host.
//
// The same value hashes differently on the two backends; what holds
// everywhere is that equal values hash equal. `Double` and `Float` have no
// hash: a float is not a total order and `NaN` equals nothing, so a float
// has no business as a key.
// @compilerrules [cmp-groups, backend-intrinsic, cmp-hash-values]
export intrinsic fn hash(value: Int) [] -> Long => value
export intrinsic fn hash(value: Long) [] -> Long => value
export intrinsic fn hash(value: Byte) [] -> Long => value
export intrinsic fn hash(value: Char) [] -> Long => value
export intrinsic fn hash(value: Bool) [] -> Long => value
export intrinsic fn hash(value: Str) [] -> Long => value

// The hash of a list, from its elements' hashes in order: a `List<Person>`
// hashes by `Person`'s own `hash`. It agrees with a list's `==`, which
// compares element by element.
//
// @param hash  found at the call site: the hash of one element
// @compilerrules [col-hashed-ordered, implicit-recursive]
export fn hash<T>(value: List<T>, ?hash: (x: T) -> Long) [] -> Long => value {
    let h = 7L
    for x in value {
        h = mix_hash(h, hash(x))
    }
    return h
}

// The hash of a pair, from its elements' own hashes.
export fn hash<A, B>(value: (A, B), ?hash: (x: A) -> Long, ?hash: (x: B) -> Long) [] -> Long => value {
    return mix_hash(mix_hash(7L, hash(value.0)), hash(value.1))
}

// The hash of a triple, from its elements' own hashes.
export fn hash<A, B, C>(value: (A, B, C), ?hash: (x: A) -> Long, ?hash: (x: B) -> Long, ?hash: (x: C) -> Long) [] -> Long => value {
    return mix_hash(mix_hash(mix_hash(7L, hash(value.0)), hash(value.1)), hash(value.2))
}

// Combines a running hash with the next part's hash: `seed * 31 + value`,
// wrapping on overflow.
//
// It is what the structural hashes use (a list's, a tuple's, and the ones
// `by auto` writes for a struct), and a hand-written `hash` may use it too:
//
// ```
// struct Span { start: Int, end: Int }
//
// fn hash(s: Span) [] -> Long => s {
//     return mix_hash(hash(s.start), hash(s.end))
// }
// ```
// @compilerrules [op-wrap]
export fn mix_hash(seed: Long, value: Long) [] -> Long => seed, value {
    return seed * 31L + value
}
