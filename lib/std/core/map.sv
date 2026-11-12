// Maps from keys to values, which remember the order their keys were first
// added: iterating, printing and comparing a map visit its keys in that
// order, the same on every backend.
//
// A `Map<K, V>` is a hash table kept in insertion order, like Java's
// `LinkedHashMap`. Writing to a key that is already there replaces its value
// and keeps the key where it was; removing an entry keeps the order of the
// rest. Reading, writing, removing and [contains_key] take O(1) on average.
//
// ```
// let ages = Mut Map("ada": 36, "grace": 45)
// put(ages, "linus", 28)
// put(ages, "ada", 37)                     // replaces the value; ada stays first
// let ada = get(ages, "ada") ?: 0          // 37
// let gone = remove(ages, "grace")         // 45, moved out
// println("${ages}")                       // {ada: 37, linus: 28}
// for name in ages {
//     let age = get(ages, name)
//     if age is Int {
//         println("${name} is ${age}")     // ada is 37, then linus is 28
//     }
// }
// ```
//
// **Entries are written as pairs.** `Map("ada": 36)` is `Map(("ada", 36))`:
// the `key: value` sugar builds the pair. A key repeated among the entries
// keeps the position of its first appearance and the value of its last.
//
// **Keys must be hashable.** `Int`, `Long`, `Str`, `Char` and `Bool` are;
// `Double` and `Float` are not, since floating-point equality would not mean
// the same on both backends. A struct becomes a key type by having a `hash`
// and an `eq`: write `: Hashed<self> by auto` on it, or write the two
// functions yourself (see the Comparison and Hashing topic). A map may also
// be keyed by functions you name, `Mut Map(hash = my_hash, eq = my_eq)`;
// they become part of its type, so two maps keyed differently do not mix.
//
// **Reads borrow, writes move.** [get] answers a read-only view of the value
// in the map, so reading copies nothing; write `copy(get(m, k))` to keep one
// of your own, and use [at] to change a value in place. [put] and [replace]
// move the key and value into the map; [remove] moves the value back out,
// and only reads the key you pass.
//
// **Proven keys.** `k is KeyOf(m)` tests that a key is present in one
// particular map; with the claim in its type, [get] and [at] answer the value
// itself. [put] keeps the claim, since writing never removes a key; any other
// change to the map gives it up.
//
// ```
// let stock = Mut Map("pear": 3)
// let k = "pear"
// if k is KeyOf(stock) {
//     put(stock, "fig", 5)                 // KeyOf claims survive a put
//     let n = get(stock, k)                // 3, an Int: no None to handle
// }
// ```
//
// **Values may be obligations.** A map's values may be linear (values that
// must be used exactly once), and the map is then linear itself. [replace]
// is its write, since it hands back what it displaced, [remove] takes a value
// out, and [drain] ends the map. [put] would drop the value it replaces, so
// it refuses linear values. Keys are never linear: a repeated key would
// drop one of the two.
// @compilerrules [col-key-eligible, col-insertion-order, backend-parity]

// `canbe Mut` opts Map into the language-level `Mut` auto-qualifier:
// `Mut Map<K, V>` is Kotlin's `MutableMap<K, V>`. Rust has no ordered hash
// map in its standard library, so the backends ship one (`SalvoMap` on
// Kotlin, a slab with an open-addressing index on Rust). Own module so a
// program that never uses a map emits nothing for it.

// A map from keys of type `K` to values of type `V`, kept in insertion
// order.
//
// `Map<K, V>` cannot be changed; `Mut Map<K, V>` can, and is what [put],
// [replace] and [remove] take. The `(?hash, ?eq)` slot is the pair of
// functions the keys are hashed and compared by; nobody writes it for the
// usual case, where it is the key type's own. Two maps are `==` when they
// hold equal values under the same keys; a map prints as `{a: 1, b: 2}`;
// `for k in m` walks its keys.
//
// ```
// let empty: Map<Str, Int> = Map()
// let ages = Map("ada": 36, "grace": 45)
// println("${size(ages)} ${ages}")         // 2 {ada: 36, grace: 45}
// ```
// @compilerrules [type-canbe-mut, col-key-eligible, obligation-by, linear-container, mod-used-only, qual-refn-type]
// @compilerknown
export iterable platform type Map<K, V canbe linear>(?hash: (K) -> Long, ?eq: (K, K) -> Bool) canbe Mut : Iter<
    self,
    MapKeyIter<K>,
    K
