// `runtime.streams`: the process's host stream table, a service on the
// runtime's core [runtime-layers] (the runtime record).
//
// Every host producer — a file `HostRawFs` opened, a body a network client
// received, a buffer — registers a host stream here and gets a handle back,
// and `stream.host`'s `HostRawStreams` reads and writes whatever is
// registered, whoever registered it. The table, the read-ahead buffer, line
// splitting, the byte position, failure recording and strict decoding are
// Salvo; the host supplies only the stream objects and four leaf operations
// on them (read a chunk, write, flush, close).
//
// The table is locked only to find an entry: an operation **checks the entry
// out**, works on it with no lock held, and checks it back in, so a slow read
// of one stream never blocks another. A stream token is linear, so two
// operations on one handle at once are a host program's doing (a reader
// thread); the second waits until the first checks the entry back in.

import runtime

// ---- what only the host can do [platform-type]

// A host stream to read: `Box<dyn Read + Send>`, an `InputStream`.
export linear platform type HostIn canbe Mut

// A host stream to write.
export linear platform type HostOut canbe Mut

// What one host read answers: up to the asked-for bytes (none at the end),
// or what failed.
export struct HostRead {
    data: Bytes,
    error: Str?
}

// Reads up to [max] bytes; empty at the end of the stream.
platform fn host_read(h: Mut HostIn, max: Int) [] -> HostRead => h: Mut, max

// Releases the stream.
platform fn host_close_in(h: Mut HostIn) [] -> None => !h

// Writes all of [data], answering what failed.
platform fn host_write(h: Mut HostOut, data: Bytes) [] -> Str? => h: Mut, data

// Pushes written bytes on, answering what failed.
platform fn host_flush(h: Mut HostOut) [] -> Str? => h: Mut

// Releases the stream, answering what failed.
platform fn host_close_out(h: Mut HostOut) [] -> Str? => !h

// A host stream over [data]: what `raw_from_bytes` registers.
platform fn host_bytes_in(data: Bytes) [] -> Mut HostIn => !data

// [stream-provider] A handle this table never minted is another provider's
// stream — a `MemFs` token handed to the host — which is a program bug, and
// traps naming the rule.
platform fn not_ours(handle: Long) [] -> Never => handle

// ---- the entries

// [stream-table] A failure recorded against a stream: strict decoding
// ([utf8]) or what the host reported. Once one is recorded, later reads
// report the end and the close reports why; the same for writes at flush and
// close.
export struct Fault {
    utf8: Bool,
    message: Str
}

// [stream-table] A readable stream. [position] counts bytes *handed to the
// reader* — bytes read ahead for a line do not count until they are taken —
// so a file's position is exactly the offset `open_read_at` takes.
export linear struct InEntry canbe Mut {
    source: Str,
    host: Mut HostIn,
    position: Long,
    ahead: Mut Bytes,
    failed: Fault?
}

// [stream-table] A writable stream. [position] counts bytes accepted.
export linear struct OutEntry canbe Mut {
    source: Str,
    host: Mut HostOut,
    position: Long,
    failed: Fault?
}

fn drop_in_entry(e: Mut InEntry) [] -> None => !e {
    let {source, host, position, ahead, failed} = e
    host_close_in(host)
}

fn drop_out_entry(e: Mut OutEntry) [] -> None => !e {
    let {source, host, position, failed} = e
    let _closed = host_close_out(host)
}

// What checking an entry out answers: the entry, or that another operation
// has it ([me] recorded to be woken when it comes back), or that the table
// never minted the handle.
struct Busy {}
struct Unknown {}

// [stream-receive] A receive in flight: the answer's token, until the reader
// thread has read.
linear struct Pending { handle: Long, done: Reply<Chunk> }

fn drop_pending(p: Pending) [] -> None => !p {
    let {handle, done} = p
    send(done, Chunk { data: bytes_of(), end: true, fault: None, source: "" })
}

effect StreamTable {
    fn next_handle() -> Long
    fn put_in(handle: Long, e: Mut InEntry) -> None => !handle, !e
    fn put_out(handle: Long, e: Mut OutEntry) -> None => !handle, !e
    fn take_in(handle: Long, me: Parker) -> Mut InEntry | Busy | Unknown => handle, !me
    fn take_out(handle: Long, me: Parker) -> Mut OutEntry | Busy | Unknown => handle, !me
    // Entries checked out are not in the table; [remove] forgets one for
    // good (a close).
    fn forget(handle: Long) -> None => handle
    fn add_pending(p: Pending) -> None => !p
    fn take_pending(handle: Long) -> Pending? => handle
}

