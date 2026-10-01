package salvo.net

import salvo.*
import salvo.core.actor.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.time.*

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

fun toStr__2(e: NodeEndpoint): String {
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

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun toStr__3(e: Union2<Unreachable, WireFailed>): String {
    when (e) {
        is Union2.U1<*, *> -> {
            return "unreachable: ${toStr__2((e.value as Unreachable).to)}"
        }
        is Union2.U2<*, *> -> {
            return "wire failed to ${toStr__2((e.value as WireFailed).to)}: ${(e.value as WireFailed).reason}"
        }
    }
}

interface Inbound {
    fun receiveFrame(from: NodeEndpoint, frame: salvo.SalvoBytes)
}

class __Stub_Inbound(private val addr: Int) : Inbound {
    override fun receiveFrame(from: NodeEndpoint, frame: salvo.SalvoBytes) {
        salvo.SalvoSched.sendWire(addr, __Msg_Inbound.ReceiveFrame(from, frame), __PROTO_Inbound, __Codec___Msg_Inbound)
    }
}

class __Mon_Inbound(private val inner: Inbound) : Inbound {
    override fun receiveFrame(from: NodeEndpoint, frame: salvo.SalvoBytes) =
        synchronized(inner) { inner.receiveFrame(from, frame) }
}

sealed class __Msg_Inbound {
    class ReceiveFrame(val from: NodeEndpoint, val frame: salvo.SalvoBytes) : __Msg_Inbound()
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
    fun deliver(to: NodeEndpoint, frame: salvo.SalvoBytes): Union2<Unit, Union2<Unreachable, WireFailed>>
    fun localEndpoint(): NodeEndpoint
}

class __Mon_Transport(private val inner: Transport) : Transport {
    override fun listen(at: NodeEndpoint, sink: Int): Union2<Unit, Union2<Unreachable, WireFailed>> =
        synchronized(inner) { inner.listen(at, sink) }
    override fun unlisten(at: NodeEndpoint) =
        synchronized(inner) { inner.unlisten(at) }
    override fun deliver(to: NodeEndpoint, frame: salvo.SalvoBytes): Union2<Unit, Union2<Unreachable, WireFailed>> =
        synchronized(inner) { inner.deliver(to, frame) }
    override fun localEndpoint(): NodeEndpoint =
        synchronized(inner) { inner.localEndpoint() }
}

// The interface a `platform handler` of `Transport` implements [platform-abi].
interface TransportPlatform {
    fun listen(at: NodeEndpoint, sink: Int): Union2<Unit, Union2<Unreachable, WireFailed>>
    fun unlisten(at: NodeEndpoint)
    fun deliver(to: NodeEndpoint, frame: salvo.SalvoBytes): Union2<Unit, Union2<Unreachable, WireFailed>>
    fun localEndpoint(): NodeEndpoint
}

open class __Platform_Transport(private val impl: TransportPlatform) : Transport {
    override fun listen(at: NodeEndpoint, sink: Int): Union2<Unit, Union2<Unreachable, WireFailed>> = impl.listen(at, sink)
    override fun unlisten(at: NodeEndpoint) = impl.unlisten(at)
    override fun deliver(to: NodeEndpoint, frame: salvo.SalvoBytes): Union2<Unit, Union2<Unreachable, WireFailed>> = impl.deliver(to, frame)
    override fun localEndpoint(): NodeEndpoint = impl.localEndpoint()
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

interface Outbound {
    fun sendFrame(to: NodeEndpoint, frame: salvo.SalvoBytes)
}

class __Stub_Outbound(private val addr: Int) : Outbound {
    override fun sendFrame(to: NodeEndpoint, frame: salvo.SalvoBytes) {
        salvo.SalvoSched.sendWire(addr, __Msg_Outbound.SendFrame(to, frame), __PROTO_Outbound, __Codec___Msg_Outbound)
    }
}

class __Mon_Outbound(private val inner: Outbound) : Outbound {
    override fun sendFrame(to: NodeEndpoint, frame: salvo.SalvoBytes) =
        synchronized(inner) { inner.sendFrame(to, frame) }
}

sealed class __Msg_Outbound {
    class SendFrame(val to: NodeEndpoint, val frame: salvo.SalvoBytes) : __Msg_Outbound()
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
    internal val __mailboxCapacity: Int = 256
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Sending> = mutableMapOf()

    override fun sendFrame(to: NodeEndpoint, frame: salvo.SalvoBytes) {
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
            is __Cont_Sending.SendFrame -> handler.sendFrame(c.to, value as salvo.SalvoBytes)
        }
    }

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
    internal val __mailboxCapacity: Int = 256
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_Receiving> = mutableMapOf()

    override fun receiveFrame(from: NodeEndpoint, frame: salvo.SalvoBytes) {
        val _delivered = salvo.SalvoSched.deliverFrame((frame).toByteArray())
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
            is __Cont_Receiving.ReceiveFrame -> handler.receiveFrame(c.from, value as salvo.SalvoBytes)
        }
    }

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

fun connect(transport: Transport, me: NodeEndpoint): Boolean {
    return connect__2(transport, me, salvo.SalvoSched.pool(1))
}

fun connect__2(transport: Transport, me: NodeEndpoint, on: Int): Boolean {
    if (salvo.SalvoSched.connected()) {
        return false
    }
    val sending = run { val __h = Sending(transport); val __a = salvo.SalvoSched.spawn(on, __h.__mailboxCapacity, __Actor_Sending(__h), __Actor_Sending.__DECODE); __a }
    val receiving = run { val __h = Receiving(); val __a = salvo.SalvoSched.spawn(on, __h.__mailboxCapacity, __Actor_Receiving(__h), __Actor_Receiving.__DECODE); __a }
    run { val __out = sending; salvo.SalvoSched.setWire { __ep, __frame -> val __to = salvo.salvoDecode(salvo.SalvoBytes(__ep), __Codec_NodeEndpoint); if (__to != null) salvo.SalvoSched.sendWire(__out, __Msg_Outbound.SendFrame(__to, salvo.SalvoBytes(__frame)), __PROTO_Outbound, __Codec___Msg_Outbound) } }
    val _listening = transport.listen(me, receiving)
    salvo.SalvoSched.addRoute((NodeId(salvo.SalvoSched.hereNode())).id, salvo.salvoEncode(me, __Codec_NodeEndpoint).toByteArray())
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

class __Mon_NodeGroup(private val inner: NodeGroup) : NodeGroup {
    override fun members(out: salvo.SalvoReply) =
        synchronized(inner) { inner.members(out) }
    override fun subscribe(w: Int) =
        synchronized(inner) { inner.subscribe(w) }
    override fun leave() =
        synchronized(inner) { inner.leave() }
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

class __Mon_NodeGroupWatcher(private val inner: NodeGroupWatcher) : NodeGroupWatcher {
    override fun joined(n: Node) =
        synchronized(inner) { inner.joined(n) }
    override fun left(n: Node, why: String) =
        synchronized(inner) { inner.left(n, why) }
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

class StaticNodeGroup(private val name: String, private val all: List<NodeEndpoint>, private val __dep_Transport: Transport) : NodeGroup {
    private var known: MutableMap<NodeId, Node> = salvo.SalvoHashMap<NodeId, Node>(::hash__2, ::eq__2).also { __m -> __m.putAll(listOf()) }
    private var watchers: MutableList<Int> = mutableListOf<Int>()
    internal val __mailboxCapacity: Int = 64
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_StaticNodeGroup> = mutableMapOf()

    override fun members(out: salvo.SalvoReply) {
        val allKnown: MutableList<Node> = mutableListOf<Node>()
        for (id in known.keys.toMutableList()) {
            val n = known[id]
            if (!(n == null)) {
                allKnown.add(n)
            }
        }
        salvo.SalvoSched.replyWire(out, allKnown, salvo.ListCodec(__Codec_Node))
    }

    override fun subscribe(w: Int) {
        watchers.add(w)
    }

    override fun leave() {
        salvo.SalvoSched.leaveGroup()
    }

    fun hello(node: NodeId, at: NodeEndpoint, protocols: List<Pair<String, String>>) {
        if (known.containsKey(node)) {
            return
        }
        val n = Node(id = node, at = at)
        known.put(node, n)
        for (w in watchers) {
            salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
        }
    }

    fun gone(node: NodeId) {
        val n = known.remove(node)
        if (n == null) {
            return
        }
        for (w in watchers) {
            salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Left(n, "left"), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
        }
    }

    fun introduced(peers: List<NodeEndpoint>) {
    }

    fun init() {
        val me = __dep_Transport.localEndpoint()
        val _connected = connect(__dep_Transport, me)
        salvo.SalvoSched.setGroup(name, salvo.salvoEncode(me, __Codec_NodeEndpoint).toByteArray())
        salvo.SalvoSched.watchPeers(__addr!!, { __n, __ep, __t -> __Priv_StaticNodeGroup.Hello(NodeId(__n), salvo.salvoDecode(salvo.SalvoBytes(__ep), __Codec_NodeEndpoint)!!, __t) }, { __n -> __Priv_StaticNodeGroup.Gone(NodeId(__n)) }, { __ps -> __Priv_StaticNodeGroup.Introduced(__ps.mapNotNull { salvo.salvoDecode(salvo.SalvoBytes(it), __Codec_NodeEndpoint) }) })
        for (e in all) {
            if (!eq(e, me)) {
                val _sent = __dep_Transport.deliver(e, salvo.SalvoBytes(salvo.SalvoSched.helloFrame()))
            }
        }
    }
}

sealed class __Cont_StaticNodeGroup {
    class Members() : __Cont_StaticNodeGroup()
    class Subscribe() : __Cont_StaticNodeGroup()
    class Hello(val node: NodeId, val at: NodeEndpoint) : __Cont_StaticNodeGroup()
    class Gone() : __Cont_StaticNodeGroup()
    class Introduced() : __Cont_StaticNodeGroup()
}

sealed class __Priv_StaticNodeGroup {
    object Init : __Priv_StaticNodeGroup()
    class Hello(val node: NodeId, val at: NodeEndpoint, val protocols: List<Pair<String, String>>) : __Priv_StaticNodeGroup()
    class Gone(val node: NodeId) : __Priv_StaticNodeGroup()
    class Introduced(val peers: List<NodeEndpoint>) : __Priv_StaticNodeGroup()
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
            is __Priv_StaticNodeGroup.Hello -> handler.hello(m.node, m.at, m.protocols)
            is __Priv_StaticNodeGroup.Gone -> handler.gone(m.node)
            is __Priv_StaticNodeGroup.Introduced -> handler.introduced(m.peers)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_StaticNodeGroup.Members -> handler.members(value as salvo.SalvoReply)
            is __Cont_StaticNodeGroup.Subscribe -> handler.subscribe(value as Int)
            is __Cont_StaticNodeGroup.Hello -> handler.hello(c.node, c.at, value as List<Pair<String, String>>)
            is __Cont_StaticNodeGroup.Gone -> handler.gone(value as NodeId)
            is __Cont_StaticNodeGroup.Introduced -> handler.introduced(value as List<NodeEndpoint>)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_StaticNodeGroup.Members -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_StaticNodeGroup.Subscribe -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_StaticNodeGroup.Hello -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.PairCodec(salvo.StrCodec, salvo.StrCodec))) })(payload)
            is __Cont_StaticNodeGroup.Gone -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_NodeId) })(payload)
            is __Cont_StaticNodeGroup.Introduced -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(__Codec_NodeEndpoint)) })(payload)
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

