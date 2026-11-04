// The filesystem of the machine the program runs on: [DefaultFs], the
// handler a program binds to give `Fs` the real disk, and the host layer
// under it.
//
// The machine's filesystem is bound at the program's entry point in four
// lines, the streams first because `Fs` needs a `Streams` to open files
// into: the host's stream table and its handler from `std.stream.host`,
// then the host's files and [DefaultFs] from here.
//
// ```
// import std.fs
// import std.fs.path
// import std.fs.host
// import std.stream.host
//
// fn main() [use] -> None {
//     use StdConsole()
//     use HostRawStreams()
//     use DefaultStreams()
//     use HostRawFs()
//     use DefaultFs()
//     println("${exists(path("salvo.toml"))}")
// }
// ```
//
// Paths are passed to the host as their text. A relative path is relative
// to the directory the program was started in, and `/` separates segments
// on every host.
//
// What this filesystem opens is read through the host's `Streams`, the same
// one that reads network bodies, so a stream from here must not be handed
// to a `MemFs`, nor the other way round: a stream belongs to the provider
// that opened it, and using it with another stops the program.
// @compilerrules [stream-provider]

// Its own module, and deliberately: a program that fakes the filesystem
// (`fs.mem`) imports `fs` without reaching this module at all. `DefaultFs` is
// a *dependent* handler, and a reachable dependent handler switches the whole
// program to the fused effect emission, so the handler nobody in a test uses
// must not arrive with the surface.

import std.fs
import std.fs.path
import std.stream

// ===== the raw seam =====

// The host's filesystem in plain values: the layer under [DefaultFs].
//
// Files are named by their path's text, an open file is a `Long` handle in
// the host's stream table, and failures are plain [FsError]s with no
// obligation. Application code declares `Fs`, never this: only a program's
// entry point (to bind [HostRawFs]) and [DefaultFs] reach it, which keeps
// every use of the raw layer easy to find. Its members mirror `Fs`'s.
//
// The handles index the host's *stream* table, so reading and writing them
// is `std.stream.host`'s `RawStreams`, whose host class reads the same
// table.
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

// Deliberately *not* `threadsafe`: an open-file table keyed by handle is
// one-writer state, so the compiler serializes the instance on both
// backends and the host keeps plain maps.

// The host's implementation of [RawFs], shipped with std for each backend.
// Bind it with `use HostRawFs()`, after the host's streams and before
// [DefaultFs].
// @compilerrules [platform-handler, platform-threadsafe]
export platform handler HostRawFs for RawFs

// The machine's filesystem, as `Fs`: what a program binds, with `use
// DefaultFs()`, to read and write real files.
//
// It works over [RawFs], which must be bound first ([HostRawFs]), and needs
// a `Streams` around it, since that is what it opens files into. Each open
// answers a stream token that must be closed, and each failure the host
// reports is answered as `Err Checked<FsError>`. The dependency on [RawFs]
// is supplied where the handler is bound, so no function that uses files
// mentions it.
export handler DefaultFs [RawFs] for Fs {
    fn open_read(path: Path) -> Ok InStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let r = raw_open_read(path_text)
        when r {
            is Ok { return ok(InStream { handle: r }) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn open_read_at(path: Path, offset: Long) -> Ok InStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let r = raw_open_read_at(path_text, offset)
        when r {
            is Ok { return ok(InStream { handle: r }) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn open_write(path: Path) -> Ok OutStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let r = raw_open_write(path_text)
        when r {
            is Ok { return ok(OutStream { handle: r }) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn open_append(path: Path) -> Ok OutStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let r = raw_open_append(path_text)
        when r {
            is Ok { return ok(OutStream { handle: r }) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn exists(path: Path) -> Bool => path {
        let path_text = to_str(path)
        return raw_exists(path_text)
    }

    fn metadata(path: Path) -> Ok FileInfo | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let r = raw_metadata(path_text)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn list_dir(path: Path) -> Ok List<Str> | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let r = raw_list_dir(path_text)
        when r {
            is Ok { return ok(r) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn create_dirs(path: Path) -> Ok None | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let r = raw_create_dirs(path_text)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn delete(path: Path) -> Ok None | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let r = raw_delete(path_text)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<FsError>(r)) }
        }
    }

    fn rename_path(from: Path, to: Path) -> Ok None | Err Checked<FsError> => from, to {
        let from_text = to_str(from)
        let to_text = to_str(to)
        let r = raw_rename_path(from_text, to_text)
        when r {
            is Ok { return ok(None) }
            is Err { return err(checked<FsError>(r)) }
        }
    }
}
