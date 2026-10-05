// Streams: the byte streams every producer in a program hands out — files,
// network bodies, buffers — and the one effect that reads and writes all of
// them.
//
// The layering (user decisions 2026-09-29, the stream layering):
//
//   * `InStream`/`OutStream` are linear tokens; `Streams` is the effect with
//     every operation on them, declared here beside the types, so every
//     consuming member is a discharger by the same-file rule [linear-group].
//   * Producers *mint into* the `Streams` in scope and do not read: `fs`
//     opens paths into streams (`effect Fs [Streams]` [effect-prereq]), a
//     network client hands out bodies the same way. One stream type, read
//     the same way whoever opened it.
//   * The host's implementation is in `stream.host` (`RawStreams`,
//     `HostRawStreams`, `DefaultStreams`), the `fs.host` shape: plain handles
//     below, obligations above. The in-memory world is `fs.mem`'s `MemFs`,
//     which wears `Fs` and `Streams` both.
//
// A handle belongs to the table that minted it. Handing a stream from one
// provider to another (a `MemFs` stream to the host's `Streams`) is a program
// bug, and it traps [stream-provider]; every table draws from one counter
// [stream-handle], so it can never be mistaken for another live stream.
//
// Not part of `core`, so it arrives by asking: `import stream`.

import runtime.streams

// [stream-handle] A handle for a new stream, unique in this process: every
// stream table — the host's and the in-memory ones alike — draws from this one
// counter, so a handle handed to the wrong table is *unknown* there, never
// another stream's. The counter is the runtime's stream table's.
export fn fresh_handle() [] -> Long {
    return fresh_handle@runtime.streams()
}

// ===== errors =====

// What went wrong reading or writing a stream: droppable, storable, and what
// `detach` hands back. Every failing member answers `Err Checked<StreamError>`
// [checked-type]. [source] names the stream — a path for a file, a
// description for anything else — since a stream need not have a path.
export type StreamError = InvalidUtf8 | StreamFailed

// The bytes read are not valid UTF-8. Decoding is strict on both backends.
export struct InvalidUtf8 { source: Str }
// Anything else the provider reported: an I/O error, a dropped connection.
export struct StreamFailed { source: Str, message: Str }

// The text of a failure [interp-to-str].
export fn to_str(kind: StreamError) [] -> Str => kind {
    when kind {
        is InvalidUtf8 { return "not valid UTF-8: ${kind.source}" }
        is StreamFailed { return "stream failed: ${kind.source}: ${kind.message}" }
    }
}

// ===== the tokens =====

// A stream open for reading. An opaque token: the provider behind it owns the
// position, the buffer and the resource, so the only thing here is which
// stream this is. Linear, so a stream that is never closed is a compile error
// rather than a leak [linear-group]; `close` is the discharger.
// [noremote] A stream is a handle into *this* process's stream table, so it
// has no wire form — and neither does anything holding one.
export noremote linear struct InStream { handle: Long }

// A stream open for writing. Its `close` flushes.
export noremote linear struct OutStream { handle: Long }

// ===== a non-blocking read's answer =====

// One non-blocking read's bytes, and the stream back: a read consumes the
// stream and its answer returns it, so exactly one read is ever in flight and
// a `close` mid-read cannot be written — the checker cannot see a pending
// reply, so the token's absence is what makes the rule hold. Linear by
// containment: the stream still has to be closed. [bytes] is never empty.
export linear struct Packet { bytes: Bytes, stream: InStream }

// What `receive` answers: bytes and the stream back, or the end — the provider
// has closed the stream, so nothing is left to close — or a failure, likewise
// closed.
export type Received = Ok Packet | End | Err Checked<StreamError>

// The stream has ended. The provider closed it, so there is no token here.
export struct End {}

// Gives up a packet's stream before its end: `close` on the stream inside,
// the bytes dropped. The usual way to consume a `Packet` is to take it apart —
// `let {bytes, stream} = packet` — and carry on reading `stream`.
export fn close(p: Packet) [Streams] -> Ok None | Err Checked<StreamError> => !p {
    let {bytes, stream} = p
    return close(stream)
}

// ===== the effect =====

