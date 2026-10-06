// [test-file] The tests of module `core.map` — its annex. Today these cover
// the dependent qualifier `KeyOf` ([qual-depend], the refinement-types
// sequence, step 2): the filled `is` test, its `assert!` form, and the
// narrowing both install.

test "a contained key passes the KeyOf test" {
    let m = mut_map_of(("a", 1), ("b", 2))
    let k = "a"
    expect(k is KeyOf(m), "the map holds an entry under `a`")
}

test "an absent key fails the KeyOf test" {
    let m = mut_map_of(("a", 1))
    let k = "z"
    expect(!(k is KeyOf(m)), "the map holds nothing under `z`")
}

test "the same key is a different fact per map" {
    let with_it = mut_map_of(("a", 1))
    let without_it = mut_map_of(("b", 2))
    let k = "a"
    expect(k is KeyOf(with_it), "present in the first map")
    expect(!(k is KeyOf(without_it)), "absent from the second")
}

test "assert! establishes the claim for the rest of the scope" {
    let m = mut_map_of(("a", 1))
    let k = "a"
    assert!(k is KeyOf(m))
    // The claim is in `k`'s type from here on, and the total [get] consumes
    // it: no `None` arm, nothing to `!`. Written as a **comparison** rather
    // than `expect_eq` on purpose — a borrowed Copy scalar in that position
    // is what [rs-cmp-deref] fixed (the defect this line found).
    expect(get(m, k) == 1, "the total read answers the stored value")
}

test "put preserves KeyOf claims, so the total read follows a write" {
    // [qual-preserve] Writing under a key never removes one: the claim
    // survives the mutation, and the total `get` still applies below it.
    let m = mut_map_of(("a", 1))
    let k = "a"
    assert!(k is KeyOf(m))
    put(m, "b", 2)
    expect_eq(get(m, k), 1)
}

// ---- the shared behaviour (ROADMAP §0j step 7): what both backends' host
// maps must agree on.

fn len_hash(s: Str) [] -> Long => s {
    return to_long(size(s))
}

fn len_eq(a: Str, b: Str) [] -> Bool => a, b {
    return size(a) == size(b)
}

fn put_generic<K, V>(m: Mut Map<K, V>(?hash, ?eq), k: K, v: V) [] -> None => m: Mut, !k, !v {
    put(m, k, v)
}

test "an overwrite keeps the key's place; a removed key comes back last" {
    let m = mut_map_of(("a", 1), ("b", 2), ("c", 3))
    put(m, "a", 10)
    let _gone = remove(m, "b")
    put(m, "b", 20)
    expect_eq("${m}", "{a: 10, c: 3, b: 20}")
}

test "a repeated key keeps its first place and takes its last value" {
    let m = map_of(("k", 1), ("j", 2), ("k", 3))
    let lit: Map<Str, Int> = {"x": 1, "y": 2, "x": 9}
    expect_eq("${m} ${lit}", "{k: 3, j: 2} {x: 9, y: 2}")
}

test "replace answers what it displaced" {
    let m = mut_map_of(("a", 1))
    let old = replace(m, "a", 5)
    let fresh = replace(m, "b", 6)
    expect(old! == 1 && fresh is None, "the old value, then nothing")
    expect_eq("${m}", "{a: 5, b: 6}")
}

test "removing many keeps the rest findable and in order" {
    let m: Mut Map<Int, Int> = mut_map_of()
    for i in range(0, 50) {
        put(m, copy(i), i * 2)
    }
    for i in range(1, 50) {
        if i % 10 != 0 {
            let _gone = remove(m, i)
        }
    }
    expect_eq("${m}", "{0: 0, 10: 20, 20: 40, 30: 60, 40: 80}")
    expect(get(m, 30)! == 60 && get(m, 31) is None, "survivors found, the removed not")
}

test "a custom identity keys the map, through a generic call too" {
    let m: Mut Map<Str, Int>(len_hash, len_eq) = {"ab": 1}
    put(m, "xy", 2)
    put_generic(m, "zz", 3)
    expect_eq("${m} ${size(m)}", "{zz: 3} 1")
}

test "drain hands every value over, in insertion order" {
    let m: Mut Map<Int, Mut List<Int>> = mut_map_of()
    put(m, 2, mut_list_of(20))
    put(m, 1, mut_list_of(10))
    let seen = into_values(m)
    expect_eq("${size(seen)} ${get(get(seen, 0)!, 0)!} ${get(get(seen, 1)!, 0)!}", "2 20 10")
}

test "two maps are equal with the same entries in any order" {
    let a = map_of(("x", 1), ("y", 2))
    let b = mut_map_of(("y", 2), ("x", 1))
    expect(a == b, "same entries, other order")
    put(b, "x", 5)
    expect(a != b, "a value differs")
    let _gone = remove(b, "x")
    expect(a != b, "a key is missing")
}

test "entries and values walk the live entries in insertion order, borrowed" {
    let m = mut_map_of(("a", 1), ("b", 2), ("c", 3))
    let _gone = remove(m, "b")
    let seen = mut_str("")
    for e in entries(m) {
        append(seen, "${e.key}=${e.value} ")
    }
    let total = 0
    for v in values(m) {
        total = total + v
    }
    expect_eq("${seen}${total}", "a=1 c=3 4")
}
