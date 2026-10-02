package salvo.time

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

data class Duration(
    val nanos: Long,
)

object __Codec_Duration : salvo.WireCodec<Duration> {
    override fun enc(v: Duration, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.nanos, out)
    }
    override fun dec(inp: salvo.WireIn): Duration = Duration(salvo.LongCodec.dec(inp))
}

data class Instant(
    val nanos: Long,
)

object __Codec_Instant : salvo.WireCodec<Instant> {
    override fun enc(v: Instant, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.nanos, out)
    }
    override fun dec(inp: salvo.WireIn): Instant = Instant(salvo.LongCodec.dec(inp))
}

data class Tick(
    val nanos: Long,
)

object __Codec_Tick : salvo.WireCodec<Tick> {
    override fun enc(v: Tick, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.nanos, out)
    }
    override fun dec(inp: salvo.WireIn): Tick = Tick(salvo.LongCodec.dec(inp))
}

fun nanos(n: Long): Duration {
    return Duration(nanos = n)
}

fun micros(n: Long): Duration {
    return Duration(nanos = n * 1000L)
}

fun millis(n: Long): Duration {
    return Duration(nanos = n * 1000000L)
}

fun seconds(n: Long): Duration {
    return Duration(nanos = n * 1000000000L)
}

fun minutes(n: Long): Duration {
    return Duration(nanos = n * 60000000000L)
}

fun hours(n: Long): Duration {
    return Duration(nanos = n * 3600000000000L)
}

fun toNanos(d: Duration): Long {
    return d.nanos
}

fun toMicros(d: Duration): Long {
    return d.nanos / 1000L
}

fun toMillis(d: Duration): Long {
    return d.nanos / 1000000L
}

fun toSeconds(d: Duration): Long {
    return d.nanos / 1000000000L
}

fun plus(d1: Duration, d2: Duration): Duration {
    return Duration(nanos = d1.nanos + d2.nanos)
}

fun minus(d1: Duration, d2: Duration): Duration {
    return Duration(nanos = d1.nanos - d2.nanos)
}

fun times(d: Duration, n: Long): Duration {
    return Duration(nanos = d.nanos * n)
}

fun abs(d: Duration): Duration {
    if (d.nanos < 0) {
        return Duration(nanos = 0L - d.nanos)
    }
    return d
}

fun toStr__6(d: Duration): String {
    if (d.nanos < 0) {
        val positive = Duration(nanos = 0L - d.nanos)
        return "-${toStr__6(positive)}"
    }
    if (d.nanos == (0).toLong()) {
        return "0s"
    }
    if (d.nanos % 1000000000 == (0).toLong()) {
        return "${d.nanos / 1000000000L}s"
    }
    if (d.nanos % 1000000 == (0).toLong()) {
        return "${d.nanos / 1000000L}ms"
    }
    if (d.nanos % 1000 == (0).toLong()) {
        return "${d.nanos / 1000L}us"
    }
    return "${d.nanos}ns"
}

fun epochNano(n: Long): Instant {
    return Instant(nanos = n)
}

fun epochMilli(n: Long): Instant {
    return Instant(nanos = n * 1000000L)
}

fun epochSecond(n: Long): Instant {
    return Instant(nanos = n * 1000000000L)
}

fun toEpochNano(at: Instant): Long {
    return at.nanos
}

fun toEpochMilli(at: Instant): Long {
    return at.nanos / 1000000L
}

fun toEpochSecond(at: Instant): Long {
    return at.nanos / 1000000000L
}

fun between(start: Instant, end: Instant): Duration {
    return Duration(nanos = end.nanos - start.nanos)
}

fun between__2(start: Tick, end: Tick): Duration {
    return Duration(nanos = end.nanos - start.nanos)
}

fun plus__2(at: Instant, d: Duration): Instant {
    return Instant(nanos = at.nanos + d.nanos)
}

fun minus__2(at: Instant, d: Duration): Instant {
    return Instant(nanos = at.nanos - d.nanos)
}

fun plus__3(at: Tick, d: Duration): Tick {
    return Tick(nanos = at.nanos + d.nanos)
}

fun minus__3(at: Tick, d: Duration): Tick {
    return Tick(nanos = at.nanos - d.nanos)
}

interface Ticker {
    fun tick(): Tick
}

class __Mon_Ticker(private val inner: Ticker) : Ticker {
    override fun tick(): Tick {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.tick() }
    }
}

