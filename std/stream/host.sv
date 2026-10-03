// The host's streams: the raw seam over the process-wide stream table, and the
// handler that turns it into `stream`'s `Streams` — the `fs.host` shape
// (`RawFs`, `HostRawFs`, `DefaultFs`) for streams.
//
// Its own module, so a program that fakes the world (`MemFs`) never reaches a
// host class.

import stream
import runtime.streams

// ===== the raw seam =====

// The host's stream table, in plain values: handles are `Long`s and failures
// are droppable kinds, so the platform class implements an interface with no
// obligations in it. Every host producer — a file `HostRawFs` opened, a body a
// network client received — registers into the same table, which is what lets
// one `Streams` read them all.
//
// Read errors are *recorded* by the host rather than returned: a failed
// `raw_read_line` reports the end of the stream and `raw_close_read` reports
// why. Write errors behave the same way, surfacing at `raw_flush` /
// `raw_close_write`. A handle the table never minted traps [stream-provider].
export effect RawStreams {
    // The next line, without its terminator; absent at the end of the stream
    // *or* after a recorded failure.
    fn raw_read_line(handle: Long) -> Str | None
    // Everything left in the stream.
    fn raw_read_all(handle: Long) -> Ok Str | Err StreamError
    // Up to `max` bytes, undecoded.
    fn raw_read_bytes(handle: Long, max: Int) -> Ok Bytes | Err StreamError
    // Up to `max` bytes appended to `buf`, answering how many.
    fn raw_read_to_bytes(handle: Long, buf: Mut Bytes, max: Int) -> Ok Int | Err StreamError => buf: Mut
    // Everything left, decoded strictly and appended to `buf`, answering how
    // many bytes were consumed.
    fn raw_read_to_str(handle: Long, buf: Mut Str) -> Ok Long | Err StreamError => buf: Mut
    // The next line, without its terminator, appended to `buf`; `false` at the
    // end of the stream or after a recorded failure.
    fn raw_read_line_to_str(handle: Long, buf: Mut Str) -> Bool => buf: Mut
    // Bytes consumed so far, counted below the decoder.
    fn raw_read_position(handle: Long) -> Long
    // Releases the read handle, reporting any failure recorded on it.
    fn raw_close_read(handle: Long) -> Ok None | Err StreamError

    // Accepts text, answering how many bytes of it were written.
    fn raw_write(handle: Long, text: Str) -> Long => text
    // Accepts bytes as they are, answering how many were written.
    fn raw_write_bytes(handle: Long, data: Bytes) -> Long => data
    // Bytes accepted so far.
    fn raw_write_position(handle: Long) -> Long
    // Pushes accepted bytes on, reporting a recorded failure.
    fn raw_flush(handle: Long) -> Ok None | Err StreamError
    // Flushes and releases the write handle.
    fn raw_close_write(handle: Long) -> Ok None | Err StreamError

    // [stream-receive] Reads the next bytes on a thread of the host's own and
    // answers on [reply] [platform-reply]: the bytes (never empty), or `End`,
    // or a failure. At `End` or a failure the host has already released the
    // handle, so there is nothing left to close.
    fn raw_receive(handle: Long, reply: Reply<Ok Bytes | End | Err StreamError>) -> None => !reply
    // [stream-from-bytes] Registers a readable stream over [data] in the
    // process's table, answering its handle.
    fn raw_from_bytes(data: Bytes) -> Long => !data
}

// [stream-receive] The host's answer, turned into a `Received`: the token the
// request gave up is minted again around the same handle, so it travels back
// to the caller inside the packet. At `End` or a failure the host released the
// handle, so no token is minted and nothing is owed.
send fn host_received(reply: Reply<Received>, handle: Long, got: Ok Bytes | End | Err StreamError)
=> !reply, !handle, !got {
    when got {
        is Ok { reply.send(ok(Packet { bytes: got, stream: InStream { handle: handle } })) }
        is End { reply.send(got) }
        is Err { reply.send(err(checked<StreamError>(got))) }
    }
}

