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

fun open_file(console: Console, name: String): FileHandle {
    println(console, "1. open $name")
    return FileHandle(name = name)
}

fun close__3(console: Console, handle: FileHandle) {
    println(console, "1. close ${handle.name}")
    (handle).let {}
}

fun read_size(console: Console, name: String, want: Int): Int {
    val there_is = name.length
    val handle = open_file(console, name)
    if (want > there_is) {
        println(console, "1. asked for more than there is")
        close__3(console, handle)
        return there_is
    }
    close__3(console, handle)
    return want
}

fun parse_port(text: String): Int {
    val n = text.toIntOrNull()
    if (n == null) {
        throw ThrowSignal("not a number: $text", "Str")
    }
    if (n < 1) {
        throw ThrowSignal("port must be positive", "Str")
    }
    return n
}

fun port_of(config: String): Int {
    val port = parse_port(config)
    return port * 1
}

fun port_from_file(console: Console, name: String, text: String): Int {
    val handle = open_file(console, name)
    val from = handle.name
    close__3(console, handle)
    println(console, "2. reading a port out of $from")
    return parse_port(text)
}

fun strict_port(text: String): Int {
    if (text.length == 0) {
        throw ThrowSignal("empty", "Str")
    }
    val n = text.toIntOrNull()
    if (n == null) {
        throw ThrowSignal(text.length, "Int")
    }
    return n
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun report(console: Console, label: String, config: String) {
    val outcome = try {
        U2_1<Int, String>(port_of(config))
    } catch (__signal: ThrowSignal) {
        U2_2<Int, String>(__signal.payload as String)
    }
    when (outcome) {
        is U2_1<*, *> -> {
            println(console, "3. $label: port ${(outcome.value as Int)}")
        }
        is U2_2<*, *> -> {
            println(console, "3. $label: rejected — ${(outcome.value as String)}")
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun main() {
    val console: Console = StdOutConsole()
    val small = read_size(console, "notes.txt", 3)
    println(console, "1. read $small")
    val clamped = read_size(console, "notes.txt", 99)
    println(console, "1. read $clamped")
    report(console, "good", "8080")
    report(console, "bad", "http")
    val guarded = try {
        U2_1<Int, String>(port_from_file(console, "ports.txt", "-1"))
    } catch (__signal: ThrowSignal) {
        U2_2<Int, String>(__signal.payload as String)
    }
    when (guarded) {
        is U2_1<*, *> -> {
            println(console, "3. guarded: ${(guarded.value as Int)}")
        }
        is U2_2<*, *> -> {
            println(console, "3. guarded: rejected — ${(guarded.value as String)}")
        }
    }
    val mixed = try {
        U2_1<Int, Union2<String, Int>>(strict_port(""))
    } catch (__signal: ThrowSignal) {
        when (__signal.tag) {
            "Str" -> U2_2<Int, Union2<String, Int>>(U2_1<String, Int>(__signal.payload as String))
            "Int" -> U2_2<Int, Union2<String, Int>>(U2_2<String, Int>(__signal.payload as Int))
            else -> throw __signal
        }
    }
    when (mixed) {
        is U2_1<*, *> -> {
            println(console, "3. mixed: ${(mixed.value as Int)}")
        }
        is U2_2<*, *> -> {
            val why: Union2<String, Int> = (mixed.value as Union2<String, Int>)
            when (why) {
                is U2_1<*, *> -> {
                    println(console, "3. mixed: message ${(why.value as String)}")
                }
                is U2_2<*, *> -> {
                    println(console, "3. mixed: length ${(why.value as Int)}")
                }
            }
        }
    }
    val outer = try {
        val inner = try {
            U2_1<Int, String>(parse_port("nope"))
        } catch (__signal: ThrowSignal) {
            U2_2<Int, String>(__signal.payload as String)
        }
        U2_1<Int, String>(when (inner) {
            is U2_1<*, *> -> {
                val got: Int = (inner.value as Int)
                got
            }
            is U2_2<*, *> -> {
                println(console, "3. inner caught: ${(inner.value as String)}")
                parse_port("also nope")
            }
        })
    } catch (__signal: ThrowSignal) {
        U2_2<Int, String>(__signal.payload as String)
    }
    when (outer) {
        is U2_1<*, *> -> {
            println(console, "3. outer: ${(outer.value as Int)}")
        }
        is U2_2<*, *> -> {
            println(console, "3. outer caught: ${(outer.value as String)}")
        }
    }
}
