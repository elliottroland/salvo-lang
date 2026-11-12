// [prop-rng] [prop-gen] [prop-derive] The generators property tests draw
// from: reproducible from a seed, replayable from their draws, simplest at a
// zero draw, and within the claims their types make.

struct Point : ToStr<self> by auto {
    x: Int
    y: Int
}

fn gen() -> Gen<Point> by random

struct Leaf {
    value: Int
}

struct Pair {
    left: Int
    right: Str
}

type Node = Leaf | Pair | None

fn gen() -> Gen<Leaf> by random
fn gen() -> Gen<Pair> by random
fn gen() -> Gen<Node> by random

test "the same seed draws the same values" {
    let g: Gen<List<Int>> = gen()
    let a = live_rng(11L)
    let b = live_rng(11L)
    let i = 0
    while i < 20 {
        expect_eq(g.run(a), g.run(b))
        i = i + 1
    }
}

test "a run replays from its draws" {
    let g: Gen<Str> = gen()
    let live = live_rng(5L)
    let first = g.run(live)
    let again = replay_rng(draws_of(live))
    expect_eq(g.run(again), first)
}

test "zero draws make the simplest values" {
    let ints: Gen<Int> = gen()
    let strs: Gen<Str> = gen()
    let lists: Gen<List<Int>> = gen()
    let opts: Gen<Int?> = gen()
    let bools: Gen<Bool> = gen()
    let ranged: Gen<InRange(3, 9) Int> = gen(3, 9)
    let z = replay_rng(List<Long>())
    expect_eq(ints.run(z), 0)
    expect_eq(strs.run(z), "")
    expect_eq(size(lists.run(z)), 0)
    expect(opts.run(z) is None, "an optional is None")
    expect(!bools.run(z), "a Bool is false")
    expect_eq(ranged.run(z) + 0, 3)
    let p: Gen<Point> = gen()
    let pz = p.run(z)
    expect_eq(pz.x, 0)
    expect_eq(pz.y, 0)
}

// A plain `Int` generator over a range, for the combinators.
fn ints_in(lo: Int, hi: Int) -> Gen<Int> {
    let g: Gen<InRange(lo, hi) Int> = gen(lo, hi)
    return map(g, (n) -> n + 0)
}

test "a range generator stays in its range and reaches both ends" {
    let g: Gen<InRange(-2, 2) Int> = gen(-2, 2)
    let rng = live_rng(3L)
    let low = false
    let high = false
    let i = 0
    while i < 200 {
        let n = g.run(rng) + 0
        expect(n >= -2 && n <= 2, "${n} is in -2..2")
        if n == -2 {
            low = true
        }
        if n == 2 {
            high = true
        }
        i = i + 1
    }
    expect(low && high, "both ends were drawn")
}

test "index and non-empty generators keep their claims" {
    let lists: Gen<NonEmpty List<Int>> = gen()
    let rng = live_rng(9L)
    let i = 0
    while i < 100 {
        let xs = lists.run(rng)
        expect(size(xs) >= 1, "a non-empty list")
        let ix: Gen<Idx(xs) Int> = gen(xs)
        let k = ix.run(rng)
        expect(k >= 0 && k < size(xs), "an index of the list")
        i = i + 1
    }
}

test "doubles include the special values" {
    let g: Gen<Double> = gen()
    let rng = live_rng(1L)
    let nan = false
    let inf = false
    let i = 0
    while i < 2000 {
        let d = g.run(rng)
        if d != d {
            nan = true
        }
        if d == 1.0 / 0.0 {
            inf = true
        }
        i = i + 1
    }
    expect(nan, "a NaN was drawn")
    expect(inf, "an Infinity was drawn")
}

test "combinators" {
    let doubled: Gen<Int> = map(ints_in(0, 4), (n) -> n * 2)
    let evens: Gen<Int> = map_not_null(ints_in(0, 4), (n) -> if n % 2 == 0 {
        n
    } else {
        None
    })
    let either = one_of(ints_in(0, 4), ints_in(100, 104))
    let mostly: Gen<Int> = weighted((9, ints_in(0, 4)), (1, ints_in(100, 104)))
    let rng = live_rng(4L)
    let big = 0
    let i = 0
    while i < 100 {
        expect_eq(doubled.run(rng) % 2, 0)
        expect_eq(evens.run(rng) % 2, 0)
        let e = either.run(rng)
        expect(e <= 4 || (e >= 100 && e <= 104), "one of the two ranges")
        if mostly.run(rng) >= 100 {
            big = big + 1
        }
        i = i + 1
    }
    expect(big < 50, "the rare one is rare: ${big}")
}

test "a derived union draws each arm" {
    let g: Gen<Node> = gen()
    let rng = live_rng(2L)
    let leaves = 0
    let pairs = 0
    let nones = 0
    let i = 0
    while i < 100 {
        let n = g.run(rng)
        if n is Leaf {
            leaves = leaves + 1
        } elif n is Pair {
            pairs = pairs + 1
        } else {
            nones = nones + 1
        }
        i = i + 1
    }
    expect(leaves > 0 && pairs > 0 && nones > 0, "every arm: ${leaves} ${pairs} ${nones}")
}

// std's generators under the property runner itself.
test property(runs = 50) "a range's values lie in it"(n: InRange(-3, 3) Int) {
    expect(n + 0 >= -3 && n + 0 <= 3, "${n} is in range")
}

test property(runs = 50) "a derived struct's fields are drawn"(p: Point, xs: NonEmpty List<Int>, i: Idx(xs) Int) {
    expect_eq(p.x + p.y, p.y + p.x)
    expect(i < size(xs), "an index of the list")
}

test "check holds for a property that holds" {
    check(50, (xs: List<Int>) -> {
        expect_eq(size(xs), size(xs))
    })
}
