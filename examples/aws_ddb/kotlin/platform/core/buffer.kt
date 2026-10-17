// [buffer-type] std's fixed-length runs, for `core.buffer`'s `IntBuffer` and
// `LongBuffer` (ROADMAP §0j 6g): Kotlin's `IntArray` and `LongArray`, one
// class for both kinds. Callers prove every index (`Idx`), so the reads and
// writes index directly.
package salvo.platform.core.buffer

typealias IntBuffer = IntArray
typealias MutIntBuffer = IntArray
typealias LongBuffer = LongArray
typealias MutLongBuffer = LongArray

// [platform-iterable] The host's loop over a buffer: its slots.
fun eachIntBuffer(buf: IntBuffer): Iterable<Int> = buf.asIterable()

fun eachLongBuffer(buf: LongBuffer): Iterable<Long> = buf.asIterable()

// [platform-value-type] The copy generated code makes where Salvo copies a
// buffer: a new array, since one class serves both kinds and a plain buffer
// may be the very array something else holds as `Mut`.
fun copy(buf: IntBuffer): MutIntBuffer = buf.copyOf()

fun copy(buf: LongBuffer): MutLongBuffer = buf.copyOf()

fun newInt(size: Int, fill: Int): MutIntBuffer = IntArray(maxOf(size, 0)) { fill }

fun intSize(buf: IntBuffer): Int = buf.size

fun intGet(buf: IntBuffer, index: Int): Int = buf[index]

fun intReplace(buf: MutIntBuffer, index: Int, value: Int): Int {
    val old = buf[index]
    buf[index] = value
    return old
}

fun intClear(buf: MutIntBuffer, fill: Int) = buf.fill(fill)

fun newLong(size: Int, fill: Long): MutLongBuffer = LongArray(maxOf(size, 0)) { fill }

fun longSize(buf: LongBuffer): Int = buf.size

fun longGet(buf: LongBuffer, index: Int): Long = buf[index]

fun longReplace(buf: MutLongBuffer, index: Int, value: Long): Long {
    val old = buf[index]
    buf[index] = value
    return old
}

fun longClear(buf: MutLongBuffer, fill: Long) = buf.fill(fill)
