package salvo.std.core.compare

import salvo.*

fun<T> hash__List(value: List<T>, hash: (T) -> Long): Long {
    var h: Long = 7L
    for (x in salvo.platform.std.core.list.each(value)) {
        h = salvo.std.core.compare.mixHash(h, hash(x))
    }
    return h
}

fun<A, B> hash__TupleAB(value: kotlin.Pair<A, B>, hash: (A) -> Long, hash__1: (B) -> Long): Long {
    return salvo.std.core.compare.mixHash(salvo.std.core.compare.mixHash(7L, hash(value.first)), hash__1(value.second))
}

fun<A, B, C> hash__TupleABC(value: kotlin.Triple<A, B, C>, hash: (A) -> Long, hash__1: (B) -> Long, hash__2: (C) -> Long): Long {
    return salvo.std.core.compare.mixHash(salvo.std.core.compare.mixHash(salvo.std.core.compare.mixHash(7L, hash(value.first)), hash__1(value.second)), hash__2(value.third))
}

fun mixHash(seed: Long, value: Long): Long {
    return ((seed * 31L) + value)
}

