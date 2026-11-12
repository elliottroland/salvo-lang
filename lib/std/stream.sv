// Byte streams: the files, network bodies and in-memory buffers a program
// reads and writes, and [Streams], the one effect that reads and writes all
// of them.
//
// A stream is a **token**: an [InStream] open for reading or an [OutStream]
// open for writing. Code that has one reads or writes it through the
// members of [Streams], whoever opened it. Producers hand tokens out: `fs`
// opens a path into one (`open_read`, `open_write`), a network client hands
// out a response body, and [Streams]' own `from_bytes` makes one over bytes
// already in hand. A function that reads a stream declares `[Streams]`; the
// program's composition root decides what implements it: the machine's
// streams ([std.stream.host]), or [std.fs.mem]'s [MemFs], an in-memory
// world for tests.
//
// ```
// import std.stream
// import std.fs.mem
//
// fn main() [use] -> None {
//     use StdConsole()
//     use MemFs()                                      // streams in memory
//     let s = from_bytes(to_bytes("title\nline one\nline two\n"))
//     let title = read_line(s) ?: "untitled"           // "title"
//     let body = read_all(s)                           // the rest, as text
//     let closed = close(s)
//     when body {
//         is Ok { println("${title}: ${body}") }
//         is Err { println(to_str(detach(body))) }
//     }
//     if closed is Err {
//         println(to_str(detach(closed)))
//     }
// }
// ```
//
// **A stream must be closed.** [InStream] and [OutStream] are linear: a
// path on which a stream is neither closed nor handed on does not compile,
// so a stream cannot leak. `close` answers `Ok None | Err
// Checked<StreamError>`, and a `Checked` error must be looked at too:
// `detach(e)` hands back the [StreamError], and `ignore(e)` says it does not
// matter. An iterator over a stream ([lines], [chunks]) holds the stream, so
// closing the iterator closes the stream.
//
// **Failures are recorded, and reported at the close.** A read that fails
// reports the end of the stream (`read_line` answers `None`), and a write
// that fails still answers a byte count (0 on the host), so a loop does not
// check a result per line. The failure is recorded, and `flush` or `close` reports
// it. The members that answer data (`read_all`, `read_bytes`, `read_to`)
// answer a failure directly as well.
//
// **Text and bytes share one stream.** Text is UTF-8, decoded strictly: bytes
// that are not valid UTF-8 are an [InvalidUtf8] failure, never replacement
// characters. Text and byte reads take from the same position, counted in
// bytes, so a text read continues exactly where a byte read stopped.
// `position` reports how many bytes have been read or written; streams only
// go forward, and there is no seek.
//
// **Writes may be buffered.** The host's file streams buffer what is
// written, so the bytes reach the file at `flush`, or at `close`, which
// flushes. A write failure can therefore surface late, which is one more
// reason `close` answers one.
//
// **Most reads block; `receive` does not.** Every member of [Streams] but
// `receive` runs to completion on the calling thread, so a read from a slow
// source occupies that thread until data arrives. `receive` hands the
// stream to the provider and returns at once; the next chunk arrives later
// on a reply, together with the stream, so exactly one read of a stream is
// in flight at a time. [pipe] copies a stream that way: it asks for the
// next chunk only once the last one is written, so a slow writer slows the
// reader rather than piling up chunks in memory.
//
// **A stream belongs to its provider.** Each implementation of [Streams]
// keeps its own table of streams, and a token is a handle into one of them.
// Handing a stream from one provider to another (a [MemFs] stream to the
// host's [Streams]) is a program bug, and it traps. Every table draws its
// handles from one counter ([fresh_handle]), so such a handle is never
// mistaken for another live stream. Use the host's streams or [MemFs]
// throughout, never a mix.
//
// Streams are not part of `std.core`: a program that uses them writes
// `import std.stream`.
// @compilerrules [linear-group, effect-prereq, stream-provider, stream-handle, fs-errors-at-close]

import std.runtime.streams

// A handle for a new stream, unique in this process.
//
// For an implementation of [Streams], which mints a token around each
// handle it registers. Every stream table, the host's and the in-memory ones
// alike, draws from this one counter, so a handle handed to the wrong table
// is unknown there, never another stream's.
//
// ```
// let a = fresh_handle()
// let b = fresh_handle()
// println("${b > a}")                                  // true
// ```
// @compilerrules [stream-handle]
export fn fresh_handle() [] -> Long {
    return fresh_handle@runtime.streams()
}

