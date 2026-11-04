// A filesystem confined to one directory: [RestrictedFs] wraps the
// filesystem already bound and lets the code under it reach only the files
// under a root directory.
//
// Bind it inside the scope it should confine. It **intercepts** `Fs`: it
// passes every operation on to whichever filesystem was bound before it,
// so the same handler confines the machine's files in production and a
// `MemFs` in a test. The code under it was not written to be confined and
// does not need to be.
//
// ```
// import std.fs
// import std.fs.path
// import std.fs.mem
// import std.fs.restricted
//
// fn main() [use] -> None {
//     use StdConsole()
//     use MemFs()
//     if true {
//         use RestrictedFs(path("sandbox"))
//         let wrote = write_str(path("notes/a.txt"), "hi")    // writes sandbox/notes/a.txt
//         if wrote is Err {
//             ignore(wrote)
//         }
//         let secret = read_to_str(path("../secret.txt"))
//         when secret {
//             is Ok { println("read outside the sandbox") }
//             is Err { println("${detach(secret)}") }      // path escapes the root: ../secret.txt
//         }
//     }
//     println("${exists(path("sandbox/notes/a.txt"))}")   // true
// }
// ```
//
// **Paths are rebased.** Every path the code under it uses is taken
// relative to the root: `notes/a.txt` becomes `<root>/notes/a.txt`, and the
// code never learns where it is really running. Write relative paths in
// code meant to run confined.
//
// **Escapes are refused, and say so.** A path that would leave the root is
// answered as `Err PathEscapes`, not as `NotFound`. That covers an absolute
// path (it names a place of its own, which is not this handler's to grant)
// and a relative one whose `..` segments climb above the root:
// `sub/../probe.txt` stays inside, `../secret.txt` does not. `exists` has
// no error to answer, so for an escaping path it answers `false`.
//
// **Not a security boundary.** The check is on the text of the path alone.
// A symbolic link inside the root that points outside it is followed by the
// host, so the confinement keeps honest code to its directory but does not
// stand up to code trying to get out. Only the opens are checked; reading
// and writing a stream that was opened is the `Streams` handler's, as
// usual.

// Closing the symlink hole needs the host (`openat2(RESOLVE_BENEATH)`,
// `O_RESOLVE_BENEATH`, cap-std), which under this layering belongs to
// `RawFs`: recorded as the hardening path rather than pretended away here.

import std.fs
import std.fs.path
import std.stream

// The path a restricted handler actually uses, or `None` when the request
// resolves outside its root. Absolute paths are refused outright: they say
// where they want to be, and that is not this handler's to grant.
//
// Resolution is right to left, counting the `..` segments still owed, which
// is how `a/../b` stays inside while `../b` does not — and needs no stack.
fn fs_resolve(root: Str, path: Str) [] -> Str? => root, path {
    if starts_with(path, "/") {
        return None
    }
    let segs = split(path, "/")
    let kept: Mut List<Str> = Mut List()
    let skip = 0
    let i = size(segs) - 1
    while i >= 0 {
        let seg = get(segs, i) ?: unreachable!()
        if seg == ".." {
            skip = skip + 1
        } else {
            if size(seg) == 0 || seg == "." {
                // An empty or `.` segment says nothing.
            } else {
                if skip > 0 {
                    skip = skip - 1
                } else {
                    kept.add(copy(seg))
                }
            }
        }
        i = i - 1
    }
    // Still owing a `..` at the top means the path climbed out of the root.
    if skip > 0 {
        return None
    }
    let parts: Mut List<Str> = Mut List()
    let j = size(kept) - 1
    while j >= 0 {
        parts.add(copy(get(kept, j) ?: unreachable!()))
        j = j - 1
    }
    let rel = join(parts, "/")
    if size(rel) == 0 {
        return copy(root)
    }
    return "${root}/${rel}"
}

// The refusal, as its own function because every member needs it.
fn fs_escaped(path: Str) [] -> Checked<FsError> => path {
    return checked<FsError>(PathEscapes { path: copy(path) })
}

// Confines the filesystem bound before it to the directory [root]: every
// path is taken relative to [root], and one that would leave it is refused
// as `Err PathEscapes`.
//
// [root] itself is a path in the filesystem being wrapped, relative to its
// working directory or absolute. Binding the handler consumes it, so pass a
// `copy` when the path is needed again afterwards. The module documentation
// above describes the rules and their limits.
//
// ```
// import std.fs
// import std.fs.path
// import std.fs.mem
// import std.fs.restricted
//
// fn confined(root: Path) [use] -> None => !root {
//     use MemFs()
//     use RestrictedFs(root)
//     let gone = delete(path("/etc/hosts"))                  // refused: absolute
//     if gone is Err {
//         ignore(gone)
//     }
// }
// ```
export handler RestrictedFs(root: Path) [Fs] for Fs {
    fn open_read(path: Path) -> Ok InStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return open_read(Path { value: real })
    }

    fn open_read_at(path: Path, offset: Long) -> Ok InStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return open_read_at(Path { value: real }, offset)
    }

    fn open_write(path: Path) -> Ok OutStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return open_write(Path { value: real })
    }

    fn open_append(path: Path) -> Ok OutStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return open_append(Path { value: real })
    }

    // A refused path simply does not exist, since the answer is a `Bool` with
    // nowhere to put a reason.
    fn exists(path: Path) -> Bool => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return false
        }
        return exists(Path { value: real })
    }

    fn metadata(path: Path) -> Ok FileInfo | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return metadata(Path { value: real })
    }

    fn list_dir(path: Path) -> Ok List<Str> | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return list_dir(Path { value: real })
    }

    fn create_dirs(path: Path) -> Ok None | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return create_dirs(Path { value: real })
    }

    fn delete(path: Path) -> Ok None | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return delete(Path { value: real })
    }

    // Both ends are checked: a rename is two paths, and either may escape.
    fn rename_path(from: Path, to: Path) -> Ok None | Err Checked<FsError> => from, to {
        let from_text = to_str(from)
        let to_text = to_str(to)
        let real_from = fs_resolve(to_str(root), from_text)
        if real_from is None {
            return err(fs_escaped(from_text))
        }
        let real_to = fs_resolve(to_str(root), to_text)
        if real_to is None {
            return err(fs_escaped(to_text))
        }
        return rename_path(Path { value: real_from }, Path { value: real_to })
    }
}
