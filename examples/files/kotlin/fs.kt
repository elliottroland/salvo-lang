package salvo.fs

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
import salvo.stream.*

data class NotFound(
    val path: String,
)

object __Codec_NotFound : salvo.WireCodec<NotFound> {
    override fun enc(v: NotFound, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
    }
    override fun dec(inp: salvo.WireIn): NotFound = NotFound(salvo.StrCodec.dec(inp))
}

data class PermissionDenied(
    val path: String,
)

object __Codec_PermissionDenied : salvo.WireCodec<PermissionDenied> {
    override fun enc(v: PermissionDenied, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
    }
    override fun dec(inp: salvo.WireIn): PermissionDenied = PermissionDenied(salvo.StrCodec.dec(inp))
}

data class AlreadyExists(
    val path: String,
)

object __Codec_AlreadyExists : salvo.WireCodec<AlreadyExists> {
    override fun enc(v: AlreadyExists, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
    }
    override fun dec(inp: salvo.WireIn): AlreadyExists = AlreadyExists(salvo.StrCodec.dec(inp))
}

data class NotADirectory(
    val path: String,
)

object __Codec_NotADirectory : salvo.WireCodec<NotADirectory> {
    override fun enc(v: NotADirectory, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
    }
    override fun dec(inp: salvo.WireIn): NotADirectory = NotADirectory(salvo.StrCodec.dec(inp))
}

data class PathEscapes(
    val path: String,
)

object __Codec_PathEscapes : salvo.WireCodec<PathEscapes> {
    override fun enc(v: PathEscapes, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
    }
    override fun dec(inp: salvo.WireIn): PathEscapes = PathEscapes(salvo.StrCodec.dec(inp))
}

data class IoError(
    val path: String,
    val message: String,
)

object __Codec_IoError : salvo.WireCodec<IoError> {
    override fun enc(v: IoError, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
        salvo.StrCodec.enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): IoError = IoError(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class Streaming(
    val error: Union2<InvalidUtf8, StreamFailed>,
)

object __Codec_Streaming : salvo.WireCodec<Streaming> {
    override fun enc(v: Streaming, out: salvo.WireOut) {
        salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed).enc(v.error, out)
    }
    override fun dec(inp: salvo.WireIn): Streaming = Streaming(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed).dec(inp))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun to_str(kind: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): String {
    when (kind) {
        is U7_1<*, *, *, *, *, *, *> -> {
            return "no such file or directory: ${(kind.value as NotFound).path}"
        }
        is U7_2<*, *, *, *, *, *, *> -> {
            return "permission denied: ${(kind.value as PermissionDenied).path}"
        }
        is U7_3<*, *, *, *, *, *, *> -> {
            return "already exists: ${(kind.value as AlreadyExists).path}"
        }
        is U7_4<*, *, *, *, *, *, *> -> {
            return "not a directory: ${(kind.value as NotADirectory).path}"
        }
        is U7_5<*, *, *, *, *, *, *> -> {
            return "path escapes the root: ${(kind.value as PathEscapes).path}"
        }
        is U7_6<*, *, *, *, *, *, *> -> {
            return "io error: ${(kind.value as IoError).path}: ${(kind.value as IoError).message}"
        }
        is U7_7<*, *, *, *, *, *, *> -> {
            return to_str__4((kind.value as Streaming).error)
        }
    }
}

fun fs_stream_error(e: Checked<Union2<InvalidUtf8, StreamFailed>>): Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
    return checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(U7_7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(Streaming(error = detach(e))))
}

data class FileInfo(
    val size: Long,
    val is_dir: Boolean,
)

object __Codec_FileInfo : salvo.WireCodec<FileInfo> {
    override fun enc(v: FileInfo, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.size, out)
        salvo.BoolCodec.enc(v.is_dir, out)
    }
    override fun dec(inp: salvo.WireIn): FileInfo = FileInfo(salvo.LongCodec.dec(inp), salvo.BoolCodec.dec(inp))
}

