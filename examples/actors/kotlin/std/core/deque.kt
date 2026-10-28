package salvo.std.core.deque

import salvo.*

fun<T> emptyDequePlatform(): salvo.platform.std.core.deque.MutDeque<T> = salvo.platform.std.core.deque.emptyDeque()

fun<T> dequeFromPlatform(first: T, rest: Array<T>): salvo.platform.std.core.deque.MutDeque<T> = salvo.platform.std.core.deque.dequeFrom(first, rest)

fun<T> intoMutPlatform(d: kotlin.collections.ArrayDeque<T>): salvo.platform.std.core.deque.MutDeque<T> = salvo.platform.std.core.deque.intoMut(d)

fun<T> endEmptyPlatform(d: salvo.platform.std.core.deque.MutDeque<T>) = salvo.platform.std.core.deque.endEmpty(d)

fun<T> dequeOf(): kotlin.collections.ArrayDeque<T> {
    return salvo.std.core.deque.emptyDequePlatform()
}

fun<T> dequeOf__T(first: T): kotlin.collections.ArrayDeque<T> {
    val d: salvo.platform.std.core.deque.MutDeque<T> = salvo.std.core.deque.emptyDequePlatform()
    salvo.std.core.deque.addLastPlatform(d, first)
    return d
}

fun<T> dequeOf__T_TArray(first: T, rest: Array<T>): kotlin.collections.ArrayDeque<T> {
    return salvo.std.core.deque.dequeFromPlatform(first, rest)
}

fun<T> mutDequeOf(): salvo.platform.std.core.deque.MutDeque<T> {
    return salvo.std.core.deque.emptyDequePlatform()
}

fun<T> mutDequeOf__T(first: T): salvo.platform.std.core.deque.MutDeque<T> {
    val d: salvo.platform.std.core.deque.MutDeque<T> = salvo.std.core.deque.emptyDequePlatform()
    salvo.std.core.deque.addLastPlatform(d, first)
    return d
}

fun<T> mutDequeOf__T_TArray(first: T, rest: Array<T>): salvo.platform.std.core.deque.MutDeque<T> {
    return salvo.std.core.deque.dequeFromPlatform(first, rest)
}

fun<T> dequeBy(size: Int, init: (Int) -> T): kotlin.collections.ArrayDeque<T> {
    return salvo.std.core.deque.mutDequeBy(size, init)
}

fun<T> mutDequeBy(size: Int, init: (Int) -> T): salvo.platform.std.core.deque.MutDeque<T> {
    val d: salvo.platform.std.core.deque.MutDeque<T> = salvo.std.core.deque.emptyDequePlatform()
    var i: Int = 0
    while (true) {
        if (!((i < size))) {
            break
        }
        salvo.std.core.deque.addLastPlatform(d, init(i))
        i = (i + 1)
    }
    return d
}

fun<T> addLastPlatform(d: salvo.platform.std.core.deque.MutDeque<T>, elem: T) = salvo.platform.std.core.deque.addLast(d, elem)

fun<T> addFirstPlatform(d: salvo.platform.std.core.deque.MutDeque<T>, elem: T) = salvo.platform.std.core.deque.addFirst(d, elem)

fun<T> removeFirstPlatform(d: salvo.platform.std.core.deque.MutDeque<T>): T? = salvo.platform.std.core.deque.removeFirst(d)

fun<T> removeLastPlatform(d: salvo.platform.std.core.deque.MutDeque<T>): T? = salvo.platform.std.core.deque.removeLast(d)

fun<T> removeAtPlatform(d: salvo.platform.std.core.deque.MutDeque<T>, index: Int): T? = salvo.platform.std.core.deque.removeAt(d, index)

fun<T> getPlatform(d: kotlin.collections.ArrayDeque<T>, index: Int): T? = salvo.platform.std.core.deque.get(d, index)

fun<T> get(d: kotlin.collections.ArrayDeque<T>, index: Int): T {
    return salvo.std.core.deque.getAtPlatform(d, (index + 0))
}

fun<T> getAtPlatform(d: kotlin.collections.ArrayDeque<T>, index: Int): T = salvo.platform.std.core.deque.getAt(d, index)

fun<T> atPlatform(d: kotlin.collections.ArrayDeque<T>, index: Int): T? = salvo.platform.std.core.deque.at(d, index)

