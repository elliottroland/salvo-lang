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
// with `: auto Ordered<self>`, or hand-written and `@`-scoped
// [cmp-auto] [cmp-canonical]. `Double`/`Float` are excluded — Rust's `f64`
// has no total order — and a **union** is excluded on principle: comparing
// values of different types has no obvious meaning, where hashing them would
// have been fine.
//
// The ordering used is the key's **canonical** one. Parameterizing these types
// by an ordering (`SortedSet<T, ?cmp = cmp>`, so two differently ordered sets
// are different types) is designed and not built — see ROADMAP.md.
//
// Strings order by **code point** on both backends, which the Kotlin side
// has to arrange deliberately (its `String.compareTo` is UTF-16 code-unit
// order) [kt-ordered].

export intrinsic type SortedSet<T> canbe Mut

// Constructor. The elements are stored, so they are moved; duplicates
// collapse, and the result is in order however the arguments were written.
export intrinsic fn sorted_set_of<T>(...elems: T[]) [] -> SortedSet<T>

// Mutable constructor
export intrinsic fn mut_sorted_set_of<T>(...elems: T[]) [] -> Mut SortedSet<T>

// Adds an element, reporting whether it was new.
export intrinsic fn add<T>(set: Mut SortedSet<T>, elem: T) [] -> Bool => set: Mut, !elem

// Removes an element, reporting whether it was there.
export intrinsic fn remove<T>(set: Mut SortedSet<T>, elem: T) [] -> Bool => set: Mut, elem

// Whether the set holds an element equal to [elem].
export intrinsic fn contains<T>(set: SortedSet<T>, elem: T) [] -> Bool => set, elem

// Returns the number of elements in the set
export intrinsic fn size<T>(set: SortedSet<T>) [] -> Int => set

// The smallest element, or `None` when the set is empty. Cheap here, where
// on an unordered `Set` it would be a scan — which is the reason to reach
// for a sorted collection in the first place.
export intrinsic fn min<T>(set: SortedSet<T>) [] -> T? => set

// The largest element, or `None` when the set is empty.
export intrinsic fn max<T>(set: SortedSet<T>) [] -> T? => set

// The elements as a list, in order.
export intrinsic fn to_list<T>(set: SortedSet<T>) [] -> List<T> => set

// The text form: `{1, 2, 3}` in **sorted** order [col-to-str].
export intrinsic fn to_str<T>(set: SortedSet<T>) [] -> Str => set

// [iter-pass] A fresh pass over the elements, in order. A snapshot, like the
// unordered collections' passes and for the same reason (there is no index
// to walk) — see `core.set`.
export fn iter<T>(set: SortedSet<T>) [] -> Mut SetYield<T> => set {
    return Mut SetYield<T> { items: to_list(set), at: 0 }
}

export intrinsic type SortedMap<K, V> canbe Mut

// Constructor, from entries written as pairs. A repeated key takes the value
// of its last appearance [col-duplicate-keys]; position is irrelevant here,
// since the order is the keys'.
export intrinsic fn sorted_map_of<K, V>(...entries: (K, V)[]) [] -> SortedMap<K, V>

// Mutable constructor
export intrinsic fn mut_sorted_map_of<K, V>(...entries: (K, V)[]) [] -> Mut SortedMap<K, V>

// Possibly gets the value stored under [key], **borrowed** out of the map
// [copy-opt-in].
export intrinsic fn get<K, V>(map: SortedMap<K, V>, key: K) [] -> (proj[from: map] V)?
    => map, key

// Stores [value] under [key], replacing any value already there.
export intrinsic fn put<K, V>(map: Mut SortedMap<K, V>, key: K, value: V) [] -> None
    => map: Mut, !key, !value

// Removes the entry under [key] and hands its value back.
export intrinsic fn remove<K, V>(map: Mut SortedMap<K, V>, key: K) [] -> V? => map: Mut, key

// Whether the map holds an entry under [key].
export intrinsic fn contains_key<K, V>(map: SortedMap<K, V>, key: K) [] -> Bool => map, key

// Returns the number of entries in the map
export intrinsic fn size<K, V>(map: SortedMap<K, V>) [] -> Int => map

// The smallest key, or `None` when the map is empty.
export intrinsic fn first_key<K, V>(map: SortedMap<K, V>) [] -> K? => map

// The largest key, or `None` when the map is empty.
export intrinsic fn last_key<K, V>(map: SortedMap<K, V>) [] -> K? => map

// The keys, in order.
export intrinsic fn keys<K, V>(map: SortedMap<K, V>) [] -> List<K> => map

// The text form: `{a: 1, b: 2}` in **key order** [col-to-str].
export intrinsic fn to_str<K, V>(map: SortedMap<K, V>) [] -> Str => map

// [iter-pass] A fresh pass over the map's **keys**, in order — the same
// reading `core.map` takes, where a value is reached with `get`.
export fn iter<K, V>(map: SortedMap<K, V>) [] -> Mut MapKeyYield<K> => map {
    return Mut MapKeyYield<K> { items: keys(map), at: 0 }
}
