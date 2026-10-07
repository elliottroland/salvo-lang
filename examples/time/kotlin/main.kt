package salvo.main

import salvo.*

fun verdict(started: salvo.time.Tick, at: salvo.time.Tick, budget: salvo.time.Duration): String {
    val took: salvo.time.Duration = salvo.time.between__Tick_Tick(started, at)
    if ((salvo.time.cmp__Duration_Duration(took, budget) > 0)) {
        return "late by ${salvo.time.toStr(salvo.time.minus__Duration_Duration(took, budget))}"
    }
    return "in time, ${salvo.time.toStr(salvo.time.minus__Duration_Duration(budget, took))} to spare"
}

fun overdue(ticker: salvo.time.Ticker, started: salvo.time.Tick, budget: salvo.time.Duration): Boolean {
    return (salvo.time.cmp__Duration_Duration(salvo.time.elapsed(ticker, started), budget) > 0)
}

class SteppingTicker(private val step: salvo.time.Duration) : salvo.time.Ticker {
    var at: Long = 0L
    override fun tick(): salvo.time.Tick {
        at = (at + step.nanos)
        return salvo.time.Tick(nanos = at)
    }
}

interface Session {
    fun open(started: salvo.time.Tick, budget: salvo.time.Duration, out: salvo.SalvoReply)
    fun expire(started: salvo.time.Tick, budget: salvo.time.Duration, out: salvo.SalvoReply, f: salvo.time.Fired)
}

class __Mon_Session(
    private val inner: Session,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Session {
    override fun open(started: salvo.time.Tick, budget: salvo.time.Duration, out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.open(started, budget, out) } finally { lock.unlock() }
    }
    override fun expire(started: salvo.time.Tick, budget: salvo.time.Duration, out: salvo.SalvoReply, f: salvo.time.Fired) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.expire(started, budget, out, f) } finally { lock.unlock() }
    }
}

sealed class __Msg_Session {
    class Open(val started: salvo.time.Tick, val budget: salvo.time.Duration, val out: salvo.SalvoReply) : __Msg_Session()
    class Expire(val started: salvo.time.Tick, val budget: salvo.time.Duration, val out: salvo.SalvoReply, val f: salvo.time.Fired) : __Msg_Session()
}

object __Codec___Msg_Session : salvo.WireCodec<__Msg_Session> {
    override fun enc(v: __Msg_Session, out: salvo.WireOut) {
        when (v) {
            is __Msg_Session.Open -> { out.u8(0); salvo.time.__Codec_Tick.enc(v.started, out); salvo.time.__Codec_Duration.enc(v.budget, out); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_Session.Expire -> { out.u8(1); salvo.time.__Codec_Tick.enc(v.started, out); salvo.time.__Codec_Duration.enc(v.budget, out); salvo.ReplyCodec.enc(v.out, out); salvo.time.__Codec_Fired.enc(v.f, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Session = when (inp.u8()) {
            0 -> __Msg_Session.Open(salvo.time.__Codec_Tick.dec(inp), salvo.time.__Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp))
            1 -> __Msg_Session.Expire(salvo.time.__Codec_Tick.dec(inp), salvo.time.__Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp), salvo.time.__Codec_Fired.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Session: String = "7ed70285b7e6568c"

class __Stub_Session(private val addr: Int) : Session {
    override fun open(started: salvo.time.Tick, budget: salvo.time.Duration, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Session.Open(started, budget, out), __PROTO_Session, __Codec___Msg_Session)
    }
    override fun expire(started: salvo.time.Tick, budget: salvo.time.Duration, out: salvo.SalvoReply, f: salvo.time.Fired) {
        salvo.SalvoSched.sendWire(addr, __Msg_Session.Expire(started, budget, out, f), __PROTO_Session, __Codec___Msg_Session)
    }
}

class Sessions(private val __dep0: salvo.time.Timer) : Session {
    val __mailboxCapacity: Int = 8
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Sessions> = mutableMapOf()
    override fun open(started: salvo.time.Tick, budget: salvo.time.Duration, out: salvo.SalvoReply) {
        __dep0.after(salvo.time.plus__Duration_Duration(budget, salvo.time.seconds(1L)), run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_Sessions.Expire(started, budget, out); __r })
    }
    override fun expire(started: salvo.time.Tick, budget: salvo.time.Duration, out: salvo.SalvoReply, f: salvo.time.Fired) {
        salvo.SalvoSched.replyWire(out, verdict(started, f.at, budget), salvo.StrCodec)
    }
}

sealed class __Cont_Sessions {
    class Open(val started: salvo.time.Tick, val budget: salvo.time.Duration) : __Cont_Sessions()
    class Expire(val started: salvo.time.Tick, val budget: salvo.time.Duration, val out: salvo.SalvoReply) : __Cont_Sessions()
}

class __Actor_Sessions(private val handler: Sessions) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatchSession(msg as __Msg_Session)
    }

    private fun __dispatchSession(m: __Msg_Session) {
        when (m) {
            is __Msg_Session.Open -> handler.open(m.started, m.budget, m.out)
            is __Msg_Session.Expire -> handler.expire(m.started, m.budget, m.out, m.f)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Sessions.Open -> handler.open(c.started, c.budget, value as salvo.SalvoReply)
            is __Cont_Sessions.Expire -> handler.expire(c.started, c.budget, c.out, value as salvo.time.Fired)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Sessions.Open -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Sessions.Expire -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.time.__Codec_Fired) })(payload)
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

class TestTicker(private val timer: Int) : salvo.time.Ticker {
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun tick(): salvo.time.Tick {
        val fired: salvo.time.Fired = run {
            val (answer, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.time.__Codec_Fired) })
            salvo.SalvoSched.sendWire(timer, salvo.time.__Msg_Timer.After(salvo.time.nanos(0L), answer), salvo.time.__PROTO_Timer, salvo.time.__Codec___Msg_Timer)
            salvo.SalvoSched.awaitReply(__wid) as salvo.time.Fired
        }
        return fired.at
    }
}

