package salvo.main

import salvo.*
import salvo.core.actor.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.console.*
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
    override fun next(out: salvo.SalvoReply) =
        synchronized(inner) { inner.next(out) }
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
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) =
        synchronized(inner) { inner.reserve(sku, qty, out) }
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
    override fun query(word: String, out: salvo.SalvoReply) =
        synchronized(inner) { inner.query(word, out) }
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
    override fun lookup(key: String, out: salvo.SalvoReply) =
        synchronized(inner) { inner.lookup(key, out) }
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
    private var pending: MutableList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
    private var left: Int = 0
    private var total: Int = 0
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Gathering> = mutableMapOf()

    override fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply) {
        pending.add(out)
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
            val out = (pending).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
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
    private var pending: MutableList<salvo.SalvoReply> = mutableListOf<salvo.SalvoReply>()
    internal val __mailboxCapacity: Int = 16
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Racing> = mutableMapOf()

    override fun race(key: String, members: List<Int>, out: salvo.SalvoReply) {
        pending.add(out)
        for (m in members) {
            salvo.SalvoSched.sendWire(m, __Msg_Lookup.Lookup(key, run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!);              __parked[__s] = __Cont_Racing.First(); __r }), __PROTO_Lookup, __Codec___Msg_Lookup)
        }
    }

    fun first(answer: String) {
        val out = (pending).let { __l -> if (__l.isEmpty()) null else __l.removeAt(0) }
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

    override fun leader(): Long? {
        val peers = run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(__Codec_Node)) })
            salvo.SalvoSched.sendWire(nodes, __Msg_NodeGroup.Members(out), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<Node>
        }
        var best_host = me.host
        var best = salvo.SalvoSched.hereNode()
        for (n in peers) {
            if (salvo.__salvoCompare(n.at.host, best_host) > 0) {
                best_host = n.at.host
                best = n.id
            }
        }
        return best
    }
}

fun<__Fx> fresh_id(__fx: __Fx): String where __Fx : __Has_Sequencer {
    return run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        __fx.__fx_Sequencer.next(out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
}

fun<__Fx> checkout(__fx: __Fx, skus: List<String>) where __Fx : __Has_Inventory, __Fx : __Has_Console {
    val shards: MutableList<String> = mutableListOf<String>()
    for (sku in skus) {
        val answer = run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
            __fx.__fx_Inventory.reserve(sku, 1, out)
            salvo.SalvoSched.awaitReply(__wid) as String
        }
        val parts = answer.split(":")
        shards.add((parts.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:217:26")))
        println(__fx, "  $sku: ${(parts.getOrNull(1) ?: throw AssertionError("salvo: value is absent at main:218:30"))} reserved on its shard so far")
    }
    println(__fx, "  apple and apple on one shard: ${(((shards.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:220:51"))) == ((shards.getOrNull(2) ?: throw AssertionError("salvo: value is absent at main:220:68"))))}")
    println(__fx, "  apple and fig on one shard: ${(((shards.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:221:49"))) == ((shards.getOrNull(3) ?: throw AssertionError("salvo: value is absent at main:221:66"))))}")
    println(__fx, "  apple and pear on one shard: ${(((shards.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:222:50"))) == ((shards.getOrNull(1) ?: throw AssertionError("salvo: value is absent at main:222:67"))))}")
}

fun<__Fx> count(__fx: __Fx, word: String): Int where __Fx : __Has_Search {
    return run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        __fx.__fx_Search.query(word, out)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
}

