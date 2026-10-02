package salvo.main

import salvo.core.actor.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.console.*
import salvo.core.deque.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.range.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.net.*
import salvo.time.*

interface Sequencer {
    fun next(out: salvo.SalvoReply)
}

class __Stub_Sequencer(private val addr: Int) : Sequencer {
    override fun next(out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Sequencer.Next(out), __PROTO_Sequencer, __Codec___Msg_Sequencer)
    }
}

class __Mon_Sequencer(private val inner: Sequencer) : Sequencer {
    override fun next(out: salvo.SalvoReply) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.next(out) }
    }
}

sealed class __Msg_Sequencer {
    class Next(val out: salvo.SalvoReply) : __Msg_Sequencer()
}

object __Codec___Msg_Sequencer : salvo.WireCodec<__Msg_Sequencer> {
    override fun enc(v: __Msg_Sequencer, out: salvo.WireOut) {
        when (v) {
            is __Msg_Sequencer.Next -> { out.u8(0); salvo.ReplyCodec.enc(v.out, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Sequencer = when (inp.u8()) {
            0 -> __Msg_Sequencer.Next(salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Sequencer`. */
const val __PROTO_Sequencer: String = "7a5334482e5247f7"

interface Inventory {
    fun reserve(sku: String, qty: Int, out: salvo.SalvoReply)
}

class __Stub_Inventory(private val addr: Int) : Inventory {
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Inventory.Reserve(sku, qty, out), __PROTO_Inventory, __Codec___Msg_Inventory)
    }
}

class __Mon_Inventory(private val inner: Inventory) : Inventory {
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.reserve(sku, qty, out) }
    }
}

sealed class __Msg_Inventory {
    class Reserve(val sku: String, val qty: Int, val out: salvo.SalvoReply) : __Msg_Inventory()
}

object __Codec___Msg_Inventory : salvo.WireCodec<__Msg_Inventory> {
    override fun enc(v: __Msg_Inventory, out: salvo.WireOut) {
        when (v) {
            is __Msg_Inventory.Reserve -> { out.u8(0); salvo.StrCodec.enc(v.sku, out); salvo.IntCodec.enc(v.qty, out); salvo.ReplyCodec.enc(v.out, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Inventory = when (inp.u8()) {
            0 -> __Msg_Inventory.Reserve(salvo.StrCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Inventory`. */
const val __PROTO_Inventory: String = "d3482a697a944808"

interface Search {
    fun query(word: String, out: salvo.SalvoReply)
}

class __Stub_Search(private val addr: Int) : Search {
    override fun query(word: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Search.Query(word, out), __PROTO_Search, __Codec___Msg_Search)
    }
}

class __Mon_Search(private val inner: Search) : Search {
    override fun query(word: String, out: salvo.SalvoReply) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.query(word, out) }
    }
}

sealed class __Msg_Search {
    class Query(val word: String, val out: salvo.SalvoReply) : __Msg_Search()
}

object __Codec___Msg_Search : salvo.WireCodec<__Msg_Search> {
    override fun enc(v: __Msg_Search, out: salvo.WireOut) {
        when (v) {
            is __Msg_Search.Query -> { out.u8(0); salvo.StrCodec.enc(v.word, out); salvo.ReplyCodec.enc(v.out, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Search = when (inp.u8()) {
            0 -> __Msg_Search.Query(salvo.StrCodec.dec(inp), salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Search`. */
const val __PROTO_Search: String = "ed817fc30774f01d"

interface Lookup {
    fun lookup(key: String, out: salvo.SalvoReply)
}

class __Stub_Lookup(private val addr: Int) : Lookup {
    override fun lookup(key: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Lookup.Lookup(key, out), __PROTO_Lookup, __Codec___Msg_Lookup)
    }
}

class __Mon_Lookup(private val inner: Lookup) : Lookup {
    override fun lookup(key: String, out: salvo.SalvoReply) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.lookup(key, out) }
    }
}

sealed class __Msg_Lookup {
    class Lookup(val key: String, val out: salvo.SalvoReply) : __Msg_Lookup()
}

object __Codec___Msg_Lookup : salvo.WireCodec<__Msg_Lookup> {
    override fun enc(v: __Msg_Lookup, out: salvo.WireOut) {
        when (v) {
            is __Msg_Lookup.Lookup -> { out.u8(0); salvo.StrCodec.enc(v.key, out); salvo.ReplyCodec.enc(v.out, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Lookup = when (inp.u8()) {
            0 -> __Msg_Lookup.Lookup(salvo.StrCodec.dec(inp), salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Lookup`. */
const val __PROTO_Lookup: String = "7c0f441570dc9a6f"

class Sequencing(private val who: String) : Sequencer {
    private var n: Int = 0
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Sequencing> = mutableMapOf()

    override fun next(out: salvo.SalvoReply) {
        n = n + 1
        salvo.SalvoSched.replyWire(out, "$who#$n", salvo.StrCodec)
    }
}

sealed class __Cont_Sequencing {
    class Next() : __Cont_Sequencing()
}

class __Actor_Sequencing(private val handler: Sequencing) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Sequencer)
    }

    private fun __dispatch(m: __Msg_Sequencer) {
        when (m) {
            is __Msg_Sequencer.Next -> handler.next(m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Sequencing.Next -> handler.next(value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Sequencing.Next -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Sequencer -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Sequencer)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Stocking(private val shard: String) : Inventory {
    private var served: Int = 0
    internal val __mailboxCapacity: Int = 32
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Stocking> = mutableMapOf()

    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        served = served + qty
        salvo.SalvoSched.replyWire(out, "$shard:$served", salvo.StrCodec)
    }
}

sealed class __Cont_Stocking {
    class Reserve(val sku: String, val qty: Int) : __Cont_Stocking()
}

class __Actor_Stocking(private val handler: Stocking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Inventory)
    }

    private fun __dispatch(m: __Msg_Inventory) {
        when (m) {
            is __Msg_Inventory.Reserve -> handler.reserve(m.sku, m.qty, m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Stocking.Reserve -> handler.reserve(c.sku, c.qty, value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Stocking.Reserve -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Inventory -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Inventory)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Indexing(private val words: List<String>) : Search {
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Indexing> = mutableMapOf()

    override fun query(word: String, out: salvo.SalvoReply) {
        var n = 0
        for (w in words) {
            if (((w) == (word))) {
                n = n + 1
            }
        }
        salvo.SalvoSched.replyWire(out, n, salvo.IntCodec)
    }
}

sealed class __Cont_Indexing {
    class Query(val word: String) : __Cont_Indexing()
}

class __Actor_Indexing(private val handler: Indexing) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Search)
    }

    private fun __dispatch(m: __Msg_Search) {
        when (m) {
            is __Msg_Search.Query -> handler.query(m.word, m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Indexing.Query -> handler.query(c.word, value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Indexing.Query -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Search -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Search)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Looking(private val who: String) : Lookup {
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Looking> = mutableMapOf()

    override fun lookup(key: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, "$key from $who", salvo.StrCodec)
    }
}

sealed class __Cont_Looking {
    class Lookup(val key: String) : __Cont_Looking()
}

class __Actor_Looking(private val handler: Looking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Lookup)
    }

    private fun __dispatch(m: __Msg_Lookup) {
        when (m) {
            is __Msg_Lookup.Lookup -> handler.lookup(m.key, m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Looking.Lookup -> handler.lookup(c.key, value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Looking.Lookup -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Lookup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Lookup)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class SlowLooking(private val who: String, private val timer: Int) : Lookup {
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_SlowLooking> = mutableMapOf()

    override fun lookup(key: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(timer, __Msg_Timer.After(millis(150L), run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_SlowLooking.Answer(key, out); __r }), __PROTO_Timer, __Codec___Msg_Timer)
    }

    fun answer(key: String, out: salvo.SalvoReply, fired: Fired) {
        salvo.SalvoSched.replyWire(out, "$key from $who", salvo.StrCodec)
    }
}

sealed class __Cont_SlowLooking {
    class Lookup(val key: String) : __Cont_SlowLooking()
    class Answer(val key: String, val out: salvo.SalvoReply) : __Cont_SlowLooking()
}

sealed class __Priv_SlowLooking {
    class Answer(val key: String, val out: salvo.SalvoReply, val fired: Fired) : __Priv_SlowLooking()
}

class __Actor_SlowLooking(private val handler: SlowLooking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Lookup -> __dispatchLookup(msg)
            is __Priv_SlowLooking -> __dispatchPriv(msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    private fun __dispatchLookup(m: __Msg_Lookup) {
        when (m) {
            is __Msg_Lookup.Lookup -> handler.lookup(m.key, m.out)
        }
    }

    private fun __dispatchPriv(m: __Priv_SlowLooking) {
        when (m) {
            is __Priv_SlowLooking.Answer -> handler.answer(m.key, m.out, m.fired)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_SlowLooking.Lookup -> handler.lookup(c.key, value as salvo.SalvoReply)
            is __Cont_SlowLooking.Answer -> handler.answer(c.key, c.out, value as Fired)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_SlowLooking.Lookup -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_SlowLooking.Answer -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Fired) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Lookup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Lookup)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Scattering(private val group: Int, private val gather: Int) : Search {
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Scattering> = mutableMapOf()

    override fun query(word: String, out: salvo.SalvoReply) {
        val members = run {
            val (ms, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
            salvo.SalvoSched.sendWire(group, __Msg_ActorGroup.Members(ms), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<Int>
        }
        salvo.SalvoSched.sendWire(gather, __Msg_Gather.Scatter(word, members, out), __PROTO_Gather, __Codec___Msg_Gather)
    }
}

sealed class __Cont_Scattering {
    class Query(val word: String) : __Cont_Scattering()
}

class __Actor_Scattering(private val handler: Scattering) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Search)
    }

    private fun __dispatch(m: __Msg_Search) {
        when (m) {
            is __Msg_Search.Query -> handler.query(m.word, m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Scattering.Query -> handler.query(c.word, value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Scattering.Query -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Search -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Search)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

interface Gather {
    fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply)
}

class __Stub_Gather(private val addr: Int) : Gather {
    override fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Gather.Scatter(word, members, out), __PROTO_Gather, __Codec___Msg_Gather)
    }
}

class __Mon_Gather(private val inner: Gather) : Gather {
    override fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.scatter(word, members, out) }
    }
}

sealed class __Msg_Gather {
    class Scatter(val word: String, val members: List<Int>, val out: salvo.SalvoReply) : __Msg_Gather()
}

object __Codec___Msg_Gather : salvo.WireCodec<__Msg_Gather> {
    override fun enc(v: __Msg_Gather, out: salvo.WireOut) {
        when (v) {
            is __Msg_Gather.Scatter -> { out.u8(0); salvo.StrCodec.enc(v.word, out); salvo.ListCodec(salvo.AddrCodec).enc(v.members, out); salvo.ReplyCodec.enc(v.out, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Gather = when (inp.u8()) {
            0 -> __Msg_Gather.Scatter(salvo.StrCodec.dec(inp), salvo.ListCodec(salvo.AddrCodec).dec(inp), salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Gather`. */
const val __PROTO_Gather: String = "98712ca205da344c"

class Gathering : Gather {
    private var pending: kotlin.collections.ArrayDeque<salvo.SalvoReply> = kotlin.collections.ArrayDeque<salvo.SalvoReply>(listOf<salvo.SalvoReply>())
    private var left: Int = 0
    private var total: Int = 0
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Gathering> = mutableMapOf()

    override fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply) {
        pending.addLast(out)
        left = members.size
        total = 0
        for (m in members) {
            salvo.SalvoSched.sendWire(m, __Msg_Search.Query(word, run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_Gathering.Partial(); __r }), __PROTO_Search, __Codec___Msg_Search)
        }
    }

    fun partial(n: Int) {
        total = total + n
        left = left - 1
        if (left == 0) {
            val out = pending.removeFirstOrNull()
            when {
                out != null -> {
                    salvo.SalvoSched.replyWire(out, total, salvo.IntCodec)
                }
                else -> {
                }
            }
        }
    }
}

sealed class __Cont_Gathering {
    class Scatter(val word: String, val members: List<Int>) : __Cont_Gathering()
    class Partial() : __Cont_Gathering()
}

sealed class __Priv_Gathering {
    class Partial(val n: Int) : __Priv_Gathering()
}

class __Actor_Gathering(private val handler: Gathering) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Gather -> __dispatchGather(msg)
            is __Priv_Gathering -> __dispatchPriv(msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    private fun __dispatchGather(m: __Msg_Gather) {
        when (m) {
            is __Msg_Gather.Scatter -> handler.scatter(m.word, m.members, m.out)
        }
    }

    private fun __dispatchPriv(m: __Priv_Gathering) {
        when (m) {
            is __Priv_Gathering.Partial -> handler.partial(m.n)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Gathering.Scatter -> handler.scatter(c.word, c.members, value as salvo.SalvoReply)
            is __Cont_Gathering.Partial -> handler.partial(value as Int)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Gathering.Scatter -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Gathering.Partial -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Gather -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Gather)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Hedging(private val group: Int, private val racer: Int) : Lookup {
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Hedging> = mutableMapOf()

    override fun lookup(key: String, out: salvo.SalvoReply) {
        val members = run {
            val (ms, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
            salvo.SalvoSched.sendWire(group, __Msg_ActorGroup.Members(ms), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<Int>
        }
        salvo.SalvoSched.sendWire(racer, __Msg_Race.Race(key, members, out), __PROTO_Race, __Codec___Msg_Race)
    }
}

sealed class __Cont_Hedging {
    class Lookup(val key: String) : __Cont_Hedging()
}

class __Actor_Hedging(private val handler: Hedging) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Lookup)
    }

    private fun __dispatch(m: __Msg_Lookup) {
        when (m) {
            is __Msg_Lookup.Lookup -> handler.lookup(m.key, m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Hedging.Lookup -> handler.lookup(c.key, value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Hedging.Lookup -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Lookup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Lookup)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

interface Race {
    fun race(key: String, members: List<Int>, out: salvo.SalvoReply)
}

class __Stub_Race(private val addr: Int) : Race {
    override fun race(key: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Race.Race(key, members, out), __PROTO_Race, __Codec___Msg_Race)
    }
}

class __Mon_Race(private val inner: Race) : Race {
    override fun race(key: String, members: List<Int>, out: salvo.SalvoReply) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.race(key, members, out) }
    }
}

sealed class __Msg_Race {
    class Race(val key: String, val members: List<Int>, val out: salvo.SalvoReply) : __Msg_Race()
}

object __Codec___Msg_Race : salvo.WireCodec<__Msg_Race> {
    override fun enc(v: __Msg_Race, out: salvo.WireOut) {
        when (v) {
            is __Msg_Race.Race -> { out.u8(0); salvo.StrCodec.enc(v.key, out); salvo.ListCodec(salvo.AddrCodec).enc(v.members, out); salvo.ReplyCodec.enc(v.out, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Race = when (inp.u8()) {
            0 -> __Msg_Race.Race(salvo.StrCodec.dec(inp), salvo.ListCodec(salvo.AddrCodec).dec(inp), salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Race`. */
const val __PROTO_Race: String = "5e0ec4d63d5f53dd"

class Racing : Race {
    private var pending: kotlin.collections.ArrayDeque<salvo.SalvoReply> = kotlin.collections.ArrayDeque<salvo.SalvoReply>(listOf<salvo.SalvoReply>())
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Racing> = mutableMapOf()

    override fun race(key: String, members: List<Int>, out: salvo.SalvoReply) {
        pending.addLast(out)
        for (m in members) {
            salvo.SalvoSched.sendWire(m, __Msg_Lookup.Lookup(key, run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_Racing.First(); __r }), __PROTO_Lookup, __Codec___Msg_Lookup)
        }
    }

    fun first(answer: String) {
        val out = pending.removeFirstOrNull()
        when {
            out != null -> {
                salvo.SalvoSched.replyWire(out, answer, salvo.StrCodec)
            }
            else -> {
                (answer).let {}
            }
        }
    }
}

sealed class __Cont_Racing {
    class Race(val key: String, val members: List<Int>) : __Cont_Racing()
    class First() : __Cont_Racing()
}

sealed class __Priv_Racing {
    class First(val answer: String) : __Priv_Racing()
}

class __Actor_Racing(private val handler: Racing) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Race -> __dispatchRace(msg)
            is __Priv_Racing -> __dispatchPriv(msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    private fun __dispatchRace(m: __Msg_Race) {
        when (m) {
            is __Msg_Race.Race -> handler.race(m.key, m.members, m.out)
        }
    }

    private fun __dispatchPriv(m: __Priv_Racing) {
        when (m) {
            is __Priv_Racing.First -> handler.first(m.answer)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Racing.Race -> handler.race(c.key, c.members, value as salvo.SalvoReply)
            is __Cont_Racing.First -> handler.first(value as String)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Racing.Race -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Racing.First -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Race -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Race)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class LastHost(private val nodes: Int, private val me: NodeEndpoint) : Leader {

    override fun leader(): NodeId? {
        val peers = run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(__Codec_Node)) })
            salvo.SalvoSched.sendWire(nodes, __Msg_NodeGroup.Members(out), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<Node>
        }
        var bestHost = me.host
        var best = NodeId(salvo.SalvoSched.hereNode())
        for (n in peers) {
            if (salvo.__salvoCompare(n.at.host, bestHost) > 0) {
                bestHost = n.at.host
                best = n.id
            }
        }
        return best
    }
}

fun freshId(sequencer: Sequencer): String {
    return run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        sequencer.next(out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
}

fun checkout(inventory: Inventory, console: Console, skus: List<String>) {
    val shards: MutableList<String> = mutableListOf<String>()
    for (sku in skus) {
        val answer = run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
            inventory.reserve(sku, 1, out)
            salvo.SalvoSched.awaitReply(__wid) as String
        }
        val parts = answer.split(":")
        shards.add((parts.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:217:26")))
        println(console, "  $sku: ${(parts.getOrNull(1) ?: throw AssertionError("salvo: value is absent at main:218:30"))} reserved on its shard so far")
    }
    println(console, "  apple and apple on one shard: ${(((shards.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:220:51"))) == ((shards.getOrNull(2) ?: throw AssertionError("salvo: value is absent at main:220:68"))))}")
    println(console, "  apple and fig on one shard: ${(((shards.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:221:49"))) == ((shards.getOrNull(3) ?: throw AssertionError("salvo: value is absent at main:221:66"))))}")
    println(console, "  apple and pear on one shard: ${(((shards.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:222:50"))) == ((shards.getOrNull(1) ?: throw AssertionError("salvo: value is absent at main:222:67"))))}")
}

fun count(search: Search, word: String): Int {
    return run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        search.query(word, out)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
}

fun find(lookup: Lookup, key: String): String {
    return run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        lookup.lookup(key, out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
}

fun twoIds(leader: Leader, console: Console, seq: Int) {
    val pick: Pick = Elected(leader)
    val sequencer: Sequencer = __Route_Sequencer(seq, pick)
    println(console, "  ${freshId(sequencer)} ${freshId(sequencer)}")
}

fun shop(console: Console, stock: Int) {
    val pick: Pick = Sharded()
    val inventory: Inventory = __Route_Inventory(stock, pick)
    checkout(inventory, console, listOf<String>("apple", "pear", "apple", "fig", "pear"))
}

interface Boot {
    fun boot(done: salvo.SalvoReply)
    fun stop(done: salvo.SalvoReply)
}

class __Stub_Boot(private val addr: Int) : Boot {
    override fun boot(done: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Boot.Boot(done), __PROTO_Boot, __Codec___Msg_Boot)
    }
    override fun stop(done: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Boot.Stop(done), __PROTO_Boot, __Codec___Msg_Boot)
    }
}

class __Mon_Boot(private val inner: Boot) : Boot {
    override fun boot(done: salvo.SalvoReply) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.boot(done) }
    }
    override fun stop(done: salvo.SalvoReply) {
        check(!Thread.holdsLock(inner)) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        synchronized(inner) { inner.stop(done) }
    }
}

sealed class __Msg_Boot {
    class Boot(val done: salvo.SalvoReply) : __Msg_Boot()
    class Stop(val done: salvo.SalvoReply) : __Msg_Boot()
}

object __Codec___Msg_Boot : salvo.WireCodec<__Msg_Boot> {
    override fun enc(v: __Msg_Boot, out: salvo.WireOut) {
        when (v) {
            is __Msg_Boot.Boot -> { out.u8(0); salvo.ReplyCodec.enc(v.done, out) }
            is __Msg_Boot.Stop -> { out.u8(1); salvo.ReplyCodec.enc(v.done, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Boot = when (inp.u8()) {
            0 -> __Msg_Boot.Boot(salvo.ReplyCodec.dec(inp))
            1 -> __Msg_Boot.Stop(salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `Boot`. */
const val __PROTO_Boot: String = "4b15e647d0ca92a7"

class Booting(private val at: NodeEndpoint, private val all: List<NodeEndpoint>, private val net: Int, private val __dep_Transport: Transport) : Boot {
    private var nodes: Int? = null
    internal val __mailboxCapacity: Int = 2
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Booting> = mutableMapOf()

    override fun boot(done: salvo.SalvoReply) {
        val p = salvo.SalvoSched.pool(1)
        val group = run { val __h = StaticNodeGroup("cluster", all, __dep_Transport); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_StaticNodeGroup(__h), __Actor_StaticNodeGroup.__DECODE); salvo.SalvoSched.send(__a, __Priv_StaticNodeGroup.Init); __a }
        nodes = group
        val seq = actorGroup(group, { Protocol("Sequencer", salvo.main.__PROTO_Sequencer) })
        val mine = run { val __spawned = run { val __h = Sequencing("b"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sequencing(__h), __Actor_Sequencing.__DECODE); __a }; salvo.SalvoSched.sendWire(seq, salvo.net.__Msg_ActorGroup.Join(__spawned), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __spawned }
        val stock = actorGroup(group, { Protocol("Inventory", salvo.main.__PROTO_Inventory) })
        run { val __spawned = run { val __h = Stocking("shard-b"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Stocking(__h), __Actor_Stocking.__DECODE); __a }; salvo.SalvoSched.sendWire(stock, salvo.net.__Msg_ActorGroup.Join(__spawned), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __spawned }
        val index = actorGroup(group, { Protocol("Search", salvo.main.__PROTO_Search) })
        run { val __spawned = run { val __h = Indexing(listOf<String>("salvo", "actors", "salvo", "nodes")); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Indexing(__h), __Actor_Indexing.__DECODE); __a }; salvo.SalvoSched.sendWire(index, salvo.net.__Msg_ActorGroup.Join(__spawned), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __spawned }
        val looks = actorGroup(group, { Protocol("Lookup", salvo.main.__PROTO_Lookup) })
        val timer = run { val __h = DefaultTimer(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_DefaultTimer(__h), __Actor_DefaultTimer.__DECODE); __a }
        run { val __spawned = run { val __h = SlowLooking("b (slow)", timer); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_SlowLooking(__h), __Actor_SlowLooking.__DECODE); __a }; salvo.SalvoSched.sendWire(looks, salvo.net.__Msg_ActorGroup.Join(__spawned), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __spawned }
        salvo.SalvoSched.replyWire(done, mine, salvo.AddrCodec)
    }

    override fun stop(done: salvo.SalvoReply) {
        val group = nodes
        if (!(group == null)) {
            salvo.SalvoSched.sendWire(group, __Msg_NodeGroup.Leave(), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
        }
        salvo.SalvoSched.replyWire(done, true, salvo.BoolCodec)
    }
}

sealed class __Cont_Booting {
    class Boot() : __Cont_Booting()
    class Stop() : __Cont_Booting()
}

class __Actor_Booting(private val handler: Booting) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Boot)
    }

    private fun __dispatch(m: __Msg_Boot) {
        when (m) {
            is __Msg_Boot.Boot -> handler.boot(m.done)
            is __Msg_Boot.Stop -> handler.stop(m.done)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Booting.Boot -> handler.boot(value as salvo.SalvoReply)
            is __Cont_Booting.Stop -> handler.stop(value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Booting.Boot -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_Booting.Stop -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Boot -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Boot)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

fun settle(timer: Int) {
    val _f = run {
        val (f, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Fired) })
        salvo.SalvoSched.sendWire(timer, __Msg_Timer.After(millis(400L), f), __PROTO_Timer, __Codec___Msg_Timer)
        salvo.SalvoSched.awaitReply(__wid) as Fired
    }
}

fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("ActorGroup", salvo.net.__PROTO_ActorGroup), Pair("ActorGroupWatcher", salvo.net.__PROTO_ActorGroupWatcher), Pair("Boot", salvo.main.__PROTO_Boot), Pair("Faults", salvo.core.actor.__PROTO_Faults), Pair("Gather", salvo.main.__PROTO_Gather), Pair("Inbound", salvo.net.__PROTO_Inbound), Pair("Inventory", salvo.main.__PROTO_Inventory), Pair("Lookup", salvo.main.__PROTO_Lookup), Pair("MemNet", salvo.net.__PROTO_MemNet), Pair("NodeGroup", salvo.net.__PROTO_NodeGroup), Pair("NodeGroupWatcher", salvo.net.__PROTO_NodeGroupWatcher), Pair("Outbound", salvo.net.__PROTO_Outbound), Pair("Race", salvo.main.__PROTO_Race), Pair("Search", salvo.main.__PROTO_Search), Pair("Sequencer", salvo.main.__PROTO_Sequencer), Pair("Timer", salvo.time.__PROTO_Timer), Pair("TimerCtl", salvo.time.__PROTO_TimerCtl), Pair("Wheel", salvo.runtime.timers.__PROTO_Wheel)))
    val console: Console = StdOutConsole()
    val a = NodeEndpoint(host = "a", port = 1)
    val b = NodeEndpoint(host = "b", port = 1)
    val all = listOf<NodeEndpoint>(a, b)
    val network = run { val __h = MemNetwork(); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.pool(1), __h.__mailboxCapacity, __Actor_MemNetwork(__h), __Actor_MemNetwork.__DECODE); __a }
    val transport: Transport = MemTransport(a, network)
    val p = salvo.SalvoSched.pool(2)
    val nodes = run { val __h = StaticNodeGroup("cluster", all, transport); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_StaticNodeGroup(__h), __Actor_StaticNodeGroup.__DECODE); salvo.SalvoSched.send(__a, __Priv_StaticNodeGroup.Init); __a }
    val seq = actorGroup(nodes, { Protocol("Sequencer", salvo.main.__PROTO_Sequencer) })
    run { val __spawned = run { val __h = Sequencing("a"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sequencing(__h), __Actor_Sequencing.__DECODE); __a }; salvo.SalvoSched.sendWire(seq, salvo.net.__Msg_ActorGroup.Join(__spawned), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __spawned }
    val stock = actorGroup(nodes, { Protocol("Inventory", salvo.main.__PROTO_Inventory) })
    run { val __spawned = run { val __h = Stocking("shard-a"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Stocking(__h), __Actor_Stocking.__DECODE); __a }; salvo.SalvoSched.sendWire(stock, salvo.net.__Msg_ActorGroup.Join(__spawned), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __spawned }
    val index = actorGroup(nodes, { Protocol("Search", salvo.main.__PROTO_Search) })
    run { val __spawned = run { val __h = Indexing(listOf<String>("salvo", "is", "salvo")); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Indexing(__h), __Actor_Indexing.__DECODE); __a }; salvo.SalvoSched.sendWire(index, salvo.net.__Msg_ActorGroup.Join(__spawned), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __spawned }
    val looks = actorGroup(nodes, { Protocol("Lookup", salvo.main.__PROTO_Lookup) })
    run { val __spawned = run { val __h = Looking("a"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Looking(__h), __Actor_Looking.__DECODE); __a }; salvo.SalvoSched.sendWire(looks, salvo.net.__Msg_ActorGroup.Join(__spawned), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __spawned }
    val pb = salvo.SalvoSched.poolAt((NodeId(salvo.SalvoSched.newNode())).id, 1)
    val booter = run { val __h = Booting(b, all, network, MemTransport(b, network)); val __a = salvo.SalvoSched.spawn(pb, __h.__mailboxCapacity, __Actor_Booting(__h), __Actor_Booting.__DECODE); __a }
    val remoteSeq = run {
        val (done, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })
        salvo.SalvoSched.sendWire(booter, __Msg_Boot.Boot(done), __PROTO_Boot, __Codec___Msg_Boot)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    val timer = run { val __h = DefaultTimer(); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.pool(1), __h.__mailboxCapacity, __Actor_DefaultTimer(__h), __Actor_DefaultTimer.__DECODE); __a }
    settle(timer)
    val members = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(__Codec_Node)) })
        salvo.SalvoSched.sendWire(nodes, __Msg_NodeGroup.Members(out), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Node>
    }
    println(console, "nodes: ${members.size + 1}, sequencers: ${run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
        salvo.SalvoSched.sendWire(seq, __Msg_ActorGroup.Members(out), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Int>
    }.size}")
    println(console, "singleton (b's sequencer is remote: ${!eq__2(NodeId(salvo.SalvoSched.addrIdentity(remoteSeq).node), NodeId(salvo.SalvoSched.hereNode()))}):")
    val leader: Leader = LastHost(nodes, a)
    twoIds(leader, console, seq)
    println(console, "sharded:")
    shop(console, stock)
    println(console, "scatter:")
    val search: Search = Scattering(index, run { val __h = Gathering(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Gathering(__h), __Actor_Gathering.__DECODE); __a })
    println(console, "  salvo: ${count(search, "salvo")}, actors: ${count(search, "actors")}, none: ${count(search, "none")}")
    println(console, "hedge:")
    val lookup: Lookup = Hedging(looks, run { val __h = Racing(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Racing(__h), __Actor_Racing.__DECODE); __a })
    println(console, "  ${find(lookup, "k1")}")
    val _stopped = run {
        val (done, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.BoolCodec) })
        salvo.SalvoSched.sendWire(booter, __Msg_Boot.Stop(done), __PROTO_Boot, __Codec___Msg_Boot)
        salvo.SalvoSched.awaitReply(__wid) as Boolean
    }
    settle(timer)
    println(console, "after b left:")
    println(console, "  sequencers: ${run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
        salvo.SalvoSched.sendWire(seq, __Msg_ActorGroup.Members(out), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Int>
    }.size}")
    twoIds(leader, console, seq)
}

class __Route_Inventory(private val group: Int, private val __dep_Pick: Pick) : Inventory {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont___Route_Inventory> = mutableMapOf()

    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        val __target = routeTo__2(__dep_Pick, group, salvo.SalvoSched.keyHash(salvo.salvoEncode(sku, salvo.StrCodec).toByteArray()))
        salvo.SalvoSched.sendWire(__target, __Msg_Inventory.Reserve(sku, qty, out), __PROTO_Inventory, __Codec___Msg_Inventory)
    }
}

sealed class __Cont___Route_Inventory {
    class Reserve(val sku: String, val qty: Int) : __Cont___Route_Inventory()
}

class __Actor___Route_Inventory(private val handler: __Route_Inventory) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Inventory)
    }

    private fun __dispatch(m: __Msg_Inventory) {
        when (m) {
            is __Msg_Inventory.Reserve -> handler.reserve(m.sku, m.qty, m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont___Route_Inventory.Reserve -> handler.reserve(c.sku, c.qty, value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont___Route_Inventory.Reserve -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Inventory -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Inventory)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class __Route_Lookup(private val group: Int, private val __dep_Pick: Pick) : Lookup {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont___Route_Lookup> = mutableMapOf()

    override fun lookup(key: String, out: salvo.SalvoReply) {
        val __target = routeTo(__dep_Pick, group)
        salvo.SalvoSched.sendWire(__target, __Msg_Lookup.Lookup(key, out), __PROTO_Lookup, __Codec___Msg_Lookup)
    }
}

sealed class __Cont___Route_Lookup {
    class Lookup(val key: String) : __Cont___Route_Lookup()
}

class __Actor___Route_Lookup(private val handler: __Route_Lookup) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Lookup)
    }

    private fun __dispatch(m: __Msg_Lookup) {
        when (m) {
            is __Msg_Lookup.Lookup -> handler.lookup(m.key, m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont___Route_Lookup.Lookup -> handler.lookup(c.key, value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont___Route_Lookup.Lookup -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Lookup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Lookup)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class __Route_Search(private val group: Int, private val __dep_Pick: Pick) : Search {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont___Route_Search> = mutableMapOf()

    override fun query(word: String, out: salvo.SalvoReply) {
        val __target = routeTo(__dep_Pick, group)
        salvo.SalvoSched.sendWire(__target, __Msg_Search.Query(word, out), __PROTO_Search, __Codec___Msg_Search)
    }
}

sealed class __Cont___Route_Search {
    class Query(val word: String) : __Cont___Route_Search()
}

class __Actor___Route_Search(private val handler: __Route_Search) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Search)
    }

    private fun __dispatch(m: __Msg_Search) {
        when (m) {
            is __Msg_Search.Query -> handler.query(m.word, m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont___Route_Search.Query -> handler.query(c.word, value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont___Route_Search.Query -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Search -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Search)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class __Route_Sequencer(private val group: Int, private val __dep_Pick: Pick) : Sequencer {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont___Route_Sequencer> = mutableMapOf()

    override fun next(out: salvo.SalvoReply) {
        val __target = routeTo(__dep_Pick, group)
        salvo.SalvoSched.sendWire(__target, __Msg_Sequencer.Next(out), __PROTO_Sequencer, __Codec___Msg_Sequencer)
    }
}

sealed class __Cont___Route_Sequencer {
    class Next() : __Cont___Route_Sequencer()
}

class __Actor___Route_Sequencer(private val handler: __Route_Sequencer) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Sequencer)
    }

    private fun __dispatch(m: __Msg_Sequencer) {
        when (m) {
            is __Msg_Sequencer.Next -> handler.next(m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont___Route_Sequencer.Next -> handler.next(value as salvo.SalvoReply)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont___Route_Sequencer.Next -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Sequencer -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Sequencer)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}
