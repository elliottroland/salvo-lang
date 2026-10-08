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
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.next(out) } finally { lock.unlock() }
    }
}

class __Stub_Sequencer(private val addr: Int) : Sequencer {
    override fun next(out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Sequencer.Next(out), __PROTO_Sequencer, __Codec___Msg_Sequencer)
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
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.reserve(sku, qty, out) } finally { lock.unlock() }
    }
}

class __Stub_Inventory(private val addr: Int) : Inventory {
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Inventory.Reserve(sku, qty, out), __PROTO_Inventory, __Codec___Msg_Inventory)
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
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.query(word, out) } finally { lock.unlock() }
    }
}

class __Stub_Search(private val addr: Int) : Search {
    override fun query(word: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Search.Query(word, out), __PROTO_Search, __Codec___Msg_Search)
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
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.lookup(key, out) } finally { lock.unlock() }
    }
}

class __Stub_Lookup(private val addr: Int) : Lookup {
    override fun lookup(key: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Lookup.Lookup(key, out), __PROTO_Lookup, __Codec___Msg_Lookup)
    }
}

class Sequencing(private val who: String) : Sequencer {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Sequencing> = mutableMapOf()
    var n: Int = 0
    override fun next(out: salvo.SalvoReply) {
        n = (n + 1)
        salvo.SalvoSched.replyWire(out, "${who}#${n}", salvo.StrCodec)
    }
}

class __Actor_Sequencing(private val handler: Sequencing) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_Sequencing_Sequencer(handler, msg as __Msg_Sequencer)
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
                __PROTO_Sequencer -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Sequencer)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Stocking(private val shard: String) : Inventory {
    val __mailboxCapacity: Int = 32
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Stocking> = mutableMapOf()
    var served: Int = 0
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        served = (served + qty)
        salvo.SalvoSched.replyWire(out, "${shard}:${served}", salvo.StrCodec)
    }
}

class __Actor_Stocking(private val handler: Stocking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_Stocking_Inventory(handler, msg as __Msg_Inventory)
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
                __PROTO_Inventory -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Inventory)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Indexing(private val words: List<String>) : Search {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Indexing> = mutableMapOf()
    override fun query(word: String, out: salvo.SalvoReply) {
        var n: Int = 0
        for (w in salvo.platform.core.list.each(words)) {
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
        __dispatch_Indexing_Search(handler, msg as __Msg_Search)
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
                __PROTO_Search -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Search)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Looking(private val who: String) : Lookup {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Looking> = mutableMapOf()
    override fun lookup(key: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, "${key} from ${who}", salvo.StrCodec)
    }
}

class __Actor_Looking(private val handler: Looking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_Looking_Lookup(handler, msg as __Msg_Lookup)
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
                __PROTO_Lookup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Lookup)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class SlowLooking(private val who: String, private val timer: Int) : Lookup {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_SlowLooking> = mutableMapOf()
    override fun lookup(key: String, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(timer, salvo.time.__Msg_Timer.After(salvo.time.millis(150L), run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_SlowLooking.Answer(key, out); __r }), salvo.time.__PROTO_Timer, salvo.time.__Codec___Msg_Timer)
    }
    fun answer(key: String, out: salvo.SalvoReply, fired: salvo.time.Fired) {
        salvo.SalvoSched.replyWire(out, "${key} from ${who}", salvo.StrCodec)
    }
}

