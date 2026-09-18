// [qual-overload] `NonEmpty` over the *containers*, beside `core.list`'s
// `NonEmpty of List<T>`.
//
// One qualifier name, five subject types: which one a use means is decided by
// the subject, the way a function overload is decided by its arguments. That
// is what makes `NonEmpty` mean what it says about whatever it qualifies
// rather than needing a `NonEmptySet`, a `NonEmptyMap`, and so on.
//
// Why these live in a module of their own rather than beside their containers:
// the accessor overloads below have to *delegate* to the plain versions, and a
// scope selector names a module (`min@core.sorted`). Declared next to them the
// selector would re-pick this overload — a `NonEmpty` argument still ranks it
// first — and recurse forever. `core.list`'s `first` dodges that by delegating
// to `get` instead, which has no competing overload; there is no such
// alternative for `min`, so the module boundary is the answer.

// The claim over an unordered set. `add` cannot promise it — a mutating
// function may not hand back a qualifier it has never heard of — so the
// qualifier says it on `add`'s behalf [qual-refn].
export qualifier NonEmpty<T> of Set<T> {
    fn qualifies(set: Set<T>) [] -> Bool {
        return size(set) > 0
    }

    refn add(set: Mut Set<T>, elem: T) => set: +NonEmpty
}

// …and over a map, where `put` is the operation that establishes it.
export qualifier NonEmpty<K, V> of Map<K, V> {
    fn qualifies(map: Map<K, V>) [] -> Bool {
        return size(map) > 0
    }

    refn put(map: Mut Map<K, V>, key: K, value: V) => map: +NonEmpty
}

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
// argument [fn-overload-rank].
export fn min<T>(set: NonEmpty SortedSet<T>) [] -> T => set {
    return min@core.sorted(set)!
}

export fn max<T>(set: NonEmpty SortedSet<T>) [] -> T => set {
    return max@core.sorted(set)!
}

export fn first_key<K, V>(map: NonEmpty SortedMap<K, V>) [] -> K => map {
    return first_key@core.sorted(map)!
}

export fn last_key<K, V>(map: NonEmpty SortedMap<K, V>) [] -> K => map {
    return last_key@core.sorted(map)!
}
