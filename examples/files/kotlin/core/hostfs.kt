package salvo.core.hostfs

import salvo.*
import salvo.core.bytes.*
import salvo.core.fs.*
import salvo.core.list.*
import salvo.core.nonempty.*
import salvo.core.result.*
import salvo.core.sorted.*
import salvo.core.string.*

interface RawFs {
    fun raw_open_read(path: String): Union2<Long, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_open_read_at(path: String, offset: Long): Union2<Long, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_open_write(path: String): Union2<Long, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_open_append(path: String): Union2<Long, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_exists(path: String): Boolean
    fun raw_metadata(path: String): Union2<FileInfo, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_list_dir(path: String): Union2<List<String>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_create_dirs(path: String): Union2<Unit, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_delete(path: String): Union2<Unit, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_rename_path(from: String, to: String): Union2<Unit, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_read_line(handle: Long): String?
    fun raw_read_all(handle: Long): Union2<String, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_read_bytes(handle: Long, max: Int): Union2<salvo.SalvoBytes, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_read_to_bytes(handle: Long, buf: salvo.SalvoBytes, max: Int): Union2<Int, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_read_to_str(handle: Long, buf: StringBuilder): Union2<Long, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_read_line_to_str(handle: Long, buf: StringBuilder): Boolean
    fun raw_read_position(handle: Long): Long
    fun raw_close_read(handle: Long): Union2<Unit, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_write(handle: Long, text: String): Long
    fun raw_write_bytes(handle: Long, data: salvo.SalvoBytes): Long
    fun raw_write_position(handle: Long): Long
    fun raw_flush(handle: Long): Union2<Unit, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
    fun raw_close_write(handle: Long): Union2<Unit, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>
}

class DefaultFs<__Fx>(private val __fx: __Fx) : Fs where __Fx : __Has_RawFs {

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun open_read(path: String): Union2<InStream, FsError> {
        val r = __fx.__fx_RawFs.raw_open_read(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<InStream, FsError>(ok(InStream(handle = (r.value as Long))))
            }
            is U2_2<*, *> -> {
                return U2_2<InStream, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun open_read_at(path: String, offset: Long): Union2<InStream, FsError> {
        val r = __fx.__fx_RawFs.raw_open_read_at(path, offset)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<InStream, FsError>(ok(InStream(handle = (r.value as Long))))
            }
            is U2_2<*, *> -> {
                return U2_2<InStream, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun open_write(path: String): Union2<OutStream, FsError> {
        val r = __fx.__fx_RawFs.raw_open_write(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<OutStream, FsError>(ok(OutStream(handle = (r.value as Long))))
            }
            is U2_2<*, *> -> {
                return U2_2<OutStream, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun open_append(path: String): Union2<OutStream, FsError> {
        val r = __fx.__fx_RawFs.raw_open_append(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<OutStream, FsError>(ok(OutStream(handle = (r.value as Long))))
            }
            is U2_2<*, *> -> {
                return U2_2<OutStream, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    override fun exists(path: String): Boolean {
        return __fx.__fx_RawFs.raw_exists(path)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun metadata(path: String): Union2<FileInfo, FsError> {
        val r = __fx.__fx_RawFs.raw_metadata(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<FileInfo, FsError>(ok((r.value as FileInfo)))
            }
            is U2_2<*, *> -> {
                return U2_2<FileInfo, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun list_dir(path: String): Union2<List<String>, FsError> {
        val r = __fx.__fx_RawFs.raw_list_dir(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<List<String>, FsError>(ok((r.value as List<String>)))
            }
            is U2_2<*, *> -> {
                return U2_2<List<String>, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun create_dirs(path: String): Union2<Unit, FsError> {
        val r = __fx.__fx_RawFs.raw_create_dirs(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, FsError>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun delete(path: String): Union2<Unit, FsError> {
        val r = __fx.__fx_RawFs.raw_delete(path)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, FsError>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun rename_path(from: String, to: String): Union2<Unit, FsError> {
        val r = __fx.__fx_RawFs.raw_rename_path(from, to)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, FsError>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    override fun read_line(s: InStream): String? {
        return __fx.__fx_RawFs.raw_read_line(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_all(s: InStream): Union2<String, FsError> {
        val r = __fx.__fx_RawFs.raw_read_all(s.handle)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<String, FsError>(ok((r.value as String)))
            }
            is U2_2<*, *> -> {
                return U2_2<String, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, FsError> {
        val r = __fx.__fx_RawFs.raw_read_bytes(s.handle, max)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<salvo.SalvoBytes, FsError>(ok((r.value as salvo.SalvoBytes)))
            }
            is U2_2<*, *> -> {
                return U2_2<salvo.SalvoBytes, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, FsError> {
        val r = __fx.__fx_RawFs.raw_read_to_bytes(s.handle, buf, max)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Int, FsError>(ok((r.value as Int)))
            }
            is U2_2<*, *> -> {
                return U2_2<Int, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, FsError> {
        val r = __fx.__fx_RawFs.raw_read_to_str(s.handle, buf)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Long, FsError>(ok((r.value as Long)))
            }
            is U2_2<*, *> -> {
                return U2_2<Long, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    override fun read_line_to(s: InStream, buf: StringBuilder): Boolean {
        return __fx.__fx_RawFs.raw_read_line_to_str(s.handle, buf)
    }

    override fun position(s: InStream): Long {
        return __fx.__fx_RawFs.raw_read_position(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun close(s: InStream): Union2<Unit, FsError> {
        val r = __fx.__fx_RawFs.raw_close_read(s.handle)
        (s).let {}
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, FsError>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    override fun write(s: OutStream, text: String): Long {
        return __fx.__fx_RawFs.raw_write(s.handle, text)
    }

    override fun write_line(s: OutStream, text: String): Long {
        return __fx.__fx_RawFs.raw_write(s.handle, "$text\n")
    }

    override fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long {
        return __fx.__fx_RawFs.raw_write_bytes(s.handle, data)
    }

    override fun position__2(s: OutStream): Long {
        return __fx.__fx_RawFs.raw_write_position(s.handle)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun flush(s: OutStream): Union2<Unit, FsError> {
        val r = __fx.__fx_RawFs.raw_flush(s.handle)
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, FsError>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun close__2(s: OutStream): Union2<Unit, FsError> {
        val r = __fx.__fx_RawFs.raw_close_write(s.handle)
        (s).let {}
        when (r) {
            is U2_1<*, *> -> {
                return U2_1<Unit, FsError>(ok(Unit))
            }
            is U2_2<*, *> -> {
                return U2_2<Unit, FsError>(err(FsError(kind = (r.value as Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>))))
            }
        }
    }
}
