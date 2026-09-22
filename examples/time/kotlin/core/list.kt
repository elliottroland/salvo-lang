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

fun<T> NonEmpty_qualifies(list: List<T>): Boolean {
    return list.size > 0
}

fun<T> non_empty_list(first: T, rest: Array<T>): List<T> {
    return listOf<T>(first, *rest)
}

fun<T> first(list: List<T>): T {
    return list.getOrNull(0)!!
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
