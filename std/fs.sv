// The filesystem surface: an `Fs` effect carrying both path operations and
// stream operations, linear stream tokens, and an error the caller cannot drop
// in silence.
//
// This module is the *surface* — types, the effect, the pass and the
// one-shots. The filesystem of the machine the program runs on lives next
// door in `fs.host` (`RawFs`, `HostRawFs`, `DefaultFs`), so a program that
// fakes the filesystem never links a host handler — and a whole-module import
// names one module, so `import fs` does not bring either of them
// [mod-import-module].
//
// The layering, bottom to top:
//
//   * `RawFs` (`fs.host`) trades in plain `Long` handles and bare
//     `FsError`s. Its one implementation is the host's
//     (`platform handler HostRawFs`), so no target-language class ever holds
//     a Salvo obligation.
//   * `Fs` is what programs declare. Its handler `DefaultFs [RawFs]` mints
//     the linear tokens, wraps failures in `Checked<FsError>`, and delegates
//     downwards; the dependency is supplied at `use` and appears in nobody's
//     signature.
//   * The pass and the one-shots are ordinary Salvo over `Fs` members, so a
//     double that fakes `Fs` fakes them too.

// ===== errors =====

// What went wrong: droppable, storable, and what `detach` hands back for
// aggregation. Every failing member answers `Err Checked<FsError>`, so the
// error cannot be dropped in silence — but the value inside is an ordinary
// union with no obligation of its own [checked-type].
export type FsError = NotFound | PermissionDenied | AlreadyExists | NotADirectory
                 | PathEscapes | InvalidUtf8 | StaleHandle | IoError

// Nothing exists at the path.
export struct NotFound { path: Str }
// The path exists and the operation was not permitted.
export struct PermissionDenied { path: Str }
// The path exists where the operation needed it not to.
export struct AlreadyExists { path: Str }
// A path component that had to be a directory was not one.
export struct NotADirectory { path: Str }
// A restricted handler refused the path: it resolves outside its root.
export struct PathEscapes { path: Str }
// The bytes read are not valid UTF-8. Decoding is strict on both backends.
export struct InvalidUtf8 { path: Str }
// The token was minted by a handler that is no longer the one in scope, so
// nothing here knows what it refers to.
export struct StaleHandle { path: Str }
// Anything the host reported that the kinds above do not name.
export struct IoError { path: Str, message: Str }

// An error that must be acknowledged: `ignore` it, `detach` it, or narrow the
// result that carries it to its `Ok` arm — which is what "it was fine" costs.
// The obligation is `Checked<FsError>` [checked-type], so the mechanism is the
// one every "you must look at this" answer in std uses, and `ignore`/`detach`
// are the generic ones.

// The text of a failure [interp-to-str].
export fn to_str(kind: FsError) [] -> Str => kind {
    when kind {
        is NotFound { return "no such file or directory: ${kind.path}" }
        is PermissionDenied { return "permission denied: ${kind.path}" }
        is AlreadyExists { return "already exists: ${kind.path}" }
        is NotADirectory { return "not a directory: ${kind.path}" }
        is PathEscapes { return "path escapes the root: ${kind.path}" }
        is InvalidUtf8 { return "not valid UTF-8: ${kind.path}" }
        is StaleHandle { return "stale stream token: ${kind.path}" }
        is IoError { return "io error: ${kind.path}: ${kind.message}" }
    }
}

// ===== the effect programs use =====

// What a file's metadata says. Enough for the operations v1 has.
export struct FileInfo { size: Long, is_dir: Bool }

// A stream open for reading. An opaque token: the handler behind it owns the
// position, the buffer and the resource, so the only thing here is which
// stream this is. Linear, so a stream that is never closed is a compile
// error rather than a leak [linear-group]; `close` is the discharger, and it
// is an effect member, so a double discharges it too.
export linear struct InStream { handle: Long }

// A stream open for writing. Its `close` flushes.
export linear struct OutStream { handle: Long }

