package salvo.std.core.console

import salvo.*

interface Console {
    fun print(message: String)
    fun printerr(message: String)
    fun readln(): String?
}

class __Mon_Console(
    private val inner: Console,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Console {
    override fun print(message: String) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { inner.print(message) } finally { lock.unlock() }
    }
    override fun printerr(message: String) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { inner.printerr(message) } finally { lock.unlock() }
    }
    override fun readln(): String? {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.readln() } finally { lock.unlock() }
    }
}

interface ConsolePlatform {
    fun print(message: String)
    fun printerr(message: String)
    fun readln(): String?
}

open class __Platform_Console(private val impl: ConsolePlatform) : Console {
    override fun print(message: String) = impl.print(message)
    override fun printerr(message: String) = impl.printerr(message)
    override fun readln(): String? = impl.readln()
}


class __Platform_StdConsole() : __Platform_Console(salvo.platform.std.core.console.StdConsole())

fun println(console: Console, message: String) {
    console.print(message)
    console.print("\n")
}

