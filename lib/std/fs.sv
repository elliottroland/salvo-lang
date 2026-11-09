// Files and directories: the `Fs` effect, which a program declares to use
// the filesystem, its errors, and the one-call reads and writes built on it.
//
// **The effect.** A function that touches files declares `[Fs]`. The
// operations of [Fs] work on paths: open a file, test that it exists, read
// its metadata, list, create, delete and rename. Opening a file answers a
// **stream** (`InStream` or `OutStream`, from `std.stream`), and everything
// that reads or writes it, closing included, is the `Streams` effect's, so a
// file is read exactly like a network body. `Streams` is a prerequisite of
// `Fs`: wherever `Fs` is available so is `Streams`, and a function that
// opens and reads a file still declares `[Fs]` alone.
//
// **Choosing the filesystem.** Which filesystem `Fs` reaches is decided by
// the handler bound around the code, at the program's entry point. The
// code that uses files does not change:
//
// - the machine's, from `std.fs.host` over `std.stream.host`: four `use`
//   lines, streams first;
// - [MemFs] (`std.fs.mem`), an in-memory filesystem with no disk, for
//   tests;
// - [RestrictedFs] (`std.fs.restricted`), which wraps either of them and
//   confines the code under it to one directory.
//
// Each lives in its own module, so `import std.fs` brings this surface and
// nothing else; [Path] comes from `import std.fs.path`, and a program that
// names `InStream` or `StreamError` imports `std.stream`.
//
// ```
// import std.fs
// import std.fs.path
// import std.fs.host
// import std.stream.host
//
// fn count_lines(name: Str) [Fs] -> Int => name {
//     let all = read_lines(path(name))
//     when all {
//         is Ok { return size(all) }
//         is Err {
//             ignore(all)
//             return 0
//         }
//     }
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use HostRawStreams()
//     use DefaultStreams()
//     use HostRawFs()
//     use DefaultFs()
//     let wrote = write_str(path("notes.txt"), "alpha\nbeta\n")
//     if wrote is Err {
//         println("cannot write: ${detach(wrote)}")
//         return None
//     }
//     println("${count_lines("notes.txt")} lines")       // 2 lines
// }
// ```
//
// A test (in a `.test.sv` file) runs the same code against [MemFs], which
// starts empty and touches no disk:
//
// ```
// import std.fs
// import std.fs.path
// import std.fs.mem
//
// test "a written file reads back" {
//     use MemFs()
//     let wrote = write_str(path("data/a.txt"), "hello")
//     expect(wrote is Ok, "wrote the file")
//     let back = read_to_str(path("data/a.txt"))
//     if back is Ok {
//         expect_eq(back, "hello")
//     } else {
//         ignore(back)
//         throw(Failure { message: "read failed" })
//     }
// }
// ```
//
// **Errors must be looked at.** Every operation that can fail answers `Ok
// value | Err Checked<FsError>`, and nothing is thrown. A `Checked` error is
// linear: the compiler refuses a program that drops it unread. Acknowledge
// one with `ignore(e)`, take the plain [FsError] out with `detach(e)` to
// print or store it, or narrow the result to its `Ok` arm, which shows
// there was no error. A one-call function that also reads or writes the
// file reports a stream failure as the [Streaming] kind, so it has one
// error type.
//
// **Streams must be closed.** An `InStream` or `OutStream` is linear too: a
// path through the code that does not `close` it does not compile. The
// one-call functions ([read_to_str], [read_lines], [write_str],
// [read_to_bytes], [write_bytes_to], [copy_file]) open, work and close, so
// no stream reaches the caller; reach for [open_read] and the others when
// the file is too big to hold at once or needs reading in parts.
//
// **Offsets are bytes.** Files hold bytes. A stream's `position`, the
// count a write answers and the offset [open_read_at] takes are all byte
// counts, which `byte_size` gives for a string; `size` counts characters.
// Text is decoded as strict UTF-8, so bytes that are not valid UTF-8 are an
// `InvalidUtf8` error rather than garbled text. Streams only move forward:
// to read from an offset, open the file again at it.
// @compilerrules [effect-prereq, mod-import-module, type-checked]

