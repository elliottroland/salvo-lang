// Kotlin defines for random [backend-define-handler].

define handler DefaultRandom of Random {
    define fn random() -> Float {
        inline: ``
        kotlin.random.Random.nextFloat()
        ``
    }
}