// ===== errors =====

// What went wrong reading or writing a stream.
//
// Every member of [Streams] that can fail answers `Err
// Checked<StreamError>`, which must be looked at; `detach` hands back this
// plain value, which may be stored or dropped like any other. Each kind
// names its stream by `source`: a path for a file, a description for
// anything else, since a stream need not have a path.
//
// ```
// let e: StreamError = StreamFailed { source: "upload", message: "connection reset" }
// println(to_str(e))                       // stream failed: upload: connection reset
// ```
// @compilerrules [type-checked]
export type StreamError = InvalidUtf8 | StreamFailed

// The bytes read are not valid UTF-8; `source` is the stream they were read
// from. Decoding is strict on both backends.
export struct InvalidUtf8 {
    source: Str
}

// Anything else the provider reported, such as an I/O error or a dropped
// connection: `source` is the stream, `message` the provider's account.
export struct StreamFailed {
    source: Str
    message: Str
}

// The text of a failure, naming its stream: what `${e}` renders.
// @compilerrules [interp-to-str]
export fn to_str(kind: StreamError) [] -> Str => kind {
    when kind {
        is InvalidUtf8 {
            return "not valid UTF-8: ${kind.source}"
        }
        is StreamFailed {
            return "stream failed: ${kind.source}: ${kind.message}"
        }
    }
}

// ===== the tokens =====

// A stream open for reading.
//
// A token: the provider that minted it keeps the position, the buffer and
// the resource, and the token's `handle` says only which stream it is. That is why no
// read needs a `Mut` stream. It is **linear**, so a stream that is never
// closed is a compile error rather than a leak; `close` (a [Streams]
// member) is what discharges it.
//
// A token is a handle into this process's stream table, so it cannot be
// sent to another process, and neither can anything holding one.
// @compilerrules [linear-group, net-noremote]
export noremote linear struct InStream {
    handle: Long
}

// A stream open for writing.
//
// Linear like [InStream]; its `close` flushes what was written and then
// releases the stream.
// @compilerrules [linear-group, net-noremote]
export noremote linear struct OutStream {
    handle: Long
}

// ===== a non-blocking read's answer =====

// One chunk a `receive` read, and the stream back.
//
// `receive` consumes the stream and this hands it back, so exactly one read
// of a stream is in flight at a time, and a `close` in the middle of one
// cannot be written: the token is not there to close. `bytes` is never
// empty. A packet is linear because its `stream` is, so the stream still
// has to be closed. The usual way to use one is to take it apart and read
// on:
//
// ```
// import std.stream
//
// // Counts a stream's bytes without blocking a worker: each chunk's
// // answer asks for the next.
// send fn count_from(total: Long, done: Reply<Long>, got: Received) [Streams] => !done, !got {
//     when got {
//         is Ok {
//             let {bytes, stream} = got
//             receive(stream, replyto count_from(total + to_long(size(bytes)), done))
//         }
//         is End { done.send(total) }                  // closed already
//         is Err {
//             ignore(got)
//             done.send(-1L)
//         }
//     }
// }
// ```
export linear struct Packet {
    bytes: Bytes
    stream: InStream
}

// What `receive` answers: a [Packet] with the bytes and the stream back,
// or [End], or a failure. At the end or on a failure the provider has
// closed the stream already, so nothing is left to close.
export type Received = Ok Packet | End | Err Checked<StreamError>

// The stream has ended. The provider closed it, so there is no token here.
export struct End {}

// Closes a packet's stream before its end, dropping the bytes.
export fn close(p: Packet) [Streams] -> Ok None | Err Checked<StreamError> => !p {
    let { bytes, stream } = p
    return close(stream)
}

// ===== the effect =====

