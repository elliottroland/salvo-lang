# aws_s3

Amazon S3 from Salvo, with no AWS anywhere, and objects as **streams**. `aws.s3`
is **generated** from the service's Smithy model by `modules/aws/codegen`
(`PutObject` and `GetObject`); this program is written against its `S3` effect
and runs against two handlers that need no SDK, over the in-memory filesystem.

Run it:

```bash
cd examples/aws_s3
cargo run -q --manifest-path ../../Cargo.toml -- run
```

## What to look for

**A body is a `stream.InStream`**, the same token a file is. `PutObject` from a
file hands over the stream `open_read` minted, as the input's `body`; `GetObject`
to a file takes the answer's `body` out and `pipe`s it into the stream
`open_write` minted — a chain of non-blocking reads, closing both ends. Both
ends are in the one `Streams` in scope: here `MemFs`, which wears `Fs` and
`Streams` both. On the host it is the stream table every host producer
registers into, which is why the real glue needs no adapter
(`modules/aws/demo/s3_live`).

**Holding a stream makes a struct linear.** `PutObjectInput` and
`GetObjectOutput` are `linear struct`s, so neither can be dropped with its body
unread: the input is handed on whole, and the output is taken apart with
`let {body, content_length} = got` — the stream owes, the rest does not. The
module generates a `close` for each, which gives the whole value up.

**Two doubles.** `FakeS3` is generated with the module: it records each call by
name (`S3Calls.calls()`), closes a body it is handed without reading it, and
answers `GetObject` with an empty body. `MemS3`, written in this file, keeps
what it is given — reading the body through `Streams` as the host glue reads
one out of the host's table — and answers a missing key with the model's
`NoSuchKey`.

**No SDK in the build.** Nothing imports `aws.s3.host`, so no host glue is
reached and neither SDK joins the build.
