package salvo.main

import salvo.*
import salvo.core.bytes.bytesOf
import salvo.core.bytes.clearPlatform as clearPlatform__core_bytes
import salvo.core.bytes.mutBytes
import salvo.core.bytes.sizePlatform as sizePlatform__core_bytes
import salvo.core.bytes.toHexPlatform
import salvo.core.bytes.toStrPlatform
import salvo.core.checked.Checked
import salvo.core.checked.detach
import salvo.core.console.Console
import salvo.core.console.println
import salvo.core.iterator.Finished
import salvo.core.list.addPlatform
import salvo.core.list.at
import salvo.core.list.sizePlatform as sizePlatform__core_list
import salvo.core.list.toStr
import salvo.core.string.byteSizePlatform
import salvo.core.string.clearPlatform as clearPlatform__core_string
import salvo.core.string.mutStr
import salvo.core.string.sizePlatform as sizePlatform__core_string
import salvo.fs.AlreadyExists
import salvo.fs.Fs
import salvo.fs.IoError
import salvo.fs.NotADirectory
import salvo.fs.NotFound
import salvo.fs.PathEscapes
import salvo.fs.PermissionDenied
import salvo.fs.Streaming
import salvo.fs.__Mon_Fs
import salvo.fs.copyFile
import salvo.fs.host.DefaultFs
import salvo.fs.host.RawFs
import salvo.fs.host.__Mon_RawFs
import salvo.fs.mem.MemFs
import salvo.fs.openChunks
import salvo.fs.path.path
import salvo.fs.readToBytes
import salvo.fs.readToStr
import salvo.fs.restricted.RestrictedFs
import salvo.fs.writeStr
import salvo.stream.Chunks
import salvo.stream.InStream
import salvo.stream.InvalidUtf8
import salvo.stream.OutStream
import salvo.stream.StreamFailed
import salvo.stream.close__Chunks
import salvo.stream.close__Lines
import salvo.stream.host.DefaultStreams
import salvo.stream.host.HostRawStreams
import salvo.stream.host.RawStreams
import salvo.stream.host.kind
import salvo.stream.lines
import salvo.stream.next__Chunks
import salvo.stream.next__Lines

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun kindName__FsError(kind: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): String {
    if (kind is Union7.U1<*, *, *, *, *, *, *>) {
        return "not found"
    }
    if (kind is Union7.U4<*, *, *, *, *, *, *>) {
        return "not a directory"
    }
    if (kind is Union7.U5<*, *, *, *, *, *, *>) {
        return "escapes the sandbox"
    }
    if (kind is Union7.U7<*, *, *, *, *, *, *>) {
        return kindName__StreamError((kind.value as Streaming).error)
    }
    return "other"
}

