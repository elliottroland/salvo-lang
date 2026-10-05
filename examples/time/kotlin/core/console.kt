package salvo.core.console


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

// The interface a `platform handler` of `Console` implements [platform-abi].
interface ConsolePlatform {
    fun print(message: String)
}

open class __Platform_Console(private val impl: ConsolePlatform) : Console {
    override fun print(message: String) = impl.print(message)
}

class __Platform_StdOutConsole() : salvo.core.console.__Platform_Console(salvo.platform.core.console.StdOutConsole())

fun println(console: Console, message: String) {
    console.print(message)
    console.print("\n")
}
