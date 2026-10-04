package salvo.stream.host

import salvo.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.deque.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.runtime.streams.*
import salvo.stream.*

interface RawStreams {
    fun rawReadLine(handle: Long): String?
    fun rawReadAll(handle: Long): Union2<String, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadBytes(handle: Long, max: Int): Union2<salvo.platform.core.bytes.Bytes, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadToBytes(handle: Long, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadToStr(handle: Long, buf: StringBuilder): Union2<Long, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadLineToStr(handle: Long, buf: StringBuilder): Boolean
    fun rawReadPosition(handle: Long): Long
    fun rawCloseRead(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun rawWrite(handle: Long, text: String): Long
    fun rawWriteBytes(handle: Long, data: salvo.platform.core.bytes.Bytes): Long
    fun rawWritePosition(handle: Long): Long
    fun rawFlush(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun rawCloseWrite(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun rawReceive(handle: Long, reply: salvo.SalvoReply)
    fun rawFromBytes(data: salvo.platform.core.bytes.Bytes): Long
}

class __Mon_RawStreams(
    private val inner: RawStreams,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : RawStreams {
    override fun rawReadLine(handle: Long): String? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadLine(handle) } finally { lock.unlock() }
    }
    override fun rawReadAll(handle: Long): Union2<String, Union2<InvalidUtf8, StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadAll(handle) } finally { lock.unlock() }
    }
    override fun rawReadBytes(handle: Long, max: Int): Union2<salvo.platform.core.bytes.Bytes, Union2<InvalidUtf8, StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadBytes(handle, max) } finally { lock.unlock() }
    }
    override fun rawReadToBytes(handle: Long, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadToBytes(handle, buf, max) } finally { lock.unlock() }
    }
    override fun rawReadToStr(handle: Long, buf: StringBuilder): Union2<Long, Union2<InvalidUtf8, StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadToStr(handle, buf) } finally { lock.unlock() }
    }
    override fun rawReadLineToStr(handle: Long, buf: StringBuilder): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadLineToStr(handle, buf) } finally { lock.unlock() }
    }
    override fun rawReadPosition(handle: Long): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadPosition(handle) } finally { lock.unlock() }
    }
    override fun rawCloseRead(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawCloseRead(handle) } finally { lock.unlock() }
    }
    override fun rawWrite(handle: Long, text: String): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawWrite(handle, text) } finally { lock.unlock() }
    }
    override fun rawWriteBytes(handle: Long, data: salvo.platform.core.bytes.Bytes): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawWriteBytes(handle, data) } finally { lock.unlock() }
    }
    override fun rawWritePosition(handle: Long): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawWritePosition(handle) } finally { lock.unlock() }
    }
    override fun rawFlush(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawFlush(handle) } finally { lock.unlock() }
    }
    override fun rawCloseWrite(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawCloseWrite(handle) } finally { lock.unlock() }
    }
    override fun rawReceive(handle: Long, reply: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.rawReceive(handle, reply) } finally { lock.unlock() }
    }
    override fun rawFromBytes(data: salvo.platform.core.bytes.Bytes): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawFromBytes(data) } finally { lock.unlock() }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun hostReceived(reply: salvo.SalvoReply, handle: Long, got: Union3<salvo.platform.core.bytes.Bytes, End, Union2<InvalidUtf8, StreamFailed>>) {
    when (got) {
        is Union3.U1<*, *, *> -> {
            reply.send(Union3.U1<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Packet(bytes = (got.value as salvo.platform.core.bytes.Bytes), stream = InStream(handle = handle)))))
        }
        is Union3.U2<*, *, *> -> {
            reply.send(Union3.U2<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as End)))
        }
        is Union3.U3<*, *, *> -> {
            reply.send(Union3.U3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((got.value as Union2<InvalidUtf8, StreamFailed>)))))
        }
    }
}

class HostRawStreams : RawStreams {

