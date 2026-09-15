# files

Reading and writing files: one `Fs` effect for paths *and* streams, linear
stream tokens, a linear error, text and bytes off the same stream — and the
same program run against three filesystems.

Run it:

```bash
cargo run -- run --backend rust   --src examples/files/salvo
cargo run -- run --backend kotlin --src examples/files/salvo
```

It works inside `tmp/files-example/`, relative to wherever you run it, and
removes everything it made.

## What to look for

**The output prints twice, identically.** The first block is the real
filesystem; the second is `MemFs`, an in-memory fake with no host handler
anywhere. Nothing in `workflow()` changes between them — it declares `[Fs]`,
and `use` decides what that means. That is what putting the stream operations
*on the effect* buys: a double can fake reading and writing, not just opening,
so a test of file code needs no files. The fake stores bytes and counts byte
offsets, so `position` answers 17 in both blocks; a fake that stored text and
counted characters would agree here and disagree on the first non-ASCII file.

**The sandbox wraps code that was not written for it.** `RestrictedFs(root)`
*intercepts* `Fs` — it depends on the effect it implements, so it binds to the
handler already registered and every relative path underneath is rebased onto
its root. `workflow()` never learns it is confined. Containment is lexical and
resolved right to left, so `sub/../probe.txt` stays inside while
`../secret.txt` is refused as `PathEscapes`, and an absolute path is refused
rather than rebased. It is a least-authority tool for honest code, not a
boundary against an adversary in the process: the refusal is distinguishable
on purpose.

**Two obligations, both checked.** An open stream is a linear token: it has to
be closed on every path, and `close` is the discharger — which for the `Lines`
pass means closing the *pass* closes the file, because `lines(s)` moved the
obligation into it. An `Err FsError` is linear too, so a result that is never
looked at does not compile. There are three ways to settle one, and the program
uses all of them: narrow to `Ok` (which is what "it was fine" costs), `ignore`
it, or `detach` its kind — the last because nothing linear may live in a
composite, so a `List<FsErrorKind>` is the only way to collect failures.

**Errors surface where they can be handled once.** `write` and `write_line`
answer a byte count and nothing else; a failed write is recorded and reported
by `flush`/`close`. `read_line` reports the end of the stream whether the file
ended or a read failed, and `close` says which. So a loop over lines narrows
no result per line, and one failure is reported in one place.

**Bytes and text are one stream.** `write_bytes`/`read_bytes` trade in
`Bytes` — std's byte buffer, not a list of octets — and every count and offset
is bytes, so the `read_all` after a three-byte `read_bytes` picks up exactly
where it stopped. `open_read_at(path, 5)` lands *between* the bytes of a
character, which is a legal seek: the strict UTF-8 decode afterwards is what
fails, and because the failure is recorded, `close` reports it a second time.

**Three ways to read, and the difference is who owns the buffer.** Steps 5–9
walk them. `read_bytes(s, n)` answers a fresh buffer, which is what you want
until it is in a loop. `read_to(s, buf, n)` *appends* into a buffer of yours,
so a loop reuses one — and because it appends rather than overwriting, `size`
is always the data rather than a count you have to carry separately;
`read_line_to(s, builder)` is the same trade for text, where the win is a line
per iteration with no new string. `chunks(s, n)` hands the loop to a pass, one
fresh buffer per step (a pass that recycled its own would overwrite what you
are still holding). And `copy_file`/`read_to_bytes` keep the buffer entirely
inside std, which is where it belongs for the common case.

**Offsets, not seeks.** Streams are forward-only. `position` reports where a
stream is, `write` answers how far it moved, and `open_read_at` reopens at a
remembered offset — which is how the program re-reads the record it just
appended.

## The layering, bottom to top

| layer | what it is |
|---|---|
| `HostRawFs` | a `platform handler`: the host class shipped per backend (`std/platform/core/hostfs.{kt,rs}`), plain `Long` handles, no obligations in it |
| `DefaultFs [RawFs]` | ordinary Salvo: mints the linear tokens, maps host failures into `FsError`, discharges in Salvo |
| `RestrictedFs(root) [Fs]` | ordinary Salvo, and an interceptor: policy in the opens, pass-throughs for the streams |
| `MemFs` | ordinary Salvo, no dependency, no host: the whole surface faked in memory |

Only the first is target-specific, and it is the only file in the stack a
backend had to be told about.