class __Actor_SlowLooking(private val handler: SlowLooking) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Lookup -> __dispatch_SlowLooking_Lookup(handler, msg)
            is __Priv_SlowLooking -> __dispatch_priv_SlowLooking(handler, msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_SlowLooking.Lookup -> handler.lookup(c.key, value as salvo.SalvoReply)
            is __Cont_SlowLooking.Answer -> handler.answer(c.key, c.out, value as salvo.time.Fired)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_SlowLooking.Lookup -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_SlowLooking.Answer -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.time.__Codec_Fired) })(payload)
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
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Scattering> = mutableMapOf()
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun query(word: String, out: salvo.SalvoReply) {
        val members: List<Int> = run {
            val (ms, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
            salvo.SalvoSched.sendWire(group, salvo.net.__Msg_ActorGroup.Members(ms), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<Int>
        }
        salvo.SalvoSched.sendWire(gather, __Msg_Gather.Scatter(word, members, out), __PROTO_Gather, __Codec___Msg_Gather)
    }
}

class __Actor_Scattering(private val handler: Scattering) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_Scattering_Search(handler, msg as __Msg_Search)
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
                __PROTO_Search -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Search)?.let { Pair(true, it) } ?: Pair(false, null)
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
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.scatter(word, members, out) } finally { lock.unlock() }
    }
}

class __Stub_Gather(private val addr: Int) : Gather {
    override fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Gather.Scatter(word, members, out), __PROTO_Gather, __Codec___Msg_Gather)
    }
}

class Gathering : Gather {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Gathering> = mutableMapOf()
    var pending: salvo.platform.core.deque.MutDeque<salvo.SalvoReply> = salvo.core.deque.mutDequeOf()
    var left: Int = 0
    var total: Int = 0
    override fun scatter(word: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.core.deque.addLastPlatform(pending, out)
        left = salvo.core.list.sizePlatform(members)
        total = 0
        for (m in salvo.platform.core.list.each(members)) {
            salvo.SalvoSched.sendWire(m, __Msg_Search.Query(word, run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_Gathering.Partial(); __r }), __PROTO_Search, __Codec___Msg_Search)
        }
    }
    fun partial(n: Int) {
        total = (total + n)
        left = (left - 1)
        if (((left) == (0))) {
            val out: salvo.SalvoReply? = salvo.core.deque.removeFirstPlatform(pending)
            when {
                (out != null) -> {
                    val out_1: salvo.SalvoReply = out!!
                    salvo.SalvoSched.replyWire(out_1, total, salvo.IntCodec)
                }
                (out == null) -> {
                }
                else -> throw IllegalStateException("salvo: unreachable arm")
            }
        }
    }
}

class __Actor_Gathering(private val handler: Gathering) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Gather -> __dispatch_Gathering_Gather(handler, msg)
            is __Priv_Gathering -> __dispatch_priv_Gathering(handler, msg)
            else -> error("a message of one of this actor's protocols")
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
                __PROTO_Gather -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Gather)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Hedging(private val group: Int, private val racer: Int) : Lookup {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Hedging> = mutableMapOf()
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun lookup(key: String, out: salvo.SalvoReply) {
        val members: List<Int> = run {
            val (ms, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
            salvo.SalvoSched.sendWire(group, salvo.net.__Msg_ActorGroup.Members(ms), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<Int>
        }
        salvo.SalvoSched.sendWire(racer, __Msg_Race.Race(key, members, out), __PROTO_Race, __Codec___Msg_Race)
    }
}

class __Actor_Hedging(private val handler: Hedging) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_Hedging_Lookup(handler, msg as __Msg_Lookup)
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
                __PROTO_Lookup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Lookup)?.let { Pair(true, it) } ?: Pair(false, null)
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
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.race(key, members, out) } finally { lock.unlock() }
    }
}

class __Stub_Race(private val addr: Int) : Race {
    override fun race(key: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_Race.Race(key, members, out), __PROTO_Race, __Codec___Msg_Race)
    }
}

