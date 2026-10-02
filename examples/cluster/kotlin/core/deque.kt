package salvo.core.deque

import salvo.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

fun<T> iter__3(d: kotlin.collections.ArrayDeque<T>): DequeYield<T> {
    return DequeYield(items = d, at = 0)
}

data class DequeYield<T>(
    var items: kotlin.collections.ArrayDeque<T>,
    var at: Int,
)

fun<T> next__3(p: DequeYield<T>): Union2<T, Finished> {
    val elem = p.items.getOrNull(p.at)
    if (elem == null) {
        return Union2.U2<T, Finished>(finished())
    }
    p.at = p.at + 1
    return Union2.U1<T, Finished>(emitted(elem))
}

data class __Iter_reversed_Deque<T>(
    var d: kotlin.collections.ArrayDeque<T>,
    var at: Int,
)

fun<T> reversed(d: kotlin.collections.ArrayDeque<T>): __Iter_reversed_Deque<T> {
    return __Iter_reversed_Deque(d = d, at = d.size - 1)
}

fun<T> next__4(__p: __Iter_reversed_Deque<T>): Union2<T, Finished> {
    val elem = __p.d.getOrNull(__p.at)
    if (elem == null) {
        return Union2.U2<T, Finished>(finished())
    }
    __p.at = __p.at - 1
    return Union2.U1<T, Finished>(emitted(elem))
}
