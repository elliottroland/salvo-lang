// std's in-memory filesystem and the restriction over it
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- the_memory_filesystem

import std.fs
import std.fs.path
import std.fs.mem
import std.fs.restricted
import std.stream

fn describe(e: FsError) [] -> Str => e {
    if e is NotFound {
        return "not found"
    }
    if e is PathEscapes {
        return "escapes"
    }
    if e is NotADirectory {
        return "not a directory"
    }
    if e is Streaming {
        return describe(e.error)
    }
    return "other"
}

fn describe(e: StreamError) [] -> Str => e {
    if e is InvalidUtf8 {
        return "not utf-8"
    }
    return "other"
}

fn read_and_say(label: Str, path: Path) [Fs, Console] -> None => label, path {
    let text = read_to_str(path)
    when text {
        is Ok { println("${label}: ${size(text)}") }
        is Err { println("${label}: ${describe(detach(text))}") }
    }
}

fn main() [use] -> None {
    use StdConsole()
    // No host handler anywhere: `MemFs` fakes the whole of `Fs`, streams
    // included, so this program touches no disk.
    use MemFs()

    let wrote = write_str(path("notes/a.txt"), "alpha\nbeta\ngamma\n")
    when wrote {
        is Ok { println("wrote ${wrote}") }
        is Err { println("wrote: ${describe(detach(wrote))}") }
    }

    let lines = read_lines(path("notes/a.txt"))
    when lines {
        is Ok { println("lines: ${lines}") }
        is Err { println("lines: ${describe(detach(lines))}") }
    }

    // The byte offset the fake has to agree with the host about: "alpha\n" is
    // six bytes, and the position after reading "beta\n" is eleven.
    let tail = open_read_at(path("notes/a.txt"), 6)
    when tail {
        is Ok {
            let s: InStream = tail
            let line = read_line(s)
            when line {
                is Str { println("at 6: ${line}") }
                is None { println("at 6: end") }
            }
            println("position: ${position(s)}")
            let shut = close(s)
            when shut {
                is Ok { println("closed") }
                is Err { println("close: ${describe(detach(shut))}") }
            }
        }
        is Err { println("at 6: ${describe(detach(tail))}") }
    }

    let listed = list_dir(path("notes"))
    when listed {
        is Ok { println("dir: ${listed}") }
        is Err { println("dir: ${describe(detach(listed))}") }
    }


    // [fs-bytes] The byte surface: a file that is not text, written and read
    // back exactly. `to_byte` keeps the low 8 bits and a `Byte` is unsigned on
    // both backends, so 255 prints as 255 on both — a signed byte would print
    // -1 on one of them [backend-parity].
    let out = open_write(path("notes/raw.bin"))
    when out {
        is Ok {
            let w: OutStream = out
            let data = bytes_of(to_byte(0), to_byte(255), to_byte(200))
            let n = write_bytes(w, data)
            let m = write(w, "hé")
            println("bytes: ${n} + ${m} at ${position(w)}")
            let shut = close(w)
            when shut {
                is Ok { println("wrote raw") }
                is Err { println("wrote raw: ${describe(detach(shut))}") }
            }
        }
        is Err { println("raw open: ${describe(detach(out))}") }
    }
    let raw = open_read(path("notes/raw.bin"))
    when raw {
        is Ok {
            let s: InStream = raw
            let head = read_bytes(s, 3)
            when head {
                is Ok { println("read: ${head} at ${position(s)} (${to_hex(head)})") }
                is Err { println("read: ${describe(detach(head))}") }
            }
            // Bytes and text off one stream: the position is bytes either way,
            // so the text read picks up exactly where the byte read stopped.
            let text = read_all(s)
            when text {
                is Ok { println("then: ${text}") }
                is Err { println("then: ${describe(detach(text))}") }
            }
            let shut = close(s)
            when shut {
                is Ok { println("read raw") }
                is Err { println("read raw: ${describe(detach(shut))}") }
            }
        }
        is Err { println("raw read: ${describe(detach(raw))}") }
    }
    // Opening between the bytes of a character is a *seek*, not a decode: it
    // succeeds, and the strict decode afterwards is what fails — recorded, so
    // `close` reports it too [fs-errors-at-close].
    let split = open_read_at(path("notes/raw.bin"), 5)
    when split {
        is Ok {
            let s: InStream = split
            let bad = read_all(s)
            when bad {
                is Ok { println("split: ${bad}") }
                is Err { println("split: ${describe(detach(bad))}") }
            }
            let shut = close(s)
            when shut {
                is Ok { println("split closed") }
                is Err { println("split close: ${describe(detach(shut))}") }
            }
        }
        is Err { println("split open: ${describe(detach(split))}") }
    }

    // [fs-read-to] The fill-a-buffer read: one buffer, cleared and refilled,
    // instead of a payload per step.
    let held = open_read(path("notes/raw.bin"))
    when held {
        is Ok {
            let s: InStream = held
            let buf = mut_bytes()
            let steps = 0
            let moved = 0
            let reading = true
            while reading {
                clear(buf)
                let got = read_to(s, buf, 2)
                if got is Err {
                    println("read_to: ${describe(detach(got))}")
                    reading = false
                } else {
                    let n: Int = got
                    if n == 0 {
                        reading = false
                    } else {
                        steps = steps + 1
                        moved = moved + n
                    }
                }
            }
            println("filled ${moved} bytes in ${steps} reads")
            let shut = close(s)
            when shut {
                is Ok { println("filled closed") }
                is Err { println("filled close: ${describe(detach(shut))}") }
            }
        }
        is Err { println("filled open: ${describe(detach(held))}") }
    }
    // The chunk pass: `for` over a stream's bytes, a fresh buffer per step.
    let ch = open_chunks(path("notes/raw.bin"), 4)
    when ch {
        is Ok {
            let p = ch
            let seen = 0
            for chunk in p {
                seen = seen + size(chunk)
            }
            println("pass saw ${seen} bytes")
            let shut = close(p)
            when shut {
                is Ok { println("pass closed") }
                is Err { println("pass close: ${describe(detach(shut))}") }
            }
        }
        is Err { println("pass open: ${describe(detach(ch))}") }
    }
    // And the text side of `read_to`: a line per iteration, one builder.
    let lined = open_read(path("notes/a.txt"))
    when lined {
        is Ok {
            let s: InStream = lined
            let line = Mut Str()
            let lines_seen = 0
            let reading = true
            while reading {
                clear(line)
                if read_line_to(s, line) {
                    lines_seen = lines_seen + 1
                } else {
                    reading = false
                }
            }
            println("read ${lines_seen} lines into one builder")
            let shut = close(s)
            when shut {
                is Ok { println("lines closed") }
                is Err { println("lines close: ${describe(detach(shut))}") }
            }
        }
        is Err { println("lines open: ${describe(detach(lined))}") }
    }
    // The one-shots that own the buffer: a whole-file copy and a whole-file
    // byte read, with no stream and no buffer in sight.
    let copied = copy_file(path("notes/a.txt"), path("notes/copy.txt"))
    when copied {
        is Ok { println("copied ${copied}") }
        is Err { println("copied: ${describe(detach(copied))}") }
    }
    let all_bytes = read_to_bytes(path("notes/copy.txt"))
    when all_bytes {
        is Ok { println("copy holds ${size(all_bytes)} bytes") }
        is Err { println("copy read: ${describe(detach(all_bytes))}") }
    }
    let gone_copy = delete(path("notes/copy.txt"))
    when gone_copy {
        is Ok { println("copy deleted") }
        is Err { println("copy deleted: ${describe(detach(gone_copy))}") }
    }
    let gone_bin = delete(path("notes/raw.bin"))
    when gone_bin {
        is Ok { println("raw deleted") }
        is Err { println("raw deleted: ${describe(detach(gone_bin))}") }
    }

    // The restriction, over the same memory, for the length of the block: an
    // interceptor wraps the handler already registered.
    if true {
        use RestrictedFs(path("notes"))
        read_and_say("inside", path("a.txt"))
        read_and_say("through ..", path("sub/../a.txt"))
        read_and_say("escape", path("../secret.txt"))
        read_and_say("absolute", path("/etc/hosts"))
    }
    // Out of the block the unrestricted filesystem answers again.
    read_and_say("unrestricted", path("notes/a.txt"))
}
//= stdout
//= wrote 17
//= lines: [alpha, beta, gamma]
//= at 6: beta
//= position: 11
//= closed
//= dir: [a.txt]
//= bytes: 3 + 3 at 6
//= wrote raw
//= read: [0, 255, 200] at 3 (00ffc8)
//= then: hé
//= read raw
//= split: not utf-8
//= split close: not utf-8
//= filled 6 bytes in 3 reads
//= filled closed
//= pass saw 6 bytes
//= pass closed
//= read 3 lines into one builder
//= lines closed
//= copied 17
//= copy holds 17 bytes
//= copy deleted
//= raw deleted
//= inside: 17
//= through ..: 17
//= escape: escapes
//= absolute: escapes
//= unrestricted: 17
