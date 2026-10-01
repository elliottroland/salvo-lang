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

class __Mon_RawFs(private val inner: RawFs) : RawFs {
    override fun rawOpenRead(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.rawOpenRead(path) }
    override fun rawOpenReadAt(path: String, offset: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.rawOpenReadAt(path, offset) }
    override fun rawOpenWrite(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.rawOpenWrite(path) }
    override fun rawOpenAppend(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.rawOpenAppend(path) }
    override fun rawExists(path: String): Boolean =
        synchronized(inner) { inner.rawExists(path) }
    override fun rawMetadata(path: String): Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.rawMetadata(path) }
    override fun rawListDir(path: String): Union2<List<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.rawListDir(path) }
    override fun rawCreateDirs(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.rawCreateDirs(path) }
    override fun rawDelete(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.rawDelete(path) }
    override fun rawRenamePath(from: String, to: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.rawRenamePath(from, to) }
}

class DefaultFs(private val __dep_RawFs: RawFs, private val __dep_Streams: Streams) : Fs {

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun openRead(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawOpenRead(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = (r.value as Long))))
            }
            is U2_2<*, *> -> {
                return U2_2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun openReadAt(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawOpenReadAt(path, offset)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(InStream(handle = (r.value as Long))))
            }
            is U2_2<*, *> -> {
                return U2_2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun openWrite(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawOpenWrite(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = (r.value as Long))))
            }
            is U2_2<*, *> -> {
                return U2_2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun openAppend(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawOpenAppend(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(OutStream(handle = (r.value as Long))))
            }
            is U2_2<*, *> -> {
                return U2_2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    override fun exists(path: String): Boolean {
        return __dep_RawFs.rawExists(path)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawMetadata(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok((r.value as FileInfo)))
            }
            is U2_2<*, *> -> {
                return U2_2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun listDir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawListDir(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok((r.value as List<String>)))
            }
            is U2_2<*, *> -> {
                return U2_2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun createDirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawCreateDirs(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun delete(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawDelete(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun renamePath(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.rawRenamePath(from, to)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>((r.value as Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>))))
            }
        }
    }
}
