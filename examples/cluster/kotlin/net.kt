package salvo.net

import salvo.*

data class NodeEndpoint(
    val host: String,
    val port: Int,
)

object __Codec_NodeEndpoint : salvo.WireCodec<NodeEndpoint> {
    override fun enc(v: NodeEndpoint, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.host, out)
        salvo.IntCodec.enc(v.port, out)
    }
    override fun dec(inp: salvo.WireIn): NodeEndpoint = NodeEndpoint(salvo.StrCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun toStr__NodeEndpoint(e: NodeEndpoint): String {
    return "${e.host}:${e.port}"
}

data class Unreachable(
    val to: NodeEndpoint,
)

object __Codec_Unreachable : salvo.WireCodec<Unreachable> {
    override fun enc(v: Unreachable, out: salvo.WireOut) {
        __Codec_NodeEndpoint.enc(v.to, out)
    }
    override fun dec(inp: salvo.WireIn): Unreachable = Unreachable(__Codec_NodeEndpoint.dec(inp))
}

data class WireFailed(
    val to: NodeEndpoint,
    val reason: String,
)

object __Codec_WireFailed : salvo.WireCodec<WireFailed> {
    override fun enc(v: WireFailed, out: salvo.WireOut) {
        __Codec_NodeEndpoint.enc(v.to, out)
        salvo.StrCodec.enc(v.reason, out)
    }
    override fun dec(inp: salvo.WireIn): WireFailed = WireFailed(__Codec_NodeEndpoint.dec(inp), salvo.StrCodec.dec(inp))
}


// Factories for the host: one per arm of the union [platform-factory].
object NetErrors {
    fun unreachable(value: Unreachable): Union2<Unreachable, WireFailed> = salvo.Union2.U1(value)
    fun wireFailed(value: WireFailed): Union2<Unreachable, WireFailed> = salvo.Union2.U2(value)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun toStr__NetError(e: Union2<Unreachable, WireFailed>): String {
    return when {
        (e is Union2.U1<*, *>) -> {
            val e_1: Unreachable = ((e as Union2.U1<*, *>).value as Unreachable)
            return "unreachable: ${toStr__NodeEndpoint(e_1.to)}"
        }
        (e is Union2.U2<*, *>) -> {
            val e_2: WireFailed = ((e as Union2.U2<*, *>).value as WireFailed)
            return "wire failed to ${toStr__NodeEndpoint(e_2.to)}: ${e_2.reason}"
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

interface Inbound {
    fun receiveFrame(from: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes)
}

class __Mon_Inbound(
    private val inner: Inbound,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Inbound {
    override fun receiveFrame(from: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.receiveFrame(from, frame) } finally { lock.unlock() }
    }
}

interface Transport {
    fun listen(at: NodeEndpoint, sink: Int): Union2<Unit, Union2<Unreachable, WireFailed>>
    fun unlisten(at: NodeEndpoint)
    fun deliver(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes): Union2<Unit, Union2<Unreachable, WireFailed>>
    fun localEndpoint(): NodeEndpoint
}

class __Mon_Transport(
    private val inner: Transport,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Transport {
    override fun listen(at: NodeEndpoint, sink: Int): Union2<Unit, Union2<Unreachable, WireFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.listen(at, sink) } finally { lock.unlock() }
    }
    override fun unlisten(at: NodeEndpoint) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.unlisten(at) } finally { lock.unlock() }
    }
    override fun deliver(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes): Union2<Unit, Union2<Unreachable, WireFailed>> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.deliver(to, frame) } finally { lock.unlock() }
    }
    override fun localEndpoint(): NodeEndpoint {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.localEndpoint() } finally { lock.unlock() }
    }
}

interface TransportPlatform {
    fun listen(at: NodeEndpoint, sink: Int): Union2<Unit, Union2<Unreachable, WireFailed>>
    fun unlisten(at: NodeEndpoint)
    fun deliver(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes): Union2<Unit, Union2<Unreachable, WireFailed>>
    fun localEndpoint(): NodeEndpoint
}

open class __Platform_Transport(private val impl: TransportPlatform) : Transport {
    override fun listen(at: NodeEndpoint, sink: Int): Union2<Unit, Union2<Unreachable, WireFailed>> = impl.listen(at, sink)
    override fun unlisten(at: NodeEndpoint) = impl.unlisten(at)
    override fun deliver(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes): Union2<Unit, Union2<Unreachable, WireFailed>> = impl.deliver(to, frame)
    override fun localEndpoint(): NodeEndpoint = impl.localEndpoint()
}

// Factories for the host: one per arm of the union [platform-factory].
object Listen {
    fun ok(value: Unit): Union2<Unit, Union2<Unreachable, WireFailed>> = salvo.Union2.U1(value)
    fun err(value: Union2<Unreachable, WireFailed>): Union2<Unit, Union2<Unreachable, WireFailed>> = salvo.Union2.U2(value)
}

// Factories for the host: one per arm of the union [platform-factory].
object Deliver {
    fun ok(value: Unit): Union2<Unit, Union2<Unreachable, WireFailed>> = salvo.Union2.U1(value)
    fun err(value: Union2<Unreachable, WireFailed>): Union2<Unit, Union2<Unreachable, WireFailed>> = salvo.Union2.U2(value)
}

data class NodeId(
    val id: Long,
)

object __Codec_NodeId : salvo.WireCodec<NodeId> {
    override fun enc(v: NodeId, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.id, out)
    }
    override fun dec(inp: salvo.WireIn): NodeId = NodeId(salvo.LongCodec.dec(inp))
}

fun thisNode(): NodeId {
    return NodeId(id = salvo.runtime.routing.hereNode())
}

fun newNode(): NodeId {
    return NodeId(id = salvo.runtime.routing.newNode())
}

fun poolAt(node: NodeId, size: Int): Int {
    return (salvo.runtime.routing.poolAt(node.id, size))
}

fun addRoute(node: NodeId, at: NodeEndpoint) {
    salvo.runtime.routing.route(node.id, salvo.salvoEncode(at, __Codec_NodeEndpoint))
}

fun routeFrames(out: Int) {
    bindOutboundPlatform(salvo.runtime.routing.hereNode(), (out), ::forwardFrame)
    salvo.runtime.routing.outboundBound()
}

fun forwardFrame(out: Int, to: salvo.platform.core.bytes.Bytes, frame: salvo.platform.core.bytes.Bytes) {
    val __subject_1: NodeEndpoint? = salvo.salvoDecode(to, __Codec_NodeEndpoint)
    if ((__subject_1 != null)) {
        val ep: NodeEndpoint = __subject_1!!
        val sending: Int = (out)
        salvo.SalvoSched.sendWire(sending, __Msg_Outbound.SendFrame(ep, frame), __PROTO_Outbound, __Codec___Msg_Outbound)
    }
}

fun bindOutboundPlatform(node: Long, out: Int, hook: (Int, salvo.platform.core.bytes.Bytes, salvo.platform.core.bytes.Bytes) -> Unit) = salvo.platform.net.bindOutbound(node, out, hook)

fun deliverFrame(data: salvo.platform.core.bytes.Bytes): Boolean {
    return salvo.runtime.routing.deliver(data)
}

fun credits(a: Int): Int? {
    val c: Int? = salvo.runtime.routing.credits((a))
    if ((c == null)) {
        return null
    }
    val c_1: Int = c!!
    if ((c_1 < 0)) {
        return 0
    }
    val c_2: Int = c!!
    return c_2
}

interface Outbound {
    fun sendFrame(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes)
}

class __Mon_Outbound(
    private val inner: Outbound,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Outbound {
    override fun sendFrame(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.sendFrame(to, frame) } finally { lock.unlock() }
    }
}

class Sending(private val __dep0: Transport) : Outbound {
    val __mailboxCapacity: Int = 256
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Sending> = mutableMapOf()
    override fun sendFrame(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes) {
        val _sent: Union2<Unit, Union2<Unreachable, WireFailed>> = __dep0.deliver(to, frame)
    }
}

class __Actor_Sending(private val handler: Sending) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_Sending_Outbound(handler, msg as __Msg_Outbound)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Sending.SendFrame -> handler.sendFrame(c.to, value as salvo.platform.core.bytes.Bytes)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Sending.SendFrame -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.BytesCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Outbound -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Outbound)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class Receiving : Inbound {
    val __mailboxCapacity: Int = 256
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Receiving> = mutableMapOf()
    override fun receiveFrame(from: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes) {
        val _delivered: Boolean = deliverFrame(frame)
    }
}

class __Actor_Receiving(private val handler: Receiving) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_Receiving_Inbound(handler, msg as __Msg_Inbound)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_Receiving.ReceiveFrame -> handler.receiveFrame(c.from, value as salvo.platform.core.bytes.Bytes)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_Receiving.ReceiveFrame -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.BytesCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_Inbound -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_Inbound)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

fun connected(): Boolean {
    return salvo.runtime.routing.connected()
}

fun connect__NodeEndpoint(transport: Transport, me: NodeEndpoint): Boolean {
    return connect__NodeEndpoint_Pool(transport, me, salvo.core.actor.pool(1))
}

fun connect__NodeEndpoint_Pool(transport: Transport, me: NodeEndpoint, on: Int): Boolean {
    if (connected()) {
        return false
    }
    val sending: Int = run { val __h = Sending(transport); val __a = salvo.SalvoSched.spawn(on, __h.__mailboxCapacity, __Actor_Sending(__h), __Actor_Sending.__DECODE); __a }
    val receiving: Int = run { val __h = Receiving(); val __a = salvo.SalvoSched.spawn(on, __h.__mailboxCapacity, __Actor_Receiving(__h), __Actor_Receiving.__DECODE); __a }
    routeFrames(sending)
    val _listening: Union2<Unit, Union2<Unreachable, WireFailed>> = transport.listen(me, receiving)
    addRoute(thisNode(), me)
    return true
}

