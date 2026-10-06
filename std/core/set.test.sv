// [test-file] The tests of module `core.set`: the behaviour both backends'
// host sets must share (ROADMAP §0j step 7) — insertion order, duplicates,
// removal and compaction, a custom identity, and the same set reached through
// a concrete call and a generic one.

fn fold_hash(s: Str) [] -> Long => s {
    return hash(to_lower(s))
}

fn fold_eq(a: Str, b: Str) [] -> Bool => a, b {
    return to_lower(a) == to_lower(b)
}

// Adds through a generic fn: the identity arrives forwarded, not resolved here.
fn add_generic<T>(s: Mut Set<T>(?hash, ?eq), x: T) [] -> Bool => s: Mut, !x {
    return add(s, x)
}

fn contains_generic<T>(s: Set<T>(?hash, ?eq), x: T) [] -> Bool => s, x {
    return contains(s, x)
}

test "a set iterates in insertion order, and a re-added element goes last" {
    let s = mut_set_of(3, 1, 2)
    let _gone = remove(s, 3)
    let _back = add(s, 3)
    expect_eq("${s}", "{1, 2, 3}")
}

test "a duplicate in a constructor or a literal collapses" {
    let a = set_of(2, 1, 2)
    let b = {"x", "y", "x"}
    expect_eq("${a} ${size(a)}", "{2, 1} 2")
    expect_eq("${b}", "{x, y}")
}

test "adding an element already there answers false and changes nothing" {
    let s = mut_set_of("a")
    expect(!add(s, "a"), "already present")
    expect(add(s, "b"), "new")
    expect_eq(size(s), 2)
}

test "removing many keeps the rest findable and in order" {
    let s: Mut Set<Int> = mut_set_of()
    for i in range(0, 40) {
        let _new = add(s, i)
    }
    for i in range(0, 35) {
        let _gone = remove(s, i)
    }
    expect_eq("${s}", "{35, 36, 37, 38, 39}")
    expect(contains(s, 37) && !contains(s, 3), "the survivors are found, the removed are not")
}

test "a custom identity keys the set" {
    let s: Mut Set<Str>(fold_hash, fold_eq) = {"Ab"}
    expect(!add(s, "aB"), "equal under the fold")
    expect(contains(s, "AB"), "found under the fold")
    expect_eq(size(s), 1)
}

test "a set built concretely is found through a generic call, and back" {
    let s = mut_set_of("a")
    expect(add_generic(s, "b"), "added generically")
    expect(contains(s, "b") && contains_generic(s, "a"), "both paths agree")
    let f: Mut Set<Str>(fold_hash, fold_eq) = {"Q"}
    expect(!add_generic(f, "q"), "the generic path keeps the custom identity")
}

test "to_set and set_by keep first-appearance order" {
    expect_eq("${to_set([3, 3, 1, 2, 1])}", "{3, 1, 2}")
    expect_eq("${set_by(4, i -> i % 2)}", "{0, 1}")
}

test "two sets are equal with the same elements in any order" {
    let a = set_of(1, 2, 3)
    let b = mut_set_of(3, 1)
    expect(a != b, "different sizes")
    let _new = add(b, 2)
    expect(a == b, "same elements, other order")
}
