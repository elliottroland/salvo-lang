// `path`: a filesystem path as a value of its own, so its operations —
// parent, file name, extension, joining — are a `Path`'s and stay out of
// `Str`'s surface (user decision 2026-10-04) [fs-path-type].
//
// A path is text with `/` between its segments, on every backend: Salvo's
// filesystem speaks `/` whatever the host does, as the JVM and Rust's own
// `Path` both accept it. Nothing here touches a disk, so a `Path` needs no
// effect; `fs` takes one wherever it takes a path's text.

// A path. Build one with [path] and read it back with [to_str];
// two paths are equal when their text is (`a/b` and `a/b/` are different
// paths, as on the host).
// @compilerrules [fs-path-type]
export struct Path : Hashed<self> by auto {
    value: Str
}

// The path [value] spells.
// @compilerrules [fs-path-type]
export fn path(text: Str) [] -> Path => text {
    return Path { value: copy(text) }
}

// The path's text.
// @compilerrules [fs-path-type]
export fn to_str(p: Path) [] -> Str => p {
    return copy(p.value)
}

// Whether the path starts at the root.
// @compilerrules [fs-path-type]
export fn is_absolute(p: Path) [] -> Bool => p {
    return starts_with(p.value, "/")
}

// [p] with [child] beneath it. An absolute [child] replaces [p],
// as the hosts' own joins do; a trailing `/` on [p] is not doubled.
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

// [p] with [child]'s segments beneath it.
// @compilerrules [fs-path-type]
export fn join(p: Path, child: Path) [] -> Path => p, child {
    return join(p, child.value)
}

// The path without its last segment, or `None` when there is
// nothing above it (`a`, `/`, the empty path). `parent(/a)` is `/`.
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

// The last segment, or `None` for the root and the empty path.
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

// The file name's extension, without its dot: `gz` for
// `a.tar.gz`. `None` when there is no dot, or the only one leads the name
// (`.profile`).
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

// The file name without its extension: `a.tar` for `a.tar.gz`.
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

// [p] with its extension replaced by [ext] (added when it has
// none); an empty [ext] removes it.
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

// The path's segments, in order, without empty ones: `/a//b/` is
// `[a, b]`. A list of your own.
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
