package salvo.std.fs.mem

import salvo.*

data class MemRead(
    val source: String,
    val data: salvo.platform.std.core.bytes.Bytes,
    val at: Int,
    val failed: Boolean,
)

object __Codec_MemRead : salvo.WireCodec<MemRead> {
    override fun enc(v: MemRead, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.source, out)
        salvo.BytesCodec.enc(v.data, out)
        salvo.IntCodec.enc(v.at, out)
        salvo.BoolCodec.enc(v.failed, out)
    }
    override fun dec(inp: salvo.WireIn): MemRead = MemRead(salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.BoolCodec.dec(inp))
}

data class MemWrite(
    val path: String,
    val buffer: salvo.platform.std.core.bytes.Bytes,
)

object __Codec_MemWrite : salvo.WireCodec<MemWrite> {
    override fun enc(v: MemWrite, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
        salvo.BytesCodec.enc(v.buffer, out)
    }
    override fun dec(inp: salvo.WireIn): MemWrite = MemWrite(salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

class MemFs : salvo.std.fs.Fs, salvo.std.stream.Streams {
    var files: salvo.platform.std.core.map.MutMap<String, salvo.platform.std.core.bytes.Bytes> = salvo.std.core.map.Mut_Map_platform(arrayOf<kotlin.Pair<String, salvo.platform.std.core.bytes.Bytes>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var reads: salvo.platform.std.core.map.MutMap<Long, MemRead> = salvo.std.core.map.Mut_Map_platform(arrayOf<kotlin.Pair<Long, MemRead>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var writes: salvo.platform.std.core.map.MutMap<Long, MemWrite> = salvo.std.core.map.Mut_Map_platform(arrayOf<kotlin.Pair<Long, MemWrite>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    override fun openRead(path: salvo.std.fs.path.Path): Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val content: salvo.platform.std.core.bytes.Bytes? = salvo.std.core.map.getPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((content == null)) {
            return Union2.U2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union7.U1<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>(salvo.std.fs.NotFound(path = pathText)))))
        }
        val handle: Long = salvo.std.stream.freshHandle()
        val content_1: salvo.platform.std.core.bytes.Bytes = content!!
        salvo.std.core.map.putPlatform(reads, handle, MemRead(source = pathText, data = content_1, at = 0, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.stream.InStream(handle = handle)))
    }
    override fun openReadAt(path: salvo.std.fs.path.Path, offset: Long): Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val content: salvo.platform.std.core.bytes.Bytes? = salvo.std.core.map.getPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((content == null)) {
            return Union2.U2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union7.U1<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>(salvo.std.fs.NotFound(path = pathText)))))
        }
        var at: Int = (offset).toInt()
        if ((at < 0)) {
            at = 0
        }
        val content_1: salvo.platform.std.core.bytes.Bytes = content!!
        val end: Int = salvo.std.core.bytes.sizePlatform(content_1)
        if ((at > end)) {
            at = end
        }
        val handle: Long = salvo.std.stream.freshHandle()
        salvo.std.core.map.putPlatform(reads, handle, MemRead(source = pathText, data = content_1, at = at, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.stream.InStream(handle = handle)))
    }
    override fun openWrite(path: salvo.std.fs.path.Path): Union2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val handle: Long = salvo.std.stream.freshHandle()
        val empty: salvo.platform.std.core.bytes.MutBytes = salvo.std.core.bytes.mutBytes(arrayOf<salvo.platform.std.core.bytes.Bytes>())
        salvo.std.core.map.putPlatform(writes, handle, MemWrite(path = pathText, buffer = empty), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.stream.OutStream(handle = handle)))
    }
    override fun openAppend(path: salvo.std.fs.path.Path): Union2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val existing: salvo.platform.std.core.bytes.Bytes? = salvo.std.core.map.getPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        val start: salvo.platform.std.core.bytes.MutBytes = salvo.std.core.bytes.mutBytes(arrayOf<salvo.platform.std.core.bytes.Bytes>())
        if ((existing == null)) {
        } else {
            val existing_1: salvo.platform.std.core.bytes.Bytes = existing!!
            salvo.std.core.bytes.appendPlatform(start, existing_1)
        }
        val handle: Long = salvo.std.stream.freshHandle()
        salvo.std.core.map.putPlatform(writes, handle, MemWrite(path = pathText, buffer = start), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.stream.OutStream(handle = handle)))
    }
    override fun exists(path: salvo.std.fs.path.Path): Boolean {
        val pathText: String = salvo.std.fs.path.toStr(path)
        if (salvo.std.core.map.containsKeyPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            return true
        }
        return salvo.std.fs.mem.fsHasChildren(files, pathText)
    }
    override fun metadata(path: salvo.std.fs.path.Path): Union2<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        val content: salvo.platform.std.core.bytes.Bytes? = salvo.std.core.map.getPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((content == null)) {
            if (salvo.std.fs.mem.fsHasChildren(files, pathText)) {
                return Union2.U1<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.fs.FileInfo(size = 0L, isDir = true)))
            }
            return Union2.U2<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union7.U1<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>(salvo.std.fs.NotFound(path = pathText)))))
        }
        val content_1: salvo.platform.std.core.bytes.Bytes = content!!
        return Union2.U1<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(salvo.std.fs.FileInfo(size = (salvo.std.core.bytes.sizePlatform(content_1)).toLong(), isDir = false)))
    }
    override fun listDir(path: salvo.std.fs.path.Path): Union2<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        if (salvo.std.core.map.containsKeyPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            return Union2.U2<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union7.U4<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>(salvo.std.fs.NotADirectory(path = pathText)))))
        }
        if (!(salvo.std.fs.mem.fsHasChildren(files, pathText))) {
            return Union2.U2<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union7.U1<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>(salvo.std.fs.NotFound(path = pathText)))))
        }
        val names: salvo.platform.std.core.set.MutSet<String> = salvo.std.core.set.Mut_Set_platform(arrayOf<String>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        val prefix: String = "${pathText}/"
        for (key in salvo.platform.std.core.map.each(files)) {
            if (salvo.std.core.string.startsWithPlatform(key, prefix)) {
                val rest: String = salvo.std.core.string.trimPrefixPlatform(key, prefix)
                val cut: Int? = salvo.std.core.string.indexOfPlatform(rest, "/")
                var name: String = rest
                if ((cut != null)) {
                    val cut_1: Int = cut!!
                    val head: String? = salvo.std.core.string.substrPlatform(rest, 0, cut_1)
                    if ((head != null)) {
                        val head_2: String = head!!
                        name = head_2
                    }
                }
                salvo.std.core.set.addPlatform(names, name, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            }
        }
        val sorted: List<String> = salvo.std.core.list.sort(salvo.std.core.set.toListPlatform(names), { __a0, __a1 -> salvo.__salvoCompare(__a0, __a1) })
        return Union2.U1<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(sorted))
    }
    override fun createDirs(path: salvo.std.fs.path.Path): Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        return Union2.U1<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(Unit))
    }
    override fun delete(path: salvo.std.fs.path.Path): Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val pathText: String = salvo.std.fs.path.toStr(path)
        if (salvo.std.core.map.containsKeyPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            salvo.std.core.map.removePlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            return Union2.U1<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(Unit))
        }
        if (salvo.std.fs.mem.fsHasChildren(files, pathText)) {
            return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union7.U6<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>(salvo.std.fs.IoError(path = pathText, message = "directory not empty")))))
        }
        return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union7.U1<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>(salvo.std.fs.NotFound(path = pathText)))))
    }
    override fun renamePath(from: salvo.std.fs.path.Path, to: salvo.std.fs.path.Path): Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> {
        val fromText: String = salvo.std.fs.path.toStr(from)
        val toText: String = salvo.std.fs.path.toStr(to)
        val content: salvo.platform.std.core.bytes.Bytes? = salvo.std.core.map.getPlatform(files, fromText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((content == null)) {
            return Union2.U2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union7.U1<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>(salvo.std.fs.NotFound(path = fromText)))))
        }
        val content_1: salvo.platform.std.core.bytes.Bytes = content!!
        val bytes: salvo.platform.std.core.bytes.Bytes = content_1
        salvo.std.core.map.removePlatform(files, fromText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.std.core.map.putPlatform(files, toText, bytes, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>>(salvo.std.core.result.ok(Unit))
    }
    override fun readLine(s: salvo.std.stream.InStream): String? {
        return salvo.std.fs.mem.memReadLine(reads, s.handle)
    }
    override fun readAll(s: salvo.std.stream.InStream): Union2<String, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> {
        return salvo.std.fs.mem.memReadAll(reads, s.handle)
    }
    override fun readBytes(s: salvo.std.stream.InStream, max: Int): Union2<salvo.platform.std.core.bytes.Bytes, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> {
        return salvo.std.fs.mem.memReadBytes(reads, s.handle, max)
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo__InStream_Bytes_Int(s: salvo.std.stream.InStream, buf: salvo.platform.std.core.bytes.MutBytes, max: Int): Union2<Int, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> {
        val got: Union2<salvo.platform.std.core.bytes.Bytes, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = salvo.std.fs.mem.memReadBytes(reads, s.handle, max)
        if ((got is Union2.U2<*, *>)) {
            val got_1: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
            return Union2.U2<Int, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(got_1)
        }
        val got_2: salvo.platform.std.core.bytes.Bytes = ((got as Union2.U1<*, *>).value as salvo.platform.std.core.bytes.Bytes)
        val data: salvo.platform.std.core.bytes.Bytes = got_2
        salvo.std.core.bytes.appendPlatform(buf, data)
        return Union2.U1<Int, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.ok(salvo.std.core.bytes.sizePlatform(data)))
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo__InStream_Str(s: salvo.std.stream.InStream, buf: salvo.platform.std.core.string.MutStr): Union2<Long, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> {
        val got: Union2<String, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = salvo.std.fs.mem.memReadAll(reads, s.handle)
        if ((got is Union2.U2<*, *>)) {
            val got_1: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
            return Union2.U2<Long, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(got_1)
        }
        val got_2: String = ((got as Union2.U1<*, *>).value as String)
        val text: String = got_2
        salvo.std.core.string.appendPlatform(buf, text)
        return Union2.U1<Long, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.ok(salvo.std.core.string.byteSizePlatform(text)))
    }
    override fun readLineTo(s: salvo.std.stream.InStream, buf: salvo.platform.std.core.string.MutStr): Boolean {
        val line: String? = salvo.std.fs.mem.memReadLine(reads, s.handle)
        return when {
            (line != null) -> {
                val line_1: String = line!!
                salvo.std.core.string.appendPlatform(buf, line_1)
                return true
            }
            (line == null) -> {
                return false
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.fs.mem:220:9"))
            }
        }
    }
    override fun position__InStream(s: salvo.std.stream.InStream): Long {
        return (kotlin.run {
            val __proj_1: MemRead = salvo.std.fs.mem.memReadState(reads, s.handle)
            __proj_1.at
        }).toLong()
    }
    override fun close__InStream(s: salvo.std.stream.InStream): Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> {
        val open: MemRead = salvo.std.fs.mem.memReadState(reads, s.handle)
        val failed: Boolean = open.failed
        val source: String = open.source
        salvo.std.core.map.removePlatform(reads, s.handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        kotlin.run { s; Unit }
        if (failed) {
            return Union2.U2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>(salvo.std.stream.InvalidUtf8(source = source)))))
        }
        return Union2.U1<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.ok(Unit))
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun receive(s: salvo.std.stream.InStream, reply: salvo.SalvoReply) {
        val got: Union2<salvo.platform.std.core.bytes.Bytes, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = salvo.std.fs.mem.memReadBytes(reads, s.handle, 65536)
        if ((got is Union2.U2<*, *>)) {
            val got_1: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
            val open: MemRead = salvo.std.fs.mem.memReadState(reads, s.handle)
            salvo.std.core.map.removePlatform(reads, s.handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            kotlin.run { s; Unit }
            reply.send(Union3.U3<salvo.std.stream.Packet, salvo.std.stream.End, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>(salvo.std.stream.StreamFailed(source = open.source, message = "read failed"))))))
            salvo.std.core.checked.ignore(got_1)
            null
            return
        }
        val got_2: salvo.platform.std.core.bytes.Bytes = ((got as Union2.U1<*, *>).value as salvo.platform.std.core.bytes.Bytes)
        val data: salvo.platform.std.core.bytes.Bytes = got_2
        if ((salvo.std.core.bytes.sizePlatform(data) == 0)) {
            salvo.std.core.map.removePlatform(reads, s.handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            kotlin.run { s; Unit }
            reply.send(Union3.U2<salvo.std.stream.Packet, salvo.std.stream.End, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.stream.End()))
            null
            return
        }
        reply.send(Union3.U1<salvo.std.stream.Packet, salvo.std.stream.End, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.ok(salvo.std.stream.Packet(bytes = data, stream = s))))
    }
    override fun fromBytes(data: salvo.platform.std.core.bytes.Bytes): salvo.std.stream.InStream {
        val handle: Long = salvo.std.stream.freshHandle()
        salvo.std.core.map.putPlatform(reads, handle, MemRead(source = "<bytes>", data = data, at = 0, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return salvo.std.stream.InStream(handle = handle)
    }
    override fun write(s: salvo.std.stream.OutStream, text: String): Long {
        return salvo.std.fs.mem.memAppend(writes, s.handle, salvo.std.core.bytes.toBytesPlatform(text))
    }
    override fun writeLine(s: salvo.std.stream.OutStream, text: String): Long {
        return salvo.std.fs.mem.memAppend(writes, s.handle, salvo.std.core.bytes.toBytesPlatform("${text}\n"))
    }
    override fun writeBytes(s: salvo.std.stream.OutStream, data: salvo.platform.std.core.bytes.Bytes): Long {
        return salvo.std.fs.mem.memAppend(writes, s.handle, data)
    }
    override fun position__OutStream(s: salvo.std.stream.OutStream): Long {
        return (salvo.std.core.bytes.sizePlatform(kotlin.run {
            val __proj_1: MemWrite = salvo.std.fs.mem.memWriteState(writes, s.handle)
            __proj_1.buffer
        })).toLong()
    }
    override fun flush(s: salvo.std.stream.OutStream): Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> {
        val open: MemWrite = salvo.std.fs.mem.memWriteState(writes, s.handle)
        salvo.std.core.map.putPlatform(files, open.path, open.buffer, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.ok(Unit))
    }
    override fun close__OutStream(s: salvo.std.stream.OutStream): Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> {
        val open: MemWrite = salvo.std.fs.mem.memWriteState(writes, s.handle)
        salvo.std.core.map.putPlatform(files, open.path, open.buffer, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.std.core.map.removePlatform(writes, s.handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        kotlin.run { s; Unit }
        return Union2.U1<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.ok(Unit))
    }
}

fun fsHasChildren(files: salvo.platform.std.core.map.Map<String, salvo.platform.std.core.bytes.Bytes>, path: String): Boolean {
    val prefix: String = "${path}/"
    for (key in salvo.platform.std.core.map.each(files)) {
        if (salvo.std.core.string.startsWithPlatform(key, prefix)) {
            return true
        }
    }
    return false
}

fun memFindNewline(data: salvo.platform.std.core.bytes.Bytes, from: Int): Int {
    val end: Int = salvo.std.core.bytes.sizePlatform(data)
    var i: Int = from
    while (true) {
        if (!((i < end))) {
            break
        }
        if (((kotlin.run {
            val __pick_1: UByte? = salvo.std.core.bytes.getPlatform(data, i)
            (if ((__pick_1 != null)) {
                val __picked_2: UByte = __pick_1!!
                __picked_2
            } else {
                throw AssertionError("salvo: unreachable at std.fs.mem:328:35")
            })
        }).toInt() == 10)) {
            return i
        }
        i = (i + 1)
    }
    return end
}

fun memAppend(writes: salvo.platform.std.core.map.MutMap<Long, MemWrite>, handle: Long, data: salvo.platform.std.core.bytes.Bytes): Long {
    val open: MemWrite = salvo.std.fs.mem.memWriteState(writes, handle)
    val grown: salvo.platform.std.core.bytes.MutBytes = salvo.std.core.bytes.mutBytes(arrayOf<salvo.platform.std.core.bytes.Bytes>(open.buffer))
    salvo.std.core.bytes.appendPlatform(grown, data)
    val buffer: salvo.platform.std.core.bytes.Bytes = grown
    salvo.std.core.map.putPlatform(writes, handle, MemWrite(path = open.path, buffer = buffer), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    return (salvo.std.core.bytes.sizePlatform(data)).toLong()
}

fun memReadState(reads: salvo.platform.std.core.map.Map<Long, MemRead>, handle: Long): MemRead {
    val open: MemRead? = salvo.std.core.map.getPlatform(reads, handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((open == null)) {
        throw AssertionError(("salvo: " + ("stream handle ${handle} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at std.fs.mem:356:9"))
    }
    val open_1: MemRead = open!!
    return MemRead(source = open_1.source, data = open_1.data, at = open_1.at, failed = open_1.failed)
}

fun memWriteState(writes: salvo.platform.std.core.map.Map<Long, MemWrite>, handle: Long): MemWrite {
    val open: MemWrite? = salvo.std.core.map.getPlatform(writes, handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((open == null)) {
        throw AssertionError(("salvo: " + ("stream handle ${handle} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at std.fs.mem:364:9"))
    }
    val open_1: MemWrite = open!!
    return MemWrite(path = open_1.path, buffer = open_1.buffer)
}

fun memReadLine(reads: salvo.platform.std.core.map.MutMap<Long, MemRead>, handle: Long): String? {
    val open: MemRead = salvo.std.fs.mem.memReadState(reads, handle)
    if (open.failed) {
        return null
    }
    val at: Int = open.at
    val bytes: salvo.platform.std.core.bytes.Bytes = open.data
    val end: Int = salvo.std.core.bytes.sizePlatform(bytes)
    if ((at >= end)) {
        return null
    }
    val stop: Int = salvo.std.fs.mem.memFindNewline(bytes, at)
    val line: salvo.platform.std.core.bytes.Bytes = kotlin.run {
        val __pick_1: salvo.platform.std.core.bytes.Bytes? = salvo.std.core.bytes.slicePlatform(bytes, at, stop)
        (if ((__pick_1 != null)) {
            val __picked_2: salvo.platform.std.core.bytes.Bytes = __pick_1!!
            __picked_2
        } else {
            throw AssertionError("salvo: unreachable at std.fs.mem:388:42")
        })
    }
    var nextAt: Int = stop
    if ((stop < end)) {
        nextAt = (stop + 1)
    }
    val text: String? = salvo.std.core.bytes.strOfBytesPlatform(line)
    if ((text == null)) {
        salvo.std.core.map.putPlatform(reads, handle, MemRead(source = open.source, data = bytes, at = nextAt, failed = true), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return null
    }
    salvo.std.core.map.putPlatform(reads, handle, MemRead(source = open.source, data = bytes, at = nextAt, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    val text_4: String = text!!
    return salvo.std.core.string.trimSuffixPlatform(text_4, "\r")
}

fun memReadAll(reads: salvo.platform.std.core.map.MutMap<Long, MemRead>, handle: Long): Union2<String, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> {
    val open: MemRead = salvo.std.fs.mem.memReadState(reads, handle)
    val source: String = open.source
    if (open.failed) {
        return Union2.U2<String, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>(salvo.std.stream.InvalidUtf8(source = source)))))
    }
    val bytes: salvo.platform.std.core.bytes.Bytes = open.data
    val end: Int = salvo.std.core.bytes.sizePlatform(bytes)
    val rest: salvo.platform.std.core.bytes.Bytes = kotlin.run {
        val __pick_1: salvo.platform.std.core.bytes.Bytes? = salvo.std.core.bytes.slicePlatform(bytes, open.at, end)
        (if ((__pick_1 != null)) {
            val __picked_2: salvo.platform.std.core.bytes.Bytes = __pick_1!!
            __picked_2
        } else {
            throw AssertionError("salvo: unreachable at std.fs.mem:412:46")
        })
    }
    val text: String? = salvo.std.core.bytes.strOfBytesPlatform(rest)
    if ((text == null)) {
        salvo.std.core.map.putPlatform(reads, handle, MemRead(source = source, data = bytes, at = end, failed = true), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U2<String, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>(salvo.std.stream.InvalidUtf8(source = source)))))
    }
    salvo.std.core.map.putPlatform(reads, handle, MemRead(source = source, data = bytes, at = end, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    val text_4: String = text!!
    return Union2.U1<String, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.ok(text_4))
}

fun memReadBytes(reads: salvo.platform.std.core.map.MutMap<Long, MemRead>, handle: Long, max: Int): Union2<salvo.platform.std.core.bytes.Bytes, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> {
    val open: MemRead = salvo.std.fs.mem.memReadState(reads, handle)
    val source: String = open.source
    if (open.failed) {
        return Union2.U2<salvo.platform.std.core.bytes.Bytes, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>(salvo.std.stream.InvalidUtf8(source = source)))))
    }
    val bytes: salvo.platform.std.core.bytes.Bytes = open.data
    var stop: Int = (open.at + max)
    if ((max < 0)) {
        stop = open.at
    }
    val end: Int = salvo.std.core.bytes.sizePlatform(bytes)
    if ((stop > end)) {
        stop = end
    }
    val taken: salvo.platform.std.core.bytes.Bytes = kotlin.run {
        val __pick_1: salvo.platform.std.core.bytes.Bytes? = salvo.std.core.bytes.slicePlatform(bytes, open.at, stop)
        (if ((__pick_1 != null)) {
            val __picked_2: salvo.platform.std.core.bytes.Bytes = __pick_1!!
            __picked_2
        } else {
            throw AssertionError("salvo: unreachable at std.fs.mem:438:48")
        })
    }
    salvo.std.core.map.putPlatform(reads, handle, MemRead(source = source, data = bytes, at = stop, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    return Union2.U1<salvo.platform.std.core.bytes.Bytes, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>(salvo.std.core.result.ok(taken))
}