handler Streams() of StreamTable {
    next: Long = 0
    in_keys: Mut List<Long> = mut_list_of()
    ins: Mut List<Mut InEntry> = mut_list_of()
    out_keys: Mut List<Long> = mut_list_of()
    outs: Mut List<Mut OutEntry> = mut_list_of()
    // Handles checked out, and who waits for one to come back.
    busy: Mut List<Long> = mut_list_of()
    waiting: Mut List<Parker> = mut_list_of()
    pending: Mut List<Pending> = mut_list_of()

    fn next_handle() -> Long {
        next = next + 1
        return copy(next)
    }

    fn put_in(handle: Long, e: Mut InEntry) -> None => !handle, !e {
        unbusy(busy, waiting, copy(handle))
        add(in_keys, handle)
        add(ins, e)
    }

    fn put_out(handle: Long, e: Mut OutEntry) -> None => !handle, !e {
        unbusy(busy, waiting, copy(handle))
        add(out_keys, handle)
        add(outs, e)
    }

    fn take_in(handle: Long, me: Parker) -> Mut InEntry | Busy | Unknown => handle, !me {
        let at = index_in(in_keys, handle)
        if at >= 0 {
            let _k = remove_at(in_keys, copy(at))
            add(busy, copy(handle))
            return remove_at(ins, at)!
        }
        if index_in(busy, handle) >= 0 {
            add(waiting, me)
            return Busy {}
        }
        return Unknown {}
    }

    fn take_out(handle: Long, me: Parker) -> Mut OutEntry | Busy | Unknown => handle, !me {
        let at = index_in(out_keys, handle)
        if at >= 0 {
            let _k = remove_at(out_keys, copy(at))
            add(busy, copy(handle))
            return remove_at(outs, at)!
        }
        if index_in(busy, handle) >= 0 {
            add(waiting, me)
            return Busy {}
        }
        return Unknown {}
    }

    fn forget(handle: Long) -> None => handle {
        unbusy(busy, waiting, copy(handle))
    }

    fn add_pending(p: Pending) -> None => !p {
        add(pending, p)
    }

    fn take_pending(handle: Long) -> Pending? => handle {
        let i = 0
        while i < size(pending) {
            if get(pending, i)!.handle == handle {
                return remove_at(pending, i)
            }
            i = i + 1
        }
        return None
    }
}

use Streams()

fn index_in(keys: List<Long>, handle: Long) [] -> Int => keys, handle {
    let i = 0
    while i < size(keys) {
        if get(keys, i)! == handle {
            return i
        }
        i = i + 1
    }
    return -1
}

// An entry came back: it is no longer checked out, and whoever waited for
// one looks again.
fn unbusy(busy: Mut List<Long>, waiting: Mut List<Parker>, handle: Long) [] -> None
=> busy: Mut, waiting: Mut, !handle {
    let at = index_in(busy, handle)
    if at >= 0 {
        let _h = remove_at(busy, at)
    }
    while size(waiting) > 0 {
        unpark(remove_at(waiting, 0)!)
    }
}

// ---- registering [stream-table]

// [stream-handle] A handle for a new stream, unique in the process: every
// stream table, host and in-memory, draws from this one counter, so a handle
// reaching the wrong table is unknown there rather than another stream's.
export fn fresh_handle() [] -> Long {
    return next_handle()
}

// Registers a readable host stream named [source], at byte [position].
export fn register_in(source: Str, host: Mut HostIn, position: Long) [] -> Long => !source, !host, !position {
    let handle = fresh_handle()
    put_in(copy(handle), Mut InEntry { source: source, host: host, position: position, ahead: mut_bytes(), failed: None })
    return handle
}

// Registers a writable host stream named [source], at byte [position].
export fn register_out(source: Str, host: Mut HostOut, position: Long) [] -> Long => !source, !host, !position {
    let handle = fresh_handle()
    put_out(copy(handle), Mut OutEntry { source: source, host: host, position: position, failed: None })
    return handle
}

// Registers a readable stream over [data].
export fn register_bytes(data: Bytes) [] -> Long => !data {
    return register_in("<bytes>", host_bytes_in(data), 0L)
}

