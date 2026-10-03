package salvo.runtime

import salvo.*
import salvo.core.actor.*
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

class __Mon_RuntimeHost(
    private val inner: RuntimeHost,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : RuntimeHost {
    override fun secureBits(): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.secureBits() } finally { lock.unlock() }
    }
    override fun report(line: String) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.report(line) } finally { lock.unlock() }
    }
    override fun monoNanos(): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.monoNanos() } finally { lock.unlock() }
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

fun bodyOfPlatform(f: (Int, Long, salvo.platform.runtime.Dyn) -> Unit): salvo.platform.runtime.RtBody {
    return salvo.platform.runtime.bodyOf(f)
}

fun activatePlatform(b: salvo.platform.runtime.RtBody, kind: Int, slot: Long, value: salvo.platform.runtime.Dyn): RtRan {
    return salvo.platform.runtime.activate(b, kind, slot, value)
}

fun dropBodyPlatform(b: salvo.platform.runtime.RtBody) {
    return salvo.platform.runtime.dropBody(b)
}

fun grantedPlatform(addr: Int, from: Long) {
    return salvo.platform.runtime.granted(addr, from)
}

fun flushFramesPlatform() {
    return salvo.platform.runtime.flushFrames()
}

fun exitProcessPlatform(code: Int): Nothing {
    return salvo.platform.runtime.exitProcess(code)
}

data class RtRan(
    val body: salvo.platform.runtime.RtBody,
    val fault: String?,
)

fun dropRan(r: RtRan) {
    val __destructured1 = r
    val body = __destructured1.body
    val fault = __destructured1.fault
    dropBodyPlatform(body)
}

fun<T> slotOfPlatform(v: T): salvo.platform.runtime.RtSlot<T> {
    return salvo.platform.runtime.slotOf(v)
}

fun<T> slotEmptyPlatform(): salvo.platform.runtime.RtSlot<T> {
    return salvo.platform.runtime.slotEmpty()
}

fun<T> slotTakePlatform(s: salvo.platform.runtime.RtSlot<T>): T? {
    return salvo.platform.runtime.slotTake(s)
}

fun<T> slotPutPlatform(s: salvo.platform.runtime.RtSlot<T>, v: T) {
    return salvo.platform.runtime.slotPut(s, v)
}

fun<T> dropSlotPlatform(s: salvo.platform.runtime.RtSlot<T>) {
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

fun noFrame(): Int {
    return -1
}

fun taskFrame(): Int {
    return -2
}

data class RtDelivered(
    val msg: salvo.platform.runtime.Dyn,
    val from: Long,
)

data class RtAnswered(
    val slot: Long,
    val value: salvo.platform.runtime.Dyn,
)

data class RtReported(
    val reason: String,
)

object __Codec_RtReported : salvo.WireCodec<RtReported> {
    override fun enc(v: RtReported, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.reason, out)
    }
    override fun dec(inp: salvo.WireIn): RtReported = RtReported(salvo.StrCodec.dec(inp))
}

fun dropDelivered(d: RtDelivered) {
    val __destructured2 = d
    val msg = __destructured2.msg
    val from = __destructured2.from
    dropDynPlatform(msg)
}

fun dropAnswered(a: RtAnswered) {
    val __destructured3 = a
    val slot = __destructured3.slot
    val value = __destructured3.value
    dropDynPlatform(value)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun dropEntry(e: Union3<RtDelivered, RtAnswered, RtReported>) {
    if (e is Union3.U1<*, *, *>) {
        val d = e.value as RtDelivered
        dropDelivered(d)
    } else if (e is Union3.U2<*, *, *>) {
        val a = e.value as RtAnswered
        dropAnswered(a)
    } else {
        ((e.value as RtReported)).let {}
    }
}

data class RtActorRec(
    var body: salvo.platform.runtime.RtSlot<salvo.platform.runtime.RtBody>,
    var pool: Int,
    var bound: Int,
    var queue: kotlin.collections.ArrayDeque<Union3<RtDelivered, RtAnswered, RtReported>>,
    var slots: kotlin.collections.ArrayDeque<Long>,
    var userLen: Int,
    var gate: Long?,
    var running: Boolean,
    var dead: Boolean,
    var exitReason: String,
    var blocked: MutableList<salvo.platform.runtime.Parker>,
    var watchers: MutableList<RtToken>,
    var owed: Int,
    var ready: Boolean,
)

fun dropActorRec(a: RtActorRec) {
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
    val exitReason = __destructured4.exitReason
    val blocked = __destructured4.blocked
    val watchers = __destructured4.watchers
    val owed = __destructured4.owed
    val ready = __destructured4.ready
    dropSlotPlatform(body)
    (queue).toList().forEach({ e -> dropEntry(e) })
    (watchers).toList().forEach({ t -> dropToken(t) })
}

data class RtWaiterRec(
    var pool: Int,
    var value: salvo.platform.runtime.RtSlot<salvo.platform.runtime.Dyn>,
    var filled: Boolean,
    var parker: salvo.platform.runtime.Parker?,
    var waiting: Int,
    var waitingActor: Int,
)

fun dropWaiterRec(w: RtWaiterRec) {
    val __destructured5 = w
    val pool = __destructured5.pool
    val value = __destructured5.value
    val filled = __destructured5.filled
    val parker = __destructured5.parker
    val waiting = __destructured5.waiting
    val waitingActor = __destructured5.waitingActor
    dropSlotPlatform(value)
}

data class RtTaskRun(
    val body: salvo.platform.runtime.RtBody,
    val value: salvo.platform.runtime.Dyn,
)

fun dropTaskRun(t: RtTaskRun) {
    val __destructured6 = t
    val body = __destructured6.body
    val value = __destructured6.value
    dropBodyPlatform(body)
    dropDynPlatform(value)
}

data class RtPoolRec(
    var idle: MutableList<salvo.platform.runtime.Parker>,
    var tasks: kotlin.collections.ArrayDeque<RtTaskRun>,
    var ready: kotlin.collections.ArrayDeque<Int>,
    var sink: Int,
    var owed: Int,
)

fun dropPoolRec(p: RtPoolRec) {
    val __destructured7 = p
    val idle = __destructured7.idle
    val tasks = __destructured7.tasks
    val ready = __destructured7.ready
    val sink = __destructured7.sink
    val owed = __destructured7.owed
    (tasks).toList().forEach({ t -> dropTaskRun(t) })
}

data class RtToActor(
    val addr: Int,
)

object __Codec_RtToActor : salvo.WireCodec<RtToActor> {
    override fun enc(v: RtToActor, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.addr, out)
    }
    override fun dec(inp: salvo.WireIn): RtToActor = RtToActor(salvo.IntCodec.dec(inp))
}

