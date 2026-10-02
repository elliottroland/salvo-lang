// [test-file] The tests of module `runtime`.

test "an unpark before the park makes the park return at once" {
    let p = this_parker()
    unpark(p)
    park(p)
    expect(true, "the park returned")
}

test "a timed park returns when nobody unparks" {
    park_nanos(this_parker(), 1000000)
    expect(true, "the timed park returned")
}

test "an unpark token is used up by one park" {
    let p = this_parker()
    unpark(p)
    unpark(p)
    park(p)
    // A second park would wait: one token, not a count.
    park_nanos(p, 1000000)
    expect(true, "the second park timed out")
}
