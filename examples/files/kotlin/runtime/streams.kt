package salvo.runtime.streams

import salvo.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.deque.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.runtime.*

// [mod-use] The module's `use` #0, bound on first use.
private val __moduleUse0: StreamTable by lazy {
    val stream_table: StreamTable = __Mon_StreamTable(Streams())
    stream_table
}

data class HostRead(
    val data: salvo.SalvoBytes,
    val error: String?,
)

object __Codec_HostRead : salvo.WireCodec<HostRead> {
    override fun enc(v: HostRead, out: salvo.WireOut) {
        salvo.BytesCodec.enc(v.data, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.error, out)
    }
    override fun dec(inp: salvo.WireIn): HostRead = HostRead(salvo.BytesCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

fun hostReadPlatform(h: salvo.platform.runtime.streams.HostIn, max: Int): HostRead {
    return salvo.platform.runtime.streams.hostRead(h, max)
}

fun hostCloseInPlatform(h: salvo.platform.runtime.streams.HostIn) {
    return salvo.platform.runtime.streams.hostCloseIn(h)
}

fun hostWritePlatform(h: salvo.platform.runtime.streams.HostOut, data: salvo.SalvoBytes): String? {
    return salvo.platform.runtime.streams.hostWrite(h, data)
}

fun hostFlushPlatform(h: salvo.platform.runtime.streams.HostOut): String? {
    return salvo.platform.runtime.streams.hostFlush(h)
}

fun hostCloseOutPlatform(h: salvo.platform.runtime.streams.HostOut): String? {
    return salvo.platform.runtime.streams.hostCloseOut(h)
}

fun hostBytesInPlatform(data: salvo.SalvoBytes): salvo.platform.runtime.streams.HostIn {
    return salvo.platform.runtime.streams.hostBytesIn(data)
}

fun notOursPlatform(handle: Long): Nothing {
    return salvo.platform.runtime.streams.notOurs(handle)
}

data class Fault(
    val utf8: Boolean,
    val message: String,
)

object __Codec_Fault : salvo.WireCodec<Fault> {
    override fun enc(v: Fault, out: salvo.WireOut) {
        salvo.BoolCodec.enc(v.utf8, out)
        salvo.StrCodec.enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): Fault = Fault(salvo.BoolCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class InEntry(
    var source: String,
    var host: salvo.platform.runtime.streams.HostIn,
    var position: Long,
    var ahead: salvo.SalvoBytes,
    var failed: Fault?,
)

data class OutEntry(
    var source: String,
    var host: salvo.platform.runtime.streams.HostOut,
    var position: Long,
    var failed: Fault?,
)

fun dropInEntry(e: InEntry) {
    val __destructured1 = e
    val source = __destructured1.source
    val host = __destructured1.host
    val position = __destructured1.position
    val ahead = __destructured1.ahead
    val failed = __destructured1.failed
    hostCloseInPlatform(host)
}

fun dropOutEntry(e: OutEntry) {
    val __destructured2 = e
    val source = __destructured2.source
    val host = __destructured2.host
    val position = __destructured2.position
    val failed = __destructured2.failed
    val _closed = hostCloseOutPlatform(host)
}

class Busy

object __Codec_Busy : salvo.WireCodec<Busy> {
    override fun enc(v: Busy, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Busy = Busy()
}

class Unknown

object __Codec_Unknown : salvo.WireCodec<Unknown> {
    override fun enc(v: Unknown, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Unknown = Unknown()
}

data class Pending(
    val handle: Long,
    val done: salvo.SalvoReply,
)

object __Codec_Pending : salvo.WireCodec<Pending> {
    override fun enc(v: Pending, out: salvo.WireOut) {
        salvo.LongCodec.enc(v.handle, out)
        salvo.ReplyCodec.enc(v.done, out)
    }
    override fun dec(inp: salvo.WireIn): Pending = Pending(salvo.LongCodec.dec(inp), salvo.ReplyCodec.dec(inp))
}

fun dropPending(p: Pending) {
    val __destructured3 = p
    val handle = __destructured3.handle
    val done = __destructured3.done
    salvo.SalvoSched.replyWire(done, Chunk(data = salvo.SalvoBytes.of(arrayOf<UByte>()), end = true, fault = null, source = ""), __Codec_Chunk)
}

interface StreamTable {
    fun nextHandle(): Long
    fun putIn(handle: Long, e: InEntry)
    fun putOut(handle: Long, e: OutEntry)
    fun takeIn(handle: Long, me: salvo.platform.runtime.Parker): Union3<InEntry, Busy, Unknown>
    fun takeOut(handle: Long, me: salvo.platform.runtime.Parker): Union3<OutEntry, Busy, Unknown>
    fun forget(handle: Long)
    fun addPending(p: Pending)
    fun takePending(handle: Long): Pending?
}

class __Mon_StreamTable(
    private val inner: StreamTable,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : StreamTable {
    override fun nextHandle(): Long {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.nextHandle() } finally { lock.unlock() }
    }
    override fun putIn(handle: Long, e: InEntry) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.putIn(handle, e) } finally { lock.unlock() }
    }
    override fun putOut(handle: Long, e: OutEntry) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.putOut(handle, e) } finally { lock.unlock() }
    }
    override fun takeIn(handle: Long, me: salvo.platform.runtime.Parker): Union3<InEntry, Busy, Unknown> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.takeIn(handle, me) } finally { lock.unlock() }
    }
    override fun takeOut(handle: Long, me: salvo.platform.runtime.Parker): Union3<OutEntry, Busy, Unknown> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.takeOut(handle, me) } finally { lock.unlock() }
    }
    override fun forget(handle: Long) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.forget(handle) } finally { lock.unlock() }
    }
    override fun addPending(p: Pending) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.addPending(p) } finally { lock.unlock() }
    }
    override fun takePending(handle: Long): Pending? {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.takePending(handle) } finally { lock.unlock() }
    }
}