data class RtToWaiter(
    val wid: Int,
)

object __Codec_RtToWaiter : salvo.WireCodec<RtToWaiter> {
    override fun enc(v: RtToWaiter, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.wid, out)
    }
    override fun dec(inp: salvo.WireIn): RtToWaiter = RtToWaiter(salvo.IntCodec.dec(inp))
}

data class RtToTask(
    val pool: Int,
    val body: salvo.platform.runtime.RtBody,
)

fun dropToTask(t: RtToTask) {
    val __destructured8 = t
    val pool = __destructured8.pool
    val body = __destructured8.body
    dropBodyPlatform(body)
}

data class RtToken(
    val target: Union3<RtToActor, RtToWaiter, RtToTask>,
    val slot: Long,
    val tracked: Boolean,
)

fun dropToken(t: RtToken) {
    answer(t, erasePlatform(0))
}

data class RtWaiterMint(
    val token: RtToken,
    val wid: Int,
)

fun dropWaiterMint(m: RtWaiterMint) {
    val __destructured9 = m
    val token = __destructured9.token
    val wid = __destructured9.wid
    dropToken(token)
}

data class RtRunActor(
    val addr: Int,
    val pool: Int,
    val kind: Int,
    val slot: Long,
    val value: salvo.platform.runtime.Dyn,
    val body: salvo.platform.runtime.RtBody,
)

data class RtRunTask(
    val pool: Int,
    val body: salvo.platform.runtime.RtBody,
    val value: salvo.platform.runtime.Dyn,
)

fun dropRunActor(a: RtRunActor) {
    val __destructured10 = a
    val addr = __destructured10.addr
    val pool = __destructured10.pool
    val kind = __destructured10.kind
    val slot = __destructured10.slot
    val value = __destructured10.value
    val body = __destructured10.body
    dropDynPlatform(value)
    dropBodyPlatform(body)
}

fun dropRunTask(t: RtRunTask) {
    val __destructured11 = t
    val pool = __destructured11.pool
    val body = __destructured11.body
    val value = __destructured11.value
    dropBodyPlatform(body)
    dropDynPlatform(value)
}

class RtSent

object __Codec_RtSent : salvo.WireCodec<RtSent> {
    override fun enc(v: RtSent, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): RtSent = RtSent()
}

class RtDead

object __Codec_RtDead : salvo.WireCodec<RtDead> {
    override fun enc(v: RtDead, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): RtDead = RtDead()
}

data class RtFull(
    val msg: salvo.platform.runtime.Dyn,
)

fun dropFull(f: RtFull) {
    val __destructured12 = f
    val msg = __destructured12.msg
    dropDynPlatform(msg)
}

data class RtIdleHook(
    val pool: Int,
    val token: RtToken,
)

fun dropIdleHook(h: RtIdleHook) {
    val __destructured13 = h
    val pool = __destructured13.pool
    val token = __destructured13.token
    dropToken(token)
}

data class RtGot(
    val value: salvo.platform.runtime.Dyn,
)

class RtSleep

object __Codec_RtSleep : salvo.WireCodec<RtSleep> {
    override fun enc(v: RtSleep, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): RtSleep = RtSleep()
}

class RtAgain

object __Codec_RtAgain : salvo.WireCodec<RtAgain> {
    override fun enc(v: RtAgain, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): RtAgain = RtAgain()
}

data class RtStuck(
    val report: String,
)

object __Codec_RtStuck : salvo.WireCodec<RtStuck> {
    override fun enc(v: RtStuck, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.report, out)
    }
    override fun dec(inp: salvo.WireIn): RtStuck = RtStuck(salvo.StrCodec.dec(inp))
}

fun dropGot(g: RtGot) {
    val __destructured14 = g
    val value = __destructured14.value
    dropDynPlatform(value)
}

interface SchedTable {
    fun newPool(sink: Int): Int
    fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.RtBody): Int
    fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union3<RtSent, RtDead, RtFull>
    fun enqueueRemote(addr: Int, msg: salvo.platform.runtime.Dyn, from: Long): Boolean
    fun kill(addr: Int, reason: String)
    fun mintActor(addr: Int, gated: Boolean): RtToken
    fun mintTask(pool: Int, body: salvo.platform.runtime.RtBody): RtToken
    fun mintWaiter(pool: Int): RtWaiterMint
    fun deliver(t: RtToken, value: salvo.platform.runtime.Dyn)
    fun watchActor(addr: Int, t: RtToken)
    fun idleHook(pool: Int, t: RtToken)
    fun nextWork(pool: Int, idle: salvo.platform.runtime.Parker): Union2<RtRunActor, RtRunTask>?
    fun waitStep(wid: Int, pool: Int, own: Int, frame: Int, me: salvo.platform.runtime.Parker): Union6<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>
    fun finish(addr: Int, body: salvo.platform.runtime.RtBody, fault: String?)
    fun taskDone(pool: Int, fault: String?)
    fun poolOfActor(addr: Int): Int
    fun external(delta: Int)
    fun room(addr: Int): Int
    fun isDead(addr: Int): Boolean
    fun queued(addr: Int): Int
}

