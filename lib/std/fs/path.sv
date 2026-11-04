// Filesystem paths: the `Path` type and the operations that take one
// apart and put one together (parent directory, file name, extension,
// joining). Import it with `import std.fs.path`.
//
// A `Path` is a value of its own rather than a `Str`, so a path cannot be
// passed where other text is meant, nor the other way round. Every
// operation of `std.fs` takes its paths as `Path`s. Nothing here touches a
// disk, so none of these functions needs an effect: a path names a file
// that may or may not exist.
//
// ```
// let db = path("/var/data/board.db")
// let dir = parent(db)                                   // /var/data
// let name = file_name(db)                               // board.db
// let ext = extension(db)                                // db
// let backup = with_extension(db, "bak")                 // /var/data/board.bak
// let log = join(path("/var/log"), "board.log")          // /var/log/board.log
// println(to_str(backup))
// ```
//
// **`/` on every host.** A path is text with `/` between its segments,
// whatever the operating system, and both backends' hosts accept that. A
// path that starts with `/` is absolute; any other is relative to the
// directory the program was started in (or, under a `RestrictedFs`, to its
// root).
//
// **A path is its text.** Two paths are equal when their text is, so
// `a/b` and `a/b/` are different paths, as they are to the host. Nothing
// is normalized: `.` and `..` segments are kept as written, and are
// resolved, if at all, by the filesystem that receives the path.
// @compilerrules [fs-path-type]

// A filesystem path. Make one with [path] and read its text back with
// [to_str].
//
// Two paths are equal when their text is (`a/b` and `a/b/` are different
// paths), and a `Path` can be a map key or a set element.
//
// ```
// let seen: Mut Set<Path> = Mut Set()
// seen.add(path("a/b"))
// let again = contains(seen, path("a/b"))                // true
// let other = path("a/b") == path("a/b/")                // false
// ```
// @compilerrules [fs-path-type]
export struct Path : Hashed<self> by auto {
    value: Str
}

// The path [text] spells, taken as it is: nothing is checked or
// normalized.
// @compilerrules [fs-path-type]
export fn path(text: Str) [] -> Path => text {
    return Path { value: copy(text) }
}

// The path's text, exactly as it was given to [path] or built by [join].
// It is also what interpolating a path prints.
// @compilerrules [fs-path-type]
export fn to_str(p: Path) [] -> Str => p {
    return copy(p.value)
}

// Whether [p] starts at the root, that is, with `/`.
// @compilerrules [fs-path-type]
export fn is_absolute(p: Path) [] -> Bool => p {
    return starts_with(p.value, "/")
}

// [p] with [child] beneath it, joined by one `/`.
//
// A trailing `/` on [p] is not doubled, and an empty [p] answers [child]
// alone. An absolute [child] replaces [p] entirely, as the hosts' own joins
// do. [child] may hold several segments (`"a/b"`).
//
// ```
// let a = join(path("/var"), "data")                     // /var/data
// let b = join(path("/var/"), "data")                    // /var/data
// let c = join(path("/var"), "/etc")                     // /etc
// ```
// @compilerrules [fs-path-type]
export fn join(p: Path, child: Str) [] -> Path => p, child {
    if starts_with(child, "/") || is_empty(p.value) {
        return path(child)
    }
    if ends_with(p.value, "/") {
        return path("${p.value}${child}")
    }
    return path("${p.value}/${child}")
}

// [join] with a `Path` as the child: [p] with [child]'s segments beneath
// it, and an absolute [child] replacing [p].
// @compilerrules [fs-path-type]
export fn join(p: Path, child: Path) [] -> Path => p, child {
    return join(p, child.value)
}

