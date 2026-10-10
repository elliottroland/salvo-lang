// Maps from keys to values, which **iterate in insertion order**
// [col-insertion-order] — the same on every backend. The semantics are
// `LinkedHashMap`'s, exactly: overwriting an existing key updates its value
// and *keeps its original position*, and removing an entry preserves the
// order of the rest. Kotlin gets that from `LinkedHashMap`; Rust has no
// ordered hash map in its standard library, so the backend ships one.
//
// `canbe Mut` opts Map into the language-level `Mut` auto-qualifier
// [type-canbe-mut]: `Mut Map<K, V>` is Kotlin's `MutableMap<K, V>`.
//
// [col-key-eligible] The **key** type must be hashable: the intrinsic key
// types (`Int`, `Long`, `Str`, `Char`, `Bool`) in this version, with
// `Double`/`Float` deliberately excluded (Rust's `f64` is neither `Eq` nor
// `Hash`, so a float-keyed map is not representable on both backends
// [backend-parity]). A struct joins in by having a `hash` and an `eq` — one
// token with `: Hashed<self> by auto`, or hand-written and `@`-scoped
// [obligation-by] [cmp-canonical]. **Values** are
// unrestricted — and, with `<V canbe linear>`, may be **obligations**
// [linear-container]: `Map<Int, Reply<Str>>` is a linear type whose terminal
// is [drain], while a key never can be (keys are compared and retained, and
// a repeated key would drop one).
//
// Own module so a program that never uses a map emits nothing for it
// [mod-used-only].
export iterable platform type Map<K, V canbe linear>(?hash: (K) -> Long, ?eq: (K, K) -> Bool) canbe Mut : Iter<self, K>

// Constructor, from entries written as pairs: `map_of(("a", 1), ("b", 2))`.
// The keys and values are stored in the new map, so they are moved: a
// variadic tail is owned and needs no entry in the clause [deduce-syntax].
// A repeated key keeps the position of its first appearance and takes the
// value of its last [col-duplicate-keys].
export platform fn map_of<K, V canbe linear>(...entries: (K, V)[], ?Hashed<K>) [] -> Map<K, V>(?hash, ?eq)

// Mutable constructor
export platform fn mut_map_of<K, V canbe linear>(...entries: (K, V)[], ?Hashed<K>) [] -> Mut Map<K, V>(?hash, ?eq)

// [col-by] Builds a map from [size] generated entries: `map_by(3, i -> (i, i
// * i))` maps each index to its square. A repeated key takes the value of its
// last appearance [col-duplicate-keys].
export platform fn map_by<K, V canbe linear>(size: Int, init: (Int) -> (K, V), ?Hashed<K>) [] -> Map<K, V>(?hash, ?eq)
=> size, init

// Mutable variant
export platform fn mut_map_by<K, V canbe linear>(size: Int, init: (Int) -> (K, V), ?Hashed<K>) [] -> Mut Map<K, V>(?hash, ?eq)
=> size, init

// [col-convert] A map from a list of pairs — the first element of each pair
// is the key, the second the value. A repeated key takes the value of its
// last appearance [col-duplicate-keys].
export platform fn to_map<K, V>(pairs: List<(K, V)>, ?Hashed<K>) [] -> Map<K, V>(?hash, ?eq)
=> pairs

// [col-convert] A map from a list of *anything*, with [entry] saying what
// key and value each element becomes. The two forms are the same function
// spelled for the two sources people actually have: a list of pairs, or a
// list plus a rule.
export fn to_map<T, K, V>(items: List<T>, entry: (T) -> (K, V), ?Hashed<K>) [] -> Map<K, V>(?hash, ?eq)
=> items, entry {
    let map: Mut Map<K, V>(?hash, ?eq) = mut_map_of()
    for x in items {
        let (k, v) = entry(x)
        put(map, k, v)
    }
    return map
}

// Possibly gets the value stored under [key]. The value is **borrowed** —
// a view into the map, like [get] on a list — so reading a map copies
// nothing and a caller that wants to keep the value says `copy`
// [copy-opt-in]. The key is only read, so it is kept.
export platform fn get<K, V>(map: Map<K, V>(?hash, ?eq), key: K) [] -> (proj(map) V)? => map, key

// [col-locate] [ref-handle] The mutable handle to the value stored under
// [key], or `None` when the key is absent — `at` for a map, the by-key
// mint (the list's is by index). `get` is the read-only sibling. A handle
// is a position in the map: an in-place write through it moves nothing,
// while adding or removing an entry ends it [fate-poison].
export fn at<K, V>(map: Map<K, Mut V>(?hash, ?eq), key: K) [] -> ref(map) Mut V? => map, key {
    return get(map, key)
}

