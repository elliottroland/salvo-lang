// [test-file] The tests of module `path` [path-type].

fn text(p: Path?) [] -> Str => p {
    if p is Path q {
        return to_str(q)
    }
    return "-"
}

fn name(s: Str?) [] -> Str => s {
    if s is Str t {
        return copy(t)
    }
    return "-"
}

test "join puts a child beneath, and an absolute child replaces" {
    expect_eq(to_str(join(path("/var"), "data")), "/var/data")
    expect_eq(to_str(join(path("/var/"), "data")), "/var/data")
    expect_eq(to_str(join(path("/var"), "/etc")), "/etc")
    expect_eq(to_str(join(path(""), "a")), "a")
}

test "parent walks up to the root" {
    expect_eq(text(parent(path("/var/data/board.db"))), "/var/data")
    expect_eq(text(parent(path("/var/data/"))), "/var")
    expect_eq(text(parent(path("/var"))), "/")
    expect_eq(text(parent(path("/"))), "-")
    expect_eq(text(parent(path("board.db"))), "-")
}

test "file_name, extension and stem" {
    let p = path("/var/data/board.tar.gz")
    expect_eq(name(file_name(p)), "board.tar.gz")
    expect_eq(name(extension(p)), "gz")
    expect_eq(name(stem(p)), "board.tar")
    expect_eq(name(extension(path("/home/.profile"))), "-")
    expect_eq(name(stem(path("/home/.profile"))), ".profile")
    expect_eq(name(file_name(path("/"))), "-")
}

test "with_extension, segments, is_absolute and equality" {
    expect_eq(to_str(with_extension(path("/a/b.txt"), "md")), "/a/b.md")
    expect_eq(to_str(with_extension(path("b"), "md")), "b.md")
    expect_eq(to_str(with_extension(path("/a/b.txt"), "")), "/a/b")
    expect_eq(join(segments(path("/a//b/")), ","), "a,b")
    expect(is_absolute(path("/a")), "absolute")
    expect(!is_absolute(path("a")), "relative")
    expect(path("a/b") == path("a/b"), "same text")
}