data class Node(
    val id: NodeId,
    val at: NodeEndpoint,
)

object __Codec_Node : salvo.WireCodec<Node> {
    override fun enc(v: Node, out: salvo.WireOut) {
        __Codec_NodeId.enc(v.id, out)
        __Codec_NodeEndpoint.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): Node = Node(__Codec_NodeId.dec(inp), __Codec_NodeEndpoint.dec(inp))
}

interface NodeGroup {
    fun members(out: salvo.SalvoReply)
    fun subscribe(w: Int)
    fun leave()
}

class __Mon_NodeGroup(
    private val inner: NodeGroup,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : NodeGroup {
    override fun members(out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.members(out) } finally { lock.unlock() }
    }
    override fun subscribe(w: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.subscribe(w) } finally { lock.unlock() }
    }
    override fun leave() {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.leave() } finally { lock.unlock() }
    }
}

interface NodeGroupWatcher {
    fun joined(n: Node)
    fun left(n: Node, why: String)
}

class __Mon_NodeGroupWatcher(
    private val inner: NodeGroupWatcher,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : NodeGroupWatcher {
    override fun joined(n: Node) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.joined(n) } finally { lock.unlock() }
    }
    override fun left(n: Node, why: String) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.left(n, why) } finally { lock.unlock() }
    }
}

data class Hello(
    val group: String,
    val at: NodeEndpoint,
    val protocols: List<Pair<String, String>>,
)

object __Codec_Hello : salvo.WireCodec<Hello> {
    override fun enc(v: Hello, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.group, out)
        __Codec_NodeEndpoint.enc(v.at, out)
        salvo.ListCodec(salvo.PairCodec(salvo.StrCodec, salvo.StrCodec)).enc(v.protocols, out)
    }
    override fun dec(inp: salvo.WireIn): Hello = Hello(salvo.StrCodec.dec(inp), __Codec_NodeEndpoint.dec(inp), salvo.ListCodec(salvo.PairCodec(salvo.StrCodec, salvo.StrCodec)).dec(inp))
}

data class Ack(
    val group: String,
    val at: NodeEndpoint,
    val protocols: List<Pair<String, String>>,
)

object __Codec_Ack : salvo.WireCodec<Ack> {
    override fun enc(v: Ack, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.group, out)
        __Codec_NodeEndpoint.enc(v.at, out)
        salvo.ListCodec(salvo.PairCodec(salvo.StrCodec, salvo.StrCodec)).enc(v.protocols, out)
    }
    override fun dec(inp: salvo.WireIn): Ack = Ack(salvo.StrCodec.dec(inp), __Codec_NodeEndpoint.dec(inp), salvo.ListCodec(salvo.PairCodec(salvo.StrCodec, salvo.StrCodec)).dec(inp))
}

class Leaving

object __Codec_Leaving : salvo.WireCodec<Leaving> {
    override fun enc(v: Leaving, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Leaving = Leaving()
}

data class Intro(
    val peers: List<NodeEndpoint>,
)

object __Codec_Intro : salvo.WireCodec<Intro> {
    override fun enc(v: Intro, out: salvo.WireOut) {
        salvo.ListCodec(__Codec_NodeEndpoint).enc(v.peers, out)
    }
    override fun dec(inp: salvo.WireIn): Intro = Intro(salvo.ListCodec(__Codec_NodeEndpoint).dec(inp))
}

fun sendControl(to: NodeId, channel: String, payload: salvo.platform.core.bytes.Bytes) {
    salvo.runtime.routing.sendControl(to.id, channel, payload)
}

fun nodeLeft(node: NodeId) {
    salvo.runtime.routing.nodeLeft(node.id)
}

fun setPeerProtocols(node: NodeId, table: List<Pair<String, String>>) {
    salvo.runtime.routing.setPeerProtocols(node.id, table)
}

fun peerProtocol(node: NodeId, protocol: String): String? {
    return salvo.runtime.routing.peerProtocol(node.id, protocol)
}

fun helloFrame(transport: Transport, group: String): salvo.platform.core.bytes.Bytes {
    val hello: Union4<Hello, Ack, Leaving, Intro> = Union4.U1<Hello, Ack, Leaving, Intro>(Hello(group = group, at = transport.localEndpoint(), protocols = salvo.runtime.routing.localProtocols()))
    return salvo.runtime.routing.controlFrame("", salvo.salvoEncode(hello, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro)))
}

data class PeerHello(
    val node: NodeId,
    val at: NodeEndpoint,
)

object __Codec_PeerHello : salvo.WireCodec<PeerHello> {
    override fun enc(v: PeerHello, out: salvo.WireOut) {
        __Codec_NodeId.enc(v.node, out)
        __Codec_NodeEndpoint.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): PeerHello = PeerHello(__Codec_NodeId.dec(inp), __Codec_NodeEndpoint.dec(inp))
}

data class PeerGone(
    val node: NodeId,
)

object __Codec_PeerGone : salvo.WireCodec<PeerGone> {
    override fun enc(v: PeerGone, out: salvo.WireOut) {
        __Codec_NodeId.enc(v.node, out)
    }
    override fun dec(inp: salvo.WireIn): PeerGone = PeerGone(__Codec_NodeId.dec(inp))
}

data class PeerIntro(
    val peers: List<NodeEndpoint>,
)

object __Codec_PeerIntro : salvo.WireCodec<PeerIntro> {
    override fun enc(v: PeerIntro, out: salvo.WireOut) {
        salvo.ListCodec(__Codec_NodeEndpoint).enc(v.peers, out)
    }
    override fun dec(inp: salvo.WireIn): PeerIntro = PeerIntro(salvo.ListCodec(__Codec_NodeEndpoint).dec(inp))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun handshake(transport: Transport, group: String, from: NodeId, data: salvo.platform.core.bytes.Bytes): Union3<PeerHello, PeerGone, PeerIntro>? {
    val msg: Union4<Hello, Ack, Leaving, Intro>? = salvo.salvoDecode(data, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro))
    return when {
        (msg is Union4.U1<*, *, *, *>) -> {
            val msg_1: Hello = ((msg as Union4.U1<*, *, *, *>).value as Hello)
            if (!(((msg_1.group) == (group)))) {
                return null
            }
            addRoute(from, msg_1.at)
            setPeerProtocols(from, msg_1.protocols)
            val ack: Union4<Hello, Ack, Leaving, Intro> = Union4.U2<Hello, Ack, Leaving, Intro>(Ack(group = group, at = transport.localEndpoint(), protocols = salvo.runtime.routing.localProtocols()))
            sendControl(from, "", salvo.salvoEncode(ack, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro)))
            return Union3.U1<PeerHello, PeerGone, PeerIntro>(PeerHello(node = from, at = msg_1.at))
        }
        (msg is Union4.U2<*, *, *, *>) -> {
            val msg_2: Ack = ((msg as Union4.U2<*, *, *, *>).value as Ack)
            if (!(((msg_2.group) == (group)))) {
                return null
            }
            addRoute(from, msg_2.at)
            setPeerProtocols(from, msg_2.protocols)
            return Union3.U1<PeerHello, PeerGone, PeerIntro>(PeerHello(node = from, at = msg_2.at))
        }
        (msg is Union4.U3<*, *, *, *>) -> {
            val msg_3: Leaving = ((msg as Union4.U3<*, *, *, *>).value as Leaving)
            nodeLeft(from)
            return Union3.U2<PeerHello, PeerGone, PeerIntro>(PeerGone(node = from))
        }
        (msg is Union4.U4<*, *, *, *>) -> {
            val msg_4: Intro = ((msg as Union4.U4<*, *, *, *>).value as Intro)
            return Union3.U3<PeerHello, PeerGone, PeerIntro>(PeerIntro(peers = msg_4.peers))
        }
        (msg == null) -> {
            return null
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

fun introduce(node: NodeId, peers: List<NodeEndpoint>) {
    val intro: Union4<Hello, Ack, Leaving, Intro> = Union4.U4<Hello, Ack, Leaving, Intro>(Intro(peers = peers))
    sendControl(node, "", salvo.salvoEncode(intro, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro)))
}

fun leaveGroup(known: salvo.platform.core.map.Map<NodeId, Node>) {
    for (id in salvo.platform.core.list.each(salvo.core.map.keysPlatform(known))) {
        val leaving: Union4<Hello, Ack, Leaving, Intro> = Union4.U3<Hello, Ack, Leaving, Intro>(Leaving())
        sendControl(id, "", salvo.salvoEncode(leaving, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro)))
    }
}

