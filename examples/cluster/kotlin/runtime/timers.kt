package salvo.runtime.timers

import salvo.core.actor.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.deque.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.runtime.*
import salvo.time.*

// [mod-use] The module's `use` #0, bound on first use.
private val __moduleUse0: DeadlineTable by lazy {
    val deadline_table: DeadlineTable = __Mon_DeadlineTable(Deadlines())
    deadline_table
}

// [mod-use] The module's `use` #1, bound on first use.
private val __moduleUse1: Wheel by lazy {
    val wheel: Wheel = __Stub_Wheel(run { val __h = Wheeling(); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.thread(), __h.__mailboxCapacity, __Actor_Wheeling(__h), __Actor_Wheeling.__DECODE); __a })
    wheel
}

interface DeadlineTable {
    fun register(at: Long, done: salvo.SalvoReply): Boolean
    fun wheelParker(p: salvo.platform.runtime.Parker)
    fun waker(): salvo.platform.runtime.Parker?
    fun takeDue(now: Long): salvo.platform.core.list.MutList<salvo.SalvoReply>
    fun nextDeadline(): Long?
    fun clear()
}

class __Mon_DeadlineTable(
    private val inner: DeadlineTable,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : DeadlineTable {
    override fun register(at: Long, done: salvo.SalvoReply): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.register(at, done) } finally { lock.unlock() }
    }
    override fun wheelParker(p: salvo.platform.runtime.Parker) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.wheelParker(p) } finally { lock.unlock() }
    }
    override fun waker(): salvo.platform.runtime.Parker? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.waker() } finally { lock.unlock() }
    }
    override fun takeDue(now: Long): salvo.platform.core.list.MutList<salvo.SalvoReply> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.takeDue(now) } finally { lock.unlock() }
    }
    override fun nextDeadline(): Long? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.nextDeadline() } finally { lock.unlock() }
    }
    override fun clear() {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.clear() } finally { lock.unlock() }
    }
}

class Deadlines : DeadlineTable {
    private var ats: salvo.platform.core.list.MutList<Long> = mutableListOf<Long>()
    private var dones: salvo.platform.core.list.MutList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
    private var running: Boolean = false
    private var parker: salvo.platform.runtime.Parker? = null
    private var forgotten: salvo.platform.core.list.MutList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()

    override fun register(at: Long, done: salvo.SalvoReply): Boolean {
        addPlatform(ats, at)
        addPlatform(dones, done)
        if (running) {
            return false
        }
        running = true
        return true
    }

    override fun wheelParker(p: salvo.platform.runtime.Parker) {
        parker = p
    }

    override fun waker(): salvo.platform.runtime.Parker? {
        if (running) {
            return parker
        }
        return null
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun takeDue(now: Long): salvo.platform.core.list.MutList<salvo.SalvoReply> {
        val due: salvo.platform.core.list.MutList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
        var i = 0
        while (i < sizePlatform(ats)) {
            if ((getPlatform(ats, i) ?: throw AssertionError("salvo: value is absent at runtime.timers:72:16")) <= now) {
                val _at = removeAtPlatform(ats, i)
                var __is1 = removeAtPlatform(dones, i)
                if (__is1 != null) {
                    val r = __is1 as salvo.SalvoReply
                    addPlatform(due, r)
                }
            } else {
                i = i + 1
            }
        }
        return due
    }

    override fun nextDeadline(): Long? {
        var earliest: Long? = null
        for (at in ats) {
            if ((earliest == null) || at < earliest) {
                earliest = at
            }
        }
        if (earliest == null) {
            running = false
        }
        return earliest
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun clear() {
        while (true) {
            var __is2 = removeFirstPlatform(dones)
            if (!(__is2 != null)) break
            val r = __is2 as salvo.SalvoReply
            addPlatform(forgotten, r)
        }
        while (removeFirstPlatform(ats) != null) {
        }
        running = false
    }
}

fun answerAll(due: salvo.platform.core.list.MutList<salvo.SalvoReply>, now: Long) {
    drain__2(due, { r -> salvo.SalvoSched.replyWire(r, Fired(at = Tick(nanos = now)), __Codec_Fired) })
}

interface Wheel {
    fun run()
}

class __Stub_Wheel(private val addr: Int) : Wheel {
    override fun run() {
        salvo.SalvoSched.sendWire(addr, __Msg_Wheel.Run(), __PROTO_Wheel, __Codec___Msg_Wheel)
    }
}

class __Mon_Wheel(
    private val inner: Wheel,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Wheel {
    override fun run() {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.run() } finally { lock.unlock() }
    }
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

/** [protocol-hash] The canonical hash of `Wheel`. */
const val __PROTO_Wheel: String = "df3353758a650c52"

class Wheeling : Wheel {
    internal val __mailboxCapacity: Int = 2
    internal var __addr: Int? = null

    override fun run() {
        __moduleUse0.wheelParker(thisParkerPlatform())
        while (true) {
            val now = nowNanos()
            val due = __moduleUse0.takeDue(now)
            answerAll(due, now)
            val until = __moduleUse0.nextDeadline()
            if (until == null) {
                return
            }
            val wait = until - nowNanos()
            if (wait > 0) {
                parkNanosPlatform(thisParkerPlatform(), wait)
            }
        }
    }
}

class __Actor_Wheeling(private val handler: Wheeling) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Wheel)
    }

    private fun __dispatch(m: __Msg_Wheel) {
        when (m) {
            is __Msg_Wheel.Run -> handler.run()
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        error("this protocol has no continuation targets")
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Wheel -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Wheel)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

fun afterNanos(delay: Long, done: salvo.SalvoReply) {
    var wait = delay
    if (wait < 0) {
        wait = 0L
    }
    if (virtualRuntime()) {
        if (__moduleUse0.register(nowNanos() + wait, done)) {
            armClock()
        }
        return
    }
    if (__moduleUse0.register(nowNanos() + wait, done)) {
        __moduleUse1.run()
        return
    }
    val p = __moduleUse0.waker()
    if (p != null) {
        unparkPlatform(p)
    }
}

fun armClock() {
    onClock(mintTaskOn(mainPool(), bodyOfPlatform({ kind, slot, value ->
    dropDynPlatform(value)
    advance()
})))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun advance() {
    val until = __moduleUse0.nextDeadline()
    if (until != null) {
        val at = until as Long
        setVirtualNow(at)
        val now = nowNanos()
        val due = __moduleUse0.takeDue(now)
        answerAll(due, now)
        if (__moduleUse0.nextDeadline() != null) {
            armClock()
        }
    }
}

fun resetTimers() {
    __moduleUse0.clear()
}