class GossipNodeGroup(private val name: String, private val seeds: List<NodeEndpoint>, private val __dep_Transport: Transport) : NodeGroup {
    private var known: MutableMap<NodeId, Node> = salvo.SalvoHashMap<NodeId, Node>(::hash__2, ::eq__2).also { __m -> __m.putAll(listOf()) }
    private var dialed: MutableSet<String> = linkedSetOf<String>().also { __s -> __s.addAll(listOf()) }
    private var watchers: MutableList<Int> = mutableListOf<Int>()
    internal val __mailboxCapacity: Int = 64
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_GossipNodeGroup> = mutableMapOf()

    override fun members(out: salvo.SalvoReply) {
        val allKnown: MutableList<Node> = mutableListOf<Node>()
        for (id in known.keys.toMutableList()) {
            val n = known[id]
            if (!(n == null)) {
                allKnown.add(n)
            }
        }
        salvo.SalvoSched.replyWire(out, allKnown, salvo.ListCodec(__Codec_Node))
    }

    override fun subscribe(w: Int) {
        watchers.add(w)
    }

    override fun leave() {
        salvo.SalvoSched.leaveGroup()
    }

    fun hello(node: NodeId, at: NodeEndpoint, protocols: List<Pair<String, String>>) {
        if (known.containsKey(node)) {
            return
        }
        val others: MutableList<NodeEndpoint> = mutableListOf<NodeEndpoint>()
        for (id in known.keys.toMutableList()) {
            val n = known[id]
            if (!(n == null)) {
                others.add(n.at)
            }
            salvo.SalvoSched.introduce((id).id, (listOf<NodeEndpoint>(at)).map { salvo.salvoEncode(it, __Codec_NodeEndpoint).toByteArray() })
        }
        salvo.SalvoSched.introduce((node).id, (others).map { salvo.salvoEncode(it, __Codec_NodeEndpoint).toByteArray() })
        dialed.add(toStr__2(at))
        val n = Node(id = node, at = at)
        known.put(node, n)
        for (w in watchers) {
            salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Joined(n), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
        }
    }