class StaticNodeGroup(private val name: String, private val all: List<NodeEndpoint>, private val __dep0: Transport) : NodeGroup {
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_StaticNodeGroup> = mutableMapOf()
    var known: salvo.platform.core.map.MutMap<NodeId, Node> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<NodeId, Node>>(), ::hash__NodeId, ::eq__NodeId_NodeId)
    var watchers: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    fun init() {
        val me: NodeEndpoint = __dep0.localEndpoint()
        val _connected: Boolean = connect__NodeEndpoint(__dep0, me)
        salvo.SalvoSched.watchControl("", __addr!!) { __n, __d -> __Priv_StaticNodeGroup.Control(NodeId(__n), salvo.SalvoBytes(__d)) }
        for (e in salvo.platform.core.list.each(all)) {
            if (!(eq__NodeEndpoint_NodeEndpoint(e, me))) {
                val _sent: Union2<Unit, Union2<Unreachable, WireFailed>> = __dep0.deliver(e, helloFrame(__dep0, name))
            }
        }
    }
    override fun members(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, knownNodes(known), salvo.ListCodec(__Codec_Node))
    }
    override fun subscribe(w: Int) {
        for (id in salvo.platform.core.list.each(salvo.core.map.keysPlatform(known))) {
            val n: Node? = salvo.core.map.getPlatform(known, id, ::hash__NodeId, ::eq__NodeId_NodeId)
            if (!((n == null))) {
                val n_1: Node = n!!
                salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n_1), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
            }
        }
        salvo.core.list.addPlatform(watchers, w)
    }
    override fun leave() {
        leaveGroup(known)
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    fun control(from: NodeId, data: salvo.platform.core.bytes.Bytes) {
        val event: Union3<PeerHello, PeerGone, PeerIntro>? = handshake(__dep0, name, from, data)
        when {
            (event is Union3.U1<*, *, *>) -> {
                val event_1: PeerHello = ((event as Union3.U1<*, *, *>).value as PeerHello)
                if (salvo.core.map.containsKeyPlatform(known, event_1.node, ::hash__NodeId, ::eq__NodeId_NodeId)) {
                    return
                }
                val n: Node = Node(id = event_1.node, at = event_1.at)
                salvo.core.map.putPlatform(known, event_1.node, n, ::hash__NodeId, ::eq__NodeId_NodeId)
                for (w in salvo.platform.core.list.each(watchers)) {
                    salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
                }
            }
            (event is Union3.U2<*, *, *>) -> {
                val event_2: PeerGone = ((event as Union3.U2<*, *, *>).value as PeerGone)
                val n: Node? = salvo.core.map.removePlatform(known, event_2.node, ::hash__NodeId, ::eq__NodeId_NodeId)
                if ((n == null)) {
                    return
                }
                for (w in salvo.platform.core.list.each(watchers)) {
                    val n_3: Node = n!!
                    salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Left(n_3, "left"), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
                }
            }
            (event is Union3.U3<*, *, *>) -> {
                val event_4: PeerIntro = ((event as Union3.U3<*, *, *>).value as PeerIntro)
            }
            (event == null) -> {
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
}

class __Actor_StaticNodeGroup(private val handler: StaticNodeGroup) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_NodeGroup -> __dispatch_StaticNodeGroup_NodeGroup(handler, msg)
            is __Priv_StaticNodeGroup -> __dispatch_priv_StaticNodeGroup(handler, msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_StaticNodeGroup.Members -> handler.members(value as salvo.SalvoReply)
            is __Cont_StaticNodeGroup.Subscribe -> handler.subscribe(value as Int)
            is __Cont_StaticNodeGroup.Control -> handler.control(c.from, value as salvo.platform.core.bytes.Bytes)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_StaticNodeGroup.Members -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_StaticNodeGroup.Subscribe -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_StaticNodeGroup.Control -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.BytesCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_NodeGroup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_NodeGroup)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

fun knownNodes(known: salvo.platform.core.map.Map<NodeId, Node>): List<Node> {
    val allKnown: salvo.platform.core.list.MutList<Node> = mutableListOf<Node>()
    for (id in salvo.platform.core.list.each(salvo.core.map.keysPlatform(known))) {
        val n: Node? = salvo.core.map.getPlatform(known, id, ::hash__NodeId, ::eq__NodeId_NodeId)
        if (!((n == null))) {
            val n_1: Node = n!!
            salvo.core.list.addPlatform(allKnown, n_1)
        }
    }
    return allKnown
}

class GossipNodeGroup(private val name: String, private val seeds: List<NodeEndpoint>, private val __dep0: Transport) : NodeGroup {
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_GossipNodeGroup> = mutableMapOf()
    var known: salvo.platform.core.map.MutMap<NodeId, Node> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<NodeId, Node>>(), ::hash__NodeId, ::eq__NodeId_NodeId)
    var dialed: salvo.platform.core.set.MutSet<String> = salvo.core.set.mutSetOfPlatform(arrayOf<String>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var watchers: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    fun init() {
        val me: NodeEndpoint = __dep0.localEndpoint()
        val _connected: Boolean = connect__NodeEndpoint(__dep0, me)
        salvo.SalvoSched.watchControl("", __addr!!) { __n, __d -> __Priv_GossipNodeGroup.Control(NodeId(__n), salvo.SalvoBytes(__d)) }
        for (e in salvo.platform.core.list.each(seeds)) {
            dial(__dep0, dialed, name, e)
        }
    }
    override fun members(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, knownNodes(known), salvo.ListCodec(__Codec_Node))
    }
    override fun subscribe(w: Int) {
        for (id in salvo.platform.core.list.each(salvo.core.map.keysPlatform(known))) {
            val n: Node? = salvo.core.map.getPlatform(known, id, ::hash__NodeId, ::eq__NodeId_NodeId)
            if (!((n == null))) {
                val n_1: Node = n!!
                salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n_1), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
            }
        }
        salvo.core.list.addPlatform(watchers, w)
    }
    override fun leave() {
        leaveGroup(known)
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    fun control(from: NodeId, data: salvo.platform.core.bytes.Bytes) {
        val event: Union3<PeerHello, PeerGone, PeerIntro>? = handshake(__dep0, name, from, data)
        when {
            (event is Union3.U1<*, *, *>) -> {
                val event_1: PeerHello = ((event as Union3.U1<*, *, *>).value as PeerHello)
                if (salvo.core.map.containsKeyPlatform(known, event_1.node, ::hash__NodeId, ::eq__NodeId_NodeId)) {
                    return
                }
                val others: salvo.platform.core.list.MutList<NodeEndpoint> = mutableListOf<NodeEndpoint>()
                for (id in salvo.platform.core.list.each(salvo.core.map.keysPlatform(known))) {
                    val n: Node? = salvo.core.map.getPlatform(known, id, ::hash__NodeId, ::eq__NodeId_NodeId)
                    if (!((n == null))) {
                        val n_2: Node = n!!
                        salvo.core.list.addPlatform(others, n_2.at)
                    }
                    introduce(id, listOf<NodeEndpoint>(event_1.at))
                }
                introduce(event_1.node, others)
                salvo.core.set.addPlatform(dialed, toStr__NodeEndpoint(event_1.at), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
                val n: Node = Node(id = event_1.node, at = event_1.at)
                salvo.core.map.putPlatform(known, event_1.node, n, ::hash__NodeId, ::eq__NodeId_NodeId)
                for (w in salvo.platform.core.list.each(watchers)) {
                    salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
                }
            }
            (event is Union3.U2<*, *, *>) -> {
                val event_3: PeerGone = ((event as Union3.U2<*, *, *>).value as PeerGone)
                val n: Node? = salvo.core.map.removePlatform(known, event_3.node, ::hash__NodeId, ::eq__NodeId_NodeId)
                if ((n == null)) {
                    return
                }
                for (w in salvo.platform.core.list.each(watchers)) {
                    val n_4: Node = n!!
                    salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Left(n_4, "left"), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
                }
            }
            (event is Union3.U3<*, *, *>) -> {
                val event_5: PeerIntro = ((event as Union3.U3<*, *, *>).value as PeerIntro)
                for (e in salvo.platform.core.list.each(event_5.peers)) {
                    dial(__dep0, dialed, name, e)
                }
            }
            (event == null) -> {
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
}

class __Actor_GossipNodeGroup(private val handler: GossipNodeGroup) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_NodeGroup -> __dispatch_GossipNodeGroup_NodeGroup(handler, msg)
            is __Priv_GossipNodeGroup -> __dispatch_priv_GossipNodeGroup(handler, msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_GossipNodeGroup.Members -> handler.members(value as salvo.SalvoReply)
            is __Cont_GossipNodeGroup.Subscribe -> handler.subscribe(value as Int)
            is __Cont_GossipNodeGroup.Control -> handler.control(c.from, value as salvo.platform.core.bytes.Bytes)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_GossipNodeGroup.Members -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_GossipNodeGroup.Subscribe -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_GossipNodeGroup.Control -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.BytesCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_NodeGroup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_NodeGroup)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun dial(transport: Transport, dialed: salvo.platform.core.set.MutSet<String>, group: String, e: NodeEndpoint) {
    if ((eq__NodeEndpoint_NodeEndpoint(e, transport.localEndpoint()) || salvo.core.set.containsPlatform(dialed, toStr__NodeEndpoint(e), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
        return
    }
    salvo.core.set.addPlatform(dialed, toStr__NodeEndpoint(e), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    val sent: Union2<Unit, Union2<Unreachable, WireFailed>> = transport.deliver(e, helloFrame(transport, group))
    if ((sent is Union2.U2<*, *>)) {
        val sent_1: Union2<Unreachable, WireFailed> = ((sent as Union2.U2<*, *>).value as Union2<Unreachable, WireFailed>)
        val _forgot: Boolean = salvo.core.set.removePlatform(dialed, toStr__NodeEndpoint(e), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
}

data class Protocol(
    val name: String,
    val hash: String,
)

object __Codec_Protocol : salvo.WireCodec<Protocol> {
    override fun enc(v: Protocol, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.name, out)
        salvo.StrCodec.enc(v.hash, out)
    }
    override fun dec(inp: salvo.WireIn): Protocol = Protocol(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

interface ActorGroup {
    fun join(member: Int)
    fun leave(member: Int)
    fun members(out: salvo.SalvoReply)
    fun subscribe(w: Int)
    fun refresh()
}

class __Mon_ActorGroup(
    private val inner: ActorGroup,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : ActorGroup {
    override fun join(member: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.join(member) } finally { lock.unlock() }
    }
    override fun leave(member: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.leave(member) } finally { lock.unlock() }
    }
    override fun members(out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.members(out) } finally { lock.unlock() }
    }
    override fun subscribe(w: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.subscribe(w) } finally { lock.unlock() }
    }
    override fun refresh() {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.refresh() } finally { lock.unlock() }
    }
}

interface ActorGroupWatcher {
    fun joined(member: Int)
    fun left(member: Int)
}

class __Mon_ActorGroupWatcher(
    private val inner: ActorGroupWatcher,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : ActorGroupWatcher {
    override fun joined(member: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.joined(member) } finally { lock.unlock() }
    }
    override fun left(member: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.left(member) } finally { lock.unlock() }
    }
}

fun shareMembers(name: String, hash: String, node: NodeId, members: List<Int>) {
    sendControl(node, name, salvo.salvoEncode(Pair(hash, members), salvo.PairCodec(salvo.StrCodec, salvo.ListCodec(salvo.AddrCodec))))
}

fun pending(a: Int): Int {
    return salvo.runtime.routing.pending((a))
}

fun actorGroup__Addr(nodes: Int, protocol: () -> Protocol): Int {
    val proto: Protocol = protocol()
    val name: String = proto.name
    return actorGroup__Str_Addr(name, nodes, protocol)
}

fun actorGroup__Str_Addr(name: String, nodes: Int, protocol: () -> Protocol): Int {
    val __destructured_1: Pair<Int, Int> = run { val __h = ActorGrouping(name = name, proto = protocol()); val __a = salvo.SalvoSched.spawn(salvo.core.actor.pool(1), __h.__mailboxCapacity, __Actor_ActorGrouping(__h), __Actor_ActorGrouping.__DECODE); salvo.SalvoSched.send(__a, __Priv_ActorGrouping.Init); Pair(__a, __a) }
    val group: Int = __destructured_1.first
    val watcher: Int = __destructured_1.second
    salvo.SalvoSched.sendWire(nodes, __Msg_NodeGroup.Subscribe(watcher), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
    return group
}

fun join(group: Int, member: Int) {
    salvo.SalvoSched.sendWire(group, __Msg_ActorGroup.Join(member), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
}

class ActorGrouping(private val name: String, private val proto: Protocol) : ActorGroup, NodeGroupWatcher {
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_ActorGrouping> = mutableMapOf()
    var all: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    var peers: salvo.platform.core.list.MutList<NodeId> = mutableListOf<NodeId>()
    var watchers: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    fun init() {
        salvo.SalvoSched.watchControl(name, __addr!!) { __n, __d -> __Priv_ActorGrouping.Control(NodeId(__n), salvo.SalvoBytes(__d)) }
    }
    override fun join(member: Int) {
        if (!(admit(all, member))) {
            return
        }
        mirror(__addr!!, all)
        for (w in salvo.platform.core.list.each(watchers)) {
            salvo.SalvoSched.sendWire(w, __Msg_ActorGroupWatcher.Joined(member), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
        }
        for (p in salvo.platform.core.list.each(peers)) {
            shareMembers(name, proto.hash, p, listOf<Int>(member))
        }
    }
    override fun leave(member: Int) {
        if (!(withdraw(all, member))) {
            return
        }
        mirror(__addr!!, all)
        for (w in salvo.platform.core.list.each(watchers)) {
            salvo.SalvoSched.sendWire(w, __Msg_ActorGroupWatcher.Left(member), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
        }
    }
    override fun joined(n: Node) {
        shareMembers(name, proto.hash, n.id, all)
    }
    override fun left(n: Node, why: String) {
        val gone: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
        for (m in salvo.platform.core.list.each(all)) {
            if (eq__NodeId_NodeId(nodeOf(m), n.id)) {
                salvo.core.list.addPlatform(gone, m)
            }
        }
        for (m in salvo.platform.core.list.each(gone)) {
            if (withdraw(all, m)) {
                for (w in salvo.platform.core.list.each(watchers)) {
                    salvo.SalvoSched.sendWire(w, __Msg_ActorGroupWatcher.Left(m), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
                }
            }
        }
        if ((salvo.core.list.sizePlatform(gone) > 0)) {
            mirror(__addr!!, all)
        }
    }
    override fun members(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, all.toMutableList(), salvo.ListCodec(salvo.AddrCodec))
    }
    override fun refresh() {
        viewRefresh(__addr!!)
    }
    override fun subscribe(w: Int) {
        salvo.core.list.addPlatform(watchers, w)
    }
    fun control(from: NodeId, data: salvo.platform.core.bytes.Bytes) {
        val got: Pair<String, List<Int>>? = salvo.salvoDecode(data, salvo.PairCodec(salvo.StrCodec, salvo.ListCodec(salvo.AddrCodec)))
        if ((got == null)) {
            return
        }
        val got_1: Pair<String, List<Int>> = got!!
        if (!(((got_1.first) == (proto.hash)))) {
            return
        }
        if (!(containsNode(peers, from))) {
            salvo.core.list.addPlatform(peers, from)
            shareMembers(name, proto.hash, from, all)
        }
        var changed: Boolean = false
        val got_2: Pair<String, List<Int>> = got!!
        for (m in salvo.platform.core.list.each(got_2.second)) {
            if (admit(all, m)) {
                changed = true
                for (w in salvo.platform.core.list.each(watchers)) {
                    salvo.SalvoSched.sendWire(w, __Msg_ActorGroupWatcher.Joined(m), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
                }
            }
        }
        if (changed) {
            mirror(__addr!!, all)
        }
    }
}

class __Actor_ActorGrouping(private val handler: ActorGrouping) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_ActorGroup -> __dispatch_ActorGrouping_ActorGroup(handler, msg)
            is __Msg_NodeGroupWatcher -> __dispatch_ActorGrouping_NodeGroupWatcher(handler, msg)
            is __Priv_ActorGrouping -> __dispatch_priv_ActorGrouping(handler, msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_ActorGrouping.Join -> handler.join(value as Int)
            is __Cont_ActorGrouping.Leave -> handler.leave(value as Int)
            is __Cont_ActorGrouping.Members -> handler.members(value as salvo.SalvoReply)
            is __Cont_ActorGrouping.Subscribe -> handler.subscribe(value as Int)
            is __Cont_ActorGrouping.Joined -> handler.joined(value as Node)
            is __Cont_ActorGrouping.Left -> handler.left(c.n, value as String)
            is __Cont_ActorGrouping.Control -> handler.control(c.from, value as salvo.platform.core.bytes.Bytes)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_ActorGrouping.Join -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_ActorGrouping.Leave -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_ActorGrouping.Members -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_ActorGrouping.Subscribe -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_ActorGrouping.Joined -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Node) })(payload)
            is __Cont_ActorGrouping.Left -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })(payload)
            is __Cont_ActorGrouping.Control -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.BytesCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_ActorGroup -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_ActorGroup)?.let { Pair(true, it) } ?: Pair(false, null)
                __PROTO_NodeGroupWatcher -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_NodeGroupWatcher)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

fun mirror(group: Int, members: List<Int>) {
    viewSet(group, members)
}

fun admit(list: salvo.platform.core.list.MutList<Int>, a: Int): Boolean {
    for (x in salvo.platform.core.list.each(list)) {
        if (salvo.core.actor.eq(x, a)) {
            return false
        }
    }
    salvo.core.list.addPlatform(list, a)
    return true
}

fun containsNode(list: List<NodeId>, n: NodeId): Boolean {
    for (x in salvo.platform.core.list.each(list)) {
        if (eq__NodeId_NodeId(x, n)) {
            return true
        }
    }
    return false
}

fun withdraw(list: salvo.platform.core.list.MutList<Int>, a: Int): Boolean {
    var mutIndex: Int? = null
    var i: Int = 0
    for (x in salvo.platform.core.list.each(list)) {
        if (salvo.core.actor.eq(x, a)) {
            mutIndex = i
        }
        i = (i + 1)
    }
    if ((mutIndex == null)) {
        return false
    }
    val mutIndex_1: Int = mutIndex!!
    val _removed: Int? = salvo.core.list.removeAtPlatform(list, mutIndex_1)
    return true
}

fun nodeOf(a: Int): NodeId {
    return NodeId(id = run {
        val __proj_1: salvo.runtime.routing.RemoteRef = salvo.runtime.routing.identity((a))
        __proj_1.node
    })
}

fun viewSet(group: Int, members: List<Int>) {
    val ixs: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    for (m in salvo.platform.core.list.each(members)) {
        salvo.core.list.addPlatform(ixs, (m))
    }
    salvo.runtime.routing.viewSet((group), ixs)
}

fun viewMembers(group: Int): List<Int> {
    val out: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    for (ix in salvo.platform.core.list.each(salvo.runtime.routing.viewMembers((group)))) {
        salvo.core.list.addPlatform(out, (ix))
    }
    return out
}

fun viewVersion(group: Int): Long {
    return salvo.runtime.routing.viewVersion((group))
}

fun viewRefresh(group: Int) {
    salvo.runtime.routing.viewRefresh((group))
}

fun viewWait(group: Int, seen: Long, nanos: Long) {
    salvo.runtime.routing.viewWait((group), seen, nanos)
}

data class RouteMember(
    val addr: Int,
    val local: Boolean,
)

object __Codec_RouteMember : salvo.WireCodec<RouteMember> {
    override fun enc(v: RouteMember, out: salvo.WireOut) {
        salvo.AddrCodec.enc(v.addr, out)
        salvo.BoolCodec.enc(v.local, out)
    }
    override fun dec(inp: salvo.WireIn): RouteMember = RouteMember(salvo.AddrCodec.dec(inp), salvo.BoolCodec.dec(inp))
}

data class RouteView(
    val members: List<RouteMember>,
)

object __Codec_RouteView : salvo.WireCodec<RouteView> {
    override fun enc(v: RouteView, out: salvo.WireOut) {
        salvo.ListCodec(__Codec_RouteMember).enc(v.members, out)
    }
    override fun dec(inp: salvo.WireIn): RouteView = RouteView(salvo.ListCodec(__Codec_RouteMember).dec(inp))
}

fun defaultRouteConfig(): RouteConfig {
    return RouteConfig(firstWait = salvo.time.Duration(nanos = 1000000L), maxWait = salvo.time.Duration(nanos = 5000000000L))
}

data class RouteConfig(
    val firstWait: salvo.time.Duration = salvo.time.Duration(nanos = 1000000L),
    val maxWait: salvo.time.Duration = salvo.time.Duration(nanos = 5000000000L),
)

object __Codec_RouteConfig : salvo.WireCodec<RouteConfig> {
    override fun enc(v: RouteConfig, out: salvo.WireOut) {
        salvo.time.__Codec_Duration.enc(v.firstWait, out)
        salvo.time.__Codec_Duration.enc(v.maxWait, out)
    }
    override fun dec(inp: salvo.WireIn): RouteConfig = RouteConfig(salvo.time.__Codec_Duration.dec(inp), salvo.time.__Codec_Duration.dec(inp))
}

interface RouteSelector {
    fun changed(view: RouteView)
    fun select(view: RouteView, key: Long?): Int?
}

class __Mon_RouteSelector(
    private val inner: RouteSelector,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : RouteSelector {
    override fun changed(view: RouteView) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.changed(view) } finally { lock.unlock() }
    }
    override fun select(view: RouteView, key: Long?): Int? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.select(view, key) } finally { lock.unlock() }
    }
}

data class RoutePick(
    val to: Int,
    val version: Long,
)

object __Codec_RoutePick : salvo.WireCodec<RoutePick> {
    override fun enc(v: RoutePick, out: salvo.WireOut) {
        salvo.AddrCodec.enc(v.to, out)
        salvo.LongCodec.enc(v.version, out)
    }
    override fun dec(inp: salvo.WireIn): RoutePick = RoutePick(salvo.AddrCodec.dec(inp), salvo.LongCodec.dec(inp))
}

fun routePick__Addr_RouteConfig_Long(routeSelector: RouteSelector, group: Int, config: RouteConfig, seen: Long): RoutePick {
    return routeKeyed(routeSelector, group, config, seen, null)
}

fun routePick__Addr_RouteConfig_Long_Long(routeSelector: RouteSelector, group: Int, config: RouteConfig, seen: Long, key: Long): RoutePick {
    return routeKeyed(routeSelector, group, config, seen, key)
}

fun routeKeyed(routeSelector: RouteSelector, group: Int, config: RouteConfig, seen: Long, key: Long?): RoutePick {
    var wait: Long = config.firstWait.nanos
    val cap: Long = config.maxWait.nanos
    var last: Long = seen
    while (true) {
        if (!(true)) {
            break
        }
        val version: Long = viewVersion(group)
        val view: RouteView = routeView(group)
        if (!(((version) == (last)))) {
            routeSelector.changed(view)
            last = version
        }
        val picked: Int? = routeSelector.select(view, key)
        if (!((picked == null))) {
            val picked_1: Int = picked!!
            return RoutePick(to = picked_1, version = last)
        }
        viewWait(group, version, wait)
        wait = (wait * 2L)
        if ((wait > cap)) {
            wait = cap
        }
    }
    return routeKeyed(routeSelector, group, config, seen, key)
}

fun routeView(group: Int): RouteView {
    val members: salvo.platform.core.list.MutList<RouteMember> = mutableListOf<RouteMember>()
    for (m in salvo.platform.core.list.each(viewMembers(group))) {
        salvo.core.list.addPlatform(members, RouteMember(addr = m, local = eq__NodeId_NodeId(nodeOf(m), thisNode())))
    }
    return RouteView(members = members.toMutableList())
}

class LeastLoaded(private val preferLocal: Boolean) : RouteSelector {
    override fun changed(view: RouteView) {
    }
    override fun select(view: RouteView, key: Long?): Int? {
        var best: RouteMember? = null
        var bestPending: Int = 0
        for (a in salvo.platform.core.list.each(view.members)) {
            val load: Int = pending(a.addr)
            if ((best == null)) {
                best = a
                bestPending = load
            } else {
                val best_1: RouteMember = best!!
                val b: RouteMember = best_1
                val take: Boolean = (if (((preferLocal && a.local) && !(b.local))) {
                    true
                } else if (((preferLocal && !(a.local)) && b.local)) {
                    false
                } else {
                    (load < bestPending)
                })
                if (take) {
                    best = a
                    bestPending = load
                }
            }
        }
        val chosen: RouteMember = run {
            val __elv_2: RouteMember? = best
            when {
                (__elv_2 == null) -> {
                    run {
                        return null
                    }
                }
                else -> {
                    val __some_3: RouteMember = __elv_2!!
                    __some_3
                }
            }
        }
        return chosen.addr
    }
}

class Sharded : RouteSelector {
    override fun changed(view: RouteView) {
    }
    override fun select(view: RouteView, key: Long?): Int? {
        val n: Int = salvo.core.list.sizePlatform(view.members)
        if (((n) == (0))) {
            return null
        }
        val k: Long = run {
            val __elv_1: Long? = key
            when {
                (__elv_1 == null) -> {
                    0L
                }
                else -> {
                    val __some_2: Long = __elv_1!!
                    __some_2
                }
            }
        }
        val magnitude: Long = (if ((k < 0L)) {
            (0L - k)
        } else {
            k
        })
        val slot: Int = ((magnitude % (n).toLong())).toInt()
        val picked: RouteMember = run {
            val __elv_3: RouteMember? = salvo.core.list.getPlatform(view.members, slot)
            when {
                (__elv_3 == null) -> {
                    run {
                        return null
                    }
                }
                else -> {
                    val __some_4: RouteMember = __elv_3!!
                    __some_4
                }
            }
        }
        return picked.addr
    }
}

interface Leader {
    fun leader(): NodeId?
}

class __Mon_Leader(
    private val inner: Leader,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Leader {
    override fun leader(): NodeId? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.leader() } finally { lock.unlock() }
    }
}

class StaticLeader(private val node: NodeId) : Leader {
    override fun leader(): NodeId? {
        return node
    }
}

class Elected(private val __dep0: Leader) : RouteSelector {
    var chosen: Int? = null
    override fun changed(view: RouteView) {
        chosen = null
        val l: NodeId = run {
            val __elv_1: NodeId? = __dep0.leader()
            when {
                (__elv_1 == null) -> {
                    run {
                        return
                    }
                }
                else -> {
                    val __some_2: NodeId = __elv_1!!
                    __some_2
                }
            }
        }
        for (a in salvo.platform.core.list.each(view.members)) {
            if (eq__NodeId_NodeId(nodeOf(a.addr), l)) {
                chosen = a.addr
                return
            }
        }
    }
    override fun select(view: RouteView, key: Long?): Int? {
        return chosen
    }
}


class __Platform_HostTcpTransport(bind: NodeEndpoint) : __Platform_Transport(salvo.platform.net.HostTcpTransport(bind))

interface MemNet {
    fun attach(at: NodeEndpoint, sink: Int)
    fun detach(at: NodeEndpoint)
    fun route(from: NodeEndpoint, to: NodeEndpoint, out: salvo.SalvoReply)
    fun partition(a: NodeEndpoint, b: NodeEndpoint)
    fun heal(a: NodeEndpoint, b: NodeEndpoint)
    fun kill(node: NodeEndpoint)
    fun delivered(out: salvo.SalvoReply)
}

class __Mon_MemNet(
    private val inner: MemNet,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : MemNet {
    override fun attach(at: NodeEndpoint, sink: Int) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.attach(at, sink) } finally { lock.unlock() }
    }
    override fun detach(at: NodeEndpoint) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.detach(at) } finally { lock.unlock() }
    }
    override fun route(from: NodeEndpoint, to: NodeEndpoint, out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.route(from, to, out) } finally { lock.unlock() }
    }
    override fun partition(a: NodeEndpoint, b: NodeEndpoint) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.partition(a, b) } finally { lock.unlock() }
    }
    override fun heal(a: NodeEndpoint, b: NodeEndpoint) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.heal(a, b) } finally { lock.unlock() }
    }
    override fun kill(node: NodeEndpoint) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.kill(node) } finally { lock.unlock() }
    }
    override fun delivered(out: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.delivered(out) } finally { lock.unlock() }
    }
}

