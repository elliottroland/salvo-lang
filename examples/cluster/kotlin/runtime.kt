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

fun bodyOfPlatform(f: (Int, Long, salvo.platform.runtime.Dyn) -> Unit): salvo.platform.runtime.Body {
    return salvo.platform.runtime.bodyOf(f)
}

fun activatePlatform(b: salvo.platform.runtime.Body, kind: Int, slot: Long, value: salvo.platform.runtime.Dyn): Ran {
    return salvo.platform.runtime.activate(b, kind, slot, value)
}

fun dropBodyPlatform(b: salvo.platform.runtime.Body) {
    return salvo.platform.runtime.dropBody(b)
}

fun grantedPlatform(addr: Int, pool: Int, from: Long) {
    return salvo.platform.runtime.granted(addr, pool, from)
}

fun flushFramesPlatform() {
    return salvo.platform.runtime.flushFrames()
}

fun exitProcessPlatform(code: Int): Nothing {
    return salvo.platform.runtime.exitProcess(code)
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

fun noFrame(): Int {
    return -1
}

fun taskFrame(): Int {
    return -2
}

data class Delivered(
    val msg: salvo.platform.runtime.Dyn,
    val from: Long,
)

data class Answered(
    val slot: Long,
    val value: salvo.platform.runtime.Dyn,
)

data class Reported(
    val reason: String,
)

object __Codec_Reported : salvo.WireCodec<Reported> {
    override fun enc(v: Reported, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.reason, out)
    }
    override fun dec(inp: salvo.WireIn): Reported = Reported(salvo.StrCodec.dec(inp))
}

fun dropDelivered(d: Delivered) {
    val __destructured2 = d
    val msg = __destructured2.msg
    val from = __destructured2.from
    dropDynPlatform(msg)
}

