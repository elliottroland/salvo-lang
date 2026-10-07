package salvo.stream.host

import salvo.*

interface RawStreams {
    fun rawReadLine(handle: Long): String?
    fun rawReadAll(handle: Long): Union2<String, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>
    fun rawReadBytes(handle: Long, max: Int): Union2<salvo.platform.core.bytes.Bytes, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>
    fun rawReadToBytes(handle: Long, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>
    fun rawReadToStr(handle: Long, buf: salvo.platform.core.string.MutStr): Union2<Long, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>
    fun rawReadLineToStr(handle: Long, buf: salvo.platform.core.string.MutStr): Boolean
    fun rawReadPosition(handle: Long): Long
    fun rawCloseRead(handle: Long): Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>
    fun rawWrite(handle: Long, text: String): Long
    fun rawWriteBytes(handle: Long, data: salvo.platform.core.bytes.Bytes): Long
    fun rawWritePosition(handle: Long): Long
    fun rawFlush(handle: Long): Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>
    fun rawCloseWrite(handle: Long): Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>
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
    override fun rawReadAll(handle: Long): Union2<String, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadAll(handle) } finally { lock.unlock() }
    }
    override fun rawReadBytes(handle: Long, max: Int): Union2<salvo.platform.core.bytes.Bytes, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadBytes(handle, max) } finally { lock.unlock() }
    }
    override fun rawReadToBytes(handle: Long, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadToBytes(handle, buf, max) } finally { lock.unlock() }
    }
    override fun rawReadToStr(handle: Long, buf: salvo.platform.core.string.MutStr): Union2<Long, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadToStr(handle, buf) } finally { lock.unlock() }
    }
    override fun rawReadLineToStr(handle: Long, buf: salvo.platform.core.string.MutStr): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadLineToStr(handle, buf) } finally { lock.unlock() }
    }
    override fun rawReadPosition(handle: Long): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadPosition(handle) } finally { lock.unlock() }
    }
    override fun rawCloseRead(handle: Long): Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
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
    override fun rawFlush(handle: Long): Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawFlush(handle) } finally { lock.unlock() }
    }
    override fun rawCloseWrite(handle: Long): Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
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
fun hostReceived(reply: salvo.SalvoReply, handle: Long, got: Union3<salvo.platform.core.bytes.Bytes, salvo.stream.End, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>) {
    when {
        (got is Union3.U1<*, *, *>) -> {
            val got_1: salvo.platform.core.bytes.Bytes = ((got as Union3.U1<*, *, *>).value as salvo.platform.core.bytes.Bytes)
            reply.send(Union3.U1<salvo.stream.Packet, salvo.stream.End, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(salvo.stream.Packet(bytes = got_1, stream = salvo.stream.InStream(handle = handle)))))
        }
        (got is Union3.U2<*, *, *>) -> {
            val got_2: salvo.stream.End = ((got as Union3.U2<*, *, *>).value as salvo.stream.End)
            reply.send(Union3.U2<salvo.stream.Packet, salvo.stream.End, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(got_2))
        }
        (got is Union3.U3<*, *, *>) -> {
            val got_3: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed> = ((got as Union3.U3<*, *, *>).value as Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>)
            reply.send(Union3.U3<salvo.stream.Packet, salvo.stream.End, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(got_3))))
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

