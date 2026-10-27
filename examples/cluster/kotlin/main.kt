package salvo.main

import salvo.*

interface Sequencer {
    fun next(out: salvo.SalvoReply)
}

class __Mon_Sequencer(
    private val inner: Sequencer,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Sequencer {
    override fun next(out: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.next(out) } finally { lock.unlock() }
    }
}

interface Inventory {
    fun reserve(sku: String, qty: Int, out: salvo.SalvoReply)
}

class __Mon_Inventory(
    private val inner: Inventory,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Inventory {
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.reserve(sku, qty, out) } finally { lock.unlock() }
    }
}

interface Search {
    fun query(word: String, out: salvo.SalvoReply)
}

class __Mon_Search(
    private val inner: Search,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Search {
    override fun query(word: String, out: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.query(word, out) } finally { lock.unlock() }
    }
}

interface Lookup {
    fun lookup(key: String, out: salvo.SalvoReply)
}

class __Mon_Lookup(
    private val inner: Lookup,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Lookup {
    override fun lookup(key: String, out: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.lookup(key, out) } finally { lock.unlock() }
    }
}

class Sequencing(private val who: String) : Sequencer {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Sequencing> = kotlin.collections.mutableMapOf()
    var n: Int = 0
    override fun next(out: salvo.SalvoReply) {
        n = (n + 1)
        salvo.SalvoSched.replyWire(out, "${who}#${n}", salvo.StrCodec)
    }
}

class __Actor_Sequencing(private val handler: Sequencing) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch_Sequencing_Sequencer(handler, msg as __Msg_Sequencer)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Sequencing.Next -> handler.next(value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Sequencer -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Sequencer)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Stocking(private val shard: String) : Inventory {
    val __mailboxCapacity: Int = 32
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Stocking> = kotlin.collections.mutableMapOf()
    var served: Int = 0
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        served = (served + qty)
        salvo.SalvoSched.replyWire(out, "${shard}:${served}", salvo.StrCodec)
    }
}

class __Actor_Stocking(private val handler: Stocking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch_Stocking_Inventory(handler, msg as __Msg_Inventory)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Stocking.Reserve -> handler.reserve(c.sku, c.qty, value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Inventory -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Inventory)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Indexing(private val words: List<String>) : Search {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Indexing> = kotlin.collections.mutableMapOf()
    override fun query(word: String, out: salvo.SalvoReply) {
        var n: Int = 0
        for (w in salvo.platform.std.core.list.each(words)) {
            if (((w) == (word))) {
                n = (n + 1)
            }
        }
        salvo.SalvoSched.replyWire(out, n, salvo.IntCodec)
    }
}

class __Actor_Indexing(private val handler: Indexing) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch_Indexing_Search(handler, msg as __Msg_Search)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Indexing.Query -> handler.query(c.word, value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Search -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Search)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Looking(private val who: String) : Lookup {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Looking> = kotlin.collections.mutableMapOf()
    override fun lookup(key: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, "${key} from ${who}", salvo.StrCodec)
    }
}

class __Actor_Looking(private val handler: Looking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch_Looking_Lookup(handler, msg as __Msg_Lookup)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Looking.Lookup -> handler.lookup(c.key, value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Lookup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Lookup)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class SlowLooking(private val who: String, private val timer: Int) : Lookup {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_SlowLooking> = kotlin.collections.mutableMapOf()
    override fun lookup(key: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(timer, salvo.std.time.__Msg_Timer.After(salvo.std.time.millis(150L), kotlin.run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_SlowLooking.Answer(key, out); __r }), salvo.std.time.__PROTO_Timer, salvo.std.time.__Codec___Msg_Timer)
    }
    fun answer(key: String, out: salvo.SalvoReply, fired: salvo.std.time.Fired) {
        salvo.SalvoSched.replyWire(out, "${key} from ${who}", salvo.StrCodec)
    }
}

class __Actor_SlowLooking(private val handler: SlowLooking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Lookup -> salvo.main.__dispatch_SlowLooking_Lookup(handler, msg)
            is __Priv_SlowLooking -> salvo.main.__dispatch_priv_SlowLooking(handler, msg)
            else -> kotlin.error("a message of one of this actor's protocols")
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_SlowLooking.Lookup -> handler.lookup(c.key, value as salvo.SalvoReply)
            is __Cont_SlowLooking.Answer -> handler.answer(c.key, c.out, value as salvo.std.time.Fired)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_SlowLooking.Lookup -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_SlowLooking.Answer -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.std.time.__Codec_Fired) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Lookup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Lookup)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Scattering(private val group: Int, private val gather: Int) : Search {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Scattering> = kotlin.collections.mutableMapOf()
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun query(word: String, out: salvo.SalvoReply) {
        val members: List<Int> = kotlin.run {
            val (ms, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
            salvo.SalvoSched.sendWire(group, salvo.std.net.__Msg_ActorGroup.Members(ms), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<Int>
        }
        salvo.SalvoSched.sendWire(gather, __Msg_Gather.Scatter(word, members, out), __PROTO_Gather, __Codec___Msg_Gather)
    }
}

class __Actor_Scattering(private val handler: Scattering) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch_Scattering_Search(handler, msg as __Msg_Search)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Scattering.Query -> handler.query(c.word, value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Search -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Search)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

interface Gather {
    fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply)
}

class __Mon_Gather(
    private val inner: Gather,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Gather {
    override fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.scatter(word, members, out) } finally { lock.unlock() }
    }
}

