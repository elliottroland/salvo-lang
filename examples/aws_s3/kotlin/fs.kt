package salvo.fs

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.deque.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.path.*
import salvo.stream.*

// Factories for the host: one per arm of the union [platform-factory].
object FsErrors {
    fun notFound(value: NotFound): Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming> = salvo.Union7.U1(value)
    fun permissionDenied(value: PermissionDenied): Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming> = salvo.Union7.U2(value)
    fun alreadyExists(value: AlreadyExists): Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming> = salvo.Union7.U3(value)
    fun notADirectory(value: NotADirectory): Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming> = salvo.Union7.U4(value)
    fun pathEscapes(value: PathEscapes): Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming> = salvo.Union7.U5(value)
    fun ioError(value: IoError): Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming> = salvo.Union7.U6(value)
    fun streaming(value: Streaming): Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming> = salvo.Union7.U7(value)
}

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

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun toStr(kind: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): String {
    when (kind) {
        is Union7.U1<*, *, *, *, *, *, *> -> {
            return "no such file or directory: ${(kind.value as NotFound).path}"
        }
        is Union7.U2<*, *, *, *, *, *, *> -> {
            return "permission denied: ${(kind.value as PermissionDenied).path}"
        }
        is Union7.U3<*, *, *, *, *, *, *> -> {
            return "already exists: ${(kind.value as AlreadyExists).path}"
        }
        is Union7.U4<*, *, *, *, *, *, *> -> {
            return "not a directory: ${(kind.value as NotADirectory).path}"
        }
        is Union7.U5<*, *, *, *, *, *, *> -> {
            return "path escapes the root: ${(kind.value as PathEscapes).path}"
        }
        is Union7.U6<*, *, *, *, *, *, *> -> {
            return "io error: ${(kind.value as IoError).path}: ${(kind.value as IoError).message}"
        }
        is Union7.U7<*, *, *, *, *, *, *> -> {
            return toStr__5((kind.value as Streaming).error)
        }
    }
}

fun fsStreamError(e: Checked<Union2<InvalidUtf8, StreamFailed>>): Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
    return checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(Streaming(error = detach(e))))
}

data class FileInfo(
    val size: Long,
    val isDir: Boolean,
)

object __Codec_FileInfo : salvo.WireCodec<FileInfo> {
    override fun enc(v: FileInfo, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.size, out)
        salvo.BoolCodec.enc(v.isDir, out)
    }
    override fun dec(inp: salvo.WireIn): FileInfo = FileInfo(salvo.LongCodec.dec(inp), salvo.BoolCodec.dec(inp))
}

