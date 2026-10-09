package salvo.fs.mem

import salvo.*

data class MemRead(
    val source: String,
    val data: salvo.platform.core.bytes.Bytes,
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
    val buffer: salvo.platform.core.bytes.Bytes,
)

object __Codec_MemWrite : salvo.WireCodec<MemWrite> {
    override fun enc(v: MemWrite, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.path, out)
        salvo.BytesCodec.enc(v.buffer, out)
    }
    override fun dec(inp: salvo.WireIn): MemWrite = MemWrite(salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

class MemFs : salvo.fs.Fs, salvo.stream.Streams {
    var files: salvo.platform.core.map.MutMap<String, salvo.platform.core.bytes.Bytes> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<String, salvo.platform.core.bytes.Bytes>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var reads: salvo.platform.core.map.MutMap<Long, MemRead> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Long, MemRead>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var writes: salvo.platform.core.map.MutMap<Long, MemWrite> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Long, MemWrite>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    override fun openRead(path: salvo.fs.path.Path): Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val content: salvo.platform.core.bytes.Bytes? = salvo.core.map.getPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((content == null)) {
            return Union2.U2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(salvo.core.checked.checked(Union7.U1<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>(salvo.fs.NotFound(path = pathText)))))
        }
        val handle: Long = salvo.stream.freshHandle()
        val content_1: salvo.platform.core.bytes.Bytes = content!!
        salvo.core.map.putPlatform(reads, handle, MemRead(source = pathText, data = content_1, at = 0, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(salvo.stream.InStream(handle = handle)))
    }
    override fun openReadAt(path: salvo.fs.path.Path, offset: Long): Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val content: salvo.platform.core.bytes.Bytes? = salvo.core.map.getPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((content == null)) {
            return Union2.U2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(salvo.core.checked.checked(Union7.U1<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>(salvo.fs.NotFound(path = pathText)))))
        }
        var at: Int = (offset).toInt()
        if ((at < 0)) {
            at = 0
        }
        val content_1: salvo.platform.core.bytes.Bytes = content!!
        val end: Int = salvo.core.bytes.sizePlatform(content_1)
        if ((at > end)) {
            at = end
        }
        val handle: Long = salvo.stream.freshHandle()
        salvo.core.map.putPlatform(reads, handle, MemRead(source = pathText, data = content_1, at = at, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(salvo.stream.InStream(handle = handle)))
    }
    override fun openWrite(path: salvo.fs.path.Path): Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val handle: Long = salvo.stream.freshHandle()
        val empty: salvo.platform.core.bytes.MutBytes = salvo.core.bytes.mutBytes(arrayOf<salvo.platform.core.bytes.Bytes>())
        salvo.core.map.putPlatform(writes, handle, MemWrite(path = pathText, buffer = empty), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(salvo.stream.OutStream(handle = handle)))
    }
    override fun openAppend(path: salvo.fs.path.Path): Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val existing: salvo.platform.core.bytes.Bytes? = salvo.core.map.getPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        val start: salvo.platform.core.bytes.MutBytes = salvo.core.bytes.mutBytes(arrayOf<salvo.platform.core.bytes.Bytes>())
        if ((existing == null)) {
        } else {
            val existing_1: salvo.platform.core.bytes.Bytes = existing!!
            salvo.core.bytes.appendPlatform(start, existing_1)
        }
        val handle: Long = salvo.stream.freshHandle()
        salvo.core.map.putPlatform(writes, handle, MemWrite(path = pathText, buffer = start), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(salvo.stream.OutStream(handle = handle)))
    }
    override fun exists(path: salvo.fs.path.Path): Boolean {
        val pathText: String = salvo.fs.path.toStr(path)
        if (salvo.core.map.containsKeyPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            return true
        }
        return fsHasChildren(files, pathText)
    }
    override fun metadata(path: salvo.fs.path.Path): Union2<salvo.fs.FileInfo, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        val content: salvo.platform.core.bytes.Bytes? = salvo.core.map.getPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((content == null)) {
            if (fsHasChildren(files, pathText)) {
                return Union2.U1<salvo.fs.FileInfo, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(salvo.fs.FileInfo(size = 0L, isDir = true)))
            }
            return Union2.U2<salvo.fs.FileInfo, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(salvo.core.checked.checked(Union7.U1<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>(salvo.fs.NotFound(path = pathText)))))
        }
        val content_1: salvo.platform.core.bytes.Bytes = content!!
        return Union2.U1<salvo.fs.FileInfo, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(salvo.fs.FileInfo(size = (salvo.core.bytes.sizePlatform(content_1)).toLong(), isDir = false)))
    }
    override fun listDir(path: salvo.fs.path.Path): Union2<List<String>, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        if (salvo.core.map.containsKeyPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            return Union2.U2<List<String>, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(salvo.core.checked.checked(Union7.U4<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>(salvo.fs.NotADirectory(path = pathText)))))
        }
        if (!(fsHasChildren(files, pathText))) {
            return Union2.U2<List<String>, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(salvo.core.checked.checked(Union7.U1<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>(salvo.fs.NotFound(path = pathText)))))
        }
        val names: salvo.platform.core.set.MutSet<String> = salvo.core.set.mutSetOfPlatform(arrayOf<String>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        val prefix: String = "${pathText}/"
        for (key in salvo.platform.core.map.each(files)) {
            if (salvo.core.string.startsWithPlatform(key, prefix)) {
                val rest: String = salvo.core.string.trimPrefixPlatform(key, prefix)
                val cut: Int? = salvo.core.string.indexOfPlatform(rest, "/")
                var name: String = rest
                if ((cut != null)) {
                    val cut_1: Int = cut!!
                    val head: String? = salvo.core.string.substrPlatform(rest, 0, cut_1)
                    if ((head != null)) {
                        val head_2: String = head!!
                        name = head_2
                    }
                }
                salvo.core.set.addPlatform(names, name, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            }
        }
        val sorted: List<String> = salvo.core.list.sort(salvo.core.set.toListPlatform(names), { __a0, __a1 -> salvo.__salvoCompare(__a0, __a1) })
        return Union2.U1<List<String>, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(sorted))
    }
    override fun createDirs(path: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        return Union2.U1<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(Unit))
    }
    override fun delete(path: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val pathText: String = salvo.fs.path.toStr(path)
        if (salvo.core.map.containsKeyPlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            salvo.core.map.removePlatform(files, pathText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            return Union2.U1<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(Unit))
        }
        if (fsHasChildren(files, pathText)) {
            return Union2.U2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(salvo.core.checked.checked(Union7.U6<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>(salvo.fs.IoError(path = pathText, message = "directory not empty")))))
        }
        return Union2.U2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(salvo.core.checked.checked(Union7.U1<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>(salvo.fs.NotFound(path = pathText)))))
    }
    override fun renamePath(from: salvo.fs.path.Path, to: salvo.fs.path.Path): Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> {
        val fromText: String = salvo.fs.path.toStr(from)
        val toText: String = salvo.fs.path.toStr(to)
        val content: salvo.platform.core.bytes.Bytes? = salvo.core.map.getPlatform(files, fromText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((content == null)) {
            return Union2.U2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.err(salvo.core.checked.checked(Union7.U1<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>(salvo.fs.NotFound(path = fromText)))))
        }
        val content_1: salvo.platform.core.bytes.Bytes = content!!
        val bytes: salvo.platform.core.bytes.Bytes = content_1
        salvo.core.map.removePlatform(files, fromText, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.core.map.putPlatform(files, toText, bytes, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>>(salvo.core.result.ok(Unit))
    }
    override fun readLine(s: salvo.stream.InStream): String? {
        return memReadLine(reads, s.handle)
    }
    override fun readAll(s: salvo.stream.InStream): Union2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        return memReadAll(reads, s.handle)
    }
    override fun readBytes(s: salvo.stream.InStream, max: Int): Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        return memReadBytes(reads, s.handle, max)
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo__InStream_Bytes_Int(s: salvo.stream.InStream, buf: salvo.platform.core.bytes.MutBytes, max: Int): Union2<Int, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val got: Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = memReadBytes(reads, s.handle, max)
        if ((got is Union2.U2<*, *>)) {
            val got_1: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
            return Union2.U2<Int, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(got_1)
        }
        val got_2: salvo.platform.core.bytes.Bytes = ((got as Union2.U1<*, *>).value as salvo.platform.core.bytes.Bytes)
        val data: salvo.platform.core.bytes.Bytes = got_2
        salvo.core.bytes.appendPlatform(buf, data)
        return Union2.U1<Int, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(salvo.core.bytes.sizePlatform(data)))
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun readTo__InStream_Str(s: salvo.stream.InStream, buf: salvo.platform.core.string.MutStr): Union2<Long, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val got: Union2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = memReadAll(reads, s.handle)
        if ((got is Union2.U2<*, *>)) {
            val got_1: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
            return Union2.U2<Long, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(got_1)
        }
        val got_2: String = ((got as Union2.U1<*, *>).value as String)
        val text: String = got_2
        salvo.core.string.appendPlatform(buf, text)
        return Union2.U1<Long, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(salvo.core.string.byteSizePlatform(text)))
    }
    override fun readLineTo(s: salvo.stream.InStream, buf: salvo.platform.core.string.MutStr): Boolean {
        val line: String? = memReadLine(reads, s.handle)
        return when {
            (line != null) -> {
                val line_1: String = line!!
                salvo.core.string.appendPlatform(buf, line_1)
                return true
            }
            (line == null) -> {
                return false
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
    override fun position__InStream(s: salvo.stream.InStream): Long {
        return (run {
            val __proj_1: MemRead = memReadState(reads, s.handle)
            __proj_1.at
        }).toLong()
    }
    override fun close__InStream(s: salvo.stream.InStream): Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val open: MemRead = memReadState(reads, s.handle)
        val failed: Boolean = open.failed
        val source: String = open.source
        salvo.core.map.removePlatform(reads, s.handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        run { s; Unit }
        if (failed) {
            return Union2.U2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U1<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>(salvo.stream.InvalidUtf8(source = source)))))
        }
        return Union2.U1<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(Unit))
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun receive(s: salvo.stream.InStream, reply: salvo.SalvoReply) {
        val got: Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = memReadBytes(reads, s.handle, 65536)
        if ((got is Union2.U2<*, *>)) {
            val got_1: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
            val open: MemRead = memReadState(reads, s.handle)
            salvo.core.map.removePlatform(reads, s.handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            run { s; Unit }
            reply.send(Union3.U3<salvo.stream.Packet, salvo.stream.End, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>(salvo.stream.StreamFailed(source = open.source, message = "read failed"))))))
            salvo.core.checked.ignore(got_1)
            null
            return
        }
        val got_2: salvo.platform.core.bytes.Bytes = ((got as Union2.U1<*, *>).value as salvo.platform.core.bytes.Bytes)
        val data: salvo.platform.core.bytes.Bytes = got_2
        if ((salvo.core.bytes.sizePlatform(data) == 0)) {
            salvo.core.map.removePlatform(reads, s.handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            run { s; Unit }
            reply.send(Union3.U2<salvo.stream.Packet, salvo.stream.End, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.stream.End()))
            null
            return
        }
        reply.send(Union3.U1<salvo.stream.Packet, salvo.stream.End, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(salvo.stream.Packet(bytes = data, stream = s))))
    }
    override fun fromBytes(data: salvo.platform.core.bytes.Bytes): salvo.stream.InStream {
        val handle: Long = salvo.stream.freshHandle()
        salvo.core.map.putPlatform(reads, handle, MemRead(source = "<bytes>", data = data, at = 0, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return salvo.stream.InStream(handle = handle)
    }
    override fun write(s: salvo.stream.OutStream, text: String): Long {
        return memAppend(writes, s.handle, salvo.core.bytes.toBytesPlatform(text))
    }
    override fun writeLine(s: salvo.stream.OutStream, text: String): Long {
        return memAppend(writes, s.handle, salvo.core.bytes.toBytesPlatform("${text}\n"))
    }
    override fun writeBytes(s: salvo.stream.OutStream, data: salvo.platform.core.bytes.Bytes): Long {
        return memAppend(writes, s.handle, data)
    }
    override fun position__OutStream(s: salvo.stream.OutStream): Long {
        return (salvo.core.bytes.sizePlatform(run {
            val __proj_1: MemWrite = memWriteState(writes, s.handle)
            __proj_1.buffer
        })).toLong()
    }
    override fun flush(s: salvo.stream.OutStream): Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val open: MemWrite = memWriteState(writes, s.handle)
        salvo.core.map.putPlatform(files, open.path, open.buffer, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U1<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(Unit))
    }
    override fun close__OutStream(s: salvo.stream.OutStream): Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
        val open: MemWrite = memWriteState(writes, s.handle)
        salvo.core.map.putPlatform(files, open.path, open.buffer, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.core.map.removePlatform(writes, s.handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        run { s; Unit }
        return Union2.U1<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(Unit))
    }
}

