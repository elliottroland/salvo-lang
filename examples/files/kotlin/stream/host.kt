package salvo.stream.host

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.result.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.stream.*

interface RawStreams {
    fun raw_read_line(handle: Long): String?
    fun raw_read_all(handle: Long): Union2<String, Union2<InvalidUtf8, StreamFailed>>
    fun raw_read_bytes(handle: Long, max: Int): Union2<salvo.SalvoBytes, Union2<InvalidUtf8, StreamFailed>>
    fun raw_read_to_bytes(handle: Long, buf: salvo.SalvoBytes, max: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>>
    fun raw_read_to_str(handle: Long, buf: StringBuilder): Union2<Long, Union2<InvalidUtf8, StreamFailed>>
    fun raw_read_line_to_str(handle: Long, buf: StringBuilder): Boolean
    fun raw_read_position(handle: Long): Long
    fun raw_close_read(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun raw_write(handle: Long, text: String): Long
    fun raw_write_bytes(handle: Long, data: salvo.SalvoBytes): Long
    fun raw_write_position(handle: Long): Long
    fun raw_flush(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
    fun raw_close_write(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>>
}

class __Mon_RawStreams(private val inner: RawStreams) : RawStreams {
    override fun raw_read_line(handle: Long): String? =
        synchronized(inner) { inner.raw_read_line(handle) }
    override fun raw_read_all(handle: Long): Union2<String, Union2<InvalidUtf8, StreamFailed>> =
        synchronized(inner) { inner.raw_read_all(handle) }
    override fun raw_read_bytes(handle: Long, max: Int): Union2<salvo.SalvoBytes, Union2<InvalidUtf8, StreamFailed>> =
        synchronized(inner) { inner.raw_read_bytes(handle, max) }
    override fun raw_read_to_bytes(handle: Long, buf: salvo.SalvoBytes, max: Int): Union2<Int, Union2<InvalidUtf8, StreamFailed>> =
        synchronized(inner) { inner.raw_read_to_bytes(handle, buf, max) }
    override fun raw_read_to_str(handle: Long, buf: StringBuilder): Union2<Long, Union2<InvalidUtf8, StreamFailed>> =
        synchronized(inner) { inner.raw_read_to_str(handle, buf) }
    override fun raw_read_line_to_str(handle: Long, buf: StringBuilder): Boolean =
        synchronized(inner) { inner.raw_read_line_to_str(handle, buf) }
    override fun raw_read_position(handle: Long): Long =
        synchronized(inner) { inner.raw_read_position(handle) }
    override fun raw_close_read(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> =
        synchronized(inner) { inner.raw_close_read(handle) }
    override fun raw_write(handle: Long, text: String): Long =
        synchronized(inner) { inner.raw_write(handle, text) }
    override fun raw_write_bytes(handle: Long, data: salvo.SalvoBytes): Long =
        synchronized(inner) { inner.raw_write_bytes(handle, data) }
    override fun raw_write_position(handle: Long): Long =
        synchronized(inner) { inner.raw_write_position(handle) }
    override fun raw_flush(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> =
        synchronized(inner) { inner.raw_flush(handle) }
    override fun raw_close_write(handle: Long): Union2<Unit, Union2<InvalidUtf8, StreamFailed>> =
        synchronized(inner) { inner.raw_close_write(handle) }
}

class DefaultStreams(private val __dep_RawStreams: RawStreams) : Streams {

    override fun read_line(s: InStream): String? {
        return __dep_RawStreams.raw_read_line(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_all(s: InStream): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.raw_read_all(s.handle)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok((r.value as String)))
            }
            is U2_2<*, *> -> {
                return U2_2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.raw_read_bytes(s.handle, max)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok((r.value as salvo.SalvoBytes)))
            }
            is U2_2<*, *> -> {
                return U2_2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.raw_read_to_bytes(s.handle, buf, max)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok((r.value as Int)))
            }
            is U2_2<*, *> -> {
                return U2_2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.raw_read_to_str(s.handle, buf)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok((r.value as Long)))
            }
            is U2_2<*, *> -> {
                return U2_2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    override fun read_line_to(s: InStream, buf: StringBuilder): Boolean {
        return __dep_RawStreams.raw_read_line_to_str(s.handle, buf)
    }

    override fun position(s: InStream): Long {
        return __dep_RawStreams.raw_read_position(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun close(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.raw_close_read(s.handle)
        (s).let {}
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    override fun write(s: OutStream, text: String): Long {
        return __dep_RawStreams.raw_write(s.handle, text)
    }

    override fun write_line(s: OutStream, text: String): Long {
        return __dep_RawStreams.raw_write(s.handle, "$text\n")
    }

    override fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long {
        return __dep_RawStreams.raw_write_bytes(s.handle, data)
    }

    override fun position__2(s: OutStream): Long {
        return __dep_RawStreams.raw_write_position(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.raw_flush(s.handle)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun close__2(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val r = __dep_RawStreams.raw_close_write(s.handle)
        (s).let {}
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>((r.value as Union2<InvalidUtf8, StreamFailed>))))
            }
        }
    }
}