// The layering, bottom to top. `RawFs` (`fs.host`) trades in plain `Long`
// handles and bare `FsError`s; its one implementation is the host's
// (`platform handler HostRawFs`), which registers what it opens in the
// host's stream table, so no target-language class ever holds a Salvo
// obligation. `Fs` is what programs declare; its handler `DefaultFs
// [RawFs]` mints the linear tokens and wraps failures in
// `Checked<FsError>`, and the dependency is supplied at `use` and appears
// in nobody's signature. The one-shots are ordinary Salvo over `Fs` and
// `Streams` members, so a double that fakes both fakes them too.

import std.fs.path
import std.stream

// ===== errors =====

// Why a filesystem operation failed: [NotFound], [PermissionDenied] or one
// of the other kinds that follow it here.
//
// An `FsError` is a plain value, with no obligation of its own: it is what
// `detach` hands back from the `Checked<FsError>` an operation answers, so
// it can be printed, compared by kind and kept in a list. Each kind but
// [Streaming] names the path as text; interpolating one gives a readable
// message.
//
// ```
// fn describe(e: FsError) [] -> Str => e {
//     if e is NotFound {
//         return "missing: ${e.path}"
//     }
//     return to_str(e)
// }
// ```
// @compilerrules [type-checked]
export type FsError = NotFound | PermissionDenied | AlreadyExists | NotADirectory
                 | PathEscapes | IoError | Streaming

// Nothing exists at the path.
export struct NotFound { path: Str }
// Something exists at the path, but the operation was not permitted on it.
export struct PermissionDenied { path: Str }
// Something exists at the path where the operation needed nothing to be.
export struct AlreadyExists { path: Str }
// A part of the path that had to be a directory is not one, such as a file
// passed to [list_dir].
export struct NotADirectory { path: Str }
// [RestrictedFs] refused the path, because it resolves outside the
// directory the handler is confined to.
export struct PathEscapes { path: Str }
// Any other failure the filesystem reported, with its own description in
// [message].
export struct IoError { path: Str, message: Str }
// Reading or writing the file's stream failed: [error] says why. A one-call
// function such as [read_to_str] reports a failed read this way. It wraps
// the `StreamError` rather than adding its kinds to this union, so each
// error has exactly one text form.
// @compilerrules [interp-to-str]
export struct Streaming { error: StreamError }

// An error that must be acknowledged: `ignore` it, `detach` it, or narrow the
// result that carries it to its `Ok` arm — which is what "it was fine" costs.
// The obligation is `Checked<FsError>`, so the mechanism is the
// one every "you must look at this" answer in std uses, and `ignore`/`detach`
// are the generic ones.
// @compilerrules [type-checked]

// A one-line description of the failure, naming the path, such as
// `no such file or directory: notes.txt`. This is what interpolating an
// [FsError] prints.
// @compilerrules [interp-to-str]
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

// What [metadata] reports about a path: its [size] in bytes, and [is_dir],
// whether it is a directory.
export struct FileInfo { size: Long, is_dir: Bool }

