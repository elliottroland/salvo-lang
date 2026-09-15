// An in-memory filesystem: a `MemFs` fakes the whole of `Fs`, streams
// included, so a test needs no disk and no host handler at all.
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

// What an open read stream is: which file, how far into its bytes the reader
// has consumed, and whether a strict decode has already failed on it (which
// ends iteration and surfaces at `close`).
struct MemRead { path: Str, at: Int, failed: Bool }

// What an open write stream is: which file, and the bytes written so far.
// Closing (or flushing) publishes the buffer.
struct MemWrite { path: Str, buffer: Bytes }

handler MemFs of Fs {
    files: Mut Map<Str, Bytes> = mut_map_of()
    reads: Mut Map<Long, MemRead> = mut_map_of()
    writes: Mut Map<Long, MemWrite> = mut_map_of()
    next_handle: Long = 0

    fn open_read(path: Str) -> Ok InStream | Err FsError => path {
        if !contains_key(files, path) {
            return err(FsError { kind: NotFound { path: copy(path) } })
        }
        next_handle = next_handle + 1
        put(reads, copy(next_handle), MemRead { path: copy(path), at: 0, failed: false })
        return ok(InStream { handle: copy(next_handle) })
    }

    fn open_read_at(path: Str, offset: Long) -> Ok InStream | Err FsError => path {
        let content = get(files, path)
        if content is None {
            return err(FsError { kind: NotFound { path: copy(path) } })
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
        next_handle = next_handle + 1
        put(reads, copy(next_handle), MemRead { path: copy(path), at: at, failed: false })
        return ok(InStream { handle: copy(next_handle) })
    }

    fn open_write(path: Str) -> Ok OutStream | Err FsError => path {
        next_handle = next_handle + 1
        let empty = mut_bytes()
        put(writes, copy(next_handle), MemWrite { path: copy(path), buffer: empty })
        return ok(OutStream { handle: copy(next_handle) })
    }

    fn open_append(path: Str) -> Ok OutStream | Err FsError => path {
        let existing = get(files, path)
        let start = mut_bytes()
        if existing is None {
            // Nothing there yet: appending starts from empty.
        } else {
            start.append(existing)
        }
        next_handle = next_handle + 1
        put(writes, copy(next_handle), MemWrite { path: copy(path), buffer: start })
        return ok(OutStream { handle: copy(next_handle) })
    }

    fn exists(path: Str) -> Bool => path {
        if contains_key(files, path) {
            return true
        }
        return fs_has_children(files, path)
    }

    fn metadata(path: Str) -> Ok FileInfo | Err FsError => path {
        let content = get(files, path)
        if content is None {
            if fs_has_children(files, path) {
                return ok(FileInfo { size: 0, is_dir: true })
            }
            return err(FsError { kind: NotFound { path: copy(path) } })
        }
        return ok(FileInfo { size: to_long(size(content)), is_dir: false })
    }

    fn list_dir(path: Str) -> Ok List<Str> | Err FsError => path {
        if contains_key(files, path) {
            return err(FsError { kind: NotADirectory { path: copy(path) } })
        }
        if !fs_has_children(files, path) {
            return err(FsError { kind: NotFound { path: copy(path) } })
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

    fn create_dirs(path: Str) -> Ok None | Err FsError => path {
        // Directories are implicit here: there is nothing to create, and
        // reporting success is what the host does for an existing tree.
        return ok(None)
    }

    fn delete(path: Str) -> Ok None | Err FsError => path {
        if contains_key(files, path) {
            remove(files, path)
            return ok(None)
        }
        if fs_has_children(files, path) {
            return err(FsError {
                kind: IoError { path: copy(path), message: "directory not empty" }
            })
        }
        return err(FsError { kind: NotFound { path: copy(path) } })
    }

    fn rename_path(from: Str, to: Str) -> Ok None | Err FsError => from, to {
        let content = get(files, from)
        if content is None {
            return err(FsError { kind: NotFound { path: copy(from) } })
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
        return mem_read_line(reads, files, s.handle)
    }

    fn read_all(s: InStream) -> Ok Str | Err FsError => s {
        return mem_read_all(reads, files, s.handle)
    }

    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err FsError => s {
        return mem_read_bytes(reads, files, s.handle, max)
    }

    // The fill-a-buffer reads [fs-read-to]: each is its returning sibling with
    // the destination handed in, so the fake has the same three shapes the
    // host does.
    fn read_to(s: InStream, buf: Mut Bytes, max: Int) -> Ok Int | Err FsError => s, buf: Mut {
        let got = mem_read_bytes(reads, files, s.handle, max)
        if got is Err {
            return got
        }
        let data: Bytes = got
        buf.append(data)
        return ok(size(data))
    }

    fn read_to(s: InStream, buf: Mut Str) -> Ok Long | Err FsError => s, buf: Mut {
        let got = mem_read_all(reads, files, s.handle)
        if got is Err {
            return got
        }
        let text: Str = got
        buf.append(text)
        return ok(byte_size(text))
    }

    fn read_line_to(s: InStream, buf: Mut Str) -> Bool => s, buf: Mut {
        let line = mem_read_line(reads, files, s.handle)
        when line {
            is Str {
                buf.append(line)
                return true
            }
            is None { return false }
        }
    }

    fn position(s: InStream) -> Long => s {
        let open = get(reads, s.handle)
        if open is None {
            return 0
        }
        // The bytes consumed, and nothing to convert: the store *is* bytes.
        return to_long(open.at)
    }
    fn close(s: InStream) -> Ok None | Err FsError => !s {
        let open = get(reads, s.handle)
        let failed = false
        let path = "<stream>"
        if open is MemRead {
            failed = copy(open.failed)
            path = copy(open.path)
        }
        remove(reads, s.handle)
        discard(s)
        if failed {
            // The recorded read failure, reported where the host reports it.
            return err(FsError { kind: InvalidUtf8 { path: path } })
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
        let open = get(writes, s.handle)
        if open is None {
            return 0
        }
        return to_long(size(open.buffer))
    }

    fn flush(s: OutStream) -> Ok None | Err FsError => s {
        let open = get(writes, s.handle)
        if open is None {
            return err(FsError { kind: StaleHandle { path: "<stream>" } })
        }
        put(files, copy(open.path), copy(open.buffer))
        return ok(None)
    }

    fn close(s: OutStream) -> Ok None | Err FsError => !s {
        let open = get(writes, s.handle)
        if open is None {
            discard(s)
            return err(FsError { kind: StaleHandle { path: "<stream>" } })
        }
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
    let open = get(writes, handle)
    if open is None {
        return 0
    }
    let grown = mut_bytes(open.buffer)
    grown.append(data)
    let buffer: Bytes = grown
    put(writes, copy(handle), MemWrite { path: copy(open.path), buffer: buffer })
    return to_long(size(data))
}


// The reads, as functions over the handler's own state — see the note on the
// members that call them.

// The next line at the stream's position, without its terminator; absent at
// the end of the file or after a strict-decode failure (which is recorded, so
// `close` can report it).
fn mem_read_line(reads: Mut Map<Long, MemRead>, files: Map<Str, Bytes>, handle: Long) [] -> Str | None => reads: Mut, files, handle {
    let open = get(reads, handle)
    if open is None {
        return None
    }
    if open.failed {
        return None
    }
    let at = open.at
    let path: Str = copy(open.path)
    let content = get(files, path)
    if content is None {
        return None
    }
    let bytes: Bytes = copy(content)
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
        put(reads, copy(handle), MemRead { path: copy(path), at: next_at, failed: true })
        return None
    }
    put(reads, copy(handle), MemRead { path: copy(path), at: next_at, failed: false })
    // `\r\n` and `\n` both end a line, and neither is part of it.
    return trim_suffix(text, "\r")
}

// Everything left in the stream, decoded strictly.
fn mem_read_all(reads: Mut Map<Long, MemRead>, files: Map<Str, Bytes>, handle: Long) [] -> Ok Str | Err FsError => reads: Mut, files, handle {
    let open = get(reads, handle)
    if open is None {
        return err(FsError { kind: StaleHandle { path: "<stream>" } })
    }
    let path: Str = copy(open.path)
    if open.failed {
        return err(FsError { kind: InvalidUtf8 { path: path } })
    }
    let content = get(files, path)
    if content is None {
        return err(FsError { kind: NotFound { path: copy(path) } })
    }
    let bytes: Bytes = copy(content)
    let end = size(bytes)
    let rest = slice(bytes, open.at, end)!
    let text = str_of_bytes(rest)
    if text is None {
        put(reads, copy(handle), MemRead { path: copy(path), at: end, failed: true })
        return err(FsError { kind: InvalidUtf8 { path: copy(path) } })
    }
    put(reads, copy(handle), MemRead { path: copy(path), at: end, failed: false })
    return ok(text)
}

// Up to [max] bytes from the stream's position, undecoded.
fn mem_read_bytes(reads: Mut Map<Long, MemRead>, files: Map<Str, Bytes>, handle: Long, max: Int) [] -> Ok Bytes | Err FsError => reads: Mut, files, handle {
    let open = get(reads, handle)
    if open is None {
        return err(FsError { kind: StaleHandle { path: "<stream>" } })
    }
    let path: Str = copy(open.path)
    if open.failed {
        return err(FsError { kind: InvalidUtf8 { path: path } })
    }
    let content = get(files, path)
    if content is None {
        return err(FsError { kind: NotFound { path: copy(path) } })
    }
    let bytes: Bytes = copy(content)
    let stop = open.at + max
    if max < 0 {
        stop = open.at
    }
    let end = size(bytes)
    if stop > end {
        stop = end
    }
    let taken = slice(bytes, open.at, stop)!
    put(reads, copy(handle), MemRead { path: copy(path), at: stop, failed: false })
    return ok(taken)
}
