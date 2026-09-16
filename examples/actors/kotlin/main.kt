package salvo.main

import salvo.*
import salvo.core.actor.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.console.*
import salvo.core.fs.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

interface Counter {
    fun bump(n: Int)
    fun total(out: salvo.SalvoReply)
}

class __Stub_Counter(private val addr: Int) : Counter {
    override fun bump(n: Int) {
        salvo.SalvoSched.send(addr, __Msg_Counter.Bump(n))
    }
    override fun total(out: salvo.SalvoReply) {
        salvo.SalvoSched.send(addr, __Msg_Counter.Total(out))
    }
}

sealed class __Msg_Counter {
    class Bump(val n: Int) : __Msg_Counter()
    class Total(val out: salvo.SalvoReply) : __Msg_Counter()
}

sealed class __Cont_Counter {
    class Bump() : __Cont_Counter()
    class Total() : __Cont_Counter()
}

class Counting : Counter {
    private var sum: Int = 0
    internal val __mailboxCapacity: Int = 8
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Counter> = mutableMapOf()

    override fun bump(n: Int) {
        sum = sum + n
    }

    override fun total(out: salvo.SalvoReply) {
        out.send(sum)
    }
}

class __Actor_Counting(private val handler: Counting) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Counter)
    }

    private fun __dispatch(m: __Msg_Counter) {
        when (m) {
            is __Msg_Counter.Bump -> handler.bump(m.n)
            is __Msg_Counter.Total -> handler.total(m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Counter.Bump -> handler.bump(value as Int)
            is __Cont_Counter.Total -> handler.total(value as salvo.SalvoReply)
        }
    }
}

interface Ledger {
    fun report(label: String, out: salvo.SalvoReply)
    fun reported(label: String, out: salvo.SalvoReply, total: Int)
}

class __Stub_Ledger(private val addr: Int) : Ledger {
    override fun report(label: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.send(addr, __Msg_Ledger.Report(label, out))
    }
    override fun reported(label: String, out: salvo.SalvoReply, total: Int) {
        salvo.SalvoSched.send(addr, __Msg_Ledger.Reported(label, out, total))
    }
}

sealed class __Msg_Ledger {
    class Report(val label: String, val out: salvo.SalvoReply) : __Msg_Ledger()
    class Reported(val label: String, val out: salvo.SalvoReply, val total: Int) : __Msg_Ledger()
}

sealed class __Cont_Ledger {
    class Report(val label: String) : __Cont_Ledger()
    class Reported(val label: String, val out: salvo.SalvoReply) : __Cont_Ledger()
}

class Bookkeeping<__Fx>(private val __fx: __Fx) : Ledger where __Fx : __Has_Counter {
    internal val __mailboxCapacity: Int = 4
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Ledger> = mutableMapOf()

    override fun report(label: String, out: salvo.SalvoReply) {
        __fx.__fx_Counter.total(run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_Ledger.Reported(label, out); __r })
    }

    override fun reported(label: String, out: salvo.SalvoReply, total: Int) {
        out.send("$label=$total")
    }
}

class __Actor_Bookkeeping<__Fx>(private val handler: Bookkeeping<__Fx>) : salvo.SalvoActor where __Fx : __Has_Counter {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Ledger)
    }

    private fun __dispatch(m: __Msg_Ledger) {
        when (m) {
            is __Msg_Ledger.Report -> handler.report(m.label, m.out)
            is __Msg_Ledger.Reported -> handler.reported(m.label, m.out, m.total)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Ledger.Report -> handler.report(c.label, value as salvo.SalvoReply)
            is __Cont_Ledger.Reported -> handler.reported(c.label, c.out, value as Int)
        }
    }
}

interface Desk {
    fun ticket(out: salvo.SalvoReply)
    fun serve(name: String)
    fun close_up(reason: String)
}

class __Stub_Desk(private val addr: Int) : Desk {
    override fun ticket(out: salvo.SalvoReply) {
        salvo.SalvoSched.send(addr, __Msg_Desk.Ticket(out))
    }
    override fun serve(name: String) {
        salvo.SalvoSched.send(addr, __Msg_Desk.Serve(name))
    }
    override fun close_up(reason: String) {
        salvo.SalvoSched.send(addr, __Msg_Desk.CloseUp(reason))
    }
}

sealed class __Msg_Desk {
    class Ticket(val out: salvo.SalvoReply) : __Msg_Desk()
    class Serve(val name: String) : __Msg_Desk()
    class CloseUp(val reason: String) : __Msg_Desk()
}

