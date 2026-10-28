// std's platform types and handlers: what their host files and seams emit.

//--- bytes_is_a_value_platform_type
// [type-bytes] [type-byte] The whole `Bytes` surface in one program: both
// constructors, the read side, the `Mut` side, structural equality, a copy
// that does not alias, the text bridge and a `for` over the buffer.
//
// [platform-value-type] `Bytes` is a value platform type whose Rust host is
// `pub type Bytes = Vec<u8>` — unboxed, no class — re-exported from
// `core::bytes`, and its operations are the platform wrappers.
fn main() [use] -> None {
    use StdConsole()
    // The two constructors: a fixed buffer, and one under construction.
    //~ emits(std/core/bytes.rs): pub use crate::platform_std_core_bytes::Bytes;
    // std's host file for core.bytes travels with the program.
    //~ emits(platform/std/core/bytes.rs): pub type Bytes = Vec<u8>;
    let fixed = bytes_of(to_byte(0), to_byte(255), to_byte(200))
    println("fixed: ${fixed} / ${to_hex(fixed)} / ${size(fixed)}")
    let buf = mut_bytes()
    buf.add(to_byte(104))
    buf.append(to_bytes("é"))
    println("built: ${buf} / ${to_hex(buf)}")
    // A `Mut Bytes` reaches the read surface by dropping its `Mut`.
    let text = str_of_bytes(buf)
    when text {
        is Str { println("text: ${text}") }
        is None { println("text: invalid") }
    }
    // Invalid UTF-8 decodes to nothing, strictly, on both backends.
    let broken = mut_bytes()
    broken.add(to_byte(200))
    println("broken: ${str_of_bytes(broken) is None}")
    // Reading: element, slice, search — each optional where it can miss.
    println("at 1: ${to_int(get(fixed, 1) ?: unreachable!())} / oob: ${get(fixed, 9) is None}")
    println("slice: ${slice(fixed, 1, 3) ?: unreachable!()} / bad: ${slice(fixed, 1, 9) is None}")
    println("index: ${index_of(fixed, to_byte(200)) ?: unreachable!()} / ${index_of(fixed, to_byte(7)) is None}")
    // Writing in place, and the clear that makes a buffer reusable.
    let scratch = mut_bytes(fixed)
    scratch.set(0, to_byte(1))
    scratch.set(9, to_byte(2))
    println("set: ${scratch}")
    clear(scratch)
    println("cleared: ${scratch} / ${size(scratch)}")
    // `==` is structural, and `copy` really copies.
    let a = bytes_of(to_byte(1), to_byte(2))
    let b = bytes_of(to_byte(1), to_byte(2))
    println("equal: ${a == b} / ${a == fixed}")
    let dup = copy(a)
    let grow = mut_bytes(dup)
    grow.add(to_byte(3))
    println("copy independent: ${a} vs ${grow}")
    // A `for` over a buffer, and the combinators over the same pass.
    let total = 0
    for byte in fixed {
        total = total + to_int(byte)
    }
    println("sum: ${total}")
    let doubled = map(iter(a), each -> to_int(each) * 2)
    println("mapped: ${doubled}")
}

//--- the_fs_surface_emits_a_host_seam_and_owned_tokens
// std's `fs`, exercised end to end against real files: the
// `platform handler HostRawFs` at the bottom, `DefaultFs [RawFs]` above it,
// linear stream tokens discharged by an effect member, the `Lines` pass,
// the one-shots, a `position` after a ranged open, and a failure path whose
// `Checked<FsError>` is acknowledged once. (`__DIR__` was a scratch
// directory when this program ran; what is emitted does not depend on it.)
//
// [rs-platform-handler] What the layering emits: no struct for the platform
// handler, `DefaultFs` as an ordinary Salvo handler reaching `RawFs` through
// the fusion, and the consuming member taking its token **by value** so the
// discharge is a move rather than a clone.
import std.fs
import std.fs.path
// The host seams are their own modules [mod-used-only]: a program that
// never opens a file links none of them.
//~ emits(std/fs/host.rs): pub trait __Stateful_RawFs: Send {
//~ emits(std/fs/host.rs): pub struct DefaultFs
// The platform handler's struct is the host's.
//~ !emits(std/fs/host.rs): pub struct HostRawFs
// std ships the host files, and they travel into the output.
//~ emits(platform/std/fs/host.rs): pub struct HostRawFs {}
import std.fs.host
//~ emits(std/stream.rs): pub trait __Stateful_Streams: Send {
// The token is consumed: by value, not `&InStream`.
//~ emits(std/stream.rs): fn close__InStream(&mut self, s: crate::std_stream::InStream)
// …and kept members borrow it.
//~ emits(std/stream.rs): fn read_line(&mut self, s: &crate::std_stream::InStream) -> Option<String>
// [effect-available] One overload set: the pass's discharger is a
// *fn* named `close`, beside the two members of that name.
//~ emits(std/stream.rs): pub fn close__Lines(streams: &crate::std_stream::Streams, mut p: crate::std_stream::Lines)
import std.stream
// [stream-table] `HostRawStreams` is Salvo over the runtime's table.
//~ emits(std/stream/host.rs): fn raw_close_read(&self, handle: i64)
//~ emits(std/stream/host.rs): pub struct DefaultStreams
//~ emits(std/stream/host.rs): pub struct HostRawStreams
//~ emits(platform/std/runtime/streams.rs): pub struct HostIn {
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
