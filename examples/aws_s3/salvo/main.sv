// Amazon S3 from Salvo, with no AWS anywhere: objects in and out as streams,
// against the generated fake and an in-memory double, over an in-memory
// filesystem.
//
// `aws.s3` is generated from the service's Smithy model
// (`modules/aws/codegen`). An object's body is a `stream.InStream` — the same
// token a file is — so PutObject *from a file* hands over the stream
// `open_read` minted, and GetObject *to a file* `pipe`s the body it answers
// with into the stream `open_write` minted, without blocking a worker. Both
// ends live in the one `Streams` in scope: here `MemFs`, which wears `Fs` and
// `Streams` both; on the host, the table every host producer registers into.
//
// Nothing here imports `aws.s3.host`, so the build needs neither SDK.

import aws
import aws.s3
import fs
import fs.mem
import stream

// What a failure says, in a line.
fn describe(e: S3Error) [] -> Str => e {
    if e is AwsError {
        return "${e.code}: ${e.message}"
    }
    if e is NoSuchKey {
        return "no such key"
    }
    return "the service refused"
}

// PutObject from a file. The input holds the stream, so it is linear: it is
// handed on whole, and the handler closes the body.
fn upload(bucket: Str, key: Str, path: Str) [S3, Fs, Console] => bucket, key, path {
    let opened = open_read(path)
    if opened is Err {
        println("open ${path}: ${detach(opened)}")
        return None
    }
    let put = waitfor r: Reply<Ok PutObjectOutput | Err Checked<S3Error>> {
        put_object(PutObjectInput { bucket: copy(bucket), key: copy(key), body: opened }, r)
    }
    when put {
        is Ok { println("put ${key}: etag ${put.e_tag ?: "?"}") }
        is Err { println("put ${key}: ${describe(detach(put))}") }
    }
}

// GetObject to a file. The output holds the body, so it is linear too: take
// the stream out, and `pipe` copies it into the file and closes both.
fn download(bucket: Str, key: Str, path: Str) [S3, Fs, Console] => bucket, key, path {
    let got = waitfor r: Reply<Ok GetObjectOutput | Err Checked<S3Error>> {
        get_object(GetObjectInput { bucket: copy(bucket), key: copy(key) }, r)
    }
    if got is Err {
        println("get ${key}: ${describe(detach(got))}")
        return None
    }
    let {body, content_length} = got
    println("get ${key}: ${content_length ?: -1L} bytes")
    let target = open_write(path)
    if target is Err {
        println("open ${path}: ${detach(target)}")
        let closed = close(body)
        if closed is Err {
            ignore(closed)
        }
        return None
    }
    let copied = waitfor r: Reply<Ok Long | Err Checked<StreamError>> {
        pipe(body, target, r)
    }
    when copied {
        is Ok { println("piped ${copied} bytes into ${path}") }
        is Err { println("pipe: ${detach(copied)}") }
    }
}

// Puts a file, gets it back into another, and prints what arrived.
fn round_trip(key: Str) [S3, Fs, Console] => key {
    upload("notes", copy(key), "notes.txt")
    download("notes", copy(key), "back.txt")
    let back = read_to_str("back.txt")
    when back {
        is Ok {
            println("back.txt: ${size(back)} bytes")
            print(back)
        }
        is Err { println("back.txt: ${detach(back)}") }
    }
}

// An object store in memory, written the way a test would write one: the
// generated effect is the contract, so any handler of it will do. It reads a
// body it is handed through the `Streams` in scope, as the host glue reads
// one out of the host's table, and answers a body minted in the same place.
handler MemS3() [Streams] of S3 {
    objects: Mut Map<Str, Bytes> = mut_map_of()

    fn put_object(input: PutObjectInput, reply: Reply<Ok PutObjectOutput | Err Checked<S3Error>>) -> None
    => !input, !reply {
        let {bucket, key, body} = input
        let buf = mut_bytes()
        let filled = fill_from(body, buf)
        let closed = close(body)
        if closed is Err {
            ignore(closed)
        }
        if filled is Err {
            reply.send(err(checked<S3Error>(AwsError { code: "StreamFailed", message: "${detach(filled)}" })))
            return None
        }
        let data: Bytes = buf
        let tag = "\"${size(data)}\""
        put(objects, "${bucket}/${key}", data)
        reply.send(ok(PutObjectOutput { e_tag: tag }))
    }

    fn get_object(input: GetObjectInput, reply: Reply<Ok GetObjectOutput | Err Checked<S3Error>>) -> None
    => !input, !reply {
        let found = get(objects, "${input.bucket}/${input.key}")
        if found is None {
            reply.send(err(checked<S3Error>(NoSuchKey {})))
            return None
        }
        let data: Bytes = copy(found)
        let length = to_long(size(data))
        reply.send(ok(GetObjectOutput { body: from_bytes(data), content_length: length }))
    }
}

fn main() [use] {
    use StdOutConsole()
    use MemFs()
    let written = write_str("notes.txt", "hello from Salvo\nsecond line\n")
    if written is Err {
        println("write: ${detach(written)}")
        return None
    }

    // 1. The generated recording fake: the body it is handed is closed unread,
    //    and the body it answers with is empty.
    println("-- FakeS3 --")
    if true {
        use FakeS3()
        round_trip("greeting.txt")
        println("calls: ${calls()}")
    }

    // 2. A double that keeps what it is given, for a test that needs answers.
    println("-- MemS3 --")
    if true {
        use MemS3()
        round_trip("greeting.txt")
        // A modeled error arrives as its own arm of `S3Error`.
        download("notes", "missing.txt", "missing.txt")
    }
}
