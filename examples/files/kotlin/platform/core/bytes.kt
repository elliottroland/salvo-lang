// [platform-value-type] std's byte buffer, for `core.bytes`'s
// `platform type Bytes canbe Mut` (ROADMAP 0.7). The class is the runtime's
// `salvo.SalvoBytes`, which the wire codecs and the stream table also use;
// one class serves both kinds, so `MutBytes` is the same class.
package salvo.platform.core.bytes

typealias Bytes = salvo.SalvoBytes
typealias MutBytes = salvo.SalvoBytes

// [platform-value-type] The copy generated code makes where Salvo copies a
// buffer: a new one, since the buffer can be written.
fun copy(data: Bytes): MutBytes = salvo.SalvoBytes(data)

fun emptyBytes(): MutBytes = salvo.SalvoBytes()

fun toBytes(str: String): Bytes = salvo.SalvoBytes.ofUtf8(str)

// Strict: a malformed byte is `null`, never a replacement character.
fun strOfBytes(data: Bytes): String? = data.asString()

fun eq(a: Bytes, b: Bytes): Boolean = a == b

fun size(data: Bytes): Int = data.size

fun get(data: Bytes, index: Int): UByte? = data.getOrNull(index)

fun slice(data: Bytes, start: Int, end: Int): Bytes? = data.slice(start, end)

fun indexOf(data: Bytes, byte: UByte): Int? = data.indexOf(byte)

fun add(data: MutBytes, byte: UByte) = data.add(byte)

fun append(data: MutBytes, more: Bytes) = data.append(more)

fun set(data: MutBytes, index: Int, byte: UByte): Boolean = data.setAt(index, byte)

fun clear(data: MutBytes) = data.clear()

fun toStr(data: Bytes): String = data.toString()

fun toHex(data: Bytes): String = data.toHex()