class Gathering : Gather {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Gathering> = kotlin.collections.mutableMapOf()
    var pending: salvo.platform.std.core.deque.MutDeque<salvo.SalvoReply> = salvo.std.core.deque.mutDequeOf()
    var left: Int = 0
    var total: Int = 0
    override fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.std.core.deque.addLastPlatform(pending, out)
        left = salvo.std.core.list.sizePlatform(members)
        total = 0
        for (m in salvo.platform.std.core.list.each(members)) {
            salvo.SalvoSched.sendWire(m, __Msg_Search.Query(word, kotlin.run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_Gathering.Partial(); __r }), __PROTO_Search, __Codec___Msg_Search)
        }
    }
    fun partial(n: Int) {
        total = (total + n)
        left = (left - 1)
        if ((left == 0)) {
            val out: salvo.SalvoReply? = salvo.std.core.deque.removeFirstPlatform(pending)
            when {
                (out != null) -> {
                    val out_1: salvo.SalvoReply = out!!
                    salvo.SalvoSched.replyWire(out_1, total, salvo.IntCodec)
                }
                (out == null) -> {
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:141:13"))
                }
            }
        }
    }
}

class __Actor_Gathering(private val handler: Gathering) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Gather -> salvo.main.__dispatch_Gathering_Gather(handler, msg)
            is __Priv_Gathering -> salvo.main.__dispatch_priv_Gathering(handler, msg)
            else -> kotlin.error("a message of one of this actor's protocols")
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Gathering.Scatter -> handler.scatter(c.word, c.members, value as salvo.SalvoReply)
            is __Cont_Gathering.Partial -> handler.partial(value as Int)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Gather -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Gather)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Hedging(private val group: Int, private val racer: Int) : Lookup {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Hedging> = kotlin.collections.mutableMapOf()
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun lookup(key: String, out: salvo.SalvoReply) {
        val members: List<Int> = kotlin.run {
            val (ms, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
            salvo.SalvoSched.sendWire(group, salvo.std.net.__Msg_ActorGroup.Members(ms), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<Int>
        }
        salvo.SalvoSched.sendWire(racer, __Msg_Race.Race(key, members, out), __PROTO_Race, __Codec___Msg_Race)
    }
}

class __Actor_Hedging(private val handler: Hedging) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch_Hedging_Lookup(handler, msg as __Msg_Lookup)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Hedging.Lookup -> handler.lookup(c.key, value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Lookup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Lookup)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

interface Race {
    fun race(key: String, members: List<Int>, out: salvo.SalvoReply)
}

class __Mon_Race(
    private val inner: Race,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Race {
    override fun race(key: String, members: List<Int>, out: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.race(key, members, out) } finally { lock.unlock() }
    }
}

class Racing : Race {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Racing> = kotlin.collections.mutableMapOf()
    var pending: salvo.platform.std.core.deque.MutDeque<salvo.SalvoReply> = salvo.std.core.deque.mutDequeOf()
    override fun race(key: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.std.core.deque.addLastPlatform(pending, out)
        for (m in salvo.platform.std.core.list.each(members)) {
            salvo.SalvoSched.sendWire(m, __Msg_Lookup.Lookup(key, kotlin.run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_Racing.First(); __r }), __PROTO_Lookup, __Codec___Msg_Lookup)
        }
    }
    fun first(answer: String) {
        val out: salvo.SalvoReply? = salvo.std.core.deque.removeFirstPlatform(pending)
        when {
            (out != null) -> {
                val out_1: salvo.SalvoReply = out!!
                salvo.SalvoSched.replyWire(out_1, answer, salvo.StrCodec)
            }
            (out == null) -> {
                kotlin.run { answer; Unit }
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:177:9"))
            }
        }
    }
}

class __Actor_Racing(private val handler: Racing) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Race -> salvo.main.__dispatch_Racing_Race(handler, msg)
            is __Priv_Racing -> salvo.main.__dispatch_priv_Racing(handler, msg)
            else -> kotlin.error("a message of one of this actor's protocols")
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Racing.Race -> handler.race(c.key, c.members, value as salvo.SalvoReply)
            is __Cont_Racing.First -> handler.first(value as String)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Race -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Race)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class LastHost(private val nodes: Int, private val me: salvo.std.net.NodeEndpoint) : salvo.std.net.Leader {
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun leader(): salvo.std.net.NodeId? {
        val peers: List<salvo.std.net.Node> = kotlin.run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.std.net.__Codec_Node)) })
            salvo.SalvoSched.sendWire(nodes, salvo.std.net.__Msg_NodeGroup.Members(out), salvo.std.net.__PROTO_NodeGroup, salvo.std.net.__Codec___Msg_NodeGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<salvo.std.net.Node>
        }
        var bestHost: String = me.host
        var best: salvo.std.net.NodeId = salvo.std.net.thisNode()
        for (n in salvo.platform.std.core.list.each(peers)) {
            if ((salvo.__salvoCompare(n.at.host, bestHost) > 0)) {
                bestHost = n.at.host
                best = n.id
            }
        }
        return best
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun freshId(sequencer: Sequencer): String {
    return kotlin.run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        sequencer.next(out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun checkout(inventory: Inventory, console: salvo.std.core.console.Console, skus: List<String>) {
    val shards: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    for (sku in salvo.platform.std.core.list.each(skus)) {
        val answer: String = kotlin.run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
            inventory.reserve(sku, 1, out)
            salvo.SalvoSched.awaitReply(__wid) as String
        }
        val parts: salvo.platform.std.core.list.MutList<String> = salvo.std.core.string.splitPlatform(answer, ":")
        salvo.std.core.list.addPlatform(shards, kotlin.run {
            val __pick_1: String? = salvo.std.core.list.getPlatform(parts, 0)
            (if ((__pick_1 != null)) {
                val __picked_2: String = __pick_1!!
                __picked_2
            } else {
                throw AssertionError("salvo: unreachable at main:228:43")
            })
        })
        salvo.std.core.console.println(console, "  ${sku}: ${kotlin.run {
            val __pick_4: String? = salvo.std.core.list.getPlatform(parts, 1)
            (if ((__pick_4 != null)) {
                val __picked_5: String = __pick_4!!
                __picked_5
            } else {
                throw AssertionError("salvo: unreachable at main:229:47")
            })
        }} reserved on its shard so far")
    }
    salvo.std.core.console.println(console, "  apple and apple on one shard: ${((kotlin.run {
        val __pick_7: String? = salvo.std.core.list.getPlatform(shards, 0)
        (if ((__pick_7 != null)) {
            val __picked_8: String = __pick_7!!
            __picked_8
        } else {
            throw AssertionError("salvo: unreachable at main:231:69")
        })
    }) == (kotlin.run {
        val __pick_10: String? = salvo.std.core.list.getPlatform(shards, 2)
        (if ((__pick_10 != null)) {
            val __picked_11: String = __pick_10!!
            __picked_11
        } else {
            throw AssertionError("salvo: unreachable at main:231:103")
        })
    }))}")
    salvo.std.core.console.println(console, "  apple and fig on one shard: ${((kotlin.run {
        val __pick_13: String? = salvo.std.core.list.getPlatform(shards, 0)
        (if ((__pick_13 != null)) {
            val __picked_14: String = __pick_13!!
            __picked_14
        } else {
            throw AssertionError("salvo: unreachable at main:232:67")
        })
    }) == (kotlin.run {
        val __pick_16: String? = salvo.std.core.list.getPlatform(shards, 3)
        (if ((__pick_16 != null)) {
            val __picked_17: String = __pick_16!!
            __picked_17
        } else {
            throw AssertionError("salvo: unreachable at main:232:101")
        })
    }))}")
    salvo.std.core.console.println(console, "  apple and pear on one shard: ${((kotlin.run {
        val __pick_19: String? = salvo.std.core.list.getPlatform(shards, 0)
        (if ((__pick_19 != null)) {
            val __picked_20: String = __pick_19!!
            __picked_20
        } else {
            throw AssertionError("salvo: unreachable at main:233:68")
        })
    }) == (kotlin.run {
        val __pick_22: String? = salvo.std.core.list.getPlatform(shards, 1)
        (if ((__pick_22 != null)) {
            val __picked_23: String = __pick_22!!
            __picked_23
        } else {
            throw AssertionError("salvo: unreachable at main:233:102")
        })
    }))}")
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun count(search: Search, word: String): Int {
    return kotlin.run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        search.query(word, out)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun find(lookup: Lookup, key: String): String {
    return kotlin.run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        lookup.lookup(key, out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
}

fun twoIds(leader: salvo.std.net.Leader, console: salvo.std.core.console.Console, seq: Int) {
    val __use_1: salvo.std.net.Elected = salvo.std.net.Elected(config = salvo.std.net.defaultRouteConfig(), leader)
    val __lock___use_1 = java.util.concurrent.locks.ReentrantLock()
    val __handle_2: salvo.std.net.RouteSelector = salvo.std.net.__Mon_RouteSelector(__use_1, __lock___use_1)
    val __use_3: __Route_Sequencer = __Route_Sequencer(group = seq, __handle_2)
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: Sequencer = __Mon_Sequencer(__use_3, __lock___use_3)
    salvo.std.core.console.println(console, "  ${salvo.main.freshId(__handle_4)} ${salvo.main.freshId(__handle_4)}")
}

fun shop(console: salvo.std.core.console.Console, stock: Int) {
    val __use_1: salvo.std.net.Sharded = salvo.std.net.Sharded(config = salvo.std.net.defaultRouteConfig())
    val __lock___use_1 = java.util.concurrent.locks.ReentrantLock()
    val __handle_2: salvo.std.net.RouteSelector = salvo.std.net.__Mon_RouteSelector(__use_1, __lock___use_1)
    val __use_3: __Route_Inventory = __Route_Inventory(group = stock, __handle_2)
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: Inventory = __Mon_Inventory(__use_3, __lock___use_3)
    salvo.main.checkout(__handle_4, console, listOf<String>("apple", "pear", "apple", "fig", "pear"))
}

interface Boot {
    fun boot(done: salvo.SalvoReply)
    fun stop(done: salvo.SalvoReply)
}

class __Mon_Boot(
    private val inner: Boot,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Boot {
    override fun boot(done: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.boot(done) } finally { lock.unlock() }
    }
    override fun stop(done: salvo.SalvoReply) {
        kotlin.check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.stop(done) } finally { lock.unlock() }
    }
}

class Booting(private val at: salvo.std.net.NodeEndpoint, private val all: List<salvo.std.net.NodeEndpoint>, private val net: Int, private val __dep0: salvo.std.net.Transport) : Boot {
    val __mailboxCapacity: Int = 2
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Booting> = kotlin.collections.mutableMapOf()
    var nodes: Int? = null
    override fun boot(done: salvo.SalvoReply) {
        val p: Int = salvo.std.core.actor.pool(1)
        val group: Int = kotlin.run { val __h = salvo.std.net.StaticNodeGroup(name = "cluster", all = all, __dep0); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, salvo.std.net.__Actor_StaticNodeGroup(__h), salvo.std.net.__Actor_StaticNodeGroup.__DECODE); salvo.SalvoSched.send(__a, salvo.std.net.__Priv_StaticNodeGroup.Init); __a }
        nodes = group
        val seq: Int = salvo.std.net.actorGroup__Addr(group, {  -> salvo.std.net.Protocol("Sequencer", __PROTO_Sequencer) })
        val mine: Int = kotlin.run { val __h = Sequencing(who = "b"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sequencing(__h), __Actor_Sequencing.__DECODE); salvo.SalvoSched.sendWire(seq, salvo.std.net.__Msg_ActorGroup.Join(__a), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup); __a }
        val stock: Int = salvo.std.net.actorGroup__Addr(group, {  -> salvo.std.net.Protocol("Inventory", __PROTO_Inventory) })
        kotlin.run { val __h = Stocking(shard = "shard-b"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Stocking(__h), __Actor_Stocking.__DECODE); salvo.SalvoSched.sendWire(stock, salvo.std.net.__Msg_ActorGroup.Join(__a), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup); __a }
        val index: Int = salvo.std.net.actorGroup__Addr(group, {  -> salvo.std.net.Protocol("Search", __PROTO_Search) })
        kotlin.run { val __h = Indexing(words = listOf<String>("salvo", "actors", "salvo", "nodes")); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Indexing(__h), __Actor_Indexing.__DECODE); salvo.SalvoSched.sendWire(index, salvo.std.net.__Msg_ActorGroup.Join(__a), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup); __a }
        val looks: Int = salvo.std.net.actorGroup__Addr(group, {  -> salvo.std.net.Protocol("Lookup", __PROTO_Lookup) })
        val timer: Int = kotlin.run { val __h = salvo.std.time.DefaultTimer(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, salvo.std.time.__Actor_DefaultTimer(__h), salvo.std.time.__Actor_DefaultTimer.__DECODE); __a }
        kotlin.run { val __h = SlowLooking(who = "b (slow)", timer = timer); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_SlowLooking(__h), __Actor_SlowLooking.__DECODE); salvo.SalvoSched.sendWire(looks, salvo.std.net.__Msg_ActorGroup.Join(__a), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup); __a }
        salvo.SalvoSched.replyWire(done, mine, salvo.AddrCodec)
    }
    override fun stop(done: salvo.SalvoReply) {
        val group: Int? = nodes
        if (!((group == null))) {
            val group_1: Int = group!!
            salvo.SalvoSched.sendWire(group_1, salvo.std.net.__Msg_NodeGroup.Leave(), salvo.std.net.__PROTO_NodeGroup, salvo.std.net.__Codec___Msg_NodeGroup)
        }
        salvo.SalvoSched.replyWire(done, true, salvo.BoolCodec)
    }
}

