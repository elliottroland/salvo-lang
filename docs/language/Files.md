# Files

The filesystem is the first place all of this meets: an effect for the capability, linear tokens for the streams, a linear error that cannot be dropped in silence, a pass for the lines, and a `platform handler` at the very bottom.

A program that reads a file declares `[Fs]` and nothing else:

```
fn first_line(path: Str) [Fs] -> Ok Str | Err FsError => path {
    let opened = open_read(path)
    if opened is Err {
        return opened                    // the error travels; it still owes
    }
    let s: InStream = opened             // s owes: a stream must be closed
    let line = read_line(s)
    let closed = close(s)                // the discharger, and it reports
    if closed is Err {
        return closed
    }
    when line {
        is Str { return ok(line) }
        is None { return err(FsError { kind: IoError {path: copy(path), message: "empty"} }) }
    }
}
```

Four things in that function are the language's, not the library's:

* **`open_read` returns a union with a linear arm**, so the result *is* the stream: forget to look at it and the program does not compile; narrow it to `Err` and the stream was never opened.
* **`InStream` is linear**, so the `close` is not politeness. Its only field is a handle — the position, the buffer and the resource live in the handler — which is why no stream operation needs `Mut`.
* **`FsError` is linear too.** An error you do not care about takes one call to say so: `ignore(e)`. One you want to keep costs `detach(e)`, which hands back the plain `FsErrorKind` (a linear value may not be stored, so this is the way into a `List<FsErrorKind>`). Narrowing a result to its `Ok` arm discharges the error that was never there.
* **Failures are returned, never thrown.** An effect member may declare no effects, `Throw` included, so every fallible member answers `Ok T | Err FsError`.

Reading the lines is ordinary iteration, over a pass that owns the stream:

```
fn print_file(path: Str) [Fs, Console] -> None => path {
    let opened = open_read(path)
    if opened is Err {
        println("cannot read ${path}: ${to_str(opened)}")
        ignore(opened)
        return None
    }
    let p = lines(opened)                // the stream's obligation moves in
    for line in p {
        println(line)
    }
    let closed = close(p)          // closing the pass closes the stream
    if closed is Err {
        ignore(closed)
    }
}
```

And the 90% case needs none of it — `read_to_str(path)`, `read_lines(path)`, `write_str(path, text)` open, work and close, so no token ever reaches the caller.

The composition root is where the filesystem is chosen:

```
fn main() [use] {
    use StdOutConsole()
    use HostRawFs()      // the host's: real files, plain handles
    use DefaultFs()      // Salvo: mints the tokens, maps the errors
    let text = read_to_str("notes.txt")
    when text {
        is Ok { println(text) }
        is Err { println(to_str(text)) ignore(text) }
    }
}
```

`DefaultFs` depends on `RawFs` and says so on its declaration, so nothing above it mentions the raw layer; `RawFs` trades in `Long` handles and droppable error kinds, so the host class never holds a Salvo obligation — the `close` that discharges a token is Salvo code, checked. Swapping the bottom swaps the filesystem: a handler of your own that implements `Fs` fakes the whole surface, streams included, because the stream operations are *members* rather than free functions.

Byte offsets are exact and usable: `write` and `write_line` answer how many bytes they took, `position` reports the consumed byte offset of a stream, and `open_read_at(path, offset)` reopens at one. Byte counts are `byte_size(str)`, deliberately a different function from `size(str)`, which counts characters. There is no seek — streams are forward-only.

Bytes are readable and writable as themselves: `read_bytes(s, max)` answers up to `max` bytes as a `Bytes` and `write_bytes(s, data)` writes them back, with nothing encoded or decoded on the way, so a file that is not text is handled by the same surface. Text and byte operations share one stream and one position, both counted in bytes, so a text read continues exactly where a byte read stopped. Text is decoded **strictly**: an offset that lands mid-codepoint is a legal seek — it is bytes, and bytes have no characters — and it is the *decode* that fails, as `Err InvalidUtf8` rather than as mojibake. That failure is recorded too, so `close` reports it a second time.

```
let s: InStream = opened
let head = read_bytes(s, 4)      // Ok Bytes | Err FsError
let rest = read_all(s)           // continues after those four bytes
```

Every read has a **fill-a-buffer** form, for the loop where allocating a payload per step is the cost: `read_to(s, buf, max)` appends up to `max` bytes to a `Mut Bytes` of yours and answers how many, `read_to(s, builder)` appends the rest of the stream to a `Mut Str`, and `read_line_to(s, builder)` appends the next line and answers whether there was one. They *append* rather than overwrite, so `size(buf)` is the data — there is no "only the first n are meaningful" convention — and `clear(buf)` between steps is what makes one buffer serve a whole loop.

```
let buf = mut_bytes()
let reading = true
while reading {
    clear(buf)
    let got = read_to(s, buf, 65536)     // Ok Int | Err FsError
    when got {
        is Ok { if got == 0 { reading = false } else { consume(buf) } }
        is Err { ignore(got) reading = false }
    }
}
```

Or hand the loop over: `chunks(s, size)` is a pass over a stream's bytes (a fresh buffer per step) as `lines(s)` is over its lines, and the one-shots keep the buffer out of sight entirely — `copy_file(from, to)`, `copy_stream(s, w)`, `read_to_bytes(path)`, `write_bytes_to(path, data)`.

Two more handlers come with the surface, and neither is a special case of anything:

```
fn main() [use] {
    use StdOutConsole()
    use MemFs()                      // a filesystem in memory: no host, no disk
    if true {
        use RestrictedFs("notes")    // ...scoped to one directory, for this block
        // code in here writes "a.txt" and cannot reach "../secret.txt"
    }
}
```

`MemFs` fakes the *whole* of `Fs`, streams included — which is what putting the stream operations on the effect bought — so a test needs no filesystem at all. Its files are **bytes**, as a real one's are, and it counts byte offsets exactly as the host does, because a fake that stored text and counted characters would let tests pass while production broke.

`RestrictedFs(root)` is an **interceptor**: it declares the effect it implements, so it wraps whichever filesystem is already registered, and the same handler restricts the host's files in production and a `MemFs` in a test of the restriction itself. Paths are rebased — code under it never learns where it is really running — and one that resolves outside the root comes back as `Err PathEscapes` rather than pretending not to exist. The check is lexical, so it is not symlink-safe; that hardening belongs to the host layer and is not pretended away here.
