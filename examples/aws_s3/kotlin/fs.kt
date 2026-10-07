package salvo.fs

import salvo.*


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
    val error: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>,
)

object __Codec_Streaming : salvo.WireCodec<Streaming> {
    override fun enc(v: Streaming, out: salvo.WireOut) {
        salvo.Union2Codec(salvo.stream.__Codec_InvalidUtf8, salvo.stream.__Codec_StreamFailed).enc(v.error, out)
    }
    override fun dec(inp: salvo.WireIn): Streaming = Streaming(salvo.Union2Codec(salvo.stream.__Codec_InvalidUtf8, salvo.stream.__Codec_StreamFailed).dec(inp))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun toStr(kind: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): String {
    return when {
        (kind is Union7.U1<*, *, *, *, *, *, *>) -> {
            val kind_1: NotFound = ((kind as Union7.U1<*, *, *, *, *, *, *>).value as NotFound)
            return "no such file or directory: ${kind_1.path}"
        }
        (kind is Union7.U2<*, *, *, *, *, *, *>) -> {
            val kind_2: PermissionDenied = ((kind as Union7.U2<*, *, *, *, *, *, *>).value as PermissionDenied)
            return "permission denied: ${kind_2.path}"
        }
        (kind is Union7.U3<*, *, *, *, *, *, *>) -> {
            val kind_3: AlreadyExists = ((kind as Union7.U3<*, *, *, *, *, *, *>).value as AlreadyExists)
            return "already exists: ${kind_3.path}"
        }
        (kind is Union7.U4<*, *, *, *, *, *, *>) -> {
            val kind_4: NotADirectory = ((kind as Union7.U4<*, *, *, *, *, *, *>).value as NotADirectory)
            return "not a directory: ${kind_4.path}"
        }
        (kind is Union7.U5<*, *, *, *, *, *, *>) -> {
            val kind_5: PathEscapes = ((kind as Union7.U5<*, *, *, *, *, *, *>).value as PathEscapes)
            return "path escapes the root: ${kind_5.path}"
        }
        (kind is Union7.U6<*, *, *, *, *, *, *>) -> {
            val kind_6: IoError = ((kind as Union7.U6<*, *, *, *, *, *, *>).value as IoError)
            return "io error: ${kind_6.path}: ${kind_6.message}"
        }
        (kind is Union7.U7<*, *, *, *, *, *, *>) -> {
            val kind_7: Streaming = ((kind as Union7.U7<*, *, *, *, *, *, *>).value as Streaming)
            return salvo.stream.toStr(kind_7.error)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

fun fsStreamError(e: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>): salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
    return salvo.core.checked.checked(Union7.U7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(Streaming(error = salvo.core.checked.detach(e))))
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
    fun openRead(path: salvo.fs.path.Path): Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun openReadAt(path: salvo.fs.path.Path, offset: Long): Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun openWrite(path: salvo.fs.path.Path): Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun openAppend(path: salvo.fs.path.Path): Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun exists(path: salvo.fs.path.Path): Boolean
    fun metadata(path: salvo.fs.path.Path): Union2<FileInfo, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun listDir(path: salvo.fs.path.Path): Union2<List<String>, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun createDirs(path: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun delete(path: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
    fun renamePath(from: salvo.fs.path.Path, to: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>
}

class __Mon_Fs(
    private val inner: Fs,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Fs {
    override fun openRead(path: salvo.fs.path.Path): Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.openRead(path) } finally { lock.unlock() }
    }
    override fun openReadAt(path: salvo.fs.path.Path, offset: Long): Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.openReadAt(path, offset) } finally { lock.unlock() }
    }
    override fun openWrite(path: salvo.fs.path.Path): Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.openWrite(path) } finally { lock.unlock() }
    }
    override fun openAppend(path: salvo.fs.path.Path): Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.openAppend(path) } finally { lock.unlock() }
    }
    override fun exists(path: salvo.fs.path.Path): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.exists(path) } finally { lock.unlock() }
    }
    override fun metadata(path: salvo.fs.path.Path): Union2<FileInfo, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.metadata(path) } finally { lock.unlock() }
    }
    override fun listDir(path: salvo.fs.path.Path): Union2<List<String>, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.listDir(path) } finally { lock.unlock() }
    }
    override fun createDirs(path: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.createDirs(path) } finally { lock.unlock() }
    }
    override fun delete(path: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.delete(path) } finally { lock.unlock() }
    }
    override fun renamePath(from: salvo.fs.path.Path, to: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.renamePath(from, to) } finally { lock.unlock() }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun openLines(fs: Fs, streams: salvo.stream.Streams, path: salvo.fs.path.Path): Union2<salvo.stream.Lines, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> = fs.openRead(path)
    if ((opened is Union2.U2<*, *>)) {
        val opened_1: salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)
        return Union2.U2<salvo.stream.Lines, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(opened_1)
    }
    val opened_2: salvo.stream.InStream = ((opened as Union2.U1<*, *>).value as salvo.stream.InStream)
    return Union2.U1<salvo.stream.Lines, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.ok(salvo.stream.lines(opened_2)))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun openChunks(fs: Fs, streams: salvo.stream.Streams, path: salvo.fs.path.Path, size: Int): Union2<salvo.stream.Chunks, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> = fs.openRead(path)
    if ((opened is Union2.U2<*, *>)) {
        val opened_1: salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)
        return Union2.U2<salvo.stream.Chunks, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(opened_1)
    }
    val opened_2: salvo.stream.InStream = ((opened as Union2.U1<*, *>).value as salvo.stream.InStream)
    return Union2.U1<salvo.stream.Chunks, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.ok(salvo.stream.chunks(opened_2, size)))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readToStr(fs: Fs, streams: salvo.stream.Streams, path: salvo.fs.path.Path): Union2<String, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> = fs.openRead(path)
    if ((opened is Union2.U2<*, *>)) {
        val opened_1: salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)
        return Union2.U2<String, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(opened_1)
    }
    val opened_2: salvo.stream.InStream = ((opened as Union2.U1<*, *>).value as salvo.stream.InStream)
    val s: salvo.stream.InStream = opened_2
    val content: Union2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.readAll(s)
    if ((content is Union2.U2<*, *>)) {
        val content_3: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((content as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        val closed: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
        if ((closed is Union2.U2<*, *>)) {
            val closed_4: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
            salvo.core.checked.ignore(closed_4)
        }
        return Union2.U2<String, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(content_3)))
    }
    val closed: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
    if ((closed is Union2.U2<*, *>)) {
        val closed_5: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        return Union2.U2<String, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(closed_5)))
    }
    val content_6: String = ((content as Union2.U1<*, *>).value as String)
    return Union2.U1<String, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.ok(content_6))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readLines(fs: Fs, streams: salvo.stream.Streams, path: salvo.fs.path.Path): Union2<List<String>, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> = fs.openRead(path)
    if ((opened is Union2.U2<*, *>)) {
        val opened_1: salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)
        return Union2.U2<List<String>, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(opened_1)
    }
    val opened_2: salvo.stream.InStream = ((opened as Union2.U1<*, *>).value as salvo.stream.InStream)
    val p: salvo.stream.Lines = salvo.stream.lines(opened_2)
    val out: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    while (true) {
        val __step_4: Union2<String, salvo.core.iterator.Finished> = salvo.stream.next__Lines(streams, p)
        when {
            (__step_4 is Union2.U1<*, *>) -> {
                val __emitted_5: String = ((__step_4 as Union2.U1<*, *>).value as String)
                val line: String = __emitted_5
                salvo.core.list.addPlatform(out, line)
            }
            else -> {
                break
            }
        }
    }
    val closed: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = salvo.stream.close__Lines(streams, p)
    if ((closed is Union2.U2<*, *>)) {
        val closed_6: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        return Union2.U2<List<String>, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(closed_6)))
    }
    val done: List<String> = out
    return Union2.U1<List<String>, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun writeStr(fs: Fs, streams: salvo.stream.Streams, path: salvo.fs.path.Path, content: String): Union2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened: Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> = fs.openWrite(path)
    if ((opened is Union2.U2<*, *>)) {
        val opened_1: salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)
        return Union2.U2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(opened_1)
    }
    val opened_2: salvo.stream.OutStream = ((opened as Union2.U1<*, *>).value as salvo.stream.OutStream)
    val s: salvo.stream.OutStream = opened_2
    val written: Long = streams.write(s, content)
    val closed: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__OutStream(s)
    if ((closed is Union2.U2<*, *>)) {
        val closed_3: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        return Union2.U2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(closed_3)))
    }
    return Union2.U1<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.ok(written))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readToBytes(fs: Fs, streams: salvo.stream.Streams, path: salvo.fs.path.Path): Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> = fs.openRead(path)
    if ((opened is Union2.U2<*, *>)) {
        val opened_1: salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)
        return Union2.U2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(opened_1)
    }
    val opened_2: salvo.stream.InStream = ((opened as Union2.U1<*, *>).value as salvo.stream.InStream)
    val s: salvo.stream.InStream = opened_2
    val buf: salvo.platform.core.bytes.MutBytes = salvo.core.bytes.mutBytes(arrayOf<salvo.platform.core.bytes.Bytes>())
    val filling: Union2<Long, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = salvo.stream.fillFrom(streams, s, buf)
    if ((filling is Union2.U2<*, *>)) {
        val filling_3: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((filling as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        val closed: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
        if ((closed is Union2.U2<*, *>)) {
            val closed_4: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
            salvo.core.checked.ignore(closed_4)
        }
        return Union2.U2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(filling_3)))
    }
    val closed: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
    if ((closed is Union2.U2<*, *>)) {
        val closed_5: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        return Union2.U2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(closed_5)))
    }
    val done: salvo.platform.core.bytes.Bytes = buf
    return Union2.U1<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.ok(done))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun writeBytesTo(fs: Fs, streams: salvo.stream.Streams, path: salvo.fs.path.Path, data: salvo.platform.core.bytes.Bytes): Union2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened: Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> = fs.openWrite(path)
    if ((opened is Union2.U2<*, *>)) {
        val opened_1: salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)
        return Union2.U2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(opened_1)
    }
    val opened_2: salvo.stream.OutStream = ((opened as Union2.U1<*, *>).value as salvo.stream.OutStream)
    val s: salvo.stream.OutStream = opened_2
    val written: Long = streams.writeBytes(s, data)
    val closed: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__OutStream(s)
    if ((closed is Union2.U2<*, *>)) {
        val closed_3: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        return Union2.U2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(closed_3)))
    }
    return Union2.U1<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.ok(written))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun copyFile(fs: Fs, streams: salvo.stream.Streams, from: salvo.fs.path.Path, to: salvo.fs.path.Path): Union2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    val opened: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> = fs.openRead(from)
    if ((opened is Union2.U2<*, *>)) {
        val opened_1: salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)
        return Union2.U2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(opened_1)
    }
    val opened_2: salvo.stream.InStream = ((opened as Union2.U1<*, *>).value as salvo.stream.InStream)
    val s: salvo.stream.InStream = opened_2
    val created: Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> = fs.openWrite(to)
    if ((created is Union2.U2<*, *>)) {
        val created_3: salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = ((created as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)
        val closed: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
        if ((closed is Union2.U2<*, *>)) {
            val closed_4: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
            salvo.core.checked.ignore(closed_4)
        }
        return Union2.U2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(created_3)
    }
    val created_5: salvo.stream.OutStream = ((created as Union2.U1<*, *>).value as salvo.stream.OutStream)
    val w: salvo.stream.OutStream = created_5
    val moved: Union2<Long, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = salvo.stream.copyStream(streams, s, w)
    val shutW: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__OutStream(w)
    val shutS: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
    if ((moved is Union2.U2<*, *>)) {
        val moved_6: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((moved as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        if ((shutW is Union2.U2<*, *>)) {
            val shutW_7: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((shutW as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
            salvo.core.checked.ignore(shutW_7)
        }
        if ((shutS is Union2.U2<*, *>)) {
            val shutS_8: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((shutS as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
            salvo.core.checked.ignore(shutS_8)
        }
        return Union2.U2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(moved_6)))
    }
    if ((shutW is Union2.U2<*, *>)) {
        val shutW_9: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((shutW as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        if ((shutS is Union2.U2<*, *>)) {
            val shutS_10: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((shutS as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
            salvo.core.checked.ignore(shutS_10)
        }
        return Union2.U2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(shutW_9)))
    }
    if ((shutS is Union2.U2<*, *>)) {
        val shutS_11: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((shutS as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
        return Union2.U2<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.err(fsStreamError(shutS_11)))
    }
    val moved_12: Long = ((moved as Union2.U1<*, *>).value as Long)
    return Union2.U1<Long, salvo.core.checked.Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(salvo.core.result.ok(moved_12))
}

