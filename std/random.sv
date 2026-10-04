// Random number generation. Not part of `core`, so it must be imported
// explicitly: `import random.Random` / `import random.DefaultRandom`.

import runtime.random_double

export effect Random {
    // A uniform double in [0, 1).
    fn random() -> Double
}

// [random-default] The default generator: the runtime's, seeded by OS
// entropy — and by the test's seed in an actor test, so a run repeats
// [test-actor]. Not cryptographic.
export handler DefaultRandom() of Random {
    fn random() -> Double {
        return random_double()
    }
}
