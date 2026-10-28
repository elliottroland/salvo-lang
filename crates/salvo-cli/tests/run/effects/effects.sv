// effects
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- effects

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

fn draw() [Random<Int>, Random<Str>, Console] {
    let n: Int = next_random()
    let s: Str = next_random()
    println("${s}: ${n}")
}

fn lucky_number() [Random<Int>] -> Int {
    return next_random()
}

fn main() [use] {
    use StdConsole
    use CyclicRandom(List(10, 20, 30))
    use CyclicRandom(List("a", "b"))
    draw()
    draw()
    println("lucky: ${next_random<Int>()}")
    println("again: ${lucky_number()}")
}
//= stdout
//= a: 10
//= b: 20
//= lucky: 30
//= again: 10
