package salvo.main

import salvo.*

fun verdict(started: salvo.std.time.Tick, at: salvo.std.time.Tick, budget: salvo.std.time.Duration): String {
    val took: salvo.std.time.Duration = salvo.std.time.between__Tick_Tick(started, at)
    if ((salvo.std.time.cmp__Duration_Duration(took, budget) > 0)) {
        return "late by ${salvo.std.time.toStr(salvo.std.time.minus__Duration_Duration(took, budget))}"
    }
    return "in time, ${salvo.std.time.toStr(salvo.std.time.minus__Duration_Duration(budget, took))} to spare"
}

fun overdue(ticker: salvo.std.time.Ticker, started: salvo.std.time.Tick, budget: salvo.std.time.Duration): Boolean {
    return (salvo.std.time.cmp__Duration_Duration(salvo.std.time.elapsed(ticker, started), budget) > 0)
}

class SteppingTicker(private val step: salvo.std.time.Duration) : salvo.std.time.Ticker {
    var at: Long = 0L
    override fun tick(): salvo.std.time.Tick {
        at = (at + step.nanos)
        return salvo.std.time.Tick(nanos = at)
    }
}

interface Session {
    fun open(started: salvo.std.time.Tick, budget: salvo.std.time.Duration, out: salvo.SalvoReply)
    fun expire(started: salvo.std.time.Tick, budget: salvo.std.time.Duration, out: salvo.SalvoReply, f: salvo.std.time.Fired)
}

class __Mon_Session(
    private val inner: Session,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Session {
    override fun open(started: salvo.std.time.Tick, budget: salvo.std.time.Duration, out: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { inner.open(started, budget, out) } finally { lock.unlock() }
    }
    override fun expire(started: salvo.std.time.Tick, budget: salvo.std.time.Duration, out: salvo.SalvoReply, f: salvo.std.time.Fired) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { inner.expire(started, budget, out, f) } finally { lock.unlock() }
    }
}

class Sessions(private val __dep0: salvo.std.time.Timer) : Session {
    val __mailboxCapacity: Int = 8
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Sessions> = kotlin.collections.mutableMapOf()
    override fun open(started: salvo.std.time.Tick, budget: salvo.std.time.Duration, out: salvo.SalvoReply) {
        __dep0.after(salvo.std.time.plus__Duration_Duration(budget, salvo.std.time.seconds(1L)), kotlin.run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_Sessions.Expire(started, budget, out); __r })
    }
    override fun expire(started: salvo.std.time.Tick, budget: salvo.std.time.Duration, out: salvo.SalvoReply, f: salvo.std.time.Fired) {
        salvo.SalvoSched.replyWire(out, salvo.main.verdict(started, f.at, budget), salvo.StrCodec)
    }
}

class __Actor_Sessions(private val handler: Sessions) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch_Sessions_Session(handler, msg as __Msg_Session)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Sessions.Open -> handler.open(c.started, c.budget, value as salvo.SalvoReply)
            is __Cont_Sessions.Expire -> handler.expire(c.started, c.budget, c.out, value as salvo.std.time.Fired)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Sessions.Open -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Sessions.Expire -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.std.time.__Codec_Fired) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Session -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Session)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class TestTicker(private val timer: Int) : salvo.std.time.Ticker {
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun tick(): salvo.std.time.Tick {
        val fired: salvo.std.time.Fired = kotlin.run {
            val (answer, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.std.time.__Codec_Fired) })
            salvo.SalvoSched.sendWire(timer, salvo.std.time.__Msg_Timer.After(salvo.std.time.nanos(0L), answer), salvo.std.time.__PROTO_Timer, salvo.std.time.__Codec___Msg_Timer)
            salvo.SalvoSched.awaitReply(__wid) as salvo.std.time.Fired
        }
        return fired.at
    }
}

interface Sleeper {
    fun nap(wait: salvo.std.time.Duration, out: salvo.SalvoReply)
    fun woke(started: salvo.std.time.Tick, out: salvo.SalvoReply, f: salvo.std.time.Fired)
}

class __Mon_Sleeper(
    private val inner: Sleeper,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Sleeper {
    override fun nap(wait: salvo.std.time.Duration, out: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { inner.nap(wait, out) } finally { lock.unlock() }
    }
    override fun woke(started: salvo.std.time.Tick, out: salvo.SalvoReply, f: salvo.std.time.Fired) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [actor-monitor-handler]" }
        lock.lock()
        try { inner.woke(started, out, f) } finally { lock.unlock() }
    }
}

