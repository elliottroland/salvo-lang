// Random number generation. Not part of `std.core`, so it must be imported
// explicitly: `import std.random.Random` / `import std.random.DefaultRandom`.

import std.runtime.random_double

export effect Random {
    // A uniform double in [0, 1).
    fn random() -> Double
}

// The default generator: the runtime's, seeded by OS
// entropy — and by the test's seed in an actor test, so a run repeats.
// Not cryptographic.
// @compilerrules [std-random-default, test-actor]
export handler DefaultRandom() of Random {
    fn random() -> Double {
        return random_double()
    }
}