class HostRawStreams : RawStreams {
    override fun rawReadLine(handle: Long): String? {
        return nextLine(handle)
    }
    override fun rawReadAll(handle: Long): Union2<String, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        val e: salvo.runtime.streams.InEntry = salvo.runtime.streams.checkoutIn(handle)
        val r: salvo.runtime.streams.Read = salvo.runtime.streams.readAll(e)
        val source: String = e.source
        if ((r.fault != null)) {
            val f: salvo.runtime.streams.Fault = r.fault!!
            salvo.runtime.streams.checkinIn(handle, e)
            return Union2.U2<String, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(source, f)))
        }
        val text: String? = salvo.runtime.streams.decode(e, r.data)
        salvo.runtime.streams.checkinIn(handle, e)
        if ((text != null)) {
            val t: String = text!!
            return Union2.U1<String, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.ok(t))
        }
        return Union2.U2<String, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(source, salvo.runtime.streams.Fault(utf8 = true, message = ""))))
    }
    override fun rawReadBytes(handle: Long, max: Int): Union2<salvo.platform.core.bytes.Bytes, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        val e: salvo.runtime.streams.InEntry = salvo.runtime.streams.checkoutIn(handle)
        val r: salvo.runtime.streams.Read = salvo.runtime.streams.readUpTo(e, max)
        val source: String = e.source
        salvo.runtime.streams.checkinIn(handle, e)
        if ((r.fault != null)) {
            val f: salvo.runtime.streams.Fault = r.fault!!
            return Union2.U2<salvo.platform.core.bytes.Bytes, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(source, f)))
        }
        return Union2.U1<salvo.platform.core.bytes.Bytes, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.ok(r.data))
    }
    override fun rawReadToBytes(handle: Long, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        val e: salvo.runtime.streams.InEntry = salvo.runtime.streams.checkoutIn(handle)
        val r: salvo.runtime.streams.Read = salvo.runtime.streams.readUpTo(e, max)
        val source: String = e.source
        salvo.runtime.streams.checkinIn(handle, e)
        if ((r.fault != null)) {
            val f: salvo.runtime.streams.Fault = r.fault!!
            return Union2.U2<Int, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(source, f)))
        }
        salvo.core.bytes.appendPlatform(buf, r.data)
        return Union2.U1<Int, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.ok(salvo.core.bytes.sizePlatform(r.data)))
    }
    override fun rawReadToStr(handle: Long, buf: salvo.platform.core.string.MutStr): Union2<Long, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        val e: salvo.runtime.streams.InEntry = salvo.runtime.streams.checkoutIn(handle)
        val r: salvo.runtime.streams.Read = salvo.runtime.streams.readAll(e)
        val source: String = e.source
        if ((r.fault != null)) {
            val f: salvo.runtime.streams.Fault = r.fault!!
            salvo.runtime.streams.checkinIn(handle, e)
            return Union2.U2<Long, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(source, f)))
        }
        val count: Long = (salvo.core.bytes.sizePlatform(r.data)).toLong()
        val text: String? = salvo.runtime.streams.decode(e, r.data)
        salvo.runtime.streams.checkinIn(handle, e)
        if ((text != null)) {
            val t: String = text!!
            salvo.core.string.appendPlatform(buf, t)
            return Union2.U1<Long, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.ok(count))
        }
        return Union2.U2<Long, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(source, salvo.runtime.streams.Fault(utf8 = true, message = ""))))
    }
    override fun rawReadLineToStr(handle: Long, buf: salvo.platform.core.string.MutStr): Boolean {
        val line: String? = nextLine(handle)
        if ((line != null)) {
            val t: String = line!!
            salvo.core.string.appendPlatform(buf, t)
            return true
        }
        return false
    }
    override fun rawReadPosition(handle: Long): Long {
        val e: salvo.runtime.streams.InEntry = salvo.runtime.streams.checkoutIn(handle)
        val at: Long = e.position
        salvo.runtime.streams.checkinIn(handle, e)
        return at
    }
    override fun rawCloseRead(handle: Long): Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        val e: salvo.runtime.streams.InEntry = salvo.runtime.streams.checkoutIn(handle)
        val source: String = e.source
        val failed: salvo.runtime.streams.Fault? = salvo.runtime.streams.closeIn(handle, e)
        if ((failed != null)) {
            val f: salvo.runtime.streams.Fault = failed!!
            return Union2.U2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(source, f)))
        }
        return Union2.U1<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.ok(Unit))
    }
    override fun rawWrite(handle: Long, text: String): Long {
        val e: salvo.runtime.streams.OutEntry = salvo.runtime.streams.checkoutOut(handle)
        val n: Long = salvo.runtime.streams.write(e, salvo.core.bytes.toBytesPlatform(text))
        salvo.runtime.streams.checkinOut(handle, e)
        return n
    }
    override fun rawWriteBytes(handle: Long, data: salvo.platform.core.bytes.Bytes): Long {
        val e: salvo.runtime.streams.OutEntry = salvo.runtime.streams.checkoutOut(handle)
        val n: Long = salvo.runtime.streams.write(e, data)
        salvo.runtime.streams.checkinOut(handle, e)
        return n
    }
    override fun rawWritePosition(handle: Long): Long {
        val e: salvo.runtime.streams.OutEntry = salvo.runtime.streams.checkoutOut(handle)
        val at: Long = e.position
        salvo.runtime.streams.checkinOut(handle, e)
        return at
    }
    override fun rawFlush(handle: Long): Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        val e: salvo.runtime.streams.OutEntry = salvo.runtime.streams.checkoutOut(handle)
        val source: String = e.source
        val failed: salvo.runtime.streams.Fault? = salvo.runtime.streams.flush(e)
        salvo.runtime.streams.checkinOut(handle, e)
        if ((failed != null)) {
            val f: salvo.runtime.streams.Fault = failed!!
            return Union2.U2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(source, f)))
        }
        return Union2.U1<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.ok(Unit))
    }
    override fun rawCloseWrite(handle: Long): Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> {
        val e: salvo.runtime.streams.OutEntry = salvo.runtime.streams.checkoutOut(handle)
        val source: String = e.source
        val failed: salvo.runtime.streams.Fault? = salvo.runtime.streams.closeOut(handle, e)
        if ((failed != null)) {
            val f: salvo.runtime.streams.Fault = failed!!
            return Union2.U2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(source, f)))
        }
        return Union2.U1<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.ok(Unit))
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun rawReceive(handle: Long, reply: salvo.SalvoReply) {
        salvo.runtime.streams.receive(handle, run { val __c0 = reply; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.runtime.streams.__Codec_Chunk) }) { __v -> chunkReceived(__c0, __v as salvo.runtime.streams.Chunk) } })
    }
    override fun rawFromBytes(data: salvo.platform.core.bytes.Bytes): Long {
        return salvo.runtime.streams.registerBytes(data)
    }
}