class Racing : Race {
    val __mailboxCapacity: Int = 16
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Racing> = mutableMapOf()
    var pending: salvo.platform.core.deque.MutDeque<salvo.SalvoReply> = salvo.core.deque.mutDequeOf()
    override fun race(key: String, members: List<Int>, out: salvo.SalvoReply) {
        salvo.core.deque.addLastPlatform(pending, out)
        for (m in salvo.platform.core.list.each(members)) {
            salvo.SalvoSched.sendWire(m, __Msg_Lookup.Lookup(key, run { val (__r, __s) = salvo.SalvoSched.mint(__addr!!); __parked[__s] = __Cont_Racing.First(); __r }), __PROTO_Lookup, __Codec___Msg_Lookup)
        }
    }
    fun first(answer: String) {
        val out: salvo.SalvoReply? = salvo.core.deque.removeFirstPlatform(pending)
        when {
            (out != null) -> {
                val out_1: salvo.SalvoReply = out!!
                salvo.SalvoSched.replyWire(out_1, answer, salvo.StrCodec)
            }
            (out == null) -> {
                run { answer; Unit }
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
}

class __Actor_Racing(private val handler: Racing) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_Race -> __dispatch_Racing_Race(handler, msg)
            is __Priv_Racing -> __dispatch_priv_Racing(handler, msg)
            else -> error("a message of one of this actor's protocols")
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
                __PROTO_Race -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Race)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class LastHost(private val nodes: Int, private val me: salvo.net.NodeEndpoint) : salvo.net.Leader {
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun leader(): salvo.net.NodeId? {
        val peers: List<salvo.net.Node> = run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.net.__Codec_Node)) })
            salvo.SalvoSched.sendWire(nodes, salvo.net.__Msg_NodeGroup.Members(out), salvo.net.__PROTO_NodeGroup, salvo.net.__Codec___Msg_NodeGroup)
            salvo.SalvoSched.awaitReply(__wid) as List<salvo.net.Node>
        }
        var bestHost: String = me.host
        var best: salvo.net.NodeId = salvo.net.thisNode()
        for (n in salvo.platform.core.list.each(peers)) {
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
    return run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        sequencer.next(out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun checkout(inventory: Inventory, console: salvo.core.console.Console, skus: List<String>) {
    val shards: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    for (sku in salvo.platform.core.list.each(skus)) {
        val answer: String = run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
            inventory.reserve(sku, 1, out)
            salvo.SalvoSched.awaitReply(__wid) as String
        }
        val parts: salvo.platform.core.list.MutList<String> = salvo.core.string.splitPlatform(answer, ":")
        salvo.core.list.addPlatform(shards, run {
            val __nn_1: String? = salvo.core.list.getPlatform(parts, 0)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:220:26"))
                }
                else -> {
                    val __some_2: String = __nn_1!!
                    __some_2
                }
            }
        })
        salvo.core.console.println(console, "  ${sku}: ${run {
            val __nn_3: String? = salvo.core.list.getPlatform(parts, 1)
            when {
                (__nn_3 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:221:30"))
                }
                else -> {
                    val __some_4: String = __nn_3!!
                    __some_4
                }
            }
        }} reserved on its shard so far")
    }
    salvo.core.console.println(console, "  apple and apple on one shard: ${((run {
        val __nn_5: String? = salvo.core.list.getPlatform(shards, 0)
        when {
            (__nn_5 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:223:51"))
            }
            else -> {
                val __some_6: String = __nn_5!!
                __some_6
            }
        }
    }) == (run {
        val __nn_7: String? = salvo.core.list.getPlatform(shards, 2)
        when {
            (__nn_7 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:223:68"))
            }
            else -> {
                val __some_8: String = __nn_7!!
                __some_8
            }
        }
    }))}")
    salvo.core.console.println(console, "  apple and fig on one shard: ${((run {
        val __nn_9: String? = salvo.core.list.getPlatform(shards, 0)
        when {
            (__nn_9 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:224:49"))
            }
            else -> {
                val __some_10: String = __nn_9!!
                __some_10
            }
        }
    }) == (run {
        val __nn_11: String? = salvo.core.list.getPlatform(shards, 3)
        when {
            (__nn_11 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:224:66"))
            }
            else -> {
                val __some_12: String = __nn_11!!
                __some_12
            }
        }
    }))}")
    salvo.core.console.println(console, "  apple and pear on one shard: ${((run {
        val __nn_13: String? = salvo.core.list.getPlatform(shards, 0)
        when {
            (__nn_13 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:225:50"))
            }
            else -> {
                val __some_14: String = __nn_13!!
                __some_14
            }
        }
    }) == (run {
        val __nn_15: String? = salvo.core.list.getPlatform(shards, 1)
        when {
            (__nn_15 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:225:67"))
            }
            else -> {
                val __some_16: String = __nn_15!!
                __some_16
            }
        }
    }))}")
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun count(search: Search, word: String): Int {
    return run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.IntCodec) })
        search.query(word, out)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun find(lookup: Lookup, key: String): String {
    return run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })
        lookup.lookup(key, out)
        salvo.SalvoSched.awaitReply(__wid) as String
    }
}

