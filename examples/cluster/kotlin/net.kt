package salvo.net

import salvo.*
import salvo.core.actor.eq
import salvo.core.actor.pool
import salvo.core.checked.detach
import salvo.core.compare.mixHash
import salvo.core.list.addPlatform as addPlatform__core_list
import salvo.core.list.all
import salvo.core.list.at
import salvo.core.list.count
import salvo.core.list.getPlatform as getPlatform__core_list
import salvo.core.list.last
import salvo.core.list.partition
import salvo.core.list.removeAtPlatform
import salvo.core.list.sizePlatform
import salvo.core.map.containsKeyPlatform
import salvo.core.map.getPlatform as getPlatform__core_map
import salvo.core.map.keysPlatform
import salvo.core.map.putPlatform
import salvo.core.map.removePlatform as removePlatform__core_map
import salvo.core.result.err
import salvo.core.result.ok
import salvo.core.seq.count
import salvo.core.set.addPlatform as addPlatform__core_set
import salvo.core.set.containsPlatform
import salvo.core.set.removePlatform as removePlatform__core_set
import salvo.runtime.Delivered
import salvo.runtime.routing.connected as connected__runtime_routing
import salvo.runtime.routing.controlFrame
import salvo.runtime.routing.credits as credits__runtime_routing
import salvo.runtime.routing.deliver
import salvo.runtime.routing.hereNode
import salvo.runtime.routing.identity
import salvo.runtime.routing.localProtocols
import salvo.runtime.routing.newNode as newNode__runtime_routing
import salvo.runtime.routing.nodeLeft as nodeLeft__runtime_routing
import salvo.runtime.routing.outboundBound
import salvo.runtime.routing.peerProtocol as peerProtocol__runtime_routing
import salvo.runtime.routing.pending as pending__runtime_routing
import salvo.runtime.routing.poolAt as poolAt__runtime_routing
import salvo.runtime.routing.route
import salvo.runtime.routing.sendControl as sendControl__runtime_routing
import salvo.runtime.routing.setPeerProtocols as setPeerProtocols__runtime_routing
import salvo.runtime.routing.viewMembers as viewMembers__runtime_routing
import salvo.runtime.routing.viewRefresh as viewRefresh__runtime_routing
import salvo.runtime.routing.viewSet as viewSet__runtime_routing
import salvo.runtime.routing.viewVersion as viewVersion__runtime_routing
import salvo.runtime.routing.viewWait as viewWait__runtime_routing
import salvo.time.Duration
import salvo.time.__Codec_Duration
import salvo.time.nanos

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
    when (e) {
        is Union2.U1<*, *> -> {
            return "unreachable: ${toStr__NodeEndpoint((e.value as Unreachable).to)}"
        }
        is Union2.U2<*, *> -> {
            return "wire failed to ${toStr__NodeEndpoint((e.value as WireFailed).to)}: ${(e.value as WireFailed).reason}"
        }
    }
}

interface Inbound {
    fun receiveFrame(from: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes)
}

class __Stub_Inbound(private val addr: Int) : Inbound {
    override fun receiveFrame(from: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes) {
        salvo.SalvoSched.sendWire(addr, __Msg_Inbound.ReceiveFrame(from, frame), __PROTO_Inbound, __Codec___Msg_Inbound)
    }
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

/** [protocol-hash] The canonical hash of `Inbound`. */
const val __PROTO_Inbound: String = "8e46ddb2a3e90b03"

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

// The interface a `platform handler` of `Transport` implements [platform-abi].
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
    return NodeId(id = hereNode())
}

fun newNode(): NodeId {
    return NodeId(id = newNode__runtime_routing())
}

fun poolAt(node: NodeId, size: Int): Int {
    return (poolAt__runtime_routing(node.id, size))
}

fun addRoute(node: NodeId, at: NodeEndpoint) {
    route(node.id, salvo.salvoEncode(at, __Codec_NodeEndpoint))
}

