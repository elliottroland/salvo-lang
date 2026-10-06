package salvo.core.index

import salvo.*
import salvo.core.array.next
import salvo.core.bytes.next
import salvo.core.iterator.Finished
import salvo.core.iterator.emitted
import salvo.core.iterator.finished
import salvo.core.list.sizePlatform
import salvo.core.set.next
import salvo.core.string.next

fun<C> Idx_qualifies(index: Int, c: C, size: (C) -> Int): Boolean {
    return index >= 0 && index < size(c)
}

fun NotEq_qualifies(j: Int, i: Int): Boolean {
    return j != i
}

data class __Iter_indices_List<T>(
    var list: List<T>,
    var at: Int,
)

fun<T> indices(list: List<T>): __Iter_indices_List<T> {
    return __Iter_indices_List(list = list, at = 0)
}

fun<T> next__Iter_indices_List(__p: __Iter_indices_List<T>): Union2<Int, Finished> {
    if (__p.at >= sizePlatform(__p.list)) {
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
    return __Iter_rev_indices_List(list = list, at = sizePlatform(list) - 1)
}

fun<T> next__Iter_rev_indices_List(__p: __Iter_rev_indices_List<T>): Union2<Int, Finished> {
    if (__p.at < 0) {
        return Union2.U2<Int, Finished>(finished())
    }
    val index = __p.at
    __p.at = __p.at - 1
    return Union2.U1<Int, Finished>(emitted(index))
}