    fun gone(node: NodeId) {
        val n = known.remove(node)
        if (n == null) {
            return
        }
        for (w in watchers) {
            salvo.SalvoSched.sendWire(w, __Msg_NodeGroupWatcher.Left(n, "left"), __PROTO_NodeGroupWatcher, __Codec___Msg_NodeGroupWatcher)
        }
    }

    fun introduced(peers: List<NodeEndpoint>) {
        for (e in peers) {
            dial(__dep_Transport, dialed, e)
        }
    }

    fun init() {
        val me = __dep_Transport.localEndpoint()
        val _connected = connect(__dep_Transport, me)
        salvo.SalvoSched.setGroup(name, salvo.salvoEncode(me, __Codec_NodeEndpoint).toByteArray())
        salvo.SalvoSched.watchPeers(__addr!!, { __n, __ep, __t -> __Priv_GossipNodeGroup.Hello(NodeId(__n), salvo.salvoDecode(salvo.SalvoBytes(__ep), __Codec_NodeEndpoint)!!, __t) }, { __n -> __Priv_GossipNodeGroup.Gone(NodeId(__n)) }, { __ps -> __Priv_GossipNodeGroup.Introduced(__ps.mapNotNull { salvo.salvoDecode(salvo.SalvoBytes(it), __Codec_NodeEndpoint) }) })
        for (e in seeds) {
            dial(__dep_Transport, dialed, e)
        }
    }
}

