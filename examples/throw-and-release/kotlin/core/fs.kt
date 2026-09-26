package salvo.core.fs

import salvo.*
import salvo.core.bytes.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

data class NotFound(
    val path: String,
)

data class PermissionDenied(
    val path: String,
)

data class AlreadyExists(
    val path: String,
)

data class NotADirectory(
    val path: String,
)

data class PathEscapes(
    val path: String,
)

data class InvalidUtf8(
    val path: String,
)

data class StaleHandle(
    val path: String,
)

data class IoError(
    val path: String,
    val message: String,
)

data class FsError(
    val kind: Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>,
)

fun ignore__2(e: FsError) {
    (e).let {}
}

fun detach(e: FsError): Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError> {
    val kind = e.kind
    (e).let {}
    return kind
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun to_str(kind: Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>): String {
    when (kind) {
        is U8_1<*, *, *, *, *, *, *, *> -> {
            return "no such file or directory: ${(kind.value as NotFound).path}"
        }
        is U8_2<*, *, *, *, *, *, *, *> -> {
            return "permission denied: ${(kind.value as PermissionDenied).path}"
        }
        is U8_3<*, *, *, *, *, *, *, *> -> {
            return "already exists: ${(kind.value as AlreadyExists).path}"
        }
        is U8_4<*, *, *, *, *, *, *, *> -> {
            return "not a directory: ${(kind.value as NotADirectory).path}"
        }
        is U8_5<*, *, *, *, *, *, *, *> -> {
            return "path escapes the root: ${(kind.value as PathEscapes).path}"
        }
        is U8_6<*, *, *, *, *, *, *, *> -> {
            return "not valid UTF-8: ${(kind.value as InvalidUtf8).path}"
        }
        is U8_7<*, *, *, *, *, *, *, *> -> {
            return "stale stream token: ${(kind.value as StaleHandle).path}"
        }
        is U8_8<*, *, *, *, *, *, *, *> -> {
            return "io error: ${(kind.value as IoError).path}: ${(kind.value as IoError).message}"
        }
    }
}

fun to_str__2(e: FsError): String {
    return to_str(e.kind)
}

data class FileInfo(
    val size: Long,
    val is_dir: Boolean,
)

data class InStream(
    val handle: Long,
)

data class OutStream(
    val handle: Long,
)

interface Fs {
    fun open_read(path: String): Union2<InStream, FsError>
    fun open_read_at(path: String, offset: Long): Union2<InStream, FsError>
    fun open_write(path: String): Union2<OutStream, FsError>
    fun open_append(path: String): Union2<OutStream, FsError>
    fun exists(path: String): Boolean
    fun metadata(path: String): Union2<FileInfo, FsError>
    fun list_dir(path: String): Union2<List<String>, FsError>
    fun create_dirs(path: String): Union2<Unit, FsError>
    fun delete(path: String): Union2<Unit, FsError>
    fun rename_path(from: String, to: String): Union2<Unit, FsError>
    fun read_line(s: InStream): String?
    fun read_all(s: InStream): Union2<String, FsError>
    fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, FsError>
    fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, FsError>
    fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, FsError>
    fun read_line_to(s: InStream, buf: StringBuilder): Boolean
    fun position(s: InStream): Long
    fun close(s: InStream): Union2<Unit, FsError>
    fun write(s: OutStream, text: String): Long
    fun write_line(s: OutStream, text: String): Long
    fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long
    fun position__2(s: OutStream): Long
    fun flush(s: OutStream): Union2<Unit, FsError>
    fun close__2(s: OutStream): Union2<Unit, FsError>
}

class __Mon_Fs(private val inner: Fs) : Fs {
    override fun open_read(path: String): Union2<InStream, FsError> =
        synchronized(inner) { inner.open_read(path) }
    override fun open_read_at(path: String, offset: Long): Union2<InStream, FsError> =
        synchronized(inner) { inner.open_read_at(path, offset) }
    override fun open_write(path: String): Union2<OutStream, FsError> =
        synchronized(inner) { inner.open_write(path) }
    override fun open_append(path: String): Union2<OutStream, FsError> =
        synchronized(inner) { inner.open_append(path) }
    override fun exists(path: String): Boolean =
        synchronized(inner) { inner.exists(path) }
    override fun metadata(path: String): Union2<FileInfo, FsError> =
        synchronized(inner) { inner.metadata(path) }
    override fun list_dir(path: String): Union2<List<String>, FsError> =
        synchronized(inner) { inner.list_dir(path) }
    override fun create_dirs(path: String): Union2<Unit, FsError> =
        synchronized(inner) { inner.create_dirs(path) }
    override fun delete(path: String): Union2<Unit, FsError> =
        synchronized(inner) { inner.delete(path) }
    override fun rename_path(from: String, to: String): Union2<Unit, FsError> =
        synchronized(inner) { inner.rename_path(from, to) }
    override fun read_line(s: InStream): String? =
        synchronized(inner) { inner.read_line(s) }
    override fun read_all(s: InStream): Union2<String, FsError> =
        synchronized(inner) { inner.read_all(s) }
    override fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, FsError> =
        synchronized(inner) { inner.read_bytes(s, max) }
    override fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, FsError> =
        synchronized(inner) { inner.read_to(s, buf, max) }
    override fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, FsError> =
        synchronized(inner) { inner.read_to__2(s, buf) }
    override fun read_line_to(s: InStream, buf: StringBuilder): Boolean =
        synchronized(inner) { inner.read_line_to(s, buf) }
    override fun position(s: InStream): Long =
        synchronized(inner) { inner.position(s) }
    override fun close(s: InStream): Union2<Unit, FsError> =
        synchronized(inner) { inner.close(s) }
    override fun write(s: OutStream, text: String): Long =
        synchronized(inner) { inner.write(s, text) }
    override fun write_line(s: OutStream, text: String): Long =
        synchronized(inner) { inner.write_line(s, text) }
    override fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long =
        synchronized(inner) { inner.write_bytes(s, data) }
    override fun position__2(s: OutStream): Long =
        synchronized(inner) { inner.position__2(s) }
    override fun flush(s: OutStream): Union2<Unit, FsError> =
        synchronized(inner) { inner.flush(s) }
    override fun close__2(s: OutStream): Union2<Unit, FsError> =
        synchronized(inner) { inner.close__2(s) }
}

data class Lines(
    var s: InStream,
)

fun lines(s: InStream): Lines {
    return Lines(s = s)
}

fun next__3(fs: Fs, p: Lines): Union2<String, Finished> {
    val line = fs.read_line(p.s)
    when {
        line != null -> {
            return U2_1<String, Finished>(emitted(line))
        }
        else -> {
            return U2_2<String, Finished>(finished())
        }
    }
}

fun close(fs: Fs, p: Lines): Union2<Unit, FsError> {
    return fs.close(p.s)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun open_lines(fs: Fs, path: String): Union2<Lines, FsError> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Lines, FsError>((opened.value as FsError))
    }
    return U2_1<Lines, FsError>(ok(lines((opened.value as InStream))))
}

