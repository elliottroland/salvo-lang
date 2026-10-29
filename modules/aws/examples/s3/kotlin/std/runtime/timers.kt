package salvo.std.runtime.timers

import salvo.*

val __module_use0: Deadlines by kotlin.lazy {
    val __module_use0: Deadlines = Deadlines()
    __module_use0
}

val __module_use0_0: DeadlineTable by kotlin.lazy {
    val __lock___module_use0 = java.util.concurrent.locks.ReentrantLock()
    val __module_use0_0: DeadlineTable = __Mon_DeadlineTable(__module_use0, __lock___module_use0)
    __module_use0_0
}

val __module_use1: Wheel by kotlin.lazy {
    val __module_use1: Wheel = __Stub_Wheel(kotlin.run { val __h = Wheeling(); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.thread(), __h.__mailboxCapacity, __Actor_Wheeling(__h), __Actor_Wheeling.__DECODE); __a })
    __module_use1
}

interface DeadlineTable {
    fun register(at: Long, done: salvo.SalvoReply): Boolean
    fun wheelParker(p: salvo.platform.std.runtime.Parker)
    fun waker(): salvo.platform.std.runtime.Parker?
    fun takeDue(now: Long): salvo.platform.std.core.list.MutList<salvo.SalvoReply>
    fun nextDeadline(): Long?
    fun clear()
}

class __Mon_DeadlineTable(
    private val inner: DeadlineTable,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : DeadlineTable {
    override fun register(at: Long, done: salvo.SalvoReply): Boolean {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.register(at, done) } finally { lock.unlock() }
    }
    override fun wheelParker(p: salvo.platform.std.runtime.Parker) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { inner.wheelParker(p) } finally { lock.unlock() }
    }
    override fun waker(): salvo.platform.std.runtime.Parker? {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.waker() } finally { lock.unlock() }
    }
    override fun takeDue(now: Long): salvo.platform.std.core.list.MutList<salvo.SalvoReply> {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.takeDue(now) } finally { lock.unlock() }
    }
    override fun nextDeadline(): Long? {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { return inner.nextDeadline() } finally { lock.unlock() }
    }
    override fun clear() {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { inner.clear() } finally { lock.unlock() }
    }
}

class Deadlines : DeadlineTable {
    var ats: salvo.platform.std.core.list.MutList<Long> = mutableListOf<Long>()
    var dones: salvo.platform.std.core.list.MutList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
    var running: Boolean = false
    var parker: salvo.platform.std.runtime.Parker? = null
    var forgotten: salvo.platform.std.core.list.MutList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
    override fun register(at: Long, done: salvo.SalvoReply): Boolean {
        salvo.std.core.list.addPlatform(ats, at)
        salvo.std.core.list.addPlatform(dones, done)
        if (running) {
            return false
        }
        running = true
        return true
    }
    override fun wheelParker(p: salvo.platform.std.runtime.Parker) {
        parker = p
    }
    override fun waker(): salvo.platform.std.runtime.Parker? {
        if (running) {
            return parker
        }
        return null
    }
    override fun takeDue(now: Long): salvo.platform.std.core.list.MutList<salvo.SalvoReply> {
        val due: salvo.platform.std.core.list.MutList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
        var i: Int = 0
        while (true) {
            if (!((i < salvo.std.core.list.sizePlatform(ats)))) {
                break
            }
            if ((kotlin.run {
                val __pick_1: Long? = salvo.std.core.list.getPlatform(ats, i)
                (if ((__pick_1 != null)) {
                    val __picked_2: Long = __pick_1!!
                    __picked_2
                } else {
                    throw AssertionError("salvo: unreachable at std.runtime.timers:75:32")
                })
            } <= now)) {
                val _at: Long? = salvo.std.core.list.removeAtPlatform(ats, i)
                val __subject_4: salvo.SalvoReply? = salvo.std.core.list.removeAtPlatform(dones, i)
                if ((__subject_4 != null)) {
                    val r: salvo.SalvoReply = __subject_4!!
                    salvo.std.core.list.addPlatform(due, r)
                }
            } else {
                i = (i + 1)
            }
        }
        return due
    }
    override fun nextDeadline(): Long? {
        var earliest: Long? = null
        for (at in salvo.platform.std.core.list.each(ats)) {
            if ((if ((earliest == null)) {
                true
            } else {
                val earliest_1: Long = earliest!!
                (at < earliest_1)
            })) {
                earliest = at
            }
        }
        if ((earliest == null)) {
            running = false
        }
        return earliest
    }
    override fun clear() {
        while (true) {
            val __subject_1: salvo.SalvoReply? = salvo.std.core.list.removeFirstPlatform(dones)
            if (!((__subject_1 != null))) {
                break
            }
            val r: salvo.SalvoReply = __subject_1!!
            salvo.std.core.list.addPlatform(forgotten, r)
        }
        while (true) {
            if (!((salvo.std.core.list.removeFirstPlatform(ats) != null))) {
                break
            }
        }
        running = false
    }
}