fun routeFrames(out: Int) {
    bindOutboundPlatform(hereNode(), (out), ::forwardFrame)
    outboundBound()
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun forwardFrame(out: Int, to: salvo.platform.core.bytes.Bytes, frame: salvo.platform.core.bytes.Bytes) {
    var __is1 = salvo.salvoDecode(to, __Codec_NodeEndpoint)
    if (__is1 != null) {
        val ep = __is1 as NodeEndpoint
        val sending: Int = (out)
        salvo.SalvoSched.sendWire(sending, __Msg_Outbound.SendFrame(ep, frame), __PROTO_Outbound, __Codec___Msg_Outbound)
    }
}

fun bindOutboundPlatform(node: Long, out: Int, hook: (Int, salvo.platform.core.bytes.Bytes, salvo.platform.core.bytes.Bytes) -> Unit) {
    return salvo.platform.net.bindOutbound(node, out, hook)
}

fun deliverFrame(data: salvo.platform.core.bytes.Bytes): Boolean {
    return deliver(data)
}

fun credits(a: Int): Int? {
    val c = credits__runtime_routing((a))
    if (c == null) {
        return null
    }
    if (c < 0) {
        return 0
    }
    return c
}

interface Outbound {
    fun sendFrame(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes)
}

class __Stub_Outbound(private val addr: Int) : Outbound {
    override fun sendFrame(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes) {
        salvo.SalvoSched.sendWire(addr, __Msg_Outbound.SendFrame(to, frame), __PROTO_Outbound, __Codec___Msg_Outbound)
    }
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

/** [protocol-hash] The canonical hash of `Outbound`. */
const val __PROTO_Outbound: String = "e754249e848e9986"

class Sending(private val __dep_Transport: Transport) : Outbound {
    val __mailboxCapacity: Int = 256
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_Sending> = mutableMapOf()

    override fun sendFrame(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes) {
        val _sent = __dep_Transport.deliver(to, frame)
    }
}

sealed class __Cont_Sending {
    class SendFrame(val to: NodeEndpoint) : __Cont_Sending()
}

class __Actor_Sending(private val handler: Sending) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Outbound)
    }

    private fun __dispatch(m: __Msg_Outbound) {
        when (m) {
            is __Msg_Outbound.SendFrame -> handler.sendFrame(m.to, m.frame)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
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
        val _delivered = deliverFrame(frame)
    }
}

sealed class __Cont_Receiving {
    class ReceiveFrame(val from: NodeEndpoint) : __Cont_Receiving()
}

class __Actor_Receiving(private val handler: Receiving) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_Inbound)
    }

    private fun __dispatch(m: __Msg_Inbound) {
        when (m) {
            is __Msg_Inbound.ReceiveFrame -> handler.receiveFrame(m.from, m.frame)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
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
    return connected__runtime_routing()
}

fun connect__NodeEndpoint(transport: Transport, me: NodeEndpoint): Boolean {
    return connect__NodeEndpoint_Pool(transport, me, pool(1))
}

fun connect__NodeEndpoint_Pool(transport: Transport, me: NodeEndpoint, on: Int): Boolean {
    if (connected()) {
        return false
    }
    val sending = run { val __h = Sending(transport); val __a = salvo.SalvoSched.spawn(on, __h.__mailboxCapacity, __Actor_Sending(__h), __Actor_Sending.__DECODE); __a }
    val receiving = run { val __h = Receiving(); val __a = salvo.SalvoSched.spawn(on, __h.__mailboxCapacity, __Actor_Receiving(__h), __Actor_Receiving.__DECODE); __a }
    routeFrames(sending)
    val _listening = transport.listen(me, receiving)
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

/** [protocol-hash] The canonical hash of `NodeGroup`. */
const val __PROTO_NodeGroup: String = "471318a2c85d1f6d"

interface NodeGroupWatcher {
    fun joined(n: Node)
    fun left(n: Node, why: String)
}

class __Stub_NodeGroupWatcher(private val addr: Int) : NodeGroupWatcher {
    override fun joined(n: Node) {
        salvo.SalvoSched.sendWire(addr, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
    }
    override fun left(n: Node, why: String) {
        salvo.SalvoSched.sendWire(addr, __Msg_NodeGroupWatcher.Left(n, why), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
    }
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

/** [protocol-hash] The canonical hash of `NodeGroupWatcher`. */
const val __PROTO_NodeGroupWatcher: String = "db3e0aa5831c2e44"

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
    sendControl__runtime_routing(to.id, channel, payload)
}

fun nodeLeft(node: NodeId) {
    nodeLeft__runtime_routing(node.id)
}

fun setPeerProtocols(node: NodeId, table: List<Pair<String, String>>) {
    setPeerProtocols__runtime_routing(node.id, table)
}

fun peerProtocol(node: NodeId, protocol: String): String? {
    return peerProtocol__runtime_routing(node.id, protocol)
}

fun helloFrame(transport: Transport, group: String): salvo.platform.core.bytes.Bytes {
    val hello: Union4<Hello, Ack, Leaving, Intro> = Union4.U1<Hello, Ack, Leaving, Intro>(Hello(group = group, at = transport.localEndpoint(), protocols = localProtocols()))
    return controlFrame("", salvo.salvoEncode(hello, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro)))
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
    val msg = salvo.salvoDecode(data, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro))
    when (msg) {
        is Union4.U1<*, *, *, *> -> {
            if (!(((msg?.value as Hello).group) == (group))) {
                return null
            }
            addRoute(from, (msg?.value as Hello).at)
            setPeerProtocols(from, (msg?.value as Hello).protocols)
            val ack: Union4<Hello, Ack, Leaving, Intro> = Union4.U2<Hello, Ack, Leaving, Intro>(Ack(group = group, at = transport.localEndpoint(), protocols = localProtocols()))
            sendControl(from, "", salvo.salvoEncode(ack, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro)))
            return Union3.U1<PeerHello, PeerGone, PeerIntro>(PeerHello(node = from, at = (msg?.value as Hello).at))
        }
        is Union4.U2<*, *, *, *> -> {
            if (!(((msg?.value as Ack).group) == (group))) {
                return null
            }
            addRoute(from, (msg?.value as Ack).at)
            setPeerProtocols(from, (msg?.value as Ack).protocols)
            return Union3.U1<PeerHello, PeerGone, PeerIntro>(PeerHello(node = from, at = (msg?.value as Ack).at))
        }
        is Union4.U3<*, *, *, *> -> {
            nodeLeft(from)
            return Union3.U2<PeerHello, PeerGone, PeerIntro>(PeerGone(node = from))
        }
        is Union4.U4<*, *, *, *> -> {
            return Union3.U3<PeerHello, PeerGone, PeerIntro>(PeerIntro(peers = (msg?.value as Intro).peers))
        }
        null -> {
            return null
        }
    }
}

fun introduce(node: NodeId, peers: List<NodeEndpoint>) {
    val intro: Union4<Hello, Ack, Leaving, Intro> = Union4.U4<Hello, Ack, Leaving, Intro>(Intro(peers = peers))
    sendControl(node, "", salvo.salvoEncode(intro, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro)))
}