sealed class __Cont_Desk {
    class Ticket() : __Cont_Desk()
    class Serve() : __Cont_Desk()
    class CloseUp() : __Cont_Desk()
}

class Desking(private val room: Int) : Desk {
    private var waiting: MutableList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
    internal val __mailboxCapacity: Int = room
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Desk> = mutableMapOf()

    override fun ticket(out: salvo.SalvoReply) {
        waiting.add(out)
    }

    override fun serve(name: String) {
        val next = (waiting).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
        when {
            next != null -> {
                next.send("served $name")
            }
            else -> {
                (name).let {}
            }
        }
    }

    override fun close_up(reason: String) {
        (waiting).toList().forEach({ r -> r.send("closed: $reason") })
        waiting = mutableListOf<salvo.SalvoReply>()
    }
}

class __Actor_Desking(private val handler: Desking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Desk)
    }

    private fun __dispatch(m: __Msg_Desk) {
        when (m) {
            is __Msg_Desk.Ticket -> handler.ticket(m.out)
            is __Msg_Desk.Serve -> handler.serve(m.name)
            is __Msg_Desk.CloseUp -> handler.close_up(m.reason)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Desk.Ticket -> handler.ticket(value as salvo.SalvoReply)
            is __Cont_Desk.Serve -> handler.serve(value as String)
            is __Cont_Desk.CloseUp -> handler.close_up(value as String)
        }
    }
}

interface Fragile {
    fun crash()
}

class __Stub_Fragile(private val addr: Int) : Fragile {
    override fun crash() {
        salvo.SalvoSched.send(addr, __Msg_Fragile.Crash())
    }
}

sealed class __Msg_Fragile {
    class Crash() : __Msg_Fragile()
}

class Breaking : Fragile {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null

    override fun crash() {
        val empty: List<Int> = listOf<Int>()
        val boom = empty.getOrNull(7)!!
        (boom).let {}
    }
}

class __Actor_Breaking(private val handler: Breaking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Fragile)
    }

    private fun __dispatch(m: __Msg_Fragile) {
        when (m) {
            is __Msg_Fragile.Crash -> handler.crash()
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        error("this protocol has no continuation targets")
    }
}

fun main() {
    val __fx = __Fx_1(StdOutConsole())
    val workers = salvo.SalvoSched.pool(2)
    val counter = run { val __h = Counting(); salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Counting(__h)) }
    salvo.SalvoSched.send(counter, __Msg_Counter.Bump(2))
    salvo.SalvoSched.send(counter, __Msg_Counter.Bump(3))
    val sum = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.send(counter, __Msg_Counter.Total(out))
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    println(__fx, "1. counter total is $sum")
    val ledger = run { val __h = Bookkeeping(__Fx_2(__Stub_Counter(counter))); salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Bookkeeping(__h)) }
    val line = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.send(ledger, __Msg_Ledger.Report("counter", out))
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    println(__fx, "3. ledger says $line")
    val desk = run { val __h = Desking(8); salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Desking(__h)) }
    val first = run {
        val (a, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.send(desk, __Msg_Desk.Ticket(a))
        val second = run {
            val (b, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.send(desk, __Msg_Desk.Ticket(b))
            salvo.SalvoSched.send(desk, __Msg_Desk.Serve("ada"))
            salvo.SalvoSched.send(desk, __Msg_Desk.CloseUp("end of day"))
            salvo.SalvoSched.awaitReply(__wid) as String
        }
        println(__fx, "4. second waiter got: $second")
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    println(__fx, "4. first waiter got: $first")
    val fragile = run { val __h = Breaking(); salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Breaking(__h)) }
    val exit = run {
        val (gone, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.watch(fragile, gone, { __reason -> Exit(__reason) })
        salvo.SalvoSched.send(fragile, __Msg_Fragile.Crash())
        salvo.SalvoSched.awaitReply(__wid) as Exit
    }
    println(__fx, "5. it died with a reason: ${exit.reason.length > 0}")
    salvo.SalvoSched.send(fragile, __Msg_Fragile.Crash())
    val __fx2 = __Fx_3(__fx.__fx_Console, Counting())
    __fx2.__fx_Counter.bump(4)
    __fx2.__fx_Counter.bump(5)
    val inline = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        __fx2.__fx_Counter.total(out)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    println(__fx2, "6. inline total is $inline")
    println(__fx2, "done")
}

class __Fx_1(
    override val __fx_Console: Console,
) : __Has_Console

class __Fx_2(
    override val __fx_Counter: Counter,
) : __Has_Counter

class __Fx_3(
    override val __fx_Console: Console,
    override val __fx_Counter: Counter,
) : __Has_Console, __Has_Counter
