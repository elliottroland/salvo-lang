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

fun<T> emptyDequePlatform(): salvo.platform.core.deque.MutDeque<T> {
    return salvo.platform.core.deque.emptyDeque()
}

fun<T> dequeFromPlatform(first: T, rest: Array<T>): salvo.platform.core.deque.MutDeque<T> {
    return salvo.platform.core.deque.dequeFrom(first, rest)
}

fun<T> intoMutPlatform(d: kotlin.collections.ArrayDeque<T>): salvo.platform.core.deque.MutDeque<T> {
    return salvo.platform.core.deque.intoMut(d)
}

fun<T> endEmptyPlatform(d: salvo.platform.core.deque.MutDeque<T>) {
    return salvo.platform.core.deque.endEmpty(d)
}

fun<T> dequeOf(): kotlin.collections.ArrayDeque<T> {
    return emptyDequePlatform<T>()
}

fun<T> dequeOf__2(first: T): kotlin.collections.ArrayDeque<T> {
    val d = emptyDequePlatform<T>()
    addLastPlatform(d, first)
    return d
}

fun<T> dequeOf__3(first: T, rest: Array<T>): kotlin.collections.ArrayDeque<T> {
    return dequeFromPlatform(first, rest)
}

fun<T> mutDequeOf(): salvo.platform.core.deque.MutDeque<T> {
    return emptyDequePlatform<T>()
}

fun<T> mutDequeOf__2(first: T): salvo.platform.core.deque.MutDeque<T> {
    val d = emptyDequePlatform<T>()
    addLastPlatform(d, first)
    return d
}

fun<T> mutDequeOf__3(first: T, rest: Array<T>): salvo.platform.core.deque.MutDeque<T> {
    return dequeFromPlatform(first, rest)
}

fun<T> dequeBy(size: Int, init: (Int) -> T): kotlin.collections.ArrayDeque<T> {
    return mutDequeBy(size, init)
}

fun<T> mutDequeBy(size: Int, init: (Int) -> T): salvo.platform.core.deque.MutDeque<T> {
    val d = emptyDequePlatform<T>()
    var i = 0
    while (i < size) {
        addLastPlatform(d, init(i))
        i = i + 1
    }
    return d
}

fun<T> addLastPlatform(d: salvo.platform.core.deque.MutDeque<T>, elem: T) {
    return salvo.platform.core.deque.addLast(d, elem)
}

fun<T> addFirstPlatform(d: salvo.platform.core.deque.MutDeque<T>, elem: T) {
    return salvo.platform.core.deque.addFirst(d, elem)
}

fun<T> removeFirstPlatform(d: salvo.platform.core.deque.MutDeque<T>): T? {
    return salvo.platform.core.deque.removeFirst(d)
}

fun<T> removeLastPlatform(d: salvo.platform.core.deque.MutDeque<T>): T? {
    return salvo.platform.core.deque.removeLast(d)
}

fun<T> removeAtPlatform(d: salvo.platform.core.deque.MutDeque<T>, index: Int): T? {
    return salvo.platform.core.deque.removeAt(d, index)
}

fun<T> getPlatform(d: kotlin.collections.ArrayDeque<T>, index: Int): T? {
    return salvo.platform.core.deque.get(d, index)
}

fun<T> firstPlatform(d: kotlin.collections.ArrayDeque<T>): T? {
    return salvo.platform.core.deque.first(d)
}

fun<T> lastPlatform(d: kotlin.collections.ArrayDeque<T>): T? {
    return salvo.platform.core.deque.last(d)
}

fun<T> sizePlatform(d: kotlin.collections.ArrayDeque<T>): Int {
    return salvo.platform.core.deque.size(d)
}

fun<T> drain(d: kotlin.collections.ArrayDeque<T>, each: (T) -> Unit) {
    val m = intoMutPlatform(d)
    while (sizePlatform(m) > 0) {
        each((removeFirstPlatform(m) ?: throw AssertionError("salvo: value is absent at core.deque:101:14")))
    }
    endEmptyPlatform(m)
}

fun<T> toStr(d: kotlin.collections.ArrayDeque<T>, toStr: (T) -> String): String {
    val out = mutStr(arrayOf("["))
    var i = 0
    for (x in salvo.platform.core.deque.each(d)) {
        if (i > 0) {
            appendPlatform(out, ", ")
        }
        appendPlatform(out, toStr(x))
        i = i + 1
    }
    appendPlatform(out, "]")
    return out.toString()
}

fun<T> toList(d: kotlin.collections.ArrayDeque<T>, copy: (T) -> T): List<T> {
    val out = mutableListOf<T>()
    for (x in salvo.platform.core.deque.each(d)) {
        addPlatform(out, copy(x))
    }
    return out
}

fun<T> toDeque(list: List<T>, copy: (T) -> T): kotlin.collections.ArrayDeque<T> {
    val out = emptyDequePlatform<T>()
    for (x in salvo.platform.core.list.each(list)) {
        addLastPlatform(out, copy(x))
    }
    return out
}

fun<T> iter__3(d: kotlin.collections.ArrayDeque<T>): DequeYield<T> {
    return DequeYield(items = d, at = 0)
}

data class DequeYield<T>(
    var items: kotlin.collections.ArrayDeque<T>,
    var at: Int,
)

fun<T> next__3(p: DequeYield<T>): Union2<T, Finished> {
    val elem = getPlatform(p.items, p.at)
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
    return __Iter_reversed_Deque(d = d, at = sizePlatform(d) - 1)
}

fun<T> next__4(__p: __Iter_reversed_Deque<T>): Union2<T, Finished> {
    val elem = getPlatform(__p.d, __p.at)
    if (elem == null) {
        return Union2.U2<T, Finished>(finished())
    }
    __p.at = __p.at - 1
    return Union2.U1<T, Finished>(emitted(elem))
}
