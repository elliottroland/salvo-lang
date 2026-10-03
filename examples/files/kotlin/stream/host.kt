package salvo.stream.host

import salvo.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.result.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.stream.*

interface RawStreams {
    fun rawReadLine(handle: Long): String?
    fun rawReadAll(handle: Long): Union2<String, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadBytes(handle: Long, max: Int): Union2<salvo.SalvoBytes, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadToBytes(handle: Long, buf: salvo.SalvoBytes, max: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadToStr(handle: Long, buf: StringBuilder): Union2<Long, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadLineToStr(handle: Long, buf: StringBuilder): Boolean
    fun rawReadPosition(handle: Long): Long
    fun rawCloseRead(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun rawWrite(handle: Long, text: String): Long
    fun rawWriteBytes(handle: Long, data: salvo.SalvoBytes): Long
    fun rawWritePosition(handle: Long): Long
    fun rawFlush(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun rawCloseWrite(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun rawReceive(handle: Long, reply: salvo.SalvoReply)
    fun rawFromBytes(data: salvo.SalvoBytes): Long
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
    override fun rawReadBytes(handle: Long, max: Int): Union2<salvo.SalvoBytes, Union2<InvalidUtf8, StreamFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawReadBytes(handle, max) } finally { lock.unlock() }
    }
    override fun rawReadToBytes(handle: Long, buf: salvo.SalvoBytes, max: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>> {
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
    override fun rawWriteBytes(handle: Long, data: salvo.SalvoBytes): Long {
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
    override fun rawFromBytes(data: salvo.SalvoBytes): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawFromBytes(data) } finally { lock.unlock() }
    }
}

// The interface a `platform handler` of `RawStreams` implements [platform-abi].
interface RawStreamsPlatform {
    fun rawReadLine(handle: Long): String?
    fun rawReadAll(handle: Long): Union2<String, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadBytes(handle: Long, max: Int): Union2<salvo.SalvoBytes, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadToBytes(handle: Long, buf: salvo.SalvoBytes, max: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadToStr(handle: Long, buf: StringBuilder): Union2<Long, Union2<InvalidUtf8, StreamFailed>>
    fun rawReadLineToStr(handle: Long, buf: StringBuilder): Boolean
    fun rawReadPosition(handle: Long): Long
    fun rawCloseRead(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun rawWrite(handle: Long, text: String): Long
    fun rawWriteBytes(handle: Long, data: salvo.SalvoBytes): Long
    fun rawWritePosition(handle: Long): Long
    fun rawFlush(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun rawCloseWrite(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun rawReceive(handle: Long, reply: salvo.SalvoReply)
    fun rawFromBytes(data: salvo.SalvoBytes): Long
}

open class __Platform_RawStreams(private val impl: RawStreamsPlatform) : RawStreams {
    override fun rawReadLine(handle: Long): String? = impl.rawReadLine(handle)
    override fun rawReadAll(handle: Long): Union2<String, Union2<InvalidUtf8, StreamFailed>> = impl.rawReadAll(handle)
    override fun rawReadBytes(handle: Long, max: Int): Union2<salvo.SalvoBytes, Union2<InvalidUtf8, StreamFailed>> = impl.rawReadBytes(handle, max)
    override fun rawReadToBytes(handle: Long, buf: salvo.SalvoBytes, max: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>> = impl.rawReadToBytes(handle, buf, max)
    override fun rawReadToStr(handle: Long, buf: StringBuilder): Union2<Long, Union2<InvalidUtf8, StreamFailed>> = impl.rawReadToStr(handle, buf)
    override fun rawReadLineToStr(handle: Long, buf: StringBuilder): Boolean = impl.rawReadLineToStr(handle, buf)
    override fun rawReadPosition(handle: Long): Long = impl.rawReadPosition(handle)
    override fun rawCloseRead(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> = impl.rawCloseRead(handle)
    override fun rawWrite(handle: Long, text: String): Long = impl.rawWrite(handle, text)
    override fun rawWriteBytes(handle: Long, data: salvo.SalvoBytes): Long = impl.rawWriteBytes(handle, data)
    override fun rawWritePosition(handle: Long): Long = impl.rawWritePosition(handle)
    override fun rawFlush(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> = impl.rawFlush(handle)
    override fun rawCloseWrite(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> = impl.rawCloseWrite(handle)
    override fun rawReceive(handle: Long, reply: salvo.SalvoReply) = impl.rawReceive(handle, reply)
    override fun rawFromBytes(data: salvo.SalvoBytes): Long = impl.rawFromBytes(data)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawReadAll {
    fun ok(value: String): Union2<String, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U1(value)
    fun err(value: Union2<InvalidUtf8, StreamFailed>): Union2<String, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawReadBytes {
    fun ok(value: salvo.SalvoBytes): Union2<salvo.SalvoBytes, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U1(value)
    fun err(value: Union2<InvalidUtf8, StreamFailed>): Union2<salvo.SalvoBytes, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawReadToBytes {
    fun ok(value: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U1(value)
    fun err(value: Union2<InvalidUtf8, StreamFailed>): Union2<Int, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawReadToStr {
    fun ok(value: Long): Union2<Long, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U1(value)
    fun err(value: Union2<InvalidUtf8, StreamFailed>): Union2<Long, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawCloseRead {
    fun ok(value: Unit): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U1(value)
    fun err(value: Union2<InvalidUtf8, StreamFailed>): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawFlush {
    fun ok(value: Unit): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U1(value)
    fun err(value: Union2<InvalidUtf8, StreamFailed>): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawCloseWrite {
    fun ok(value: Unit): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U1(value)
    fun err(value: Union2<InvalidUtf8, StreamFailed>): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawReceive {
    fun ok(value: salvo.SalvoBytes): Union3<salvo.SalvoBytes, End, Union2<InvalidUtf8, StreamFailed>> = salvo.Union3.U1(value)
    fun end(value: End): Union3<salvo.SalvoBytes, End, Union2<InvalidUtf8, StreamFailed>> = salvo.Union3.U2(value)
    fun err(value: Union2<InvalidUtf8, StreamFailed>): Union3<salvo.SalvoBytes, End, Union2<InvalidUtf8, StreamFailed>> = salvo.Union3.U3(value)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun hostReceived(reply: salvo.SalvoReply, handle: Long, got: Union3<salvo.SalvoBytes, End, Union2<InvalidUtf8, StreamFailed>>) {
    when (got) {
        is Union3.U1<*, *, *> -> {
            reply.send(Union3.U1<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Packet(bytes = (got.value as salvo.SalvoBytes), stream = InStream(handle = handle)))))
        }
        is Union3.U2<*, *, *> -> {
            reply.send(Union3.U2<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as End)))
        }
        is Union3.U3<*, *, *> -> {
            reply.send(Union3.U3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((got.value as Union2<InvalidUtf8, StreamFailed>)))))
        }
    }
}

class __Platform_HostRawStreams() : salvo.stream.host.__Platform_RawStreams(salvo.platform.stream.host.HostRawStreams())

class DefaultStreams(private val __dep_RawStreams: RawStreams) : Streams {

    override fun readLine(s: InStream): String? {
        return __dep_RawStreams.rawReadLine(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
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

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun readBytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.rawReadBytes(s.handle, max)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok((r.value as salvo.SalvoBytes)))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun readTo(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>> {
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

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
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

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
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

    override fun receive(s: InStream, reply: salvo.SalvoReply) {
        val handle = s.handle
        (s).let {}
        __dep_RawStreams.rawReceive(handle, run { val __c0 = reply; val __c1 = handle; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union3Codec(salvo.BytesCodec, __Codec_End, salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed))) }) { __v -> hostReceived(__c0, __c1, __v as Union3<salvo.SalvoBytes, End, Union2<InvalidUtf8, StreamFailed>>) } })
    }

    override fun fromBytes(data: salvo.SalvoBytes): InStream {
        return InStream(handle = __dep_RawStreams.rawFromBytes(data))
    }

    override fun write(s: OutStream, text: String): Long {
        return __dep_RawStreams.rawWrite(s.handle, text)
    }

    override fun writeLine(s: OutStream, text: String): Long {
        return __dep_RawStreams.rawWrite(s.handle, "$text\n")
    }

    override fun writeBytes(s: OutStream, data: salvo.SalvoBytes): Long {
        return __dep_RawStreams.rawWriteBytes(s.handle, data)
    }

    override fun position__2(s: OutStream): Long {
        return __dep_RawStreams.rawWritePosition(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
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

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
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