class Streams : StreamTable {
    private var next: Long = 0L
    private var inKeys: MutableList<Long> = mutableListOf<Long>()
    private var ins: MutableList<InEntry> = mutableListOf<InEntry>()
    private var outKeys: MutableList<Long> = mutableListOf<Long>()
    private var outs: MutableList<OutEntry> = mutableListOf<OutEntry>()
    private var busy: MutableList<Long> = mutableListOf<Long>()
    private var waiting: MutableList<salvo.platform.runtime.Parker> = mutableListOf<salvo.platform.runtime.Parker>()
    private var pending: MutableList<Pending> = mutableListOf<Pending>()

    override fun nextHandle(): Long {
        next = next + 1
        return next
    }

    override fun putIn(handle: Long, e: InEntry) {
        unbusy(busy, waiting, handle)
        inKeys.add(handle)
        ins.add(e)
    }

    override fun putOut(handle: Long, e: OutEntry) {
        unbusy(busy, waiting, handle)
        outKeys.add(handle)
        outs.add(e)
    }

    override fun takeIn(handle: Long, me: salvo.platform.runtime.Parker): Union3<InEntry, Busy, Unknown> {
        val at = indexIn(inKeys, handle)
        if (at >= 0) {
            val _k = (inKeys).let { __l -> (at).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
            busy.add(handle)
            return Union3.U1<InEntry, Busy, Unknown>(((ins).let { __l -> (at).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } } ?: throw AssertionError("salvo: value is absent at runtime.streams:159:20")))
        }
        if (indexIn(busy, handle) >= 0) {
            waiting.add(me)
            return Union3.U2<InEntry, Busy, Unknown>(Busy())
        }
        return Union3.U3<InEntry, Busy, Unknown>(Unknown())
    }

