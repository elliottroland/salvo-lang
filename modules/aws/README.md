# aws

AWS services for Salvo programs. Each service is a **generated** Salvo module —
`aws.sqs` and `aws.s3` today — whose effect has one member per operation. Every member takes
a `Reply` and returns at once; the host implementation answers from the
platform SDK's own asynchronous machinery (aws-sdk-kotlin on the JVM,
aws-sdk-rust on Rust), so no Salvo worker waits on the network. An S3 object's body is a
`stream.InStream`, the same token a file is, so an object goes from a file and
into a file with no adapter. `modules/aws/DESIGN.md` has the design and every
decision behind it.

```
modules/aws/
├── salvo.toml          the dependency's manifest (host libraries for the glue)
├── DESIGN.md           the design
├── smithy-build.json   what to generate: one projection per service
├── models/             the pinned Smithy models (and COMMIT, where they came from)
├── codegen/            the generator: a smithy-build plugin, built with Gradle
├── salvo/
│   ├── aws.sv          hand-written: AwsConfig, Credentials, Region, AwsError
│   ├── aws/sqs.sv      generated: shapes, errors, the `Sqs` effect, `FakeSqs`
│   ├── aws/sqs/host.sv generated: `HostSqs`, the platform handler
│   ├── aws/s3.sv       generated: `PutObject`/`GetObject`, the `S3` effect, `FakeS3`
│   ├── aws/s3/host.sv  generated: `HostS3`
│   └── platform/aws/{sqs,s3}/host.{kt,rs}   generated host glue
├── lib/kotlin/         (not checked in) the Kotlin SDK jars, fetched by Gradle
└── demo/               programs against the real SDKs, run by hand:
    ├── sqs_live/       + local_sqs.py, an in-memory SQS
    └── s3_live/        + local_s3.py, an in-memory S3
```

## Using it

A project names the module under `[dependencies]` and says where modules live:

```toml
[build]
modules = "../../modules"   # the directory holding `aws/`

[dependencies]
aws = "0.1.0"
```

`import aws.sqs` brings the surface. A program that uses only `FakeSqs` (or a
handler of its own) needs no SDK at all: the SDKs are needed only when
`aws.sqs.host` is imported, because a dependency's host libraries join a build
only when its platform code is reached. `examples/aws_sqs/` is that case, and
runs in the test suite.

`import aws.s3` works the same way, with one addition: bodies are streams, so a
program also needs a `Streams` bound (`import stream`). `FakeS3` mints and
closes its bodies through that `Streams`, and so does a hand-written double.
With `HostS3`, bind the host's streams — `HostRawStreams()` then
`DefaultStreams()` — since the glue registers a response body in the host's
stream table and reads a request body out of it; a `MemFs` stream handed to
`HostS3` traps. `examples/aws_s3/` runs the fake and an in-memory double over
`MemFs` in the test suite.