interface Clock {
    fun now(): Instant
    fun toInstant(at: Tick): Instant
    fun toTick(at: Instant): Tick
}

class __Mon_Clock(private val inner: Clock) : Clock {
    override fun now(): Instant {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.now() }
    }
    override fun toInstant(at: Tick): Instant {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.toInstant(at) }
    }
    override fun toTick(at: Instant): Tick {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        return synchronized(inner) { inner.toTick(at) }
    }
}

fun elapsed(ticker: Ticker, since: Tick): Duration {
    return between__2(since, ticker.tick())
}

class DefaultTicker : Ticker {

    override fun tick(): Tick {
        return Tick(nanos = salvo.SalvoTime.monoNanos())
    }
}

class DefaultClock : Clock {
    private var baseTick: Long = salvo.SalvoTime.monoNanos()
    private var baseEpoch: Long = salvo.SalvoTime.epochNanos()

    override fun now(): Instant {
        return Instant(nanos = salvo.SalvoTime.epochNanos())
    }

    override fun toInstant(at: Tick): Instant {
        return Instant(nanos = baseEpoch + (at.nanos - baseTick))
    }

    override fun toTick(at: Instant): Tick {
        return Tick(nanos = baseTick + (at.nanos - baseEpoch))
    }
}

data class Fired(
    val at: Tick,
)

object __Codec_Fired : salvo.WireCodec<Fired> {
    override fun enc(v: Fired, out: salvo.WireOut) {
        __Codec_Tick.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): Fired = Fired(__Codec_Tick.dec(inp))
}

interface Timer {
    fun after(wait: Duration, done: salvo.SalvoReply)
}

class __Stub_Timer(private val addr: Int) : Timer {
    override fun after(wait: Duration, done: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Timer.After(wait, done), __PROTO_Timer, __Codec___Msg_Timer)
    }
}

class __Mon_Timer(private val inner: Timer) : Timer {
    override fun after(wait: Duration, done: salvo.SalvoReply) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.after(wait, done) }
    }
}

sealed class __Msg_Timer {
    class After(val wait: Duration, val done: salvo.SalvoReply) : __Msg_Timer()
}

