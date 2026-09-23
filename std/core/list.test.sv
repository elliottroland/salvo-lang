// [test-file] The tests of module `core.list` — its annex. Today these cover
// the step-0 iteration vocabulary of the refinement-types sequence
// ([col-reversed], [col-enumerate]); the module's older surface is covered by
// the compiler's own corpus and e2e tests.

test "reversed walks back to front" {
    let xs = list_of(1, 2, 3)
    let out: Mut List<Int> = mut_list_of()
    for x in reversed(xs) {
        add(out, copy(x))
    }
    expect_eq(to_str(out), "[3, 2, 1]")
}

test "reversed of an empty list emits nothing" {
    let xs: List<Int> = []
    let seen: Mut List<Int> = mut_list_of()
    for x in reversed(xs) {
        add(seen, copy(x))
    }
    expect_eq(size(seen), 0)
}

test "reversed is a pass, so the list is still usable after it" {
    let xs = list_of("a", "b")
    for _x in reversed(xs) {
    }
    expect_eq(size(xs), 2)
}

test "enumerate pairs ascending indices with elements" {
    let xs = list_of("a", "b", "c")
    let out: Mut List<Str> = mut_list_of()
    for pair in enumerate(xs) {
        add(out, "${pair.index}:${pair.elem}")
    }
    expect_eq(to_str(out), "[0:a, 1:b, 2:c]")
}

test "enumerate_rev descends from the last index to zero" {
    let xs = list_of("a", "b", "c")
    let out: Mut List<Str> = mut_list_of()
    for pair in enumerate_rev(xs) {
        add(out, "${pair.index}:${pair.elem}")
    }
    expect_eq(to_str(out), "[2:c, 1:b, 0:a]")
}

test "enumerate of an empty list emits nothing" {
    let xs: List<Str> = []
    let seen: Mut List<Str> = mut_list_of()
    for pair in enumerate(xs) {
        add(seen, "${pair.index}")
    }
    expect_eq(size(seen), 0)
}

test "the descending index loop reads every element" {
    // The founding example of the refinement-types design, in its step-0
    // shape: the element arrives with its index, no index arithmetic, and
    // the `get` it replaces is gone entirely.
    let xs = list_of(10, 20, 30)
    let total: Mut List<Int> = mut_list_of()
    for pair in enumerate_rev(xs) {
        add(total, pair.elem + pair.index)
    }
    expect_eq(to_str(total), "[32, 21, 10]")
}
