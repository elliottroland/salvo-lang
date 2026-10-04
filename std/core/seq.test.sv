// [test-file] The tests of module `core.seq`: the lazy adaptors [seq-lazy].

// Every natural number from [n] up: an endless source, which only a lazy
// adaptor can stop.
iter fn naturals(n: Int) -> Emitted Int | Finished {
    state {
        at: Int = n
    }
    let v = at.copy()
    at = at + 1
    return emitted(v)
}

fn ints(xs: List<Int>) [] -> Str => xs {
    return to_str(xs)
}

test "taking stops an endless source" {
    let out: Mut List<Int> = mut_list_of()
    for x in taking(naturals(1), 3) {
        add(out, x)
    }
    expect_eq(to_str(out), "[1, 2, 3]")
}

test "taking_while stops at the first refusal" {
    let out: Mut List<Int> = mut_list_of()
    for x in taking_while(naturals(1), (x: Int) -> x * x < 20) {
        add(out, x)
    }
    expect_eq(to_str(out), "[1, 2, 3, 4]")
}

test "skipping and skipping_while drop a prefix" {
    let xs = list_of(1, 2, 3, 4, 5)
    let a: Mut List<Int> = mut_list_of()
    for x in skipping(iter(xs), 3) {
        add(a, copy(x))
    }
    let b: Mut List<Int> = mut_list_of()
    for x in skipping_while(iter(xs), (x: Int) -> x < 4) {
        add(b, copy(x))
    }
    expect_eq(to_str(a), "[4, 5]")
    expect_eq(to_str(b), "[4, 5]")
}

test "adaptors compose, and to_list drives them" {
    let pass = taking(skipping(naturals(10), 2), 3)
    let firsts = to_list(pass)
    expect_eq(ints(firsts), "[12, 13, 14]")
    let small = taking_while(naturals(0), (x: Int) -> x < 7)
    expect_eq(size(to_list(small)), 7)
}

test "a list's own pass is borrowed, so the list stays usable" {
    let xs = list_of("a", "b", "c")
    let two = taking(iter(xs), 2)
    expect_eq(size(to_list(two)), 2)
    expect_eq(size(xs), 3)
}

test "mapping and filtering are lazy over an endless source" {
    let out: Mut List<Int> = mut_list_of()
    for x in taking(filtering(mapping(naturals(1), (x: Int) -> x * x), (x: Int) -> x % 2 == 1), 3) {
        add(out, x)
    }
    expect_eq(to_str(out), "[1, 9, 25]")
}

test "a container's own to_list still wins" {
    let s = set_of("b", "a")
    expect_eq(size(to_list(s)), 2)
}