fun twoIds(leader: salvo.net.Leader, console: salvo.core.console.Console, seq: Int) {
    val __use_1: salvo.net.Elected = salvo.net.Elected(leader)
    val __lock___use_1 = java.util.concurrent.locks.ReentrantLock()
    val __handle_2: salvo.net.RouteSelector = salvo.net.__Mon_RouteSelector(__use_1, __lock___use_1)
    val __use_3: __Route_Sequencer = __Route_Sequencer(group = seq, config = salvo.net.defaultRouteConfig(), __handle_2)
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: Sequencer = __Mon_Sequencer(__use_3, __lock___use_3)
    salvo.core.console.println(console, "  ${freshId(__handle_4)} ${freshId(__handle_4)}")
}

fun shop(console: salvo.core.console.Console, stock: Int) {
    val __use_1: salvo.net.Sharded = salvo.net.Sharded()
    val __lock___use_1 = java.util.concurrent.locks.ReentrantLock()
    val __handle_2: salvo.net.RouteSelector = salvo.net.__Mon_RouteSelector(__use_1, __lock___use_1)
    val __use_3: __Route_Inventory = __Route_Inventory(group = stock, config = salvo.net.defaultRouteConfig(), __handle_2)
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: Inventory = __Mon_Inventory(__use_3, __lock___use_3)
    checkout(__handle_4, console, listOf<String>("apple", "pear", "apple", "fig", "pear"))
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
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.boot(done) } finally { lock.unlock() }
    }
    override fun stop(done: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.stop(done) } finally { lock.unlock() }
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