sealed class __Cont_GossipNodeGroup {
    class Members() : __Cont_GossipNodeGroup()
    class Subscribe() : __Cont_GossipNodeGroup()
    class Hello(val node: NodeId, val at: NodeEndpoint) : __Cont_GossipNodeGroup()
    class Gone() : __Cont_GossipNodeGroup()
    class Introduced() : __Cont_GossipNodeGroup()
}

sealed class __Priv_GossipNodeGroup {
    object Init : __Priv_GossipNodeGroup()
    class Hello(val node: NodeId, val at: NodeEndpoint, val protocols: List<Pair<String, String>>) : __Priv_GossipNodeGroup()
    class Gone(val node: NodeId) : __Priv_GossipNodeGroup()
    class Introduced(val peers: List<NodeEndpoint>) : __Priv_GossipNodeGroup()
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
            is __Priv_GossipNodeGroup.Hello -> handler.hello(m.node, m.at, m.protocols)
            is __Priv_GossipNodeGroup.Gone -> handler.gone(m.node)
            is __Priv_GossipNodeGroup.Introduced -> handler.introduced(m.peers)
        }
    }

    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {
        handler.__addr = ctx.addr
        // A reply whose continuation is gone: nothing to run.
        val c = handler.__parked.remove(slot) ?: return
        when (c) {
            is __Cont_GossipNodeGroup.Members -> handler.members(value as salvo.SalvoReply)
            is __Cont_GossipNodeGroup.Subscribe -> handler.subscribe(value as Int)
            is __Cont_GossipNodeGroup.Hello -> handler.hello(c.node, c.at, value as List<Pair<String, String>>)
            is __Cont_GossipNodeGroup.Gone -> handler.gone(value as NodeId)
            is __Cont_GossipNodeGroup.Introduced -> handler.introduced(value as List<NodeEndpoint>)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_GossipNodeGroup.Members -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_GossipNodeGroup.Subscribe -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_GossipNodeGroup.Hello -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.PairCodec(salvo.StrCodec, salvo.StrCodec))) })(payload)
            is __Cont_GossipNodeGroup.Gone -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_NodeId) })(payload)
            is __Cont_GossipNodeGroup.Introduced -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(__Codec_NodeEndpoint)) })(payload)
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

fun dial(transport: Transport, dialed: MutableSet<String>, e: NodeEndpoint) {
    if (eq(e, transport.localEndpoint()) || dialed.contains(toStr__2(e))) {
        return
    }
    dialed.add(toStr__2(e))
    val _sent = transport.deliver(e, salvo.SalvoBytes(salvo.SalvoSched.helloFrame()))
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
}