fun kindName__StreamError(kind: Union2<InvalidUtf8, StreamFailed>): String {
    if (kind is Union2.U1<*, *>) {
        return "not valid UTF-8"
    }
    return "other"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun workflow(fs: Fs, console: Console, streams: salvo.stream.Streams) {
    val wrote = writeStr(fs, streams, path("notes.txt"), "alpha\nbeta\ngamma\n")
    when (wrote) {
        is Union2.U1<*, *> -> {
            println(console, "wrote ${(wrote.value as Long)} bytes")
        }
        is Union2.U2<*, *> -> {
            println(console, "write failed: ${kindName__FsError(detach((wrote.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val text = readToStr(fs, streams, path("notes.txt"))
    when (text) {
        is Union2.U1<*, *> -> {
            println(console, "read back ${byteSizePlatform((text.value as String))} bytes")
        }
        is Union2.U2<*, *> -> {
            println(console, "read failed: ${kindName__FsError(detach((text.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val opened = fs.openRead(path("notes.txt"))
    when (opened) {
        is Union2.U1<*, *> -> {
            val p = lines((opened.value as InStream))
            while (true) {
                val __loop1_step = next__Lines(streams, p)
                if (__loop1_step !is Union2.U1<String, Finished>) { break }
                val line = __loop1_step.value
                println(console, "line: $line")
            }
            val closed = close__Lines(streams, p)
            if (closed is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__StreamError(detach((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "open failed: ${kindName__FsError(detach((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val out = fs.openAppend(path("notes.txt"))
    when (out) {
        is Union2.U1<*, *> -> {
            val w: OutStream = (out.value as OutStream)
            val at = streams.position__OutStream(w)
            val n = streams.writeLine(w, "delta")
            println(console, "appended $n bytes at offset $at")
            val shut = streams.close__OutStream(w)
            if (shut is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__StreamError(detach((shut.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
            val resumed = fs.openReadAt(path("notes.txt"), at)
            when (resumed) {
                is Union2.U1<*, *> -> {
                    val s: InStream = (resumed.value as InStream)
                    val line = streams.readLine(s)
                    when {
                        line != null -> {
                            println(console, "at $at: $line")
                        }
                        else -> {
                            println(console, "at $at: end of file")
                        }
                    }
                    val done = streams.close__InStream(s)
                    if (done is Union2.U2<*, *>) {
                        println(console, "close failed: ${kindName__StreamError(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                    }
                }
                is Union2.U2<*, *> -> {
                    println(console, "reopen failed: ${kindName__FsError(detach((resumed.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
                }
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "append failed: ${kindName__FsError(detach((out.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val bin = fs.openWrite(path("raw.bin"))
    when (bin) {
        is Union2.U1<*, *> -> {
            val w: OutStream = (bin.value as OutStream)
            val data = bytesOf(arrayOf((0).toUByte(), (255).toUByte(), (200).toUByte()))
            val n = streams.writeBytes(w, data)
            val m = streams.write(w, "hé")
            println(console, "wrote $n raw bytes and $m encoded")
            val shut = streams.close__OutStream(w)
            if (shut is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__StreamError(detach((shut.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "raw open failed: ${kindName__FsError(detach((bin.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val raw = fs.openRead(path("raw.bin"))
    when (raw) {
        is Union2.U1<*, *> -> {
            val s: InStream = (raw.value as InStream)
            val head = streams.readBytes(s, 3)
            when (head) {
                is Union2.U1<*, *> -> {
                    println(console, "first three: ${toStrPlatform((head.value as salvo.platform.core.bytes.Bytes))} = ${toHexPlatform((head.value as salvo.platform.core.bytes.Bytes))}")
                }
                is Union2.U2<*, *> -> {
                    println(console, "byte read failed: ${kindName__StreamError(detach((head.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                }
            }
            val tail = streams.readAll(s)
            when (tail) {
                is Union2.U1<*, *> -> {
                    println(console, "the rest, as text: ${(tail.value as String)}")
                }
                is Union2.U2<*, *> -> {
                    println(console, "decode failed: ${kindName__StreamError(detach((tail.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                }
            }
            val done = streams.close__InStream(s)
            if (done is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__StreamError(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "raw read failed: ${kindName__FsError(detach((raw.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val split = fs.openReadAt(path("raw.bin"), 5L)
    when (split) {
        is Union2.U1<*, *> -> {
            val s: InStream = (split.value as InStream)
            val broken = streams.readAll(s)
            when (broken) {
                is Union2.U1<*, *> -> {
                    println(console, "unexpected: ${(broken.value as String)} decoded")
                }
                is Union2.U2<*, *> -> {
                    println(console, "mid-character: ${kindName__StreamError(detach((broken.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                }
            }
            val done = streams.close__InStream(s)
            when (done) {
                is Union2.U1<*, *> -> {
                    println(console, "unexpected: the failure was not recorded")
                }
                is Union2.U2<*, *> -> {
                    println(console, "and again at close: ${kindName__StreamError(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                }
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "split open failed: ${kindName__FsError(detach((split.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val held = fs.openRead(path("raw.bin"))
    when (held) {
        is Union2.U1<*, *> -> {
            val s: InStream = (held.value as InStream)
            val buf = mutBytes(arrayOf())
            var steps = 0
            var moved = 0
            var reading = true
            while (reading) {
                clearPlatform__core_bytes(buf)
                val got = streams.readTo__InStream_Bytes_Int(s, buf, 4)
                when (got) {
                    is Union2.U1<*, *> -> {
                        val n: Int = (got.value as Int)
                        if (n == 0) {
                            reading = false
                        } else {
                            steps = steps + 1
                            moved = moved + n
                        }
                    }
                    is Union2.U2<*, *> -> {
                        println(console, "fill failed: ${kindName__StreamError(detach((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                        reading = false
                    }
                }
            }
            println(console, "filled $moved bytes in $steps reads, one buffer")
            val done = streams.close__InStream(s)
            if (done is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__StreamError(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "fill open failed: ${kindName__FsError(detach((held.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val lined = fs.openRead(path("notes.txt"))
    when (lined) {
        is Union2.U1<*, *> -> {
            val s: InStream = (lined.value as InStream)
            val line = mutStr(arrayOf())
            var longest = 0
            var reading = true
            while (reading) {
                clearPlatform__core_string(line)
                if (streams.readLineTo(s, line)) {
                    if (sizePlatform__core_string(line.toString()) > longest) {
                        longest = sizePlatform__core_string(line.toString())
                    }
                } else {
                    reading = false
                }
            }
            println(console, "longest line: $longest characters")
            val done = streams.close__InStream(s)
            if (done is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__StreamError(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "lines open failed: ${kindName__FsError(detach((lined.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val ch = openChunks(fs, streams, path("raw.bin"), 4)
    when (ch) {
        is Union2.U1<*, *> -> {
            val p = (ch.value as Chunks)
            var seen = 0
            while (true) {
                val __loop2_step = next__Chunks(streams, p)
                if (__loop2_step !is Union2.U1<salvo.platform.core.bytes.Bytes, Finished>) { break }
                val chunk = __loop2_step.value
                seen = seen + sizePlatform__core_bytes(chunk)
            }
            println(console, "chunks saw $seen bytes")
            val done = close__Chunks(streams, p)
            if (done is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__StreamError(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "chunks failed: ${kindName__FsError(detach((ch.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val copied = copyFile(fs, streams, path("notes.txt"), path("notes-copy.txt"))
    when (copied) {
        is Union2.U1<*, *> -> {
            println(console, "copied ${(copied.value as Long)} bytes")
        }
        is Union2.U2<*, *> -> {
            println(console, "copy failed: ${kindName__FsError(detach((copied.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val whole = readToBytes(fs, streams, path("raw.bin"))
    when (whole) {
        is Union2.U1<*, *> -> {
            println(console, "raw.bin is ${sizePlatform__core_bytes((whole.value as salvo.platform.core.bytes.Bytes))} bytes: ${toHexPlatform((whole.value as salvo.platform.core.bytes.Bytes))}")
        }
        is Union2.U2<*, *> -> {
            println(console, "byte read failed: ${kindName__FsError(detach((whole.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val failures: salvo.platform.core.list.MutList<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = mutableListOf<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>()
    val missing = readToStr(fs, streams, path("nope.txt"))
    when (missing) {
        is Union2.U1<*, *> -> {
            println(console, "unexpected: ${(missing.value as String)}")
        }
        is Union2.U2<*, *> -> {
            addPlatform(failures, detach((missing.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))
        }
    }
    val notADir = fs.listDir(path("notes.txt"))
    when (notADir) {
        is Union2.U1<*, *> -> {
            println(console, "unexpected: ${toStr((notADir.value as List<String>), { __i0 -> __i0 })}")
        }
        is Union2.U2<*, *> -> {
            addPlatform(failures, detach((notADir.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))
        }
    }
    println(console, "failures: ${sizePlatform__core_list(failures)}")
    for (kind in salvo.platform.core.list.each(failures)) {
        println(console, "  ${kindName__FsError(kind)}")
    }
    for (name in salvo.platform.core.list.each(listOf<String>("notes.txt", "notes-copy.txt", "raw.bin"))) {
        val gone = fs.delete(path(name))
        if (gone is Union2.U2<*, *>) {
            println(console, "delete failed: ${kindName__FsError(detach((gone.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    println(console, "cleaned up")
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun sandboxEdges(fs: Fs, console: Console, streams: salvo.stream.Streams) {
    val inside = writeStr(fs, streams, path("sub/../probe.txt"), "inside\n")
    when (inside) {
        is Union2.U1<*, *> -> {
            println(console, "through `..`: wrote ${(inside.value as Long)} bytes")
        }
        is Union2.U2<*, *> -> {
            println(console, "through `..`: ${kindName__FsError(detach((inside.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val up = readToStr(fs, streams, path("../secret.txt"))
    when (up) {
        is Union2.U1<*, *> -> {
            println(console, "unexpected: read outside the sandbox")
        }
        is Union2.U2<*, *> -> {
            println(console, "climbing out: ${kindName__FsError(detach((up.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val absolute = readToStr(fs, streams, path("/etc/hosts"))
    when (absolute) {
        is Union2.U1<*, *> -> {
            println(console, "unexpected: an absolute path resolved")
        }
        is Union2.U2<*, *> -> {
            println(console, "absolute path: ${kindName__FsError(detach((absolute.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val probe = path("probe.txt")
    println(console, "probe still there: ${fs.exists(probe)}")
    val gone = fs.delete(probe)
    if (gone is Union2.U2<*, *>) {
        println(console, "delete failed: ${kindName__FsError(detach((gone.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Faults", salvo.core.actor.__PROTO_Faults)))
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val raw_streams: RawStreams = HostRawStreams()
    val streams: salvo.stream.Streams = DefaultStreams(raw_streams)
    val raw_fs: RawFs = __Mon_RawFs(salvo.fs.host.__Platform_HostRawFs())
    val fs: Fs = DefaultFs(raw_fs, streams)
    val root = path("tmp/files-example")
    val made = fs.createDirs(root)
    if (made is Union2.U2<*, *>) {
        println(console, "cannot create the working directory: ${kindName__FsError(detach((made.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        return
    }
    println(console, "-- the real filesystem, scoped to one directory --")
    if (true) {
        val fs2: Fs = RestrictedFs(root, fs, streams)
        workflow(fs2, console, streams)
        sandboxEdges(fs2, console, streams)
    }
    val gone = fs.delete(root)
    if (gone is Union2.U2<*, *>) {
        println(console, "cleanup failed: ${kindName__FsError(detach((gone.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
    }
    println(console, "-- the same code, with no disk at all --")
    if (true) {
        val __h = MemFs()
        val __l = java.util.concurrent.locks.ReentrantLock()
        val fs3: Fs = __Mon_Fs(__h, __l)
        val streams2: salvo.stream.Streams = salvo.stream.__Mon_Streams(__h, __l)
        workflow(fs3, console, streams2)
    }
}
