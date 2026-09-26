package salvo.core.fs

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
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
    fun open_read(path: String): Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun open_read_at(path: String, offset: Long): Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun open_write(path: String): Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun open_append(path: String): Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun exists(path: String): Boolean
    fun metadata(path: String): Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun list_dir(path: String): Union2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun create_dirs(path: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun delete(path: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun rename_path(from: String, to: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun read_line(s: InStream): String?
    fun read_all(s: InStream): Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun read_line_to(s: InStream, buf: StringBuilder): Boolean
    fun position(s: InStream): Long
    fun close(s: InStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun write(s: OutStream, text: String): Long
    fun write_line(s: OutStream, text: String): Long
    fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long
    fun position__2(s: OutStream): Long
    fun flush(s: OutStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
    fun close__2(s: OutStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>
}

class __Mon_Fs(private val inner: Fs) : Fs {
    override fun open_read(path: String): Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.open_read(path) }
    override fun open_read_at(path: String, offset: Long): Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.open_read_at(path, offset) }
    override fun open_write(path: String): Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.open_write(path) }
    override fun open_append(path: String): Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.open_append(path) }
    override fun exists(path: String): Boolean =
        synchronized(inner) { inner.exists(path) }
    override fun metadata(path: String): Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.metadata(path) }
    override fun list_dir(path: String): Union2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.list_dir(path) }
    override fun create_dirs(path: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.create_dirs(path) }
    override fun delete(path: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.delete(path) }
    override fun rename_path(from: String, to: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.rename_path(from, to) }
    override fun read_line(s: InStream): String? =
        synchronized(inner) { inner.read_line(s) }
    override fun read_all(s: InStream): Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.read_all(s) }
    override fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.read_bytes(s, max) }
    override fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.read_to(s, buf, max) }
    override fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.read_to__2(s, buf) }
    override fun read_line_to(s: InStream, buf: StringBuilder): Boolean =
        synchronized(inner) { inner.read_line_to(s, buf) }
    override fun position(s: InStream): Long =
        synchronized(inner) { inner.position(s) }
    override fun close(s: InStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.close(s) }
    override fun write(s: OutStream, text: String): Long =
        synchronized(inner) { inner.write(s, text) }
    override fun write_line(s: OutStream, text: String): Long =
        synchronized(inner) { inner.write_line(s, text) }
    override fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long =
        synchronized(inner) { inner.write_bytes(s, data) }
    override fun position__2(s: OutStream): Long =
        synchronized(inner) { inner.position__2(s) }
    override fun flush(s: OutStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.flush(s) }
    override fun close__2(s: OutStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> =
        synchronized(inner) { inner.close__2(s) }
}

data class Lines(
    var s: InStream,
)

fun lines(s: InStream): Lines {
    return Lines(s = s)
}

fun<__Fx> next__3(__fx: __Fx, p: Lines): Union2<String, Finished> where __Fx : __Has_Fs {
    val line = __fx.__fx_Fs.read_line(p.s)
    when {
        line != null -> {
            return U2_1<String, Finished>(emitted(line))
        }
        else -> {
            return U2_2<String, Finished>(finished())
        }
    }
}

fun<__Fx> close(__fx: __Fx, p: Lines): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    return __fx.__fx_Fs.close(p.s)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> open_lines(__fx: __Fx, path: String): Union2<Lines, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    val opened = __fx.__fx_Fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Lines, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    return U2_1<Lines, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(lines((opened.value as InStream))))
}

data class Chunks(
    var s: InStream,
    var size: Int,
)

