package salvo.std.core.buffer

import salvo.*

fun intBuffer(size: Int, fill: Int): salvo.platform.std.core.buffer.MutIntBuffer {
    return newIntPlatform(size, fill)
}

fun longBuffer(size: Int, fill: Long): salvo.platform.std.core.buffer.MutLongBuffer {
    return newLongPlatform(size, fill)
}

fun size__IntBuffer(buf: salvo.platform.std.core.buffer.IntBuffer): Int {
    return intSizePlatform(buf)
}

fun size__LongBuffer(buf: salvo.platform.std.core.buffer.LongBuffer): Int {
    return longSizePlatform(buf)
}

fun get__IntBuffer_Int(buf: salvo.platform.std.core.buffer.IntBuffer, index: Int): Int? {
    if (salvo.std.core.index.Idx_qualifies(index, buf, ::size__IntBuffer)) {
        return intGetPlatform(buf, (index + 0))
    }
    return null
}

fun get__LongBuffer_Int(buf: salvo.platform.std.core.buffer.LongBuffer, index: Int): Long? {
    if (salvo.std.core.index.Idx_qualifies(index, buf, ::size__LongBuffer)) {
        return longGetPlatform(buf, (index + 0))
    }
    return null
}

fun get__IntBuffer_IdxInt(buf: salvo.platform.std.core.buffer.IntBuffer, index: Int): Int {
    return intGetPlatform(buf, (index + 0))
}

fun get__LongBuffer_IdxInt(buf: salvo.platform.std.core.buffer.LongBuffer, index: Int): Long {
    return longGetPlatform(buf, (index + 0))
}

fun replace__IntBuffer_Int_Int(buf: salvo.platform.std.core.buffer.MutIntBuffer, index: Int, value: Int): Int {
    return intReplacePlatform(buf, (index + 0), value)
}

fun replace__LongBuffer_Int_Long(buf: salvo.platform.std.core.buffer.MutLongBuffer, index: Int, value: Long): Long {
    return longReplacePlatform(buf, (index + 0), value)
}

fun clear__IntBuffer_Int(buf: salvo.platform.std.core.buffer.MutIntBuffer, fill: Int) {
    intClearPlatform(buf, fill)
}

fun clear__LongBuffer_Long(buf: salvo.platform.std.core.buffer.MutLongBuffer, fill: Long) {
    longClearPlatform(buf, fill)
}

data class IntBufferIter(
    var buf: salvo.platform.std.core.buffer.IntBuffer,
    var at: Int,
)

fun iter__IntBuffer(buf: salvo.platform.std.core.buffer.IntBuffer): IntBufferIter {
    return IntBufferIter(buf = buf, at = 0)
}

fun next__IntBufferIter(__p: IntBufferIter): Union2<Int, salvo.std.core.iterator.Finished> {
    if ((__p.at >= intSizePlatform(__p.buf))) {
        return Union2.U2<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val value: Int = intGetPlatform(__p.buf, __p.at)
    __p.at = (__p.at + 1)
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(value))
}

data class LongBufferIter(
    var buf: salvo.platform.std.core.buffer.LongBuffer,
    var at: Int,
)

fun iter__LongBuffer(buf: salvo.platform.std.core.buffer.LongBuffer): LongBufferIter {
    return LongBufferIter(buf = buf, at = 0)
}

fun next__LongBufferIter(__p: LongBufferIter): Union2<Long, salvo.std.core.iterator.Finished> {
    if ((__p.at >= longSizePlatform(__p.buf))) {
        return Union2.U2<Long, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val value: Long = longGetPlatform(__p.buf, __p.at)
    __p.at = (__p.at + 1)
    return Union2.U1<Long, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(value))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun toStr__IntBuffer(buf: salvo.platform.std.core.buffer.IntBuffer): String {
    val out: salvo.platform.std.core.string.MutStr = salvo.std.core.string.Mut_Str(arrayOf<String>("["))
    val __pass_1: salvo.std.core.range.Range = salvo.std.core.range.range__Int(size__IntBuffer(buf))
    while (true) {
        val __step_2: Union2<Int, salvo.std.core.iterator.Finished> = salvo.std.core.range.next(__pass_1)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: Int = ((__step_2 as Union2.U1<*, *>).value as Int)
                val i: Int = __emitted_3
                if ((i > 0)) {
                    salvo.std.core.string.appendPlatform(out, ", ")
                }
                salvo.std.core.string.appendPlatform(out, (intGetPlatform(buf, i)).toString())
            }
            else -> {
                break
            }
        }
    }
    salvo.std.core.string.appendPlatform(out, "]")
    return out.toString()
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun toStr__LongBuffer(buf: salvo.platform.std.core.buffer.LongBuffer): String {
    val out: salvo.platform.std.core.string.MutStr = salvo.std.core.string.Mut_Str(arrayOf<String>("["))
    val __pass_1: salvo.std.core.range.Range = salvo.std.core.range.range__Int(size__LongBuffer(buf))
    while (true) {
        val __step_2: Union2<Int, salvo.std.core.iterator.Finished> = salvo.std.core.range.next(__pass_1)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: Int = ((__step_2 as Union2.U1<*, *>).value as Int)
                val i: Int = __emitted_3
                if ((i > 0)) {
                    salvo.std.core.string.appendPlatform(out, ", ")
                }
                salvo.std.core.string.appendPlatform(out, (longGetPlatform(buf, i)).toString())
            }
            else -> {
                break
            }
        }
    }
    salvo.std.core.string.appendPlatform(out, "]")
    return out.toString()
}

fun newIntPlatform(size: Int, fill: Int): salvo.platform.std.core.buffer.MutIntBuffer = salvo.platform.std.core.buffer.newInt(size, fill)

fun intSizePlatform(buf: salvo.platform.std.core.buffer.IntBuffer): Int = salvo.platform.std.core.buffer.intSize(buf)

fun intGetPlatform(buf: salvo.platform.std.core.buffer.IntBuffer, index: Int): Int = salvo.platform.std.core.buffer.intGet(buf, index)

fun intReplacePlatform(buf: salvo.platform.std.core.buffer.MutIntBuffer, index: Int, value: Int): Int = salvo.platform.std.core.buffer.intReplace(buf, index, value)

fun intClearPlatform(buf: salvo.platform.std.core.buffer.MutIntBuffer, fill: Int) = salvo.platform.std.core.buffer.intClear(buf, fill)

fun newLongPlatform(size: Int, fill: Long): salvo.platform.std.core.buffer.MutLongBuffer = salvo.platform.std.core.buffer.newLong(size, fill)

fun longSizePlatform(buf: salvo.platform.std.core.buffer.LongBuffer): Int = salvo.platform.std.core.buffer.longSize(buf)

fun longGetPlatform(buf: salvo.platform.std.core.buffer.LongBuffer, index: Int): Long = salvo.platform.std.core.buffer.longGet(buf, index)

fun longReplacePlatform(buf: salvo.platform.std.core.buffer.MutLongBuffer, index: Int, value: Long): Long = salvo.platform.std.core.buffer.longReplace(buf, index, value)

fun longClearPlatform(buf: salvo.platform.std.core.buffer.MutLongBuffer, fill: Long) = salvo.platform.std.core.buffer.longClear(buf, fill)

