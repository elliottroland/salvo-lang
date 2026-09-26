package salvo.core.set

import salvo.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.nonempty.*
import salvo.core.sorted.*
import salvo.core.string.*

fun<T> iter__6(set: Set<T>): SetYield<T> {
    return SetYield(items = set.toMutableList(), at = 0)
}

data class SetYield<T>(
    var items: List<T>,
    var at: Int,
)

fun<T> next__11(p: SetYield<T>): Union2<T, Finished> {
    val elem = p.items.getOrNull(p.at)
    if (elem == null) {
        return U2_2<T, Finished>(finished())
    }
    p.at = p.at + 1
    return U2_1<T, Finished>(emitted(elem))
}