class __Actor_Booting(private val handler: Booting) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch_Booting_Boot(handler, msg as __Msg_Boot)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Booting.Boot -> handler.boot(value as salvo.SalvoReply)
            is __Cont_Booting.Stop -> handler.stop(value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Boot -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Boot)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun settle(timer: Int) {
    val _f: salvo.std.time.Fired = kotlin.run {
        val (f, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.std.time.__Codec_Fired) })
        salvo.SalvoSched.sendWire(timer, salvo.std.time.__Msg_Timer.After(salvo.std.time.millis(400L), f), salvo.std.time.__PROTO_Timer, salvo.std.time.__Codec___Msg_Timer)
        salvo.SalvoSched.awaitReply(__wid) as salvo.std.time.Fired
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(kotlin.collections.listOf(kotlin.Pair("ActorGroup", salvo.std.net.__PROTO_ActorGroup), kotlin.Pair("ActorGroupWatcher", salvo.std.net.__PROTO_ActorGroupWatcher), kotlin.Pair("Boot", salvo.main.__PROTO_Boot), kotlin.Pair("Faults", salvo.std.core.actor.__PROTO_Faults), kotlin.Pair("Gather", salvo.main.__PROTO_Gather), kotlin.Pair("Inbound", salvo.std.net.__PROTO_Inbound), kotlin.Pair("Inventory", salvo.main.__PROTO_Inventory), kotlin.Pair("Lookup", salvo.main.__PROTO_Lookup), kotlin.Pair("MemNet", salvo.std.net.__PROTO_MemNet), kotlin.Pair("NodeGroup", salvo.std.net.__PROTO_NodeGroup), kotlin.Pair("NodeGroupWatcher", salvo.std.net.__PROTO_NodeGroupWatcher), kotlin.Pair("Outbound", salvo.std.net.__PROTO_Outbound), kotlin.Pair("Race", salvo.main.__PROTO_Race), kotlin.Pair("Search", salvo.main.__PROTO_Search), kotlin.Pair("Sequencer", salvo.main.__PROTO_Sequencer), kotlin.Pair("Timer", salvo.std.time.__PROTO_Timer), kotlin.Pair("TimerCtl", salvo.std.time.__PROTO_TimerCtl), kotlin.Pair("Wheel", salvo.std.runtime.timers.__PROTO_Wheel)))
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val a: salvo.std.net.NodeEndpoint = salvo.std.net.NodeEndpoint(host = "a", port = 1)
    val b: salvo.std.net.NodeEndpoint = salvo.std.net.NodeEndpoint(host = "b", port = 1)
    val all: List<salvo.std.net.NodeEndpoint> = listOf<salvo.std.net.NodeEndpoint>(a, b)
    val network: Int = kotlin.run { val __h = salvo.std.net.MemNetwork(); val __a = salvo.SalvoSched.spawn(salvo.std.core.actor.pool(1), __h.__mailboxCapacity, salvo.std.net.__Actor_MemNetwork(__h), salvo.std.net.__Actor_MemNetwork.__DECODE); __a }
    val __use_3: salvo.std.net.MemTransport = salvo.std.net.MemTransport(me = a, net = network)
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: salvo.std.net.Transport = salvo.std.net.__Mon_Transport(__use_3, __lock___use_3)
    val p: Int = salvo.std.core.actor.pool(2)
    val nodes: Int = kotlin.run { val __h = salvo.std.net.StaticNodeGroup(name = "cluster", all = all, __handle_4); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, salvo.std.net.__Actor_StaticNodeGroup(__h), salvo.std.net.__Actor_StaticNodeGroup.__DECODE); salvo.SalvoSched.send(__a, salvo.std.net.__Priv_StaticNodeGroup.Init); __a }
    val seq: Int = salvo.std.net.actorGroup__Addr(nodes, {  -> salvo.std.net.Protocol("Sequencer", __PROTO_Sequencer) })
    kotlin.run { val __h = Sequencing(who = "a"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sequencing(__h), __Actor_Sequencing.__DECODE); salvo.SalvoSched.sendWire(seq, salvo.std.net.__Msg_ActorGroup.Join(__a), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup); __a }
    val stock: Int = salvo.std.net.actorGroup__Addr(nodes, {  -> salvo.std.net.Protocol("Inventory", __PROTO_Inventory) })
    kotlin.run { val __h = Stocking(shard = "shard-a"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Stocking(__h), __Actor_Stocking.__DECODE); salvo.SalvoSched.sendWire(stock, salvo.std.net.__Msg_ActorGroup.Join(__a), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup); __a }
    val index: Int = salvo.std.net.actorGroup__Addr(nodes, {  -> salvo.std.net.Protocol("Search", __PROTO_Search) })
    kotlin.run { val __h = Indexing(words = listOf<String>("salvo", "is", "salvo")); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Indexing(__h), __Actor_Indexing.__DECODE); salvo.SalvoSched.sendWire(index, salvo.std.net.__Msg_ActorGroup.Join(__a), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup); __a }
    val looks: Int = salvo.std.net.actorGroup__Addr(nodes, {  -> salvo.std.net.Protocol("Lookup", __PROTO_Lookup) })
    kotlin.run { val __h = Looking(who = "a"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Looking(__h), __Actor_Looking.__DECODE); salvo.SalvoSched.sendWire(looks, salvo.std.net.__Msg_ActorGroup.Join(__a), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup); __a }
    val pb: Int = salvo.std.net.poolAt(salvo.std.net.newNode(), 1)
    val booter: Int = kotlin.run { val __h = Booting(at = b, all = all, net = network, salvo.std.net.__Mon_Transport(salvo.std.net.MemTransport(me = b, net = network))); val __a = salvo.SalvoSched.spawn(pb, __h.__mailboxCapacity, __Actor_Booting(__h), __Actor_Booting.__DECODE); __a }
    val remoteSeq: Int = kotlin.run {
        val (done, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })
        salvo.SalvoSched.sendWire(booter, __Msg_Boot.Boot(done), __PROTO_Boot, __Codec___Msg_Boot)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    val timer: Int = kotlin.run { val __h = salvo.std.time.DefaultTimer(); val __a = salvo.SalvoSched.spawn(salvo.std.core.actor.pool(1), __h.__mailboxCapacity, salvo.std.time.__Actor_DefaultTimer(__h), salvo.std.time.__Actor_DefaultTimer.__DECODE); __a }
    salvo.main.settle(timer)
    val members: List<salvo.std.net.Node> = kotlin.run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.std.net.__Codec_Node)) })
        salvo.SalvoSched.sendWire(nodes, salvo.std.net.__Msg_NodeGroup.Members(out), salvo.std.net.__PROTO_NodeGroup, salvo.std.net.__Codec___Msg_NodeGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<salvo.std.net.Node>
    }
    salvo.std.core.console.println(__handle_2, "nodes: ${(salvo.std.core.list.sizePlatform(members) + 1)}, sequencers: ${salvo.std.core.list.sizePlatform(kotlin.run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
        salvo.SalvoSched.sendWire(seq, salvo.std.net.__Msg_ActorGroup.Members(out), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Int>
    })}")
    salvo.std.core.console.println(__handle_2, "singleton (b's sequencer is remote: ${!(salvo.std.net.eq__NodeId_NodeId(salvo.std.net.nodeOf(remoteSeq), salvo.std.net.thisNode()))}):")
    val __use_5: LastHost = LastHost(nodes = nodes, me = a)
    val __lock___use_5 = java.util.concurrent.locks.ReentrantLock()
    val __handle_6: salvo.std.net.Leader = salvo.std.net.__Mon_Leader(__use_5, __lock___use_5)
    salvo.main.twoIds(__handle_6, __handle_2, seq)
    salvo.std.core.console.println(__handle_2, "sharded:")
    salvo.main.shop(__handle_2, stock)
    salvo.std.core.console.println(__handle_2, "scatter:")
    val __use_7: Scattering = Scattering(group = index, gather = kotlin.run { val __h = Gathering(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Gathering(__h), __Actor_Gathering.__DECODE); __a })
    val __lock___use_7 = java.util.concurrent.locks.ReentrantLock()
    val __handle_8: Search = __Mon_Search(__use_7, __lock___use_7)
    salvo.std.core.console.println(__handle_2, "  salvo: ${salvo.main.count(__handle_8, "salvo")}, actors: ${salvo.main.count(__handle_8, "actors")}, none: ${salvo.main.count(__handle_8, "none")}")
    salvo.std.core.console.println(__handle_2, "hedge:")
    val __use_9: Hedging = Hedging(group = looks, racer = kotlin.run { val __h = Racing(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Racing(__h), __Actor_Racing.__DECODE); __a })
    val __lock___use_9 = java.util.concurrent.locks.ReentrantLock()
    val __handle_10: Lookup = __Mon_Lookup(__use_9, __lock___use_9)
    salvo.std.core.console.println(__handle_2, "  ${salvo.main.find(__handle_10, "k1")}")
    val _stopped: Boolean = kotlin.run {
        val (done, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.BoolCodec) })
        salvo.SalvoSched.sendWire(booter, __Msg_Boot.Stop(done), __PROTO_Boot, __Codec___Msg_Boot)
        salvo.SalvoSched.awaitReply(__wid) as Boolean
    }
    salvo.main.settle(timer)
    salvo.std.core.console.println(__handle_2, "after b left:")
    salvo.std.core.console.println(__handle_2, "  sequencers: ${salvo.std.core.list.sizePlatform(kotlin.run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
        salvo.SalvoSched.sendWire(seq, salvo.std.net.__Msg_ActorGroup.Members(out), salvo.std.net.__PROTO_ActorGroup, salvo.std.net.__Codec___Msg_ActorGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Int>
    })}")
    salvo.main.twoIds(__handle_6, __handle_2, seq)
}