interface Sleeper {
    fun nap(wait: salvo.time.Duration, out: salvo.SalvoReply)
    fun woke(started: salvo.time.Tick, out: salvo.SalvoReply, f: salvo.time.Fired)
}

class __Mon_Sleeper(
    private val inner: Sleeper,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Sleeper {
    override fun nap(wait: salvo.time.Duration, out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.nap(wait, out) } finally { lock.unlock() }
    }
    override fun woke(started: salvo.time.Tick, out: salvo.SalvoReply, f: salvo.time.Fired) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.woke(started, out, f) } finally { lock.unlock() }
    }
}

sealed class __Msg_Sleeper {
    class Nap(val wait: salvo.time.Duration, val out: salvo.SalvoReply) : __Msg_Sleeper()
    class Woke(val started: salvo.time.Tick, val out: salvo.SalvoReply, val f: salvo.time.Fired) : __Msg_Sleeper()
}

object __Codec___Msg_Sleeper : salvo.WireCodec<__Msg_Sleeper> {
    override fun enc(v: __Msg_Sleeper, out: salvo.WireOut) {
        when (v) {
            is __Msg_Sleeper.Nap -> { out.u8(0); salvo.time.__Codec_Duration.enc(v.wait, out); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_Sleeper.Woke -> { out.u8(1); salvo.time.__Codec_Tick.enc(v.started, out); salvo.ReplyCodec.enc(v.out, out); salvo.time.__Codec_Fired.enc(v.f, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Sleeper = when (inp.u8()) {
            0 -> __Msg_Sleeper.Nap(salvo.time.__Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp))
            1 -> __Msg_Sleeper.Woke(salvo.time.__Codec_Tick.dec(inp), salvo.ReplyCodec.dec(inp), salvo.time.__Codec_Fired.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Sleeper: String = "2385b53950dcbd9c"

class __Stub_Sleeper(private val addr: Int) : Sleeper {
    override fun nap(wait: salvo.time.Duration, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Sleeper.Nap(wait, out), __PROTO_Sleeper, __Codec___Msg_Sleeper)
    }
    override fun woke(started: salvo.time.Tick, out: salvo.SalvoReply, f: salvo.time.Fired) {
        salvo.SalvoSched.sendWire(addr, __Msg_Sleeper.Woke(started, out, f), __PROTO_Sleeper, __Codec___Msg_Sleeper)
    }
}

class Napping(private val __dep0: salvo.time.Timer, private val __dep1: salvo.time.Ticker) : Sleeper {
    val __mailboxCapacity: Int = 8
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Napping> = mutableMapOf()
    override fun nap(wait: salvo.time.Duration, out: salvo.SalvoReply) {
        __dep0.after(wait, run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_Napping.Woke(__dep1.tick(), out); __r })
    }
    override fun woke(started: salvo.time.Tick, out: salvo.SalvoReply, f: salvo.time.Fired) {
        salvo.SalvoSched.replyWire(out, "napped ${salvo.time.toStr(salvo.time.elapsed(__dep1, started))}", salvo.StrCodec)
    }
}

sealed class __Cont_Napping {
    class Nap(val wait: salvo.time.Duration) : __Cont_Napping()
    class Woke(val started: salvo.time.Tick, val out: salvo.SalvoReply) : __Cont_Napping()
}

class __Actor_Napping(private val handler: Napping) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatchSleeper(msg as __Msg_Sleeper)
    }

    private fun __dispatchSleeper(m: __Msg_Sleeper) {
        when (m) {
            is __Msg_Sleeper.Nap -> handler.nap(m.wait, m.out)
            is __Msg_Sleeper.Woke -> handler.woke(m.started, m.out, m.f)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Napping.Nap -> handler.nap(c.wait, value as salvo.SalvoReply)
            is __Cont_Napping.Woke -> handler.woke(c.started, c.out, value as salvo.time.Fired)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Napping.Nap -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Napping.Woke -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.time.__Codec_Fired) })(payload)
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

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Faults", salvo.core.actor.__PROTO_Faults), Pair("Session", salvo.main.__PROTO_Session), Pair("Sleeper", salvo.main.__PROTO_Sleeper), Pair("Timer", salvo.time.__PROTO_Timer), Pair("TimerCtl", salvo.time.__PROTO_TimerCtl), Pair("Wheel", salvo.runtime.timers.__PROTO_Wheel)))
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val budget: salvo.time.Duration = salvo.time.millis(1500L)
    salvo.core.console.println(__handle_2, "budget ${salvo.time.toStr(budget)}, doubled ${salvo.time.toStr(salvo.time.times(budget, 2L))}, in millis ${salvo.time.toMillis(budget)}")
    val stamp: salvo.time.Instant = salvo.time.epochMilli(1700000000000L)
    salvo.core.console.println(__handle_2, "stamp ${salvo.time.toEpochSecond(stamp)}s, a minute later ${salvo.time.toEpochSecond(salvo.time.plus__Instant_Duration(stamp, salvo.time.minutes(1L)))}s")
    val __use_3: salvo.time.DefaultClock = salvo.time.DefaultClock()
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: salvo.time.Clock = salvo.time.__Mon_Clock(__use_3, __lock___use_3)
    val __use_5: salvo.time.DefaultTicker = salvo.time.DefaultTicker()
    val __lock___use_5 = java.util.concurrent.locks.ReentrantLock()
    val __handle_6: salvo.time.Ticker = salvo.time.__Mon_Ticker(__use_5, __lock___use_5)
    salvo.core.console.println(__handle_2, "wall clock is set: ${(salvo.time.toEpochSecond(__handle_4.now()) > 1600000000L)}")
    val __use_7: SteppingTicker = SteppingTicker(step = salvo.time.millis(500L))
    val __lock___use_7 = java.util.concurrent.locks.ReentrantLock()
    val __handle_8: salvo.time.Ticker = salvo.time.__Mon_Ticker(__use_7, __lock___use_7)
    val started: salvo.time.Tick = __handle_8.tick()
    salvo.core.console.println(__handle_2, "overdue after one more read: ${overdue(__handle_8, started, budget)}")
    salvo.core.console.println(__handle_2, "overdue after three: ${overdue(__handle_8, started, budget)} ${overdue(__handle_8, started, budget)} ${overdue(__handle_8, started, budget)}")
    salvo.core.console.println(__handle_2, verdict(salvo.time.Tick(nanos = 0L), salvo.time.Tick(nanos = 1000000000L), budget))
    salvo.core.console.println(__handle_2, verdict(salvo.time.Tick(nanos = 0L), salvo.time.Tick(nanos = 2000000000L), budget))
    val p: Int = salvo.core.actor.pool(1)
    val __destructured_9: Pair<Int, Int> = run { val __h = salvo.time.ManualTime(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, salvo.time.__Actor_ManualTime(__h), salvo.time.__Actor_ManualTime.__DECODE); Pair(__a, __a) }
    val timer: Int = __destructured_9.first
    val ctl: Int = __destructured_9.second
    val sessions: Int = run { val __h = Sessions(salvo.time.__Stub_Timer(timer)); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sessions(__h), __Actor_Sessions.__DECODE); __a }
    val outcome: String = run {
        val (answer, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(sessions, __Msg_Session.Open(salvo.time.Tick(nanos = 0L), budget, answer), __PROTO_Session, __Codec___Msg_Session)
        run {
            val (settled, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.core.actor.__Codec_Idle) })
            salvo.core.actor.onIdle(p, settled)
            salvo.SalvoSched.awaitReply(__wid) as salvo.core.actor.Idle
        }
        salvo.SalvoSched.sendWire(ctl, salvo.time.__Msg_TimerCtl.Advance(salvo.time.millis(2500L)), salvo.time.__PROTO_TimerCtl, salvo.time.__Codec___Msg_TimerCtl)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    salvo.core.console.println(__handle_2, "session: ${outcome}")
    val sleeper: Int = run { val __h = Napping(salvo.time.__Stub_Timer(timer), salvo.time.__Mon_Ticker(TestTicker(timer = timer))); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.thread(), __h.__mailboxCapacity, __Actor_Napping(__h), __Actor_Napping.__DECODE); __a }
    val napped: String = run {
        val (answer, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(sleeper, __Msg_Sleeper.Nap(salvo.time.seconds(2L), answer), __PROTO_Sleeper, __Codec___Msg_Sleeper)
        run {
            val (settled, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.core.actor.__Codec_Idle) })
            salvo.core.actor.onIdle(p, settled)
            salvo.SalvoSched.awaitReply(__wid) as salvo.core.actor.Idle
        }
        salvo.SalvoSched.sendWire(ctl, salvo.time.__Msg_TimerCtl.Advance(salvo.time.seconds(2L)), salvo.time.__PROTO_TimerCtl, salvo.time.__Codec___Msg_TimerCtl)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    salvo.core.console.println(__handle_2, napped)
}

