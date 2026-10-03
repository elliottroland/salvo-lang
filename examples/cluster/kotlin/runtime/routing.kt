package salvo.runtime.routing

import salvo.*
import salvo.core.actor.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.deque.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.runtime.*

// [mod-use] The module's `use` #0, bound on first use.
private val __moduleUse0: RtRouteTable by lazy {
    val rt_route_table: RtRouteTable = __Mon_RtRouteTable(RtRoutes().also { it.init() })
    rt_route_table
}

fun decodeMessagePlatform(addr: Int, proto: String, payload: salvo.SalvoBytes): salvo.platform.runtime.Dyn? {
    return salvo.platform.runtime.routing.decodeMessage(addr, proto, payload)
}

fun decodeWaiterAnswerPlatform(wid: Int, payload: salvo.SalvoBytes): salvo.platform.runtime.Dyn? {
    return salvo.platform.runtime.routing.decodeWaiterAnswer(wid, payload)
}

fun decodeTaskAnswerPlatform(key: Long, payload: salvo.SalvoBytes): salvo.platform.runtime.Dyn? {
    return salvo.platform.runtime.routing.decodeTaskAnswer(key, payload)
}

fun rawAnswerPlatform(payload: salvo.SalvoBytes): salvo.platform.runtime.Dyn {
    return salvo.platform.runtime.routing.rawAnswer(payload)
}

fun controlMessagePlatform(sink: Int, from: Long, payload: salvo.SalvoBytes): salvo.platform.runtime.Dyn? {
    return salvo.platform.runtime.routing.controlMessage(sink, from, payload)
}

fun wireOutPlatform(from: Long, to: salvo.SalvoBytes, frame: salvo.SalvoBytes) {
    return salvo.platform.runtime.routing.wireOut(from, to, frame)
}

data class RtMsgFrame(
    val to: Long,
    val actor: Long,
    val bits: Long,
    val from: Long,
    val proto: String,
    val payload: salvo.SalvoBytes,
)

object __Codec_RtMsgFrame : salvo.WireCodec<RtMsgFrame> {
    override fun enc(v: RtMsgFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.LongCodec.enc(v.actor, out)
        salvo.LongCodec.enc(v.bits, out)
        salvo.LongCodec.enc(v.from, out)
        salvo.StrCodec.enc(v.proto, out)
        salvo.BytesCodec.enc(v.payload, out)
    }
    override fun dec(inp: salvo.WireIn): RtMsgFrame = RtMsgFrame(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class RtAnswerFrame(
    val to: Long,
    val kind: Int,
    val id: Long,
    val slot: Long,
    val bits: Long,
    val payload: salvo.SalvoBytes,
)

object __Codec_RtAnswerFrame : salvo.WireCodec<RtAnswerFrame> {
    override fun enc(v: RtAnswerFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.IntCodec.enc(v.kind, out)
        salvo.LongCodec.enc(v.id, out)
        salvo.LongCodec.enc(v.slot, out)
        salvo.LongCodec.enc(v.bits, out)
        salvo.BytesCodec.enc(v.payload, out)
    }
    override fun dec(inp: salvo.WireIn): RtAnswerFrame = RtAnswerFrame(salvo.LongCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class RtGrantFrame(
    val to: Long,
    val host: Long,
    val actor: Long,
    val bits: Long,
    val n: Int,
)

object __Codec_RtGrantFrame : salvo.WireCodec<RtGrantFrame> {
    override fun enc(v: RtGrantFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.LongCodec.enc(v.host, out)
        salvo.LongCodec.enc(v.actor, out)
        salvo.LongCodec.enc(v.bits, out)
        salvo.IntCodec.enc(v.n, out)
    }
    override fun dec(inp: salvo.WireIn): RtGrantFrame = RtGrantFrame(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.IntCodec.dec(inp))
}

data class RtOpenFrame(
    val to: Long,
    val actor: Long,
    val bits: Long,
    val from: Long,
)

object __Codec_RtOpenFrame : salvo.WireCodec<RtOpenFrame> {
    override fun enc(v: RtOpenFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.LongCodec.enc(v.actor, out)
        salvo.LongCodec.enc(v.bits, out)
        salvo.LongCodec.enc(v.from, out)
    }
    override fun dec(inp: salvo.WireIn): RtOpenFrame = RtOpenFrame(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp))
}

data class RtControlFrame(
    val to: Long,
    val from: Long,
    val channel: String,
    val payload: salvo.SalvoBytes,
)

object __Codec_RtControlFrame : salvo.WireCodec<RtControlFrame> {
    override fun enc(v: RtControlFrame, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.to, out)
        salvo.LongCodec.enc(v.from, out)
        salvo.StrCodec.enc(v.channel, out)
        salvo.BytesCodec.enc(v.payload, out)
    }
    override fun dec(inp: salvo.WireIn): RtControlFrame = RtControlFrame(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class RtRemoteRef(
    val node: Long,
    val actor: Long,
    val bits: Long,
)

object __Codec_RtRemoteRef : salvo.WireCodec<RtRemoteRef> {
    override fun enc(v: RtRemoteRef, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.node, out)
        salvo.LongCodec.enc(v.actor, out)
        salvo.LongCodec.enc(v.bits, out)
    }
    override fun dec(inp: salvo.WireIn): RtRemoteRef = RtRemoteRef(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp))
}

data class RtReplyParts(
    val node: Long,
    val kind: Int,
    val id: Long,
    val slot: Long,
    val bits: Long,
)

object __Codec_RtReplyParts : salvo.WireCodec<RtReplyParts> {
    override fun enc(v: RtReplyParts, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.node, out)
        salvo.IntCodec.enc(v.kind, out)
        salvo.LongCodec.enc(v.id, out)
        salvo.LongCodec.enc(v.slot, out)
        salvo.LongCodec.enc(v.bits, out)
    }
    override fun dec(inp: salvo.WireIn): RtReplyParts = RtReplyParts(salvo.LongCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp))
}