class Booting(private val at: salvo.net.NodeEndpoint, private val all: List<salvo.net.NodeEndpoint>, private val net: Int, private val __dep0: salvo.net.Transport) : Boot {
    val __mailboxCapacity: Int = 2
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Booting> = mutableMapOf()
    var nodes: Int? = null
    override fun boot(done: salvo.SalvoReply) {
        val p: Int = salvo.core.actor.pool(1)
        val group: Int = run { val __h = salvo.net.StaticNodeGroup(name = "cluster", all = all, __dep0); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, salvo.net.__Actor_StaticNodeGroup(__h), salvo.net.__Actor_StaticNodeGroup.__DECODE); salvo.SalvoSched.send(__a, salvo.net.__Priv_StaticNodeGroup.Init); __a }
        nodes = group
        val seq: Int = salvo.net.actorGroup__Addr(group, {  -> salvo.net.Protocol("Sequencer", __PROTO_Sequencer) })
        val mine: Int = run { val __h = Sequencing(who = "b"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sequencing(__h), __Actor_Sequencing.__DECODE); salvo.SalvoSched.sendWire(seq, salvo.net.__Msg_ActorGroup.Join(__a), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __a }
        val stock: Int = salvo.net.actorGroup__Addr(group, {  -> salvo.net.Protocol("Inventory", __PROTO_Inventory) })
        run { val __h = Stocking(shard = "shard-b"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Stocking(__h), __Actor_Stocking.__DECODE); salvo.SalvoSched.sendWire(stock, salvo.net.__Msg_ActorGroup.Join(__a), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __a }
        val index: Int = salvo.net.actorGroup__Addr(group, {  -> salvo.net.Protocol("Search", __PROTO_Search) })
        run { val __h = Indexing(words = listOf<String>("salvo", "actors", "salvo", "nodes")); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Indexing(__h), __Actor_Indexing.__DECODE); salvo.SalvoSched.sendWire(index, salvo.net.__Msg_ActorGroup.Join(__a), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __a }
        val looks: Int = salvo.net.actorGroup__Addr(group, {  -> salvo.net.Protocol("Lookup", __PROTO_Lookup) })
        val timer: Int = run { val __h = salvo.time.DefaultTimer(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, salvo.time.__Actor_DefaultTimer(__h), salvo.time.__Actor_DefaultTimer.__DECODE); __a }
        run { val __h = SlowLooking(who = "b (slow)", timer = timer); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_SlowLooking(__h), __Actor_SlowLooking.__DECODE); salvo.SalvoSched.sendWire(looks, salvo.net.__Msg_ActorGroup.Join(__a), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __a }
        salvo.SalvoSched.replyWire(done, mine, salvo.AddrCodec)
    }
    override fun stop(done: salvo.SalvoReply) {
        val group: Int? = nodes
        if (!((group == null))) {
            val group_1: Int = group!!
            salvo.SalvoSched.sendWire(group_1, salvo.net.__Msg_NodeGroup.Leave(), salvo.net.__PROTO_NodeGroup, salvo.net.__Codec___Msg_NodeGroup)
        }
        salvo.SalvoSched.replyWire(done, true, salvo.BoolCodec)
    }
}

class __Actor_Booting(private val handler: Booting) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_Booting_Boot(handler, msg as __Msg_Boot)
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
                __PROTO_Boot -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Boot)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun settle(timer: Int) {
    val _f: salvo.time.Fired = run {
        val (f, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.time.__Codec_Fired) })
        salvo.SalvoSched.sendWire(timer, salvo.time.__Msg_Timer.After(salvo.time.millis(400L), f), salvo.time.__PROTO_Timer, salvo.time.__Codec___Msg_Timer)
        salvo.SalvoSched.awaitReply(__wid) as salvo.time.Fired
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("ActorGroup", salvo.net.__PROTO_ActorGroup), Pair("ActorGroupWatcher", salvo.net.__PROTO_ActorGroupWatcher), Pair("Boot", salvo.main.__PROTO_Boot), Pair("Faults", salvo.core.actor.__PROTO_Faults), Pair("Gather", salvo.main.__PROTO_Gather), Pair("Inbound", salvo.net.__PROTO_Inbound), Pair("Inventory", salvo.main.__PROTO_Inventory), Pair("Lookup", salvo.main.__PROTO_Lookup), Pair("MemNet", salvo.net.__PROTO_MemNet), Pair("NodeGroup", salvo.net.__PROTO_NodeGroup), Pair("NodeGroupWatcher", salvo.net.__PROTO_NodeGroupWatcher), Pair("Outbound", salvo.net.__PROTO_Outbound), Pair("Race", salvo.main.__PROTO_Race), Pair("Search", salvo.main.__PROTO_Search), Pair("Sequencer", salvo.main.__PROTO_Sequencer), Pair("Timer", salvo.time.__PROTO_Timer), Pair("TimerCtl", salvo.time.__PROTO_TimerCtl), Pair("Wheel", salvo.runtime.timers.__PROTO_Wheel)))
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val a: salvo.net.NodeEndpoint = salvo.net.NodeEndpoint(host = "a", port = 1)
    val b: salvo.net.NodeEndpoint = salvo.net.NodeEndpoint(host = "b", port = 1)
    val all: List<salvo.net.NodeEndpoint> = listOf<salvo.net.NodeEndpoint>(a, b)
    val network: Int = run { val __h = salvo.net.MemNetwork(); val __a = salvo.SalvoSched.spawn(salvo.core.actor.pool(1), __h.__mailboxCapacity, salvo.net.__Actor_MemNetwork(__h), salvo.net.__Actor_MemNetwork.__DECODE); __a }
    val __use_3: salvo.net.MemTransport = salvo.net.MemTransport(me = a, net = network)
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: salvo.net.Transport = salvo.net.__Mon_Transport(__use_3, __lock___use_3)
    val p: Int = salvo.core.actor.pool(2)
    val nodes: Int = run { val __h = salvo.net.StaticNodeGroup(name = "cluster", all = all, __handle_4); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, salvo.net.__Actor_StaticNodeGroup(__h), salvo.net.__Actor_StaticNodeGroup.__DECODE); salvo.SalvoSched.send(__a, salvo.net.__Priv_StaticNodeGroup.Init); __a }
    val seq: Int = salvo.net.actorGroup__Addr(nodes, {  -> salvo.net.Protocol("Sequencer", __PROTO_Sequencer) })
    run { val __h = Sequencing(who = "a"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Sequencing(__h), __Actor_Sequencing.__DECODE); salvo.SalvoSched.sendWire(seq, salvo.net.__Msg_ActorGroup.Join(__a), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __a }
    val stock: Int = salvo.net.actorGroup__Addr(nodes, {  -> salvo.net.Protocol("Inventory", __PROTO_Inventory) })
    run { val __h = Stocking(shard = "shard-a"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Stocking(__h), __Actor_Stocking.__DECODE); salvo.SalvoSched.sendWire(stock, salvo.net.__Msg_ActorGroup.Join(__a), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __a }
    val index: Int = salvo.net.actorGroup__Addr(nodes, {  -> salvo.net.Protocol("Search", __PROTO_Search) })
    run { val __h = Indexing(words = listOf<String>("salvo", "is", "salvo")); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Indexing(__h), __Actor_Indexing.__DECODE); salvo.SalvoSched.sendWire(index, salvo.net.__Msg_ActorGroup.Join(__a), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __a }
    val looks: Int = salvo.net.actorGroup__Addr(nodes, {  -> salvo.net.Protocol("Lookup", __PROTO_Lookup) })
    run { val __h = Looking(who = "a"); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Looking(__h), __Actor_Looking.__DECODE); salvo.SalvoSched.sendWire(looks, salvo.net.__Msg_ActorGroup.Join(__a), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup); __a }
    val pb: Int = salvo.net.poolAt(salvo.net.newNode(), 1)
    val booter: Int = run { val __h = Booting(at = b, all = all, net = network, salvo.net.__Mon_Transport(salvo.net.MemTransport(me = b, net = network))); val __a = salvo.SalvoSched.spawn(pb, __h.__mailboxCapacity, __Actor_Booting(__h), __Actor_Booting.__DECODE); __a }
    val remoteSeq: Int = run {
        val (done, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })
        salvo.SalvoSched.sendWire(booter, __Msg_Boot.Boot(done), __PROTO_Boot, __Codec___Msg_Boot)
        salvo.SalvoSched.awaitReply(__wid) as Int
    }
    val timer: Int = run { val __h = salvo.time.DefaultTimer(); val __a = salvo.SalvoSched.spawn(salvo.core.actor.pool(1), __h.__mailboxCapacity, salvo.time.__Actor_DefaultTimer(__h), salvo.time.__Actor_DefaultTimer.__DECODE); __a }
    settle(timer)
    val members: List<salvo.net.Node> = run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.net.__Codec_Node)) })
        salvo.SalvoSched.sendWire(nodes, salvo.net.__Msg_NodeGroup.Members(out), salvo.net.__PROTO_NodeGroup, salvo.net.__Codec___Msg_NodeGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<salvo.net.Node>
    }
    salvo.core.console.println(__handle_2, "nodes: ${(salvo.core.list.sizePlatform(members) + 1)}, sequencers: ${salvo.core.list.sizePlatform(run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
        salvo.SalvoSched.sendWire(seq, salvo.net.__Msg_ActorGroup.Members(out), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Int>
    })}")
    salvo.core.console.println(__handle_2, "singleton (b's sequencer is remote: ${!(salvo.net.eq__NodeId_NodeId(salvo.net.nodeOf(remoteSeq), salvo.net.thisNode()))}):")
    val __use_5: LastHost = LastHost(nodes = nodes, me = a)
    val __lock___use_5 = java.util.concurrent.locks.ReentrantLock()
    val __handle_6: salvo.net.Leader = salvo.net.__Mon_Leader(__use_5, __lock___use_5)
    twoIds(__handle_6, __handle_2, seq)
    salvo.core.console.println(__handle_2, "sharded:")
    shop(__handle_2, stock)
    salvo.core.console.println(__handle_2, "scatter:")
    val __use_7: Scattering = Scattering(group = index, gather = run { val __h = Gathering(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Gathering(__h), __Actor_Gathering.__DECODE); __a })
    val __lock___use_7 = java.util.concurrent.locks.ReentrantLock()
    val __handle_8: Search = __Mon_Search(__use_7, __lock___use_7)
    salvo.core.console.println(__handle_2, "  salvo: ${count(__handle_8, "salvo")}, actors: ${count(__handle_8, "actors")}, none: ${count(__handle_8, "none")}")
    salvo.core.console.println(__handle_2, "hedge:")
    val __use_9: Hedging = Hedging(group = looks, racer = run { val __h = Racing(); val __a = salvo.SalvoSched.spawn(p, __h.__mailboxCapacity, __Actor_Racing(__h), __Actor_Racing.__DECODE); __a })
    val __lock___use_9 = java.util.concurrent.locks.ReentrantLock()
    val __handle_10: Lookup = __Mon_Lookup(__use_9, __lock___use_9)
    salvo.core.console.println(__handle_2, "  ${find(__handle_10, "k1")}")
    val _stopped: Boolean = run {
        val (done, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.BoolCodec) })
        salvo.SalvoSched.sendWire(booter, __Msg_Boot.Stop(done), __PROTO_Boot, __Codec___Msg_Boot)
        salvo.SalvoSched.awaitReply(__wid) as Boolean
    }
    settle(timer)
    salvo.core.console.println(__handle_2, "after b left:")
    salvo.core.console.println(__handle_2, "  sequencers: ${salvo.core.list.sizePlatform(run {
        val (out, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })
        salvo.SalvoSched.sendWire(seq, salvo.net.__Msg_ActorGroup.Members(out), salvo.net.__PROTO_ActorGroup, salvo.net.__Codec___Msg_ActorGroup)
        salvo.SalvoSched.awaitReply(__wid) as List<Int>
    })}")
    twoIds(__handle_6, __handle_2, seq)
}

