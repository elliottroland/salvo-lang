package salvo.core.result

import salvo.*

fun<T> ok(value: T): T {
    return value
}

fun<T> err(value: T): T {
    return value
}

fun<T> Ok(value: T): T {
    return value
}

fun<T> Err(value: T): T {
    return value
}

