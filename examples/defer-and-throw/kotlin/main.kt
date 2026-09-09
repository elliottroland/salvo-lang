package salvo.main

import salvo.*
import salvo.core.`throw`.*
import salvo.core.array.*
import salvo.core.console.*
import salvo.core.list.*
import salvo.core.result.*
import salvo.core.string.*

fun lifo(console: Console) {
    println(console, "1. enter")
    try {
        try {
            println(console, "1. body")
        } finally {
            println(console, "1. second declared, first to run")
        }
    } finally {
        println(console, "1. first declared, last to run")
    }
}

fun per_iteration(console: Console) {
    var i = 0
    while (i < 4) {
        try {
            i = i + 1
            if (i == 2) {
                continue
            }
            if (i == 3) {
                break
            }
            println(console, "1. working on iteration $i")
        } finally {
            println(console, "1. leaving iteration $i")
        }
    }
}

data class FileHandle(
    val name: String,
)

fun open_file(console: Console, name: String): FileHandle {
    println(console, "2. open $name")
    return FileHandle(name = name)
}

fun close(console: Console, handle: FileHandle) {
    println(console, "2. close ${handle.name}")
}

fun read_size(console: Console, name: String, want: Int): Int {
    val there_is = name.length
    val handle = open_file(console, name)
    try {
        if (want > there_is) {
            println(console, "2. asked for more than there is")
            return there_is
        }
        return want
    } finally {
        close(console, handle)
    }
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
    try {
        return parse_port(text)
    } finally {
        close(console, handle)
    }
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

fun report(console: Console, label: String, config: String) {
    val outcome = try {
        U2_1<Int, String>(port_of(config))
    } catch (__signal: ThrowSignal) {
        U2_2<Int, String>(__signal.payload as String)
    }
    when (outcome) {
        is U2_1<*, *> -> {
            println(console, "4. $label: port ${(outcome.value as Int)}")
        }
        is U2_2<*, *> -> {
            println(console, "4. $label: rejected — ${(outcome.value as String)}")
        }
    }
}

fun main() {
    val console: Console = StdOutConsole()
    lifo(console)
    per_iteration(console)
    val small = read_size(console, "notes.txt", 3)
    println(console, "2. read $small")
    val clamped = read_size(console, "notes.txt", 99)
    println(console, "2. read $clamped")
    report(console, "good", "8080")
    report(console, "bad", "http")
    val guarded = try {
        U2_1<Int, String>(port_from_file(console, "ports.txt", "-1"))
    } catch (__signal: ThrowSignal) {
        U2_2<Int, String>(__signal.payload as String)
    }
    when (guarded) {
        is U2_1<*, *> -> {
            println(console, "4. guarded: ${(guarded.value as Int)}")
        }
        is U2_2<*, *> -> {
            println(console, "4. guarded: rejected — ${(guarded.value as String)}")
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
            println(console, "4. mixed: ${(mixed.value as Int)}")
        }
        is U2_2<*, *> -> {
            val why: Union2<String, Int> = (mixed.value as Union2<String, Int>)
            when (why) {
                is U2_1<*, *> -> {
                    println(console, "4. mixed: message ${(why.value as String)}")
                }
                is U2_2<*, *> -> {
                    println(console, "4. mixed: length ${(why.value as Int)}")
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
                println(console, "4. inner caught: ${(inner.value as String)}")
                parse_port("also nope")
            }
        })
    } catch (__signal: ThrowSignal) {
        U2_2<Int, String>(__signal.payload as String)
    }
    when (outer) {
        is U2_1<*, *> -> {
            println(console, "4. outer: ${(outer.value as Int)}")
        }
        is U2_2<*, *> -> {
            println(console, "4. outer caught: ${(outer.value as String)}")
        }
    }
}
