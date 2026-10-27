package salvo.std.fs.restricted

import salvo.*

fun fsResolve(root: String, path: String): String? {
    if (salvo.std.core.string.startsWithPlatform(path, "/")) {
        return null
    }
    val segs: salvo.platform.std.core.list.MutList<String> = salvo.std.core.string.splitPlatform(path, "/")
    val kept: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    var skip: Int = 0
    var i: Int = (salvo.std.core.list.sizePlatform(segs) - 1)
    while (true) {
        if (!((i >= 0))) {
            break
        }
        val seg: String = kotlin.run {
            val __pick_1: String? = salvo.std.core.list.getPlatform(segs, i)
            (if ((__pick_1 != null)) {
                val __picked_2: String = __pick_1!!
                __picked_2
            } else {
                throw AssertionError("salvo: unreachable at std.fs.restricted:36:35")
            })
        }
        if (((seg) == (".."))) {
            skip = (skip + 1)
        } else {
            if (((salvo.std.core.string.sizePlatform(seg) == 0) || ((seg) == (".")))) {
            } else {
                if ((skip > 0)) {
                    skip = (skip - 1)
                } else {
                    salvo.std.core.list.addPlatform(kept, seg)
                }
            }
        }
        i = (i - 1)
    }
    if ((skip > 0)) {
        return null
    }
    val parts: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    var j: Int = (salvo.std.core.list.sizePlatform(kept) - 1)
    while (true) {
        if (!((j >= 0))) {
            break
        }
        salvo.std.core.list.addPlatform(parts, kotlin.run {
            val __pick_4: String? = salvo.std.core.list.getPlatform(kept, j)
            (if ((__pick_4 != null)) {
                val __picked_5: String = __pick_4!!
                __picked_5
            } else {
                throw AssertionError("salvo: unreachable at std.fs.restricted:59:40")
            })
        })
        j = (j - 1)
    }
    val rel: String = salvo.std.core.string.joinPlatform(parts, "/")
    if ((salvo.std.core.string.sizePlatform(rel) == 0)) {
        return root
    }
    return "${root}/${rel}"
}

fun fsEscaped(path: String): salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> {
    return salvo.std.core.checked.checked(Union7.U5<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>(salvo.std.fs.PathEscapes(path = path)))
}

class RestrictedFs(private val root: salvo.std.fs.path.Path, private val __dep0: salvo.std.fs.Fs, private val __dep1: salvo.std.stream.Streams) : salvo.std.fs.Fs {
    override fun openRead(path: salvo.std.fs.path.Path): Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val real: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.openRead(salvo.std.fs.path.Path(value = real_1))
    }
    override fun openReadAt(path: salvo.std.fs.path.Path, offset: Long): Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val real: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.openReadAt(salvo.std.fs.path.Path(value = real_1), offset)
    }
    override fun openWrite(path: salvo.std.fs.path.Path): Union2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val real: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.openWrite(salvo.std.fs.path.Path(value = real_1))
    }
    override fun openAppend(path: salvo.std.fs.path.Path): Union2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val real: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.openAppend(salvo.std.fs.path.Path(value = real_1))
    }
    override fun exists(path: salvo.std.fs.path.Path): Boolean {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val real: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return false
        }
        val real_1: String = real!!
        return __dep0.exists(salvo.std.fs.path.Path(value = real_1))
    }
    override fun metadata(path: salvo.std.fs.path.Path): Union2<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val real: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.metadata(salvo.std.fs.path.Path(value = real_1))
    }
    override fun listDir(path: salvo.std.fs.path.Path): Union2<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val real: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.listDir(salvo.std.fs.path.Path(value = real_1))
    }
    override fun createDirs(path: salvo.std.fs.path.Path): Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val real: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.createDirs(salvo.std.fs.path.Path(value = real_1))
    }
    override fun delete(path: salvo.std.fs.path.Path): Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val real: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.delete(salvo.std.fs.path.Path(value = real_1))
    }
    override fun renamePath(from: salvo.std.fs.path.Path, to: salvo.std.fs.path.Path): Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val fromText: String = salvo.std.fs.path.toStr(from)
        val toText: String = salvo.std.fs.path.toStr(to)
        val realFrom: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), fromText)
        if ((realFrom == null)) {
            return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(fromText)))
        }
        val realTo: String? = salvo.std.fs.restricted.fsResolve(salvo.std.fs.path.toStr(root), toText)
        if ((realTo == null)) {
            return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.fs.restricted.fsEscaped(toText)))
        }
        val realFrom_1: String = realFrom!!
        val realTo_2: String = realTo!!
        return __dep0.renamePath(salvo.std.fs.path.Path(value = realFrom_1), salvo.std.fs.path.Path(value = realTo_2))
    }
}

