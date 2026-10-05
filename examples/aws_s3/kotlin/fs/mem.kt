package salvo.fs.mem

import salvo.*
import salvo.core.bytes.appendPlatform as appendPlatform__core_bytes
import salvo.core.bytes.getPlatform as getPlatform__core_bytes
import salvo.core.bytes.mutBytes
import salvo.core.bytes.sizePlatform
import salvo.core.bytes.slicePlatform
import salvo.core.bytes.strOfBytesPlatform
import salvo.core.bytes.toBytesPlatform
import salvo.core.checked.Checked
import salvo.core.checked.checked
import salvo.core.checked.ignore
import salvo.core.list.at
import salvo.core.list.sort
import salvo.core.map.containsKeyPlatform
import salvo.core.map.getPlatform as getPlatform__core_map
import salvo.core.map.putPlatform
import salvo.core.map.removePlatform
import salvo.core.result.err
import salvo.core.result.ok
import salvo.core.set.addPlatform
import salvo.core.set.toListPlatform
import salvo.core.sorted.max
import salvo.core.string.appendPlatform as appendPlatform__core_string
import salvo.core.string.byteSizePlatform
import salvo.core.string.indexOfPlatform
import salvo.core.string.startsWithPlatform
import salvo.core.string.substrPlatform
import salvo.core.string.trimPrefixPlatform
import salvo.core.string.trimSuffixPlatform
import salvo.fs.AlreadyExists
import salvo.fs.FileInfo
import salvo.fs.Fs
import salvo.fs.IoError
import salvo.fs.NotADirectory
import salvo.fs.NotFound
import salvo.fs.PathEscapes
import salvo.fs.PermissionDenied
import salvo.fs.Streaming
import salvo.fs.path.Path
import salvo.fs.path.path
import salvo.fs.path.toStr
import salvo.stream.End
import salvo.stream.InStream
import salvo.stream.InvalidUtf8
import salvo.stream.OutStream
import salvo.stream.Packet
import salvo.stream.StreamFailed
import salvo.stream.freshHandle

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
    private var files: salvo.platform.core.map.MutMap<String, salvo.platform.core.bytes.Bytes> = linkedMapOf<String, salvo.platform.core.bytes.Bytes>().also { __m -> __m.putAll(listOf()) }
    private var reads: salvo.platform.core.map.MutMap<Long, MemRead> = linkedMapOf<Long, MemRead>().also { __m -> __m.putAll(listOf()) }
    private var writes: salvo.platform.core.map.MutMap<Long, MemWrite> = linkedMapOf<Long, MemWrite>().also { __m -> __m.putAll(listOf()) }

    override fun openRead(path: Path): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val content = getPlatform__core_map(files, pathText)
        if (content == null) {
            return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = pathText)))))
        }
        val handle = freshHandle()
        putPlatform(reads, handle, MemRead(source = pathText, data = content, at = 0, failed = false))
        return Union2.U1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = handle)))
    }

    override fun openReadAt(path: Path, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val content = getPlatform__core_map(files, pathText)
        if (content == null) {
            return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = pathText)))))
        }
        var at = (offset).toInt()
        if (at < 0) {
            at = 0
        }
        val end = sizePlatform(content)
        if (at > end) {
            at = end
        }
        val handle = freshHandle()
        putPlatform(reads, handle, MemRead(source = pathText, data = content, at = at, failed = false))
        return Union2.U1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = handle)))
    }

    override fun openWrite(path: Path): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val handle = freshHandle()
        val empty = mutBytes(arrayOf())
        putPlatform(writes, handle, MemWrite(path = pathText, buffer = empty))
        return Union2.U1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = handle)))
    }

    override fun openAppend(path: Path): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val existing = getPlatform__core_map(files, pathText)
        val start = mutBytes(arrayOf())
        if (existing == null) {
        } else {
            appendPlatform__core_bytes(start, existing)
        }
        val handle = freshHandle()
        putPlatform(writes, handle, MemWrite(path = pathText, buffer = start))
        return Union2.U1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = handle)))
    }

    override fun exists(path: Path): Boolean {
        val pathText = toStr(path)
        if (containsKeyPlatform(files, pathText)) {
            return true
        }
        return fsHasChildren(files, pathText)
    }

    override fun metadata(path: Path): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val content = getPlatform__core_map(files, pathText)
        if (content == null) {
            if (fsHasChildren(files, pathText)) {
                return Union2.U1<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(FileInfo(size = 0L, isDir = true)))
            }
            return Union2.U2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = pathText)))))
        }
        return Union2.U1<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(FileInfo(size = (sizePlatform(content)).toLong(), isDir = false)))
    }

    override fun listDir(path: Path): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        if (containsKeyPlatform(files, pathText)) {
            return Union2.U2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U4<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotADirectory(path = pathText)))))
        }
        if (!fsHasChildren(files, pathText)) {
            return Union2.U2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = pathText)))))
        }
        val names: salvo.platform.core.set.MutSet<String> = linkedSetOf<String>().also { __s -> __s.addAll(listOf()) }
        val prefix = "$pathText/"
        for (key in salvo.platform.core.map.each(files)) {
            if (startsWithPlatform(key, prefix)) {
                val rest = trimPrefixPlatform(key, prefix)
                val cut = indexOfPlatform(rest, "/")
                var name = rest
                if (cut != null) {
                    val head = substrPlatform(rest, 0, cut)
                    if (head != null) {
                        name = head
                    }
                }
                addPlatform(names, name)
            }
        }
        val sorted: List<String> = sort(toListPlatform(names), { __i0, __i1 -> salvo.__salvoCompare(__i0, __i1) })
        return Union2.U1<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(sorted))
    }

    override fun createDirs(path: Path): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        return Union2.U1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
    }

    override fun delete(path: Path): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        if (containsKeyPlatform(files, pathText)) {
            removePlatform(files, pathText)
            return Union2.U1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
        }
        if (fsHasChildren(files, pathText)) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U6<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(IoError(path = pathText, message = "directory not empty")))))
        }
        return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = pathText)))))
    }

    override fun renamePath(from: Path, to: Path): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val fromText = toStr(from)
        val toText = toStr(to)
        val content = getPlatform__core_map(files, fromText)
        if (content == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = fromText)))))
        }
        val bytes: salvo.platform.core.bytes.Bytes = content
        removePlatform(files, fromText)
        putPlatform(files, toText, bytes)
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
    override fun readTo__InStream_Bytes_Int(s: InStream, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val got = memReadBytes(reads, s.handle, max)
        if (got is Union2.U2<*, *>) {
            return Union2.U2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val data: salvo.platform.core.bytes.Bytes = (got.value as salvo.platform.core.bytes.Bytes)
        appendPlatform__core_bytes(buf, data)
        return Union2.U1<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(sizePlatform(data)))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo__InStream_Str(s: InStream, buf: salvo.platform.core.string.MutStr): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val got = memReadAll(reads, s.handle)
        if (got is Union2.U2<*, *>) {
            return Union2.U2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val text: String = (got.value as String)
        appendPlatform__core_string(buf, text)
        return Union2.U1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(byteSizePlatform(text)))
    }

    override fun readLineTo(s: InStream, buf: salvo.platform.core.string.MutStr): Boolean {
        val line = memReadLine(reads, s.handle)
        when {
            line != null -> {
                appendPlatform__core_string(buf, line)
                return true
            }
            else -> {
                return false
            }
        }
    }

    override fun position__InStream(s: InStream): Long {
        return (memReadState(reads, s.handle).at).toLong()
    }

    override fun close__InStream(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val open = memReadState(reads, s.handle)
        val failed = open.failed
        val source = open.source
        removePlatform(reads, s.handle)
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
            removePlatform(reads, s.handle)
            (s).let {}
            reply.send(Union3.U3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U2<InvalidUtf8, StreamFailed>(StreamFailed(source = open.source, message = "read failed"))))))
            ignore((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
            return
        }
        val data: salvo.platform.core.bytes.Bytes = (got.value as salvo.platform.core.bytes.Bytes)
        if (sizePlatform(data) == 0) {
            removePlatform(reads, s.handle)
            (s).let {}
            reply.send(Union3.U2<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(End()))
            return
        }
        reply.send(Union3.U1<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Packet(bytes = data, stream = s))))
    }

    override fun fromBytes(data: salvo.platform.core.bytes.Bytes): InStream {
        val handle = freshHandle()
        putPlatform(reads, handle, MemRead(source = "<bytes>", data = data, at = 0, failed = false))
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

    override fun position__OutStream(s: OutStream): Long {
        return (sizePlatform(memWriteState(writes, s.handle).buffer)).toLong()
    }

    override fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val open = memWriteState(writes, s.handle)
        putPlatform(files, open.path, open.buffer)
        return Union2.U1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
    }

    override fun close__OutStream(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val open = memWriteState(writes, s.handle)
        putPlatform(files, open.path, open.buffer)
        removePlatform(writes, s.handle)
        (s).let {}
        return Union2.U1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
    }
}

