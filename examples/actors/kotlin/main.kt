package salvo.main

import salvo.*

interface Counter {
    fun bump(n: Int)
    fun total(out: salvo.SalvoReply)
}

class __Mon_Counter(
    private val inner: Counter,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Counter {
    override fun bump(n: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.bump(n) } finally { lock.unlock() }
    }
    override fun total(out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.total(out) } finally { lock.unlock() }
    }
}

class __Stub_Counter(private val addr: Int) : Counter {
    override fun bump(n: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_Counter.Bump(n), __PROTO_Counter, __Codec___Msg_Counter)
    }
    override fun total(out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Counter.Total(out), __PROTO_Counter, __Codec___Msg_Counter)
    }
}

class Counting : Counter {
    val __mailboxCapacity: Int = 8
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Counting> = mutableMapOf()
    var sum: Int = 0
    override fun bump(n: Int) {
        sum = (sum + n)
    }
    override fun total(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, sum, salvo.IntCodec)
    }
}

class __Actor_Counting(private val handler: Counting) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatchCounter(msg as __Msg_Counter)
    }

    private fun __dispatchCounter(m: __Msg_Counter) {
        when (m) {
            is __Msg_Counter.Bump -> handler.bump(m.n)
            is __Msg_Counter.Total -> handler.total(m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Counting.Bump -> handler.bump(value as Int)
            is __Cont_Counting.Total -> handler.total(value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Counting.Bump -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })(payload)
            is __Cont_Counting.Total -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Counter -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Counter)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

interface Ledger {
    fun report(label: String, out: salvo.SalvoReply)
    fun reported(label: String, out: salvo.SalvoReply, total: Int)
}

class __Mon_Ledger(
    private val inner: Ledger,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Ledger {
    override fun report(label: String, out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.report(label, out) } finally { lock.unlock() }
    }
    override fun reported(label: String, out: salvo.SalvoReply, total: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.reported(label, out, total) } finally { lock.unlock() }
    }
}

class __Stub_Ledger(private val addr: Int) : Ledger {
    override fun report(label: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Ledger.Report(label, out), __PROTO_Ledger, __Codec___Msg_Ledger)
    }
    override fun reported(label: String, out: salvo.SalvoReply, total: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_Ledger.Reported(label, out, total), __PROTO_Ledger, __Codec___Msg_Ledger)
    }
}

class Bookkeeping(private val __dep0: Counter) : Ledger {
    val __mailboxCapacity: Int = 4
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Bookkeeping> = mutableMapOf()
    override fun report(label: String, out: salvo.SalvoReply) {
        __dep0.total(run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_Bookkeeping.Reported(label, out); __r })
    }
    override fun reported(label: String, out: salvo.SalvoReply, total: Int) {
        salvo.SalvoSched.replyWire(out, "${label}=${total}", salvo.StrCodec)
    }
}

class __Actor_Bookkeeping(private val handler: Bookkeeping) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatchLedger(msg as __Msg_Ledger)
    }

    private fun __dispatchLedger(m: __Msg_Ledger) {
        when (m) {
            is __Msg_Ledger.Report -> handler.report(m.label, m.out)
            is __Msg_Ledger.Reported -> handler.reported(m.label, m.out, m.total)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Bookkeeping.Report -> handler.report(c.label, value as salvo.SalvoReply)
            is __Cont_Bookkeeping.Reported -> handler.reported(c.label, c.out, value as Int)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Bookkeeping.Report -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Bookkeeping.Reported -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Ledger -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Ledger)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

interface Desk {
    fun ticket(out: salvo.SalvoReply)
    fun serve(name: String)
    fun closeUp(reason: String)
}

class __Mon_Desk(
    private val inner: Desk,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Desk {
    override fun ticket(out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.ticket(out) } finally { lock.unlock() }
    }
    override fun serve(name: String) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.serve(name) } finally { lock.unlock() }
    }
    override fun closeUp(reason: String) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.closeUp(reason) } finally { lock.unlock() }
    }
}

