package salvo.core.buffer

import salvo.*

fun intBuffer(size: Int, fill: Int): salvo.platform.core.buffer.MutIntBuffer {
    return newIntPlatform(size, fill)
}

fun longBuffer(size: Int, fill: Long): salvo.platform.core.buffer.MutLongBuffer {
    return newLongPlatform(size, fill)
}

fun size__IntBuffer(buf: salvo.platform.core.buffer.IntBuffer): Int {
    return intSizePlatform(buf)
}

fun size__LongBuffer(buf: salvo.platform.core.buffer.LongBuffer): Int {
    return longSizePlatform(buf)
}

fun get__IntBuffer_Int(buf: salvo.platform.core.buffer.IntBuffer, index: Int): Int? {
    if (salvo.core.index.Idx_qualifies(index, buf, ::size__IntBuffer)) {
        return intGetPlatform(buf, (index + 0))
    }
    return null
}

fun get__LongBuffer_Int(buf: salvo.platform.core.buffer.LongBuffer, index: Int): Long? {
    if (salvo.core.index.Idx_qualifies(index, buf, ::size__LongBuffer)) {
        return longGetPlatform(buf, (index + 0))
    }
    return null
}

fun get__IntBuffer_IdxInt(buf: salvo.platform.core.buffer.IntBuffer, index: Int): Int {
    return intGetPlatform(buf, (index + 0))
}

fun get__LongBuffer_IdxInt(buf: salvo.platform.core.buffer.LongBuffer, index: Int): Long {
    return longGetPlatform(buf, (index + 0))
}

fun replace__IntBuffer_Int_Int(buf: salvo.platform.core.buffer.MutIntBuffer, index: Int, value: Int): Int {
    return intReplacePlatform(buf, (index + 0), value)
}

fun replace__LongBuffer_Int_Long(buf: salvo.platform.core.buffer.MutLongBuffer, index: Int, value: Long): Long {
    return longReplacePlatform(buf, (index + 0), value)
}

fun clear__IntBuffer_Int(buf: salvo.platform.core.buffer.MutIntBuffer, fill: Int) {
    intClearPlatform(buf, fill)
}

fun clear__LongBuffer_Long(buf: salvo.platform.core.buffer.MutLongBuffer, fill: Long) {
    longClearPlatform(buf, fill)
}

data class __Iter_iter_IntBuffer(
    var buf: salvo.platform.core.buffer.IntBuffer,
    var at: Int,
)

fun iter__IntBuffer(buf: salvo.platform.core.buffer.IntBuffer): __Iter_iter_IntBuffer {
    return __Iter_iter_IntBuffer(buf = buf, at = 0)
}

fun next__Iter_iter_IntBuffer(__p: __Iter_iter_IntBuffer): Union2<Int, salvo.core.iterator.Finished> {
    if ((__p.at >= intSizePlatform(__p.buf))) {
        return Union2.U2<Int, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val value: Int = intGetPlatform(__p.buf, __p.at)
    __p.at = (__p.at + 1)
    return Union2.U1<Int, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(value))
}

data class __Iter_iter_LongBuffer(
    var buf: salvo.platform.core.buffer.LongBuffer,
    var at: Int,
)

fun iter__LongBuffer(buf: salvo.platform.core.buffer.LongBuffer): __Iter_iter_LongBuffer {
    return __Iter_iter_LongBuffer(buf = buf, at = 0)
}

fun next__Iter_iter_LongBuffer(__p: __Iter_iter_LongBuffer): Union2<Long, salvo.core.iterator.Finished> {
    if ((__p.at >= longSizePlatform(__p.buf))) {
        return Union2.U2<Long, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val value: Long = longGetPlatform(__p.buf, __p.at)
    __p.at = (__p.at + 1)
    return Union2.U1<Long, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(value))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun toStr__IntBuffer(buf: salvo.platform.core.buffer.IntBuffer): String {
    val out: salvo.platform.core.string.MutStr = salvo.core.string.mutStr(arrayOf<String>("["))
    val __pass_1: salvo.core.range.__Iter_range_Int_Int_Int = salvo.core.range.range__Int(size__IntBuffer(buf))
    while (true) {
        val __step_2: Union2<Int, salvo.core.iterator.Finished> = salvo.core.range.next(__pass_1)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: Int = ((__step_2 as Union2.U1<*, *>).value as Int)
                val i: Int = __emitted_3
                if ((i > 0)) {
                    salvo.core.string.appendPlatform(out, ", ")
                }
                salvo.core.string.appendPlatform(out, (intGetPlatform(buf, i)).toString())
            }
            else -> {
                break
            }
        }
    }
    salvo.core.string.appendPlatform(out, "]")
    return out.toString()
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun toStr__LongBuffer(buf: salvo.platform.core.buffer.LongBuffer): String {
    val out: salvo.platform.core.string.MutStr = salvo.core.string.mutStr(arrayOf<String>("["))
    val __pass_1: salvo.core.range.__Iter_range_Int_Int_Int = salvo.core.range.range__Int(size__LongBuffer(buf))
    while (true) {
        val __step_2: Union2<Int, salvo.core.iterator.Finished> = salvo.core.range.next(__pass_1)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: Int = ((__step_2 as Union2.U1<*, *>).value as Int)
                val i: Int = __emitted_3
                if ((i > 0)) {
                    salvo.core.string.appendPlatform(out, ", ")
                }
                salvo.core.string.appendPlatform(out, (longGetPlatform(buf, i)).toString())
            }
            else -> {
                break
            }
        }
    }
    salvo.core.string.appendPlatform(out, "]")
    return out.toString()
}

fun newIntPlatform(size: Int, fill: Int): salvo.platform.core.buffer.MutIntBuffer = salvo.platform.core.buffer.newInt(size, fill)

fun intSizePlatform(buf: salvo.platform.core.buffer.IntBuffer): Int = salvo.platform.core.buffer.intSize(buf)

fun intGetPlatform(buf: salvo.platform.core.buffer.IntBuffer, index: Int): Int = salvo.platform.core.buffer.intGet(buf, index)

fun intReplacePlatform(buf: salvo.platform.core.buffer.MutIntBuffer, index: Int, value: Int): Int = salvo.platform.core.buffer.intReplace(buf, index, value)

fun intClearPlatform(buf: salvo.platform.core.buffer.MutIntBuffer, fill: Int) = salvo.platform.core.buffer.intClear(buf, fill)

fun newLongPlatform(size: Int, fill: Long): salvo.platform.core.buffer.MutLongBuffer = salvo.platform.core.buffer.newLong(size, fill)

fun longSizePlatform(buf: salvo.platform.core.buffer.LongBuffer): Int = salvo.platform.core.buffer.longSize(buf)

fun longGetPlatform(buf: salvo.platform.core.buffer.LongBuffer, index: Int): Long = salvo.platform.core.buffer.longGet(buf, index)

fun longReplacePlatform(buf: salvo.platform.core.buffer.MutLongBuffer, index: Int, value: Long): Long = salvo.platform.core.buffer.longReplace(buf, index, value)

fun longClearPlatform(buf: salvo.platform.core.buffer.MutLongBuffer, fill: Long) = salvo.platform.core.buffer.longClear(buf, fill)

