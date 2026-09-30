# aws_sqs

Amazon SQS from Salvo, with no AWS anywhere. `aws.sqs` is **generated** from
the service's Smithy model by `modules/aws/codegen`; this program is written
against its `Sqs` effect and runs against two handlers that need no SDK.

Run it:

```bash
cd examples/aws_sqs
cargo run -q --manifest-path ../../Cargo.toml -- run
```

## What to look for

**Every operation takes a `Reply` and returns at once.** `create_queue(input,
reply)` hands the request and a continuation to whatever handles `Sqs`; the
answer arrives on the reply. Here the program waits for each (`waitfor`); a
real service would be driven the same way or with `replyto` continuations. The
host implementation (`aws.sqs.host`'s `HostSqs`) answers from the SDK's own
async machinery — coroutines on the JVM, tokio on Rust — so no Salvo worker
waits on the network.

**Errors are data.** Each answer is `Ok <Output> | Err Checked<SqsFailure>`,
where `SqsFailure = SqsError | AwsError`: `SqsError` is the service's answer —
its `code` a union of literals, one per error the model names plus `Other` for
the rest, and its HTTP `status` — and `AwsError` is a call that got no answer.
`e.code is "QueueDoesNotExist"` asks for one code, and `e.code is KmsErrorCode`
for the KMS group. `Checked` means the program must look at a failure
(`detach`, `ignore`, or narrowing to `Ok`).

**Two doubles.** `FakeSqs` is generated with the module: it records each call
by name (read back through `SqsCalls.calls()`) and answers an empty success
wherever the output allows one. `MemSqs`, written in this file, is what a test
writes when it needs real answers — the generated effect is the contract, so
any handler of it will do.

**No SDK in the build.** Nothing imports `aws.sqs.host`, so no host glue is
reached — and a dependency's host libraries (`[rust] crates`, `[kotlin] libs`
in `modules/aws/salvo.toml`) join a build only when its platform code is. The
checked-in `rust/` has no `Cargo.toml` for that reason. The same program against
the real SDKs is `modules/aws/demo/sqs_live`.
