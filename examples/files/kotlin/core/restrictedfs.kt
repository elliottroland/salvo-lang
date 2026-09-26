package salvo.core.restrictedfs

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.fs.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.nonempty.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

fun fs_resolve(root: String, path: String): String? {
    if (path.startsWith("/")) {
        return null
    }
    val segs = path.split("/")
    val kept: MutableList<String> = mutableListOf<String>()
    var skip = 0
    var i = segs.size - 1
    while (i >= 0) {
        val seg = (segs.getOrNull(i) ?: throw AssertionError("salvo: value is absent at core.restrictedfs:32:19"))
        if (seg == "..") {
            skip = skip + 1
        } else {
            if (seg.length == 0 || seg == ".") {
            } else {
                if (skip > 0) {
                    skip = skip - 1
                } else {
                    kept.add(seg)
                }
            }
        }
        i = i - 1
    }
    if (skip > 0) {
        return null
    }
    val parts: MutableList<String> = mutableListOf<String>()
    var j = kept.size - 1
    while (j >= 0) {
        parts.add((kept.getOrNull(j) ?: throw AssertionError("salvo: value is absent at core.restrictedfs:55:24")))
        j = j - 1
    }
    val rel = parts.joinToString("/")
    if (rel.length == 0) {
        return root
    }
    return "$root/$rel"
}

fun fs_escaped(path: String): Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
    return checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>(U8_5<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>(PathEscapes(path = path)))
}

class RestrictedFs(private val root: String, private val __dep_Fs: __Has_Fs) : Fs {

    override fun open_read(path: String): Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val real = fs_resolve(root, path)
        if (real == null) {
            return U2_2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(path)))
        }
        return __dep_Fs.__fx_Fs.open_read(real)
    }

    override fun open_read_at(path: String, offset: Long): Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val real = fs_resolve(root, path)
        if (real == null) {
            return U2_2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(path)))
        }
        return __dep_Fs.__fx_Fs.open_read_at(real, offset)
    }

    override fun open_write(path: String): Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val real = fs_resolve(root, path)
        if (real == null) {
            return U2_2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(path)))
        }
        return __dep_Fs.__fx_Fs.open_write(real)
    }

    override fun open_append(path: String): Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val real = fs_resolve(root, path)
        if (real == null) {
            return U2_2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(path)))
        }
        return __dep_Fs.__fx_Fs.open_append(real)
    }

    override fun exists(path: String): Boolean {
        val real = fs_resolve(root, path)
        if (real == null) {
            return false
        }
        return __dep_Fs.__fx_Fs.exists(real)
    }

    override fun metadata(path: String): Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val real = fs_resolve(root, path)
        if (real == null) {
            return U2_2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(path)))
        }
        return __dep_Fs.__fx_Fs.metadata(real)
    }

    override fun list_dir(path: String): Union2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val real = fs_resolve(root, path)
        if (real == null) {
            return U2_2<List<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(path)))
        }
        return __dep_Fs.__fx_Fs.list_dir(real)
    }

    override fun create_dirs(path: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val real = fs_resolve(root, path)
        if (real == null) {
            return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(path)))
        }
        return __dep_Fs.__fx_Fs.create_dirs(real)
    }

    override fun delete(path: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val real = fs_resolve(root, path)
        if (real == null) {
            return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(path)))
        }
        return __dep_Fs.__fx_Fs.delete(real)
    }

    override fun rename_path(from: String, to: String): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        val real_from = fs_resolve(root, from)
        if (real_from == null) {
            return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(from)))
        }
        val real_to = fs_resolve(root, to)
        if (real_to == null) {
            return U2_2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>(err(fs_escaped(to)))
        }
        return __dep_Fs.__fx_Fs.rename_path(real_from, real_to)
    }

    override fun read_line(s: InStream): String? {
        return __dep_Fs.__fx_Fs.read_line(s)
    }

    override fun read_all(s: InStream): Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __dep_Fs.__fx_Fs.read_all(s)
    }

    override fun read_bytes(s: InStream, max: Int): Union2<salvo.SalvoBytes, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __dep_Fs.__fx_Fs.read_bytes(s, max)
    }

    override fun read_to(s: InStream, buf: salvo.SalvoBytes, max: Int): Union2<Int, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __dep_Fs.__fx_Fs.read_to(s, buf, max)
    }

    override fun read_to__2(s: InStream, buf: StringBuilder): Union2<Long, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __dep_Fs.__fx_Fs.read_to__2(s, buf)
    }

    override fun read_line_to(s: InStream, buf: StringBuilder): Boolean {
        return __dep_Fs.__fx_Fs.read_line_to(s, buf)
    }

    override fun position(s: InStream): Long {
        return __dep_Fs.__fx_Fs.position(s)
    }

    override fun close(s: InStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __dep_Fs.__fx_Fs.close(s)
    }

    override fun write(s: OutStream, text: String): Long {
        return __dep_Fs.__fx_Fs.write(s, text)
    }

    override fun write_line(s: OutStream, text: String): Long {
        return __dep_Fs.__fx_Fs.write_line(s, text)
    }

    override fun write_bytes(s: OutStream, data: salvo.SalvoBytes): Long {
        return __dep_Fs.__fx_Fs.write_bytes(s, data)
    }

    override fun position__2(s: OutStream): Long {
        return __dep_Fs.__fx_Fs.position__2(s)
    }

    override fun flush(s: OutStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __dep_Fs.__fx_Fs.flush(s)
    }

    override fun close__2(s: OutStream): Union2<Unit, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __dep_Fs.__fx_Fs.close__2(s)
    }
}