fun dropAnswered(a: Answered) {
    val __destructured3 = a
    val slot = __destructured3.slot
    val value = __destructured3.value
    dropDynPlatform(value)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun dropEntry(e: Union3<Delivered, Answered, Reported>) {
    if (e is Union3.U1<*, *, *>) {
        val d = e.value as Delivered
        dropDelivered(d)
    } else if (e is Union3.U2<*, *, *>) {
        val a = e.value as Answered
        dropAnswered(a)
    } else {
        ((e.value as Reported)).let {}
    }
}

data class ActorRec(
    var body: salvo.platform.runtime.Slot<salvo.platform.runtime.Body>,
    var pool: Int,
    var bound: Int,
    var queue: kotlin.collections.ArrayDeque<Union3<Delivered, Answered, Reported>>,
    var slots: kotlin.collections.ArrayDeque<Long>,
    var userLen: Int,
    var gate: Long?,
    var running: Boolean,
    var dead: Boolean,
    var exitReason: String,
    var blocked: MutableList<salvo.platform.runtime.Parker>,
    var watchers: MutableList<Token>,
    var owed: Int,
    var ready: Boolean,
    var proxy: Boolean,
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
    val exitReason = __destructured4.exitReason
    val blocked = __destructured4.blocked
    val watchers = __destructured4.watchers
    val owed = __destructured4.owed
    val ready = __destructured4.ready
    val proxy = __destructured4.proxy
    dropSlotPlatform(body)
    (queue).toList().forEach({ e -> dropEntry(e) })
    (watchers).toList().forEach({ t -> dropToken(t) })
}

data class WaiterRec(
    var pool: Int,
    var value: salvo.platform.runtime.Slot<salvo.platform.runtime.Dyn>,
    var filled: Boolean,
    var parker: salvo.platform.runtime.Parker?,
    var waiting: Int,
    var waitingActor: Int,
)

fun dropWaiterRec(w: WaiterRec) {
    val __destructured5 = w
    val pool = __destructured5.pool
    val value = __destructured5.value
    val filled = __destructured5.filled
    val parker = __destructured5.parker
    val waiting = __destructured5.waiting
    val waitingActor = __destructured5.waitingActor
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
    var ready: kotlin.collections.ArrayDeque<Int>,
    var sink: Int,
    var owed: Int,
    var dedicated: Boolean,
    var retired: Boolean,
)

fun dropPoolRec(p: PoolRec) {
    val __destructured7 = p
    val idle = __destructured7.idle
    val tasks = __destructured7.tasks
    val ready = __destructured7.ready
    val sink = __destructured7.sink
    val owed = __destructured7.owed
    val dedicated = __destructured7.dedicated
    val retired = __destructured7.retired
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
    val tracked: Boolean,
)

fun dropToken(t: Token) {
    answer(t, erasePlatform(0))
}

data class WaiterMint(
    val token: Token,
    val wid: Int,
)

fun dropWaiterMint(m: WaiterMint) {
    val __destructured9 = m
    val token = __destructured9.token
    val wid = __destructured9.wid
    dropToken(token)
}

data class RunActor(
    val addr: Int,
    val pool: Int,
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

class Retire

object __Codec_Retire : salvo.WireCodec<Retire> {
    override fun enc(v: Retire, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Retire = Retire()
}

fun dropRunActor(a: RunActor) {
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

data class Remote(
    val msg: salvo.platform.runtime.Dyn,
)

fun dropRemote(r: Remote) {
    val __destructured12 = r
    val msg = __destructured12.msg
    dropDynPlatform(msg)
}

fun dropFull(f: Full) {
    val __destructured13 = f
    val msg = __destructured13.msg
    dropDynPlatform(msg)
}

data class IdleHook(
    val pool: Int,
    val token: Token,
)

fun dropIdleHook(h: IdleHook) {
    val __destructured14 = h
    val pool = __destructured14.pool
    val token = __destructured14.token
    dropToken(token)
}

data class Got(
    val value: salvo.platform.runtime.Dyn,
)

class Sleep

object __Codec_Sleep : salvo.WireCodec<Sleep> {
    override fun enc(v: Sleep, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Sleep = Sleep()
}

class Again

object __Codec_Again : salvo.WireCodec<Again> {
    override fun enc(v: Again, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Again = Again()
}

data class Stuck(
    val report: String,
)

object __Codec_Stuck : salvo.WireCodec<Stuck> {
    override fun enc(v: Stuck, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.report, out)
    }
    override fun dec(inp: salvo.WireIn): Stuck = Stuck(salvo.StrCodec.dec(inp))
}

fun dropGot(g: Got) {
    val __destructured15 = g
    val value = __destructured15.value
    dropDynPlatform(value)
}

interface SchedTable {
    fun newPool(sink: Int, dedicated: Boolean): Int
    fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.Body): Int
    fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union4<Sent, Dead, Full, Remote>
    fun setProxy(addr: Int)
    fun enqueueRemote(addr: Int, msg: salvo.platform.runtime.Dyn, from: Long): Boolean
    fun kill(addr: Int, reason: String)
    fun mintActor(addr: Int, gated: Boolean): Token
    fun mintTask(pool: Int, body: salvo.platform.runtime.Body): Token
    fun mintWaiter(pool: Int): WaiterMint
    fun deliver(t: Token, value: salvo.platform.runtime.Dyn)
    fun watchActor(addr: Int, t: Token)
    fun idleHook(pool: Int, t: Token)
    fun nextWork(pool: Int, idle: salvo.platform.runtime.Parker): Union3<RunActor, RunTask, Retire>?
    fun retiredWorkers(): Int
    fun waitStep(wid: Int, pool: Int, own: Int, frame: Int, me: salvo.platform.runtime.Parker): Union6<Got, RunActor, RunTask, Sleep, Again, Stuck>
    fun finish(addr: Int, body: salvo.platform.runtime.Body, fault: String?)
    fun taskDone(pool: Int, fault: String?)
    fun poolOfActor(addr: Int): Int
    fun poolOfWaiter(wid: Int): Int
    fun external(delta: Int)
    fun room(addr: Int): Int
    fun isDead(addr: Int): Boolean
    fun queued(addr: Int): Int
}

class __Mon_SchedTable(
    private val inner: SchedTable,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : SchedTable {
    override fun newPool(sink: Int, dedicated: Boolean): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.newPool(sink, dedicated) } finally { lock.unlock() }
    }
    override fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.Body): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.newActor(pool, bound, body) } finally { lock.unlock() }
    }
    override fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union4<Sent, Dead, Full, Remote> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.enqueue(addr, msg, waiter) } finally { lock.unlock() }
    }
    override fun setProxy(addr: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.setProxy(addr) } finally { lock.unlock() }
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
    override fun mintActor(addr: Int, gated: Boolean): Token {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.mintActor(addr, gated) } finally { lock.unlock() }
    }
    override fun mintTask(pool: Int, body: salvo.platform.runtime.Body): Token {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.mintTask(pool, body) } finally { lock.unlock() }
    }
    override fun mintWaiter(pool: Int): WaiterMint {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.mintWaiter(pool) } finally { lock.unlock() }
    }
    override fun deliver(t: Token, value: salvo.platform.runtime.Dyn) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.deliver(t, value) } finally { lock.unlock() }
    }
    override fun watchActor(addr: Int, t: Token) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.watchActor(addr, t) } finally { lock.unlock() }
    }
    override fun idleHook(pool: Int, t: Token) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.idleHook(pool, t) } finally { lock.unlock() }
    }
    override fun nextWork(pool: Int, idle: salvo.platform.runtime.Parker): Union3<RunActor, RunTask, Retire>? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.nextWork(pool, idle) } finally { lock.unlock() }
    }
    override fun retiredWorkers(): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.retiredWorkers() } finally { lock.unlock() }
    }
    override fun waitStep(wid: Int, pool: Int, own: Int, frame: Int, me: salvo.platform.runtime.Parker): Union6<Got, RunActor, RunTask, Sleep, Again, Stuck> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.waitStep(wid, pool, own, frame, me) } finally { lock.unlock() }
    }
    override fun finish(addr: Int, body: salvo.platform.runtime.Body, fault: String?) {
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
    override fun poolOfWaiter(wid: Int): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.poolOfWaiter(wid) } finally { lock.unlock() }
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
    private var actors: MutableList<ActorRec> = mutableListOf<ActorRec>()
    private var waiters: MutableList<WaiterRec> = mutableListOf<WaiterRec>()
    private var pools: MutableList<PoolRec> = mutableListOf<PoolRec>()
    private var idleHooks: MutableList<IdleHook> = mutableListOf<IdleHook>()
    private var nextSlot: Long = 0L
    private var active: Int = 0
    private var parkedFrames: Int = 0
    private var mainWaits: Int = 0
    private var externals: Int = 0
    private var retired: Int = 0

    override fun newPool(sink: Int, dedicated: Boolean): Int {
        pools.add(PoolRec(idle = mutableListOf<salvo.platform.runtime.Parker>(), tasks = kotlin.collections.ArrayDeque<TaskRun>(listOf<TaskRun>()), ready = kotlin.collections.ArrayDeque<Int>(listOf<Int>()), sink = sink, owed = 0, dedicated = dedicated, retired = false))
        return pools.size - 1
    }

    override fun newActor(pool: Int, bound: Int, body: salvo.platform.runtime.Body): Int {
        actors.add(ActorRec(body = slotOfPlatform(body), pool = pool, bound = bound, queue = kotlin.collections.ArrayDeque<Union3<Delivered, Answered, Reported>>(listOf<Union3<Delivered, Answered, Reported>>()), slots = kotlin.collections.ArrayDeque<Long>(listOf<Long>()), userLen = 0, gate = null, running = false, dead = false, exitReason = "", blocked = mutableListOf<salvo.platform.runtime.Parker>(), watchers = mutableListOf<Token>(), owed = 0, ready = false, proxy = false))
        return actors.size - 1
    }

    override fun enqueue(addr: Int, msg: salvo.platform.runtime.Dyn, waiter: salvo.platform.runtime.Parker): Union4<Sent, Dead, Full, Remote> {
        if (addr < 0 || addr >= actors.size) {
            dropDynPlatform(msg)
            return Union4.U2<Sent, Dead, Full, Remote>(Dead())
        }
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:472:17"))
        if (a.dead) {
            dropDynPlatform(msg)
            return Union4.U2<Sent, Dead, Full, Remote>(Dead())
        }
        if (a.proxy) {
            return Union4.U4<Sent, Dead, Full, Remote>(Remote(msg = msg))
        }
        if (a.userLen >= a.bound) {
            a.blocked.add(waiter)
            return Union4.U3<Sent, Dead, Full, Remote>(Full(msg = msg))
        }
        val e: Union3<Delivered, Answered, Reported> = Union3.U1<Delivered, Answered, Reported>(Delivered(msg = msg, from = (-1).toLong()))
        a.queue.addLast(e)
        a.slots.addLast((-1).toLong())
        a.userLen = a.userLen + 1
        if (markReady(a, pools, addr)) {
            val pool = a.pool
            wakePool(pools, pool)
        }
        return Union4.U1<Sent, Dead, Full, Remote>(Sent())
    }

    override fun enqueueRemote(addr: Int, msg: salvo.platform.runtime.Dyn, from: Long): Boolean {
        if (addr < 0 || addr >= actors.size) {
            dropDynPlatform(msg)
            return false
        }
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:500:17"))
        if (a.dead) {
            dropDynPlatform(msg)
            return false
        }
        val e: Union3<Delivered, Answered, Reported> = Union3.U1<Delivered, Answered, Reported>(Delivered(msg = msg, from = from))
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
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:517:17"))
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
        val watchers: MutableList<Token> = mutableListOf<Token>()
        while (true) {
            var __is2 = (a.watchers).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is2 != null)) break
            val t = __is2 as Token
            watchers.add(t)
        }
        while (true) {
            var __is3 = (watchers).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is3 != null)) break
            val t = __is3 as Token
            deliverTo(actors, waiters, pools, t, erasePlatform(Exit(reason = reason)))
        }
        (watchers).toList().forEach({ t -> dropToken(t) })
    }

    override fun mintActor(addr: Int, gated: Boolean): Token {
        nextSlot = nextSlot + 1
        val slot = nextSlot
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:539:17"))
        if (gated) {
            a.gate = slot
        }
        a.owed = a.owed + 1
        return Token(target = Union3.U1<ToActor, ToWaiter, ToTask>(ToActor(addr = addr)), slot = slot, tracked = true)
    }

    override fun mintTask(pool: Int, body: salvo.platform.runtime.Body): Token {
        nextSlot = nextSlot + 1
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:549:17"))
        p.owed = p.owed + 1
        return Token(target = Union3.U3<ToActor, ToWaiter, ToTask>(ToTask(pool = pool, body = body)), slot = nextSlot, tracked = true)
    }

    override fun mintWaiter(pool: Int): WaiterMint {
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:555:17"))
        p.owed = p.owed + 1
        waiters.add(WaiterRec(pool = pool, value = slotEmptyPlatform(), filled = false, parker = null, waiting = 0, waitingActor = -1))
        val wid = waiters.size - 1
        nextSlot = nextSlot + 1
        val t = Token(target = Union3.U2<ToActor, ToWaiter, ToTask>(ToWaiter(wid = wid)), slot = nextSlot, tracked = true)
        return WaiterMint(token = t, wid = wid)
    }

    override fun deliver(t: Token, value: salvo.platform.runtime.Dyn) {
        deliverTo(actors, waiters, pools, t, value)
    }

    override fun watchActor(addr: Int, t: Token) {
        val watch = untrack(actors, waiters, pools, t)
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:572:17"))
        if (a.dead) {
            val reason = a.exitReason
            deliverTo(actors, waiters, pools, watch, erasePlatform(Exit(reason = reason)))
            return
        }
        a.watchers.add(watch)
    }

    override fun idleHook(pool: Int, t: Token) {
        val hook = untrack(actors, waiters, pools, t)
        idleHooks.add(IdleHook(pool = pool, token = hook))
        wakeAllPools(pools)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun nextWork(pool: Int, idle: salvo.platform.runtime.Parker): Union3<RunActor, RunTask, Retire>? {
        if ((pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:588:12")).retired) {
            retired = retired + 1
            return Union3.U3<RunActor, RunTask, Retire>(Retire())
        }
        val w = takeWork(actors, pools, pool, -1)
        if (w is Union2.U1<*, *>) {
            val ra = w?.value as RunActor
            active = active + 1
            return Union3.U1<RunActor, RunTask, Retire>(ra)
        }
        if (w is Union2.U2<*, *>) {
            val rt = w?.value as RunTask
            active = active + 1
            return Union3.U2<RunActor, RunTask, Retire>(rt)
        }
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:601:17"))
        p.idle.add(idle)
        if (active == parkedFrames && quiet(actors, waiters, pools, externals)) {
            if (!(idleHooks.size == 0) && active == 0) {
                fireIdle(actors, waiters, pools, idleHooks)
            }
            wakeWaiters(waiters)
        }
        return null
    }

    override fun retiredWorkers(): Int {
        return retired
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun waitStep(wid: Int, pool: Int, own: Int, frame: Int, me: salvo.platform.runtime.Parker): Union6<Got, RunActor, RunTask, Sleep, Again, Stuck> {
        val w = (waiters.getOrNull(wid) ?: throw AssertionError("salvo: value is absent at runtime:622:17"))
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
            return Union6.U1<Got, RunActor, RunTask, Sleep, Again, Stuck>(Got(value = v))
        }
        val work = takeWork(actors, pools, pool, own)
        if (work is Union2.U1<*, *>) {
            val ra = work?.value as RunActor
            active = active + 1
            return Union6.U2<Got, RunActor, RunTask, Sleep, Again, Stuck>(ra)
        }
        if (work is Union2.U2<*, *>) {
            val rt = work?.value as RunTask
            active = active + 1
            return Union6.U3<Got, RunActor, RunTask, Sleep, Again, Stuck>(rt)
        }
        val q = quiet(actors, waiters, pools, externals)
        if (!(idleHooks.size == 0) && active == 0 && q) {
            fireIdle(actors, waiters, pools, idleHooks)
            return Union6.U5<Got, RunActor, RunTask, Sleep, Again, Stuck>(Again())
        }
        if (active == parkedFrames && mainWaits > 0 && q) {
            return Union6.U6<Got, RunActor, RunTask, Sleep, Again, Stuck>(Stuck(report = deadlockReport(actors, waiters, own)))
        }
        val parked = (waiters.getOrNull(wid) ?: throw AssertionError("salvo: value is absent at runtime:670:22"))
        parked.parker = me
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:672:17"))
        p.idle.add(me)
        return Union6.U4<Got, RunActor, RunTask, Sleep, Again, Stuck>(Sleep())
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun finish(addr: Int, body: salvo.platform.runtime.Body, fault: String?) {
        active = active - 1
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:679:17"))
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
            dropEntry((a.queue.removeFirstOrNull() ?: throw AssertionError("salvo: value is absent at runtime:695:24")))
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
        val watchers: MutableList<Token> = mutableListOf<Token>()
        while (true) {
            var __is5 = (a.watchers).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is5 != null)) break
            val t = __is5 as Token
            watchers.add(t)
        }
        if (watchers.size == 0) {
            reportFault(actors, pools, pool, reason)
        }
        while (true) {
            var __is6 = (watchers).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
            if (!(__is6 != null)) break
            val t = __is6 as Token
            deliverTo(actors, waiters, pools, t, erasePlatform(Exit(reason = reason)))
        }
        (watchers).toList().forEach({ t -> dropToken(t) })
        retireIfDone(actors, pools, pool)
        wakeAllPools(pools)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun taskDone(pool: Int, fault: String?) {
        active = active - 1
        if (fault != null) {
            val reason = fault as String
            reportFault(actors, pools, pool, reason)
        }
        retireIfDone(actors, pools, pool)
    }

    override fun poolOfActor(addr: Int): Int {
        return (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:730:21")).pool
    }

    override fun setProxy(addr: Int) {
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:734:17"))
        a.proxy = true
    }

    override fun poolOfWaiter(wid: Int): Int {
        return (waiters.getOrNull(wid) ?: throw AssertionError("salvo: value is absent at runtime:739:21")).pool
    }

    override fun room(addr: Int): Int {
        val a = (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:743:17"))
        return a.bound - a.userLen
    }

    override fun isDead(addr: Int): Boolean {
        return (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:748:21")).dead
    }

    override fun queued(addr: Int): Int {
        return (actors.getOrNull(addr) ?: throw AssertionError("salvo: value is absent at runtime:752:21")).userLen
    }

    override fun external(delta: Int) {
        externals = externals + delta
        if (externals < 0) {
            externals = 0
        }
        wakeAllPools(pools)
    }

    fun init() {
        pools.add(PoolRec(idle = mutableListOf<salvo.platform.runtime.Parker>(), tasks = kotlin.collections.ArrayDeque<TaskRun>(listOf<TaskRun>()), ready = kotlin.collections.ArrayDeque<Int>(listOf<Int>()), sink = -1, owed = 0, dedicated = false, retired = false))
    }
}

sealed class __Priv_Scheduler {
    object Init : __Priv_Scheduler()
}

fun untrack(actors: MutableList<ActorRec>, waiters: MutableList<WaiterRec>, pools: MutableList<PoolRec>, t: Token): Token {
    val __destructured16 = t
    val target = __destructured16.target
    val slot = __destructured16.slot
    val tracked = __destructured16.tracked
    if (tracked) {
        release(actors, waiters, pools, target)
    }
    return Token(target = target, slot = slot, tracked = false)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun release(actors: MutableList<ActorRec>, waiters: MutableList<WaiterRec>, pools: MutableList<PoolRec>, target: Union3<ToActor, ToWaiter, ToTask>) {
    if (target is Union3.U1<*, *, *>) {
        val to = target.value as ToActor
        val a = (actors.getOrNull(to.addr) ?: throw AssertionError("salvo: value is absent at runtime:779:17"))
        if (a.owed > 0) {
            a.owed = a.owed - 1
        }
    } else if (target is Union3.U2<*, *, *>) {
        val tw = target.value as ToWaiter
        val pool = (waiters.getOrNull(tw.wid) ?: throw AssertionError("salvo: value is absent at runtime:784:25")).pool
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:785:17"))
        if (p.owed > 0) {
            p.owed = p.owed - 1
        }
    } else {
        val p = (pools.getOrNull((target.value as ToTask).pool) ?: throw AssertionError("salvo: value is absent at runtime:790:17"))
        if (p.owed > 0) {
            p.owed = p.owed - 1
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun deliverTo(actors: MutableList<ActorRec>, waiters: MutableList<WaiterRec>, pools: MutableList<PoolRec>, t: Token, value: salvo.platform.runtime.Dyn) {
    val __destructured17 = t
    val target = __destructured17.target
    val slot = __destructured17.slot
    val tracked = __destructured17.tracked
    if (tracked) {
        release(actors, waiters, pools, target)
    }
    if (target is Union3.U1<*, *, *>) {
        val to = target.value as ToActor
        val a = (actors.getOrNull(to.addr) ?: throw AssertionError("salvo: value is absent at runtime:806:17"))
        if (a.dead) {
            dropDynPlatform(value)
            return
        }
        a.slots.addLast(slot)
        val e: Union3<Delivered, Answered, Reported> = Union3.U2<Delivered, Answered, Reported>(Answered(slot = slot, value = value))
        a.queue.addLast(e)
        if (markReady(a, pools, to.addr)) {
            val pool = a.pool
            wakePool(pools, pool)
        }
    } else if (target is Union3.U2<*, *, *>) {
        val tw = target.value as ToWaiter
        val w = (waiters.getOrNull(tw.wid) ?: throw AssertionError("salvo: value is absent at runtime:819:17"))
        slotPutPlatform(w.value, value)
        w.filled = true
        if (w.parker != null) {
            val p = w.parker as salvo.platform.runtime.Parker
            unparkPlatform(p)
        }
    } else {
        val __destructured18 = (target.value as ToTask)
        val pool = __destructured18.pool
        val body = __destructured18.body
        val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:827:17"))
        p.tasks.addLast(TaskRun(body = body, value = value))
        wakePool(pools, pool)
    }
}

fun reportFault(actors: MutableList<ActorRec>, pools: MutableList<PoolRec>, pool: Int, reason: String) {
    val sink = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:838:21")).sink
    if (sink >= 0) {
        val s = (actors.getOrNull(sink) ?: throw AssertionError("salvo: value is absent at runtime:840:17"))
        if (!s.dead) {
            val e: Union3<Delivered, Answered, Reported> = Union3.U3<Delivered, Answered, Reported>(Reported(reason = reason))
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

fun quiet(actors: MutableList<ActorRec>, waiters: MutableList<WaiterRec>, pools: MutableList<PoolRec>, externals: Int): Boolean {
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
fun fireIdle(actors: MutableList<ActorRec>, waiters: MutableList<WaiterRec>, pools: MutableList<PoolRec>, hooks: MutableList<IdleHook>) {
    while (true) {
        var __is7 = (hooks).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        if (!(__is7 != null)) break
        val h = __is7 as IdleHook
        val __destructured19 = h
        val pool = __destructured19.pool
        val token = __destructured19.token
        var gates = 0
        var tokens = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:890:27")).owed
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

fun deadlockReport(actors: MutableList<ActorRec>, waiters: MutableList<WaiterRec>, own: Int): String {
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
fun takeWork(actors: MutableList<ActorRec>, pools: MutableList<PoolRec>, pool: Int, exclude: Int): Union2<RunActor, RunTask>? {
    val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:939:13"))
    val task = p.tasks.removeFirstOrNull()
    if (task != null) {
        val t = task as TaskRun
        val __destructured20 = t
        val body = __destructured20.body
        val value = __destructured20.value
        return Union2.U2<RunActor, RunTask>(RunTask(pool = pool, body = body, value = value))
    }
    while (true) {
        var __is8 = p.ready.removeFirstOrNull()
        if (!(__is8 != null)) break
        val i = __is8 as Int
        val a = (actors.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime:946:17"))
        a.ready = false
        if (i != exclude && !a.running && !a.dead) {
            val at = deliverable(a.slots, a.gate)
            if (at != null) {
                val k = at as Int
                val _slot = (a.slots).let { __l -> (k).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
                val e = ((a.queue).let { __l -> (k).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } } ?: throw AssertionError("salvo: value is absent at runtime:952:25"))
                a.running = true
                val body = (slotTakePlatform(a.body) ?: throw AssertionError("salvo: value is absent at runtime:954:28"))
                return workOf(a, i, e, body)
            }
        }
    }
    return null
}

fun markReady(a: ActorRec, pools: MutableList<PoolRec>, addr: Int): Boolean {
    if (a.ready || a.running || a.dead) {
        return false
    }
    if (deliverable(a.slots, a.gate) == null) {
        return false
    }
    a.ready = true
    val p = (pools.getOrNull(a.pool) ?: throw AssertionError("salvo: value is absent at runtime:973:13"))
    p.ready.addLast(addr)
    return true
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun workOf(a: ActorRec, addr: Int, e: Union3<Delivered, Answered, Reported>, body: salvo.platform.runtime.Body): Union2<RunActor, RunTask>? {
    if (e is Union3.U1<*, *, *>) {
        val d = e.value as Delivered
        a.userLen = a.userLen - 1
        val woken = (a.blocked).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        if (woken != null) {
            val b = woken as salvo.platform.runtime.Parker
            unparkPlatform(b)
        }
        val __destructured21 = d
        val msg = __destructured21.msg
        val from = __destructured21.from
        if (from >= 0) {
            grantedPlatform(addr, a.pool, from)
        }
        return Union2.U1<RunActor, RunTask>(RunActor(addr = addr, pool = a.pool, kind = 0, slot = 0L, value = msg, body = body))
    }
    if (e is Union3.U3<*, *, *>) {
        val r = e.value as Reported
        return Union2.U1<RunActor, RunTask>(RunActor(addr = addr, pool = a.pool, kind = 2, slot = 0L, value = erasePlatform(r.reason), body = body))
    }
    val __destructured22 = (e.value as Answered)
    val slot = __destructured22.slot
    val value = __destructured22.value
    var opens = false
    if (a.gate != null) {
        val g = a.gate as Long
        opens = g == slot
    }
    if (opens) {
        a.gate = null
    }
    return Union2.U1<RunActor, RunTask>(RunActor(addr = addr, pool = a.pool, kind = 1, slot = slot, value = value, body = body))
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
        if ((slots.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime:1021:12")) == gate) {
            return i
        }
        i = i + 1
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun wakePool(pools: MutableList<PoolRec>, pool: Int) {
    val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:1036:13"))
    var __is9 = (p.idle).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
    if (__is9 != null) {
        val w = __is9 as salvo.platform.runtime.Parker
        unparkPlatform(w)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun wakeEvery(pools: MutableList<PoolRec>, pool: Int) {
    val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:1044:13"))
    while (true) {
        var __is10 = (p.idle).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        if (!(__is10 != null)) break
        val w = __is10 as salvo.platform.runtime.Parker
        unparkPlatform(w)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun wakeWaiters(waiters: MutableList<WaiterRec>) {
    for (w in waiters) {
        if (w.waiting > 0 && (w.parker != null)) {
            val p = w.parker as salvo.platform.runtime.Parker
            unparkPlatform(p)
        }
    }
}

fun retireIfDone(actors: MutableList<ActorRec>, pools: MutableList<PoolRec>, pool: Int) {
    val p = (pools.getOrNull(pool) ?: throw AssertionError("salvo: value is absent at runtime:1066:13"))
    if (!p.dedicated || p.retired || p.tasks.size > 0 || p.owed > 0) {
        return
    }
    for (a in actors) {
        if (a.pool == pool && !a.dead) {
            return
        }
    }
    p.retired = true
    wakeEvery(pools, pool)
}

fun wakeAllPools(pools: MutableList<PoolRec>) {
    var i = 0
    while (i < pools.size) {
        wakeEvery(pools, i)
        i = i + 1
    }
}

fun newPoolOf(n: Int, sink: Int): Int {
    return startPool(n, sink, false)
}

fun newDedicatedPool(): Int {
    return startPool(1, -1, true)
}

fun retiredWorkerCount(): Int {
    return __moduleUse1.retiredWorkers()
}

fun startPool(n: Int, sink: Int, dedicated: Boolean): Int {
    val id = __moduleUse1.newPool(sink, dedicated)
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
    val back = sendOrBack(addr, msg)
    if (back != null) {
        val d = back as salvo.platform.runtime.Dyn
        dropDynPlatform(d)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun sendOrBack(addr: Int, msg: salvo.platform.runtime.Dyn): salvo.platform.runtime.Dyn? {
    var r = __moduleUse1.enqueue(addr, msg, thisParkerPlatform())
    while (r is Union4.U3<*, *, *, *>) {
        val full = r.value as Full
        if (herePoolPlatform() == mainPool() && hereActorPlatform() == noFrame() && __moduleUse1.poolOfActor(addr) == mainPool()) {
            __moduleUse0.report("salvo: deadlock: the main pool's actor $addr has a full mailbox and the only thread that could drain it is the one sending: the main pool has one worker, `main` itself, and it serves work only inside a `waitfor` — send fewer messages before waiting, raise the handler's `mailbox` capacity, or place the actor on a pool of its own")
            exitProcessPlatform(1)
        }
        parkPlatform(thisParkerPlatform())
        val __destructured23 = full
        val back = __destructured23.msg
        r = __moduleUse1.enqueue(addr, back, thisParkerPlatform())
    }
    if (r is Union4.U3<*, *, *, *>) {
        val full = r.value as Full
        dropFull(full)
        return null
    }
    if (r is Union4.U4<*, *, *, *>) {
        val remote = r.value as Remote
        val __destructured24 = remote
        val back = __destructured24.msg
        return back
    }
    return null
}

fun markProxy(addr: Int) {
    __moduleUse1.setProxy(addr)
}

fun mint(addr: Int, gated: Boolean): Token {
    return __moduleUse1.mintActor(addr, gated)
}

fun mintTaskOn(pool: Int, body: salvo.platform.runtime.Body): Token {
    return __moduleUse1.mintTask(pool, body)
}

fun waiter(): WaiterMint {
    return __moduleUse1.mintWaiter(herePoolPlatform())
}

fun answer(t: Token, value: salvo.platform.runtime.Dyn) {
    __moduleUse1.deliver(t, value)
}

fun watch(addr: Int, t: Token) {
    __moduleUse1.watchActor(addr, t)
}

fun onIdle(pool: Int, t: Token) {
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
            val g = step.value as Got
            val __destructured25 = g
            val value = __destructured25.value
            return value
        }
        if (step is Union6.U2<*, *, *, *, *, *>) {
            val ra = step.value as RunActor
            runActor(ra)
        } else if (step is Union6.U3<*, *, *, *, *, *>) {
            val rt = step.value as RunTask
            runTask(rt)
        } else if (step is Union6.U6<*, *, *, *, *, *>) {
            val s = step.value as Stuck
            __moduleUse0.report(s.report)
            exitProcessPlatform(1)
        } else if (step is Union6.U4<*, *, *, *, *, *>) {
            parkPlatform(thisParkerPlatform())
        }
    }
    return unerasePlatform(erasePlatform(0))
}

fun runActor(ra: RunActor) {
    flushFramesPlatform()
    val __destructured26 = ra
    val addr = __destructured26.addr
    val pool = __destructured26.pool
    val kind = __destructured26.kind
    val slot = __destructured26.slot
    val value = __destructured26.value
    val body = __destructured26.body
    val savedPool = herePoolPlatform()
    val savedActor = hereActorPlatform()
    setHerePlatform(pool, addr)
    val ran = activatePlatform(body, kind, slot, value)
    setHerePlatform(savedPool, savedActor)
    val __destructured27 = ran
    val back = __destructured27.body
    val fault = __destructured27.fault
    __moduleUse1.finish(addr, back, fault)
}

fun runTask(rt: RunTask) {
    val __destructured28 = rt
    val pool = __destructured28.pool
    val body = __destructured28.body
    val value = __destructured28.value
    val savedPool = herePoolPlatform()
    val savedActor = hereActorPlatform()
    setHerePlatform(pool, taskFrame())
    val ran = activatePlatform(body, 1, 0L, value)
    setHerePlatform(savedPool, savedActor)
    val __destructured29 = ran
    val done = __destructured29.body
    val fault = __destructured29.fault
    dropBodyPlatform(done)
    __moduleUse1.taskDone(pool, fault)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun servePool(pool: Int) {
    setHerePlatform(pool, noFrame())
    while (true) {
        val w = __moduleUse1.nextWork(pool, thisParkerPlatform())
        if (w is Union3.U1<*, *, *>) {
            val ra = w?.value as RunActor
            runActor(ra)
        } else if (w is Union3.U2<*, *, *>) {
            val rt = w?.value as RunTask
            runTask(rt)
        } else if (w is Union3.U3<*, *, *>) {
            return
        } else {
            parkPlatform(thisParkerPlatform())
        }
    }
}

fun tokenToActor(addr: Int, slot: Long): Token {
    return Token(target = Union3.U1<ToActor, ToWaiter, ToTask>(ToActor(addr = addr)), slot = slot, tracked = false)
}

fun tokenToWaiter(wid: Int, slot: Long): Token {
    return Token(target = Union3.U2<ToActor, ToWaiter, ToTask>(ToWaiter(wid = wid)), slot = slot, tracked = false)
}

data class Exported(
    val kind: Int,
    val id: Int,
    val slot: Long,
    val body: salvo.platform.runtime.Body?,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun exportToken(t: Token): Exported {
    val __destructured30 = t
    val target = __destructured30.target
    val slot = __destructured30.slot
    val tracked = __destructured30.tracked
    if (target is Union3.U3<*, *, *>) {
        val tt = target.value as ToTask
        val __destructured31 = tt
        val pool = __destructured31.pool
        val body = __destructured31.body
        return Exported(kind = 2, id = pool, slot = slot, body = body)
    }
    var kind = 0
    var id = 0
    if (target is Union3.U1<*, *, *>) {
        val to = target.value as ToActor
        id = to.addr
    } else if (target is Union3.U2<*, *, *>) {
        val tw = target.value as ToWaiter
        kind = 1
        id = tw.wid
    }
    return Exported(kind = kind, id = id, slot = slot, body = null)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun dropExported(e: Exported) {
    val __destructured32 = e
    val kind = __destructured32.kind
    val id = __destructured32.id
    val slot = __destructured32.slot
    val body = __destructured32.body
    if (body != null) {
        val b = body as salvo.platform.runtime.Body
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

fun actorPool(addr: Int): Int {
    return __moduleUse1.poolOfActor(addr)
}

fun waiterPool(wid: Int): Int {
    return __moduleUse1.poolOfWaiter(wid)
}

fun spawnInert(): Int {
    return spawnBody(0, 0, bodyOfPlatform({ kind, slot, value -> dropDynPlatform(value) }))
}
