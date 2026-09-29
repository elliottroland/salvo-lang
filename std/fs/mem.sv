// An in-memory filesystem: a `MemFs` fakes the whole of `Fs` *and* `Streams`
// — one handler wearing both faces, since a write stream has to publish into
// the file map when it closes — so a test needs no disk and no host handler
// at all. A test of anything else that produces streams (a fake network
// client minting bodies) binds `MemFs` too: its streams are the `Streams` in
// scope.
//
// A fresh `MemFs` is **empty**; write into it with the ordinary surface
// (`write_str`, `open_write`) and read it back. Directories are implicit:
// a path is a key, and a directory exists exactly while something under it
// does.
//
// A file here is **bytes**, exactly as it is on a real filesystem, and every
// offset counts bytes. That is the hazard this type exists to get right: a
// fake that stored text and counted characters would let unit tests pass
// while production broke the moment a file left ASCII. Text reads decode
// strictly, off the same bytes, and a decode failure is *recorded* and
// reported by `close` — the host's behavior, reproduced rather than
// approximated.

import fs
import stream

// What an open read stream is: what it reads — a snapshot of the file's bytes
// when it was opened, and a description for errors — how far the reader has
// consumed, and whether a strict decode has already failed on it (which ends
// iteration and surfaces at `close`).
struct MemRead { source: Str, data: Bytes, at: Int, failed: Bool }

// What an open write stream is: which file, and the bytes written so far.
// Closing (or flushing) publishes the buffer.
struct MemWrite { path: Str, buffer: Bytes }