class __Mon_SchedTable(
    private val inner: SchedTable,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : SchedTable {
    override fun newPool(sink: Int): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.newPool(sink) } finally { lock.unlock() }
    }
    override fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.RtBody): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.newActor(pool, bound, body) } finally { lock.unlock() }
    }
    override fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union3<RtSent, RtDead, RtFull> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.enqueue(addr, msg, waiter) } finally { lock.unlock() }
    }
    override fun enqueueRemote(addr: Int, msg: salvo.platform.runtime.Dyn, from: Long): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.enqueueRemote(addr, msg, from) } finally { lock.unlock() }
    }
    override fun kill(addr: Int, reason: String) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.kill(addr, reason) } finally { lock.unlock() }
    }
    override fun mintActor(addr: Int, gated: Boolean): RtToken {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.mintActor(addr, gated) } finally { lock.unlock() }
    }
    override fun mintTask(pool: Int, body: salvo.platform.runtime.RtBody): RtToken {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.mintTask(pool, body) } finally { lock.unlock() }
    }
    override fun mintWaiter(pool: Int): RtWaiterMint {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.mintWaiter(pool) } finally { lock.unlock() }
    }
    override fun deliver(t: RtToken, value: salvo.platform.runtime.Dyn) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.deliver(t, value) } finally { lock.unlock() }
    }
    override fun watchActor(addr: Int, t: RtToken) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.watchActor(addr, t) } finally { lock.unlock() }
    }
    override fun idleHook(pool: Int, t: RtToken) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.idleHook(pool, t) } finally { lock.unlock() }
    }
    override fun nextWork(pool: Int, idle: salvo.platform.runtime.Parker): Union2<RtRunActor, RtRunTask>? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.nextWork(pool, idle) } finally { lock.unlock() }
    }
    override fun waitStep(wid: Int, pool: Int, own: Int, frame: Int, me: salvo.platform.runtime.Parker): Union6<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.waitStep(wid, pool, own, frame, me) } finally { lock.unlock() }
    }
    override fun finish(addr: Int, body: salvo.platform.runtime.RtBody, fault: String?) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.finish(addr, body, fault) } finally { lock.unlock() }
    }
    override fun taskDone(pool: Int, fault: String?) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.taskDone(pool, fault) } finally { lock.unlock() }
    }
    override fun poolOfActor(addr: Int): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.poolOfActor(addr) } finally { lock.unlock() }
    }
    override fun external(delta: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.external(delta) } finally { lock.unlock() }
    }
    override fun room(addr: Int): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.room(addr) } finally { lock.unlock() }
    }
    override fun isDead(addr: Int): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.isDead(addr) } finally { lock.unlock() }
    }
    override fun queued(addr: Int): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.queued(addr) } finally { lock.unlock() }
    }
}

class Scheduler : SchedTable {
    private var actors: MutableList<RtActorRec> = mutableListOf<RtActorRec>()
    private var waiters: MutableList<RtWaiterRec> = mutableListOf<RtWaiterRec>()
    private var pools: MutableList<RtPoolRec> = mutableListOf<RtPoolRec>()
    private var idleHooks: MutableList<RtIdleHook> = mutableListOf<RtIdleHook>()
    private var nextSlot: Long = 0L
    private var active: Int = 0
    private var parkedFrames: Int = 0
    private var mainWaits: Int = 0
    private var externals: Int = 0

    override fun newPool(sink: Int): Int {
        pools.add(RtPoolRec(idle = mutableListOf<salvo.platform.runtime.Parker>(), tasks = kotlin.collections.ArrayDeque<RtTaskRun>(listOf<RtTaskRun>()), ready = kotlin.collections.ArrayDeque<Int>(listOf<Int>()), sink = sink, owed = 0))
        return pools.size - 1
    }

