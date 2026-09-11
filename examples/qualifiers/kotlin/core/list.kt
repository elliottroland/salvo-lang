package salvo.core.list

import salvo.*
import salvo.core.array.*
import salvo.core.iterator.*
import salvo.core.string.*

fun<T> iter__2(list: List<T>): ListYield<T> {
    return ListYield(items = list, at = 0)
}

data class ListYield<T>(
    var items: List<T>,
    var at: Int,
)

fun<T> next__2(p: ListYield<T>): Union2<T, Finished> {
    val elem = p.items.getOrNull(p.at)
    if (elem == null) {
        return U2_2<T, Finished>(finished())
    }
    p.at = p.at + 1
    return U2_1<T, Finished>(emitted(elem))
}
