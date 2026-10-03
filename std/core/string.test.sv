// [test-file] The tests of module `core.string` — its annex. Today these
// cover the span surface ([col-span], the refinement-types sequence, step
// 3): the claimed pair, its filled test, and the total `substr`.

test "a span within the string passes the SpanOf test" {
    let s = "salvo"
    let sp = Span { start: 1, end: 4 }
    expect(sp is SpanOf(s), "1..4 lies within a five-character string")
}

test "a span past the end fails the SpanOf test" {
    let s = "salvo"
    let sp = Span { start: 2, end: 9 }
    expect(!(sp is SpanOf(s)), "9 is past the end")
}

test "a backwards span fails the SpanOf test" {
    let s = "salvo"
    let sp = Span { start: 3, end: 1 }
    expect(!(sp is SpanOf(s)), "start must not pass end")
}

test "a proven span slices with no optional" {
    // [col-span] The total `substr`: the optionality was paid at the test,
    // not at the use.
    let s = "salvo"
    let sp = Span { start: 1, end: 4 }
    assert!(sp is SpanOf(s))
    expect_eq(substr(s, sp), "alv")
}

test "an unproven range still answers the optional" {
    let s = "salvo"
    expect(substr(s, 2, 9) is None, "the range is not in the string")
}

// ===== [str-search] [str-salvo] [str-mut-results] =====

// An index or -1, for comparing.
fn at(i: Int?) [] -> Int {
    if i is Int {
        return i
    }
    return -1
}

test "index_of from an offset, and last_index_of" {
    let s = "a/b/c"
    expect_eq(at(index_of(s, "/", 2)), 3)
    expect_eq(at(index_of(s, "/", 4)), -1)
    expect_eq(at(last_index_of(s, "/")), 3)
    expect_eq(at(last_index_of(s, "x")), -1)
}

test "split answers a list of your own" {
    let parts = split("a/b/c", "/")
    let _name = remove_back(parts, 1)
    expect_eq(join(parts, "/"), "a/b")
}

test "replace, the one-sided trims, repeat and is_empty" {
    expect_eq(replace("a-b-c", "-", "+"), "a+b+c")
    expect_eq(trim_start("  x "), "x ")
    expect_eq(trim_end("  x "), "  x")
    expect_eq(repeat("ab", 3), "ababab")
    expect(is_empty(""), "empty")
    expect(!is_empty(" "), "a space is a character")
}

test "lines drops a carriage return and the empty last line" {
    let ls = lines("one\r\ntwo\nthree\n")
    expect_eq(join(ls, "|"), "one|two|three")
}

test "split_once and split_last" {
    let p = split_last("/var/data/board.db", "/")
    if p is None {
        throw(Failure { message: "no slash found" })
    }
    let (dir, name) = p
    expect_eq(dir, "/var/data")
    expect_eq(name, "board.db")
    expect(split_once("abc", "/") is None, "no separator")
}