class __Stub_Desk(private val addr: Int) : Desk {
    override fun ticket(out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Desk.Ticket(out), __PROTO_Desk, __Codec___Msg_Desk)
    }
    override fun serve(name: String) {
        salvo.SalvoSched.sendWire(addr, __Msg_Desk.Serve(name), __PROTO_Desk, __Codec___Msg_Desk)
    }
    override fun closeUp(reason: String) {
        salvo.SalvoSched.sendWire(addr, __Msg_Desk.CloseUp(reason), __PROTO_Desk, __Codec___Msg_Desk)
    }
}

class Desking(private val room: Int) : Desk {
    val __mailboxCapacity: Int = room
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Desking> = mutableMapOf()
    var waiting: salvo.platform.core.deque.MutDeque<salvo.SalvoReply> = salvo.core.deque.mutDequeOf()
    override fun ticket(out: salvo.SalvoReply) {
        salvo.core.deque.addLastPlatform(waiting, out)
    }
    override fun serve(name: String) {
        val next: salvo.SalvoReply? = salvo.core.deque.removeFirstPlatform(waiting)
        when {
            (next != null) -> {
                val next_1: salvo.SalvoReply = next!!
                salvo.SalvoSched.replyWire(next_1, "served ${name}", salvo.StrCodec)
            }
            (next == null) -> {
                run { name; Unit }
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
    override fun closeUp(reason: String) {
        salvo.core.deque.drain(waiting, fun(r: salvo.SalvoReply) {
            return salvo.SalvoSched.replyWire(r, "closed: ${reason}", salvo.StrCodec)
        })
        waiting = salvo.core.deque.mutDequeOf()
    }
}

class __Actor_Desking(private val handler: Desking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatchDesk(msg as __Msg_Desk)
    }

    private fun __dispatchDesk(m: __Msg_Desk) {
        when (m) {
            is __Msg_Desk.Ticket -> handler.ticket(m.out)
            is __Msg_Desk.Serve -> handler.serve(m.name)
            is __Msg_Desk.CloseUp -> handler.closeUp(m.reason)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Desking.Ticket -> handler.ticket(value as salvo.SalvoReply)
            is __Cont_Desking.Serve -> handler.serve(value as String)
            is __Cont_Desking.CloseUp -> handler.closeUp(value as String)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Desking.Ticket -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Desking.Serve -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })(payload)
            is __Cont_Desking.CloseUp -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Desk -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Desk)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

interface Fragile {
    fun crash()
}

class __Mon_Fragile(
    private val inner: Fragile,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Fragile {
    override fun crash() {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.crash() } finally { lock.unlock() }
    }
}

class __Stub_Fragile(private val addr: Int) : Fragile {
    override fun crash() {
        salvo.SalvoSched.sendWire(addr, __Msg_Fragile.Crash(), __PROTO_Fragile, __Codec___Msg_Fragile)
    }
}

class Breaking : Fragile {
    val __mailboxCapacity: Int = 1
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Breaking> = mutableMapOf()
    override fun crash() {
        val empty: List<Int> = listOf<Int>()
        val boom: Int = run {
            val __nn_1: Int? = salvo.core.list.getPlatform(empty, 7)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:145:20"))
                }
                else -> {
                    val __some_2: Int = __nn_1!!
                    __some_2
                }
            }
        }
        run { boom; Unit }
    }
}

class __Actor_Breaking(private val handler: Breaking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatchFragile(msg as __Msg_Fragile)
    }

    private fun __dispatchFragile(m: __Msg_Fragile) {
        when (m) {
            is __Msg_Fragile.Crash -> handler.crash()
        }
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
                __PROTO_Fragile -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Fragile)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

