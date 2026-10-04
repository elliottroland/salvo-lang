package salvo.stream

import salvo.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.deque.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.runtime.streams.*

fun freshHandle__2(): Long {
    return freshHandle()
}

// Factories for the host: one per arm of the union [platform-factory].
object StreamErrors {
    fun invalidUtf8(value: InvalidUtf8): Union2<InvalidUtf8, StreamFailed> = salvo.Union2.U1(value)
    fun streamFailed(value: StreamFailed): Union2<InvalidUtf8, StreamFailed> = salvo.Union2.U2(value)
}

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

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun toStr__5(kind: Union2<InvalidUtf8, StreamFailed>): String {
    when (kind) {
        is Union2.U1<*, *> -> {
            return "not valid UTF-8: ${(kind.value as InvalidUtf8).source}"
        }
        is Union2.U2<*, *> -> {
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

class __Mon_Streams(
    private val inner: Streams,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Streams {
    override fun readLine(s: InStream): String? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readLine(s) } finally { lock.unlock() }
    }
    override fun readAll(s: InStream): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readAll(s) } finally { lock.unlock() }
    }
    override fun readBytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readBytes(s, max) } finally { lock.unlock() }
    }
    override fun readTo(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readTo(s, buf, max) } finally { lock.unlock() }
    }
    override fun readTo__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readTo__2(s, buf) } finally { lock.unlock() }
    }
    override fun readLineTo(s: InStream, buf: StringBuilder): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readLineTo(s, buf) } finally { lock.unlock() }
    }
    override fun position(s: InStream): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.position(s) } finally { lock.unlock() }
    }
    override fun close(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.close(s) } finally { lock.unlock() }
    }
    override fun write(s: OutStream, text: String): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.write(s, text) } finally { lock.unlock() }
    }
    override fun writeLine(s: OutStream, text: String): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.writeLine(s, text) } finally { lock.unlock() }
    }
    override fun writeBytes(s: OutStream, data: salvo.SalvoBytes): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.writeBytes(s, data) } finally { lock.unlock() }
    }
    override fun position__2(s: OutStream): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.position__2(s) } finally { lock.unlock() }
    }
    override fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.flush(s) } finally { lock.unlock() }
    }
    override fun close__2(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.close__2(s) } finally { lock.unlock() }
    }
    override fun receive(s: InStream, reply: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.receive(s, reply) } finally { lock.unlock() }
    }
    override fun fromBytes(data: salvo.SalvoBytes): InStream {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.fromBytes(data) } finally { lock.unlock() }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun pipe(streams: Streams, from: InStream, to: OutStream, done: salvo.SalvoReply) {
    streams.receive(from, run { val __e0 = streams; val __c0 = to; val __c1 = done; val __c2 = 0L; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { _: ByteArray -> Pair(false, null) }) { __v -> pipeStep(__e0, __c0, __c1, __c2, __v as Union3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>) } })
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun pipeStep(streams: Streams, to: OutStream, done: salvo.SalvoReply, moved: Long, got: Union3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>) {
    when (got) {
        is Union3.U1<*, *, *> -> {
            val __destructured2 = (got.value as Packet)
            val bytes = __destructured2.bytes
            val stream = __destructured2.stream
            val written = streams.writeBytes(to, bytes)
            streams.receive(stream, run { val __e0 = streams; val __c0 = to; val __c1 = done; val __c2 = moved + written; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { _: ByteArray -> Pair(false, null) }) { __v -> pipeStep(__e0, __c0, __c1, __c2, __v as Union3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>) } })
        }
        is Union3.U2<*, *, *> -> {
            val closed = streams.close__2(to)
            when (closed) {
                is Union2.U1<*, *> -> {
                    salvo.SalvoSched.replyWire(done, Union2.U1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(moved)), salvo.Union2Codec(salvo.LongCodec, __Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))))
                }
                is Union2.U2<*, *> -> {
                    salvo.SalvoSched.replyWire(done, Union2.U2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>)), salvo.Union2Codec(salvo.LongCodec, __Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))))
                }
            }
        }
        is Union3.U3<*, *, *> -> {
            val closed = streams.close__2(to)
            when (closed) {
                is Union2.U1<*, *> -> {
                }
                is Union2.U2<*, *> -> {
                    ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
                }
            }
            salvo.SalvoSched.replyWire(done, Union2.U2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>)), salvo.Union2Codec(salvo.LongCodec, __Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))))
        }
    }
}

data class Lines(
    var s: InStream,
)

fun lines__2(s: InStream): Lines {
    return Lines(s = s)
}

