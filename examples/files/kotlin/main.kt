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
fun<__Fx> workflow(__fx: __Fx) where __Fx : __Has_Fs, __Fx : __Has_Console {
    val wrote = write_str(__fx, "notes.txt", "alpha\nbeta\ngamma\n")
    when (wrote) {
        is U2_1<*, *> -> {
            println(__fx, "wrote ${(wrote.value as Long)} bytes")
        }
        is U2_2<*, *> -> {
            println(__fx, "write failed: ${kind_name(detach((wrote.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val text = read_to_str(__fx, "notes.txt")
    when (text) {
        is U2_1<*, *> -> {
            println(__fx, "read back ${(text.value as String).toByteArray(Charsets.UTF_8).size.toLong()} bytes")
        }
        is U2_2<*, *> -> {
            println(__fx, "read failed: ${kind_name(detach((text.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val opened = __fx.__fx_Fs.open_read("notes.txt")
    when (opened) {
        is U2_1<*, *> -> {
            val p = lines((opened.value as InStream))
            while (true) {
                val __loop1_step = next__11(__fx, p)
                if (__loop1_step !is U2_1<String, Finished>) { break }
                val line = __loop1_step.value
                println(__fx, "line: $line")
            }
            val closed = close(__fx, p)
            if (closed is U2_2<*, *>) {
                println(__fx, "close failed: ${kind_name(detach((closed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(__fx, "open failed: ${kind_name(detach((opened.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val out = __fx.__fx_Fs.open_append("notes.txt")
    when (out) {
        is U2_1<*, *> -> {
            val w: OutStream = (out.value as OutStream)
            val at = __fx.__fx_Fs.position__2(w)
            val n = __fx.__fx_Fs.write_line(w, "delta")
            println(__fx, "appended $n bytes at offset $at")
            val shut = __fx.__fx_Fs.close__2(w)
            if (shut is U2_2<*, *>) {
                println(__fx, "close failed: ${kind_name(detach((shut.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
            val resumed = __fx.__fx_Fs.open_read_at("notes.txt", at)
            when (resumed) {
                is U2_1<*, *> -> {
                    val s: InStream = (resumed.value as InStream)
                    val line = __fx.__fx_Fs.read_line(s)
                    when {
                        line != null -> {
                            println(__fx, "at $at: $line")
                        }
                        else -> {
                            println(__fx, "at $at: end of file")
                        }
                    }
                    val done = __fx.__fx_Fs.close(s)
                    if (done is U2_2<*, *>) {
                        println(__fx, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                    }
                }
                is U2_2<*, *> -> {
                    println(__fx, "reopen failed: ${kind_name(detach((resumed.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
        }
        is U2_2<*, *> -> {
            println(__fx, "append failed: ${kind_name(detach((out.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val bin = __fx.__fx_Fs.open_write("raw.bin")
    when (bin) {
        is U2_1<*, *> -> {
            val w: OutStream = (bin.value as OutStream)
            val data = salvo.SalvoBytes.of(arrayOf<UByte>((0).toUByte(), (255).toUByte(), (200).toUByte()))
            val n = __fx.__fx_Fs.write_bytes(w, data)
            val m = __fx.__fx_Fs.write(w, "hé")
            println(__fx, "wrote $n raw bytes and $m encoded")
            val shut = __fx.__fx_Fs.close__2(w)
            if (shut is U2_2<*, *>) {
                println(__fx, "close failed: ${kind_name(detach((shut.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(__fx, "raw open failed: ${kind_name(detach((bin.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val raw = __fx.__fx_Fs.open_read("raw.bin")
    when (raw) {
        is U2_1<*, *> -> {
            val s: InStream = (raw.value as InStream)
            val head = __fx.__fx_Fs.read_bytes(s, 3)
            when (head) {
                is U2_1<*, *> -> {
                    println(__fx, "first three: ${(head.value as salvo.SalvoBytes).toString()} = ${(head.value as salvo.SalvoBytes).toHex()}")
                }
                is U2_2<*, *> -> {
                    println(__fx, "byte read failed: ${kind_name(detach((head.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
            val tail = __fx.__fx_Fs.read_all(s)
            when (tail) {
                is U2_1<*, *> -> {
                    println(__fx, "the rest, as text: ${(tail.value as String)}")
                }
                is U2_2<*, *> -> {
                    println(__fx, "decode failed: ${kind_name(detach((tail.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
            val done = __fx.__fx_Fs.close(s)
            if (done is U2_2<*, *>) {
                println(__fx, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(__fx, "raw read failed: ${kind_name(detach((raw.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val split = __fx.__fx_Fs.open_read_at("raw.bin", 5L)
    when (split) {
        is U2_1<*, *> -> {
            val s: InStream = (split.value as InStream)
            val broken = __fx.__fx_Fs.read_all(s)
            when (broken) {
                is U2_1<*, *> -> {
                    println(__fx, "unexpected: ${(broken.value as String)} decoded")
                }
                is U2_2<*, *> -> {
                    println(__fx, "mid-character: ${kind_name(detach((broken.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
            val done = __fx.__fx_Fs.close(s)
            when (done) {
                is U2_1<*, *> -> {
                    println(__fx, "unexpected: the failure was not recorded")
                }
                is U2_2<*, *> -> {
                    println(__fx, "and again at close: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                }
            }
        }
        is U2_2<*, *> -> {
            println(__fx, "split open failed: ${kind_name(detach((split.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val held = __fx.__fx_Fs.open_read("raw.bin")
    when (held) {
        is U2_1<*, *> -> {
            val s: InStream = (held.value as InStream)
            val buf = salvo.SalvoBytes.joined()
            var steps = 0
            var moved = 0
            var reading = true
            while (reading) {
                buf.clear()
                val got = __fx.__fx_Fs.read_to(s, buf, 4)
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
                        println(__fx, "fill failed: ${kind_name(detach((got.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
                        reading = false
                    }
                }
            }
            println(__fx, "filled $moved bytes in $steps reads, one buffer")
            val done = __fx.__fx_Fs.close(s)
            if (done is U2_2<*, *>) {
                println(__fx, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(__fx, "fill open failed: ${kind_name(detach((held.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val lined = __fx.__fx_Fs.open_read("notes.txt")
    when (lined) {
        is U2_1<*, *> -> {
            val s: InStream = (lined.value as InStream)
            val line = StringBuilder()
            var longest = 0
            var reading = true
            while (reading) {
                line.clear()
                if (__fx.__fx_Fs.read_line_to(s, line)) {
                    if (line.toString().length > longest) {
                        longest = line.toString().length
                    }
                } else {
                    reading = false
                }
            }
            println(__fx, "longest line: $longest characters")
            val done = __fx.__fx_Fs.close(s)
            if (done is U2_2<*, *>) {
                println(__fx, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(__fx, "lines open failed: ${kind_name(detach((lined.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val ch = open_chunks(__fx, "raw.bin", 4)
    when (ch) {
        is U2_1<*, *> -> {
            val p = (ch.value as Chunks)
            var seen = 0
            while (true) {
                val __loop2_step = next__12(__fx, p)
                if (__loop2_step !is U2_1<salvo.SalvoBytes, Finished>) { break }
                val chunk = __loop2_step.value
                seen = seen + chunk.size
            }
            println(__fx, "pass saw $seen bytes")
            val done = close__2(__fx, p)
            if (done is U2_2<*, *>) {
                println(__fx, "close failed: ${kind_name(detach((done.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
            }
        }
        is U2_2<*, *> -> {
            println(__fx, "chunks failed: ${kind_name(detach((ch.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val copied = copy_file(__fx, "notes.txt", "notes-copy.txt")
    when (copied) {
        is U2_1<*, *> -> {
            println(__fx, "copied ${(copied.value as Long)} bytes")
        }
        is U2_2<*, *> -> {
            println(__fx, "copy failed: ${kind_name(detach((copied.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val whole = read_to_bytes(__fx, "raw.bin")
    when (whole) {
        is U2_1<*, *> -> {
            println(__fx, "raw.bin is ${(whole.value as salvo.SalvoBytes).size} bytes: ${(whole.value as salvo.SalvoBytes).toHex()}")
        }
        is U2_2<*, *> -> {
            println(__fx, "byte read failed: ${kind_name(detach((whole.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val failures: MutableList<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> = mutableListOf<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>()
    val missing = read_to_str(__fx, "nope.txt")
    when (missing) {
        is U2_1<*, *> -> {
            println(__fx, "unexpected: ${(missing.value as String)}")
        }
        is U2_2<*, *> -> {
            failures.add(detach((missing.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))
        }
    }
    val not_a_dir = __fx.__fx_Fs.list_dir("notes.txt")
    when (not_a_dir) {
        is U2_1<*, *> -> {
            println(__fx, "unexpected: ${(not_a_dir.value as List<String>).joinToString(", ", "[", "]")}")
        }
        is U2_2<*, *> -> {
            failures.add(detach((not_a_dir.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))
        }
    }
    println(__fx, "failures: ${failures.size}")
    for (kind in failures) {
        println(__fx, "  ${kind_name(kind)}")
    }
    for (name in listOf<String>("notes.txt", "notes-copy.txt", "raw.bin")) {
        val gone = __fx.__fx_Fs.delete(name)
        if (gone is U2_2<*, *>) {
            println(__fx, "delete failed: ${kind_name(detach((gone.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    println(__fx, "cleaned up")
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<__Fx> sandbox_edges(__fx: __Fx) where __Fx : __Has_Fs, __Fx : __Has_Console {
    val inside = write_str(__fx, "sub/../probe.txt", "inside\n")
    when (inside) {
        is U2_1<*, *> -> {
            println(__fx, "through `..`: wrote ${(inside.value as Long)} bytes")
        }
        is U2_2<*, *> -> {
            println(__fx, "through `..`: ${kind_name(detach((inside.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val up = read_to_str(__fx, "../secret.txt")
    when (up) {
        is U2_1<*, *> -> {
            println(__fx, "unexpected: read outside the sandbox")
        }
        is U2_2<*, *> -> {
            println(__fx, "climbing out: ${kind_name(detach((up.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val absolute = read_to_str(__fx, "/etc/hosts")
    when (absolute) {
        is U2_1<*, *> -> {
            println(__fx, "unexpected: an absolute path resolved")
        }
        is U2_2<*, *> -> {
            println(__fx, "absolute path: ${kind_name(detach((absolute.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        }
    }
    val probe = "probe.txt"
    println(__fx, "probe still there: ${__fx.__fx_Fs.exists(probe)}")
    val gone = __fx.__fx_Fs.delete(probe)
    if (gone is U2_2<*, *>) {
        println(__fx, "delete failed: ${kind_name(detach((gone.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun main() {
    val __fx = __Fx_1(StdOutConsole())
    val __fx2 = __Fx_2(__fx.__fx_Console, salvo.platform.fs.host.HostRawFs())
    val __fx3 = __Fx_3(__fx2.__fx_Console, DefaultFs(__fx2), __fx2.__fx_RawFs)
    val root = "tmp/files-example"
    val made = __fx3.__fx_Fs.create_dirs(root)
    if (made is U2_2<*, *>) {
        println(__fx3, "cannot create the working directory: ${kind_name(detach((made.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
        return
    }
    println(__fx3, "-- the real filesystem, scoped to one directory --")
    if (true) {
        val __fx4 = __Fx_3(__fx3.__fx_Console, RestrictedFs(root, __fx3), __fx3.__fx_RawFs)
        workflow(__fx4)
        sandbox_edges(__fx4)
    }
    val gone = __fx3.__fx_Fs.delete(root)
    if (gone is U2_2<*, *>) {
        println(__fx3, "cleanup failed: ${kind_name(detach((gone.value as Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>)))}")
    }
    println(__fx3, "-- the same code, with no disk at all --")
    if (true) {
        val __fx5 = __Fx_3(__fx3.__fx_Console, __Mon_Fs(MemFs()), __fx3.__fx_RawFs)
        workflow(__fx5)
    }
}

class __Fx_1(
    override val __fx_Console: Console,
) : __Has_Console

class __Fx_2(
    override val __fx_Console: Console,
    override val __fx_RawFs: RawFs,
) : __Has_Console, __Has_RawFs

class __Fx_3(
    override val __fx_Console: Console,
    override val __fx_Fs: Fs,
    override val __fx_RawFs: RawFs,
) : __Has_Console, __Has_Fs, __Has_RawFs
