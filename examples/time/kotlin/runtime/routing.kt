package salvo.runtime.routing

import salvo.*

val __module_use0: Routes by lazy {
    val __module_use0: Routes = Routes()
    __module_use0
}

val __module_use0_0: RouteTable by lazy {
    val __lock___module_use0 = java.util.concurrent.locks.ReentrantLock()
    val __module_use0_0: RouteTable = __Mon_RouteTable(__module_use0, __lock___module_use0)
    __module_use0_0
}

fun decodeMessagePlatform(addr: Int, proto: String, payload: salvo.platform.core.bytes.Bytes): salvo.platform.runtime.Dyn? = salvo.platform.runtime.routing.decodeMessage(addr, proto, payload)

fun decodeWaiterAnswerPlatform(wid: Int, payload: salvo.platform.core.bytes.Bytes): salvo.platform.runtime.Dyn? = salvo.platform.runtime.routing.decodeWaiterAnswer(wid, payload)

fun decodeTaskAnswerPlatform(key: Long, payload: salvo.platform.core.bytes.Bytes): salvo.platform.runtime.Dyn? = salvo.platform.runtime.routing.decodeTaskAnswer(key, payload)

fun rawAnswerPlatform(payload: salvo.platform.core.bytes.Bytes): salvo.platform.runtime.Dyn = salvo.platform.runtime.routing.rawAnswer(payload)

fun controlMessagePlatform(sink: Int, from: Long, payload: salvo.platform.core.bytes.Bytes): salvo.platform.runtime.Dyn? = salvo.platform.runtime.routing.controlMessage(sink, from, payload)

fun wireOutPlatform(from: Long, to: salvo.platform.core.bytes.Bytes, frame: salvo.platform.core.bytes.Bytes) = salvo.platform.runtime.routing.wireOut(from, to, frame)

data class MsgFrame(
    val to: Long,
    val actor: Long,
    val bits: Long,
    val from: Long,
    val proto: String,
    val payload: salvo.platform.core.bytes.Bytes,
)

