package salvo.core.iterator

class Finished

fun<T> emitted(value: T): T {
    return value
}

fun finished(): Finished {
    return Finished()
}
