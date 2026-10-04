package salvo.fs.mem

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
import salvo.fs.*
import salvo.stream.*

data class MemRead(
    val source: String,
    val data: salvo.platform.core.bytes.Bytes,
    val at: Int,
    val failed: Boolean,
)

object __Codec_MemRead : salvo.WireCodec<MemRead> {
    override fun enc(v: MemRead, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.source, out)
        salvo.BytesCodec.enc(v.data, out)
        salvo.IntCodec.enc(v.at, out)
        salvo.BoolCodec.enc(v.failed, out)
    }
    override fun dec(inp: salvo.WireIn): MemRead = MemRead(salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.BoolCodec.dec(inp))
}

data class MemWrite(
    val path: String,
    val buffer: salvo.platform.core.bytes.Bytes,
)

object __Codec_MemWrite : salvo.WireCodec<MemWrite> {
    override fun enc(v: MemWrite, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
        salvo.BytesCodec.enc(v.buffer, out)
    }
    override fun dec(inp: salvo.WireIn): MemWrite = MemWrite(salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

class MemFs : Fs, salvo.stream.Streams {
    private var files: MutableMap<String, salvo.platform.core.bytes.Bytes> = linkedMapOf<String, salvo.platform.core.bytes.Bytes>().also { __m -> __m.putAll(listOf()) }
    private var reads: MutableMap<Long, MemRead> = linkedMapOf<Long, MemRead>().also { __m -> __m.putAll(listOf()) }
    private var writes: MutableMap<Long, MemWrite> = linkedMapOf<Long, MemWrite>().also { __m -> __m.putAll(listOf()) }

    override fun openRead(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val content = files[path]
        if (content == null) {
            return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
        }
        val handle = freshHandle__2()
        reads.put(handle, MemRead(source = path, data = content, at = 0, failed = false))
        return Union2.U1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = handle)))
    }

    override fun openReadAt(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val content = files[path]
        if (content == null) {
            return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
        }
        var at = (offset).toInt()
        if (at < 0) {
            at = 0
        }
        val end = sizePlatform(content)
        if (at > end) {
            at = end
        }
        val handle = freshHandle__2()
        reads.put(handle, MemRead(source = path, data = content, at = at, failed = false))
        return Union2.U1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = handle)))
    }

    override fun openWrite(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val handle = freshHandle__2()
        val empty = mutBytes(arrayOf())
        writes.put(handle, MemWrite(path = path, buffer = empty))
        return Union2.U1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = handle)))
    }

    override fun openAppend(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val existing = files[path]
        val start = mutBytes(arrayOf())
        if (existing == null) {
        } else {
            appendPlatform(start, existing)
        }
        val handle = freshHandle__2()
        writes.put(handle, MemWrite(path = path, buffer = start))
        return Union2.U1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = handle)))
    }

    override fun exists(path: String): Boolean {
        if (files.containsKey(path)) {
            return true
        }
        return fsHasChildren(files, path)
    }

    override fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val content = files[path]
        if (content == null) {
            if (fsHasChildren(files, path)) {
                return Union2.U1<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(FileInfo(size = 0L, isDir = true)))
            }
            return Union2.U2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
        }
        return Union2.U1<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(FileInfo(size = (sizePlatform(content)).toLong(), isDir = false)))
    }

    override fun listDir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        if (files.containsKey(path)) {
            return Union2.U2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U4<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotADirectory(path = path)))))
        }
        if (!fsHasChildren(files, path)) {
            return Union2.U2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
        }
        val names: MutableSet<String> = linkedSetOf<String>().also { __s -> __s.addAll(listOf()) }
        val prefix = "$path/"
        for (key in files.keys) {
            if (key.startsWith(prefix)) {
                val rest = key.removePrefix(prefix)
                val cut = rest.indexOf("/").takeIf { it >= 0 }
                var name = rest
                if (cut != null) {
                    val head = run { val __s = rest; val __i = 0; val __j = cut; if (__i >= 0 && __j >= __i && __j <= __s.length) __s.substring(__i, __j) else null }
                    if (head != null) {
                        name = head
                    }
                }
                names.add(name)
            }
        }
        val sorted: List<String> = sort(names.toMutableList(), { __i0, __i1 -> salvo.__salvoCompare(__i0, __i1) })
        return Union2.U1<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(sorted))
    }

    override fun createDirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        return Union2.U1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
    }

    override fun delete(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        if (files.containsKey(path)) {
            files.remove(path)
            return Union2.U1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
        }
        if (fsHasChildren(files, path)) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U6<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(IoError(path = path, message = "directory not empty")))))
        }
        return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
    }

    override fun renamePath(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val content = files[from]
        if (content == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = from)))))
        }
        val bytes: salvo.platform.core.bytes.Bytes = content
        files.remove(from)
        files.put(to, bytes)
        return Union2.U1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
    }

    override fun readLine(s: InStream): String? {
        return memReadLine(reads, s.handle)
    }

    override fun readAll(s: InStream): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        return memReadAll(reads, s.handle)
    }

    override fun readBytes(s: InStream, max: Int): Union2<salvo.platform.core.bytes.Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        return memReadBytes(reads, s.handle, max)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo(s: InStream, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val got = memReadBytes(reads, s.handle, max)
        if (got is Union2.U2<*, *>) {
            return Union2.U2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val data: salvo.platform.core.bytes.Bytes = (got.value as salvo.platform.core.bytes.Bytes)
        appendPlatform(buf, data)
        return Union2.U1<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(sizePlatform(data)))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val got = memReadAll(reads, s.handle)
        if (got is Union2.U2<*, *>) {
            return Union2.U2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val text: String = (got.value as String)
        buf.append(text)
        return Union2.U1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(text.toByteArray(Charsets.UTF_8).size.toLong()))
    }

    override fun readLineTo(s: InStream, buf: StringBuilder): Boolean {
        val line = memReadLine(reads, s.handle)
        when {
            line != null -> {
                buf.append(line)
                return true
            }
            else -> {
                return false
            }
        }
    }

    override fun position(s: InStream): Long {
        return (memReadState(reads, s.handle).at).toLong()
    }

    override fun close(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val open = memReadState(reads, s.handle)
        val failed = open.failed
        val source = open.source
        reads.remove(s.handle)
        (s).let {}
        if (failed) {
            return Union2.U2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
        }
        return Union2.U1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun receive(s: InStream, reply: salvo.SalvoReply) {
        val got = memReadBytes(reads, s.handle, 65536)
        if (got is Union2.U2<*, *>) {
            val open = memReadState(reads, s.handle)
            reads.remove(s.handle)
            (s).let {}
            reply.send(Union3.U3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U2<InvalidUtf8, StreamFailed>(StreamFailed(source = open.source, message = "read failed"))))))
            ignore((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
            return
        }
        val data: salvo.platform.core.bytes.Bytes = (got.value as salvo.platform.core.bytes.Bytes)
        if (sizePlatform(data) == 0) {
            reads.remove(s.handle)
            (s).let {}
            reply.send(Union3.U2<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(End()))
            return
        }
        reply.send(Union3.U1<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Packet(bytes = data, stream = s))))
    }

    override fun fromBytes(data: salvo.platform.core.bytes.Bytes): InStream {
        val handle = freshHandle__2()
        reads.put(handle, MemRead(source = "<bytes>", data = data, at = 0, failed = false))
        return InStream(handle = handle)
    }

    override fun write(s: OutStream, text: String): Long {
        return memAppend(writes, s.handle, toBytesPlatform(text))
    }

    override fun writeLine(s: OutStream, text: String): Long {
        return memAppend(writes, s.handle, toBytesPlatform("$text\n"))
    }

    override fun writeBytes(s: OutStream, data: salvo.platform.core.bytes.Bytes): Long {
        return memAppend(writes, s.handle, data)
    }

    override fun position__2(s: OutStream): Long {
        return (sizePlatform(memWriteState(writes, s.handle).buffer)).toLong()
    }

    override fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val open = memWriteState(writes, s.handle)
        files.put(open.path, open.buffer)
        return Union2.U1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
    }

    override fun close__2(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val open = memWriteState(writes, s.handle)
        files.put(open.path, open.buffer)
        writes.remove(s.handle)
        (s).let {}
        return Union2.U1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
    }
}

