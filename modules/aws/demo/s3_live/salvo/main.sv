// Amazon S3 for real, with the bodies as streams: puts a file, gets it back
// into another file, and asks for a key that is not there — through
// `aws.s3.host`'s `HostS3`, the platform's SDK.
//
// Both bodies go through the host's one stream table [stream-table]: the file
// `open_read` minted is the request body the glue reads, and the response body
// the glue registered is what `pipe` copies into the file `open_write` minted —
// without blocking a worker while the network is slow.
//
// Not part of the test suite: it needs the SDKs and a service to talk to. It
// points at `http://localhost:4566` — `local_s3.py` beside it, or LocalStack;
// see ../../README.md. It writes under `out/`.

import aws
import aws.s3
import aws.s3.host
import fs
import fs.host
import stream
import stream.host

fn describe(e: S3Error) [] -> Str => e {
    if e is AwsError {
        return "${e.code}: ${e.message}"
    }
    if e is NoSuchKey {
        return "no such key"
    }
    return "the service said no"
}

// The size of the file at [path], or absent when it cannot be read — which
// `put_object` then refuses, since a streamed body must say its length.
fn size_of(path: Str) [Fs] -> Long? => path {
    let info = metadata(path)
    when info {
        is Ok { return info.size }
        is Err {
            ignore(info)
            return None
        }
    }
}

// PutObject from a file: the stream `open_read` minted is the body, streamed
// as it is read, so the file's size goes with it as `content_length`.
fn upload(bucket: Str, key: Str, path: Str, length: Long?) [S3, Fs, Console] => bucket, key, path, !length {
    let opened = open_read(path)
    if opened is Err {
        println("open ${path}: ${detach(opened)}")
        return None
    }
    let put = waitfor r: Reply<Ok PutObjectOutput | Err Checked<S3Error>> {
        put_object(PutObjectInput { bucket: copy(bucket), key: copy(key), body: opened, content_length: length, content_type: "text/plain" }, r)
    }
    when put {
        is Ok { println("put ${key}: etag ${put.e_tag ?: "?"}") }
        is Err { println("put ${key}: ${describe(detach(put))}") }
    }
}

// GetObject to a file: `pipe` copies the response body into the file without
// blocking a worker, and closes both.
fn download(bucket: Str, key: Str, path: Str) [S3, Fs, Console] => bucket, key, path {
    let got = waitfor r: Reply<Ok GetObjectOutput | Err Checked<S3Error>> {
        get_object(GetObjectInput { bucket: copy(bucket), key: copy(key) }, r)
    }
    if got is Err {
        println("get ${key}: ${describe(detach(got))}")
        return None
    }
    let {body, content_length, content_type} = got
    println("get ${key}: ${content_length ?: -1L} bytes of ${content_type ?: "?"}")
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

fn main() [use] {
    use StdOutConsole()
    use HostRawStreams()
    use DefaultStreams()
    use HostRawFs()
    use DefaultFs()
    use HostS3(AwsConfig {
        credentials: ProfileCredentials {},
        region: Region { code: "eu-west-1" },
        endpoint: "http://localhost:4566"
    })
    let made = create_dirs("out")
    if made is Err {
        println("create out/: ${detach(made)}")
        return None
    }
    let written = write_str("out/upload.txt", "hello from Salvo\nsecond line\n")
    if written is Err {
        println("write: ${detach(written)}")
        return None
    }
    upload("salvo-demo", "greeting.txt", "out/upload.txt", size_of("out/upload.txt"))
    // Without its length a streamed body is refused, and nothing is sent.
    upload("salvo-demo", "unsized.txt", "out/upload.txt", None)
    download("salvo-demo", "greeting.txt", "out/download.txt")
    let back = read_to_str("out/download.txt")
    when back {
        is Ok { print("out/download.txt says:\n${back}") }
        is Err { println("read back: ${detach(back)}") }
    }
    // A modeled error arrives as its own arm of `S3Error`.
    download("salvo-demo", "missing.txt", "out/missing.txt")
}
