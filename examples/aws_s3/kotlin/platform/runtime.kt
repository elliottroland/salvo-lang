// Host implementation of the platform declarations of Salvo module `runtime`:
// the primitives only the host can provide (RUNTIME.md §11.3). Written by
// hand, against the generated `runtime.sv.kt`.
package salvo.platform.runtime

import java.util.concurrent.locks.LockSupport

// `threadsafe platform type Parker` [runtime-parker]: a thread's handle.
// `LockSupport.unpark` keeps a permit, which is the contract `Parker` states.
class Parker(val thread: Thread)

fun thisParker(): Parker = Parker(Thread.currentThread())

private fun own(p: Parker) {
    check(p.thread === Thread.currentThread()) {
        "salvo: a thread parked on another thread's parker [runtime-parker]"
    }
}

fun park(p: Parker) {
    own(p)
    LockSupport.park()
}

fun parkNanos(p: Parker, nanos: Long) {
    own(p)
    LockSupport.parkNanos(if (nanos < 0) 0 else nanos)
}

fun unpark(p: Parker) {
    LockSupport.unpark(p.thread)
}

// `threadsafe platform handler HostRuntime` [runtime-host]: stateless but for
// the `SecureRandom`, which is itself thread-safe.
class HostRuntime : salvo.runtime.RuntimeHostPlatform {
    private val random = java.security.SecureRandom()

    // [addr-capability] OS entropy, through the platform's secure source.
    override fun secureBits(): Long = random.nextLong()

    override fun report(line: String) {
        System.err.println(line)
    }

    // [time-timer] The clock `time.tick()` reads.
    override fun monoNanos(): Long = salvo.SalvoTime.monoNanos()
}
