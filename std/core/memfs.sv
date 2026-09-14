// An in-memory filesystem: a `MemFs` fakes the whole of `Fs`, streams
// included, so a test needs no disk and no host handler at all.
//
// A fresh `MemFs` is **empty**; write into it with the ordinary surface
// (`write_str`, `open_write`) and read it back. Directories are implicit:
// a path is a key, and a directory exists exactly while something under it
// does.
//
// Byte offsets are the hazard this type exists to get right: `position` and
// `open_read_at` count *bytes* here as they do on the host, while the
// content is sliced by characters — a fake that counted characters would let
// unit tests pass while production broke.

// What an open read stream is: which file, and how far into it the reader
// has consumed (in characters — `position` converts).
struct MemRead { path: Str, at: Int }

// What an open write stream is: which file, and what has been written so far.
// Closing (or flushing) publishes the buffer.
struct MemWrite { path: Str, buffer: Str }

handler MemFs of Fs {
    files: Mut Map<Str, Str> = mut_map_of()
    reads: Mut Map<Long, MemRead> = mut_map_of()
    writes: Mut Map<Long, MemWrite> = mut_map_of()
    next_handle: Long = 0

    fn open_read(path: Str) -> Ok InStream | Err FsError => path {
        let content = get(files, path)
        if content is None {
            return err(FsError { kind: NotFound { path: copy(path) } })
        }
        next_handle = next_handle + 1
        put(reads, copy(next_handle), MemRead { path: copy(path), at: 0 })
        return ok(InStream { handle: copy(next_handle) })
    }

    fn open_read_at(path: Str, offset: Long) -> Ok InStream | Err FsError => path {
        let content = get(files, path)
        if content is None {
            return err(FsError { kind: NotFound { path: copy(path) } })
        }
        let text: Str = copy(content)
        // Walk characters until the byte count reaches the offset. Landing
        // *between* the bytes of a character is the host's strict-decode
        // failure, reported the same way.
        let at = 0
        let seen: Long = 0
        while seen < offset {
            let chunk = substr(text, at, at + 1)
            if chunk is None {
                return err(FsError { kind: InvalidUtf8 { path: copy(path) } })
            }
            seen = seen + byte_size(chunk)
            at = at + 1
        }
        if seen > offset {
            return err(FsError { kind: InvalidUtf8 { path: copy(path) } })
        }
        next_handle = next_handle + 1
        put(reads, copy(next_handle), MemRead { path: copy(path), at: copy(at) })
        return ok(InStream { handle: copy(next_handle) })
    }

    fn open_write(path: Str) -> Ok OutStream | Err FsError => path {
        next_handle = next_handle + 1
        put(writes, copy(next_handle), MemWrite { path: copy(path), buffer: "" })
        return ok(OutStream { handle: copy(next_handle) })
    }

    fn open_append(path: Str) -> Ok OutStream | Err FsError => path {
        let existing = get(files, path)
        let start = "" 
        if existing is Str {
            start = copy(existing)
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
        if content is Str {
            return ok(FileInfo { size: byte_size(content), is_dir: false })
        }
        if fs_has_children(files, path) {
            return ok(FileInfo { size: 0, is_dir: true })
        }
        return err(FsError { kind: NotFound { path: copy(path) } })
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
        let text: Str = copy(content)
        remove(files, from)
        put(files, copy(to), text)
        return ok(None)
    }

    fn read_line(s: InStream) -> Str | None => s {
        let open = get(reads, s.handle)
        if open is None {
            return None
        }
        let at = open.at
        let content = get(files, open.path)
        if content is None {
            return None
        }
        let text: Str = copy(content)
        let rest = substr(text, at, size(text))
        if rest is None {
            return None
        }
        let tail: Str = copy(rest)
        if size(tail) == 0 {
            return None
        }
        let cut = index_of(tail, "\n")
        if cut is None {
            put(reads, s.handle, MemRead { path: copy(open.path), at: size(text) })
            return tail
        }
        let end: Int = copy(cut)
        let line = substr(tail, 0, end)
        put(reads, s.handle, MemRead { path: copy(open.path), at: at + end + 1 })
        if line is None {
            return None
        }
        return trim_suffix(line, "\r")
    }

    fn read_all(s: InStream) -> Ok Str | Err FsError => s {
        let open = get(reads, s.handle)
        if open is None {
            return err(FsError { kind: StaleHandle { path: "<stream>" } })
        }
        let content = get(files, open.path)
        if content is None {
            return err(FsError { kind: NotFound { path: copy(open.path) } })
        }
        let text: Str = copy(content)
        let rest = substr(text, open.at, size(text))
        put(reads, s.handle, MemRead { path: copy(open.path), at: size(text) })
        if rest is None {
            return ok("")
        }
        return ok(rest)
    }

    fn position(s: InStream) -> Long => s {
        let open = get(reads, s.handle)
        if open is None {
            return 0
        }
        let content = get(files, open.path)
        if content is None {
            return 0
        }
        let consumed = substr(content, 0, open.at)
        if consumed is None {
            return 0
        }
        return byte_size(consumed)
    }

    fn close(s: InStream) -> Ok None | Err FsError => !s {
        remove(reads, s.handle)
        discard(s)
        return ok(None)
    }

    fn write(s: OutStream, text: Str) -> Long => s, text {
        let open = get(writes, s.handle)
        if open is None {
            return 0
        }
        put(writes, s.handle, MemWrite {
            path: copy(open.path),
            buffer: "${open.buffer}${text}"
        })
        return byte_size(text)
    }

    fn write_line(s: OutStream, text: Str) -> Long => s, text {
        let open = get(writes, s.handle)
        if open is None {
            return 0
        }
        let line = "${text}\n"
        put(writes, s.handle, MemWrite {
            path: copy(open.path),
            buffer: "${open.buffer}${line}"
        })
        return byte_size(line)
    }

    fn position(s: OutStream) -> Long => s {
        let open = get(writes, s.handle)
        if open is None {
            return 0
        }
        return byte_size(open.buffer)
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
fn fs_has_children(files: Map<Str, Str>, path: Str) [] -> Bool => files, path {
    let prefix = "${path}/"
    for key in files {
        if starts_with(key, prefix) {
            return true
        }
    }
    return false
}