class __Mon_ActorGroup(private val inner: ActorGroup) : ActorGroup {
    override fun join(member: Int) =
        synchronized(inner) { inner.join(member) }
    override fun leave(member: Int) =
        synchronized(inner) { inner.leave(member) }
    override fun members(out: salvo.SalvoReply) =
        synchronized(inner) { inner.members(out) }
    override fun subscribe(w: Int) =
        synchronized(inner) { inner.subscribe(w) }
}

sealed class __Msg_ActorGroup {
    class Join(val member: Int) : __Msg_ActorGroup()
    class Leave(val member: Int) : __Msg_ActorGroup()
    class Members(val out: salvo.SalvoReply) : __Msg_ActorGroup()
    class Subscribe(val w: Int) : __Msg_ActorGroup()
}

object __Codec___Msg_ActorGroup : salvo.WireCodec<__Msg_ActorGroup> {
    override fun enc(v: __Msg_ActorGroup, out: salvo.WireOut) {
        when (v) {
            is __Msg_ActorGroup.Join -> { out.u8(0); salvo.AddrCodec.enc(v.member, out) }
            is __Msg_ActorGroup.Leave -> { out.u8(1); salvo.AddrCodec.enc(v.member, out) }
            is __Msg_ActorGroup.Members -> { out.u8(2); salvo.ReplyCodec.enc(v.out, out) }
            is __Msg_ActorGroup.Subscribe -> { out.u8(3); salvo.AddrCodec.enc(v.w, out) }
        }
    }
    override fun dec(inp: salvo.WireIn): __Msg_ActorGroup = when (inp.u8()) {
            0 -> __Msg_ActorGroup.Join(salvo.AddrCodec.dec(inp))
            1 -> __Msg_ActorGroup.Leave(salvo.AddrCodec.dec(inp))
            2 -> __Msg_ActorGroup.Members(salvo.ReplyCodec.dec(inp))
            3 -> __Msg_ActorGroup.Subscribe(salvo.AddrCodec.dec(inp))
        else -> throw salvo.WireError()
    }
}

/** [protocol-hash] The canonical hash of `ActorGroup`. */
const val __PROTO_ActorGroup: String = "d8ddd42c92bfcc7f"

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

class __Mon_ActorGroupWatcher(private val inner: ActorGroupWatcher) : ActorGroupWatcher {
    override fun joined(member: Int) =
        synchronized(inner) { inner.joined(member) }
    override fun left(member: Int) =
        synchronized(inner) { inner.left(member) }
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

fun actorGroup(nodes: Int, protocol: () -> Protocol): Int {
    val proto = protocol()
    val name = proto.name
    return actorGroup__2(name, nodes, protocol)
}

fun actorGroup__2(name: String, nodes: Int, protocol: () -> Protocol): Int {
    val (group, watcher) = run { val __h = ActorGrouping(name, protocol()); val __a = salvo.SalvoSched.spawn(salvo.SalvoSched.pool(1), __h.__mailboxCapacity, __Actor_ActorGrouping(__h), __Actor_ActorGrouping.__DECODE); salvo.SalvoSched.send(__a, __Priv_ActorGrouping.Init); Pair(__a, __a) }
    salvo.SalvoSched.sendWire(nodes, __Msg_NodeGroup.Subscribe(watcher), __PROTO_NodeGroup, __Codec___Msg_NodeGroup)
    return group
}

fun join(group: Int, member: Int) {
    salvo.SalvoSched.sendWire(group, __Msg_ActorGroup.Join(member), __PROTO_ActorGroup, __Codec___Msg_ActorGroup)
}

class ActorGrouping(private val name: String, private val proto: Protocol) : ActorGroup, NodeGroupWatcher {
    private var all: MutableList<Int> = mutableListOf<Int>()
    private var peers: MutableList<NodeId> = mutableListOf<NodeId>()
    private var watchers: MutableList<Int> = mutableListOf<Int>()
    internal val __mailboxCapacity: Int = 64
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_ActorGrouping> = mutableMapOf()

