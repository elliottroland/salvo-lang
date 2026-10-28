package salvo.std.fs.host

import salvo.*

interface RawFs {
    fun rawOpenRead(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawOpenReadAt(path: String, offset: Long): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawOpenWrite(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawOpenAppend(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawExists(path: String): Boolean
    fun rawMetadata(path: String): Union2<salvo.std.fs.FileInfo, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawListDir(path: String): Union2<List<String>, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawCreateDirs(path: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawDelete(path: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawRenamePath(from: String, to: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
}

class __Mon_RawFs(
    private val inner: RawFs,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : RawFs {
    override fun rawOpenRead(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawOpenRead(path) } finally { lock.unlock() }
    }
    override fun rawOpenReadAt(path: String, offset: Long): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawOpenReadAt(path, offset) } finally { lock.unlock() }
    }
    override fun rawOpenWrite(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawOpenWrite(path) } finally { lock.unlock() }
    }
    override fun rawOpenAppend(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawOpenAppend(path) } finally { lock.unlock() }
    }
    override fun rawExists(path: String): Boolean {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawExists(path) } finally { lock.unlock() }
    }
    override fun rawMetadata(path: String): Union2<salvo.std.fs.FileInfo, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawMetadata(path) } finally { lock.unlock() }
    }
    override fun rawListDir(path: String): Union2<List<String>, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawListDir(path) } finally { lock.unlock() }
    }
    override fun rawCreateDirs(path: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawCreateDirs(path) } finally { lock.unlock() }
    }
    override fun rawDelete(path: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawDelete(path) } finally { lock.unlock() }
    }
    override fun rawRenamePath(from: String, to: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.rawRenamePath(from, to) } finally { lock.unlock() }
    }
}

