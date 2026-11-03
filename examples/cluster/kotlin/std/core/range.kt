package salvo.std.core.range

import salvo.*

data class RangeIter(
    var end: Int,
    var step: Int,
    var i: Int,
)

object __Codec_RangeIter : salvo.WireCodec<RangeIter> {
    override fun enc(v: RangeIter, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.end, out)
        salvo.IntCodec.enc(v.step, out)
        salvo.IntCodec.enc(v.i, out)
    }
    override fun dec(inp: salvo.WireIn): RangeIter = RangeIter(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun range__Int_Int_Int(start: Int, end: Int, step: Int): RangeIter {
    return RangeIter(end = end, step = step, i = start)
}

fun next(p: RangeIter): Union2<Int, salvo.std.core.iterator.Finished> {
    val value: Int = p.i
    if ((((p.step == 0) || ((p.step > 0) && (p.i >= p.end))) || ((p.step < 0) && (p.i <= p.end)))) {
        return Union2.U2<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    p.i = (p.i + p.step)
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(value))
}

fun range__Int_Int(start: Int, end: Int): RangeIter {
    val step: Int = (if ((start < end)) {
        1
    } else if ((start > end)) {
        (-1)
    } else {
        0
    })
    return salvo.std.core.range.range__Int_Int_Int(start, end, step)
}

fun range__Int(end: Int): RangeIter {
    return salvo.std.core.range.range__Int_Int(0, end)
}

fun InRange_qualifies(n: Int, lo: Int, hi: Int): Boolean {
    return ((n >= lo) && (n <= hi))
}

