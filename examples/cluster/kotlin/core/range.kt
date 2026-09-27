package salvo.core.range

import salvo.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.string.*

data class Range(
    val start: Int,
    val end: Int,
    val step: Int,
)

object __Codec_Range : salvo.WireCodec<Range> {
    override fun enc(v: Range, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.start, out)
        salvo.IntCodec.enc(v.end, out)
        salvo.IntCodec.enc(v.step, out)
    }
    override fun dec(inp: salvo.WireIn): Range = Range(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

data class __Pass_Range(
    var step: Int,
    var end: Int,
    var i: Int,
)

object __Codec___Pass_Range : salvo.WireCodec<__Pass_Range> {
    override fun enc(v: __Pass_Range, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.step, out)
        salvo.IntCodec.enc(v.end, out)
        salvo.IntCodec.enc(v.i, out)
    }
    override fun dec(inp: salvo.WireIn): __Pass_Range = __Pass_Range(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun iter__5(range: Range): __Pass_Range {
    return __Pass_Range(end = range.end, step = range.step, i = range.start)
}

fun next__8(__p: __Pass_Range): Union2<Int, Finished> {
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

fun range(start: Int, end: Int, step: Int): Range {
    return Range(start = start, end = end, step = step)
}

fun range__2(start: Int, end: Int): Range {
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

fun range__3(end: Int): Range {
    return range__2(0, end)
}

fun InRange_qualifies(n: Int, lo: Int, hi: Int): Boolean {
    return n >= lo && n <= hi
}
