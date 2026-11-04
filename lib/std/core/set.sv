// Sets: collections that hold each element at most once and remember the
// order elements were first added. Iterating, printing and converting a set
// visit its elements in that order, the same on every backend.
//
// A `Set<T>` is a hash set kept in insertion order, like Java's
// `LinkedHashSet`. [add], [remove] and [contains] take O(1) on average;
// removing an element keeps the order of the rest. "The same element" means
// equal by the element type's `hash` and `eq`.
//
// ```
// let seen = Mut Set("ada", "grace")
// let fresh = add(seen, "linus")           // true: it was new
// let again = add(seen, "ada")             // false: already there
// let had = remove(seen, "grace")          // true
// println("${seen} ${contains(seen, "ada")}")   // {ada, linus} true
// for name in seen {
//     println(name)                        // ada, then linus
// }
// ```
//
// **Elements must be hashable.** `Int`, `Long`, `Str`, `Char` and `Bool` are;
// `Double` and `Float` are not, since floating-point equality would not mean
// the same on both backends. A struct becomes an element type by having a
// `hash` and an `eq`: write `: Hashed<self> by auto` on it, or write the two
// functions yourself (see the Comparison and Hashing topic). A set may also
// be kept by functions you name, `Mut Set(hash = my_hash, eq = my_eq)`; they
// become part of its type, so two sets kept differently do not mix.
//
// **Duplicates collapse.** Building a set from values with repeats keeps one
// of each: the position of its first appearance, the value of its last.
// [to_list] answers a `Distinct List`, a list the compiler knows holds no
// duplicates.
//
// **No obligations.** A set cannot hold linear values (values that must be
// used exactly once): adding an element equal to one already there would
// drop one of the two.
// @compilerrules [col-key-eligible, col-insertion-order, backend-parity]

// `canbe Mut` opts Set into the language-level `Mut` auto-qualifier: a
// backend may map `Mut Set<T>` to a different native type (Kotlin's
// `MutableSet<T>`) or to the same one. Both backends ship their own
// insertion-ordered slab (`SalvoSet` on Kotlin; Rust's own `HashSet` has
// no order to speak of), each operation handed the `hash` and `eq` its call
// resolved. Own module rather than part of `std.core.list` so a
// program that never uses a set emits no code for it.

// A set of elements of type `T`, each held at most once, kept in insertion
// order.
//
// `Set<T>` cannot be changed; `Mut Set<T>` can, and is what [add] and
// [remove] take. The `(?hash, ?eq)` slot is the pair of functions elements
// are hashed and compared by; nobody writes it for the usual case, where it
// is the element type's own. Two sets are `==` when they hold the same
// elements in any order; a set prints as `{1, 2, 3}`; `for x in s` walks its
// elements.
//
// ```
// let empty: Set<Int> = Set()
// let primes = Set(2, 3, 5, 3)             // {2, 3, 5}
// ```
// @compilerrules [type-canbe-mut, col-key-eligible, obligation-by, mod-used-only, qual-refn-type]
// @compilerknown
export iterable platform type Set<T>(?hash: (T) -> Long, ?eq: (T, T) -> Bool) canbe Mut : Iter<self, SetIter<T>, T> {
    // Adding an element makes a set non-empty, which `add` cannot promise
    // itself: a mutating function may not hand back a claim it has never
    // heard of.
    refn add(set: Mut Set<T>, elem: T) => set: +NonEmpty
}

// Builds a set of the elements written; with none, an empty set.
//
// The elements are moved into the set, a spread with them: `Set(a, ...xs)`
// spends the array `xs`. A repeated element is kept once, at the position
// of its first appearance, so the set may be smaller than the argument
// list. Elements are hashed by their type's `hash` and `eq`, found at the
// call site, or by the ones passed as `hash = ...` and `eq = ...`.
//
// ```
// let names = Set("ada", "grace", "ada")   // {ada, grace}
// ```
// @compilerrules [deduce-syntax, fn-type-named, col-duplicate-keys]
export platform fn Set<T>(...elems: T[], ?Hashed<T>) [] -> Set<T>(?hash, ?eq) => !elems