// The filesystem, as an effect: the operations on paths. Declare `[Fs]` on
// a function that uses files.
//
// Opening a file answers a stream registered with the `Streams` handler
// bound around the code. `Streams` is a prerequisite of `Fs`, so the
// stream's reads, writes and `close` are available wherever `Fs` is,
// without declaring `Streams` too. Every failure is answered as `Err
// Checked<FsError>`, never thrown: an effect's operations cannot throw.
//
// Which filesystem this is depends on the handler: `DefaultFs` from
// `std.fs.host` for the machine's, [MemFs] for one in memory, [RestrictedFs]
// to confine either to a directory. A handler of your own that implements
// `Fs` and `Streams` replaces the whole surface, one-call functions
// included.
//
// ```
// import std.fs
// import std.fs.path
// import std.stream
//
// fn first_line(file: Path) [Fs] -> Str? => file {
//     let opened = open_read(file)
//     if opened is Err {
//         ignore(opened)
//         return None
//     }
//     let s: InStream = opened
//     let line = read_line(s)
//     let closed = close(s)
//     if closed is Err {
//         ignore(closed)
//     }
//     return line
// }
// ```
// @compilerrules [effect-prereq]
export effect Fs [Streams] {
    // Opens a file for reading from its start, answering a stream that must be closed.
    fn open_read(path: Path) -> Ok InStream | Err Checked<FsError> => path
    // Opens a file for reading from a byte offset. Streams only move forward, so this is how a reader starts again further on.
    fn open_read_at(path: Path, offset: Long) -> Ok InStream | Err Checked<FsError> => path
    // Opens a file for writing, creating it, or replacing what is there if it exists.
    fn open_write(path: Path) -> Ok OutStream | Err Checked<FsError> => path
    // Opens a file for writing at its end, creating it if it does not exist.
    fn open_append(path: Path) -> Ok OutStream | Err Checked<FsError> => path
    // Whether anything, a file or a directory, exists at the path. It has no error: a path that cannot be reached does not exist.
    fn exists(path: Path) -> Bool => path
    // The path's size and whether it is a directory.
    fn metadata(path: Path) -> Ok FileInfo | Err Checked<FsError> => path
    // The names (not paths) of the entries directly inside a directory, read all at once; `NotADirectory` for a file.
    fn list_dir(path: Path) -> Ok List<Str> | Err Checked<FsError> => path
    // Creates the directory and every missing directory above it; succeeds when it already exists.
    fn create_dirs(path: Path) -> Ok None | Err Checked<FsError> => path
    // Deletes a file, or a directory that is empty.
    fn delete(path: Path) -> Ok None | Err Checked<FsError> => path
    // Renames or moves a path, replacing what is at the destination if anything is.
    fn rename_path(from: Path, to: Path) -> Ok None | Err Checked<FsError> => from, to
}

// ===== opening as a sequence =====

// Opens a file as an iterator over its lines, read as the loop asks for
// them.
//
// The iterator (`Lines`, from `std.stream`) owns the open stream, so
// closing the iterator closes the file, and it must be closed. The lines
// come without their line endings. Use it for a file too big to read at
// once; [read_lines] reads every line in one call.
//
// ```
// import std.fs
// import std.fs.path
// import std.stream
//
// fn longest(file: Path) [Fs] -> Int => file {
//     let opened = open_lines(file)
//     if opened is Err {
//         ignore(opened)
//         return 0
//     }
//     let ls = opened
//     let best = 0
//     for line in ls {
//         if size(line) > best {
//             best = size(line)
//         }
//     }
//     let closed = close(ls)
//     if closed is Err {
//         ignore(closed)
//     }
//     return best
// }
// ```
export fn open_lines(path: Path) [Fs] -> Ok Mut Lines | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    return ok(lines(opened))
}

// Opens a file as an iterator over its bytes, in chunks of up to [size]
// bytes each.
//
// Each chunk is a new `Bytes`, so a chunk can be kept after the loop moves
// on. The iterator (`Chunks`, from `std.stream`) owns the open stream and
// must be closed, which closes the file.
//
// ```
// import std.fs
// import std.fs.path
// import std.stream
//
// fn total_bytes(file: Path) [Fs] -> Int => file {
//     let opened = open_chunks(file, 65536)
//     if opened is Err {
//         ignore(opened)
//         return 0
//     }
//     let cs = opened
//     let n = 0
//     for chunk in cs {
//         n = n + size(chunk)
//     }
//     let closed = close(cs)
//     if closed is Err {
//         ignore(closed)
//     }
//     return n
// }
// ```
export fn open_chunks(path: Path, size: Int) [Fs] -> Ok Mut Chunks | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    return ok(chunks(opened, size))
}

// ===== the one-shots =====

