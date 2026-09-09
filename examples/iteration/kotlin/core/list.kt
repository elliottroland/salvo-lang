package salvo.core.list

import salvo.*
import salvo.core.array.*
import salvo.core.iterator.*

fun<T> iter__2(list: List<T>): ListYield<T> {
    return ListYield(items = list, at = 0)
}

data class ListYield<T>(
    var items: List<T>,
    var at: Int,
)

fun<T> next__2(pass: ListYield<T>): Union2<T, Finished> {
    val elem = pass.items.getOrNull(pass.at)
    if (elem == null) {
        return U2_2<T, Finished>(finished())
    }
    pass.at = pass.at + 1
    return U2_1<T, Finished>(emitted(elem))
}
