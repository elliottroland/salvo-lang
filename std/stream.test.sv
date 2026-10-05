// [test-file] The tests of module `stream`.

import fs
import fs.mem
import fs.path
import codec.encode
import codec.decode

test "fresh handles never repeat" {
    let a = fresh_handle()
    let b = fresh_handle()
    expect(a != b, "two fresh handles differ")
    expect(b > a, "the counter only goes up")
}

// Reads what `receive` hands back until `End`, counting chunks — the
// non-blocking read, driven synchronously here by `waitfor`.
fn drain(s: InStream, seen: Long, chunks: Int) [Streams] -> (Long, Int) => !s {
    let got = waitfor r: Reply<Received> {
        receive(s, r)
    }
    when got {
        is Ok {
            let {bytes, stream} = got
            return drain(stream, seen + to_long(size(bytes)), chunks + 1)
        }
        is End { return (seen, chunks) }
        is Err {
            ignore(got)
            return (-1, chunks)
        }
    }
}

// [stream-receive] [stream-from-bytes] A buffer as a stream, read without
// blocking: the bytes arrive, then `End` — which has closed the stream, so
// nothing is owed.
test "receive reads a buffer to its end" {
    use MemFs()
    let s = from_bytes(to_bytes("hello"))
    let (seen, chunks) = drain(s, 0, 0)
    expect(seen == to_long(5), "five bytes seen")
    expect_eq(chunks, 1)
}

// [stream-pipe] A buffer piped into a file: the copy's answer is the byte
// count, and the file holds what went in.
test "pipe copies a stream into a file" {
    use MemFs()
    let out = open_write(path("copy.txt"))
    when out {
        is Ok {
            let moved = waitfor done: Reply<Ok Long | Err Checked<StreamError>> {
                pipe(from_bytes(to_bytes("a\nb\n")), out, done)
            }
            when moved {
                is Ok { expect(moved == to_long(4), "four bytes moved") }
                is Err {
                    ignore(moved)
                    expect(false, "pipe failed")
                }
            }
        }
        is Err {
            ignore(out)
            expect(false, "could not open")
        }
    }
    let back = read_to_str(path("copy.txt"))
    when back {
        is Ok { expect_eq(back, "a\nb\n") }
        is Err {
            ignore(back)
            expect(false, "read back failed")
        }
    }
}

// ===== [stream-values] =====

struct Rec : Hashed<self> by auto {
    id: Str,
    n: Long
}

// The number, or a marker: `End` is -1000, an error -2000.
fn num(r: Ok Long | End | Err Checked<StreamError>) [] -> Long => !r {
    when r {
        is Ok { return r }
        is End { return -1000L }
        is Err {
            ignore(r)
            return -2000L
        }
    }
}

fn num(r: Ok Int | End | Err Checked<StreamError>) [] -> Long => !r {
    when r {
        is Ok { return to_long(r) }
        is End { return -1000L }
        is Err {
            ignore(r)
            return -2000L
        }
    }
}

fn rec(r: Ok Rec | End | Err Checked<StreamError>) [] -> Rec => !r {
    when r {
        is Ok { return r }
        is End { return Rec { id: "end", n: 0L } }
        is Err {
            ignore(r)
            return Rec { id: "err", n: 0L }
        }
    }
}

fn done(r: Ok None | Err Checked<StreamError>) [] -> None => !r {
    if r is Err {
        ignore(r)
    }
}

test "numbers and values round-trip through a file" {
    use MemFs()
    let opened = open_write(path("/recs"))
    if opened is Err {
        ignore(opened)
        throw(Failure { message: "open failed" })
    }
    let out: OutStream = opened
    let _a = write_int(out, -2)
    let _b = write_long(out, 1099511627776L)
    let _c = write_value(out, Rec { id: "x", n: -7L })
    done(close(out))
    let reading = open_read(path("/recs"))
    if reading is Err {
        ignore(reading)
        throw(Failure { message: "reopen failed" })
    }
    let s: InStream = reading
    let i = num(read_int(s))
    let l = num(read_long(s))
    let r = rec(read_value<Rec>(s))
    let end = num(read_int(s))
    done(close(s))
    expect_eq(i, -2L)
    expect_eq(l, 1099511627776L)
    expect_eq(r.id, "x")
    expect_eq(r.n, -7L)
    expect_eq(end, -1000L)
}