    override fun rawReadLine(handle: Long): String? {
        return nextLine(handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawReadAll(handle: Long): Union2<String, Union2<InvalidUtf8, StreamFailed>> {
        val e = checkoutIn(handle)
        val r = readAll(e)
        val source = e.source
        if (r.fault != null) {
            val f = r.fault as salvo.runtime.streams.Fault
            checkinIn(handle, e)
            return Union2.U2<String, Union2<InvalidUtf8, StreamFailed>>(err(kind(source, f)))
        }
        val text = decode(e, r.data)
        checkinIn(handle, e)
        if (text != null) {
            val t = text as String
            return Union2.U1<String, Union2<InvalidUtf8, StreamFailed>>(ok(t))
        }
        return Union2.U2<String, Union2<InvalidUtf8, StreamFailed>>(err(kind(source, salvo.runtime.streams.Fault(utf8 = true, message = ""))))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawReadBytes(handle: Long, max: Int): Union2<salvo.platform.core.bytes.Bytes, Union2<InvalidUtf8, StreamFailed>> {
        val e = checkoutIn(handle)
        val r = readUpTo(e, max)
        val source = e.source
        checkinIn(handle, e)
        if (r.fault != null) {
            val f = r.fault as salvo.runtime.streams.Fault
            return Union2.U2<salvo.platform.core.bytes.Bytes, Union2<InvalidUtf8, StreamFailed>>(err(kind(source, f)))
        }
        return Union2.U1<salvo.platform.core.bytes.Bytes, Union2<InvalidUtf8, StreamFailed>>(ok(r.data))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawReadToBytes(handle: Long, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>> {
        val e = checkoutIn(handle)
        val r = readUpTo(e, max)
        val source = e.source
        checkinIn(handle, e)
        if (r.fault != null) {
            val f = r.fault as salvo.runtime.streams.Fault
            return Union2.U2<Int, Union2<InvalidUtf8, StreamFailed>>(err(kind(source, f)))
        }
        appendPlatform(buf, r.data)
        return Union2.U1<Int, Union2<InvalidUtf8, StreamFailed>>(ok(sizePlatform(r.data)))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawReadToStr(handle: Long, buf: StringBuilder): Union2<Long, Union2<InvalidUtf8, StreamFailed>> {
        val e = checkoutIn(handle)
        val r = readAll(e)
        val source = e.source
        if (r.fault != null) {
            val f = r.fault as salvo.runtime.streams.Fault
            checkinIn(handle, e)
            return Union2.U2<Long, Union2<InvalidUtf8, StreamFailed>>(err(kind(source, f)))
        }
        val count = (sizePlatform(r.data)).toLong()
        val text = decode(e, r.data)
        checkinIn(handle, e)
        if (text != null) {
            val t = text as String
            buf.append(t)
            return Union2.U1<Long, Union2<InvalidUtf8, StreamFailed>>(ok(count))
        }
        return Union2.U2<Long, Union2<InvalidUtf8, StreamFailed>>(err(kind(source, salvo.runtime.streams.Fault(utf8 = true, message = ""))))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawReadLineToStr(handle: Long, buf: StringBuilder): Boolean {
        val line = nextLine(handle)
        if (line != null) {
            val t = line as String
            buf.append(t)
            return true
        }
        return false
    }

    override fun rawReadPosition(handle: Long): Long {
        val e = checkoutIn(handle)
        val at = e.position
        checkinIn(handle, e)
        return at
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawCloseRead(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> {
        val e = checkoutIn(handle)
        val source = e.source
        val failed = closeIn(handle, e)
        if (failed != null) {
            val f = failed as salvo.runtime.streams.Fault
            return Union2.U2<Unit, Union2<InvalidUtf8, StreamFailed>>(err(kind(source, f)))
        }
        return Union2.U1<Unit, Union2<InvalidUtf8, StreamFailed>>(ok(Unit))
    }

    override fun rawWrite(handle: Long, text: String): Long {
        val e = checkoutOut(handle)
        val n = write(e, toBytesPlatform(text))
        checkinOut(handle, e)
        return n
    }

    override fun rawWriteBytes(handle: Long, data: salvo.platform.core.bytes.Bytes): Long {
        val e = checkoutOut(handle)
        val n = write(e, data)
        checkinOut(handle, e)
        return n
    }

    override fun rawWritePosition(handle: Long): Long {
        val e = checkoutOut(handle)
        val at = e.position
        checkinOut(handle, e)
        return at
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawFlush(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> {
        val e = checkoutOut(handle)
        val source = e.source
        val failed = flush__2(e)
        checkinOut(handle, e)
        if (failed != null) {
            val f = failed as salvo.runtime.streams.Fault
            return Union2.U2<Unit, Union2<InvalidUtf8, StreamFailed>>(err(kind(source, f)))
        }
        return Union2.U1<Unit, Union2<InvalidUtf8, StreamFailed>>(ok(Unit))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawCloseWrite(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> {
        val e = checkoutOut(handle)
        val source = e.source
        val failed = closeOut(handle, e)
        if (failed != null) {
            val f = failed as salvo.runtime.streams.Fault
            return Union2.U2<Unit, Union2<InvalidUtf8, StreamFailed>>(err(kind(source, f)))
        }
        return Union2.U1<Unit, Union2<InvalidUtf8, StreamFailed>>(ok(Unit))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawReceive(handle: Long, reply: salvo.SalvoReply) {
        receive(handle, run { val __c0 = reply; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Chunk) }) { __v -> chunkReceived(__c0, __v as Chunk) } })
    }

    override fun rawFromBytes(data: salvo.platform.core.bytes.Bytes): Long {
        return registerBytes(data)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun chunkReceived(reply: salvo.SalvoReply, c: Chunk) {
    if (c.fault != null) {
        val f = c.fault as salvo.runtime.streams.Fault
        salvo.SalvoSched.replyWire(reply, Union3.U3<salvo.platform.core.bytes.Bytes, End, Union2<InvalidUtf8, StreamFailed>>(err(kind(c.source, f))), salvo.Union3Codec(salvo.BytesCodec, __Codec_End, salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed)))
    } else if (c.end) {
        salvo.SalvoSched.replyWire(reply, Union3.U2<salvo.platform.core.bytes.Bytes, End, Union2<InvalidUtf8, StreamFailed>>(End()), salvo.Union3Codec(salvo.BytesCodec, __Codec_End, salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed)))
    } else {
        salvo.SalvoSched.replyWire(reply, Union3.U1<salvo.platform.core.bytes.Bytes, End, Union2<InvalidUtf8, StreamFailed>>(ok(c.data)), salvo.Union3Codec(salvo.BytesCodec, __Codec_End, salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed)))
    }
}

fun nextLine(handle: Long): String? {
    val e = checkoutIn(handle)
    val r = readLine(e)
    var text: String? = null
    if (!r.end && (r.fault == null)) {
        text = decode(e, r.data)
    }
    checkinIn(handle, e)
    return text
}

fun kind(source: String, f: salvo.runtime.streams.Fault): Union2<InvalidUtf8, StreamFailed> {
    if (f.utf8) {
        return Union2.U1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source))
    }
    return Union2.U2<InvalidUtf8, StreamFailed>(StreamFailed(source = source, message = f.message))
}

class DefaultStreams(private val __dep_RawStreams: RawStreams) : salvo.stream.Streams {

    override fun readLine(s: InStream): String? {
        return __dep_RawStreams.rawReadLine(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readAll(s: InStream): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.rawReadAll(s.handle)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok((r.value as String)))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readBytes(s: InStream, max: Int): Union2<salvo.platform.core.bytes.Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.rawReadBytes(s.handle, max)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<salvo.platform.core.bytes.Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok((r.value as salvo.platform.core.bytes.Bytes)))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<salvo.platform.core.bytes.Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo(s: InStream, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.rawReadToBytes(s.handle, buf, max)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok((r.value as Int)))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.rawReadToStr(s.handle, buf)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok((r.value as Long)))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    override fun readLineTo(s: InStream, buf: StringBuilder): Boolean {
        return __dep_RawStreams.rawReadLineToStr(s.handle, buf)
    }

    override fun position(s: InStream): Long {
        return __dep_RawStreams.rawReadPosition(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun close(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.rawCloseRead(s.handle)
        (s).let {}
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun receive(s: InStream, reply: salvo.SalvoReply) {
        val handle = s.handle
        (s).let {}
        __dep_RawStreams.rawReceive(handle, run { val __c0 = reply; val __c1 = handle; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union3Codec(salvo.BytesCodec, __Codec_End, salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))) }) { __v -> hostReceived(__c0, __c1, __v as Union3<salvo.platform.core.bytes.Bytes, End, Union2<InvalidUtf8, StreamFailed>>) } })
    }

    override fun fromBytes(data: salvo.platform.core.bytes.Bytes): InStream {
        return InStream(handle = __dep_RawStreams.rawFromBytes(data))
    }

    override fun write(s: OutStream, text: String): Long {
        return __dep_RawStreams.rawWrite(s.handle, text)
    }

    override fun writeLine(s: OutStream, text: String): Long {
        return __dep_RawStreams.rawWrite(s.handle, "$text\n")
    }

    override fun writeBytes(s: OutStream, data: salvo.platform.core.bytes.Bytes): Long {
        return __dep_RawStreams.rawWriteBytes(s.handle, data)
    }

    override fun position__2(s: OutStream): Long {
        return __dep_RawStreams.rawWritePosition(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.rawFlush(s.handle)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun close__2(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.rawCloseWrite(s.handle)
        (s).let {}
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }
}