    override fun join(member: Int) {
        if (!admit(all, member)) {
            return
        }
        mirror(__addr!!, all)
        for (w in watchers) {
            salvo.SalvoSched.sendWire(w, __Msg_ActorGroupWatcher.Joined(member), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
        }
        for (p in peers) {
            salvo.SalvoSched.shareMembers(name, (p).id, listOf<Int>(member))
        }
    }

    override fun leave(member: Int) {
        if (!withdraw(all, member)) {
            return
        }
        mirror(__addr!!, all)
        for (w in watchers) {
            salvo.SalvoSched.sendWire(w, __Msg_ActorGroupWatcher.Left(member), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
        }
    }

    override fun joined(n: Node) {
    }

    override fun left(n: Node, why: String) {
        val gone: MutableList<Int> = mutableListOf<Int>()
        for (m in all) {
            if (eq__2(NodeId(salvo.SalvoSched.addrIdentity(m).node), n.id)) {
                gone.add(m)
            }
        }
        for (m in gone) {
            if (withdraw(all, m)) {
                for (w in watchers) {
                    salvo.SalvoSched.sendWire(w, __Msg_ActorGroupWatcher.Left(m), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
                }
            }
        }
        if (gone.size > 0) {
            mirror(__addr!!, all)
        }
    }

    override fun members(out: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(out, all.toMutableList(), salvo.ListCodec(salvo.AddrCodec))
    }

    override fun subscribe(w: Int) {
        watchers.add(w)
    }

    fun peer(node: NodeId) {
        val theirs = salvo.SalvoSched.peerProtocol((node).id, proto.name)
        if ((theirs == null) || !((theirs) == (proto.hash))) {
            return
        }
        if (containsNode(peers, node)) {
            return
        }
        peers.add(node)
        salvo.SalvoSched.shareMembers(name, (node).id, all.toMutableList())
    }

    fun merged(from: NodeId, found: List<Int>) {
        if (!containsNode(peers, from)) {
            val theirs = salvo.SalvoSched.peerProtocol((from).id, proto.name)
            if ((theirs == null) || !((theirs) == (proto.hash))) {
                return
            }
            peers.add(from)
        }
        var changed = false
        for (m in found) {
            if (admit(all, m)) {
                changed = true
                for (w in watchers) {
                    salvo.SalvoSched.sendWire(w, __Msg_ActorGroupWatcher.Joined(m), __PROTO_ActorGroupWatcher, __Codec___Msg_ActorGroupWatcher)
                }
            }
        }
        if (changed) {
            mirror(__addr!!, all)
        }
    }

    fun init() {
        run { val __me = __addr!!; salvo.SalvoSched.publish(name, __me, __me, { __n -> __Priv_ActorGrouping.Peer(NodeId(__n)) }, { __n, __ids -> __Priv_ActorGrouping.Merged(NodeId(__n), __ids.map { salvo.SalvoSched.importAddr(it) }) }) }
    }
}

sealed class __Cont_ActorGrouping {
    class Join() : __Cont_ActorGrouping()
    class Leave() : __Cont_ActorGrouping()
    class Joined() : __Cont_ActorGrouping()
    class Left(val n: Node) : __Cont_ActorGrouping()
    class Members() : __Cont_ActorGrouping()
    class Subscribe() : __Cont_ActorGrouping()
    class Peer() : __Cont_ActorGrouping()
    class Merged(val from: NodeId) : __Cont_ActorGrouping()
}

sealed class __Priv_ActorGrouping {
    object Init : __Priv_ActorGrouping()
    class Peer(val node: NodeId) : __Priv_ActorGrouping()
    class Merged(val from: NodeId, val found: List<Int>) : __Priv_ActorGrouping()
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
            is __Priv_ActorGrouping.Peer -> handler.peer(m.node)
            is __Priv_ActorGrouping.Merged -> handler.merged(m.from, m.found)
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
            is __Cont_ActorGrouping.Peer -> handler.peer(value as NodeId)
            is __Cont_ActorGrouping.Merged -> handler.merged(c.from, value as List<Int>)
        }
    }

    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {
        val c = handler.__parked[slot] ?: return Pair(false, null)
        return when (c) {
            is __Cont_ActorGrouping.Join -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_ActorGrouping.Leave -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_ActorGrouping.Members -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ReplyCodec) })(payload)
            is __Cont_ActorGrouping.Subscribe -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.AddrCodec) })(payload)
            is __Cont_ActorGrouping.Joined -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_Node) })(payload)
            is __Cont_ActorGrouping.Left -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.StrCodec) })(payload)
            is __Cont_ActorGrouping.Peer -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), __Codec_NodeId) })(payload)
            is __Cont_ActorGrouping.Merged -> ({ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.ListCodec(salvo.AddrCodec)) })(payload)
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
    salvo.SalvoSched.viewSet(group, members)
}

fun admit(list: MutableList<Int>, a: Int): Boolean {
    for (x in list) {
        if ((salvo.SalvoSched.addrIdentity(x) == salvo.SalvoSched.addrIdentity(a))) {
            return false
        }
    }
    list.add(a)
    return true
}

fun containsNode(list: List<NodeId>, n: NodeId): Boolean {
    for (x in list) {
        if (eq__2(x, n)) {
            return true
        }
    }
    return false
}

