// The machine's streams: the handlers that give [std.stream]'s [Streams]
// effect its host implementation, over the one process-wide stream table
// every host stream is registered in.
//
// Two handlers, bound in this order at a program's composition root:
//
// - [HostRawStreams] implements [RawStreams], the same operations in plain
//   values: a stream is a `Long` handle and a failure an ordinary
//   [StreamError].
// - [DefaultStreams] implements [Streams] over [RawStreams]: it hands out
//   and takes back the linear `InStream`/`OutStream` tokens, and wraps each
//   failure in `Checked`, so it cannot be dropped unread.
//
// ```
// import std.stream
// import std.stream.host
//
// fn main() [use] -> None {
//     use StdConsole()
//     use HostRawStreams()                             // the host's stream table
//     use DefaultStreams()                             // `Streams` over it
//     let p = lines(from_bytes(to_bytes("one\ntwo\n")))
//     for line in p {
//         println(line)                                // one, then two
//     }
//     let closed = close(p)
//     if closed is Err {
//         ignore(closed)
//     }
// }
// ```
//
// A program that reads files binds [std.fs.host]'s [HostRawFs] and
// [DefaultFs] after these two: what [HostRawFs] opens is registered in the
// same table, so a file, a network body and a buffer are all read by the
// same [Streams].
//
// **Only for the real world.** A program or test that fakes the filesystem
// binds [std.fs.mem]'s [MemFs] instead, which implements [Streams] itself,
// and never imports this module. Do not mix the two: a stream belongs to
// the table that minted it, and handing a `MemFs` stream to these handlers
// (or the reverse) traps.
//
// **Behavior.** Lines are split out of a read-ahead buffer in front of the
// host stream, which later reads take from first; text is decoded strictly
// as UTF-8; a read or write failure is recorded and reported by `flush` or
// `close`; and `receive` reads on a host thread of its own, so it cannot be
// used in an actor test. The table
// itself is [std.runtime.streams], whose page has the details.
// @compilerrules [stream-table, stream-provider]

import std.runtime.streams
import std.stream

// ===== the raw seam =====

// The operations of [Streams] in plain values, over the host's stream table:
// a stream is its `Long` handle, and a failure is a [StreamError] that may
// be dropped.
//
// Nothing here is linear, so an implementation (a platform class, or a fake
// for a test of [DefaultStreams]) holds no obligations; [DefaultStreams]
// adds the tokens and the `Checked` wrapper on top. Every host producer (a
// file `HostRawFs` opened, a body a network client received) registers in
// the same table, which is what lets one [Streams] read them all.
//
// Failures are **recorded**, not returned by every call: a failed
// [raw_read_line] reports the end of the stream and [raw_close_read]
// reports why, and a failed write surfaces at [raw_flush] or
// [raw_close_write]. A handle the table never minted traps.
// @compilerrules [stream-provider]
export effect RawStreams {
    // The next line, without its terminator; absent at the end of the stream
    // *or* after a recorded failure.
    fn raw_read_line(handle: Long) -> Str | None
    // Everything left in the stream.
    fn raw_read_all(handle: Long) -> Ok Str | Err StreamError
    // Up to `max` bytes, undecoded. May answer fewer before the end; an empty
    // answer means the end.
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

    // Accepts text, answering how many bytes of it were written: none once
    // a failure is recorded.
    fn raw_write(handle: Long, text: Str) -> Long => text
    // Accepts bytes as they are, answering how many were written: none once
    // a failure is recorded.
    fn raw_write_bytes(handle: Long, data: Bytes) -> Long => data
    // Bytes accepted so far.
    fn raw_write_position(handle: Long) -> Long
    // Pushes accepted bytes on, reporting a recorded failure.
    fn raw_flush(handle: Long) -> Ok None | Err StreamError
    // Flushes and releases the write handle.
    fn raw_close_write(handle: Long) -> Ok None | Err StreamError

    // Reads the next bytes on a thread of the host's own and answers on
    // [reply]: the bytes (never empty), or `End`, or a failure. At `End` or a
    // failure the host has already released the handle, so there is nothing
    // left to close.
    // @compilerrules [stream-receive, platform-reply]
    fn raw_receive(handle: Long, reply: Reply<Ok Bytes | End | Err StreamError>) -> None => !reply
    // Registers a readable stream over [data] in the process's table,
    // answering its handle.
    // @compilerrules [stream-from-bytes]
    fn raw_from_bytes(data: Bytes) -> Long => !data
}