fun chunkReceived(reply: salvo.SalvoReply, c: salvo.runtime.streams.Chunk) {
    if ((c.fault != null)) {
        val f: salvo.runtime.streams.Fault = c.fault!!
        salvo.SalvoSched.replyWire(reply, Union3.U3<salvo.platform.core.bytes.Bytes, salvo.stream.End, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.err(kind(c.source, f))), salvo.Union3Codec(salvo.BytesCodec, salvo.stream.__Codec_End, salvo.Union2Codec(salvo.stream.__Codec_InvalidUtf8, salvo.stream.__Codec_StreamFailed)))
    } else if (c.end) {
        salvo.SalvoSched.replyWire(reply, Union3.U2<salvo.platform.core.bytes.Bytes, salvo.stream.End, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.stream.End()), salvo.Union3Codec(salvo.BytesCodec, salvo.stream.__Codec_End, salvo.Union2Codec(salvo.stream.__Codec_InvalidUtf8, salvo.stream.__Codec_StreamFailed)))
    } else {
        salvo.SalvoSched.replyWire(reply, Union3.U1<salvo.platform.core.bytes.Bytes, salvo.stream.End, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>(salvo.core.result.ok(c.data)), salvo.Union3Codec(salvo.BytesCodec, salvo.stream.__Codec_End, salvo.Union2Codec(salvo.stream.__Codec_InvalidUtf8, salvo.stream.__Codec_StreamFailed)))
    }
}

fun nextLine(handle: Long): String? {
    val e: salvo.runtime.streams.InEntry = salvo.runtime.streams.checkoutIn(handle)
    val r: salvo.runtime.streams.Read = salvo.runtime.streams.readLine(e)
    var text: String? = null
    if ((!(r.end) && (r.fault == null))) {
        text = salvo.runtime.streams.decode(e, r.data)
    }
    salvo.runtime.streams.checkinIn(handle, e)
    return text
}

fun kind(source: String, f: salvo.runtime.streams.Fault): Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed> {
    if (f.utf8) {
        return Union2.U1<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>(salvo.stream.InvalidUtf8(source = source))
    }
    return Union2.U2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>(salvo.stream.StreamFailed(source = source, message = f.message))
}