// [p] without its last segment, or `None` when there is nothing above it.
//
// Trailing `/`s are ignored, so the parent of `/var/data/` is `/var`. The
// parent of `/var` is `/`. A single relative segment (`board.db`), `/` and
// the empty path have no parent.
//
// ```
// let up = parent(path("/var/data/board.db"))            // /var/data
// let top = parent(path("/var"))                         // /
// let none = parent(path("board.db"))                    // None
// ```
// @compilerrules [fs-path-type]
export fn parent(p: Path) [] -> Path? => p {
    let text = trim_trailing_slashes(p.value)
    let cut = split_last(text, "/")
    if cut is None {
        return None
    }
    let (dir, _name) = cut
    if is_empty(dir) {
        if starts_with(text, "/") && size(text) > 1 {
            return path("/")
        }
        return None
    }
    return path(dir)
}

// The last segment of [p], or `None` for `/` and the empty path. Trailing
// `/`s are ignored, so the file name of `/var/data/` is `data`.
//
// ```
// let name = file_name(path("/var/data/board.db"))       // board.db
// let dir = file_name(path("/var/data/"))                // data
// ```
// @compilerrules [fs-path-type]
export fn file_name(p: Path) [] -> Str? => p {
    let text = trim_trailing_slashes(p.value)
    let cut = split_last(text, "/")
    if cut is None {
        if is_empty(text) {
            return None
        }
        return text
    }
    let (_dir, name) = cut
    if is_empty(name) {
        return None
    }
    return name
}

// The extension of [p]'s file name, without its dot: what follows the
// last dot.
//
// `None` when the file name has no dot, when its only dot is its first
// character (`.profile`, a hidden file rather than an extension), or when
// [p] has no file name.
//
// ```
// let gz = extension(path("backup.tar.gz"))              // gz
// let none = extension(path(".profile"))                 // None
// ```
// @compilerrules [fs-path-type]
export fn extension(p: Path) [] -> Str? => p {
    let name = file_name(p)
    if name is Str n {
        let cut = split_last(n, ".")
        if cut is None {
            return None
        }
        let (stem, ext) = cut
        if is_empty(stem) {
            return None
        }
        return ext
    }
    return None
}

// [p]'s file name without its [extension]: `a.tar` for `a.tar.gz`. A
// file name with no extension (`README`, `.profile`) is answered whole, and
// a path with no file name answers `None`.
// @compilerrules [fs-path-type]
export fn stem(p: Path) [] -> Str? => p {
    let name = file_name(p)
    if name is Str n {
        let cut = split_last(n, ".")
        if cut is None {
            return n
        }
        let (stem, _ext) = cut
        if is_empty(stem) {
            return n
        }
        return stem
    }
    return None
}

// [p] with its extension replaced by [ext], or added when it has none. An
// empty [ext] removes the extension. A path with no file name (`/`) is
// answered unchanged.
//
// ```
// let bak = with_extension(path("data/board.db"), "bak")    // data/board.bak
// let bare = with_extension(path("notes.txt"), "")          // notes
// ```
// @compilerrules [fs-path-type]
export fn with_extension(p: Path, ext: Str) [] -> Path => p, ext {
    let s = stem(p)
    if s is Str base {
        let name = if is_empty(ext) { base } else { "${base}.${ext}" }
        let up = parent(p)
        if up is Path dir {
            return join(dir, name)
        }
        return path(name)
    }
    return path(copy(p.value))
}

// [p]'s segments, in order, without the empty ones: `/a//b/` is `[a, b]`.
// The list is a new one, yours to change.
// @compilerrules [fs-path-type, str-mut-results]
export fn segments(p: Path) [] -> Mut List<Str> => p {
    let out = Mut List<Str>()
    for part in split(p.value, "/") {
        if !is_empty(part) {
            add(out, copy(part))
        }
    }
    return out
}

// [text] without trailing `/`s, keeping a lone root `/`.
fn trim_trailing_slashes(text: Str) [] -> Str => text {
    let t = copy(text)
    while size(t) > 1 && ends_with(t, "/") {
        t = trim_suffix(t, "/")
    }
    return t
}