// The filesystem. Path operations and stream operations are members of one
// effect, so application code declares `[Fs]` and nothing else — and a
// double that implements `Fs` fakes the whole surface, streams included.
//
// Every failure is a returned `Err Checked<FsError>`, never a `[Throw]`: an effect
// member may declare no effects of its own.
export effect Fs {
    // Opens a file for reading, from the beginning.
    fn open_read(path: Str) -> Ok InStream | Err Checked<FsError> => path
    // Opens a file for reading, positioned at a byte offset. Targeted reads
    // are a fresh open rather than a seek: streams stay forward-only.
    fn open_read_at(path: Str, offset: Long) -> Ok InStream | Err Checked<FsError> => path
    // Opens a file for writing, creating it or truncating what is there.
    fn open_write(path: Str) -> Ok OutStream | Err Checked<FsError> => path
    // Opens a file for writing at its end, creating it if absent.
    fn open_append(path: Str) -> Ok OutStream | Err Checked<FsError> => path
    // Whether anything exists at the path.
    fn exists(path: Str) -> Bool => path
    // The path's metadata.
    fn metadata(path: Str) -> Ok FileInfo | Err Checked<FsError> => path
    // The entry names directly inside a directory, eagerly.
    fn list_dir(path: Str) -> Ok List<Str> | Err Checked<FsError> => path
    // Creates the directory and every missing parent of it.
    fn create_dirs(path: Str) -> Ok None | Err Checked<FsError> => path
    // Deletes a file, or an empty directory.
    fn delete(path: Str) -> Ok None | Err Checked<FsError> => path
    // Renames a path, replacing the destination if it exists.
    fn rename_path(from: Str, to: Str) -> Ok None | Err Checked<FsError> => from, to

    // The next line, without its terminator (`\n` and `\r\n` both end a
    // line, neither is returned). Absent means the stream ended — or that a
    // read failed, which `close` reports: iteration stops either way, and
    // one error surfaces in one place.
    fn read_line(s: InStream) -> Str | None => s
    // Everything left in the stream, decoded strictly.
    fn read_all(s: InStream) -> Ok Str | Err Checked<FsError> => s
    // Up to [max] bytes, exactly as they lie in the file: no decoding, so a
    // read may stop in the middle of a character and a file that is not text
    // is read all the same. Fewer bytes than asked for means the stream
    // ended; an empty buffer means it has ended already [fs-bytes].
    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err Checked<FsError> => s
    // Up to [max] bytes **appended to [buf]**, answering how many. The
    // fill-a-buffer read [fs-read-to]: one buffer, `clear`ed and refilled for
    // as long as a loop runs, instead of a fresh payload per chunk. Appending
    // rather than overwriting is what keeps `size(buf)` the truth — there is
    // no "only the first n bytes are meaningful" convention to remember.
    fn read_to(s: InStream, buf: Mut Bytes, max: Int) -> Ok Int | Err Checked<FsError> => s, buf: Mut
    // Everything left in the stream, decoded strictly and **appended to
    // [buf]**, answering how many bytes were consumed. `read_all` with the
    // string handed in [fs-read-to].
    fn read_to(s: InStream, buf: Mut Str) -> Ok Long | Err Checked<FsError> => s, buf: Mut
    // The next line, without its terminator, **appended to [buf]**; `false`
    // means the stream ended — or that a read failed, which `close` reports.
    // `read_line` with the string handed in, which is the one that matters in
    // a loop: a line per iteration costs no new string [fs-read-to].
    fn read_line_to(s: InStream, buf: Mut Str) -> Bool => s, buf: Mut
    // Bytes consumed so far — the offset a later `open_read_at` would use
    // to resume here. Not a seek.
    fn position(s: InStream) -> Long => s
    // Releases the stream, reporting a failure recorded during reading.
    fn close(s: InStream) -> Ok None | Err Checked<FsError> => !s

    // Writes text, answering how many bytes it took. A failed write is
    // recorded and surfaces at `flush` or `close`, so writing in a loop does
    // not narrow a result per line.
    fn write(s: OutStream, text: Str) -> Long => s, text
    // Writes text and a `\n`, answering how many bytes both took.
    fn write_line(s: OutStream, text: Str) -> Long => s, text
    // Writes bytes as they are, answering how many were accepted. The byte
    // side of `write`: nothing is encoded, so what goes in is what the file
    // holds [fs-bytes].
    fn write_bytes(s: OutStream, data: Bytes) -> Long => s, data
    // Bytes accepted so far.
    fn position(s: OutStream) -> Long => s
    // Pushes accepted bytes to the host, reporting a recorded failure.
    fn flush(s: OutStream) -> Ok None | Err Checked<FsError> => s
    // Flushes and releases the stream.
    fn close(s: OutStream) -> Ok None | Err Checked<FsError> => !s
}