export handler MemFs of Fs, Streams {
    files: Mut Map<Str, Bytes> = mut_map_of()
    reads: Mut Map<Long, MemRead> = mut_map_of()
    writes: Mut Map<Long, MemWrite> = mut_map_of()

    fn open_read(path: Str) -> Ok InStream | Err Checked<FsError> => path {
        let content = get(files, path)
        if content is None {
            return err(checked<FsError>(NotFound { path: copy(path) }))
        }
        let handle = fresh_handle()
        put(reads, copy(handle), MemRead { source: copy(path), data: copy(content), at: 0, failed: false })
        return ok(InStream { handle: copy(handle) })
    }

    fn open_read_at(path: Str, offset: Long) -> Ok InStream | Err Checked<FsError> => path {
        let content = get(files, path)
        if content is None {
            return err(checked<FsError>(NotFound { path: copy(path) }))
        }
        // A seek is a byte offset and nothing else — landing between the bytes
        // of a character is legal here exactly as it is on the host, and the
        // *decode* is what fails afterwards. Past the end reads as the end.
        let at = to_int(offset)
        if at < 0 {
            at = 0
        }
        let end = size(content)
        if at > end {
            at = end
        }
        let handle = fresh_handle()
        put(reads, copy(handle), MemRead { source: copy(path), data: copy(content), at: at, failed: false })
        return ok(InStream { handle: copy(handle) })
    }

    fn open_write(path: Str) -> Ok OutStream | Err Checked<FsError> => path {
        let handle = fresh_handle()
        let empty = mut_bytes()
        put(writes, copy(handle), MemWrite { path: copy(path), buffer: empty })
        return ok(OutStream { handle: copy(handle) })
    }

    fn open_append(path: Str) -> Ok OutStream | Err Checked<FsError> => path {
        let existing = get(files, path)
        let start = mut_bytes()
        if existing is None {
            // Nothing there yet: appending starts from empty.
        } else {
            start.append(existing)
        }
        let handle = fresh_handle()
        put(writes, copy(handle), MemWrite { path: copy(path), buffer: start })
        return ok(OutStream { handle: copy(handle) })
    }

    fn exists(path: Str) -> Bool => path {
        if contains_key(files, path) {
            return true
        }
        return fs_has_children(files, path)
    }

    fn metadata(path: Str) -> Ok FileInfo | Err Checked<FsError> => path {
        let content = get(files, path)
        if content is None {
            if fs_has_children(files, path) {
                return ok(FileInfo { size: 0, is_dir: true })
            }
            return err(checked<FsError>(NotFound { path: copy(path) }))
        }
        return ok(FileInfo { size: to_long(size(content)), is_dir: false })
    }

    fn list_dir(path: Str) -> Ok List<Str> | Err Checked<FsError> => path {
        if contains_key(files, path) {
            return err(checked<FsError>(NotADirectory { path: copy(path) }))
        }
        if !fs_has_children(files, path) {
            return err(checked<FsError>(NotFound { path: copy(path) }))
        }
        // A `Set` keeps insertion order and does the deduplication, so the
        // sorted list below is the same on both backends [col-insertion-order].
        let names: Mut Set<Str> = mut_set_of()
        let prefix = "${path}/"
        for key in files {
            if starts_with(key, prefix) {
                let rest = trim_prefix(key, prefix)
                let cut = index_of(rest, "/")
                let name = rest
                if cut is Int {
                    let head = substr(rest, 0, cut)
                    if head is Str {
                        name = copy(head)
                    }
                }
                names.add(copy(name))
            }
        }
        let sorted: List<Str> = sort(to_list(names))
        return ok(sorted)
    }

    fn create_dirs(path: Str) -> Ok None | Err Checked<FsError> => path {
        // Directories are implicit here: there is nothing to create, and
        // reporting success is what the host does for an existing tree.
        return ok(None)
    }

    fn delete(path: Str) -> Ok None | Err Checked<FsError> => path {
        if contains_key(files, path) {
            remove(files, path)
            return ok(None)
        }
        if fs_has_children(files, path) {
            return err(checked<FsError>(IoError { path: copy(path), message: "directory not empty" }))
        }
        return err(checked<FsError>(NotFound { path: copy(path) }))
    }

    fn rename_path(from: Str, to: Str) -> Ok None | Err Checked<FsError> => from, to {
        let content = get(files, from)
        if content is None {
            return err(checked<FsError>(NotFound { path: copy(from) }))
        }
        let bytes: Bytes = copy(content)
        remove(files, from)
        put(files, copy(to), bytes)
        return ok(None)
    }

    // The reads delegate to module functions rather than to each other: a
    // handler member may not call a member of the effect it implements, and
    // the shared logic has to live somewhere both can reach.
    fn read_line(s: InStream) -> Str | None => s {
        return mem_read_line(reads, s.handle)
    }

    fn read_all(s: InStream) -> Ok Str | Err Checked<StreamError> => s {
        return mem_read_all(reads, s.handle)
    }

    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err Checked<StreamError> => s {
        return mem_read_bytes(reads, s.handle, max)
    }

    // The fill-a-buffer reads [fs-read-to]: each is its returning sibling with
    // the destination handed in, so the fake has the same three shapes the
    // host does.
    fn read_to(s: InStream, buf: Mut Bytes, max: Int) -> Ok Int | Err Checked<StreamError> => s, buf: Mut {
        let got = mem_read_bytes(reads, s.handle, max)
        if got is Err {
            return got
        }
        let data: Bytes = got
        buf.append(data)
        return ok(size(data))
    }

    fn read_to(s: InStream, buf: Mut Str) -> Ok Long | Err Checked<StreamError> => s, buf: Mut {
        let got = mem_read_all(reads, s.handle)
        if got is Err {
            return got
        }
        let text: Str = got
        buf.append(text)
        return ok(byte_size(text))
    }

    fn read_line_to(s: InStream, buf: Mut Str) -> Bool => s, buf: Mut {
        let line = mem_read_line(reads, s.handle)
        when line {
            is Str {
                buf.append(line)
                return true
            }
            is None { return false }
        }
    }

    fn position(s: InStream) -> Long => s {
        // The bytes consumed, and nothing to convert: the store *is* bytes.
        return to_long(mem_read_state(reads, s.handle).at)
    }
    fn close(s: InStream) -> Ok None | Err Checked<StreamError> => !s {
        let open = mem_read_state(reads, s.handle)
        let failed = copy(open.failed)
        let source = copy(open.source)
        remove(reads, s.handle)
        discard(s)
        if failed {
            // The recorded read failure, reported where the host reports it.
            return err(checked<StreamError>(InvalidUtf8 { source: source }))
        }
        return ok(None)
    }

    fn write(s: OutStream, text: Str) -> Long => s, text {
        return mem_append(writes, s.handle, to_bytes(text))
    }

    fn write_line(s: OutStream, text: Str) -> Long => s, text {
        return mem_append(writes, s.handle, to_bytes("${text}\n"))
    }

    fn write_bytes(s: OutStream, data: Bytes) -> Long => s, data {
        return mem_append(writes, s.handle, copy(data))
    }

    fn position(s: OutStream) -> Long => s {
        return to_long(size(mem_write_state(writes, s.handle).buffer))
    }

    fn flush(s: OutStream) -> Ok None | Err Checked<StreamError> => s {
        let open = mem_write_state(writes, s.handle)
        put(files, copy(open.path), copy(open.buffer))
        return ok(None)
    }

    fn close(s: OutStream) -> Ok None | Err Checked<StreamError> => !s {
        let open = mem_write_state(writes, s.handle)
        put(files, copy(open.path), copy(open.buffer))
        remove(writes, s.handle)
        discard(s)
        return ok(None)
    }
}

// Whether anything in [files] lives *under* [path] — which is what makes a
// directory exist in a filesystem whose directories are implicit.
fn fs_has_children(files: Map<Str, Bytes>, path: Str) [] -> Bool => files, path {
    let prefix = "${path}/"
    for key in files {
        if starts_with(key, prefix) {
            return true
        }
    }
    return false
}

