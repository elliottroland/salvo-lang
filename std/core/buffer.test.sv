test "a buffer is made full and keeps its length" {
    let b = int_buffer(3, 7)
    expect(size(b) == 3, "three slots")
    expect(get(b, 0)! == 7, "filled")
    expect(get(b, 3) is None, "past the end")
    expect(get(b, -1) is None, "before the start")
    expect(size(int_buffer(-2, 0)) == 0, "a negative size is empty")
}

test "a proven index reads and writes with no optional, and the claim survives" {
    let b = int_buffer(4, 0)
    let i = 2
    assert!(i is Idx(b))
    expect(replace(b, i, 5) == 0, "the old value comes back")
    clear(b, 1)
    expect(get(b, i) == 1, "clear refills; the claim held through both writes")
}

test "a long buffer is the same over Long" {
    let b = long_buffer(2, 3L)
    let i = 1
    assert!(i is Idx(b))
    expect(replace(b, i, 9L) == 3L, "the old value")
    expect("${b}" == "[3, 9]", "prints as a list")
}

test "a copy is its own buffer, and a for loop reads every slot" {
    let b = int_buffer(3, 1)
    let c = copy(b)
    let i = 0
    assert!(i is Idx(c))
    ignore_int(replace(c, i, 4))
    expect("${b} ${c}" == "[1, 1, 1] [4, 1, 1]", "the copy did not alias")
    let total = 0
    for x in c {
        total = total + x
    }
    expect(total == 6, "every slot")
}

fn ignore_int(x: Int) [] -> None => x {
    return None
}
