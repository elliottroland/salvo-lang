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
