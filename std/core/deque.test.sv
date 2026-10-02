// [test-file] The tests of module `core.deque` [col-deque].

test "both ends add and remove in order" {
    let d: Mut Deque<Int> = mut_deque_of(2, 3)
    add_first(d, 1)
    add_last(d, 4)
    expect_eq(to_str(d), "[1, 2, 3, 4]")
    expect_eq(remove_first(d)!, 1)
    expect_eq(remove_last(d)!, 4)
    expect_eq(to_str(d), "[2, 3]")
}

test "an empty deque answers None at both ends" {
    let d: Mut Deque<Int> = mut_deque_of()
    expect(remove_first(d) is None, "remove_first")
    expect(remove_last(d) is None, "remove_last")
    expect(first(d) is None, "first")
    expect(last(d) is None, "last")
    expect_eq(size(d), 0)
}

test "get and remove_at count from the front and answer None out of range" {
    let d: Mut Deque<Str> = mut_deque_of("a", "b", "c")
    add_first(d, "z")
    expect_eq(copy(get(d, 1)!), "a")
    expect(get(d, 4) is None, "past the end")
    expect(get(d, -1) is None, "negative")
    expect_eq(remove_at(d, 2)!, "b")
    expect(remove_at(d, 9) is None, "remove past the end")
    expect_eq(to_str(d), "[z, a, c]")
}

test "iteration runs front to back and reversed back to front" {
    let d = to_deque([1, 2, 3])
    let fwd: Mut List<Int> = mut_list_of()
    for x in d {
        add(fwd, copy(x))
    }
    let back: Mut List<Int> = mut_list_of()
    for x in reversed(d) {
        add(back, copy(x))
    }
    expect_eq(to_str(fwd), "[1, 2, 3]")
    expect_eq(to_str(back), "[3, 2, 1]")
}

test "deque_by builds from the index and to_list keeps the order" {
    let d = deque_by(4, i -> i * i)
    expect_eq(to_str(to_list(d)), "[0, 1, 4, 9]")
}

test "a copy is its own deque" {
    let d: Mut Deque<Int> = mut_deque_of(1)
    let e: Mut Deque<Int> = copy(d)
    add_last(e, 2)
    expect_eq(size(d), 1)
    expect_eq(size(e), 2)
}

test "drain consumes the deque, handing every element over" {
    let d: Mut Deque<Str> = mut_deque_of("b", "c")
    add_first(d, "a")
    expect_eq(to_str(d), "[a, b, c]")
    drain(d, x -> discard(x))
}
