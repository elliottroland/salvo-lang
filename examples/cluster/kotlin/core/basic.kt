package salvo.core.basic

import salvo.core.checked.toStr
import salvo.core.deque.toStr
import salvo.core.list.toStr
import salvo.core.map.toStr
import salvo.core.set.toStr

fun<A, B> toStr__TupleAB(value: Pair<A, B>, toStr: (A) -> String, toStr__1: (B) -> String): String {
    return "(${toStr(value.first)}, ${toStr__1(value.second)})"
}

fun<A, B, C> toStr__TupleABC(value: Triple<A, B, C>, toStr: (A) -> String, toStr__1: (B) -> String, toStr__2: (C) -> String): String {
    return "(${toStr(value.first)}, ${toStr__1(value.second)}, ${toStr__2(value.third)})"
}
