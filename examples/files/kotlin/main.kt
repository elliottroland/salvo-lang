package salvo.main

import salvo.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun kindName__FsError(kind: Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>): String {
    if ((kind is Union7.U1<*, *, *, *, *, *, *>)) {
        val kind_1: salvo.fs.NotFound = ((kind as Union7.U1<*, *, *, *, *, *, *>).value as salvo.fs.NotFound)
        return "not found"
    }
    if ((kind is Union7.U4<*, *, *, *, *, *, *>)) {
        val kind_2: salvo.fs.NotADirectory = ((kind as Union7.U4<*, *, *, *, *, *, *>).value as salvo.fs.NotADirectory)
        return "not a directory"
    }
    if ((kind is Union7.U5<*, *, *, *, *, *, *>)) {
        val kind_3: salvo.fs.PathEscapes = ((kind as Union7.U5<*, *, *, *, *, *, *>).value as salvo.fs.PathEscapes)
        return "escapes the sandbox"
    }
    if ((kind is Union7.U7<*, *, *, *, *, *, *>)) {
        val kind_4: salvo.fs.Streaming = ((kind as Union7.U7<*, *, *, *, *, *, *>).value as salvo.fs.Streaming)
        return kindName__StreamError(kind_4.error)
    }
    return "other"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun kindName__StreamError(kind: Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>): String {
    if ((kind is Union2.U1<*, *>)) {
        val kind_1: salvo.stream.InvalidUtf8 = ((kind as Union2.U1<*, *>).value as salvo.stream.InvalidUtf8)
        return "not valid UTF-8"
    }
    return "other"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun workflow(fs: salvo.fs.Fs, console: salvo.core.console.Console, streams: salvo.stream.Streams) {
    val wrote: Union2<Long, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = salvo.fs.writeStr(fs, streams, salvo.fs.path.path("notes.txt"), "alpha\nbeta\ngamma\n")
    when {
        (wrote is Union2.U1<*, *>) -> {
            val wrote_1: Long = ((wrote as Union2.U1<*, *>).value as Long)
            salvo.core.console.println(console, "wrote ${wrote_1} bytes")
        }
        (wrote is Union2.U2<*, *>) -> {
            val wrote_2: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((wrote as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "write failed: ${kindName__FsError(salvo.core.checked.detach(wrote_2))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val text: Union2<String, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = salvo.fs.readToStr(fs, streams, salvo.fs.path.path("notes.txt"))
    when {
        (text is Union2.U1<*, *>) -> {
            val text_3: String = ((text as Union2.U1<*, *>).value as String)
            salvo.core.console.println(console, "read back ${salvo.core.string.byteSizePlatform(text_3)} bytes")
        }
        (text is Union2.U2<*, *>) -> {
            val text_4: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((text as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "read failed: ${kindName__FsError(salvo.core.checked.detach(text_4))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val opened: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.openRead(salvo.fs.path.path("notes.txt"))
    when {
        (opened is Union2.U1<*, *>) -> {
            val opened_5: salvo.stream.InStream = ((opened as Union2.U1<*, *>).value as salvo.stream.InStream)
            val p: salvo.stream.Lines = salvo.stream.lines(opened_5)
            while (true) {
                val __step_7: Union2<String, salvo.core.iterator.Finished> = salvo.stream.next__Lines(streams, p)
                when {
                    (__step_7 is Union2.U1<*, *>) -> {
                        val __emitted_8: String = ((__step_7 as Union2.U1<*, *>).value as String)
                        val line: String = __emitted_8
                        salvo.core.console.println(console, "line: ${line}")
                    }
                    else -> {
                        break
                    }
                }
            }
            val closed: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = salvo.stream.close__Lines(streams, p)
            if ((closed is Union2.U2<*, *>)) {
                val closed_9: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                salvo.core.console.println(console, "close failed: ${kindName__StreamError(salvo.core.checked.detach(closed_9))}")
            }
        }
        (opened is Union2.U2<*, *>) -> {
            val opened_10: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "open failed: ${kindName__FsError(salvo.core.checked.detach(opened_10))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val out: Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.openAppend(salvo.fs.path.path("notes.txt"))
    when {
        (out is Union2.U1<*, *>) -> {
            val out_11: salvo.stream.OutStream = ((out as Union2.U1<*, *>).value as salvo.stream.OutStream)
            val w: salvo.stream.OutStream = out_11
            val at: Long = streams.position__OutStream(w)
            val n: Long = streams.writeLine(w, "delta")
            salvo.core.console.println(console, "appended ${n} bytes at offset ${at}")
            val shut: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__OutStream(w)
            if ((shut is Union2.U2<*, *>)) {
                val shut_12: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((shut as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                salvo.core.console.println(console, "close failed: ${kindName__StreamError(salvo.core.checked.detach(shut_12))}")
            }
            val resumed: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.openReadAt(salvo.fs.path.path("notes.txt"), at)
            when {
                (resumed is Union2.U1<*, *>) -> {
                    val resumed_13: salvo.stream.InStream = ((resumed as Union2.U1<*, *>).value as salvo.stream.InStream)
                    val s: salvo.stream.InStream = resumed_13
                    val line: String? = streams.readLine(s)
                    when {
                        (line != null) -> {
                            val line_14: String = line!!
                            salvo.core.console.println(console, "at ${at}: ${line_14}")
                        }
                        (line == null) -> {
                            salvo.core.console.println(console, "at ${at}: end of file")
                        }
                        else -> throw IllegalStateException("salvo: unreachable arm")
                    }
                    val done: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
                    if ((done is Union2.U2<*, *>)) {
                        val done_15: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                        salvo.core.console.println(console, "close failed: ${kindName__StreamError(salvo.core.checked.detach(done_15))}")
                    }
                }
                (resumed is Union2.U2<*, *>) -> {
                    val resumed_16: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((resumed as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
                    salvo.core.console.println(console, "reopen failed: ${kindName__FsError(salvo.core.checked.detach(resumed_16))}")
                }
                else -> throw IllegalStateException("salvo: unreachable arm")
            }
        }
        (out is Union2.U2<*, *>) -> {
            val out_17: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((out as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "append failed: ${kindName__FsError(salvo.core.checked.detach(out_17))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val bin: Union2<salvo.stream.OutStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.openWrite(salvo.fs.path.path("raw.bin"))
    when {
        (bin is Union2.U1<*, *>) -> {
            val bin_18: salvo.stream.OutStream = ((bin as Union2.U1<*, *>).value as salvo.stream.OutStream)
            val w: salvo.stream.OutStream = bin_18
            val data: salvo.platform.core.bytes.Bytes = salvo.core.bytes.bytesOf(arrayOf<UByte>((0).toUByte(), (255).toUByte(), (200).toUByte()))
            val n: Long = streams.writeBytes(w, data)
            val m: Long = streams.write(w, "hé")
            salvo.core.console.println(console, "wrote ${n} raw bytes and ${m} encoded")
            val shut: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__OutStream(w)
            if ((shut is Union2.U2<*, *>)) {
                val shut_19: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((shut as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                salvo.core.console.println(console, "close failed: ${kindName__StreamError(salvo.core.checked.detach(shut_19))}")
            }
        }
        (bin is Union2.U2<*, *>) -> {
            val bin_20: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((bin as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "raw open failed: ${kindName__FsError(salvo.core.checked.detach(bin_20))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val raw: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.openRead(salvo.fs.path.path("raw.bin"))
    when {
        (raw is Union2.U1<*, *>) -> {
            val raw_21: salvo.stream.InStream = ((raw as Union2.U1<*, *>).value as salvo.stream.InStream)
            val s: salvo.stream.InStream = raw_21
            val head: Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.readBytes(s, 3)
            when {
                (head is Union2.U1<*, *>) -> {
                    val head_22: salvo.platform.core.bytes.Bytes = ((head as Union2.U1<*, *>).value as salvo.platform.core.bytes.Bytes)
                    salvo.core.console.println(console, "first three: ${salvo.core.bytes.toStrPlatform(head_22)} = ${salvo.core.bytes.toHexPlatform(head_22)}")
                }
                (head is Union2.U2<*, *>) -> {
                    val head_23: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((head as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                    salvo.core.console.println(console, "byte read failed: ${kindName__StreamError(salvo.core.checked.detach(head_23))}")
                }
                else -> throw IllegalStateException("salvo: unreachable arm")
            }
            val tail: Union2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.readAll(s)
            when {
                (tail is Union2.U1<*, *>) -> {
                    val tail_24: String = ((tail as Union2.U1<*, *>).value as String)
                    salvo.core.console.println(console, "the rest, as text: ${tail_24}")
                }
                (tail is Union2.U2<*, *>) -> {
                    val tail_25: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((tail as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                    salvo.core.console.println(console, "decode failed: ${kindName__StreamError(salvo.core.checked.detach(tail_25))}")
                }
                else -> throw IllegalStateException("salvo: unreachable arm")
            }
            val done: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
            if ((done is Union2.U2<*, *>)) {
                val done_26: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                salvo.core.console.println(console, "close failed: ${kindName__StreamError(salvo.core.checked.detach(done_26))}")
            }
        }
        (raw is Union2.U2<*, *>) -> {
            val raw_27: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((raw as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "raw read failed: ${kindName__FsError(salvo.core.checked.detach(raw_27))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val split: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.openReadAt(salvo.fs.path.path("raw.bin"), 5L)
    when {
        (split is Union2.U1<*, *>) -> {
            val split_28: salvo.stream.InStream = ((split as Union2.U1<*, *>).value as salvo.stream.InStream)
            val s: salvo.stream.InStream = split_28
            val broken: Union2<String, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.readAll(s)
            when {
                (broken is Union2.U1<*, *>) -> {
                    val broken_29: String = ((broken as Union2.U1<*, *>).value as String)
                    salvo.core.console.println(console, "unexpected: ${broken_29} decoded")
                }
                (broken is Union2.U2<*, *>) -> {
                    val broken_30: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((broken as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                    salvo.core.console.println(console, "mid-character: ${kindName__StreamError(salvo.core.checked.detach(broken_30))}")
                }
                else -> throw IllegalStateException("salvo: unreachable arm")
            }
            val done: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
            when {
                (done is Union2.U1<*, *>) -> {
                    val done_31: Unit = ((done as Union2.U1<*, *>).value as Unit)
                    salvo.core.console.println(console, "unexpected: the failure was not recorded")
                }
                (done is Union2.U2<*, *>) -> {
                    val done_32: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                    salvo.core.console.println(console, "and again at close: ${kindName__StreamError(salvo.core.checked.detach(done_32))}")
                }
                else -> throw IllegalStateException("salvo: unreachable arm")
            }
        }
        (split is Union2.U2<*, *>) -> {
            val split_33: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((split as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "split open failed: ${kindName__FsError(salvo.core.checked.detach(split_33))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val held: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.openRead(salvo.fs.path.path("raw.bin"))
    when {
        (held is Union2.U1<*, *>) -> {
            val held_34: salvo.stream.InStream = ((held as Union2.U1<*, *>).value as salvo.stream.InStream)
            val s: salvo.stream.InStream = held_34
            val buf: salvo.platform.core.bytes.MutBytes = salvo.core.bytes.mutBytes(arrayOf<salvo.platform.core.bytes.Bytes>())
            var steps: Int = 0
            var moved: Int = 0
            var reading: Boolean = true
            while (true) {
                if (!(reading)) {
                    break
                }
                salvo.core.bytes.clearPlatform(buf)
                val got: Union2<Int, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.readTo__InStream_Bytes_Int(s, buf, 4)
                when {
                    (got is Union2.U1<*, *>) -> {
                        val got_35: Int = ((got as Union2.U1<*, *>).value as Int)
                        val n: Int = got_35
                        if (((n) == (0))) {
                            reading = false
                        } else {
                            steps = (steps + 1)
                            moved = (moved + n)
                        }
                    }
                    (got is Union2.U2<*, *>) -> {
                        val got_36: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                        salvo.core.console.println(console, "fill failed: ${kindName__StreamError(salvo.core.checked.detach(got_36))}")
                        reading = false
                    }
                    else -> throw IllegalStateException("salvo: unreachable arm")
                }
            }
            salvo.core.console.println(console, "filled ${moved} bytes in ${steps} reads, one buffer")
            val done: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
            if ((done is Union2.U2<*, *>)) {
                val done_37: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                salvo.core.console.println(console, "close failed: ${kindName__StreamError(salvo.core.checked.detach(done_37))}")
            }
        }
        (held is Union2.U2<*, *>) -> {
            val held_38: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((held as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "fill open failed: ${kindName__FsError(salvo.core.checked.detach(held_38))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val lined: Union2<salvo.stream.InStream, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.openRead(salvo.fs.path.path("notes.txt"))
    when {
        (lined is Union2.U1<*, *>) -> {
            val lined_39: salvo.stream.InStream = ((lined as Union2.U1<*, *>).value as salvo.stream.InStream)
            val s: salvo.stream.InStream = lined_39
            val line: salvo.platform.core.string.MutStr = salvo.core.string.mutStr(arrayOf<String>())
            var longest: Int = 0
            var reading: Boolean = true
            while (true) {
                if (!(reading)) {
                    break
                }
                salvo.core.string.clearPlatform(line)
                if (streams.readLineTo(s, line)) {
                    if ((salvo.core.string.sizePlatform(line.toString()) > longest)) {
                        longest = salvo.core.string.sizePlatform(line.toString())
                    }
                } else {
                    reading = false
                }
            }
            salvo.core.console.println(console, "longest line: ${longest} characters")
            val done: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = streams.close__InStream(s)
            if ((done is Union2.U2<*, *>)) {
                val done_40: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                salvo.core.console.println(console, "close failed: ${kindName__StreamError(salvo.core.checked.detach(done_40))}")
            }
        }
        (lined is Union2.U2<*, *>) -> {
            val lined_41: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((lined as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "lines open failed: ${kindName__FsError(salvo.core.checked.detach(lined_41))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val ch: Union2<salvo.stream.Chunks, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = salvo.fs.openChunks(fs, streams, salvo.fs.path.path("raw.bin"), 4)
    when {
        (ch is Union2.U1<*, *>) -> {
            val ch_42: salvo.stream.Chunks = ((ch as Union2.U1<*, *>).value as salvo.stream.Chunks)
            val p: salvo.stream.Chunks = ch_42
            var seen: Int = 0
            while (true) {
                val __step_44: Union2<salvo.platform.core.bytes.Bytes, salvo.core.iterator.Finished> = salvo.stream.next__Chunks(streams, p)
                when {
                    (__step_44 is Union2.U1<*, *>) -> {
                        val __emitted_45: salvo.platform.core.bytes.Bytes = ((__step_44 as Union2.U1<*, *>).value as salvo.platform.core.bytes.Bytes)
                        val chunk: salvo.platform.core.bytes.Bytes = __emitted_45
                        seen = (seen + salvo.core.bytes.sizePlatform(chunk))
                    }
                    else -> {
                        break
                    }
                }
            }
            salvo.core.console.println(console, "chunks saw ${seen} bytes")
            val done: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>> = salvo.stream.close__Chunks(streams, p)
            if ((done is Union2.U2<*, *>)) {
                val done_46: salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.stream.InvalidUtf8, salvo.stream.StreamFailed>>)
                salvo.core.console.println(console, "close failed: ${kindName__StreamError(salvo.core.checked.detach(done_46))}")
            }
        }
        (ch is Union2.U2<*, *>) -> {
            val ch_47: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((ch as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "chunks failed: ${kindName__FsError(salvo.core.checked.detach(ch_47))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val copied: Union2<Long, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = salvo.fs.copyFile(fs, streams, salvo.fs.path.path("notes.txt"), salvo.fs.path.path("notes-copy.txt"))
    when {
        (copied is Union2.U1<*, *>) -> {
            val copied_48: Long = ((copied as Union2.U1<*, *>).value as Long)
            salvo.core.console.println(console, "copied ${copied_48} bytes")
        }
        (copied is Union2.U2<*, *>) -> {
            val copied_49: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((copied as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "copy failed: ${kindName__FsError(salvo.core.checked.detach(copied_49))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val whole: Union2<salvo.platform.core.bytes.Bytes, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = salvo.fs.readToBytes(fs, streams, salvo.fs.path.path("raw.bin"))
    when {
        (whole is Union2.U1<*, *>) -> {
            val whole_50: salvo.platform.core.bytes.Bytes = ((whole as Union2.U1<*, *>).value as salvo.platform.core.bytes.Bytes)
            salvo.core.console.println(console, "raw.bin is ${salvo.core.bytes.sizePlatform(whole_50)} bytes: ${salvo.core.bytes.toHexPlatform(whole_50)}")
        }
        (whole is Union2.U2<*, *>) -> {
            val whole_51: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((whole as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "byte read failed: ${kindName__FsError(salvo.core.checked.detach(whole_51))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val failures: salvo.platform.core.list.MutList<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = mutableListOf<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>()
    val missing: Union2<String, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = salvo.fs.readToStr(fs, streams, salvo.fs.path.path("nope.txt"))
    when {
        (missing is Union2.U1<*, *>) -> {
            val missing_52: String = ((missing as Union2.U1<*, *>).value as String)
            salvo.core.console.println(console, "unexpected: ${missing_52}")
        }
        (missing is Union2.U2<*, *>) -> {
            val missing_53: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((missing as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.list.addPlatform(failures, salvo.core.checked.detach(missing_53))
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val notADir: Union2<List<String>, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.listDir(salvo.fs.path.path("notes.txt"))
    when {
        (notADir is Union2.U1<*, *>) -> {
            val notADir_54: List<String> = ((notADir as Union2.U1<*, *>).value as List<String>)
            salvo.core.console.println(console, "unexpected: ${salvo.core.list.toStr(notADir_54, { __a0 -> __a0 })}")
        }
        (notADir is Union2.U2<*, *>) -> {
            val notADir_55: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((notADir as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.list.addPlatform(failures, salvo.core.checked.detach(notADir_55))
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    salvo.core.console.println(console, "failures: ${salvo.core.list.sizePlatform(failures)}")
    for (kind in salvo.platform.core.list.each(failures)) {
        salvo.core.console.println(console, "  ${kindName__FsError(kind)}")
    }
    for (name in salvo.platform.core.list.each(listOf<String>("notes.txt", "notes-copy.txt", "raw.bin"))) {
        val gone: Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.delete(salvo.fs.path.path(name))
        if ((gone is Union2.U2<*, *>)) {
            val gone_56: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((gone as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "delete failed: ${kindName__FsError(salvo.core.checked.detach(gone_56))}")
        }
    }
    salvo.core.console.println(console, "cleaned up")
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun sandboxEdges(fs: salvo.fs.Fs, console: salvo.core.console.Console, streams: salvo.stream.Streams) {
    val inside: Union2<Long, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = salvo.fs.writeStr(fs, streams, salvo.fs.path.path("sub/../probe.txt"), "inside\n")
    when {
        (inside is Union2.U1<*, *>) -> {
            val inside_1: Long = ((inside as Union2.U1<*, *>).value as Long)
            salvo.core.console.println(console, "through `..`: wrote ${inside_1} bytes")
        }
        (inside is Union2.U2<*, *>) -> {
            val inside_2: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((inside as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "through `..`: ${kindName__FsError(salvo.core.checked.detach(inside_2))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val up: Union2<String, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = salvo.fs.readToStr(fs, streams, salvo.fs.path.path("../secret.txt"))
    when {
        (up is Union2.U1<*, *>) -> {
            val up_3: String = ((up as Union2.U1<*, *>).value as String)
            salvo.core.console.println(console, "unexpected: read outside the sandbox")
        }
        (up is Union2.U2<*, *>) -> {
            val up_4: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((up as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "climbing out: ${kindName__FsError(salvo.core.checked.detach(up_4))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val absolute: Union2<String, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = salvo.fs.readToStr(fs, streams, salvo.fs.path.path("/etc/hosts"))
    when {
        (absolute is Union2.U1<*, *>) -> {
            val absolute_5: String = ((absolute as Union2.U1<*, *>).value as String)
            salvo.core.console.println(console, "unexpected: an absolute path resolved")
        }
        (absolute is Union2.U2<*, *>) -> {
            val absolute_6: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((absolute as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
            salvo.core.console.println(console, "absolute path: ${kindName__FsError(salvo.core.checked.detach(absolute_6))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val probe: salvo.fs.path.Path = salvo.fs.path.path("probe.txt")
    salvo.core.console.println(console, "probe still there: ${fs.exists(probe)}")
    val gone: Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = fs.delete(probe)
    if ((gone is Union2.U2<*, *>)) {
        val gone_7: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((gone as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
        salvo.core.console.println(console, "delete failed: ${kindName__FsError(salvo.core.checked.detach(gone_7))}")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Faults", salvo.core.actor.__PROTO_Faults)))
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val __use_3: salvo.stream.host.HostRawStreams = salvo.stream.host.HostRawStreams()
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: salvo.stream.host.RawStreams = salvo.stream.host.__Mon_RawStreams(__use_3, __lock___use_3)
    val __use_5: salvo.stream.host.DefaultStreams = salvo.stream.host.DefaultStreams(__handle_4)
    val __lock___use_5 = java.util.concurrent.locks.ReentrantLock()
    val __handle_6: salvo.stream.Streams = salvo.stream.__Mon_Streams(__use_5, __lock___use_5)
    val __use_7: salvo.fs.host.__Platform_HostRawFs = salvo.fs.host.__Platform_HostRawFs()
    val __lock___use_7 = java.util.concurrent.locks.ReentrantLock()
    val __handle_8: salvo.fs.host.RawFs = salvo.fs.host.__Mon_RawFs(__use_7, __lock___use_7)
    val __use_9: salvo.fs.host.DefaultFs = salvo.fs.host.DefaultFs(__handle_8, __handle_6)
    val __lock___use_9 = java.util.concurrent.locks.ReentrantLock()
    val __handle_10: salvo.fs.Fs = salvo.fs.__Mon_Fs(__use_9, __lock___use_9)
    val root: salvo.fs.path.Path = salvo.fs.path.path("tmp/files-example")
    val made: Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = __handle_10.createDirs(root)
    if ((made is Union2.U2<*, *>)) {
        val made_11: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((made as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
        salvo.core.console.println(__handle_2, "cannot create the working directory: ${kindName__FsError(salvo.core.checked.detach(made_11))}")
        null
        return
    }
    salvo.core.console.println(__handle_2, "-- the real filesystem, scoped to one directory --")
    run {
        val __use_12: salvo.fs.restricted.RestrictedFs = salvo.fs.restricted.RestrictedFs(root = root, __handle_10, __handle_6)
        val __lock___use_12 = java.util.concurrent.locks.ReentrantLock()
        val __handle_13: salvo.fs.Fs = salvo.fs.__Mon_Fs(__use_12, __lock___use_12)
        workflow(__handle_13, __handle_2, __handle_6)
        sandboxEdges(__handle_13, __handle_2, __handle_6)
    }
    val gone: Union2<Unit, salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>> = __handle_10.delete(root)
    if ((gone is Union2.U2<*, *>)) {
        val gone_14: salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>> = ((gone as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union7<salvo.fs.NotFound, salvo.fs.PermissionDenied, salvo.fs.AlreadyExists, salvo.fs.NotADirectory, salvo.fs.PathEscapes, salvo.fs.IoError, salvo.fs.Streaming>>)
        salvo.core.console.println(__handle_2, "cleanup failed: ${kindName__FsError(salvo.core.checked.detach(gone_14))}")
    }
    salvo.core.console.println(__handle_2, "-- the same code, with no disk at all --")
    run {
        val __use_15: salvo.fs.mem.MemFs = salvo.fs.mem.MemFs()
        val __lock___use_15 = java.util.concurrent.locks.ReentrantLock()
        val __handle_16: salvo.fs.Fs = salvo.fs.__Mon_Fs(__use_15, __lock___use_15)
        val __handle_17: salvo.stream.Streams = salvo.stream.__Mon_Streams(__use_15, __lock___use_15)
        workflow(__handle_16, __handle_2, __handle_17)
    }
}