data class Chunks(
    var s: InStream,
    var size: Int,
)

fun chunks(s: InStream, size: Int): Chunks {
    return Chunks(s = s, size = size)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun next__4(fs: Fs, p: Chunks): Union2<salvo.SalvoBytes, Finished> {
    val got = fs.read_bytes(p.s, p.size)
    if (got is U2_2<*, *>) {
        ignore__2((got.value as FsError))
        return U2_2<salvo.SalvoBytes, Finished>(finished())
    }
    val data: salvo.SalvoBytes = (got.value as salvo.SalvoBytes)
    if (data.size == 0) {
        return U2_2<salvo.SalvoBytes, Finished>(finished())
    }
    return U2_1<salvo.SalvoBytes, Finished>(emitted(data))
}

fun close__2(fs: Fs, p: Chunks): Union2<Unit, FsError> {
    return fs.close(p.s)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun open_chunks(fs: Fs, path: String, size: Int): Union2<Chunks, FsError> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Chunks, FsError>((opened.value as FsError))
    }
    return U2_1<Chunks, FsError>(ok(chunks((opened.value as InStream), size)))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun read_to_str(fs: Fs, path: String): Union2<String, FsError> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<String, FsError>((opened.value as FsError))
    }
    val s: InStream = (opened.value as InStream)
    val content = fs.read_all(s)
    if (content is U2_2<*, *>) {
        val closed = fs.close(s)
        if (closed is U2_2<*, *>) {
            ignore__2((closed.value as FsError))
        }
        return U2_2<String, FsError>((content.value as FsError))
    }
    val closed = fs.close(s)
    if (closed is U2_2<*, *>) {
        return U2_2<String, FsError>((closed.value as FsError))
    }
    return U2_1<String, FsError>((content.value as String))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun read_lines(fs: Fs, path: String): Union2<List<String>, FsError> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<List<String>, FsError>((opened.value as FsError))
    }
    val p = lines((opened.value as InStream))
    val out: MutableList<String> = mutableListOf<String>()
    while (true) {
        val __loop1_step = next__3(fs, p)
        if (__loop1_step !is U2_1<String, Finished>) { break }
        val line = __loop1_step.value
        out.add(line)
    }
    val closed = close(fs, p)
    if (closed is U2_2<*, *>) {
        return U2_2<List<String>, FsError>((closed.value as FsError))
    }
    val done: List<String> = out
    return U2_1<List<String>, FsError>(ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun write_str(fs: Fs, path: String, content: String): Union2<Long, FsError> {
    val opened = fs.open_write(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Long, FsError>((opened.value as FsError))
    }
    val s: OutStream = (opened.value as OutStream)
    val written = fs.write(s, content)
    val closed = fs.close__2(s)
    if (closed is U2_2<*, *>) {
        return U2_2<Long, FsError>((closed.value as FsError))
    }
    return U2_1<Long, FsError>(ok(written))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun read_to_bytes(fs: Fs, path: String): Union2<salvo.SalvoBytes, FsError> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<salvo.SalvoBytes, FsError>((opened.value as FsError))
    }
    val s: InStream = (opened.value as InStream)
    val buf = salvo.SalvoBytes.joined()
    val filling = fill_from(fs, s, buf)
    if (filling is U2_2<*, *>) {
        val closed = fs.close(s)
        if (closed is U2_2<*, *>) {
            ignore__2((closed.value as FsError))
        }
        return U2_2<salvo.SalvoBytes, FsError>((filling.value as FsError))
    }
    val closed = fs.close(s)
    if (closed is U2_2<*, *>) {
        return U2_2<salvo.SalvoBytes, FsError>((closed.value as FsError))
    }
    val done: salvo.SalvoBytes = buf
    return U2_1<salvo.SalvoBytes, FsError>(ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun write_bytes_to(fs: Fs, path: String, data: salvo.SalvoBytes): Union2<Long, FsError> {
    val opened = fs.open_write(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Long, FsError>((opened.value as FsError))
    }
    val s: OutStream = (opened.value as OutStream)
    val written = fs.write_bytes(s, data)
    val closed = fs.close__2(s)
    if (closed is U2_2<*, *>) {
        return U2_2<Long, FsError>((closed.value as FsError))
    }
    return U2_1<Long, FsError>(ok(written))
}

fun fs_chunk_size(): Int {
    return 65536
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun fill_from(fs: Fs, s: InStream, buf: salvo.SalvoBytes): Union2<Long, FsError> {
    var total: Long = 0L
    var reading = true
    while (reading) {
        val got = fs.read_to(s, buf, fs_chunk_size())
        if (got is U2_2<*, *>) {
            return U2_2<Long, FsError>((got.value as FsError))
        }
        val n: Int = (got.value as Int)
        total = total + (n).toLong()
        if (n == 0) {
            reading = false
        }
    }
    return U2_1<Long, FsError>(ok(total))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun copy_stream(fs: Fs, s: InStream, w: OutStream): Union2<Long, FsError> {
    val buf = salvo.SalvoBytes.joined()
    var total: Long = 0L
    var copying = true
    while (copying) {
        buf.clear()
        val got = fs.read_to(s, buf, fs_chunk_size())
        if (got is U2_2<*, *>) {
            return U2_2<Long, FsError>((got.value as FsError))
        }
        val n: Int = (got.value as Int)
        if (n == 0) {
            copying = false
        } else {
            total = total + fs.write_bytes(w, buf)
        }
    }
    return U2_1<Long, FsError>(ok(total))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun copy_file(fs: Fs, from: String, to: String): Union2<Long, FsError> {
    val opened = fs.open_read(from)
    if (opened is U2_2<*, *>) {
        return U2_2<Long, FsError>((opened.value as FsError))
    }
    val s: InStream = (opened.value as InStream)
    val created = fs.open_write(to)
    if (created is U2_2<*, *>) {
        val closed = fs.close(s)
        if (closed is U2_2<*, *>) {
            ignore__2((closed.value as FsError))
        }
        return U2_2<Long, FsError>((created.value as FsError))
    }
    val w: OutStream = (created.value as OutStream)
    val moved = copy_stream(fs, s, w)
    val shut_w = fs.close__2(w)
    val shut_s = fs.close(s)
    if (moved is U2_2<*, *>) {
        if (shut_w is U2_2<*, *>) {
            ignore__2((shut_w.value as FsError))
        }
        if (shut_s is U2_2<*, *>) {
            ignore__2((shut_s.value as FsError))
        }
        return U2_2<Long, FsError>((moved.value as FsError))
    }
    if (shut_w is U2_2<*, *>) {
        if (shut_s is U2_2<*, *>) {
            ignore__2((shut_s.value as FsError))
        }
        return U2_2<Long, FsError>((shut_w.value as FsError))
    }
    if (shut_s is U2_2<*, *>) {
        return U2_2<Long, FsError>((shut_s.value as FsError))
    }
    return U2_1<Long, FsError>((moved.value as Long))
}
