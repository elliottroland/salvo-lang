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
    val sched_table: SchedTable = __Mon_SchedTable(Scheduler().also { it.init() })
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

fun bodyOfPlatform(f: (Int, Long, salvo.platform.runtime.Dyn) -> Unit): salvo.platform.runtime.Body {
    return salvo.platform.runtime.bodyOf(f)
}

fun activatePlatform(b: salvo.platform.runtime.Body, kind: Int, slot: Long, value: salvo.platform.runtime.Dyn): Ran {
    return salvo.platform.runtime.activate(b, kind, slot, value)
}

fun dropBodyPlatform(b: salvo.platform.runtime.Body) {
    return salvo.platform.runtime.dropBody(b)
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

fun<T> slotOfPlatform(v: T): salvo.platform.runtime.Slot<T> {
    return salvo.platform.runtime.slotOf(v)
}

fun<T> slotEmptyPlatform(): salvo.platform.runtime.Slot<T> {
    return salvo.platform.runtime.slotEmpty()
}

fun<T> slotTakePlatform(s: salvo.platform.runtime.Slot<T>): T? {
    return salvo.platform.runtime.slotTake(s)
}

fun<T> slotPutPlatform(s: salvo.platform.runtime.Slot<T>, v: T) {
    return salvo.platform.runtime.slotPut(s, v)
}

fun<T> dropSlotPlatform(s: salvo.platform.runtime.Slot<T>) {
    return salvo.platform.runtime.dropSlot(s)
}

fun herePoolPlatform(): Int {
    return salvo.platform.runtime.herePool()
}

fun hereActorPlatform(): Int {
    return salvo.platform.runtime.hereActor()
}

fun setHerePlatform(pool: Int, actor: Int) {
    return salvo.platform.runtime.setHere(pool, actor)
}

fun mainPool(): Int {
    return 0
}

data class Delivered(
    val msg: salvo.platform.runtime.Dyn,
)

data class Answered(
    val slot: Long,
    val value: salvo.platform.runtime.Dyn,
)

fun dropDelivered(d: Delivered) {
    val __destructured2 = d
    val msg = __destructured2.msg
    dropDynPlatform(msg)
}

fun dropAnswered(a: Answered) {
    val __destructured3 = a
    val slot = __destructured3.slot
    val value = __destructured3.value
    dropDynPlatform(value)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun dropEntry(e: Union2<Delivered, Answered>) {
    if (e is Union2.U1<*, *>) {
        val d = e.value as Delivered
        dropDelivered(d)
    } else {
        dropAnswered((e.value as Answered))
    }
}

data class ActorRec(
    var body: salvo.platform.runtime.Slot<salvo.platform.runtime.Body>,
    var pool: Int,
    var bound: Int,
    var queue: kotlin.collections.ArrayDeque<Union2<Delivered, Answered>>,
    var slots: kotlin.collections.ArrayDeque<Long>,
    var userLen: Int,
    var gate: Long?,
    var running: Boolean,
    var dead: Boolean,
    var blocked: MutableList<salvo.platform.runtime.Parker>,
)

fun dropActorRec(a: ActorRec) {
    val __destructured4 = a
    val body = __destructured4.body
    val pool = __destructured4.pool
    val bound = __destructured4.bound
    val queue = __destructured4.queue
    val slots = __destructured4.slots
    val userLen = __destructured4.userLen
    val gate = __destructured4.gate
    val running = __destructured4.running
    val dead = __destructured4.dead
    val blocked = __destructured4.blocked
    dropSlotPlatform(body)
    (queue).toList().forEach({ e -> dropEntry(e) })
}

data class WaiterRec(
    var value: salvo.platform.runtime.Slot<salvo.platform.runtime.Dyn>,
    var parker: salvo.platform.runtime.Parker?,
)

fun dropWaiterRec(w: WaiterRec) {
    val __destructured5 = w
    val value = __destructured5.value
    val parker = __destructured5.parker
    dropSlotPlatform(value)
}

data class TaskRun(
    val body: salvo.platform.runtime.Body,
    val value: salvo.platform.runtime.Dyn,
)

fun dropTaskRun(t: TaskRun) {
    val __destructured6 = t
    val body = __destructured6.body
    val value = __destructured6.value
    dropBodyPlatform(body)
    dropDynPlatform(value)
}

data class PoolRec(
    var idle: MutableList<salvo.platform.runtime.Parker>,
    var tasks: kotlin.collections.ArrayDeque<TaskRun>,
)

fun dropPoolRec(p: PoolRec) {
    val __destructured7 = p
    val idle = __destructured7.idle
    val tasks = __destructured7.tasks
    (tasks).toList().forEach({ t -> dropTaskRun(t) })
}

data class ToActor(
    val addr: Int,
)

object __Codec_ToActor : salvo.WireCodec<ToActor> {
    override fun enc(v: ToActor, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.addr, out)
    }
    override fun dec(inp: salvo.WireIn): ToActor = ToActor(salvo.IntCodec.dec(inp))
}

data class ToWaiter(
    val wid: Int,
)

object __Codec_ToWaiter : salvo.WireCodec<ToWaiter> {
    override fun enc(v: ToWaiter, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.wid, out)
    }
    override fun dec(inp: salvo.WireIn): ToWaiter = ToWaiter(salvo.IntCodec.dec(inp))
}