fun<__Fx> find(__fx: __Fx, key: String): String where __Fx : __Has_Lookup {
    return run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        __fx.__fx_Lookup.lookup(key, out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
}

fun<__Fx> two_ids(__fx: __Fx, seq: Int) where __Fx : __Has_Leader, __Fx : __Has_Console {
    val __fx2 = __Fx_1(__fx.__fx_Console, __fx.__fx_Leader, Elected(__fx))
    val __fx3 = __Fx_3(__fx2.__fx_Console, __fx2.__fx_Leader, __fx2.__fx_Pick, __Route_Sequencer(seq, __Fx_2(__fx2.__fx_Pick)))
    println(__fx3, "  ${fresh_id(__fx3)} ${fresh_id(__fx3)}")
}

fun<__Fx> shop(__fx: __Fx, stock: Int) where __Fx : __Has_Console {
    val __fx2 = __Fx_4(__fx.__fx_Console, Sharded())
    val __fx3 = __Fx_5(__fx2.__fx_Console, __Route_Inventory(stock, __Fx_2(__fx2.__fx_Pick)), __fx2.__fx_Pick)
    checkout(__fx3, listOf<String>("apple", "pear", "apple", "fig", "pear"))
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

class Booting<__Fx>(private val at: NodeEndpoint, private val all: List<NodeEndpoint>, private val net: Int, private val __fx: __Fx) : Boot where __Fx : __Has_Transport {
    private var nodes: Int? = null
    internal val __mailboxCapacity: Int = 2
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Booting> = mutableMapOf()

    override fun boot(done: salvo.SalvoReply) {
        val p = salvo.SalvoSched.pool(1)
        run { val __out = run { val __h = Sending(__Fx_6(MemTransport(at, net))); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sending(__h), __Actor_Sending.__DECODE) }; salvo.SalvoSched.setWire { __ep, __frame -> val __to = salvo.salvoDecode(salvo.SalvoBytes(__ep), __Codec_NodeEndpoint); if (__to != null) salvo.SalvoSched.sendWire(__out, __Msg_Outbound.Frame(__to, salvo.SalvoBytes(__frame)), __PROTO_Outbound, __Codec___Msg_Outbound) } }
        __fx.__fx_Transport.listen(at, run { val __h = Receiving(); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Receiving(__h), __Actor_Receiving.__DECODE) })
        salvo.SalvoSched.addRoute(salvo.SalvoSched.hereNode(), salvo.salvoEncode(at, __Codec_NodeEndpoint).toByteArray())
        val group = start_group(run { val __h = StaticNodeGroup("cluster", at, all, __Fx_6(MemTransport(at, net))); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_StaticNodeGroup(__h), __Actor_StaticNodeGroup.__DECODE); Pair(__a, __a) })
        nodes = group
        val seq = attach__2(Protocol("Sequencer", salvo.main.__PROTO_Sequencer), group)
        val mine = run { val __h = Sequencing("b"); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sequencing(__h), __Actor_Sequencing.__DECODE) }
        join(seq, mine)
        val stock = attach__2(Protocol("Inventory", salvo.main.__PROTO_Inventory), group)
        join(stock, run { val __h = Stocking("shard-b"); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Stocking(__h), __Actor_Stocking.__DECODE) })
        val index = attach__2(Protocol("Search", salvo.main.__PROTO_Search), group)
        join(index, run { val __h = Indexing(listOf<String>("salvo", "actors", "salvo", "nodes")); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Indexing(__h), __Actor_Indexing.__DECODE) })
        val looks = attach__2(Protocol("Lookup", salvo.main.__PROTO_Lookup), group)
        val timer = run { val __h = DefaultTimer(); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_DefaultTimer(__h), __Actor_DefaultTimer.__DECODE) }
        join(looks, run { val __h = SlowLooking("b (slow)", timer); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_SlowLooking(__h), __Actor_SlowLooking.__DECODE) })
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

