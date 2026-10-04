package salvo.fs.host

import salvo.*
import salvo.core.checked.*
import salvo.core.list.*
import salvo.core.result.*
import salvo.core.string.*
import salvo.fs.*
import salvo.stream.*

interface RawFs {
    fun rawOpenRead(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawOpenReadAt(path: String, offset: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawOpenWrite(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawOpenAppend(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawExists(path: String): Boolean
    fun rawMetadata(path: String): Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawListDir(path: String): Union2<List<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawCreateDirs(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawDelete(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawRenamePath(from: String, to: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
}

class __Mon_RawFs(
    private val inner: RawFs,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : RawFs {
    override fun rawOpenRead(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawOpenRead(path) } finally { lock.unlock() }
    }
    override fun rawOpenReadAt(path: String, offset: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawOpenReadAt(path, offset) } finally { lock.unlock() }
    }
    override fun rawOpenWrite(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawOpenWrite(path) } finally { lock.unlock() }
    }
    override fun rawOpenAppend(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawOpenAppend(path) } finally { lock.unlock() }
    }
    override fun rawExists(path: String): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawExists(path) } finally { lock.unlock() }
    }
    override fun rawMetadata(path: String): Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawMetadata(path) } finally { lock.unlock() }
    }
    override fun rawListDir(path: String): Union2<List<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawListDir(path) } finally { lock.unlock() }
    }
    override fun rawCreateDirs(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawCreateDirs(path) } finally { lock.unlock() }
    }
    override fun rawDelete(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawDelete(path) } finally { lock.unlock() }
    }
    override fun rawRenamePath(from: String, to: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.rawRenamePath(from, to) } finally { lock.unlock() }
    }
}

// The interface a `platform handler` of `RawFs` implements [platform-abi].
interface RawFsPlatform {
    fun rawOpenRead(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawOpenReadAt(path: String, offset: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawOpenWrite(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawOpenAppend(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawExists(path: String): Boolean
    fun rawMetadata(path: String): Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawListDir(path: String): Union2<List<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawCreateDirs(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawDelete(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun rawRenamePath(from: String, to: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
}

open class __Platform_RawFs(private val impl: RawFsPlatform) : RawFs {
    override fun rawOpenRead(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = impl.rawOpenRead(path)
    override fun rawOpenReadAt(path: String, offset: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = impl.rawOpenReadAt(path, offset)
    override fun rawOpenWrite(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = impl.rawOpenWrite(path)
    override fun rawOpenAppend(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = impl.rawOpenAppend(path)
    override fun rawExists(path: String): Boolean = impl.rawExists(path)
    override fun rawMetadata(path: String): Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = impl.rawMetadata(path)
    override fun rawListDir(path: String): Union2<List<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = impl.rawListDir(path)
    override fun rawCreateDirs(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = impl.rawCreateDirs(path)
    override fun rawDelete(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = impl.rawDelete(path)
    override fun rawRenamePath(from: String, to: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = impl.rawRenamePath(from, to)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawOpenRead {
    fun ok(value: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawOpenReadAt {
    fun ok(value: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawOpenWrite {
    fun ok(value: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawOpenAppend {
    fun ok(value: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawMetadata {
    fun ok(value: FileInfo): Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawListDir {
    fun ok(value: List<String>): Union2<List<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): Union2<List<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawCreateDirs {
    fun ok(value: Unit): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawDelete {
    fun ok(value: Unit): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object RawRenamePath {
    fun ok(value: Unit): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U1(value)
    fun err(value: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = salvo.Union2.U2(value)
}

class __Platform_HostRawFs() : salvo.fs.host.__Platform_RawFs(salvo.platform.fs.host.HostRawFs())

class DefaultFs(private val __dep_RawFs: RawFs, private val __dep_salvo_stream_Streams: salvo.stream.Streams) : Fs {

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun openRead(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawOpenRead(path)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = (r.value as Long))))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun openReadAt(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawOpenReadAt(path, offset)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = (r.value as Long))))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun openWrite(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawOpenWrite(path)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = (r.value as Long))))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun openAppend(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawOpenAppend(path)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = (r.value as Long))))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    override fun exists(path: String): Boolean {
        return __dep_RawFs.rawExists(path)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawMetadata(path)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok((r.value as FileInfo)))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun listDir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawListDir(path)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok((r.value as List<String>)))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun createDirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawCreateDirs(path)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun delete(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawDelete(path)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun renamePath(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawRenamePath(from, to)
        when (r) {
            is Union2.U1<*, *> -> {
                return Union2.U1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
            }
            is Union2.U2<*, *> -> {
                return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }
}
