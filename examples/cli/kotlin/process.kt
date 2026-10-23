package salvo.process

import salvo.*

interface Process {
    fun exit(code: Int): Nothing
}

class __Mon_Process(
    private val inner: Process,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Process {
    override fun exit(code: Int): Nothing {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.exit(code) } finally { lock.unlock() }
    }
}

interface ProcessPlatform {
    fun exit(code: Int): Nothing
}

open class __Platform_Process(private val impl: ProcessPlatform) : Process {
    override fun exit(code: Int): Nothing = impl.exit(code)
}


class __Platform_HostProcess() : __Platform_Process(salvo.platform.process.HostProcess())

