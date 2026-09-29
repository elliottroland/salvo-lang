// The filesystem of the machine the program runs on: the raw host seam and
// the handler that turns it into `fs`'s `Fs`. What it opens is registered in
// the host's stream table, so the streams are read by `stream.host`'s
// `DefaultStreams` — which is why a host filesystem is bound over host streams:
// `use HostRawStreams()`, `use DefaultStreams()`, then `use HostRawFs()`,
// `use DefaultFs()`.
//
// Its own module, and deliberately: a program that fakes the filesystem
// (`fs.mem`) imports `fs` without reaching this module at all. `DefaultFs` is
// a *dependent* handler, and a reachable dependent handler switches the whole
// program to the fused effect emission, so the handler nobody in a test uses
// must not arrive with the surface.

import fs
import stream

// ===== the raw seam =====

// The host's filesystem, in plain values: handles are `Long`s and failures
// are droppable kinds, so the platform class implements an interface with no
// obligations in it. Reachable only by declaring `[RawFs]`, which nothing
// but a composition root and `DefaultFs` does — which is what keeps the
// audit a grep.
//
// The opens answer a handle in the host's *stream* table: reading and writing
// it is `stream.host`'s `RawStreams`, whose host class reads the same table.
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
}

// The one implementation of `RawFs`: a host class per backend, shipped with
// std [platform-handler]. Deliberately *not* `threadsafe`
// [threadsafe-platform]: an open-file table keyed by handle is one-writer
// state, so the compiler serializes the instance on both backends and the
// host keeps plain maps.
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
}
