// The filesystem of the machine the program runs on: the raw host seam and
// the handler that turns it into `core.fs`'s `Fs`.
//
// Its own module, and deliberately: `core.fs` is reachable from any program
// that iterates or interpolates (it declares a `next` and a `to_str`), while
// nothing reaches *this* module unless the program names `HostRawFs` or
// `DefaultFs`. A dependent handler switches the whole program to the fused
// effect emission, so a filesystem nobody uses must not be linked.

// ===== the raw seam =====

// The host's filesystem, in plain values: handles are `Long`s and failures
// are droppable kinds, so the platform class implements an interface with no
// obligations in it. Reachable only by declaring `[RawFs]`, which nothing
// but a composition root and `DefaultFs` does — which is what keeps the
// audit a grep.
//
// Read errors are *recorded* by the host rather than returned: a failed
// `raw_read_line` reports the end of the stream and `raw_close_read`
// reports why. Write errors behave the same way, surfacing at
// `raw_flush`/`raw_close_write`.
effect RawFs {
    // Opens a file for reading, from the beginning.
    fn raw_open_read(path: Str) -> Ok Long | Err FsErrorKind => path
    // Opens a file for reading, positioned at a byte offset.
    fn raw_open_read_at(path: Str, offset: Long) -> Ok Long | Err FsErrorKind => path
    // Opens a file for writing, creating it or truncating what is there.
    fn raw_open_write(path: Str) -> Ok Long | Err FsErrorKind => path
    // Opens a file for writing at its end, creating it if absent.
    fn raw_open_append(path: Str) -> Ok Long | Err FsErrorKind => path
    // Whether anything exists at the path.
    fn raw_exists(path: Str) -> Bool => path
    // The path's metadata.
    fn raw_metadata(path: Str) -> Ok FileInfo | Err FsErrorKind => path
    // The entry names directly inside a directory, eagerly.
    fn raw_list_dir(path: Str) -> Ok List<Str> | Err FsErrorKind => path
    // Creates the directory and every missing parent of it.
    fn raw_create_dirs(path: Str) -> Ok None | Err FsErrorKind => path
    // Deletes a file, or an empty directory.
    fn raw_delete(path: Str) -> Ok None | Err FsErrorKind => path
    // Renames a path, replacing the destination if it exists.
    fn raw_rename_path(from: Str, to: Str) -> Ok None | Err FsErrorKind => from, to

    // The next line, without its terminator; absent at the end of the
    // stream *or* after a recorded failure.
    fn raw_read_line(handle: Long) -> Str | None
    // Everything left in the stream.
    fn raw_read_all(handle: Long) -> Ok Str | Err FsErrorKind
    // Bytes consumed so far, counted below the decoder.
    fn raw_read_position(handle: Long) -> Long
    // Releases the read handle, reporting any failure recorded on it.
    fn raw_close_read(handle: Long) -> Ok None | Err FsErrorKind

    // Accepts text, answering how many bytes of it were written.
    fn raw_write(handle: Long, text: Str) -> Long => text
    // Bytes accepted so far.
    fn raw_write_position(handle: Long) -> Long
    // Pushes accepted bytes to the host, reporting a recorded failure.
    fn raw_flush(handle: Long) -> Ok None | Err FsErrorKind
    // Flushes and releases the write handle.
    fn raw_close_write(handle: Long) -> Ok None | Err FsErrorKind
}

// The one implementation of `RawFs`: a host class per backend, shipped with
// std [platform-handler].
platform handler HostRawFs of RawFs

handler DefaultFs [RawFs] of Fs {
    fn open_read(path: Str) -> Ok InStream | Err FsError => path {
        let r = raw_open_read(path)
        when r {
            is Ok { return ok(InStream { handle: r }) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn open_read_at(path: Str, offset: Long) -> Ok InStream | Err FsError => path {
        let r = raw_open_read_at(path, offset)
        when r {
            is Ok { return ok(InStream { handle: r }) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn open_write(path: Str) -> Ok OutStream | Err FsError => path {
        let r = raw_open_write(path)
        when r {
            is Ok { return ok(OutStream { handle: r }) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn open_append(path: Str) -> Ok OutStream | Err FsError => path {
        let r = raw_open_append(path)
        when r {
            is Ok { return ok(OutStream { handle: r }) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn exists(path: Str) -> Bool => path {
        return raw_exists(path)
    }

    fn metadata(path: Str) -> Ok FileInfo | Err FsError => path {
        let r = raw_metadata(path)
        when r {
            is Ok { return ok(r) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn list_dir(path: Str) -> Ok List<Str> | Err FsError => path {
        let r = raw_list_dir(path)
        when r {
            is Ok { return ok(r) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn create_dirs(path: Str) -> Ok None | Err FsError => path {
        let r = raw_create_dirs(path)
        when r {
            is Ok { return ok(None) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn delete(path: Str) -> Ok None | Err FsError => path {
        let r = raw_delete(path)
        when r {
            is Ok { return ok(None) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn rename_path(from: Str, to: Str) -> Ok None | Err FsError => from, to {
        let r = raw_rename_path(from, to)
        when r {
            is Ok { return ok(None) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn read_line(s: InStream) -> Str | None => s {
        return raw_read_line(s.handle)
    }

    fn read_all(s: InStream) -> Ok Str | Err FsError => s {
        let r = raw_read_all(s.handle)
        when r {
            is Ok { return ok(r) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn position(s: InStream) -> Long => s {
        return raw_read_position(s.handle)
    }

    fn close(s: InStream) -> Ok None | Err FsError => !s {
        let r = raw_close_read(s.handle)
        discard(s)
        when r {
            is Ok { return ok(None) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn write(s: OutStream, text: Str) -> Long => s, text {
        return raw_write(s.handle, text)
    }

    fn write_line(s: OutStream, text: Str) -> Long => s, text {
        return raw_write(s.handle, "${text}\n")
    }

    fn position(s: OutStream) -> Long => s {
        return raw_write_position(s.handle)
    }

    fn flush(s: OutStream) -> Ok None | Err FsError => s {
        let r = raw_flush(s.handle)
        when r {
            is Ok { return ok(None) }
            is Err { return err(FsError { kind: r }) }
        }
    }

    fn close(s: OutStream) -> Ok None | Err FsError => !s {
        let r = raw_close_write(s.handle)
        discard(s)
        when r {
            is Ok { return ok(None) }
            is Err { return err(FsError { kind: r }) }
        }
    }
}
