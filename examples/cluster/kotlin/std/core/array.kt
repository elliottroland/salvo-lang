package salvo.std.core.array

import salvo.*

fun<T> iter(array: Array<T>): ArrayNext<T> {
    return ArrayNext<T>(items = array, at = 0)
}

data class ArrayNext<T>(
    var items: Array<T>,
    var at: Int,
)

fun<T> next(p: ArrayNext<T>): Union2<T, salvo.std.core.iterator.Finished> {
    val elem: T? = p.items.getOrNull(p.at)
    if ((elem == null)) {
        return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    p.at = (p.at + 1)
    val elem_1: T = elem!!
    return Union2.U1<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(elem_1))
}

