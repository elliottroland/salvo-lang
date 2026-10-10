package salvo.core.ref

import salvo.*

fun<T> NotSame_qualifies(b: T, a: T): Boolean {
    return !((a === b))
}

