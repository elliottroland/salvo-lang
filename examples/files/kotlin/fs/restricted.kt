package salvo.fs.restricted

import salvo.*
import salvo.core.checked.Checked
import salvo.core.checked.checked
import salvo.core.list.addPlatform
import salvo.core.list.getPlatform
import salvo.core.list.sizePlatform as sizePlatform__core_list
import salvo.core.result.err
import salvo.core.string.joinPlatform
import salvo.core.string.sizePlatform as sizePlatform__core_string
import salvo.core.string.splitPlatform
import salvo.core.string.startsWithPlatform
import salvo.fs.AlreadyExists
import salvo.fs.FileInfo
import salvo.fs.Fs
import salvo.fs.IoError
import salvo.fs.NotADirectory
import salvo.fs.NotFound
import salvo.fs.PathEscapes
import salvo.fs.PermissionDenied
import salvo.fs.Streaming
import salvo.fs.createDirs
import salvo.fs.delete
import salvo.fs.exists
import salvo.fs.listDir
import salvo.fs.metadata
import salvo.fs.openAppend
import salvo.fs.openRead
import salvo.fs.openReadAt
import salvo.fs.openWrite
import salvo.fs.renamePath
import salvo.stream.InStream
import salvo.stream.OutStream

fun fsResolve(root: String, path: String): String? {
    if (startsWithPlatform(path, "/")) {
        return null
    }
    val segs = splitPlatform(path, "/")
    val kept: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    var skip = 0
    var i = sizePlatform__core_list(segs) - 1
    while (i >= 0) {
        val seg = (getPlatform(segs, i) ?: throw AssertionError("salvo: value is absent at fs.restricted:35:19"))
        if (seg == "..") {
            skip = skip + 1
        } else {
            if (sizePlatform__core_string(seg) == 0 || seg == ".") {
            } else {
                if (skip > 0) {
                    skip = skip - 1
                } else {
                    addPlatform(kept, seg)
                }
            }
        }
        i = i - 1
    }
    if (skip > 0) {
        return null
    }
    val parts: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    var j = sizePlatform__core_list(kept) - 1
    while (j >= 0) {
        addPlatform(parts, (getPlatform(kept, j) ?: throw AssertionError("salvo: value is absent at fs.restricted:58:24")))
        j = j - 1
    }
    val rel = joinPlatform(parts, "/")
    if (sizePlatform__core_string(rel) == 0) {
        return root
    }
    return "$root/$rel"
}

fun fsEscaped(path: String): Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
    return checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>(Union7.U5<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>(PathEscapes(path = path)))
}

class RestrictedFs(private val root: String, private val __dep_Fs: Fs, private val __dep_salvo_stream_Streams: salvo.stream.Streams) : Fs {

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