class MemNetwork : MemNet {
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_MemNetwork> = mutableMapOf()
    var listeners: salvo.platform.core.map.MutMap<NodeEndpoint, Int> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<NodeEndpoint, Int>>(), ::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint)
    var cuts: salvo.platform.core.set.MutSet<String> = salvo.core.set.mutSetOfPlatform(arrayOf<String>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var dead: salvo.platform.core.set.MutSet<NodeEndpoint> = salvo.core.set.mutSetOfPlatform(arrayOf<NodeEndpoint>(), ::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint)
    var count: Int = 0
    override fun attach(at: NodeEndpoint, sink: Int) {
        salvo.core.set.removePlatform(dead, at, ::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint)
        salvo.core.map.putPlatform(listeners, at, sink, ::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint)
    }
    override fun detach(at: NodeEndpoint) {
        salvo.core.map.removePlatform(listeners, at, ::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint)
    }
    override fun route(from: NodeEndpoint, to: NodeEndpoint, out: salvo.SalvoReply) {
        if ((salvo.core.set.containsPlatform(dead, to, ::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint) || salvo.core.set.containsPlatform(cuts, cutKey(from, to), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            salvo.SalvoSched.replyWire(out, null, salvo.OptCodec(salvo.AddrCodec))
            return
        }
        val sink: Int? = salvo.core.map.getPlatform(listeners, to, ::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint)
        if ((sink == null)) {
            salvo.SalvoSched.replyWire(out, null, salvo.OptCodec(salvo.AddrCodec))
            return
        }
        count = (count + 1)
        val sink_1: Int = sink!!
        salvo.SalvoSched.replyWire(out, sink_1, salvo.OptCodec(salvo.AddrCodec))
    }
    override fun partition(a: NodeEndpoint, b: NodeEndpoint) {
        salvo.core.set.addPlatform(cuts, cutKey(a, b), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.core.set.addPlatform(cuts, cutKey(b, a), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
    override fun heal(a: NodeEndpoint, b: NodeEndpoint) {
        salvo.core.set.removePlatform(cuts, cutKey(a, b), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.core.set.removePlatform(cuts, cutKey(b, a), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
    override fun kill(node: NodeEndpoint) {
        salvo.core.map.removePlatform(listeners, node, ::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint)
        salvo.core.set.addPlatform(dead, node, ::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint)
    }
    override fun delivered(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, count, salvo.IntCodec)
    }
}

class __Actor_MemNetwork(private val handler: MemNetwork) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch_MemNetwork_MemNet(handler, msg as __Msg_MemNet)
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_MemNetwork.Attach -> handler.attach(c.at, value as Int)
            is __Cont_MemNetwork.Detach -> handler.detach(value as NodeEndpoint)
            is __Cont_MemNetwork.Route -> handler.route(c.from, c.to, value as salvo.SalvoReply)
            is __Cont_MemNetwork.Partition -> handler.partition(c.a, value as NodeEndpoint)
            is __Cont_MemNetwork.Heal -> handler.heal(c.a, value as NodeEndpoint)
            is __Cont_MemNetwork.Kill -> handler.kill(value as NodeEndpoint)
            is __Cont_MemNetwork.Delivered -> handler.delivered(value as salvo.SalvoReply)
        }
    }

    @Suppress("REDUNDANT_ELSE_IN_WHEN")
    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_MemNetwork.Attach -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_MemNetwork.Detach -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_NodeEndpoint) })(payload)
            is __Cont_MemNetwork.Route -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_MemNetwork.Partition -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_NodeEndpoint) })(payload)
            is __Cont_MemNetwork.Heal -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_NodeEndpoint) })(payload)
            is __Cont_MemNetwork.Kill -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_NodeEndpoint) })(payload)
            is __Cont_MemNetwork.Delivered -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            else -> Pair(false, null)
        }
    }

    companion object {
        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = { proto, payload ->
            when (proto) {
                __PROTO_MemNet -> salvo.salvoDecode(salvo.SalvoBytes(payload), __Codec___Msg_MemNet)?.let { Pair(true, it) } ?: Pair(false, null)
                else -> Pair(false, null)
            }
        }
    }
}

