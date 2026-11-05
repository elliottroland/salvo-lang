// The process's host stream table: every stream the host opens (a file, a
// network body, a buffer of bytes) is registered here under a handle, and
// read and written through it, whoever opened it.
//
// This is std's own module, as everything under `std.runtime` is: a program
// cannot import it. A program reads and writes streams through [std.stream]'s
// [Streams] effect; on the host, [std.stream.host]'s [HostRawStreams] is the
// handler that works on this table, and [std.fs.host]'s [HostRawFs]
// registers the files it opens here. Host code (an SDK's glue, written in
// Rust or Kotlin) registers its own streams through the runtime's entry
// points, `salvo_stream_register_in`/`_out` on Rust and
// `SalvoStreams.registerIn`/`registerOut` on Kotlin, and gets back a handle
// to wrap in a stream token.
//
// **What is Salvo and what is the host's.** The table, the read-ahead buffer,
// line splitting, the byte position, failure recording and strict UTF-8
// decoding are all written here, in Salvo. The host supplies only the stream
// objects ([HostIn], [HostOut]) and a few leaf operations on them: read a
// chunk, write, flush, close, and a stream over bytes in memory.
//
// **Checking out.** The table is locked only while an entry is found. An
// operation **checks the entry out** of the table, works on it with no lock
// held, and checks it back in, so a slow read of one stream never blocks
// work on another. A second operation on a handle that is checked out waits
// until the entry comes back. Stream tokens are linear, so a program cannot
// start two operations on one stream at once; the wait is for host code,
// such as the reader thread [receive] starts.
//
// ```no-check
// // How stream.host's HostRawStreams reads up to `max` bytes.
// let e = checkout_in(copy(handle))
// let r = read_up_to(e, max)
// let source = copy(e.source)
// checkin_in(handle, e)
// if r.fault is Fault f {
//     return err(kind(source, f))
// }
// return ok(r.data)
// ```
//
// **Failures are recorded.** When a read fails, or its bytes are not valid
// UTF-8, the failure is recorded on the entry: that read and every later one
// report the end of the stream, and closing it answers the failure. A failed
// write is recorded the same way; later writes are skipped, and the next
// flush or the close answers it.
//
// **A handle belongs to one table.** Every stream table in the program, this
// one and the in-memory ones ([std.fs.mem]'s [MemFs]), draws its handles from
// one counter ([fresh_handle]). A handle this table did not mint is
// another provider's stream, which is a program bug, and the operation traps
// rather than reading the wrong stream.
// @compilerrules [runtime-layers, stream-table, stream-provider, stream-handle]

import std.runtime

// ---- what only the host can do
// @compilerrules [platform-type]

// A host stream open for reading: a `Box<dyn Read + Send>` on Rust, an
// `InputStream` on Kotlin.
//
// Linear: whoever holds one must close it. An entry in the table holds it
// until the stream is closed or [take_for_host] hands it to host code.
export linear platform type HostIn canbe Mut

// A host stream open for writing: a `Box<dyn Write + Send>` on Rust, an
// `OutputStream` on Kotlin. Linear, like [HostIn].
export linear platform type HostOut canbe Mut

// What one host read answers: the bytes read, or what failed.
export struct HostRead {
    // Up to the number of bytes asked for; none at the end of the stream.
    data: Bytes,
    // The host's message when the read failed.
    error: Str?
}

// Reads up to [max] bytes in one host read; empty at the end of the
// stream. A read may answer fewer bytes than asked for before the end.
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

// Traps for a handle this table never minted: another provider's stream (a
// `MemFs` token handed to the host), which is a program bug. The trap names
// the rule.
// @compilerrules [stream-provider]
platform fn not_ours(handle: Long) [] -> Never => handle

// ---- the entries

// A failure recorded against a stream: bytes that were not valid UTF-8, or
// an error the host reported.
//
// Once a read records one, that read and every later one report the end of
// the stream, and [close_in] answers it. Once a write records one, later
// writes are skipped, and [flush] or [close_out] answers it.
// @compilerrules [stream-table]
export struct Fault {
    // `true` when the failure is bytes that did not decode as UTF-8.
    utf8: Bool,
    // The host's message; empty for a decoding failure.
    message: Str
}

// A readable stream in the table: the host stream and what this module
// keeps about it.
//
// Linear, because it owns the host stream: it is closed by [close_in], or
// handed to host code by [take_for_host].
// @compilerrules [stream-table]
export linear struct InEntry canbe Mut {
    // What the stream is, for error messages: a path, or a description.
    source: Str,
    // The host stream.
    host: Mut HostIn,
    // Bytes handed to the reader so far. Bytes read ahead for a line do not
    // count until they are taken, so for a file this is exactly the offset
    // `open_read_at` would resume at.
    position: Long,
    // Bytes read from the host and not yet handed out.
    ahead: Mut Bytes,
    // The failure recorded on the stream, if any.
    failed: Fault?
}