class DefaultStreams(private val __dep0: RawStreams) : salvo.stream.Streams {
    override fun readLine(s: salvo.stream.InStream): String? {
        return __dep0.rawReadLine(s.handle)
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readAll(s: salvo.stream.InStream): Union2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val r: Union2<String, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = __dep0.rawReadAll(s.handle)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: String = ((r as Union2.U1<*, *>).value as String)
                return Union2.U1<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(r_1))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed> = ((r as Union2.U2<*, *>).value as Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>)
                return Union2.U2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(r_2)))
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readBytes(s: salvo.stream.InStream, max: Int): Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val r: Union2<salvo.platform.core.bytes.Bytes, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = __dep0.rawReadBytes(s.handle, max)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: salvo.platform.core.bytes.Bytes = ((r as Union2.U1<*, *>).value as salvo.platform.core.bytes.Bytes)
                return Union2.U1<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(r_1))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed> = ((r as Union2.U2<*, *>).value as Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>)
                return Union2.U2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(r_2)))
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo__InStream_Bytes_Int(s: salvo.stream.InStream, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val r: Union2<Int, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = __dep0.rawReadToBytes(s.handle, buf, max)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Int = ((r as Union2.U1<*, *>).value as Int)
                return Union2.U1<Int, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(r_1))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed> = ((r as Union2.U2<*, *>).value as Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>)
                return Union2.U2<Int, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(r_2)))
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo__InStream_Str(s: salvo.stream.InStream, buf: salvo.platform.core.string.MutStr): Union2<Long, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val r: Union2<Long, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = __dep0.rawReadToStr(s.handle, buf)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Long = ((r as Union2.U1<*, *>).value as Long)
                return Union2.U1<Long, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(r_1))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed> = ((r as Union2.U2<*, *>).value as Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>)
                return Union2.U2<Long, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(r_2)))
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
    override fun readLineTo(s: salvo.stream.InStream, buf: salvo.platform.core.string.MutStr): Boolean {
        return __dep0.rawReadLineToStr(s.handle, buf)
    }
    override fun position__InStream(s: salvo.stream.InStream): Long {
        return __dep0.rawReadPosition(s.handle)
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun close__InStream(s: salvo.stream.InStream): Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val r: Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = __dep0.rawCloseRead(s.handle)
        run { s; Unit }
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Unit = ((r as Union2.U1<*, *>).value as Unit)
                return Union2.U1<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(Unit))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed> = ((r as Union2.U2<*, *>).value as Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>)
                return Union2.U2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(r_2)))
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun receive(s: salvo.stream.InStream, reply: salvo.SalvoReply) {
        val handle: Long = s.handle
        run { s; Unit }
        __dep0.rawReceive(handle, run { val __c0 = reply; val __c1 = handle; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union3Codec(salvo.BytesCodec, salvo.stream.__Codec_End, salvo.Union2Codec(salvo.stream.__Codec_InvalidUtf8, salvo.stream.__Codec_StreamFailed))) }) { __v -> hostReceived(__c0, __c1, __v as Union3<salvo.platform.core.bytes.Bytes, salvo.stream.End, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>) } })
    }
    override fun fromBytes(data: salvo.platform.core.bytes.Bytes): salvo.stream.InStream {
        return salvo.stream.InStream(handle = __dep0.rawFromBytes(data))
    }
    override fun write(s: salvo.stream.OutStream, text: String): Long {
        return __dep0.rawWrite(s.handle, text)
    }
    override fun writeLine(s: salvo.stream.OutStream, text: String): Long {
        return __dep0.rawWrite(s.handle, "${text}\n")
    }
    override fun writeBytes(s: salvo.stream.OutStream, data: salvo.platform.core.bytes.Bytes): Long {
        return __dep0.rawWriteBytes(s.handle, data)
    }
    override fun position__OutStream(s: salvo.stream.OutStream): Long {
        return __dep0.rawWritePosition(s.handle)
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun flush(s: salvo.stream.OutStream): Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val r: Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = __dep0.rawFlush(s.handle)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Unit = ((r as Union2.U1<*, *>).value as Unit)
                return Union2.U1<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(Unit))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed> = ((r as Union2.U2<*, *>).value as Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>)
                return Union2.U2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(r_2)))
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun close__OutStream(s: salvo.stream.OutStream): Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val r: Union2<Unit, Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = __dep0.rawCloseWrite(s.handle)
        run { s; Unit }
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Unit = ((r as Union2.U1<*, *>).value as Unit)
                return Union2.U1<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(Unit))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed> = ((r as Union2.U2<*, *>).value as Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>)
                return Union2.U2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(r_2)))
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
}

