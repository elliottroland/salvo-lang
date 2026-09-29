package salvo.stream

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

data class InvalidUtf8(
    val source: String,
)

object __Codec_InvalidUtf8 : salvo.WireCodec<InvalidUtf8> {
    override fun enc(v: InvalidUtf8, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.source, out)
    }
    override fun dec(inp: salvo.WireIn): InvalidUtf8 = InvalidUtf8(salvo.StrCodec.dec(inp))
}

data class StreamFailed(
    val source: String,
    val message: String,
)

object __Codec_StreamFailed : salvo.WireCodec<StreamFailed> {
    override fun enc(v: StreamFailed, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.source, out)
        salvo.StrCodec.enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): StreamFailed = StreamFailed(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun to_str__4(kind: Union2<InvalidUtf8, StreamFailed>): String {
    when (kind) {
        is U2_1<*, *> -> {
            return "not valid UTF-8: ${(kind.value as InvalidUtf8).source}"
        }
        is U2_2<*, *> -> {
            return "stream failed: ${(kind.value as StreamFailed).source}: ${(kind.value as StreamFailed).message}"
        }
    }
}

data class InStream(
    val handle: Long,
)

data class OutStream(
    val handle: Long,
)

interface Streams {
    fun read_line(s: InStream): String?
    fun read_all(s: InStream): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun read_line_to(s: InStream, buf: StringBuilder): Boolean
    fun position(s: InStream): Long
    fun close(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun write(s: OutStream, text: String): Long
    fun write_line(s: OutStream, text: String): Long
    fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long
    fun position__2(s: OutStream): Long
    fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun close__2(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>
}

class __Mon_Streams(private val inner: Streams) : Streams {
    override fun read_line(s: InStream): String? =
        synchronized(inner) { inner.read_line(s) }
    override fun read_all(s: InStream): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.read_all(s) }
    override fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.read_bytes(s, max) }
    override fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.read_to(s, buf, max) }
    override fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.read_to__2(s, buf) }
    override fun read_line_to(s: InStream, buf: StringBuilder): Boolean =
        synchronized(inner) { inner.read_line_to(s, buf) }
    override fun position(s: InStream): Long =
        synchronized(inner) { inner.position(s) }
    override fun close(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.close(s) }
    override fun write(s: OutStream, text: String): Long =
        synchronized(inner) { inner.write(s, text) }
    override fun write_line(s: OutStream, text: String): Long =
        synchronized(inner) { inner.write_line(s, text) }
    override fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long =
        synchronized(inner) { inner.write_bytes(s, data) }
    override fun position__2(s: OutStream): Long =
        synchronized(inner) { inner.position__2(s) }
    override fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.flush(s) }
    override fun close__2(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.close__2(s) }
}

data class Lines(
    var s: InStream,
)

fun lines(s: InStream): Lines {
    return Lines(s = s)
}

fun next__13(streams: Streams, p: Lines): Union2<String, Finished> {
    val line = streams.read_line(p.s)
    when {
        line != null -> {
            return U2_1<String, Finished>(emitted(line))
        }
        else -> {
            return U2_2<String, Finished>(finished())
        }
    }
}

fun close(streams: Streams, p: Lines): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(p.s)
}

data class Chunks(
    var s: InStream,
    var size: Int,
)

fun chunks(s: InStream, size: Int): Chunks {
    return Chunks(s = s, size = size)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun next__14(streams: Streams, p: Chunks): Union2<salvo.SalvoBytes, Finished> {
    val got = streams.read_bytes(p.s, p.size)
    if (got is U2_2<*, *>) {
        ignore((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        return U2_2<salvo.SalvoBytes, Finished>(finished())
    }
    val data: salvo.SalvoBytes = (got.value as salvo.SalvoBytes)
    if (data.size == 0) {
        return U2_2<salvo.SalvoBytes, Finished>(finished())
    }
    return U2_1<salvo.SalvoBytes, Finished>(emitted(data))
}

fun close__2(streams: Streams, p: Chunks): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(p.s)
}

fun stream_chunk_size(): Int {
    return 65536
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun fill_from(streams: Streams, s: InStream, buf: salvo.SalvoBytes): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    var total: Long = 0L
    var reading = true
    while (reading) {
        val got = streams.read_to(s, buf, stream_chunk_size())
        if (got is U2_2<*, *>) {
            return U2_2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val n: Int = (got.value as Int)
        total = total + (n).toLong()
        if (n == 0) {
            reading = false
        }
    }
    return U2_1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(total))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun copy_stream(streams: Streams, s: InStream, w: OutStream): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val buf = salvo.SalvoBytes.joined()
    var total: Long = 0L
    var copying = true
    while (copying) {
        buf.clear()
        val got = streams.read_to(s, buf, stream_chunk_size())
        if (got is U2_2<*, *>) {
            return U2_2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val n: Int = (got.value as Int)
        if (n == 0) {
            copying = false
        } else {
            total = total + streams.write_bytes(w, buf)
        }
    }
    return U2_1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(total))
}