interface Fs {
    fun openRead(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun openReadAt(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun openWrite(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun openAppend(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun exists(path: String): Boolean
    fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun listDir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun createDirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun delete(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun renamePath(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
}

class __Mon_Fs(
    private val inner: Fs,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Fs {
    override fun openRead(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.openRead(path) } finally { lock.unlock() }
    }
    override fun openReadAt(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.openReadAt(path, offset) } finally { lock.unlock() }
    }
    override fun openWrite(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.openWrite(path) } finally { lock.unlock() }
    }
    override fun openAppend(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.openAppend(path) } finally { lock.unlock() }
    }
    override fun exists(path: String): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.exists(path) } finally { lock.unlock() }
    }
    override fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.metadata(path) } finally { lock.unlock() }
    }
    override fun listDir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.listDir(path) } finally { lock.unlock() }
    }
    override fun createDirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.createDirs(path) } finally { lock.unlock() }
    }
    override fun delete(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.delete(path) } finally { lock.unlock() }
    }
    override fun renamePath(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.renamePath(from, to) } finally { lock.unlock() }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun openLines(fs: Fs, streams: Streams, path: String): Union2<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.openRead(path)
    if (opened is Union2.U2<*, *>) {
        return Union2.U2<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    return Union2.U1<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(lines__2((opened.value as InStream))))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun openChunks(fs: Fs, streams: Streams, path: String, size: Int): Union2<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.openRead(path)
    if (opened is Union2.U2<*, *>) {
        return Union2.U2<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    return Union2.U1<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(chunks((opened.value as InStream), size)))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readToStr(fs: Fs, streams: Streams, path: String): Union2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.openRead(path)
    if (opened is Union2.U2<*, *>) {
        return Union2.U2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: InStream = (opened.value as InStream)
    val content = streams.readAll(s)
    if (content is Union2.U2<*, *>) {
        val closed = streams.close(s)
        if (closed is Union2.U2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return Union2.U2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((content.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    val closed = streams.close(s)
    if (closed is Union2.U2<*, *>) {
        return Union2.U2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    return Union2.U1<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok((content.value as String)))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readLines(fs: Fs, streams: Streams, path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.openRead(path)
    if (opened is Union2.U2<*, *>) {
        return Union2.U2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val p = lines__2((opened.value as InStream))
    val out: MutableList<String> = mutableListOf<String>()
    while (true) {
        val __loop1_step = next__21(streams, p)
        if (__loop1_step !is Union2.U1<String, Finished>) { break }
        val line = __loop1_step.value
        out.add(line)
    }
    val closed = close__2(streams, p)
    if (closed is Union2.U2<*, *>) {
        return Union2.U2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    val done: List<String> = out
    return Union2.U1<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun writeStr(fs: Fs, streams: Streams, path: String, content: String): Union2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.openWrite(path)
    if (opened is Union2.U2<*, *>) {
        return Union2.U2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: OutStream = (opened.value as OutStream)
    val written = streams.write(s, content)
    val closed = streams.close__2(s)
    if (closed is Union2.U2<*, *>) {
        return Union2.U2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    return Union2.U1<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(written))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readToBytes(fs: Fs, streams: Streams, path: String): Union2<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.openRead(path)
    if (opened is Union2.U2<*, *>) {
        return Union2.U2<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: InStream = (opened.value as InStream)
    val buf = salvo.SalvoBytes.joined()
    val filling = fillFrom(streams, s, buf)
    if (filling is Union2.U2<*, *>) {
        val closed = streams.close(s)
        if (closed is Union2.U2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return Union2.U2<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((filling.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    val closed = streams.close(s)
    if (closed is Union2.U2<*, *>) {
        return Union2.U2<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    val done: salvo.SalvoBytes = buf
    return Union2.U1<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun writeBytesTo(fs: Fs, streams: Streams, path: String, data: salvo.SalvoBytes): Union2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.openWrite(path)
    if (opened is Union2.U2<*, *>) {
        return Union2.U2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: OutStream = (opened.value as OutStream)
    val written = streams.writeBytes(s, data)
    val closed = streams.close__2(s)
    if (closed is Union2.U2<*, *>) {
        return Union2.U2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    return Union2.U1<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(written))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun copyFile(fs: Fs, streams: Streams, from: String, to: String): Union2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened = fs.openRead(from)
    if (opened is Union2.U2<*, *>) {
        return Union2.U2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val s: InStream = (opened.value as InStream)
    val created = fs.openWrite(to)
    if (created is Union2.U2<*, *>) {
        val closed = streams.close(s)
        if (closed is Union2.U2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return Union2.U2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>((created.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
    }
    val w: OutStream = (created.value as OutStream)
    val moved = copyStream(streams, s, w)
    val shutW = streams.close__2(w)
    val shutS = streams.close(s)
    if (moved is Union2.U2<*, *>) {
        if (shutW is Union2.U2<*, *>) {
            ignore((shutW.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        if (shutS is Union2.U2<*, *>) {
            ignore((shutS.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return Union2.U2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((moved.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    if (shutW is Union2.U2<*, *>) {
        if (shutS is Union2.U2<*, *>) {
            ignore((shutS.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return Union2.U2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((shutW.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    if (shutS is Union2.U2<*, *>) {
        return Union2.U2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsStreamError((shutS.value as Checked<Union2<InvalidUtf8, StreamFailed>>))))
    }
    return Union2.U1<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok((moved.value as Long)))
}

fun openRead(fs: Fs, streams: Streams, p: Path): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return fs.openRead(toStr__4(p))
}

fun openReadAt(fs: Fs, streams: Streams, p: Path, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return fs.openReadAt(toStr__4(p), offset)
}

fun openWrite(fs: Fs, streams: Streams, p: Path): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return fs.openWrite(toStr__4(p))
}

fun openAppend(fs: Fs, streams: Streams, p: Path): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return fs.openAppend(toStr__4(p))
}

fun exists(fs: Fs, streams: Streams, p: Path): Boolean {
    return fs.exists(toStr__4(p))
}

fun metadata(fs: Fs, streams: Streams, p: Path): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return fs.metadata(toStr__4(p))
}

fun listDir(fs: Fs, streams: Streams, p: Path): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return fs.listDir(toStr__4(p))
}

fun createDirs(fs: Fs, streams: Streams, p: Path): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return fs.createDirs(toStr__4(p))
}

fun delete(fs: Fs, streams: Streams, p: Path): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return fs.delete(toStr__4(p))
}

fun openLines__2(fs: Fs, streams: Streams, p: Path): Union2<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return openLines(fs, streams, toStr__4(p))
}

fun openChunks__2(fs: Fs, streams: Streams, p: Path, size: Int): Union2<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return openChunks(fs, streams, toStr__4(p), size)
}

fun readToStr__2(fs: Fs, streams: Streams, p: Path): Union2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return readToStr(fs, streams, toStr__4(p))
}

fun readLines__2(fs: Fs, streams: Streams, p: Path): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return readLines(fs, streams, toStr__4(p))
}

fun writeStr__2(fs: Fs, streams: Streams, p: Path, content: String): Union2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return writeStr(fs, streams, toStr__4(p), content)
}

fun readToBytes__2(fs: Fs, streams: Streams, p: Path): Union2<salvo.SalvoBytes, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return readToBytes(fs, streams, toStr__4(p))
}

fun writeBytesTo__2(fs: Fs, streams: Streams, p: Path, data: salvo.SalvoBytes): Union2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return writeBytesTo(fs, streams, toStr__4(p), salvo.SalvoBytes(data))
}

fun renamePath(fs: Fs, streams: Streams, from: Path, to: Path): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return fs.renamePath(toStr__4(from), toStr__4(to))
}

fun copyFile__2(fs: Fs, streams: Streams, from: Path, to: Path): Union2<Long, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    return copyFile(fs, streams, toStr__4(from), toStr__4(to))
}
