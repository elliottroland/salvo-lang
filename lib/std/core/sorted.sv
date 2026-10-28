// Sorted collections: a `SortedSet<T>` and a `SortedMap<K, V>`, kept in the
// **natural order of their keys** rather than in insertion order
// [col-sorted]. Kotlin gets them from `TreeSet`/`TreeMap`, Rust from
// `BTreeSet`/`BTreeMap`.
//
// These are **separate types** from `Set` and `Map`, not a qualifier on them
// (user decision 2026-09-12): sortedness changes how a collection behaves,
// and a qualifier is droppable by design — there would be no way to stop a
// `Sorted Set` being passed where a plain `Set` is expected and quietly
// losing the property the callee relies on.
//
// [col-key-eligible] A key here must be **orderable**, which is a different
// bar from the hashability `Set`/`Map` ask for: the intrinsic ordered types
// (`Int`, `Long`, `Str`, `Char`, `Bool`), a `List` or tuple of orderable
// things (compared lexicographically), or a struct with a `cmp` — one token
// with `: Ordered<self> by auto`, or hand-written and `@`-scoped
// [obligation-by] [cmp-canonical]. `Double`/`Float` are excluded — Rust's `f64`
// has no total order — and a **union** is excluded on principle: comparing
// values of different types has no obvious meaning, where hashing them would
// have been fine.
//
// The ordering used is the key's **canonical** one. Parameterizing these types
// by an ordering (`SortedSet<T>(?cmp = cmp)`, so two differently ordered sets
// are different types) is designed and not built — see ROADMAP.md.
//
// Strings order by **code point** on both backends, which the Kotlin side
// has to arrange deliberately (its `String.compareTo` is UTF-16 code-unit
// order) [kt-ordered].

export iterable platform type SortedSet<T>(?cmp: (T, T) -> Int) canbe Mut : Iter<self, SetNext<T>, T>

// Constructor. The elements are stored, so they are moved; duplicates
// collapse, and the result is in order however the arguments were written.
//
// The constructor **is** the type-named function:
// `SortedSet(...)` resolves to this declaration (canonical key `SortedSet`;
// `Mut_SortedSet` for the mutable pair), the key carried verbatim to the
// platform shim's `<Key>_platform` wrapper.
// @compilerrules [fn-type-named]
export platform fn SortedSet<T>(...elems: T[], ?Ordered<T>) [] -> SortedSet<T>(?cmp)

// Mutable constructor
export platform fn Mut SortedSet<T>(...elems: T[], ?Ordered<T>) [] -> Mut SortedSet<T>(?cmp)

// Adds an element, reporting whether it was new.
export platform fn add<T>(set: Mut SortedSet<T>(?cmp), elem: T) [] -> Bool => set: Mut, !elem

// Removes an element, reporting whether it was there.
export fn remove<T>(set: Mut SortedSet<T>(?cmp), elem: T) [] -> Bool => set: Mut, elem {
    return remove_elem(set, elem)
}

// The host's [remove] and [size] for each container, under names of their own:
// two platform fns of one module may not overload each other.
platform fn remove_elem<T>(set: Mut SortedSet<T>(?cmp), elem: T) [] -> Bool => set: Mut, elem
platform fn set_size<T>(set: SortedSet<T>) [] -> Int => set
platform fn remove_key<K, V>(map: Mut SortedMap<K, V>(?cmp), key: K) [] -> V? => map: Mut, key
platform fn map_size<K, V>(map: SortedMap<K, V>) [] -> Int => map

// Whether the set holds an element equal to [elem].
export platform fn contains<T>(set: SortedSet<T>(?cmp), elem: T) [] -> Bool => set, elem

// Returns the number of elements in the set
export fn size<T>(set: SortedSet<T>) [] -> Int => set {
    return set_size(set)
}

// The smallest element, or `None` when the set is empty. Cheap here, where
// on an unordered `Set` it would be a scan — which is the reason to reach
// for a sorted collection in the first place.
export platform fn min<T>(set: SortedSet<T>) [] -> T? => set

// The largest element, or `None` when the set is empty.
export platform fn max<T>(set: SortedSet<T>) [] -> T? => set

// The elements as a list, in order.
export platform fn to_list<T>(set: SortedSet<T>) [] -> List<T> => set

// The text form: `{1, 2, 3}` in **sorted** order, each element
// by its own `to_str`.
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

// A fresh iterator over the elements, in order. A snapshot, like the
// unordered collections' iterators and for the same reason (there is no index
// to walk) — see `std.core.set`.
// @compilerrules [iter-mint]
export fn iter<T>(set: SortedSet<T>) [] -> Mut SetNext<T> => set {
    return Mut SetNext<T> { items: to_list(set), at: 0 }
}

export iterable platform type SortedMap<K, V>(?cmp: (K, K) -> Int) canbe Mut : Iter<self, MapKeyNext<K>, K>