// Every operation on a stream. Producers (`Fs`, a network client) mint the
// tokens; this reads and writes them, whoever minted them.
//
// Every failure is a returned `Err Checked<StreamError>`: an effect member may
// declare no effects of its own.
export effect Streams {
    // The next line, without its terminator (`\n` and `\r\n` both end a
    // line, neither is returned). Absent means the stream ended — or that a
    // read failed, which `close` reports: iteration stops either way, and
    // one error surfaces in one place.
    fn read_line(s: InStream) -> Str | None => s
    // Everything left in the stream, decoded strictly.
    fn read_all(s: InStream) -> Ok Str | Err Checked<StreamError> => s
    // Up to [max] bytes, exactly as they lie in the stream: no decoding, so a
    // read may stop in the middle of a character and data that is not text is
    // read all the same. Fewer bytes than asked for means the stream ended; an
    // empty buffer means it has ended already [fs-bytes].
    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err Checked<StreamError> => s
    // Up to [max] bytes **appended to [buf]**, answering how many. The
    // fill-a-buffer read [fs-read-to]: one buffer, `clear`ed and refilled for
    // as long as a loop runs, instead of a fresh payload per chunk.
    fn read_to(s: InStream, buf: Mut Bytes, max: Int) -> Ok Int | Err Checked<StreamError> => s, buf: Mut
    // Everything left, decoded strictly and **appended to [buf]**, answering
    // how many bytes were consumed.
    fn read_to(s: InStream, buf: Mut Str) -> Ok Long | Err Checked<StreamError> => s, buf: Mut
    // The next line, without its terminator, **appended to [buf]**; `false`
    // means the stream ended — or that a read failed, which `close` reports.
    fn read_line_to(s: InStream, buf: Mut Str) -> Bool => s, buf: Mut
    // Bytes consumed so far. For a file, the offset a later `open_read_at`
    // would use to resume here. Not a seek.
    fn position(s: InStream) -> Long => s
    // Releases the stream, reporting a failure recorded during reading.
    fn close(s: InStream) -> Ok None | Err Checked<StreamError> => !s

    // Writes text, answering how many bytes it took. A failed write is
    // recorded and surfaces at `flush` or `close`, so writing in a loop does
    // not narrow a result per line.
    fn write(s: OutStream, text: Str) -> Long => s, text
    // Writes text and a `\n`, answering how many bytes both took.
    fn write_line(s: OutStream, text: Str) -> Long => s, text
    // Writes bytes as they are, answering how many were accepted.
    fn write_bytes(s: OutStream, data: Bytes) -> Long => s, data
    // Bytes accepted so far.
    fn position(s: OutStream) -> Long => s
    // Pushes accepted bytes on, reporting a recorded failure.
    fn flush(s: OutStream) -> Ok None | Err Checked<StreamError> => s
    // Flushes and releases the stream.
    fn close(s: OutStream) -> Ok None | Err Checked<StreamError> => !s

    // [stream-receive] The next bytes, **without blocking**: hands the stream
    // and a continuation to the provider and returns at once. The answer
    // arrives on [reply] — the bytes and the stream back, or `End`, or a
    // failure (both of which have closed the stream). A host provider reads on
    // a thread of its own and completes the reply from there [platform-reply];
    // an in-memory one answers straight away.
    fn receive(s: InStream, reply: Reply<Received>) -> None => !s, !reply

    // [stream-from-bytes] A stream over bytes already in hand — a request
    // body, a test's fixture. Minted in this `Streams`' own table, so it is
    // read like any other stream here, and only here [stream-provider].
    fn from_bytes(data: Bytes) -> InStream => !data
}

// ===== copying without blocking =====

// [stream-pipe] Copies everything left in [from] into [to] **without blocking a
// worker**, answering on [done] how many bytes moved. Each chunk is a
// `receive`; the copy runs as a chain of continuations on the pool this was
// called on, so a slow producer (a network body) occupies no thread while it
// is slow. Both streams are closed when the copy ends, whatever happened, and
// the first failure is the one reported. The synchronous shape is
// [copy_stream].
export fn pipe(from: InStream, to: OutStream, done: Reply<Ok Long | Err Checked<StreamError>>) [Streams] -> None
=> !from, !to, !done {
    receive(from, replyto pipe_step(to, done, 0))
}

// One step of [pipe]: write what arrived and ask for more, or finish.
send fn pipe_step(to: OutStream, done: Reply<Ok Long | Err Checked<StreamError>>, moved: Long, got: Received) [Streams]
=> !to, !done, !moved, !got {
    when got {
        is Ok {
            let {bytes, stream} = got
            let written = write_bytes(to, bytes)
            receive(stream, replyto pipe_step(to, done, moved + written))
        }
        is End {
            let closed = close(to)
            when closed {
                is Ok { done.send(ok(moved)) }
                is Err { done.send(closed) }
            }
        }
        is Err {
            let closed = close(to)
            when closed {
                is Ok {}
                is Err { ignore(closed) }
            }
            done.send(got)
        }
    }
}

