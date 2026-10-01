package salvo.stream

import salvo.*
import salvo.core.actor.*
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
fun toStr__4(kind: Union2<InvalidUtf8, StreamFailed>): String {
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

data class Packet(
    val bytes: salvo.SalvoBytes,
    val stream: InStream,
)

class End

object __Codec_End : salvo.WireCodec<End> {
    override fun enc(v: End, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): End = End()
}

fun close(streams: Streams, p: Packet): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val __destructured1 = p
    val bytes = __destructured1.bytes
    val stream = __destructured1.stream
    return streams.close(stream)
}

interface Streams {
    fun readLine(s: InStream): String?
    fun readAll(s: InStream): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun readBytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun readTo(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun readTo__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun readLineTo(s: InStream, buf: StringBuilder): Boolean
    fun position(s: InStream): Long
    fun close(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun write(s: OutStream, text: String): Long
    fun writeLine(s: OutStream, text: String): Long
    fun writeBytes(s: OutStream, data: salvo.SalvoBytes): Long
    fun position__2(s: OutStream): Long
    fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun close__2(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun receive(s: InStream, reply: salvo.SalvoReply)
    fun fromBytes(data: salvo.SalvoBytes): InStream
}

class __Mon_Streams(private val inner: Streams) : Streams {
    override fun readLine(s: InStream): String? =
        synchronized(inner) { inner.readLine(s) }
    override fun readAll(s: InStream): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.readAll(s) }
    override fun readBytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.readBytes(s, max) }
    override fun readTo(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.readTo(s, buf, max) }
    override fun readTo__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.readTo__2(s, buf) }
    override fun readLineTo(s: InStream, buf: StringBuilder): Boolean =
        synchronized(inner) { inner.readLineTo(s, buf) }
    override fun position(s: InStream): Long =
        synchronized(inner) { inner.position(s) }
    override fun close(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.close(s) }
    override fun write(s: OutStream, text: String): Long =
        synchronized(inner) { inner.write(s, text) }
    override fun writeLine(s: OutStream, text: String): Long =
        synchronized(inner) { inner.writeLine(s, text) }
    override fun writeBytes(s: OutStream, data: salvo.SalvoBytes): Long =
        synchronized(inner) { inner.writeBytes(s, data) }
    override fun position__2(s: OutStream): Long =
        synchronized(inner) { inner.position__2(s) }
    override fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.flush(s) }
    override fun close__2(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> =
        synchronized(inner) { inner.close__2(s) }
    override fun receive(s: InStream, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.receive(s, reply) }
    override fun fromBytes(data: salvo.SalvoBytes): InStream =
        synchronized(inner) { inner.fromBytes(data) }
}

fun pipe(streams: Streams, from: InStream, to: OutStream, done: salvo.SalvoReply) {
    streams.receive(from, run { val __e0 = streams; val __c0 = to; val __c1 = done; val __c2 = 0L; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { _: ByteArray -> Pair(false, null) }) { __v -> pipeStep(__e0, __c0, __c1, __c2, __v as Union3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>) } })
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun pipeStep(streams: Streams, to: OutStream, done: salvo.SalvoReply, moved: Long, got: Union3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>) {
    when (got) {
        is U3_1<*, *, *> -> {
            val __destructured2 = (got.value as Packet)
            val bytes = __destructured2.bytes
            val stream = __destructured2.stream
            val written = streams.writeBytes(to, bytes)
            streams.receive(stream, run { val __e0 = streams; val __c0 = to; val __c1 = done; val __c2 = moved + written; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { _: ByteArray -> Pair(false, null) }) { __v -> pipeStep(__e0, __c0, __c1, __c2, __v as Union3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>) } })
        }
        is U3_2<*, *, *> -> {
            val closed = streams.close__2(to)
            when (closed) {
                is U2_1<*, *> -> {
                    salvo.SalvoSched.replyWire(done, U2_1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(moved)), salvo.Union2Codec(salvo.LongCodec, __Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))))
                }
                is U2_2<*, *> -> {
                    salvo.SalvoSched.replyWire(done, U2_2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>)), salvo.Union2Codec(salvo.LongCodec, __Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))))
                }
            }
        }
        is U3_3<*, *, *> -> {
            val closed = streams.close__2(to)
            when (closed) {
                is U2_1<*, *> -> {
                }
                is U2_2<*, *> -> {
                    ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
                }
            }
            salvo.SalvoSched.replyWire(done, U2_2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>)), salvo.Union2Codec(salvo.LongCodec, __Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))))
        }
    }
}

data class Lines(
    var s: InStream,
)

fun lines(s: InStream): Lines {
    return Lines(s = s)
}

fun next__13(streams: Streams, p: Lines): Union2<String, Finished> {
    val line = streams.readLine(p.s)
    when {
        line != null -> {
            return U2_1<String, Finished>(emitted(line))
        }
        else -> {
            return U2_2<String, Finished>(finished())
        }
    }
}

fun close__2(streams: Streams, p: Lines): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
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
    val got = streams.readBytes(p.s, p.size)
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

fun close__3(streams: Streams, p: Chunks): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(p.s)
}

fun streamChunkSize(): Int {
    return 65536
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun fillFrom(streams: Streams, s: InStream, buf: salvo.SalvoBytes): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    var total: Long = 0L
    var reading = true
    while (reading) {
        val got = streams.readTo(s, buf, streamChunkSize())
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
fun copyStream(streams: Streams, s: InStream, w: OutStream): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val buf = salvo.SalvoBytes.joined()
    var total: Long = 0L
    var copying = true
    while (copying) {
        buf.clear()
        val got = streams.readTo(s, buf, streamChunkSize())
        if (got is U2_2<*, *>) {
            return U2_2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val n: Int = (got.value as Int)
        if (n == 0) {
            copying = false
        } else {
            total = total + streams.writeBytes(w, buf)
        }
    }
    return U2_1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(total))
}
