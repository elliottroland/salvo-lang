// streams: whose table a stream belongs to, and pipes
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [stream-provider] A stream belongs to the table that minted it: a handle
// the host's table never minted traps, naming the rule. The program never
// gets to print.
//--- a_stream_the_host_never_minted_traps
import std.stream
import std.stream.host

fn main() [use] {
    use StdConsole()
    use HostRawStreams()
    use DefaultStreams()
    let s = InStream { handle: 987654 }
    let line = read_line(s)
    println("read ${line is None}")
    let closed = close(s)
    when closed {
        is Ok { println("closed") }
        is Err { ignore(closed) }
    }
}
//= trap stream handle 987654 is not the host's: a stream belongs to the provider that minted it [stream-provider]

// [stream-provider] And so does one handed to `MemFs`.
//--- a_stream_memfs_never_minted_traps
import std.stream
import std.fs.mem

fn main() [use] {
    use StdConsole()
    use MemFs()
    let s = InStream { handle: 987654 }
    let line = read_line(s)
    println("read ${line is None}")
    let closed = close(s)
    when closed {
        is Ok { println("closed") }
        is Err { ignore(closed) }
    }
}
//= trap stream handle 987654 was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]

// [stream-receive] [stream-pipe] [stream-from-bytes] Non-blocking copies,
// against both worlds: a buffer piped into a file (GetObject-to-a-file's
// shape), and a file piped into another (a file is a body:
// PutObject-from-a-file's). On the host, `receive` reads on a host thread
// and completes its reply from there [platform-reply]; in memory it answers
// at once. Same stdout everywhere. The host files are relative paths, in
// the program's own working directory.
//--- streams_pipe_without_blocking
import std.fs
import std.fs.path
import std.fs.host
import std.fs.mem
import std.stream
import std.stream.host

fn describe(r: Ok Long | Err Checked<StreamError>) [] -> Str => !r {
    when r {
        is Ok { return "piped ${r} bytes" }
        is Err { return "failed: ${detach(r)}" }
    }
}

// GetObject-to-a-file's shape: a stream from somewhere, into a file, without
// blocking a worker.
fn copy_in(body: InStream, path: Path) [Fs] -> Str => !body, path {
    let out = open_write(path)
    when out {
        is Ok {
            let result = waitfor done: Reply<Ok Long | Err Checked<StreamError>> {
                pipe(body, out, done)
            }
            return describe(result)
        }
        is Err {
            let closed = close(body)
            if closed is Err { ignore(closed) }
            return "open failed: ${detach(out)}"
        }
    }
}

fn run() [Fs, Console] {
    let body = from_bytes(to_bytes("hello, streams\nsecond line\n"))
    println(copy_in(body, path("piped.txt")))
    let back = read_lines(path("piped.txt"))
    when back {
        is Ok { println("lines: ${back}") }
        is Err { println("read: ${detach(back)}") }
    }
    // A file is a body too: PutObject-from-a-file's shape.
    let file = open_read(path("piped.txt"))
    when file {
        is Ok { println(copy_in(file, path("piped2.txt"))) }
        is Err { println("reopen: ${detach(file)}") }
    }
    let n = read_to_str(path("piped2.txt"))
    when n {
        is Ok { println("copy of the copy: ${byte_size(n)} bytes") }
        is Err { println("read2: ${detach(n)}") }
    }
    let a = delete(path("piped.txt"))
    if a is Err { ignore(a) }
    let b = delete(path("piped2.txt"))
    if b is Err { ignore(b) }
}

fn main() [use] {
    use StdConsole()
    use HostRawStreams()
    use DefaultStreams()
    use HostRawFs()
    use DefaultFs()
    println("-- host --")
    run()
    println("-- mem --")
    if true {
        use MemFs()
        run()
    }
}
//= stdout
//= -- host --
//= piped 27 bytes
//= lines: [hello, streams, second line]
//= piped 27 bytes
//= copy of the copy: 27 bytes
//= -- mem --
//= piped 27 bytes
//= lines: [hello, streams, second line]
//= piped 27 bytes
//= copy of the copy: 27 bytes
