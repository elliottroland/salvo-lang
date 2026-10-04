// [test-file] The tests of module `random` [random-default].

import runtime

test "draws lie in [0, 1) and differ" {
    use DefaultRandom()
    let a = random()
    let b = random()
    expect(a >= 0.0 && a < 1.0, "in range")
    expect(a != b, "two draws differ")
}

test actor(seed: 11) "the seed decides the draws" {
    use DefaultRandom()
    let a = random()
    let b = random()
    enter_virtual(11L)
    expect(random() == a, "the first draw repeats")
    expect(random() == b, "the second draw repeats")
}
