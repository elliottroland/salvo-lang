// Sorted collections: a `SortedSet<T>` and a `SortedMap<K, V>`, kept in the
// order of their keys rather than the order they were added.
//
// Each is a sorted array searched by bisection, on both backends. Lookups
// ([contains], [get], [contains_key]) take O(log n); the smallest and
// largest key ([min], [max], [first_key], [last_key]) take O(1); adding and
// removing take O(log n) to find the place and O(n) to shift the elements
// after it. They suit collections read more often than they change.
//
// ```
// let names: Mut SortedSet<Str> = Mut SortedSet("pear", "apple")
// add(names, "fig")
// println("${names}")                      // {apple, fig, pear}
// let smallest = min(names)                // "apple", a Str: names is non-empty
// let prices = Mut SortedMap("pear": 3, "apple": 2)
// put(prices, "fig", 5)
// println("${prices}")                     // {apple: 2, fig: 5, pear: 3}
// let cheapest = first_key(prices)         // "apple"
// ```
//
// **Separate types, not a claim on `Set` or `Map`.** Sortedness changes how
// a collection behaves, and a qualifier can be dropped on the way into a
// function: a `Sorted Set` passed where a plain `Set` is expected would
// quietly lose the property a caller relied on. (A `Sorted List`, in
// `std.core.list`, is a claim about an ordinary list, which any change but
// `add_sorted` gives up.)
//
// **Keys must be orderable.** `Int`, `Long`, `Str`, `Char` and `Bool` are, and
// so are a `List` or tuple of orderable things, compared element by element.
// A struct becomes orderable by having a `cmp`: write
// `: Ordered<self> by auto` on it, or write the function yourself (see the
// Comparison and Hashing topic). `Double` and `Float` are not orderable,
// having no total order on both backends, and neither is a union, since
// comparing values of different types has no obvious meaning. Strings order
// by code point on both backends.
//
// **One member per tie.** Two keys are the same member when the ordering
// answers 0 for them; equality plays no part. A set sorted by age keeps one
// person per age.
//
// **Other orderings.** The ordering is the key type's own `cmp` unless you
// pass one, `Mut SortedSet(cmp = by_age)`. It becomes part of the type, so
// two sets ordered differently do not mix, and [min], [max], [first_key] and
// [last_key] answer first and last in that ordering.
//
// ```
// fn desc(a: Int, b: Int) -> Int {
//     return cmp(b, a)
// }
//
// fn example() -> None {
//     let down = Mut SortedSet(1, 3, 2, cmp = desc)
//     add(down, 5)                         // {5, 3, 2, 1}
//     let last = max(down)                 // 1: last in this ordering
// }
// ```
// @compilerrules [col-sorted, col-key-eligible, obligation-by, kt-ordered]

// The Kotlin side has to arrange code-point order deliberately: its
// `String.compareTo` is UTF-16 code-unit order. Both backends keep a sorted
// array (`SalvoSortedSet`/`SalvoSortedMap` on Kotlin, a `Vec` on Rust), and
// every operation is handed the `cmp` its call resolved.

// A set of elements of type `T`, each held at most once, kept in the order
// of the set's ordering.
//
// `SortedSet<T>` cannot be changed; `Mut SortedSet<T>` can, and is what
// [add] and [remove] take. The `(?cmp)` slot is the ordering; nobody writes
// it for the usual case, where it is the element type's own. Two sorted sets
// are `==` when they hold the same members; a sorted set prints as
// `{1, 2, 3}`, in order; `for x in s` walks it in order.
//
// ```
// let ranks = SortedSet(30, 10, 20, 10)    // {10, 20, 30}
// ```
// @compilerrules [qual-refn-type]
// @compilerknown
export iterable platform type SortedSet<T>(?cmp: (T, T) -> Int) canbe Mut : Iter<self, SetIter<T>, T> {
    // Adding an element makes the set non-empty.
    refn add(set: Mut SortedSet<T>, elem: T) => set: +NonEmpty
}

// Builds a sorted set of the elements written; with none, an empty set.
//
// The elements are moved in, a spread with them. However the arguments are
// written, the set is in order, and elements that tie collapse to one. The
// ordering is the element type's `cmp`, found at the call site, or the one
// passed as `cmp = ...`.
//
// ```
// let ranks = SortedSet(30, 10, 20, 10)    // {10, 20, 30}
// let empty: SortedSet<Str> = SortedSet()
// ```
// @compilerrules [fn-type-named, col-duplicate-keys]
export platform fn SortedSet<T>(...elems: T[], ?Ordered<T>) [] -> SortedSet<T>(?cmp) => !elems

