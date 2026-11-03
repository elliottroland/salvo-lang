package salvo.std.core.nonempty

import salvo.*

fun<T> NonEmpty_qualifies(value: T, size: (T) -> Int): Boolean {
    return (size(value) > 0)
}

