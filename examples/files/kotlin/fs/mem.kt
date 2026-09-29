package salvo.fs.mem

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
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
    val data: salvo.SalvoBytes,
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
    val buffer: salvo.SalvoBytes,
)

object __Codec_MemWrite : salvo.WireCodec<MemWrite> {
    override fun enc(v: MemWrite, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
        salvo.BytesCodec.enc(v.buffer, out)
    }
    override fun dec(inp: salvo.WireIn): MemWrite = MemWrite(salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

class MemFs : Fs, Streams {
    private var files: MutableMap<String, salvo.SalvoBytes> = linkedMapOf<String, salvo.SalvoBytes>().also { __m -> __m.putAll(listOf()) }
    private var reads: MutableMap<Long, MemRead> = linkedMapOf<Long, MemRead>().also { __m -> __m.putAll(listOf()) }
    private var writes: MutableMap<Long, MemWrite> = linkedMapOf<Long, MemWrite>().also { __m -> __m.putAll(listOf()) }

    override fun open_read(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val content = files[path]
        if (content == null) {
            return U2_2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(U7_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
        }
        val handle = salvo.SalvoSched.freshHandle()
        reads.put(handle, MemRead(source = path, data = salvo.SalvoBytes(content), at = 0, failed = false))
        return U2_1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = handle)))
    }

    override fun open_read_at(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val content = files[path]
        if (content == null) {
            return U2_2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(U7_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
        }
        var at = (offset).toInt()
        if (at < 0) {
            at = 0
        }
        val end = content.size
        if (at > end) {
            at = end
        }
        val handle = salvo.SalvoSched.freshHandle()
        reads.put(handle, MemRead(source = path, data = salvo.SalvoBytes(content), at = at, failed = false))
        return U2_1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = handle)))
    }

    override fun open_write(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val handle = salvo.SalvoSched.freshHandle()
        val empty = salvo.SalvoBytes.joined()
        writes.put(handle, MemWrite(path = path, buffer = empty))
        return U2_1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = handle)))
    }

    override fun open_append(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val existing = files[path]
        val start = salvo.SalvoBytes.joined()
        if (existing == null) {
        } else {
            start.append(existing)
        }
        val handle = salvo.SalvoSched.freshHandle()
        writes.put(handle, MemWrite(path = path, buffer = start))
        return U2_1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = handle)))
    }

    override fun exists(path: String): Boolean {
        if (files.containsKey(path)) {
            return true
        }
        return fs_has_children(files, path)
    }

    override fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val content = files[path]
        if (content == null) {
            if (fs_has_children(files, path)) {
                return U2_1<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(FileInfo(size = 0L, is_dir = true)))
            }
            return U2_2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(U7_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
        }
        return U2_1<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(FileInfo(size = (content.size).toLong(), is_dir = false)))
    }

    override fun list_dir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        if (files.containsKey(path)) {
            return U2_2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(U7_4<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotADirectory(path = path)))))
        }
        if (!fs_has_children(files, path)) {
            return U2_2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(U7_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
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
        return U2_1<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(sorted))
    }

    override fun create_dirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        return U2_1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
    }

    override fun delete(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        if (files.containsKey(path)) {
            files.remove(path)
            return U2_1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
        }
        if (fs_has_children(files, path)) {
            return U2_2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(U7_6<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(IoError(path = path, message = "directory not empty")))))
        }
        return U2_2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(U7_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = path)))))
    }

    override fun rename_path(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val content = files[from]
        if (content == null) {
            return U2_2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(U7_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(NotFound(path = from)))))
        }
        val bytes: salvo.SalvoBytes = salvo.SalvoBytes(content)
        files.remove(from)
        files.put(to, bytes)
        return U2_1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
    }

    override fun read_line(s: InStream): String? {
        return mem_read_line(reads, s.handle)
    }

    override fun read_all(s: InStream): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        return mem_read_all(reads, s.handle)
    }

    override fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        return mem_read_bytes(reads, s.handle, max)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val got = mem_read_bytes(reads, s.handle, max)
        if (got is U2_2<*, *>) {
            return U2_2<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val data: salvo.SalvoBytes = (got.value as salvo.SalvoBytes)
        buf.append(data)
        return U2_1<Int, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(data.size))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val got = mem_read_all(reads, s.handle)
        if (got is U2_2<*, *>) {
            return U2_2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        val text: String = (got.value as String)
        buf.append(text)
        return U2_1<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(text.toByteArray(Charsets.UTF_8).size.toLong()))
    }

    override fun read_line_to(s: InStream, buf: StringBuilder): Boolean {
        val line = mem_read_line(reads, s.handle)
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
        return (mem_read_state(reads, s.handle).at).toLong()
    }

    override fun close(s: InStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val open = mem_read_state(reads, s.handle)
        val failed = open.failed
        val source = open.source
        reads.remove(s.handle)
        (s).let {}
        if (failed) {
            return U2_2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(U2_1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
        }
        return U2_1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
    }

    override fun write(s: OutStream, text: String): Long {
        return mem_append(writes, s.handle, salvo.SalvoBytes.ofUtf8(text))
    }

    override fun write_line(s: OutStream, text: String): Long {
        return mem_append(writes, s.handle, salvo.SalvoBytes.ofUtf8("$text\n"))
    }

    override fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long {
        return mem_append(writes, s.handle, salvo.SalvoBytes(data))
    }

    override fun position__2(s: OutStream): Long {
        return (mem_write_state(writes, s.handle).buffer.size).toLong()
    }

    override fun flush(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val open = mem_write_state(writes, s.handle)
        files.put(open.path, salvo.SalvoBytes(open.buffer))
        return U2_1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
    }

    override fun close__2(s: OutStream): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        val open = mem_write_state(writes, s.handle)
        files.put(open.path, salvo.SalvoBytes(open.buffer))
        writes.remove(s.handle)
        (s).let {}
        return U2_1<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(Unit))
    }
}

