package salvo.main

import salvo.core.actor.*
import salvo.core.console.*
import salvo.core.list.*
import salvo.core.string.*
import salvo.time.*

fun verdict(started: Tick, at: Tick, budget: Duration): String {
    val took = between__2(started, at)
    if (cmp__2(took, budget) > 0) {
        return "late by ${toStr__7(minus(took, budget))}"
    }
    return "in time, ${toStr__7(minus(budget, took))} to spare"
}

fun overdue(ticker: Ticker, started: Tick, budget: Duration): Boolean {
    return cmp__2(elapsed(ticker, started), budget) > 0
}

class SteppingTicker(private val step: Duration) : Ticker {
    private var at: Long = 0L

    override fun tick(): Tick {
        at = at + step.nanos
        return Tick(nanos = at)
    }
}

interface Session {
    fun open(started: Tick, budget: Duration, out: salvo.SalvoReply)
    fun expire(started: Tick, budget: Duration, out: salvo.SalvoReply, f: Fired)
}

class __Stub_Session(private val addr: Int) : Session {
    override fun open(started: Tick, budget: Duration, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Session.Open(started, budget, out), __PROTO_Session, __Codec___Msg_Session)
    }
    override fun expire(started: Tick, budget: Duration, out: salvo.SalvoReply, f: Fired) {
        salvo.SalvoSched.sendWire(addr, __Msg_Session.Expire(started, budget, out, f), __PROTO_Session, __Codec___Msg_Session)
    }
}

class __Mon_Session(
    private val inner: Session,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Session {
    override fun open(started: Tick, budget: Duration, out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.open(started, budget, out) } finally { lock.unlock() }
    }
    override fun expire(started: Tick, budget: Duration, out: salvo.SalvoReply, f: Fired) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.expire(started, budget, out, f) } finally { lock.unlock() }
    }
}

sealed class __Msg_Session {
    class Open(val started: Tick, val budget: Duration, val out: salvo.SalvoReply) : __Msg_Session()
    class Expire(val started: Tick, val budget: Duration, val out: salvo.SalvoReply, val f: Fired) : __Msg_Session()
}

