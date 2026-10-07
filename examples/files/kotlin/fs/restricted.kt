package salvo.fs.restricted

import salvo.*

fun fsResolve(root: String, path: String): String? {
    if (salvo.core.string.startsWithPlatform(path, "/")) {
        return null
    }
    val segs: salvo.platform.core.list.MutList<String> = salvo.core.string.splitPlatform(path, "/")
    val kept: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    var skip: Int = 0
    var i: Int = (salvo.core.list.sizePlatform(segs) - 1)
    while (true) {
        if (!((i >= 0))) {
            break
        }
        val seg: String = run {
            val __nn_1: String? = salvo.core.list.getPlatform(segs, i)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at fs.restricted:36:19"))
                }
                else -> {
                    val __some_2: String = __nn_1!!
                    __some_2
                }
            }
        }
        if (((seg) == (".."))) {
            skip = (skip + 1)
        } else {
            if ((((salvo.core.string.sizePlatform(seg)) == (0)) || ((seg) == (".")))) {
            } else {
                if ((skip > 0)) {
                    skip = (skip - 1)
                } else {
                    salvo.core.list.addPlatform(kept, seg)
                }
            }
        }
        i = (i - 1)
    }
    if ((skip > 0)) {
        return null
    }
    val parts: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    var j: Int = (salvo.core.list.sizePlatform(kept) - 1)
    while (true) {
        if (!((j >= 0))) {
            break
        }
        salvo.core.list.addPlatform(parts, run {
            val __nn_3: String? = salvo.core.list.getPlatform(kept, j)
            when {
                (__nn_3 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at fs.restricted:59:24"))
                }
                else -> {
                    val __some_4: String = __nn_3!!
                    __some_4
                }
            }
        })
        j = (j - 1)
    }
    val rel: String = salvo.core.string.joinPlatform(parts, "/")
    if (((salvo.core.string.sizePlatform(rel)) == (0))) {
        return root
    }
    return "${root}/${rel}"
}

fun fsEscaped(path: String): salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> {
    return salvo.core.checked.checked(Union7.U5<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>(salvo.fs.PathEscapes(path = path)))
}

class RestrictedFs(private val root: salvo.fs.path.Path, private val __dep0: salvo.fs.Fs, private val __dep1: salvo.stream.Streams) : salvo.fs.Fs {
    override fun openRead(path: salvo.fs.path.Path): Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val real: String? = fsResolve(salvo.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.openRead(salvo.fs.path.Path(text = real_1))
    }
    override fun openReadAt(path: salvo.fs.path.Path, offset: Long): Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val real: String? = fsResolve(salvo.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.openReadAt(salvo.fs.path.Path(text = real_1), offset)
    }
    override fun openWrite(path: salvo.fs.path.Path): Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val real: String? = fsResolve(salvo.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.openWrite(salvo.fs.path.Path(text = real_1))
    }
    override fun openAppend(path: salvo.fs.path.Path): Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val real: String? = fsResolve(salvo.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.openAppend(salvo.fs.path.Path(text = real_1))
    }
    override fun exists(path: salvo.fs.path.Path): Boolean {
        val pathText: String = salvo.fs.path.toStr(path)
        val real: String? = fsResolve(salvo.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return false
        }
        val real_1: String = real!!
        return __dep0.exists(salvo.fs.path.Path(text = real_1))
    }
    override fun metadata(path: salvo.fs.path.Path): Union2<salvo.fs.FileInfo, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val real: String? = fsResolve(salvo.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<salvo.fs.FileInfo, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.metadata(salvo.fs.path.Path(text = real_1))
    }
    override fun listDir(path: salvo.fs.path.Path): Union2<List<String>, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val real: String? = fsResolve(salvo.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<List<String>, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.listDir(salvo.fs.path.Path(text = real_1))
    }
    override fun createDirs(path: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val real: String? = fsResolve(salvo.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.createDirs(salvo.fs.path.Path(text = real_1))
    }
    override fun delete(path: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val real: String? = fsResolve(salvo.fs.path.toStr(root), pathText)
        if ((real == null)) {
            return Union2.U2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(pathText)))
        }
        val real_1: String = real!!
        return __dep0.delete(salvo.fs.path.Path(text = real_1))
    }
    override fun renamePath(from: salvo.fs.path.Path, to: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val fromText: String = salvo.fs.path.toStr(from)
        val toText: String = salvo.fs.path.toStr(to)
        val realFrom: String? = fsResolve(salvo.fs.path.toStr(root), fromText)
        if ((realFrom == null)) {
            return Union2.U2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(fromText)))
        }
        val realTo: String? = fsResolve(salvo.fs.path.toStr(root), toText)
        if ((realTo == null)) {
            return Union2.U2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(fsEscaped(toText)))
        }
        val realFrom_1: String = realFrom!!
        val realTo_2: String = realTo!!
        return __dep0.renamePath(salvo.fs.path.Path(text = realFrom_1), salvo.fs.path.Path(text = realTo_2))
    }
}