data class ToTask(
    val pool: Int,
    val body: salvo.platform.runtime.Body,
)

fun dropToTask(t: ToTask) {
    val __destructured8 = t
    val pool = __destructured8.pool
    val body = __destructured8.body
    dropBodyPlatform(body)
}

data class Token(
    val target: Union3<ToActor, ToWaiter, ToTask>,
    val slot: Long,
)

data class WaiterMint(
    val token: Token,
    val wid: Int,
)

fun dropWaiterMint(m: WaiterMint) {
    val __destructured9 = m
    val token = __destructured9.token
    val wid = __destructured9.wid
    answer(token, erasePlatform(0))
}

data class RunActor(
    val addr: Int,
    val kind: Int,
    val slot: Long,
    val value: salvo.platform.runtime.Dyn,
    val body: salvo.platform.runtime.Body,
)

data class RunTask(
    val pool: Int,
    val body: salvo.platform.runtime.Body,
    val value: salvo.platform.runtime.Dyn,
)

fun dropRunActor(a: RunActor) {
    val __destructured10 = a
    val addr = __destructured10.addr
    val kind = __destructured10.kind
    val slot = __destructured10.slot
    val value = __destructured10.value
    val body = __destructured10.body
    dropDynPlatform(value)
    dropBodyPlatform(body)
}

fun dropRunTask(t: RunTask) {
    val __destructured11 = t
    val pool = __destructured11.pool
    val body = __destructured11.body
    val value = __destructured11.value
    dropBodyPlatform(body)
    dropDynPlatform(value)
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
    val __destructured12 = f
    val msg = __destructured12.msg
    dropDynPlatform(msg)
}

data class Got(
    val value: salvo.platform.runtime.Dyn,
)

fun dropGot(g: Got) {
    val __destructured13 = g
    val value = __destructured13.value
    dropDynPlatform(value)
}

class Sleep

object __Codec_Sleep : salvo.WireCodec<Sleep> {
    override fun enc(v: Sleep, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Sleep = Sleep()
}

interface SchedTable {
    fun newPool(): Int
    fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.Body): Int
    fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union3<Sent, Dead, Full>
    fun mintActor(addr: Int, gated: Boolean): Token
    fun mintTask(pool: Int, body: salvo.platform.runtime.Body): Token
    fun mintWaiter(): WaiterMint
    fun deliver(t: Token, value: salvo.platform.runtime.Dyn)
    fun nextWork(pool: Int, exclude: Int, idle: salvo.platform.runtime.Parker): Union2<RunActor, RunTask>?
    fun waitStep(wid: Int, pool: Int, exclude: Int, me: salvo.platform.runtime.Parker): Union4<Got, RunActor, RunTask, Sleep>
    fun finish(addr: Int, body: salvo.platform.runtime.Body, fault: String?)
    fun poolOfActor(addr: Int): Int
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
    override fun mintActor(addr: Int, gated: Boolean): Token {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.mintActor(addr, gated) }
    }
    override fun mintTask(pool: Int, body: salvo.platform.runtime.Body): Token {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.mintTask(pool, body) }
    }
    override fun mintWaiter(): WaiterMint {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.mintWaiter() }
    }
    override fun deliver(t: Token, value: salvo.platform.runtime.Dyn) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.deliver(t, value) }
    }
    override fun nextWork(pool: Int, exclude: Int, idle: salvo.platform.runtime.Parker): Union2<RunActor, RunTask>? {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.nextWork(pool, exclude, idle) }
    }
    override fun waitStep(wid: Int, pool: Int, exclude: Int, me: salvo.platform.runtime.Parker): Union4<Got, RunActor, RunTask, Sleep> {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.waitStep(wid, pool, exclude, me) }
    }
    override fun finish(addr: Int, body: salvo.platform.runtime.Body, fault: String?) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.finish(addr, body, fault) }
    }
    override fun poolOfActor(addr: Int): Int {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.poolOfActor(addr) }
    }
}