interface Fs {
    fun open_read(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun open_read_at(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun open_write(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun open_append(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun exists(path: String): Boolean
    fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun list_dir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun create_dirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun delete(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun rename_path(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
}

class __Mon_Fs(private val inner: Fs) : Fs {
    override fun open_read(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> =
        synchronized(inner) { inner.open_read(path) }
    override fun open_read_at(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> =
        synchronized(inner) { inner.open_read_at(path, offset) }
    override fun open_write(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> =
        synchronized(inner) { inner.open_write(path) }
    override fun open_append(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> =
        synchronized(inner) { inner.open_append(path) }
    override fun exists(path: String): Boolean =
        synchronized(inner) { inner.exists(path) }
    override fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> =
        synchronized(inner) { inner.metadata(path) }
    override fun list_dir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> =
        synchronized(inner) { inner.list_dir(path) }
    override fun create_dirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> =
        synchronized(inner) { inner.create_dirs(path) }
    override fun delete(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> =
        synchronized(inner) { inner.delete(path) }
    override fun rename_path(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> =
        synchronized(inner) { inner.rename_path(from, to) }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun open_lines(fs: Fs, streams: Streams, path: String): Union2<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    return U2_1<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(lines((opened.value as InStream))))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun open_chunks(fs: Fs, streams: Streams, path: String, size: Int): Union2<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    return U2_1<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(chunks((opened.value as InStream), size)))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun read_to_str(fs: Fs, streams: Streams, path: String): Union2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: InStream = (opened.value as InStream)
    val content = streams.read_all(s)
    if (content is U2_2<*, *>) {
        val closed = streams.close(s)
        if (closed is U2_2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return U2_2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((content.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    val closed = streams.close(s)
    if (closed is U2_2<*, *>) {
        return U2_2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    return U2_1<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok((content.value as String)))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun read_lines(fs: Fs, streams: Streams, path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val p = lines((opened.value as InStream))
    val out: MutableList<String> = mutableListOf<String>()
    while (true) {
        val __loop1_step = next__13(streams, p)
        if (__loop1_step !is U2_1<String, Finished>) { break }
        val line = __loop1_step.value
        out.add(line)
    }
    val closed = close__2(streams, p)
    if (closed is U2_2<*, *>) {
        return U2_2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    val done: List<String> = out
    return U2_1<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun write_str(fs: Fs, streams: Streams, path: String, content: String): Union2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.open_write(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: OutStream = (opened.value as OutStream)
    val written = streams.write(s, content)
    val closed = streams.close__2(s)
    if (closed is U2_2<*, *>) {
        return U2_2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    return U2_1<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(written))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun read_to_bytes(fs: Fs, streams: Streams, path: String): Union2<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        return U2_2<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: InStream = (opened.value as InStream)
    val buf = salvo.SalvoBytes.joined()
    val filling = fill_from(streams, s, buf)
    if (filling is U2_2<*, *>) {
        val closed = streams.close(s)
        if (closed is U2_2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return U2_2<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((filling.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    val closed = streams.close(s)
    if (closed is U2_2<*, *>) {
        return U2_2<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    val done: salvo.SalvoBytes = buf
    return U2_1<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun write_bytes_to(fs: Fs, streams: Streams, path: String, data: salvo.SalvoBytes): Union2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.open_write(path)
    if (opened is U2_2<*, *>) {
        return U2_2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: OutStream = (opened.value as OutStream)
    val written = streams.write_bytes(s, data)
    val closed = streams.close__2(s)
    if (closed is U2_2<*, *>) {
        return U2_2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    return U2_1<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(written))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun copy_file(fs: Fs, streams: Streams, from: String, to: String): Union2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.open_read(from)
    if (opened is U2_2<*, *>) {
        return U2_2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: InStream = (opened.value as InStream)
    val created = fs.open_write(to)
    if (created is U2_2<*, *>) {
        val closed = streams.close(s)
        if (closed is U2_2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return U2_2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((created.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val w: OutStream = (created.value as OutStream)
    val moved = copy_stream(streams, s, w)
    val shut_w = streams.close__2(w)
    val shut_s = streams.close(s)
    if (moved is U2_2<*, *>) {
        if (shut_w is U2_2<*, *>) {
            ignore((shut_w.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        if (shut_s is U2_2<*, *>) {
            ignore((shut_s.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return U2_2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((moved.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    if (shut_w is U2_2<*, *>) {
        if (shut_s is U2_2<*, *>) {
            ignore((shut_s.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return U2_2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((shut_w.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    if (shut_s is U2_2<*, *>) {
        return U2_2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fs_stream_error((shut_s.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    return U2_1<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok((moved.value as Long)))
}