class __Actor_Booting<__Fx>(private val handler: Booting<__Fx>) : salvo.SalvoActor where __Fx : __Has_Transport {
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
    salvo.SalvoSched.setProtocols(listOf(Pair("ActorChanges", salvo.net.__PROTO_ActorChanges), Pair("ActorGroup", salvo.net.__PROTO_ActorGroup), Pair("Boot", salvo.main.__PROTO_Boot), Pair("Faults", salvo.core.actor.__PROTO_Faults), Pair("Gather", salvo.main.__PROTO_Gather), Pair("Inbound", salvo.net.__PROTO_Inbound), Pair("Inventory", salvo.main.__PROTO_Inventory), Pair("Lookup", salvo.main.__PROTO_Lookup), Pair("MemNet", salvo.net.__PROTO_MemNet), Pair("NodeChanges", salvo.net.__PROTO_NodeChanges), Pair("NodeGroup", salvo.net.__PROTO_NodeGroup), Pair("Outbound", salvo.net.__PROTO_Outbound), Pair("PeerEvents", salvo.net.__PROTO_PeerEvents), Pair("Race", salvo.main.__PROTO_Race), Pair("Search", salvo.main.__PROTO_Search), Pair("Sequencer", salvo.main.__PROTO_Sequencer), Pair("Timer", salvo.time.__PROTO_Timer), Pair("TimerCtl", salvo.time.__PROTO_TimerCtl)))
    val __fx = __Fx_7(StdOutConsole())
    val a = NodeEndpoint(host = "a", port = 1)
    val b = NodeEndpoint(host = "b", port = 1)
    val all = listOf<NodeEndpoint>(a, b)
    val network = run { val __h = MemNetwork(); salvo.SalvoSched.spawn(salvo.SalvoSched.pool(1), __h.__mailboxCapacity, __Actor_MemNetwork(__h), __Actor_MemNetwork.__DECODE) }
    val __fx2 = __Fx_8(__fx.__fx_Console, MemTransport(a, network))
    val p = salvo.SalvoSched.pool(2)
    run { val __out = run { val __h = Sending(__Fx_6(__fx2.__fx_Transport)); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sending(__h), __Actor_Sending.__DECODE) }; salvo.SalvoSched.setWire { __ep, __frame -> val __to = salvo.salvoDecode(salvo.SalvoBytes(__ep), __Codec_NodeEndpoint); if (__to != null) salvo.SalvoSched.sendWire(__out, __Msg_Outbound.Frame(__to, salvo.SalvoBytes(__frame)), __PROTO_Outbound, __Codec___Msg_Outbound) } }
    __fx2.__fx_Transport.listen(a, run { val __h = Receiving(); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Receiving(__h), __Actor_Receiving.__DECODE) })
    salvo.SalvoSched.addRoute(salvo.SalvoSched.hereNode(), salvo.salvoEncode(a, __Codec_NodeEndpoint).toByteArray())
    val nodes = start_group(run { val __h = StaticNodeGroup("cluster", a, all, __Fx_6(__fx2.__fx_Transport)); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_StaticNodeGroup(__h), __Actor_StaticNodeGroup.__DECODE); Pair(__a, __a) })
    val seq = attach__2(Protocol("Sequencer", salvo.main.__PROTO_Sequencer), nodes)
    join(seq, run { val __h = Sequencing("a"); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sequencing(__h), __Actor_Sequencing.__DECODE) })
    val stock = attach__2(Protocol("Inventory", salvo.main.__PROTO_Inventory), nodes)
    join(stock, run { val __h = Stocking("shard-a"); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Stocking(__h), __Actor_Stocking.__DECODE) })
    val index = attach__2(Protocol("Search", salvo.main.__PROTO_Search), nodes)
    join(index, run { val __h = Indexing(listOf<String>("salvo", "is", "salvo")); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Indexing(__h), __Actor_Indexing.__DECODE) })
    val looks = attach__2(Protocol("Lookup", salvo.main.__PROTO_Lookup), nodes)
    join(looks, run { val __h = Looking("a"); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Looking(__h), __Actor_Looking.__DECODE) })
    val pb = salvo.SalvoSched.poolAt(salvo.SalvoSched.newNode(), 1)
    val booter = run { val __h = Booting(b, all, network, __Fx_6(MemTransport(b, network))); salvo.SalvoSched.spawn(pb, __h.__mailboxCapacity, __Actor_Booting(__h), __Actor_Booting.__DECODE) }
    val remote_seq = run {
        val (done, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })
        salvo.SalvoSched.sendWire(booter, __Msg_Boot.Boot(done), __PROTO_Boot, __Codec___Msg_Boot)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    val timer = run { val __h = DefaultTimer(); salvo.SalvoSched.spawn(salvo.SalvoSched.pool(1), __h.__mailboxCapacity, __Actor_DefaultTimer(__h), __Actor_DefaultTimer.__DECODE) }
    settle(timer)
    val members = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(__Codec_Node)) })
        salvo.SalvoSched.sendWire(nodes, __Msg_NodeGroup.Members(out), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Node>
    }
    println(__fx2, "nodes: ${members.size + 1}, sequencers: ${run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
        salvo.SalvoSched.sendWire(seq, __Msg_ActorGroup.Members(out), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Int>
    }.size}")
    println(__fx2, "singleton (b's sequencer is remote: ${!((salvo.SalvoSched.addrIdentity(remote_seq).node) == (salvo.SalvoSched.hereNode()))}):")
    val __fx3 = __Fx_9(__fx2.__fx_Console, LastHost(nodes, a), __fx2.__fx_Transport)
    two_ids(__fx3, seq)
    println(__fx3, "sharded:")
    shop(__fx3, stock)
    println(__fx3, "scatter:")
    val __fx4 = __Fx_10(__fx3.__fx_Console, __fx3.__fx_Leader, Scattering(index, run { val __h = Gathering(); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Gathering(__h), __Actor_Gathering.__DECODE) }), __fx3.__fx_Transport)
    println(__fx4, "  salvo: ${count(__fx4, "salvo")}, actors: ${count(__fx4, "actors")}, none: ${count(__fx4, "none")}")
    println(__fx4, "hedge:")
    val __fx5 = __Fx_11(__fx4.__fx_Console, __fx4.__fx_Leader, Hedging(looks, run { val __h = Racing(); salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Racing(__h), __Actor_Racing.__DECODE) }), __fx4.__fx_Search, __fx4.__fx_Transport)
    println(__fx5, "  ${find(__fx5, "k1")}")
    val _stopped = run {
        val (done, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.BoolCodec) })
        salvo.SalvoSched.sendWire(booter, __Msg_Boot.Stop(done), __PROTO_Boot, __Codec___Msg_Boot)
        salvo.SalvoSched.awaitReply(__wid) as Boolean
    }
    settle(timer)
    println(__fx5, "after b left:")
    println(__fx5, "  sequencers: ${run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
        salvo.SalvoSched.sendWire(seq, __Msg_ActorGroup.Members(out), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Int>
    }.size}")
    two_ids(__fx5, seq)
}

