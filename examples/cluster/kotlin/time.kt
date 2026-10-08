package salvo.time

import salvo.*

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
    return Duration(nanos = (n * 1000L))
}

fun millis(n: Long): Duration {
    return Duration(nanos = (n * 1000000L))
}

fun seconds(n: Long): Duration {
    return Duration(nanos = (n * 1000000000L))
}

fun minutes(n: Long): Duration {
    return Duration(nanos = (n * 60000000000L))
}

fun hours(n: Long): Duration {
    return Duration(nanos = (n * 3600000000000L))
}

fun toNanos(d: Duration): Long {
    return d.nanos
}

fun toMicros(d: Duration): Long {
    return (d.nanos / 1000L)
}

fun toMillis(d: Duration): Long {
    return (d.nanos / 1000000L)
}

fun toSeconds(d: Duration): Long {
    return (d.nanos / 1000000000L)
}

fun plus__Duration_Duration(d1: Duration, d2: Duration): Duration {
    return Duration(nanos = (d1.nanos + d2.nanos))
}

fun minus__Duration_Duration(d1: Duration, d2: Duration): Duration {
    return Duration(nanos = (d1.nanos - d2.nanos))
}

fun times(d: Duration, n: Long): Duration {
    return Duration(nanos = (d.nanos * n))
}

fun abs(d: Duration): Duration {
    if ((d.nanos < 0L)) {
        return Duration(nanos = (0L - d.nanos))
    }
    return d
}

fun toStr(d: Duration): String {
    if ((d.nanos < 0L)) {
        val positive: Duration = Duration(nanos = (0L - d.nanos))
        return "-${toStr(positive)}"
    }
    if (((d.nanos) == (0L))) {
        return "0s"
    }
    if ((((d.nanos % 1000000000L)) == (0L))) {
        return "${(d.nanos / 1000000000L)}s"
    }
    if ((((d.nanos % 1000000L)) == (0L))) {
        return "${(d.nanos / 1000000L)}ms"
    }
    if ((((d.nanos % 1000L)) == (0L))) {
        return "${(d.nanos / 1000L)}us"
    }
    return "${d.nanos}ns"
}

fun epochNano(n: Long): Instant {
    return Instant(nanos = n)
}

fun epochMilli(n: Long): Instant {
    return Instant(nanos = (n * 1000000L))
}

fun epochSecond(n: Long): Instant {
    return Instant(nanos = (n * 1000000000L))
}

fun toEpochNano(at: Instant): Long {
    return at.nanos
}

fun toEpochMilli(at: Instant): Long {
    return (at.nanos / 1000000L)
}

fun toEpochSecond(at: Instant): Long {
    return (at.nanos / 1000000000L)
}

fun between__Instant_Instant(start: Instant, end: Instant): Duration {
    return Duration(nanos = (end.nanos - start.nanos))
}

fun between__Tick_Tick(start: Tick, end: Tick): Duration {
    return Duration(nanos = (end.nanos - start.nanos))
}

fun plus__Instant_Duration(at: Instant, d: Duration): Instant {
    return Instant(nanos = (at.nanos + d.nanos))
}

fun minus__Instant_Duration(at: Instant, d: Duration): Instant {
    return Instant(nanos = (at.nanos - d.nanos))
}

fun plus__Tick_Duration(at: Tick, d: Duration): Tick {
    return Tick(nanos = (at.nanos + d.nanos))
}

fun minus__Tick_Duration(at: Tick, d: Duration): Tick {
    return Tick(nanos = (at.nanos - d.nanos))
}

interface Ticker {
    fun tick(): Tick
}

class __Mon_Ticker(
    private val inner: Ticker,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Ticker {
    override fun tick(): Tick {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.tick() } finally { lock.unlock() }
    }
}

interface Clock {
    fun now(): Instant
    fun toInstant(at: Tick): Instant
    fun toTick(at: Instant): Tick
}

