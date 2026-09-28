package salvo.main

import salvo.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.console.*
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

fun kind_name(kind: Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>): String {
    if (kind is U8_1<*, *, *, *, *, *, *, *>) {
        return "not found"
    }
    if (kind is U8_4<*, *, *, *, *, *, *, *>) {
        return "not a directory"
    }
    if (kind is U8_5<*, *, *, *, *, *, *, *>) {
        return "escapes the sandbox"
    }
    if (kind is U8_6<*, *, *, *, *, *, *, *>) {
        return "not valid UTF-8"
    }
    return "other"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun workflow(fs: Fs, console: Console) {
    val wrote = write_str(fs, "notes.txt", "alpha\nbeta\ngamma\n")
    when (wrote) {
        is U2_1<*, *> -> {
            println(console, "wrote ${(wrote.value as Long)} bytes")
        }
        is U2_2<*, *> -> {
            println(console, "write failed: ${kind_name(detach((wrote.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val text = read_to_str(fs, "notes.txt")
    when (text) {
        is U2_1<*, *> -> {
            println(console, "read back ${(text.value as String).toByteArray(Charsets.UTF_8).size.toLong()} bytes")
        }
        is U2_2<*, *> -> {
            println(console, "read failed: ${kind_name(detach((text.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val opened = fs.open_read("notes.txt")
    when (opened) {
        is U2_1<*, *> -> {
            val p = lines((opened.value as InStream))
            while (true) {
                val __loop1_step = next__13(fs, p)
                if (__loop1_step !is U2_1<String, Finished>) { break }
                val line = __loop1_step.value
                println(console, "line: $line")
            }
            val closed = close(fs, p)
            if (closed is U2_2<*, *>) {
                println(console, "close failed: ${kind_name(detach((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(console, "open failed: ${kind_name(detach((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val out = fs.open_append("notes.txt")
    when (out) {
        is U2_1<*, *> -> {
            val w: OutStream = (out.value as OutStream)
            val at = fs.position__2(w)
            val n = fs.write_line(w, "delta")
            println(console, "appended $n bytes at offset $at")
            val shut = fs.close__2(w)
            if (shut is U2_2<*, *>) {
                println(console, "close failed: ${kind_name(detach((shut.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
            val resumed = fs.open_read_at("notes.txt", at)
            when (resumed) {
                is U2_1<*, *> -> {
                    val s: InStream = (resumed.value as InStream)
                    val line = fs.read_line(s)
                    when {
                        line != null -> {
                            println(console, "at $at: $line")
                        }
                        else -> {
                            println(console, "at $at: end of file")
                        }
                    }
                    val done = fs.close(s)
                    if (done is U2_2<*, *>) {
                        println(console, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                    }
                }
                is U2_2<*, *> -> {
                    println(console, "reopen failed: ${kind_name(detach((resumed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
        }
        is U2_2<*, *> -> {
            println(console, "append failed: ${kind_name(detach((out.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val bin = fs.open_write("raw.bin")
    when (bin) {
        is U2_1<*, *> -> {
            val w: OutStream = (bin.value as OutStream)
            val data = salvo.SalvoBytes.of(arrayOf<UByte>((0).toUByte(), (255).toUByte(), (200).toUByte()))
            val n = fs.write_bytes(w, data)
            val m = fs.write(w, "hé")
            println(console, "wrote $n raw bytes and $m encoded")
            val shut = fs.close__2(w)
            if (shut is U2_2<*, *>) {
                println(console, "close failed: ${kind_name(detach((shut.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(console, "raw open failed: ${kind_name(detach((bin.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val raw = fs.open_read("raw.bin")
    when (raw) {
        is U2_1<*, *> -> {
            val s: InStream = (raw.value as InStream)
            val head = fs.read_bytes(s, 3)
            when (head) {
                is U2_1<*, *> -> {
                    println(console, "first three: ${(head.value as salvo.SalvoBytes).toString()} = ${(head.value as salvo.SalvoBytes).toHex()}")
                }
                is U2_2<*, *> -> {
                    println(console, "byte read failed: ${kind_name(detach((head.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
            val tail = fs.read_all(s)
            when (tail) {
                is U2_1<*, *> -> {
                    println(console, "the rest, as text: ${(tail.value as String)}")
                }
                is U2_2<*, *> -> {
                    println(console, "decode failed: ${kind_name(detach((tail.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
            val done = fs.close(s)
            if (done is U2_2<*, *>) {
                println(console, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(console, "raw read failed: ${kind_name(detach((raw.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val split = fs.open_read_at("raw.bin", 5L)
    when (split) {
        is U2_1<*, *> -> {
            val s: InStream = (split.value as InStream)
            val broken = fs.read_all(s)
            when (broken) {
                is U2_1<*, *> -> {
                    println(console, "unexpected: ${(broken.value as String)} decoded")
                }
                is U2_2<*, *> -> {
                    println(console, "mid-character: ${kind_name(detach((broken.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
            val done = fs.close(s)
            when (done) {
                is U2_1<*, *> -> {
                    println(console, "unexpected: the failure was not recorded")
                }
                is U2_2<*, *> -> {
                    println(console, "and again at close: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
        }
        is U2_2<*, *> -> {
            println(console, "split open failed: ${kind_name(detach((split.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val held = fs.open_read("raw.bin")
    when (held) {
        is U2_1<*, *> -> {
            val s: InStream = (held.value as InStream)
            val buf = salvo.SalvoBytes.joined()
            var steps = 0
            var moved = 0
            var reading = true
            while (reading) {
                buf.clear()
                val got = fs.read_to(s, buf, 4)
                when (got) {
                    is U2_1<*, *> -> {
                        val n: Int = (got.value as Int)
                        if (n == 0) {
                            reading = false
                        } else {
                            steps = steps + 1
                            moved = moved + n
                        }
                    }
                    is U2_2<*, *> -> {
                        println(console, "fill failed: ${kind_name(detach((got.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                        reading = false
                    }
                }
            }
            println(console, "filled $moved bytes in $steps reads, one buffer")
            val done = fs.close(s)
            if (done is U2_2<*, *>) {
                println(console, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(console, "fill open failed: ${kind_name(detach((held.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val lined = fs.open_read("notes.txt")
    when (lined) {
        is U2_1<*, *> -> {
            val s: InStream = (lined.value as InStream)
            val line = StringBuilder()
            var longest = 0
            var reading = true
            while (reading) {
                line.clear()
                if (fs.read_line_to(s, line)) {
                    if (line.toString().length > longest) {
                        longest = line.toString().length
                    }
                } else {
                    reading = false
                }
            }
            println(console, "longest line: $longest characters")
            val done = fs.close(s)
            if (done is U2_2<*, *>) {
                println(console, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(console, "lines open failed: ${kind_name(detach((lined.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val ch = open_chunks(fs, "raw.bin", 4)
    when (ch) {
        is U2_1<*, *> -> {
            val p = (ch.value as Chunks)
            var seen = 0
            while (true) {
                val __loop2_step = next__14(fs, p)
                if (__loop2_step !is U2_1<salvo.SalvoBytes, Finished>) { break }
                val chunk = __loop2_step.value
                seen = seen + chunk.size
            }
            println(console, "chunks saw $seen bytes")
            val done = close__2(fs, p)
            if (done is U2_2<*, *>) {
                println(console, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(console, "chunks failed: ${kind_name(detach((ch.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val copied = copy_file(fs, "notes.txt", "notes-copy.txt")
    when (copied) {
        is U2_1<*, *> -> {
            println(console, "copied ${(copied.value as Long)} bytes")
        }
        is U2_2<*, *> -> {
            println(console, "copy failed: ${kind_name(detach((copied.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val whole = read_to_bytes(fs, "raw.bin")
    when (whole) {
        is U2_1<*, *> -> {
            println(console, "raw.bin is ${(whole.value as salvo.SalvoBytes).size} bytes: ${(whole.value as salvo.SalvoBytes).toHex()}")
        }
        is U2_2<*, *> -> {
            println(console, "byte read failed: ${kind_name(detach((whole.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val failures: MutableList<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> = mutableListOf<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>()
    val missing = read_to_str(fs, "nope.txt")
    when (missing) {
        is U2_1<*, *> -> {
            println(console, "unexpected: ${(missing.value as String)}")
        }
        is U2_2<*, *> -> {
            failures.add(detach((missing.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))
        }
    }
    val not_a_dir = fs.list_dir("notes.txt")
    when (not_a_dir) {
        is U2_1<*, *> -> {
            println(console, "unexpected: ${(not_a_dir.value as List<String>).joinToString(", ", "[", "]")}")
        }
        is U2_2<*, *> -> {
            failures.add(detach((not_a_dir.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))
        }
    }
    println(console, "failures: ${failures.size}")
    for (kind in failures) {
        println(console, "  ${kind_name(kind)}")
    }
    for (name in listOf<String>("notes.txt", "notes-copy.txt", "raw.bin")) {
        val gone = fs.delete(name)
        if (gone is U2_2<*, *>) {
            println(console, "delete failed: ${kind_name(detach((gone.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    println(console, "cleaned up")
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun sandbox_edges(fs: Fs, console: Console) {
    val inside = write_str(fs, "sub/../probe.txt", "inside\n")
    when (inside) {
        is U2_1<*, *> -> {
            println(console, "through `..`: wrote ${(inside.value as Long)} bytes")
        }
        is U2_2<*, *> -> {
            println(console, "through `..`: ${kind_name(detach((inside.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val up = read_to_str(fs, "../secret.txt")
    when (up) {
        is U2_1<*, *> -> {
            println(console, "unexpected: read outside the sandbox")
        }
        is U2_2<*, *> -> {
            println(console, "climbing out: ${kind_name(detach((up.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val absolute = read_to_str(fs, "/etc/hosts")
    when (absolute) {
        is U2_1<*, *> -> {
            println(console, "unexpected: an absolute path resolved")
        }
        is U2_2<*, *> -> {
            println(console, "absolute path: ${kind_name(detach((absolute.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val probe = "probe.txt"
    println(console, "probe still there: ${fs.exists(probe)}")
    val gone = fs.delete(probe)
    if (gone is U2_2<*, *>) {
        println(console, "delete failed: ${kind_name(detach((gone.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun main() {
    val console: Console = StdOutConsole()
    val raw_fs: RawFs = __Mon_RawFs(salvo.platform.fs.host.HostRawFs())
    val fs: Fs = DefaultFs(raw_fs)
    val root = "tmp/files-example"
    val made = fs.create_dirs(root)
    if (made is U2_2<*, *>) {
        println(console, "cannot create the working directory: ${kind_name(detach((made.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        return
    }
    println(console, "-- the real filesystem, scoped to one directory --")
    if (true) {
        val fs2: Fs = RestrictedFs(root, fs)
        workflow(fs2, console)
        sandbox_edges(fs2, console)
    }
    val gone = fs.delete(root)
    if (gone is U2_2<*, *>) {
        println(console, "cleanup failed: ${kind_name(detach((gone.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
    }
    println(console, "-- the same code, with no disk at all --")
    if (true) {
        val fs3: Fs = __Mon_Fs(MemFs())
        workflow(fs3, console)
    }
}
