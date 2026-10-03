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

// [runtime-kept-fn] A daemon thread, so it does not hold the JVM open after
// `main` returns.
fun startThread(body: () -> Unit) {
    val t = Thread { body() }
    t.isDaemon = true
    t.start()
}

// [runtime-kept-fn] The fault boundary: a throw is answered as its message.
fun guarded(body: () -> Unit): String? =
    try {
        body()
        null
    } catch (t: Throwable) {
        t.message ?: t.javaClass.simpleName
    }

// [runtime-sched] `linear platform type Dyn`: an erased value.
class Dyn(val v: Any?)

fun <T> erase(v: T): Dyn = Dyn(v)

@Suppress("UNCHECKED_CAST")
fun <T> unerase(d: Dyn): T = d.v as T

fun dropDyn(d: Dyn) {}

// [runtime-sched] `linear platform type RtBody`: what an activation runs.
class RtBody(val f: (Int, Long, Dyn) -> Unit)

fun bodyOf(f: (Int, Long, Dyn) -> Unit): RtBody = RtBody(f)

fun activate(b: RtBody, kind: Int, slot: Long, msg: Dyn): salvo.runtime.RtRan =
    try {
        b.f(kind, slot, msg)
        salvo.runtime.RtRan(b, null)
    } catch (t: Throwable) {
        salvo.runtime.RtRan(b, t.message ?: t.javaClass.simpleName)
    }

fun dropBody(b: RtBody) {}

// [remote-backpressure] The routing layer's: a GRANT staged for the next
// flush, and the flush.
fun granted(addr: Int, from: Long) = salvo.SalvoSched.granted(addr, from)

fun flushFrames() = salvo.SalvoSched.flushFrames()

fun exitProcess(code: Int): Nothing = kotlin.system.exitProcess(code)

// [runtime-sched] `linear platform type RtSlot<T>`: a cell of at most one value.
class RtSlot<T>(var v: T?)

fun <T> slotOf(v: T): RtSlot<T> = RtSlot(v)

fun <T> slotEmpty(): RtSlot<T> = RtSlot(null)

fun <T> slotTake(s: RtSlot<T>): T? {
    val out = s.v
    s.v = null
    return out
}

fun <T> slotPut(s: RtSlot<T>, v: T) {
    s.v = v
}

fun <T> dropSlot(s: RtSlot<T>) {}

// [runtime-sched] Where this thread is: its pool and the actor whose
// activation it is inside (-1 for none). Main's thread starts on pool 0.
private val here = ThreadLocal.withInitial { intArrayOf(0, -1) }

fun herePool(): Int = here.get()[0]

fun hereActor(): Int = here.get()[1]

fun setHere(pool: Int, actor: Int) {
    here.set(intArrayOf(pool, actor))
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