class __Route_Inventory(private val group: Int, private val config: salvo.net.RouteConfig, private val __dep0: salvo.net.RouteSelector) : Inventory {
    val __mailboxCapacity: Int = 1
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont___Route_Inventory> = mutableMapOf()
    var seen: Long = -1L
    override fun reserve(sku: String, qty: Int, out: salvo.SalvoReply) {
        val __pick: salvo.net.RoutePick = salvo.net.routePick__Addr_RouteConfig_Long_Long(__dep0, group, config, seen, salvo.SalvoSched.keyHash(salvo.salvoEncode(sku, salvo.StrCodec).toByteArray()))
        seen = __pick.version
        salvo.SalvoSched.sendWire(__pick.to, __Msg_Inventory.Reserve(sku, qty, out), __PROTO_Inventory, __Codec___Msg_Inventory)
    }
}

class __Actor___Route_Inventory(private val handler: __Route_Inventory) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch___Route_Inventory_Inventory(handler, msg as __Msg_Inventory)
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
                __PROTO_Inventory -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Inventory)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class __Route_Lookup(private val group: Int, private val config: salvo.net.RouteConfig, private val __dep0: salvo.net.RouteSelector) : Lookup {
    val __mailboxCapacity: Int = 1
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont___Route_Lookup> = mutableMapOf()
    var seen: Long = -1L
    override fun lookup(key: String, out: salvo.SalvoReply) {
        val __pick: salvo.net.RoutePick = salvo.net.routePick__Addr_RouteConfig_Long(__dep0, group, config, seen)
        seen = __pick.version
        salvo.SalvoSched.sendWire(__pick.to, __Msg_Lookup.Lookup(key, out), __PROTO_Lookup, __Codec___Msg_Lookup)
    }
}

