# Salvo examples

Worked examples of the language as it is *today*. Each one is a program that
compiles and runs on both backends, checked in together with the code the
compiler generated for it and the output it prints.

| example | what it shows |
|---|---|
| [`iteration/`](iteration/) | every form of iteration: native container loops, a hand-written iterator struct, `iter fn`s, a source, a step under another name, combinators of your own, and the sequence functions |
| [`effects/`](effects/) | several effects at once: handler state, a handler that depends on another effect, interception (a handler wrapping the effect it implements), shadowing, one member name on two effects, and two instances of one generic effect |
| [`throw-and-release/`](throw-and-release/) | `throw`/`try` and the `Ok T \| Thrown M` outcome, with a linear resource released on every path — including before a call that may throw |
| [`qualifiers/`](qualifiers/) | where a qualifier claim comes from, what survives a call, and state versus provenance |
| [`borrowing/`](borrowing/) | borrowing with no references: read projections and views, mutable element handles, two handles proven apart (`NotEq`) or declared aliasable (`canbe`), field-granular mutation — and the four shapes rustc refuses |
| [`collections/`](collections/) | the four collections: the literals, insertion versus key order, what may be a key, equality versus ordering, the generated constructors, and the `NonEmpty`/`Sorted`/`Distinct` claims a list can carry |
| [`linearity/`](linearity/) | values the compiler will not let you forget: where the obligation comes from, that it moves, that a keeping call borrows instead, and how a generic opts in (`canbe linear`, `once`) |
| [`actors/`](actors/) | actors: an `actor effect` and `send fn`, a linear reply token, one actor answering through another (`replyto` parking a continuation), a queue of obligations drained on shutdown, death and `watch` — and the same handler bound synchronously with `use`, which is not an actor at all |
| [`files/`](files/) | the filesystem: one `Fs` effect for paths and streams, linear tokens and a linear error, `Bytes` and text off one stream, the fill-a-buffer reads and the copy one-shots — and one program run against the disk, a sandbox and an in-memory double |
| [`time/`](time/) | time: spans and the two timelines, reading a clock as a capability, time-as-data as the posture, a deadline as a message, and virtual time in a test — `ManualTime`'s two faces, and a test clock the timer itself backs |
| [`cluster/`](cluster/) | actors across machines: two virtual nodes over the in-memory transport, actor groups found by name, a singleton behind an election (`Elected`), shards by `Key` (`Sharded`), scatter and hedge as hand-written routers `of any E`, and failover when a node leaves |
| [`aws_sqs/`](aws_sqs/) | a **generated** AWS service: `aws.sqs` from its Smithy model, every operation non-blocking through a `Reply`, the generated recording fake and a hand-written in-memory double — no SDK in the build |
| [`aws_s3/`](aws_s3/) | a **generated** AWS service with **streaming bodies**: `aws.s3`'s `PutObject` from a file and `GetObject` to a file with `pipe`, bodies as `stream.InStream` in linear structs, the generated fake and an in-memory double over `MemFs` — no SDK in the build |
| [`aws_profile/`](aws_profile/) | a **dependency**: the `aws` module from `modules/aws/`, named under `[dependencies]` and found under `[build] modules`; a dependency's modules as ordinary modules, and a module's own doc comment |

Every one of these is **checked by the test suite** (added 2026-09-16, after
`examples/effects/` was found broken for a day): each backend asserts that the
checked-in generated tree is what the compiler writes today — which also fails
if an example's source stops checking — and that the program runs to
`expected.txt`. Both pick up every directory here with a `salvo/` tree, so
a new example needs no registration.

More will be added as features land. This tree replaced `experiments/`, which
held hand-written *prototypes* of designs not yet built; the prototypes' value
was the findings they produced, and those live in COMPLETED.md.

## Layout

Every example is one directory:

```
<example>/
├── README.md      what the program is chosen to show, and what to look for
├── salvo.toml     the manifest: `src = "salvo"`, `backend = "*"`, the two targets —
│                  and, for an example using a dependency, `modules = "../../modules"`
│                  plus a `[dependencies]` table (the shared modules live in `modules/`
│                  at the repository root, so a module is written once)
├── salvo/         the Salvo source (the only hand-written code)
├── rust/          the generated Rust, exactly as `salvo compile` wrote it
├── kotlin/        the generated Kotlin, likewise
├── expected.txt   the stdout — byte-identical on both backends
└── salvo.lock     the protocol lock, where the example declares actor effects
```

The generated trees sit *beside* the sources rather than inside them: a `.rs`
or `.kt` file inside a source directory is indistinguishable from a
hand-written companion file [backend-companion], so it would be picked up by
the next build.

## Running and regenerating one

Each example is a project: its `salvo.toml` names `salvo/` as the source root,
`backend = "*"`, and `rust/`/`kotlin/` as the two targets, so from inside the
directory both trees regenerate with no flags:

```bash
cd examples/iteration
cargo run -q --manifest-path ../../Cargo.toml -- compile    # both backends, into rust/ and kotlin/
cargo run -q --manifest-path ../../Cargo.toml -- run        # runs both
```

The flag forms below still work and are what the test suite spells out.

```bash
cargo run -- run --backend rust   --src examples/iteration/salvo
cargo run -- run --backend kotlin --src examples/iteration/salvo

# after a compiler or std change, refresh the checked-in output:
cargo run -- compile --backend rust   --src examples/iteration/salvo --target examples/iteration/rust
cargo run -- compile --backend kotlin --src examples/iteration/salvo --target examples/iteration/kotlin
cargo run -- run     --backend rust   --src examples/iteration/salvo > examples/iteration/expected.txt
```

`salvo run` needs the backend's own toolchain (`rustc` / `kotlinc`) on PATH.

## Conventions for adding or updating an example

- **The Salvo source must compile and run on *both* backends, to the same
  stdout.** That equality is the point of checking the output in: a divergence
  is a [backend-parity] defect, not an example bug.
- **Use `std` rather than rolling your own.** An example that hand-writes a
  list, a result type or an iterator teaches the wrong thing and stops being
  evidence that std works. Write your own only where *that* is the subject
  (`iteration/` writes a pass by hand because the manual form is one of the
  forms it is showing).
- **Regenerate `rust/`, `kotlin/` and `expected.txt` in the same change as the
  source.** Stale generated code is worse than none: it is read as what the
  compiler does.
- **Old examples are not kept out of respect.** When a language change makes an
  example illustrate code that no longer works, rewrite it or delete it —
  there is no compatibility guarantee to document and no historical value in a
  program that does not compile. Deleting one is the expected outcome, not a
  loss; the decision log in COMPLETED.md is where history belongs.
- **Comment the source for a reader who knows some other language.** The
  comments are the explanation; the README says what to look for and why the
  shapes were chosen.