// Every operation on a stream: reading, writing, closing, and reading
// without blocking.
//
// Producers (`Fs`, a network client) mint the tokens; this reads and writes
// them, whoever minted them. A function that works on streams declares
// `[Streams]`, and the composition root binds an implementation:
// [std.stream.host]'s `DefaultStreams` over `HostRawStreams` for the
// machine's streams, or [MemFs] for streams in memory. `Fs` declares this
// effect as its prerequisite, so code that declares `[Fs]` can use these
// members too.
//
// Every member but `receive` completes before it returns, blocking the
// calling thread while it waits for the source. Every failure is a returned
// `Err Checked<StreamError>`, since an effect member may declare no effects
// of its own (so none can throw).
//
// ```
// // Writes a report, line by line; a failed write surfaces at the close.
// fn write_report(out: OutStream, items: List<Str>) [Streams] -> Ok Long | Err Checked<StreamError>
// => !out, items {
//     let total: Long = 0
//     for item in items {
//         total = total + write_line(out, item)
//     }
//     let closed = close(out)
//     if closed is Err {
//         return closed
//     }
//     return ok(total)
// }
// ```
// @compilerrules [effect-prereq, fs-errors-at-close]
export effect Streams {
    // The next line, without its terminator (`\n` and `\r\n` both end a
    // line, neither is returned). The last line need not end in one.
    // `None` means the stream ended, or that a read failed, which `close`
    // reports: a loop stops either way, and the error surfaces in one place.
    fn read_line(s: InStream) -> Str | None => s
    // Everything left in the stream, decoded strictly.
    fn read_all(s: InStream) -> Ok Str | Err Checked<StreamError> => s
    // Up to `max` bytes, exactly as they lie in the stream. Nothing is
    // decoded, so a read may stop in the middle of a character, and data
    // that is not text is read all the same. A read may answer fewer bytes
    // than asked for before the end (the host answers what one read of its
    // source gave); an empty answer means the stream has ended.
    // @compilerrules [fs-bytes]
    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err Checked<StreamError> => s
    // Up to `max` bytes **appended to `buf`**, answering how many; 0 means
    // the stream has ended. The fill-a-buffer read: one buffer, `clear`ed and
    // refilled for as long as a loop runs, instead of a fresh payload per
    // chunk. Like `read_bytes`, it may answer fewer than `max` before the
    // end.
    // @compilerrules [fs-read-to]
    fn read_to(s: InStream, buf: Mut Bytes, max: Int) -> Ok Int | Err Checked<StreamError> => s, buf: Mut
    // Everything left, decoded strictly and **appended to [buf]**, answering
    // how many bytes were consumed.
    fn read_to(s: InStream, buf: Mut Str) -> Ok Long | Err Checked<StreamError> => s, buf: Mut
    // The next line, without its terminator, **appended to [buf]**; `false`
    // means the stream ended — or that a read failed, which `close` reports.
    fn read_line_to(s: InStream, buf: Mut Str) -> Bool => s, buf: Mut
    // Bytes read so far, counted in bytes whether they were read as text or
    // as bytes. For a file, the offset a later `open_read_at` would use to
    // resume here. Reading it does not move the stream.
    fn position(s: InStream) -> Long => s
    // Releases the stream, reporting a failure recorded while reading it.
    // The token is consumed either way.
    fn close(s: InStream) -> Ok None | Err Checked<StreamError> => !s

    // Writes text as UTF-8, answering how many bytes it took. A failed write
    // is recorded and surfaces at `flush` or `close`, so a loop of writes
    // does not check a result per line; on the host, a write after a
    // recorded failure writes nothing and answers 0.
    fn write(s: OutStream, text: Str) -> Long => s, text
    // Writes text and a `\n`, answering how many bytes both took.
    fn write_line(s: OutStream, text: Str) -> Long => s, text
    // Writes bytes as they are, answering how many were accepted.
    fn write_bytes(s: OutStream, data: Bytes) -> Long => s, data
    // Bytes accepted so far.
    fn position(s: OutStream) -> Long => s
    // Pushes the bytes written so far on to their destination (a buffered
    // file's bytes to the file), reporting a recorded failure.
    fn flush(s: OutStream) -> Ok None | Err Checked<StreamError> => s
    // Flushes and releases the stream, reporting a failure recorded while
    // writing, flushing or closing it. The token is consumed either way.
    fn close(s: OutStream) -> Ok None | Err Checked<StreamError> => !s

    // The next bytes, **without blocking**: hands the stream and a reply to
    // the provider and returns at once. The answer arrives on [reply] as a
    // later activation, never inside this call: a [Packet] with the bytes
    // (up to 64 KiB, never empty) and the stream back, or `End`, or a
    // failure; both of the last two have closed the stream. The host reads
    // on a thread of its own, started for the call, and answers from there;
    // [MemFs] answers straight away. In an actor test, use [MemFs]: a host
    // thread would stop the test.
    // @compilerrules [stream-receive, platform-reply, test-actor]
    fn receive(s: InStream, reply: Reply<Received>) -> None => !s, !reply

    // A stream over bytes already in hand: a request body, a test's
    // fixture. Minted in this `Streams`' own table, so it is read like any
    // other stream here, and only here.
    // @compilerrules [stream-from-bytes, stream-provider]
    fn from_bytes(data: Bytes) -> InStream => !data
}

