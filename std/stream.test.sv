// [test-file] The tests of module `stream`.

test "fresh handles never repeat" {
    let a = fresh_handle()
    let b = fresh_handle()
    expect(a != b, "two fresh handles differ")
    expect(b > a, "the counter only goes up")
}
