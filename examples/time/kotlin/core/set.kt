package salvo.core.set

import salvo.*
import salvo.core.bytes.*
import salvo.core.deque.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.sorted.*
import salvo.core.string.*

fun<T> iter__6(set: Set<T>): SetYield<T> {
    return SetYield(items = set.toMutableList(), at = 0)
}

data class SetYield<T>(
    var items: List<T>,
    var at: Int,
)

class __Codec_SetYield<T>(private val __c_T: salvo.WireCodec<T>) : salvo.WireCodec<SetYield<T>> {
    override fun enc(v: SetYield<T>, out: salvo.WireOut) {
        salvo.ListCodec(__c_T).enc(v.items, out)
        salvo.IntCodec.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): SetYield<T> = SetYield(salvo.ListCodec(__c_T).dec(inp), salvo.IntCodec.dec(inp))
}

fun<T> next__13(p: SetYield<T>): Union2<T, Finished> {
    val elem = p.items.getOrNull(p.at)
    if (elem == null) {
        return Union2.U2<T, Finished>(finished())
    }
    p.at = p.at + 1
    return Union2.U1<T, Finished>(emitted(elem))
}

fun<T> NonEmpty_qualifies(set: Set<T>): Boolean {
    return set.size > 0
}
