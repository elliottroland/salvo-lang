package salvo.stream

import salvo.*

fun freshHandle(): Long {
    return salvo.runtime.streams.freshHandle()
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
fun toStr(kind: Union2<InvalidUtf8, StreamFailed>): String {
    return when {
        (kind is Union2.U1<*, *>) -> {
            val kind_1: InvalidUtf8 = ((kind as Union2.U1<*, *>).value as InvalidUtf8)
            return "not valid UTF-8: ${kind_1.source}"
        }
        (kind is Union2.U2<*, *>) -> {
            val kind_2: StreamFailed = ((kind as Union2.U2<*, *>).value as StreamFailed)
            return "stream failed: ${kind_2.source}: ${kind_2.message}"
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

data class InStream(
    val handle: Long,
)

data class OutStream(
    val handle: Long,
)

data class Packet(
    val bytes: salvo.platform.core.bytes.Bytes,
    val stream: InStream,
)

class End

object __Codec_End : salvo.WireCodec<End> {
    override fun enc(v: End, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): End = End()
}

fun close__Packet(streams: Streams, p: Packet): Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val __destructured_1: Packet = p
    val bytes: salvo.platform.core.bytes.Bytes = __destructured_1.bytes
    val stream: InStream = __destructured_1.stream
    return streams.close__InStream(stream)
}

interface Streams {
    fun readLine(s: InStream): String?
    fun readAll(s: InStream): Union2<String, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun readBytes(s: InStream, max: Int): Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun readTo__InStream_Bytes_Int(s: InStream, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun readTo__InStream_Str(s: InStream, buf: salvo.platform.core.string.MutStr): Union2<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun readLineTo(s: InStream, buf: salvo.platform.core.string.MutStr): Boolean
    fun position__InStream(s: InStream): Long
    fun close__InStream(s: InStream): Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun write(s: OutStream, text: String): Long
    fun writeLine(s: OutStream, text: String): Long
    fun writeBytes(s: OutStream, data: salvo.platform.core.bytes.Bytes): Long
    fun position__OutStream(s: OutStream): Long
    fun flush(s: OutStream): Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun close__OutStream(s: OutStream): Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>
    fun receive(s: InStream, reply: salvo.SalvoReply)
    fun fromBytes(data: salvo.platform.core.bytes.Bytes): InStream
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
    override fun readAll(s: InStream): Union2<String, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readAll(s) } finally { lock.unlock() }
    }
    override fun readBytes(s: InStream, max: Int): Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readBytes(s, max) } finally { lock.unlock() }
    }
    override fun readTo__InStream_Bytes_Int(s: InStream, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readTo__InStream_Bytes_Int(s, buf, max) } finally { lock.unlock() }
    }
    override fun readTo__InStream_Str(s: InStream, buf: salvo.platform.core.string.MutStr): Union2<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readTo__InStream_Str(s, buf) } finally { lock.unlock() }
    }
    override fun readLineTo(s: InStream, buf: salvo.platform.core.string.MutStr): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.readLineTo(s, buf) } finally { lock.unlock() }
    }
    override fun position__InStream(s: InStream): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.position__InStream(s) } finally { lock.unlock() }
    }
    override fun close__InStream(s: InStream): Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.close__InStream(s) } finally { lock.unlock() }
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
    override fun writeBytes(s: OutStream, data: salvo.platform.core.bytes.Bytes): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.writeBytes(s, data) } finally { lock.unlock() }
    }
    override fun position__OutStream(s: OutStream): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.position__OutStream(s) } finally { lock.unlock() }
    }
    override fun flush(s: OutStream): Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.flush(s) } finally { lock.unlock() }
    }
    override fun close__OutStream(s: OutStream): Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.close__OutStream(s) } finally { lock.unlock() }
    }
    override fun receive(s: InStream, reply: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.receive(s, reply) } finally { lock.unlock() }
    }
    override fun fromBytes(data: salvo.platform.core.bytes.Bytes): InStream {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.fromBytes(data) } finally { lock.unlock() }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun pipe(streams: Streams, from: InStream, to: OutStream, done: salvo.SalvoReply) {
    streams.receive(from, run { val __e0 = streams; val __c0 = to; val __c1 = done; val __c2 = 0L; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { _: ByteArray -> Pair(false, null) }) { __v -> pipeStep(__e0, __c0, __c1, __c2, __v as Union3<Packet, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>) } })
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun pipeStep(streams: Streams, to: OutStream, done: salvo.SalvoReply, moved: Long, got: Union3<Packet, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>) {
    when {
        (got is Union3.U1<*, *, *>) -> {
            val got_1: Packet = ((got as Union3.U1<*, *, *>).value as Packet)
            val __destructured_2: Packet = got_1
            val bytes: salvo.platform.core.bytes.Bytes = __destructured_2.bytes
            val stream: InStream = __destructured_2.stream
            val written: Long = streams.writeBytes(to, bytes)
            streams.receive(stream, run { val __e0 = streams; val __c0 = to; val __c1 = done; val __c2 = (moved + written); salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { _: ByteArray -> Pair(false, null) }) { __v -> pipeStep(__e0, __c0, __c1, __c2, __v as Union3<Packet, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>) } })
        }
        (got is Union3.U2<*, *, *>) -> {
            val got_3: End = ((got as Union3.U2<*, *, *>).value as End)
            val closed: Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> = streams.close__OutStream(to)
            when {
                (closed is Union2.U1<*, *>) -> {
                    val closed_4: Unit = ((closed as Union2.U1<*, *>).value as Unit)
                    salvo.SalvoSched.replyWire(done, Union2.U1<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(salvo.core.result.ok(moved)), salvo.Union2Codec(salvo.LongCodec, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))))
                }
                (closed is Union2.U2<*, *>) -> {
                    val closed_5: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
                    salvo.SalvoSched.replyWire(done, Union2.U2<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(closed_5), salvo.Union2Codec(salvo.LongCodec, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))))
                }
                else -> throw IllegalStateException("salvo: unreachable arm")
            }
        }
        (got is Union3.U3<*, *, *>) -> {
            val got_6: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((got as Union3.U3<*, *, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
            val closed: Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> = streams.close__OutStream(to)
            when {
                (closed is Union2.U1<*, *>) -> {
                    val closed_7: Unit = ((closed as Union2.U1<*, *>).value as Unit)
                }
                (closed is Union2.U2<*, *>) -> {
                    val closed_8: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
                    salvo.core.checked.ignore(closed_8)
                }
                else -> throw IllegalStateException("salvo: unreachable arm")
            }
            salvo.SalvoSched.replyWire(done, Union2.U2<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(got_6), salvo.Union2Codec(salvo.LongCodec, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))))
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

data class Lines(
    var s: InStream,
)

fun lines(s: InStream): Lines {
    return Lines(s = s)
}

fun next__Lines(streams: Streams, p: Lines): Union2<String, salvo.core.iterator.Finished> {
    val line: String? = streams.readLine(p.s)
    return when {
        (line != null) -> {
            val line_1: String = line!!
            return Union2.U1<String, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(line_1))
        }
        (line == null) -> {
            return Union2.U2<String, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

fun close__Lines(streams: Streams, p: Lines): Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close__InStream(p.s)
}

data class Chunks(
    var s: InStream,
    var size: Int,
)

fun chunks(s: InStream, size: Int): Chunks {
    return Chunks(s = s, size = size)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun next__Chunks(streams: Streams, p: Chunks): Union2<salvo.platform.core.bytes.Bytes, salvo.core.iterator.Finished> {
    val got: Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> = streams.readBytes(p.s, p.size)
    if ((got is Union2.U2<*, *>)) {
        val got_1: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
        salvo.core.checked.ignore(got_1)
        return Union2.U2<salvo.platform.core.bytes.Bytes, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val got_2: salvo.platform.core.bytes.Bytes = ((got as Union2.U1<*, *>).value as salvo.platform.core.bytes.Bytes)
    val data: salvo.platform.core.bytes.Bytes = got_2
    if (((salvo.core.bytes.sizePlatform(data)) == (0))) {
        return Union2.U2<salvo.platform.core.bytes.Bytes, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    return Union2.U1<salvo.platform.core.bytes.Bytes, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(data))
}

fun close__Chunks(streams: Streams, p: Chunks): Union2<Unit, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close__InStream(p.s)
}

fun streamChunkSize(): Int {
    return 65536
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun fillFrom(streams: Streams, s: InStream, buf: salvo.platform.core.bytes.MutBytes): Union2<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
    var total: Long = 0L
    var reading: Boolean = true
    while (true) {
        if (!(reading)) {
            break
        }
        val got: Union2<Int, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> = streams.readTo__InStream_Bytes_Int(s, buf, streamChunkSize())
        if ((got is Union2.U2<*, *>)) {
            val got_1: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
            return Union2.U2<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(got_1)
        }
        val got_2: Int = ((got as Union2.U1<*, *>).value as Int)
        val n: Int = got_2
        total = (total + (n).toLong())
        if (((n) == (0))) {
            reading = false
        }
    }
    return Union2.U1<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(salvo.core.result.ok(total))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun copyStream(streams: Streams, s: InStream, w: OutStream): Union2<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val buf: salvo.platform.core.bytes.MutBytes = salvo.core.bytes.mutBytes(arrayOf<salvo.platform.core.bytes.Bytes>())
    var total: Long = 0L
    var copying: Boolean = true
    while (true) {
        if (!(copying)) {
            break
        }
        salvo.core.bytes.clearPlatform(buf)
        val got: Union2<Int, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> = streams.readTo__InStream_Bytes_Int(s, buf, streamChunkSize())
        if ((got is Union2.U2<*, *>)) {
            val got_1: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
            return Union2.U2<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(got_1)
        }
        val got_2: Int = ((got as Union2.U1<*, *>).value as Int)
        val n: Int = got_2
        if (((n) == (0))) {
            copying = false
        } else {
            total = (total + streams.writeBytes(w, buf))
        }
    }
    return Union2.U1<Long, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(salvo.core.result.ok(total))
}

fun writeInt(streams: Streams, s: OutStream, v: Int): Long {
    return streams.writeBytes(s, fixedBytes((v).toLong(), 4))
}

fun writeLong(streams: Streams, s: OutStream, v: Long): Long {
    return streams.writeBytes(s, fixedBytes(v, 8))
}

fun<T> writeValue(streams: Streams, s: OutStream, v: T, encode: (T) -> salvo.platform.core.bytes.Bytes): Long {
    val data: salvo.platform.core.bytes.Bytes = encode(v)
    val n: Long = writeInt(streams, s, salvo.core.bytes.sizePlatform(data))
    return (n + streams.writeBytes(s, data))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readInt(streams: Streams, s: InStream): Union3<Int, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val r: Union3<Long, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> = readFixed(streams, s, 4)
    return when {
        (r is Union3.U1<*, *, *>) -> {
            val r_1: Long = ((r as Union3.U1<*, *, *>).value as Long)
            return Union3.U1<Int, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(salvo.core.result.ok((r_1).toInt()))
        }
        (r is Union3.U2<*, *, *>) -> {
            val r_2: End = ((r as Union3.U2<*, *, *>).value as End)
            return Union3.U2<Int, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(End())
        }
        (r is Union3.U3<*, *, *>) -> {
            val r_3: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((r as Union3.U3<*, *, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
            return Union3.U3<Int, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(r_3)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

fun readLong(streams: Streams, s: InStream): Union3<Long, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return readFixed(streams, s, 8)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<T> readValue(streams: Streams, s: InStream, decode: (salvo.platform.core.bytes.Bytes) -> T?): Union3<T, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val len: Union3<Int, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> = readInt(streams, s)
    if ((len is Union3.U2<*, *, *>)) {
        val len_1: End = ((len as Union3.U2<*, *, *>).value as End)
        return Union3.U2<T, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(End())
    }
    if ((len is Union3.U3<*, *, *>)) {
        val len_2: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((len as Union3.U3<*, *, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
        return Union3.U3<T, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(len_2)
    }
    val len_3: Int = ((len as Union3.U1<*, *, *>).value as Int)
    val n: Int = len_3
    val data: Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> = streams.readBytes(s, n)
    if ((data is Union2.U2<*, *>)) {
        val data_4: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((data as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
        return Union3.U3<T, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(data_4)
    }
    val data_5: salvo.platform.core.bytes.Bytes = ((data as Union2.U1<*, *>).value as salvo.platform.core.bytes.Bytes)
    val bytes: salvo.platform.core.bytes.Bytes = data_5
    if ((salvo.core.bytes.sizePlatform(bytes) < n)) {
        return Union3.U3<T, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U2<InvalidUtf8, StreamFailed>(StreamFailed(source = "read_value", message = "the stream ended inside a value")))))
    }
    val v: T? = decode(bytes)
    if ((v == null)) {
        return Union3.U3<T, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U2<InvalidUtf8, StreamFailed>(StreamFailed(source = "read_value", message = "the bytes are not a value of the type asked for")))))
    }
    val v_6: T = v!!
    return Union3.U1<T, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(salvo.core.result.ok(v_6))
}

fun fixedBytes(v: Long, width: Int): salvo.platform.core.bytes.Bytes {
    val out: salvo.platform.core.bytes.MutBytes = salvo.core.bytes.mutBytes(arrayOf<salvo.platform.core.bytes.Bytes>())
    var rest: Long = v
    var i: Int = 0
    while (true) {
        if (!((i < width))) {
            break
        }
        var low: Long = (rest % 256L)
        if ((low < 0L)) {
            low = (low + 256L)
        }
        salvo.core.bytes.addPlatform(out, ((low).toInt()).toUByte())
        rest = ((rest - low) / 256L)
        i = (i + 1)
    }
    val back: salvo.platform.core.bytes.MutBytes = salvo.core.bytes.mutBytes(arrayOf<salvo.platform.core.bytes.Bytes>())
    var j: Int = (width - 1)
    while (true) {
        if (!((j >= 0))) {
            break
        }
        salvo.core.bytes.addPlatform(back, run {
            val __nn_1: UByte? = salvo.core.bytes.getPlatform(out, j)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at stream:421:19"))
                }
                else -> {
                    val __some_2: UByte = __nn_1!!
                    __some_2
                }
            }
        })
        j = (j - 1)
    }
    return back
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readFixed(streams: Streams, s: InStream, width: Int): Union3<Long, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val r: Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>> = streams.readBytes(s, width)
    if ((r is Union2.U2<*, *>)) {
        val r_1: salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>> = ((r as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>)
        return Union3.U3<Long, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(r_1)
    }
    val r_2: salvo.platform.core.bytes.Bytes = ((r as Union2.U1<*, *>).value as salvo.platform.core.bytes.Bytes)
    val data: salvo.platform.core.bytes.Bytes = r_2
    if (((salvo.core.bytes.sizePlatform(data)) == (0))) {
        return Union3.U2<Long, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(End())
    }
    if ((salvo.core.bytes.sizePlatform(data) < width)) {
        return Union3.U3<Long, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U2<InvalidUtf8, StreamFailed>(StreamFailed(source = "read", message = "the stream ended inside a number")))))
    }
    var first: Long = ((run {
        val __nn_3: UByte? = salvo.core.bytes.getPlatform(data, 0)
        when {
            (__nn_3 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at stream:440:32"))
            }
            else -> {
                val __some_4: UByte = __nn_3!!
                __some_4
            }
        }
    }).toInt()).toLong()
    if ((first >= 128L)) {
        first = (first - 256L)
    }
    var v: Long = first
    var i: Int = 1
    while (true) {
        if (!((i < width))) {
            break
        }
        v = ((v * 256L) + ((run {
            val __nn_5: UByte? = salvo.core.bytes.getPlatform(data, i)
            when {
                (__nn_5 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at stream:447:39"))
                }
                else -> {
                    val __some_6: UByte = __nn_5!!
                    __some_6
                }
            }
        }).toInt()).toLong())
        i = (i + 1)
    }
    return Union3.U1<Long, End, salvo.core.checked.Checked<Union2<InvalidUtf8, StreamFailed>>>(salvo.core.result.ok(v))
}