class Napping(private val __dep0: salvo.std.time.Timer, private val __dep1: salvo.std.time.Ticker) : Sleeper {
    val __mailboxCapacity: Int = 8
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Napping> = kotlin.collections.mutableMapOf()
    override fun nap(wait: salvo.std.time.Duration, out: salvo.SalvoReply) {
        __dep0.after(wait, kotlin.run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_Napping.Woke(__dep1.tick(), out); __r })
    }
    override fun woke(started: salvo.std.time.Tick, out: salvo.SalvoReply, f: salvo.std.time.Fired) {
        salvo.SalvoSched.replyWire(out, "napped ${salvo.std.time.toStr(salvo.std.time.elapsed(__dep1, started))}", salvo.StrCodec)
    }
}

class __Actor_Napping(private val handler: Napping) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch_Napping_Sleeper(handler, msg as __Msg_Sleeper)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Napping.Nap -> handler.nap(c.wait, value as salvo.SalvoReply)
            is __Cont_Napping.Woke -> handler.woke(c.started, c.out, value as salvo.std.time.Fired)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Napping.Nap -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Napping.Woke -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.std.time.__Codec_Fired) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Sleeper -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Sleeper)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(kotlin.collections.listOf(kotlin.Pair("Faults", salvo.std.core.actor.__PROTO_Faults), kotlin.Pair("Session", salvo.main.__PROTO_Session), kotlin.Pair("Sleeper", salvo.main.__PROTO_Sleeper), kotlin.Pair("Timer", salvo.std.time.__PROTO_Timer), kotlin.Pair("TimerCtl", salvo.std.time.__PROTO_TimerCtl), kotlin.Pair("Wheel", salvo.std.runtime.timers.__PROTO_Wheel)))
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val budget: salvo.std.time.Duration = salvo.std.time.millis(1500L)
    salvo.std.core.console.println(__handle_2, "budget ${salvo.std.time.toStr(budget)}, doubled ${salvo.std.time.toStr(salvo.std.time.times(budget, 2L))}, in millis ${salvo.std.time.toMillis(budget)}")
    val stamp: salvo.std.time.Instant = salvo.std.time.epochMilli(1700000000000L)
    salvo.std.core.console.println(__handle_2, "stamp ${salvo.std.time.toEpochSecond(stamp)}s, a minute later ${salvo.std.time.toEpochSecond(salvo.std.time.plus__Instant_Duration(stamp, salvo.std.time.minutes(1L)))}s")
    val __use_3: salvo.std.time.DefaultClock = salvo.std.time.DefaultClock()
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: salvo.std.time.Clock = salvo.std.time.__Mon_Clock(__use_3, __lock___use_3)
    val __use_5: salvo.std.time.DefaultTicker = salvo.std.time.DefaultTicker()
    val __lock___use_5 = java.util.concurrent.locks.ReentrantLock()
    val __handle_6: salvo.std.time.Ticker = salvo.std.time.__Mon_Ticker(__use_5, __lock___use_5)
    salvo.std.core.console.println(__handle_2, "wall clock is set: ${(salvo.std.time.toEpochSecond(__handle_4.now()) > 1600000000L)}")
    val __use_7: SteppingTicker = SteppingTicker(step = salvo.std.time.millis(500L))
    val __lock___use_7 = java.util.concurrent.locks.ReentrantLock()
    val __handle_8: salvo.std.time.Ticker = salvo.std.time.__Mon_Ticker(__use_7, __lock___use_7)
    val started: salvo.std.time.Tick = __handle_8.tick()
    salvo.std.core.console.println(__handle_2, "overdue after one more read: ${salvo.main.overdue(__handle_8, started, budget)}")
    salvo.std.core.console.println(__handle_2, "overdue after three: ${salvo.main.overdue(__handle_8, started, budget)} ${salvo.main.overdue(__handle_8, started, budget)} ${salvo.main.overdue(__handle_8, started, budget)}")
    salvo.std.core.console.println(__handle_2, salvo.main.verdict(salvo.std.time.Tick(nanos = 0L), salvo.std.time.Tick(nanos = 1000000000L), budget))
    salvo.std.core.console.println(__handle_2, salvo.main.verdict(salvo.std.time.Tick(nanos = 0L), salvo.std.time.Tick(nanos = 2000000000L), budget))
    val p: Int = salvo.std.core.actor.pool(1)
    val __destructured_9: kotlin.Pair<Int, Int> = kotlin.run { val __h = salvo.std.time.ManualTime(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, salvo.std.time.__Actor_ManualTime(__h), salvo.std.time.__Actor_ManualTime.__DECODE); kotlin.Pair(__a, __a) }
    val timer: Int = __destructured_9.first
    val ctl: Int = __destructured_9.second
    val sessions: Int = kotlin.run { val __h = Sessions(salvo.std.time.__Stub_Timer(timer)); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sessions(__h), __Actor_Sessions.__DECODE); __a }
    val outcome: String = kotlin.run {
        val (answer, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(sessions, __Msg_Session.Open(salvo.std.time.Tick(nanos = 0L), budget, answer), __PROTO_Session, __Codec___Msg_Session)
        kotlin.run {
            val (settled, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.std.core.actor.__Codec_Idle) })
            salvo.std.core.actor.onIdle(p, settled)
            salvo.SalvoSched.awaitReply(__wid) as salvo.std.core.actor.Idle
        }
        salvo.SalvoSched.sendWire(ctl, salvo.std.time.__Msg_TimerCtl.Advance(salvo.std.time.millis(2500L)), salvo.std.time.__PROTO_TimerCtl, salvo.std.time.__Codec___Msg_TimerCtl)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    salvo.std.core.console.println(__handle_2, "session: ${outcome}")
    val sleeper: Int = kotlin.run { val __h = Napping(salvo.std.time.__Stub_Timer(timer), salvo.std.time.__Mon_Ticker(TestTicker(timer = timer))); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.thread(), __h.__mailboxCapacity, __Actor_Napping(__h), __Actor_Napping.__DECODE); __a }
    val napped: String = kotlin.run {
        val (answer, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(sleeper, __Msg_Sleeper.Nap(salvo.std.time.seconds(2L), answer), __PROTO_Sleeper, __Codec___Msg_Sleeper)
        kotlin.run {
            val (settled, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.std.core.actor.__Codec_Idle) })
            salvo.std.core.actor.onIdle(p, settled)
            salvo.SalvoSched.awaitReply(__wid) as salvo.std.core.actor.Idle
        }
        salvo.SalvoSched.sendWire(ctl, salvo.std.time.__Msg_TimerCtl.Advance(salvo.std.time.seconds(2L)), salvo.std.time.__PROTO_TimerCtl, salvo.std.time.__Codec___Msg_TimerCtl)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    salvo.std.core.console.println(__handle_2, napped)
}