class __Route_Inventory(private val group: Int, private val __dep0: salvo.std.net.RouteSelector) : Inventory {
    val __mailboxCapacity: Int = 1
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont___Route_Inventory> = kotlin.collections.mutableMapOf()
    var seen: Long = -1L
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        val __pick: salvo.std.net.RoutePick = salvo.std.net.routePick__Addr_Long_Long(__dep0, group, seen, salvo.SalvoSched.keyHash(salvo.salvoEncode(sku, salvo.StrCodec).toByteArray()))
        seen = __pick.version
        salvo.SalvoSched.sendWire(__pick.to, __Msg_Inventory.Reserve(sku, qty, out), __PROTO_Inventory, __Codec___Msg_Inventory)
    }
}

class __Actor___Route_Inventory(private val handler: __Route_Inventory) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch___Route_Inventory_Inventory(handler, msg as __Msg_Inventory)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont___Route_Inventory.Reserve -> handler.reserve(c.sku, c.qty, value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Inventory -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Inventory)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class __Route_Sequencer(private val group: Int, private val __dep0: salvo.std.net.RouteSelector) : Sequencer {
    val __mailboxCapacity: Int = 1
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont___Route_Sequencer> = kotlin.collections.mutableMapOf()
    var seen: Long = -1L
    override fun next(out: salvo.SalvoReply) {
        val __pick: salvo.std.net.RoutePick = salvo.std.net.routePick__Addr_Long(__dep0, group, seen)
        seen = __pick.version
        salvo.SalvoSched.sendWire(__pick.to, __Msg_Sequencer.Next(out), __PROTO_Sequencer, __Codec___Msg_Sequencer)
    }
}

