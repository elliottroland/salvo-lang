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

test "a proven index reads the element with no optional" {
    // [col-idx] The total `get`: after the assert the claim is in `i`'s
    // type, the qualified overload wins [fn-overload-rank], and no `!`
    // appears anywhere.
    let xs = list_of(10, 20, 30)
    let i = 2
    assert!(i is Idx(xs))
    expect_eq(get(xs, i), 30)
}

test "an unproven index still answers the optional" {
    let xs = list_of(10, 20, 30)
    let i = 7
    expect(get(xs, i) is None, "out of range answers None")
}

test "an out-of-range index fails the Idx test" {
    let xs = list_of(1)
    let i = 5
    expect(!(i is Idx(xs)), "5 is not an index of a one-element list")
}

test "a proven pair swaps totally" {
    // [col-idx] The total `swap`: no `Bool` to check — the claims did the
    // checking [col-bounds].
    let xs: Mut List<Int> = [1, 2, 3]
    let i = 0
    let j = 2
    assert!(i is Idx(xs))
    assert!(j is Idx(xs))
    swap(xs, i, j)
    expect_eq(to_str(xs), "[3, 2, 1]")
}

test "add and swap preserve Idx claims" {
    // [qual-preserve] Growth keeps every existing index valid, and an
    // exchange moves no boundary — so the total `get` still applies after
    // both.
    let xs: Mut List<Int> = [1, 2, 3]
    let i = 1
    assert!(i is Idx(xs))
    add(xs, 4)
    // `i + 1` carries no claim, so this is the fallible overload: its answer is
    // a `Checked<Bool>` and has to be looked at [col-bounds].
    expect(detach(swap(xs, i + 1, i + 1)), "in range, so it swapped")
    expect_eq(get(xs, i), 2)
}

test "the founding example: a descending index loop is total" {
    // The loop the refinement-types design was opened with (2026-09-23):
    // from `size(xs) - 1` down to `0`, `get(xs, i)` answers the element —
    // no `!` anywhere. The pass's element carries `Idx(xs)` [qual-depend],
    // and the total `get` consumes it [col-idx].
    let xs = list_of(1, 2, 3)
    let digits = 0
    for i in rev_indices(xs) {
        digits = digits * 10 + get(xs, i)
    }
    expect_eq(digits, 321)
}

test "indices walks front to back with the same claim" {
    let xs = list_of(5, 6)
    let digits = 0
    for i in indices(xs) {
        digits = digits * 10 + get(xs, i)
    }
    expect_eq(digits, 56)
}

test "binary_search answers a proven index" {
    // [col-idx] The found arm carries `Idx(list)`: narrowing the optional
    // is the last check the result ever needs.
    let xs = sort(list_of(30, 10, 20))
    let found = binary_search(xs, 20)
    if found is Int {
        expect_eq(get(xs, found), 20)
    } else {
        expect(false, "20 is in the list")
    }
}

test "the qualifier pick applies an Idx claim or takes the right side" {
    // [pick-qualifies] `expr Idx(xs)?: break` is the runtime pick: when
    // the claim holds the value carries it — the total `get` below needs
    // no `!` — and when it does not, the right side runs.
    let xs = list_of(10, 20, 30)
    let sum = 0
    let i = 0
    while i < 10 {
        let child = i * 2 + 1 Idx(xs)?: break
        sum = sum + get(xs, child)
        i = i + 1
    }
    // i = 0 reads index 1 (20); i = 1 asks for index 3, which does not
    // exist, so the pick took the `break`.
    expect_eq(sum, 20)
}

test "a pick with a value right side joins like an if" {
    let xs = list_of(7)
    let picked = 5 Idx(xs)?: 0
    // 5 is out of range, so the right side's value is the expression's.
    expect_eq(picked + 0, 0)
}

test "an out-of-range swap answers false, and the answer cannot be dropped" {
    // [col-bounds] The failure a swap reports is invisible at the call site,
    // which is why the answer is a `Checked`: `detach` reads it, `ignore` says
    // the miss was expected. Dropping it is a compile-time error.
    let xs: Mut List<Int> = [1, 2]
    expect(!detach(swap(xs, 0, 9)), "out of range, so nothing moved")
    expect_eq(to_str(xs), "[1, 2]")
    ignore(swap(xs, 0, 1))
    expect_eq(to_str(xs), "[2, 1]")
}