object __Codec___Msg_Session : salvo.WireCodec<__Msg_Session> {
    override fun enc(v: __Msg_Session, out: salvo.WireOut) {
        when (v) {
            is __Msg_Session.Open -> { out.u8(0); __Codec_Tick.enc(v.started, out); __Codec_Duration.enc(v.budget, out); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_Session.Expire -> { out.u8(1); __Codec_Tick.enc(v.started, out); __Codec_Duration.enc(v.budget, out); salvo.ReplyCodec.enc(v.out, out); __Codec_Fired.enc(v.f, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Session = when (inp.u8()) {
            0 -> __Msg_Session.Open(__Codec_Tick.dec(inp), __Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp))
            1 -> __Msg_Session.Expire(__Codec_Tick.dec(inp), __Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp), __Codec_Fired.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Session`. */
const val __PROTO_Session: String = "7ed70285b7e6568c"

class Sessions(private val __dep_Timer: Timer) : Session {
    internal val __mailboxCapacity: Int = 8
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Sessions> = mutableMapOf()

    override fun open(started: Tick, budget: Duration, out: salvo.SalvoReply) {
        __dep_Timer.after(plus(budget, seconds(1L)), run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_Sessions.Expire(started, budget, out); __r })
    }

    override fun expire(started: Tick, budget: Duration, out: salvo.SalvoReply, f: Fired) {
        salvo.SalvoSched.replyWire(out, verdict(started, f.at, budget), salvo.StrCodec)
    }
}

sealed class __Cont_Sessions {
    class Open(val started: Tick, val budget: Duration) : __Cont_Sessions()
    class Expire(val started: Tick, val budget: Duration, val out: salvo.SalvoReply) : __Cont_Sessions()
}

class __Actor_Sessions(private val handler: Sessions) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Session)
    }

    private fun __dispatch(m: __Msg_Session) {
        when (m) {
            is __Msg_Session.Open -> handler.open(m.started, m.budget, m.out)
            is __Msg_Session.Expire -> handler.expire(m.started, m.budget, m.out, m.f)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Sessions.Open -> handler.open(c.started, c.budget, value as salvo.SalvoReply)
            is __Cont_Sessions.Expire -> handler.expire(c.started, c.budget, c.out, value as Fired)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Sessions.Open -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Sessions.Expire -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Fired) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Session -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Session)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class TestTicker(private val timer: Int) : Ticker {

    override fun tick(): Tick {
        val fired = run {
            val (answer, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Fired) })
            salvo.SalvoSched.sendWire(timer, __Msg_Timer.After(nanos(0L), answer), __PROTO_Timer, __Codec___Msg_Timer)
            salvo.SalvoSched.awaitReply(__wid) as Fired
        }
        return fired.at
    }
}

interface Sleeper {
    fun nap(wait: Duration, out: salvo.SalvoReply)
    fun woke(started: Tick, out: salvo.SalvoReply, f: Fired)
}

class __Stub_Sleeper(private val addr: Int) : Sleeper {
    override fun nap(wait: Duration, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Sleeper.Nap(wait, out), __PROTO_Sleeper, __Codec___Msg_Sleeper)
    }
    override fun woke(started: Tick, out: salvo.SalvoReply, f: Fired) {
        salvo.SalvoSched.sendWire(addr, __Msg_Sleeper.Woke(started, out, f), __PROTO_Sleeper, __Codec___Msg_Sleeper)
    }
}

class __Mon_Sleeper(
    private val inner: Sleeper,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Sleeper {
    override fun nap(wait: Duration, out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.nap(wait, out) } finally { lock.unlock() }
    }
    override fun woke(started: Tick, out: salvo.SalvoReply, f: Fired) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.woke(started, out, f) } finally { lock.unlock() }
    }
}

sealed class __Msg_Sleeper {
    class Nap(val wait: Duration, val out: salvo.SalvoReply) : __Msg_Sleeper()
    class Woke(val started: Tick, val out: salvo.SalvoReply, val f: Fired) : __Msg_Sleeper()
}

object __Codec___Msg_Sleeper : salvo.WireCodec<__Msg_Sleeper> {
    override fun enc(v: __Msg_Sleeper, out: salvo.WireOut) {
        when (v) {
            is __Msg_Sleeper.Nap -> { out.u8(0); __Codec_Duration.enc(v.wait, out); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_Sleeper.Woke -> { out.u8(1); __Codec_Tick.enc(v.started, out); salvo.ReplyCodec.enc(v.out, out); __Codec_Fired.enc(v.f, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Sleeper = when (inp.u8()) {
            0 -> __Msg_Sleeper.Nap(__Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp))
            1 -> __Msg_Sleeper.Woke(__Codec_Tick.dec(inp), salvo.ReplyCodec.dec(inp), __Codec_Fired.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Sleeper`. */
const val __PROTO_Sleeper: String = "2385b53950dcbd9c"

class Napping(private val __dep_Timer: Timer, private val __dep_Ticker: Ticker) : Sleeper {
    internal val __mailboxCapacity: Int = 8
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Napping> = mutableMapOf()

    override fun nap(wait: Duration, out: salvo.SalvoReply) {
        __dep_Timer.after(wait, run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_Napping.Woke(__dep_Ticker.tick(), out); __r })
    }

    override fun woke(started: Tick, out: salvo.SalvoReply, f: Fired) {
        salvo.SalvoSched.replyWire(out, "napped ${toStr__7(elapsed(__dep_Ticker, started))}", salvo.StrCodec)
    }
}

sealed class __Cont_Napping {
    class Nap(val wait: Duration) : __Cont_Napping()
    class Woke(val started: Tick, val out: salvo.SalvoReply) : __Cont_Napping()
}

class __Actor_Napping(private val handler: Napping) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Sleeper)
    }

    private fun __dispatch(m: __Msg_Sleeper) {
        when (m) {
            is __Msg_Sleeper.Nap -> handler.nap(m.wait, m.out)
            is __Msg_Sleeper.Woke -> handler.woke(m.started, m.out, m.f)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Napping.Nap -> handler.nap(c.wait, value as salvo.SalvoReply)
            is __Cont_Napping.Woke -> handler.woke(c.started, c.out, value as Fired)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Napping.Nap -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Napping.Woke -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Fired) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Sleeper -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Sleeper)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Faults", salvo.core.actor.__PROTO_Faults), Pair("Session", salvo.main.__PROTO_Session), Pair("Sleeper", salvo.main.__PROTO_Sleeper), Pair("Timer", salvo.time.__PROTO_Timer), Pair("TimerCtl", salvo.time.__PROTO_TimerCtl), Pair("Wheel", salvo.runtime.timers.__PROTO_Wheel)))
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val budget = millis(1500L)
    println(console, "budget ${toStr__7(budget)}, doubled ${toStr__7(times(budget, 2L))}, in millis ${toMillis(budget)}")
    val stamp = epochMilli(1700000000000L)
    println(console, "stamp ${toEpochSecond(stamp)}s, a minute later ${toEpochSecond(plus__2(stamp, minutes(1L)))}s")
    val clock: Clock = __Mon_Clock(DefaultClock())
    val ticker: Ticker = DefaultTicker()
    println(console, "wall clock is set: ${toEpochSecond(clock.now()) > 1600000000}")
    val ticker2: Ticker = __Mon_Ticker(SteppingTicker(millis(500L)))
    val started = ticker2.tick()
    println(console, "overdue after one more read: ${overdue(ticker2, started, budget)}")
    println(console, "overdue after three: ${overdue(ticker2, started, budget)} ${overdue(ticker2, started, budget)} ${overdue(ticker2, started, budget)}")
    println(console, verdict(Tick(nanos = 0L), Tick(nanos = 1000000000L), budget))
    println(console, verdict(Tick(nanos = 0L), Tick(nanos = 2000000000L), budget))
    val p = salvo.SalvoSched.pool(1)
    val (timer, ctl) = run { val __h = ManualTime(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_ManualTime(__h), __Actor_ManualTime.__DECODE); Pair(__a, __a) }
    val sessions = run { val __h = Sessions(__Stub_Timer(timer)); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sessions(__h), __Actor_Sessions.__DECODE); __a }
    val outcome = run {
        val (answer, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(sessions, __Msg_Session.Open(Tick(nanos = 0L), budget, answer), __PROTO_Session, __Codec___Msg_Session)
        run {
            val (settled, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Idle) })
            salvo.SalvoSched.onIdle(p, settled, { __gates, __tokens -> Idle(__gates, __tokens) })
            salvo.SalvoSched.awaitReply(__wid) as Idle
        }
        salvo.SalvoSched.sendWire(ctl, __Msg_TimerCtl.Advance(millis(2500L)), __PROTO_TimerCtl, __Codec___Msg_TimerCtl)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    println(console, "session: $outcome")
    val sleeper = run { val __h = Napping(__Stub_Timer(timer), TestTicker(timer)); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.thread(), __h.__mailboxCapacity, __Actor_Napping(__h), __Actor_Napping.__DECODE); __a }
    val napped = run {
        val (answer, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(sleeper, __Msg_Sleeper.Nap(seconds(2L), answer), __PROTO_Sleeper, __Codec___Msg_Sleeper)
        run {
            val (settled, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Idle) })
            salvo.SalvoSched.onIdle(p, settled, { __gates, __tokens -> Idle(__gates, __tokens) })
            salvo.SalvoSched.awaitReply(__wid) as Idle
        }
        salvo.SalvoSched.sendWire(ctl, __Msg_TimerCtl.Advance(seconds(2L)), __PROTO_TimerCtl, __Codec___Msg_TimerCtl)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    println(console, napped)
}
