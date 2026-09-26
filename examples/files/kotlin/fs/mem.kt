package salvo.fs.mem

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.nonempty.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.fs.*

data class MemRead(
    val path: String,
    val at: Int,
    val failed: Boolean,
)

data class MemWrite(
    val path: String,
    val buffer: salvo.SalvoBytes,
)

class MemFs : Fs {
    private var files: MutableMap<String, salvo.SalvoBytes> = linkedMapOf<String, salvo.SalvoBytes>().also { __m -> __m.putAll(listOf()) }
    private var reads: MutableMap<Long, MemRead> = linkedMapOf<Long, MemRead>().also { __m -> __m.putAll(listOf()) }
    private var writes: MutableMap<Long, MemWrite> = linkedMapOf<Long, MemWrite>().also { __m -> __m.putAll(listOf()) }
    private var next_handle: Long = 0L

    override fun open_read(path: String): Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        if (!files.containsKey(path)) {
            return U2_2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(NotFound(path = path)))))
        }
        next_handle = next_handle + 1
        reads.put(next_handle, MemRead(path = path, at = 0, failed = false))
        return U2_1<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(InStream(handle = next_handle)))
    }

    override fun open_read_at(path: String, offset: Long): Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val content = files[path]
        if (content == null) {
            return U2_2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(NotFound(path = path)))))
        }
        var at = (offset).toInt()
        if (at < 0) {
            at = 0
        }
        val end = content.size
        if (at > end) {
            at = end
        }
        next_handle = next_handle + 1
        reads.put(next_handle, MemRead(path = path, at = at, failed = false))
        return U2_1<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(InStream(handle = next_handle)))
    }

    override fun open_write(path: String): Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        next_handle = next_handle + 1
        val empty = salvo.SalvoBytes.joined()
        writes.put(next_handle, MemWrite(path = path, buffer = empty))
        return U2_1<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(OutStream(handle = next_handle)))
    }

    override fun open_append(path: String): Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val existing = files[path]
        val start = salvo.SalvoBytes.joined()
        if (existing == null) {
        } else {
            start.append(existing)
        }
        next_handle = next_handle + 1
        writes.put(next_handle, MemWrite(path = path, buffer = start))
        return U2_1<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(OutStream(handle = next_handle)))
    }

    override fun exists(path: String): Boolean {
        if (files.containsKey(path)) {
            return true
        }
        return fs_has_children(files, path)
    }

    override fun metadata(path: String): Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val content = files[path]
        if (content == null) {
            if (fs_has_children(files, path)) {
                return U2_1<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(FileInfo(size = 0L, is_dir = true)))
            }
            return U2_2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(NotFound(path = path)))))
        }
        return U2_1<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(FileInfo(size = (content.size).toLong(), is_dir = false)))
    }

    override fun list_dir(path: String): Union2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        if (files.containsKey(path)) {
            return U2_2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_4<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(NotADirectory(path = path)))))
        }
        if (!fs_has_children(files, path)) {
            return U2_2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(NotFound(path = path)))))
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
        return U2_1<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(sorted))
    }

    override fun create_dirs(path: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return U2_1<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(Unit))
    }

    override fun delete(path: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        if (files.containsKey(path)) {
            files.remove(path)
            return U2_1<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(Unit))
        }
        if (fs_has_children(files, path)) {
            return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(IoError(path = path, message = "directory not empty")))))
        }
        return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(NotFound(path = path)))))
    }

    override fun rename_path(from: String, to: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val content = files[from]
        if (content == null) {
            return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(NotFound(path = from)))))
        }
        val bytes: salvo.SalvoBytes = salvo.SalvoBytes(content)
        files.remove(from)
        files.put(to, bytes)
        return U2_1<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(Unit))
    }

    override fun read_line(s: InStream): String? {
        return mem_read_line(reads, files, s.handle)
    }

    override fun read_all(s: InStream): Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return mem_read_all(reads, files, s.handle)
    }

    override fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return mem_read_bytes(reads, files, s.handle, max)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val got = mem_read_bytes(reads, files, s.handle, max)
        if (got is U2_2<*, *>) {
            return U2_2<Int, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((got.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        val data: salvo.SalvoBytes = (got.value as salvo.SalvoBytes)
        buf.append(data)
        return U2_1<Int, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(data.size))
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val got = mem_read_all(reads, files, s.handle)
        if (got is U2_2<*, *>) {
            return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((got.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        val text: String = (got.value as String)
        buf.append(text)
        return U2_1<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(text.toByteArray(Charsets.UTF_8).size.toLong()))
    }

    override fun read_line_to(s: InStream, buf: StringBuilder): Boolean {
        val line = mem_read_line(reads, files, s.handle)
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
        val open = reads[s.handle]
        if (open == null) {
            return 0L
        }
        return (open.at).toLong()
    }

    override fun close(s: InStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val open = reads[s.handle]
        var failed = false
        var path = "<stream>"
        if (open != null) {
            failed = open.failed
            path = open.path
        }
        reads.remove(s.handle)
        (s).let {}
        if (failed) {
            return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_6<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(InvalidUtf8(path = path)))))
        }
        return U2_1<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(Unit))
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
        val open = writes[s.handle]
        if (open == null) {
            return 0L
        }
        return (open.buffer.size).toLong()
    }

    override fun flush(s: OutStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val open = writes[s.handle]
        if (open == null) {
            return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(StaleHandle(path = "<stream>")))))
        }
        files.put(open.path, salvo.SalvoBytes(open.buffer))
        return U2_1<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(Unit))
    }

    override fun close__2(s: OutStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val open = writes[s.handle]
        if (open == null) {
            (s).let {}
            return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(StaleHandle(path = "<stream>")))))
        }
        files.put(open.path, salvo.SalvoBytes(open.buffer))
        writes.remove(s.handle)
        (s).let {}
        return U2_1<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(Unit))
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
        if (((data.getOrNull(i) ?: throw AssertionError("salvo: value is absent at fs.mem:293:19"))).toInt() == 10) {
            return i
        }
        i = i + 1
    }
    return end
}