// The constructor **is** the type-named function: `SortedSet(...)` resolves
// to this declaration (canonical key `SortedSet`; `Mut_SortedSet` for the
// mutable pair), the key carried verbatim to the platform shim's
// `<Key>_platform` wrapper.

// [SortedSet], answering a mutable set: `Mut SortedSet()` is an empty set to
// fill.
export platform fn Mut SortedSet<T>(...elems: T[], ?Ordered<T>) [] -> Mut SortedSet<T>(?cmp) => !elems

// Adds an element to the set, answering whether it was new.
//
// `false` means an element that ties with it was already there, and then
// the set is unchanged. The set is known non-empty afterwards.
//
// ```
// let s = Mut SortedSet(2, 1)
// let fresh = add(s, 3)                    // true; s is {1, 2, 3}
// let again = add(s, 1)                    // false
// ```
export platform fn add<T>(set: Mut SortedSet<T>(?cmp), elem: T) [] -> Bool => set: Mut, !elem

// Removes an element from the set, answering whether it was there.
//
// The element you pass is only read, so you keep it.
export fn remove<T>(set: Mut SortedSet<T>(?cmp), elem: T) [] -> Bool => set: Mut, elem {
    return remove_elem(set, elem)
}

// The host's [remove] and [size] for each container, under names of their own:
// two platform fns of one module may not overload each other.
platform fn remove_elem<T>(set: Mut SortedSet<T>(?cmp), elem: T) [] -> Bool => set: Mut, elem
platform fn set_size<T>(set: SortedSet<T>) [] -> Int => set
platform fn remove_key<K, V>(map: Mut SortedMap<K, V>(?cmp), key: K) [] -> V? => map: Mut, key
platform fn map_size<K, V>(map: SortedMap<K, V>) [] -> Int => map

// Whether the set holds an element that ties with a value under its
// ordering.
//
// O(log n).
export platform fn contains<T>(set: SortedSet<T>(?cmp), elem: T) [] -> Bool => set, elem

// The number of elements in the set.
export fn size<T>(set: SortedSet<T>) [] -> Int => set {
    return set_size(set)
}

// The first element in the set's order, the smallest, or `None` when the
// set is empty.
//
// O(1), where an unordered `Set` would need a scan: the reason to reach for
// a sorted collection. On a set known to be `NonEmpty` (just added to, or
// tested with `is NonEmpty`) the overload below answers the element itself.
//
// ```
// let ranked = SortedSet(30, 10)
// let lo = min(ranked)                     // 10, as an Int?
// if ranked is NonEmpty {
//     let lowest = min(ranked)             // 10, an Int
// }
// ```
export platform fn min<T>(set: SortedSet<T>) [] -> T? => set

// The last element in the set's order, the largest, or `None` when the set
// is empty.
//
// O(1). Like [min], it has an overload for a `NonEmpty` set that answers
// the element itself.
export platform fn max<T>(set: SortedSet<T>) [] -> T? => set

// The elements of the set as a new list, in order.
export platform fn to_list<T>(set: SortedSet<T>) [] -> List<T> => set

