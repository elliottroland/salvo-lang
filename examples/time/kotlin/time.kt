package salvo.time

import salvo.*
import salvo.core.actor.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.fs.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

data class Duration(
    val nanos: Long,
) : Comparable<Duration> {
    override fun compareTo(other: Duration): Int {
        run { val __c = salvo.__salvoCompare(nanos, other.nanos); if (__c != 0) return __c }
        return 0
    }
}

data class Instant(
    val nanos: Long,
) : Comparable<Instant> {
    override fun compareTo(other: Instant): Int {
        run { val __c = salvo.__salvoCompare(nanos, other.nanos); if (__c != 0) return __c }
        return 0
    }
}

data class Tick(
    val nanos: Long,
) : Comparable<Tick> {
    override fun compareTo(other: Tick): Int {
        run { val __c = salvo.__salvoCompare(nanos, other.nanos); if (__c != 0) return __c }
        return 0
    }
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

fun to_nanos(d: Duration): Long {
    return d.nanos
}

fun to_micros(d: Duration): Long {
    return d.nanos / 1000L
}

fun to_millis(d: Duration): Long {
    return d.nanos / 1000000L
}

fun to_seconds(d: Duration): Long {
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

fun to_str__4(d: Duration): String {
    if (d.nanos < 0) {
        val positive = Duration(nanos = 0L - d.nanos)
        return "-${to_str__4(positive)}"
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

fun epoch_nano(n: Long): Instant {
    return Instant(nanos = n)
}

fun epoch_milli(n: Long): Instant {
    return Instant(nanos = n * 1000000L)
}

fun epoch_second(n: Long): Instant {
    return Instant(nanos = n * 1000000000L)
}

fun to_epoch_nano(at: Instant): Long {
    return at.nanos
}

fun to_epoch_milli(at: Instant): Long {
    return at.nanos / 1000000L
}

fun to_epoch_second(at: Instant): Long {
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
    override fun tick(): Tick =
        synchronized(inner) { inner.tick() }
}

interface Clock {
    fun now(): Instant
    fun to_instant(at: Tick): Instant
    fun to_tick(at: Instant): Tick
}

class __Mon_Clock(private val inner: Clock) : Clock {
    override fun now(): Instant =
        synchronized(inner) { inner.now() }
    override fun to_instant(at: Tick): Instant =
        synchronized(inner) { inner.to_instant(at) }
    override fun to_tick(at: Instant): Tick =
        synchronized(inner) { inner.to_tick(at) }
}

fun<__Fx> elapsed(__fx: __Fx, since: Tick): Duration where __Fx : __Has_Ticker {
    return between__2(since, __fx.__fx_Ticker.tick())
}

class DefaultTicker : Ticker {

    override fun tick(): Tick {
        return Tick(nanos = salvo.SalvoTime.monoNanos())
    }
}

class DefaultClock : Clock {
    private var base_tick: Long = salvo.SalvoTime.monoNanos()
    private var base_epoch: Long = salvo.SalvoTime.epochNanos()

    override fun now(): Instant {
        return Instant(nanos = salvo.SalvoTime.epochNanos())
    }

    override fun to_instant(at: Tick): Instant {
        return Instant(nanos = base_epoch + (at.nanos - base_tick))
    }

    override fun to_tick(at: Instant): Tick {
        return Tick(nanos = base_tick + (at.nanos - base_epoch))
    }
}

data class Fired(
    val at: Tick,
)

interface Timer {
    fun after(wait: Duration, done: salvo.SalvoReply)
}

class __Stub_Timer(private val addr: Int) : Timer {
    override fun after(wait: Duration, done: salvo.SalvoReply) {
        salvo.SalvoSched.send(addr, __Msg_Timer.After(wait, done))
    }
}

sealed class __Msg_Timer {
    class After(val wait: Duration, val done: salvo.SalvoReply) : __Msg_Timer()
}

class DefaultTimer : Timer {
    internal val __mailboxCapacity: Int = 64
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_DefaultTimer> = mutableMapOf()

    override fun after(wait: Duration, done: salvo.SalvoReply) {
        salvo.SalvoSched.after((wait).nanos, done) { __at -> Fired(Tick(__at)) }
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
}

interface TimerCtl {
    fun advance(by: Duration)
}

class __Stub_TimerCtl(private val addr: Int) : TimerCtl {
    override fun advance(by: Duration) {
        salvo.SalvoSched.send(addr, __Msg_TimerCtl.Advance(by))
    }
}

sealed class __Msg_TimerCtl {
    class Advance(val by: Duration) : __Msg_TimerCtl()
}

class ManualTime : Timer, TimerCtl {
    private var now: Long = 0L
    private var deadlines: MutableList<Long> = mutableListOf<Long>()
    private var pending: MutableList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
    internal val __mailboxCapacity: Int = 64
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_ManualTime> = mutableMapOf()

    override fun after(wait: Duration, done: salvo.SalvoReply) {
        if (wait.nanos <= 0) {
            done.send(Fired(at = Tick(nanos = now)))
        } else {
            deadlines.add(now + wait.nanos)
            pending.add(done)
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun advance(by: Duration) {
        val target = now + by.nanos
        while (true) {
            var __is1 = earliest_due(deadlines, target)
            if (!(__is1 != null)) break
            val at = __is1 as Int
            val deadline = deadlines.getOrNull(at)!!
            (deadlines).let { __l -> (at).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
            now = deadline
            var __is2 = (pending).let { __l -> (at).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
            if (__is2 != null) {
                val token = __is2 as salvo.SalvoReply
                token.send(Fired(at = Tick(nanos = deadline)))
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
}

fun earliest_due(deadlines: List<Long>, target: Long): Int? {
    var best = -1
    var best_at = 0L
    var i = 0
    while (i < deadlines.size) {
        val at = deadlines.getOrNull(i)!!
        if (at <= target && (best < 0 || at < best_at)) {
            best = i
            best_at = at
        }
        i = i + 1
    }
    if (best < 0) {
        return null
    }
    return best
}

fun cmp(a: Duration, b: Duration): Int {
    return salvo.__salvoCompare(a, b)
}

fun hash(value: Duration): Long {
    return value.hashCode().toLong()
}

fun eq(a: Duration, b: Duration): Boolean {
    return a == b
}

fun cmp__2(a: Instant, b: Instant): Int {
    return salvo.__salvoCompare(a, b)
}

fun hash__2(value: Instant): Long {
    return value.hashCode().toLong()
}

fun eq__2(a: Instant, b: Instant): Boolean {
    return a == b
}

fun cmp__3(a: Tick, b: Tick): Int {
    return salvo.__salvoCompare(a, b)
}

fun hash__3(value: Tick): Long {
    return value.hashCode().toLong()
}

fun eq__3(a: Tick, b: Tick): Boolean {
    return a == b
}