> {
    // `put` is the operation that makes a map non-empty, and it cannot say
    // so itself.
    refn put(map: Mut Map<K, V>, key: K, value: V) => map: +NonEmpty
}

// Builds a map from entries written as pairs, in order; with none, an
// empty map.
//
// `Map("a": 1, "b": 2)` and `Map(("a", 1), ("b", 2))` are the same call.
// The keys and values are moved into the map, a spread with them:
// `Map(e, ...es)` spends the array `es`. A key repeated among the entries
// keeps the position of its first appearance and takes the value of its
// last. The keys are hashed by the key type's `hash` and `eq`, found at the
// call site, or by the ones passed as `hash = ...` and `eq = ...`.
//
// ```
// let ages = Map("ada": 36, "grace": 45)
// let twice = Map("a": 1, "b": 2, "a": 3)  // {a: 3, b: 2}
// ```
// @compilerrules [deduce-syntax, fn-type-named, col-duplicate-keys]
export platform fn Map<K, V canbe linear>(...entries: (K, V)[], ?Hashed<K>) [] -> Map<K, V>(?hash, ?eq) => !entries

// The constructor **is** the type-named function: `Map(...)` resolves to
// this declaration (canonical key `Map`; `Mut_Map` for the mutable pair),
// the key carried verbatim to the platform shim's `<Key>_platform` wrapper.

// [Map], answering a mutable map: `Mut Map()` is an empty map to fill.
//
// ```
// let seen: Mut Map<Str, Int> = Mut Map()
// put(seen, "a", 1)
// ```
export platform fn Mut Map<K, V canbe linear>(
    ...entries: (K, V)[],
    ?Hashed<K>
) [] -> Mut Map<K, V>(?hash, ?eq) => !entries

// Builds a map from a given number of entries, each computed from its index.
//
// The function is called once per index, from 0 up, and answers a
// `(key, value)` pair. A key it answers more than once keeps the value of
// its last appearance.
//
// ```
// let squares = map_by(3, (i) -> (i, i * i))   // {0: 0, 1: 1, 2: 4}
// ```
// @compilerrules [col-by]
export platform fn map_by<K, V canbe linear, [E]>(
    size: Int,
    init: (Int) [E] -> (K, V),
    ?Hashed<K>
) -> Map<K, V>(?hash, ?eq)
=> size, init

// [map_by], answering a mutable map.
export platform fn mut_map_by<K, V canbe linear, [E]>(
    size: Int,
    init: (Int) [E] -> (K, V),
    ?Hashed<K>
) -> Mut Map<K, V>(?hash, ?eq)
=> size, init

// Builds a map from a list of `(key, value)` pairs, in list order.
//
// A key repeated in the list keeps the value of its last appearance.
//
// ```
// let pairs = List(("a", 1), ("b", 2))
// let m = to_map(pairs)                    // {a: 1, b: 2}
// ```
// @compilerrules [col-convert]
export platform fn to_map<K, V>(pairs: List<(K, V)>, ?Hashed<K>) [] -> Map<K, V>(?hash, ?eq)
=> pairs

// [to_map] from a list of anything, with a function saying what key and
// value each element becomes.
//
// ```
// let words = List("pear", "fig")
// let lengths = to_map(words, (w) -> (w, size(w)))   // {pear: 4, fig: 3}
// ```
// @compilerrules [col-convert]
export fn to_map<T, K, V, [E]>(items: List<T>, entry: (T) [E] -> (K, V), ?Hashed<K>) -> Map<K, V>(?hash, ?eq)
=> items, entry {
    // The two forms are the same function spelled for the two sources people
    // actually have: a list of pairs, or a list plus a rule.
    let map: Mut Map<K, V>(?hash, ?eq) = Mut Map()
    for x in items {
        let (k, v) = entry(x)
        put(map, k, v)
    }
    return map
}

// The value stored under a key, or `None` when the key is absent.
//
// The value is borrowed: a read-only view of the map's own value, so
// reading copies nothing. Write `copy(get(m, k))` to keep one of your own,
// and use [at] to change it in place. The key is only read. A key proven
// present (`k is KeyOf(m)`) picks the overload below, which answers the
// value itself.
//
// ```
// let ages = Map("ada": 36)
// let ada = get(ages, "ada") ?: 0          // 36
// let bob = get(ages, "bob")               // None
// ```
// @compilerrules [copy-opt-in]
// @compilerknown
export platform fn get<K, V>(map: Map<K, V>(?hash, ?eq), key: K) [] -> (proj(map) V)? => map, key