object __Codec_MsgFrame : salvo.WireCodec<MsgFrame> {
    override fun enc(v: MsgFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.LongCodec.enc(v.actor, out)
        salvo.LongCodec.enc(v.bits, out)
        salvo.LongCodec.enc(v.from, out)
        salvo.StrCodec.enc(v.proto, out)
        salvo.BytesCodec.enc(v.payload, out)
    }
    override fun dec(inp: salvo.WireIn): MsgFrame = MsgFrame(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class AnswerFrame(
    val to: Long,
    val kind: Int,
    val id: Long,
    val slot: Long,
    val bits: Long,
    val payload: salvo.platform.core.bytes.Bytes,
)

object __Codec_AnswerFrame : salvo.WireCodec<AnswerFrame> {
    override fun enc(v: AnswerFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.IntCodec.enc(v.kind, out)
        salvo.LongCodec.enc(v.id, out)
        salvo.LongCodec.enc(v.slot, out)
        salvo.LongCodec.enc(v.bits, out)
        salvo.BytesCodec.enc(v.payload, out)
    }
    override fun dec(inp: salvo.WireIn): AnswerFrame = AnswerFrame(salvo.LongCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class GrantFrame(
    val to: Long,
    val host: Long,
    val actor: Long,
    val bits: Long,
    val n: Int,
)

object __Codec_GrantFrame : salvo.WireCodec<GrantFrame> {
    override fun enc(v: GrantFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.LongCodec.enc(v.host, out)
        salvo.LongCodec.enc(v.actor, out)
        salvo.LongCodec.enc(v.bits, out)
        salvo.IntCodec.enc(v.n, out)
    }
    override fun dec(inp: salvo.WireIn): GrantFrame = GrantFrame(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.IntCodec.dec(inp))
}

data class OpenFrame(
    val to: Long,
    val actor: Long,
    val bits: Long,
    val from: Long,
)

object __Codec_OpenFrame : salvo.WireCodec<OpenFrame> {
    override fun enc(v: OpenFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.LongCodec.enc(v.actor, out)
        salvo.LongCodec.enc(v.bits, out)
        salvo.LongCodec.enc(v.from, out)
    }
    override fun dec(inp: salvo.WireIn): OpenFrame = OpenFrame(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp))
}

data class ControlFrame(
    val to: Long,
    val from: Long,
    val channel: String,
    val payload: salvo.platform.core.bytes.Bytes,
)

object __Codec_ControlFrame : salvo.WireCodec<ControlFrame> {
    override fun enc(v: ControlFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.LongCodec.enc(v.from, out)
        salvo.StrCodec.enc(v.channel, out)
        salvo.BytesCodec.enc(v.payload, out)
    }
    override fun dec(inp: salvo.WireIn): ControlFrame = ControlFrame(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class RemoteRef(
    val node: Long,
    val actor: Long,
    val bits: Long,
)

object __Codec_RemoteRef : salvo.WireCodec<RemoteRef> {
    override fun enc(v: RemoteRef, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.node, out)
        salvo.LongCodec.enc(v.actor, out)
        salvo.LongCodec.enc(v.bits, out)
    }
    override fun dec(inp: salvo.WireIn): RemoteRef = RemoteRef(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp))
}

data class ReplyParts(
    val node: Long,
    val kind: Int,
    val id: Long,
    val slot: Long,
    val bits: Long,
)

object __Codec_ReplyParts : salvo.WireCodec<ReplyParts> {
    override fun enc(v: ReplyParts, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.node, out)
        salvo.IntCodec.enc(v.kind, out)
        salvo.LongCodec.enc(v.id, out)
        salvo.LongCodec.enc(v.slot, out)
        salvo.LongCodec.enc(v.bits, out)
    }
    override fun dec(inp: salvo.WireIn): ReplyParts = ReplyParts(salvo.LongCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp))
}

data class ControlKey(
    val node: Long,
    val channel: String,
)

object __Codec_ControlKey : salvo.WireCodec<ControlKey> {
    override fun enc(v: ControlKey, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.node, out)
        salvo.StrCodec.enc(v.channel, out)
    }
    override fun dec(inp: salvo.WireIn): ControlKey = ControlKey(salvo.LongCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class Parked(
    val from: Long,
    val to: Long,
    val frame: salvo.platform.core.bytes.Bytes,
)

object __Codec_Parked : salvo.WireCodec<Parked> {
    override fun enc(v: Parked, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.from, out)
        salvo.LongCodec.enc(v.to, out)
        salvo.BytesCodec.enc(v.frame, out)
    }
    override fun dec(inp: salvo.WireIn): Parked = Parked(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class Staged(
    val from: Long,
    val to: salvo.platform.core.bytes.Bytes,
    val frame: salvo.platform.core.bytes.Bytes,
)

object __Codec_Staged : salvo.WireCodec<Staged> {
    override fun enc(v: Staged, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.from, out)
        salvo.BytesCodec.enc(v.to, out)
        salvo.BytesCodec.enc(v.frame, out)
    }
    override fun dec(inp: salvo.WireIn): Staged = Staged(salvo.LongCodec.dec(inp), salvo.BytesCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class ExportedTask(
    val pool: Int,
    val body: salvo.platform.runtime.Body,
)

fun dropExportedTask(t: ExportedTask) {
    val __destructured_1: ExportedTask = t
    val pool: Int = __destructured_1.pool
    val body: salvo.platform.runtime.Body = __destructured_1.body
    salvo.runtime.dropBodyPlatform(body)
}

data class Found(
    val idx: Int,
)

object __Codec_Found : salvo.WireCodec<Found> {
    override fun enc(v: Found, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.idx, out)
    }
    override fun dec(inp: salvo.WireIn): Found = Found(salvo.IntCodec.dec(inp))
}

class MakeProxy

object __Codec_MakeProxy : salvo.WireCodec<MakeProxy> {
    override fun enc(v: MakeProxy, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MakeProxy = MakeProxy()
}

class MakeDead

object __Codec_MakeDead : salvo.WireCodec<MakeDead> {
    override fun enc(v: MakeDead, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): MakeDead = MakeDead()
}

interface RouteTable {
    fun nodeOfPool(pool: Int): Long
    fun adoptPool(pool: Int, node: Long)
    fun addNode(): Long
    fun hosts(node: Long): Boolean
    fun identityOf(addr: Int, pool: Int): RemoteRef
    fun findImport(r: RemoteRef, here: Long): Union3<Found, MakeProxy, MakeDead>
    fun registerProxy(r: RemoteRef, idx: Int, here: Long): Int
    fun registerDead(idx: Int): Int
    fun isProxy(addr: Int): Boolean
    fun proxyRef(addr: Int): RemoteRef?
    fun takeCredit(addr: Int, me: salvo.platform.runtime.Parker): Int
    fun stage(from: Long, to: Long, frame: salvo.platform.core.bytes.Bytes)
    fun grant(addr: Int, pool: Int, from: Long, n: Int)
    fun takeOutbox(): salvo.platform.core.list.MutList<Staged>
    fun addRoute(node: Long, at: salvo.platform.core.bytes.Bytes)
    fun setOutbound(node: Long)
    fun hasOutbound(node: Long): Boolean
    fun accepts(to: Long, actor: Long, claimed: Long): Boolean
    fun received(idx: Int)
    fun credited(r: RemoteRef, to: Long, n: Int): Boolean
    fun held(idx: Int): Int
    fun putTask(key: Long, t: ExportedTask)
    fun takeTask(key: Long): ExportedTask?
    fun watchChannel(node: Long, channel: String, sink: Int)
    fun channelSink(node: Long, channel: String): Int
    fun setProtocols(table: List<Pair<String, String>>)
    fun protocols(): List<Pair<String, String>>
    fun setPeer(node: Long, table: List<Pair<String, String>>)
    fun peerHash(node: Long, protocol: String): String?
    fun forgetNode(node: Long): List<Int>
    fun creditsOf(addr: Int): Int?
    fun setView(group: Int, members: List<Int>)
    fun viewOf(group: Int): List<Int>
    fun versionOf(group: Int): Long
    fun bump(group: Int)
    fun addViewWaiter(group: Int, seen: Long, me: salvo.platform.runtime.Parker): Long
    fun dropViewWaiter(id: Long)
}

class __Mon_RouteTable(
    private val inner: RouteTable,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : RouteTable {
    override fun nodeOfPool(pool: Int): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.nodeOfPool(pool) } finally { lock.unlock() }
    }
    override fun adoptPool(pool: Int, node: Long) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.adoptPool(pool, node) } finally { lock.unlock() }
    }
    override fun addNode(): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.addNode() } finally { lock.unlock() }
    }
    override fun hosts(node: Long): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.hosts(node) } finally { lock.unlock() }
    }
    override fun identityOf(addr: Int, pool: Int): RemoteRef {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.identityOf(addr, pool) } finally { lock.unlock() }
    }
    override fun findImport(r: RemoteRef, here: Long): Union3<Found, MakeProxy, MakeDead> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.findImport(r, here) } finally { lock.unlock() }
    }
    override fun registerProxy(r: RemoteRef, idx: Int, here: Long): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.registerProxy(r, idx, here) } finally { lock.unlock() }
    }
    override fun registerDead(idx: Int): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.registerDead(idx) } finally { lock.unlock() }
    }
    override fun isProxy(addr: Int): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.isProxy(addr) } finally { lock.unlock() }
    }
    override fun proxyRef(addr: Int): RemoteRef? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.proxyRef(addr) } finally { lock.unlock() }
    }
    override fun takeCredit(addr: Int, me: salvo.platform.runtime.Parker): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.takeCredit(addr, me) } finally { lock.unlock() }
    }
    override fun stage(from: Long, to: Long, frame: salvo.platform.core.bytes.Bytes) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.stage(from, to, frame) } finally { lock.unlock() }
    }
    override fun grant(addr: Int, pool: Int, from: Long, n: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.grant(addr, pool, from, n) } finally { lock.unlock() }
    }
    override fun takeOutbox(): salvo.platform.core.list.MutList<Staged> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.takeOutbox() } finally { lock.unlock() }
    }
    override fun addRoute(node: Long, at: salvo.platform.core.bytes.Bytes) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.addRoute(node, at) } finally { lock.unlock() }
    }
    override fun setOutbound(node: Long) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.setOutbound(node) } finally { lock.unlock() }
    }
    override fun hasOutbound(node: Long): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.hasOutbound(node) } finally { lock.unlock() }
    }
    override fun accepts(to: Long, actor: Long, claimed: Long): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.accepts(to, actor, claimed) } finally { lock.unlock() }
    }
    override fun received(idx: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.received(idx) } finally { lock.unlock() }
    }
    override fun credited(r: RemoteRef, to: Long, n: Int): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.credited(r, to, n) } finally { lock.unlock() }
    }
    override fun held(idx: Int): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.held(idx) } finally { lock.unlock() }
    }
    override fun putTask(key: Long, t: ExportedTask) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.putTask(key, t) } finally { lock.unlock() }
    }
    override fun takeTask(key: Long): ExportedTask? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.takeTask(key) } finally { lock.unlock() }
    }
    override fun watchChannel(node: Long, channel: String, sink: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.watchChannel(node, channel, sink) } finally { lock.unlock() }
    }
    override fun channelSink(node: Long, channel: String): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.channelSink(node, channel) } finally { lock.unlock() }
    }
    override fun setProtocols(table: List<Pair<String, String>>) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.setProtocols(table) } finally { lock.unlock() }
    }
    override fun protocols(): List<Pair<String, String>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.protocols() } finally { lock.unlock() }
    }
    override fun setPeer(node: Long, table: List<Pair<String, String>>) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.setPeer(node, table) } finally { lock.unlock() }
    }
    override fun peerHash(node: Long, protocol: String): String? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.peerHash(node, protocol) } finally { lock.unlock() }
    }
    override fun forgetNode(node: Long): List<Int> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.forgetNode(node) } finally { lock.unlock() }
    }
    override fun creditsOf(addr: Int): Int? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.creditsOf(addr) } finally { lock.unlock() }
    }
    override fun setView(group: Int, members: List<Int>) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.setView(group, members) } finally { lock.unlock() }
    }
    override fun viewOf(group: Int): List<Int> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.viewOf(group) } finally { lock.unlock() }
    }
    override fun versionOf(group: Int): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.versionOf(group) } finally { lock.unlock() }
    }
    override fun bump(group: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.bump(group) } finally { lock.unlock() }
    }
    override fun addViewWaiter(group: Int, seen: Long, me: salvo.platform.runtime.Parker): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.addViewWaiter(group, seen, me) } finally { lock.unlock() }
    }
    override fun dropViewWaiter(id: Long) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.dropViewWaiter(id) } finally { lock.unlock() }
    }
}

