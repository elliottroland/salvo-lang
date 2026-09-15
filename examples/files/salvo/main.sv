// The filesystem, end to end.
//
// One effect, `Fs`, carries both the path operations and the stream
// operations. Everything below is written against `[Fs]` and nothing else, so
// the *same* code runs against three filesystems in this one program: the
// machine's, a sandbox scoped to one directory, and an in-memory fake with no
// disk at all. The output of the last two blocks is identical, which is the
// point of the design — a double that fakes the whole surface makes a test of
// file code a test with no files.
//
// The two things the compiler will not let you forget:
//
//   * an **open stream** is linear — the token must be closed on every path,
//     and `close` is what discharges it;
//   * an **error** is linear too — `Err FsError` must be acknowledged, by
//     `ignore`, by `detach` (which hands back the droppable kind), or by
//     narrowing the result to its `Ok` arm.

// The name of a failure without the path it happened to. Paths differ between
// the real filesystem (where the sandbox has rebased them) and the fake, so
// naming the *kind* is what keeps the two runs comparable — and `is` narrowing
// over the union is how you read one.
fn kind_name(kind: FsErrorKind) [] -> Str => kind {
    if kind is NotFound {
        return "not found"
    }
    if kind is NotADirectory {
        return "not a directory"
    }
    if kind is PathEscapes {
        return "escapes the sandbox"
    }
    if kind is InvalidUtf8 {
        return "not valid UTF-8"
    }
    return "other"
}