fun next__21(streams: Streams, p: Lines): Union2<String, Finished> {
    val line = streams.readLine(p.s)
    when {
        line != null -> {
            return Union2.U1<String, Finished>(emitted(line))
        }
        else -> {
            return Union2.U2<String, Finished>(finished())
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

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun next__22(streams: Streams, p: Chunks): Union2<salvo.SalvoBytes, Finished> {
    val got = streams.readBytes(p.s, p.size)
    if (got is Union2.U2<*, *>) {
        ignore((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        return Union2.U2<salvo.SalvoBytes, Finished>(finished())
    }
    val data: salvo.SalvoBytes = (got.value as salvo.SalvoBytes)
    if (data.size == 0) {
        return Union2.U2<salvo.SalvoBytes, Finished>(finished())
    }
    return Union2.U1<salvo.SalvoBytes, Finished>(emitted(data))
}

fun close__3(streams: Streams, p: Chunks): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(p.s)
}

fun streamChunkSize(): Int {
    return 65536
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun fillFrom(streams: Streams, s: InStream, buf: salvo.SalvoBytes): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    var total: Long = 0L
    var reading = true
    while (reading) {
        val got = streams.readTo(s, buf, streamChunkSize())
        if (got is Union2.U2<*, *>) {
            return Union2.U2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val n: Int = (got.value as Int)
        total = total + (n).toLong()
        if (n == 0) {
            reading = false
        }
    }
    return Union2.U1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(total))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun copyStream(streams: Streams, s: InStream, w: OutStream): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val buf = salvo.SalvoBytes.joined()
    var total: Long = 0L
    var copying = true
    while (copying) {
        buf.clear()
        val got = streams.readTo(s, buf, streamChunkSize())
        if (got is Union2.U2<*, *>) {
            return Union2.U2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val n: Int = (got.value as Int)
        if (n == 0) {
            copying = false
        } else {
            total = total + streams.writeBytes(w, buf)
        }
    }
    return Union2.U1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(total))
}

fun writeInt(streams: Streams, s: OutStream, v: Int): Long {
    return streams.writeBytes(s, fixedBytes((v).toLong(), 4))
}

fun writeLong(streams: Streams, s: OutStream, v: Long): Long {
    return streams.writeBytes(s, fixedBytes(v, 8))
}

fun<T> writeValue(streams: Streams, s: OutStream, v: T, encode: (T) -> salvo.SalvoBytes): Long {
    val data = encode(v)
    val n = writeInt(streams, s, data.size)
    return n + streams.writeBytes(s, data)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readInt(streams: Streams, s: InStream): Union3<Int, End, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val r = readFixed(streams, s, 4)
    when (r) {
        is Union3.U1<*, *, *> -> {
            return Union3.U1<Int, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(((r.value as Long)).toInt()))
        }
        is Union3.U2<*, *, *> -> {
            return Union3.U2<Int, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(End())
        }
        is Union3.U3<*, *, *> -> {
            return Union3.U3<Int, End, Checked<Union2<InvalidUtf8, StreamFailed>>>((r.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
    }
}

fun readLong(streams: Streams, s: InStream): Union3<Long, End, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return readFixed(streams, s, 8)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<T> readValue(streams: Streams, s: InStream, decode: (salvo.SalvoBytes) -> T?): Union3<T, End, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val len = readInt(streams, s)
    if (len is Union3.U2<*, *, *>) {
        return Union3.U2<T, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(End())
    }
    if (len is Union3.U3<*, *, *>) {
        return Union3.U3<T, End, Checked<Union2<InvalidUtf8, StreamFailed>>>((len.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
    }
    val n: Int = (len.value as Int)
    val data = streams.readBytes(s, n)
    if (data is Union2.U2<*, *>) {
        return Union3.U3<T, End, Checked<Union2<InvalidUtf8, StreamFailed>>>((data.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
    }
    val bytes: salvo.SalvoBytes = (data.value as salvo.SalvoBytes)
    if (bytes.size < n) {
        return Union3.U3<T, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U2<InvalidUtf8, StreamFailed>(StreamFailed(source = "read_value", message = "the stream ended inside a value")))))
    }
    val v = decode(bytes)
    if (v == null) {
        return Union3.U3<T, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U2<InvalidUtf8, StreamFailed>(StreamFailed(source = "read_value", message = "the bytes are not a value of the type asked for")))))
    }
    return Union3.U1<T, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(v))
}

fun fixedBytes(v: Long, width: Int): salvo.SalvoBytes {
    val out = salvo.SalvoBytes.joined()
    var rest = v
    var i = 0
    while (i < width) {
        var low = rest % 256L
        if (low < 0L) {
            low = low + 256L
        }
        out.add(((low).toInt()).toUByte())
        rest = (rest - low) / 256L
        i = i + 1
    }
    val back = salvo.SalvoBytes.joined()
    var j = width - 1
    while (j >= 0) {
        back.add((out.getOrNull(j) ?: throw AssertionError("salvo: value is absent at stream:421:19")))
        j = j - 1
    }
    return back
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readFixed(streams: Streams, s: InStream, width: Int): Union3<Long, End, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val r = streams.readBytes(s, width)
    if (r is Union2.U2<*, *>) {
        return Union3.U3<Long, End, Checked<Union2<InvalidUtf8, StreamFailed>>>((r.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
    }
    val data: salvo.SalvoBytes = (r.value as salvo.SalvoBytes)
    if (data.size == 0) {
        return Union3.U2<Long, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(End())
    }
    if (data.size < width) {
        return Union3.U3<Long, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U2<InvalidUtf8, StreamFailed>(StreamFailed(source = "read", message = "the stream ended inside a number")))))
    }
    var first = (((data.getOrNull(0) ?: throw AssertionError("salvo: value is absent at stream:440:32"))).toInt()).toLong()
    if (first >= 128L) {
        first = first - 256L
    }
    var v = first
    var i = 1
    while (i < width) {
        v = v * 256L + (((data.getOrNull(i) ?: throw AssertionError("salvo: value is absent at stream:447:39"))).toInt()).toLong()
        i = i + 1
    }
    return Union3.U1<Long, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(v))
}
