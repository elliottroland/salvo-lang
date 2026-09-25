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