fun versionIn(versions: salvo.platform.core.map.Map<Int, Long>, group: Int): Long {
    val v: Long? = salvo.core.map.getPlatform(versions, group, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((v != null)) {
        val n: Long = v!!
        return n
    }
    return 0L
}

fun bumpIn(versions: salvo.platform.core.map.MutMap<Int, Long>, waiters: salvo.platform.core.list.MutList<ViewWaiter>, group: Int) {
    salvo.core.map.putPlatform(versions, group, (versionIn(versions, group) + 1L), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var i: Int = 0
    while (true) {
        if (!((i < salvo.core.list.sizePlatform(waiters)))) {
            break
        }
        if ((run {
            val __proj_3: ViewWaiter = run {
                val __nn_1: ViewWaiter? = salvo.core.list.getPlatform(waiters, i)
                when {
                    (__nn_1 == null) -> {
                        throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:171:12"))
                    }
                    else -> {
                        val __some_2: ViewWaiter = __nn_1!!
                        __some_2
                    }
                }
            }
            __proj_3.group
        } == group)) {
            val w: ViewWaiter = run {
                val __nn_4: ViewWaiter? = salvo.core.list.removeAtPlatform(waiters, i)
                when {
                    (__nn_4 == null) -> {
                        throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:172:21"))
                    }
                    else -> {
                        val __some_5: ViewWaiter = __nn_4!!
                        __some_5
                    }
                }
            }
            salvo.runtime.unparkPlatform(w.parker)
        } else {
            i = (i + 1)
        }
    }
}

data class ViewWaiter(
    val group: Int,
    val id: Long,
    val parker: salvo.platform.runtime.Parker,
)

class Routes : RouteTable {
    var nodeId: Long = 0L
    var hosted: salvo.platform.core.set.MutSet<Long> = salvo.core.set.mutSetOfPlatform(arrayOf<Long>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var poolNode: salvo.platform.core.map.MutMap<Int, Long> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Int, Long>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var bits: salvo.platform.core.map.MutMap<Int, Long> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Int, Long>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var remote: salvo.platform.core.map.MutMap<Int, RemoteRef> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Int, RemoteRef>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var proxies: salvo.platform.core.map.MutMap<RemoteRef, Int> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<RemoteRef, Int>>(), ::hash__RemoteRef, ::eq__RemoteRef_RemoteRef)
    var credits: salvo.platform.core.map.MutMap<Int, Int> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Int, Int>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var heldN: salvo.platform.core.map.MutMap<Int, Int> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Int, Int>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var routes: salvo.platform.core.map.MutMap<Long, salvo.platform.core.bytes.Bytes> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Long, salvo.platform.core.bytes.Bytes>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var outbound: salvo.platform.core.set.MutSet<Long> = salvo.core.set.mutSetOfPlatform(arrayOf<Long>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var parked: salvo.platform.core.list.MutList<Parked> = mutableListOf<Parked>()
    var outbox: salvo.platform.core.list.MutList<Staged> = mutableListOf<Staged>()
    var taskKeys: salvo.platform.core.list.MutList<Long> = mutableListOf<Long>()
    var tasks: salvo.platform.core.list.MutList<ExportedTask> = mutableListOf<ExportedTask>()
    var controls: salvo.platform.core.map.MutMap<ControlKey, Int> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<ControlKey, Int>>(), ::hash__ControlKey, ::eq__ControlKey_ControlKey)
    var local: List<Pair<String, String>> = listOf<Pair<String, String>>()
    var peers: salvo.platform.core.map.MutMap<Long, List<Pair<String, String>>> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Long, List<Pair<String, String>>>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var deadEntry: Int = (-1)
    var creditWaiters: salvo.platform.core.list.MutList<salvo.platform.runtime.Parker> = mutableListOf<salvo.platform.runtime.Parker>()
    var views: salvo.platform.core.map.MutMap<Int, List<Int>> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Int, List<Int>>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var versions: salvo.platform.core.map.MutMap<Int, Long> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Int, Long>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var viewWaiters: salvo.platform.core.list.MutList<ViewWaiter> = mutableListOf<ViewWaiter>()
    var nextWaiter: Long = 0L
    init {
        nodeId = freshNode()
        salvo.core.set.addPlatform(hosted, nodeId, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.core.map.putPlatform(poolNode, 0, nodeId, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
    override fun nodeOfPool(pool: Int): Long {
        return nodeIn(poolNode, nodeId, pool)
    }
    override fun adoptPool(pool: Int, node: Long) {
        salvo.core.map.putPlatform(poolNode, pool, node, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
    override fun addNode(): Long {
        val n: Long = freshNode()
        salvo.core.set.addPlatform(hosted, n, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        return n
    }
    override fun hosts(node: Long): Boolean {
        return salvo.core.set.containsPlatform(hosted, node, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
    override fun identityOf(addr: Int, pool: Int): RemoteRef {
        return identityIn(remote, bits, poolNode, nodeId, addr, pool)
    }
    override fun findImport(r: RemoteRef, here: Long): Union3<Found, MakeProxy, MakeDead> {
        if ((r.node == here)) {
            val idx: Int = (r.actor).toInt()
            val b: Long? = salvo.core.map.getPlatform(bits, idx, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            if ((if ((!(salvo.core.map.containsKeyPlatform(remote, idx, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) && (b != null))) {
                val known: Long = b!!
                (known == r.bits)
            } else {
                false
            })) {
                val known: Long = b!!
                return Union3.U1<Found, MakeProxy, MakeDead>(Found(idx = idx))
            }
            if ((deadEntry >= 0)) {
                return Union3.U1<Found, MakeProxy, MakeDead>(Found(idx = deadEntry))
            }
            return Union3.U3<Found, MakeProxy, MakeDead>(MakeDead())
        }
        val p: Int? = salvo.core.map.getPlatform(proxies, r, ::hash__RemoteRef, ::eq__RemoteRef_RemoteRef)
        if ((p != null)) {
            val idx: Int = p!!
            return Union3.U1<Found, MakeProxy, MakeDead>(Found(idx = idx))
        }
        return Union3.U2<Found, MakeProxy, MakeDead>(MakeProxy())
    }
    override fun registerProxy(r: RemoteRef, idx: Int, here: Long): Int {
        val existing: Int? = salvo.core.map.getPlatform(proxies, r, ::hash__RemoteRef, ::eq__RemoteRef_RemoteRef)
        if ((existing != null)) {
            val e: Int = existing!!
            return e
        }
        salvo.core.map.putPlatform(remote, idx, r, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.core.map.putPlatform(proxies, r, idx, ::hash__RemoteRef, ::eq__RemoteRef_RemoteRef)
        salvo.core.map.putPlatform(credits, idx, 0, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        val frame: salvo.platform.core.bytes.Bytes = salvo.salvoEncode(Union5.U4<MsgFrame, AnswerFrame, GrantFrame, OpenFrame, ControlFrame>(OpenFrame(to = r.node, actor = r.actor, bits = r.bits, from = here)), salvo.Union5Codec(__Codec_MsgFrame, __Codec_AnswerFrame, __Codec_GrantFrame, __Codec_OpenFrame, __Codec_ControlFrame))
        stageIn(routes, outbound, outbox, parked, here, r.node, frame)
        return idx
    }
    override fun registerDead(idx: Int): Int {
        if ((deadEntry >= 0)) {
            return deadEntry
        }
        deadEntry = idx
        return deadEntry
    }
    override fun isProxy(addr: Int): Boolean {
        return salvo.core.map.containsKeyPlatform(remote, addr, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
    override fun proxyRef(addr: Int): RemoteRef? {
        val r: RemoteRef? = salvo.core.map.getPlatform(remote, addr, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((r != null)) {
            val found: RemoteRef = r!!
            return found
        }
        return null
    }
    override fun takeCredit(addr: Int, me: salvo.platform.runtime.Parker): Int {
        val c: Int? = salvo.core.map.getPlatform(credits, addr, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((c != null)) {
            val n: Int = c!!
            if ((n > 0)) {
                salvo.core.map.putPlatform(credits, addr, (n - 1), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
                salvo.core.map.putPlatform(heldN, addr, (heldIn(heldN, addr) + 1), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
                return 1
            }
            salvo.core.list.addPlatform(creditWaiters, me)
            return 0
        }
        return (-1)
    }
    override fun stage(from: Long, to: Long, frame: salvo.platform.core.bytes.Bytes) {
        stageIn(routes, outbound, outbox, parked, from, to, frame)
    }
    override fun grant(addr: Int, pool: Int, from: Long, n: Int) {
        salvo.core.map.putPlatform(heldN, addr, (heldIn(heldN, addr) + n), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        val me: RemoteRef = identityIn(remote, bits, poolNode, nodeId, addr, pool)
        val frame: salvo.platform.core.bytes.Bytes = salvo.salvoEncode(Union5.U3<MsgFrame, AnswerFrame, GrantFrame, OpenFrame, ControlFrame>(GrantFrame(to = from, host = me.node, actor = me.actor, bits = me.bits, n = n)), salvo.Union5Codec(__Codec_MsgFrame, __Codec_AnswerFrame, __Codec_GrantFrame, __Codec_OpenFrame, __Codec_ControlFrame))
        stageIn(routes, outbound, outbox, parked, me.node, from, frame)
    }
    override fun takeOutbox(): salvo.platform.core.list.MutList<Staged> {
        val out: salvo.platform.core.list.MutList<Staged> = mutableListOf<Staged>()
        while (true) {
            if (!((salvo.core.list.sizePlatform(outbox) > 0))) {
                break
            }
            salvo.core.list.addPlatform(out, run {
                val __nn_1: Staged? = salvo.core.list.removeAtPlatform(outbox, 0)
                when {
                    (__nn_1 == null) -> {
                        throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:327:22"))
                    }
                    else -> {
                        val __some_2: Staged = __nn_1!!
                        __some_2
                    }
                }
            })
        }
        return out
    }
    override fun addRoute(node: Long, at: salvo.platform.core.bytes.Bytes) {
        salvo.core.map.putPlatform(routes, node, at, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        restage(routes, outbound, outbox, parked)
    }
    override fun setOutbound(node: Long) {
        salvo.core.set.addPlatform(outbound, node, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        restage(routes, outbound, outbox, parked)
    }
    override fun hasOutbound(node: Long): Boolean {
        return salvo.core.set.containsPlatform(outbound, node, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
    override fun accepts(to: Long, actor: Long, claimed: Long): Boolean {
        if (!(salvo.core.set.containsPlatform(hosted, to, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            return false
        }
        val idx: Int = (actor).toInt()
        val b: Long? = salvo.core.map.getPlatform(bits, idx, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((if ((!(salvo.core.map.containsKeyPlatform(remote, idx, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) && (b != null))) {
            val known: Long = b!!
            (known == claimed)
        } else {
            false
        })) {
            val known: Long = b!!
            return true
        }
        return false
    }
    override fun received(idx: Int) {
        val h: Int = heldIn(heldN, idx)
        if ((h > 0)) {
            salvo.core.map.putPlatform(heldN, idx, (h - 1), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        }
    }
    override fun credited(r: RemoteRef, to: Long, n: Int): Boolean {
        if (!(salvo.core.set.containsPlatform(hosted, to, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            return false
        }
        val p: Int? = salvo.core.map.getPlatform(proxies, r, ::hash__RemoteRef, ::eq__RemoteRef_RemoteRef)
        if ((p != null)) {
            val idx: Int = p!!
            val i: Int = idx
            val c: Int? = salvo.core.map.getPlatform(credits, i, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            var now: Int = 0
            if ((c != null)) {
                val have: Int = c!!
                now = have
            }
            salvo.core.map.putPlatform(credits, i, (now + n), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            var h: Int = (heldIn(heldN, i) - n)
            if ((h < 0)) {
                h = 0
            }
            salvo.core.map.putPlatform(heldN, i, h, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            wakeSenders(creditWaiters)
            return true
        }
        return false
    }
    override fun held(idx: Int): Int {
        return heldIn(heldN, idx)
    }
    override fun putTask(key: Long, t: ExportedTask) {
        salvo.core.list.addPlatform(taskKeys, key)
        salvo.core.list.addPlatform(tasks, t)
    }
    override fun takeTask(key: Long): ExportedTask? {
        var i: Int = 0
        while (true) {
            if (!((i < salvo.core.list.sizePlatform(taskKeys)))) {
                break
            }
            if ((run {
                val __nn_1: Long? = salvo.core.list.getPlatform(taskKeys, i)
                when {
                    (__nn_1 == null) -> {
                        throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:401:16"))
                    }
                    else -> {
                        val __some_2: Long = __nn_1!!
                        __some_2
                    }
                }
            } == key)) {
                val _k: Long? = salvo.core.list.removeAtPlatform(taskKeys, i)
                return salvo.core.list.removeAtPlatform(tasks, i)
            }
            i = (i + 1)
        }
        return null
    }
    override fun watchChannel(node: Long, channel: String, sink: Int) {
        salvo.core.map.putPlatform(controls, ControlKey(node = node, channel = channel), sink, ::hash__ControlKey, ::eq__ControlKey_ControlKey)
    }
    override fun channelSink(node: Long, channel: String): Int {
        val s: Int? = salvo.core.map.getPlatform(controls, ControlKey(node = node, channel = channel), ::hash__ControlKey, ::eq__ControlKey_ControlKey)
        if ((s != null)) {
            val sink: Int = s!!
            return sink
        }
        return (-1)
    }
    override fun setProtocols(table: List<Pair<String, String>>) {
        local = table
    }
    override fun protocols(): List<Pair<String, String>> {
        return local
    }
    override fun setPeer(node: Long, table: List<Pair<String, String>>) {
        salvo.core.map.putPlatform(peers, node, table, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
    override fun peerHash(node: Long, protocol: String): String? {
        val t: List<Pair<String, String>>? = salvo.core.map.getPlatform(peers, node, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((t != null)) {
            val table: List<Pair<String, String>> = t!!
            for (entry in salvo.platform.core.list.each(table)) {
                val __destructured_1: Pair<String, String> = entry
                val name: String = __destructured_1.first
                val hash: String = __destructured_1.second
                if (((name) == (protocol))) {
                    return hash
                }
            }
        }
        return null
    }
    override fun forgetNode(node: Long): List<Int> {
        val _route: salvo.platform.core.bytes.Bytes? = salvo.core.map.removePlatform(routes, node, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        val _peer: List<Pair<String, String>>? = salvo.core.map.removePlatform(peers, node, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        val gone: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
        for (idx in salvo.platform.core.list.each(salvo.core.map.keysPlatform(remote))) {
            val r: RemoteRef? = salvo.core.map.getPlatform(remote, idx, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
            if ((r != null)) {
                val found: RemoteRef = r!!
                if ((found.node == node)) {
                    salvo.core.list.addPlatform(gone, idx)
                }
            }
        }
        wakeSenders(creditWaiters)
        return gone
    }
    override fun setView(group: Int, members: List<Int>) {
        salvo.core.map.putPlatform(views, group, members, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        bumpIn(versions, viewWaiters, group)
    }
    override fun viewOf(group: Int): List<Int> {
        val v: List<Int>? = salvo.core.map.getPlatform(views, group, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((v != null)) {
            val found: List<Int> = v!!
            return found
        }
        return listOf<Int>()
    }
    override fun versionOf(group: Int): Long {
        return versionIn(versions, group)
    }
    override fun bump(group: Int) {
        bumpIn(versions, viewWaiters, group)
    }
    override fun addViewWaiter(group: Int, seen: Long, me: salvo.platform.runtime.Parker): Long {
        if ((versionIn(versions, group) != seen)) {
            return (-1L)
        }
        nextWaiter = (nextWaiter + 1L)
        salvo.core.list.addPlatform(viewWaiters, ViewWaiter(group = group, id = nextWaiter, parker = me))
        return nextWaiter
    }
    override fun dropViewWaiter(id: Long) {
        var i: Int = 0
        while (true) {
            if (!((i < salvo.core.list.sizePlatform(viewWaiters)))) {
                break
            }
            if ((run {
                val __proj_3: ViewWaiter = run {
                    val __nn_1: ViewWaiter? = salvo.core.list.getPlatform(viewWaiters, i)
                    when {
                        (__nn_1 == null) -> {
                            throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:496:16"))
                        }
                        else -> {
                            val __some_2: ViewWaiter = __nn_1!!
                            __some_2
                        }
                    }
                }
                __proj_3.id
            } == id)) {
                val _w: ViewWaiter? = salvo.core.list.removeAtPlatform(viewWaiters, i)
                return
            }
            i = (i + 1)
        }
    }
    override fun creditsOf(addr: Int): Int? {
        val c: Int? = salvo.core.map.getPlatform(credits, addr, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((c != null)) {
            val n: Int = c!!
            return n
        }
        return null
    }
}

fun nodeIn(poolNode: salvo.platform.core.map.Map<Int, Long>, nodeId: Long, pool: Int): Long {
    val n: Long? = salvo.core.map.getPlatform(poolNode, pool, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((n != null)) {
        val v: Long = n!!
        return v
    }
    return nodeId
}

fun heldIn(heldN: salvo.platform.core.map.Map<Int, Int>, idx: Int): Int {
    val h: Int? = salvo.core.map.getPlatform(heldN, idx, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((h != null)) {
        val n: Int = h!!
        return n
    }
    return 0
}

fun identityIn(remote: salvo.platform.core.map.Map<Int, RemoteRef>, bits: salvo.platform.core.map.MutMap<Int, Long>, poolNode: salvo.platform.core.map.Map<Int, Long>, nodeId: Long, addr: Int, pool: Int): RemoteRef {
    val r: RemoteRef? = salvo.core.map.getPlatform(remote, addr, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((r != null)) {
        val found: RemoteRef = r!!
        return found
    }
    val n: Long = nodeIn(poolNode, nodeId, pool)
    val b: Long? = salvo.core.map.getPlatform(bits, addr, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((b != null)) {
        val known: Long = b!!
        return RemoteRef(node = n, actor = (addr).toLong(), bits = known)
    }
    val minted: Long = salvo.runtime.identityBits()
    salvo.core.map.putPlatform(bits, addr, minted, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    return RemoteRef(node = n, actor = (addr).toLong(), bits = minted)
}

fun wakeSenders(waiters: salvo.platform.core.list.MutList<salvo.platform.runtime.Parker>) {
    while (true) {
        if (!((salvo.core.list.sizePlatform(waiters) > 0))) {
            break
        }
        salvo.runtime.unparkPlatform(run {
            val __nn_1: salvo.platform.runtime.Parker? = salvo.core.list.removeAtPlatform(waiters, 0)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:554:16"))
                }
                else -> {
                    val __some_2: salvo.platform.runtime.Parker = __nn_1!!
                    __some_2
                }
            }
        })
    }
}

fun stageIn(routes: salvo.platform.core.map.Map<Long, salvo.platform.core.bytes.Bytes>, outbound: salvo.platform.core.set.Set<Long>, outbox: salvo.platform.core.list.MutList<Staged>, parked: salvo.platform.core.list.MutList<Parked>, from: Long, to: Long, frame: salvo.platform.core.bytes.Bytes) {
    val ep: salvo.platform.core.bytes.Bytes? = salvo.core.map.getPlatform(routes, to, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((ep != null)) {
        val at: salvo.platform.core.bytes.Bytes = ep!!
        if (salvo.core.set.containsPlatform(outbound, from, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            salvo.core.list.addPlatform(outbox, Staged(from = from, to = at, frame = frame))
            return
        }
    }
    salvo.core.list.addPlatform(parked, Parked(from = from, to = to, frame = frame))
}

fun restage(routes: salvo.platform.core.map.Map<Long, salvo.platform.core.bytes.Bytes>, outbound: salvo.platform.core.set.Set<Long>, outbox: salvo.platform.core.list.MutList<Staged>, parked: salvo.platform.core.list.MutList<Parked>) {
    val waiting: salvo.platform.core.list.MutList<Parked> = mutableListOf<Parked>()
    while (true) {
        if (!((salvo.core.list.sizePlatform(parked) > 0))) {
            break
        }
        salvo.core.list.addPlatform(waiting, run {
            val __nn_1: Parked? = salvo.core.list.removeAtPlatform(parked, 0)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:577:22"))
                }
                else -> {
                    val __some_2: Parked = __nn_1!!
                    __some_2
                }
            }
        })
    }
    for (p in salvo.platform.core.list.each(waiting)) {
        stageIn(routes, outbound, outbox, parked, p.from, p.to, p.frame)
    }
}

fun freshNode(): Long {
    val b: Long = salvo.runtime.identityBits()
    if ((b < 0L)) {
        return (-(b + 1L))
    }
    return b
}

fun hereNode(): Long {
    return __module_use0_0.nodeOfPool(salvo.runtime.currentPool())
}

fun adopt(pool: Int) {
    __module_use0_0.adoptPool(pool, hereNode())
}

fun newNode(): Long {
    return __module_use0_0.addNode()
}

fun poolAt(node: Long, n: Int): Int {
    val p: Int = salvo.runtime.newPoolOf(n, (-1))
    __module_use0_0.adoptPool(p, node)
    return p
}

fun identity(addr: Int): RemoteRef {
    return __module_use0_0.identityOf(addr, salvo.runtime.actorPool(addr))
}

fun sameActor(a: Int, b: Int): Boolean {
    if ((a == b)) {
        return true
    }
    if ((!(__module_use0_0.isProxy(a)) && !(__module_use0_0.isProxy(b)))) {
        return false
    }
    return eq__RemoteRef_RemoteRef(identity(a), identity(b))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun importAddr(node: Long, actor: Long, bits: Long): Int {
    val r: RemoteRef = RemoteRef(node = node, actor = actor, bits = bits)
    val here: Long = hereNode()
    val found: Union3<Found, MakeProxy, MakeDead> = __module_use0_0.findImport(r, here)
    if ((found is Union3.U1<*, *, *>)) {
        val f: Found = ((found as Union3.U1<*, *, *>).value as Found)
        return f.idx
    }
    val idx: Int = salvo.runtime.spawnInert()
    if ((found is Union3.U3<*, *, *>)) {
        val found_1: MakeDead = ((found as Union3.U3<*, *, *>).value as MakeDead)
        salvo.runtime.killActor(idx, "unknown identity")
        return __module_use0_0.registerDead(idx)
    }
    val got: Int = __module_use0_0.registerProxy(r, idx, here)
    if ((got == idx)) {
        salvo.runtime.markProxy(idx)
    }
    flush()
    return got
}

fun remote(addr: Int): Boolean {
    return __module_use0_0.isProxy(addr)
}

fun sendRemote(addr: Int, proto: String, payload: salvo.platform.core.bytes.Bytes) {
    val r: RemoteRef? = __module_use0_0.proxyRef(addr)
    if ((r == null)) {
        return
    }
    while (true) {
        if (!(true)) {
            break
        }
        val got: Int = __module_use0_0.takeCredit(addr, salvo.runtime.thisParkerPlatform())
        if ((got == 1)) {
            val from: Long = hereNode()
            val r_1: RemoteRef = r!!
            val frame: salvo.platform.core.bytes.Bytes = salvo.salvoEncode(Union5.U1<MsgFrame, AnswerFrame, GrantFrame, OpenFrame, ControlFrame>(MsgFrame(to = r_1.node, actor = r_1.actor, bits = r_1.bits, from = from, proto = proto, payload = payload)), salvo.Union5Codec(__Codec_MsgFrame, __Codec_AnswerFrame, __Codec_GrantFrame, __Codec_OpenFrame, __Codec_ControlFrame))
            __module_use0_0.stage(from, r_1.node, frame)
            flush()
            return
        }
        if (((got < 0) || salvo.runtime.mailboxDead(addr))) {
            return
        }
        salvo.runtime.parkPlatform(salvo.runtime.thisParkerPlatform())
        if (salvo.runtime.mailboxDead(addr)) {
            return
        }
    }
}

fun answerRemote(t: ReplyParts, payload: salvo.platform.core.bytes.Bytes) {
    val from: Long = hereNode()
    val frame: salvo.platform.core.bytes.Bytes = salvo.salvoEncode(Union5.U2<MsgFrame, AnswerFrame, GrantFrame, OpenFrame, ControlFrame>(AnswerFrame(to = t.node, kind = t.kind, id = t.id, slot = t.slot, bits = t.bits, payload = payload)), salvo.Union5Codec(__Codec_MsgFrame, __Codec_AnswerFrame, __Codec_GrantFrame, __Codec_OpenFrame, __Codec_ControlFrame))
    __module_use0_0.stage(from, t.node, frame)
    flush()
}

fun exportReply(e: salvo.runtime.Exported): ReplyParts {
    val __destructured_1: salvo.runtime.Exported = e
    val kind: Int = __destructured_1.kind
    val id: Int = __destructured_1.id
    val slot: Long = __destructured_1.slot
    val body: salvo.platform.runtime.Body? = __destructured_1.body
    if ((kind == 2)) {
        if ((body != null)) {
            val b: salvo.platform.runtime.Body = body!!
            __module_use0_0.putTask(slot, ExportedTask(pool = id, body = b))
        }
        return ReplyParts(node = __module_use0_0.nodeOfPool(id), kind = 2, id = slot, slot = slot, bits = 0L)
    }
    if ((body != null)) {
        val b: salvo.platform.runtime.Body = body!!
        salvo.runtime.dropBodyPlatform(b)
    }
    if ((kind == 1)) {
        return ReplyParts(node = __module_use0_0.nodeOfPool(salvo.runtime.waiterPool(id)), kind = 1, id = (id).toLong(), slot = slot, bits = 0L)
    }
    val me: RemoteRef = identity(id)
    return ReplyParts(node = me.node, kind = 0, id = me.actor, slot = slot, bits = me.bits)
}

fun creditBack(addr: Int, pool: Int, from: Long) {
    __module_use0_0.grant(addr, pool, from, 1)
}

fun flush() {
    val out: salvo.platform.core.list.MutList<Staged> = __module_use0_0.takeOutbox()
    for (s in salvo.platform.core.list.each(out)) {
        wireOutPlatform(s.from, s.to, s.frame)
    }
}

fun route(node: Long, at: salvo.platform.core.bytes.Bytes) {
    __module_use0_0.addRoute(node, at)
    flush()
}

fun outboundBound() {
    __module_use0_0.setOutbound(hereNode())
    flush()
}

fun connected(): Boolean {
    return __module_use0_0.hasOutbound(hereNode())
}

fun credits(addr: Int): Int? {
    return __module_use0_0.creditsOf(addr)
}

fun pending(addr: Int): Int {
    if (__module_use0_0.isProxy(addr)) {
        return __module_use0_0.held(addr)
    }
    return salvo.runtime.mailboxQueued(addr)
}

fun watchControl(channel: String, sink: Int) {
    __module_use0_0.watchChannel(hereNode(), channel, sink)
}

fun sendControl(to: Long, channel: String, payload: salvo.platform.core.bytes.Bytes) {
    val from: Long = hereNode()
    val frame: salvo.platform.core.bytes.Bytes = salvo.salvoEncode(Union5.U5<MsgFrame, AnswerFrame, GrantFrame, OpenFrame, ControlFrame>(ControlFrame(to = to, from = from, channel = channel, payload = payload)), salvo.Union5Codec(__Codec_MsgFrame, __Codec_AnswerFrame, __Codec_GrantFrame, __Codec_OpenFrame, __Codec_ControlFrame))
    __module_use0_0.stage(from, to, frame)
    flush()
}

fun controlFrame(channel: String, payload: salvo.platform.core.bytes.Bytes): salvo.platform.core.bytes.Bytes {
    return salvo.salvoEncode(Union5.U5<MsgFrame, AnswerFrame, GrantFrame, OpenFrame, ControlFrame>(ControlFrame(to = 0L, from = hereNode(), channel = channel, payload = payload)), salvo.Union5Codec(__Codec_MsgFrame, __Codec_AnswerFrame, __Codec_GrantFrame, __Codec_OpenFrame, __Codec_ControlFrame))
}

fun nodeLeft(node: Long) {
    for (idx in salvo.platform.core.list.each(__module_use0_0.forgetNode(node))) {
        salvo.runtime.killActor(idx, "node left")
    }
}

fun registerProtocols(table: List<Pair<String, String>>) {
    __module_use0_0.setProtocols(table)
}

fun localProtocols(): List<Pair<String, String>> {
    return __module_use0_0.protocols()
}

fun setPeerProtocols(node: Long, table: List<Pair<String, String>>) {
    __module_use0_0.setPeer(node, table)
}

fun peerProtocol(node: Long, protocol: String): String? {
    return __module_use0_0.peerHash(node, protocol)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun deliver(data: salvo.platform.core.bytes.Bytes): Boolean {
    val f: Union5<MsgFrame, AnswerFrame, GrantFrame, OpenFrame, ControlFrame>? = salvo.salvoDecode(data, salvo.Union5Codec(__Codec_MsgFrame, __Codec_AnswerFrame, __Codec_GrantFrame, __Codec_OpenFrame, __Codec_ControlFrame))
    if ((f is Union5.U1<*, *, *, *, *>)) {
        val m: MsgFrame = ((f as Union5.U1<*, *, *, *, *>).value as MsgFrame)
        if (!(__module_use0_0.accepts(m.to, m.actor, m.bits))) {
            return false
        }
        val idx: Int = (m.actor).toInt()
        val msg: salvo.platform.runtime.Dyn? = decodeMessagePlatform(idx, m.proto, m.payload)
        if ((msg != null)) {
            val v: salvo.platform.runtime.Dyn = msg!!
            __module_use0_0.received(idx)
            val _queued: Boolean = salvo.runtime.deliverRemote(idx, v, m.from)
            return true
        }
        return false
    }
    if ((f is Union5.U2<*, *, *, *, *>)) {
        val a: AnswerFrame = ((f as Union5.U2<*, *, *, *, *>).value as AnswerFrame)
        return deliverAnswer(a)
    }
    if ((f is Union5.U3<*, *, *, *, *>)) {
        val g: GrantFrame = ((f as Union5.U3<*, *, *, *, *>).value as GrantFrame)
        return __module_use0_0.credited(RemoteRef(node = g.host, actor = g.actor, bits = g.bits), g.to, g.n)
    }
    if ((f is Union5.U4<*, *, *, *, *>)) {
        val o: OpenFrame = ((f as Union5.U4<*, *, *, *, *>).value as OpenFrame)
        if (!(__module_use0_0.accepts(o.to, o.actor, o.bits))) {
            return false
        }
        val idx: Int = (o.actor).toInt()
        var room: Int = (salvo.runtime.mailboxRoom(idx) - __module_use0_0.held(idx))
        if ((room < 1)) {
            room = 1
        }
        __module_use0_0.grant(idx, salvo.runtime.actorPool(idx), o.from, room)
        flush()
        return true
    }
    if ((f is Union5.U5<*, *, *, *, *>)) {
        val c: ControlFrame = ((f as Union5.U5<*, *, *, *, *>).value as ControlFrame)
        var node: Long = c.to
        if ((node == 0L)) {
            node = hereNode()
        }
        if (!(__module_use0_0.hosts(node))) {
            return false
        }
        val sink: Int = __module_use0_0.channelSink(node, c.channel)
        if ((sink < 0)) {
            return false
        }
        val msg: salvo.platform.runtime.Dyn? = controlMessagePlatform(sink, c.from, c.payload)
        if ((msg != null)) {
            val v: salvo.platform.runtime.Dyn = msg!!
            val queued: Boolean = salvo.runtime.deliverRemote(sink, v, ((-1)).toLong())
            return queued
        }
        return false
    }
    return false
}

fun deliverAnswer(a: AnswerFrame): Boolean {
    if (!(__module_use0_0.hosts(a.to))) {
        return false
    }
    if ((a.kind == 0)) {
        if (!(__module_use0_0.accepts(a.to, a.id, a.bits))) {
            return false
        }
        salvo.runtime.answer(salvo.runtime.tokenToActor((a.id).toInt(), a.slot), rawAnswerPlatform(a.payload))
        return true
    }
    if ((a.kind == 1)) {
        val wid: Int = (a.id).toInt()
        val v: salvo.platform.runtime.Dyn? = decodeWaiterAnswerPlatform(wid, a.payload)
        if ((v != null)) {
            val value: salvo.platform.runtime.Dyn = v!!
            salvo.runtime.answer(salvo.runtime.tokenToWaiter(wid, a.slot), value)
            return true
        }
        return false
    }
    val t: ExportedTask? = __module_use0_0.takeTask(a.id)
    if ((t != null)) {
        val task: ExportedTask = t!!
        val v: salvo.platform.runtime.Dyn? = decodeTaskAnswerPlatform(a.id, a.payload)
        val __destructured_1: ExportedTask = task
        val pool: Int = __destructured_1.pool
        val body: salvo.platform.runtime.Body = __destructured_1.body
        if ((v != null)) {
            val value: salvo.platform.runtime.Dyn = v!!
            salvo.runtime.answer(salvo.runtime.mintTaskOn(pool, body), value)
            return true
        }
        salvo.runtime.dropBodyPlatform(body)
    }
    return false
}

fun viewSet(group: Int, members: List<Int>) {
    __module_use0_0.setView(group, members)
}

fun viewMembers(group: Int): List<Int> {
    val left: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    for (m in salvo.platform.core.list.each(__module_use0_0.viewOf(group))) {
        salvo.core.list.addPlatform(left, m)
    }
    val out: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    while (true) {
        if (!((salvo.core.list.sizePlatform(left) > 0))) {
            break
        }
        var best: Int = 0
        var i: Int = 1
        while (true) {
            if (!((i < salvo.core.list.sizePlatform(left)))) {
                break
            }
            if (before(identity(run {
                val __nn_1: Int? = salvo.core.list.getPlatform(left, i)
                when {
                    (__nn_1 == null) -> {
                        throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:937:37"))
                    }
                    else -> {
                        val __some_2: Int = __nn_1!!
                        __some_2
                    }
                }
            }), identity(run {
                val __nn_3: Int? = salvo.core.list.getPlatform(left, best)
                when {
                    (__nn_3 == null) -> {
                        throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:937:68"))
                    }
                    else -> {
                        val __some_4: Int = __nn_3!!
                        __some_4
                    }
                }
            }))) {
                best = i
            }
            i = (i + 1)
        }
        salvo.core.list.addPlatform(out, run {
            val __nn_5: Int? = salvo.core.list.removeAtPlatform(left, best)
            when {
                (__nn_5 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at runtime.routing:942:18"))
                }
                else -> {
                    val __some_6: Int = __nn_5!!
                    __some_6
                }
            }
        })
    }
    return out.toMutableList()
}

fun before(a: RemoteRef, b: RemoteRef): Boolean {
    if ((a.node != b.node)) {
        return (a.node < b.node)
    }
    return (a.actor <= b.actor)
}

fun viewVersion(group: Int): Long {
    return __module_use0_0.versionOf(group)
}

fun viewRefresh(group: Int) {
    __module_use0_0.bump(group)
}

fun viewWait(group: Int, seen: Long, nanos: Long) {
    val me: salvo.platform.runtime.Parker = salvo.runtime.thisParkerPlatform()
    val id: Long = __module_use0_0.addViewWaiter(group, seen, me)
    if ((id < 0L)) {
        return
    }
    salvo.runtime.parkNanosPlatform(me, nanos)
    __module_use0_0.dropViewWaiter(id)
}

fun hash__RemoteRef(value: RemoteRef): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, (value.node).hashCode().toLong())
    h = salvo.core.compare.mixHash(h, (value.actor).hashCode().toLong())
    h = salvo.core.compare.mixHash(h, (value.bits).hashCode().toLong())
    return h
}

fun eq__RemoteRef_RemoteRef(a: RemoteRef, b: RemoteRef): Boolean {
    if (!(((a.node) == (b.node)))) {
        return false
    }
    if (!(((a.actor) == (b.actor)))) {
        return false
    }
    if (!(((a.bits) == (b.bits)))) {
        return false
    }
    return true
}

fun hash__ControlKey(value: ControlKey): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, (value.node).hashCode().toLong())
    h = salvo.core.compare.mixHash(h, (value.channel).hashCode().toLong())
    return h
}

fun eq__ControlKey_ControlKey(a: ControlKey, b: ControlKey): Boolean {
    if (!(((a.node) == (b.node)))) {
        return false
    }
    if (!(((a.channel) == (b.channel)))) {
        return false
    }
    return true
}