class MemTransport(private val me: NodeEndpoint, private val net: Int) : Transport {
    override fun listen(at: NodeEndpoint, sink: Int): Union2<Unit, Union2<Unreachable, WireFailed>> {
        salvo.SalvoSched.sendWire(net, __Msg_MemNet.Attach(at, sink), __PROTO_MemNet, __Codec___Msg_MemNet)
        return Union2.U1<Unit, Union2<Unreachable, WireFailed>>(salvo.core.result.ok(Unit))
    }
    override fun unlisten(at: NodeEndpoint) {
        salvo.SalvoSched.sendWire(net, __Msg_MemNet.Detach(at), __PROTO_MemNet, __Codec___Msg_MemNet)
    }
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun deliver(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes): Union2<Unit, Union2<Unreachable, WireFailed>> {
        val sink: Int? = run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.OptCodec(salvo.AddrCodec)) })
            salvo.SalvoSched.sendWire(net, __Msg_MemNet.Route(me, to, out), __PROTO_MemNet, __Codec___Msg_MemNet)
            salvo.SalvoSched.awaitReply(__wid) as Int?
        }
        if ((sink == null)) {
            return Union2.U2<Unit, Union2<Unreachable, WireFailed>>(Union2.U1<Unreachable, WireFailed>(salvo.core.result.err(Unreachable(to = to))))
        }
        val sink_1: Int = sink!!
        salvo.SalvoSched.sendWire(sink_1, __Msg_Inbound.ReceiveFrame(me, frame), __PROTO_Inbound, __Codec___Msg_Inbound)
        return Union2.U1<Unit, Union2<Unreachable, WireFailed>>(salvo.core.result.ok(Unit))
    }
    override fun localEndpoint(): NodeEndpoint {
        return me
    }
}