// ===== copying without blocking =====

// Copies everything left in one stream into another **without blocking a
// worker**, answering on a reply how many bytes moved.
//
// Returns at once. Each chunk is a `receive`, and the copy runs as a chain
// of continuations on the pool this was called on, so a slow producer (a
// network body) occupies no thread while it is slow. The next chunk is
// asked for only once the last one is written. Both streams are closed when
// the copy ends, whatever happened, and the first failure is the one
// reported. The blocking form is [copy_stream].
//
// ```
// fn copy_into(from: InStream, to: OutStream) [Streams] -> Ok Long | Err Checked<StreamError>
// => !from, !to {
//     return waitfor done: Reply<Ok Long | Err Checked<StreamError>> {
//         pipe(from, to, done)
//     }
// }
// ```
//
// @param from  the stream to read; closed when the copy ends
// @param to    the stream to write; closed when the copy ends
// @param done  answered once, with the bytes moved or the first failure
// @compilerrules [stream-pipe]
export fn pipe(from: InStream, to: OutStream, done: Reply<Ok Long | Err Checked<StreamError>>) [Streams] -> None
=> !from, !to, !done {
    receive(from, replyto pipe_step(to, done, 0))
}

// One step of [pipe]: write what arrived and ask for more, or finish.
send fn pipe_step(to: OutStream, done: Reply<Ok Long | Err Checked<StreamError>>, moved: Long, got: Received) [Streams]
=> !to, !done, !moved, !got {
    when got {
        is Ok {
            let { bytes, stream } = got
            let written = write_bytes(to, bytes)
            receive(stream, replyto pipe_step(to, done, moved + written))
        }
        is End {
            let closed = close(to)
            when closed {
                is Ok {
                    done.send(ok(moved))
                }
                is Err {
                    done.send(closed)
                }
            }
        }
        is Err {
            let closed = close(to)
            when closed {
                is Ok {}
                is Err {
                    ignore(closed)
                }
            }
            done.send(got)
        }
    }
}

// ===== reading lines as a sequence =====

// An iterator over a stream's lines, as [lines] answers.
//
// It holds the stream, so it is linear too: bind it, loop over it, and
// close it when you are done, which closes the stream. Each step reads one
// line, as `read_line` does; a failure ends the iteration and is reported
// by the close.
// @compilerrules [linear-group]
export linear struct Lines : Next<self, Str, [Streams]> canbe Mut {
    s: InStream
}

// An iterator over the lines of a stream, without their terminators.
//
// The stream moves into the iterator, so closing the iterator is closing
// the stream.
//
// ```
// // Counts a stream's lines, closing it; -1 if reading failed.
// fn count_lines(s: InStream) [Streams] -> Int => !s {
//     let p = lines(s)
//     let n = 0
//     for _line in p {
//         n = n + 1
//     }
//     let closed = close(p)
//     if closed is Err {
//         ignore(closed)
//         return -1
//     }
//     return n
// }
// ```
//
// @param s  the stream to read, which the iterator takes over
export fn lines(s: InStream) [] -> Mut Lines => !s {
    return Mut Lines { s: s }
}

// A [Lines] iterator's step: the next line, or the end of the sequence.
//
// Declares `[Streams]`, since reading is an effect, so a `for` loop over a
// [Lines], or [map] or [filter] over one, performs it too.
export fn next(p: Mut Lines) [Streams] -> Emitted Str | Finished => p: Mut {
    let line = read_line(p.s)
    when line {
        is Str {
            return emitted(line)
        }
        is None {
            return finished()
        }
    }
}

// Closes the stream a [Lines] iterator reads, reporting a failure recorded
// while reading it.
// @compilerrules [effect-available]
export fn close(p: Lines) [Streams] -> Ok None | Err Checked<StreamError> => !p {
    return close(p.s)
}