fun fs_has_children(files: Map<String, salvo.SalvoBytes>, path: String): Boolean {
    val prefix = "$path/"
    for (key in files.keys) {
        if (key.startsWith(prefix)) {
            return true
        }
    }
    return false
}

fun mem_find_newline(data: salvo.SalvoBytes, from: Int): Int {
    val end = data.size
    var i = from
    while (i < end) {
        if (((data.getOrNull(i) ?: throw AssertionError("salvo: value is absent at fs.mem:280:19"))).toInt() == 10) {
            return i
        }
        i = i + 1
    }
    return end
}

fun mem_append(writes: MutableMap<Long, MemWrite>, handle: Long, data: salvo.SalvoBytes): Long {
    val open = mem_write_state(writes, handle)
    val grown = salvo.SalvoBytes.joined(open.buffer)
    grown.append(data)
    val buffer: salvo.SalvoBytes = grown
    writes.put(handle, MemWrite(path = open.path, buffer = buffer))
    return (data.size).toLong()
}

fun mem_read_state(reads: Map<Long, MemRead>, handle: Long): MemRead {
    val open = reads[handle]
    if (open == null) {
        throw AssertionError(("salvo: " + ("stream handle $handle was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at fs.mem:307:9"))
    }
    return MemRead(source = open.source, data = salvo.SalvoBytes(open.data), at = open.at, failed = open.failed)
}

fun mem_write_state(writes: Map<Long, MemWrite>, handle: Long): MemWrite {
    val open = writes[handle]
    if (open == null) {
        throw AssertionError(("salvo: " + ("stream handle $handle was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at fs.mem:315:9"))
    }
    return MemWrite(path = open.path, buffer = salvo.SalvoBytes(open.buffer))
}

fun mem_read_line(reads: MutableMap<Long, MemRead>, handle: Long): String? {
    val open = mem_read_state(reads, handle)
    if (open.failed) {
        return null
    }
    val at = open.at
    val bytes: salvo.SalvoBytes = salvo.SalvoBytes(open.data)
    val end = bytes.size
    if (at >= end) {
        return null
    }
    val stop = mem_find_newline(bytes, at)
    val line = (bytes.slice(at, stop) ?: throw AssertionError("salvo: value is absent at fs.mem:339:16"))
    var next_at = stop
    if (stop < end) {
        next_at = stop + 1
    }
    val text = line.asString()
    if (text == null) {
        reads.put(handle, MemRead(source = open.source, data = bytes, at = next_at, failed = true))
        return null
    }
    reads.put(handle, MemRead(source = open.source, data = bytes, at = next_at, failed = false))
    return text.removeSuffix("\r")
}

fun mem_read_all(reads: MutableMap<Long, MemRead>, handle: Long): Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val open = mem_read_state(reads, handle)
    val source: String = open.source
    if (open.failed) {
        return U2_2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(U2_1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
    }
    val bytes: salvo.SalvoBytes = salvo.SalvoBytes(open.data)
    val end = bytes.size
    val rest = (bytes.slice(open.at, end) ?: throw AssertionError("salvo: value is absent at fs.mem:363:16"))
    val text = rest.asString()
    if (text == null) {
        reads.put(handle, MemRead(source = source, data = bytes, at = end, failed = true))
        return U2_2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(U2_1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
    }
    reads.put(handle, MemRead(source = source, data = bytes, at = end, failed = false))
    return U2_1<String, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(text))
}

fun mem_read_bytes(reads: MutableMap<Long, MemRead>, handle: Long, max: Int): Union2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    val open = mem_read_state(reads, handle)
    val source: String = open.source
    if (open.failed) {
        return U2_2<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(err(checked<Union2<InvalidUtf8, StreamFailed>>(U2_1<InvalidUtf8, StreamFailed>(InvalidUtf8(source = source)))))
    }
    val bytes: salvo.SalvoBytes = salvo.SalvoBytes(open.data)
    var stop = open.at + max
    if (max < 0) {
        stop = open.at
    }
    val end = bytes.size
    if (stop > end) {
        stop = end
    }
    val taken = (bytes.slice(open.at, stop) ?: throw AssertionError("salvo: value is absent at fs.mem:389:17"))
    reads.put(handle, MemRead(source = source, data = bytes, at = stop, failed = false))
    return U2_1<salvo.SalvoBytes, Checked<Union2<InvalidUtf8, StreamFailed>>>(ok(taken))
}