// The one implementation of `RawStreams`, in Salvo over the runtime's
// stream table (`runtime.streams`, [stream-table]): each member checks the
// stream out, works on it with no lock held, and checks it back in. Stateless,
// so the compiler shares it with no lock.
export handler HostRawStreams() of RawStreams {
    fn raw_read_line(handle: Long) -> Str | None {
        return next_line(handle)
    }

    fn raw_read_all(handle: Long) -> Ok Str | Err StreamError {
        let e = checkout_in(copy(handle))
        let r = read_all(e)
        let source = copy(e.source)
        if r.fault is Fault f {
            checkin_in(handle, e)
            return err(kind(source, f))
        }
        let text = decode(e, r.data)
        checkin_in(handle, e)
        if text is Str t {
            return ok(t)
        }
        return err(kind(source, Fault { utf8: true, message: "" }))
    }

    fn raw_read_bytes(handle: Long, max: Int) -> Ok Bytes | Err StreamError {
        let e = checkout_in(copy(handle))
        let r = read_up_to(e, max)
        let source = copy(e.source)
        checkin_in(handle, e)
        if r.fault is Fault f {
            return err(kind(source, f))
        }
        return ok(r.data)
    }

    fn raw_read_to_bytes(handle: Long, buf: Mut Bytes, max: Int) -> Ok Int | Err StreamError => buf: Mut {
        let e = checkout_in(copy(handle))
        let r = read_up_to(e, max)
        let source = copy(e.source)
        checkin_in(handle, e)
        if r.fault is Fault f {
            return err(kind(source, f))
        }
        append(buf, r.data)
        return ok(size(r.data))
    }

    fn raw_read_to_str(handle: Long, buf: Mut Str) -> Ok Long | Err StreamError => buf: Mut {
        let e = checkout_in(copy(handle))
        let r = read_all(e)
        let source = copy(e.source)
        if r.fault is Fault f {
            checkin_in(handle, e)
            return err(kind(source, f))
        }
        let count = to_long(size(r.data))
        let text = decode(e, r.data)
        checkin_in(handle, e)
        if text is Str t {
            append(buf, t)
            return ok(count)
        }
        return err(kind(source, Fault { utf8: true, message: "" }))
    }

    fn raw_read_line_to_str(handle: Long, buf: Mut Str) -> Bool => buf: Mut {
        let line = next_line(handle)
        if line is Str t {
            append(buf, t)
            return true
        }
        return false
    }

    fn raw_read_position(handle: Long) -> Long {
        let e = checkout_in(copy(handle))
        let at = copy(e.position)
        checkin_in(handle, e)
        return at
    }

    fn raw_close_read(handle: Long) -> Ok None | Err StreamError {
        let e = checkout_in(copy(handle))
        let source = copy(e.source)
        let failed = close_in(handle, e)
        if failed is Fault f {
            return err(kind(source, f))
        }
        return ok(None)
    }

    fn raw_write(handle: Long, text: Str) -> Long => text {
        let e = checkout_out(copy(handle))
        let n = write(e, to_bytes(text))
        checkin_out(handle, e)
        return n
    }

    fn raw_write_bytes(handle: Long, data: Bytes) -> Long => data {
        let e = checkout_out(copy(handle))
        let n = write(e, data)
        checkin_out(handle, e)
        return n
    }

    fn raw_write_position(handle: Long) -> Long {
        let e = checkout_out(copy(handle))
        let at = copy(e.position)
        checkin_out(handle, e)
        return at
    }

    fn raw_flush(handle: Long) -> Ok None | Err StreamError {
        let e = checkout_out(copy(handle))
        let source = copy(e.source)
        let failed = flush(e)
        checkin_out(handle, e)
        if failed is Fault f {
            return err(kind(source, f))
        }
        return ok(None)
    }

    fn raw_close_write(handle: Long) -> Ok None | Err StreamError {
        let e = checkout_out(copy(handle))
        let source = copy(e.source)
        let failed = close_out(handle, e)
        if failed is Fault f {
            return err(kind(source, f))
        }
        return ok(None)
    }

    // [stream-receive] The read runs on a thread of the runtime's, and its
    // chunk comes back as a task that turns it into this protocol's answer.
    fn raw_receive(handle: Long, reply: Reply<Ok Bytes | End | Err StreamError>) -> None => !reply {
        receive(handle, replyto chunk_received(reply))
    }

    fn raw_from_bytes(data: Bytes) -> Long => !data {
        return register_bytes(data)
    }
}

