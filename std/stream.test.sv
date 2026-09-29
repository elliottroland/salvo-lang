// [test-file] The tests of module `stream`.

import fs
import fs.mem

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
    let out = open_write("copy.txt")
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
    let back = read_to_str("copy.txt")
    when back {
        is Ok { expect_eq(back, "a\nb\n") }
        is Err {
            ignore(back)
            expect(false, "read back failed")
        }
    }
}