fun formatted(label: String, out: salvo.SalvoReply, total: Int) {
    salvo.SalvoSched.replyWire(out, "${label} totalled ${total}", salvo.StrCodec)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun reportLine(counter: Int, label: String, out: salvo.SalvoReply) {
    salvo.SalvoSched.sendWire(counter, __Msg_Counter.Total(run { val __c0 = label; val __c1 = out; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) }) { __v -> formatted(__c0, __c1, __v as Int) } }), __PROTO_Counter, __Codec___Msg_Counter)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Counter", salvo.main.__PROTO_Counter), Pair("Desk", salvo.main.__PROTO_Desk), Pair("Faults", salvo.core.actor.__PROTO_Faults), Pair("Fragile", salvo.main.__PROTO_Fragile), Pair("Ledger", salvo.main.__PROTO_Ledger)))
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val workers: Int = salvo.core.actor.pool(2)
    val counter: Int = run { val __h = Counting(); val __a = salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Counting(__h), __Actor_Counting.__DECODE); __a }
    salvo.SalvoSched.sendWire(counter, __Msg_Counter.Bump(2), __PROTO_Counter, __Codec___Msg_Counter)
    salvo.SalvoSched.sendWire(counter, __Msg_Counter.Bump(3), __PROTO_Counter, __Codec___Msg_Counter)
    val sum: Int = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        salvo.SalvoSched.sendWire(counter, __Msg_Counter.Total(out), __PROTO_Counter, __Codec___Msg_Counter)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    salvo.core.console.println(__handle_2, "1. counter total is ${sum}")
    val ledger: Int = run { val __h = Bookkeeping(__Stub_Counter(counter)); val __a = salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Bookkeeping(__h), __Actor_Bookkeeping.__DECODE); __a }
    val line: String = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(ledger, __Msg_Ledger.Report("counter", out), __PROTO_Ledger, __Codec___Msg_Ledger)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    salvo.core.console.println(__handle_2, "3. ledger says ${line}")
    val desk: Int = run { val __h = Desking(room = 8); val __a = salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Desking(__h), __Actor_Desking.__DECODE); __a }
    val first: String = run {
        val (a, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(desk, __Msg_Desk.Ticket(a), __PROTO_Desk, __Codec___Msg_Desk)
        val second: String = run {
            val (b, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
            salvo.SalvoSched.sendWire(desk, __Msg_Desk.Ticket(b), __PROTO_Desk, __Codec___Msg_Desk)
            salvo.SalvoSched.sendWire(desk, __Msg_Desk.Serve("ada"), __PROTO_Desk, __Codec___Msg_Desk)
            salvo.SalvoSched.sendWire(desk, __Msg_Desk.CloseUp("end of day"), __PROTO_Desk, __Codec___Msg_Desk)
            salvo.SalvoSched.awaitReply(__wid) as String
        }
        salvo.core.console.println(__handle_2, "4. second waiter got: ${second}")
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    salvo.core.console.println(__handle_2, "4. first waiter got: ${first}")
    val fragile: Int = run { val __h = Breaking(); val __a = salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Breaking(__h), __Actor_Breaking.__DECODE); __a }
    val exit: salvo.core.actor.Exit = run {
        val (gone, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.core.actor.__Codec_Exit) })
        salvo.core.actor.watch(fragile, gone)
        salvo.SalvoSched.sendWire(fragile, __Msg_Fragile.Crash(), __PROTO_Fragile, __Codec___Msg_Fragile)
        salvo.SalvoSched.awaitReply(__wid) as salvo.core.actor.Exit
    }
    salvo.core.console.println(__handle_2, "5. it died with a reason: ${(salvo.core.string.sizePlatform(exit.reason) > 0)}")
    salvo.SalvoSched.sendWire(fragile, __Msg_Fragile.Crash(), __PROTO_Fragile, __Codec___Msg_Fragile)
    val __use_3: Counting = Counting()
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: Counter = __Mon_Counter(__use_3, __lock___use_3)
    __handle_4.bump(4)
    __handle_4.bump(5)
    val inline: Int = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        __handle_4.total(out)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    salvo.core.console.println(__handle_2, "6. inline total is ${inline}")
    val mine: Int = run { val __h = Counting(); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.currentPool(), __h.__mailboxCapacity, __Actor_Counting(__h), __Actor_Counting.__DECODE); __a }
    salvo.SalvoSched.sendWire(mine, __Msg_Counter.Bump(6), __PROTO_Counter, __Codec___Msg_Counter)
    val local: Int = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        salvo.SalvoSched.sendWire(mine, __Msg_Counter.Total(out), __PROTO_Counter, __Codec___Msg_Counter)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    salvo.core.console.println(__handle_2, "7. the main pool's own actor totalled ${local}")
    val line8: String = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        reportLine(mine, "the counter", out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    salvo.core.console.println(__handle_2, "8. ${line8}")
    salvo.core.console.println(__handle_2, "done")
}