class __Route_Inventory<__Fx>(private val group: Int, private val __fx: __Fx) : Inventory where __Fx : __Has_Pick {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont___Route_Inventory> = mutableMapOf()

    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        val __target = route_to__2(__fx, group, salvo.SalvoSched.keyHash(salvo.salvoEncode(sku, salvo.StrCodec).toByteArray()))
        salvo.SalvoSched.sendWire(__target, __Msg_Inventory.Reserve(sku, qty, out), __PROTO_Inventory, __Codec___Msg_Inventory)
    }
}

sealed class __Cont___Route_Inventory {
    class Reserve(val sku: String, val qty: Int) : __Cont___Route_Inventory()
}

class __Actor___Route_Inventory<__Fx>(private val handler: __Route_Inventory<__Fx>) : salvo.SalvoActor where __Fx : __Has_Pick {
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

class __Route_Lookup<__Fx>(private val group: Int, private val __fx: __Fx) : Lookup where __Fx : __Has_Pick {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont___Route_Lookup> = mutableMapOf()

    override fun lookup(key: String, out: salvo.SalvoReply) {
        val __target = route_to(__fx, group)
        salvo.SalvoSched.sendWire(__target, __Msg_Lookup.Lookup(key, out), __PROTO_Lookup, __Codec___Msg_Lookup)
    }
}

sealed class __Cont___Route_Lookup {
    class Lookup(val key: String) : __Cont___Route_Lookup()
}

class __Actor___Route_Lookup<__Fx>(private val handler: __Route_Lookup<__Fx>) : salvo.SalvoActor where __Fx : __Has_Pick {
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

class __Route_Search<__Fx>(private val group: Int, private val __fx: __Fx) : Search where __Fx : __Has_Pick {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont___Route_Search> = mutableMapOf()

