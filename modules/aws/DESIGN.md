# The `aws` module — design

How AWS services become Salvo: generated from the public Smithy models, wrapping
the host SDKs each backend already has, non-blocking, with streaming bodies as
ordinary Salvo streams. Decided with the user on 2026-09-29 (the decision log in
the repository's COMPLETED.md has the entry); nothing below is built yet except
`ProfileCredentials`.

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
| D6 | Enums | union of unit tags (`when` exhaustively) | a qualifier-carrying `Str` |
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
| 16 | Enum tag names | namespaced under the enum's type (`StorageClass.Standard`) — needs **dot-names under a `type`**, a small language extension, over prefixed names |
| 17 | Open enums | every enum gets an `Unknown Str` arm, as both SDKs do |
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
│   ├── aws/s3.sv              # generated
│   └── platform/aws/          # generated host glue [platform-tree]
│       ├── sqs.kt  sqs.rs
│       └── s3.kt   s3.rs
└── tests/                     # Salvo: a fake per service, over MemByteSource
```

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
| enum / intEnum | union of unit tags (D6), namespaced under the enum's type — `StorageClass.Standard` (16) — plus an `Unknown Str` arm, since Smithy enums are open (17) | `@enumValue` kept in the docs |
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
    fn send_message(input: SendMessageInput, reply: Reply<Ok SendMessageOutput | Err SqsError>) -> None
        => !input, !reply
    fn receive_message(input: ReceiveMessageInput, reply: Reply<Ok ReceiveMessageOutput | Err SqsError>) -> None
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

## 5. Streaming: `std.stream` and `ByteSource`

S3 bodies are `@streaming blob` in both directions, and the host SDKs expose
them as asynchronous byte streams (`ByteStream` / `SdkBody`). The general piece
is a std module, with nothing AWS in it.

### The stream type is `InStream`

`InStream` (and `OutStream`) **move from `fs` to `std.stream`**; `fs` imports
them. Both a file and a network body are forward-only byte inputs with a linear
`close`; the differences — blocking versus asynchronous reading, `FsError`
versus `StreamError` — belong to the *effect reading them*, not to the stream.
One type means no adapter: a file opened by `Fs.open_read` can be read
asynchronously by `ByteSource.read`, provided each host keeps **one stream
table** behind `HostRawFs` and `HostByteSource`. Handle provenance was already a
runtime matter (a `MemFs` stream handed to `HostRawFs` fails at runtime today);
reusing the type adds no new hole and `stream`'s docs state the existing one. A
handle from the wrong provider **traps** with a message (10): it is a program
bug, not a condition to handle. The host pair shares handles through the table;
the mem pair (`MemFs`, `MemByteSource`) does not — handler state is not global,
deliberately — so a test may `use` both as long as it does not cross them, and
`MemHost of Fs, ByteSource` is the combined handler for when it must (13). That
asymmetry is the case for making the mismatch a *compile-time* error, recorded
in §9.

### One effect reads every stream

```
// std.stream
export noremote linear struct InStream { handle: Long }

// One read's answer. `Chunk` returns the stream for the next read; `End` and
// `Err` mean the provider has closed it — nothing left to close, nothing leaked.
export linear struct Chunk { bytes: Bytes, stream: InStream }     // `bytes` is never empty (8)
export type Read = Ok Chunk | End | Err Checked<StreamError>      // linear like `FsError` (9)

export effect ByteSource {
    // Non-blocking: hands the stream and the continuation to the provider and
    // returns. The stream travels with the request and comes back inside the
    // answer, so exactly one read is ever in flight and nobody can `close` a
    // stream mid-read — the checker cannot see a pending reply, so the token's
    // absence is what makes the rule hold.
    fn read(stream: InStream, reply: Reply<Read>) -> None => !stream, !reply
    // Gives a stream up before its end.
    fn close(stream: InStream) -> None => !stream
    // A stream over a buffer already in hand (D5: the mint lives here).
    fn from_bytes(bytes: Bytes) -> InStream => !bytes
}

export threadsafe platform handler HostByteSource() of ByteSource
export handler MemByteSource() of ByteSource      // pure Salvo; `read` answers at once
```

Reading is the parked-continuation idiom, and the linear token makes the loop
shape mandatory rather than conventional:

```
send fn on_chunk(r: Read, out: Mut Bytes) [ByteSource] {
    when r {
        is Ok  { append(out, r.bytes); read(r.stream, replyto on_chunk(out)) }
        is End { done(out) }
        is Err { report(r) }
    }
}
```

A `GetObjectOutput` holding an `InStream` is linear by the container rule, so a
caller that forgets a body gets a compile error — the property the actor-per-
body shape could not enforce. `Reply<T>` must admit a linear `T` (`Chunk`);
`std/core/actor.sv` declares `Reply<T>` without `canbe linear` today, and that
is one of the std changes in §7.

### Producers

Consumers always go through the effect. Producers come from two sides:

- **Host producers** register an asynchronous byte producer in the runtime's
  stream table and get a handle: the S3 glue for a `GetObject` body, an SDK
  stream from anywhere else, and `HostRawFs` for files (the same table).
- **Salvo producers** use `from_bytes`. A fake S3 mints bodies with it; a real
  `PutObject` caller mints one from a buffer.

The asymmetry, stated: a **host consumer** (the real `PutObject`) can only pull
from a stream the *host* registered, so its body comes from
`HostByteSource.from_bytes` or from a host-registered file, not from a Salvo
producer that generates chunks over time. That wants a writer pair
(`from_writer() -> (InStream, OutStream)` pushing into a host queue) and is
deferred until something needs it. Also deferred: asynchronous line reading
(`Lines` stays `Fs`-only).

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
3. **`std.stream`.** `InStream`/`OutStream` moved from `fs` (the sweep:
   `std/fs*`, `examples/files/`, the fs tests, the [fs-…] rules), `ByteSource`,
   `Chunk`/`Read`/`StreamError`, `HostByteSource` with a runtime stream table
   shared with `HostRawFs`, `MemByteSource`; `Reply<T canbe linear>`.

## 8. Sequence

1. §7.1 and §7.2 in the compiler, each with std/test coverage.
2. §7.3.
3. `codegen/` as a `smithy-build` plugin: shapes, unions, enums, errors, the
   effect and handler declaration, the two glues; `document` and event streams
   refused loudly.
4. SQS end to end: generated module, a fake in `tests/`, an example that sends
   and receives one message against the fake (the real handler needs
   credentials and runs by hand).
5. S3 `GetObject`/`PutObject` over `ByteSource`.

## 9. Deferred, recorded

`Document`; event streams; paginators and waiters (ordinary Salvo over the
operations, later); the writer pair for Salvo-produced bodies and host-minted
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