// Stores [value] under [key], replacing any value already there. The map
// takes ownership of both, so both are moved; a key that is already present
// keeps its position in the iteration order [col-insertion-order].
//
// [linear-container] It answers nothing, so it **drops** whatever it
// replaced — which is why it is closed to linear values: storing one under an
// occupied key would discard an obligation in silence. [replace] is the form
// that hands the displaced value back, and the diagnostic names it.
export platform fn put<K, V>(map: Mut Map<K, V>(?hash, ?eq), key: K, value: V) [] -> None
=> map: Mut, !key, !value

// [linear-container] Stores [value] under [key] and answers what was there,
// or `None` for a fresh key: [put] with the displaced value handed back
// instead of dropped, which is the only shape a map of obligations can have a
// write in.
export platform fn replace<K, V canbe linear>(map: Mut Map<K, V>(?hash, ?eq), key: K, value: V) [] -> V?
=> map: Mut, !key, !value

// Removes the entry under [key] and hands its value back, or `None` when
// there was none. The value is **moved out** of the map — which is what
// makes a map usable as a table of things you take back out again — while
// the key is only read, so it is kept.
//
// [linear-container] This is take-by-move, so it is how an obligation leaves
// a map: the `V?` shape makes the absence check the union narrow
// [linear-union-arm], and nothing is aliased or dropped on the way.
export platform fn remove<K, V canbe linear>(map: Mut Map<K, V>(?hash, ?eq), key: K) [] -> V? => map: Mut, key

// Whether the map holds an entry under [key].
export platform fn contains_key<K, V>(map: Map<K, V>(?hash, ?eq), key: K) [] -> Bool => map, key

// [qual-depend] The claim that a key is **present in one particular map** —
// the first dependent qualifier: its value slot names the map the claim is
// about, so `KeyOf(m) Str` and `KeyOf(m2) Str` are different facts. Tested
// with the filled block (`k is KeyOf(m)`), whose runtime tier is exactly
// [contains_key]; established claims are bound to the map's fate roots and
// stripped by any mutation of it (the conservative direction — `preserve`
// entries opt specific calls back in, a later step of the sequence).
export qualifier KeyOf<K, V>(map: Map<K, V>) of K {
    fn qualifies(key: K, map: Map<K, V>(?hash, ?eq)) -> Bool {
        return contains_key(map, key)
    }

    // [qual-preserve] Writing under a key never removes one, so every
    // existing `KeyOf` claim survives a [put] — the opt-back from the
    // conservative rule that any mutation of the map strips them.
    refn put(map: Mut Map<K, V>, key: K, value: V) => map: preserve KeyOf
}

// The lowering behind the total [get]: a presence the claim already proved.
// Private — the claim is the only door.
platform fn get_present<K, V>(map: Map<K, V>(?hash, ?eq), key: K) [] -> proj(map) V => map, key

// [qual-depend] The **total** read: a key carrying the claim answers the
// value itself — the `None` arm was paid where the key was tested. Ranked
// above the optional [get] by its qualifier [fn-overload-rank].
export fn get<K, V>(map: Map<K, V>(?hash, ?eq), key: KeyOf(map) K) [] -> proj(map) V => map, key {
    return get_present(map, key)
}

// Returns the number of entries in the map
export platform fn size<K, V canbe linear>(map: Map<K, V>) [] -> Int => map

// [linear-container] The **terminal**, as a list's [drain] is: consumes the
// map and hands every value to [each], in insertion order. The keys go with
// the map — they were never obligations — so what the callback sees is the
// values, one at a time, each moved in.
export fn drain<K, V canbe linear>(map: Map<K, V>, each: (x: V) -> None) [] -> None
=>[each] !x => !map, each {
    drain(into_values(map), each)
}

// [linear-container] The values, in insertion order, out of a map it consumes:
// what [drain] is written with.
platform fn into_values<K, V canbe linear>(map: Map<K, V>) [] -> List<V> => !map

// The text form of a map, for string interpolation [interp-to-str]:
// `{a: 1, b: 2}` in insertion order — the map *literal* that would build it
// [col-to-str]. Each key and value renders by its own `to_str`: two
// implicits of one name at different types, which a call inside the body
// tells apart by its argument [implicit-resolve-body].
export fn to_str<K, V>(map: Map<K, V>(?hash, ?eq), ?to_str: (k: K) -> Str, ?to_str: (v: V) -> Str) [] -> Str
=> map {
    let out = mut_str("{")
    let i = 0
    for k in map {
        if i > 0 {
            append(out, ", ")
        }
        append(out, to_str(k))
        append(out, ": ")
        append(out, to_str(get(map, k)!))
        i = i + 1
    }
    append(out, "}")
    return out
}