// ---- checking out

// The readable entry [handle], checked out: waits while another operation
// has it; traps for a handle the table never minted [stream-provider].
export fn checkout_in(handle: Long) [] -> Mut InEntry => handle {
    while true {
        let got = take_in(copy(handle), this_parker())
        if got is Mut InEntry e {
            return e
        }
        if got is Unknown {
            not_ours(copy(handle))
        }
        park(this_parker())
    }
    return checkout_in(handle)
}

// The writable entry [handle], checked out.
export fn checkout_out(handle: Long) [] -> Mut OutEntry => handle {
    while true {
        let got = take_out(copy(handle), this_parker())
        if got is Mut OutEntry e {
            return e
        }
        if got is Unknown {
            not_ours(copy(handle))
        }
        park(this_parker())
    }
    return checkout_out(handle)
}

// Checks [e] back in under [handle].
export fn checkin_in(handle: Long, e: Mut InEntry) [] -> None => !handle, !e {
    put_in(handle, e)
}

export fn checkin_out(handle: Long, e: Mut OutEntry) [] -> None => !handle, !e {
    put_out(handle, e)
}

// Closes a checked-out readable entry for good, answering any failure
// recorded on it.
export fn close_in(handle: Long, e: Mut InEntry) [] -> Fault? => !handle, !e {
    forget(handle)
    let {source, host, position, ahead, failed} = e
    host_close_in(host)
    return failed
}

// Flushes and closes a checked-out writable entry for good, answering what
// failed: a failure recorded before, then the flush's, then the close's.
export fn close_out(handle: Long, e: Mut OutEntry) [] -> Fault? => !handle, !e {
    forget(handle)
    let {source, host, position, failed} = e
    let flushing = host_flush(host)
    let closing = host_close_out(host)
    if failed is Fault f {
        return f
    }
    if flushing is Str message {
        return Fault { utf8: false, message: message }
    }
    if closing is Str message {
        return Fault { utf8: false, message: message }
    }
    return None
}

// ---- reading [stream-table]

fn record(e: Mut InEntry, message: Str) [] -> Fault => e: Mut, !message {
    let f = Fault { utf8: false, message: message }
    e.failed = copy(f)
    return f
}

// Takes the first [n] bytes of the read-ahead, counting them.
fn take_ahead(e: Mut InEntry, n: Int) [] -> Bytes => e: Mut, n {
    let all = size(e.ahead)
    let front = slice(e.ahead, 0, copy(n)) ?: bytes_of()
    let rest = slice(e.ahead, copy(n), all) ?: bytes_of()
    e.ahead = mut_bytes(rest)
    e.position = e.position + to_long(n)
    return front
}

// A read's outcome: the bytes (a line, everything left, a chunk), whether
// the stream had ended, and the failure that stopped it.
export struct Read {
    data: Bytes,
    end: Bool,
    fault: Fault?
}

// The next line, without `\n` or `\r\n`; `end` at the end of the stream.
export fn read_line(e: Mut InEntry) [] -> Read => e: Mut {
    if e.failed is Fault f {
        return Read { data: bytes_of(), end: true, fault: copy(f) }
    }
    while true {
        let at = index_of(e.ahead, to_byte(10))
        if at is Int i {
            let line = take_ahead(e, i + 1)
            let n = size(line) - 1
            if n > 0 && to_int(get(line, n - 1)!) == 13 {
                n = n - 1
            }
            return Read { data: slice(line, 0, n) ?: bytes_of(), end: false, fault: None }
        }
        let got = host_read(e.host, 8192)
        if got.error is Str message {
            return Read { data: bytes_of(), end: true, fault: record(e, copy(message)) }
        }
        if size(got.data) == 0 {
            if size(e.ahead) == 0 {
                return Read { data: bytes_of(), end: true, fault: None }
            }
            let rest = take_ahead(e, size(e.ahead))
            return Read { data: rest, end: false, fault: None }
        }
        append(e.ahead, got.data)
    }
    return read_line(e)
}

// Everything left.
export fn read_all(e: Mut InEntry) [] -> Read => e: Mut {
    if e.failed is Fault f {
        return Read { data: bytes_of(), end: true, fault: copy(f) }
    }
    let out = mut_bytes(take_ahead(e, size(e.ahead)))
    while true {
        let got = host_read(e.host, 65536)
        if got.error is Str message {
            return Read { data: bytes_of(), end: true, fault: record(e, copy(message)) }
        }
        if size(got.data) == 0 {
            return Read { data: copy(out), end: true, fault: None }
        }
        e.position = e.position + to_long(size(got.data))
        append(out, got.data)
    }
    return read_all(e)
}