fun leaveGroup(known: Map<NodeId, Node>) {
    for (id in salvo.platform.core.list.each(keysPlatform(known))) {
        val leaving: Union4<Hello, Ack, Leaving, Intro> = Union4.U3<Hello, Ack, Leaving, Intro>(Leaving())
        sendControl(id, "", salvo.salvoEncode(leaving, salvo.Union4Codec(__Codec_Hello, __Codec_Ack, __Codec_Leaving, __Codec_Intro)))
    }
}

class StaticNodeGroup(private val name: String, private val all: List<NodeEndpoint>, private val __dep_Transport: Transport) : NodeGroup {
    private var known: salvo.platform.core.map.MutMap<NodeId, Node> = salvo.SalvoHashMap<NodeId, Node>(::hash__NodeId, ::eq__NodeId_NodeId).also { __m -> __m.putAll(listOf()) }
    private var watchers: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_StaticNodeGroup> = mutableMapOf()

    override fun members(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, knownNodes(known), salvo.ListCodec(__Codec_Node))
    }

    override fun subscribe(w: Int) {
        for (id in salvo.platform.core.list.each(keysPlatform(known))) {
            val n = getPlatform__core_map(known, id)
            if (!(n == null)) {
                salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
            }
        }
        addPlatform__core_list(watchers, w)
    }

    override fun leave() {
        leaveGroup(known)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    fun control(from: NodeId, data: salvo.platform.core.bytes.Bytes) {
        val event = handshake(__dep_Transport, name, from, data)
        when (event) {
            is Union3.U1<*, *, *> -> {
                if (containsKeyPlatform(known, (event?.value as PeerHello).node)) {
                    return
                }
                val n = Node(id = (event?.value as PeerHello).node, at = (event?.value as PeerHello).at)
                putPlatform(known, (event?.value as PeerHello).node, n)
                for (w in salvo.platform.core.list.each(watchers)) {
                    salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
                }
            }
            is Union3.U2<*, *, *> -> {
                val n = removePlatform__core_map(known, (event?.value as PeerGone).node)
                if (n == null) {
                    return
                }
                for (w in salvo.platform.core.list.each(watchers)) {
                    salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Left(n, "left"), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
                }
            }
            is Union3.U3<*, *, *> -> {
            }
            null -> {
            }
        }
    }

    fun init() {
        val me = __dep_Transport.localEndpoint()
        val _connected = connect__NodeEndpoint(__dep_Transport, me)
        salvo.SalvoSched.watchControl("", __addr!!) { __n, __d -> __Priv_StaticNodeGroup.Control(NodeId(__n), salvo.SalvoBytes(__d)) }
        for (e in salvo.platform.core.list.each(all)) {
            if (!eq__NodeEndpoint_NodeEndpoint(e, me)) {
                val _sent = __dep_Transport.deliver(e, helloFrame(__dep_Transport, name))
            }
        }
    }
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

class __Actor_StaticNodeGroup(private val handler: StaticNodeGroup) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_NodeGroup -> __dispatchNodeGroup(msg)
            is __Priv_StaticNodeGroup -> __dispatchPriv(msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    private fun __dispatchNodeGroup(m: __Msg_NodeGroup) {
        when (m) {
            is __Msg_NodeGroup.Members -> handler.members(m.out)
            is __Msg_NodeGroup.Subscribe -> handler.subscribe(m.w)
            is __Msg_NodeGroup.Leave -> handler.leave()
        }
    }

    private fun __dispatchPriv(m: __Priv_StaticNodeGroup) {
        when (m) {
            is __Priv_StaticNodeGroup.Init -> handler.init()
            is __Priv_StaticNodeGroup.Control -> handler.control(m.from, m.data)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
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

fun knownNodes(known: Map<NodeId, Node>): List<Node> {
    val allKnown: salvo.platform.core.list.MutList<Node> = mutableListOf<Node>()
    for (id in salvo.platform.core.list.each(keysPlatform(known))) {
        val n = getPlatform__core_map(known, id)
        if (!(n == null)) {
            addPlatform__core_list(allKnown, n)
        }
    }
    return allKnown
}

class GossipNodeGroup(private val name: String, private val seeds: List<NodeEndpoint>, private val __dep_Transport: Transport) : NodeGroup {
    private var known: salvo.platform.core.map.MutMap<NodeId, Node> = salvo.SalvoHashMap<NodeId, Node>(::hash__NodeId, ::eq__NodeId_NodeId).also { __m -> __m.putAll(listOf()) }
    private var dialed: salvo.platform.core.set.MutSet<String> = linkedSetOf<String>().also { __s -> __s.addAll(listOf()) }
    private var watchers: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_GossipNodeGroup> = mutableMapOf()

    override fun members(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, knownNodes(known), salvo.ListCodec(__Codec_Node))
    }

    override fun subscribe(w: Int) {
        for (id in salvo.platform.core.list.each(keysPlatform(known))) {
            val n = getPlatform__core_map(known, id)
            if (!(n == null)) {
                salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
            }
        }
        addPlatform__core_list(watchers, w)
    }

    override fun leave() {
        leaveGroup(known)
    }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    fun control(from: NodeId, data: salvo.platform.core.bytes.Bytes) {
        val event = handshake(__dep_Transport, name, from, data)
        when (event) {
            is Union3.U1<*, *, *> -> {
                if (containsKeyPlatform(known, (event?.value as PeerHello).node)) {
                    return
                }
                val others: salvo.platform.core.list.MutList<NodeEndpoint> = mutableListOf<NodeEndpoint>()
                for (id in salvo.platform.core.list.each(keysPlatform(known))) {
                    val n = getPlatform__core_map(known, id)
                    if (!(n == null)) {
                        addPlatform__core_list(others, n.at)
                    }
                    introduce(id, listOf<NodeEndpoint>((event?.value as PeerHello).at))
                }
                introduce((event?.value as PeerHello).node, others)
                addPlatform__core_set(dialed, toStr__NodeEndpoint((event?.value as PeerHello).at))
                val n = Node(id = (event?.value as PeerHello).node, at = (event?.value as PeerHello).at)
                putPlatform(known, (event?.value as PeerHello).node, n)
                for (w in salvo.platform.core.list.each(watchers)) {
                    salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
                }
            }
            is Union3.U2<*, *, *> -> {
                val n = removePlatform__core_map(known, (event?.value as PeerGone).node)
                if (n == null) {
                    return
                }
                for (w in salvo.platform.core.list.each(watchers)) {
                    salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Left(n, "left"), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
                }
            }
            is Union3.U3<*, *, *> -> {
                for (e in salvo.platform.core.list.each((event?.value as PeerIntro).peers)) {
                    dial(__dep_Transport, dialed, name, e)
                }
            }
            null -> {
            }
        }
    }

    fun init() {
        val me = __dep_Transport.localEndpoint()
        val _connected = connect__NodeEndpoint(__dep_Transport, me)
        salvo.SalvoSched.watchControl("", __addr!!) { __n, __d -> __Priv_GossipNodeGroup.Control(NodeId(__n), salvo.SalvoBytes(__d)) }
        for (e in salvo.platform.core.list.each(seeds)) {
            dial(__dep_Transport, dialed, name, e)
        }
    }
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

class __Actor_GossipNodeGroup(private val handler: GossipNodeGroup) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_NodeGroup -> __dispatchNodeGroup(msg)
            is __Priv_GossipNodeGroup -> __dispatchPriv(msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    private fun __dispatchNodeGroup(m: __Msg_NodeGroup) {
        when (m) {
            is __Msg_NodeGroup.Members -> handler.members(m.out)
            is __Msg_NodeGroup.Subscribe -> handler.subscribe(m.w)
            is __Msg_NodeGroup.Leave -> handler.leave()
        }
    }

    private fun __dispatchPriv(m: __Priv_GossipNodeGroup) {
        when (m) {
            is __Priv_GossipNodeGroup.Init -> handler.init()
            is __Priv_GossipNodeGroup.Control -> handler.control(m.from, m.data)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
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

fun dial(transport: Transport, dialed: salvo.platform.core.set.MutSet<String>, group: String, e: NodeEndpoint) {
    if (eq__NodeEndpoint_NodeEndpoint(e, transport.localEndpoint()) || containsPlatform(dialed, toStr__NodeEndpoint(e))) {
        return
    }
    addPlatform__core_set(dialed, toStr__NodeEndpoint(e))
    val sent = transport.deliver(e, helloFrame(transport, group))
    if (sent is Union2.U2<*, *>) {
        val _forgot = removePlatform__core_set(dialed, toStr__NodeEndpoint(e))
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

/** [protocol-hash] The canonical hash of `ActorGroup`. */
const val __PROTO_ActorGroup: String = "695f43128bdada4a"

interface ActorGroupWatcher {
    fun joined(member: Int)
    fun left(member: Int)
}

class __Stub_ActorGroupWatcher(private val addr: Int) : ActorGroupWatcher {
    override fun joined(member: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_ActorGroupWatcher.Joined(member), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
    }
    override fun left(member: Int) {
        salvo.SalvoSched.sendWire(addr, __Msg_ActorGroupWatcher.Left(member), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
    }
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

/** [protocol-hash] The canonical hash of `ActorGroupWatcher`. */
const val __PROTO_ActorGroupWatcher: String = "9fa424e5858bb3bd"

fun shareMembers(name: String, hash: String, node: NodeId, members: List<Int>) {
    sendControl(node, name, salvo.salvoEncode(Pair(hash, members), salvo.PairCodec(salvo.StrCodec, salvo.ListCodec(salvo.AddrCodec))))
}

fun pending(a: Int): Int {
    return pending__runtime_routing((a))
}

fun actorGroup__Addr(nodes: Int, protocol: () -> Protocol): Int {
    val proto = protocol()
    val name = proto.name
    return actorGroup__Str_Addr(name, nodes, protocol)
}

fun actorGroup__Str_Addr(name: String, nodes: Int, protocol: () -> Protocol): Int {
    val (group, watcher) = run { val __h = ActorGrouping(name, protocol()); val __a = salvo.SalvoSched.spawn(pool(1), __h.__mailboxCapacity, __Actor_ActorGrouping(__h), __Actor_ActorGrouping.__DECODE); salvo.SalvoSched.send(__a, __Priv_ActorGrouping.Init); Pair(__a, __a) }
    salvo.SalvoSched.sendWire(nodes, __Msg_NodeGroup.Subscribe(watcher), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
    return group
}

fun join(group: Int, member: Int) {
    salvo.SalvoSched.sendWire(group, __Msg_ActorGroup.Join(member), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
}

class ActorGrouping(private val name: String, private val proto: Protocol) : ActorGroup, NodeGroupWatcher {
    private var all: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    private var peers: salvo.platform.core.list.MutList<NodeId> = mutableListOf<NodeId>()
    private var watchers: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_ActorGrouping> = mutableMapOf()

    override fun join(member: Int) {
        if (!admit(all, member)) {
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
        if (!withdraw(all, member)) {
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
                addPlatform__core_list(gone, m)
            }
        }
        for (m in salvo.platform.core.list.each(gone)) {
            if (withdraw(all, m)) {
                for (w in salvo.platform.core.list.each(watchers)) {
                    salvo.SalvoSched.sendWire(w, __Msg_ActorGroupWatcher.Left(m), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
                }
            }
        }
        if (sizePlatform(gone) > 0) {
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
        addPlatform__core_list(watchers, w)
    }

    fun control(from: NodeId, data: salvo.platform.core.bytes.Bytes) {
        val got = salvo.salvoDecode(data, salvo.PairCodec(salvo.StrCodec, salvo.ListCodec(salvo.AddrCodec)))
        if (got == null) {
            return
        }
        if (!((got.first) == (proto.hash))) {
            return
        }
        if (!containsNode(peers, from)) {
            addPlatform__core_list(peers, from)
            shareMembers(name, proto.hash, from, all)
        }
        var changed = false
        for (m in salvo.platform.core.list.each(got.second)) {
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

    fun init() {
        salvo.SalvoSched.watchControl(name, __addr!!) { __n, __d -> __Priv_ActorGrouping.Control(NodeId(__n), salvo.SalvoBytes(__d)) }
    }
}

sealed class __Cont_ActorGrouping {
    class Join() : __Cont_ActorGrouping()
    class Leave() : __Cont_ActorGrouping()
    class Joined() : __Cont_ActorGrouping()
    class Left(val n: Node) : __Cont_ActorGrouping()
    class Members() : __Cont_ActorGrouping()
    class Subscribe() : __Cont_ActorGrouping()
    class Control(val from: NodeId) : __Cont_ActorGrouping()
}

sealed class __Priv_ActorGrouping {
    object Init : __Priv_ActorGrouping()
    class Control(val from: NodeId, val data: salvo.platform.core.bytes.Bytes) : __Priv_ActorGrouping()
}

class __Actor_ActorGrouping(private val handler: ActorGrouping) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        when (msg) {
            is __Msg_ActorGroup -> __dispatchActorGroup(msg)
            is __Msg_NodeGroupWatcher -> __dispatchNodeGroupWatcher(msg)
            is __Priv_ActorGrouping -> __dispatchPriv(msg)
            else -> error("a message of one of this actor's protocols")
        }
    }

    private fun __dispatchActorGroup(m: __Msg_ActorGroup) {
        when (m) {
            is __Msg_ActorGroup.Join -> handler.join(m.member)
            is __Msg_ActorGroup.Leave -> handler.leave(m.member)
            is __Msg_ActorGroup.Members -> handler.members(m.out)
            is __Msg_ActorGroup.Subscribe -> handler.subscribe(m.w)
            is __Msg_ActorGroup.Refresh -> handler.refresh()
        }
    }

    private fun __dispatchNodeGroupWatcher(m: __Msg_NodeGroupWatcher) {
        when (m) {
            is __Msg_NodeGroupWatcher.Joined -> handler.joined(m.n)
            is __Msg_NodeGroupWatcher.Left -> handler.left(m.n, m.why)
        }
    }

    private fun __dispatchPriv(m: __Priv_ActorGrouping) {
        when (m) {
            is __Priv_ActorGrouping.Init -> handler.init()
            is __Priv_ActorGrouping.Control -> handler.control(m.from, m.data)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
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
        if (eq(x, a)) {
            return false
        }
    }
    addPlatform__core_list(list, a)
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
    var i = 0
    for (x in salvo.platform.core.list.each(list)) {
        if (eq(x, a)) {
            mutIndex = i
        }
        i = i + 1
    }
    if (mutIndex == null) {
        return false
    }
    val _removed = removeAtPlatform(list, mutIndex)
    return true
}

fun nodeOf(a: Int): NodeId {
    return NodeId(id = identity((a)).node)
}

fun viewSet(group: Int, members: List<Int>) {
    val ixs = mutableListOf<Int>()
    for (m in salvo.platform.core.list.each(members)) {
        addPlatform__core_list(ixs, (m))
    }
    viewSet__runtime_routing((group), ixs)
}

fun viewMembers(group: Int): List<Int> {
    val out = mutableListOf<Int>()
    for (ix in salvo.platform.core.list.each(viewMembers__runtime_routing((group)))) {
        addPlatform__core_list(out, (ix))
    }
    return out
}

fun viewVersion(group: Int): Long {
    return viewVersion__runtime_routing((group))
}

fun viewRefresh(group: Int) {
    viewRefresh__runtime_routing((group))
}

fun viewWait(group: Int, seen: Long, nanos: Long) {
    viewWait__runtime_routing((group), seen, nanos)
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
    return RouteConfig()
}

data class RouteConfig(
    val firstWait: Duration = Duration(nanos = 1000000L),
    val maxWait: Duration = Duration(nanos = 5000000000L),
)

object __Codec_RouteConfig : salvo.WireCodec<RouteConfig> {
    override fun enc(v: RouteConfig, out: salvo.WireOut) {
        __Codec_Duration.enc(v.firstWait, out)
        __Codec_Duration.enc(v.maxWait, out)
    }
    override fun dec(inp: salvo.WireIn): RouteConfig = RouteConfig(__Codec_Duration.dec(inp), __Codec_Duration.dec(inp))
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

fun routePick__Addr_RouteConfig_Long(route_selector: RouteSelector, group: Int, config: RouteConfig, seen: Long): RoutePick {
    return routeKeyed(route_selector, group, config, seen, null)
}

fun routePick__Addr_RouteConfig_Long_Long(route_selector: RouteSelector, group: Int, config: RouteConfig, seen: Long, key: Long): RoutePick {
    return routeKeyed(route_selector, group, config, seen, key)
}

fun routeKeyed(route_selector: RouteSelector, group: Int, config: RouteConfig, seen: Long, key: Long?): RoutePick {
    var wait = config.firstWait.nanos
    val cap = config.maxWait.nanos
    var last = seen
    while (true) {
        val version = viewVersion(group)
        val view = routeView(group)
        if (version != last) {
            route_selector.changed(view)
            last = version
        }
        val picked = route_selector.select(view, key)
        if (!(picked == null)) {
            return RoutePick(to = picked, version = last)
        }
        viewWait(group, version, wait)
        wait = wait * 2
        if (wait > cap) {
            wait = cap
        }
    }
    return routeKeyed(route_selector, group, config, seen, key)
}

fun routeView(group: Int): RouteView {
    val members: salvo.platform.core.list.MutList<RouteMember> = mutableListOf<RouteMember>()
    for (m in salvo.platform.core.list.each(viewMembers(group))) {
        addPlatform__core_list(members, RouteMember(addr = m, local = eq__NodeId_NodeId(nodeOf(m), thisNode())))
    }
    return RouteView(members = members.toMutableList())
}

class LeastLoaded(private val preferLocal: Boolean) : RouteSelector {

    override fun changed(view: RouteView) {
    }

    override fun select(view: RouteView, key: Long?): Int? {
        var best: RouteMember? = null
        var bestPending = 0
        for (a in salvo.platform.core.list.each(view.members)) {
            val load = pending(a.addr)
            if (best == null) {
                best = a
                bestPending = load
            } else {
                val b: RouteMember = best
                val take = when {
                    preferLocal && a.local && !b.local -> {
                        true
                    }
                    preferLocal && !a.local && b.local -> {
                        false
                    }
                    else -> {
                        load < bestPending
                    }
                }
                if (take) {
                    best = a
                    bestPending = load
                }
            }
        }
        val chosen: RouteMember = (best ?: return null)
        return chosen.addr
    }
}

class Sharded : RouteSelector {

    override fun changed(view: RouteView) {
    }

    override fun select(view: RouteView, key: Long?): Int? {
        val n = sizePlatform(view.members)
        if (n == 0) {
            return null
        }
        val k = (key ?: 0L)
        val magnitude = if (k < 0L) {
            0L - k
        } else {
            k
        }
        val slot = (magnitude % (n).toLong()).toInt()
        val picked = (getPlatform__core_list(view.members, slot) ?: return null)
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

class Elected(private val __dep_Leader: Leader) : RouteSelector {
    private var chosen: Int? = null

    override fun changed(view: RouteView) {
        chosen = null
        val l = (__dep_Leader.leader() ?: return)
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

class __Platform_HostTcpTransport(bind: NodeEndpoint) : salvo.net.__Platform_Transport(salvo.platform.net.HostTcpTransport(bind))

interface MemNet {
    fun attach(at: NodeEndpoint, sink: Int)
    fun detach(at: NodeEndpoint)
    fun route(from: NodeEndpoint, to: NodeEndpoint, out: salvo.SalvoReply)
    fun partition(a: NodeEndpoint, b: NodeEndpoint)
    fun heal(a: NodeEndpoint, b: NodeEndpoint)
    fun kill(node: NodeEndpoint)
    fun delivered(out: salvo.SalvoReply)
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

/** [protocol-hash] The canonical hash of `MemNet`. */
const val __PROTO_MemNet: String = "2815c14392023d5e"

class MemNetwork : MemNet {
    private var listeners: salvo.platform.core.map.MutMap<NodeEndpoint, Int> = salvo.SalvoHashMap<NodeEndpoint, Int>(::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint).also { __m -> __m.putAll(listOf()) }
    private var cuts: salvo.platform.core.set.MutSet<String> = linkedSetOf<String>().also { __s -> __s.addAll(listOf()) }
    private var dead: salvo.platform.core.set.MutSet<NodeEndpoint> = salvo.SalvoHashSet<NodeEndpoint>(::hash__NodeEndpoint, ::eq__NodeEndpoint_NodeEndpoint).also { __s -> __s.addAll(listOf()) }
    private var count: Int = 0
    val __mailboxCapacity: Int = 64
    var __addr: Int? = null
    val __parked: MutableMap<Long, __Cont_MemNetwork> = mutableMapOf()

    override fun attach(at: NodeEndpoint, sink: Int) {
        removePlatform__core_set(dead, at)
        putPlatform(listeners, at, sink)
    }

    override fun detach(at: NodeEndpoint) {
        removePlatform__core_map(listeners, at)
    }

    override fun route(from: NodeEndpoint, to: NodeEndpoint, out: salvo.SalvoReply) {
        if (containsPlatform(dead, to) || containsPlatform(cuts, cutKey(from, to))) {
            salvo.SalvoSched.replyWire(out, null, salvo.OptCodec(salvo.AddrCodec))
            return
        }
        val sink = getPlatform__core_map(listeners, to)
        if (sink == null) {
            salvo.SalvoSched.replyWire(out, null, salvo.OptCodec(salvo.AddrCodec))
            return
        }
        count = count + 1
        salvo.SalvoSched.replyWire(out, sink, salvo.OptCodec(salvo.AddrCodec))
    }

    override fun partition(a: NodeEndpoint, b: NodeEndpoint) {
        addPlatform__core_set(cuts, cutKey(a, b))
        addPlatform__core_set(cuts, cutKey(b, a))
    }

    override fun heal(a: NodeEndpoint, b: NodeEndpoint) {
        removePlatform__core_set(cuts, cutKey(a, b))
        removePlatform__core_set(cuts, cutKey(b, a))
    }

    override fun kill(node: NodeEndpoint) {
        removePlatform__core_map(listeners, node)
        addPlatform__core_set(dead, node)
    }

    override fun delivered(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, count, salvo.IntCodec)
    }
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

class __Actor_MemNetwork(private val handler: MemNetwork) : salvo.SalvoActor {
    override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {
        handler.__addr = ctx.addr
        __dispatch(msg as __Msg_MemNet)
    }

    private fun __dispatch(m: __Msg_MemNet) {
        when (m) {
            is __Msg_MemNet.Attach -> handler.attach(m.at, m.sink)
            is __Msg_MemNet.Detach -> handler.detach(m.at)
            is __Msg_MemNet.Route -> handler.route(m.from, m.to, m.out)
            is __Msg_MemNet.Partition -> handler.partition(m.a, m.b)
            is __Msg_MemNet.Heal -> handler.heal(m.a, m.b)
            is __Msg_MemNet.Kill -> handler.kill(m.node)
            is __Msg_MemNet.Delivered -> handler.delivered(m.out)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
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
        return Union2.U1<Unit, Union2<Unreachable, WireFailed>>(ok(Unit))
    }

    override fun unlisten(at: NodeEndpoint) {
        salvo.SalvoSched.sendWire(net, __Msg_MemNet.Detach(at), __PROTO_MemNet, __Codec___Msg_MemNet)
    }

    override fun deliver(to: NodeEndpoint, frame: salvo.platform.core.bytes.Bytes): Union2<Unit, Union2<Unreachable, WireFailed>> {
        val sink = run {
            val (out, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.OptCodec(salvo.AddrCodec)) })
            salvo.SalvoSched.sendWire(net, __Msg_MemNet.Route(me, to, out), __PROTO_MemNet, __Codec___Msg_MemNet)
            salvo.SalvoSched.awaitReply(__wid) as Int?
        }
        if (sink == null) {
            return Union2.U2<Unit, Union2<Unreachable, WireFailed>>(Union2.U1<Unreachable, WireFailed>(err(Unreachable(to = to))))
        }
        salvo.SalvoSched.sendWire(sink, __Msg_Inbound.ReceiveFrame(me, frame), __PROTO_Inbound, __Codec___Msg_Inbound)
        return Union2.U1<Unit, Union2<Unreachable, WireFailed>>(ok(Unit))
    }

    override fun localEndpoint(): NodeEndpoint {
        return me
    }
}

fun cutKey(a: NodeEndpoint, b: NodeEndpoint): String {
    return "${toStr__NodeEndpoint(a)}>${toStr__NodeEndpoint(b)}"
}

fun cmp(a: NodeEndpoint, b: NodeEndpoint): Int {
    val c__c1 = salvo.__salvoCompare(a.host, b.host)
    if (c__c1 != 0) {
        return c__c1
    }
    val c__c2 = (a.port).compareTo(b.port)
    if (c__c2 != 0) {
        return c__c2
    }
    return 0
}

fun hash__NodeEndpoint(value: NodeEndpoint): Long {
    var h = 17L
    h = mixHash(h, (value.host).hashCode().toLong())
    h = mixHash(h, (value.port).hashCode().toLong())
    return h
}

fun eq__NodeEndpoint_NodeEndpoint(a: NodeEndpoint, b: NodeEndpoint): Boolean {
    if (!((a.host) == (b.host))) {
        return false
    }
    if (!((a.port) == (b.port))) {
        return false
    }
    return true
}

fun hash__NodeId(value: NodeId): Long {
    var h = 17L
    h = mixHash(h, (value.id).hashCode().toLong())
    return h
}

fun eq__NodeId_NodeId(a: NodeId, b: NodeId): Boolean {
    if (!((a.id) == (b.id))) {
        return false
    }
    return true
}

fun hash__Node(value: Node): Long {
    var h = 17L
    h = mixHash(h, hash__NodeId(value.id))
    h = mixHash(h, hash__NodeEndpoint(value.at))
    return h
}

fun eq__Node_Node(a: Node, b: Node): Boolean {
    if (!eq__NodeId_NodeId(a.id, b.id)) {
        return false
    }
    if (!eq__NodeEndpoint_NodeEndpoint(a.at, b.at)) {
        return false
    }
    return true
}