    override fun takeOut(handle: Long, me: salvo.platform.runtime.Parker): Union3<OutEntry, Busy, Unknown> {
        val at = indexIn(outKeys, handle)
        if (at >= 0) {
            val _k = (outKeys).let { __l -> (at).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
            busy.add(handle)
            return Union3.U1<OutEntry, Busy, Unknown>(((outs).let { __l -> (at).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } } ?: throw AssertionError("salvo: value is absent at runtime.streams:173:20")))
        }
        if (indexIn(busy, handle) >= 0) {
            waiting.add(me)
            return Union3.U2<OutEntry, Busy, Unknown>(Busy())
        }
        return Union3.U3<OutEntry, Busy, Unknown>(Unknown())
    }

    override fun forget(handle: Long) {
        unbusy(busy, waiting, handle)
    }

    override fun addPending(p: Pending) {
        pending.add(p)
    }

    override fun takePending(handle: Long): Pending? {
        var i = 0
        while (i < pending.size) {
            if ((pending.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime.streams:193:16")).handle == handle) {
                return (pending).let { __l -> (i).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
            }
            i = i + 1
        }
        return null
    }
}

fun indexIn(keys: List<Long>, handle: Long): Int {
    var i = 0
    while (i < keys.size) {
        if ((keys.getOrNull(i) ?: throw AssertionError("salvo: value is absent at runtime.streams:207:12")) == handle) {
            return i
        }
        i = i + 1
    }
    return -1
}

fun unbusy(busy: MutableList<Long>, waiting: MutableList<salvo.platform.runtime.Parker>, handle: Long) {
    val at = indexIn(busy, handle)
    if (at >= 0) {
        val _h = (busy).let { __l -> (at).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } }
    }
    while (waiting.size > 0) {
        unparkPlatform(((waiting).let { __l -> (0).let { __i -> if (__i >= 0 && __i < __l.size) __l.removeAt(__i) else null } } ?: throw AssertionError("salvo: value is absent at runtime.streams:224:16")))
    }
}

fun freshHandle(): Long {
    return __moduleUse0.nextHandle()
}

fun registerIn(source: String, host: salvo.platform.runtime.streams.HostIn, position: Long): Long {
    val handle = freshHandle()
    __moduleUse0.putIn(handle, InEntry(source = source, host = host, position = position, ahead = salvo.SalvoBytes.joined(), failed = null))
    return handle
}

fun registerOut(source: String, host: salvo.platform.runtime.streams.HostOut, position: Long): Long {
    val handle = freshHandle()
    __moduleUse0.putOut(handle, OutEntry(source = source, host = host, position = position, failed = null))
    return handle
}

fun registerBytes(data: salvo.SalvoBytes): Long {
    return registerIn("<bytes>", hostBytesInPlatform(data), 0L)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun checkoutIn(handle: Long): InEntry {
    while (true) {
        val got = __moduleUse0.takeIn(handle, thisParkerPlatform())
        if (got is Union3.U1<*, *, *>) {
            val e = got.value as InEntry
            return e
        }
        if (got is Union3.U3<*, *, *>) {
            notOursPlatform(handle)
        }
        parkPlatform(thisParkerPlatform())
    }
    return checkoutIn(handle)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun checkoutOut(handle: Long): OutEntry {
    while (true) {
        val got = __moduleUse0.takeOut(handle, thisParkerPlatform())
        if (got is Union3.U1<*, *, *>) {
            val e = got.value as OutEntry
            return e
        }
        if (got is Union3.U3<*, *, *>) {
            notOursPlatform(handle)
        }
        parkPlatform(thisParkerPlatform())
    }
    return checkoutOut(handle)
}

fun checkinIn(handle: Long, e: InEntry) {
    __moduleUse0.putIn(handle, e)
}

fun checkinOut(handle: Long, e: OutEntry) {
    __moduleUse0.putOut(handle, e)
}

fun closeIn(handle: Long, e: InEntry): Fault? {
    __moduleUse0.forget(handle)
    val __destructured4 = e
    val source = __destructured4.source
    val host = __destructured4.host
    val position = __destructured4.position
    val ahead = __destructured4.ahead
    val failed = __destructured4.failed
    hostCloseInPlatform(host)
    return failed
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun closeOut(handle: Long, e: OutEntry): Fault? {
    __moduleUse0.forget(handle)
    val __destructured5 = e
    val source = __destructured5.source
    val host = __destructured5.host
    val position = __destructured5.position
    val failed = __destructured5.failed
    val flushing = hostFlushPlatform(host)
    val closing = hostCloseOutPlatform(host)
    if (failed != null) {
        val f = failed as Fault
        return f
    }
    if (flushing != null) {
        val message = flushing as String
        return Fault(utf8 = false, message = message)
    }
    if (closing != null) {
        val message = closing as String
        return Fault(utf8 = false, message = message)
    }
    return null
}

fun record(e: InEntry, message: String): Fault {
    val f = Fault(utf8 = false, message = message)
    e.failed = f
    return f
}

fun takeAhead(e: InEntry, n: Int): salvo.SalvoBytes {
    val all = e.ahead.size
    val front = (e.ahead.slice(0, n) ?: salvo.SalvoBytes.of(arrayOf<UByte>()))
    val rest = (e.ahead.slice(n, all) ?: salvo.SalvoBytes.of(arrayOf<UByte>()))
    e.ahead = salvo.SalvoBytes.joined(rest)
    e.position = e.position + (n).toLong()
    return front
}

data class Read(
    val data: salvo.SalvoBytes,
    val end: Boolean,
    val fault: Fault?,
)

object __Codec_Read : salvo.WireCodec<Read> {
    override fun enc(v: Read, out: salvo.WireOut) {
        salvo.BytesCodec.enc(v.data, out)
        salvo.BoolCodec.enc(v.end, out)
        salvo.OptCodec(__Codec_Fault).enc(v.fault, out)
    }
    override fun dec(inp: salvo.WireIn): Read = Read(salvo.BytesCodec.dec(inp), salvo.BoolCodec.dec(inp), salvo.OptCodec(__Codec_Fault).dec(inp))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readLine(e: InEntry): Read {
    if (e.failed != null) {
        val f = e.failed as Fault
        return Read(data = salvo.SalvoBytes.of(arrayOf<UByte>()), end = true, fault = f)
    }
    while (true) {
        val at = e.ahead.indexOf((10).toUByte())
        if (at != null) {
            val i = at as Int
            val line = takeAhead(e, i + 1)
            var n = line.size - 1
            if (n > 0 && ((line.getOrNull(n - 1) ?: throw AssertionError("salvo: value is absent at runtime.streams:362:32"))).toInt() == 13) {
                n = n - 1
            }
            return Read(data = (line.slice(0, n) ?: salvo.SalvoBytes.of(arrayOf<UByte>())), end = false, fault = null)
        }
        val got = hostReadPlatform(e.host, 8192)
        if (got.error != null) {
            val message = got.error as String
            return Read(data = salvo.SalvoBytes.of(arrayOf<UByte>()), end = true, fault = record(e, message))
        }
        if (got.data.size == 0) {
            if (e.ahead.size == 0) {
                return Read(data = salvo.SalvoBytes.of(arrayOf<UByte>()), end = true, fault = null)
            }
            val rest = takeAhead(e, e.ahead.size)
            return Read(data = rest, end = false, fault = null)
        }
        e.ahead.append(got.data)
    }
    return readLine(e)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readAll(e: InEntry): Read {
    if (e.failed != null) {
        val f = e.failed as Fault
        return Read(data = salvo.SalvoBytes.of(arrayOf<UByte>()), end = true, fault = f)
    }
    val out = salvo.SalvoBytes.joined(takeAhead(e, e.ahead.size))
    while (true) {
        val got = hostReadPlatform(e.host, 65536)
        if (got.error != null) {
            val message = got.error as String
            return Read(data = salvo.SalvoBytes.of(arrayOf<UByte>()), end = true, fault = record(e, message))
        }
        if (got.data.size == 0) {
            return Read(data = salvo.SalvoBytes(out), end = true, fault = null)
        }
        e.position = e.position + (got.data.size).toLong()
        out.append(got.data)
    }
    return readAll(e)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readUpTo(e: InEntry, max: Int): Read {
    if (e.failed != null) {
        val f = e.failed as Fault
        return Read(data = salvo.SalvoBytes.of(arrayOf<UByte>()), end = true, fault = f)
    }
    if (max <= 0) {
        return Read(data = salvo.SalvoBytes.of(arrayOf<UByte>()), end = false, fault = null)
    }
    if (e.ahead.size > 0) {
        var n = max
        if (e.ahead.size < n) {
            n = e.ahead.size
        }
        return Read(data = takeAhead(e, n), end = false, fault = null)
    }
    val got = hostReadPlatform(e.host, max)
    if (got.error != null) {
        val message = got.error as String
        return Read(data = salvo.SalvoBytes.of(arrayOf<UByte>()), end = true, fault = record(e, message))
    }
    e.position = e.position + (got.data.size).toLong()
    return Read(data = salvo.SalvoBytes(got.data), end = got.data.size == 0, fault = null)
}

fun decode(e: InEntry, data: salvo.SalvoBytes): String? {
    val text = data.asString()
    if (text == null) {
        e.failed = Fault(utf8 = true, message = "")
    }
    return text
}

fun recordOut(e: OutEntry, message: String) {
    e.failed = Fault(utf8 = false, message = message)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun write(e: OutEntry, data: salvo.SalvoBytes): Long {
    if (e.failed != null) {
        val earlier = e.failed as Fault
        return 0L
    }
    val failed = hostWritePlatform(e.host, data)
    if (failed != null) {
        val message = failed as String
        recordOut(e, message)
        return 0L
    }
    e.position = e.position + (data.size).toLong()
    return (data.size).toLong()
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun flush__2(e: OutEntry): Fault? {
    val failed = hostFlushPlatform(e.host)
    if (failed != null) {
        val message = failed as String
        e.failed = Fault(utf8 = false, message = message)
    }
    val f = e.failed
    e.failed = null
    return f
}

data class Chunk(
    val data: salvo.SalvoBytes,
    val end: Boolean,
    val fault: Fault?,
    val source: String,
)

object __Codec_Chunk : salvo.WireCodec<Chunk> {
    override fun enc(v: Chunk, out: salvo.WireOut) {
        salvo.BytesCodec.enc(v.data, out)
        salvo.BoolCodec.enc(v.end, out)
        salvo.OptCodec(__Codec_Fault).enc(v.fault, out)
        salvo.StrCodec.enc(v.source, out)
    }
    override fun dec(inp: salvo.WireIn): Chunk = Chunk(salvo.BytesCodec.dec(inp), salvo.BoolCodec.dec(inp), salvo.OptCodec(__Codec_Fault).dec(inp), salvo.StrCodec.dec(inp))
}

fun receive(handle: Long, done: salvo.SalvoReply) {
    __moduleUse0.addPending(Pending(handle = handle, done = done))
    externalBegin()
    startReader(handle)
}

fun startReader(handle: Long) {
    startThreadPlatform({  ->
    readAndAnswer(handle)
})
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun readAndAnswer(handle: Long) {
    val e = checkoutIn(handle)
    val got = readUpTo(e, 65536)
    val source = e.source
    if (got.end || !(got.fault == null)) {
        val _closed = closeIn(handle, e)
    } else {
        checkinIn(handle, e)
    }
    val p = __moduleUse0.takePending(handle)
    if (p != null) {
        val found = p as Pending
        val __destructured6 = found
        val _h = __destructured6.handle
        val done = __destructured6.done
        salvo.SalvoSched.replyWire(done, Chunk(data = got.data, end = got.end, fault = got.fault, source = source), __Codec_Chunk)
    }
    externalEnd()
}

fun takeForHost(handle: Long): InEntry {
    val e = checkoutIn(handle)
    __moduleUse0.forget(handle)
    return e
}
