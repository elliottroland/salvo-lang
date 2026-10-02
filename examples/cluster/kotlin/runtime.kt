package salvo.runtime

import salvo.*
import salvo.core.actor.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.deque.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

// [mod-use] The module's `use` #0, bound on first use.
private val __moduleUse0: RuntimeHost by lazy {
    val runtime_host: RuntimeHost = salvo.runtime.__Platform_HostRuntime()
    runtime_host
}

// [mod-use] The module's `use` #1, bound on first use.
private val __moduleUse1: SchedTable by lazy {
    val sched_table: SchedTable = __Mon_SchedTable(Scheduler())
    sched_table
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

fun startThreadPlatform(body: () -> Unit) {
    return salvo.platform.runtime.startThread(body)
}

fun guardedPlatform(body: () -> Unit): String? {
    return salvo.platform.runtime.guarded(body)
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

fun<T> erasePlatform(v: T): salvo.platform.runtime.Dyn {
    return salvo.platform.runtime.erase(v)
}

fun<T> unerasePlatform(d: salvo.platform.runtime.Dyn): T {
    return salvo.platform.runtime.unerase(d)
}

fun dropDynPlatform(d: salvo.platform.runtime.Dyn) {
    return salvo.platform.runtime.dropDyn(d)
}

fun bodyOfPlatform(f: (salvo.platform.runtime.Dyn) -> Unit): salvo.platform.runtime.Body {
    return salvo.platform.runtime.bodyOf(f)
}

fun activatePlatform(b: salvo.platform.runtime.Body, msg: salvo.platform.runtime.Dyn): Ran {
    return salvo.platform.runtime.activate(b, msg)
}

data class Ran(
    val body: salvo.platform.runtime.Body,
    val fault: String?,
)

fun dropRan(r: Ran) {
    val __destructured1 = r
    val body = __destructured1.body
    val fault = __destructured1.fault
    dropBodyPlatform(body)
}

fun dropBodyPlatform(b: salvo.platform.runtime.Body) {
    return salvo.platform.runtime.dropBody(b)
}

fun<T> slotOfPlatform(v: T): salvo.platform.runtime.Slot<T> {
    return salvo.platform.runtime.slotOf(v)
}

fun<T> slotTakePlatform(s: salvo.platform.runtime.Slot<T>): T? {
    return salvo.platform.runtime.slotTake(s)
}

fun<T> slotPutPlatform(s: salvo.platform.runtime.Slot<T>, v: T) {
    return salvo.platform.runtime.slotPut(s, v)
}

fun dropSlotPlatform(s: salvo.platform.runtime.Slot<salvo.platform.runtime.Body>) {
    return salvo.platform.runtime.dropSlot(s)
}

data class ActorRec(
    var body: salvo.platform.runtime.Slot<salvo.platform.runtime.Body>,
    var pool: Int,
    var bound: Int,
    var queue: kotlin.collections.ArrayDeque<salvo.platform.runtime.Dyn>,
    var running: Boolean,
    var dead: Boolean,
    var blocked: MutableList<salvo.platform.runtime.Parker>,
)

fun dropActorRec(a: ActorRec) {
    val __destructured2 = a
    val body = __destructured2.body
    val pool = __destructured2.pool
    val bound = __destructured2.bound
    val queue = __destructured2.queue
    val running = __destructured2.running
    val dead = __destructured2.dead
    val blocked = __destructured2.blocked
    dropSlotPlatform(body)
    (queue).toList().forEach({ d -> dropDynPlatform(d) })
}

data class Job(
    val addr: Int,
    val msg: salvo.platform.runtime.Dyn,
    val body: salvo.platform.runtime.Body,
)

fun dropJob(j: Job) {
    val __destructured3 = j
    val addr = __destructured3.addr
    val msg = __destructured3.msg
    val body = __destructured3.body
    dropDynPlatform(msg)
    dropBodyPlatform(body)
}

interface SchedTable {
    fun newPool(): Int
    fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.Body): Int
    fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union3<Sent, Dead, Full>
    fun nextJob(pool: Int, idle: salvo.platform.runtime.Parker): Job?
    fun finish(addr: Int, body: salvo.platform.runtime.Body, fault: String?)
}

class __Mon_SchedTable(private val inner: SchedTable) : SchedTable {
    override fun newPool(): Int {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.newPool() }
    }
    override fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.Body): Int {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.newActor(pool, bound, body) }
    }
    override fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union3<Sent, Dead, Full> {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.enqueue(addr, msg, waiter) }
    }
    override fun nextJob(pool: Int, idle: salvo.platform.runtime.Parker): Job? {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.nextJob(pool, idle) }
    }
    override fun finish(addr: Int, body: salvo.platform.runtime.Body, fault: String?) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.finish(addr, body, fault) }
    }
}

class Sent

