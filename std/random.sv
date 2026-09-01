// Random number generation. Not part of `core`, so it must be imported
// explicitly: `import random.Random` / `import random.DefaultRandom`.

effect Random {
    // A uniformly distributed float in [0, 1).
    fn random() -> Float
}

external handler DefaultRandom of Random
