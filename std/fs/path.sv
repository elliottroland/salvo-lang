// `path`: a filesystem path as a value of its own, so its operations —
// parent, file name, extension, joining — are a `Path`'s and stay out of
// `Str`'s surface (user decision 2026-10-04) [path-type].
//
// A path is text with `/` between its segments, on every backend: Salvo's
// filesystem speaks `/` whatever the host does, as the JVM and Rust's own
// `Path` both accept it. Nothing here touches a disk, so a `Path` needs no
// effect; `fs` takes one wherever it takes a path's text.

// [path-type] A path. Build one with [path] and read it back with [to_str];
// two paths are equal when their text is (`a/b` and `a/b/` are different
// paths, as on the host).
export struct Path : Hashed<self> by auto {
    text: Str
}

// [path-type] The path [text] spells.
export fn path(text: Str) [] -> Path => text {
    return Path { text: copy(text) }
}

// [path-type] The path's text.
export fn to_str(p: Path) [] -> Str => p {
    return copy(p.text)
}

// [path-type] Whether the path starts at the root.
export fn is_absolute(p: Path) [] -> Bool => p {
    return starts_with(p.text, "/")
}

// [path-type] [p] with [child] beneath it. An absolute [child] replaces [p],
// as the hosts' own joins do; a trailing `/` on [p] is not doubled.
export fn join(p: Path, child: Str) [] -> Path => p, child {
    if starts_with(child, "/") || is_empty(p.text) {
        return path(child)
    }
    if ends_with(p.text, "/") {
        return path("${p.text}${child}")
    }
    return path("${p.text}/${child}")
}

// [path-type] [p] with [child]'s segments beneath it.
export fn join(p: Path, child: Path) [] -> Path => p, child {
    return join(p, child.text)
}

// [path-type] The path without its last segment, or `None` when there is
// nothing above it (`a`, `/`, the empty path). `parent(/a)` is `/`.
export fn parent(p: Path) [] -> Path? => p {
    let text = trim_trailing_slashes(p.text)
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

// [path-type] The last segment, or `None` for the root and the empty path.
export fn file_name(p: Path) [] -> Str? => p {
    let text = trim_trailing_slashes(p.text)
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

// [path-type] The file name's extension, without its dot: `gz` for
// `a.tar.gz`. `None` when there is no dot, or the only one leads the name
// (`.profile`).
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

// [path-type] The file name without its extension: `a.tar` for `a.tar.gz`.
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

// [path-type] [p] with its extension replaced by [ext] (added when it has
// none); an empty [ext] removes it.
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
    return path(copy(p.text))
}

// [path-type] The path's segments, in order, without empty ones: `/a//b/` is
// `[a, b]`. A list of your own [str-mut-results].
export fn segments(p: Path) [] -> Mut List<Str> => p {
    let out = mut_list_of<Str>()
    for part in split(p.text, "/") {
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
