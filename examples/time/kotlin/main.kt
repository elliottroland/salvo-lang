package salvo.main

import salvo.*
import salvo.core.actor.*
import salvo.core.console.*
import salvo.core.string.*
import salvo.time.*

fun verdict(started: Tick, at: Tick, budget: Duration): String {
    val took = between__2(started, at)
    if (cmp(took, budget) > 0) {
        return "late by ${to_str__4(minus(took, budget))}"
    }
    return "in time, ${to_str__4(minus(budget, took))} to spare"
}

fun<__Fx> overdue(__fx: __Fx, started: Tick, budget: Duration): Boolean where __Fx : __Has_Ticker {
    return cmp(elapsed(__fx, started), budget) > 0
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
        salvo.SalvoSched.send(addr, __Msg_Session.Open(started, budget, out))
    }
    override fun expire(started: Tick, budget: Duration, out: salvo.SalvoReply, f: Fired) {
        salvo.SalvoSched.send(addr, __Msg_Session.Expire(started, budget, out, f))
    }
}

sealed class __Msg_Session {
    class Open(val started: Tick, val budget: Duration, val out: salvo.SalvoReply) : __Msg_Session()
    class Expire(val started: Tick, val budget: Duration, val out: salvo.SalvoReply, val f: Fired) : __Msg_Session()
}

class Sessions<__Fx>(private val __fx: __Fx) : Session where __Fx : __Has_Timer {
    internal val __mailboxCapacity: Int = 8
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Sessions> = mutableMapOf()

    override fun open(started: Tick, budget: Duration, out: salvo.SalvoReply) {
        __fx.__fx_Timer.after(plus(budget, seconds(1L)), run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_Sessions.Expire(started, budget, out); __r })
    }

    override fun expire(started: Tick, budget: Duration, out: salvo.SalvoReply, f: Fired) {
        out.send(verdict(started, f.at, budget))
    }
}

sealed class __Cont_Sessions {
    class Open(val started: Tick, val budget: Duration) : __Cont_Sessions()
    class Expire(val started: Tick, val budget: Duration, val out: salvo.SalvoReply) : __Cont_Sessions()
}

class __Actor_Sessions<__Fx>(private val handler: Sessions<__Fx>) : salvo.SalvoActor where __Fx : __Has_Timer {
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
}

class TestTicker(private val timer: Int) : Ticker {