data class RtControlKey(
    val node: Long,
    val channel: String,
)

object __Codec_RtControlKey : salvo.WireCodec<RtControlKey> {
    override fun enc(v: RtControlKey, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.node, out)
        salvo.StrCodec.enc(v.channel, out)
    }
    override fun dec(inp: salvo.WireIn): RtControlKey = RtControlKey(salvo.LongCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class RtParked(
    val from: Long,
    val to: Long,
    val frame: salvo.SalvoBytes,
)

object __Codec_RtParked : salvo.WireCodec<RtParked> {
    override fun enc(v: RtParked, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.from, out)
        salvo.LongCodec.enc(v.to, out)
        salvo.BytesCodec.enc(v.frame, out)
    }
    override fun dec(inp: salvo.WireIn): RtParked = RtParked(salvo.LongCodec.dec(inp), salvo.LongCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class RtStaged(
    val from: Long,
    val to: salvo.SalvoBytes,
    val frame: salvo.SalvoBytes,
)

object __Codec_RtStaged : salvo.WireCodec<RtStaged> {
    override fun enc(v: RtStaged, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.from, out)
        salvo.BytesCodec.enc(v.to, out)
        salvo.BytesCodec.enc(v.frame, out)
    }
    override fun dec(inp: salvo.WireIn): RtStaged = RtStaged(salvo.LongCodec.dec(inp), salvo.BytesCodec.dec(inp), salvo.BytesCodec.dec(inp))
}

data class RtExportedTask(
    val pool: Int,
    val body: salvo.platform.runtime.RtBody,
)

fun dropExportedTask(t: RtExportedTask) {
    val __destructured1 = t
    val pool = __destructured1.pool
    val body = __destructured1.body
    dropBodyPlatform(body)
}

data class RtFound(
    val idx: Int,
)

object __Codec_RtFound : salvo.WireCodec<RtFound> {
    override fun enc(v: RtFound, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.idx, out)
    }
    override fun dec(inp: salvo.WireIn): RtFound = RtFound(salvo.IntCodec.dec(inp))
}

class RtMakeProxy

object __Codec_RtMakeProxy : salvo.WireCodec<RtMakeProxy> {
    override fun enc(v: RtMakeProxy, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): RtMakeProxy = RtMakeProxy()
}

class RtMakeDead

object __Codec_RtMakeDead : salvo.WireCodec<RtMakeDead> {
    override fun enc(v: RtMakeDead, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): RtMakeDead = RtMakeDead()
}