class __Actor___Route_Sequencer(private val handler: __Route_Sequencer) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        salvo.main.__dispatch___Route_Sequencer_Sequencer(handler, msg as __Msg_Sequencer)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont___Route_Sequencer.Next -> handler.next(value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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
                __PROTO_Sequencer -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Sequencer)?.let { kotlin.Pair(true, it) } ?: kotlin.Pair(false, null)
                else -> Pair(false, null)
            }
        }
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

const val __PROTO_Sequencer: String = "7a5334482e5247f7"


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

const val __PROTO_Inventory: String = "d3482a697a944808"


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

const val __PROTO_Search: String = "ed817fc30774f01d"


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

const val __PROTO_Lookup: String = "7c0f441570dc9a6f"


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

const val __PROTO_Gather: String = "98712ca205da344c"


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

const val __PROTO_Race: String = "5e0ec4d63d5f53dd"


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

const val __PROTO_Boot: String = "4b15e647d0ca92a7"


sealed class __Cont_Sequencing {
    class Next() : __Cont_Sequencing()
}


sealed class __Cont_Stocking {
    class Reserve(val sku: String, val qty: Int) : __Cont_Stocking()
}


sealed class __Cont_Indexing {
    class Query(val word: String) : __Cont_Indexing()
}


sealed class __Cont_Looking {
    class Lookup(val key: String) : __Cont_Looking()
}