**Enums and errors are data.** An enum is a union of its wire values and `Other
Str` (`"STANDARD" | "GLACIER" | … | Other Str`), a plain string at run time.
Every operation answers `Err Checked<SqsFailure>` (or `S3Failure`): `SqsError`
when the service answered — `code` (a union of the modeled codes and `Other`,
normalized to the model's names on both backends), `message`, HTTP `status`,
`request_id` — or `AwsError` when there was no answer at all.

A structure holding a body (`PutObjectInput`, `GetObjectOutput`) is a
`linear struct`: take the stream out with `let {body} = output`, or give the
whole value up with the generated `close`.

**An upload streams, so it must say its length.** `PutObjectInput.content_length`
is the body's size in bytes — for a file, `metadata(path)`'s `size`. Without
it, `put_object` answers `AwsError { code: "MissingContentLength" }` and sends
nothing (`FakeS3` refuses the same way, so a test catches it); a body shorter or
longer than it answers `AwsError { code: "StreamFailed" }` and nothing is
stored. Nothing is buffered to find a length out, so memory stays at a few
64 KiB chunks whatever the object's size. A failed upload is not retried: a
stream is read once.

## Setting up from scratch

What a new machine needs, in order. Everything below is run from the
repository root unless it says otherwise.

1. **The Salvo toolchain** — as for the rest of the repository: a Rust
   toolchain (`rustup`, with `cargo` and `rustc` on `PATH`), and `kotlinc`
   (2.x) for the Kotlin backend. `cargo build` builds the compiler.

2. **A JDK, 21 or newer** (the generator's Gradle toolchain asks for 21; any
   OpenJDK/Corretto works). Gradle itself is *not* needed — `codegen/gradlew`
   downloads the version it is pinned to (9.8.0) on first use:

   ```bash
   java -version        # 21+
   ```

3. **Network access** on the first run of each of these, for Maven Central,
   crates.io, and (only when refreshing models) GitHub.

4. **Python 3** (standard library only) for the local stand-ins the demos
   talk to.

### Regenerating the modules

Only needed after changing the generator, `smithy-build.json`, or a model; the
generated files are checked in.

```bash
cd modules/aws/codegen
./gradlew generate          # runs smithy-build over ../smithy-build.json,
                            # writes ../salvo/aws/**.sv and ../salvo/platform/**
```

Then check the surface still types: `../../../target/debug/salvo analyze` from
`modules/aws`.

**Adding an operation** is a line in `smithy-build.json`'s `operations` list.
**Adding a service** is a projection: import its model, name the module, the
effect, the Rust crate and the Kotlin package — and add the crate to
`salvo.toml`'s `[rust] crates` and the artifact to `[kotlin] artifacts` and to
`codegen/build.gradle.kts`'s `kotlinSdk` configuration (then run
`./gradlew fetchKotlinSdk` again). Two more settings exist for services that
need them: `forcePathStyle` (S3: with an `endpoint` override, address buckets
in the path, which a local stand-in needs) and `omitMembers`, a list of member
shape ids to leave out where the SDKs customize a member away from the model
(S3's `Expires` is a string in the model and a timestamp in both SDKs). `errorGroups` names a group of error codes by the prefix of their model
names (SQS: `{"KmsErrorCode": "Kms"}`), a literal sub-union a caller tests in
one `is`.

### Refreshing a model

The models are vendored from [`aws/api-models-aws`](https://github.com/aws/api-models-aws)
at the commit in `models/COMMIT`. To move to a newer one:

```bash
cd modules/aws/models
sha=$(curl -s https://api.github.com/repos/aws/api-models-aws/commits/main \
      | python3 -c 'import sys,json; print(json.load(sys.stdin)["sha"])')
base="https://raw.githubusercontent.com/aws/api-models-aws/$sha/models"
curl -sSL -o sqs-2012-11-05.json "$base/sqs/service/2012-11-05/sqs-2012-11-05.json"
curl -sSL -o s3-2006-03-01.json  "$base/s3/service/2006-03-01/s3-2006-03-01.json"
printf 'aws/api-models-aws %s\nsqs-2012-11-05.json  <- models/sqs/service/2012-11-05/sqs-2012-11-05.json\ns3-2006-03-01.json   <- models/s3/service/2006-03-01/s3-2006-03-01.json\n' "$sha" > COMMIT
```

then regenerate. A newer model can name members an older SDK lacks (S3 adds
checksum algorithms), so move the SDK versions forward with it.

### Building against the real SDKs

The host glue compiles against the SDKs, which have to be fetched once:

- **Kotlin** — the compiler does not resolve Maven coordinates yet, so the
  module's Gradle build copies the SDK's jar closure into `lib/kotlin/`, which
  `[kotlin] libs` puts on the classpath:

  ```bash
  cd modules/aws/codegen
  ./gradlew fetchKotlinSdk    # ~40 jars into ../lib/kotlin (S3 and SQS)
  ```

- **Rust** — nothing to do by hand: with `[rust] crates` declared, `salvo run`
  writes a `Cargo.toml` and builds with `cargo`, which fetches `aws-config`,
  `aws-sdk-s3`, `aws-sdk-sqs` and `tokio` from crates.io. The first build takes
  a minute or two, and so does each first build of a new output directory.

### Running the live demos

Both demos need credentials the SDK will accept; the stand-ins do not check
them, so any pair will do:

```bash
mkdir -p /tmp/salvo-aws/.aws
printf '[default]\naws_access_key_id = AKIDEXAMPLE\naws_secret_access_key = secret\n' \
  > /tmp/salvo-aws/.aws/credentials
```

Both listen on port 4566, LocalStack's, so run one stand-in at a time.

#### SQS

`demo/sqs_live` creates a queue, sends and receives a message, deletes the
queue, and asks for a queue that no longer exists (a modeled error). It talks
to `http://localhost:4566`, so it runs against either a local stand-in or
LocalStack, and needs credentials the SDK can load.

With the stand-in beside it (Python 3, standard library only):

```bash
cd modules/aws/demo/sqs_live
python3 local_sqs.py &                 # an in-memory SQS on :4566

# `HOME` points the profile lookup at those credentials; rustup and cargo
# still need to find their own homes.
RUSTUP_HOME=$HOME/.rustup CARGO_HOME=$HOME/.cargo HOME=/tmp/salvo-aws \
  AWS_EC2_METADATA_DISABLED=true \
  ../../../../target/debug/salvo run --backend rust
HOME=/tmp/salvo-aws ../../../../target/debug/salvo run --backend kotlin

kill %1
```

Both print:

```
created http://localhost:4566/000000000000/salvo-demo
sent <a message id>
received: hello from Salvo
deleted the queue
get_queue_url failed: no such queue (HTTP 400)
```

Against real AWS, drop the `endpoint` in `demo/sqs_live/salvo/main.sv`, set
the region, and use your own profile.

#### S3

`demo/s3_live` writes `out/upload.txt`, puts it as an object (the file's stream
is the request body), gets it back into `out/download.txt` with `pipe` (the
response body is a stream in the host's table, copied without blocking a
worker), reads that file back, and asks for a key that does not exist.
`local_s3.py` keeps objects in memory and creates a bucket on the first put.

```bash
cd modules/aws/demo/s3_live
python3 local_s3.py &                  # an in-memory S3 on :4566

RUSTUP_HOME=$HOME/.rustup CARGO_HOME=$HOME/.cargo HOME=/tmp/salvo-aws \
  AWS_EC2_METADATA_DISABLED=true \
  ../../../../target/debug/salvo run --backend rust
HOME=/tmp/salvo-aws ../../../../target/debug/salvo run --backend kotlin

kill %1
```

Both print:

```
put greeting.txt: etag "9bc5924990fcdf76fc78dba03c9dae80"
get greeting.txt: 29 bytes of text/plain
piped 29 bytes into out/download.txt
out/download.txt says:
hello from Salvo
second line
get missing.txt: no such key (HTTP 404)
```

Against real AWS, drop the `endpoint` in `demo/s3_live/salvo/main.sv`, set the
region and a bucket you own, and use your own profile.

To build the generated Rust yourself, `salvo compile --backend rust --target
out/rust` and then `cargo build --manifest-path out/rust/Cargo.toml`, as the
`compile` hint says.

## What the generator does not map yet

It stops with an error naming the shape rather than emitting something
approximate: documents, event streams, Smithy unions, big numbers, and
non-scalar `@default`s. Enum-keyed maps are `Map<Str, V>` keyed by the enum's
wire value; timestamps are `time.Instant`; a `@streaming` blob is a
`stream.InStream`, streamed both ways. Paginators and waiters are not generated.
