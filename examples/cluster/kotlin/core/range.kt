package salvo.core.range

import salvo.*

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

fun range__Int_Int_Int(start: Int, end: Int, step: Int): __Iter_range_Int_Int_Int {
    return __Iter_range_Int_Int_Int(start = start, end = end, step = step, i = start)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun next(__p: __Iter_range_Int_Int_Int): Union2<Int, salvo.core.iterator.Finished> {
    val next: Int = __p.i
    return (if (((__p.step) == (0))) {
        Union2.U1<salvo.core.iterator.Finished, Int>(salvo.core.iterator.finished())
    } else if (((__p.step > 0) && (__p.i >= __p.end))) {
        Union2.U1<salvo.core.iterator.Finished, Int>(salvo.core.iterator.finished())
    } else if (((__p.step < 0) && (__p.i <= __p.end))) {
        Union2.U1<salvo.core.iterator.Finished, Int>(salvo.core.iterator.finished())
    } else {
        __p.i = (__p.i + __p.step)
        Union2.U2<salvo.core.iterator.Finished, Int>(salvo.core.iterator.emitted(next))
    }).let { when (it) { is Union2.U1<*, *> -> Union2.U2<Int, salvo.core.iterator.Finished>(it.value as salvo.core.iterator.Finished); is Union2.U2<*, *> -> Union2.U1<Int, salvo.core.iterator.Finished>(it.value as Int); else -> throw IllegalStateException("salvo: unreachable union arm") } }
}

fun range__Int_Int(start: Int, end: Int): __Iter_range_Int_Int_Int {
    val step: Int = (if ((start < end)) {
        1
    } else if ((start > end)) {
        (-1)
    } else {
        0
    })
    return range__Int_Int_Int(start, end, step)
}

fun range__Int(end: Int): __Iter_range_Int_Int_Int {
    return range__Int_Int(0, end)
}

fun InRange_qualifies(n: Int, lo: Int, hi: Int): Boolean {
    return ((n >= lo) && (n <= hi))
}

