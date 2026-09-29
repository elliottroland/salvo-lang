// The host's streams: the raw seam over the process-wide stream table, and the
// handler that turns it into `stream`'s `Streams` — the `fs.host` shape
// (`RawFs`, `HostRawFs`, `DefaultFs`) for streams.
//
// Its own module, so a program that fakes the world (`MemFs`) never reaches a
// host class.

import stream

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
}

// The one implementation of `RawStreams`: a host class per backend, shipped
// with std [platform-handler]. `threadsafe` [threadsafe-platform]: the table
// is the whole process's, locked per stream by the host, so the compiler
// shares the instance with no lock of its own.
export threadsafe platform handler HostRawStreams of RawStreams

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