class __Actor___Route_Lookup(private val handler: __Route_Lookup) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch___Route_Lookup_Lookup(handler, msg as __Msg_Lookup)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont___Route_Lookup.Lookup -> handler.lookup(c.key, value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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

class __Route_Search(private val group: Int, private val config: salvo.net.RouteConfig, private val __dep0: salvo.net.RouteSelector) : Search {
    val __mailboxCapacity: Int = 1
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont___Route_Search> = mutableMapOf()
    var seen: Long = -1L
    override fun query(word: String, out: salvo.SalvoReply) {
        val __pick: salvo.net.RoutePick = salvo.net.routePick__Addr_RouteConfig_Long(__dep0, group, config, seen)
        seen = __pick.version
        salvo.SalvoSched.sendWire(__pick.to, __Msg_Search.Query(word, out), __PROTO_Search, __Codec___Msg_Search)
    }
}

class __Actor___Route_Search(private val handler: __Route_Search) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch___Route_Search_Search(handler, msg as __Msg_Search)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont___Route_Search.Query -> handler.query(c.word, value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
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

class __Route_Sequencer(private val group: Int, private val config: salvo.net.RouteConfig, private val __dep0: salvo.net.RouteSelector) : Sequencer {
    val __mailboxCapacity: Int = 1
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont___Route_Sequencer> = mutableMapOf()
    var seen: Long = -1L
    override fun next(out: salvo.SalvoReply) {
        val __pick: salvo.net.RoutePick = salvo.net.routePick__Addr_RouteConfig_Long(__dep0, group, config, seen)
        seen = __pick.version
        salvo.SalvoSched.sendWire(__pick.to, __Msg_Sequencer.Next(out), __PROTO_Sequencer, __Codec___Msg_Sequencer)
    }
}

class __Actor___Route_Sequencer(private val handler: __Route_Sequencer) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch___Route_Sequencer_Sequencer(handler, msg as __Msg_Sequencer)
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
                __PROTO_Sequencer -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Sequencer)?.let { Pair(true, it) } ?: Pair(false, null)
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
    class Answer(val key: String, val out: salvo.SalvoReply, val fired: salvo.time.Fired) : __Priv_SlowLooking()
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


sealed class __Cont___Route_Lookup {
    class Lookup(val key: String) : __Cont___Route_Lookup()
}


sealed class __Cont___Route_Search {
    class Query(val word: String) : __Cont___Route_Search()
}


sealed class __Cont___Route_Sequencer {
    class Next() : __Cont___Route_Sequencer()
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
fun __dispatch___Route_Lookup_Lookup(__handler: __Route_Lookup, __msg: __Msg_Lookup) {
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
fun __dispatch___Route_Search_Search(__handler: __Route_Search, __msg: __Msg_Search) {
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
fun __dispatch___Route_Sequencer_Sequencer(__handler: __Route_Sequencer, __msg: __Msg_Sequencer) {
    when {
        (__msg is __Msg_Sequencer.Next) -> {
            val out = (__msg as __Msg_Sequencer.Next).out
            __handler.next(out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

