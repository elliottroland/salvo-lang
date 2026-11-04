package salvo.std.core.array

import salvo.*

fun<T> iter(array: Array<T>): ArrayIter<T> {
    return ArrayIter<T>(items = array, at = 0)
}

data class ArrayIter<T>(
    var items: Array<T>,
    var at: Int,
)

fun<T> next(p: ArrayIter<T>): Union2<T, salvo.std.core.iterator.Finished> {
    val elem: T? = p.items.getOrNull(p.at)
    if ((elem == null)) {
        return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    p.at = (p.at + 1)
    val elem_1: T = elem!!
    return Union2.U1<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(elem_1))
}

fun<T, E> drain(e: E, items: Array<T>, each: (E, T) -> Unit) {
    val list: salvo.platform.std.core.list.MutList<T> = mutableListOf<T>()
    salvo.std.core.list.appendPlatform(list, items)
    salvo.std.core.list.drain(e, list, each)
}

