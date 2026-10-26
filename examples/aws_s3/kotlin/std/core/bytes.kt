package salvo.std.core.bytes

import salvo.*

fun bytesOf(elems: Array<UByte>): salvo.platform.std.core.bytes.Bytes {
    val out: salvo.platform.std.core.bytes.MutBytes = emptyBytesPlatform()
    for (b in elems) {
        addPlatform(out, b)
    }
    return out
}

fun mutBytes(parts: Array<salvo.platform.std.core.bytes.Bytes>): salvo.platform.std.core.bytes.MutBytes {
    val out: salvo.platform.std.core.bytes.MutBytes = emptyBytesPlatform()
    for (part in parts) {
        appendPlatform(out, part)
    }
    return out
}

fun emptyBytesPlatform(): salvo.platform.std.core.bytes.MutBytes = salvo.platform.std.core.bytes.emptyBytes()

fun toBytesPlatform(str: String): salvo.platform.std.core.bytes.Bytes = salvo.platform.std.core.bytes.toBytes(str)

fun strOfBytesPlatform(data: salvo.platform.std.core.bytes.Bytes): String? = salvo.platform.std.core.bytes.strOfBytes(data)

fun eqPlatform(a: salvo.platform.std.core.bytes.Bytes, b: salvo.platform.std.core.bytes.Bytes): Boolean = salvo.platform.std.core.bytes.eq(a, b)

fun sizePlatform(data: salvo.platform.std.core.bytes.Bytes): Int = salvo.platform.std.core.bytes.size(data)

fun getPlatform(data: salvo.platform.std.core.bytes.Bytes, index: Int): UByte? = salvo.platform.std.core.bytes.get(data, index)

fun slicePlatform(data: salvo.platform.std.core.bytes.Bytes, start: Int, end: Int): salvo.platform.std.core.bytes.Bytes? = salvo.platform.std.core.bytes.slice(data, start, end)

fun indexOfPlatform(data: salvo.platform.std.core.bytes.Bytes, byte: UByte): Int? = salvo.platform.std.core.bytes.indexOf(data, byte)

fun addPlatform(data: salvo.platform.std.core.bytes.MutBytes, byte: UByte) = salvo.platform.std.core.bytes.add(data, byte)

fun appendPlatform(data: salvo.platform.std.core.bytes.MutBytes, more: salvo.platform.std.core.bytes.Bytes) = salvo.platform.std.core.bytes.append(data, more)

fun setPlatform(data: salvo.platform.std.core.bytes.MutBytes, index: Int, byte: UByte): Boolean = salvo.platform.std.core.bytes.set(data, index, byte)

fun clearPlatform(data: salvo.platform.std.core.bytes.MutBytes) = salvo.platform.std.core.bytes.clear(data)

fun toStrPlatform(data: salvo.platform.std.core.bytes.Bytes): String = salvo.platform.std.core.bytes.toStr(data)

fun toHexPlatform(data: salvo.platform.std.core.bytes.Bytes): String = salvo.platform.std.core.bytes.toHex(data)

fun iter(data: salvo.platform.std.core.bytes.Bytes): BytesYield {
    return BytesYield(data = data, at = 0)
}

data class BytesYield(
    var data: salvo.platform.std.core.bytes.Bytes,
    var at: Int,
)

fun next(p: BytesYield): Union2<UByte, salvo.std.core.iterator.Finished> {
    val b: UByte? = getPlatform(p.data, p.at)
    if ((b == null)) {
        return Union2.U2<UByte, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    p.at = (p.at + 1)
    val b_1: UByte = b!!
    return Union2.U1<UByte, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(b_1))
}