    override fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.RtBody): Int {
        actors.add(RtActorRec(body = slotOfPlatform(body), pool = pool, bound = bound, queue = kotlin.collections.ArrayDeque<Union3<RtDelivered, RtAnswered, RtReported>>(listOf<Union3<RtDelivered, RtAnswered, RtReported>>()), slots = kotlin.collections.ArrayDeque<Long>(listOf<Long>()), userLen = 0, gate = null, running = false, dead = false, exitReason = "", blocked = mutableListOf<salvo.platform.runtime.Parker>(), watchers = mutableListOf<RtToken>(), owed = 0, ready = false))
        return actors.size - 1
    }

    override fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union3<RtSent, RtDead, RtFull> {
        if (addr < 0 || addr >= actors.size) {
            dropDynPlatform(msg)
            return Union3.U2<RtSent, RtDead, RtFull>(RtDead())
        }
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:449:17"))
        if (a.dead) {
            dropDynPlatform(msg)
            return Union3.U2<RtSent, RtDead, RtFull>(RtDead())
        }
        if (a.userLen >= a.bound) {
            a.blocked.add(waiter)
            return Union3.U3<RtSent, RtDead, RtFull>(RtFull(msg = msg))
        }
        val e: Union3<RtDelivered, RtAnswered, RtReported> = Union3.U1<RtDelivered, RtAnswered, RtReported>(RtDelivered(msg = msg, from = (-1).toLong()))
        a.queue.addLast(e)
        a.slots.addLast((-1).toLong())
        a.userLen = a.userLen + 1
        if (markReady(a, pools, addr)) {
            val pool = a.pool
            wakePool(pools, pool)
        }
        return Union3.U1<RtSent, RtDead, RtFull>(RtSent())
    }

    override fun enqueueRemote(addr: Int, msg: salvo.platform.runtime.Dyn, from: Long): Boolean {
        if (addr < 0 || addr >= actors.size) {
            dropDynPlatform(msg)
            return false
        }
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:474:17"))
        if (a.dead) {
            dropDynPlatform(msg)
            return false
        }
        val e: Union3<RtDelivered, RtAnswered, RtReported> = Union3.U1<RtDelivered, RtAnswered, RtReported>(RtDelivered(msg = msg, from = from))
        a.queue.addLast(e)
        a.slots.addLast((-1).toLong())
        a.userLen = a.userLen + 1
        if (markReady(a, pools, addr)) {
            val pool = a.pool
            wakePool(pools, pool)
        }
        return true
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun kill(addr: Int, reason: String) {
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:491:17"))
        if (a.dead) {
            return
        }
        a.dead = true
        a.exitReason = reason
        while (true) {
            var __is1 = (a.blocked).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is1 != null)) break
            val b = __is1 as salvo.platform.runtime.Parker
            unparkPlatform(b)
        }
        val watchers: MutableList<RtToken> = mutableListOf<RtToken>()
        while (true) {
            var __is2 = (a.watchers).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is2 != null)) break
            val t = __is2 as RtToken
            watchers.add(t)
        }
        while (true) {
            var __is3 = (watchers).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is3 != null)) break
            val t = __is3 as RtToken
            deliverTo(actors, waiters, pools, t, erasePlatform(Exit(reason = reason)))
        }
        (watchers).toList().forEach({ t -> dropToken(t) })
    }

    override fun mintActor(addr: Int, gated: Boolean): RtToken {
        nextSlot = nextSlot + 1
        val slot = nextSlot
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:513:17"))
        if (gated) {
            a.gate = slot
        }
        a.owed = a.owed + 1
        return RtToken(target = Union3.U1<RtToActor, RtToWaiter, RtToTask>(RtToActor(addr = addr)), slot = slot, tracked = true)
    }

    override fun mintTask(pool: Int, body: salvo.platform.runtime.RtBody): RtToken {
        nextSlot = nextSlot + 1
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:523:17"))
        p.owed = p.owed + 1
        return RtToken(target = Union3.U3<RtToActor, RtToWaiter, RtToTask>(RtToTask(pool = pool, body = body)), slot = nextSlot, tracked = true)
    }

    override fun mintWaiter(pool: Int): RtWaiterMint {
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:529:17"))
        p.owed = p.owed + 1
        waiters.add(RtWaiterRec(pool = pool, value = slotEmptyPlatform(), filled = false, parker = null, waiting = 0, waitingActor = -1))
        val wid = waiters.size - 1
        nextSlot = nextSlot + 1
        val t = RtToken(target = Union3.U2<RtToActor, RtToWaiter, RtToTask>(RtToWaiter(wid = wid)), slot = nextSlot, tracked = true)
        return RtWaiterMint(token = t, wid = wid)
    }

    override fun deliver(t: RtToken, value: salvo.platform.runtime.Dyn) {
        deliverTo(actors, waiters, pools, t, value)
    }

    override fun watchActor(addr: Int, t: RtToken) {
        val watch = untrack(actors, waiters, pools, t)
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:546:17"))
        if (a.dead) {
            val reason = a.exitReason
            deliverTo(actors, waiters, pools, watch, erasePlatform(Exit(reason = reason)))
            return
        }
        a.watchers.add(watch)
    }

    override fun idleHook(pool: Int, t: RtToken) {
        val hook = untrack(actors, waiters, pools, t)
        idleHooks.add(RtIdleHook(pool = pool, token = hook))
        wakeAllPools(pools)
    }

    override fun nextWork(pool: Int, idle: salvo.platform.runtime.Parker): Union2<RtRunActor, RtRunTask>? {
        val w = takeWork(actors, pools, pool, -1)
        if (w == null) {
            val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:564:21"))
            p.idle.add(idle)
            if (active == parkedFrames && quiet(actors, waiters, pools, externals)) {
                if (!(idleHooks.size == 0) && active == 0) {
                    fireIdle(actors, waiters, pools, idleHooks)
                }
                wakeWaiters(waiters)
            }
            return null
        }
        active = active + 1
        return w
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun waitStep(wid: Int, pool: Int, own: Int, frame: Int, me: salvo.platform.runtime.Parker): Union6<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck> {
        val w = (waiters.getOrNull(wid) ?: throw AssertionError("salvo: value is absent at runtime:584:17"))
        if (w.waiting == 0) {
            w.waiting = frame
            w.waitingActor = own
            if (frame == 1) {
                parkedFrames = parkedFrames + 1
            } else {
                mainWaits = mainWaits + 1
            }
        }
        val got = slotTakePlatform(w.value)
        if (got != null) {
            val v = got as salvo.platform.runtime.Dyn
            w.filled = false
            w.parker = null
            if (w.waiting == 1) {
                parkedFrames = parkedFrames - 1
            } else {
                mainWaits = mainWaits - 1
            }
            w.waiting = 0
            w.waitingActor = -1
            wakePool(pools, pool)
            return Union6.U1<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>(RtGot(value = v))
        }
        val work = takeWork(actors, pools, pool, own)
        if (work is Union2.U1<*, *>) {
            val ra = work?.value as RtRunActor
            active = active + 1
            return Union6.U2<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>(ra)
        }
        if (work is Union2.U2<*, *>) {
            val rt = work?.value as RtRunTask
            active = active + 1
            return Union6.U3<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>(rt)
        }
        val q = quiet(actors, waiters, pools, externals)
        if (!(idleHooks.size == 0) && active == 0 && q) {
            fireIdle(actors, waiters, pools, idleHooks)
            return Union6.U5<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>(RtAgain())
        }
        if (active == parkedFrames && mainWaits > 0 && q) {
            return Union6.U6<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>(RtStuck(report = deadlockReport(actors, waiters, own)))
        }
        val parked = (waiters.getOrNull(wid) ?: throw AssertionError("salvo: value is absent at runtime:632:22"))
        parked.parker = me
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:634:17"))
        p.idle.add(me)
        return Union6.U4<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>(RtSleep())
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun finish(addr: Int, body: salvo.platform.runtime.RtBody, fault: String?) {
        active = active - 1
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:641:17"))
        a.running = false
        if (fault == null) {
            slotPutPlatform(a.body, body)
            val _again = markReady(a, pools, addr)
            return
        }
        dropBodyPlatform(body)
        val reason = fault
        a.dead = true
        a.exitReason = reason
        a.gate = null
        a.userLen = 0
        while (a.queue.size > 0) {
            dropEntry((a.queue.removeFirstOrNull() ?: throw AssertionError("salvo: value is absent at runtime:657:24")))
        }
        while (a.slots.size > 0) {
            val _s = a.slots.removeFirstOrNull()
        }
        while (true) {
            var __is4 = (a.blocked).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is4 != null)) break
            val b = __is4 as salvo.platform.runtime.Parker
            unparkPlatform(b)
        }
        val pool = a.pool
        val watchers: MutableList<RtToken> = mutableListOf<RtToken>()
        while (true) {
            var __is5 = (a.watchers).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is5 != null)) break
            val t = __is5 as RtToken
            watchers.add(t)
        }
        if (watchers.size == 0) {
            reportFault(actors, pools, pool, reason)
        }
        while (true) {
            var __is6 = (watchers).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is6 != null)) break
            val t = __is6 as RtToken
            deliverTo(actors, waiters, pools, t, erasePlatform(Exit(reason = reason)))
        }
        (watchers).toList().forEach({ t -> dropToken(t) })
        wakeAllPools(pools)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun taskDone(pool: Int, fault: String?) {
        active = active - 1
        if (fault != null) {
            val reason = fault as String
            reportFault(actors, pools, pool, reason)
        }
    }

    override fun poolOfActor(addr: Int): Int {
        return (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:690:21")).pool
    }

    override fun room(addr: Int): Int {
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:694:17"))
        return a.bound - a.userLen
    }

    override fun isDead(addr: Int): Boolean {
        return (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:699:21")).dead
    }

    override fun queued(addr: Int): Int {
        return (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:703:21")).userLen
    }

    override fun external(delta: Int) {
        externals = externals + delta
        if (externals < 0) {
            externals = 0
        }
        wakeAllPools(pools)
    }

    fun init() {
        pools.add(RtPoolRec(idle = mutableListOf<salvo.platform.runtime.Parker>(), tasks = kotlin.collections.ArrayDeque<RtTaskRun>(listOf<RtTaskRun>()), ready = kotlin.collections.ArrayDeque<Int>(listOf<Int>()), sink = -1, owed = 0))
    }
}

sealed class __Priv_Scheduler {
    object Init : __Priv_Scheduler()
}

fun untrack(actors: MutableList<RtActorRec>, waiters: MutableList<RtWaiterRec>, pools: MutableList<RtPoolRec>, t: RtToken): RtToken {
    val __destructured15 = t
    val target = __destructured15.target
    val slot = __destructured15.slot
    val tracked = __destructured15.tracked
    if (tracked) {
        release(actors, waiters, pools, target)
    }
    return RtToken(target = target, slot = slot, tracked = false)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun release(actors: MutableList<RtActorRec>, waiters: MutableList<RtWaiterRec>, pools: MutableList<RtPoolRec>, target: Union3<RtToActor, RtToWaiter, RtToTask>) {
    if (target is Union3.U1<*, *, *>) {
        val to = target.value as RtToActor
        val a = (actors.getOrNull(to.addr) ?: throw AssertionError("salvo: value is absent at runtime:730:17"))
        if (a.owed > 0) {
            a.owed = a.owed - 1
        }
    } else if (target is Union3.U2<*, *, *>) {
        val tw = target.value as RtToWaiter
        val pool = (waiters.getOrNull(tw.wid) ?: throw AssertionError("salvo: value is absent at runtime:735:25")).pool
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:736:17"))
        if (p.owed > 0) {
            p.owed = p.owed - 1
        }
    } else {
        val p = (pools.getOrNull((target.value as RtToTask).pool) ?: throw AssertionError("salvo: value is absent at runtime:741:17"))
        if (p.owed > 0) {
            p.owed = p.owed - 1
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun deliverTo(actors: MutableList<RtActorRec>, waiters: MutableList<RtWaiterRec>, pools: MutableList<RtPoolRec>, t: RtToken, value: salvo.platform.runtime.Dyn) {
    val __destructured16 = t
    val target = __destructured16.target
    val slot = __destructured16.slot
    val tracked = __destructured16.tracked
    if (tracked) {
        release(actors, waiters, pools, target)
    }
    if (target is Union3.U1<*, *, *>) {
        val to = target.value as RtToActor
        val a = (actors.getOrNull(to.addr) ?: throw AssertionError("salvo: value is absent at runtime:757:17"))
        if (a.dead) {
            dropDynPlatform(value)
            return
        }
        a.slots.addLast(slot)
        val e: Union3<RtDelivered, RtAnswered, RtReported> = Union3.U2<RtDelivered, RtAnswered, RtReported>(RtAnswered(slot = slot, value = value))
        a.queue.addLast(e)
        if (markReady(a, pools, to.addr)) {
            val pool = a.pool
            wakePool(pools, pool)
        }
    } else if (target is Union3.U2<*, *, *>) {
        val tw = target.value as RtToWaiter
        val w = (waiters.getOrNull(tw.wid) ?: throw AssertionError("salvo: value is absent at runtime:770:17"))
        slotPutPlatform(w.value, value)
        w.filled = true
        if (w.parker != null) {
            val p = w.parker as salvo.platform.runtime.Parker
            unparkPlatform(p)
        }
    } else {
        val __destructured17 = (target.value as RtToTask)
        val pool = __destructured17.pool
        val body = __destructured17.body
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:778:17"))
        p.tasks.addLast(RtTaskRun(body = body, value = value))
        wakePool(pools, pool)
    }
}

fun reportFault(actors: MutableList<RtActorRec>, pools: MutableList<RtPoolRec>, pool: Int, reason: String) {
    val sink = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:789:21")).sink
    if (sink >= 0) {
        val s = (actors.getOrNull(sink) ?: throw AssertionError("salvo: value is absent at runtime:791:17"))
        if (!s.dead) {
            val e: Union3<RtDelivered, RtAnswered, RtReported> = Union3.U3<RtDelivered, RtAnswered, RtReported>(RtReported(reason = reason))
            s.queue.addLast(e)
            s.slots.addLast((-1).toLong())
            if (markReady(s, pools, sink)) {
                val sinkPool = s.pool
                wakePool(pools, sinkPool)
            }
            return
        }
    }
    __moduleUse0.report("salvo: an uncaught fault on pool $pool: $reason")
}

fun quiet(actors: MutableList<RtActorRec>, waiters: MutableList<RtWaiterRec>, pools: MutableList<RtPoolRec>, externals: Int): Boolean {
    if (externals > 0) {
        return false
    }
    for (p in pools) {
        if (p.tasks.size > 0) {
            return false
        }
    }
    for (a in actors) {
        if (!a.running && !a.dead && !(deliverable(a.slots, a.gate) == null)) {
            return false
        }
    }
    for (w in waiters) {
        if (w.waiting > 0 && w.filled) {
            return false
        }
    }
    return true
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun fireIdle(actors: MutableList<RtActorRec>, waiters: MutableList<RtWaiterRec>, pools: MutableList<RtPoolRec>, hooks: MutableList<RtIdleHook>) {
    while (true) {
        var __is7 = (hooks).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        if (!(__is7 != null)) break
        val h = __is7 as RtIdleHook
        val __destructured18 = h
        val pool = __destructured18.pool
        val token = __destructured18.token
        var gates = 0
        var tokens = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:841:27")).owed
        for (a in actors) {
            if (a.pool == pool) {
                tokens = tokens + a.owed
                if (!(a.gate == null) && !a.dead) {
                    gates = gates + 1
                }
            }
        }
        deliverTo(actors, waiters, pools, token, erasePlatform(Idle(parkedGates = gates, parkedTokens = tokens)))
    }
}

fun deadlockReport(actors: MutableList<RtActorRec>, waiters: MutableList<RtWaiterRec>, own: Int): String {
    val occupied: MutableList<String> = mutableListOf<String>()
    for (w in waiters) {
        if (w.waiting > 0 && w.waitingActor >= 0) {
            occupied.add("actor ${w.waitingActor}")
        }
    }
    val gated: MutableList<String> = mutableListOf<String>()
    var i = 0
    for (a in actors) {
        if (!(a.gate == null) && !a.dead) {
            gated.add("actor $i")
        }
        i = i + 1
    }
    val who = if (own >= 0) {
        "actor $own"
    } else {
        "main"
    }
    val clauses: MutableList<String> = mutableListOf<String>()
    if (occupied.size > 0) {
        clauses.add("parked in a wait: ${occupied.joinToString(", ")}")
    }
    if (gated.size > 0) {
        clauses.add("parked gates: ${gated.joinToString(", ")}")
    }
    val detail = if (clauses.size == 0) {
        ""
    } else {
        " (${clauses.joinToString("; ")})"
    }
    return "salvo: deadlock: nothing can run while $who waits$detail"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun takeWork(actors: MutableList<RtActorRec>, pools: MutableList<RtPoolRec>, pool: Int, exclude: Int): Union2<RtRunActor, RtRunTask>? {
    val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:890:13"))
    val task = p.tasks.removeFirstOrNull()
    if (task != null) {
        val t = task as RtTaskRun
        val __destructured19 = t
        val body = __destructured19.body
        val value = __destructured19.value
        return Union2.U2<RtRunActor, RtRunTask>(RtRunTask(pool = pool, body = body, value = value))
    }
    while (true) {
        var __is8 = p.ready.removeFirstOrNull()
        if (!(__is8 != null)) break
        val i = __is8 as Int
        val a = (actors.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime:897:17"))
        a.ready = false
        if (i != exclude && !a.running && !a.dead) {
            val at = deliverable(a.slots, a.gate)
            if (at != null) {
                val k = at as Int
                val _slot = (a.slots).let { __l -> (k).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
                val e = ((a.queue).let { __l -> (k).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } } ?: throw AssertionError("salvo: value is absent at runtime:903:25"))
                a.running = true
                val body = (slotTakePlatform(a.body) ?: throw AssertionError("salvo: value is absent at runtime:905:28"))
                return workOf(a, i, e, body)
            }
        }
    }
    return null
}

fun markReady(a: RtActorRec, pools: MutableList<RtPoolRec>, addr: Int): Boolean {
    if (a.ready || a.running || a.dead) {
        return false
    }
    if (deliverable(a.slots, a.gate) == null) {
        return false
    }
    a.ready = true
    val p = (pools.getOrNull(a.pool) ?: throw AssertionError("salvo: value is absent at runtime:924:13"))
    p.ready.addLast(addr)
    return true
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun workOf(a: RtActorRec, addr: Int, e: Union3<RtDelivered, RtAnswered, RtReported>, body: salvo.platform.runtime.RtBody): Union2<RtRunActor, RtRunTask>? {
    if (e is Union3.U1<*, *, *>) {
        val d = e.value as RtDelivered
        a.userLen = a.userLen - 1
        val woken = (a.blocked).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        if (woken != null) {
            val b = woken as salvo.platform.runtime.Parker
            unparkPlatform(b)
        }
        val __destructured20 = d
        val msg = __destructured20.msg
        val from = __destructured20.from
        if (from >= 0) {
            grantedPlatform(addr, from)
        }
        return Union2.U1<RtRunActor, RtRunTask>(RtRunActor(addr = addr, pool = a.pool, kind = 0, slot = 0L, value = msg, body = body))
    }
    if (e is Union3.U3<*, *, *>) {
        val r = e.value as RtReported
        return Union2.U1<RtRunActor, RtRunTask>(RtRunActor(addr = addr, pool = a.pool, kind = 2, slot = 0L, value = erasePlatform(r.reason), body = body))
    }
    val __destructured21 = (e.value as RtAnswered)
    val slot = __destructured21.slot
    val value = __destructured21.value
    var opens = false
    if (a.gate != null) {
        val g = a.gate as Long
        opens = g == slot
    }
    if (opens) {
        a.gate = null
    }
    return Union2.U1<RtRunActor, RtRunTask>(RtRunActor(addr = addr, pool = a.pool, kind = 1, slot = slot, value = value, body = body))
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
        if ((slots.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime:972:12")) == gate) {
            return i
        }
        i = i + 1
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun wakePool(pools: MutableList<RtPoolRec>, pool: Int) {
    val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:987:13"))
    var __is9 = (p.idle).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
    if (__is9 != null) {
        val w = __is9 as salvo.platform.runtime.Parker
        unparkPlatform(w)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun wakeEvery(pools: MutableList<RtPoolRec>, pool: Int) {
    val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:995:13"))
    while (true) {
        var __is10 = (p.idle).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        if (!(__is10 != null)) break
        val w = __is10 as salvo.platform.runtime.Parker
        unparkPlatform(w)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun wakeWaiters(waiters: MutableList<RtWaiterRec>) {
    for (w in waiters) {
        if (w.waiting > 0 && (w.parker != null)) {
            val p = w.parker as salvo.platform.runtime.Parker
            unparkPlatform(p)
        }
    }
}

fun wakeAllPools(pools: MutableList<RtPoolRec>) {
    var i = 0
    while (i < pools.size) {
        wakeEvery(pools, i)
        i = i + 1
    }
}

fun newPoolOf(n: Int, sink: Int): Int {
    val id = __moduleUse1.newPool(sink)
    var i = 0
    while (i < n) {
        startThreadPlatform({  ->
    servePool(id)
})
        i = i + 1
    }
    return id
}

fun spawnBody(pool: Int, bound: Int, body: salvo.platform.runtime.RtBody): Int {
    return __moduleUse1.newActor(pool, bound, body)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun sendDyn(addr: Int, msg: salvo.platform.runtime.Dyn) {
    var r = __moduleUse1.enqueue(addr, msg, thisParkerPlatform())
    while (r is Union3.U3<*, *, *>) {
        val full = r.value as RtFull
        if (herePoolPlatform() == mainPool() && hereActorPlatform() == noFrame() && __moduleUse1.poolOfActor(addr) == mainPool()) {
            __moduleUse0.report("salvo: deadlock: the main pool's actor $addr has a full mailbox and the only thread that could drain it is the one sending: the main pool has one worker, `main` itself, and it serves work only inside a `waitfor` — send fewer messages before waiting, raise the handler's `mailbox` capacity, or place the actor on a pool of its own")
            exitProcessPlatform(1)
        }
        parkPlatform(thisParkerPlatform())
        val __destructured22 = full
        val back = __destructured22.msg
        r = __moduleUse1.enqueue(addr, back, thisParkerPlatform())
    }
    if (r is Union3.U3<*, *, *>) {
        val full = r.value as RtFull
        dropFull(full)
    }
}

fun mint(addr: Int, gated: Boolean): RtToken {
    return __moduleUse1.mintActor(addr, gated)
}

fun mintTaskOn(pool: Int, body: salvo.platform.runtime.RtBody): RtToken {
    return __moduleUse1.mintTask(pool, body)
}

fun waiter(): RtWaiterMint {
    return __moduleUse1.mintWaiter(herePoolPlatform())
}

fun answer(t: RtToken, value: salvo.platform.runtime.Dyn) {
    __moduleUse1.deliver(t, value)
}

fun watch(addr: Int, t: RtToken) {
    __moduleUse1.watchActor(addr, t)
}

fun onIdle(pool: Int, t: RtToken) {
    __moduleUse1.idleHook(pool, t)
}

fun externalBegin() {
    __moduleUse1.external(1)
}

fun externalEnd() {
    __moduleUse1.external(-1)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun awaitAnswer(wid: Int): salvo.platform.runtime.Dyn {
    val pool = herePoolPlatform()
    val own = hereActorPlatform()
    val frame = if (own == noFrame()) {
        2
    } else {
        1
    }
    while (true) {
        val step = __moduleUse1.waitStep(wid, pool, own, frame, thisParkerPlatform())
        if (step is Union6.U1<*, *, *, *, *, *>) {
            val g = step.value as RtGot
            val __destructured23 = g
            val value = __destructured23.value
            return value
        }
        if (step is Union6.U2<*, *, *, *, *, *>) {
            val ra = step.value as RtRunActor
            runActor(ra)
        } else if (step is Union6.U3<*, *, *, *, *, *>) {
            val rt = step.value as RtRunTask
            runTask(rt)
        } else if (step is Union6.U6<*, *, *, *, *, *>) {
            val s = step.value as RtStuck
            __moduleUse0.report(s.report)
            exitProcessPlatform(1)
        } else if (step is Union6.U4<*, *, *, *, *, *>) {
            parkPlatform(thisParkerPlatform())
        }
    }
    return unerasePlatform(erasePlatform(0))
}

fun runActor(ra: RtRunActor) {
    flushFramesPlatform()
    val __destructured24 = ra
    val addr = __destructured24.addr
    val pool = __destructured24.pool
    val kind = __destructured24.kind
    val slot = __destructured24.slot
    val value = __destructured24.value
    val body = __destructured24.body
    val savedPool = herePoolPlatform()
    val savedActor = hereActorPlatform()
    setHerePlatform(pool, addr)
    val ran = activatePlatform(body, kind, slot, value)
    setHerePlatform(savedPool, savedActor)
    val __destructured25 = ran
    val back = __destructured25.body
    val fault = __destructured25.fault
    __moduleUse1.finish(addr, back, fault)
}

fun runTask(rt: RtRunTask) {
    val __destructured26 = rt
    val pool = __destructured26.pool
    val body = __destructured26.body
    val value = __destructured26.value
    val savedPool = herePoolPlatform()
    val savedActor = hereActorPlatform()
    setHerePlatform(pool, taskFrame())
    val ran = activatePlatform(body, 1, 0L, value)
    setHerePlatform(savedPool, savedActor)
    val __destructured27 = ran
    val done = __destructured27.body
    val fault = __destructured27.fault
    dropBodyPlatform(done)
    __moduleUse1.taskDone(pool, fault)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun servePool(pool: Int) {
    setHerePlatform(pool, noFrame())
    while (true) {
        val w = __moduleUse1.nextWork(pool, thisParkerPlatform())
        if (w is Union2.U1<*, *>) {
            val ra = w?.value as RtRunActor
            runActor(ra)
        } else if (w is Union2.U2<*, *>) {
            val rt = w?.value as RtRunTask
            runTask(rt)
        } else {
            parkPlatform(thisParkerPlatform())
        }
    }
}

fun tokenToActor(addr: Int, slot: Long): RtToken {
    return RtToken(target = Union3.U1<RtToActor, RtToWaiter, RtToTask>(RtToActor(addr = addr)), slot = slot, tracked = false)
}

fun tokenToWaiter(wid: Int, slot: Long): RtToken {
    return RtToken(target = Union3.U2<RtToActor, RtToWaiter, RtToTask>(RtToWaiter(wid = wid)), slot = slot, tracked = false)
}

data class RtExported(
    val kind: Int,
    val id: Int,
    val slot: Long,
    val body: salvo.platform.runtime.RtBody?,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun exportToken(t: RtToken): RtExported {
    val __destructured28 = t
    val target = __destructured28.target
    val slot = __destructured28.slot
    val tracked = __destructured28.tracked
    if (target is Union3.U3<*, *, *>) {
        val tt = target.value as RtToTask
        val __destructured29 = tt
        val pool = __destructured29.pool
        val body = __destructured29.body
        return RtExported(kind = 2, id = pool, slot = slot, body = body)
    }
    var kind = 0
    var id = 0
    if (target is Union3.U1<*, *, *>) {
        val to = target.value as RtToActor
        id = to.addr
    } else if (target is Union3.U2<*, *, *>) {
        val tw = target.value as RtToWaiter
        kind = 1
        id = tw.wid
    }
    return RtExported(kind = kind, id = id, slot = slot, body = null)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun dropExported(e: RtExported) {
    val __destructured30 = e
    val kind = __destructured30.kind
    val id = __destructured30.id
    val slot = __destructured30.slot
    val body = __destructured30.body
    if (body != null) {
        val b = body as salvo.platform.runtime.RtBody
        dropBodyPlatform(b)
    }
}

fun deliverRemote(addr: Int, msg: salvo.platform.runtime.Dyn, from: Long): Boolean {
    return __moduleUse1.enqueueRemote(addr, msg, from)
}

fun killActor(addr: Int, reason: String) {
    __moduleUse1.kill(addr, reason)
}

fun mailboxRoom(addr: Int): Int {
    return __moduleUse1.room(addr)
}

fun mailboxQueued(addr: Int): Int {
    return __moduleUse1.queued(addr)
}

fun currentPool(): Int {
    return herePoolPlatform()
}

fun mailboxDead(addr: Int): Boolean {
    return __moduleUse1.isDead(addr)
}

fun identityBits(): Long {
    return freshBits()
}
