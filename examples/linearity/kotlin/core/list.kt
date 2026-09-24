package salvo.core.list

import salvo.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.iterator.*
import salvo.core.map.*
import salvo.core.nonempty.*
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
    (list).let { __l -> (i + 0).let { __i -> (j + 0).let { __j -> if (__i >= 0 && __i < __l.size && __j >= 0 && __j < __l.size) { val __t = __l[__i]; __l[__i] = __l[__j]; __l[__j] = __t; true } else false } } }
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
    return (list.getOrNull(0) ?: throw AssertionError("salvo: value is absent at core.list:250:12"))
}

fun<T> iter__3(list: List<T>): ListYield<T> {
    return ListYield(items = list, at = 0)
}

data class ListYield<T>(
    var items: List<T>,
    var at: Int,
)

fun<T> next__5(p: ListYield<T>): Union2<T, Finished> {
    val elem = p.items.getOrNull(p.at)
    if (elem == null) {
        return U2_2<T, Finished>(finished())
    }
    p.at = p.at + 1
    return U2_1<T, Finished>(emitted(elem))
}

fun<T> reversed(list: List<T>): ListRevYield<T> {
    return ListRevYield(items = list, at = list.size - 1)
}

data class ListRevYield<T>(
    var items: List<T>,
    var at: Int,
)

fun<T> next__6(p: ListRevYield<T>): Union2<T, Finished> {
    val elem = p.items.getOrNull(p.at)
    if (elem == null) {
        return U2_2<T, Finished>(finished())
    }
    p.at = p.at - 1
    return U2_1<T, Finished>(emitted(elem))
}

fun<T> indices(list: List<T>): IdxYield<T> {
    return IdxYield(items = list, at = 0, step = 1)
}

fun<T> rev_indices(list: List<T>): IdxYield<T> {
    return IdxYield(items = list, at = list.size - 1, step = -1)
}

data class IdxYield<T>(
    var items: List<T>,
    var at: Int,
    var step: Int,
)

fun<T> next__7(p: IdxYield<T>): Union2<Int, Finished> {
    if (p.at < 0 || p.at >= p.items.size) {
        return U2_2<Int, Finished>(finished())
    }
    val index = p.at
    p.at = p.at + p.step
    return U2_1<Int, Finished>(emitted(index))
}

data class Enumerated<T>(
    val index: Int,
    val elem: T,
)

fun<T> enumerate(list: List<T>): ListEnumYield<T> {
    return ListEnumYield(items = list, at = 0, step = 1)
}

fun<T> enumerate_rev(list: List<T>): ListEnumYield<T> {
    return ListEnumYield(items = list, at = list.size - 1, step = -1)
}

data class ListEnumYield<T>(
    var items: List<T>,
    var at: Int,
    var step: Int,
)

fun<T> next__8(p: ListEnumYield<T>): Union2<Enumerated<T>, Finished> {
    val elem = p.items.getOrNull(p.at)
    if (elem == null) {
        return U2_2<Enumerated<T>, Finished>(finished())
    }
    val index = p.at
    p.at = p.at + p.step
    return U2_1<Enumerated<T>, Finished>(emitted(Enumerated(index = index, elem = elem)))
}

fun<T> sort(list: List<T>, cmp: (T, T) -> Int): List<T> {
    return (list).let { __l -> (cmp).let { __c -> __l.sortedWith(Comparator { __a, __b -> __c(__a, __b) }).toMutableList() } }
}

fun<T> mut_sort(list: List<T>, cmp: (T, T) -> Int): MutableList<T> {
    return (list).let { __l -> (cmp).let { __c -> __l.sortedWith(Comparator { __a, __b -> __c(__a, __b) }).toMutableList() } }
}

fun<T> add_sorted(list: MutableList<T>, elem: T, cmp: (T, T) -> Int) {
    (list).let { __l -> (elem).let { __e -> (cmp).let { __c -> __l.add(__l.indexOfFirst { __c(it, __e) >= 0 }.let { if (it < 0) __l.size else it }, __e) } } }
}

fun<T> binary_search(list: List<T>, elem: T, cmp: (T, T) -> Int): Int? {
    return (list).let { __l -> (elem).let { __e -> (cmp).let { __c -> __l.indexOfFirst { __c(it, __e) >= 0 }.let { if (it >= 0 && __c(__l[it], __e) == 0) it else null } } } }
}
