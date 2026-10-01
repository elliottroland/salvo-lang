// Host implementation of the platform declarations of Salvo module `stream.host`.
//
// Generated once by `salvo platform generate`, then written by hand: the
// process's stream table lives in the runtime (`salvo.SalvoStreams`,
// [stream-table]), and this class reads and writes whatever is registered
// there — a file `HostRawFs` opened, a network body, a buffer.
package salvo.platform.stream.host

import salvo.*
import salvo.stream.*
import salvo.stream.host.*

import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.nio.charset.StandardCharsets

/** `stream.StreamError`, as the generated union. */
private typealias Kind = Union2<InvalidUtf8, StreamFailed>

private fun kind(source: String, fault: SalvoFault): Kind = when (fault) {
    is SalvoFault.Utf8 -> U2_1(InvalidUtf8(source))
    is SalvoFault.Failed -> U2_2(StreamFailed(source, fault.message))
}

/** Strict UTF-8, recording `InvalidUtf8` against the stream; null on failure. */
private fun decode(stream: SalvoIn, bytes: ByteArray): String? =
    try {
        StandardCharsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
            .decode(ByteBuffer.wrap(bytes))
            .toString()
    } catch (e: java.nio.charset.CharacterCodingException) {
        stream.utf8Failed()
        null
    }

// `threadsafe platform handler HostRawStreams` — THE CONTRACT: every member
// below is safe to run concurrently with every other. The state is the
// runtime's table, which locks per stream (each entry is its own monitor).
class HostRawStreams : RawStreams {
    override fun rawReadLine(handle: Long): String? {
        val stream = SalvoStreams.inStream(handle)
        synchronized(stream) {
            val bytes = try {
                stream.readLineBytes()
            } catch (e: SalvoFaultException) {
                return null
            } ?: return null
            return decode(stream, bytes)
        }
    }

    override fun rawReadAll(handle: Long): Union2<String, Kind> {
        val stream = SalvoStreams.inStream(handle)
        synchronized(stream) {
            val bytes = try {
                stream.readAllBytes()
            } catch (e: SalvoFaultException) {
                return U2_2(kind(stream.source, e.fault))
            }
            val text = decode(stream, bytes) ?: return U2_2(kind(stream.source, SalvoFault.Utf8))
            return U2_1(text)
        }
    }

    override fun rawReadBytes(handle: Long, max: Int): Union2<SalvoBytes, Kind> {
        val out = SalvoBytes()
        return when (val filled = rawReadToBytes(handle, out, max)) {
            is U2_2 -> U2_2(filled.value)
            else -> U2_1(out)
        }
    }

    override fun rawReadToBytes(handle: Long, buf: SalvoBytes, max: Int): Union2<Int, Kind> {
        val stream = SalvoStreams.inStream(handle)
        synchronized(stream) {
            val got = try {
                stream.readUpTo(max)
            } catch (e: SalvoFaultException) {
                return U2_2(kind(stream.source, e.fault))
            }
            buf.appendArray(got, got.size)
            return U2_1(got.size)
        }
    }

    override fun rawReadToStr(handle: Long, buf: StringBuilder): Union2<Long, Kind> {
        return when (val all = rawReadAll(handle)) {
            is U2_2 -> U2_2(all.value)
            is U2_1 -> {
                val text = all.value
                buf.append(text)
                U2_1(text.toByteArray(StandardCharsets.UTF_8).size.toLong())
            }
        }
    }

    override fun rawReadLineToStr(handle: Long, buf: StringBuilder): Boolean {
        val line = rawReadLine(handle) ?: return false
        buf.append(line)
        return true
    }

    override fun rawReadPosition(handle: Long): Long {
        val stream = SalvoStreams.inStream(handle)
        synchronized(stream) { return stream.position }
    }

    override fun rawCloseRead(handle: Long): Union2<Unit, Kind> {
        val stream = SalvoStreams.takeIn(handle)
        synchronized(stream) {
            stream.closeInput()
            val failed = stream.failed ?: return U2_1(Unit)
            return U2_2(kind(stream.source, failed))
        }
    }

    override fun rawWrite(handle: Long, text: String): Long {
        val stream = SalvoStreams.outStream(handle)
        synchronized(stream) { return stream.writeData(text.toByteArray(StandardCharsets.UTF_8)) }
    }

    override fun rawWriteBytes(handle: Long, data: SalvoBytes): Long {
        val stream = SalvoStreams.outStream(handle)
        synchronized(stream) { return stream.writeData(data.toByteArray()) }
    }

    override fun rawWritePosition(handle: Long): Long {
        val stream = SalvoStreams.outStream(handle)
        synchronized(stream) { return stream.position }
    }

    override fun rawFlush(handle: Long): Union2<Unit, Kind> {
        val stream = SalvoStreams.outStream(handle)
        synchronized(stream) {
            val failed = stream.flushData() ?: return U2_1(Unit)
            return U2_2(kind(stream.source, failed))
        }
    }

    /**
     * [stream-receive] Reads on a thread of its own and completes [reply] from
     * there [platform-reply]: the caller's worker never waits. At the end, or
     * on a failure, the stream is released before answering, so the caller has
     * nothing left to close.
     */
    override fun rawReceive(handle: Long, reply: SalvoReply) {
        val host = reply.hosted()
        val stream = SalvoStreams.inStream(handle)
        val reader = Thread {
            val answer: Union3<SalvoBytes, End, Kind> = synchronized(stream) {
                try {
                    val got = stream.readUpTo(65536)
                    if (got.isNotEmpty()) {
                        U3_1(SalvoBytes(got))
                    } else {
                        // Released here, as the contract says: out of the
                        // table *and* closed — a JVM stream is not closed by
                        // being dropped, as a Rust one is.
                        SalvoStreams.takeIn(handle).closeInput()
                        U3_2(End())
                    }
                } catch (e: SalvoFaultException) {
                    SalvoStreams.takeIn(handle).closeInput()
                    U3_3(kind(stream.source, e.fault))
                }
            }
            host.send(answer)
        }
        reader.isDaemon = true
        reader.start()
    }

    /** [stream-from-bytes] A readable stream over [data], registered in the process's table. */
    override fun rawFromBytes(data: SalvoBytes): Long =
        SalvoStreams.registerIn("<bytes>", java.io.ByteArrayInputStream(data.toByteArray()), 0)

    override fun rawCloseWrite(handle: Long): Union2<Unit, Kind> {
        val stream = SalvoStreams.takeOut(handle)
        synchronized(stream) {
            val failed = stream.closeOutput() ?: return U2_1(Unit)
            return U2_2(kind(stream.source, failed))
        }
    }
}