class Scheduler : SchedTable {
    private var actors: MutableList<ActorRec> = mutableListOf<ActorRec>()
    private var waiters: MutableList<WaiterRec> = mutableListOf<WaiterRec>()
    private var pools: MutableList<PoolRec> = mutableListOf<PoolRec>()
    private var nextSlot: Long = 0L

    override fun newPool(): Int {
        pools.add(PoolRec(idle = mutableListOf<salvo.platform.runtime.Parker>(), tasks = kotlin.collections.ArrayDeque<TaskRun>(listOf<TaskRun>())))
        return pools.size - 1
    }

    override fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.Body): Int {
        actors.add(ActorRec(body = slotOfPlatform(body), pool = pool, bound = bound, queue = kotlin.collections.ArrayDeque<Union2<Delivered, Answered>>(listOf<Union2<Delivered, Answered>>()), slots = kotlin.collections.ArrayDeque<Long>(listOf<Long>()), userLen = 0, gate = null, running = false, dead = false, blocked = mutableListOf<salvo.platform.runtime.Parker>()))
        return actors.size - 1
    }

    override fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union3<Sent, Dead, Full> {
        if (addr < 0 || addr >= actors.size) {
            dropDynPlatform(msg)
            return Union3.U2<Sent, Dead, Full>(Dead())
        }
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:346:17"))
        if (a.dead) {
            dropDynPlatform(msg)
            return Union3.U2<Sent, Dead, Full>(Dead())
        }
        if (a.userLen >= a.bound) {
            a.blocked.add(waiter)
            return Union3.U3<Sent, Dead, Full>(Full(msg = msg))
        }
        val e: Union2<Delivered, Answered> = Union2.U1<Delivered, Answered>(Delivered(msg = msg))
        a.queue.addLast(e)
        a.slots.addLast((-1).toLong())
        a.userLen = a.userLen + 1
        val pool = a.pool
        wakePool(pools, pool)
        return Union3.U1<Sent, Dead, Full>(Sent())
    }

    override fun mintActor(addr: Int, gated: Boolean): Token {
        nextSlot = nextSlot + 1
        val slot = nextSlot
        if (gated) {
            val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:368:21"))
            a.gate = slot
        }
        return Token(target = Union3.U1<ToActor, ToWaiter, ToTask>(ToActor(addr = addr)), slot = slot)
    }

    override fun mintTask(pool: Int, body: salvo.platform.runtime.Body): Token {
        nextSlot = nextSlot + 1
        return Token(target = Union3.U3<ToActor, ToWaiter, ToTask>(ToTask(pool = pool, body = body)), slot = nextSlot)
    }

    override fun mintWaiter(): WaiterMint {
        waiters.add(WaiterRec(value = slotEmptyPlatform(), parker = null))
        val wid = waiters.size - 1
        nextSlot = nextSlot + 1
        val t = Token(target = Union3.U2<ToActor, ToWaiter, ToTask>(ToWaiter(wid = wid)), slot = nextSlot)
        return WaiterMint(token = t, wid = wid)
    }

    override fun deliver(t: Token, value: salvo.platform.runtime.Dyn) {
        val __destructured14 = t
        val target = __destructured14.target
        val slot = __destructured14.slot
        deliverTo(actors, waiters, pools, target, slot, value)
    }

    override fun nextWork(pool: Int, exclude: Int, idle: salvo.platform.runtime.Parker): Union2<RunActor, RunTask>? {
        val w = takeWork(actors, pools, pool, exclude)
        if (w == null) {
            val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:395:21"))
            p.idle.add(idle)
            return null
        }
        return w
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun waitStep(wid: Int, pool: Int, exclude: Int, me: salvo.platform.runtime.Parker): Union4<Got, RunActor, RunTask, Sleep> {
        val w = (waiters.getOrNull(wid) ?: throw AssertionError("salvo: value is absent at runtime:404:17"))
        val got = slotTakePlatform(w.value)
        if (got != null) {
            val v = got as salvo.platform.runtime.Dyn
            w.parker = null
            return Union4.U1<Got, RunActor, RunTask, Sleep>(Got(value = v))
        }
        val work = takeWork(actors, pools, pool, exclude)
        if (work is Union2.U1<*, *>) {
            val ra = work?.value as RunActor
            return Union4.U2<Got, RunActor, RunTask, Sleep>(ra)
        }
        if (work is Union2.U2<*, *>) {
            val rt = work?.value as RunTask
            return Union4.U3<Got, RunActor, RunTask, Sleep>(rt)
        }
        w.parker = me
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:418:17"))
        p.idle.add(me)
        return Union4.U4<Got, RunActor, RunTask, Sleep>(Sleep())
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun finish(addr: Int, body: salvo.platform.runtime.Body, fault: String?) {
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:424:17"))
        a.running = false
        if (fault == null) {
            slotPutPlatform(a.body, body)
            if (a.queue.size > 0) {
                val pool = a.pool
                wakePool(pools, pool)
            }
            return
        }
        dropBodyPlatform(body)
        a.dead = true
        a.gate = null
        a.userLen = 0
        while (a.queue.size > 0) {
            dropEntry((a.queue.removeFirstOrNull() ?: throw AssertionError("salvo: value is absent at runtime:439:24")))
        }
        while (a.slots.size > 0) {
            val _s = a.slots.removeFirstOrNull()
        }
        while (true) {
            var __is1 = (a.blocked).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is1 != null)) break
            val b = __is1 as salvo.platform.runtime.Parker
            unparkPlatform(b)
        }
    }

    override fun poolOfActor(addr: Int): Int {
        return (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:450:21")).pool
    }

    fun init() {
        pools.add(PoolRec(idle = mutableListOf<salvo.platform.runtime.Parker>(), tasks = kotlin.collections.ArrayDeque<TaskRun>(listOf<TaskRun>())))
    }
}