// A handle to the value stored under a key, through which it can be changed
// in place, or `None` when the key is absent.
//
// [get] reads and `at` handles. A handle is a position in the map: a write
// through it moves nothing, while adding or removing an entry ends it. The
// handle is writable when the value type is `Mut X`.
//
// ```
// struct Item canbe Mut {
//     qty: Int
// }
//
// fn restock(stock: Map<Str, Mut Item>) -> None {
//     let item = at(stock, "pear") ?: return
//     item.qty = item.qty + 10             // the map's own item changes
// }
// ```
// @compilerrules [col-locate, ref-handle, fate-poison]
// @compilerknown
export platform fn at<K, V>(map: Map<K, V>(?hash, ?eq), key: K) [] -> ref(map) V? => map, key

// Stores a value under a key, replacing any value already there.
//
// The key and value are moved into the map. A key already present keeps
// its position in the iteration order. The value replaced is dropped, so a
// map of linear values cannot use `put`: [replace] is the write that hands
// the old value back. `KeyOf` claims on the map survive a `put`, and the map
// is known non-empty after it.
//
// ```
// let m: Mut Map<Str, Int> = Mut Map()
// put(m, "b", 1)
// put(m, "a", 2)
// put(m, "b", 3)                           // {b: 3, a: 2}
// ```
// @compilerrules [linear-container]
export platform fn put<K, V>(map: Mut Map<K, V>(?hash, ?eq), key: K, value: V) [] -> None
=> map: Mut, !key, !value

// Stores a value under a key and answers the value that was there, or
// `None` for a new key.
//
// [put] with the displaced value handed back instead of dropped: the write
// a map of linear values uses, since nothing is lost.
//
// ```
// let m = Mut Map("a": 1)
// let old = replace(m, "a", 2)             // 1
// let new = replace(m, "b", 3)             // None
// ```
// @compilerrules [linear-container]
export platform fn replace<K, V canbe linear>(map: Mut Map<K, V>(?hash, ?eq), key: K, value: V) [] -> V?
=> map: Mut, !key, !value

// Removes the entry under a key and answers its value, or `None` when there
// was none.
//
// The value is moved out of the map, which is how a linear value leaves
// one, while the key you pass is only read. The other entries keep their
// order.
//
// ```
// let m = Mut Map("a": 1, "b": 2)
// let a = remove(m, "a")                   // 1; m is {b: 2}
// ```
// @compilerrules [linear-container, linear-union-arm]
export platform fn remove<K, V canbe linear>(map: Mut Map<K, V>(?hash, ?eq), key: K) [] -> V? => map: Mut, key

// Whether the map holds an entry under a key.
//
// To use the value as well, test `k is KeyOf(m)` instead: it answers the
// same question and lets [get] answer the value itself.
export platform fn contains_key<K, V>(map: Map<K, V>(?hash, ?eq), key: K) [] -> Bool => map, key

// The claim that a key is present in one particular map.
//
// `k is KeyOf(m)` tests it, by [contains_key]. The claim is about `m`
// itself: `KeyOf(m) Str` and `KeyOf(m2) Str` are different facts. With it,
// [get] and [at] answer the value itself. Writing with [put] keeps every
// `KeyOf` claim on the map, since it never removes a key; any other change
// to the map gives them up. See the Dependent Qualifiers topic.
//
// ```
// let m = Mut Map("a": 1)
// let k = "a"
// if k is KeyOf(m) {
//     put(m, "b", 2)                       // keeps the claim
//     let v = get(m, k)                    // 1, an Int
// }
// ```
// @compilerrules [qual-depend]
export qualifier KeyOf<K, V>(map: Map<K, V>) for K {
    fn qualifies(key: K, map: Map<K, V>(?hash, ?eq)) -> Bool {
        return contains_key(map, key)
    }

    // Writing under a key never removes one, so every existing `KeyOf` claim
    // survives a [put]: the opt-back from the conservative rule that any
    // mutation of the map strips them.
    // @compilerrules [qual-preserve]
    refn put(map: Mut Map<K, V>, key: K, value: V) => map: preserve KeyOf
}

// The lowering behind the total [get]: a presence the claim already proved.
// Private — the claim is the only door.
// @compilerknown
platform fn get_present<K, V>(map: Map<K, V>(?hash, ?eq), key: K) [] -> proj(map) V => map, key