// ===== reading bytes as a sequence =====

// An iterator over a stream's bytes, a chunk at a time, as [chunks] answers.
//
// Like [Lines] it holds the stream, so it is linear too, and closing it
// closes the stream.
// @compilerrules [linear-group]
export linear struct Chunks : Next<self, Bytes, [Streams]> canbe Mut {
    s: InStream
    // How many bytes a step asks for.
    size: Int
}

// An iterator over the bytes of a stream, in chunks of up to [size] bytes.
//
// Each chunk is a fresh buffer, which the caller may keep. A chunk may be
// shorter than [size] before the end, as `read_bytes` may. A failure ends
// the iteration and is reported by the close.
//
// ```
// fn total_bytes(s: InStream) [Streams] -> Long => !s {
//     let c = chunks(s, 4096)
//     let total: Long = 0
//     for chunk in c {
//         total = total + to_long(size(chunk))
//     }
//     let closed = close(c)
//     if closed is Err {
//         ignore(closed)
//     }
//     return total
// }
// ```
//
// @param s     the stream to read, which the iterator takes over
// @param size  the most bytes a chunk holds
export fn chunks(s: InStream, size: Int) [] -> Mut Chunks => !s {
    return Mut Chunks { s: s, size: size }
}

// A [Chunks] iterator's step: the next chunk, or the end of the sequence.
//
// Each chunk is a fresh buffer. To read without allocating one per step,
// use `read_to` into a buffer of your own, as [copy_stream] does.
// @compilerrules [fs-read-to]
export fn next(p: Mut Chunks) [Streams] -> Emitted Bytes | Finished => p: Mut {
    // A fresh buffer, deliberately: an iterator that handed back its own
    // buffer would have the next step overwrite what the caller is still
    // holding.
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

// Closes the stream a [Chunks] iterator reads, reporting a failure
// recorded while reading it.
export fn close(p: Chunks) [Streams] -> Ok None | Err Checked<StreamError> => !p {
    return close(p.s)
}

// ===== copying, with the buffer kept out of sight =====

// How many bytes a copy moves at a time. Big enough that the call is not the
// cost, small enough to be nobody's memory problem.
fn stream_chunk_size() [] -> Int {
    return 65536
}

// Appends everything left in a stream to a buffer, answering how many bytes
// that was.
//
// `read_to` in a loop, 64 KiB at a time, into the one buffer. The stream is
// not closed: whoever opened it closes it.
//
// ```
// fn slurp(s: InStream) [Streams] -> Ok Bytes | Err Checked<StreamError> => !s {
//     let buf = mut_bytes()
//     let filled = fill_from(s, buf)
//     let closed = close(s)
//     if filled is Err {
//         if closed is Err {
//             ignore(closed)
//         }
//         return filled
//     }
//     if closed is Err {
//         return closed
//     }
//     return ok(copy(buf))
// }
// ```
//
// @param s    the stream to read to its end
// @param buf  the buffer the bytes are appended to
// @compilerrules [fs-read-to]
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

// Copies everything left in one stream into another, answering how many
// bytes moved.
//
// Reads 64 KiB at a time into one buffer and writes each chunk. Neither
// stream is closed: whoever opened them closes them. It blocks the thread it
// runs on for as long as the copy takes; [pipe] is the form that does not.
//
// @param s  the stream to read to its end
// @param w  the stream to write to
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

// ===== numbers and values =====
//
// Fixed-width numbers, big-endian and two's complement, and whole values in
// Salvo's canonical encoding behind a length: what a file of records is
// written and read back with. Ordinary Salvo over `write_bytes` and
// `read_bytes`, so a fake `Streams` reads and writes them too. A read
// answers `End` when the stream ends before the first byte, and an error
// when it ends inside a number or a value, or the bytes do not decode as
// the value asked for.
// @compilerrules [stream-values]

// Writes an `Int` as 4 bytes, big-endian, answering the bytes written.
// [read_int] reads it back.
// @compilerrules [stream-values]
export fn write_int(s: OutStream, v: Int) [Streams] -> Long => s {
    return write_bytes(s, fixed_bytes(to_long(v), 4))
}

// Writes a `Long` as 8 bytes, big-endian, answering the bytes written.
// [read_long] reads it back.
// @compilerrules [stream-values]
export fn write_long(s: OutStream, v: Long) [Streams] -> Long => s {
    return write_bytes(s, fixed_bytes(v, 8))
}

// Writes a value in Salvo's canonical encoding, behind its length, so that
// [read_value] can read it back; answers the bytes written.
//
// The encoding is the one actors use across the network, so a file written
// by a program on one backend reads back in a program on the other. Any
// value that can be sent to another process can be written. The encoder is
// found at the call site: import `std.codec.encode`, which supplies one for
// the value's type.
//
// ```
// import std.stream
// import std.codec.encode
//
// struct Point : Hashed<self> by auto {
//     x: Int,
//     y: Int
// }
//
// // A count, then the points.
// fn save_points(out: OutStream, points: List<Point>) [Streams] -> Ok None | Err Checked<StreamError>
// => !out, points {
//     let _n = write_int(out, size(points))
//     for p in points {
//         let _w = write_value(out, copy(p))
//     }
//     return close(out)
// }
// ```
//
// @param s       the stream to write to
// @param v       the value
// @param encode  found at the call site: the value's encoding
// @compilerrules [stream-values, net-noremote, net-wire-format]
export fn write_value<T>(s: OutStream, v: T, ?encode: (v: T) -> Bytes) [Streams] -> Long
=> s, !v =>[encode] !v {
    let data = encode(v)
    let n = write_int(s, size(data))
    return n + write_bytes(s, data)
}

// Reads a 4-byte big-endian `Int`, as [write_int] wrote it: `End` when the
// stream had ended, an error when it ended inside the number.
// @compilerrules [stream-values]
export fn read_int(s: InStream) [Streams] -> Ok Int | End | Err Checked<StreamError> => s {
    let r = read_fixed(s, 4)
    when r {
        is Ok {
            return ok(to_int(r))
        }
        is End {
            return End {}
        }
        is Err {
            return r
        }
    }
}

// Reads an 8-byte big-endian `Long`, as [write_long] wrote it: `End` when
// the stream had ended, an error when it ended inside the number.
// @compilerrules [stream-values]
export fn read_long(s: InStream) [Streams] -> Ok Long | End | Err Checked<StreamError> => s {
    return read_fixed(s, 8)
}

// Reads a value [write_value] wrote: its length, then that many bytes,
// decoded as a `T`.
//
// Name the type at the call, `read_value<Point>(s)`. The decoder is found at
// the call site: import `std.codec.decode`. The answer is `End` when the
// stream had ended, and an error when it ends inside the value or its bytes
// are not a `T`.
//
// ```
// import std.stream
// import std.codec.encode
// import std.codec.decode
//
// struct Point : Hashed<self> by auto {
//     x: Int,
//     y: Int
// }
//
// // Reads what `save_points` wrote, closing the stream.
// fn load_points(s: InStream) [Streams] -> Mut List<Point> => !s {
//     let out = Mut List<Point>()
//     let n = read_int(s)
//     if n is Ok {
//         let i = 0
//         while i < n {
//             let p = read_value<Point>(s)
//             when p {
//                 is Ok { add(out, p) }
//                 is End {}
//                 is Err { ignore(p) }
//             }
//             i = i + 1
//         }
//     } elif n is Err {
//         ignore(n)
//     }
//     let closed = close(s)
//     if closed is Err {
//         ignore(closed)
//     }
//     return out
// }
// ```
//
// @param s       the stream to read
// @param decode  found at the call site: decodes the bytes as a `T`
// @compilerrules [stream-values]
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
        return err(
            checked<StreamError>(StreamFailed { source: "read_value", message: "the stream ended inside a value" })
        )
    }
    let v = decode(bytes)
    if v is None {
        return err(
            checked<
                StreamError
            >(StreamFailed { source: "read_value", message: "the bytes are not a value of the type asked for" })
        )
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
        add(back, get(out, j) ?: unreachable!())
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
    let first = to_long(to_int(get(data, 0) ?: unreachable!()))
    if first >= 128L {
        first = first - 256L
    }
    let v = first
    let i = 1
    while i < width {
        v = v * 256L + to_long(to_int(get(data, i) ?: unreachable!()))
        i = i + 1
    }
    return ok(v)
}
