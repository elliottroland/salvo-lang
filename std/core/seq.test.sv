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

test "take stops an endless source" {
    let out: Mut List<Int> = mut_list_of()
    for x in take(naturals(1), 3) {
        add(out, x)
    }
    expect_eq(to_str(out), "[1, 2, 3]")
}

test "take_while stops at the first refusal" {
    let out: Mut List<Int> = mut_list_of()
    for x in take_while(naturals(1), (x: Int) -> x * x < 20) {
        add(out, x)
    }
    expect_eq(to_str(out), "[1, 2, 3, 4]")
}

test "skip and skip_while drop a prefix" {
    let xs = list_of(1, 2, 3, 4, 5)
    let a: Mut List<Int> = mut_list_of()
    for x in skip(iter(xs), 3) {
        add(a, copy(x))
    }
    let b: Mut List<Int> = mut_list_of()
    for x in skip_while(iter(xs), (x: Int) -> x < 4) {
        add(b, copy(x))
    }
    expect_eq(to_str(a), "[4, 5]")
    expect_eq(to_str(b), "[4, 5]")
}

test "adaptors compose, and collect drives them" {
    let pass = take(skip(naturals(10), 2), 3)
    let firsts = collect(pass)
    expect_eq(ints(firsts), "[12, 13, 14]")
    let small = take_while(naturals(0), (x: Int) -> x < 7)
    expect_eq(size(collect(small)), 7)
}

test "a list's own pass is borrowed, so the list stays usable" {
    let xs = list_of("a", "b", "c")
    let two = take(iter(xs), 2)
    expect_eq(size(collect(two)), 2)
    expect_eq(size(xs), 3)
}
