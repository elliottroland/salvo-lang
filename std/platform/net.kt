// Host implementation of the platform declarations of Salvo module `net`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: the
// Kotlin compiler checks it, against the interfaces the backend generates
// from the `platform handler` declarations.
package salvo.platform.net

import salvo.*
import salvo.net.*

import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.EOFException
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicReference

// `threadsafe platform handler HostTcpTransport` — THE CONTRACT YOU ARE SIGNING:
// this instance is shared across every thread of the program with NO
// lock around it. Every member below may run concurrently with every
// other, so any mutable state needs its own synchronization
// (`ConcurrentHashMap`, atomics, `synchronized` blocks of your own). If
// the host cannot promise that, delete `threadsafe` from the Salvo
// declaration: the compiler then serializes the instance for you
// [threadsafe-platform].
//
// Signed 2026-09-26: both tables are `ConcurrentHashMap`s, a listener's sink
// is an `AtomicReference`, and a connection is written inside `synchronized`
// on that connection alone — so two pools delivering to two peers never
// wait on each other.
//
// The wire is the Rust host's, byte for byte: one TCP connection per (this
// node, peer), opened lazily by the first `deliver`; it opens with a hello
// (the sender's endpoint as `host\nport`) and then carries frames, each a
// 4-byte big-endian length and that many bytes. A `listen` starts one
// acceptor thread per endpoint and one reader per connection; a reader turns
// each frame into a send on the registered `Inbound` addr through the
// generated forwarding stub.
class HostTcpTransport(private val bind: NodeEndpoint) : TransportPlatform {
    private class Peer(val socket: Socket) {
        val out = DataOutputStream(socket.getOutputStream().buffered())
    }

    private val peers = ConcurrentHashMap<NodeEndpoint, Peer>()
    private val listening = ConcurrentHashMap<NodeEndpoint, AtomicReference<Int?>>()

    private fun unreachable(to: NodeEndpoint): Union2<Unit, Union2<Unreachable, WireFailed>> =
        Union2.U2(Union2.U1(Unreachable(to)))

    private fun wireFailed(to: NodeEndpoint, reason: String): Union2<Unit, Union2<Unreachable, WireFailed>> =
        Union2.U2(Union2.U2(WireFailed(to, reason)))

    private fun writeFrame(out: DataOutputStream, bytes: ByteArray) {
        out.writeInt(bytes.size)
        out.write(bytes)
        out.flush()
    }

    /** One frame, or `null` at a clean close. */
    private fun readFrame(input: DataInputStream): ByteArray? {
        val n = try {
            input.readInt()
        } catch (e: EOFException) {
            return null
        }
        val buf = ByteArray(n)
        input.readFully(buf)
        return buf
    }

    private fun encodeHello(e: NodeEndpoint): ByteArray = "${e.host}\n${e.port}".toByteArray()

    private fun decodeHello(bytes: ByteArray): NodeEndpoint? {
        val text = String(bytes)
        val cut = text.indexOf('\n')
        if (cut < 0) return null
        val port = text.substring(cut + 1).toIntOrNull() ?: return null
        return NodeEndpoint(text.substring(0, cut), port)
    }

    /** One accepted connection: hello, then frames to the current sink. */
    private fun serve(socket: Socket, sink: AtomicReference<Int?>) {
        try {
            val input = DataInputStream(socket.getInputStream().buffered())
            val from = decodeHello(readFrame(input) ?: return) ?: return
            while (true) {
                val frame = readFrame(input) ?: return
                val addr = sink.get() ?: return
                __Stub_Inbound(addr).receiveFrame(from, salvo.SalvoBytes(frame))
            }
        } catch (e: Exception) {
            // The peer went away mid-frame: this reader is done.
        } finally {
            try { socket.close() } catch (e: Exception) {}
        }
    }

    override fun listen(at: NodeEndpoint, sink: Int): Union2<Unit, Union2<Unreachable, WireFailed>> {
        val existing = listening[at]
        if (existing != null) {
            // A second `listen` replaces the sink; the acceptor keeps going.
            existing.set(sink)
            return Union2.U1(Unit)
        }
        val server = try {
            ServerSocket(at.port, 50, java.net.InetAddress.getByName(at.host))
        } catch (e: Exception) {
            return wireFailed(at, e.toString())
        }
        val slot = AtomicReference<Int?>(sink)
        listening[at] = slot
        // An open listener is a source of work the scheduler cannot see, so it
        // must not declare the program idle or deadlocked while one is open:
        // tell it [threadsafe-platform] [actor-on-idle].
        salvo.SalvoSched.externalBegin()
        Thread {
            try {
                while (slot.get() != null) {
                    val socket = server.accept()
                    if (slot.get() == null) { socket.close(); break }
                    Thread { serve(socket, slot) }.apply { isDaemon = true }.start()
                }
            } catch (e: Exception) {
                // The server socket closed under us: the acceptor is done.
            } finally {
                try { server.close() } catch (e: Exception) {}
            }
        }.apply { isDaemon = true }.start()
        return Union2.U1(Unit)
    }

    override fun unlisten(at: NodeEndpoint) {
        val slot = listening.remove(at) ?: return
        slot.set(null)
        // Wake the acceptor so it sees the cleared slot and exits.
        try { Socket(at.host, at.port).close() } catch (e: Exception) {}
        salvo.SalvoSched.externalEnd()
    }

    override fun deliver(to: NodeEndpoint, frame: salvo.SalvoBytes): Union2<Unit, Union2<Unreachable, WireFailed>> {
        val peer = peers[to] ?: run {
            val socket = try {
                Socket(to.host, to.port)
            } catch (e: Exception) {
                return unreachable(to)
            }
            val fresh = Peer(socket)
            try {
                writeFrame(fresh.out, encodeHello(bind))
            } catch (e: Exception) {
                return wireFailed(to, e.toString())
            }
            // Two pools racing to open the same peer: keep the first, close
            // the loser.
            val prior = peers.putIfAbsent(to, fresh)
            if (prior != null) {
                try { socket.close() } catch (e: Exception) {}
                prior
            } else {
                fresh
            }
        }
        return try {
            synchronized(peer) { writeFrame(peer.out, frame.toByteArray()) }
            Union2.U1(Unit)
        } catch (e: Exception) {
            // A failed connection is dropped; the next `deliver` reconnects.
            peers.remove(to, peer)
            try { peer.socket.close() } catch (e2: Exception) {}
            wireFailed(to, e.toString())
        }
    }

    override fun localEndpoint(): NodeEndpoint = bind
}