// ===== reading lines as a sequence =====

// An iterator over a stream's lines: `for line in p` drives it. It holds the
// stream, so it carries the obligation too — bind it, loop over it, and
// discharge it when you are done [linear-group].
export linear struct Lines : Yield<self, Str> canbe Mut {
    s: InStream
}

// Reads `s` as a sequence of lines. The stream's obligation moves into the
// iterator, which is why closing the iterator is closing the stream.
export fn lines(s: InStream) [] -> Mut Lines => !s {
    return Mut Lines { s: s }
}

// The iterator's step: a line, or the end of the sequence. Declares
// `[Streams]`, since reading is an effect — so `for`, `map`, `filter` and
// `reduce` inherit it at the call site.
export fn next(p: Mut Lines) [Streams] -> Emitted Str | Finished => p: Mut {
    let line = read_line(p.s)
    when line {
        is Str { return emitted(line) }
        is None { return finished() }
    }
}

// Closes the stream the iterator reads, reporting what reading recorded. Named
// `close` like every other discharger: a member and a fn of one name are one
// overload set, and the argument type picks [effect-available].
export fn close(p: Lines) [Streams] -> Ok None | Err Checked<StreamError> => !p {
    return close(p.s)
}

// ===== reading bytes as a sequence =====

// An iterator over a stream's chunks: `for chunk in c` drives it, and each step
// is up to [size] bytes. Like [Lines] it holds the stream, so it carries the
// obligation too [linear-group].
export linear struct Chunks : Yield<self, Bytes> canbe Mut {
    s: InStream,
    // How many bytes a step asks for.
    size: Int
}

// Reads `s` as a sequence of chunks of up to [size] bytes each.
export fn chunks(s: InStream, size: Int) [] -> Mut Chunks => !s {
    return Mut Chunks { s: s, size: size }
}

// The iterator's step. Each chunk is a **fresh** buffer, deliberately: an
// iterator that handed back its own buffer would have the next step overwrite
// what the caller is still holding. The allocation-free shape is `read_to`
// into a buffer of your own — which is what [copy_stream] does [fs-read-to].
export fn next(p: Mut Chunks) [Streams] -> Emitted Bytes | Finished => p: Mut {
    let got = read_bytes(p.s, p.size)
    if got is Err {
        // The provider recorded the failure, and `close` reports it — so
        // iteration ends here rather than losing the reason [fs-errors-at-close].
        ignore(got)
        return finished()
    }
    let data: Bytes = got
    if size(data) == 0 {
        return finished()
    }
    return emitted(data)
}

// Closes the stream the iterator reads, reporting what reading recorded.
export fn close(p: Chunks) [Streams] -> Ok None | Err Checked<StreamError> => !p {
    return close(p.s)
}

// ===== copying, with the buffer kept out of sight =====

// How many bytes a copy moves at a time. Big enough that the call is not the
// cost, small enough to be nobody's memory problem.
fn stream_chunk_size() [] -> Int {
    return 65536
}

// Appends everything left in [s] to [buf], answering how many bytes moved.
// One buffer for the whole read: this is `read_to` in a loop, which is the
// point of `read_to` existing [fs-read-to].
export fn fill_from(s: InStream, buf: Mut Bytes) [Streams] -> Ok Long | Err Checked<StreamError> => s, buf: Mut {
    let total: Long = 0
    let reading = true
    while reading {
        let got = read_to(s, buf, stream_chunk_size())
        if got is Err {
            return got
        }
        let n: Int = got
        total = total + to_long(n)
        if n == 0 {
            reading = false
        }
    }
    return ok(total)
}

// Copies everything left in [s] into [w], answering how many bytes moved.
// Neither token is consumed: whoever opened them closes them. Synchronous — it
// blocks the worker it runs on for as long as the copy takes.
export fn copy_stream(s: InStream, w: OutStream) [Streams] -> Ok Long | Err Checked<StreamError> => s, w {
    let buf = mut_bytes()
    let total: Long = 0
    let copying = true
    while copying {
        clear(buf)
        let got = read_to(s, buf, stream_chunk_size())
        if got is Err {
            return got
        }
        let n: Int = got
        if n == 0 {
            copying = false
        } else {
            total = total + write_bytes(w, buf)
        }
    }
    return ok(total)
}