class __Mon_Clock(
    private val inner: Clock,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Clock {
    override fun now(): Instant {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.now() } finally { lock.unlock() }
    }
    override fun toInstant(at: Tick): Instant {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.toInstant(at) } finally { lock.unlock() }
    }
    override fun toTick(at: Instant): Tick {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.toTick(at) } finally { lock.unlock() }
    }
}

fun elapsed(ticker: Ticker, since: Tick): Duration {
    return between__Tick_Tick(since, ticker.tick())
}

class DefaultTicker : Ticker {
    override fun tick(): Tick {
        return Tick(nanos = monotonicNanos())
    }
}

class DefaultClock : Clock {
    var baseTick: Long = monotonicNanos()
    var baseEpoch: Long = epochNanosPlatform()
    override fun now(): Instant {
        return Instant(nanos = epochNanosPlatform())
    }
    override fun toInstant(at: Tick): Instant {
        return Instant(nanos = (baseEpoch + (at.nanos - baseTick)))
    }
    override fun toTick(at: Instant): Tick {
        return Tick(nanos = (baseTick + (at.nanos - baseEpoch)))
    }
}

fun monotonicNanos(): Long {
    return salvo.runtime.nowNanos()
}

fun epochNanosPlatform(): Long = salvo.platform.time.epochNanos()

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

class __Mon_Timer(
    private val inner: Timer,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Timer {
    override fun after(wait: Duration, done: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.after(wait, done) } finally { lock.unlock() }
    }
}

class DefaultTimer : Timer {
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_DefaultTimer> = mutableMapOf()
    override fun after(wait: Duration, done: salvo.SalvoReply) {
        salvo.runtime.timers.afterNanos(wait.nanos, done)
    }
}

class __Actor_DefaultTimer(private val handler: DefaultTimer) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_DefaultTimer_Timer(handler, msg as __Msg_Timer)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_DefaultTimer.After -> handler.after(c.wait, value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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

class __Mon_TimerCtl(
    private val inner: TimerCtl,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : TimerCtl {
    override fun advance(by: Duration) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.advance(by) } finally { lock.unlock() }
    }
}

class ManualTime : Timer, TimerCtl {
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_ManualTime> = mutableMapOf()
    var now: Long = 0L
    var deadlines: salvo.platform.core.list.MutList<Long> = mutableListOf<Long>()
    var pending: salvo.platform.core.list.MutList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
    override fun after(wait: Duration, done: salvo.SalvoReply) {
        if ((wait.nanos <= 0L)) {
            salvo.SalvoSched.replyWire(done, Fired(at = Tick(nanos = now)), __Codec_Fired)
        } else {
            salvo.core.list.addPlatform(deadlines, (now + wait.nanos))
            salvo.core.list.addPlatform(pending, done)
        }
    }
    override fun advance(by: Duration) {
        val target: Long = (now + by.nanos)
        while (true) {
            val __subject_1: Int? = earliestDue(deadlines, target)
            if (!((__subject_1 != null))) {
                break
            }
            val at: Int = __subject_1!!
            val deadline: Long = run {
                val __nn_2: Long? = salvo.core.list.getPlatform(deadlines, at)
                when {
                    (__nn_2 == null) -> {
                        throw AssertionError(("salvo: " + ("value is absent") + " at time:477:33"))
                    }
                    else -> {
                        val __some_3: Long = __nn_2!!
                        __some_3
                    }
                }
            }
            salvo.core.list.removeAtPlatform(deadlines, at)
            now = deadline
            val __subject_4: salvo.SalvoReply? = salvo.core.list.removeAtPlatform(pending, at)
            if ((__subject_4 != null)) {
                val token: salvo.SalvoReply = __subject_4!!
                salvo.SalvoSched.replyWire(token, Fired(at = Tick(nanos = deadline)), __Codec_Fired)
            }
        }
        now = target
    }
}

