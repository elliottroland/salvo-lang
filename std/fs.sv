// The filesystem surface: an `Fs` effect for *paths*, and the one-shots
// built on it. Opening a file mints a stream into the `Streams` in scope —
// `effect Fs [Streams]` [effect-prereq] — and everything that reads or writes
// the stream is `stream`'s, so a file and a network body are one kind of
// stream, read one way.
//
// This module is the *surface* — errors, the effect, the one-shots. The
// filesystem of the machine the program runs on lives next door in `fs.host`
// (`RawFs`, `HostRawFs`, `DefaultFs`), over the host's streams in
// `stream.host`; `fs.mem`'s `MemFs` fakes both at once. A whole-module import
// names one module, so `import fs` brings neither [mod-import-module] — and
// neither `stream`: a program that names `InStream` imports it.
//
// The layering, bottom to top:
//
//   * `RawFs` (`fs.host`) trades in plain `Long` handles and bare
//     `FsError`s. Its one implementation is the host's
//     (`platform handler HostRawFs`), which registers what it opens in the
//     host's stream table, so no target-language class ever holds a Salvo
//     obligation.
//   * `Fs` is what programs declare. Its handler `DefaultFs [RawFs]` mints
//     the linear tokens and wraps failures in `Checked<FsError>`; the
//     dependency is supplied at `use` and appears in nobody's signature.
//   * The one-shots are ordinary Salvo over `Fs` and `Streams` members, so a
//     double that fakes both fakes them too.

import stream

// ===== errors =====

// What went wrong: droppable, storable, and what `detach` hands back for
// aggregation. Every failing member answers `Err Checked<FsError>`, so the
// error cannot be dropped in silence — but the value inside is an ordinary
// union with no obligation of its own [checked-type]. [Streaming] carries a
// `stream` failure: a one-shot that opens *and* reads reports either kind
// through one type.
export type FsError = NotFound | PermissionDenied | AlreadyExists | NotADirectory
                 | PathEscapes | IoError | Streaming

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
// Anything the host reported that the kinds above do not name.
export struct IoError { path: Str, message: Str }
// Reading or writing the file's stream failed — why is [error]'s. Kept apart
// from `stream`'s kinds rather than merged into this union, so a value of
// either type has exactly one text form [interp-to-str].
export struct Streaming { error: StreamError }

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
        is IoError { return "io error: ${kind.path}: ${kind.message}" }
        is Streaming { return to_str(kind.error) }
    }
}

// A stream failure, as the filesystem's: what a one-shot answers when the read
// behind it failed.
fn fs_stream_error(e: Checked<StreamError>) [] -> Checked<FsError> => !e {
    return checked<FsError>(Streaming { error: detach(e) })
}

// ===== the effect programs use =====

// What a file's metadata says. Enough for the operations v1 has.
export struct FileInfo { size: Long, is_dir: Bool }

// The filesystem: the operations on *paths*. Opening mints a stream into the
// `Streams` bound around it, and `Streams` is a prerequisite [effect-prereq],
// so application code still declares `[Fs]` alone and reads what it opened.
//
// Every failure is a returned `Err Checked<FsError>`, never a `[Throw]`: an
// effect member may declare no effects of its own.
export effect Fs [Streams] {
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
}

// ===== opening as a sequence =====

// Opens a file as a sequence of its lines ([Lines] is `stream`'s).
export fn open_lines(path: Str) [Fs] -> Ok Mut Lines | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    return ok(lines(opened))
}

// Opens a file as a sequence of chunks of up to [size] bytes.
export fn open_chunks(path: Str, size: Int) [Fs] -> Ok Mut Chunks | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    return ok(chunks(opened, size))
}

// ===== the one-shots =====

// A whole file as text: the 90% case, and no token reaches the caller.
export fn read_to_str(path: Str) [Fs] -> Ok Str | Err Checked<FsError> => path {
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
        return err(fs_stream_error(content))
    }
    let closed = close(s)
    if closed is Err {
        return err(fs_stream_error(closed))
    }
    return ok(content)
}

// A whole file as its lines, eagerly.
export fn read_lines(path: Str) [Fs] -> Ok List<Str> | Err Checked<FsError> => path {
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
        return err(fs_stream_error(closed))
    }
    let done: List<Str> = out
    return ok(done)
}

// Writes text to a file, creating it or replacing what is there, and answers
// how many bytes it took.
export fn write_str(path: Str, content: Str) [Fs] -> Ok Long | Err Checked<FsError> => path, content {
    let opened = open_write(path)
    if opened is Err {
        return opened
    }
    let s: OutStream = opened
    let written = write(s, content)
    let closed = close(s)
    if closed is Err {
        return err(fs_stream_error(closed))
    }
    return ok(written)
}

// The whole of a file as bytes: `read_to_str` for data that is not text.
export fn read_to_bytes(path: Str) [Fs] -> Ok Bytes | Err Checked<FsError> => path {
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
        return err(fs_stream_error(filling))
    }
    let closed = close(s)
    if closed is Err {
        return err(fs_stream_error(closed))
    }
    let done: Bytes = buf
    return ok(done)
}

// Writes bytes to a file, creating it or replacing what is there, and answers
// how many it took.
export fn write_bytes_to(path: Str, data: Bytes) [Fs] -> Ok Long | Err Checked<FsError> => path, data {
    let opened = open_write(path)
    if opened is Err {
        return opened
    }
    let s: OutStream = opened
    let written = write_bytes(s, data)
    let closed = close(s)
    if closed is Err {
        return err(fs_stream_error(closed))
    }
    return ok(written)
}

// ===== copying =====

// Copies the file at [from] onto [to], creating it or replacing what is there,
// and answers how many bytes moved. The one-shot: no token, no buffer and no
// stream reaches the caller.
export fn copy_file(from: Str, to: Str) [Fs] -> Ok Long | Err Checked<FsError> => from, to {
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
        return err(fs_stream_error(moved))
    }
    if shut_w is Err {
        if shut_s is Err {
            ignore(shut_s)
        }
        return err(fs_stream_error(shut_w))
    }
    if shut_s is Err {
        return err(fs_stream_error(shut_s))
    }
    return ok(moved)
}