object __Codec___Msg_Timer : salvo.WireCodec<__Msg_Timer> {
    override fun enc(v: __Msg_Timer, out: salvo.WireOut) {
        when (v) {
            is __Msg_Timer.After -> { out.u8(0); __Codec_Duration.enc(v.wait, out); salvo.ReplyCodec.enc(v.done, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Timer = when (inp.u8()) {
            0 -> __Msg_Timer.After(__Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Timer`. */
const val __PROTO_Timer: String = "d0432e460a159011"

class DefaultTimer : Timer {
    internal val __mailboxCapacity: Int = 64
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_DefaultTimer> = mutableMapOf()

    override fun after(wait: Duration, done: salvo.SalvoReply) {
        afterNanos(wait.nanos, done)
    }
}

sealed class __Cont_DefaultTimer {
    class After(val wait: Duration) : __Cont_DefaultTimer()
}

class __Actor_DefaultTimer(private val handler: DefaultTimer) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Timer)
    }

    private fun __dispatch(m: __Msg_Timer) {
        when (m) {
            is __Msg_Timer.After -> handler.after(m.wait, m.done)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_DefaultTimer.After -> handler.after(c.wait, value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_DefaultTimer.After -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Timer -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Timer)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

interface TimerCtl {
    fun advance(by: Duration)
}

class __Stub_TimerCtl(private val addr: Int) : TimerCtl {
    override fun advance(by: Duration) {
        salvo.SalvoSched.sendWire(addr, __Msg_TimerCtl.Advance(by), __PROTO_TimerCtl, __Codec___Msg_TimerCtl)
    }
}

class __Mon_TimerCtl(private val inner: TimerCtl) : TimerCtl {
    override fun advance(by: Duration) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.advance(by) }
    }
}

sealed class __Msg_TimerCtl {
    class Advance(val by: Duration) : __Msg_TimerCtl()
}

object __Codec___Msg_TimerCtl : salvo.WireCodec<__Msg_TimerCtl> {
    override fun enc(v: __Msg_TimerCtl, out: salvo.WireOut) {
        when (v) {
            is __Msg_TimerCtl.Advance -> { out.u8(0); __Codec_Duration.enc(v.by, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_TimerCtl = when (inp.u8()) {
            0 -> __Msg_TimerCtl.Advance(__Codec_Duration.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `TimerCtl`. */
const val __PROTO_TimerCtl: String = "93f92d20477305ad"

class ManualTime : Timer, TimerCtl {
    private var now: Long = 0L
    private var deadlines: MutableList<Long> = mutableListOf<Long>()
    private var pending: MutableList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
    internal val __mailboxCapacity: Int = 64
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_ManualTime> = mutableMapOf()

    override fun after(wait: Duration, done: salvo.SalvoReply) {
        if (wait.nanos <= 0) {
            salvo.SalvoSched.replyWire(done, Fired(at = Tick(nanos = now)), __Codec_Fired)
        } else {
            deadlines.add(now + wait.nanos)
            pending.add(done)
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun advance(by: Duration) {
        val target = now + by.nanos
        while (true) {
            var __is1 = earliestDue(deadlines, target)
            if (!(__is1 != null)) break
            val at = __is1 as Int
            val deadline = (deadlines.getOrNull(at) ?: throw AssertionError("salvo: value is absent at time:473:33"))
            (deadlines).let { __l -> (at).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
            now = deadline
            var __is2 = (pending).let { __l -> (at).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
            if (__is2 != null) {
                val token = __is2 as salvo.SalvoReply
                salvo.SalvoSched.replyWire(token, Fired(at = Tick(nanos = deadline)), __Codec_Fired)
            }
        }
        now = target
    }
}

sealed class __Cont_ManualTime {
    class After(val wait: Duration) : __Cont_ManualTime()
    class Advance() : __Cont_ManualTime()
}

class __Actor_ManualTime(private val handler: ManualTime) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Timer -> __dispatchTimer(msg)
            is __Msg_TimerCtl -> __dispatchTimerCtl(msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    private fun __dispatchTimer(m: __Msg_Timer) {
        when (m) {
            is __Msg_Timer.After -> handler.after(m.wait, m.done)
        }
    }

    private fun __dispatchTimerCtl(m: __Msg_TimerCtl) {
        when (m) {
            is __Msg_TimerCtl.Advance -> handler.advance(m.by)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_ManualTime.After -> handler.after(c.wait, value as salvo.SalvoReply)
            is __Cont_ManualTime.Advance -> handler.advance(value as Duration)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_ManualTime.After -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_ManualTime.Advance -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Duration) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Timer -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Timer)?.let { Pair(true, it) } ?: Pair(false, null)
                __PROTO_TimerCtl -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_TimerCtl)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

fun earliestDue(deadlines: List<Long>, target: Long): Int? {
    var best = -1
    var bestAt = 0L
    var i = 0
    while (i < deadlines.size) {
        val at = (deadlines.getOrNull(i) ?: throw AssertionError("salvo: value is absent at time:497:23"))
        if (at <= target && (best < 0 || at < bestAt)) {
            best = i
            bestAt = at
        }
        i = i + 1
    }
    if (best < 0) {
        return null
    }
    return best
}

fun cmp__2(a: Duration, b: Duration): Int {
    val c__c1 = (a.nanos).compareTo(b.nanos)
    if (c__c1 != 0) {
        return c__c1
    }
    return 0
}

fun hash__4(value: Duration): Long {
    var h = 17L
    h = ((h) * 31L + ((value.nanos).hashCode().toLong()))
    return h
}

fun eq__4(a: Duration, b: Duration): Boolean {
    if (!((a.nanos) == (b.nanos))) {
        return false
    }
    return true
}

fun cmp__3(a: Instant, b: Instant): Int {
    val c__c1 = (a.nanos).compareTo(b.nanos)
    if (c__c1 != 0) {
        return c__c1
    }
    return 0
}

fun hash__5(value: Instant): Long {
    var h = 17L
    h = ((h) * 31L + ((value.nanos).hashCode().toLong()))
    return h
}

fun eq__5(a: Instant, b: Instant): Boolean {
    if (!((a.nanos) == (b.nanos))) {
        return false
    }
    return true
}

fun cmp__4(a: Tick, b: Tick): Int {
    val c__c1 = (a.nanos).compareTo(b.nanos)
    if (c__c1 != 0) {
        return c__c1
    }
    return 0
}

fun hash__6(value: Tick): Long {
    var h = 17L
    h = ((h) * 31L + ((value.nanos).hashCode().toLong()))
    return h
}

fun eq__6(a: Tick, b: Tick): Boolean {
    if (!((a.nanos) == (b.nanos))) {
        return false
    }
    return true
}