class __Actor_ManualTime(private val handler: ManualTime) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Timer -> __dispatch_ManualTime_Timer(handler, msg)
            is __Msg_TimerCtl -> __dispatch_ManualTime_TimerCtl(handler, msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_ManualTime.After -> handler.after(c.wait, value as salvo.SalvoReply)
            is __Cont_ManualTime.Advance -> handler.advance(value as Duration)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
    var best: Int = (-1)
    var bestAt: Long = 0L
    var i: Int = 0
    while (true) {
        if (!((i < salvo.core.list.sizePlatform(deadlines)))) {
            break
        }
        val at: Long = run {
            val __nn_1: Long? = salvo.core.list.getPlatform(deadlines, i)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at time:501:23"))
                }
                else -> {
                    val __some_2: Long = __nn_1!!
                    __some_2
                }
            }
        }
        if (((at <= target) && ((best < 0) || (at < bestAt)))) {
            best = i
            bestAt = at
        }
        i = (i + 1)
    }
    if ((best < 0)) {
        return null
    }
    return best
}

fun cmp__Duration_Duration(a: Duration, b: Duration): Int {
    val c__c1: Int = (a.nanos).compareTo(b.nanos)
    if (!(((c__c1) == (0)))) {
        return c__c1
    }
    return 0
}

fun hash__Duration(value: Duration): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, (value.nanos).hashCode().toLong())
    return h
}

fun eq__Duration_Duration(a: Duration, b: Duration): Boolean {
    if (!(((a.nanos) == (b.nanos)))) {
        return false
    }
    return true
}

fun cmp__Instant_Instant(a: Instant, b: Instant): Int {
    val c__c1: Int = (a.nanos).compareTo(b.nanos)
    if (!(((c__c1) == (0)))) {
        return c__c1
    }
    return 0
}

fun hash__Instant(value: Instant): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, (value.nanos).hashCode().toLong())
    return h
}

fun eq__Instant_Instant(a: Instant, b: Instant): Boolean {
    if (!(((a.nanos) == (b.nanos)))) {
        return false
    }
    return true
}

fun cmp__Tick_Tick(a: Tick, b: Tick): Int {
    val c__c1: Int = (a.nanos).compareTo(b.nanos)
    if (!(((c__c1) == (0)))) {
        return c__c1
    }
    return 0
}

fun hash__Tick(value: Tick): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, (value.nanos).hashCode().toLong())
    return h
}

fun eq__Tick_Tick(a: Tick, b: Tick): Boolean {
    if (!(((a.nanos) == (b.nanos)))) {
        return false
    }
    return true
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

const val __PROTO_Timer: String = "d0432e460a159011"


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

const val __PROTO_TimerCtl: String = "93f92d20477305ad"


sealed class __Cont_DefaultTimer {
    class After(val wait: Duration) : __Cont_DefaultTimer()
}


sealed class __Cont_ManualTime {
    class After(val wait: Duration) : __Cont_ManualTime()
    class Advance() : __Cont_ManualTime()
}

class __Stub_Timer(private val addr: Int) : Timer {
    override fun after(wait: Duration, done: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Timer.After(wait, done), __PROTO_Timer, __Codec___Msg_Timer)
    }
}

class __Stub_TimerCtl(private val addr: Int) : TimerCtl {
    override fun advance(by: Duration) {
        salvo.SalvoSched.sendWire(addr, __Msg_TimerCtl.Advance(by), __PROTO_TimerCtl, __Codec___Msg_TimerCtl)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_DefaultTimer_Timer(__handler: DefaultTimer, __msg: __Msg_Timer) {
    when {
        (__msg is __Msg_Timer.After) -> {
            val wait = (__msg as __Msg_Timer.After).wait
            val done = (__msg as __Msg_Timer.After).done
            __handler.after(wait, done)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_ManualTime_Timer(__handler: ManualTime, __msg: __Msg_Timer) {
    when {
        (__msg is __Msg_Timer.After) -> {
            val wait = (__msg as __Msg_Timer.After).wait
            val done = (__msg as __Msg_Timer.After).done
            __handler.after(wait, done)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_ManualTime_TimerCtl(__handler: ManualTime, __msg: __Msg_TimerCtl) {
    when {
        (__msg is __Msg_TimerCtl.Advance) -> {
            val by = (__msg as __Msg_TimerCtl.Advance).by
            __handler.advance(by)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

