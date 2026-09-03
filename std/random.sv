// Random number generation. Not part of `core`, so it must be imported
// explicitly: `import random.Random` / `import random.DefaultRandom`.

effect Random {
    // A uniformly distributed double in [0, 1).
    fn random() -> [] Double
}

external handler DefaultRandom of Random