fun mem_append(writes: MutableMap<Long, MemWrite>, handle: Long, data: salvo.SalvoBytes): Long {
    val open = writes[handle]
    if (open == null) {
        return 0L
    }
    val grown = salvo.SalvoBytes.joined(open.buffer)
    grown.append(data)
    val buffer: salvo.SalvoBytes = grown
    writes.put(handle, MemWrite(path = open.path, buffer = buffer))
    return (data.size).toLong()
}

fun mem_read_line(reads: MutableMap<Long, MemRead>, files: Map<String, salvo.SalvoBytes>, handle: Long): String? {
    val open = reads[handle]
    if (open == null) {
        return null
    }
    if (open.failed) {
        return null
    }
    val at = open.at
    val path: String = open.path
    val content = files[path]
    if (content == null) {
        return null
    }
    val bytes: salvo.SalvoBytes = salvo.SalvoBytes(content)
    val end = bytes.size
    if (at >= end) {
        return null
    }
    val stop = mem_find_newline(bytes, at)
    val line = (bytes.slice(at, stop) ?: throw AssertionError("salvo: value is absent at fs.mem:344:16"))
    var next_at = stop
    if (stop < end) {
        next_at = stop + 1
    }
    val text = line.asString()
    if (text == null) {
        reads.put(handle, MemRead(path = path, at = next_at, failed = true))
        return null
    }
    reads.put(handle, MemRead(path = path, at = next_at, failed = false))
    return text.removeSuffix("\r")
}

fun mem_read_all(reads: MutableMap<Long, MemRead>, files: Map<String, salvo.SalvoBytes>, handle: Long): Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    val open = reads[handle]
    if (open == null) {
        return U2_2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(StaleHandle(path = "<stream>")))))
    }
    val path: String = open.path
    if (open.failed) {
        return U2_2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_6<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(InvalidUtf8(path = path)))))
    }
    val content = files[path]
    if (content == null) {
        return U2_2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(NotFound(path = path)))))
    }
    val bytes: salvo.SalvoBytes = salvo.SalvoBytes(content)
    val end = bytes.size
    val rest = (bytes.slice(open.at, end) ?: throw AssertionError("salvo: value is absent at fs.mem:375:16"))
    val text = rest.asString()
    if (text == null) {
        reads.put(handle, MemRead(path = path, at = end, failed = true))
        return U2_2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_6<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(InvalidUtf8(path = path)))))
    }
    reads.put(handle, MemRead(path = path, at = end, failed = false))
    return U2_1<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(text))
}

fun mem_read_bytes(reads: MutableMap<Long, MemRead>, files: Map<String, salvo.SalvoBytes>, handle: Long, max: Int): Union2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    val open = reads[handle]
    if (open == null) {
        return U2_2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(StaleHandle(path = "<stream>")))))
    }
    val path: String = open.path
    if (open.failed) {
        return U2_2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_6<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(InvalidUtf8(path = path)))))
    }
    val content = files[path]
    if (content == null) {
        return U2_2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_1<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(NotFound(path = path)))))
    }
    val bytes: salvo.SalvoBytes = salvo.SalvoBytes(content)
    var stop = open.at + max
    if (max < 0) {
        stop = open.at
    }
    val end = bytes.size
    if (stop > end) {
        stop = end
    }
    val taken = (bytes.slice(open.at, stop) ?: throw AssertionError("salvo: value is absent at fs.mem:408:17"))
    reads.put(handle, MemRead(path = path, at = stop, failed = false))
    return U2_1<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(taken))
}
