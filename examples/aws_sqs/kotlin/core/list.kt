package salvo.core.list

import salvo.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.deque.*
import salvo.core.iterator.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

fun<T> Idx_qualifies(index: Int, list: List<T>): Boolean {
    return index >= 0 && index < list.size
}

fun NotEq_qualifies(j: Int, i: Int): Boolean {
    return j != i
}

fun<T> get(list: List<T>, index: Int): T {
    return (list.getOrNull(index + 0) ?: throw AssertionError("salvo: value is absent at core.list:90:12"))
}

fun<T> swap(list: MutableList<T>, i: Int, j: Int) {
    ignore(Checked((list).let { __l -> (i + 0).let { __i -> (j + 0).let { __j -> if (__i >= 0 && __i < __l.size && __j >= 0 && __j < __l.size) { val __t = __l[__i]; __l[__i] = __l[__j]; __l[__j] = __t; true } else false } } }))
    return
}

fun<T> at(list: List<T>, index: Int): T? {
    return list.getOrNull(index)
}

fun<T> update(list: List<T>, index: Int, f: (T) -> Unit) {
    f(get(list, index))
    return
}

fun<T> update2(list: List<T>, i: Int, j: Int, f: (T, T) -> Unit) {
    f(get(list, i), get(list, j))
    return
}

fun<T> NonEmpty_qualifies(list: List<T>): Boolean {
    return list.size > 0
}

fun<T> first(list: List<T>): T {
    return (list.getOrNull(0) ?: throw AssertionError("salvo: value is absent at core.list:273:12"))
}

fun<T> last(list: List<T>): T? {
    return list.getOrNull(list.size - 1)
}

fun<T> isEmpty(list: List<T>): Boolean {
    return list.size == 0
}

fun<T> removeFront(list: MutableList<T>, n: Int): MutableList<T> {
    return (list).let { __l -> val __f = (0).coerceIn(0, __l.size); val __t = (n).coerceIn(__f, __l.size); val __r = __l.subList(__f, __t).toMutableList(); __l.subList(__f, __t).clear(); __r }
}

fun<T> removeBack(list: MutableList<T>, n: Int): MutableList<T> {
    var at = list.size - n
    if (at < 0) {
        at = 0
    }
    return (list).let { __l -> val __f = (at).coerceIn(0, __l.size); val __t = (list.size).coerceIn(__f, __l.size); val __r = __l.subList(__f, __t).toMutableList(); __l.subList(__f, __t).clear(); __r }
}

fun<T> removeFrontWhile(list: MutableList<T>, keep: (T) -> Boolean): MutableList<T> {
    var n = 0
    while (n < list.size && keep((list.getOrNull(n) ?: throw AssertionError("salvo: value is absent at core.list:316:34")))) {
        n = n + 1
    }
    return (list).let { __l -> val __f = (0).coerceIn(0, __l.size); val __t = (n).coerceIn(__f, __l.size); val __r = __l.subList(__f, __t).toMutableList(); __l.subList(__f, __t).clear(); __r }
}

fun<T> removeBackWhile(list: MutableList<T>, keep: (T) -> Boolean): MutableList<T> {
    var at = list.size
    while (at > 0 && keep((list.getOrNull(at - 1) ?: throw AssertionError("salvo: value is absent at core.list:328:26")))) {
        at = at - 1
    }
    return (list).let { __l -> val __f = (at).coerceIn(0, __l.size); val __t = (list.size).coerceIn(__f, __l.size); val __r = __l.subList(__f, __t).toMutableList(); __l.subList(__f, __t).clear(); __r }
}

