package salvo.main

import salvo.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun kindName__FsError(kind: Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>): String {
    if ((kind is Union7.U1<*, *, *, *, *, *, *>)) {
        val kind_1: salvo.std.fs.NotFound = ((kind as Union7.U1<*, *, *, *, *, *, *>).value as salvo.std.fs.NotFound)
        return "not found"
    }
    if ((kind is Union7.U4<*, *, *, *, *, *, *>)) {
        val kind_2: salvo.std.fs.NotADirectory = ((kind as Union7.U4<*, *, *, *, *, *, *>).value as salvo.std.fs.NotADirectory)
        return "not a directory"
    }
    if ((kind is Union7.U5<*, *, *, *, *, *, *>)) {
        val kind_3: salvo.std.fs.PathEscapes = ((kind as Union7.U5<*, *, *, *, *, *, *>).value as salvo.std.fs.PathEscapes)
        return "escapes the sandbox"
    }
    if ((kind is Union7.U7<*, *, *, *, *, *, *>)) {
        val kind_4: salvo.std.fs.Streaming = ((kind as Union7.U7<*, *, *, *, *, *, *>).value as salvo.std.fs.Streaming)
        return salvo.main.kindName__StreamError(kind_4.error)
    }
    return "other"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun kindName__StreamError(kind: Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>): String {
    if ((kind is Union2.U1<*, *>)) {
        val kind_1: salvo.std.stream.InvalidUtf8 = ((kind as Union2.U1<*, *>).value as salvo.std.stream.InvalidUtf8)
        return "not valid UTF-8"
    }
    return "other"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun workflow(fs: salvo.std.fs.Fs, console: salvo.std.core.console.Console, streams: salvo.std.stream.Streams) {
    val wrote: Union2<Long, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.writeStr(fs, streams, salvo.std.fs.path.path("notes.txt"), "alpha\nbeta\ngamma\n")
    when {
        (wrote is Union2.U1<*, *>) -> {
            val wrote_1: Long = ((wrote as Union2.U1<*, *>).value as Long)
            salvo.std.core.console.println(console, "wrote ${wrote_1} bytes")
        }
        (wrote is Union2.U2<*, *>) -> {
            val wrote_2: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((wrote as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "write failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(wrote_2))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:70:5"))
        }
    }
    val text: Union2<String, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.readToStr(fs, streams, salvo.std.fs.path.path("notes.txt"))
    when {
        (text is Union2.U1<*, *>) -> {
            val text_3: String = ((text as Union2.U1<*, *>).value as String)
            salvo.std.core.console.println(console, "read back ${salvo.std.core.string.byteSizePlatform(text_3)} bytes")
        }
        (text is Union2.U2<*, *>) -> {
            val text_4: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((text as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "read failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(text_4))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:75:5"))
        }
    }
    val opened: Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openRead(salvo.std.fs.path.path("notes.txt"))
    when {
        (opened is Union2.U1<*, *>) -> {
            val opened_5: salvo.std.stream.InStream = ((opened as Union2.U1<*, *>).value as salvo.std.stream.InStream)
            val p: salvo.std.stream.Lines = salvo.std.stream.lines(opened_5)
            while (true) {
                val __step_7: Union2<String, salvo.std.core.iterator.Finished> = salvo.std.stream.next__Lines(streams, p)
                when {
                    (__step_7 is Union2.U1<*, *>) -> {
                        val __emitted_8: String = ((__step_7 as Union2.U1<*, *>).value as String)
                        val line: String = __emitted_8
                        salvo.std.core.console.println(console, "line: ${line}")
                    }
                    else -> {
                        break
                    }
                }
            }
            val closed: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = salvo.std.stream.close__Lines(streams, p)
            if ((closed is Union2.U2<*, *>)) {
                val closed_9: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                salvo.std.core.console.println(console, "close failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(closed_9))}")
            }
        }
        (opened is Union2.U2<*, *>) -> {
            val opened_10: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "open failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(opened_10))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:84:5"))
        }
    }
    val out: Union2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openAppend(salvo.std.fs.path.path("notes.txt"))
    when {
        (out is Union2.U1<*, *>) -> {
            val out_11: salvo.std.stream.OutStream = ((out as Union2.U1<*, *>).value as salvo.std.stream.OutStream)
            val w: salvo.std.stream.OutStream = out_11
            val at: Long = streams.position__OutStream(w)
            val n: Long = streams.writeLine(w, "delta")
            salvo.std.core.console.println(console, "appended ${n} bytes at offset ${at}")
            val shut: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.close__OutStream(w)
            if ((shut is Union2.U2<*, *>)) {
                val shut_12: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((shut as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                salvo.std.core.console.println(console, "close failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(shut_12))}")
            }
            val resumed: Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openReadAt(salvo.std.fs.path.path("notes.txt"), at)
            when {
                (resumed is Union2.U1<*, *>) -> {
                    val resumed_13: salvo.std.stream.InStream = ((resumed as Union2.U1<*, *>).value as salvo.std.stream.InStream)
                    val s: salvo.std.stream.InStream = resumed_13
                    val line: String? = streams.readLine(s)
                    when {
                        (line != null) -> {
                            val line_14: String = line!!
                            salvo.std.core.console.println(console, "at ${at}: ${line_14}")
                        }
                        (line == null) -> {
                            salvo.std.core.console.println(console, "at ${at}: end of file")
                        }
                        else -> {
                            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:123:21"))
                        }
                    }
                    val done: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.close__InStream(s)
                    if ((done is Union2.U2<*, *>)) {
                        val done_15: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                        salvo.std.core.console.println(console, "close failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(done_15))}")
                    }
                }
                (resumed is Union2.U2<*, *>) -> {
                    val resumed_16: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((resumed as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
                    salvo.std.core.console.println(console, "reopen failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(resumed_16))}")
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:119:13"))
                }
            }
        }
        (out is Union2.U2<*, *>) -> {
            val out_17: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((out as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "append failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(out_17))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:102:5"))
        }
    }
    val bin: Union2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openWrite(salvo.std.fs.path.path("raw.bin"))
    when {
        (bin is Union2.U1<*, *>) -> {
            val bin_18: salvo.std.stream.OutStream = ((bin as Union2.U1<*, *>).value as salvo.std.stream.OutStream)
            val w: salvo.std.stream.OutStream = bin_18
            val data: salvo.platform.std.core.bytes.Bytes = salvo.std.core.bytes.bytesOf(arrayOf<UByte>((0).toUByte(), (255).toUByte(), (200).toUByte()))
            val n: Long = streams.writeBytes(w, data)
            val m: Long = streams.write(w, "hé")
            salvo.std.core.console.println(console, "wrote ${n} raw bytes and ${m} encoded")
            val shut: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.close__OutStream(w)
            if ((shut is Union2.U2<*, *>)) {
                val shut_19: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((shut as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                salvo.std.core.console.println(console, "close failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(shut_19))}")
            }
        }
        (bin is Union2.U2<*, *>) -> {
            val bin_20: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((bin as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "raw open failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(bin_20))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:143:5"))
        }
    }
    val raw: Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openRead(salvo.std.fs.path.path("raw.bin"))
    when {
        (raw is Union2.U1<*, *>) -> {
            val raw_21: salvo.std.stream.InStream = ((raw as Union2.U1<*, *>).value as salvo.std.stream.InStream)
            val s: salvo.std.stream.InStream = raw_21
            val head: Union2<salvo.platform.std.core.bytes.Bytes, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.readBytes(s, 3)
            when {
                (head is Union2.U1<*, *>) -> {
                    val head_22: salvo.platform.std.core.bytes.Bytes = ((head as Union2.U1<*, *>).value as salvo.platform.std.core.bytes.Bytes)
                    salvo.std.core.console.println(console, "first three: ${salvo.std.core.bytes.toStrPlatform(head_22)} = ${salvo.std.core.bytes.toHexPlatform(head_22)}")
                }
                (head is Union2.U2<*, *>) -> {
                    val head_23: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((head as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                    salvo.std.core.console.println(console, "byte read failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(head_23))}")
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:164:13"))
                }
            }
            val tail: Union2<String, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.readAll(s)
            when {
                (tail is Union2.U1<*, *>) -> {
                    val tail_24: String = ((tail as Union2.U1<*, *>).value as String)
                    salvo.std.core.console.println(console, "the rest, as text: ${tail_24}")
                }
                (tail is Union2.U2<*, *>) -> {
                    val tail_25: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((tail as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                    salvo.std.core.console.println(console, "decode failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(tail_25))}")
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:171:13"))
                }
            }
            val done: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.close__InStream(s)
            if ((done is Union2.U2<*, *>)) {
                val done_26: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                salvo.std.core.console.println(console, "close failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(done_26))}")
            }
        }
        (raw is Union2.U2<*, *>) -> {
            val raw_27: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((raw as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "raw read failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(raw_27))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:160:5"))
        }
    }
    val split: Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openReadAt(salvo.std.fs.path.path("raw.bin"), 5L)
    when {
        (split is Union2.U1<*, *>) -> {
            val split_28: salvo.std.stream.InStream = ((split as Union2.U1<*, *>).value as salvo.std.stream.InStream)
            val s: salvo.std.stream.InStream = split_28
            val broken: Union2<String, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.readAll(s)
            when {
                (broken is Union2.U1<*, *>) -> {
                    val broken_29: String = ((broken as Union2.U1<*, *>).value as String)
                    salvo.std.core.console.println(console, "unexpected: ${broken_29} decoded")
                }
                (broken is Union2.U2<*, *>) -> {
                    val broken_30: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((broken as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                    salvo.std.core.console.println(console, "mid-character: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(broken_30))}")
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:191:13"))
                }
            }
            val done: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.close__InStream(s)
            when {
                (done is Union2.U1<*, *>) -> {
                    val done_31: Unit = ((done as Union2.U1<*, *>).value as Unit)
                    salvo.std.core.console.println(console, "unexpected: the failure was not recorded")
                }
                (done is Union2.U2<*, *>) -> {
                    val done_32: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                    salvo.std.core.console.println(console, "and again at close: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(done_32))}")
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:196:13"))
                }
            }
        }
        (split is Union2.U2<*, *>) -> {
            val split_33: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((split as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "split open failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(split_33))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:187:5"))
        }
    }
    val held: Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openRead(salvo.std.fs.path.path("raw.bin"))
    when {
        (held is Union2.U1<*, *>) -> {
            val held_34: salvo.std.stream.InStream = ((held as Union2.U1<*, *>).value as salvo.std.stream.InStream)
            val s: salvo.std.stream.InStream = held_34
            val buf: salvo.platform.std.core.bytes.MutBytes = salvo.std.core.bytes.mutBytes(arrayOf<salvo.platform.std.core.bytes.Bytes>())
            var steps: Int = 0
            var moved: Int = 0
            var reading: Boolean = true
            while (true) {
                if (!(reading)) {
                    break
                }
                salvo.std.core.bytes.clearPlatform(buf)
                val got: Union2<Int, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.readTo__InStream_Bytes_Int(s, buf, 4)
                when {
                    (got is Union2.U1<*, *>) -> {
                        val got_35: Int = ((got as Union2.U1<*, *>).value as Int)
                        val n: Int = got_35
                        if ((n == 0)) {
                            reading = false
                        } else {
                            steps = (steps + 1)
                            moved = (moved + n)
                        }
                    }
                    (got is Union2.U2<*, *>) -> {
                        val got_36: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((got as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                        salvo.std.core.console.println(console, "fill failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(got_36))}")
                        reading = false
                    }
                    else -> {
                        throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:219:17"))
                    }
                }
            }
            salvo.std.core.console.println(console, "filled ${moved} bytes in ${steps} reads, one buffer")
            val done: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.close__InStream(s)
            if ((done is Union2.U2<*, *>)) {
                val done_37: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                salvo.std.core.console.println(console, "close failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(done_37))}")
            }
        }
        (held is Union2.U2<*, *>) -> {
            val held_38: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((held as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "fill open failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(held_38))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:209:5"))
        }
    }
    val lined: Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openRead(salvo.std.fs.path.path("notes.txt"))
    when {
        (lined is Union2.U1<*, *>) -> {
            val lined_39: salvo.std.stream.InStream = ((lined as Union2.U1<*, *>).value as salvo.std.stream.InStream)
            val s: salvo.std.stream.InStream = lined_39
            val line: salvo.platform.std.core.string.MutStr = salvo.std.core.string.Mut_Str(arrayOf<String>())
            var longest: Int = 0
            var reading: Boolean = true
            while (true) {
                if (!(reading)) {
                    break
                }
                salvo.std.core.string.clearPlatform(line)
                if (streams.readLineTo(s, line)) {
                    if ((salvo.std.core.string.sizePlatform(line.toString()) > longest)) {
                        longest = salvo.std.core.string.sizePlatform(line.toString())
                    }
                } else {
                    reading = false
                }
            }
            salvo.std.core.console.println(console, "longest line: ${longest} characters")
            val done: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.close__InStream(s)
            if ((done is Union2.U2<*, *>)) {
                val done_40: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                salvo.std.core.console.println(console, "close failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(done_40))}")
            }
        }
        (lined is Union2.U2<*, *>) -> {
            val lined_41: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((lined as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "lines open failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(lined_41))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:246:5"))
        }
    }
    val ch: Union2<salvo.std.stream.Chunks, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.openChunks(fs, streams, salvo.std.fs.path.path("raw.bin"), 4)
    when {
        (ch is Union2.U1<*, *>) -> {
            val ch_42: salvo.std.stream.Chunks = ((ch as Union2.U1<*, *>).value as salvo.std.stream.Chunks)
            val p: salvo.std.stream.Chunks = ch_42
            var seen: Int = 0
            while (true) {
                val __step_44: Union2<salvo.platform.std.core.bytes.Bytes, salvo.std.core.iterator.Finished> = salvo.std.stream.next__Chunks(streams, p)
                when {
                    (__step_44 is Union2.U1<*, *>) -> {
                        val __emitted_45: salvo.platform.std.core.bytes.Bytes = ((__step_44 as Union2.U1<*, *>).value as salvo.platform.std.core.bytes.Bytes)
                        val chunk: salvo.platform.std.core.bytes.Bytes = __emitted_45
                        seen = (seen + salvo.std.core.bytes.sizePlatform(chunk))
                    }
                    else -> {
                        break
                    }
                }
            }
            salvo.std.core.console.println(console, "chunks saw ${seen} bytes")
            val done: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = salvo.std.stream.close__Chunks(streams, p)
            if ((done is Union2.U2<*, *>)) {
                val done_46: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((done as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
                salvo.std.core.console.println(console, "close failed: ${salvo.main.kindName__StreamError(salvo.std.core.checked.detach(done_46))}")
            }
        }
        (ch is Union2.U2<*, *>) -> {
            val ch_47: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((ch as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "chunks failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(ch_47))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:275:5"))
        }
    }
    val copied: Union2<Long, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.copyFile(fs, streams, salvo.std.fs.path.path("notes.txt"), salvo.std.fs.path.path("notes-copy.txt"))
    when {
        (copied is Union2.U1<*, *>) -> {
            val copied_48: Long = ((copied as Union2.U1<*, *>).value as Long)
            salvo.std.core.console.println(console, "copied ${copied_48} bytes")
        }
        (copied is Union2.U2<*, *>) -> {
            val copied_49: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((copied as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "copy failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(copied_49))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:295:5"))
        }
    }
    val whole: Union2<salvo.platform.std.core.bytes.Bytes, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.readToBytes(fs, streams, salvo.std.fs.path.path("raw.bin"))
    when {
        (whole is Union2.U1<*, *>) -> {
            val whole_50: salvo.platform.std.core.bytes.Bytes = ((whole as Union2.U1<*, *>).value as salvo.platform.std.core.bytes.Bytes)
            salvo.std.core.console.println(console, "raw.bin is ${salvo.std.core.bytes.sizePlatform(whole_50)} bytes: ${salvo.std.core.bytes.toHexPlatform(whole_50)}")
        }
        (whole is Union2.U2<*, *>) -> {
            val whole_51: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((whole as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "byte read failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(whole_51))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:300:5"))
        }
    }
    val failures: salvo.platform.std.core.list.MutList<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = mutableListOf<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>()
    val missing: Union2<String, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.readToStr(fs, streams, salvo.std.fs.path.path("nope.txt"))
    when {
        (missing is Union2.U1<*, *>) -> {
            val missing_52: String = ((missing as Union2.U1<*, *>).value as String)
            salvo.std.core.console.println(console, "unexpected: ${missing_52}")
        }
        (missing is Union2.U2<*, *>) -> {
            val missing_53: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((missing as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.list.addPlatform(failures, salvo.std.core.checked.detach(missing_53))
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:310:5"))
        }
    }
    val notADir: Union2<List<String>, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.listDir(salvo.std.fs.path.path("notes.txt"))
    when {
        (notADir is Union2.U1<*, *>) -> {
            val notADir_54: List<String> = ((notADir as Union2.U1<*, *>).value as List<String>)
            salvo.std.core.console.println(console, "unexpected: ${salvo.std.core.list.toStr(notADir_54, { __a0 -> __a0 })}")
        }
        (notADir is Union2.U2<*, *>) -> {
            val notADir_55: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((notADir as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.list.addPlatform(failures, salvo.std.core.checked.detach(notADir_55))
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:315:5"))
        }
    }
    salvo.std.core.console.println(console, "failures: ${salvo.std.core.list.sizePlatform(failures)}")
    for (kind in salvo.platform.std.core.list.each(failures)) {
        salvo.std.core.console.println(console, "  ${salvo.main.kindName__FsError(kind)}")
    }
    for (name in salvo.platform.std.core.list.each(listOf<String>("notes.txt", "notes-copy.txt", "raw.bin"))) {
        val gone: Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.delete(salvo.std.fs.path.path(name))
        if ((gone is Union2.U2<*, *>)) {
            val gone_56: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((gone as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "delete failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(gone_56))}")
        }
    }
    salvo.std.core.console.println(console, "cleaned up")
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun sandboxEdges(fs: salvo.std.fs.Fs, console: salvo.std.core.console.Console, streams: salvo.std.stream.Streams) {
    val inside: Union2<Long, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.writeStr(fs, streams, salvo.std.fs.path.path("sub/../probe.txt"), "inside\n")
    when {
        (inside is Union2.U1<*, *>) -> {
            val inside_1: Long = ((inside as Union2.U1<*, *>).value as Long)
            salvo.std.core.console.println(console, "through `..`: wrote ${inside_1} bytes")
        }
        (inside is Union2.U2<*, *>) -> {
            val inside_2: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((inside as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "through `..`: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(inside_2))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:340:5"))
        }
    }
    val up: Union2<String, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.readToStr(fs, streams, salvo.std.fs.path.path("../secret.txt"))
    when {
        (up is Union2.U1<*, *>) -> {
            val up_3: String = ((up as Union2.U1<*, *>).value as String)
            salvo.std.core.console.println(console, "unexpected: read outside the sandbox")
        }
        (up is Union2.U2<*, *>) -> {
            val up_4: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((up as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "climbing out: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(up_4))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:345:5"))
        }
    }
    val absolute: Union2<String, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.readToStr(fs, streams, salvo.std.fs.path.path("/etc/hosts"))
    when {
        (absolute is Union2.U1<*, *>) -> {
            val absolute_5: String = ((absolute as Union2.U1<*, *>).value as String)
            salvo.std.core.console.println(console, "unexpected: an absolute path resolved")
        }
        (absolute is Union2.U2<*, *>) -> {
            val absolute_6: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((absolute as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "absolute path: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(absolute_6))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:350:5"))
        }
    }
    val probe: salvo.std.fs.path.Path = salvo.std.fs.path.path("probe.txt")
    salvo.std.core.console.println(console, "probe still there: ${fs.exists(probe)}")
    val gone: Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.delete(probe)
    if ((gone is Union2.U2<*, *>)) {
        val gone_7: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((gone as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
        salvo.std.core.console.println(console, "delete failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(gone_7))}")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(kotlin.collections.listOf(kotlin.Pair("Faults", salvo.std.core.actor.__PROTO_Faults)))
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val __use_3: salvo.std.stream.host.HostRawStreams = salvo.std.stream.host.HostRawStreams()
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: salvo.std.stream.host.RawStreams = salvo.std.stream.host.__Mon_RawStreams(__use_3, __lock___use_3)
    val __use_5: salvo.std.stream.host.DefaultStreams = salvo.std.stream.host.DefaultStreams(__handle_4)
    val __lock___use_5 = java.util.concurrent.locks.ReentrantLock()
    val __handle_6: salvo.std.stream.Streams = salvo.std.stream.__Mon_Streams(__use_5, __lock___use_5)
    val __use_7: salvo.std.fs.host.__Platform_HostRawFs = salvo.std.fs.host.__Platform_HostRawFs()
    val __lock___use_7 = java.util.concurrent.locks.ReentrantLock()
    val __handle_8: salvo.std.fs.host.RawFs = salvo.std.fs.host.__Mon_RawFs(__use_7, __lock___use_7)
    val __use_9: salvo.std.fs.host.DefaultFs = salvo.std.fs.host.DefaultFs(__handle_8, __handle_6)
    val __lock___use_9 = java.util.concurrent.locks.ReentrantLock()
    val __handle_10: salvo.std.fs.Fs = salvo.std.fs.__Mon_Fs(__use_9, __lock___use_9)
    val root: salvo.std.fs.path.Path = salvo.std.fs.path.path("tmp/files-example")
    val made: Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = __handle_10.createDirs(root)
    if ((made is Union2.U2<*, *>)) {
        val made_11: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((made as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
        salvo.std.core.console.println(__handle_2, "cannot create the working directory: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(made_11))}")
        null
        return
    }
    salvo.std.core.console.println(__handle_2, "-- the real filesystem, scoped to one directory --")
    kotlin.run {
        val __use_12: salvo.std.fs.restricted.RestrictedFs = salvo.std.fs.restricted.RestrictedFs(root = root, __handle_10, __handle_6)
        val __lock___use_12 = java.util.concurrent.locks.ReentrantLock()
        val __handle_13: salvo.std.fs.Fs = salvo.std.fs.__Mon_Fs(__use_12, __lock___use_12)
        salvo.main.workflow(__handle_13, __handle_2, __handle_6)
        salvo.main.sandboxEdges(__handle_13, __handle_2, __handle_6)
    }
    val gone: Union2<Unit, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = __handle_10.delete(root)
    if ((gone is Union2.U2<*, *>)) {
        val gone_14: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((gone as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
        salvo.std.core.console.println(__handle_2, "cleanup failed: ${salvo.main.kindName__FsError(salvo.std.core.checked.detach(gone_14))}")
    }
    salvo.std.core.console.println(__handle_2, "-- the same code, with no disk at all --")
    kotlin.run {
        val __use_15: salvo.std.fs.mem.MemFs = salvo.std.fs.mem.MemFs()
        val __lock___use_15 = java.util.concurrent.locks.ReentrantLock()
        val __handle_16: salvo.std.fs.Fs = salvo.std.fs.__Mon_Fs(__use_15, __lock___use_15)
        val __handle_17: salvo.std.stream.Streams = salvo.std.stream.__Mon_Streams(__use_15, __lock___use_15)
        salvo.main.workflow(__handle_16, __handle_2, __handle_17)
    }
}

