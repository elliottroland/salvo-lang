// std's filesystem [platform-handler] [linear-group]: the host seam and the
// handlers layered over it.

//--- the_fs_surface_emits_a_host_seam_and_a_dependency_field
// [kt-platform-handler] [effect-handle] [kt-handle] What the layering
// emits: the effect interfaces, no class for the platform handler, and
// `DefaultFs` capturing its dependency as a **field typed by the effect's
// interface** — the JVM reference is the handle.
// std's `fs`, exercised end to end against real files — **verbatim the
// Rust backend's program** (`FS_PROGRAM` in codegen_tests.rs, whose
// `__DIR__` the run substitutes; nothing here runs it).
//~ emits(std/fs.kt): interface Fs {
// [effect-available] The stream operations are `stream`'s now, and the
// pass's discharger is a *fn* named `close` beside `Streams`' members.
//~ emits(std/stream.kt): interface Streams {
//~ emits(std/stream.kt): fun close__Lines(streams: Streams, p: Lines)
// The host seam is its own module [mod-used-only]: a program that never
// opens a file links none of it. [effect-prereq] `DefaultFs` reaches the
// `Streams` in scope as a dependency it never wrote; the platform handler's
// class is the host's.
//~ emits(std/fs/host.kt): interface RawFs {
//~ emits(std/fs/host.kt): class DefaultFs(private val __dep0: RawFs, private val __dep1: salvo.std.stream.Streams) : salvo.std.fs.Fs
//~ !emits(std/fs/host.kt): class HostRawFs
// [stream-table] `HostRawStreams` is Salvo over the runtime's table.
//~ emits(std/stream/host.kt): interface RawStreams {
//~ emits(std/stream/host.kt): fun rawCloseRead(handle: Long)
//~ emits(std/stream/host.kt): class DefaultStreams(
//~ emits(std/stream/host.kt): class HostRawStreams
// std ships the host files, and they travel into the output (an empty
// `emits(path):` asks only that the file is emitted).
//~ emits(platform/std/fs/host.kt):
//~ emits(platform/std/runtime/streams.kt):
import std.fs
import std.fs.path
import std.fs.host
import std.stream
import std.stream.host

fn describe(e: FsError) [] -> Str => e {
    if e is NotFound {
        return "not found"
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

fn main() [use] -> None {
    use StdConsole()
    use HostRawStreams()
    use DefaultStreams()
    // The `use` site constructs the shipped host class through its package.
    //~ emits: = salvo.std.fs.host.__Platform_HostRawFs()
    //~ emits: = salvo.std.fs.host.__Mon_RawFs(__use_
    use HostRawFs()
    use DefaultFs()

    let dir = Path { value: "__DIR__" }
    let made = create_dirs(dir)
    when made {
        is Ok { println("made") }
        is Err { println("made: ${describe(detach(made))}") }
    }

    let path = Path { value: "__DIR__/notes.txt" }
    let written = write_str(path, "alpha\nbeta\ngamma\n")
    when written {
        is Ok { println("wrote ${written}") }
        is Err { println("wrote: ${describe(detach(written))}") }
    }

    let all = read_lines(path)
    when all {
        is Ok { println("lines: ${all}") }
        is Err { println("lines: ${describe(detach(all))}") }
    }

    let opened = open_read(path)
    when opened {
        is Ok {
            let p = lines(opened)
            for line in p {
                println("line: ${line}")
            }
            let closed = close(p)
            when closed {
                is Ok { println("closed") }
                is Err { println("closed: ${describe(detach(closed))}") }
            }
        }
        is Err { println("open: ${describe(detach(opened))}") }
    }

    // A ranged open: "alpha\n" is six bytes, so the next line starts there.
    let tail = open_read_at(path, 6)
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
                is Ok { println("") }
                is Err { println("tail: ${describe(detach(shut))}") }
            }
        }
        is Err { println("tail: ${describe(detach(tail))}") }
    }

    let missing = read_to_str(Path { value: "__DIR__/nope.txt" })
    when missing {
        is Ok { println("unexpected") }
        is Err { println("missing: ${describe(detach(missing))}") }
    }

    let listed = list_dir(dir)
    when listed {
        is Ok { println("dir: ${listed}") }
        is Err { println("dir: ${describe(detach(listed))}") }
    }


    // [fs-bytes] The byte surface: a file that is not text, written and read
    // back exactly. `to_byte` keeps the low 8 bits and a `Byte` is unsigned on
    // both backends, so 255 prints as 255 on both — a signed byte would print
    // -1 on one of them [backend-parity].
    let out = open_write(Path { value: "__DIR__/raw.bin" })
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
    let raw = open_read(Path { value: "__DIR__/raw.bin" })
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
    let split = open_read_at(Path { value: "__DIR__/raw.bin" }, 5)
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
    let held = open_read(Path { value: "__DIR__/raw.bin" })
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
    let ch = open_chunks(Path { value: "__DIR__/raw.bin" }, 4)
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
    let lined = open_read(path)
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
    let copied = copy_file(path, Path { value: "__DIR__/copy.txt" })
    when copied {
        is Ok { println("copied ${copied}") }
        is Err { println("copied: ${describe(detach(copied))}") }
    }
    let all_bytes = read_to_bytes(Path { value: "__DIR__/copy.txt" })
    when all_bytes {
        is Ok { println("copy holds ${size(all_bytes)} bytes") }
        is Err { println("copy read: ${describe(detach(all_bytes))}") }
    }
    let gone_copy = delete(Path { value: "__DIR__/copy.txt" })
    when gone_copy {
        is Ok { println("copy deleted") }
        is Err { println("copy deleted: ${describe(detach(gone_copy))}") }
    }
    let gone_bin = delete(Path { value: "__DIR__/raw.bin" })
    when gone_bin {
        is Ok { println("raw deleted") }
        is Err { println("raw deleted: ${describe(detach(gone_bin))}") }
    }

    let gone = delete(path)
    when gone {
        is Ok { println("deleted") }
        is Err { println("deleted: ${describe(detach(gone))}") }
    }
    let gone_dir = delete(dir)
    when gone_dir {
        is Ok { println("removed") }
        is Err { println("removed: ${describe(detach(gone_dir))}") }
    }
}
