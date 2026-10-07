package salvo.core.array

import salvo.*

fun<T> iter(array: Array<T>): ArrayYield<T> {
    return ArrayYield<T>(items = array, at = 0)
}

data class ArrayYield<T>(
    var items: Array<T>,
    var at: Int,
)

fun<T> next(p: ArrayYield<T>): Union2<T, salvo.core.iterator.Finished> {
    val elem: T? = p.items.getOrNull(p.at)
    if ((elem == null)) {
        return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    p.at = (p.at + 1)
    val elem_1: T = elem!!
    return Union2.U1<T, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(elem_1))
}

