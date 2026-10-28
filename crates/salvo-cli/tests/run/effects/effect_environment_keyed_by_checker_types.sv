// effect environment keyed by checker types
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- aliased_effects

type Count = Int

effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: List<T>, ?copy: (v: T) -> T) of Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        let value = get(values, i % values.size()) ?: unreachable!()
        i = i + 1
        return copy(value)
    }
}

fn roll() [Random<Count>] -> Count {
    return next_random()
}

fn main() [use] -> None {
    use StdConsole
    use CyclicRandom(List(7, 8))
    println("${roll()} ${roll()}")
}
//= stdout
//= 7 8
