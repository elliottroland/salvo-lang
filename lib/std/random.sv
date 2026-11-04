// Random numbers, as an effect. A function that draws a random number says
// `[Random]` in its signature, so randomness is visible at every call that
// depends on it, and a test can bind a handler of its own that answers fixed
// values.
//
// Not part of `std.core`, so it is imported: `import std.random`.
//
// ```
// fn roll() [Random] -> Int {
//     return to_int(random() * 6.0) + 1                // 1 to 6
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use DefaultRandom()
//     println("rolled ${roll()}")
// }
// ```
//
// [DefaultRandom] is not cryptographic, and it is seeded afresh on each run:
// a program must not rely on a particular sequence, except inside an actor
// test, where the test's seed fixes it.

import std.runtime.random_double

// A source of random numbers.
export effect Random {
    // A uniformly distributed `Double` in `[0, 1)`: at least `0.0` and less
    // than `1.0`.
    fn random() -> Double
}

// The default generator, the runtime's own: `use DefaultRandom()`.
//
// It is seeded from the operating system's entropy, so each run draws
// different numbers. In an actor test it is seeded by the test's seed
// instead, so a run can be repeated exactly. Not suitable for cryptography.
// @compilerrules [std-random-default, test-actor]
export handler DefaultRandom() for Random {
    fn random() -> Double {
        return random_double()
    }
}