fun fsHasChildren(files: Map<String, salvo.platform.core.bytes.Bytes>, path: String): Boolean {
    val prefix = "$path/"
    for (key in files.keys) {
        if (key.startsWith(prefix)) {
            return true
        }
    }
    return false
}

fun memFindNewline(data: salvo.platform.core.bytes.Bytes, from: Int): Int {
    val end = sizePlatform(data)
    var i = from
    while (i < end) {
        if (((getPlatform(data, i) ?: throw AssertionError("salvo: value is absent at fs.mem:311:19"))).toInt() == 10) {
            return i
        }
        i = i + 1
    }
    return end
}

fun memAppend(writes: MutableMap<Long, MemWrite>, handle: Long, data: salvo.platform.core.bytes.Bytes): Long {
    val open = memWriteState(writes, handle)
    val grown = mutBytes(arrayOf(open.buffer))
    appendPlatform(grown, data)
    val buffer: salvo.platform.core.bytes.Bytes = grown
    writes.put(handle, MemWrite(path = open.path, buffer = buffer))
    return (sizePlatform(data)).toLong()
}

fun memReadState(reads: Map<Long, MemRead>, handle: Long): MemRead {
    val open = reads[handle]
    if (open == null) {
        throw AssertionError(("salvo: " + ("stream handle $handle was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at fs.mem:338:9"))
    }
    return MemRead(source = open.source, data = open.data, at = open.at, failed = open.failed)
}

fun memWriteState(writes: Map<Long, MemWrite>, handle: Long): MemWrite {
    val open = writes[handle]
    if (open == null) {
        throw AssertionError(("salvo: " + ("stream handle $handle was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at fs.mem:346:9"))
    }
    return MemWrite(path = open.path, buffer = open.buffer)
}

fun memReadLine(reads: MutableMap<Long, MemRead>, handle: Long): String? {
    val open = memReadState(reads, handle)
    if (open.failed) {
        return null
    }
    val at = open.at
    val bytes: salvo.platform.core.bytes.Bytes = open.data
    val end = sizePlatform(bytes)
    if (at >= end) {
        return null
    }
    val stop = memFindNewline(bytes, at)
    val line = (slicePlatform(bytes, at, stop) ?: throw AssertionError("salvo: value is absent at fs.mem:370:16"))
    var nextAt = stop
    if (stop < end) {
        nextAt = stop + 1
    }
    val text = strOfBytesPlatform(line)
    if (text == null) {
        reads.put(handle, MemRead(source = open.source, data = bytes, at = nextAt, failed = true))
        return null
    }
    reads.put(handle, MemRead(source = open.source, data = bytes, at = nextAt, failed = false))
    return text.removeSuffix("\r")
}

fun memReadAll(reads: MutableMap<Long, MemRead>, handle: Long): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val open = memReadState(reads, handle)
    val source: String = open.source
    if (open.failed) {
        return Union2.U2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
    }
    val bytes: salvo.platform.core.bytes.Bytes = open.data
    val end = sizePlatform(bytes)
    val rest = (slicePlatform(bytes, open.at, end) ?: throw AssertionError("salvo: value is absent at fs.mem:394:16"))
    val text = strOfBytesPlatform(rest)
    if (text == null) {
        reads.put(handle, MemRead(source = source, data = bytes, at = end, failed = true))
        return Union2.U2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
    }
    reads.put(handle, MemRead(source = source, data = bytes, at = end, failed = false))
    return Union2.U1<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(text))
}

fun memReadBytes(reads: MutableMap<Long, MemRead>, handle: Long, max: Int): Union2<salvo.platform.core.bytes.Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val open = memReadState(reads, handle)
    val source: String = open.source
    if (open.failed) {
        return Union2.U2<salvo.platform.core.bytes.Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
    }
    val bytes: salvo.platform.core.bytes.Bytes = open.data
    var stop = open.at + max
    if (max < 0) {
        stop = open.at
    }
    val end = sizePlatform(bytes)
    if (stop > end) {
        stop = end
    }
    val taken = (slicePlatform(bytes, open.at, stop) ?: throw AssertionError("salvo: value is absent at fs.mem:420:17"))
    reads.put(handle, MemRead(source = source, data = bytes, at = stop, failed = false))
    return Union2.U1<salvo.platform.core.bytes.Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(taken))
}