// Everything a program usually does with files, against whichever filesystem
// is registered. It writes only *relative* paths, which is the convention that
// lets a sandbox rebase them.
fn workflow() [Fs, Console] -> None {
    // 1. A whole file in one call, and back again. The one-shots open, work
    //    and close, so no token reaches this code at all.
    let wrote = write_str("notes.txt", "alpha\nbeta\ngamma\n")
    when wrote {
        is Ok { println("wrote ${wrote} bytes") }
        is Err { println("write failed: ${kind_name(detach(wrote))}") }
    }
    let text = read_to_str("notes.txt")
    when text {
        is Ok { println("read back ${byte_size(text)} bytes") }
        is Err { println("read failed: ${kind_name(detach(text))}") }
    }

    // 2. The lines, as a sequence. `lines(s)` moves the stream's obligation
    //    into the pass, so closing the pass is closing the file — and `for`
    //    drives it like any other pass.
    let opened = open_read("notes.txt")
    when opened {
        is Ok {
            let p = lines(opened)
            for line in p {
                println("line: ${line}")
            }
            let closed = close(p)
            if closed is Err {
                println("close failed: ${kind_name(detach(closed))}")
            }
        }
        is Err { println("open failed: ${kind_name(detach(opened))}") }
    }

    // 3. Appending through a stream. `position` answers the byte offset —
    //    bytes, not characters, on every backend — and `write_line` answers
    //    how many bytes it took.
    let out = open_append("notes.txt")
    when out {
        is Ok {
            let w: OutStream = out
            let at = position(w)
            let n = write_line(w, "delta")
            println("appended ${n} bytes at offset ${at}")
            // `close` flushes, and reports a write that failed earlier: write
            // errors are recorded rather than returned, so the loop above
            // never had to narrow a result per line.
            let shut = close(w)
            if shut is Err {
                println("close failed: ${kind_name(detach(shut))}")
            }

            // 4. Reading that record back. Streams are forward-only, so a
            //    ranged *open* replaces a seek.
            let resumed = open_read_at("notes.txt", at)
            when resumed {
                is Ok {
                    let s: InStream = resumed
                    let line = read_line(s)
                    when line {
                        is Str { println("at ${at}: ${line}") }
                        is None { println("at ${at}: end of file") }
                    }
                    let done = close(s)
                    if done is Err {
                        println("close failed: ${kind_name(detach(done))}")
                    }
                }
                is Err { println("reopen failed: ${kind_name(detach(resumed))}") }
            }
        }
        is Err { println("append failed: ${kind_name(detach(out))}") }
    }

    // 5. A file that is not text. `Bytes` is std's byte buffer — the currency
    //    of every byte API here — and a `Byte` is an unsigned octet, built
    //    from an `Int` with `to_byte` (which keeps the low 8 bits) and read
    //    back into one with `to_int`.
    let bin = open_write("raw.bin")
    when bin {
        is Ok {
            let w: OutStream = bin
            let data = bytes_of(to_byte(0), to_byte(255), to_byte(200))
            let n = write_bytes(w, data)
            // Bytes and text go into one stream, and both counts are bytes:
            // "hé" is two characters and three bytes.
            let m = write(w, "hé")
            println("wrote ${n} raw bytes and ${m} encoded")
            let shut = close(w)
            if shut is Err {
                println("close failed: ${kind_name(detach(shut))}")
            }
        }
        is Err { println("raw open failed: ${kind_name(detach(bin))}") }
    }
    let raw = open_read("raw.bin")
    when raw {
        is Ok {
            let s: InStream = raw
            let head = read_bytes(s, 3)
            when head {
                is Ok { println("first three: ${head} = ${to_hex(head)}") }
                is Err { println("byte read failed: ${kind_name(detach(head))}") }
            }
            // One stream, one position, counted in bytes: the text read picks
            // up exactly where the byte read stopped.
            let tail = read_all(s)
            when tail {
                is Ok { println("the rest, as text: ${tail}") }
                is Err { println("decode failed: ${kind_name(detach(tail))}") }
            }
            let done = close(s)
            if done is Err {
                println("close failed: ${kind_name(detach(done))}")
            }
        }
        is Err { println("raw read failed: ${kind_name(detach(raw))}") }
    }

    // 6. Opening between the bytes of a character is a *seek*, not a decode:
    //    it succeeds, and the strict UTF-8 decode afterwards is what fails.
    //    The failure is recorded too, so `close` reports it a second time.
    let split = open_read_at("raw.bin", 5)
    when split {
        is Ok {
            let s: InStream = split
            let broken = read_all(s)
            when broken {
                is Ok { println("unexpected: ${broken} decoded") }
                is Err { println("mid-character: ${kind_name(detach(broken))}") }
            }
            let done = close(s)
            when done {
                is Ok { println("unexpected: the failure was not recorded") }
                is Err { println("and again at close: ${kind_name(detach(done))}") }
            }
        }
        is Err { println("split open failed: ${kind_name(detach(split))}") }
    }

    // 7. The fill-a-buffer reads. `read_to` appends into a buffer you own, so
    //    a loop reuses one buffer instead of allocating a payload per step —
    //    and because it appends rather than overwrites, `size(buf)` is always
    //    the data and there is no "first n bytes are meaningful" convention.
    let held = open_read("raw.bin")
    when held {
        is Ok {
            let s: InStream = held
            let buf = mut_bytes()
            let steps = 0
            let moved = 0
            let reading = true
            while reading {
                clear(buf)
                let got = read_to(s, buf, 4)
                when got {
                    is Ok {
                        let n: Int = got
                        if n == 0 {
                            reading = false
                        } else {
                            steps = steps + 1
                            moved = moved + n
                        }
                    }
                    is Err {
                        println("fill failed: ${kind_name(detach(got))}")
                        reading = false
                    }
                }
            }
            println("filled ${moved} bytes in ${steps} reads, one buffer")
            let done = close(s)
            if done is Err {
                println("close failed: ${kind_name(detach(done))}")
            }
        }
        is Err { println("fill open failed: ${kind_name(detach(held))}") }
    }
    // The same idea for text: `read_line_to` appends the next line to a
    // builder of yours, so a line per iteration costs no new string.
    let lined = open_read("notes.txt")
    when lined {
        is Ok {
            let s: InStream = lined
            let line = mut_str()
            let longest = 0
            let reading = true
            while reading {
                clear(line)
                if read_line_to(s, line) {
                    if size(line) > longest {
                        longest = size(line)
                    }
                } else {
                    reading = false
                }
            }
            println("longest line: ${longest} characters")
            let done = close(s)
            if done is Err {
                println("close failed: ${kind_name(detach(done))}")
            }
        }
        is Err { println("lines open failed: ${kind_name(detach(lined))}") }
    }

    // 8. Or let a pass do the loop: `chunks` is to bytes what `lines` is to
    //    text, and each step is a fresh buffer — a pass that handed back its
    //    own would have the next step overwrite what you are holding.
    let ch = open_chunks("raw.bin", 4)
    when ch {
        is Ok {
            let p = ch
            let seen = 0
            for chunk in p {
                seen = seen + size(chunk)
            }
            println("pass saw ${seen} bytes")
            let done = close(p)
            if done is Err {
                println("close failed: ${kind_name(detach(done))}")
            }
        }
        is Err { println("chunks failed: ${kind_name(detach(ch))}") }
    }

    // 9. And the one-shots, where the buffer is std's business: `copy_file`
    //    moves a whole file through one reused buffer, and `read_to_bytes`
    //    hands back the lot.
    let copied = copy_file("notes.txt", "notes-copy.txt")
    when copied {
        is Ok { println("copied ${copied} bytes") }
        is Err { println("copy failed: ${kind_name(detach(copied))}") }
    }
    let whole = read_to_bytes("raw.bin")
    when whole {
        is Ok { println("raw.bin is ${size(whole)} bytes: ${to_hex(whole)}") }
        is Err { println("byte read failed: ${kind_name(detach(whole))}") }
    }

    // 10. Failures, collected. A linear `FsError` may not live in a list
    //    (nothing linear may live in a composite), so `detach` hands back the
    //    droppable kind — which is the whole reason it exists.
    let failures: Mut List<FsErrorKind> = mut_list_of()
    let missing = read_to_str("nope.txt")
    when missing {
        is Ok { println("unexpected: ${missing}") }
        is Err { failures.add(detach(missing)) }
    }
    let not_a_dir = list_dir("notes.txt")
    when not_a_dir {
        is Ok { println("unexpected: ${not_a_dir}") }
        is Err { failures.add(detach(not_a_dir)) }
    }
    println("failures: ${size(failures)}")
    for kind in failures {
        println("  ${kind_name(copy(kind))}")
    }

    // 11. Tidying up: the same calls whichever filesystem answered.
    for name in ["notes.txt", "notes-copy.txt", "raw.bin"] {
        let gone = delete(name)
        if gone is Err {
            println("delete failed: ${kind_name(detach(gone))}")
        }
    }
    println("cleaned up")
}