sealed class __Cont_SlowLooking {
    class Lookup(val key: String) : __Cont_SlowLooking()
    class Answer(val key: String, val out: salvo.SalvoReply) : __Cont_SlowLooking()
}


sealed class __Priv_SlowLooking {
    class Answer(val key: String, val out: salvo.SalvoReply, val fired: salvo.std.time.Fired) : __Priv_SlowLooking()
}


sealed class __Cont_Scattering {
    class Query(val word: String) : __Cont_Scattering()
}


sealed class __Cont_Gathering {
    class Scatter(val word: String, val members: List<Int>) : __Cont_Gathering()
    class Partial() : __Cont_Gathering()
}


sealed class __Priv_Gathering {
    class Partial(val n: Int) : __Priv_Gathering()
}


sealed class __Cont_Hedging {
    class Lookup(val key: String) : __Cont_Hedging()
}


sealed class __Cont_Racing {
    class Race(val key: String, val members: List<Int>) : __Cont_Racing()
    class First() : __Cont_Racing()
}


sealed class __Priv_Racing {
    class First(val answer: String) : __Priv_Racing()
}


sealed class __Cont_Booting {
    class Boot() : __Cont_Booting()
    class Stop() : __Cont_Booting()
}


sealed class __Cont___Route_Inventory {
    class Reserve(val sku: String, val qty: Int) : __Cont___Route_Inventory()
}


