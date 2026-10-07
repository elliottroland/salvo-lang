package salvo.main

import salvo.*

data class FileHandle(
    val name: String,
)

object __Codec_FileHandle : salvo.WireCodec<FileHandle> {
    override fun enc(v: FileHandle, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.name, out)
    }
    override fun dec(inp: salvo.WireIn): FileHandle = FileHandle(salvo.StrCodec.dec(inp))
}

fun openFile(console: salvo.core.console.Console, name: String): FileHandle {
    salvo.core.console.println(console, "1. open ${name}")
    return FileHandle(name = name)
}

fun close(console: salvo.core.console.Console, handle: FileHandle) {
    salvo.core.console.println(console, "1. close ${handle.name}")
    run { handle; Unit }
}

fun readSize(console: salvo.core.console.Console, name: String, want: Int): Int {
    val thereIs: Int = salvo.core.string.sizePlatform(name)
    val handle: FileHandle = openFile(console, name)
    if ((want > thereIs)) {
        salvo.core.console.println(console, "1. asked for more than there is")
        close(console, handle)
        return thereIs
    }
    close(console, handle)
    return want
}

fun parsePort(text: String): Int {
    val n: Int? = salvo.core.string.parseIntPlatform(text)
    if ((n == null)) {
        throw ThrowSignal("not a number: ${text}", "Str")
    }
    val n_1: Int = n!!
    if ((n_1 < 1)) {
        throw ThrowSignal("port must be positive", "Str")
    }
    val n_2: Int = n!!
    return n_2
}

fun portOf(config: String): Int {
    val port: Int = parsePort(config)
    return (port * 1)
}

fun portFromFile(console: salvo.core.console.Console, name: String, text: String): Int {
    val handle: FileHandle = openFile(console, name)
    val from: String = handle.name
    close(console, handle)
    salvo.core.console.println(console, "2. reading a port out of ${from}")
    return parsePort(text)
}

fun strictPort(text: String): Int {
    if (((salvo.core.string.sizePlatform(text)) == (0))) {
        throw ThrowSignal("empty", "Str")
    }
    val n: Int? = salvo.core.string.parseIntPlatform(text)
    if ((n == null)) {
        throw ThrowSignal(salvo.core.string.sizePlatform(text), "Int")
    }
    val n_1: Int = n!!
    return n_1
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun report(console: salvo.core.console.Console, label: String, config: String) {
    val outcome: Union2<Int, String> = try {
        Union2.U1<Int, String>(portOf(config))
    } catch (__signal: ThrowSignal) {
        Union2.U2<Int, String>(__signal.payload as String)
    }
    when {
        (outcome is Union2.U1<*, *>) -> {
            val outcome_1: Int = ((outcome as Union2.U1<*, *>).value as Int)
            salvo.core.console.println(console, "3. ${label}: port ${outcome_1}")
        }
        (outcome is Union2.U2<*, *>) -> {
            val outcome_2: String = ((outcome as Union2.U2<*, *>).value as String)
            salvo.core.console.println(console, "3. ${label}: rejected — ${outcome_2}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val small: Int = readSize(__handle_2, "notes.txt", 3)
    salvo.core.console.println(__handle_2, "1. read ${small}")
    val clamped: Int = readSize(__handle_2, "notes.txt", 99)
    salvo.core.console.println(__handle_2, "1. read ${clamped}")
    report(__handle_2, "good", "8080")
    report(__handle_2, "bad", "http")
    val guarded: Union2<Int, String> = try {
        Union2.U1<Int, String>(portFromFile(__handle_2, "ports.txt", "-1"))
    } catch (__signal: ThrowSignal) {
        Union2.U2<Int, String>(__signal.payload as String)
    }
    when {
        (guarded is Union2.U1<*, *>) -> {
            val guarded_3: Int = ((guarded as Union2.U1<*, *>).value as Int)
            salvo.core.console.println(__handle_2, "3. guarded: ${guarded_3}")
        }
        (guarded is Union2.U2<*, *>) -> {
            val guarded_4: String = ((guarded as Union2.U2<*, *>).value as String)
            salvo.core.console.println(__handle_2, "3. guarded: rejected — ${guarded_4}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val mixed: Union2<Int, Union2<String, Int>> = try {
        Union2.U1<Int, Union2<String, Int>>(strictPort(""))
    } catch (__signal: ThrowSignal) {
        when (__signal.tag) {
            "Str" -> Union2.U2<Int, Union2<String, Int>>(Union2.U1<String, Int>(__signal.payload as String))
            "Int" -> Union2.U2<Int, Union2<String, Int>>(Union2.U2<String, Int>(__signal.payload as Int))
            else -> throw __signal
        }
    }
    when {
        (mixed is Union2.U1<*, *>) -> {
            val mixed_5: Int = ((mixed as Union2.U1<*, *>).value as Int)
            salvo.core.console.println(__handle_2, "3. mixed: ${mixed_5}")
        }
        (mixed is Union2.U2<*, *>) -> {
            val mixed_6: Union2<String, Int> = ((mixed as Union2.U2<*, *>).value as Union2<String, Int>)
            val why: Union2<String, Int> = mixed_6
            when {
                (why is Union2.U1<*, *>) -> {
                    val why_7: String = ((why as Union2.U1<*, *>).value as String)
                    salvo.core.console.println(__handle_2, "3. mixed: message ${why_7}")
                }
                (why is Union2.U2<*, *>) -> {
                    val why_8: Int = ((why as Union2.U2<*, *>).value as Int)
                    salvo.core.console.println(__handle_2, "3. mixed: length ${why_8}")
                }
                else -> throw IllegalStateException("salvo: unreachable arm")
            }
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val outer: Union2<Int, String> = try {
        val inner: Union2<Int, String> = try {
            Union2.U1<Int, String>(parsePort("nope"))
        } catch (__signal: ThrowSignal) {
            Union2.U2<Int, String>(__signal.payload as String)
        }
        Union2.U1<Int, String>(when {
            (inner is Union2.U1<*, *>) -> {
                val inner_9: Int = ((inner as Union2.U1<*, *>).value as Int)
                val got: Int = inner_9
                got
            }
            (inner is Union2.U2<*, *>) -> {
                val inner_10: String = ((inner as Union2.U2<*, *>).value as String)
                salvo.core.console.println(__handle_2, "3. inner caught: ${inner_10}")
                parsePort("also nope")
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        })
    } catch (__signal: ThrowSignal) {
        Union2.U2<Int, String>(__signal.payload as String)
    }
    when {
        (outer is Union2.U1<*, *>) -> {
            val outer_11: Int = ((outer as Union2.U1<*, *>).value as Int)
            salvo.core.console.println(__handle_2, "3. outer: ${outer_11}")
        }
        (outer is Union2.U2<*, *>) -> {
            val outer_12: String = ((outer as Union2.U2<*, *>).value as String)
            salvo.core.console.println(__handle_2, "3. outer caught: ${outer_12}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