fun answerAll(due: salvo.platform.std.core.list.MutList<salvo.SalvoReply>, now: Long) {
    salvo.std.core.list.drain(due, fun(r: salvo.SalvoReply) {
        return salvo.SalvoSched.replyWire(r, salvo.std.time.Fired(at = salvo.std.time.Tick(nanos = now)), salvo.std.time.__Codec_Fired)
    })
}

interface Wheel {
    fun run()
}

class __Mon_Wheel(
    private val inner: Wheel,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Wheel {
    override fun run() {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { inner.run() } finally { lock.unlock() }
    }
}

class Wheeling : Wheel {
    val __mailboxCapacity: Int = 2
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Wheeling> = kotlin.collections.mutableMapOf()
    override fun run() {
        __module_use0_0.wheelParker(salvo.std.runtime.thisParkerPlatform())
        while (true) {
            val now: Long = salvo.std.runtime.nowNanos()
            val due: salvo.platform.std.core.list.MutList<salvo.SalvoReply> = __module_use0_0.takeDue(now)
            salvo.std.runtime.timers.answerAll(due, now)
            val until: Long? = __module_use0_0.nextDeadline()
            if ((until == null)) {
                return
            }
            val until_1: Long = until!!
            val wait: Long = (until_1 - salvo.std.runtime.nowNanos())
            if ((wait > 0L)) {
                salvo.std.runtime.parkNanosPlatform(salvo.std.runtime.thisParkerPlatform(), wait)
            }
        }
        throw AssertionError("salvo: unreachable at std.runtime.timers:124:9")
    }
}

class __Actor_Wheeling(private val handler: Wheeling) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.std.runtime.timers.__dispatch_Wheeling_Wheel(handler, msg as __Msg_Wheel)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            else -> {}
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Wheel -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Wheel)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

fun afterNanos(delay: Long, done: salvo.SalvoReply) {
    var wait: Long = delay
    if ((wait < 0L)) {
        wait = 0L
    }
    if (salvo.std.runtime.virtualRuntime()) {
        if (__module_use0_0.register((salvo.std.runtime.nowNanos() + wait), done)) {
            salvo.std.runtime.timers.armClock()
        }
        return
    }
    if (__module_use0_0.register((salvo.std.runtime.nowNanos() + wait), done)) {
        __module_use1.run()
        return
    }
    val p: salvo.platform.std.runtime.Parker? = __module_use0_0.waker()
    if ((p != null)) {
        val p_1: salvo.platform.std.runtime.Parker = p!!
        salvo.std.runtime.unparkPlatform(p_1)
    }
}

fun armClock() {
    salvo.std.runtime.onClock(salvo.std.runtime.mintTaskOn(salvo.std.runtime.mainPool(), salvo.std.runtime.bodyOfPlatform(fun(kind: Int, slot: Long, value: salvo.platform.std.runtime.Dyn) {
        salvo.std.runtime.dropDynPlatform(value)
        salvo.std.runtime.timers.advance()
    })))
}

fun advance() {
    val until: Long? = __module_use0_0.nextDeadline()
    if ((until != null)) {
        val at: Long = until!!
        salvo.std.runtime.setVirtualNow(at)
        val now: Long = salvo.std.runtime.nowNanos()
        val due: salvo.platform.std.core.list.MutList<salvo.SalvoReply> = __module_use0_0.takeDue(now)
        salvo.std.runtime.timers.answerAll(due, now)
        if ((__module_use0_0.nextDeadline() != null)) {
            salvo.std.runtime.timers.armClock()
        }
    }
}

fun resetTimers() {
    __module_use0_0.clear()
}


sealed class __Msg_Wheel {
    class Run() : __Msg_Wheel()
}

object __Codec___Msg_Wheel : salvo.WireCodec<__Msg_Wheel> {
    override fun enc(v: __Msg_Wheel, out: salvo.WireOut) {
        when (v) {
            is __Msg_Wheel.Run -> { out.u8(0) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Wheel = when (inp.u8()) {
            0 -> __Msg_Wheel.Run()
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Wheel: String = "df3353758a650c52"


sealed class __Cont_Wheeling {
}

class __Stub_Wheel(private val addr: Int) : Wheel {
    override fun run() {
        salvo.SalvoSched.sendWire(addr, __Msg_Wheel.Run(), __PROTO_Wheel, __Codec___Msg_Wheel)
    }
}

fun __dispatch_Wheeling_Wheel(__handler: Wheeling, __msg: __Msg_Wheel) {
    when {
        (__msg is __Msg_Wheel.Run) -> {
            __handler.run()
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