interface RtRouteTable {
    fun nodeOfPool(pool: Int): Long
    fun adoptPool(pool: Int, node: Long)
    fun addNode(): Long
    fun hosts(node: Long): Boolean
    fun identityOf(addr: Int, pool: Int): RtRemoteRef
    fun findImport(r: RtRemoteRef, here: Long): Union3<RtFound, RtMakeProxy, RtMakeDead>
    fun registerProxy(r: RtRemoteRef, idx: Int, here: Long): Int
    fun registerDead(idx: Int): Int
    fun isProxy(addr: Int): Boolean
    fun proxyRef(addr: Int): RtRemoteRef?
    fun takeCredit(addr: Int, me: salvo.platform.runtime.Parker): Int
    fun stage(from: Long, to: Long, frame: salvo.SalvoBytes)
    fun grant(addr: Int, pool: Int, from: Long, n: Int)
    fun takeOutbox(): MutableList<RtStaged>
    fun addRoute(node: Long, at: salvo.SalvoBytes)
    fun setOutbound(node: Long)
    fun hasOutbound(node: Long): Boolean
    fun accepts(to: Long, actor: Long, claimed: Long): Boolean
    fun received(idx: Int)
    fun credited(r: RtRemoteRef, to: Long, n: Int): Boolean
    fun held(idx: Int): Int
    fun putTask(key: Long, t: RtExportedTask)
    fun takeTask(key: Long): RtExportedTask?
    fun watchChannel(node: Long, channel: String, sink: Int)
    fun channelSink(node: Long, channel: String): Int
    fun setProtocols(table: List<Pair<String, String>>)
    fun protocols(): List<Pair<String, String>>
    fun setPeer(node: Long, table: List<Pair<String, String>>)
    fun peerHash(node: Long, protocol: String): String?
    fun forgetNode(node: Long): List<Int>
    fun creditsOf(addr: Int): Int?
}

class __Mon_RtRouteTable(
    private val inner: RtRouteTable,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : RtRouteTable {
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
    override fun identityOf(addr: Int, pool: Int): RtRemoteRef {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.identityOf(addr, pool) } finally { lock.unlock() }
    }
    override fun findImport(r: RtRemoteRef, here: Long): Union3<RtFound, RtMakeProxy, RtMakeDead> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.findImport(r, here) } finally { lock.unlock() }
    }
    override fun registerProxy(r: RtRemoteRef, idx: Int, here: Long): Int {
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
    override fun proxyRef(addr: Int): RtRemoteRef? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.proxyRef(addr) } finally { lock.unlock() }
    }
    override fun takeCredit(addr: Int, me: salvo.platform.runtime.Parker): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.takeCredit(addr, me) } finally { lock.unlock() }
    }
    override fun stage(from: Long, to: Long, frame: salvo.SalvoBytes) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.stage(from, to, frame) } finally { lock.unlock() }
    }
    override fun grant(addr: Int, pool: Int, from: Long, n: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.grant(addr, pool, from, n) } finally { lock.unlock() }
    }
    override fun takeOutbox(): MutableList<RtStaged> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.takeOutbox() } finally { lock.unlock() }
    }
    override fun addRoute(node: Long, at: salvo.SalvoBytes) {
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
    override fun credited(r: RtRemoteRef, to: Long, n: Int): Boolean {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.credited(r, to, n) } finally { lock.unlock() }
    }
    override fun held(idx: Int): Int {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.held(idx) } finally { lock.unlock() }
    }
    override fun putTask(key: Long, t: RtExportedTask) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.putTask(key, t) } finally { lock.unlock() }
    }
    override fun takeTask(key: Long): RtExportedTask? {
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
}

class RtRoutes : RtRouteTable {
    private var nodeId: Long = 0L
    private var hosted: MutableSet<Long> = linkedSetOf<Long>().also { __s -> __s.addAll(listOf()) }
    private var poolNode: MutableMap<Int, Long> = linkedMapOf<Int, Long>().also { __m -> __m.putAll(listOf()) }
    private var bits: MutableMap<Int, Long> = linkedMapOf<Int, Long>().also { __m -> __m.putAll(listOf()) }
    private var remote: MutableMap<Int, RtRemoteRef> = linkedMapOf<Int, RtRemoteRef>().also { __m -> __m.putAll(listOf()) }
    private var proxies: MutableMap<RtRemoteRef, Int> = salvo.SalvoHashMap<RtRemoteRef, Int>(::hash__4, ::eq__4).also { __m -> __m.putAll(listOf()) }
    private var credits: MutableMap<Int, Int> = linkedMapOf<Int, Int>().also { __m -> __m.putAll(listOf()) }
    private var heldN: MutableMap<Int, Int> = linkedMapOf<Int, Int>().also { __m -> __m.putAll(listOf()) }
    private var routes: MutableMap<Long, salvo.SalvoBytes> = linkedMapOf<Long, salvo.SalvoBytes>().also { __m -> __m.putAll(listOf()) }
    private var outbound: MutableSet<Long> = linkedSetOf<Long>().also { __s -> __s.addAll(listOf()) }
    private var parked: MutableList<RtParked> = mutableListOf<RtParked>()
    private var outbox: MutableList<RtStaged> = mutableListOf<RtStaged>()
    private var taskKeys: MutableList<Long> = mutableListOf<Long>()
    private var tasks: MutableList<RtExportedTask> = mutableListOf<RtExportedTask>()
    private var controls: MutableMap<RtControlKey, Int> = salvo.SalvoHashMap<RtControlKey, Int>(::hash__5, ::eq__5).also { __m -> __m.putAll(listOf()) }
    private var local: List<Pair<String, String>> = listOf<Pair<String, String>>()
    private var peers: MutableMap<Long, List<Pair<String, String>>> = linkedMapOf<Long, List<Pair<String, String>>>().also { __m -> __m.putAll(listOf()) }
    private var deadEntry: Int = -1
    private var creditWaiters: MutableList<salvo.platform.runtime.Parker> = mutableListOf<salvo.platform.runtime.Parker>()

