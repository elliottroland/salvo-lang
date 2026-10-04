package salvo.main

import salvo.*
import salvo.core.console.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.throw_.*

data class FileHandle(
    val name: String,
)

object __Codec_FileHandle : salvo.WireCodec<FileHandle> {
    override fun enc(v: FileHandle, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.name, out)
    }
    override fun dec(inp: salvo.WireIn): FileHandle = FileHandle(salvo.StrCodec.dec(inp))
}

fun openFile(console: Console, name: String): FileHandle {
    println(console, "1. open $name")
    return FileHandle(name = name)
}

fun close__4(console: Console, handle: FileHandle) {
    println(console, "1. close ${handle.name}")
    (handle).let {}
}

fun readSize(console: Console, name: String, want: Int): Int {
    val thereIs = name.length
    val handle = openFile(console, name)
    if (want > thereIs) {
        println(console, "1. asked for more than there is")
        close__4(console, handle)
        return thereIs
    }
    close__4(console, handle)
    return want
}

fun parsePort(text: String): Int {
    val n = text.toIntOrNull()
    if (n == null) {
        throw ThrowSignal("not a number: $text", "Str")
    }
    if (n < 1) {
        throw ThrowSignal("port must be positive", "Str")
    }
    return n
}

fun portOf(config: String): Int {
    val port = parsePort(config)
    return port * 1
}

fun portFromFile(console: Console, name: String, text: String): Int {
    val handle = openFile(console, name)
    val from = handle.name
    close__4(console, handle)
    println(console, "2. reading a port out of $from")
    return parsePort(text)
}

fun strictPort(text: String): Int {
    if (text.length == 0) {
        throw ThrowSignal("empty", "Str")
    }
    val n = text.toIntOrNull()
    if (n == null) {
        throw ThrowSignal(text.length, "Int")
    }
    return n
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun report(console: Console, label: String, config: String) {
    val outcome = try {
        Union2.U1<Int, String>(portOf(config))
    } catch (__signal: ThrowSignal) {
        Union2.U2<Int, String>(__signal.payload as String)
    }
    when (outcome) {
        is Union2.U1<*, *> -> {
            println(console, "3. $label: port ${(outcome.value as Int)}")
        }
        is Union2.U2<*, *> -> {
            println(console, "3. $label: rejected — ${(outcome.value as String)}")
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val small = readSize(console, "notes.txt", 3)
    println(console, "1. read $small")
    val clamped = readSize(console, "notes.txt", 99)
    println(console, "1. read $clamped")
    report(console, "good", "8080")
    report(console, "bad", "http")
    val guarded = try {
        Union2.U1<Int, String>(portFromFile(console, "ports.txt", "-1"))
    } catch (__signal: ThrowSignal) {
        Union2.U2<Int, String>(__signal.payload as String)
    }
    when (guarded) {
        is Union2.U1<*, *> -> {
            println(console, "3. guarded: ${(guarded.value as Int)}")
        }
        is Union2.U2<*, *> -> {
            println(console, "3. guarded: rejected — ${(guarded.value as String)}")
        }
    }
    val mixed = try {
        Union2.U1<Int, Union2<String, Int>>(strictPort(""))
    } catch (__signal: ThrowSignal) {
        when (__signal.tag) {
            "Str" -> Union2.U2<Int, Union2<String, Int>>(Union2.U1<String, Int>(__signal.payload as String))
            "Int" -> Union2.U2<Int, Union2<String, Int>>(Union2.U2<String, Int>(__signal.payload as Int))
            else -> throw __signal
        }
    }
    when (mixed) {
        is Union2.U1<*, *> -> {
            println(console, "3. mixed: ${(mixed.value as Int)}")
        }
        is Union2.U2<*, *> -> {
            val why: Union2<String, Int> = (mixed.value as Union2<String, Int>)
            when (why) {
                is Union2.U1<*, *> -> {
                    println(console, "3. mixed: message ${(why.value as String)}")
                }
                is Union2.U2<*, *> -> {
                    println(console, "3. mixed: length ${(why.value as Int)}")
                }
            }
        }
    }
    val outer = try {
        val inner = try {
            Union2.U1<Int, String>(parsePort("nope"))
        } catch (__signal: ThrowSignal) {
            Union2.U2<Int, String>(__signal.payload as String)
        }
        Union2.U1<Int, String>(when (inner) {
            is Union2.U1<*, *> -> {
                val got: Int = (inner.value as Int)
                got
            }
            is Union2.U2<*, *> -> {
                println(console, "3. inner caught: ${(inner.value as String)}")
                parsePort("also nope")
            }
        })
    } catch (__signal: ThrowSignal) {
        Union2.U2<Int, String>(__signal.payload as String)
    }
    when (outer) {
        is Union2.U1<*, *> -> {
            println(console, "3. outer: ${(outer.value as Int)}")
        }
        is Union2.U2<*, *> -> {
            println(console, "3. outer caught: ${(outer.value as String)}")
        }
    }
}
