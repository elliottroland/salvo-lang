package salvo.runtime

import salvo.core.string.*

// [mod-use] The module's `use` #0, bound on first use.
private val __moduleUse0: RuntimeHost by lazy {
    val runtime_host: RuntimeHost = salvo.runtime.__Platform_HostRuntime()
    runtime_host
}

fun thisParkerPlatform(): salvo.platform.runtime.Parker {
    return salvo.platform.runtime.thisParker()
}

fun parkPlatform(p: salvo.platform.runtime.Parker) {
    return salvo.platform.runtime.park(p)
}

fun parkNanosPlatform(p: salvo.platform.runtime.Parker, nanos: Long) {
    return salvo.platform.runtime.parkNanos(p, nanos)
}

fun unparkPlatform(p: salvo.platform.runtime.Parker) {
    return salvo.platform.runtime.unpark(p)
}

interface RuntimeHost {
    fun secureBits(): Long
    fun report(line: String)
    fun monoNanos(): Long
}

class __Mon_RuntimeHost(private val inner: RuntimeHost) : RuntimeHost {
    override fun secureBits(): Long {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.secureBits() }
    }
    override fun report(line: String) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.report(line) }
    }
    override fun monoNanos(): Long {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.monoNanos() }
    }
}

// The interface a `platform handler` of `RuntimeHost` implements [platform-abi].
interface RuntimeHostPlatform {
    fun secureBits(): Long
    fun report(line: String)
    fun monoNanos(): Long
}

open class __Platform_RuntimeHost(private val impl: RuntimeHostPlatform) : RuntimeHost {
    override fun secureBits(): Long = impl.secureBits()
    override fun report(line: String) = impl.report(line)
    override fun monoNanos(): Long = impl.monoNanos()
}

class __Platform_HostRuntime() : salvo.runtime.__Platform_RuntimeHost(salvo.platform.runtime.HostRuntime())

fun freshBits(): Long {
    return __moduleUse0.secureBits()
}

fun nowNanos(): Long {
    return __moduleUse0.monoNanos()
}
