package salvo.main

import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.console.*
import salvo.core.deque.*
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
        salvo.SalvoSched.sendWire(addr, __Msg_Counter.Bump(n), __PROTO_Counter, __Codec___Msg_Counter)
    }
    override fun total(out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Counter.Total(out), __PROTO_Counter, __Codec___Msg_Counter)
    }
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

/** [protocol-hash] The canonical hash of `Counter`. */
const val __PROTO_Counter: String = "f39d50f9ee8f9923"

class Counting : Counter {
    private var sum: Int = 0
    internal val __mailboxCapacity: Int = 8
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Counting> = mutableMapOf()

    override fun bump(n: Int) {
        sum = sum + n
    }

    override fun total(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, sum, salvo.IntCodec)
    }
}

sealed class __Cont_Counting {
    class Bump() : __Cont_Counting()
    class Total() : __Cont_Counting()
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

class __Stub_Ledger(private val addr: Int) : Ledger {
    override fun report(label: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Ledger.Report(label, out), __PROTO_Ledger, __Codec___Msg_Ledger)
    }
    override fun reported(label: String, out: salvo.SalvoReply, total: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_Ledger.Reported(label, out, total), __PROTO_Ledger, __Codec___Msg_Ledger)
    }
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

/** [protocol-hash] The canonical hash of `Ledger`. */
const val __PROTO_Ledger: String = "4a99401c8b66babf"

class Bookkeeping(private val __dep_Counter: Counter) : Ledger {
    internal val __mailboxCapacity: Int = 4
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Bookkeeping> = mutableMapOf()

    override fun report(label: String, out: salvo.SalvoReply) {
        __dep_Counter.total(run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_Bookkeeping.Reported(label, out); __r })
    }

    override fun reported(label: String, out: salvo.SalvoReply, total: Int) {
        salvo.SalvoSched.replyWire(out, "$label=$total", salvo.StrCodec)
    }
}

sealed class __Cont_Bookkeeping {
    class Report(val label: String) : __Cont_Bookkeeping()
    class Reported(val label: String, val out: salvo.SalvoReply) : __Cont_Bookkeeping()
}

class __Actor_Bookkeeping(private val handler: Bookkeeping) : salvo.SalvoActor {
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

/** [protocol-hash] The canonical hash of `Desk`. */
const val __PROTO_Desk: String = "cb180ef2d1bfd753"

class Desking(private val room: Int) : Desk {
    private var waiting: kotlin.collections.ArrayDeque<salvo.SalvoReply> = kotlin.collections.ArrayDeque<salvo.SalvoReply>(listOf<salvo.SalvoReply>())
    internal val __mailboxCapacity: Int = room
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Desking> = mutableMapOf()

    override fun ticket(out: salvo.SalvoReply) {
        waiting.addLast(out)
    }

    override fun serve(name: String) {
        val next = waiting.removeFirstOrNull()
        when {
            next != null -> {
                salvo.SalvoSched.replyWire(next, "served $name", salvo.StrCodec)
            }
            else -> {
                (name).let {}
            }
        }
    }

    override fun closeUp(reason: String) {
        (waiting).toList().forEach({ r -> salvo.SalvoSched.replyWire(r, "closed: $reason", salvo.StrCodec) })
        waiting = kotlin.collections.ArrayDeque<salvo.SalvoReply>(listOf<salvo.SalvoReply>())
    }
}

sealed class __Cont_Desking {
    class Ticket() : __Cont_Desking()
    class Serve() : __Cont_Desking()
    class CloseUp() : __Cont_Desking()
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
            is __Msg_Desk.CloseUp -> handler.closeUp(m.reason)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
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

class __Stub_Fragile(private val addr: Int) : Fragile {
    override fun crash() {
        salvo.SalvoSched.sendWire(addr, __Msg_Fragile.Crash(), __PROTO_Fragile, __Codec___Msg_Fragile)
    }
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

/** [protocol-hash] The canonical hash of `Fragile`. */
const val __PROTO_Fragile: String = "a8c912bc262644a0"

class Breaking : Fragile {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null