// [get] at a key proven present: answers the value itself, with no `None` to
// handle.
// @compilerrules [qual-depend, fn-overload-rank]
// @compilerknown
export fn get<K, V>(map: Map<K, V>(?hash, ?eq), key: KeyOf(map) K) [] -> proj(map) V => map, key {
    return get_present(map, key)
}

// [at] at a key proven present: answers the handle itself, with no `None`
// to handle.
// @compilerrules [qual-depend, ref-handle]
// @compilerknown
export fn at<K, V>(map: Map<K, V>(?hash, ?eq), key: KeyOf(map) K) [] -> ref(map) V => map, key {
    return at_present(map, key)
}

// The mint behind the total [at]: a presence the claim already proved.
// @compilerknown
platform fn at_present<K, V>(map: Map<K, V>(?hash, ?eq), key: K) [] -> ref(map) V => map, key

// The number of entries in the map.
export platform fn size<K, V canbe linear>(map: Map<K, V>) [] -> Int => map

// Consumes the map and hands every value to a function, in insertion order.
//
// This is how a map of linear values ends. The keys go with the map, since
// they were never obligations; the function receives each value, moved in.
//
// ```
// linear struct Ticket {
//     id: Int
// }
//
// fn redeem(t: Ticket) [Console] -> None => !t {
//     println("redeemed ${t.id}")
// }
//
// fn redeem_all(by_name: Map<Str, Ticket>) [Console] -> None => !by_name {
//     drain(by_name, redeem)
// }
// ```
// @compilerrules [linear-container]
export fn drain<K, V canbe linear, [E]>(map: Map<K, V>, each: (x: V) [E] -> None) -> None
=> !map, each =>[each] !x {
    drain(into_values(map), each)
}

// The values, in insertion order, out of a map it consumes:
// what [drain] is written with.
// @compilerrules [linear-container]
platform fn into_values<K, V canbe linear>(map: Map<K, V>) [] -> List<V> => !map

// The text form of a map, `{a: 1, b: 2}` in insertion order: what `"${m}"`
// prints.
//
// Each key and value is rendered by its own `to_str`, found at the call
// site.
// @compilerrules [interp-to-str, col-to-str, implicit-resolve-body]
export fn to_str<K, V>(map: Map<K, V>(?hash, ?eq), ?to_str: (k: K) -> Str, ?to_str: (v: V) -> Str) [] -> Str
=> map {
    // Two implicits of one name at different types, which a call inside the
    // body tells apart by its argument.
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

// Whether two maps hold the same keys with equal values, in any order:
// what `a == b` means for maps.
//
// Keys are compared by the maps' own `hash` and `eq`, values by their type's
// `==`, found at the call site.
//
// ```
// let same = Map("a": 1, "b": 2) == Map("b": 2, "a": 1)   // true
// ```
// @compilerrules [op-equality, implicit-same-name]
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
        if !eq(get(a, k) ?: unreachable!(), theirs) {
            return false
        }
    }
    return true
}

// The keys, in insertion order, as a new list.
//
// ```
// let m = Map("b": 1, "a": 2)
// let ks = keys(m)                         // [b, a]
// ```
export platform fn keys<K, V>(map: Map<K, V>) [] -> List<K> => map

// A fresh iterator over the map's keys, in insertion order.
//
// A `for` loop over a map walks the same keys, and a value is reached with
// [get]. The iterator walks a copy of the keys taken when it is made (O(n)
// once), so changing the map while walking it does not change what the walk
// visits. To walk keys and values together, use [entries]; for the values
// alone, [values].
//
// ```
// let scores = Map("ada": 3, "grace": 5)
// for k in iter(scores) {
//     let v = get(scores, k)
//     if v is Int {
//         println("${k}: ${v}")            // ada: 3, then grace: 5
//     }
// }
// ```
// @compilerrules [iter-mint, kt-copy, col-key-eligible, copy-opt-in]
export fn iter<K, V>(map: Map<K, V>) [] -> Mut MapKeyIter<K> => map {
    // Iterating a map yields its keys: Python's reading of `for k in d`, and
    // here also the sound one. An iterator yielding whole entries as owned
    // `(K, V)` would have to copy a value of unconstrained generic type,
    // which the Kotlin backend refuses by construction (erased generics
    // cannot tell an immutable value from a mutable one at runtime). Keys
    // escape that because they are the immutable intrinsic types. `entries`
    // and `values` borrow instead.
    return Mut MapKeyIter<K> { items: keys(map), at: 0 }
}