fun fsHasChildren(files: salvo.platform.core.map.Map<String, salvo.platform.core.bytes.Bytes>, path: String): Boolean {
    val prefix: String = "${path}/"
    for (key in salvo.platform.core.map.each(files)) {
        if (salvo.core.string.startsWithPlatform(key, prefix)) {
            return true
        }
    }
    return false
}

fun memFindNewline(data: salvo.platform.core.bytes.Bytes, from: Int): Int {
    val end: Int = salvo.core.bytes.sizePlatform(data)
    var i: Int = from
    while (true) {
        if (!((i < end))) {
            break
        }
        if (((run {
            val __nn_1: UByte? = salvo.core.bytes.getPlatform(data, i)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at fs.mem:323:19"))
                }
                else -> {
                    val __some_2: UByte = __nn_1!!
                    __some_2
                }
            }
        }).toInt() == 10)) {
            return i
        }
        i = (i + 1)
    }
    return end
}

fun memAppend(writes: salvo.platform.core.map.MutMap<Long, MemWrite>, handle: Long, data: salvo.platform.core.bytes.Bytes): Long {
    val open: MemWrite = memWriteState(writes, handle)
    val grown: salvo.platform.core.bytes.MutBytes = salvo.core.bytes.mutBytes(arrayOf<salvo.platform.core.bytes.Bytes>(open.buffer))
    salvo.core.bytes.appendPlatform(grown, data)
    val buffer: salvo.platform.core.bytes.Bytes = grown
    salvo.core.map.putPlatform(writes, handle, MemWrite(path = open.path, buffer = buffer), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    return (salvo.core.bytes.sizePlatform(data)).toLong()
}

fun memReadState(reads: salvo.platform.core.map.Map<Long, MemRead>, handle: Long): MemRead {
    val open: MemRead? = salvo.core.map.getPlatform(reads, handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((open == null)) {
        throw AssertionError(("salvo: " + ("stream handle ${handle} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at fs.mem:350:9"))
    }
    val open_1: MemRead = open!!
    return MemRead(source = open_1.source, data = open_1.data, at = open_1.at, failed = open_1.failed)
}

fun memWriteState(writes: salvo.platform.core.map.Map<Long, MemWrite>, handle: Long): MemWrite {
    val open: MemWrite? = salvo.core.map.getPlatform(writes, handle, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((open == null)) {
        throw AssertionError(("salvo: " + ("stream handle ${handle} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]") + " at fs.mem:358:9"))
    }
    val open_1: MemWrite = open!!
    return MemWrite(path = open_1.path, buffer = open_1.buffer)
}

fun memReadLine(reads: salvo.platform.core.map.MutMap<Long, MemRead>, handle: Long): String? {
    val open: MemRead = memReadState(reads, handle)
    if (open.failed) {
        return null
    }
    val at: Int = open.at
    val bytes: salvo.platform.core.bytes.Bytes = open.data
    val end: Int = salvo.core.bytes.sizePlatform(bytes)
    if ((at >= end)) {
        return null
    }
    val stop: Int = memFindNewline(bytes, at)
    val line: salvo.platform.core.bytes.Bytes = run {
        val __nn_1: salvo.platform.core.bytes.Bytes? = salvo.core.bytes.slicePlatform(bytes, at, stop)
        when {
            (__nn_1 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at fs.mem:382:16"))
            }
            else -> {
                val __some_2: salvo.platform.core.bytes.Bytes = __nn_1!!
                __some_2
            }
        }
    }
    var nextAt: Int = stop
    if ((stop < end)) {
        nextAt = (stop + 1)
    }
    val text: String? = salvo.core.bytes.strOfBytesPlatform(line)
    if ((text == null)) {
        salvo.core.map.putPlatform(reads, handle, MemRead(source = open.source, data = bytes, at = nextAt, failed = true), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return null
    }
    salvo.core.map.putPlatform(reads, handle, MemRead(source = open.source, data = bytes, at = nextAt, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    val text_3: String = text!!
    return salvo.core.string.trimSuffixPlatform(text_3, "\r")
}

fun memReadAll(reads: salvo.platform.core.map.MutMap<Long, MemRead>, handle: Long): Union2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
    val open: MemRead = memReadState(reads, handle)
    val source: String = open.source
    if (open.failed) {
        return Union2.U2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U1<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>(salvo.stream.InvalidUtf8(source = source)))))
    }
    val bytes: salvo.platform.core.bytes.Bytes = open.data
    val end: Int = salvo.core.bytes.sizePlatform(bytes)
    val rest: salvo.platform.core.bytes.Bytes = run {
        val __nn_1: salvo.platform.core.bytes.Bytes? = salvo.core.bytes.slicePlatform(bytes, open.at, end)
        when {
            (__nn_1 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at fs.mem:406:16"))
            }
            else -> {
                val __some_2: salvo.platform.core.bytes.Bytes = __nn_1!!
                __some_2
            }
        }
    }
    val text: String? = salvo.core.bytes.strOfBytesPlatform(rest)
    if ((text == null)) {
        salvo.core.map.putPlatform(reads, handle, MemRead(source = source, data = bytes, at = end, failed = true), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return Union2.U2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U1<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>(salvo.stream.InvalidUtf8(source = source)))))
    }
    salvo.core.map.putPlatform(reads, handle, MemRead(source = source, data = bytes, at = end, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    val text_3: String = text!!
    return Union2.U1<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(text_3))
}

fun memReadBytes(reads: salvo.platform.core.map.MutMap<Long, MemRead>, handle: Long, max: Int): Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> {
    val open: MemRead = memReadState(reads, handle)
    val source: String = open.source
    if (open.failed) {
        return Union2.U2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U1<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>(salvo.stream.InvalidUtf8(source = source)))))
    }
    val bytes: salvo.platform.core.bytes.Bytes = open.data
    var stop: Int = (open.at + max)
    if ((max < 0)) {
        stop = open.at
    }
    val end: Int = salvo.core.bytes.sizePlatform(bytes)
    if ((stop > end)) {
        stop = end
    }
    val taken: salvo.platform.core.bytes.Bytes = run {
        val __nn_1: salvo.platform.core.bytes.Bytes? = salvo.core.bytes.slicePlatform(bytes, open.at, stop)
        when {
            (__nn_1 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at fs.mem:432:17"))
            }
            else -> {
                val __some_2: salvo.platform.core.bytes.Bytes = __nn_1!!
                __some_2
            }
        }
    }
    salvo.core.map.putPlatform(reads, handle, MemRead(source = source, data = bytes, at = stop, failed = false), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    return Union2.U1<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>>(salvo.core.result.ok(taken))
}