// Reads a whole file as text. It opens the file, reads it and closes it,
// so no stream reaches the caller.
//
// The bytes are decoded as strict UTF-8: a file that is not valid UTF-8 is
// an error (the [Streaming] kind, holding `InvalidUtf8`), never text with
// replacement characters. For data that is not text, use [read_to_bytes].
//
// ```
// import std.fs
// import std.fs.path
//
// fn config(name: Str) [Fs] -> Str => name {
//     let text = read_to_str(path(name))
//     when text {
//         is Ok { return text }
//         is Err {
//             ignore(text)
//             return ""
//         }
//     }
// }
// ```
// @returns  the file's text, or the first failure of the open, the read or the close
export fn read_to_str(path: Path) [Fs] -> Ok Str | Err Checked<FsError> => path {
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

// Reads a whole file as a list of its lines, without their line endings.
// It opens the file, reads every line and closes it.
//
// A line ends at `\n` or `\r\n`. A file that is not valid UTF-8 is an
// error, as for [read_to_str]. To handle the lines one at a time without
// holding them all, use [open_lines].
//
// ```
// import std.fs
// import std.fs.path
//
// fn entries(name: Str) [Fs] -> Int => name {
//     let all = read_lines(path(name))
//     when all {
//         is Ok { return size(all) }
//         is Err {
//             ignore(all)
//             return 0
//         }
//     }
// }
// ```
export fn read_lines(path: Path) [Fs] -> Ok List<Str> | Err Checked<FsError> => path {
    let opened = open_read(path)
    if opened is Err {
        return opened
    }
    let p = lines(opened)
    let out: Mut List<Str> = Mut List()
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

// Writes text to a file, creating it or replacing what is there, and
// answers how many bytes the text took in UTF-8.
//
// A write that fails is reported when the file is closed, which this
// function does before answering, so an `Ok` means the text was written.
//
// ```
// import std.fs
// import std.fs.path
//
// fn save(note: Str) [Fs, Console] -> None => note {
//     let wrote = write_str(path("note.txt"), note)
//     when wrote {
//         is Ok { println("saved ${wrote} bytes") }
//         is Err { println("not saved: ${detach(wrote)}") }
//     }
// }
// ```
export fn write_str(path: Path, content: Str) [Fs] -> Ok Long | Err Checked<FsError> => path, content {
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

// Reads a whole file as bytes, with nothing decoded: [read_to_str] for data
// that is not text.
//
// ```
// import std.fs
// import std.fs.path
//
// fn digest_input(name: Str) [Fs] -> Str => name {
//     let data = read_to_bytes(path(name))
//     when data {
//         is Ok { return to_hex(data) }
//         is Err {
//             ignore(data)
//             return ""
//         }
//     }
// }
// ```
export fn read_to_bytes(path: Path) [Fs] -> Ok Bytes | Err Checked<FsError> => path {
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

// Writes bytes to a file, creating it or replacing what is there, and
// answers how many it wrote. A failed write is reported, as for
// [write_str].
//
// ```
// import std.fs
// import std.fs.path
//
// fn save_header() [Fs] -> None {
//     let wrote = write_bytes_to(path("header.bin"), bytes_of(to_byte(1), to_byte(2)))
//     if wrote is Err {
//         ignore(wrote)
//     }
// }
// ```
export fn write_bytes_to(path: Path, data: Bytes) [Fs] -> Ok Long | Err Checked<FsError> => path, data {
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

// Copies the file at [from] to [to], creating [to] or replacing what is
// there, and answers how many bytes it copied.
//
// The bytes move through a buffer that is reused, so a large file is never
// held in memory whole, and no stream reaches the caller. Both files are
// closed whatever happens; when several steps fail, the first failure is
// the one answered.
//
// ```
// import std.fs
// import std.fs.path
//
// fn backup(file: Path) [Fs] -> Bool => file {
//     let copied = copy_file(file, with_extension(file, "bak"))
//     if copied is Err {
//         ignore(copied)
//         return false
//     }
//     return true
// }
// ```
export fn copy_file(from: Path, to: Path) [Fs] -> Ok Long | Err Checked<FsError> => from, to {
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