sealed class __Priv_Scheduler {
    object Init : __Priv_Scheduler()
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun deliverTo(actors: MutableList<ActorRec>, waiters: MutableList<WaiterRec>, pools: MutableList<PoolRec>, target: Union3<ToActor, ToWaiter, ToTask>, slot: Long, value: salvo.platform.runtime.Dyn) {
    if (target is Union3.U1<*, *, *>) {
        val to = target.value as ToActor
        val a = (actors.getOrNull(to.addr) ?: throw AssertionError("salvo: value is absent at runtime:459:17"))
        if (a.dead) {
            dropDynPlatform(value)
            return
        }
        a.slots.addLast(slot)
        val e: Union2<Delivered, Answered> = Union2.U2<Delivered, Answered>(Answered(slot = slot, value = value))
        a.queue.addLast(e)
        val pool = a.pool
        wakePool(pools, pool)
    } else if (target is Union3.U2<*, *, *>) {
        val tw = target.value as ToWaiter
        val w = (waiters.getOrNull(tw.wid) ?: throw AssertionError("salvo: value is absent at runtime:470:17"))
        slotPutPlatform(w.value, value)
        if (w.parker != null) {
            val p = w.parker as salvo.platform.runtime.Parker
            unparkPlatform(p)
        }
    } else {
        val __destructured15 = (target.value as ToTask)
        val pool = __destructured15.pool
        val body = __destructured15.body
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:477:17"))
        p.tasks.addLast(TaskRun(body = body, value = value))
        wakePool(pools, pool)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun takeWork(actors: MutableList<ActorRec>, pools: MutableList<PoolRec>, pool: Int, exclude: Int): Union2<RunActor, RunTask>? {
    val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:488:13"))
    val task = p.tasks.removeFirstOrNull()
    if (task != null) {
        val t = task as TaskRun
        val __destructured16 = t
        val body = __destructured16.body
        val value = __destructured16.value
        return Union2.U2<RunActor, RunTask>(RunTask(pool = pool, body = body, value = value))
    }
    var i = 0
    while (i < actors.size) {
        val a = (actors.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime:496:17"))
        if (a.pool == pool && i != exclude && !a.running && !a.dead) {
            val at = deliverable(a.slots, a.gate)
            if (at != null) {
                val k = at as Int
                val _slot = (a.slots).let { __l -> (k).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
                val e = ((a.queue).let { __l -> (k).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } } ?: throw AssertionError("salvo: value is absent at runtime:501:25"))
                a.running = true
                val body = (slotTakePlatform(a.body) ?: throw AssertionError("salvo: value is absent at runtime:503:28"))
                return workOf(a, i, e, body)
            }
        }
        i = i + 1
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun workOf(a: ActorRec, addr: Int, e: Union2<Delivered, Answered>, body: salvo.platform.runtime.Body): Union2<RunActor, RunTask>? {
    if (e is Union2.U1<*, *>) {
        val d = e.value as Delivered
        a.userLen = a.userLen - 1
        val woken = (a.blocked).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        if (woken != null) {
            val b = woken as salvo.platform.runtime.Parker
            unparkPlatform(b)
        }
        val __destructured17 = d
        val msg = __destructured17.msg
        return Union2.U1<RunActor, RunTask>(RunActor(addr = addr, kind = 0, slot = 0L, value = msg, body = body))
    }
    val __destructured18 = (e.value as Answered)
    val slot = __destructured18.slot
    val value = __destructured18.value
    var opens = false
    if (a.gate != null) {
        val g = a.gate as Long
        opens = g == slot
    }
    if (opens) {
        a.gate = null
    }
    return Union2.U1<RunActor, RunTask>(RunActor(addr = addr, kind = 1, slot = slot, value = value, body = body))
}

fun deliverable(slots: kotlin.collections.ArrayDeque<Long>, gate: Long?): Int? {
    if (slots.size == 0) {
        return null
    }
    if (gate == null) {
        return 0
    }
    var i = 0
    while (i < slots.size) {
        if ((slots.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime:546:12")) == gate) {
            return i
        }
        i = i + 1
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun wakePool(pools: MutableList<PoolRec>, pool: Int) {
    val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:556:13"))
    while (true) {
        var __is2 = (p.idle).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        if (!(__is2 != null)) break
        val w = __is2 as salvo.platform.runtime.Parker
        unparkPlatform(w)
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
        val __destructured19 = full
        val back = __destructured19.msg
        r = __moduleUse1.enqueue(addr, back, thisParkerPlatform())
    }
    if (r is Union3.U3<*, *, *>) {
        val full = r.value as Full
        dropFull(full)
    }
}

fun mint(addr: Int, gated: Boolean): Token {
    return __moduleUse1.mintActor(addr, gated)
}

fun mintTaskOn(pool: Int, body: salvo.platform.runtime.Body): Token {
    return __moduleUse1.mintTask(pool, body)
}

fun waiter(): WaiterMint {
    return __moduleUse1.mintWaiter()
}

fun answer(t: Token, value: salvo.platform.runtime.Dyn) {
    __moduleUse1.deliver(t, value)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun awaitAnswer(wid: Int): salvo.platform.runtime.Dyn {
    val pool = herePoolPlatform()
    val own = hereActorPlatform()
    while (true) {
        val step = __moduleUse1.waitStep(wid, pool, own, thisParkerPlatform())
        if (step is Union4.U1<*, *, *, *>) {
            val g = step.value as Got
            val __destructured20 = g
            val value = __destructured20.value
            return value
        }
        if (step is Union4.U2<*, *, *, *>) {
            val ra = step.value as RunActor
            runActor(ra)
        } else if (step is Union4.U3<*, *, *, *>) {
            val rt = step.value as RunTask
            runTask(rt)
        } else if (step is Union4.U4<*, *, *, *>) {
            parkPlatform(thisParkerPlatform())
        }
    }
    return unerasePlatform(erasePlatform(0))
}

fun runActor(ra: RunActor) {
    val __destructured21 = ra
    val addr = __destructured21.addr
    val kind = __destructured21.kind
    val slot = __destructured21.slot
    val value = __destructured21.value
    val body = __destructured21.body
    val savedPool = herePoolPlatform()
    val savedActor = hereActorPlatform()
    val pool = actorPool(addr)
    setHerePlatform(pool, addr)
    val ran = activatePlatform(body, kind, slot, value)
    setHerePlatform(savedPool, savedActor)
    val __destructured22 = ran
    val back = __destructured22.body
    val fault = __destructured22.fault
    __moduleUse1.finish(addr, back, fault)
}

fun runTask(rt: RunTask) {
    val __destructured23 = rt
    val pool = __destructured23.pool
    val body = __destructured23.body
    val value = __destructured23.value
    val savedPool = herePoolPlatform()
    val savedActor = hereActorPlatform()
    setHerePlatform(pool, -1)
    val ran = activatePlatform(body, 1, 0L, value)
    setHerePlatform(savedPool, savedActor)
    val __destructured24 = ran
    val done = __destructured24.body
    val fault = __destructured24.fault
    dropBodyPlatform(done)
}

fun actorPool(addr: Int): Int {
    return __moduleUse1.poolOfActor(addr)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun servePool(pool: Int) {
    setHerePlatform(pool, -1)
    while (true) {
        val w = __moduleUse1.nextWork(pool, -1, thisParkerPlatform())
        if (w is Union2.U1<*, *>) {
            val ra = w?.value as RunActor
            runActor(ra)
        } else if (w is Union2.U2<*, *>) {
            val rt = w?.value as RunTask
            runTask(rt)
        } else {
            parkPlatform(thisParkerPlatform())
        }
    }
}