// [op-equality] Two maps are equal when they hold the same keys with equal
// values, in any order: what `a == b` resolves to. The keys are compared by
// the map's identity, the values by their own `eq` — two implicits of one
// name, told apart by their types [implicit-same-name].
export fn eq<K, V>(a: Map<K, V>(?hash, ?eq), b: Map<K, V>(?hash, ?eq), ?eq: (x: V, y: V) -> Bool) [] -> Bool
=> a, b {
    if size(a) != size(b) {
        return false
    }
    for k in a {
        let theirs = get(b, k)
        if theirs is None {
            return false
        }
        if !eq(get(a, k)!, theirs) {
            return false
        }
    }
    return true
}

// [col-insertion-order] The keys, in insertion order — which is also what
// makes a map iterable, below.
export platform fn keys<K, V>(map: Map<K, V>) [] -> List<K> => map

// [iter-mint] A fresh iterator over the map's **keys**, in insertion order.
//
// Iterating a map yields its keys, and a value is reached with `get`:
//
// ```
// for k in iter(scores) {
//     let v = get(scores, k)
//     if v is Int {
//         println("${k}: ${v}")
//     }
// }
// ```
//
// That is Python's reading of `for k in d`, and here it is also the only
// *sound* one. An iterator yielding whole entries would have to hand back an
// owned `(K, V)`, and copying a value of unconstrained generic type is
// unsupported on the Kotlin backend by construction — erased generics
// cannot tell an immutable value from a mutable one at runtime, so it
// refuses rather than aliasing [kt-copy]. Keys escape that because they are
// the immutable intrinsic types [col-key-eligible]. Reaching values through
// `get` is not a workaround but the better shape anyway: it *borrows* them
// [copy-opt-in], where an entries iterator would copy every value in the map.
// (An `entries`/`values` surface therefore waits on a way to copy a generic
// value, or on iterators that borrow — recorded in ROADMAP.md.)
export fn iter<K, V>(map: Map<K, V>) [] -> Mut MapKeyYield<K> => map {
    return Mut MapKeyYield<K> { items: keys(map), at: 0 }
}

// [iter-protocol] The pass a map is walked by: a snapshot of its keys plus a
// position in it, owned by the iterator — the same shape `SetYield` has, and for
// the same reason (a hash map has no index to walk).
export struct MapKeyYield<K> : Yield<self, K> canbe Mut {
    // The keys, in insertion order, owned by this iterator.
    items: List<K>,
    // The index of the next key to emit.
    at: Int
}

// Advances the iterator, reporting the key at its position or the end of the map.
export fn next<K>(p: Mut MapKeyYield<K>) [] -> Emitted K | Finished => p: Mut {
    let key = snapshot_at(p.items, p.at)
    if key is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(key)
}

// [col-map-entries] The slots of a map, live or removed, in insertion order:
// what [entries] and [values] walk. [key_at] is `None` at a removed slot;
// [value_at] is only asked of a live one.
platform fn slot_count<K, V>(map: Map<K, V>) [] -> Int => map
platform fn key_at<K, V>(map: Map<K, V>, at: Int) [] -> (proj(map) K)? => map, at
platform fn value_at<K, V>(map: Map<K, V>, at: Int) [] -> proj(map) V => map, at

// [col-map-entries] One entry of a map, as it is in the map: a **view
// struct** of two borrows, as [Enumerated] is a view of a list element — so
// walking the entries copies nothing.
export struct MapEntry<K, V> {
    key: proj K,
    value: proj V
}

// [col-map-entries] The entries, in insertion order, each a borrowed
// key/value view: `for e in entries(m) { println("${e.key}: ${e.value}") }`.
export iter fn entries<K, V>(map: Map<K, V>) -> Emitted MapEntry<K, V> | Finished holds proj(map) {
    state {
        // The next slot to look at.
        at: Int = 0
    }
    while at < slot_count(map) {
        let here = at.copy()
        at = at + 1
        let key = key_at(map, here)
        if !(key is None) {
            let value = value_at(map, here)
            return emitted(MapEntry<K, V> { key: key, value: value })
        }
    }
    return finished()
}

// [col-map-entries] The values, in insertion order, borrowed.
export iter fn values<K, V>(map: Map<K, V>) -> Emitted (proj(map) V) | Finished holds proj(map) {
    state {
        at: Int = 0
    }
    while at < slot_count(map) {
        let here = at.copy()
        at = at + 1
        if !(key_at(map, here) is None) {
            let value = value_at(map, here)
            return emitted(value)
        }
    }
    return finished()
}

// [qual-overload] The same claim over a map, where `put` is the operation that
// establishes it and cannot say so itself [qual-refn].
export qualifier NonEmpty<K, V> of Map<K, V> {
    fn qualifies(map: Map<K, V>) [] -> Bool {
        return size(map) > 0
    }

    refn put(map: Mut Map<K, V>, key: K, value: V) => map: +NonEmpty
}