// The host's answer to a receive, turned into a `Received`: the token the
// request gave up is minted again around the same handle, so it travels back
// to the caller inside the packet. At `End` or a failure the host released
// the handle, so no token is minted and nothing is owed.
// @compilerrules [stream-receive]
send fn host_received(reply: Reply<Received>, handle: Long, got: Ok Bytes | End | Err StreamError)
=> !reply, !handle, !got {
    when got {
        is Ok {
            reply.send(ok(Packet { bytes: got, stream: InStream { handle: handle } }))
        }
        is End {
            reply.send(got)
        }
        is Err {
            reply.send(err(checked<StreamError>(got)))
        }
    }
}

// The host's [RawStreams]: the process's stream table, read and written.
//
// Each member checks the stream's entry out of the table, works on it with
// no lock held, and checks it back in, so operations on different streams
// never wait for each other. A line is split out of a read-ahead buffer,
// which later reads take from first; text is decoded strictly as UTF-8.
// The handler holds no state of its own (the table is the runtime's), so
// it is shared by every thread without a lock.
//
// [raw_receive] starts a host thread for each call. Bind [DefaultStreams]
// after this one to get [Streams]; see the module's example.
// @compilerrules [stream-table]
export handler HostRawStreams for RawStreams {
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

    fn raw_receive(handle: Long, reply: Reply<Ok Bytes | End | Err StreamError>) -> None => !reply {
        // The read runs on a thread of the runtime's, and its chunk comes
        // back as a task that turns it into this protocol's answer
        // [stream-receive].
        receive(handle, replyto chunk_received(reply))
    }

    fn raw_from_bytes(data: Bytes) -> Long => !data {
        return register_bytes(data)
    }
}

// A chunk from the runtime's reader, as [raw_receive]'s answer.
// @compilerrules [stream-receive]
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

// The host's [Streams]: the stream tokens and checked errors, over whatever
// [RawStreams] is bound.
//
// Each member takes the handle out of the token, calls the matching
// [RawStreams] member, and wraps a failure in `Checked<StreamError>`. The
// `close` members and `receive` consume the token, which is what
// discharges it. Bind
// [HostRawStreams] before this one.
//
// ```
// import std.stream
// import std.stream.host
//
// fn main() [use] -> None {
//     use StdConsole()
//     use HostRawStreams()
//     use DefaultStreams()
//     let s = from_bytes(to_bytes("hello"))
//     let got = waitfor r: Reply<Received> {
//         receive(s, r)                                // read on a host thread
//     }
//     when got {
//         is Ok {
//             let {bytes, stream} = got
//             println("${size(bytes)} bytes")          // 5 bytes
//             let closed = close(stream)
//             if closed is Err {
//                 ignore(closed)
//             }
//         }
//         is End { println("empty") }
//         is Err { ignore(got) }
//     }
// }
// ```
export handler DefaultStreams [RawStreams] for Streams {
    fn read_line(s: InStream) -> Str | None => s {
        return raw_read_line(s.handle)
    }

    fn read_all(s: InStream) -> Ok Str | Err Checked<StreamError> => s {
        let r = raw_read_all(s.handle)
        when r {
            is Ok {
                return ok(r)
            }
            is Err {
                return err(checked<StreamError>(r))
            }
        }
    }

    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err Checked<StreamError> => s {
        let r = raw_read_bytes(s.handle, max)
        when r {
            is Ok {
                return ok(r)
            }
            is Err {
                return err(checked<StreamError>(r))
            }
        }
    }

    fn read_to(s: InStream, buf: Mut Bytes, max: Int) -> Ok Int | Err Checked<StreamError> => s, buf: Mut {
        let r = raw_read_to_bytes(s.handle, buf, max)
        when r {
            is Ok {
                return ok(r)
            }
            is Err {
                return err(checked<StreamError>(r))
            }
        }
    }

    fn read_to(s: InStream, buf: Mut Str) -> Ok Long | Err Checked<StreamError> => s, buf: Mut {
        let r = raw_read_to_str(s.handle, buf)
        when r {
            is Ok {
                return ok(r)
            }
            is Err {
                return err(checked<StreamError>(r))
            }
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
        when r {
            is Ok {
                return ok(None)
            }
            is Err {
                return err(checked<StreamError>(r))
            }
        }
    }

    fn receive(s: InStream, reply: Reply<Received>) -> None => !s, !reply {
        // The token is given up here (`receive` consumes it, so this is a
        // discharge context) and the handle alone goes to the host, which
        // completes the continuation from its own thread
        // [stream-receive] [linear-group].
        let handle = copy(s.handle)
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
            is Ok {
                return ok(None)
            }
            is Err {
                return err(checked<StreamError>(r))
            }
        }
    }

    fn close(s: OutStream) -> Ok None | Err Checked<StreamError> => !s {
        let r = raw_close_write(s.handle)
        when r {
            is Ok {
                return ok(None)
            }
            is Err {
                return err(checked<StreamError>(r))
            }
        }
    }
}
