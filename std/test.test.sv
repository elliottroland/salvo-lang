// [test-file] The test module's own annex — `std.test` testing itself, which it
// can because an annex is per module and this one is module `test`'s.
//
// What is worth testing here is the part with behaviour: the trap vocabulary
// [test-trap-expect]. `expect`/`expect_eq` are one `if` each and are exercised
// by every other test in the tree; `trapped_by` is the intrinsic underneath, and
// the three functions over it are what a test author writes.

// A function that traps, to have something to catch. Private to the annex, so
// the shipped module has no such thing [test-visibility].
fn always_traps(n: Int) -> Int {
    assert!(n > 100, "n should exceed 100, was ${n}")
    return n
}

test "trap_of answers the message of a body that trapped" {
    let trap = trap_of(() -> { always_traps(7) })
    expect(trap is Str, "a trap message came back")
    // Two `!`s on one optional local: a read, twice — which is what
    // [rs-opt-borrow] makes possible on Rust (it used to move the value the
    // first time and fail to compile the second).
    expect(contains(trap!, "n should exceed 100, was 7"), "it is the trap's own text")
    // [assert-trap] And it names where, in Salvo's terms.
    expect(contains(trap!, "test.test:"), "the message names the Salvo location")
}

test "trap_of answers None for a body that completed" {
    expect(trap_of(() -> { always_traps(200) }) is None, "200 exceeds 100")
}

test "expect_trap passes when the body traps" {
    expect_trap(() -> { always_traps(1) }, "n below the bound")
}

test "expect_trap fails when the body does not" {
    // The assertion under test is itself a throw, so it is caught with `try`
    // rather than by the harness — which is what makes an assertion's own
    // behaviour testable [test-fail].
    let outcome = try {
        expect_trap(() -> { always_traps(500) }, "a body that cannot trap")
    }
    expect(outcome is Thrown, "a body that did not trap fails the expectation")
}

test "expect_trap_with pins which trap it meant" {
    expect_trap_with(() -> { always_traps(2) }, "was 2", "n below the bound")
}

test "expect_trap_with fails on a trap with another message" {
    let outcome = try {
        expect_trap_with(() -> { always_traps(3) }, "a different failure", "n below the bound")
    }
    expect(outcome is Thrown, "the wrong trap is not the expected one")
}

test "a trapping body may register the handlers it needs" {
    // The body is a pure fn value, so it cannot *inherit* an effect from the
    // test — it registers its own, which is what the doc comment says to do.
    expect_trap(() -> {
        use StdOutConsole()
        always_traps(4)
    }, "a body with an effect of its own")
}