// ===== numbers and values [stream-values] =====
//
// Fixed-width numbers, big-endian and two's complement, and whole values in
// Salvo's canonical encoding behind a length (user decision 2026-10-04): what
// a file of records is written and read back with. Ordinary Salvo over
// `write_bytes` and `read_bytes`, so a fake `Streams` reads and writes them
// too. A read answers `End` when the stream ends before the first byte, and
// an error when it ends inside a number or a value, or the bytes do not
// decode as the value asked for.

// [stream-values] Writes [v] as 4 bytes, big-endian; answers the bytes
// written.
export fn write_int(s: OutStream, v: Int) [Streams] -> Long => s {
    return write_bytes(s, fixed_bytes(to_long(v), 4))
}

// [stream-values] Writes [v] as 8 bytes, big-endian; answers the bytes
// written.
export fn write_long(s: OutStream, v: Long) [Streams] -> Long => s {
    return write_bytes(s, fixed_bytes(v, 8))
}

// [stream-values] Writes [v] in the canonical encoding [wire-encoding],
// behind its length as a 4-byte `Int`, so [read_value] knows where it ends;
// answers the bytes written. The codec is an implicit, `codec`'s `encode` at
// the caller's concrete type: a wire form is the instantiation's, so a
// generic body cannot name it [noremote].
export fn write_value<T>(s: OutStream, v: T, ?encode: (v: T) -> Bytes) [Streams] -> Long
=>[encode] !v => s, !v {
    let data = encode(v)
    let n = write_int(s, size(data))
    return n + write_bytes(s, data)
}

// [stream-values] Reads a 4-byte big-endian `Int`.
export fn read_int(s: InStream) [Streams] -> Ok Int | End | Err Checked<StreamError> => s {
    let r = read_fixed(s, 4)
    when r {
        is Ok { return ok(to_int(r)) }
        is End { return End {} }
        is Err { return r }
    }
}

// [stream-values] Reads an 8-byte big-endian `Long`.
export fn read_long(s: InStream) [Streams] -> Ok Long | End | Err Checked<StreamError> => s {
    return read_fixed(s, 8)
}

// [stream-values] Reads a value [write_value] wrote: its length, then that
// many bytes, decoded as a `T` by the implicit `decode` (`codec`'s, at the
// caller's type).
export fn read_value<T>(s: InStream, ?decode: (data: Bytes) -> T?) [Streams] -> Ok T | End | Err Checked<StreamError>
=> s {
    let len = read_int(s)
    if len is End {
        return End {}
    }
    if len is Err {
        return len
    }
    let n: Int = len
    let data = read_bytes(s, n)
    if data is Err {
        return data
    }
    let bytes: Bytes = data
    if size(bytes) < n {
        return err(checked<StreamError>(StreamFailed { source: "read_value", message: "the stream ended inside a value" }))
    }
    let v = decode(bytes)
    if v is None {
        return err(checked<StreamError>(StreamFailed { source: "read_value", message: "the bytes are not a value of the type asked for" }))
    }
    return ok(v)
}

// [v] as [width] big-endian bytes, two's complement: the low byte is the
// floor remainder by 256, and the rest is the exact quotient, which is an
// arithmetic shift for a negative number too.
fn fixed_bytes(v: Long, width: Int) [] -> Bytes => v, width {
    let out = mut_bytes()
    let rest = copy(v)
    let i = 0
    while i < width {
        let low = rest % 256L
        if low < 0L {
            low = low + 256L
        }
        add(out, to_byte(to_int(low)))
        rest = (rest - low) / 256L
        i = i + 1
    }
    let back = mut_bytes()
    let j = width - 1
    while j >= 0 {
        add(back, get(out, j)!)
        j = j - 1
    }
    return back
}

// [width] big-endian bytes as a number: the first byte carries the sign.
fn read_fixed(s: InStream, width: Int) [Streams] -> Ok Long | End | Err Checked<StreamError> => s, width {
    let r = read_bytes(s, width)
    if r is Err {
        return r
    }
    let data: Bytes = r
    if size(data) == 0 {
        return End {}
    }
    if size(data) < width {
        return err(checked<StreamError>(StreamFailed { source: "read", message: "the stream ended inside a number" }))
    }
    let first = to_long(to_int(get(data, 0)!))
    if first >= 128L {
        first = first - 256L
    }
    let v = first
    let i = 1
    while i < width {
        v = v * 256L + to_long(to_int(get(data, i)!))
        i = i + 1
    }
    return ok(v)
}
