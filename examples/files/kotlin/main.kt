package salvo.main

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.console.*
import salvo.core.deque.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.fs.*
import salvo.fs.host.*
import salvo.fs.mem.*
import salvo.fs.restricted.*
import salvo.stream.*
import salvo.stream.host.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun kindName(kind: Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>): String {
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
        return kindName__2((kind.value as Streaming).error)
    }
    return "other"
}

fun kindName__2(kind: Union2<InvalidUtf8, StreamFailed>): String {
    if (kind is Union2.U1<*, *>) {
        return "not valid UTF-8"
    }
    return "other"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun workflow(fs: Fs, console: Console, streams: salvo.stream.Streams) {
    val wrote = writeStr(fs, streams, "notes.txt", "alpha\nbeta\ngamma\n")
    when (wrote) {
        is Union2.U1<*, *> -> {
            println(console, "wrote ${(wrote.value as Long)} bytes")
        }
        is Union2.U2<*, *> -> {
            println(console, "write failed: ${kindName(detach((wrote.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val text = readToStr(fs, streams, "notes.txt")
    when (text) {
        is Union2.U1<*, *> -> {
            println(console, "read back ${(text.value as String).toByteArray(Charsets.UTF_8).size.toLong()} bytes")
        }
        is Union2.U2<*, *> -> {
            println(console, "read failed: ${kindName(detach((text.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val opened = fs.openRead("notes.txt")
    when (opened) {
        is Union2.U1<*, *> -> {
            val p = lines__2((opened.value as InStream))
            while (true) {
                val __loop1_step = next__21(streams, p)
                if (__loop1_step !is Union2.U1<String, Finished>) { break }
                val line = __loop1_step.value
                println(console, "line: $line")
            }
            val closed = close__2(streams, p)
            if (closed is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__2(detach((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "open failed: ${kindName(detach((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val out = fs.openAppend("notes.txt")
    when (out) {
        is Union2.U1<*, *> -> {
            val w: OutStream = (out.value as OutStream)
            val at = streams.position__2(w)
            val n = streams.writeLine(w, "delta")
            println(console, "appended $n bytes at offset $at")
            val shut = streams.close__2(w)
            if (shut is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__2(detach((shut.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
            val resumed = fs.openReadAt("notes.txt", at)
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
                    val done = streams.close(s)
                    if (done is Union2.U2<*, *>) {
                        println(console, "close failed: ${kindName__2(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                    }
                }
                is Union2.U2<*, *> -> {
                    println(console, "reopen failed: ${kindName(detach((resumed.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
                }
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "append failed: ${kindName(detach((out.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val bin = fs.openWrite("raw.bin")
    when (bin) {
        is Union2.U1<*, *> -> {
            val w: OutStream = (bin.value as OutStream)
            val data = bytesOf(arrayOf((0).toUByte(), (255).toUByte(), (200).toUByte()))
            val n = streams.writeBytes(w, data)
            val m = streams.write(w, "hé")
            println(console, "wrote $n raw bytes and $m encoded")
            val shut = streams.close__2(w)
            if (shut is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__2(detach((shut.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "raw open failed: ${kindName(detach((bin.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val raw = fs.openRead("raw.bin")
    when (raw) {
        is Union2.U1<*, *> -> {
            val s: InStream = (raw.value as InStream)
            val head = streams.readBytes(s, 3)
            when (head) {
                is Union2.U1<*, *> -> {
                    println(console, "first three: ${toStrPlatform((head.value as salvo.platform.core.bytes.Bytes))} = ${toHexPlatform((head.value as salvo.platform.core.bytes.Bytes))}")
                }
                is Union2.U2<*, *> -> {
                    println(console, "byte read failed: ${kindName__2(detach((head.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                }
            }
            val tail = streams.readAll(s)
            when (tail) {
                is Union2.U1<*, *> -> {
                    println(console, "the rest, as text: ${(tail.value as String)}")
                }
                is Union2.U2<*, *> -> {
                    println(console, "decode failed: ${kindName__2(detach((tail.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                }
            }
            val done = streams.close(s)
            if (done is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__2(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "raw read failed: ${kindName(detach((raw.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val split = fs.openReadAt("raw.bin", 5L)
    when (split) {
        is Union2.U1<*, *> -> {
            val s: InStream = (split.value as InStream)
            val broken = streams.readAll(s)
            when (broken) {
                is Union2.U1<*, *> -> {
                    println(console, "unexpected: ${(broken.value as String)} decoded")
                }
                is Union2.U2<*, *> -> {
                    println(console, "mid-character: ${kindName__2(detach((broken.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                }
            }
            val done = streams.close(s)
            when (done) {
                is Union2.U1<*, *> -> {
                    println(console, "unexpected: the failure was not recorded")
                }
                is Union2.U2<*, *> -> {
                    println(console, "and again at close: ${kindName__2(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                }
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "split open failed: ${kindName(detach((split.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val held = fs.openRead("raw.bin")
    when (held) {
        is Union2.U1<*, *> -> {
            val s: InStream = (held.value as InStream)
            val buf = mutBytes(arrayOf())
            var steps = 0
            var moved = 0
            var reading = true
            while (reading) {
                clearPlatform(buf)
                val got = streams.readTo(s, buf, 4)
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
                        println(console, "fill failed: ${kindName__2(detach((got.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
                        reading = false
                    }
                }
            }
            println(console, "filled $moved bytes in $steps reads, one buffer")
            val done = streams.close(s)
            if (done is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__2(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "fill open failed: ${kindName(detach((held.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val lined = fs.openRead("notes.txt")
    when (lined) {
        is Union2.U1<*, *> -> {
            val s: InStream = (lined.value as InStream)
            val line = StringBuilder()
            var longest = 0
            var reading = true
            while (reading) {
                line.clear()
                if (streams.readLineTo(s, line)) {
                    if (line.toString().length > longest) {
                        longest = line.toString().length
                    }
                } else {
                    reading = false
                }
            }
            println(console, "longest line: $longest characters")
            val done = streams.close(s)
            if (done is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__2(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "lines open failed: ${kindName(detach((lined.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val ch = openChunks(fs, streams, "raw.bin", 4)
    when (ch) {
        is Union2.U1<*, *> -> {
            val p = (ch.value as Chunks)
            var seen = 0
            while (true) {
                val __loop2_step = next__22(streams, p)
                if (__loop2_step !is Union2.U1<salvo.platform.core.bytes.Bytes, Finished>) { break }
                val chunk = __loop2_step.value
                seen = seen + sizePlatform(chunk)
            }
            println(console, "chunks saw $seen bytes")
            val done = close__3(streams, p)
            if (done is Union2.U2<*, *>) {
                println(console, "close failed: ${kindName__2(detach((done.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "chunks failed: ${kindName(detach((ch.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val copied = copyFile(fs, streams, "notes.txt", "notes-copy.txt")
    when (copied) {
        is Union2.U1<*, *> -> {
            println(console, "copied ${(copied.value as Long)} bytes")
        }
        is Union2.U2<*, *> -> {
            println(console, "copy failed: ${kindName(detach((copied.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val whole = readToBytes(fs, streams, "raw.bin")
    when (whole) {
        is Union2.U1<*, *> -> {
            println(console, "raw.bin is ${sizePlatform((whole.value as salvo.platform.core.bytes.Bytes))} bytes: ${toHexPlatform((whole.value as salvo.platform.core.bytes.Bytes))}")
        }
        is Union2.U2<*, *> -> {
            println(console, "byte read failed: ${kindName(detach((whole.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val failures: MutableList<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = mutableListOf<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>()
    val missing = readToStr(fs, streams, "nope.txt")
    when (missing) {
        is Union2.U1<*, *> -> {
            println(console, "unexpected: ${(missing.value as String)}")
        }
        is Union2.U2<*, *> -> {
            failures.add(detach((missing.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))
        }
    }
    val notADir = fs.listDir("notes.txt")
    when (notADir) {
        is Union2.U1<*, *> -> {
            println(console, "unexpected: ${(notADir.value as List<String>).joinToString(", ", "[", "]")}")
        }
        is Union2.U2<*, *> -> {
            failures.add(detach((notADir.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))
        }
    }
    println(console, "failures: ${failures.size}")
    for (kind in failures) {
        println(console, "  ${kindName(kind)}")
    }
    for (name in listOf<String>("notes.txt", "notes-copy.txt", "raw.bin")) {
        val gone = fs.delete(name)
        if (gone is Union2.U2<*, *>) {
            println(console, "delete failed: ${kindName(detach((gone.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    println(console, "cleaned up")
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun sandboxEdges(fs: Fs, console: Console, streams: salvo.stream.Streams) {
    val inside = writeStr(fs, streams, "sub/../probe.txt", "inside\n")
    when (inside) {
        is Union2.U1<*, *> -> {
            println(console, "through `..`: wrote ${(inside.value as Long)} bytes")
        }
        is Union2.U2<*, *> -> {
            println(console, "through `..`: ${kindName(detach((inside.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val up = readToStr(fs, streams, "../secret.txt")
    when (up) {
        is Union2.U1<*, *> -> {
            println(console, "unexpected: read outside the sandbox")
        }
        is Union2.U2<*, *> -> {
            println(console, "climbing out: ${kindName(detach((up.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val absolute = readToStr(fs, streams, "/etc/hosts")
    when (absolute) {
        is Union2.U1<*, *> -> {
            println(console, "unexpected: an absolute path resolved")
        }
        is Union2.U2<*, *> -> {
            println(console, "absolute path: ${kindName(detach((absolute.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
    val probe = "probe.txt"
    println(console, "probe still there: ${fs.exists(probe)}")
    val gone = fs.delete(probe)
    if (gone is Union2.U2<*, *>) {
        println(console, "delete failed: ${kindName(detach((gone.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
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
    val root = "tmp/files-example"
    val made = fs.createDirs(root)
    if (made is Union2.U2<*, *>) {
        println(console, "cannot create the working directory: ${kindName(detach((made.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
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
        println(console, "cleanup failed: ${kindName(detach((gone.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
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
