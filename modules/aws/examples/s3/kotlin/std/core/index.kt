package salvo.std.core.index

import salvo.*

fun<C> Idx_qualifies(index: Int, c: C, size: (C) -> Int): Boolean {
    return ((index >= 0) && (index < size(c)))
}

data class Indices<T>(
    var list: List<T>,
    var at: Int,
)

fun<T> indices(list: List<T>): Indices<T> {
    return Indices<T>(list = list, at = 0)
}

fun<T> next__Indices(__p: Indices<T>): Union2<Int, salvo.std.core.iterator.Finished> {
    if ((__p.at >= salvo.std.core.list.sizePlatform(__p.list))) {
        return Union2.U2<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val index: Int = __p.at
    __p.at = (__p.at + 1)
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(index))
}

data class RevIndices<T>(
    var list: List<T>,
    var at: Int,
)

fun<T> revIndices(list: List<T>): RevIndices<T> {
    return RevIndices<T>(list = list, at = (salvo.std.core.list.sizePlatform(list) - 1))
}

fun<T> next__RevIndices(__p: RevIndices<T>): Union2<Int, salvo.std.core.iterator.Finished> {
    if ((__p.at < 0)) {
        return Union2.U2<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val index: Int = __p.at
    __p.at = (__p.at - 1)
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(index))
}