// The iterator [iter] answers: a copy of a map's keys, owned by the
// iterator, and a position in it.
//
// A hash map has no index to walk, so the iterator walks a snapshot, as a
// set's does.
// @compilerrules [iter-protocol]
export struct MapKeyIter<K> : Next<self, K> canbe Mut {
    // The keys, in insertion order, owned by this iterator.
    items: List<K>
    // The index of the next key to emit.
    at: Int
}

// Answers the key at the iterator's position and moves past it, or the end
// once every key has been emitted.
export fn next<K>(p: Mut MapKeyIter<K>) [] -> Emitted K | Finished => p: Mut {
    let key = snapshot_at(p.items, p.at)
    if key is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(key)
}

// The slots of a map, live or removed, in insertion order:
// what [entries] and [values] walk. [key_at] is `None` at a removed slot;
// [value_at] is only asked of a live one.
// @compilerrules [col-map-entries]
platform fn slot_count<K, V>(map: Map<K, V>) [] -> Int => map
platform fn key_at<K, V>(map: Map<K, V>, at: Int) [] -> (proj(map) K)? => map, at
platform fn value_at<K, V>(map: Map<K, V>, at: Int) [] -> proj(map) V => map, at

// One entry of a map, as [entries] emits it: the key and the value, both
// borrowed from the map.
//
// Walking the entries therefore copies nothing.
// @compilerrules [col-map-entries]
export struct MapEntry<K, V> {
    // The entry's key, borrowed.
    key: proj K
    // The entry's value, borrowed.
    value: proj V
}

// Walks a map's entries in insertion order, each a key and value borrowed
// from the map.
//
// ```
// let ages = Map("ada": 36, "grace": 45)
// for e in entries(ages) {
//     println("${e.key}: ${e.value}")      // ada: 36, then grace: 45
// }
// ```
// @compilerrules [col-map-entries]
export fn entries<K, V>(map: Map<K, V>) [] -> Mut MapEntriesIter<K, V> => map {
    return Mut MapEntriesIter<K, V> { map: map, at: 0 }
}

// The iterator [entries] answers: the map, borrowed, and the next slot to
// look at.
export struct MapEntriesIter<K, V> : Next<self, MapEntry<K, V>> canbe Mut {
    // The map being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    map: proj Map<K, V>
    // The next slot to look at.
    at: Int
}

// Answers the next entry and moves past it, or the end once every entry has
// been emitted.
// @compilerrules [col-map-entries]
export fn next<K, V>(p: Mut MapEntriesIter<K, V>) [] -> Emitted MapEntry<K, V> | Finished holds proj(p) => p: Mut {
    // Removed slots are skipped.
    while p.at < slot_count(p.map) {
        let here = p.at.copy()
        p.at = p.at + 1
        let key = key_at(p.map, here)
        if !(key is None) {
            let value = value_at(p.map, here)
            return emitted(MapEntry<K, V> { key: key, value: value })
        }
    }
    return finished()
}

// Walks a map's values in insertion order, each borrowed from the map.
//
// ```
// let ages = Map("ada": 36, "grace": 45)
// let total = 0
// for age in values(ages) {
//     total = total + age                  // 81 at the end
// }
// ```
// @compilerrules [col-map-entries]
export fn values<K, V>(map: Map<K, V>) [] -> Mut MapValuesIter<K, V> => map {
    return Mut MapValuesIter<K, V> { map: map, at: 0 }
}

// The iterator [values] answers: the map, borrowed, and the next slot to
// look at.
export struct MapValuesIter<K, V> : Next<self, proj V> canbe Mut {
    // The map being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    map: proj Map<K, V>
    // The next slot to look at.
    at: Int
}

// Answers the next value and moves past it, or the end once every value has
// been emitted.
// @compilerrules [col-map-entries]
export fn next<K, V>(p: Mut MapValuesIter<K, V>) [] -> Emitted (proj(p) V) | Finished => p: Mut {
    // Removed slots are skipped.
    while p.at < slot_count(p.map) {
        let here = p.at.copy()
        p.at = p.at + 1
        if !(key_at(p.map, here) is None) {
            let value = value_at(p.map, here)
            return emitted(value)
        }
    }
    return finished()
}
