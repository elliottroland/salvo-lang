// Kotlin defines for random [backend-define-handler].

define handler DefaultRandom of Random {
    define fn random() -> Double {
        inline: ``
        kotlin.random.Random.nextDouble()
        ``
    }
}
