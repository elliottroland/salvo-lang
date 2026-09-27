package salvo.core.range

import salvo.*
import salvo.core.array.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.string.*

data class __Iter_range_Int_Int_Int(
    var start: Int,
    var end: Int,
    var step: Int,
    var i: Int,
)

object __Codec___Iter_range_Int_Int_Int : salvo.WireCodec<__Iter_range_Int_Int_Int> {
    override fun enc(v: __Iter_range_Int_Int_Int, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.start, out)
        salvo.IntCodec.enc(v.end, out)
        salvo.IntCodec.enc(v.step, out)
        salvo.IntCodec.enc(v.i, out)
    }
    override fun dec(inp: salvo.WireIn): __Iter_range_Int_Int_Int = __Iter_range_Int_Int_Int(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun range(start: Int, end: Int, step: Int): __Iter_range_Int_Int_Int {
    return __Iter_range_Int_Int_Int(start = start, end = end, step = step, i = start)
}

fun next__10(__p: __Iter_range_Int_Int_Int): Union2<Int, Finished> {
    val next = __p.i
    return when {
        __p.step == 0 -> {
            U2_1<Finished, Int>(finished())
        }
        __p.step > 0 && __p.i >= __p.end -> {
            U2_1<Finished, Int>(finished())
        }
        __p.step < 0 && __p.i <= __p.end -> {
            U2_1<Finished, Int>(finished())
        }
        else -> {
            __p.i = __p.i + __p.step
            U2_2<Finished, Int>(emitted(next))
        }
    }.let { when (it) { is U2_1<*, *> -> U2_2<Int, Finished>(it.value as Finished); is U2_2<*, *> -> U2_1<Int, Finished>(it.value as Int); } }
}

fun range__2(start: Int, end: Int): __Iter_range_Int_Int_Int {
    val step = when {
        start < end -> {
            1
        }
        start > end -> {
            -1
        }
        else -> {
            0
        }
    }
    return range(start, end, step)
}

fun range__3(end: Int): __Iter_range_Int_Int_Int {
    return range__2(0, end)
}

fun InRange_qualifies(n: Int, lo: Int, hi: Int): Boolean {
    return n >= lo && n <= hi
}