// The index of the first `\n` at or after [from], or the end of [data] when
// there is none — which is where an unterminated last line stops.
fn mem_find_newline(data: Bytes, from: Int) [] -> Int => data {
    let end = size(data)
    let i = from
    while i < end {
        if to_int(get(data, i)!) == 10 {
            return i
        }
        i = i + 1
    }
    return end
}

// Appends [data] to the open write stream [handle], answering how many bytes
// it took — the one place all three write members meet, since they differ
// only in what they turn into bytes.
fn mem_append(writes: Mut Map<Long, MemWrite>, handle: Long, data: Bytes) [] -> Long => writes: Mut, handle, data {
    let open = mem_write_state(writes, handle)
    let grown = mut_bytes(open.buffer)
    grown.append(data)
    let buffer: Bytes = grown
    put(writes, copy(handle), MemWrite { path: copy(open.path), buffer: buffer })
    return to_long(size(data))
}


// [stream-provider] The state of an open stream, or a trap: a handle this
// table never minted is a stream from another provider — a host stream handed
// to `MemFs` — which is a program bug, not a condition to report.
fn mem_read_state(reads: Map<Long, MemRead>, handle: Long) [] -> MemRead => reads, handle {
    let open = get(reads, handle)
    if open is None {
        unreachable!("stream handle ${handle} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]")
    }
    return MemRead { source: copy(open.source), data: copy(open.data), at: open.at, failed: open.failed }
}

fn mem_write_state(writes: Map<Long, MemWrite>, handle: Long) [] -> MemWrite => writes, handle {
    let open = get(writes, handle)
    if open is None {
        unreachable!("stream handle ${handle} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]")
    }
    return MemWrite { path: copy(open.path), buffer: copy(open.buffer) }
}

// The reads, as functions over the handler's own state — see the note on the
// members that call them.

// The next line at the stream's position, without its terminator; absent at
// the end of the stream or after a strict-decode failure (which is recorded,
// so `close` can report it).
fn mem_read_line(reads: Mut Map<Long, MemRead>, handle: Long) [] -> Str | None => reads: Mut, handle {
    let open = mem_read_state(reads, handle)
    if open.failed {
        return None
    }
    let at = open.at
    let bytes: Bytes = copy(open.data)
    let end = size(bytes)
    if at >= end {
        return None
    }
    // Up to and including the `\n`, which is consumed but not returned.
    let stop = mem_find_newline(bytes, at)
    let line = slice(bytes, at, stop)!
    let next_at = stop
    if stop < end {
        next_at = stop + 1
    }
    let text = str_of_bytes(line)
    if text is None {
        put(reads, copy(handle), MemRead { source: copy(open.source), data: bytes, at: next_at, failed: true })
        return None
    }
    put(reads, copy(handle), MemRead { source: copy(open.source), data: bytes, at: next_at, failed: false })
    // `\r\n` and `\n` both end a line, and neither is part of it.
    return trim_suffix(text, "\r")
}

// Everything left in the stream, decoded strictly.
fn mem_read_all(reads: Mut Map<Long, MemRead>, handle: Long) [] -> Ok Str | Err Checked<StreamError> => reads: Mut, handle {
    let open = mem_read_state(reads, handle)
    let source: Str = copy(open.source)
    if open.failed {
        return err(checked<StreamError>(InvalidUtf8 { source: source }))
    }
    let bytes: Bytes = copy(open.data)
    let end = size(bytes)
    let rest = slice(bytes, open.at, end)!
    let text = str_of_bytes(rest)
    if text is None {
        put(reads, copy(handle), MemRead { source: copy(source), data: bytes, at: end, failed: true })
        return err(checked<StreamError>(InvalidUtf8 { source: copy(source) }))
    }
    put(reads, copy(handle), MemRead { source: copy(source), data: bytes, at: end, failed: false })
    return ok(text)
}

// Up to [max] bytes from the stream's position, undecoded.
fn mem_read_bytes(reads: Mut Map<Long, MemRead>, handle: Long, max: Int) [] -> Ok Bytes | Err Checked<StreamError> => reads: Mut, handle {
    let open = mem_read_state(reads, handle)
    let source: Str = copy(open.source)
    if open.failed {
        return err(checked<StreamError>(InvalidUtf8 { source: source }))
    }
    let bytes: Bytes = copy(open.data)
    let stop = open.at + max
    if max < 0 {
        stop = open.at
    }
    let end = size(bytes)
    if stop > end {
        stop = end
    }
    let taken = slice(bytes, open.at, stop)!
    put(reads, copy(handle), MemRead { source: copy(source), data: bytes, at: stop, failed: false })
    return ok(taken)
}
