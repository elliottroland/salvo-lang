package salvo.core.buffer

import salvo.*
import salvo.core.array.iter
import salvo.core.bytes.iter
import salvo.core.checked.toStr
import salvo.core.deque.get
import salvo.core.deque.iter
import salvo.core.deque.toStr
import salvo.core.index.Idx_qualifies
import salvo.core.iterator.Finished
import salvo.core.iterator.emitted
import salvo.core.iterator.finished
import salvo.core.list.get
import salvo.core.list.iter
import salvo.core.list.toStr
import salvo.core.map.get
import salvo.core.map.iter
import salvo.core.map.toStr
import salvo.core.range.__Iter_range_Int_Int_Int
import salvo.core.range.next
import salvo.core.range.range__Int
import salvo.core.set.iter
import salvo.core.set.toStr
import salvo.core.string.appendPlatform
import salvo.core.string.iter
import salvo.core.string.mutStr

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
    if (Idx_qualifies(index, buf, ::size__IntBuffer)) {
        return intGetPlatform(buf, index + 0)
    }
    return null
}

fun get__LongBuffer_Int(buf: salvo.platform.core.buffer.LongBuffer, index: Int): Long? {
    if (Idx_qualifies(index, buf, ::size__LongBuffer)) {
        return longGetPlatform(buf, index + 0)
    }
    return null
}

fun get__IntBuffer_IdxInt(buf: salvo.platform.core.buffer.IntBuffer, index: Int): Int {
    return intGetPlatform(buf, index + 0)
}

fun get__LongBuffer_IdxInt(buf: salvo.platform.core.buffer.LongBuffer, index: Int): Long {
    return longGetPlatform(buf, index + 0)
}

fun replace__IntBuffer_Int_Int(buf: salvo.platform.core.buffer.MutIntBuffer, index: Int, value: Int): Int {
    return intReplacePlatform(buf, index + 0, value)
}

fun replace__LongBuffer_Int_Long(buf: salvo.platform.core.buffer.MutLongBuffer, index: Int, value: Long): Long {
    return longReplacePlatform(buf, index + 0, value)
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

fun next__Iter_iter_IntBuffer(__p: __Iter_iter_IntBuffer): Union2<Int, Finished> {
    if (__p.at >= intSizePlatform(__p.buf)) {
        return Union2.U2<Int, Finished>(finished())
    }
    val value = intGetPlatform(__p.buf, __p.at)
    __p.at = __p.at + 1
    return Union2.U1<Int, Finished>(emitted(value))
}

data class __Iter_iter_LongBuffer(
    var buf: salvo.platform.core.buffer.LongBuffer,
    var at: Int,
)

fun iter__LongBuffer(buf: salvo.platform.core.buffer.LongBuffer): __Iter_iter_LongBuffer {
    return __Iter_iter_LongBuffer(buf = buf, at = 0)
}

fun next__Iter_iter_LongBuffer(__p: __Iter_iter_LongBuffer): Union2<Long, Finished> {
    if (__p.at >= longSizePlatform(__p.buf)) {
        return Union2.U2<Long, Finished>(finished())
    }
    val value = longGetPlatform(__p.buf, __p.at)
    __p.at = __p.at + 1
    return Union2.U1<Long, Finished>(emitted(value))
}

fun toStr__IntBuffer(buf: salvo.platform.core.buffer.IntBuffer): String {
    val out: salvo.platform.core.string.MutStr = mutStr(arrayOf("["))
    var __loop1_pass = range__Int(size__IntBuffer(buf))
    while (true) {
        val __loop1_step = next(__loop1_pass)
        if (__loop1_step !is Union2.U1<Int, Finished>) { break }
        val i = __loop1_step.value
        if (i > 0) {
            appendPlatform(out, ", ")
        }
        appendPlatform(out, (intGetPlatform(buf, i)).toString())
    }
    appendPlatform(out, "]")
    return out.toString()
}

fun toStr__LongBuffer(buf: salvo.platform.core.buffer.LongBuffer): String {
    val out: salvo.platform.core.string.MutStr = mutStr(arrayOf("["))
    var __loop2_pass = range__Int(size__LongBuffer(buf))
    while (true) {
        val __loop2_step = next(__loop2_pass)
        if (__loop2_step !is Union2.U1<Int, Finished>) { break }
        val i = __loop2_step.value
        if (i > 0) {
            appendPlatform(out, ", ")
        }
        appendPlatform(out, (longGetPlatform(buf, i)).toString())
    }
    appendPlatform(out, "]")
    return out.toString()
}

fun newIntPlatform(size: Int, fill: Int): salvo.platform.core.buffer.MutIntBuffer {
    return salvo.platform.core.buffer.newInt(size, fill)
}

fun intSizePlatform(buf: salvo.platform.core.buffer.IntBuffer): Int {
    return salvo.platform.core.buffer.intSize(buf)
}

fun intGetPlatform(buf: salvo.platform.core.buffer.IntBuffer, index: Int): Int {
    return salvo.platform.core.buffer.intGet(buf, index)
}

fun intReplacePlatform(buf: salvo.platform.core.buffer.MutIntBuffer, index: Int, value: Int): Int {
    return salvo.platform.core.buffer.intReplace(buf, index, value)
}

fun intClearPlatform(buf: salvo.platform.core.buffer.MutIntBuffer, fill: Int) {
    return salvo.platform.core.buffer.intClear(buf, fill)
}

fun newLongPlatform(size: Int, fill: Long): salvo.platform.core.buffer.MutLongBuffer {
    return salvo.platform.core.buffer.newLong(size, fill)
}

fun longSizePlatform(buf: salvo.platform.core.buffer.LongBuffer): Int {
    return salvo.platform.core.buffer.longSize(buf)
}

fun longGetPlatform(buf: salvo.platform.core.buffer.LongBuffer, index: Int): Long {
    return salvo.platform.core.buffer.longGet(buf, index)
}

fun longReplacePlatform(buf: salvo.platform.core.buffer.MutLongBuffer, index: Int, value: Long): Long {
    return salvo.platform.core.buffer.longReplace(buf, index, value)
}

fun longClearPlatform(buf: salvo.platform.core.buffer.MutLongBuffer, fill: Long) {
    return salvo.platform.core.buffer.longClear(buf, fill)
}
