package salvo.core.array

import salvo.*
import salvo.core.iterator.*
import salvo.core.list.*

fun<T> iter(array: Array<T>): ArrayYield<T> {
    return ArrayYield(items = array, at = 0)
}

data class ArrayYield<T>(
    var items: Array<T>,
    var at: Int,
)

fun<T> next(pass: ArrayYield<T>): Union2<T, Finished> {
    val elem = pass.items.getOrNull(pass.at)
    if (elem == null) {
        return U2_2<T, Finished>(finished())
    }
    pass.at = pass.at + 1
    return U2_1<T, Finished>(emitted(elem))
}