// What the sandbox lets through and what it refuses. Containment is lexical
// and resolved right to left, so `sub/../probe.txt` stays inside while
// `../secret.txt` climbs out; an absolute path is refused rather than rebased,
// because it says where it wants to be and that is not the sandbox's to grant.
fn sandbox_edges() [Fs, Console] -> None {
    let inside = write_str("sub/../probe.txt", "inside\n")
    when inside {
        is Ok { println("through `..`: wrote ${inside} bytes") }
        is Err { println("through `..`: ${kind_name(detach(inside))}") }
    }
    let up = read_to_str("../secret.txt")
    when up {
        is Ok { println("unexpected: read outside the sandbox") }
        is Err { println("climbing out: ${kind_name(detach(up))}") }
    }
    let absolute = read_to_str("/etc/hosts")
    when absolute {
        is Ok { println("unexpected: an absolute path resolved") }
        is Err { println("absolute path: ${kind_name(detach(absolute))}") }
    }
    let probe = "probe.txt"
    println("probe still there: ${exists(probe)}")
    let gone = delete(probe)
    if gone is Err {
        println("delete failed: ${kind_name(detach(gone))}")
    }
}

fn main() [use] -> None {
    use StdOutConsole()
    // The machine's filesystem, in two layers: `HostRawFs` is the host
    // implementation (a class shipped per backend, no obligations in it), and
    // `DefaultFs` is the Salvo handler above it that mints the linear tokens
    // and maps host failures into `FsError`.
    use HostRawFs()
    use DefaultFs()

    let root = "tmp/files-example"
    let made = create_dirs(root)
    if made is Err {
        println("cannot create the working directory: ${kind_name(detach(made))}")
        return None
    }

    println("-- the real filesystem, scoped to one directory --")
    if true {
        // `RestrictedFs` *intercepts* `Fs`: it depends on the effect it
        // implements, so it wraps the handler already registered and every
        // relative path below is rebased onto `root`. The code it wraps was
        // not written to be sandboxed — that is what makes this useful.
        // `root` is copied because registering a handler *consumes* its
        // constructor arguments, and the directory name is needed again below.
        use RestrictedFs(copy(root))
        workflow()
        sandbox_edges()
    }

    // Outside the restriction again, so the working directory itself — which
    // is not inside the sandbox — can go.
    let gone = delete(root)
    if gone is Err {
        println("cleanup failed: ${kind_name(detach(gone))}")
    }

    println("-- the same code, with no disk at all --")
    if true {
        // `MemFs` fakes the whole of `Fs`, streams included, and shadows the
        // handler registered above for the length of the block. Its files are
        // bytes and its offsets are byte offsets, so the block below prints
        // exactly what the disk printed.
        use MemFs()
        workflow()
    }
}
