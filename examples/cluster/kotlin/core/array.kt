package salvo.core.array

import salvo.*
import salvo.core.deque.get
import salvo.core.iterator.Finished
import salvo.core.iterator.emitted
import salvo.core.iterator.finished
import salvo.core.list.first
import salvo.core.list.get
import salvo.core.map.get

fun<T> iter(array: Array<T>): ArrayYield<T> {
    return ArrayYield(items = array, at = 0)
}

data class ArrayYield<T>(
    var items: Array<T>,
    var at: Int,
)

fun<T> next(p: ArrayYield<T>): Union2<T, Finished> {
    val elem = p.items.getOrNull(p.at)
    if (elem == null) {
        return Union2.U2<T, Finished>(finished())
    }
    p.at = p.at + 1
    return Union2.U1<T, Finished>(emitted(elem))
}