// Constructor, from entries written as pairs. A repeated key takes the value
// of its last appearance [col-duplicate-keys]; position is irrelevant here,
// since the order is the keys'.
//
// The constructor **is** the type-named function:
// `SortedMap(...)` resolves to this declaration (canonical key `SortedMap`;
// `Mut_SortedMap` for the mutable pair), the key carried verbatim to the
// platform shim's `<Key>_platform` wrapper.
// @compilerrules [fn-type-named]
export platform fn SortedMap<K, V>(...entries: (K, V)[], ?Ordered<K>) [] -> SortedMap<K, V>(?cmp)

// Mutable constructor
export platform fn Mut SortedMap<K, V>(...entries: (K, V)[], ?Ordered<K>) [] -> Mut SortedMap<K, V>(?cmp)

// Possibly gets the value stored under [key], **borrowed** out of the map.
// @compilerrules [copy-opt-in]
export platform fn get<K, V>(map: SortedMap<K, V>(?cmp), key: K) [] -> (proj(map) V)?
=> map, key

// Stores [value] under [key], replacing any value already there.
export platform fn put<K, V>(map: Mut SortedMap<K, V>(?cmp), key: K, value: V) [] -> None
=> map: Mut, !key, !value

// Removes the entry under [key] and hands its value back.
export fn remove<K, V>(map: Mut SortedMap<K, V>(?cmp), key: K) [] -> V? => map: Mut, key {
    return remove_key(map, key)
}

// Whether the map holds an entry under [key].
export platform fn contains_key<K, V>(map: SortedMap<K, V>(?cmp), key: K) [] -> Bool => map, key

// Returns the number of entries in the map
export fn size<K, V>(map: SortedMap<K, V>) [] -> Int => map {
    return map_size(map)
}

// The smallest key, or `None` when the map is empty.
export platform fn first_key<K, V>(map: SortedMap<K, V>) [] -> K? => map

// The largest key, or `None` when the map is empty.
export platform fn last_key<K, V>(map: SortedMap<K, V>) [] -> K? => map

// The keys, in order.
export platform fn keys<K, V>(map: SortedMap<K, V>) [] -> List<K> => map

// The text form: `{a: 1, b: 2}` in **key order**.
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

// Two sorted sets are equal when they hold the same members
// under their ordering.
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

// Two sorted maps are equal when they hold the same keys (under
// their ordering) with equal values.
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

// A fresh iterator over the map's **keys**, in order — the same
// reading `std.core.map` takes, where a value is reached with `get`.
// @compilerrules [iter-mint]
export fn iter<K, V>(map: SortedMap<K, V>) [] -> Mut MapKeyNext<K> => map {
    return Mut MapKeyNext<K> { items: keys(map), at: 0 }
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

// `NonEmpty` over the two ordered containers, as `std.core.set`
// and `std.core.map` declare it over theirs.
// @compilerrules [qual-overload]
export qualifier NonEmpty<T> of SortedSet<T> {
    fn qualifies(set: SortedSet<T>) [] -> Bool {
        return size(set) > 0
    }

    refn add(set: Mut SortedSet<T>, elem: T) => set: +NonEmpty
}

export qualifier NonEmpty<K, V> of SortedMap<K, V> {
    fn qualifies(map: SortedMap<K, V>) [] -> Bool {
        return size(map) > 0
    }

    refn put(map: Mut SortedMap<K, V>, key: K, value: V) => map: +NonEmpty
}

// The dividend, and the reason the claim is worth carrying: the cheap-at-
// either-end operations of an ordered tree answer with an element instead of
// an optional. Ranked above the plain ones because they demand more of their
// argument.
//
// Each delegates through a **renamed** plain overload rather than through the
// shared name: `min@core.sorted(set)` would pick *this* overload again — the
// selector names a module, and within it a `NonEmpty` argument still ranks
// this one first — and recurse forever. `rename fn` takes the plain one out of
// the shared name *here only* (a rename is not importable), which
// is the same escape `first(NonEmpty List<T>)` makes by delegating to `get`.
// It is why these four used to live in a module of their
// own.
// @compilerrules [fn-overload-rank, fn-rename, col-of-nonempty]
export fn min<T>(set: NonEmpty SortedSet<T>) [] -> T => set {
    return min_opt(set) ?: unreachable!()
}

export fn max<T>(set: NonEmpty SortedSet<T>) [] -> T => set {
    return max_opt(set) ?: unreachable!()
}

export fn first_key<K, V>(map: NonEmpty SortedMap<K, V>) [] -> K => map {
    return first_key_opt(map) ?: unreachable!()
}

export fn last_key<K, V>(map: NonEmpty SortedMap<K, V>) [] -> K => map {
    return last_key_opt(map) ?: unreachable!()
}