interface RawFsPlatform {
    fun rawOpenRead(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawOpenReadAt(path: String, offset: Long): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawOpenWrite(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawOpenAppend(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawExists(path: String): Boolean
    fun rawMetadata(path: String): Union2<salvo.std.fs.FileInfo, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawListDir(path: String): Union2<List<String>, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawCreateDirs(path: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawDelete(path: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
    fun rawRenamePath(from: String, to: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>
}

open class __Platform_RawFs(private val impl: RawFsPlatform) : RawFs {
    override fun rawOpenRead(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = impl.rawOpenRead(path)
    override fun rawOpenReadAt(path: String, offset: Long): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = impl.rawOpenReadAt(path, offset)
    override fun rawOpenWrite(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = impl.rawOpenWrite(path)
    override fun rawOpenAppend(path: String): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = impl.rawOpenAppend(path)
    override fun rawExists(path: String): Boolean = impl.rawExists(path)
    override fun rawMetadata(path: String): Union2<salvo.std.fs.FileInfo, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = impl.rawMetadata(path)
    override fun rawListDir(path: String): Union2<List<String>, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = impl.rawListDir(path)
    override fun rawCreateDirs(path: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = impl.rawCreateDirs(path)
    override fun rawDelete(path: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = impl.rawDelete(path)
    override fun rawRenamePath(from: String, to: String): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = impl.rawRenamePath(from, to)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawOpenRead {
    fun ok(value: Long): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawOpenReadAt {
    fun ok(value: Long): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawOpenWrite {
    fun ok(value: Long): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawOpenAppend {
    fun ok(value: Long): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawMetadata {
    fun ok(value: salvo.std.fs.FileInfo): Union2<salvo.std.fs.FileInfo, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): Union2<salvo.std.fs.FileInfo, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawListDir {
    fun ok(value: List<String>): Union2<List<String>, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): Union2<List<String>, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawCreateDirs {
    fun ok(value: Unit): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawDelete {
    fun ok(value: Unit): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawRenamePath {
    fun ok(value: Unit): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = salvo.Union2.U2(value)
}


class __Platform_HostRawFs() : __Platform_RawFs(salvo.platform.std.fs.host.HostRawFs())

class DefaultFs(private val __dep0: RawFs, private val __dep1: salvo.std.stream.Streams) : salvo.std.fs.Fs {
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun openRead(path: salvo.std.fs.path.Path): Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val r: Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = __dep0.rawOpenRead(pathText)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Long = ((r as Union2.U1<*, *>).value as Long)
                return Union2.U1<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.stream.InStream(handle = r_1)))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming> = ((r as Union2.U2<*, *>).value as Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>)
                return Union2.U2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(r_2)))
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.host:63:9"))
            }
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun openReadAt(path: salvo.std.fs.path.Path, offset: Long): Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val r: Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = __dep0.rawOpenReadAt(pathText, offset)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Long = ((r as Union2.U1<*, *>).value as Long)
                return Union2.U1<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.stream.InStream(handle = r_1)))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming> = ((r as Union2.U2<*, *>).value as Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>)
                return Union2.U2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(r_2)))
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.host:72:9"))
            }
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun openWrite(path: salvo.std.fs.path.Path): Union2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val r: Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = __dep0.rawOpenWrite(pathText)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Long = ((r as Union2.U1<*, *>).value as Long)
                return Union2.U1<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.stream.OutStream(handle = r_1)))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming> = ((r as Union2.U2<*, *>).value as Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>)
                return Union2.U2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(r_2)))
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.host:81:9"))
            }
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun openAppend(path: salvo.std.fs.path.Path): Union2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val r: Union2<Long, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = __dep0.rawOpenAppend(pathText)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Long = ((r as Union2.U1<*, *>).value as Long)
                return Union2.U1<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.stream.OutStream(handle = r_1)))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming> = ((r as Union2.U2<*, *>).value as Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>)
                return Union2.U2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(r_2)))
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.host:90:9"))
            }
        }
    }
    override fun exists(path: salvo.std.fs.path.Path): Boolean {
        val pathText: String = salvo.std.fs.path.toStr(path)
        return __dep0.rawExists(pathText)
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun metadata(path: salvo.std.fs.path.Path): Union2<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val r: Union2<salvo.std.fs.FileInfo, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = __dep0.rawMetadata(pathText)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: salvo.std.fs.FileInfo = ((r as Union2.U1<*, *>).value as salvo.std.fs.FileInfo)
                return Union2.U1<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(r_1))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming> = ((r as Union2.U2<*, *>).value as Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>)
                return Union2.U2<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(r_2)))
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.host:104:9"))
            }
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun listDir(path: salvo.std.fs.path.Path): Union2<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val r: Union2<List<String>, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = __dep0.rawListDir(pathText)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: List<String> = ((r as Union2.U1<*, *>).value as List<String>)
                return Union2.U1<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(r_1))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming> = ((r as Union2.U2<*, *>).value as Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>)
                return Union2.U2<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(r_2)))
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.host:113:9"))
            }
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun createDirs(path: salvo.std.fs.path.Path): Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val r: Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = __dep0.rawCreateDirs(pathText)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Unit = ((r as Union2.U1<*, *>).value as Unit)
                return Union2.U1<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(Unit))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming> = ((r as Union2.U2<*, *>).value as Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>)
                return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(r_2)))
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.host:122:9"))
            }
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun delete(path: salvo.std.fs.path.Path): Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val r: Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = __dep0.rawDelete(pathText)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Unit = ((r as Union2.U1<*, *>).value as Unit)
                return Union2.U1<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(Unit))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming> = ((r as Union2.U2<*, *>).value as Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>)
                return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(r_2)))
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.host:131:9"))
            }
        }
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun renamePath(from: salvo.std.fs.path.Path, to: salvo.std.fs.path.Path): Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val fromText: String = salvo.std.fs.path.toStr(from)
        val toText: String = salvo.std.fs.path.toStr(to)
        val r: Union2<Unit, Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = __dep0.rawRenamePath(fromText, toText)
        return when {
            (r is Union2.U1<*, *>) -> {
                val r_1: Unit = ((r as Union2.U1<*, *>).value as Unit)
                return Union2.U1<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(Unit))
            }
            (r is Union2.U2<*, *>) -> {
                val r_2: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming> = ((r as Union2.U2<*, *>).value as Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>)
                return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(r_2)))
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.host:141:9"))
            }
        }
    }
}

