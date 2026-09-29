# aws

AWS services for Salvo programs. Each service is a **generated** Salvo module —
`aws.sqs` today — whose effect has one member per operation. Every member takes
a `Reply` and returns at once; the host implementation answers from the
platform SDK's own asynchronous machinery (aws-sdk-kotlin on the JVM,
aws-sdk-rust on Rust), so no Salvo worker waits on the network.
`modules/aws/DESIGN.md` has the design and every decision behind it.

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
│   └── platform/aws/sqs/host.{kt,rs}   generated host glue
├── lib/kotlin/         (not checked in) the Kotlin SDK jars, fetched by Gradle
└── demo/sqs_live/      a program against the real SDKs, run by hand
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
`codegen/build.gradle.kts`'s `kotlinSdk` configuration.

### Refreshing a model

The models are vendored from [`aws/api-models-aws`](https://github.com/aws/api-models-aws)
at the commit in `models/COMMIT`. To move to a newer one:

```bash
cd modules/aws/models
sha=$(curl -s https://api.github.com/repos/aws/api-models-aws/commits/main \
      | python3 -c 'import sys,json; print(json.load(sys.stdin)["sha"])')
curl -sSL -o sqs-2012-11-05.json \
  "https://raw.githubusercontent.com/aws/api-models-aws/$sha/models/sqs/service/2012-11-05/sqs-2012-11-05.json"
printf 'aws/api-models-aws %s\nsqs-2012-11-05.json  <- models/sqs/service/2012-11-05/sqs-2012-11-05.json\n' "$sha" > COMMIT
```

then regenerate.

### Building against the real SDKs

The host glue compiles against the SDKs, which have to be fetched once:

- **Kotlin** — the compiler does not resolve Maven coordinates yet, so the
  module's Gradle build copies the SDK's jar closure into `lib/kotlin/`, which
  `[kotlin] libs` puts on the classpath:

  ```bash
  cd modules/aws/codegen
  ./gradlew fetchKotlinSdk    # ~40 jars into ../lib/kotlin
  ```

- **Rust** — nothing to do by hand: with `[rust] crates` declared, `salvo run`
  writes a `Cargo.toml` and builds with `cargo`, which fetches `aws-config`,
  `aws-sdk-sqs` and `tokio` from crates.io. The first build takes a minute or
  two.

### Running the live demo

`demo/sqs_live` creates a queue, sends and receives a message, deletes the
queue, and asks for a queue that no longer exists (a modeled error). It talks
to `http://localhost:4566`, so it runs against either a local stand-in or
LocalStack, and needs credentials the SDK can load.

With the stand-in beside it (Python 3, standard library only):

```bash
cd modules/aws/demo/sqs_live
python3 local_sqs.py &                 # an in-memory SQS on :4566

# Credentials the SDK will accept (the stand-in does not check them):
mkdir -p /tmp/salvo-aws/.aws
printf '[default]\naws_access_key_id = AKIDEXAMPLE\naws_secret_access_key = secret\n' \
  > /tmp/salvo-aws/.aws/credentials

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
get_queue_url failed: no such queue
```

Against real AWS, drop the `endpoint` in `demo/sqs_live/salvo/main.sv`, set
the region, and use your own profile.

## What the generator does not map yet

It stops with an error naming the shape rather than emitting something
approximate: documents, event streams, `@streaming` blobs (S3's bodies — next,
over `std.stream`), timestamps, Smithy unions, big numbers, and non-scalar
`@default`s. Enum-keyed maps are `Map<Str, V>` keyed by the enum's wire value.
Paginators and waiters are not generated.