    override fun tick(): Tick {
        val fired = run {
            val (answer, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.send(timer, __Msg_Timer.After(nanos(0L), answer))
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
        salvo.SalvoSched.send(addr, __Msg_Sleeper.Nap(wait, out))
    }
    override fun woke(started: Tick, out: salvo.SalvoReply, f: Fired) {
        salvo.SalvoSched.send(addr, __Msg_Sleeper.Woke(started, out, f))
    }
}

sealed class __Msg_Sleeper {
    class Nap(val wait: Duration, val out: salvo.SalvoReply) : __Msg_Sleeper()
    class Woke(val started: Tick, val out: salvo.SalvoReply, val f: Fired) : __Msg_Sleeper()
}

class Napping<__Fx>(private val __fx: __Fx) : Sleeper where __Fx : __Has_Timer, __Fx : __Has_Ticker {
    internal val __mailboxCapacity: Int = 8
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Napping> = mutableMapOf()

    override fun nap(wait: Duration, out: salvo.SalvoReply) {
        __fx.__fx_Timer.after(wait, run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_Napping.Woke(__fx.__fx_Ticker.tick(), out); __r })
    }

    override fun woke(started: Tick, out: salvo.SalvoReply, f: Fired) {
        out.send("napped ${to_str__4(elapsed(__fx, started))}")
    }
}

sealed class __Cont_Napping {
    class Nap(val wait: Duration) : __Cont_Napping()
    class Woke(val started: Tick, val out: salvo.SalvoReply) : __Cont_Napping()
}

class __Actor_Napping<__Fx>(private val handler: Napping<__Fx>) : salvo.SalvoActor where __Fx : __Has_Timer, __Fx : __Has_Ticker {
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
}

fun main() {
    val __fx = __Fx_1(StdOutConsole())
    val budget = millis(1500L)
    println(__fx, "budget ${to_str__4(budget)}, doubled ${to_str__4(times(budget, 2L))}, in millis ${to_millis(budget)}")
    val stamp = epoch_milli(1700000000000L)
    println(__fx, "stamp ${to_epoch_second(stamp)}s, a minute later ${to_epoch_second(plus__2(stamp, minutes(1L)))}s")
    val __fx2 = __Fx_2(__Mon_Clock(DefaultClock()), __fx.__fx_Console)
    val __fx3 = __Fx_3(__fx2.__fx_Clock, __fx2.__fx_Console, DefaultTicker())
    println(__fx3, "wall clock is set: ${to_epoch_second(__fx3.__fx_Clock.now()) > 1600000000}")
    val __fx4 = __Fx_3(__fx3.__fx_Clock, __fx3.__fx_Console, __Mon_Ticker(SteppingTicker(millis(500L))))
    val started = __fx4.__fx_Ticker.tick()
    println(__fx4, "overdue after one more read: ${overdue(__fx4, started, budget)}")
    println(__fx4, "overdue after three: ${overdue(__fx4, started, budget)} ${overdue(__fx4, started, budget)} ${overdue(__fx4, started, budget)}")
    println(__fx4, verdict(Tick(nanos = 0L), Tick(nanos = 1000000000L), budget))
    println(__fx4, verdict(Tick(nanos = 0L), Tick(nanos = 2000000000L), budget))
    val p = salvo.SalvoSched.pool(1)
    val (timer, ctl) = run { val __h = ManualTime(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_ManualTime(__h)); Pair(__a, __a) }
    val sessions = run { val __h = Sessions(__Fx_4(__Stub_Timer(timer))); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sessions(__h)) }
    val outcome = run {
        val (answer, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.send(sessions, __Msg_Session.Open(Tick(nanos = 0L), budget, answer))
        run {
            val (settled, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.onIdle(p, settled, { __gates, __tokens -> Idle(__gates, __tokens) })
            salvo.SalvoSched.awaitReply(__wid) as Idle
        }
        salvo.SalvoSched.send(ctl, __Msg_TimerCtl.Advance(millis(2500L)))
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    println(__fx4, "session: $outcome")
    val sleeper = run { val __h = Napping(__Fx_5(TestTicker(timer), __Stub_Timer(timer))); salvo.SalvoSched.spawn(salvo.SalvoSched.thread(), __h.__mailboxCapacity, __Actor_Napping(__h)) }
    val napped = run {
        val (answer, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.send(sleeper, __Msg_Sleeper.Nap(seconds(2L), answer))
        run {
            val (settled, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.onIdle(p, settled, { __gates, __tokens -> Idle(__gates, __tokens) })
            salvo.SalvoSched.awaitReply(__wid) as Idle
        }
        salvo.SalvoSched.send(ctl, __Msg_TimerCtl.Advance(seconds(2L)))
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    println(__fx4, napped)
}

class __Fx_1(
    override val __fx_Console: Console,
) : __Has_Console

class __Fx_2(
    override val __fx_Clock: Clock,
    override val __fx_Console: Console,
) : __Has_Clock, __Has_Console

class __Fx_3(
    override val __fx_Clock: Clock,
    override val __fx_Console: Console,
    override val __fx_Ticker: Ticker,
) : __Has_Clock, __Has_Console, __Has_Ticker

class __Fx_4(
    override val __fx_Timer: Timer,
) : __Has_Timer

class __Fx_5(
    override val __fx_Ticker: Ticker,
    override val __fx_Timer: Timer,
) : __Has_Ticker, __Has_Timer