    override fun query(word: String, out: salvo.SalvoReply) {
        val __target = route_to(__fx, group)
        salvo.SalvoSched.sendWire(__target, __Msg_Search.Query(word, out), __PROTO_Search, __Codec___Msg_Search)
    }
}

sealed class __Cont___Route_Search {
    class Query(val word: String) : __Cont___Route_Search()
}

class __Actor___Route_Search<__Fx>(private val handler: __Route_Search<__Fx>) : salvo.SalvoActor where __Fx : __Has_Pick {
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

class __Route_Sequencer<__Fx>(private val group: Int, private val __fx: __Fx) : Sequencer where __Fx : __Has_Pick {
    internal val __mailboxCapacity: Int = 1
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont___Route_Sequencer> = mutableMapOf()

    override fun next(out: salvo.SalvoReply) {
        val __target = route_to(__fx, group)
        salvo.SalvoSched.sendWire(__target, __Msg_Sequencer.Next(out), __PROTO_Sequencer, __Codec___Msg_Sequencer)
    }
}

sealed class __Cont___Route_Sequencer {
    class Next() : __Cont___Route_Sequencer()
}

class __Actor___Route_Sequencer<__Fx>(private val handler: __Route_Sequencer<__Fx>) : salvo.SalvoActor where __Fx : __Has_Pick {
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

class __Fx_1(
    override val __fx_Console: Console,
    override val __fx_Leader: Leader,
    override val __fx_Pick: Pick,
) : __Has_Console, __Has_Leader, __Has_Pick

class __Fx_2(
    override val __fx_Pick: Pick,
) : __Has_Pick

class __Fx_3(
    override val __fx_Console: Console,
    override val __fx_Leader: Leader,
    override val __fx_Pick: Pick,
    override val __fx_Sequencer: Sequencer,
) : __Has_Console, __Has_Leader, __Has_Pick, __Has_Sequencer

class __Fx_4(
    override val __fx_Console: Console,
    override val __fx_Pick: Pick,
) : __Has_Console, __Has_Pick

class __Fx_5(
    override val __fx_Console: Console,
    override val __fx_Inventory: Inventory,
    override val __fx_Pick: Pick,
) : __Has_Console, __Has_Inventory, __Has_Pick

class __Fx_6(
    override val __fx_Transport: Transport,
) : __Has_Transport

class __Fx_7(
    override val __fx_Console: Console,
) : __Has_Console

class __Fx_8(
    override val __fx_Console: Console,
    override val __fx_Transport: Transport,
) : __Has_Console, __Has_Transport

class __Fx_9(
    override val __fx_Console: Console,
    override val __fx_Leader: Leader,
    override val __fx_Transport: Transport,
) : __Has_Console, __Has_Leader, __Has_Transport

class __Fx_10(
    override val __fx_Console: Console,
    override val __fx_Leader: Leader,
    override val __fx_Search: Search,
    override val __fx_Transport: Transport,
) : __Has_Console, __Has_Leader, __Has_Search, __Has_Transport

class __Fx_11(
    override val __fx_Console: Console,
    override val __fx_Leader: Leader,
    override val __fx_Lookup: Lookup,
    override val __fx_Search: Search,
    override val __fx_Transport: Transport,
) : __Has_Console, __Has_Leader, __Has_Lookup, __Has_Search, __Has_Transport
