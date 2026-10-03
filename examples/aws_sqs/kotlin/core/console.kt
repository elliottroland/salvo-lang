package salvo.core.console

import salvo.core.string.*

interface Console {
    fun print(message: String)
}

class __Mon_Console(
    private val inner: Console,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Console {
    override fun print(message: String) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.print(message) } finally { lock.unlock() }
    }
}

class StdOutConsole : Console {
    override fun print(message: String) {
        kotlin.io.print(message)
    }
}

fun println(console: Console, message: String) {
    console.print(message)
    console.print("\n")
}
