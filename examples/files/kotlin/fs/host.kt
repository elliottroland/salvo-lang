package salvo.fs.host

import salvo.*
import salvo.core.checked.*
import salvo.core.list.*
import salvo.core.result.*
import salvo.core.string.*
import salvo.fs.*
import salvo.stream.*

interface RawFs {
    fun raw_open_read(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun raw_open_read_at(path: String, offset: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun raw_open_write(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun raw_open_append(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun raw_exists(path: String): Boolean
    fun raw_metadata(path: String): Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun raw_list_dir(path: String): Union2<List<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun raw_create_dirs(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun raw_delete(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
    fun raw_rename_path(from: String, to: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>
}

class __Mon_RawFs(private val inner: RawFs) : RawFs {
    override fun raw_open_read(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.raw_open_read(path) }
    override fun raw_open_read_at(path: String, offset: Long): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.raw_open_read_at(path, offset) }
    override fun raw_open_write(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.raw_open_write(path) }
    override fun raw_open_append(path: String): Union2<Long, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.raw_open_append(path) }
    override fun raw_exists(path: String): Boolean =
        synchronized(inner) { inner.raw_exists(path) }
    override fun raw_metadata(path: String): Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.raw_metadata(path) }
    override fun raw_list_dir(path: String): Union2<List<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.raw_list_dir(path) }
    override fun raw_create_dirs(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.raw_create_dirs(path) }
    override fun raw_delete(path: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.raw_delete(path) }
    override fun raw_rename_path(from: String, to: String): Union2<Unit, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> =
        synchronized(inner) { inner.raw_rename_path(from, to) }
}

class DefaultFs(private val __dep_RawFs: RawFs, private val __dep_Streams: Streams) : Fs {

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun open_read(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.raw_open_read(path)
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
    override fun open_read_at(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.raw_open_read_at(path, offset)
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
    override fun open_write(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.raw_open_write(path)
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
    override fun open_append(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.raw_open_append(path)
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
        return __dep_RawFs.raw_exists(path)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.raw_metadata(path)
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
    override fun list_dir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.raw_list_dir(path)
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
    override fun create_dirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.raw_create_dirs(path)
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
        val r = __dep_RawFs.raw_delete(path)
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
    override fun rename_path(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val r = __dep_RawFs.raw_rename_path(from, to)
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
