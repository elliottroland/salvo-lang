# Files

## Overview

A program that touches files declares the **`Fs`** effect, and the program's
entry point decides which filesystem that means: the machine's, an in-memory
one for tests, or either confined to one directory. The code that reads and
writes does not change.

```salvo
import std.fs
import std.fs.path

fn word_count(name: Str) [Fs] -> Int => name {
    let text = read_to_str(path(name))   // Ok Str | Err Checked<FsError>
    when text {
        is Ok { return size(split(text, " ")) }
        is Err {
            ignore(text)                 // an error must be looked at
            return 0
        }
    }
}
```

The filesystem is the first place several of Salvo's features meet, and each
of them shows in that small function or just behind it:

* **An effect for the capability.** `[Fs]` in the signature says the function
  reaches the filesystem ([Effects and handlers](Effects-and-Handlers.md)).
* **An error that cannot be dropped in silence.** A failure arrives as
  `Err Checked<FsError>`, and the compiler refuses a program that ignores it
  without saying so.
* **Linear tokens for the streams.** Opening a file answers a stream that must
  be closed on every path ([Linear types](Linear-Types.md)).
* **An iterator for the lines**, and a `platform handler` at the very bottom,
  where Salvo meets the host ([Backends](Backends.md)).

The one-call functions ([`read_to_str`](../../lib/std/fs.sv#fn-read_to_str),
[`read_lines`](../../lib/std/fs.sv#fn-read_lines),
[`write_str`](../../lib/std/fs.sv#fn-write_str) and their byte forms) open,
work and close, so the 90% case never holds a stream. This page explains the
model; the module pages list every operation:

| Module | What it holds |
|---|---|
| [`std.fs`](../../lib/std/fs.sv) | the [`Fs`](../../lib/std/fs.sv#effect-fs) effect (operations on paths), [`FsError`](../../lib/std/fs.sv#type-fserror), the one-call reads and writes |
| [`std.stream`](../../lib/std/stream.sv) | the [`Streams`](../../lib/std/stream.sv#effect-streams) effect, [`InStream`](../../lib/std/stream.sv#struct-instream)/[`OutStream`](../../lib/std/stream.sv#struct-outstream), [`StreamError`](../../lib/std/stream.sv#type-streamerror), [`lines`](../../lib/std/stream.sv#fn-lines) and [`chunks`](../../lib/std/stream.sv#fn-chunks) |
| [`std.fs.path`](../../lib/std/fs/path.sv) | [`Path`](../../lib/std/fs/path.sv#struct-path) and its operations |
| [`std.fs.host`](../../lib/std/fs/host.sv), [`std.stream.host`](../../lib/std/stream/host.sv) | the machine's filesystem and streams |
| [`std.fs.mem`](../../lib/std/fs/mem.sv) | [`MemFs`](../../lib/std/fs/mem.sv#handler-memfs), an in-memory filesystem for tests |
| [`std.fs.restricted`](../../lib/std/fs/restricted.sv) | [`RestrictedFs`](../../lib/std/fs/restricted.sv#handler-restrictedfs), a filesystem confined to one directory |

## Two effects: `Fs` and `Streams`

The filesystem is two effects, layered.

* **`Streams`** (module `std.stream`) owns the stream tokens
  `InStream`/`OutStream` and every operation on them (the reads, the writes,
  `position`, `close`), whoever opened the stream: a file, a network body, a
  buffer.
* **`Fs`** (module `std.fs`) is the operations on *paths*; opening one mints a
  stream into the `Streams` bound around it.

`Streams` is a **prerequisite** of `Fs` (`effect Fs [Streams]`), so wherever
`Fs` is, `Streams` is, and a program that reads a file still declares `[Fs]`
alone.

Both are **imported, not implicit**: `import std.fs` brings the path surface
and `import std.stream` the stream one (a program that names `InStream` or
`StreamError` imports it). Each implementation is its own module:
`std.stream.host` and `std.fs.host` for the machine's, `std.fs.mem` for an
in-memory fake of both, `std.fs.restricted` for a sandbox. A whole-module
import names one module, so a test that fakes the filesystem never mentions
the host's, and a program that never opens a file links none of it.

## Opening a stream

When a file is too big to hold at once, or needs reading in parts, open it.
A function that reads a file this way still declares `[Fs]` and nothing else:

```salvo
fn first_line(path: Path) [Fs] -> Ok Str | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened                    // the error travels; it still owes
    }
    let s: InStream = opened             // s owes: a stream must be closed
    let line = read_line(s)              // a `Streams` member, in scope through `Fs`
    let closed = close(s)                // the discharger, and it reports
    if closed is Err {
        return err(checked<FsError>(Streaming {error: detach(closed)}))
    }
    when line {
        is Str { return ok(line) }
        is None { return err(checked<FsError>(IoError {path: to_str(path), message: "empty"})) }
    }
}
```

Four things in that function are the language's, not the library's:

* **`open_read` returns a union with a linear arm**, so the result *is* the
  stream: forget to look at it and the program does not compile; narrow it to
  `Err` and the stream was never opened.
* **`InStream` is linear**, so the `close` is not politeness. Its only field is
  a handle (the position, the buffer and the resource live in the handler),
  which is why no stream operation needs `Mut`.
* **The error carries an obligation too.** A path operation's failure arrives
  as `Checked<FsError>`, a stream operation's as `Checked<StreamError>`: the
  same wrapper every "you must look at this" answer in the library uses. An
  error you do not care about takes one call to say so: `ignore(e)`. One you
  want to read or keep costs `detach(e)`, which hands back the plain value (a
  linear value may not be stored, so this is the way into a `List<FsError>`).
  Narrowing a result to its `Ok` arm discharges the error that was never
  there. A one-shot that opens *and* reads reports a read failure as
  `FsError`'s `Streaming { error }` arm, so it answers one type.
* **Failures are returned, never thrown.** An effect member may declare no
  effects, `Throw` included, so every fallible member answers
  `Ok T | Err Checked<…>`.

## Reading the lines

Reading the lines is ordinary iteration ([Iterators](Iterators.md)), over an
iterator that owns the stream:

```salvo
fn print_file(path: Path) [Fs, Console] -> None => path {
    let opened = open_read(path)
    if opened is Err {
        println("cannot read ${path}: ${to_str(detach(opened))}")
        return None
    }
    let p = lines(opened)                // the stream's obligation moves in
    for line in p {
        println(line)
    }
    let closed = close(p)          // closing the iterator closes the stream
    if closed is Err {
        ignore(closed)
    }
}
```

[`open_lines`](../../lib/std/fs.sv#fn-open_lines) does the first two steps in
one call.

## Choosing the filesystem

The composition root is where the filesystem is chosen: streams first, since
`Fs` needs them bound.

```salvo
import std.fs
import std.fs.path
import std.fs.host
import std.stream.host

fn main() [use] {
    use StdConsole()
    use HostRawStreams()  // the host's stream table: plain handles
    use DefaultStreams()  // Salvo: discharges the tokens, maps the errors
    use HostRawFs()       // the host's files, registered into that table
    use DefaultFs()       // Salvo: mints the tokens, maps the errors
    let text = read_to_str(path("notes.txt"))
    when text {
        is Ok { println(text) }
        is Err { println(to_str(detach(text))) }
    }
}
```

`DefaultFs` depends on `RawFs` and says so on its declaration, and on
`Streams` through `Fs`'s prerequisite, so nothing above it mentions the raw
layer. `RawFs` and `RawStreams` trade in `Long` handles and bare errors, so the
host classes never hold a Salvo obligation: the `close` that discharges a
token is Salvo code, checked. What `HostRawFs` opens goes into one
process-wide stream table that `HostRawStreams` reads, so a file is just
another stream: a network client's body and a file are read by the same
`Streams`. Four lines is a lot for a composition root; *handler bundles* are
recorded on the roadmap with this as their first customer.

Swapping the bottom swaps the world: a handler of your own that implements
`Fs` and `Streams` fakes the whole surface.

### In memory, and confined

Two more handlers ship beside the surface, each in its own module, and neither
is a special case of anything:

```salvo
import std.fs
import std.fs.path
import std.fs.mem
import std.fs.restricted

fn main() [use] {
    use StdConsole()
    use MemFs()                      // a filesystem in memory: no host, no disk
    if true {
        use RestrictedFs(path("notes"))    // ...scoped to one directory, for this block
        // code in here writes "a.txt" and cannot reach "../secret.txt"
    }
}
```

**[`MemFs`](../../lib/std/fs/mem.sv#handler-memfs)** fakes `Fs` **and**
`Streams`: one handler wearing both faces (`for Fs, Streams`), since a write
stream has to publish into its file map when it closes. So a test needs no
filesystem and no host at all, and one `use` stands in for all four host
lines. Its files are **bytes**, as a real one's are, and it counts byte
offsets exactly as the host does, because a fake that stored text and counted
characters would let tests pass while production broke. Its page covers the
rest: it starts empty, directories are implicit, and writes appear at the
close.

**A stream belongs to the provider that minted it.** Handing a `MemFs` stream
to the host's `Streams` (or the reverse) is a program bug and **traps**. Every
stream table draws its handles from one process-wide counter, so the wrong
table always finds an *unknown* handle rather than someone else's live stream.
The rule of thumb is all-host or all-mem, never a mix.

**[`RestrictedFs(root)`](../../lib/std/fs/restricted.sv#handler-restrictedfs)**
is an **interceptor**: it declares the effect it implements, so it wraps
whichever filesystem is already registered, and the same handler restricts the
host's files in production and a `MemFs` in a test of the restriction itself.
It intercepts `Fs` only: the policy is entirely in the opens, and what they
open is read by the `Streams` in scope as usual. Paths are rebased, so code
under it never learns where it is really running, and one that resolves
outside the root comes back as `Err PathEscapes` rather than pretending not to
exist. The check is lexical, so it is not symlink-safe; that hardening belongs
to the host layer and is not pretended away here.

## Bytes and offsets

**Byte offsets are exact and usable.** `write` and `write_line` answer how
many bytes they took, `position` reports the consumed byte offset of a stream,
and `open_read_at(path, offset)` reopens at one. Byte counts are
`byte_size(str)`, deliberately a different function from `size(str)`, which
counts characters. There is no seek: streams are forward-only.

**Bytes are readable and writable as themselves.** `read_bytes(s, max)`
answers up to `max` bytes as a `Bytes` and `write_bytes(s, data)` writes them
back, with nothing encoded or decoded on the way, so a file that is not text
is handled by the same surface. Text and byte operations share one stream and
one position, both counted in bytes, so a text read continues exactly where a
byte read stopped:

```salvo
let s: InStream = opened
let head = read_bytes(s, 4)      // Ok Bytes | Err Checked<StreamError>
let rest = read_all(s)           // continues after those four bytes
```

**Text is decoded strictly.** An offset that lands mid-codepoint is a legal
seek (it is bytes, and bytes have no characters), and it is the *decode* that
fails, as `Err InvalidUtf8` rather than as mojibake. That failure is recorded
too, so `close` reports it a second time.

### Reading into a buffer

Every read has a **fill-a-buffer** form, for the loop where allocating a
payload per step is the cost:

* `read_to(s, buf, max)` appends up to `max` bytes to a `Mut Bytes` of yours
  and answers how many;
* `read_to(s, builder)` appends the rest of the stream to a `Mut Str`;
* `read_line_to(s, builder)` appends the next line and answers whether there
  was one.

They *append* rather than overwrite, so `size(buf)` is the data (there is no
"only the first n are meaningful" convention), and `clear(buf)` between steps
is what makes one buffer serve a whole loop:

```salvo
let buf = mut_bytes()
let reading = true
while reading {
    clear(buf)
    let got = read_to(s, buf, 65536)     // Ok Int | Err Checked<StreamError>
    when got {
        is Ok { if got == 0 { reading = false } else { consume(buf) } }
        is Err { ignore(got) reading = false }
    }
}
```

Or hand the loop over: [`chunks(s, size)`](../../lib/std/stream.sv#fn-chunks)
is an iterator over a stream's bytes (a fresh buffer per step) as
[`lines(s)`](../../lib/std/stream.sv#fn-lines) is over its lines; both are
`std.stream`'s, since they work on any stream. The one-shots keep the buffer
out of sight entirely: [`copy_stream(s, w)`](../../lib/std/stream.sv#fn-copy_stream)
(`std.stream`'s), and [`copy_file(from, to)`](../../lib/std/fs.sv#fn-copy_file),
[`read_to_bytes(path)`](../../lib/std/fs.sv#fn-read_to_bytes) and
[`write_bytes_to(path, data)`](../../lib/std/fs.sv#fn-write_bytes_to)
(`std.fs`'s).

## Reading without blocking

A read can also **not block**. `receive(s, reply)` hands the stream and a
continuation to the provider and returns at once; the answer arrives on the
reply: the bytes and the stream back, or `End`, or a failure (both of which
have closed the stream). Replies and `send fn` are explained in
[Concurrency](Concurrency.md).

```salvo
send fn on_chunk(total: Long, got: Received) [Streams, Console] => !got {
    when got {
        is Ok {
            let {bytes, stream} = got              // the stream comes back with the bytes
            receive(stream, replyto on_chunk(total + to_long(size(bytes))))
        }
        is End { println("${total} bytes") }       // closed already: nothing owed
        is Err { ignore(got) }
    }
}
```

Because the read consumes the stream and the answer returns it, one read is in
flight and a `close` in the middle of one cannot be written. The host reads on
a thread of its own and completes the reply from there, so no worker waits
while a slow source is slow.

[`pipe(from, to, done)`](../../lib/std/stream.sv#fn-pipe) is the copy built
this way, and `from_bytes(data)` makes a stream out of a buffer, so a request
body and a file are the same kind of thing: a body made with `from_bytes` can
be piped into a file opened with `open_write`, and a file opened with
`open_read` can be piped into an upload.

## Paths

A path is a value of its own, [`Path`](../../lib/std/fs/path.sv#struct-path),
from `import std.fs.path`, so path operations stay out of `Str`'s way:

```salvo
import std.fs.path

let db = path("/var/data/board.db")
let dir = parent(db)              // Path?: /var/data
let name = file_name(db)          // Str?: board.db
let ext = extension(db)           // Str?: db
let backup = with_extension(db, "bak")
let log = join(path("/var/log"), "board.log")
```

Every `fs` operation and one-shot takes its paths as `Path`s, and only as
`Path`s (`MemFs` and `RestrictedFs(root)` included), so a path is never
confused with any other string. An error still reports the path as text. A
`Path` is only text: nothing about it touches a disk, and the separator is `/`
on every backend.

## Numbers and values in a stream

[`write_int`](../../lib/std/stream.sv#fn-write_int),
[`write_long`](../../lib/std/stream.sv#fn-write_long),
[`read_int`](../../lib/std/stream.sv#fn-read_int) and
[`read_long`](../../lib/std/stream.sv#fn-read_long) write and read
fixed-width numbers, big-endian.
[`write_value(out, v)`](../../lib/std/stream.sv#fn-write_value) writes any
value with a wire form in Salvo's canonical encoding, behind its length, and
[`read_value<T>(in)`](../../lib/std/stream.sv#fn-read_value) reads one back,
which is what a file of records needs:

```salvo
import std.codec.encode
import std.codec.decode

let _n = write_value(out, Notice { id: "a1", body: "hello" })
let back = read_value<Notice>(input)   // Ok Notice | End | Err Checked<StreamError>
```

The encoding is the one actors use across the network
([Concurrency](Concurrency.md)), so a file written by a Kotlin program reads
back in a Rust one. A read answers `End` when the stream has ended before it,
and an error when it ends part-way through or the bytes are not a value of the
type asked for. The two `import`s bring the codec the call needs: a value's
encoding belongs to its type, so the caller, who knows the type, supplies it.