    override fun crash() {
        val empty: List<Int> = listOf<Int>()
        val boom = (empty.getOrNull(7) ?: throw AssertionError("salvo: value is absent at main:145:20"))
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
    salvo.SalvoSched.replyWire(out, "$label totalled $total", salvo.StrCodec)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun reportLine(counter: Int, label: String, out: salvo.SalvoReply) {
    salvo.SalvoSched.sendWire(counter, __Msg_Counter.Total(run { val __c0 = label; val __c1 = out; salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) }) { __v -> formatted(__c0, __c1, __v as Int) } }), __PROTO_Counter, __Codec___Msg_Counter)
}

fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Counter", salvo.main.__PROTO_Counter), Pair("Desk", salvo.main.__PROTO_Desk), Pair("Faults", salvo.core.actor.__PROTO_Faults), Pair("Fragile", salvo.main.__PROTO_Fragile), Pair("Ledger", salvo.main.__PROTO_Ledger)))
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val workers = salvo.SalvoSched.pool(2)
    val counter = run { val __h = Counting(); val __a = salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Counting(__h), __Actor_Counting.__DECODE); __a }
    salvo.SalvoSched.sendWire(counter, __Msg_Counter.Bump(2), __PROTO_Counter, __Codec___Msg_Counter)
    salvo.SalvoSched.sendWire(counter, __Msg_Counter.Bump(3), __PROTO_Counter, __Codec___Msg_Counter)
    val sum = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        salvo.SalvoSched.sendWire(counter, __Msg_Counter.Total(out), __PROTO_Counter, __Codec___Msg_Counter)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    println(console, "1. counter total is $sum")
    val ledger = run { val __h = Bookkeeping(__Stub_Counter(counter)); val __a = salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Bookkeeping(__h), __Actor_Bookkeeping.__DECODE); __a }
    val line = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(ledger, __Msg_Ledger.Report("counter", out), __PROTO_Ledger, __Codec___Msg_Ledger)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    println(console, "3. ledger says $line")
    val desk = run { val __h = Desking(8); val __a = salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Desking(__h), __Actor_Desking.__DECODE); __a }
    val first = run {
        val (a, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        salvo.SalvoSched.sendWire(desk, __Msg_Desk.Ticket(a), __PROTO_Desk, __Codec___Msg_Desk)
        val second = run {
            val (b, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
            salvo.SalvoSched.sendWire(desk, __Msg_Desk.Ticket(b), __PROTO_Desk, __Codec___Msg_Desk)
            salvo.SalvoSched.sendWire(desk, __Msg_Desk.Serve("ada"), __PROTO_Desk, __Codec___Msg_Desk)
            salvo.SalvoSched.sendWire(desk, __Msg_Desk.CloseUp("end of day"), __PROTO_Desk, __Codec___Msg_Desk)
            salvo.SalvoSched.awaitReply(__wid) as String
        }
        println(console, "4. second waiter got: $second")
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    println(console, "4. first waiter got: $first")
    val fragile = run { val __h = Breaking(); val __a = salvo.SalvoSched.spawn(workers, __h.__mailboxCapacity, __Actor_Breaking(__h), __Actor_Breaking.__DECODE); __a }
    val exit = run {
        val (gone, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Exit) })
        salvo.SalvoSched.watch(fragile, gone, { __reason -> Exit(__reason) })
        salvo.SalvoSched.sendWire(fragile, __Msg_Fragile.Crash(), __PROTO_Fragile, __Codec___Msg_Fragile)
        salvo.SalvoSched.awaitReply(__wid) as Exit
    }
    println(console, "5. it died with a reason: ${exit.reason.length > 0}")
    salvo.SalvoSched.sendWire(fragile, __Msg_Fragile.Crash(), __PROTO_Fragile, __Codec___Msg_Fragile)
    val counter2: Counter = __Mon_Counter(Counting())
    counter2.bump(4)
    counter2.bump(5)
    val inline = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        counter2.total(out)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    println(console, "6. inline total is $inline")
    val mine = run { val __h = Counting(); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.currentPool(), __h.__mailboxCapacity, __Actor_Counting(__h), __Actor_Counting.__DECODE); __a }
    salvo.SalvoSched.sendWire(mine, __Msg_Counter.Bump(6), __PROTO_Counter, __Codec___Msg_Counter)
    val local = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        salvo.SalvoSched.sendWire(mine, __Msg_Counter.Total(out), __PROTO_Counter, __Codec___Msg_Counter)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    println(console, "7. the main pool's own actor totalled $local")
    val line8 = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        reportLine(mine, "the counter", out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
    println(console, "8. $line8")
    println(console, "done")
}