sealed class __Msg_Session {
    class Open(val started: salvo.std.time.Tick, val budget: salvo.std.time.Duration, val out: salvo.SalvoReply) : __Msg_Session()
    class Expire(val started: salvo.std.time.Tick, val budget: salvo.std.time.Duration, val out: salvo.SalvoReply, val f: salvo.std.time.Fired) : __Msg_Session()
}

object __Codec___Msg_Session : salvo.WireCodec<__Msg_Session> {
    override fun enc(v: __Msg_Session, out: salvo.WireOut) {
        when (v) {
            is __Msg_Session.Open -> { out.u8(0); salvo.std.time.__Codec_Tick.enc(v.started, out); salvo.std.time.__Codec_Duration.enc(v.budget, out); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_Session.Expire -> { out.u8(1); salvo.std.time.__Codec_Tick.enc(v.started, out); salvo.std.time.__Codec_Duration.enc(v.budget, out); salvo.ReplyCodec.enc(v.out, out); salvo.std.time.__Codec_Fired.enc(v.f, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Session = when (inp.u8()) {
            0 -> __Msg_Session.Open(salvo.std.time.__Codec_Tick.dec(inp), salvo.std.time.__Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp))
            1 -> __Msg_Session.Expire(salvo.std.time.__Codec_Tick.dec(inp), salvo.std.time.__Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp), salvo.std.time.__Codec_Fired.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Session: String = "7ed70285b7e6568c"


sealed class __Msg_Sleeper {
    class Nap(val wait: salvo.std.time.Duration, val out: salvo.SalvoReply) : __Msg_Sleeper()
    class Woke(val started: salvo.std.time.Tick, val out: salvo.SalvoReply, val f: salvo.std.time.Fired) : __Msg_Sleeper()
}

object __Codec___Msg_Sleeper : salvo.WireCodec<__Msg_Sleeper> {
    override fun enc(v: __Msg_Sleeper, out: salvo.WireOut) {
        when (v) {
            is __Msg_Sleeper.Nap -> { out.u8(0); salvo.std.time.__Codec_Duration.enc(v.wait, out); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_Sleeper.Woke -> { out.u8(1); salvo.std.time.__Codec_Tick.enc(v.started, out); salvo.ReplyCodec.enc(v.out, out); salvo.std.time.__Codec_Fired.enc(v.f, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Sleeper = when (inp.u8()) {
            0 -> __Msg_Sleeper.Nap(salvo.std.time.__Codec_Duration.dec(inp), salvo.ReplyCodec.dec(inp))
            1 -> __Msg_Sleeper.Woke(salvo.std.time.__Codec_Tick.dec(inp), salvo.ReplyCodec.dec(inp), salvo.std.time.__Codec_Fired.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Sleeper: String = "2385b53950dcbd9c"


sealed class __Cont_Sessions {
    class Open(val started: salvo.std.time.Tick, val budget: salvo.std.time.Duration) : __Cont_Sessions()
    class Expire(val started: salvo.std.time.Tick, val budget: salvo.std.time.Duration, val out: salvo.SalvoReply) : __Cont_Sessions()
}


sealed class __Cont_Napping {
    class Nap(val wait: salvo.std.time.Duration) : __Cont_Napping()
    class Woke(val started: salvo.std.time.Tick, val out: salvo.SalvoReply) : __Cont_Napping()
}

class __Stub_Session(private val addr: Int) : Session {
    override fun open(started: salvo.std.time.Tick, budget: salvo.std.time.Duration, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Session.Open(started, budget, out), __PROTO_Session, __Codec___Msg_Session)
    }
    override fun expire(started: salvo.std.time.Tick, budget: salvo.std.time.Duration, out: salvo.SalvoReply, f: salvo.std.time.Fired) {
        salvo.SalvoSched.sendWire(addr, __Msg_Session.Expire(started, budget, out, f), __PROTO_Session, __Codec___Msg_Session)
    }
}

class __Stub_Sleeper(private val addr: Int) : Sleeper {
    override fun nap(wait: salvo.std.time.Duration, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Sleeper.Nap(wait, out), __PROTO_Sleeper, __Codec___Msg_Sleeper)
    }
    override fun woke(started: salvo.std.time.Tick, out: salvo.SalvoReply, f: salvo.std.time.Fired) {
        salvo.SalvoSched.sendWire(addr, __Msg_Sleeper.Woke(started, out, f), __PROTO_Sleeper, __Codec___Msg_Sleeper)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Sessions_Session(__handler: Sessions, __msg: __Msg_Session) {
    when {
        (__msg is __Msg_Session.Open) -> {
            val started = (__msg as __Msg_Session.Open).started
            val budget = (__msg as __Msg_Session.Open).budget
            val out = (__msg as __Msg_Session.Open).out
            __handler.open(started, budget, out)
        }
        (__msg is __Msg_Session.Expire) -> {
            val started = (__msg as __Msg_Session.Expire).started
            val budget = (__msg as __Msg_Session.Expire).budget
            val out = (__msg as __Msg_Session.Expire).out
            val f = (__msg as __Msg_Session.Expire).f
            __handler.expire(started, budget, out, f)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Napping_Sleeper(__handler: Napping, __msg: __Msg_Sleeper) {
    when {
        (__msg is __Msg_Sleeper.Nap) -> {
            val wait = (__msg as __Msg_Sleeper.Nap).wait
            val out = (__msg as __Msg_Sleeper.Nap).out
            __handler.nap(wait, out)
        }
        (__msg is __Msg_Sleeper.Woke) -> {
            val started = (__msg as __Msg_Sleeper.Woke).started
            val out = (__msg as __Msg_Sleeper.Woke).out
            val f = (__msg as __Msg_Sleeper.Woke).f
            __handler.woke(started, out, f)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