// The constructor **is** the type-named function: `Set(...)` resolves to
// this declaration (canonical key `Set`; `Mut_Set` for the mutable pair),
// the key carried verbatim to the platform shim's `<Key>_platform` wrapper.

// [Set], answering a mutable set: `Mut Set()` is an empty set to fill.
//
// ```
// let seen: Mut Set<Str> = Mut Set()
// add(seen, "ada")
// ```
export platform fn Mut Set<T>(...elems: T[], ?Hashed<T>) [] -> Mut Set<T>(?hash, ?eq) => !elems

// Builds a set from a given number of elements, each computed from its
// index.
//
// The function is called once per index, from 0 up. Duplicates collapse, so
// the set may hold fewer elements than the size asked for.
//
// ```
// let evens = set_by(3, (i) -> i * 2)      // {0, 2, 4}
// ```
// @compilerrules [col-by]
export platform fn set_by<T, [E]>(size: Int, init: (Int) [E] -> T, ?Hashed<T>) -> Set<T>(?hash, ?eq)
=> size, init

// [set_by], answering a mutable set.
export platform fn mut_set_by<T, [E]>(size: Int, init: (Int) [E] -> T, ?Hashed<T>) -> Mut Set<T>(?hash, ?eq)
=> size, init

// The elements of a list as a set, in order of first appearance.
//
// Duplicates collapse.
//
// ```
// let unique = to_set(List(3, 1, 3, 2))    // {3, 1, 2}
// ```
// @compilerrules [col-convert]
export platform fn to_set<T>(list: List<T>, ?Hashed<T>) [] -> Set<T>(?hash, ?eq) => list

// Adds an element to the set, answering whether it was new.
//
// `false` means an equal element was already there, and then the set is
// unchanged. A new element is moved into the set and goes last in the
// iteration order. The set is known non-empty afterwards.
//
// ```
// let seen = Mut Set("ada")
// let fresh = add(seen, "grace")           // true
// let again = add(seen, "ada")             // false
// ```
// @compilerrules [deduce-syntax]
export platform fn add<T>(set: Mut Set<T>(?hash, ?eq), elem: T) [] -> Bool => set: Mut, !elem

// Removes an element from the set, answering whether it was there.
//
// The element you pass is only read, so you keep it. The remaining elements
// keep their order.
//
// ```
// let seen = Mut Set("ada", "grace")
// let had = remove(seen, "ada")            // true; seen is {grace}
// ```
export platform fn remove<T>(set: Mut Set<T>(?hash, ?eq), elem: T) [] -> Bool => set: Mut, elem

// Whether the set holds an element equal to a value.
export platform fn contains<T>(set: Set<T>(?hash, ?eq), elem: T) [] -> Bool => set, elem

// The number of elements in the set.
export platform fn size<T>(set: Set<T>) [] -> Int => set

// The text form of a set, `{1, 2, 3}` in insertion order: what `"${s}"`
// prints.
//
// Each element is rendered by its own `to_str`, found at the call site.
// @compilerrules [interp-to-str, col-to-str, platform-value-type]
export fn to_str<T>(set: Set<T>, ?to_str: (x: T) -> Str) [] -> Str => set {
    let out = Mut Str("{")
    let i = 0
    for x in set {
        if i > 0 {
            append(out, ", ")
        }
        append(out, to_str(x))
        i = i + 1
    }
    append(out, "}")
    return out
}