fun fsHasChildren(files: Map<String, salvo.platform.core.bytes.Bytes>, path: String): Boolean {
    val prefix = "$path/"
    for (key in salvo.platform.core.map.each(files)) {
        if (startsWithPlatform(key, prefix)) {
            return true
        }
    }
    return false
}

fun memFindNewline(data: salvo.platform.core.bytes.Bytes, from: Int): Int {
    val end = sizePlatform(data)
    var i = from
    while (i < end) {
        if (((getPlatform__core_bytes(data, i) ?: throw AssertionError("salvo: value is absent at fs.mem:323:19"))).toInt() == 10) {
            return i
        }
        i = i + 1
    }
    return end
}

fun memAppend(writes: salvo.platform.core.map.MutMap<Long, MemWrite>, handle: Long, data: salvo.platform.core.bytes.Bytes): Long {
    val open = memWriteState(writes, handle)
    val grown = mutBytes(arrayOf(open.buffer))
    appendPlatform__core_bytes(grown, data)
    val buffer: salvo.platform.core.bytes.Bytes = grown
    putPlatform(writes, handle, MemWrite(path = open.path, buffer = buffer))
    return (sizePlatform(data)).toLong()
}

fun memReadState(reads: Map<Long, MemRead>, handle: Long): MemRead {
    val open = getPlatform__core_map(reads, handle)
    if (open == null) {
        throw AssertionError(("salvo: " + ("stream handle $handle was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at fs.mem:350:9"))
    }
    return MemRead(source = open.source, data = open.data, at = open.at, failed = open.failed)
}

fun memWriteState(writes: Map<Long, MemWrite>, handle: Long): MemWrite {
    val open = getPlatform__core_map(writes, handle)
    if (open == null) {
        throw AssertionError(("salvo: " + ("stream handle $handle was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at fs.mem:358:9"))
    }
    return MemWrite(path = open.path, buffer = open.buffer)
}

fun memReadLine(reads: salvo.platform.core.map.MutMap<Long, MemRead>, handle: Long): String? {
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
    val line = (slicePlatform(bytes, at, stop) ?: throw AssertionError("salvo: value is absent at fs.mem:382:16"))
    var nextAt = stop
    if (stop < end) {
        nextAt = stop + 1
    }
    val text = strOfBytesPlatform(line)
    if (text == null) {
        putPlatform(reads, handle, MemRead(source = open.source, data = bytes, at = nextAt, failed = true))
        return null
    }
    putPlatform(reads, handle, MemRead(source = open.source, data = bytes, at = nextAt, failed = false))
    return trimSuffixPlatform(text, "\r")
}

fun memReadAll(reads: salvo.platform.core.map.MutMap<Long, MemRead>, handle: Long): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val open = memReadState(reads, handle)
    val source: String = open.source
    if (open.failed) {
        return Union2.U2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
    }
    val bytes: salvo.platform.core.bytes.Bytes = open.data
    val end = sizePlatform(bytes)
    val rest = (slicePlatform(bytes, open.at, end) ?: throw AssertionError("salvo: value is absent at fs.mem:406:16"))
    val text = strOfBytesPlatform(rest)
    if (text == null) {
        putPlatform(reads, handle, MemRead(source = source, data = bytes, at = end, failed = true))
        return Union2.U2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(Union2.U1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
    }
    putPlatform(reads, handle, MemRead(source = source, data = bytes, at = end, failed = false))
    return Union2.U1<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(text))
}

fun memReadBytes(reads: salvo.platform.core.map.MutMap<Long, MemRead>, handle: Long, max: Int): Union2<salvo.platform.core.bytes.Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
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
    val taken = (slicePlatform(bytes, open.at, stop) ?: throw AssertionError("salvo: value is absent at fs.mem:432:17"))
    putPlatform(reads, handle, MemRead(source = source, data = bytes, at = stop, failed = false))
    return Union2.U1<salvo.platform.core.bytes.Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(taken))
}
