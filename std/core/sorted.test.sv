// [test-file] The tests of module `core.sorted`: the behaviour both backends'
// host sorted pair must share (ROADMAP §0j step 7) — order, `cmp`-distinct
// members, a custom ordering, and code-point order for strings.

fn by_len(a: Str, b: Str) [] -> Int => a, b {
    return size(a) - size(b)
}

fn add_generic<T>(s: Mut SortedSet<T>(?cmp), x: T) [] -> Bool => s: Mut, !x {
    return add(s, x)
}

test "a sorted set keeps its members in order, each once" {
    let s = mut_sorted_set_of(5, 1, 3, 1)
    let _a = add(s, 2)
    let _b = remove(s, 3)
    expect_eq("${s} ${size(s)} ${min_opt(s)!} ${max_opt(s)!}", "{1, 2, 5} 3 1 5")
}

test "strings sort by code point" {
    let s = sorted_set_of("😀", "｡", "a")
    expect_eq("${s}", "{a, ｡, 😀}")
}

test "a custom ordering decides membership" {
    let s: Mut SortedSet<Str>(by_len) = mut_sorted_set_of(cmp = by_len)
    let _a = add(s, "bb")
    let _b = add(s, "a")
    expect(!add(s, "zz"), "as long as `bb`, so the same member")
    expect(!add_generic(s, "yy"), "through a generic call too")
    expect_eq("${s}", "{a, bb}")
}

test "a sorted map keeps its keys in order; a repeated key takes the last value" {
    let m = mut_sorted_map_of(("b", 2), ("a", 1), ("b", 20))
    put(m, "c", 3)
    let gone = remove(m, "a")
    expect_eq("${m} ${gone!} ${first_key_opt(m)!} ${last_key_opt(m)!}", "{b: 20, c: 3} 1 b c")
    expect(get(m, "c")! == 3 && get(m, "a") is None, "found and absent")
}

test "two sorted sets and maps compare by their members" {
    expect(sorted_set_of(3, 1) == sorted_set_of(1, 3), "same members")
    expect(sorted_map_of(("a", 1)) != sorted_map_of(("a", 2)), "a value differs")
}