// [stream-receive] A chunk from the runtime's reader, as `raw_receive`'s
// answer.
send fn chunk_received(reply: Reply<Ok Bytes | End | Err StreamError>, c: Chunk) => !reply, !c {
    if c.fault is Fault f {
        send(reply, err(kind(copy(c.source), f)))
    } elif c.end {
        send(reply, End {})
    } else {
        send(reply, ok(copy(c.data)))
    }
}

// The next line, decoded strictly; `None` at the end or after a failure.
fn next_line(handle: Long) [] -> Str? => !handle {
    let e = checkout_in(copy(handle))
    let r = read_line(e)
    let text: Str? = None
    if !r.end && r.fault is None {
        text = decode(e, r.data)
    }
    checkin_in(handle, e)
    return text
}

// A recorded failure as `stream.StreamError`.
fn kind(source: Str, f: Fault) [] -> StreamError => !source, f {
    if f.utf8 {
        return InvalidUtf8 { source: source }
    }
    return StreamFailed { source: source, message: copy(f.message) }
}

// `Streams` over the host's table: wraps each failure in `Checked` and
// discharges each token, delegating the work downwards.
export handler DefaultStreams [RawStreams] of Streams {
    fn read_line(s: InStream) -> Str | None => s {
        return raw_read_line(s.handle)
    }

    fn read_all(s: InStream) -> Ok Str | Err Checked<StreamError> => s {
        let r = raw_read_all(s.handle)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<StreamError>(r)) }
        }
    }

    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err Checked<StreamError> => s {
        let r = raw_read_bytes(s.handle, max)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<StreamError>(r)) }
        }
    }

    fn read_to(s: InStream, buf: Mut Bytes, max: Int) -> Ok Int | Err Checked<StreamError> => s, buf: Mut {
        let r = raw_read_to_bytes(s.handle, buf, max)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<StreamError>(r)) }
        }
    }

    fn read_to(s: InStream, buf: Mut Str) -> Ok Long | Err Checked<StreamError> => s, buf: Mut {
        let r = raw_read_to_str(s.handle, buf)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<StreamError>(r)) }
        }
    }

    fn read_line_to(s: InStream, buf: Mut Str) -> Bool => s, buf: Mut {
        return raw_read_line_to_str(s.handle, buf)
    }

    fn position(s: InStream) -> Long => s {
        return raw_read_position(s.handle)
    }

    fn close(s: InStream) -> Ok None | Err Checked<StreamError> => !s {
        let r = raw_close_read(s.handle)
        discard(s)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<StreamError>(r)) }
        }
    }

    // [stream-receive] The token is given up here — `receive` consumes it, so
    // this is a discharge context [linear-group] — and the handle alone goes
    // to the host, which completes the continuation from its own thread.
    fn receive(s: InStream, reply: Reply<Received>) -> None => !s, !reply {
        let handle = copy(s.handle)
        discard(s)
        raw_receive(copy(handle), replyto host_received(reply, handle))
    }

    fn from_bytes(data: Bytes) -> InStream => !data {
        return InStream { handle: raw_from_bytes(data) }
    }

    fn write(s: OutStream, text: Str) -> Long => s, text {
        return raw_write(s.handle, text)
    }

    fn write_line(s: OutStream, text: Str) -> Long => s, text {
        return raw_write(s.handle, "${text}\n")
    }

    fn write_bytes(s: OutStream, data: Bytes) -> Long => s, data {
        return raw_write_bytes(s.handle, data)
    }

    fn position(s: OutStream) -> Long => s {
        return raw_write_position(s.handle)
    }

    fn flush(s: OutStream) -> Ok None | Err Checked<StreamError> => s {
        let r = raw_flush(s.handle)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<StreamError>(r)) }
        }
    }

    fn close(s: OutStream) -> Ok None | Err Checked<StreamError> => !s {
        let r = raw_close_write(s.handle)
        discard(s)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<StreamError>(r)) }
        }
    }
}