// Up to [max] bytes; none at the end.
export fn read_up_to(e: Mut InEntry, max: Int) [] -> Read => e: Mut, max {
    if e.failed is Fault f {
        return Read { data: bytes_of(), end: true, fault: copy(f) }
    }
    if max <= 0 {
        return Read { data: bytes_of(), end: false, fault: None }
    }
    if size(e.ahead) > 0 {
        let n = copy(max)
        if size(e.ahead) < n {
            n = size(e.ahead)
        }
        return Read { data: take_ahead(e, n), end: false, fault: None }
    }
    let got = host_read(e.host, max)
    if got.error is Str message {
        return Read { data: bytes_of(), end: true, fault: record(e, copy(message)) }
    }
    e.position = e.position + to_long(size(got.data))
    return Read { data: copy(got.data), end: size(got.data) == 0, fault: None }
}

// [stream-table] Decodes [data] strictly, recording a failure against [e].
export fn decode(e: Mut InEntry, data: Bytes) [] -> Str? => e: Mut, data {
    let text = str_of_bytes(data)
    if text is None {
        e.failed = Fault { utf8: true, message: "" }
    }
    return text
}

// ---- writing [stream-table]

// (A field narrowed by a test cannot be assigned a wider value in the same
// fn, so recording goes through here.)
fn record_out(e: Mut OutEntry, message: Str) [] -> None => e: Mut, !message {
    e.failed = Fault { utf8: false, message: message }
}

// Accepts [data], answering how many bytes: none once a failure is recorded.
export fn write(e: Mut OutEntry, data: Bytes) [] -> Long => e: Mut, data {
    if e.failed is Fault earlier {
        return 0L
    }
    let failed = host_write(e.host, data)
    if failed is Str message {
        record_out(e, copy(message))
        return 0L
    }
    e.position = e.position + to_long(size(data))
    return to_long(size(data))
}

// Pushes accepted bytes on, answering (and clearing) a recorded failure.
export fn flush(e: Mut OutEntry) [] -> Fault? => e: Mut {
    let failed = host_flush(e.host)
    if failed is Str message {
        e.failed = Fault { utf8: false, message: copy(message) }
    }
    let f = copy(e.failed)
    e.failed = None
    return f
}

// ---- receiving [stream-receive]

// One received chunk: the bytes (never empty unless [end]), whether the
// stream ended, the failure that stopped it, and the stream's name.
export struct Chunk {
    data: Bytes,
    end: Bool,
    fault: Fault?,
    source: Str
}

// Reads the next bytes of [handle] on a thread of its own and answers
// [done] from there: the caller's worker never waits. At the end, or on a
// failure, the stream is closed before answering. Until it answers, the
// program has an outside source of work [threadsafe-platform], so a frame
// waiting on [done] is neither idle nor deadlocked.
export fn receive(handle: Long, done: Reply<Chunk>) [] -> None => !handle, !done {
    add_pending(Pending { handle: copy(handle), done: done })
    external_begin()
    start_reader(handle)
}

fn start_reader(handle: Long) [] -> None => !handle {
    start_thread(() -> { read_and_answer(copy(handle)) })
}

fn read_and_answer(handle: Long) [] -> None => !handle {
    let e = checkout_in(copy(handle))
    let got = read_up_to(e, 65536)
    let source = copy(e.source)
    if got.end || !(got.fault is None) {
        let _closed = close_in(copy(handle), e)
    } else {
        checkin_in(copy(handle), e)
    }
    let p = take_pending(handle)
    if p is Pending found {
        let {handle: _h, done} = found
        send(done, Chunk { data: got.data, end: got.end, fault: got.fault, source: source })
    }
    external_end()
}

// ---- for host code [stream-table]

// Takes the readable stream [handle] out of the table for good, for host
// code that reads it itself (a request body an SDK uploads): the read-ahead,
// position and any recorded failure travel with it.
export fn take_for_host(handle: Long) [] -> Mut InEntry => !handle {
    let e = checkout_in(copy(handle))
    forget(handle)
    return e
}
