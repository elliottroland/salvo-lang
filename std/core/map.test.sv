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
    // The claim is in `k`'s type from here on; today that is visible to
    // tooling (hover) and to the invalidation rule — the consuming overloads
    // arrive with the signatures step.
    expect(get(m, k)! == 1, "the entry is there")
}