object __Codec_Sent : salvo.WireCodec<Sent> {
    override fun enc(v: Sent, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Sent = Sent()
}

class Dead

object __Codec_Dead : salvo.WireCodec<Dead> {
    override fun enc(v: Dead, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Dead = Dead()
}

data class Full(
    val msg: salvo.platform.runtime.Dyn,
)

fun dropFull(f: Full) {
    val __destructured4 = f
    val msg = __destructured4.msg
    dropDynPlatform(msg)
}

class Scheduler : SchedTable {
    private var actors: MutableList<ActorRec> = mutableListOf<ActorRec>()
    private var idle: MutableList<MutableList<salvo.platform.runtime.Parker>> = mutableListOf<MutableList<salvo.platform.runtime.Parker>>()

    override fun newPool(): Int {
        idle.add(mutableListOf<salvo.platform.runtime.Parker>())
        return idle.size - 1
    }

    override fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.Body): Int {
        actors.add(ActorRec(body = slotOfPlatform(body), pool = pool, bound = bound, queue = kotlin.collections.ArrayDeque<salvo.platform.runtime.Dyn>(listOf<salvo.platform.runtime.Dyn>()), running = false, dead = false, blocked = mutableListOf<salvo.platform.runtime.Parker>()))
        return actors.size - 1
    }

    override fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union3<Sent, Dead, Full> {
        if (addr < 0 || addr >= actors.size) {
            dropDynPlatform(msg)
            return Union3.U2<Sent, Dead, Full>(Dead())
        }
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:210:17"))
        if (a.dead) {
            dropDynPlatform(msg)
            return Union3.U2<Sent, Dead, Full>(Dead())
        }
        if (a.queue.size >= a.bound) {
            a.blocked.add(waiter)
            return Union3.U3<Sent, Dead, Full>(Full(msg = msg))
        }
        a.queue.addLast(msg)
        val pool = a.pool
        wakeAll(idle, pool)
        return Union3.U1<Sent, Dead, Full>(Sent())
    }

    override fun nextJob(pool: Int, idleParker: salvo.platform.runtime.Parker): Job? {
        var i = 0
        while (i < actors.size) {
            val a = (actors.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime:228:21"))
            if (a.pool == pool && !a.running && !a.dead && a.queue.size > 0) {
                val msg = (a.queue.removeFirstOrNull() ?: throw AssertionError("salvo: value is absent at runtime:230:27"))
                a.running = true
                val woken = (a.blocked).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
                if (woken != null) {
                    unparkPlatform(woken)
                }
                val body = (slotTakePlatform(a.body) ?: throw AssertionError("salvo: value is absent at runtime:236:28"))
                return Job(addr = i, msg = msg, body = body)
            }
            i = i + 1
        }
        val ps = (idle.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:241:18"))
        ps.add(idleParker)
        return null
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun finish(addr: Int, body: salvo.platform.runtime.Body, fault: String?) {
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:247:17"))
        a.running = false
        if (fault == null) {
            slotPutPlatform(a.body, body)
            if (a.queue.size > 0) {
                val pool = a.pool
                wakeAll(idle, pool)
            }
            return
        }
        dropBodyPlatform(body)
        a.dead = true
        while (true) {
            var __is1 = a.queue.removeFirstOrNull()
            if (!(__is1 != null)) break
            val d = __is1 as salvo.platform.runtime.Dyn
            dropDynPlatform(d)
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun wakeAll(idle: MutableList<MutableList<salvo.platform.runtime.Parker>>, pool: Int) {
    val ps = (idle.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:267:14"))
    while (true) {
        var __is2 = (ps).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        if (!(__is2 != null)) break
        val p = __is2 as salvo.platform.runtime.Parker
        unparkPlatform(p)
    }
}

fun newPoolOf(n: Int): Int {
    val id = __moduleUse1.newPool()
    var i = 0
    while (i < n) {
        startThreadPlatform({  ->
    servePool(id)
})
        i = i + 1
    }
    return id
}

fun spawnBody(pool: Int, bound: Int, body: salvo.platform.runtime.Body): Int {
    return __moduleUse1.newActor(pool, bound, body)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun sendDyn(addr: Int, msg: salvo.platform.runtime.Dyn) {
    var r = __moduleUse1.enqueue(addr, msg, thisParkerPlatform())
    while (r is Union3.U3<*, *, *>) {
        val full = r.value as Full
        parkPlatform(thisParkerPlatform())
        val __destructured5 = full
        val back = __destructured5.msg
        r = __moduleUse1.enqueue(addr, back, thisParkerPlatform())
    }
    if (r is Union3.U3<*, *, *>) {
        val full = r.value as Full
        dropFull(full)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun servePool(pool: Int) {
    while (true) {
        val job = __moduleUse1.nextJob(pool, thisParkerPlatform())
        if (job != null) {
            val j = job as Job
            val __destructured6 = j
            val addr = __destructured6.addr
            val msg = __destructured6.msg
            val body = __destructured6.body
            val ran = activatePlatform(body, msg)
            val __destructured7 = ran
            val back = __destructured7.body
            val fault = __destructured7.fault
            __moduleUse1.finish(addr, back, fault)
        } else {
            parkPlatform(thisParkerPlatform())
        }
    }
}
