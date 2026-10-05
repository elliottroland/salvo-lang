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
import salvo.fs.path.Path
import salvo.fs.path.path
import salvo.fs.path.toStr
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
        val seg = (getPlatform(segs, i) ?: throw AssertionError("salvo: value is absent at fs.restricted:36:19"))
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
        addPlatform(parts, (getPlatform(kept, j) ?: throw AssertionError("salvo: value is absent at fs.restricted:59:24")))
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

class RestrictedFs(private val root: Path, private val __dep_Fs: Fs, private val __dep_salvo_stream_Streams: salvo.stream.Streams) : Fs {

    override fun openRead(path: Path): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val real = fsResolve(toStr(root), pathText)
        if (real == null) {
            return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(pathText)))
        }
        return __dep_Fs.openRead(Path(text = real))
    }

    override fun openReadAt(path: Path, offset: Long): Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val real = fsResolve(toStr(root), pathText)
        if (real == null) {
            return Union2.U2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(pathText)))
        }
        return __dep_Fs.openReadAt(Path(text = real), offset)
    }

    override fun openWrite(path: Path): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val real = fsResolve(toStr(root), pathText)
        if (real == null) {
            return Union2.U2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(pathText)))
        }
        return __dep_Fs.openWrite(Path(text = real))
    }

    override fun openAppend(path: Path): Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val real = fsResolve(toStr(root), pathText)
        if (real == null) {
            return Union2.U2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(pathText)))
        }
        return __dep_Fs.openAppend(Path(text = real))
    }

    override fun exists(path: Path): Boolean {
        val pathText = toStr(path)
        val real = fsResolve(toStr(root), pathText)
        if (real == null) {
            return false
        }
        return __dep_Fs.exists(Path(text = real))
    }

    override fun metadata(path: Path): Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val real = fsResolve(toStr(root), pathText)
        if (real == null) {
            return Union2.U2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(pathText)))
        }
        return __dep_Fs.metadata(Path(text = real))
    }

    override fun listDir(path: Path): Union2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val real = fsResolve(toStr(root), pathText)
        if (real == null) {
            return Union2.U2<List<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(pathText)))
        }
        return __dep_Fs.listDir(Path(text = real))
    }

    override fun createDirs(path: Path): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val real = fsResolve(toStr(root), pathText)
        if (real == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(pathText)))
        }
        return __dep_Fs.createDirs(Path(text = real))
    }

    override fun delete(path: Path): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val pathText = toStr(path)
        val real = fsResolve(toStr(root), pathText)
        if (real == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(pathText)))
        }
        return __dep_Fs.delete(Path(text = real))
    }

    override fun renamePath(from: Path, to: Path): Union2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        val fromText = toStr(from)
        val toText = toStr(to)
        val realFrom = fsResolve(toStr(root), fromText)
        if (realFrom == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(fromText)))
        }
        val realTo = fsResolve(toStr(root), toText)
        if (realTo == null) {
            return Union2.U2<Unit, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>(err(fsEscaped(toText)))
        }
        return __dep_Fs.renamePath(Path(text = realFrom), Path(text = realTo))
    }
}
