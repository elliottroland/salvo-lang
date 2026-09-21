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
// token with `: default Hashed<self>`, or hand-written and `@`-scoped
// [cmp-default] [cmp-canonical]. **Values** are
// unrestricted — and, with `<V canbe linear>`, may be **obligations**
// [linear-container]: `Map<Int, Reply<Str>>` is a linear type whose terminal
// is [drain], while a key never can be (keys are compared and retained, and
// a repeated key would drop one).
//
// Own module so a program that never uses a map emits nothing for it
// [mod-used-only].
export intrinsic type Map<K, V canbe linear> canbe Mut

// Constructor, from entries written as pairs: `map_of(("a", 1), ("b", 2))`.
// The keys and values are stored in the new map, so they are moved: a
// variadic tail is owned and needs no entry in the clause [deduce-syntax].
// A repeated key keeps the position of its first appearance and takes the
// value of its last [col-duplicate-keys].
export intrinsic fn map_of<K, V>(...entries: (K, V)[]) [] -> Map<K, V>

// Mutable constructor
export intrinsic fn mut_map_of<K, V>(...entries: (K, V)[]) [] -> Mut Map<K, V>

// [col-by] Builds a map from [size] generated entries: `map_by(3, i -> (i, i
// * i))` maps each index to its square. A repeated key takes the value of its
// last appearance [col-duplicate-keys].
export intrinsic fn map_by<K, V>(size: Int, init: (Int) -> (K, V)) [] -> Map<K, V>
    => size, init

// Mutable variant
export intrinsic fn mut_map_by<K, V>(size: Int, init: (Int) -> (K, V)) [] -> Mut Map<K, V>
    => size, init

// [col-convert] A map from a list of pairs — the first element of each pair
// is the key, the second the value. A repeated key takes the value of its
// last appearance [col-duplicate-keys].
export intrinsic fn to_map<K, V>(pairs: List<(K, V)>) [] -> Map<K, V> => pairs

// [col-convert] A map from a list of *anything*, with [entry] saying what
// key and value each element becomes. The two forms are the same function
// spelled for the two sources people actually have: a list of pairs, or a
// list plus a rule.
export intrinsic fn to_map<T, K, V>(items: List<T>, entry: (T) -> (K, V)) [] -> Map<K, V>
    => items, entry

// Possibly gets the value stored under [key]. The value is **borrowed** —
// a view into the map, like [get] on a list — so reading a map copies
// nothing and a caller that wants to keep the value says `copy`
// [copy-opt-in]. The key is only read, so it is kept.
export intrinsic fn get<K, V>(map: Map<K, V>, key: K) [] -> (proj[from: map] V)? => map, key

// Stores [value] under [key], replacing any value already there. The map
// takes ownership of both, so both are moved; a key that is already present
// keeps its position in the iteration order [col-insertion-order].
//
// [linear-container] It answers nothing, so it **drops** whatever it
// replaced — which is why it is closed to linear values: storing one under an
// occupied key would discard an obligation in silence. [replace] is the form
// that hands the displaced value back, and the diagnostic names it.
export intrinsic fn put<K, V>(map: Mut Map<K, V>, key: K, value: V) [] -> None
    => map: Mut, !key, !value

// [linear-container] Stores [value] under [key] and answers what was there,
// or `None` for a fresh key: [put] with the displaced value handed back
// instead of dropped, which is the only shape a map of obligations can have a
// write in.
export intrinsic fn replace<K, V canbe linear>(map: Mut Map<K, V>, key: K, value: V) [] -> V?
    => map: Mut, !key, !value

// Removes the entry under [key] and hands its value back, or `None` when
// there was none. The value is **moved out** of the map — which is what
// makes a map usable as a table of things you take back out again — while
// the key is only read, so it is kept.
//
// [linear-container] This is take-by-move, so it is how an obligation leaves
// a map: the `V?` shape makes the absence check the union narrow
// [linear-union-arm], and nothing is aliased or dropped on the way.
export intrinsic fn remove<K, V canbe linear>(map: Mut Map<K, V>, key: K) [] -> V? => map: Mut, key

// Whether the map holds an entry under [key].
export intrinsic fn contains_key<K, V>(map: Map<K, V>, key: K) [] -> Bool => map, key

// Returns the number of entries in the map
export intrinsic fn size<K, V canbe linear>(map: Map<K, V>) [] -> Int => map

// [linear-container] The **terminal**, as a list's [drain] is: consumes the
// map and hands every value to [each], in insertion order. The keys go with
// the map — they were never obligations — so what the callback sees is the
// values, one at a time, each moved in.
export intrinsic fn drain<K, V canbe linear>(map: Map<K, V>, each: (x: V) -> None) [] -> None
    =>[each] !x => !map, each

// The text form of a map, for string interpolation [interp-to-str]:
// `{a: 1, b: 2}` in insertion order — the map *literal* that would build it
// [col-to-str].
export intrinsic fn to_str<K, V>(map: Map<K, V>) [] -> Str => map


// [col-insertion-order] The keys, in insertion order — which is also what
// makes a map iterable, below.
export intrinsic fn keys<K, V>(map: Map<K, V>) [] -> List<K> => map

// [iter-pass] A fresh pass over the map's **keys**, in insertion order.
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
// *sound* one. A pass yielding whole entries would have to hand back an
// owned `(K, V)`, and copying a value of unconstrained generic type is
// unsupported on the Kotlin backend by construction — erased generics
// cannot tell an immutable value from a mutable one at runtime, so it
// refuses rather than aliasing [kt-copy]. Keys escape that because they are
// the immutable intrinsic types [col-key-eligible]. Reaching values through
// `get` is not a workaround but the better shape anyway: it *borrows* them
// [copy-opt-in], where an entries pass would copy every value in the map.
// (An `entries`/`values` surface therefore waits on a way to copy a generic
// value, or on passes that borrow — recorded in ROADMAP.md.)
export fn iter<K, V>(map: Map<K, V>) [] -> Mut MapKeyYield<K> => map {
    return Mut MapKeyYield<K> { items: keys(map), at: 0 }
}

// [iter-protocol] The pass a map is walked by: a snapshot of its keys plus a
// position in it, owned by the pass — the same shape `SetYield` has, and for
// the same reason (a hash map has no index to walk).
export struct MapKeyYield<K> : Yield<self, K> canbe Mut {
    // The keys, in insertion order, owned by this pass.
    items: List<K>,
    // The index of the next key to emit.
    at: Int
}

// Advances the pass, reporting the key at its position or the end of the map.
export fn next<K>(p: Mut MapKeyYield<K>) [] -> Emitted K | Finished => p: Mut {
    let key = snapshot_at(p.items, p.at)
    if key is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(key)
}