// The filesystem of the machine the program runs on: linearity above,
// plain handles below. Every token it mints is discharged here in Salvo —
// the host never sees an obligation.
// ===== reading lines as a sequence =====

// A pass over a stream's lines: `for line in p` drives it. It holds the
// stream, so it carries the obligation too — bind it, loop over it, and
// discharge it when you are done [linear-group].
export linear struct Lines : Yield<self, Str> canbe Mut {
    s: InStream
}

// Reads `s` as a sequence of lines. The stream's obligation moves into the
// pass, which is why closing the pass is closing the stream.
export fn lines(s: InStream) [] -> Mut Lines => !s {
    return Mut Lines { s: s }
}

// The pass's step: a line, or the end of the sequence. Declares `[Fs]`,
// since reading is an effect — so `for`, `map`, `filter` and `reduce`
// inherit it at the call site.
export fn next(p: Mut Lines) [local Fs] -> Emitted Str | Finished => p: Mut {
    let line = read_line(p.s)
    when line {
        is Str { return emitted(line) }
        is None { return finished() }
    }
}

// Closes the stream the pass reads, reporting what reading recorded. Named
// `close` like every other discharger: a member and a fn of one name are one
// overload set, and the argument type picks [effect-available].
export fn close(p: Lines) [local Fs] -> Ok None | Err Checked<FsError> => !p {
    return close(p.s)
}

// Opens a file as a sequence of its lines.
export fn open_lines(path: Str) [local Fs] -> Ok Mut Lines | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    return ok(lines(opened))
}

// ===== reading bytes as a sequence =====

// A pass over a stream's chunks: `for chunk in c` drives it, and each step is
// up to [size] bytes. Like [Lines] it holds the stream, so it carries the
// obligation too [linear-group].
export linear struct Chunks : Yield<self, Bytes> canbe Mut {
    s: InStream,
    // How many bytes a step asks for.
    size: Int
}

// Reads `s` as a sequence of chunks of up to [size] bytes each. The stream's
// obligation moves into the pass, which is why closing the pass closes the
// stream.
export fn chunks(s: InStream, size: Int) [] -> Mut Chunks => !s {
    return Mut Chunks { s: s, size: size }
}

// The pass's step. Each chunk is a **fresh** buffer, deliberately: a pass that
// handed back its own buffer would have the next step overwrite what the
// caller is still holding. The allocation-free shape is `read_to` into a
// buffer of your own — which is what [copy_stream] does [fs-read-to].
export fn next(p: Mut Chunks) [local Fs] -> Emitted Bytes | Finished => p: Mut {
    let got = read_bytes(p.s, p.size)
    if got is Err {
        // The handler recorded the failure, and `close` reports it — so
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

// Closes the stream the pass reads, reporting what reading recorded.
export fn close(p: Chunks) [local Fs] -> Ok None | Err Checked<FsError> => !p {
    return close(p.s)
}

// Opens a file as a sequence of chunks of up to [size] bytes.
export fn open_chunks(path: Str, size: Int) [local Fs] -> Ok Mut Chunks | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    return ok(chunks(opened, size))
}

// ===== the one-shots =====

// A whole file as text: the 90% case, and no token reaches the caller.
export fn read_to_str(path: Str) [local Fs] -> Ok Str | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    let s: InStream = opened
    let content = read_all(s)
    if content is Err {
        // The read failed, and so may the close: report the read's error and
        // acknowledge the other.
        let closed = close(s)
        if closed is Err {
            ignore(closed)
        }
        return content
    }
    let closed = close(s)
    if closed is Err {
        return closed
    }
    return content
}