// The text form of a sorted set, `{1, 2, 3}` in order: what `"${s}"`
// prints.
//
// Each element is rendered by its own `to_str`, found at the call site.
// @compilerrules [col-to-str, platform-value-type]
export fn to_str<T>(set: SortedSet<T>, ?to_str: (x: T) -> Str) [] -> Str => set {
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

// A fresh iterator over the set's elements, in order.
//
// A `for` loop over the set walks the same elements. Like a `Set`'s, the
// iterator walks a copy of the elements taken when it is made, so changing
// the set while walking it does not change what the walk visits.
// @compilerrules [iter-mint]
export fn iter<T>(set: SortedSet<T>) [] -> Mut SetIter<T> => set {
    // A snapshot, like the unordered collections' iterators and for the same
    // reason (there is no index to walk): see `std.core.set`.
    return Mut SetIter<T> { items: to_list(set), at: 0 }
}

// A map from keys of type `K` to values of type `V`, kept in the order of
// its keys.
//
// `SortedMap<K, V>` cannot be changed; `Mut SortedMap<K, V>` can, and is
// what [put] and [remove] take. The `(?cmp)` slot is the ordering of the
// keys; nobody writes it for the usual case, where it is the key type's own.
// Two sorted maps are `==` when they hold equal values under the same keys;
// a sorted map prints as `{a: 1, b: 2}`, in key order; `for k in m` walks
// its keys in order.
//
// ```
// let prices = SortedMap("pear": 3, "apple": 2)   // {apple: 2, pear: 3}
// ```
// @compilerrules [qual-refn-type]
// @compilerknown
export iterable platform type SortedMap<K, V>(?cmp: (K, K) -> Int) canbe Mut : Iter<self, MapKeyIter<K>, K> {
    // Putting an entry makes the map non-empty.
    refn put(map: Mut SortedMap<K, V>, key: K, value: V) => map: +NonEmpty
}

// Builds a sorted map from entries written as pairs; with none, an empty
// map.
//
// `SortedMap("a": 1)` is `SortedMap(("a", 1))`. The keys and values are
// moved in. A key repeated among the entries takes the value of its last
// appearance; the order is the keys', whatever order they were written in.
//
// ```
// let prices = SortedMap("pear": 3, "apple": 2, "pear": 4)   // {apple: 2, pear: 4}
// ```
// @compilerrules [fn-type-named]
export platform fn SortedMap<K, V>(...entries: (K, V)[], ?Ordered<K>) [] -> SortedMap<K, V>(?cmp) => !entries

// The constructor **is** the type-named function: `SortedMap(...)` resolves
// to this declaration (canonical key `SortedMap`; `Mut_SortedMap` for the
// mutable pair), the key carried verbatim to the platform shim's
// `<Key>_platform` wrapper.

// [SortedMap], answering a mutable map: `Mut SortedMap()` is an empty map
// to fill.
export platform fn Mut SortedMap<K, V>(...entries: (K, V)[], ?Ordered<K>) [] -> Mut SortedMap<K, V>(?cmp) => !entries

// The value stored under a key, borrowed, or `None` when the key is absent.
//
// O(log n). The value is a read-only view of the map's own; write
// `copy(get(m, k))` to keep one of your own.
//
// ```
// let prices = SortedMap("pear": 3)
// let pear = get(prices, "pear") ?: 0      // 3
// ```
// @compilerrules [copy-opt-in]
export platform fn get<K, V>(map: SortedMap<K, V>(?cmp), key: K) [] -> (proj(map) V)?
=> map, key

// Stores a value under a key, replacing any value already there.
//
// The key and value are moved into the map, which is known non-empty
// afterwards.
export platform fn put<K, V>(map: Mut SortedMap<K, V>(?cmp), key: K, value: V) [] -> None
=> map: Mut, !key, !value

// Removes the entry under a key and answers its value, or `None` when there
// was none.
//
// The value is moved out; the key you pass is only read.
export fn remove<K, V>(map: Mut SortedMap<K, V>(?cmp), key: K) [] -> V? => map: Mut, key {
    return remove_key(map, key)
}

// Whether the map holds an entry under a key.
//
// O(log n).
export platform fn contains_key<K, V>(map: SortedMap<K, V>(?cmp), key: K) [] -> Bool => map, key

// The number of entries in the map.
export fn size<K, V>(map: SortedMap<K, V>) [] -> Int => map {
    return map_size(map)
}

// The first key in the map's order, the smallest, or `None` when the map is
// empty.
//
// O(1). On a map known to be `NonEmpty` the overload below answers the key
// itself.
//
// ```
// let prices = Mut SortedMap("pear": 3)
// put(prices, "apple", 2)
// let cheapest = first_key(prices)         // "apple", a Str: prices is non-empty
// ```
export platform fn first_key<K, V>(map: SortedMap<K, V>) [] -> K? => map

// The last key in the map's order, the largest, or `None` when the map is
// empty.
//
// O(1). Like [first_key], it has an overload for a `NonEmpty` map.
export platform fn last_key<K, V>(map: SortedMap<K, V>) [] -> K? => map

// The keys of the map as a new list, in order.
export platform fn keys<K, V>(map: SortedMap<K, V>) [] -> List<K> => map

// The text form of a sorted map, `{a: 1, b: 2}` in key order: what
// `"${m}"` prints.
// @compilerrules [col-to-str]
export fn to_str<K, V>(map: SortedMap<K, V>(?cmp), ?to_str: (k: K) -> Str, ?to_str: (v: V) -> Str) [] -> Str
=> map {
    let out = Mut Str("{")
    let i = 0
    for k in map {
        if i > 0 {
            append(out, ", ")
        }
        append(out, to_str(k))
        append(out, ": ")
        append(out, to_str(get(map, k) ?: unreachable!()))
        i = i + 1
    }
    append(out, "}")
    return out
}

// Whether two sorted sets hold the same members under their ordering: what
// `a == b` means for sorted sets.
// @compilerrules [op-equality]
export fn eq<T>(a: SortedSet<T>(?cmp), b: SortedSet<T>(?cmp)) [] -> Bool => a, b {
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

// Whether two sorted maps hold the same keys, under their ordering, with
// equal values: what `a == b` means for sorted maps.
//
// Values compare by their type's `==`, found at the call site.
// @compilerrules [op-equality]
export fn eq<K, V>(a: SortedMap<K, V>(?cmp), b: SortedMap<K, V>(?cmp), ?eq: (x: V, y: V) -> Bool) [] -> Bool
=> a, b {
    if size(a) != size(b) {
        return false
    }
    for k in a {
        let theirs = get(b, k)
        if theirs is None {
            return false
        }
        if !eq(get(a, k) ?: unreachable!(), theirs) {
            return false
        }
    }
    return true
}

// A fresh iterator over the map's keys, in order.
//
// A `for` loop over the map walks the same keys, and a value is reached with
// [get], as for a `Map`. The iterator walks a copy of the keys taken when it
// is made.
//
// ```
// let prices = SortedMap("pear": 3, "apple": 2)
// for k in iter(prices) {
//     let p = get(prices, k)
//     if p is Int {
//         println("${k}: ${p}")            // apple: 2, then pear: 3
//     }
// }
// ```
// @compilerrules [iter-mint]
export fn iter<K, V>(map: SortedMap<K, V>) [] -> Mut MapKeyIter<K> => map {
    return Mut MapKeyIter<K> { items: keys(map), at: 0 }
}

// The plain, optional-answering forms, taken out of the shared name *inside
// this module* so the `NonEmpty` overloads below can delegate to them without
// re-picking themselves. A rename is not importable, so every
// other module still sees one `min`, one `max`, one `first_key`, one
// `last_key`.
// @compilerrules [fn-rename]
rename fn min_opt = min<T>(set: SortedSet<T>)
rename fn max_opt = max<T>(set: SortedSet<T>)
rename fn first_key_opt = first_key<K, V>(map: SortedMap<K, V>)
rename fn last_key_opt = last_key<K, V>(map: SortedMap<K, V>)

// Each overload below delegates through a **renamed** plain overload rather
// than through the shared name: `min@core.sorted(set)` would pick *this*
// overload again (the selector names a module, and within it a `NonEmpty`
// argument still ranks this one first) and recurse forever. `rename fn`
// takes the plain one out of the shared name *here only*, which is the same
// escape `first(NonEmpty List<T>)` makes by delegating to `get`.

// [min] of a set known non-empty: answers the element itself.
//
// Ranked above the plain [min], so a call reaches it whenever the set's
// type carries `NonEmpty`.
// @compilerrules [fn-overload-rank, fn-rename, col-of-nonempty]
export fn min<T>(set: NonEmpty SortedSet<T>) [] -> T => set {
    return min_opt(set) ?: unreachable!()
}

// [max] of a set known non-empty: answers the element itself.
export fn max<T>(set: NonEmpty SortedSet<T>) [] -> T => set {
    return max_opt(set) ?: unreachable!()
}

// [first_key] of a map known non-empty: answers the key itself.
export fn first_key<K, V>(map: NonEmpty SortedMap<K, V>) [] -> K => map {
    return first_key_opt(map) ?: unreachable!()
}

// [last_key] of a map known non-empty: answers the key itself.
export fn last_key<K, V>(map: NonEmpty SortedMap<K, V>) [] -> K => map {
    return last_key_opt(map) ?: unreachable!()
}