fun withdraw(list: MutableList<Int>, a: Int): Boolean {
    var mutIndex: Int? = null
    var i = 0
    for (x in list) {
        if ((salvo.SalvoSched.addrIdentity(x) == salvo.SalvoSched.addrIdentity(a))) {
            mutIndex = i
        }
        i = i + 1
    }
    if (mutIndex == null) {
        return false
    }
    val _removed = (list).let { __l -> (mutIndex).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
    return true
}

data class ActorView(
    val addr: Int,
    val pending: Int,
    val local: Boolean,
)

object __Codec_ActorView : salvo.WireCodec<ActorView> {
    override fun enc(v: ActorView, out: salvo.WireOut) {
        salvo.AddrCodec.enc(v.addr, out)
        salvo.IntCodec.enc(v.pending, out)
        salvo.BoolCodec.enc(v.local, out)
    }
    override fun dec(inp: salvo.WireIn): ActorView = ActorView(salvo.AddrCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.BoolCodec.dec(inp))
}

data class ActorGroupView(
    val actors: List<ActorView>,
    val key: Long? = null,
)

object __Codec_ActorGroupView : salvo.WireCodec<ActorGroupView> {
    override fun enc(v: ActorGroupView, out: salvo.WireOut) {
        salvo.ListCodec(__Codec_ActorView).enc(v.actors, out)
        salvo.OptCodec(salvo.LongCodec).enc(v.key, out)
    }
    override fun dec(inp: salvo.WireIn): ActorGroupView = ActorGroupView(salvo.ListCodec(__Codec_ActorView).dec(inp), salvo.OptCodec(salvo.LongCodec).dec(inp))
}

interface Pick {
    fun choose(view: ActorGroupView): Int?
}

class __Mon_Pick(private val inner: Pick) : Pick {
    override fun choose(view: ActorGroupView): Int? =
        synchronized(inner) { inner.choose(view) }
}

fun routeTo(pick: Pick, group: Int): Int {
    return routeKeyed(pick, group, null)
}

fun routeTo__2(pick: Pick, group: Int, key: Long): Int {
    return routeKeyed(pick, group, key)
}

fun routeKeyed(pick: Pick, group: Int, key: Long?): Int {
    while (true) {
        val members = salvo.SalvoSched.viewMembers(group)
        val actors: MutableList<ActorView> = mutableListOf<ActorView>()
        for (m in members) {
            actors.add(ActorView(addr = m, pending = salvo.SalvoSched.pending(m), local = eq__2(NodeId(salvo.SalvoSched.addrIdentity(m).node), NodeId(salvo.SalvoSched.hereNode()))))
        }
        val picked = pick.choose(ActorGroupView(actors = actors.toMutableList(), key = key))
        if (!(picked == null)) {
            return picked
        }
        salvo.SalvoSched.parkBriefly()
    }
    return routeKeyed(pick, group, key)
}

class LeastLoaded(private val preferLocal: Boolean) : Pick {

    override fun choose(view: ActorGroupView): Int? {
        var best: ActorView? = null
        for (a in view.actors) {
            if (best == null) {
                best = a
            } else {
                val b: ActorView = best
                val take = when {
                    preferLocal && a.local && !b.local -> {
                        true
                    }
                    preferLocal && !a.local && b.local -> {
                        false
                    }
                    else -> {
                        a.pending < b.pending
                    }
                }
                if (take) {
                    best = a
                }
            }
        }
        val chosen: ActorView = (best ?: return null)
        return chosen.addr
    }
}

class Sharded : Pick {

    override fun choose(view: ActorGroupView): Int? {
        val n = view.actors.size
        if (n == 0) {
            return null
        }
        val k = (view.key ?: 0L)
        val magnitude = if (k < 0L) {
            0L - k
        } else {
            k
        }
        val slot = (magnitude % (n).toLong()).toInt()
        val picked = (view.actors.getOrNull(slot) ?: return null)
        return picked.addr
    }
}

interface Leader {
    fun leader(): NodeId?
}

class __Mon_Leader(private val inner: Leader) : Leader {
    override fun leader(): NodeId? =
        synchronized(inner) { inner.leader() }
}

class StaticLeader(private val node: NodeId) : Leader {

    override fun leader(): NodeId? {
        return node
    }
}

class Elected(private val __dep_Leader: Leader) : Pick {

    override fun choose(view: ActorGroupView): Int? {
        val l = (__dep_Leader.leader() ?: return null)
        for (a in view.actors) {
            if (eq__2(NodeId(salvo.SalvoSched.addrIdentity(a.addr).node), l)) {
                return a.addr
            }
        }
        return null
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

class __Mon_MemNet(private val inner: MemNet) : MemNet {
    override fun attach(at: NodeEndpoint, sink: Int) =
        synchronized(inner) { inner.attach(at, sink) }
    override fun detach(at: NodeEndpoint) =
        synchronized(inner) { inner.detach(at) }
    override fun route(from: NodeEndpoint, to: NodeEndpoint, out: salvo.SalvoReply) =
        synchronized(inner) { inner.route(from, to, out) }
    override fun partition(a: NodeEndpoint, b: NodeEndpoint) =
        synchronized(inner) { inner.partition(a, b) }
    override fun heal(a: NodeEndpoint, b: NodeEndpoint) =
        synchronized(inner) { inner.heal(a, b) }
    override fun kill(node: NodeEndpoint) =
        synchronized(inner) { inner.kill(node) }
    override fun delivered(out: salvo.SalvoReply) =
        synchronized(inner) { inner.delivered(out) }
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
    private var listeners: MutableMap<NodeEndpoint, Int> = salvo.SalvoHashMap<NodeEndpoint, Int>(::hash, ::eq).also { __m -> __m.putAll(listOf()) }
    private var cuts: MutableSet<String> = linkedSetOf<String>().also { __s -> __s.addAll(listOf()) }
    private var dead: MutableSet<NodeEndpoint> = salvo.SalvoHashSet<NodeEndpoint>(::hash, ::eq).also { __s -> __s.addAll(listOf()) }
    private var count: Int = 0
    internal val __mailboxCapacity: Int = 64
    internal var __addr: Int? = null
    internal val __parked: MutableMap<Long, __Cont_MemNetwork> = mutableMapOf()

    override fun attach(at: NodeEndpoint, sink: Int) {
        dead.remove(at)
        listeners.put(at, sink)
    }

    override fun detach(at: NodeEndpoint) {
        listeners.remove(at)
    }

    override fun route(from: NodeEndpoint, to: NodeEndpoint, out: salvo.SalvoReply) {
        if (dead.contains(to) || cuts.contains(cutKey(from, to))) {
            salvo.SalvoSched.replyWire(out, null, salvo.OptCodec(salvo.AddrCodec))
            return
        }
        val sink = listeners[to]
        if (sink == null) {
            salvo.SalvoSched.replyWire(out, null, salvo.OptCodec(salvo.AddrCodec))
            return
        }
        count = count + 1
        salvo.SalvoSched.replyWire(out, sink, salvo.OptCodec(salvo.AddrCodec))
    }

    override fun partition(a: NodeEndpoint, b: NodeEndpoint) {
        cuts.add(cutKey(a, b))
        cuts.add(cutKey(b, a))
    }

    override fun heal(a: NodeEndpoint, b: NodeEndpoint) {
        cuts.remove(cutKey(a, b))
        cuts.remove(cutKey(b, a))
    }

    override fun kill(node: NodeEndpoint) {
        listeners.remove(node)
        dead.add(node)
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

    override fun deliver(to: NodeEndpoint, frame: salvo.SalvoBytes): Union2<Unit, Union2<Unreachable, WireFailed>> {
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
    return "${toStr__2(a)}>${toStr__2(b)}"
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

fun hash(value: NodeEndpoint): Long {
    var h = 17L
    h = ((h) * 31L + ((value.host).hashCode().toLong()))
    h = ((h) * 31L + ((value.port).hashCode().toLong()))
    return h
}

fun eq(a: NodeEndpoint, b: NodeEndpoint): Boolean {
    if (!((a.host) == (b.host))) {
        return false
    }
    if (!((a.port) == (b.port))) {
        return false
    }
    return true
}

fun hash__2(value: NodeId): Long {
    var h = 17L
    h = ((h) * 31L + ((value.id).hashCode().toLong()))
    return h
}

fun eq__2(a: NodeId, b: NodeId): Boolean {
    if (!((a.id) == (b.id))) {
        return false
    }
    return true
}

fun hash__3(value: Node): Long {
    var h = 17L
    h = ((h) * 31L + (hash__2(value.id)))
    h = ((h) * 31L + (hash(value.at)))
    return h
}

fun eq__3(a: Node, b: Node): Boolean {
    if (!eq__2(a.id, b.id)) {
        return false
    }
    if (!eq(a.at, b.at)) {
        return false
    }
    return true
}
