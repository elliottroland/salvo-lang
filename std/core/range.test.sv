// [test-file] The tests of module `core.range` — its annex. Today these
// cover the constant-slotted qualifier `InRange` ([qual-const], the
// refinement-types sequence, step 6).

test "a value in the range passes the InRange test" {
    let n = 8080
    expect(n is InRange(0, 65535), "8080 is a port")
}

test "a value outside the range fails it" {
    let n = 70000
    expect(!(n is InRange(0, 65535)), "70000 is not a port")
}

test "the bounds are inclusive on both ends" {
    let lo = 0
    let hi = 100
    expect(lo is InRange(0, 100), "the low bound is in")
    expect(hi is InRange(0, 100), "the high bound is in")
}

test "different constants are different facts" {
    let n = 200
    assert!(n is InRange(0, 65535))
    expect(!(n is InRange(0, 100)), "a port is not thereby a percentage")
}