// A whole file as its lines, eagerly.
export fn read_lines(path: Str) [local Fs] -> Ok List<Str> | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    let p = lines(opened)
    let out: Mut List<Str> = mut_list_of()
    for line in p {
        out.add(line)
    }
    let closed = close(p)
    if closed is Err {
        return closed
    }
    let done: List<Str> = out
    return ok(done)
}

// Writes text to a file, creating it or replacing what is there, and answers
// how many bytes it took.
export fn write_str(path: Str, content: Str) [local Fs] -> Ok Long | Err Checked<FsError> => path, content {
    let opened = open_write(path)
    if opened is Err {
        return opened
    }
    let s: OutStream = opened
    let written = write(s, content)
    let closed = close(s)
    if closed is Err {
        return closed
    }
    return ok(written)
}

// The whole of a file as bytes: `read_to_str` for data that is not text.
export fn read_to_bytes(path: Str) [local Fs] -> Ok Bytes | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    let s: InStream = opened
    let buf = mut_bytes()
    let filling = fill_from(s, buf)
    if filling is Err {
        let closed = close(s)
        if closed is Err {
            ignore(closed)
        }
        return filling
    }
    let closed = close(s)
    if closed is Err {
        return closed
    }
    let done: Bytes = buf
    return ok(done)
}

// Writes bytes to a file, creating it or replacing what is there, and answers
// how many it took.
export fn write_bytes_to(path: Str, data: Bytes) [local Fs] -> Ok Long | Err Checked<FsError> => path, data {
    let opened = open_write(path)
    if opened is Err {
        return opened
    }
    let s: OutStream = opened
    let written = write_bytes(s, data)
    let closed = close(s)
    if closed is Err {
        return closed
    }
    return ok(written)
}

// ===== copying, with the buffer kept out of sight =====

// How many bytes a copy moves at a time. Big enough that the syscall is not
// the cost, small enough to be nobody's memory problem.
fn fs_chunk_size() [] -> Int {
    return 65536
}

// Appends everything left in [s] to [buf], answering how many bytes moved.
// One buffer for the whole read: this is `read_to` in a loop, which is the
// point of `read_to` existing [fs-read-to].
export fn fill_from(s: InStream, buf: Mut Bytes) [local Fs] -> Ok Long | Err Checked<FsError> => s, buf: Mut {
    let total: Long = 0
    let reading = true
    while reading {
        let got = read_to(s, buf, fs_chunk_size())
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
// Neither token is consumed: whoever opened them closes them.
export fn copy_stream(s: InStream, w: OutStream) [local Fs] -> Ok Long | Err Checked<FsError> => s, w {
    let buf = mut_bytes()
    let total: Long = 0
    let copying = true
    while copying {
        clear(buf)
        let got = read_to(s, buf, fs_chunk_size())
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

// Copies the file at [from] onto [to], creating it or replacing what is there,
// and answers how many bytes moved. The one-shot: no token, no buffer and no
// stream reaches the caller.
export fn copy_file(from: Str, to: Str) [local Fs] -> Ok Long | Err Checked<FsError> => from, to {
    let opened = open_read(from)
    if opened is Err {
        return opened
    }
    let s: InStream = opened
    let created = open_write(to)
    if created is Err {
        let closed = close(s)
        if closed is Err {
            ignore(closed)
        }
        return created
    }
    let w: OutStream = created
    let moved = copy_stream(s, w)
    // Both streams are closed whatever happened, and the *first* failure is
    // the one reported — a close that also failed is acknowledged, since a
    // linear error may not simply be dropped [linear-group].
    let shut_w = close(w)
    let shut_s = close(s)
    if moved is Err {
        if shut_w is Err {
            ignore(shut_w)
        }
        if shut_s is Err {
            ignore(shut_s)
        }
        return moved
    }
    if shut_w is Err {
        if shut_s is Err {
            ignore(shut_s)
        }
        return shut_w
    }
    if shut_s is Err {
        return shut_s
    }
    return moved
}