fun cutKey(a: NodeEndpoint, b: NodeEndpoint): String {
    return "${toStr__NodeEndpoint(a)}>${toStr__NodeEndpoint(b)}"
}

fun cmp(a: NodeEndpoint, b: NodeEndpoint): Int {
    val c__c1: Int = salvo.__salvoCompare(a.host, b.host)
    if (!(((c__c1) == (0)))) {
        return c__c1
    }
    val c__c2: Int = (a.port).compareTo(b.port)
    if (!(((c__c2) == (0)))) {
        return c__c2
    }
    return 0
}

fun hash__NodeEndpoint(value: NodeEndpoint): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, (value.host).hashCode().toLong())
    h = salvo.core.compare.mixHash(h, (value.port).hashCode().toLong())
    return h
}

fun eq__NodeEndpoint_NodeEndpoint(a: NodeEndpoint, b: NodeEndpoint): Boolean {
    if (!(((a.host) == (b.host)))) {
        return false
    }
    if (!(((a.port) == (b.port)))) {
        return false
    }
    return true
}

fun hash__NodeId(value: NodeId): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, (value.id).hashCode().toLong())
    return h
}

fun eq__NodeId_NodeId(a: NodeId, b: NodeId): Boolean {
    if (!(((a.id) == (b.id)))) {
        return false
    }
    return true
}

fun hash__Node(value: Node): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, hash__NodeId(value.id))
    h = salvo.core.compare.mixHash(h, hash__NodeEndpoint(value.at))
    return h
}

fun eq__Node_Node(a: Node, b: Node): Boolean {
    if (!(eq__NodeId_NodeId(a.id, b.id))) {
        return false
    }
    if (!(eq__NodeEndpoint_NodeEndpoint(a.at, b.at))) {
        return false
    }
    return true
}


sealed class __Msg_Inbound {
    class ReceiveFrame(val from: NodeEndpoint, val frame: salvo.platform.core.bytes.Bytes) : __Msg_Inbound()
}

