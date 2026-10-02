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

test "the module's own binding answers without being declared" {
    let a = fresh_bits()
    let b = fresh_bits()
    expect(!(a == b), "two draws differ")
}

test "a thread the runtime starts runs its body" {
    let me = this_parker()
    let waker = copy(me)
    start_thread(() -> { unpark(waker) })
    park(me)
    expect(true, "the thread woke the test")
}

test "a fault inside the boundary is answered, not raised" {
    let ok = guarded(() -> {})
    expect(ok is None, "a body that returns answers None")
    let why = guarded(() -> {
        let xs: List<Int> = []
        let _x = get(xs, 3)!
    })
    expect(!(why is None), "a faulting body answers its fault")
}
