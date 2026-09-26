// The filesystem of the machine the program runs on: the raw host seam and
// the handler that turns it into `fs`'s `Fs`.
//
// Its own module, and deliberately: a program that fakes the filesystem
// (`fs.mem`) imports `fs` without reaching this module at all. `DefaultFs` is
// a *dependent* handler, and a reachable dependent handler switches the whole
// program to the fused effect emission, so the handler nobody in a test uses
// must not arrive with the surface.

import fs

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
export effect RawFs {
    // Opens a file for reading, from the beginning.
    fn raw_open_read(path: Str) -> Ok Long | Err FsError => path
    // Opens a file for reading, positioned at a byte offset.
    fn raw_open_read_at(path: Str, offset: Long) -> Ok Long | Err FsError => path
    // Opens a file for writing, creating it or truncating what is there.
    fn raw_open_write(path: Str) -> Ok Long | Err FsError => path
    // Opens a file for writing at its end, creating it if absent.
    fn raw_open_append(path: Str) -> Ok Long | Err FsError => path
    // Whether anything exists at the path.
    fn raw_exists(path: Str) -> Bool => path
    // The path's metadata.
    fn raw_metadata(path: Str) -> Ok FileInfo | Err FsError => path
    // The entry names directly inside a directory, eagerly.
    fn raw_list_dir(path: Str) -> Ok List<Str> | Err FsError => path
    // Creates the directory and every missing parent of it.
    fn raw_create_dirs(path: Str) -> Ok None | Err FsError => path
    // Deletes a file, or an empty directory.
    fn raw_delete(path: Str) -> Ok None | Err FsError => path
    // Renames a path, replacing the destination if it exists.
    fn raw_rename_path(from: Str, to: Str) -> Ok None | Err FsError => from, to

    // The next line, without its terminator; absent at the end of the
    // stream *or* after a recorded failure.
    fn raw_read_line(handle: Long) -> Str | None
    // Everything left in the stream.
    fn raw_read_all(handle: Long) -> Ok Str | Err FsError
    // Up to `max` bytes, undecoded.
    fn raw_read_bytes(handle: Long, max: Int) -> Ok Bytes | Err FsError
    // Up to `max` bytes appended to `buf`, answering how many.
    fn raw_read_to_bytes(handle: Long, buf: Mut Bytes, max: Int) -> Ok Int | Err FsError => buf: Mut
    // Everything left, decoded strictly and appended to `buf`, answering how
    // many bytes were consumed.
    fn raw_read_to_str(handle: Long, buf: Mut Str) -> Ok Long | Err FsError => buf: Mut
    // The next line, without its terminator, appended to `buf`; `false` at the
    // end of the stream or after a recorded failure.
    fn raw_read_line_to_str(handle: Long, buf: Mut Str) -> Bool => buf: Mut
    // Bytes consumed so far, counted below the decoder.
    fn raw_read_position(handle: Long) -> Long
    // Releases the read handle, reporting any failure recorded on it.
    fn raw_close_read(handle: Long) -> Ok None | Err FsError

    // Accepts text, answering how many bytes of it were written.
    fn raw_write(handle: Long, text: Str) -> Long => text
    // Accepts bytes as they are, answering how many were written.
    fn raw_write_bytes(handle: Long, data: Bytes) -> Long => data
    // Bytes accepted so far.
    fn raw_write_position(handle: Long) -> Long
    // Pushes accepted bytes to the host, reporting a recorded failure.
    fn raw_flush(handle: Long) -> Ok None | Err FsError
    // Flushes and releases the write handle.
    fn raw_close_write(handle: Long) -> Ok None | Err FsError
}

// The one implementation of `RawFs`: a host class per backend, shipped with
// std [platform-handler].
export platform handler HostRawFs of RawFs

export handler DefaultFs [RawFs] of Fs {
    fn open_read(path: Str) -> Ok InStream | Err Checked<FsError> => path {
        let r = raw_open_read(path)
        when r {
            is Ok { return ok(InStream { handle: r }) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn open_read_at(path: Str, offset: Long) -> Ok InStream | Err Checked<FsError> => path {
        let r = raw_open_read_at(path, offset)
        when r {
            is Ok { return ok(InStream { handle: r }) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn open_write(path: Str) -> Ok OutStream | Err Checked<FsError> => path {
        let r = raw_open_write(path)
        when r {
            is Ok { return ok(OutStream { handle: r }) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn open_append(path: Str) -> Ok OutStream | Err Checked<FsError> => path {
        let r = raw_open_append(path)
        when r {
            is Ok { return ok(OutStream { handle: r }) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn exists(path: Str) -> Bool => path {
        return raw_exists(path)
    }

    fn metadata(path: Str) -> Ok FileInfo | Err Checked<FsError> => path {
        let r = raw_metadata(path)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn list_dir(path: Str) -> Ok List<Str> | Err Checked<FsError> => path {
        let r = raw_list_dir(path)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn create_dirs(path: Str) -> Ok None | Err Checked<FsError> => path {
        let r = raw_create_dirs(path)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn delete(path: Str) -> Ok None | Err Checked<FsError> => path {
        let r = raw_delete(path)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn rename_path(from: Str, to: Str) -> Ok None | Err Checked<FsError> => from, to {
        let r = raw_rename_path(from, to)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn read_line(s: InStream) -> Str | None => s {
        return raw_read_line(s.handle)
    }

    fn read_all(s: InStream) -> Ok Str | Err Checked<FsError> => s {
        let r = raw_read_all(s.handle)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err Checked<FsError> => s {
        let r = raw_read_bytes(s.handle, max)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn read_to(s: InStream, buf: Mut Bytes, max: Int) -> Ok Int | Err Checked<FsError> => s, buf: Mut {
        let r = raw_read_to_bytes(s.handle, buf, max)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn read_to(s: InStream, buf: Mut Str) -> Ok Long | Err Checked<FsError> => s, buf: Mut {
        let r = raw_read_to_str(s.handle, buf)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn read_line_to(s: InStream, buf: Mut Str) -> Bool => s, buf: Mut {
        return raw_read_line_to_str(s.handle, buf)
    }

    fn position(s: InStream) -> Long => s {
        return raw_read_position(s.handle)
    }
    fn close(s: InStream) -> Ok None | Err Checked<FsError> => !s {
        let r = raw_close_read(s.handle)
        discard(s)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn write(s: OutStream, text: Str) -> Long => s, text {
        return raw_write(s.handle, text)
    }

    fn write_line(s: OutStream, text: Str) -> Long => s, text {
        return raw_write(s.handle, "${text}\n")
    }

    fn write_bytes(s: OutStream, data: Bytes) -> Long => s, data {
        return raw_write_bytes(s.handle, data)
    }

    fn position(s: OutStream) -> Long => s {
        return raw_write_position(s.handle)
    }

    fn flush(s: OutStream) -> Ok None | Err Checked<FsError> => s {
        let r = raw_flush(s.handle)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn close(s: OutStream) -> Ok None | Err Checked<FsError> => !s {
        let r = raw_close_write(s.handle)
        discard(s)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<FsError>(r)) }
        }
    }
}
