// A filesystem scoped to one directory: `RestrictedFs(root)` **intercepts**
// `Fs`, so it wraps whichever filesystem is already registered — the host's
// in production, a `MemFs` in a test of the restriction itself.
//
// Paths are **rebased**: the code under it writes `"notes/a.txt"` and never
// learns where it is really running, which is half the point of the handler.
// A path that resolves outside the root is refused *distinguishably*, as
// `Err PathEscapes`, because this is a least-authority tool for honest code
// rather than a boundary against an adversary inside the process.
//
// The containment check is **lexical**, and therefore **not symlink-safe**: a
// symlink inside the root pointing outside it escapes. Closing that needs the
// host (`openat2(RESOLVE_BENEATH)`, `O_RESOLVE_BENEATH`, cap-std), which under
// this layering belongs to `RawFs` — recorded as the hardening path rather
// than pretended away here.

// The path a restricted handler actually uses, or `None` when the request
// resolves outside its root. Absolute paths are refused outright: they say
// where they want to be, and that is not this handler's to grant.
//
// Resolution is right to left, counting the `..` segments still owed, which
// is how `a/../b` stays inside while `../b` does not — and needs no stack.
import fs
import fs.path
import stream

fn fs_resolve(root: Str, path: Str) [] -> Str? => root, path {
    if starts_with(path, "/") {
        return None
    }
    let segs = split(path, "/")
    let kept: Mut List<Str> = mut_list_of()
    let skip = 0
    let i = size(segs) - 1
    while i >= 0 {
        let seg = get(segs, i)!
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
    let parts: Mut List<Str> = mut_list_of()
    let j = size(kept) - 1
    while j >= 0 {
        parts.add(copy(get(kept, j)!))
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

export handler RestrictedFs(root: Path) [Fs] of Fs {
    fn open_read(path: Path) -> Ok InStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return open_read(Path { text: real })
    }

    fn open_read_at(path: Path, offset: Long) -> Ok InStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return open_read_at(Path { text: real }, offset)
    }

    fn open_write(path: Path) -> Ok OutStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return open_write(Path { text: real })
    }

    fn open_append(path: Path) -> Ok OutStream | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return open_append(Path { text: real })
    }

    // A refused path simply does not exist, since the answer is a `Bool` with
    // nowhere to put a reason.
    fn exists(path: Path) -> Bool => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return false
        }
        return exists(Path { text: real })
    }

    fn metadata(path: Path) -> Ok FileInfo | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return metadata(Path { text: real })
    }

    fn list_dir(path: Path) -> Ok List<Str> | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return list_dir(Path { text: real })
    }

    fn create_dirs(path: Path) -> Ok None | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return create_dirs(Path { text: real })
    }

    fn delete(path: Path) -> Ok None | Err Checked<FsError> => path {
        let path_text = to_str(path)
        let real = fs_resolve(to_str(root), path_text)
        if real is None {
            return err(fs_escaped(path_text))
        }
        return delete(Path { text: real })
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
        return rename_path(Path { text: real_from }, Path { text: real_to })
    }
}
