# The `aws` module — design

How AWS services become Salvo: generated from the public Smithy models, wrapping
the host SDKs each backend already has, non-blocking, with streaming bodies as
ordinary Salvo streams. Decided with the user on 2026-09-29 (the decision log in
the repository's COMPLETED.md has the entry). Built: the generator, `aws.sqs`
and `aws.s3` (§8 steps 3–5; the "Built" sections below).

The module is a **user of Salvo**, not part of the compiler: it lives in
`modules/aws/`, is consumed through `[dependencies]` like any other project
([manifest-deps]), and its generator is its own tool. Where it needs the
language or the standard library to grow, that growth is specified on its own
terms, with no AWS in it, and lands in the compiler repository as ordinary
roadmap steps — §7 lists them.

## 1. Decisions

| # | Decision | Chosen | Over |
|---|---|---|---|
| D1 | How services are implemented | **Wrap the host SDKs** (aws-sdk-kotlin, aws-sdk-rust) through generated platform handlers — the `smithy-dafny` pattern | implementing SigV4 and the AWS protocols in Salvo over an HTTP effect; left open as a later replacement, since the generated effect is the contract and who implements it is a `use` |
| D2 | Where the generator lives | **`modules/aws/codegen/`, a `smithy-build` plugin on the JVM** (`salvo-client-codegen`), its own build, no dependency on the compiler | a Rust tool over the JSON AST; a `salvo` subcommand (refused: the module must not extend the CLI) |
| D3 | The Salvo shape of a service | A **plain effect** whose members take a `Reply<…>` and return at once — non-blocking without an actor kind the host cannot honour | an `actor effect` per service with a platform handler (needs a host-side mailbox); a blocking `platform effect` (wastes a worker per call) |
| D4 | Streaming bodies | `std.stream`: **`ByteSource`**, one plain effect in scope that reads any stream asynchronously, threading the linear stream token through the reply; **`InStream` reused** as the stream type, moved to `std.stream` | an `Addr<ByteSource>` actor per body; a separate `ByteStream` type |
| D5 | The mint | **Folded into `ByteSource`** (`from_bytes`) | a separate `ByteStreamMint` face — a distinction with no customer yet |
| D6 | Enums | union of unit tags (`when` exhaustively) — **superseded 2026-09-30** by a union of string literals with `Other Str` ([type-literal]) | a qualifier-carrying `Str` |
| D7 | First services | **SQS** (small, `awsJson`, no streaming) to prove the pipeline, then **S3** `GetObject`/`PutObject` for the stream | DynamoDB first |
| D8 | Models | pinned JSON ASTs from [`aws/api-models-aws`](https://github.com/aws/api-models-aws) vendored under `models/`, with the commit recorded | referencing the repository by commit |

The second sitting the same day settled the open points (numbered as they were
put to the user; all as recommended):

| # | Decision | Chosen |
|---|---|---|
| 1 | Host dependencies in the manifest | `[rust] crates = { … }`, `[kotlin] artifacts = [ … ]` in the backend sections; a dependency's are merged into the build |
| 2 | Rust build path | `Cargo.toml` + `cargo` **only when crates are declared**; the bare `rustc` path and the checked-in example trees stay |
| 3 | Kotlin artifacts | a **directory of jars** (`[kotlin] libs = "lib/kotlin"`, gitignored) on the classpath now; Maven resolution by the compiler later. The module's own Gradle build fetches the closure into `lib/kotlin/` (`fetchKotlinSdk`), so the compiler never learns about Maven |
| 4 | Host-dependency version conflicts | refused, naming both |
| 5 | A reply the host never discharges | reported to the pool's fault sink where detectable (Rust `Drop`), best-effort and documented |
| 6 | Host-minted replies | deferred with the writer pair |
| 7 | `Reply<T>` as a platform-member parameter | to *verify* at step 2, not decide |
| 8 | Chunk sizing | provider-sized; a `Chunk` is never empty; a `max` overload if ever needed |
| 9 | `StreamError` | `Err Checked<StreamError>`, linear like `FsError` |
| 10 | A handle from the wrong provider | **trap** with a message (a program bug, not a condition) — and a roadmap item to make it a *compile-time* error, §9 |
| 11 | Stream table | process-global from day one, shared by `HostRawFs` and `HostByteSource` |
| 12 | The `InStream` move | both `InStream` and `OutStream` move; `import stream.InStream` beside `import fs` is accepted (no re-export) |
| 13 | Fakes for streams | `MemByteSource` alone now; `MemHost of Fs, ByteSource` when a test needs a `MemFs` stream read asynchronously (the mem pair does not share a table, deliberately) |
| 14 | smithy-rs codegen is **not published** to Maven | depend on `smithy-kotlin-codegen`; reimplement the Rust naming rules (snake-casing, reserved words) — small, and the correction to D2's rationale |
| 15 | Handler constructor | one `AwsConfig { credentials: Credentials, region: Region, endpoint: Str? }`, `Credentials = ProfileCredentials \| EnvironmentCredentials \| DefaultChain`; `endpoint` is what LocalStack needs |
| 16 | Enum tag names | namespaced under the enum's type (`StorageClass.Standard`) — needs **dot-names under a `type`**, a small language extension, over prefixed names. **Superseded 2026-09-30**: an enum's values are its literals |
| 17 | Open enums | every enum gets an `Unknown Str` arm, as both SDKs do — now the `Other Str` arm |
| 18 | Fakes for services | **generated**: a recording `Fake<Service>` handler beside the real one |
| 19 | Pinned versions | current at the time step 4 begins |

## 2. Why wrapping, and the precedent

Both host SDKs are themselves generated from these Smithy models, by
`smithy-kotlin` and `smithy-rs`. `smithy-dafny` — Dafny is, like Salvo, a
language that transpiles to several hosts and has no runtime of its own —
generates a Dafny client as Dafny source *plus* host source *plus* a dependency
on the host SDK artifact, per target. That is this module with Kotlin and Rust
as the targets. Signing, endpoint rules, retries, credential chains and
pagination stay the hosts' problem; what we generate is the Salvo surface and
the glue. Two things make the glue mechanical: the host SDKs' member names are
derived from the model by rules that live in `smithy-kotlin`'s and `smithy-rs`'s
symbol providers, and the generator can call those providers rather than
reimplement them — which is the reason the generator is a JVM plugin (D2).
Correction (decision 14): `smithy-rs`'s codegen is not published to Maven
(`smithy-dafny` carries it as a git submodule), so the plugin depends on
`smithy-kotlin-codegen` and reimplements the Rust side's naming — snake-casing
and a reserved-word list, a page of code — which leaves the Kotlin half, the
larger one, still borrowed rather than mirrored.

### Third sitting (2026-09-29, evening): the stream layering

Building §7.3 found that moving `InStream` out of `fs` breaks the same-file
rule [linear-group] — a linear value's terminal belongs to the type's own
file, and `Fs.close` and `ByteSource.close` cannot both live there. The fix is a
layering, not a rule change, and it **replaces decisions 11–13 and the
`ByteSource` sketch** (§5 below is rewritten to match):

| # | Decision | Chosen |
|---|---|---|
| 20 | Who owns streams | **`std.stream`**: `InStream`, `OutStream` and **one effect `Streams`** with every stream operation — the synchronous reads and writes moved out of `Fs`, the non-blocking `receive`, `close`, `from_bytes`, the `Lines`/`Chunks` iterators and `copy_stream`. `fs` keeps paths and *mints* into the `Streams` in scope; S3 mints bodies the same way. All-host and all-mem are each consistent; mixing traps |
| 21 | The same-file rule | **kept** — the terminal stays with the type; the layering puts every consuming member beside it |
| 22 | Handles | **one process-wide counter** for every stream table, host and mem, so a foreign handle is unknown, never a collision |
| 23 | `[Fs, Streams]` everywhere | **effect prerequisites**: `effect Fs [Streams]` — wherever `Fs` is, `Streams` is too (transitively); every `Fs` handler has `Streams` as an implicit dependency; binding an `Fs` handler requires a `Streams` already bound. A *prerequisite*, not inheritance: inheritance would give each producer its own stream table |
| 24 | Names | `Streams`, `HostStreams`, `MemStreams`; `receive(s, reply: Reply<Received>)`, `Received = Ok Packet \| End \| Err Checked<StreamError>`, `Packet { bytes, stream }`; `FsError` keeps the path kinds, `StreamError` takes `InvalidUtf8`, `StaleHandle` and an `IoError` of its own |
| 25 | Non-blocking copy | `pipe(from: InStream, to: OutStream, done: Reply<Ok Long \| Err Checked<StreamError>>)` beside the synchronous `copy_stream` — GetObject to a file |
| 26 | Synchronous reads of a network stream | allowed, blocking the worker as a slow disk does, and documented; `receive`/`pipe` are the non-blocking route |
| 28 | Host layering for streams | `RawStreams` (plain handles and errors) + `threadsafe platform handler HostRawStreams` + `DefaultStreams [RawStreams] of Streams` in Salvo — the `RawFs`/`HostRawFs`/`DefaultFs` shape, since a host class never holds an obligation. Replaces "HostStreams" |
| 29 | The mem world | **`handler MemFs of Fs, Streams`**: one table, one file map, both faces (a write stream must publish into the file map on close); no separate `MemStreams` until a test without a filesystem needs one |
| 30 | Stream error kinds | `InvalidUtf8 { source }`, `StaleHandle { source }`, `StreamFailed { source, message }` — `source` rather than `path`, since an S3 body has none; `fs.IoError` stays with the path operations |
| 31 | The host table's home | each backend's runtime, behind a small read/write/flush/close interface, so every host file registers into it whatever modules a program reaches |
| 32 | Bindings | a host program binds four handlers; **handler bundles** are a recorded future item (ROADMAP §4b item 6), with streams + fs their first customer |
| 27 | The host table | owned by `HostStreams`, one per process; `HostRawFs` registers opened files into it, so a file is an S3 body with no adapter (PutObject from a file) |

### Built: the generator and SQS (2026-09-29)

§8 steps 3 and 4 are built: `codegen/` (a `smithy-build` plugin plus a driver,
Gradle wrapper checked in), `aws.sqs` for six operations (CreateQueue,
GetQueueUrl, SendMessage, ReceiveMessage, DeleteMessage, DeleteQueue),
`examples/aws_sqs` in the suite (the generated fake and a hand-written
in-memory double), and `demo/sqs_live` run by hand against the real SDKs and
the stand-in `local_sqs.py`, printing the same on both backends. Choices made
while building, within the decisions above — worth a second look:

- **Errors are `Err Checked<SqsFailure>`**, service-wide rather than per
  operation: `Checked` for consistency with `std.fs`/`std.stream`, one type so
  a caller's error handling is one function. (Rebuilt 2026-09-30 — see "Enums
  and errors as data" below.)
- **An `operations` allowlist** in `smithy-build.json` rather than the whole
  service: the first slice is what a program uses, and adding an operation is a
  line.
- **Enum-keyed maps are `Map<Str, V>`** keyed by the enum's wire value.
- **The fake answers an empty success** wherever the output has no required
  members, else `AwsError { code: "NotStubbed" }`, and records operation names
  through a second face `SqsCalls` — recording inputs is left for when a test
  wants it.
- **Rust names use smithy-kotlin's word-boundary splitter**, which is the one
  smithy-rs copied (decision 14); the Kotlin names come from smithy-kotlin's
  own `NamingKt`. Enums cross by wire value on both sides (`fromValue` /
  `from`), so variant naming never has to match.
- **The Kotlin artifact list lives twice** — `salvo.toml`'s `[kotlin] artifacts`
  and `codegen/build.gradle.kts`'s `kotlinSdk` — until the compiler resolves
  Maven itself.

### Built: S3 and streaming bodies (2026-09-30)

§8 step 5: `aws.s3` for `PutObject` and `GetObject`, `examples/aws_s3` in the
suite (the generated fake and a hand-written `MemS3` over `MemFs`), and
`demo/s3_live` with `local_s3.py`, run by hand through both SDKs with
identical output — a file put and got back, a 3 MiB binary round trip
byte-identical, and a modeled `NoSuchKey`. Choices made while building, within
the decisions — worth a second look:

- **A structure holding a stream is a `linear struct`**, with a generated
  `close(value)` beside it as its discharger ([linear-group] needs one in the
  type's own file). A caller takes the body out (`let {body} = output`) and the
  other members owe nothing.
- **A streaming member is always present** (`body: InStream`, no `?`): its
  `@default` is the empty body, which Salvo cannot write as a literal of a
  linear type.
- **Request bodies stream, and must say their length** (user decisions the
  same day, after a first cut that buffered): the caller's `content_length` is
  the length — found by the member bound to the `Content-Length` header, and a
  streamed input without one is refused by the generator — and without it the
  call answers `AwsError { code: "MissingContentLength" }`, sending nothing;
  never buffering to find it out, so memory is predictable. Rust: a blocking
  thread reads 64 KiB chunks into a four-slot channel an `http_body::Body`
  drains (`ByteStream::from_body_1_x`; `bytes` and `http-body` crates, tokio's
  `sync`). Kotlin: an `InputStream` over the table stream, through
  `asByteStream(length)`. A body shorter or longer than its length is a
  `StreamFailed` answer, found before the last chunk goes so nothing is stored.
  Not retryable — a stream is read once (accepted). `FakeS3` refuses a missing
  length the same way. Measured on a 200 MiB round trip: 31 MiB peak on Rust,
  and the Kotlin program passing with a 32 MiB heap.
- **The JVM exits when `main` does**, by a stopgap in the Kotlin glue: OkHttp's
  non-daemon dispatcher thread held every program using a host AWS handler open
  for 60s after its last call, so a daemon thread joins `main` and closes the
  client (`salvoCloseWhenMainEnds`). The general fix — the runtime ending the
  process — is ROADMAP §4b item 7, to be designed.
- **Response bodies stay in the SDK.** Rust wraps the `ByteStream` in a
  `std::io::Read` that blocks on the next chunk through the handler's tokio
  handle — the table is read from Salvo workers and the stream host's reader
  thread, never from a tokio task. Kotlin's body lives only inside
  `getObject`'s response block, so the coroutine registers it, answers the
  reply from inside the block, and waits for the stream's `close` before
  leaving it.
- **A request-body read failure** answers `AwsError { code: "StreamFailed" }`,
  the arm for everything the model does not name.
- **`FakeS3` closes a body it is handed unread and answers `GetObject` with an
  empty body** minted in the `Streams` in scope, so it needs `[Streams]` as a
  dependency; a stream member no longer makes an output "not stubbable".
- **Timestamps are `time.Instant`** (§4's table), nanoseconds through
  `DateTime::as_nanos` / `epochSeconds`+`nanosecondsOfSecond`; the generated
  file imports `time.Instant` alone.
- **SDK type names follow each SDK's rule**: smithy-rs Pascal-cases shape names
  (`ObjectCannedAcl`), smithy-kotlin's `defaultName(shape, service)` for Kotlin.
  SQS had no acronym in a type name, so this had not shown.
- **`omitMembers`** in the projection leaves out members the SDKs customize
  away from the model — S3's `Expires` is a string in the model and a timestamp
  in both SDKs — noted in the struct's doc comment.
- **`forcePathStyle`** in the projection: with an `endpoint` override, buckets
  are addressed in the path, which a local stand-in needs; real S3 keeps the
  virtual-host style.
- **Admonitions are not summaries**: a doc comment's first paragraph skips
  `<important>`/`<note>` blocks (S3's `PutObject` opens with an
  end-of-support notice). One SQS field comment changed with it.

### Rebuilt: the glue as platform templates (2026-09-30)

The user found host code split per backend inside one Salvo function, and the
Salvo → host → Salvo nesting of fences and holes, hard to read, and moved host
code back into `platform/`: `platform/aws/<svc>/host.sv.kt` and `.sv.rs`, host
files in which anything between backticks is Salvo. `aws/<svc>/host.sv` is
just the bodiless `threadsafe platform handler` declaration. The generator
still writes the same code; `toTemplate` rewrites its hole spelling into
markers. Rust's SDK client and runtime are host fields
(`` `struct HostSqs` { rt: … = salvo_runtime(), client: … = salvo_client(&rt, …) } ``),
with `new` and the struct laid out by the compiler.

### Rebuilt: the glue as host splices (2026-09-30, superseded the same day)

The host implementation of each service is no longer a pair of generated
`platform/aws/<svc>/host.{rs,kt}` files. It is `aws/<svc>/host.sv`: one
`threadsafe platform handler` written in place [host-splice], its members and
handler-level blocks fenced Kotlin and Rust, and every Salvo-side spelling a
hole the compiler renders — a field read (`@{input.queue_name}`), a struct
built from SDK values (`` @{ Message { body: `…` } } ``), an answer wrapped
into its union (`` @{ ok((`value` : Output)) : Ok Output | Err … } ``), a
type (`@{: Checked<SqsFailure>}`), and SDK-side helpers taking a Salvo value
as a typed host name (`` @{(`v` : MessageAttributeValue).data_type} ``).
What is left in the text is the SDKs' API and the documented host ABI
([platform-abi]: `SalvoReply.hosted()`, the stream table, `SalvoMap`), so an
emission change no longer means a generator change. Credentials dispatch
through two Salvo helpers in `aws.sv` (`profile_of`, `uses_environment`)
rather than a match on the union's host shape. Kotlin's file-level opt-in
became an `@OptIn` on the one helper that needs it. The drift test still
compiles both against both SDKs, and both demos print as before.
The glue then lost its type annotations in host text too: helpers take
`@{v : MessageAttributeValue}` and read `@{v.data_type}`, answers are declared
(`val @{answer : Ok … | Err …} = try {`), and the SDK's shapes go by short
names — Rust `use aws_sdk_sqs::types as sdk;` plus the operation modules,
Kotlin one `import …model.Message as SdkMessage` per shape used (a package
cannot be aliased, and a wildcard would collide with the Salvo struct of the
same name). The ascribed form is left only where host code binds the name
itself (`if let Some(p) = …`, a Kotlin null check).

### Rebuilt: enums and errors as data (2026-09-30)

Over unions of literals ([type-literal], the user's proposal): the glue lost
~1,200 of its lines, since the wire string now *is* the Salvo value.

- **An enum is a union of its wire values plus `Other Str`** —
  `type StorageClass = "STANDARD" | … | Other Str` — a plain string on both
  backends. The glue converts with `Sdk::from(s.as_str())`/`v.as_str()` and
  `fromValue(s)`/`v.value`; the per-enum conversion functions are gone, and
  so is the use of dot-names under a `type`.
- **One error struct per service** (user decision): `SqsError { code:
  SqsErrorCode, message, status, request_id? }`, the codes a literal union of
  every error the selected operations name plus `Other Str` (throttling,
  authorization, codes newer than the model). `SqsFailure = SqsError |
  AwsError`; `AwsError` is only a call with no response (a timeout, a dispatch,
  credentials or parse failure), coded by kind. An error's extra members become
  optional fields of the struct (S3's `storage_class`, `access_tier`, set for
  `InvalidObjectState`).
- **`errorGroups`** in the projection names a code group by prefix: SQS's
  `{"KmsErrorCode": "Kms"}` makes the seven KMS codes one sub-union, so `e.code
  is KmsErrorCode` is one test.
- **Codes normalized to the model's names**: a shape id's name, and the legacy
  `@awsQueryError` code (`AWS.SimpleQueueService.NonExistentQueue` →
  `QueueDoesNotExist`) — the glue has one `salvo_code`/`salvoCode` table, and
  the demos print the same on both SDKs.
- **One generic error conversion per glue** (`salvo_failure<E>` over any
  operation's `SdkError<E>`; Kotlin catches `ServiceException`), with a small
  per-operation `extra` for the rare error with fields.
- The **names** `SqsFailure`/`S3Failure` for the union and `SqsErrorCode` for
  the codes are the generator's choice — worth a look.

## 3. Layout

```
modules/aws/
├── salvo.toml                 # name = "aws", version, src = "salvo"
├── DESIGN.md                  # this file
├── smithy-build.json          # one `salvo-client-codegen` entry per service
├── models/                    # pinned JSON ASTs from aws/api-models-aws + COMMIT
├── codegen/                   # the smithy-build plugin: Gradle, Java/Kotlin,
│                              #   depends on smithy-kotlin and smithy-rs codegen
├── salvo/
│   ├── aws.sv                 # hand-written: ProfileCredentials, Region, AwsError
│   ├── aws/sqs.sv             # generated: shapes, errors, the effect, the handler decl
│   ├── aws/s3.sv  aws/s3/host.sv
│   └── platform/aws/          # generated host glue [platform-tree]
│       ├── sqs/host.kt  sqs/host.rs
│       └── s3/host.kt   s3/host.rs
└── demo/                      # sqs_live, s3_live: the real SDKs against local stand-ins
```

(Planned here as a `tests/` directory; the fakes are generated into each
service's module instead, and the suite exercises them from
`examples/aws_sqs` and `examples/aws_s3`.)

The generated files are checked in, as `examples/*/rust` are, and a test
regenerates from `models/` and diffs. Nothing here needs the compiler: the
generator reads a model and writes text.

## 4. The Salvo surface

### Shapes

| Smithy | Salvo | Notes |
|---|---|---|
| structure | `export struct`; `@required` members plain, others `T?`; `@default` → `= …` | |
| operation | one effect member, see below | input/output structures keep their model names |
| union | tagged union: one `qualifier` per member over its type, `type U = A_ A \| B_ B` | arm identity is positional over declaration order — the model's order, never re-sorted |
| enum | union of its string literals plus `Other Str`, since Smithy enums are open — a plain string at run time | intEnum is `Int` |
| `@error` structure | struct; the operation's `Err` arm is the union of its modeled errors plus `AwsError` (unmodeled: throttling, auth, transport) | `@retryable` is the host's business |
| list / map | `List<T>` / `Map<Str, V>` | Smithy maps are string-keyed |
| blob | `Bytes` | |
| `@streaming` blob | `InStream` (D4) | §5 |
| timestamp | `time.Instant` | `import time` in generated files |
| document | **refused** | no `Document` type in Salvo yet; an operation using one is omitted with a note in the generated file |
| event streams | **refused** | later |

Documentation traits become `//` doc comments on the struct, field or member
[doc-comment]; the service's `@documentation` becomes the module's [doc-module].

### Operations: non-blocking through the reply

A service is a **plain effect**. Each operation takes its input and a
`Reply<Ok Output | Err …>` and **returns immediately**; the host starts the
SDK's coroutine or future and, on completion, discharges the reply through the
runtime, exactly as `fire_after(wait, done: Reply<Fired>)` in `std.time` does
today. No Salvo worker waits. This is D3: asynchrony lives in the `Reply`, not
in the effect's kind, so the host needs no mailbox and every member is an
ordinary `fn`.

```
// generated, module aws.sqs
export effect Sqs {
    fn send_message(input: SendMessageInput, reply: Reply<Ok SendMessageOutput | Err Checked<SqsFailure>>) -> None
        => !input, !reply
    fn receive_message(input: ReceiveMessageInput, reply: Reply<Ok ReceiveMessageOutput | Err Checked<SqsFailure>>) -> None
        => !input, !reply
    …
}

export threadsafe platform handler HostSqs(config: AwsConfig) of Sqs
// generated too (18): a recording double for tests
export handler FakeSqs() of Sqs { … }
```

`threadsafe` [threadsafe-platform]: calls arrive from every pool at once and
the host synchronizes its own client. The calling idiom is the actor one —
`send_message(input, replyto on_sent(...))` parks a continuation and returns;
a caller that means to wait writes `waitfor` and pays for it visibly. A test
binds a Salvo fake (`use FakeSqs(...)`) whose members call `send(reply, …)`
synchronously.

`AwsConfig` (15) is hand-written in `aws.sv`: `credentials: Credentials` — a
union of `ProfileCredentials` (already there; maps onto both SDKs' profile
providers), `EnvironmentCredentials` and `DefaultChain` — plus `region` and an
optional `endpoint` override, which is what running against LocalStack needs.

## 5. Streaming: `std.stream` and `Streams`

S3 bodies are `@streaming blob` in both directions, and the host SDKs expose
them as asynchronous byte streams. The general piece is a std module with
nothing AWS in it, and it is where *every* stream lives — files included
(decisions 20–27).

```
// std.stream
export noremote linear struct InStream { handle: Long }
export noremote linear struct OutStream { handle: Long }

export struct Packet { bytes: Bytes, stream: InStream }            // linear by containment; bytes never empty
export type Received = Ok Packet | End | Err Checked<StreamError>

export effect Streams {
    // …every synchronous read and write `Fs` used to carry…
    fn read_line(s: InStream) -> Str | None => s
    fn read_bytes(s: InStream, max: Int) -> Ok Bytes | Err Checked<StreamError> => s
    fn close(s: InStream) -> Ok None | Err Checked<StreamError> => !s
    …
    // Non-blocking: the stream travels with the request and comes back in
    // the answer, so one read is in flight and a mid-read `close` is
    // unwritable.
    fn receive(s: InStream, reply: Reply<Received>) -> None => !s, !reply
    // A stream over a buffer already in hand.
    fn from_bytes(bytes: Bytes) -> InStream => !bytes
}

export threadsafe platform handler HostStreams() of Streams   // the one process table
export handler MemStreams() of Streams                         // pure Salvo

// Copies without blocking a worker: `receive` into `write`, until `End`.
export fn pipe(from: InStream, to: OutStream, done: Reply<Ok Long | Err Checked<StreamError>>) [Streams]
```

`fs` declares `effect Fs [Streams]` (decision 23): a function declaring `[Fs]`
may read the streams `Fs` opens, every `Fs` handler reaches the `Streams` in
scope, and `use DefaultFs()` needs a `Streams` bound first. `HostRawFs`
registers what it opens in `HostStreams`' table; `MemFs.open_read` is
`from_bytes(content)` on whatever `Streams` is bound. So:

- **PutObject from a file**: `put_object(input, open_read(path), reply)` — the
  host glue reads the body through the same table.
- **GetObject to a file**: `pipe(output.body, open_write(path), done)`.
- **All-mem tests**: `MemFs` files and a fake S3's bodies are the same kind of
  stream in one `MemStreams`.
- **Mixing host and mem** is an unknown handle, which traps; with one handle
  counter (decision 22) it can never be someone else's live stream. Making it
  a compile-time error is §9's provider-checked handles.

## 6. Host glue

Generated per service and backend into `salvo/platform/aws/`, so the compiler
loads it as the dependency's companions [manifest-deps] [platform-tree].

- **Kotlin**: the handler owns the SDK client and a `CoroutineScope` on
  `Dispatchers.IO`; each member `launch`es the call and completes the reply
  from the coroutine.
- **Rust**: the handler owns the SDK client and a tokio runtime; each member
  `spawn`s the future and completes the reply from the task.
- Both complete replies through the runtime's **discharge-a-Reply entry point**
  (§7.2) and register bodies in the **stream table** (§7.3). `HostTcpTransport`
  already delivers into Salvo actors from host reader threads on both backends,
  so the threading story is the existing one.

## 7. What the compiler and std must grow

Each is AWS-neutral, specified in its own terms, and a roadmap step in the
compiler repository (ROADMAP §4b). The module cannot proceed past hand-written
types without them.

1. **Host dependencies of platform code.** A companion that wraps
   `aws-sdk-s3` needs the crate on Rust and the artifact on Kotlin; `salvo run`
   today calls `rustc`/`kotlinc` with nothing else. Any platform handler over
   any library needs this: `[rust] crates = [...]`, `[kotlin] artifacts = [...]`
   in the manifest — a dependency's included when it is loaded — and the
   backends emit a `Cargo.toml` / a classpath.
2. **Host-completed continuations.** Expose to platform companions the
   runtime entry point that discharges a `Reply<T>` from host code — the one
   `fire_after` uses privately — on both backends, with the exactly-once
   contract stated in the generated skeleton. No language change: a plain
   effect member taking a `Reply` is already legal.
3. **`std.stream`** (third sitting): effect prerequisites (`effect Fs
   [Streams]`), one handle counter, the `fs`/`stream` split with `Streams`,
   `HostStreams`/`MemStreams`, then `receive`, `from_bytes` and `pipe`;
   `Reply<T canbe linear>`.

## 8. Sequence

1. §7.1 and §7.2 in the compiler, each with std/test coverage.
2. §7.3.
3. `codegen/` as a `smithy-build` plugin: shapes, unions, enums, errors, the
   effect and handler declaration, the two glues; `document` and event streams
   refused loudly.
4. SQS end to end: generated module, a fake in `tests/`, an example that sends
   and receives one message against the fake (the real handler needs
   credentials and runs by hand).
5. S3 `GetObject`/`PutObject` over `Streams`, to and from files included.

## 9. Deferred, recorded

`Document`; event streams; Smithy unions; paginators and waiters (ordinary
Salvo over the operations, later); **uploads of unknown length** — multipart
upload (`CreateMultipartUpload`/`UploadPart`/`CompleteMultipartUpload`), for a
body relayed from a stream whose length nobody knows; the writer pair for Salvo-produced bodies and host-minted
replies (6); asynchronous `Lines`; a consumer-only `ByteSource` face;
implementing the protocols in Salvo (D1's alternative); Maven resolution in the
compiler (3b); transitive dependencies for `modules/aws` itself ([manifest-deps]
leftovers).

**Provider-checked handles** (from 10, on the compiler's roadmap as §4b item 4):
a way for the checker to refuse a `MemFs` stream reaching `HostByteSource`. The
sketch to start from: a handle carries the *domain* it belongs to as a
provenance qualifier (`Host InStream`, `Mem InStream`), each handler declares
the domain its handles live in, and at a `use` the effects in scope must agree
on one domain — so the mismatch is an error at the binding, not at the read.
Open: whether a domain is a qualifier, a type parameter on the effect, or a
property of the handler declaration; and how a program that legitimately mixes
domains (a host file copied into a mem fake) says so.
