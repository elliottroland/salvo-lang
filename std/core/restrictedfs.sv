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
fn fs_escaped(path: Str) [] -> FsError => path {
    return FsError { kind: PathEscapes { path: copy(path) } }
}

export handler RestrictedFs(root: Str) [Fs] of Fs {
    fn open_read(path: Str) -> Ok InStream | Err FsError => path {
        let real = fs_resolve(root, path)
        if real is None {
            return err(fs_escaped(path))
        }
        return open_read(real)
    }

    fn open_read_at(path: Str, offset: Long) -> Ok InStream | Err FsError => path {
        let real = fs_resolve(root, path)
        if real is None {
            return err(fs_escaped(path))
        }
        return open_read_at(real, offset)
    }

    fn open_write(path: Str) -> Ok OutStream | Err FsError => path {
        let real = fs_resolve(root, path)
        if real is None {
            return err(fs_escaped(path))
        }
        return open_write(real)
    }

    fn open_append(path: Str) -> Ok OutStream | Err FsError => path {
        let real = fs_resolve(root, path)
        if real is None {
            return err(fs_escaped(path))
        }
        return open_append(real)
    }

    // A refused path simply does not exist, since the answer is a `Bool` with
    // nowhere to put a reason.
    fn exists(path: Str) -> Bool => path {
        let real = fs_resolve(root, path)
        if real is None {
            return false
        }
        return exists(real)
    }

    fn metadata(path: Str) -> Ok FileInfo | Err FsError => path {
        let real = fs_resolve(root, path)
        if real is None {
            return err(fs_escaped(path))
        }
        return metadata(real)
    }

    fn list_dir(path: Str) -> Ok List<Str> | Err FsError => path {
        let real = fs_resolve(root, path)
        if real is None {
            return err(fs_escaped(path))
        }
        return list_dir(real)
    }

    fn create_dirs(path: Str) -> Ok None | Err FsError => path {
        let real = fs_resolve(root, path)
        if real is None {
            return err(fs_escaped(path))
        }
        return create_dirs(real)
    }

    fn delete(path: Str) -> Ok None | Err FsError => path {
        let real = fs_resolve(root, path)
        if real is None {
            return err(fs_escaped(path))
        }
        return delete(real)
    }

    // Both ends are checked: a rename is two paths, and either may escape.
    fn rename_path(from: Str, to: Str) -> Ok None | Err FsError => from, to {
        let real_from = fs_resolve(root, from)
        if real_from is None {
            return err(fs_escaped(from))
        }
        let real_to = fs_resolve(root, to)
        if real_to is None {
            return err(fs_escaped(to))
        }
        return rename_path(real_from, real_to)
    }

    // The stream members are pass-throughs: the policy lives entirely in the
    // opens, and a token this handler forwarded was minted by the handler it
    // wraps — which is where it goes back to.
    fn read_line(s: InStream) -> Str | None => s {
        return read_line(s)
    }

    fn read_all(s: InStream) -> Ok Str | Err FsError => s {
        return read_all(s)
    }

    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err FsError => s {
        return read_bytes(s, max)
    }

    fn read_to(s: InStream, buf: Mut Bytes, max: Int) -> Ok Int | Err FsError => s, buf: Mut {
        return read_to(s, buf, max)
    }

    fn read_to(s: InStream, buf: Mut Str) -> Ok Long | Err FsError => s, buf: Mut {
        return read_to(s, buf)
    }

    fn read_line_to(s: InStream, buf: Mut Str) -> Bool => s, buf: Mut {
        return read_line_to(s, buf)
    }

    fn position(s: InStream) -> Long => s {
        return position(s)
    }

    fn close(s: InStream) -> Ok None | Err FsError => !s {
        return close(s)
    }

    fn write(s: OutStream, text: Str) -> Long => s, text {
        return write(s, text)
    }

    fn write_line(s: OutStream, text: Str) -> Long => s, text {
        return write_line(s, text)
    }

    fn write_bytes(s: OutStream, data: Bytes) -> Long => s, data {
        return write_bytes(s, data)
    }

    fn position(s: OutStream) -> Long => s {
        return position(s)
    }

    fn flush(s: OutStream) -> Ok None | Err FsError => s {
        return flush(s)
    }

    fn close(s: OutStream) -> Ok None | Err FsError => !s {
        return close(s)
    }
}