sealed class __Cont___Route_Sequencer {
    class Next() : __Cont___Route_Sequencer()
}

class __Stub_Sequencer(private val addr: Int) : Sequencer {
    override fun next(out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Sequencer.Next(out), __PROTO_Sequencer, __Codec___Msg_Sequencer)
    }
}

class __Stub_Inventory(private val addr: Int) : Inventory {
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Inventory.Reserve(sku, qty, out), __PROTO_Inventory, __Codec___Msg_Inventory)
    }
}

class __Stub_Search(private val addr: Int) : Search {
    override fun query(word: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Search.Query(word, out), __PROTO_Search, __Codec___Msg_Search)
    }
}

class __Stub_Lookup(private val addr: Int) : Lookup {
    override fun lookup(key: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Lookup.Lookup(key, out), __PROTO_Lookup, __Codec___Msg_Lookup)
    }
}

class __Stub_Gather(private val addr: Int) : Gather {
    override fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Gather.Scatter(word, members, out), __PROTO_Gather, __Codec___Msg_Gather)
    }
}

class __Stub_Race(private val addr: Int) : Race {
    override fun race(key: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Race.Race(key, members, out), __PROTO_Race, __Codec___Msg_Race)
    }
}

class __Stub_Boot(private val addr: Int) : Boot {
    override fun boot(done: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Boot.Boot(done), __PROTO_Boot, __Codec___Msg_Boot)
    }
    override fun stop(done: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Boot.Stop(done), __PROTO_Boot, __Codec___Msg_Boot)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Sequencing_Sequencer(__handler: Sequencing, __msg: __Msg_Sequencer) {
    when {
        (__msg is __Msg_Sequencer.Next) -> {
            val out = (__msg as __Msg_Sequencer.Next).out
            __handler.next(out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Stocking_Inventory(__handler: Stocking, __msg: __Msg_Inventory) {
    when {
        (__msg is __Msg_Inventory.Reserve) -> {
            val sku = (__msg as __Msg_Inventory.Reserve).sku
            val qty = (__msg as __Msg_Inventory.Reserve).qty
            val out = (__msg as __Msg_Inventory.Reserve).out
            __handler.reserve(sku, qty, out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Indexing_Search(__handler: Indexing, __msg: __Msg_Search) {
    when {
        (__msg is __Msg_Search.Query) -> {
            val word = (__msg as __Msg_Search.Query).word
            val out = (__msg as __Msg_Search.Query).out
            __handler.query(word, out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Looking_Lookup(__handler: Looking, __msg: __Msg_Lookup) {
    when {
        (__msg is __Msg_Lookup.Lookup) -> {
            val key = (__msg as __Msg_Lookup.Lookup).key
            val out = (__msg as __Msg_Lookup.Lookup).out
            __handler.lookup(key, out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_SlowLooking_Lookup(__handler: SlowLooking, __msg: __Msg_Lookup) {
    when {
        (__msg is __Msg_Lookup.Lookup) -> {
            val key = (__msg as __Msg_Lookup.Lookup).key
            val out = (__msg as __Msg_Lookup.Lookup).out
            __handler.lookup(key, out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_priv_SlowLooking(__handler: SlowLooking, __msg: __Priv_SlowLooking) {
    when {
        (__msg is __Priv_SlowLooking.Answer) -> {
            val key = (__msg as __Priv_SlowLooking.Answer).key
            val out = (__msg as __Priv_SlowLooking.Answer).out
            val fired = (__msg as __Priv_SlowLooking.Answer).fired
            __handler.answer(key, out, fired)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Scattering_Search(__handler: Scattering, __msg: __Msg_Search) {
    when {
        (__msg is __Msg_Search.Query) -> {
            val word = (__msg as __Msg_Search.Query).word
            val out = (__msg as __Msg_Search.Query).out
            __handler.query(word, out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Gathering_Gather(__handler: Gathering, __msg: __Msg_Gather) {
    when {
        (__msg is __Msg_Gather.Scatter) -> {
            val word = (__msg as __Msg_Gather.Scatter).word
            val members = (__msg as __Msg_Gather.Scatter).members
            val out = (__msg as __Msg_Gather.Scatter).out
            __handler.scatter(word, members, out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_priv_Gathering(__handler: Gathering, __msg: __Priv_Gathering) {
    when {
        (__msg is __Priv_Gathering.Partial) -> {
            val n = (__msg as __Priv_Gathering.Partial).n
            __handler.partial(n)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Hedging_Lookup(__handler: Hedging, __msg: __Msg_Lookup) {
    when {
        (__msg is __Msg_Lookup.Lookup) -> {
            val key = (__msg as __Msg_Lookup.Lookup).key
            val out = (__msg as __Msg_Lookup.Lookup).out
            __handler.lookup(key, out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Racing_Race(__handler: Racing, __msg: __Msg_Race) {
    when {
        (__msg is __Msg_Race.Race) -> {
            val key = (__msg as __Msg_Race.Race).key
            val members = (__msg as __Msg_Race.Race).members
            val out = (__msg as __Msg_Race.Race).out
            __handler.race(key, members, out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_priv_Racing(__handler: Racing, __msg: __Priv_Racing) {
    when {
        (__msg is __Priv_Racing.First) -> {
            val answer = (__msg as __Priv_Racing.First).answer
            __handler.first(answer)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Booting_Boot(__handler: Booting, __msg: __Msg_Boot) {
    when {
        (__msg is __Msg_Boot.Boot) -> {
            val done = (__msg as __Msg_Boot.Boot).done
            __handler.boot(done)
        }
        (__msg is __Msg_Boot.Stop) -> {
            val done = (__msg as __Msg_Boot.Stop).done
            __handler.stop(done)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch___Route_Inventory_Inventory(__handler: __Route_Inventory, __msg: __Msg_Inventory) {
    when {
        (__msg is __Msg_Inventory.Reserve) -> {
            val sku = (__msg as __Msg_Inventory.Reserve).sku
            val qty = (__msg as __Msg_Inventory.Reserve).qty
            val out = (__msg as __Msg_Inventory.Reserve).out
            __handler.reserve(sku, qty, out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch___Route_Sequencer_Sequencer(__handler: __Route_Sequencer, __msg: __Msg_Sequencer) {
    when {
        (__msg is __Msg_Sequencer.Next) -> {
            val out = (__msg as __Msg_Sequencer.Next).out
            __handler.next(out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