sealed class __Msg_Counter {
    class Bump(val n: Int) : __Msg_Counter()
    class Total(val out: salvo.SalvoReply) : __Msg_Counter()
}

object __Codec___Msg_Counter : salvo.WireCodec<__Msg_Counter> {
    override fun enc(v: __Msg_Counter, out: salvo.WireOut) {
        when (v) {
            is __Msg_Counter.Bump -> { out.u8(0); salvo.IntCodec.enc(v.n, out) }
            is __Msg_Counter.Total -> { out.u8(1); salvo.ReplyCodec.enc(v.out, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Counter = when (inp.u8()) {
            0 -> __Msg_Counter.Bump(salvo.IntCodec.dec(inp))
            1 -> __Msg_Counter.Total(salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Counter: String = "f39d50f9ee8f9923"


sealed class __Cont_Counting {
    class Bump() : __Cont_Counting()
    class Total() : __Cont_Counting()
}


sealed class __Msg_Ledger {
    class Report(val label: String, val out: salvo.SalvoReply) : __Msg_Ledger()
    class Reported(val label: String, val out: salvo.SalvoReply, val total: Int) : __Msg_Ledger()
}

object __Codec___Msg_Ledger : salvo.WireCodec<__Msg_Ledger> {
    override fun enc(v: __Msg_Ledger, out: salvo.WireOut) {
        when (v) {
            is __Msg_Ledger.Report -> { out.u8(0); salvo.StrCodec.enc(v.label, out); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_Ledger.Reported -> { out.u8(1); salvo.StrCodec.enc(v.label, out); salvo.ReplyCodec.enc(v.out, out); salvo.IntCodec.enc(v.total, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Ledger = when (inp.u8()) {
            0 -> __Msg_Ledger.Report(salvo.StrCodec.dec(inp), salvo.ReplyCodec.dec(inp))
            1 -> __Msg_Ledger.Reported(salvo.StrCodec.dec(inp), salvo.ReplyCodec.dec(inp), salvo.IntCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Ledger: String = "4a99401c8b66babf"


sealed class __Cont_Bookkeeping {
    class Report(val label: String) : __Cont_Bookkeeping()
    class Reported(val label: String, val out: salvo.SalvoReply) : __Cont_Bookkeeping()
}


sealed class __Msg_Desk {
    class Ticket(val out: salvo.SalvoReply) : __Msg_Desk()
    class Serve(val name: String) : __Msg_Desk()
    class CloseUp(val reason: String) : __Msg_Desk()
}

object __Codec___Msg_Desk : salvo.WireCodec<__Msg_Desk> {
    override fun enc(v: __Msg_Desk, out: salvo.WireOut) {
        when (v) {
            is __Msg_Desk.Ticket -> { out.u8(0); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_Desk.Serve -> { out.u8(1); salvo.StrCodec.enc(v.name, out) }
            is __Msg_Desk.CloseUp -> { out.u8(2); salvo.StrCodec.enc(v.reason, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Desk = when (inp.u8()) {
            0 -> __Msg_Desk.Ticket(salvo.ReplyCodec.dec(inp))
            1 -> __Msg_Desk.Serve(salvo.StrCodec.dec(inp))
            2 -> __Msg_Desk.CloseUp(salvo.StrCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Desk: String = "cb180ef2d1bfd753"


sealed class __Cont_Desking {
    class Ticket() : __Cont_Desking()
    class Serve() : __Cont_Desking()
    class CloseUp() : __Cont_Desking()
}


sealed class __Msg_Fragile {
    class Crash() : __Msg_Fragile()
}

object __Codec___Msg_Fragile : salvo.WireCodec<__Msg_Fragile> {
    override fun enc(v: __Msg_Fragile, out: salvo.WireOut) {
        when (v) {
            is __Msg_Fragile.Crash -> { out.u8(0) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Fragile = when (inp.u8()) {
            0 -> __Msg_Fragile.Crash()
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Fragile: String = "a8c912bc262644a0"


sealed class __Cont_Breaking {
}