// A writable stream in the table. Linear, because it owns the host stream:
// it is closed by [close_out].
// @compilerrules [stream-table]
export linear struct OutEntry canbe Mut {
    // What the stream is, for error messages: a path, or a description.
    source: Str,
    // The host stream.
    host: Mut HostOut,
    // Bytes accepted so far.
    position: Long,
    // The failure recorded on the stream, if any.
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

// A receive in flight: the reply to answer, kept until the reader thread
// has read. One dropped unanswered is answered with the end of the stream.
// @compilerrules [stream-receive]
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
    // Entries checked out are not in the table; this forgets [handle] for
    // good (a close), waking whoever waits for it.
    fn forget(handle: Long) -> None => handle
    fn add_pending(p: Pending) -> None => !p
    fn take_pending(handle: Long) -> Pending? => handle
}

handler Streams() for StreamTable {
    state {
        next: Long = 0
        in_keys: Mut List<Long> = Mut List()
        ins: Mut List<Mut InEntry> = Mut List()
        out_keys: Mut List<Long> = Mut List()
        outs: Mut List<Mut OutEntry> = Mut List()
        // Handles checked out, and who waits for one to come back.
        busy: Mut List<Long> = Mut List()
        waiting: Mut List<Parker> = Mut List()
        pending: Mut List<Pending> = Mut List()
    }

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
            return remove_at(ins, at) ?: unreachable!()
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
            return remove_at(outs, at) ?: unreachable!()
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
            if (get(pending, i) ?: unreachable!()).handle == handle {
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
        if (get(keys, i) ?: unreachable!()) == handle {
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
        unpark(remove_at(waiting, 0) ?: unreachable!())
    }
}

// ---- registering
// @compilerrules [stream-table]

// A handle for a new stream, unique in the process.
//
// Every stream table, the host's and the in-memory ones, draws from this
// one counter, so a handle that reaches the wrong table is unknown there
// rather than another stream's. Handles start at 1 and only go up.
// @compilerrules [stream-handle]
export fn fresh_handle() [] -> Long {
    return next_handle()
}

// Registers a readable host stream and answers its handle.
//
// @param source    what the stream is, for error messages: a path, or a
//                  description
// @param host      the host stream; the table owns it from now on
// @param position  the byte offset the stream starts at (non-zero for a
//                  file opened part-way)
export fn register_in(source: Str, host: Mut HostIn, position: Long) [] -> Long => !source, !host, !position {
    let handle = fresh_handle()
    put_in(copy(handle), Mut InEntry { source: source, host: host, position: position, ahead: mut_bytes(), failed: None })
    return handle
}

// Registers a writable host stream and answers its handle.
//
// @param source    what the stream is, for error messages
// @param host      the host stream; the table owns it from now on
// @param position  the byte offset writing starts at (the size of a file
//                  opened to append)
export fn register_out(source: Str, host: Mut HostOut, position: Long) [] -> Long => !source, !host, !position {
    let handle = fresh_handle()
    put_out(copy(handle), Mut OutEntry { source: source, host: host, position: position, failed: None })
    return handle
}

// Registers a readable stream over bytes in memory and answers its handle:
// what `Streams.from_bytes` mints on the host. Its source is `<bytes>`.
export fn register_bytes(data: Bytes) [] -> Long => !data {
    return register_in("<bytes>", host_bytes_in(data), 0L)
}

// ---- checking out

// Checks the readable entry [handle] out of the table, for the caller to
// work on with no lock held.
//
// Waits while another operation has the entry checked out, and traps for a
// handle this table never minted. Every checkout must be followed by
// [checkin_in], or by [close_in] when the operation ends the stream.
// @compilerrules [stream-provider]
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

// Checks the writable entry [handle] out of the table: [checkout_in] for
// the write side, ended by [checkin_out] or [close_out].
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

// Puts a readable entry back in the table under its handle, waking any
// operation waiting for it.
export fn checkin_in(handle: Long, e: Mut InEntry) [] -> None => !handle, !e {
    put_in(handle, e)
}

// Puts a writable entry back in the table under its handle, waking any
// operation waiting for it.
export fn checkin_out(handle: Long, e: Mut OutEntry) [] -> None => !handle, !e {
    put_out(handle, e)
}

// Closes a checked-out readable entry for good, answering the failure
// recorded on it, if any. The handle is unknown to the table afterwards.
export fn close_in(handle: Long, e: Mut InEntry) [] -> Fault? => !handle, !e {
    forget(handle)
    let {source, host, position, ahead, failed} = e
    host_close_in(host)
    return failed
}

// Flushes and closes a checked-out writable entry for good, answering what
// failed, if anything. The handle is unknown to the table afterwards.
//
// Only one failure is answered: one recorded by an earlier write first,
// then the flush's, then the close's. The host stream is closed whichever
// failed.
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

// ---- reading
// @compilerrules [stream-table]

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

// What a read answers: the bytes, whether the stream had ended, and the
// failure that stopped it.
export struct Read {
    // A line, everything left, or a chunk, depending on the read.
    data: Bytes,
    // `true` when there was nothing left to read, or a failure stopped it.
    end: Bool,
    // The failure recorded on the stream, by this read or an earlier one.
    fault: Fault?
}

// Reads the next line, without its terminator.
//
// A line ends at `\n`, and a `\r` just before it is dropped too. The last
// line of a stream need not end in a terminator. The host is read 8 KiB at
// a time and what follows the line stays in the read-ahead for the next
// read, so only the line and its terminator count towards the position. The
// answer is `end` when nothing is left, or when a failure is or was
// recorded. The bytes are not decoded; [decode] does that.
export fn read_line(e: Mut InEntry) [] -> Read => e: Mut {
    if e.failed is Fault f {
        return Read { data: bytes_of(), end: true, fault: copy(f) }
    }
    while true {
        let at = index_of(e.ahead, to_byte(10))
        if at is Int i {
            let line = take_ahead(e, i + 1)
            let n = size(line) - 1
            if n > 0 && to_int(get(line, n - 1) ?: unreachable!()) == 13 {
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

// Reads everything left in the stream: the read-ahead, then the host's
// bytes, 64 KiB at a time, until the end. A failure part-way answers no
// bytes, with the failure.
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

// Reads up to [max] bytes.
//
// Bytes in the read-ahead are answered first, and alone, even when there
// are fewer than [max]; otherwise this is one host read. So the answer may
// be shorter than asked for before the end. An empty answer with `end`
// means the stream has ended; a [max] of 0 or less answers no bytes and
// does not end it.
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

// Decodes bytes as UTF-8, strictly: `None` when they are not valid, which
// also records the failure on the entry, so later reads end and the close
// reports it.
// @compilerrules [stream-table]
export fn decode(e: Mut InEntry, data: Bytes) [] -> Str? => e: Mut, data {
    let text = str_of_bytes(data)
    if text is None {
        e.failed = Fault { utf8: true, message: "" }
    }
    return text
}

// ---- writing
// @compilerrules [stream-table]

// (A field narrowed by a test cannot be assigned a wider value in the same
// fn, so recording goes through here.)
fn record_out(e: Mut OutEntry, message: Str) [] -> None => e: Mut, !message {
    e.failed = Fault { utf8: false, message: message }
}

// Writes all of [data] to the host stream and answers how many bytes that
// was: all of them, or none when the write failed (which records the
// failure) or a failure was already recorded.
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

// Flushes the host stream and answers the recorded failure, if any: the
// flush's own, or one an earlier write recorded.
//
// The failure is cleared once answered, so writes after a reported flush
// are attempted again.
export fn flush(e: Mut OutEntry) [] -> Fault? => e: Mut {
    let failed = host_flush(e.host)
    if failed is Str message {
        e.failed = Fault { utf8: false, message: copy(message) }
    }
    let f = copy(e.failed)
    e.failed = None
    return f
}

// ---- receiving
// @compilerrules [stream-receive]

// What [receive] answers: one chunk of a stream, or its end.
export struct Chunk {
    // Up to 64 KiB; never empty unless [end] is `true`.
    data: Bytes,
    // `true` when the stream ended or failed; it has been closed.
    end: Bool,
    // The failure that stopped the stream, if one did.
    fault: Fault?,
    // What the stream is, for error messages.
    source: Str
}

// Reads the next chunk of a stream without blocking the caller, and answers
// on a reply.
//
// Returns at once. Each call starts a host thread of its own, which reads
// up to 64 KiB and answers [done] from there, so no worker of the
// program's waits on a slow source. At the end, or on a failure, the stream
// is closed and leaves the table before the answer is sent; otherwise it is
// checked back in for the next read. Until the answer is sent the program
// has an outside source of work, so a frame waiting on [done] is neither
// idle nor reported as deadlocked.
//
// An actor test cannot use this: a host thread's work would arrive off the
// virtual clock, so opening one stops the test with a report.
//
// @param handle  the stream to read; it must not be in use by anything else
// @param done    answered once, with a [Chunk]
// @compilerrules [platform-threadsafe, stream-receive, test-actor]
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

// ---- for host code
// @compilerrules [stream-table]

// Takes a readable stream out of the table for good, for host code that
// reads it itself (a request body an SDK uploads).
//
// The entry's read-ahead, position and any recorded failure travel with it,
// so the host reads on from exactly where Salvo stopped. The handle is
// unknown to the table afterwards. Host code reaches this through
// `salvo_stream_take_in` on Rust and `SalvoStreams.takeIn` on Kotlin.
export fn take_for_host(handle: Long) [] -> Mut InEntry => !handle {
    let e = checkout_in(copy(handle))
    forget(handle)
    return e
}