object __Codec___Msg_Inbound : salvo.WireCodec<__Msg_Inbound> {
    override fun enc(v: __Msg_Inbound, out: salvo.WireOut) {
        when (v) {
            is __Msg_Inbound.ReceiveFrame -> { out.u8(0); __Codec_NodeEndpoint.enc(v.from, out); salvo.BytesCodec.enc(v.frame, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Inbound = when (inp.u8()) {
            0 -> __Msg_Inbound.ReceiveFrame(__Codec_NodeEndpoint.dec(inp), salvo.BytesCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Inbound: String = "8e46ddb2a3e90b03"


sealed class __Msg_Outbound {
    class SendFrame(val to: NodeEndpoint, val frame: salvo.platform.core.bytes.Bytes) : __Msg_Outbound()
}

object __Codec___Msg_Outbound : salvo.WireCodec<__Msg_Outbound> {
    override fun enc(v: __Msg_Outbound, out: salvo.WireOut) {
        when (v) {
            is __Msg_Outbound.SendFrame -> { out.u8(0); __Codec_NodeEndpoint.enc(v.to, out); salvo.BytesCodec.enc(v.frame, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_Outbound = when (inp.u8()) {
            0 -> __Msg_Outbound.SendFrame(__Codec_NodeEndpoint.dec(inp), salvo.BytesCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_Outbound: String = "e754249e848e9986"


sealed class __Msg_NodeGroup {
    class Members(val out: salvo.SalvoReply) : __Msg_NodeGroup()
    class Subscribe(val w: Int) : __Msg_NodeGroup()
    class Leave() : __Msg_NodeGroup()
}

object __Codec___Msg_NodeGroup : salvo.WireCodec<__Msg_NodeGroup> {
    override fun enc(v: __Msg_NodeGroup, out: salvo.WireOut) {
        when (v) {
            is __Msg_NodeGroup.Members -> { out.u8(0); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_NodeGroup.Subscribe -> { out.u8(1); salvo.AddrCodec.enc(v.w, out) }
            is __Msg_NodeGroup.Leave -> { out.u8(2) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_NodeGroup = when (inp.u8()) {
            0 -> __Msg_NodeGroup.Members(salvo.ReplyCodec.dec(inp))
            1 -> __Msg_NodeGroup.Subscribe(salvo.AddrCodec.dec(inp))
            2 -> __Msg_NodeGroup.Leave()
        else -> throw salvo.WireError()
    }
}

const val __PROTO_NodeGroup: String = "471318a2c85d1f6d"


sealed class __Msg_NodeGroupWatcher {
    class Joined(val n: Node) : __Msg_NodeGroupWatcher()
    class Left(val n: Node, val why: String) : __Msg_NodeGroupWatcher()
}

object __Codec___Msg_NodeGroupWatcher : salvo.WireCodec<__Msg_NodeGroupWatcher> {
    override fun enc(v: __Msg_NodeGroupWatcher, out: salvo.WireOut) {
        when (v) {
            is __Msg_NodeGroupWatcher.Joined -> { out.u8(0); __Codec_Node.enc(v.n, out) }
            is __Msg_NodeGroupWatcher.Left -> { out.u8(1); __Codec_Node.enc(v.n, out); salvo.StrCodec.enc(v.why, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_NodeGroupWatcher = when (inp.u8()) {
            0 -> __Msg_NodeGroupWatcher.Joined(__Codec_Node.dec(inp))
            1 -> __Msg_NodeGroupWatcher.Left(__Codec_Node.dec(inp), salvo.StrCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_NodeGroupWatcher: String = "db3e0aa5831c2e44"


sealed class __Msg_ActorGroup {
    class Join(val member: Int) : __Msg_ActorGroup()
    class Leave(val member: Int) : __Msg_ActorGroup()
    class Members(val out: salvo.SalvoReply) : __Msg_ActorGroup()
    class Subscribe(val w: Int) : __Msg_ActorGroup()
    class Refresh() : __Msg_ActorGroup()
}

object __Codec___Msg_ActorGroup : salvo.WireCodec<__Msg_ActorGroup> {
    override fun enc(v: __Msg_ActorGroup, out: salvo.WireOut) {
        when (v) {
            is __Msg_ActorGroup.Join -> { out.u8(0); salvo.AddrCodec.enc(v.member, out) }
            is __Msg_ActorGroup.Leave -> { out.u8(1); salvo.AddrCodec.enc(v.member, out) }
            is __Msg_ActorGroup.Members -> { out.u8(2); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_ActorGroup.Subscribe -> { out.u8(3); salvo.AddrCodec.enc(v.w, out) }
            is __Msg_ActorGroup.Refresh -> { out.u8(4) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_ActorGroup = when (inp.u8()) {
            0 -> __Msg_ActorGroup.Join(salvo.AddrCodec.dec(inp))
            1 -> __Msg_ActorGroup.Leave(salvo.AddrCodec.dec(inp))
            2 -> __Msg_ActorGroup.Members(salvo.ReplyCodec.dec(inp))
            3 -> __Msg_ActorGroup.Subscribe(salvo.AddrCodec.dec(inp))
            4 -> __Msg_ActorGroup.Refresh()
        else -> throw salvo.WireError()
    }
}

const val __PROTO_ActorGroup: String = "695f43128bdada4a"


sealed class __Msg_ActorGroupWatcher {
    class Joined(val member: Int) : __Msg_ActorGroupWatcher()
    class Left(val member: Int) : __Msg_ActorGroupWatcher()
}

object __Codec___Msg_ActorGroupWatcher : salvo.WireCodec<__Msg_ActorGroupWatcher> {
    override fun enc(v: __Msg_ActorGroupWatcher, out: salvo.WireOut) {
        when (v) {
            is __Msg_ActorGroupWatcher.Joined -> { out.u8(0); salvo.AddrCodec.enc(v.member, out) }
            is __Msg_ActorGroupWatcher.Left -> { out.u8(1); salvo.AddrCodec.enc(v.member, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_ActorGroupWatcher = when (inp.u8()) {
            0 -> __Msg_ActorGroupWatcher.Joined(salvo.AddrCodec.dec(inp))
            1 -> __Msg_ActorGroupWatcher.Left(salvo.AddrCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_ActorGroupWatcher: String = "9fa424e5858bb3bd"


sealed class __Msg_MemNet {
    class Attach(val at: NodeEndpoint, val sink: Int) : __Msg_MemNet()
    class Detach(val at: NodeEndpoint) : __Msg_MemNet()
    class Route(val from: NodeEndpoint, val to: NodeEndpoint, val out: salvo.SalvoReply) : __Msg_MemNet()
    class Partition(val a: NodeEndpoint, val b: NodeEndpoint) : __Msg_MemNet()
    class Heal(val a: NodeEndpoint, val b: NodeEndpoint) : __Msg_MemNet()
    class Kill(val node: NodeEndpoint) : __Msg_MemNet()
    class Delivered(val out: salvo.SalvoReply) : __Msg_MemNet()
}

object __Codec___Msg_MemNet : salvo.WireCodec<__Msg_MemNet> {
    override fun enc(v: __Msg_MemNet, out: salvo.WireOut) {
        when (v) {
            is __Msg_MemNet.Attach -> { out.u8(0); __Codec_NodeEndpoint.enc(v.at, out); salvo.AddrCodec.enc(v.sink, out) }
            is __Msg_MemNet.Detach -> { out.u8(1); __Codec_NodeEndpoint.enc(v.at, out) }
            is __Msg_MemNet.Route -> { out.u8(2); __Codec_NodeEndpoint.enc(v.from, out); __Codec_NodeEndpoint.enc(v.to, out); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_MemNet.Partition -> { out.u8(3); __Codec_NodeEndpoint.enc(v.a, out); __Codec_NodeEndpoint.enc(v.b, out) }
            is __Msg_MemNet.Heal -> { out.u8(4); __Codec_NodeEndpoint.enc(v.a, out); __Codec_NodeEndpoint.enc(v.b, out) }
            is __Msg_MemNet.Kill -> { out.u8(5); __Codec_NodeEndpoint.enc(v.node, out) }
            is __Msg_MemNet.Delivered -> { out.u8(6); salvo.ReplyCodec.enc(v.out, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_MemNet = when (inp.u8()) {
            0 -> __Msg_MemNet.Attach(__Codec_NodeEndpoint.dec(inp), salvo.AddrCodec.dec(inp))
            1 -> __Msg_MemNet.Detach(__Codec_NodeEndpoint.dec(inp))
            2 -> __Msg_MemNet.Route(__Codec_NodeEndpoint.dec(inp), __Codec_NodeEndpoint.dec(inp), salvo.ReplyCodec.dec(inp))
            3 -> __Msg_MemNet.Partition(__Codec_NodeEndpoint.dec(inp), __Codec_NodeEndpoint.dec(inp))
            4 -> __Msg_MemNet.Heal(__Codec_NodeEndpoint.dec(inp), __Codec_NodeEndpoint.dec(inp))
            5 -> __Msg_MemNet.Kill(__Codec_NodeEndpoint.dec(inp))
            6 -> __Msg_MemNet.Delivered(salvo.ReplyCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

const val __PROTO_MemNet: String = "2815c14392023d5e"


sealed class __Cont_Sending {
    class SendFrame(val to: NodeEndpoint) : __Cont_Sending()
}


sealed class __Cont_Receiving {
    class ReceiveFrame(val from: NodeEndpoint) : __Cont_Receiving()
}


sealed class __Cont_StaticNodeGroup {
    class Members() : __Cont_StaticNodeGroup()
    class Subscribe() : __Cont_StaticNodeGroup()
    class Control(val from: NodeId) : __Cont_StaticNodeGroup()
}


sealed class __Priv_StaticNodeGroup {
    object Init : __Priv_StaticNodeGroup()
    class Control(val from: NodeId, val data: salvo.platform.core.bytes.Bytes) : __Priv_StaticNodeGroup()
}


sealed class __Cont_GossipNodeGroup {
    class Members() : __Cont_GossipNodeGroup()
    class Subscribe() : __Cont_GossipNodeGroup()
    class Control(val from: NodeId) : __Cont_GossipNodeGroup()
}


sealed class __Priv_GossipNodeGroup {
    object Init : __Priv_GossipNodeGroup()
    class Control(val from: NodeId, val data: salvo.platform.core.bytes.Bytes) : __Priv_GossipNodeGroup()
}


sealed class __Cont_ActorGrouping {
    class Join() : __Cont_ActorGrouping()
    class Leave() : __Cont_ActorGrouping()
    class Members() : __Cont_ActorGrouping()
    class Subscribe() : __Cont_ActorGrouping()
    class Joined() : __Cont_ActorGrouping()
    class Left(val n: Node) : __Cont_ActorGrouping()
    class Control(val from: NodeId) : __Cont_ActorGrouping()
}


sealed class __Priv_ActorGrouping {
    object Init : __Priv_ActorGrouping()
    class Control(val from: NodeId, val data: salvo.platform.core.bytes.Bytes) : __Priv_ActorGrouping()
}


sealed class __Cont_MemNetwork {
    class Attach(val at: NodeEndpoint) : __Cont_MemNetwork()
    class Detach() : __Cont_MemNetwork()
    class Route(val from: NodeEndpoint, val to: NodeEndpoint) : __Cont_MemNetwork()
    class Partition(val a: NodeEndpoint) : __Cont_MemNetwork()
    class Heal(val a: NodeEndpoint) : __Cont_MemNetwork()
    class Kill() : __Cont_MemNetwork()
    class Delivered() : __Cont_MemNetwork()
}

class __Stub_Inbound(private val addr: Int) : Inbound {
    override fun receiveFrame(from: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes) {
        salvo.SalvoSched.sendWire(addr, __Msg_Inbound.ReceiveFrame(from, frame), __PROTO_Inbound, __Codec___Msg_Inbound)
    }
}

class __Stub_Outbound(private val addr: Int) : Outbound {
    override fun sendFrame(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes) {
        salvo.SalvoSched.sendWire(addr, __Msg_Outbound.SendFrame(to, frame), __PROTO_Outbound, __Codec___Msg_Outbound)
    }
}

class __Stub_NodeGroup(private val addr: Int) : NodeGroup {
    override fun members(out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_NodeGroup.Members(out), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
    }
    override fun subscribe(w: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_NodeGroup.Subscribe(w), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
    }
    override fun leave() {
        salvo.SalvoSched.sendWire(addr, __Msg_NodeGroup.Leave(), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
    }
}

class __Stub_NodeGroupWatcher(private val addr: Int) : NodeGroupWatcher {
    override fun joined(n: Node) {
        salvo.SalvoSched.sendWire(addr, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
    }
    override fun left(n: Node, why: String) {
        salvo.SalvoSched.sendWire(addr, __Msg_NodeGroupWatcher.Left(n, why), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
    }
}

class __Stub_ActorGroup(private val addr: Int) : ActorGroup {
    override fun join(member: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_ActorGroup.Join(member), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
    }
    override fun leave(member: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_ActorGroup.Leave(member), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
    }
    override fun members(out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_ActorGroup.Members(out), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
    }
    override fun subscribe(w: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_ActorGroup.Subscribe(w), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
    }
    override fun refresh() {
        salvo.SalvoSched.sendWire(addr, __Msg_ActorGroup.Refresh(), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
    }
}

class __Stub_ActorGroupWatcher(private val addr: Int) : ActorGroupWatcher {
    override fun joined(member: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_ActorGroupWatcher.Joined(member), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
    }
    override fun left(member: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_ActorGroupWatcher.Left(member), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
    }
}

class __Stub_MemNet(private val addr: Int) : MemNet {
    override fun attach(at: NodeEndpoint, sink: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_MemNet.Attach(at, sink), __PROTO_MemNet, __Codec___Msg_MemNet)
    }
    override fun detach(at: NodeEndpoint) {
        salvo.SalvoSched.sendWire(addr, __Msg_MemNet.Detach(at), __PROTO_MemNet, __Codec___Msg_MemNet)
    }
    override fun route(from: NodeEndpoint, to: NodeEndpoint, out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_MemNet.Route(from, to, out), __PROTO_MemNet, __Codec___Msg_MemNet)
    }
    override fun partition(a: NodeEndpoint, b: NodeEndpoint) {
        salvo.SalvoSched.sendWire(addr, __Msg_MemNet.Partition(a, b), __PROTO_MemNet, __Codec___Msg_MemNet)
    }
    override fun heal(a: NodeEndpoint, b: NodeEndpoint) {
        salvo.SalvoSched.sendWire(addr, __Msg_MemNet.Heal(a, b), __PROTO_MemNet, __Codec___Msg_MemNet)
    }
    override fun kill(node: NodeEndpoint) {
        salvo.SalvoSched.sendWire(addr, __Msg_MemNet.Kill(node), __PROTO_MemNet, __Codec___Msg_MemNet)
    }
    override fun delivered(out: salvo.SalvoReply) {
        salvo.SalvoSched.sendWire(addr, __Msg_MemNet.Delivered(out), __PROTO_MemNet, __Codec___Msg_MemNet)
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Sending_Outbound(__handler: Sending, __msg: __Msg_Outbound) {
    when {
        (__msg is __Msg_Outbound.SendFrame) -> {
            val to = (__msg as __Msg_Outbound.SendFrame).to
            val frame = (__msg as __Msg_Outbound.SendFrame).frame
            __handler.sendFrame(to, frame)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_Receiving_Inbound(__handler: Receiving, __msg: __Msg_Inbound) {
    when {
        (__msg is __Msg_Inbound.ReceiveFrame) -> {
            val from = (__msg as __Msg_Inbound.ReceiveFrame).from
            val frame = (__msg as __Msg_Inbound.ReceiveFrame).frame
            __handler.receiveFrame(from, frame)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_StaticNodeGroup_NodeGroup(__handler: StaticNodeGroup, __msg: __Msg_NodeGroup) {
    when {
        (__msg is __Msg_NodeGroup.Members) -> {
            val out = (__msg as __Msg_NodeGroup.Members).out
            __handler.members(out)
        }
        (__msg is __Msg_NodeGroup.Subscribe) -> {
            val w = (__msg as __Msg_NodeGroup.Subscribe).w
            __handler.subscribe(w)
        }
        (__msg is __Msg_NodeGroup.Leave) -> {
            __handler.leave()
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_priv_StaticNodeGroup(__handler: StaticNodeGroup, __msg: __Priv_StaticNodeGroup) {
    when {
        (__msg is __Priv_StaticNodeGroup.Init) -> {
            __handler.init()
        }
        (__msg is __Priv_StaticNodeGroup.Control) -> {
            val from = (__msg as __Priv_StaticNodeGroup.Control).from
            val data = (__msg as __Priv_StaticNodeGroup.Control).data
            __handler.control(from, data)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_GossipNodeGroup_NodeGroup(__handler: GossipNodeGroup, __msg: __Msg_NodeGroup) {
    when {
        (__msg is __Msg_NodeGroup.Members) -> {
            val out = (__msg as __Msg_NodeGroup.Members).out
            __handler.members(out)
        }
        (__msg is __Msg_NodeGroup.Subscribe) -> {
            val w = (__msg as __Msg_NodeGroup.Subscribe).w
            __handler.subscribe(w)
        }
        (__msg is __Msg_NodeGroup.Leave) -> {
            __handler.leave()
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_priv_GossipNodeGroup(__handler: GossipNodeGroup, __msg: __Priv_GossipNodeGroup) {
    when {
        (__msg is __Priv_GossipNodeGroup.Init) -> {
            __handler.init()
        }
        (__msg is __Priv_GossipNodeGroup.Control) -> {
            val from = (__msg as __Priv_GossipNodeGroup.Control).from
            val data = (__msg as __Priv_GossipNodeGroup.Control).data
            __handler.control(from, data)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_ActorGrouping_ActorGroup(__handler: ActorGrouping, __msg: __Msg_ActorGroup) {
    when {
        (__msg is __Msg_ActorGroup.Join) -> {
            val member = (__msg as __Msg_ActorGroup.Join).member
            __handler.join(member)
        }
        (__msg is __Msg_ActorGroup.Leave) -> {
            val member = (__msg as __Msg_ActorGroup.Leave).member
            __handler.leave(member)
        }
        (__msg is __Msg_ActorGroup.Members) -> {
            val out = (__msg as __Msg_ActorGroup.Members).out
            __handler.members(out)
        }
        (__msg is __Msg_ActorGroup.Subscribe) -> {
            val w = (__msg as __Msg_ActorGroup.Subscribe).w
            __handler.subscribe(w)
        }
        (__msg is __Msg_ActorGroup.Refresh) -> {
            __handler.refresh()
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_ActorGrouping_NodeGroupWatcher(__handler: ActorGrouping, __msg: __Msg_NodeGroupWatcher) {
    when {
        (__msg is __Msg_NodeGroupWatcher.Joined) -> {
            val n = (__msg as __Msg_NodeGroupWatcher.Joined).n
            __handler.joined(n)
        }
        (__msg is __Msg_NodeGroupWatcher.Left) -> {
            val n = (__msg as __Msg_NodeGroupWatcher.Left).n
            val why = (__msg as __Msg_NodeGroupWatcher.Left).why
            __handler.left(n, why)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_priv_ActorGrouping(__handler: ActorGrouping, __msg: __Priv_ActorGrouping) {
    when {
        (__msg is __Priv_ActorGrouping.Init) -> {
            __handler.init()
        }
        (__msg is __Priv_ActorGrouping.Control) -> {
            val from = (__msg as __Priv_ActorGrouping.Control).from
            val data = (__msg as __Priv_ActorGrouping.Control).data
            __handler.control(from, data)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun __dispatch_MemNetwork_MemNet(__handler: MemNetwork, __msg: __Msg_MemNet) {
    when {
        (__msg is __Msg_MemNet.Attach) -> {
            val at = (__msg as __Msg_MemNet.Attach).at
            val sink = (__msg as __Msg_MemNet.Attach).sink
            __handler.attach(at, sink)
        }
        (__msg is __Msg_MemNet.Detach) -> {
            val at = (__msg as __Msg_MemNet.Detach).at
            __handler.detach(at)
        }
        (__msg is __Msg_MemNet.Route) -> {
            val from = (__msg as __Msg_MemNet.Route).from
            val to = (__msg as __Msg_MemNet.Route).to
            val out = (__msg as __Msg_MemNet.Route).out
            __handler.route(from, to, out)
        }
        (__msg is __Msg_MemNet.Partition) -> {
            val a = (__msg as __Msg_MemNet.Partition).a
            val b = (__msg as __Msg_MemNet.Partition).b
            __handler.partition(a, b)
        }
        (__msg is __Msg_MemNet.Heal) -> {
            val a = (__msg as __Msg_MemNet.Heal).a
            val b = (__msg as __Msg_MemNet.Heal).b
            __handler.heal(a, b)
        }
        (__msg is __Msg_MemNet.Kill) -> {
            val node = (__msg as __Msg_MemNet.Kill).node
            __handler.kill(node)
        }
        (__msg is __Msg_MemNet.Delivered) -> {
            val out = (__msg as __Msg_MemNet.Delivered).out
            __handler.delivered(out)
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

