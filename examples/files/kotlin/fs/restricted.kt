package salvo.fs.restricted

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.deque.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.fs.*
import salvo.stream.*

fun fsResolve(root: String, path: String): String? {
    if (path.startsWith("/")) {
        return null
    }
    val segs = path.split("/")
    val kept: MutableList<String> = mutableListOf<String>()
    var skip = 0
    var i = segs.size - 1
    while (i >= 0) {
        val seg = (segs.getOrNull(i) ?: throw AssertionError("salvo: value is absent at fs.restricted:35:19"))
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
        parts.add((kept.getOrNull(j) ?: throw AssertionError("salvo: value is absent at fs.restricted:58:24")))
        j = j - 1
    }
    val rel = parts.joinToString("/")
    if (rel.length == 0) {
        return root
    }
    return "$root/$rel"
}

fun fsEscaped(path: String): Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
    return checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U5<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(PathEscapes(path = path)))
}

class RestrictedFs(private val root: String, private val __dep_Fs: Fs, private val __dep_Streams: Streams) : Fs {

    override fun openRead(path: String): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val real = fsResolve(root, path)
        if (real == null) {
            return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(path)))
        }
        return __dep_Fs.openRead(real)
    }

    override fun openReadAt(path: String, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val real = fsResolve(root, path)
        if (real == null) {
            return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(path)))
        }
        return __dep_Fs.openReadAt(real, offset)
    }

    override fun openWrite(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val real = fsResolve(root, path)
        if (real == null) {
            return Union2.U2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(path)))
        }
        return __dep_Fs.openWrite(real)
    }

    override fun openAppend(path: String): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val real = fsResolve(root, path)
        if (real == null) {
            return Union2.U2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(path)))
        }
        return __dep_Fs.openAppend(real)
    }

    override fun exists(path: String): Boolean {
        val real = fsResolve(root, path)
        if (real == null) {
            return false
        }
        return __dep_Fs.exists(real)
    }

    override fun metadata(path: String): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val real = fsResolve(root, path)
        if (real == null) {
            return Union2.U2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(path)))
        }
        return __dep_Fs.metadata(real)
    }

    override fun listDir(path: String): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val real = fsResolve(root, path)
        if (real == null) {
            return Union2.U2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(path)))
        }
        return __dep_Fs.listDir(real)
    }

    override fun createDirs(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val real = fsResolve(root, path)
        if (real == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(path)))
        }
        return __dep_Fs.createDirs(real)
    }

    override fun delete(path: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val real = fsResolve(root, path)
        if (real == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(path)))
        }
        return __dep_Fs.delete(real)
    }

    override fun renamePath(from: String, to: String): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val realFrom = fsResolve(root, from)
        if (realFrom == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(from)))
        }
        val realTo = fsResolve(root, to)
        if (realTo == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(to)))
        }
        return __dep_Fs.renamePath(realFrom, realTo)
    }
}