fun<T> subList(list: List<T>, from: Int, to: Int, copy: (T) -> T): MutableList<T> {
    val out = mutableListOf<T>()
    var i = from
    if (i < 0) {
        i = 0
    }
    while (i < to && i < list.size) {
        out.add(copy((list.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.list:346:23"))))
        i = i + 1
    }
    return out
}

fun<T> findFirst(list: List<T>, pick: (T) -> Boolean): Int? {
    var i = 0
    while (i < list.size) {
        if (pick((list.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.list:356:17")))) {
            return i
        }
        i = i + 1
    }
    return null
}

fun<T> findLast(list: List<T>, pick: (T) -> Boolean): Int? {
    var i = list.size - 1
    while (i >= 0) {
        if (pick((list.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.list:368:17")))) {
            return i
        }
        i = i - 1
    }
    return null
}

fun<T> indexOf(list: List<T>, elem: T, eq: (T, T) -> Boolean): Int? {
    var i = 0
    while (i < list.size) {
        if (eq((list.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.list:380:15")), elem)) {
            return i
        }
        i = i + 1
    }
    return null
}

fun<T> lastIndexOf(list: List<T>, elem: T, eq: (T, T) -> Boolean): Int? {
    var i = list.size - 1
    while (i >= 0) {
        if (eq((list.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.list:392:15")), elem)) {
            return i
        }
        i = i - 1
    }
    return null
}

fun<T> contains(list: List<T>, elem: T, eq: (T, T) -> Boolean): Boolean {
    var i = 0
    while (i < list.size) {
        if (eq((list.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.list:404:15")), elem)) {
            return true
        }
        i = i + 1
    }
    return false
}

fun<T> any(list: List<T>, pick: (T) -> Boolean): Boolean {
    return !(findFirst(list, pick) == null)
}

fun<T> all(list: List<T>, pick: (T) -> Boolean): Boolean {
    var i = 0
    while (i < list.size) {
        if (!pick((list.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.list:421:18")))) {
            return false
        }
        i = i + 1
    }
    return true
}

fun<T> count(list: List<T>, pick: (T) -> Boolean): Int {
    var n = 0
    var i = 0
    while (i < list.size) {
        if (pick((list.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.list:434:17")))) {
            n = n + 1
        }
        i = i + 1
    }
    return n
}

fun<T> partition(list: List<T>, pick: (T) -> Boolean, copy: (T) -> T): Pair<MutableList<T>, MutableList<T>> {
    val yes = mutableListOf<T>()
    val no = mutableListOf<T>()
    var i = 0
    while (i < list.size) {
        val x = (list.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.list:451:17"))
        if (pick(x)) {
            yes.add(copy(x))
        } else {
            no.add(copy(x))
        }
        i = i + 1
    }
    return Pair(yes, no)
}

fun<T> reverse(list: MutableList<T>) {
    var i = 0
    var j = list.size - 1
    while (i < j) {
        ignore(Checked((list).let { __l -> (i).let { __i -> (j).let { __j -> if (__i >= 0 && __i < __l.size && __j >= 0 && __j < __l.size) { val __t = __l[__i]; __l[__i] = __l[__j]; __l[__j] = __t; true } else false } } }))
        i = i + 1
        j = j - 1
    }
}

fun<T> iter__4(list: List<T>): ListYield<T> {
    return ListYield(items = list, at = 0)
}

data class ListYield<T>(
    var items: List<T>,
    var at: Int,
)

fun<T> next__5(p: ListYield<T>): Union2<T, Finished> {
    val elem = p.items.getOrNull(p.at)
    if (elem == null) {
        return Union2.U2<T, Finished>(finished())
    }
    p.at = p.at + 1
    return Union2.U1<T, Finished>(emitted(elem))
}

data class __Iter_reversed_List<T>(
    var list: List<T>,
    var at: Int,
)

fun<T> reversed__2(list: List<T>): __Iter_reversed_List<T> {
    return __Iter_reversed_List(list = list, at = list.size - 1)
}

fun<T> next__6(__p: __Iter_reversed_List<T>): Union2<T, Finished> {
    val elem = __p.list.getOrNull(__p.at)
    if (elem == null) {
        return Union2.U2<T, Finished>(finished())
    }
    __p.at = __p.at - 1
    return Union2.U1<T, Finished>(emitted(elem))
}

data class __Iter_indices_List<T>(
    var list: List<T>,
    var at: Int,
)

fun<T> indices(list: List<T>): __Iter_indices_List<T> {
    return __Iter_indices_List(list = list, at = 0)
}

fun<T> next__7(__p: __Iter_indices_List<T>): Union2<Int, Finished> {
    if (__p.at >= __p.list.size) {
        return Union2.U2<Int, Finished>(finished())
    }
    val index = __p.at
    __p.at = __p.at + 1
    return Union2.U1<Int, Finished>(emitted(index))
}

data class __Iter_rev_indices_List<T>(
    var list: List<T>,
    var at: Int,
)

fun<T> revIndices(list: List<T>): __Iter_rev_indices_List<T> {
    return __Iter_rev_indices_List(list = list, at = list.size - 1)
}

fun<T> next__8(__p: __Iter_rev_indices_List<T>): Union2<Int, Finished> {
    if (__p.at < 0) {
        return Union2.U2<Int, Finished>(finished())
    }
    val index = __p.at
    __p.at = __p.at - 1
    return Union2.U1<Int, Finished>(emitted(index))
}

data class Enumerated<T>(
    val index: Int,
    val elem: T,
)

data class __Iter_enumerate_List<T>(
    var list: List<T>,
    var at: Int,
)

fun<T> enumerate(list: List<T>): __Iter_enumerate_List<T> {
    return __Iter_enumerate_List(list = list, at = 0)
}

fun<T> next__9(__p: __Iter_enumerate_List<T>): Union2<Enumerated<T>, Finished> {
    val elem = __p.list.getOrNull(__p.at)
    if (elem == null) {
        return Union2.U2<Enumerated<T>, Finished>(finished())
    }
    val index = __p.at
    __p.at = __p.at + 1
    return Union2.U1<Enumerated<T>, Finished>(emitted(Enumerated(index = index, elem = elem)))
}

data class __Iter_enumerate_rev_List<T>(
    var list: List<T>,
    var at: Int,
)

fun<T> enumerateRev(list: List<T>): __Iter_enumerate_rev_List<T> {
    return __Iter_enumerate_rev_List(list = list, at = list.size - 1)
}

fun<T> next__10(__p: __Iter_enumerate_rev_List<T>): Union2<Enumerated<T>, Finished> {
    val elem = __p.list.getOrNull(__p.at)
    if (elem == null) {
        return Union2.U2<Enumerated<T>, Finished>(finished())
    }
    val index = __p.at
    __p.at = __p.at - 1
    return Union2.U1<Enumerated<T>, Finished>(emitted(Enumerated(index = index, elem = elem)))
}

fun<T> sort(list: List<T>, cmp: (T, T) -> Int): List<T> {
    return (list).let { __l -> (cmp).let { __c -> __l.sortedWith(Comparator { __a, __b -> __c(__a, __b) }).toMutableList() } }
}

fun<T> mutSort(list: List<T>, cmp: (T, T) -> Int): MutableList<T> {
    return (list).let { __l -> (cmp).let { __c -> __l.sortedWith(Comparator { __a, __b -> __c(__a, __b) }).toMutableList() } }
}

fun<T> addSorted(list: MutableList<T>, elem: T, cmp: (T, T) -> Int) {
    (list).let { __l -> (elem).let { __e -> (cmp).let { __c -> __l.add(__l.indexOfFirst { __c(it, __e) >= 0 }.let { if (it < 0) __l.size else it }, __e) } } }
}

fun<T> binarySearch(list: List<T>, elem: T, cmp: (T, T) -> Int): Int? {
    return (list).let { __l -> (elem).let { __e -> (cmp).let { __c -> __l.indexOfFirst { __c(it, __e) >= 0 }.let { if (it >= 0 && __c(__l[it], __e) == 0) it else null } } } }
}