fun<T> at(d: kotlin.collections.ArrayDeque<T>, index: Int): T {
    return salvo.std.core.deque.atProvenPlatform(d, (index + 0))
}

fun<T> atProvenPlatform(d: kotlin.collections.ArrayDeque<T>, index: Int): T = salvo.platform.std.core.deque.atProven(d, index)

fun<T> replacePlatform(d: salvo.platform.std.core.deque.MutDeque<T>, index: Int, value: T): T = salvo.platform.std.core.deque.replace(d, index, value)

fun<T> firstPlatform(d: kotlin.collections.ArrayDeque<T>): T? = salvo.platform.std.core.deque.first(d)

fun<T> lastPlatform(d: kotlin.collections.ArrayDeque<T>): T? = salvo.platform.std.core.deque.last(d)

fun<T> sizePlatform(d: kotlin.collections.ArrayDeque<T>): Int = salvo.platform.std.core.deque.size(d)

fun<T> drain(d: kotlin.collections.ArrayDeque<T>, each: (T) -> Unit) {
    val m: salvo.platform.std.core.deque.MutDeque<T> = salvo.std.core.deque.intoMutPlatform(d)
    while (true) {
        if (!((salvo.std.core.deque.sizePlatform(m) > 0))) {
            break
        }
        each(kotlin.run {
            val __pick_1: T? = salvo.std.core.deque.removeFirstPlatform(m)
            (if ((__pick_1 != null)) {
                val __picked_2: T = __pick_1!!
                __picked_2
            } else {
                throw AssertionError("salvo: unreachable at std.core.deque:132:33")
            })
        })
    }
    salvo.std.core.deque.endEmptyPlatform(m)
}

fun<T> toStr(d: kotlin.collections.ArrayDeque<T>, toStr: (T) -> String): String {
    val out: salvo.platform.std.core.string.MutStr = salvo.std.core.string.Mut_Str(arrayOf<String>("["))
    var i: Int = 0
    for (x in salvo.platform.std.core.deque.each(d)) {
        if ((i > 0)) {
            salvo.std.core.string.appendPlatform(out, ", ")
        }
        salvo.std.core.string.appendPlatform(out, toStr(x))
        i = (i + 1)
    }
    salvo.std.core.string.appendPlatform(out, "]")
    return out.toString()
}

fun<T> toList(d: kotlin.collections.ArrayDeque<T>, copy: (T) -> T): List<T> {
    val out: salvo.platform.std.core.list.MutList<T> = mutableListOf<T>()
    for (x in salvo.platform.std.core.deque.each(d)) {
        salvo.std.core.list.addPlatform(out, copy(x))
    }
    return out
}

fun<T> toDeque(list: List<T>, copy: (T) -> T): kotlin.collections.ArrayDeque<T> {
    val out: salvo.platform.std.core.deque.MutDeque<T> = salvo.std.core.deque.emptyDequePlatform()
    for (x in salvo.platform.std.core.list.each(list)) {
        salvo.std.core.deque.addLastPlatform(out, copy(x))
    }
    return out
}

fun<T> iter(d: kotlin.collections.ArrayDeque<T>): DequeNext<T> {
    return DequeNext<T>(items = d, at = 0)
}

data class DequeNext<T>(
    var items: kotlin.collections.ArrayDeque<T>,
    var at: Int,
)

fun<T> next__DequeNext(p: DequeNext<T>): Union2<T, salvo.std.core.iterator.Finished> {
    val elem: T? = salvo.std.core.deque.getPlatform(p.items, p.at)
    if ((elem == null)) {
        return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    p.at = (p.at + 1)
    val elem_1: T = elem!!
    return Union2.U1<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(elem_1))
}

data class DequeReversed<T>(
    var d: kotlin.collections.ArrayDeque<T>,
    var at: Int,
)

fun<T> reversed(d: kotlin.collections.ArrayDeque<T>): DequeReversed<T> {
    return DequeReversed<T>(d = d, at = (salvo.std.core.deque.sizePlatform(d) - 1))
}

fun<T> next__DequeReversed(__p: DequeReversed<T>): Union2<T, salvo.std.core.iterator.Finished> {
    val elem: T? = salvo.std.core.deque.getPlatform(__p.d, __p.at)
    if ((elem == null)) {
        return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    __p.at = (__p.at - 1)
    val elem_1: T = elem!!
    return Union2.U1<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(elem_1))
}