// Whether two sets hold the same elements, in any order: what `a == b`
// means for sets.
//
// ```
// let same = Set(1, 2) == Set(2, 1)        // true
// ```
// @compilerrules [op-equality]
export fn eq<T>(a: Set<T>(?hash, ?eq), b: Set<T>(?hash, ?eq)) [] -> Bool => a, b {
    // The identity is the type's, so both sides are looked up the same way.
    if size(a) != size(b) {
        return false
    }
    for x in a {
        if !contains(b, x) {
            return false
        }
    }
    return true
}

// The claim that a list holds no duplicates.
//
// A set is the one thing that can honestly promise it, so the claim comes
// from [to_list] over a `Set`, and there is no `is Distinct` test: deciding
// it for an arbitrary list would mean comparing its elements. Any change to
// the list gives the claim up.
//
// ```
// let unique = to_list(Set(3, 1, 3))       // a Distinct List<Int>: [3, 1]
// ```
// @compilerrules [col-distinct, qual-ctor-same-file]
export qualifier Distinct<T> for List<T>

// It lives here, in `std.core.set`, rather than beside `List`: a
// constructor must be declared in the same file as the qualifier it claims.
// (`to_list` over a `SortedSet` is in `std.core.sorted` and so cannot mint
// it, which is why that one returns a plain list.)

// The elements of a set as a new list, in insertion order.
//
// A set holds no duplicates, so neither does the list, and its type says so:
// the answer is a [Distinct] list.
//
// ```
// let xs = to_list(Set("b", "a"))          // [b, a]
// ```
// @compilerrules [col-distinct]
export platform fn to_list<T>(set: Set<T>) [] -> +Distinct List<T> => set

// Reads an element **owned** out of a snapshot an iterator owns: how
// [SetIter] and `MapKeyIter` emit their elements.
// @compilerrules [kt-copy, col-key-eligible]
export intrinsic fn snapshot_at<T>(items: List<T>, index: Int) [] -> T? => items, index

// Not `copy(get(items, i))`: copying a value of an unconstrained generic
// type is unsupported on the Kotlin backend by construction (with erased
// generics it cannot tell an immutable `Str` from a `Mut List` at runtime,
// so it refuses rather than aliasing). This read is sound where `copy`
// cannot be: an element of a set is a **key**, and keys are the immutable
// intrinsic types, for which sharing the reference *is* the copy. Kept out
// of `std.core.list` deliberately: as a general `List` accessor the same
// lowering would alias a mutable element.

// A fresh iterator over the set's elements, in insertion order.
//
// A `for` loop over a set walks the same elements. The iterator walks a
// copy of the elements taken when it is made (O(n) once, then O(1) a
// step), so changing the set while walking it does not change what the walk
// visits, on both backends alike.
//
// ```
// let seen = Mut Set(1, 2)
// for x in iter(seen) {
//     println("${x}")                      // 1, then 2
// }
// ```
// @compilerrules [iter-mint, proj-field]
export fn iter<T>(set: Set<T>) [] -> Mut SetIter<T> => set {
    // A hash set has no index to walk, and the alternative, holding the
    // target's native iterator in an intrinsic iterator, needs a borrowing
    // intrinsic type neither backend has machinery for (see COMPLETED.md).
    // The snapshot makes a set mutated during a walk behave the same on both
    // backends, rather than being a race on one and a refusal on the other.
    return Mut SetIter<T> { items: to_list(set), at: 0 }
}

// The iterator [iter] answers: a copy of a set's elements, owned by the
// iterator, and a position in it.
//
// The elements it emits are its own, not borrowed from the set: an
// iterator that computes its elements owns them, where one that walks data,
// like a list's, borrows them.
// @compilerrules [iter-protocol]
export struct SetIter<T> : Next<self, T> canbe Mut {
    // The elements, in insertion order, owned by this iterator.
    items: List<T>,
    // The index of the next element to emit.
    at: Int
}

// Answers the element at the iterator's position and moves past it, or the
// end once every element has been emitted.
export fn next<T>(p: Mut SetIter<T>) [] -> Emitted T | Finished => p: Mut {
    let elem = snapshot_at(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}