    override fun nodeOfPool(pool: Int): Long {
        return nodeIn(poolNode, nodeId, pool)
    }

    override fun adoptPool(pool: Int, node: Long) {
        poolNode.put(pool, node)
    }

    override fun addNode(): Long {
        val n = freshNode()
        hosted.add(n)
        return n
    }

    override fun hosts(node: Long): Boolean {
        return hosted.contains(node)
    }

    override fun identityOf(addr: Int, pool: Int): RtRemoteRef {
        return identityIn(remote, bits, poolNode, nodeId, addr, pool)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun findImport(r: RtRemoteRef, here: Long): Union3<RtFound, RtMakeProxy, RtMakeDead> {
        if (r.node == here) {
            val idx = (r.actor).toInt()
            val b = bits[idx]
            if (!remote.containsKey(idx) && (b != null)) {
                val known = b as Long
                if (known == r.bits) {
                    return Union3.U1<RtFound, RtMakeProxy, RtMakeDead>(RtFound(idx = idx))
                }
            }
            if (deadEntry >= 0) {
                return Union3.U1<RtFound, RtMakeProxy, RtMakeDead>(RtFound(idx = deadEntry))
            }
            return Union3.U3<RtFound, RtMakeProxy, RtMakeDead>(RtMakeDead())
        }
        val p = proxies[r]
        if (p != null) {
            val idx = p as Int
            return Union3.U1<RtFound, RtMakeProxy, RtMakeDead>(RtFound(idx = idx))
        }
        return Union3.U2<RtFound, RtMakeProxy, RtMakeDead>(RtMakeProxy())
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun registerProxy(r: RtRemoteRef, idx: Int, here: Long): Int {
        val existing = proxies[r]
        if (existing != null) {
            val e = existing as Int
            return e
        }
        remote.put(idx, r)
        proxies.put(r, idx)
        credits.put(idx, 0)
        val frame = salvo.salvoEncode(Union5.U4<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>(RtOpenFrame(to = r.node, actor = r.actor, bits = r.bits, from = here)), salvo.Union5Codec(__Codec_RtMsgFrame, __Codec_RtAnswerFrame, __Codec_RtGrantFrame, __Codec_RtOpenFrame, __Codec_RtControlFrame))
        stageIn(routes, outbound, outbox, parked, here, r.node, frame)
        return idx
    }

    override fun registerDead(idx: Int): Int {
        if (deadEntry >= 0) {
            return deadEntry
        }
        deadEntry = idx
        return deadEntry
    }

    override fun isProxy(addr: Int): Boolean {
        return remote.containsKey(addr)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun proxyRef(addr: Int): RtRemoteRef? {
        val r = remote[addr]
        if (r != null) {
            val found = r as RtRemoteRef
            return found
        }
        return null
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun takeCredit(addr: Int, me: salvo.platform.runtime.Parker): Int {
        val c = credits[addr]
        if (c != null) {
            val n = c as Int
            if (n > 0) {
                credits.put(addr, n - 1)
                heldN.put(addr, heldIn(heldN, addr) + 1)
                return 1
            }
            creditWaiters.add(me)
            return 0
        }
        return -1
    }

    override fun stage(from: Long, to: Long, frame: salvo.SalvoBytes) {
        stageIn(routes, outbound, outbox, parked, from, to, frame)
    }

    override fun grant(addr: Int, pool: Int, from: Long, n: Int) {
        heldN.put(addr, heldIn(heldN, addr) + n)
        val me = identityIn(remote, bits, poolNode, nodeId, addr, pool)
        val frame = salvo.salvoEncode(Union5.U3<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>(RtGrantFrame(to = from, host = me.node, actor = me.actor, bits = me.bits, n = n)), salvo.Union5Codec(__Codec_RtMsgFrame, __Codec_RtAnswerFrame, __Codec_RtGrantFrame, __Codec_RtOpenFrame, __Codec_RtControlFrame))
        stageIn(routes, outbound, outbox, parked, me.node, from, frame)
    }

    override fun takeOutbox(): MutableList<RtStaged> {
        val out: MutableList<RtStaged> = mutableListOf<RtStaged>()
        while (outbox.size > 0) {
            out.add(((outbox).let { __l -> (0).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } } ?: throw AssertionError("salvo: value is absent at runtime.routing:281:22")))
        }
        return out
    }

    override fun addRoute(node: Long, at: salvo.SalvoBytes) {
        routes.put(node, at)
        restage(routes, outbound, outbox, parked)
    }

    override fun setOutbound(node: Long) {
        outbound.add(node)
        restage(routes, outbound, outbox, parked)
    }

    override fun hasOutbound(node: Long): Boolean {
        return outbound.contains(node)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun accepts(to: Long, actor: Long, claimed: Long): Boolean {
        if (!hosted.contains(to)) {
            return false
        }
        val idx = (actor).toInt()
        if (remote.containsKey(idx)) {
            return false
        }
        val b = bits[idx]
        if (b != null) {
            val known = b as Long
            return known == claimed
        }
        return false
    }

    override fun received(idx: Int) {
        val h = heldIn(heldN, idx)
        if (h > 0) {
            heldN.put(idx, h - 1)
        }
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun credited(r: RtRemoteRef, to: Long, n: Int): Boolean {
        if (!hosted.contains(to)) {
            return false
        }
        val p = proxies[r]
        if (p != null) {
            val idx = p as Int
            val i = idx
            val c = credits[i]
            var now = 0
            if (c != null) {
                val have = c as Int
                now = have
            }
            credits.put(i, now + n)
            var h = heldIn(heldN, i) - n
            if (h < 0) {
                h = 0
            }
            heldN.put(i, h)
            wakeSenders(creditWaiters)
            return true
        }
        return false
    }

    override fun held(idx: Int): Int {
        return heldIn(heldN, idx)
    }

    override fun putTask(key: Long, t: RtExportedTask) {
        taskKeys.add(key)
        tasks.add(t)
    }

    override fun takeTask(key: Long): RtExportedTask? {
        var i = 0
        while (i < taskKeys.size) {
            if ((taskKeys.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime.routing:358:16")) == key) {
                val _k = (taskKeys).let { __l -> (i).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
                return (tasks).let { __l -> (i).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
            }
            i = i + 1
        }
        return null
    }

    override fun watchChannel(node: Long, channel: String, sink: Int) {
        controls.put(RtControlKey(node = node, channel = channel), sink)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun channelSink(node: Long, channel: String): Int {
        val s = controls[RtControlKey(node = node, channel = channel)]
        if (s != null) {
            val sink = s as Int
            return sink
        }
        return -1
    }

    override fun setProtocols(table: List<Pair<String, String>>) {
        local = table
    }

    override fun protocols(): List<Pair<String, String>> {
        return local
    }

    override fun setPeer(node: Long, table: List<Pair<String, String>>) {
        peers.put(node, table)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun peerHash(node: Long, protocol: String): String? {
        val t = peers[node]
        if (t != null) {
            val table = t as List<Pair<String, String>>
            for (entry in table) {
                val (name, hash) = entry
                if (name == protocol) {
                    return hash
                }
            }
        }
        return null
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun forgetNode(node: Long): List<Int> {
        val _route = routes.remove(node)
        val _peer = peers.remove(node)
        val gone: MutableList<Int> = mutableListOf<Int>()
        for (idx in remote.keys.toMutableList()) {
            val r = remote[idx]
            if (r != null) {
                val found = r as RtRemoteRef
                if (found.node == node) {
                    gone.add(idx)
                }
            }
        }
        wakeSenders(creditWaiters)
        return gone
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun creditsOf(addr: Int): Int? {
        val c = credits[addr]
        if (c != null) {
            val n = c as Int
            return n
        }
        return null
    }

    fun init() {
        nodeId = freshNode()
        hosted.add(nodeId)
        poolNode.put(0, nodeId)
    }
}

sealed class __Priv_RtRoutes {
    object Init : __Priv_RtRoutes()
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun nodeIn(poolNode: Map<Int, Long>, nodeId: Long, pool: Int): Long {
    val n = poolNode[pool]
    if (n != null) {
        val v = n as Long
        return v
    }
    return nodeId
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun heldIn(heldN: Map<Int, Int>, idx: Int): Int {
    val h = heldN[idx]
    if (h != null) {
        val n = h as Int
        return n
    }
    return 0
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun identityIn(remote: Map<Int, RtRemoteRef>, bits: MutableMap<Int, Long>, poolNode: Map<Int, Long>, nodeId: Long, addr: Int, pool: Int): RtRemoteRef {
    val r = remote[addr]
    if (r != null) {
        val found = r as RtRemoteRef
        return found
    }
    val n = nodeIn(poolNode, nodeId, pool)
    val b = bits[addr]
    if (b != null) {
        val known = b as Long
        return RtRemoteRef(node = n, actor = (addr).toLong(), bits = known)
    }
    val minted = identityBits()
    bits.put(addr, minted)
    return RtRemoteRef(node = n, actor = (addr).toLong(), bits = minted)
}

fun wakeSenders(waiters: MutableList<salvo.platform.runtime.Parker>) {
    while (waiters.size > 0) {
        unparkPlatform(((waiters).let { __l -> (0).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } } ?: throw AssertionError("salvo: value is absent at runtime.routing:470:16")))
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun stageIn(routes: Map<Long, salvo.SalvoBytes>, outbound: Set<Long>, outbox: MutableList<RtStaged>, parked: MutableList<RtParked>, from: Long, to: Long, frame: salvo.SalvoBytes) {
    val ep = routes[to]
    if (ep != null) {
        val at = ep as salvo.SalvoBytes
        if (outbound.contains(from)) {
            outbox.add(RtStaged(from = from, to = salvo.SalvoBytes(at), frame = frame))
            return
        }
    }
    parked.add(RtParked(from = from, to = to, frame = frame))
}

fun restage(routes: Map<Long, salvo.SalvoBytes>, outbound: Set<Long>, outbox: MutableList<RtStaged>, parked: MutableList<RtParked>) {
    val waiting: MutableList<RtParked> = mutableListOf<RtParked>()
    while (parked.size > 0) {
        waiting.add(((parked).let { __l -> (0).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } } ?: throw AssertionError("salvo: value is absent at runtime.routing:493:22")))
    }
    for (p in waiting) {
        stageIn(routes, outbound, outbox, parked, p.from, p.to, salvo.SalvoBytes(p.frame))
    }
}

fun freshNode(): Long {
    val b = identityBits()
    if (b < 0) {
        return -(b + 1)
    }
    return b
}

fun hereNode(): Long {
    return __moduleUse0.nodeOfPool(currentPool())
}

fun adopt(pool: Int) {
    __moduleUse0.adoptPool(pool, hereNode())
}

fun newNode(): Long {
    return __moduleUse0.addNode()
}

fun poolAt(node: Long, n: Int): Int {
    val p = newPoolOf(n, -1)
    __moduleUse0.adoptPool(p, node)
    return p
}

fun identity(addr: Int): RtRemoteRef {
    return __moduleUse0.identityOf(addr, actorPool(addr))
}

fun sameActor(a: Int, b: Int): Boolean {
    if (a == b) {
        return true
    }
    if (!__moduleUse0.isProxy(a) && !__moduleUse0.isProxy(b)) {
        return false
    }
    return eq__4(identity(a), identity(b))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun importAddr(node: Long, actor: Long, bits: Long): Int {
    val r = RtRemoteRef(node = node, actor = actor, bits = bits)
    val here = hereNode()
    val found = __moduleUse0.findImport(r, here)
    if (found is Union3.U1<*, *, *>) {
        val f = found.value as RtFound
        return f.idx
    }
    val idx = spawnInert()
    if (found is Union3.U3<*, *, *>) {
        killActor(idx, "unknown identity")
        return __moduleUse0.registerDead(idx)
    }
    val got = __moduleUse0.registerProxy(r, idx, here)
    if (got == idx) {
        markProxy(idx)
    }
    flush()
    return got
}

fun remote(addr: Int): Boolean {
    return __moduleUse0.isProxy(addr)
}

fun sendRemote(addr: Int, proto: String, payload: salvo.SalvoBytes) {
    val r = __moduleUse0.proxyRef(addr)
    if (r == null) {
        return
    }
    while (true) {
        val got = __moduleUse0.takeCredit(addr, thisParkerPlatform())
        if (got == 1) {
            val from = hereNode()
            val frame = salvo.salvoEncode(Union5.U1<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>(RtMsgFrame(to = r.node, actor = r.actor, bits = r.bits, from = from, proto = proto, payload = payload)), salvo.Union5Codec(__Codec_RtMsgFrame, __Codec_RtAnswerFrame, __Codec_RtGrantFrame, __Codec_RtOpenFrame, __Codec_RtControlFrame))
            __moduleUse0.stage(from, r.node, frame)
            flush()
            return
        }
        if (got < 0 || mailboxDead(addr)) {
            return
        }
        parkPlatform(thisParkerPlatform())
        if (mailboxDead(addr)) {
            return
        }
    }
}

fun answerRemote(t: RtReplyParts, payload: salvo.SalvoBytes) {
    val from = hereNode()
    val frame = salvo.salvoEncode(Union5.U2<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>(RtAnswerFrame(to = t.node, kind = t.kind, id = t.id, slot = t.slot, bits = t.bits, payload = payload)), salvo.Union5Codec(__Codec_RtMsgFrame, __Codec_RtAnswerFrame, __Codec_RtGrantFrame, __Codec_RtOpenFrame, __Codec_RtControlFrame))
    __moduleUse0.stage(from, t.node, frame)
    flush()
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun exportReply(e: RtExported): RtReplyParts {
    val __destructured2 = e
    val kind = __destructured2.kind
    val id = __destructured2.id
    val slot = __destructured2.slot
    val body = __destructured2.body
    if (kind == 2) {
        if (body != null) {
            val b = body as salvo.platform.runtime.RtBody
            __moduleUse0.putTask(slot, RtExportedTask(pool = id, body = b))
        }
        return RtReplyParts(node = __moduleUse0.nodeOfPool(id), kind = 2, id = slot, slot = slot, bits = 0L)
    }
    if (body != null) {
        val b = body as salvo.platform.runtime.RtBody
        dropBodyPlatform(b)
    }
    if (kind == 1) {
        return RtReplyParts(node = __moduleUse0.nodeOfPool(waiterPool(id)), kind = 1, id = (id).toLong(), slot = slot, bits = 0L)
    }
    val me = identity(id)
    return RtReplyParts(node = me.node, kind = 0, id = me.actor, slot = slot, bits = me.bits)
}

fun creditBack(addr: Int, pool: Int, from: Long) {
    __moduleUse0.grant(addr, pool, from, 1)
}

fun flush() {
    val out = __moduleUse0.takeOutbox()
    for (s in out) {
        wireOutPlatform(s.from, salvo.SalvoBytes(s.to), salvo.SalvoBytes(s.frame))
    }
}

fun route(node: Long, at: salvo.SalvoBytes) {
    __moduleUse0.addRoute(node, at)
    flush()
}

fun outboundBound() {
    __moduleUse0.setOutbound(hereNode())
    flush()
}

fun connected(): Boolean {
    return __moduleUse0.hasOutbound(hereNode())
}

fun credits(addr: Int): Int? {
    return __moduleUse0.creditsOf(addr)
}

fun pending(addr: Int): Int {
    if (__moduleUse0.isProxy(addr)) {
        return __moduleUse0.held(addr)
    }
    return mailboxQueued(addr)
}

fun watchControl(channel: String, sink: Int) {
    __moduleUse0.watchChannel(hereNode(), channel, sink)
}

fun sendControl(to: Long, channel: String, payload: salvo.SalvoBytes) {
    val from = hereNode()
    val frame = salvo.salvoEncode(Union5.U5<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>(RtControlFrame(to = to, from = from, channel = channel, payload = payload)), salvo.Union5Codec(__Codec_RtMsgFrame, __Codec_RtAnswerFrame, __Codec_RtGrantFrame, __Codec_RtOpenFrame, __Codec_RtControlFrame))
    __moduleUse0.stage(from, to, frame)
    flush()
}

fun controlFrame(channel: String, payload: salvo.SalvoBytes): salvo.SalvoBytes {
    return salvo.salvoEncode(Union5.U5<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>(RtControlFrame(to = 0L, from = hereNode(), channel = channel, payload = payload)), salvo.Union5Codec(__Codec_RtMsgFrame, __Codec_RtAnswerFrame, __Codec_RtGrantFrame, __Codec_RtOpenFrame, __Codec_RtControlFrame))
}

fun nodeLeft(node: Long) {
    for (idx in __moduleUse0.forgetNode(node)) {
        killActor(idx, "node left")
    }
}

fun registerProtocols(table: List<Pair<String, String>>) {
    __moduleUse0.setProtocols(table)
}

fun localProtocols(): List<Pair<String, String>> {
    return __moduleUse0.protocols()
}

fun setPeerProtocols(node: Long, table: List<Pair<String, String>>) {
    __moduleUse0.setPeer(node, table)
}

fun peerProtocol(node: Long, protocol: String): String? {
    return __moduleUse0.peerHash(node, protocol)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun deliver(data: salvo.SalvoBytes): Boolean {
    val f = salvo.salvoDecode(data, salvo.Union5Codec(__Codec_RtMsgFrame, __Codec_RtAnswerFrame, __Codec_RtGrantFrame, __Codec_RtOpenFrame, __Codec_RtControlFrame))
    if (f is Union5.U1<*, *, *, *, *>) {
        val m = f?.value as RtMsgFrame
        if (!__moduleUse0.accepts(m.to, m.actor, m.bits)) {
            return false
        }
        val idx = (m.actor).toInt()
        val msg = decodeMessagePlatform(idx, m.proto, salvo.SalvoBytes(m.payload))
        if (msg != null) {
            val v = msg as salvo.platform.runtime.Dyn
            __moduleUse0.received(idx)
            val _queued = deliverRemote(idx, v, m.from)
            return true
        }
        return false
    }
    if (f is Union5.U2<*, *, *, *, *>) {
        val a = f?.value as RtAnswerFrame
        return deliverAnswer(a)
    }
    if (f is Union5.U3<*, *, *, *, *>) {
        val g = f?.value as RtGrantFrame
        return __moduleUse0.credited(RtRemoteRef(node = g.host, actor = g.actor, bits = g.bits), g.to, g.n)
    }
    if (f is Union5.U4<*, *, *, *, *>) {
        val o = f?.value as RtOpenFrame
        if (!__moduleUse0.accepts(o.to, o.actor, o.bits)) {
            return false
        }
        val idx = (o.actor).toInt()
        var room = mailboxRoom(idx) - __moduleUse0.held(idx)
        if (room < 1) {
            room = 1
        }
        __moduleUse0.grant(idx, actorPool(idx), o.from, room)
        flush()
        return true
    }
    if (f is Union5.U5<*, *, *, *, *>) {
        val c = f?.value as RtControlFrame
        var node = c.to
        if (node == (0).toLong()) {
            node = hereNode()
        }
        if (!__moduleUse0.hosts(node)) {
            return false
        }
        val sink = __moduleUse0.channelSink(node, c.channel)
        if (sink < 0) {
            return false
        }
        val msg = controlMessagePlatform(sink, c.from, salvo.SalvoBytes(c.payload))
        if (msg != null) {
            val v = msg as salvo.platform.runtime.Dyn
            val queued = deliverRemote(sink, v, (-1).toLong())
            return queued
        }
        return false
    }
    return false
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun deliverAnswer(a: RtAnswerFrame): Boolean {
    if (!__moduleUse0.hosts(a.to)) {
        return false
    }
    if (a.kind == 0) {
        if (!__moduleUse0.accepts(a.to, a.id, a.bits)) {
            return false
        }
        answer(tokenToActor((a.id).toInt(), a.slot), rawAnswerPlatform(salvo.SalvoBytes(a.payload)))
        return true
    }
    if (a.kind == 1) {
        val wid = (a.id).toInt()
        val v = decodeWaiterAnswerPlatform(wid, salvo.SalvoBytes(a.payload))
        if (v != null) {
            val value = v as salvo.platform.runtime.Dyn
            answer(tokenToWaiter(wid, a.slot), value)
            return true
        }
        return false
    }
    val t = __moduleUse0.takeTask(a.id)
    if (t != null) {
        val task = t as RtExportedTask
        val v = decodeTaskAnswerPlatform(a.id, salvo.SalvoBytes(a.payload))
        val __destructured3 = task
        val pool = __destructured3.pool
        val body = __destructured3.body
        if (v != null) {
            val value = v as salvo.platform.runtime.Dyn
            answer(mintTaskOn(pool, body), value)
            return true
        }
        dropBodyPlatform(body)
    }
    return false
}

fun hash__4(value: RtRemoteRef): Long {
    var h = 17L
    h = ((h) * 31L + ((value.node).hashCode().toLong()))
    h = ((h) * 31L + ((value.actor).hashCode().toLong()))
    h = ((h) * 31L + ((value.bits).hashCode().toLong()))
    return h
}

fun eq__4(a: RtRemoteRef, b: RtRemoteRef): Boolean {
    if (!((a.node) == (b.node))) {
        return false
    }
    if (!((a.actor) == (b.actor))) {
        return false
    }
    if (!((a.bits) == (b.bits))) {
        return false
    }
    return true
}

fun hash__5(value: RtControlKey): Long {
    var h = 17L
    h = ((h) * 31L + ((value.node).hashCode().toLong()))
    h = ((h) * 31L + ((value.channel).hashCode().toLong()))
    return h
}

fun eq__5(a: RtControlKey, b: RtControlKey): Boolean {
    if (!((a.node) == (b.node))) {
        return false
    }
    if (!((a.channel) == (b.channel))) {
        return false
    }
    return true
}