fun chunks(s: InStream, size: Int): Chunks {
    return Chunks(s = s, size = size)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> next__4(__fx: __Fx, p: Chunks): Union2<salvo.SalvoBytes, Finished> where __Fx : __Has_Fs {
    val got = __fx.__fx_Fs.read_bytes(p.s, p.size)
    if (got is U2_2<*, *>) {
        ignore((got.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        return U2_2<salvo.SalvoBytes, Finished>(finished())
    }
    val data: salvo.SalvoBytes = (got.value as salvo.SalvoBytes)
    if (data.size == 0) {
        return U2_2<salvo.SalvoBytes, Finished>(finished())
    }
    return U2_1<salvo.SalvoBytes, Finished>(emitted(data))
}

fun<__Fx> close__2(__fx: __Fx, p: Chunks): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    return __fx.__fx_Fs.close(p.s)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> open_chunks(__fx: __Fx, path: String, size: Int): Union2<Chunks, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    val opened = __fx.__fx_Fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Chunks, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    return U2_1<Chunks, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(chunks((opened.value as InStream), size)))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> read_to_str(__fx: __Fx, path: String): Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    val opened = __fx.__fx_Fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val s: InStream = (opened.value as InStream)
    val content = __fx.__fx_Fs.read_all(s)
    if (content is U2_2<*, *>) {
        val closed = __fx.__fx_Fs.close(s)
        if (closed is U2_2<*, *>) {
            ignore((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        return U2_2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((content.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val closed = __fx.__fx_Fs.close(s)
    if (closed is U2_2<*, *>) {
        return U2_2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    return U2_1<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((content.value as String))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> read_lines(__fx: __Fx, path: String): Union2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    val opened = __fx.__fx_Fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val p = lines((opened.value as InStream))
    val out: MutableList<String> = mutableListOf<String>()
    while (true) {
        val __loop1_step = next__3(__fx, p)
        if (__loop1_step !is U2_1<String, Finished>) { break }
        val line = __loop1_step.value
        out.add(line)
    }
    val closed = close(__fx, p)
    if (closed is U2_2<*, *>) {
        return U2_2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val done: List<String> = out
    return U2_1<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> write_str(__fx: __Fx, path: String, content: String): Union2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    val opened = __fx.__fx_Fs.open_write(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val s: OutStream = (opened.value as OutStream)
    val written = __fx.__fx_Fs.write(s, content)
    val closed = __fx.__fx_Fs.close__2(s)
    if (closed is U2_2<*, *>) {
        return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    return U2_1<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(written))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> read_to_bytes(__fx: __Fx, path: String): Union2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    val opened = __fx.__fx_Fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val s: InStream = (opened.value as InStream)
    val buf = salvo.SalvoBytes.joined()
    val filling = fill_from(__fx, s, buf)
    if (filling is U2_2<*, *>) {
        val closed = __fx.__fx_Fs.close(s)
        if (closed is U2_2<*, *>) {
            ignore((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        return U2_2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((filling.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val closed = __fx.__fx_Fs.close(s)
    if (closed is U2_2<*, *>) {
        return U2_2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val done: salvo.SalvoBytes = buf
    return U2_1<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> write_bytes_to(__fx: __Fx, path: String, data: salvo.SalvoBytes): Union2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    val opened = __fx.__fx_Fs.open_write(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val s: OutStream = (opened.value as OutStream)
    val written = __fx.__fx_Fs.write_bytes(s, data)
    val closed = __fx.__fx_Fs.close__2(s)
    if (closed is U2_2<*, *>) {
        return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    return U2_1<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(written))
}

fun fs_chunk_size(): Int {
    return 65536
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> fill_from(__fx: __Fx, s: InStream, buf: salvo.SalvoBytes): Union2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    var total: Long = 0L
    var reading = true
    while (reading) {
        val got = __fx.__fx_Fs.read_to(s, buf, fs_chunk_size())
        if (got is U2_2<*, *>) {
            return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((got.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        val n: Int = (got.value as Int)
        total = total + (n).toLong()
        if (n == 0) {
            reading = false
        }
    }
    return U2_1<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(total))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> copy_stream(__fx: __Fx, s: InStream, w: OutStream): Union2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    val buf = salvo.SalvoBytes.joined()
    var total: Long = 0L
    var copying = true
    while (copying) {
        buf.clear()
        val got = __fx.__fx_Fs.read_to(s, buf, fs_chunk_size())
        if (got is U2_2<*, *>) {
            return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((got.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        val n: Int = (got.value as Int)
        if (n == 0) {
            copying = false
        } else {
            total = total + __fx.__fx_Fs.write_bytes(w, buf)
        }
    }
    return U2_1<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(ok(total))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> copy_file(__fx: __Fx, from: String, to: String): Union2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> where __Fx : __Has_Fs {
    val opened = __fx.__fx_Fs.open_read(from)
    if (opened is U2_2<*, *>) {
        return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val s: InStream = (opened.value as InStream)
    val created = __fx.__fx_Fs.open_write(to)
    if (created is U2_2<*, *>) {
        val closed = __fx.__fx_Fs.close(s)
        if (closed is U2_2<*, *>) {
            ignore((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((created.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    val w: OutStream = (created.value as OutStream)
    val moved = copy_stream(__fx, s, w)
    val shut_w = __fx.__fx_Fs.close__2(w)
    val shut_s = __fx.__fx_Fs.close(s)
    if (moved is U2_2<*, *>) {
        if (shut_w is U2_2<*, *>) {
            ignore((shut_w.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        if (shut_s is U2_2<*, *>) {
            ignore((shut_s.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((moved.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    if (shut_w is U2_2<*, *>) {
        if (shut_s is U2_2<*, *>) {
            ignore((shut_s.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
        }
        return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((shut_w.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    if (shut_s is U2_2<*, *>) {
        return U2_2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((shut_s.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>))
    }
    return U2_1<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>((moved.value as Long))
}
