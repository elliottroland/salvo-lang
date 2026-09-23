package salvo.core.result

fun<T> ok(value: T): T {
    return value
}

fun<T> err(value: T): T {
    return value
}
