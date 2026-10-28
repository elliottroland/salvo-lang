// [iter-protocol] laziness, now a property of the pass
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- the_combinator_surface

fn double(n: Int) -> Int {
    return n * 2
}

fn is_even(n: Int) -> Bool {
    return n % 2 == 0
}

fn main() [use] -> None {
    use StdConsole()
    let xs = List(1, 2, 3, 4)
    let doubled = map(xs, double)
    println("eager ${doubled.size()}")
    let generic = map(iter(copy(xs)), double)
    println("generic ${generic.size()}")
    let snapshot = copy(xs)
    let kept = filter(iter(snapshot), is_even)
    println("kept ${kept.size()}")
    let out = map_to(Mut List<Int>(), iter(copy(xs)), double)
    println("sink ${out.size()}")
    let chained = filter_to(map_to(Mut List<Int>(), iter(copy(xs)), double), iter(xs), is_even)
    println("chained ${chained.size()}")
}
//= stdout
//= eager 4
//= generic 4
//= kept 2
//= sink 4
//= chained 6

// An **unbounded** producer that terminates only because the consumer stops.
// This was a lazy-chain test until 2026-09-10; with the lazy pair gone the
// same property is what a plain `for` with a `break` states, and it is the
// property that mattered — an `iter fn` computes one element per turn, so an
// endless source costs nothing until it is driven.
//--- a_break_out_of_an_unbounded_producer

struct Naturals {
    from: Int
}

fn naturals() -> Naturals {
    return Naturals {from: 0}
}

iter fn iter(n: Naturals) -> Emitted Int | Finished {
    state NaturalsIter {
        at = n.from
    }
    let v = copy(at)
    at = at + 1
    return emitted(v)
}

fn triple(n: Int) -> Int {
    return n * 3
}

fn is_even(n: Int) -> Bool {
    return n % 2 == 0
}

fn main() [use] -> None {
    use StdConsole()
    for v in iter(naturals()) {
        if v > 4 {
            break
        }
        if is_even(v) {
            println("v ${triple(v)}")
        }
    }
}
//= stdout
//= v 0
//= v 6
//= v 12
