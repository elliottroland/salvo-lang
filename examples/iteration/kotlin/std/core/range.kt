package salvo.std.core.range

import salvo.*

data class Range(
    var start: Int,
    var end: Int,
    var step: Int,
    var i: Int,
)

object __Codec_Range : salvo.WireCodec<Range> {
    override fun enc(v: Range, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.start, out)
        salvo.IntCodec.enc(v.end, out)
        salvo.IntCodec.enc(v.step, out)
        salvo.IntCodec.enc(v.i, out)
    }
    override fun dec(inp: salvo.WireIn): Range = Range(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun range__Int_Int_Int(start: Int, end: Int, step: Int): Range {
    return Range(start = start, end = end, step = step, i = start)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun next(__p: Range): Union2<Int, salvo.std.core.iterator.Finished> {
    val next: Int = __p.i
    return (if ((__p.step == 0)) {
        Union2.U1<salvo.std.core.iterator.Finished, Int>(salvo.std.core.iterator.finished())
    } else if (((__p.step > 0) && (__p.i >= __p.end))) {
        Union2.U1<salvo.std.core.iterator.Finished, Int>(salvo.std.core.iterator.finished())
    } else if (((__p.step < 0) && (__p.i <= __p.end))) {
        Union2.U1<salvo.std.core.iterator.Finished, Int>(salvo.std.core.iterator.finished())
    } else {
        __p.i = (__p.i + __p.step)
        Union2.U2<salvo.std.core.iterator.Finished, Int>(salvo.std.core.iterator.emitted(next))
    }).let { when (it) { is Union2.U1<*, *> -> Union2.U2<Int, salvo.std.core.iterator.Finished>(it.value as salvo.std.core.iterator.Finished); is Union2.U2<*, *> -> Union2.U1<Int, salvo.std.core.iterator.Finished>(it.value as Int); else -> throw IllegalStateException("salvo: unreachable union arm") } }
}

fun range__Int_Int(start: Int, end: Int): Range {
    val step: Int = (if ((start < end)) {
        1
    } else if ((start > end)) {
        (-1)
    } else {
        0
    })
    return range__Int_Int_Int(start, end, step)
}

fun range__Int(end: Int): Range {
    return range__Int_Int(0, end)
}

fun InRange_qualifies(n: Int, lo: Int, hi: Int): Boolean {
    return ((n >= lo) && (n <= hi))
}

