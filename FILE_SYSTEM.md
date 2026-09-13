# The filesystem — the phase-4 option space (working document)

Status: **OPEN**. Written 2026-09-12 by a read-only session, ahead of phase 4
("The filesystem, on an IO stream design", ROADMAP.md), while phase 3
(linearity: composition and conditionality) is being built. This document
lays out the decisions phase 4 needs, in the style OBLIGATIONS.md used:
options, trade-offs, and recommendations — the calls are the user's.
**Phase 3 has since landed as decided** (O-C2 union arms, `linear struct`,
same-file dischargers, no implicit discharge sites), and OBLIGATIONS.md is
deleted per its charter — its decided outcomes live in COMPLETED.md's
decision log ("Phase 3 decided" and "Phase 3 built"); references to its
sections below read against that log.

Sources: ROADMAP.md ("S-IO — streams, then the filesystem", "The sequence"
phase 4, the two due-before decisions), COMPLETED.md (the 2026-09-06 S-IO
outline and its two findings; the interop redesign record; "std takes a pass"),
LANGUAGE.md ("Effects", "Backends"), LANGUAGE_SPEC.md ([effect-handler-deps],
[use-no-dup], [effect-member-no-effects], [effect-state-store],
[platform-effect], [linear-composite], [iter-drive-in-place]), OBLIGATIONS.md
(the decided outcomes table), and the user's stated intent (below).

## 0. The stated intent

The user's sketch (2026-09-12), restated so the decisions below can be judged
against it:

1. **The filesystem is an effect** with at least two handlers.
2. **The default handler is a platform-style handler**: a utility class
   implementation shipped in each backend — deliberately *not* requiring
   per-call-site intrinsic lowering in the compiler.
3. **The restricted handler is scoped to a root directory**, and should be
   implementable *in Salvo*, delegating to the default handler underneath
   (by default).
4. **An open file (read or write) is linear**, discharged by closing the
   stream's handle.
5. The stream design should learn from Java's `InputStream`/`OutputStream`
   heritage but consider the safer/more ergonomic successors (java.nio,
   Kotlin/Okio, Rust, and others).

Points 1, 4 and 5 fit the repo's recorded direction exactly. Points 2 and 3
each collide with a rule the effect system already has; the collisions are
FS-1 and FS-2 below, and they are the decisions that shape everything else.

## 1. Fixed points — already decided, inherited here

- **Effect members may not declare effects** [effect-member-no-effects], so a
  fallible member returns `Ok T | Err E`, never `[Throw<M>]`. The S-IO outline
  settled on `Ok InputStream | Err Str` for `open`.
- **O-C2 makes that shape legal** (phase 3, decided 2026-09-12): a written
  linear union arm is fine; the value owes while un-narrowed; narrowing to the
  `Err` arm discharges. The fallible open is unblocked — this was the thing
  S-IO was deferred on.
- **A concrete linear field requires `linear struct`** (phase 3): a pass that
  stores an open stream is written `linear struct Lines { … }` with a
  same-file discharger. The wrapper-pass shape is legal.
- **No implicit discharge sites** (phase 3): the `for`-loop close splice is
  gone. A linear pass is let-bound, driven kept, and *explicitly* closed on
  every path; inline linear-pass subjects in `for` are refused. API ergonomics
  below must assume `let h = …; for … in h { … }; close(h)`, not
  `for … in open_lines(p)`.
- **std takes a pass, never `?iter`** [seq-pass]: a stream source has no
  container behind it, so S-IO's iteration surface is passes.
- **Dischargers are same-file consuming fns** (phase 3): `close` for a file —
  and the leak diagnostic names the set.
- **Nothing linear may be live across a call that may throw** [linear-throw
  reasoning, LANGUAGE.md]: stream-holding code cannot call `[Throw]` fns.
  This constrains the error model (FS-5) more than it first appears.
- **Two decisions are due before phase 4 starts**, recorded in ROADMAP.md and
  not re-litigated here: operator typing / numeric promotion (sizes and
  offsets are `Long`), and whether two effects may share a member name. FS-3
  bears heavily on the second — one of its options mostly dissolves it.
- **Async does not exist yet** and arrives (if at all) as an explicit effect
  after phase 5. Everything here is blocking IO, identically on both backends.
- **`Byte` and `Long` exist** as intrinsic types (std/core/basic.sv); there is
  no byte-buffer surface yet, and no map type (relevant to doubles, FS-6).

## 2. What other languages teach

Surveyed for the decisions they force, not as a catalogue. The short version:
every mainstream design converged on (a) a tiny pull-based core, (b)
buffering by default or nearly so, (c) one-shot conveniences for whole-file
read/write, (d) line iteration as the flagship reading mode, and (e) scoped,
enforced closing — which is the part they all enforce *dynamically* and Salvo
can enforce statically. The restricted-root handler, meanwhile, has gone from
exotic to mainstream: it is the exact model of WASI, Deno, and Capsicum.

### Java classic (`InputStream`/`OutputStream`, `Reader`/`Writer`)

The heritage the user named, and mostly a list of what not to repeat:

- **The decorator stack** (`new BufferedReader(new InputStreamReader(new
  FileInputStream(f), UTF_8))`) is the single most-complained-about piece of
  ceremony in the JDK. It exists because buffering, decoding and the raw
  resource are separate wrapper objects. Successors (Okio, kotlinx-io,
  Python) build buffering in and expose text conveniences directly.
- **The byte/char split doubles the API** (`InputStream` vs `Reader`), and the
  default-charset trap (`FileReader` using the platform charset) was only
  fixed decades later.
- **`close()` in `finally`** was so error-prone it forced a language change
  (try-with-resources). This is the strongest external argument for point 4
  of the intent: closing is a *compiler's* job. Salvo's linearity is
  try-with-resources without the dynamic scope — the obligation travels, so a
  stream can be returned, stored (post-O-C3, in a declared container), or
  closed on a path-by-path basis, none of which `try`-with-resources allows.
- **`mark`/`reset`, `available()`, single-method `read()` returning `-1`**:
  API surface nobody's successor kept. Do not carry any of it.

### java.nio (`Path`, `Files`, channels)

- The parts people actually use are the **bulk statics**: `Files.readString`,
  `readAllLines`, `write`, `exists`, `createDirectories`. Lesson: the
  one-shot, no-stream-visible conveniences are the 90% case and should be
  first-class (FS-7).
- **`Files.lines()` returns a lazily-reading `Stream<String>` that must be
  closed**, and forgetting is a file-descriptor leak the type system never
  mentions. This is *exactly* Salvo's `Lines` linear pass, with the leak made
  a compile error. Best possible validation of the design.
- **`FileSystem`/`FileSystemProvider` SPI**: a pluggable filesystem behind an
  interface, with jimfs (in-memory) as the canonical test double. That is the
  effect/handler split, discovered independently — and jimfs is the precedent
  for a `MemFs` handler (FS-6).
- Checked `IOException` everywhere is Java's error model; Salvo's analogue is
  `Err` arms in return types (FS-5), which is the same honesty without the
  ceremony of catch-or-declare at every frame.

### Kotlin (stdlib extensions, Okio, kotlinx-io)

- Kotlin's stdlib answer to Java IO is **extension conveniences**
  (`File.readText()`, `readLines()`, `writeText()`, `forEachLine`,
  `useLines { }`) plus `Closeable.use { }` for scoping. Again: bulk
  conveniences first, streams only when needed. `useLines` is notable: the
  lines sequence is only valid *inside* the block — a dynamically-scoped
  version of what the linear pass does statically.
- **Okio / kotlinx-io** is the modern JVM design: exactly two interfaces
  (`Source`/`Sink`), one `Buffer` type, buffering intrinsic, no `mark`/`reset`,
  no byte-vs-char parallel hierarchy. Its layering — `RawSource` (the host
  resource) wrapped by a buffered `Source` (the API you use) — is precisely
  the two-layer shape FS-1's recommendation lands on.

### Rust (`std::fs`, `std::io`)

- `Read`/`Write` traits with `read(&mut buf) -> io::Result<usize>` as the
  primitive; `BufReader`/`BufWriter` as explicit wrappers; conveniences
  (`fs::read_to_string`, `fs::write`) at module level. `BufRead::lines()`
  yields `io::Result<String>` **per element** — the mid-iteration error model
  worth copying (FS-5, option b).
- **RAII close is silent about errors**: `File`'s `Drop` swallows close/flush
  failures, a known wart (write data loss reported to nobody); the workaround
  is calling `sync_all` manually. Salvo's *explicit* linear close is strictly
  better here: the close call exists in source, so it has a place to return an
  error (FS-5).
- `io::Error` with an `ErrorKind` enum + message is the structured-error
  precedent (FS-5).
- `Path`/`PathBuf`/`OsStr` model non-UTF-8 filenames honestly and everyone
  pays for it daily. Recommendation inherited: paths are `Str` in v1 (FS-7).
- **cap-std** (`cap_std::fs::Dir`: a directory handle all opens are relative
  to, `openat`-based, cannot escape) is the exact restricted-handler
  semantics, productionized — and evidence the *root as a capability value*
  reading of FS-8 is buildable.

### Go, Python, and the capability systems

- **Go**: `io.Reader`/`io.Writer` single-method interfaces; explicit
  `defer f.Close()`; `bufio.Scanner` with the *check-error-after-the-loop*
  pattern (`scanner.Err()`) — the third mid-iteration error option (FS-5,
  option c).
- **Python**: `with open(p) as f` + files iterate as lines + text/binary
  chosen at open. The ergonomic ceiling for the common case; note text/binary
  as an *open-time* mode rather than a wrapper (FS-4).
- **WASI** grants filesystem access only as **preopened directory
  capabilities** — every open is relative to a granted root. **Deno** ships
  `--allow-read=DIR` / `--allow-write=DIR` as its permission model.
  **Capsicum** (FreeBSD) is `openat` relative to held descriptors after
  `cap_enter`. The restricted handler is not a nice-to-have; it is the
  direction the industry moved for supply-chain reasons, and effects give
  Salvo a cleaner seam for it than any of these had.

## 3. The decisions

Ordered so that each builds on the previous. FS-1 and FS-2 are the
architecture; FS-3 through FS-6 are the stream design; FS-7 through FS-9 are
surface and semantics.

---

### FS-1 — Where the platform seam sits: the default handler's mechanism

The intent: the default handler is "a utility class implementation in each
backend, and shouldn't need to be an intrinsic". Three existing mechanisms
could carry it, and one deferred proposal:

**O-M1 — `intrinsic handler` backed by a static runtime file.**
`intrinsic handler HostFs of RawFs` (or `of Fs`, per FS-2), exactly like
`StdOutConsole` — bodyless in Salvo, each backend supplies the
implementation. But the supply need not be *emitted* code: both backends
already ship static runtime files (`runtime/throw.kt`, `runtime/seq.rs`),
so the implementation can be an ordinary, hand-written, target-compiler-checked
utility class (`runtime/fs.kt`, `runtime/fs.rs`) that the intrinsic handler
lowers to a reference to. This satisfies the *substance* of the intent — no
per-call-site lowering, a plain class per backend — while reusing the one
declaration form that already means "std declares, the backend supplies".

- For: zero new language surface; the `StdOutConsole` precedent; the
  target compiler checks the class against the emitted interface.
- Against: the keyword says `intrinsic` for something that is behaviorally a
  platform implementation. Cosmetic, but the user explicitly flagged it.

**O-M2 — ship `platform handler`, with std as its first customer.** The
deferred proposal (user decision 2026-09-05: "a host handler of an ordinary
Salvo effect, constructed by `use`", parked "until a need arises") — this is
the need arising. `platform handler HostFs of Fs` in std would mean: the
interface is the effect's as usual, the implementation is a class the *build*
supplies rather than the compiler emits — for std, the runtime file; for
customer code (later), a class in `platform/`.

- For: the honest name; it un-defers a proposal with a real customer and
  gives application code the same power (an app wrapping its own S3 client
  as an `Fs` handler is the obvious second customer).
- Against: new declaration form + resolution rules ("where is the class?")
  designed during a phase that is already large. The customer-code half has
  `salvo platform generate` interactions O-M1 never touches.

**O-M3 — a plain `platform effect`.** Rejected for the default: a platform
effect's instance arrives as a `salvoMain` parameter constructed by the host,
so *every* program that reads a file would need a generated `platform/` file
and a host-owned `main`. Reading a file must not cost more than printing
(`use StdOutConsole()` precedent). Platform effects stay what they are: the
seam for *application*-owned host code.

**Recommendation: O-M1 now, with the runtime-file implementation, and record
O-M2 as the follow-up rename/generalization** once a customer-code handler
appears. The difference between them is almost entirely spelling; nothing in
O-M1's shape blocks migrating the declaration to `platform handler` later.

**DECISION FS-1**: O-M1 / O-M2 / O-M3 — and if O-M1, confirm that "intrinsic
handler lowered to a shipped runtime class" is an acceptable reading of
"shouldn't need to be an intrinsic".

---

### FS-2 — How the restricted handler reaches the default

The intent: `RestrictedFs(root)` is written in Salvo and delegates to the
default handler. Two existing rules forbid the direct reading:

- **[effect-handler-deps]: a handler may not depend on the effect it
  implements** — registering it would require itself. So
  `handler RestrictedFs(root: Str, fs: Fs) of Fs` is illegal as written.
- **[use-no-dup]: registering a second handler for an effect instance already
  in scope is an error** — there is no innermost-wins shadowing for effects
  (unlike `try`). So even `use DefaultFs()` then `use RestrictedFs(root)` in a
  nested scope is refused.

Two ways through:

**O-R1 — two effects: layer the filesystem.** Split the surface into a
low-level host effect and the public one:

```
effect RawFs { … }                             // one member per host op, no policy
intrinsic handler HostFs of RawFs              // FS-1's utility class

effect Fs { … }                                // the public surface
handler DefaultFs(raw: RawFs) of Fs { … }      // thin pass-through, written in Salvo
handler RestrictedFs(root: Str, raw: RawFs) of Fs { … }   // path-checks, then delegates
```

Both public handlers *depend on* `RawFs` — a different effect than the one
they implement, so [effect-handler-deps] is satisfied, and the dependency
machinery (E1, already built: supplied from the enclosing scope, not written
at the `use` site) does the wiring. The composition root writes:

```
use HostFs()
use RestrictedFs("/data/sandbox")      // or use DefaultFs()
serve(request)                         // serve declares [Fs], not [RawFs]
```

Sandboxing is by *declaration*: a function reaches `RawFs` only by declaring
`[RawFs]` in its signature, so the unrestricted capability is auditable at
every boundary it crosses — grep for `[RawFs]` and you have the trust
audit. This is Okio's `RawSource`/`Source` layering, arrived at from the
effect side.

- For: **zero new language machinery** — every rule involved already exists
  and is tested. The raw layer is simultaneously FS-1's seam, FS-6's
  testability seam, and this decision's delegation target. RestrictedFs is
  pure Salvo (path checks are string operations; std has `split`/`join`).
- Against: `RawFs` is in scope wherever `RestrictedFs` is (a dependency must
  be registered before its dependent), so the sandbox is a *visible-capability*
  discipline, not an ambient-authority revocation — a called function that
  declares `[RawFs]` gets the unrestricted host. Defensible (capabilities you
  can see beat permissions you can't), but it must be said in the docs rather
  than discovered. Also: RestrictedFs can only restrict *the host* — it cannot
  wrap an arbitrary inner `Fs` (a `MemFs`, another `RestrictedFs`), because
  the inner thing it delegates to is nominally `RawFs`.

**O-R2 — lift the self-dependency ban: interception.** Allow a handler to
depend on the effect it implements, with "binds to the handler in scope at
`use` time" semantics, and relax [use-no-dup] to innermost-wins across
*nested* scopes (same-scope duplicates stay errors). This is classical
effect-handler nesting (Koka), and it composes: `RestrictedFs(root)` over
`MemFs`, over another `RestrictedFs`, over the host.

- For: general interception — logging handlers, retrying handlers, quota
  handlers all become writable, over any inner handler. Restricting a `MemFs`
  in a test of the restriction logic itself is genuinely useful.
- Against: a real milestone, not a rider. It reopens the acyclicity argument
  ([effect-handler-deps]'s "the availability rule *is* the acyclicity
  guarantee" — self-dependency needs an explicit binds-outward rule), touches
  both backends' handler fusion, changes [use-no-dup]'s statement, and adds
  the "which instance does this member call mean" question a second
  registration always raises. None of phase 4 *needs* it: the two handlers
  the intent names are both expressible under O-R1.

**Recommendation: O-R1 for phase 4, and record O-R2 as the shape general
interception takes** if a customer appears (the first one probably being a
logging/tracing handler, not a filesystem). The migration is additive:
layering survives interception arriving later.

**DECISION FS-2**: O-R1 / O-R2 — and under O-R1, confirm the visible
`[RawFs]`-capability reading of sandboxing is acceptable.

---

### FS-3 — Where the stream operations live

The 2026-09-06 outline recorded this as "a testability question, not a
plumbing one: only members can be faked by a double." It is also, it turns
out, the member-name-collision question in disguise.

**O-P1 — stream ops as members of `Fs`.** `read_line(s: Mut InStream)`,
`write(s: Mut OutStream, text: Str)`, `close(s: InStream)` are effect
members; the stream value is a token the handler mints and interprets.

- For: a double fakes *everything* — a `MemFs` intercepts reads. All IO
  dispatches through one seam.
- Against: (a) **it forces the shared-member-name decision immediately and
  badly** — `read`/`write`/`close` are the most collision-prone names in any
  program, and effect members collide *across effects* by rule ([mod-collision]
  reasoning; the `println@Console(...)` decision due before phase 4 exists
  because of exactly this). Any future `Net` effect wants the same names.
  (b) The stream token must be handler-mintable for doubles to work, which
  pushes toward a handle-table pattern (an id the handler maps to its own
  state) — and a handler storing *linear* values in state is a composite
  store the phase-3 rules police; the workable version is a non-linear raw
  resource in handler state with a linear token outside, which is
  handle-table bookkeeping every handler must re-implement. (c) Every
  `read_line` is a dynamic dispatch through `&mut dyn`.

**O-P2 — stream ops as free fns on the stream types.** `Fs` carries only
path-level operations (`open_read`, `open_write`, `delete`, `list_dir`, …);
once opened, the stream is a self-contained value and `read_line(s)`,
`write(s, t)`, `close(s)` are ordinary fns (intrinsic, or Salvo fns over a
raw member — per FS-1/FS-2's layering).

- For: (a) **the collision problem mostly dissolves** — free fns *overload*
  (three `size` overloads exist today), so `close(InStream)` and
  `close(OutStream)` and the existing pass `close`s coexist without any new
  syntax; the only names on `Fs` are fs-flavored ones unlikely to collide.
  This may drop the "two effects sharing a member name" decision from
  *blocking* phase 4 to merely due-eventually. (b) The stream **is** the
  capability, object-capability style (WASI/cap-std): the restricted handler
  polices only opens, because a stream you could not open is a stream you do
  not hold — `RestrictedFs` shrinks to path checks. (c) No handle table; the
  stream owns its host resource directly. (d) Reads don't pay dynamic
  dispatch.
- Against: **the double can't intercept reads** — a fake `Fs` cannot mint an
  `InStream` at all if the type is host-backed. Tests either use real temp
  files (fine for integration, heavy for unit), or the code under test takes
  its input as a pass/`Str` rather than a stream (which std's pass-only
  design already encourages), or FS-6's mitigation applies.

**O-P3 — the stream as a record of closures.** The handler mints a
`linear struct InStream` whose fields are fn-typed (`read_line: () -> …`),
capturing whatever the handler wants — host resource or test vector. Full
object-capability, any handler can mint one.

- Rejected for now: fn-typed fields are `Rc<dyn Fn…>` on Rust [rs-fn-field]
  (the phase-5 sendability question arrives early), capture-carrying closures
  escaping their scope is a known unfixed hole [fate-lambda], and effects on
  those fn fields would need threading design. The shape is right in a
  language built on closures; in this one it lands on three open wounds at
  once.

**Recommendation: O-P2, with the FS-6 mitigation for testability.** The
capability reading is the one that makes the restricted handler small, the
collision decision non-blocking, and the linearity story local to the stream
value. The testability loss is real but bounded: the one-shot conveniences
(FS-7) and line passes mean most *application* code never touches a stream
type — it takes a `Str` or a pass, both trivially fakeable — and FS-6 gives
the rest a seam.

**DECISION FS-3**: O-P1 / O-P2 / O-P3. This is the load-bearing call of the
phase: FS-5 through FS-8 below assume O-P2 and would need re-derivation under
O-P1.

---

### FS-4 — The stream types themselves

Sub-decisions, mostly independent, each with a clear modern consensus:

**(a) Two types or one.** `InStream`/`OutStream` (Java, Okio `Source`/`Sink`,
Go's split interfaces) versus one `File` opened in a mode (Python, Rust's
`File` implementing both traits). Recommendation: **two types**. Separate
types make wrong-direction use a compile error rather than a runtime one,
each gets exactly the dischargers it needs, and nothing in v1 needs
read-write files. Append is `open_append -> OutStream`, not a third type.

**(b) Buffered by default.** The decorator stack is the mistake to not
repeat; `read_line` requires lookahead anyway. Recommendation: **streams are
always buffered** (the utility class holds a `BufferedReader`/`BufReader`
internally); no unbuffered variant, no wrapper types, `flush` exists on
`OutStream` for the rare explicit need, `close` flushes. This is the
Okio/kotlinx-io/Python position.

**(c) Text-first, bytes ride along or wait.** `Byte` exists but has no
buffer surface (no `Mut List<Byte>` fill-a-buffer convention yet), and `Str`
is char-based on both backends. Options: text-only v1 (lines, whole-file,
write-string; bytes deferred until a customer), or a byte layer underneath
from day one (`read(s, buf: Mut List<Byte>) -> Ok Int | Err …`, Rust/Go
shaped). Recommendation: **text-only v1** — every recorded use case
(`open_lines("a.txt")` is the canonical example throughout ROADMAP.md) is
text; a byte surface designed without a customer will be designed twice.
The `RawFs` member set (FS-2) should still be chosen so a byte layer slots
under the same seam later.

**(d) Encoding: UTF-8, strictly, on both backends.** The JVM's default
decoder *replaces* malformed input (`CodingErrorAction.REPLACE`); Rust's
`read_to_string` *errors*. Left alone, the same malformed file would produce
different program behavior per backend — a parity break [backend-never-wrong]
adjacent. Recommendation: **strict UTF-8, malformed input is an `Err`**, and
the Kotlin utility class must configure its decoder to REPORT to match. This
needs a test with a deliberately malformed file on both backends.

**(e) `read_line` strips the terminator, `\n` and `\r\n` alike** (Rust
`BufRead::lines`, Kotlin `readLine` behavior). Write-side: `write` writes
exactly what it is given; `write_line` appends `\n` (never the platform
line ending — parity again).

**(f) No seek in v1.** Random access (`Seek`, `RandomAccessFile`) is a
different customer and drags offset arithmetic (`Long` promotion) into every
signature. The operator-typing decision still gates the phase (file *sizes*
in metadata are `Long`), but seek can wait.

**DECISION FS-4**: confirm (a)–(f), individually cheap to override.

---

### FS-5 — The error model

Three sub-questions, ordered by how much they constrain the rest:

**(a) The error payload: `Err Str` or a structured error.** Options:

- **`Err Str`** — matches `Throw<Str>` convention and every existing std
  example; zero new types. Against: callers cannot dispatch on
  not-found-vs-permission without string matching, the thing every
  errno-string API is eventually cursed for.
- **A structured error**: no enums exist, but fieldless-struct unions are
  the idiom — `type FsError = NotFound | PermissionDenied | AlreadyExists |
  NotADirectory | PathEscapes | IoError` with each arm a struct carrying
  `path: Str` (and `message: Str` on `IoError`). Rust's `ErrorKind` says this
  taxonomy stabilizes at roughly a dozen arms; the restricted handler *needs*
  a distinguishable refusal (or deliberate opacity — FS-8c) and `when` over
  the union is exactly Salvo's strength.

Recommendation: **the structured union, kept small** (the six arms above).
It is the first real error taxonomy in std and will set the pattern; `Err Str`
would be re-done within a phase. Note `to_str` on it for interpolation
[interp-to-str].

**(b) Mid-iteration read errors.** A `Lines` pass's `next` returns
`Emitted Str | Finished` — where does a disk error surface? Options:

- **(i) `next` declares `[Throw<FsError>]`** — rejected by a fixed point:
  the *pass is linear and live across the call*, and nothing linear may be
  live across a call that may throw. The shape cannot type-check, which is a
  useful early answer.
- **(ii) The element is a result**: `next` returns
  `Emitted (Ok Str | Err FsError) | Finished` — Rust's `lines()` exactly.
  Honest, but every loop body pattern-matches, and the qualifier-flattening
  work (2026-09-10) exists precisely so this nests correctly.
- **(iii) Errors end iteration; `close` reports**: `next` treats a read error
  as `Finished` after recording it in the pass; `close(lines)` returns
  `Ok None | Err FsError` (Go's `Scanner.Err()`, folded into the close every
  path already writes). The loop body stays clean; the error check sits at
  the close that linearity already forces you to write — the obligation and
  the error report land on the same line.

Recommendation: **(iii)**. It is Go's pattern made safe: Go programmers
forget `scanner.Err()`; Salvo programmers cannot forget `close`, and making
`close` the reporter means the check is un-forgettable too — *if* (c) below
gives the result teeth. (ii) is the fallback if (c) resolves weakly.

**(c) Does anything force the close result to be looked at?** `close`
returning `Ok None | Err FsError` discharges the linear obligation, but the
*result* is an ordinary droppable value — an ignored close error is silent
again, which on the write side is data loss (the Rust `Drop` wart, imported).
Options: rely on [unused-var] if the result is bound and unused (does nothing
for an expression-statement call); make the result `Once`-like must-use
(D6 landed, but `Once` is at-most-once, not at-least-once — the `[1,∞]`
"relevant" obligation is explicitly not built, OBLIGATIONS.md §5); or split
the surface — `close(s: InStream) -> None` (read side, errors at close are
noise) and `close(s: OutStream) -> Ok None | Err FsError` with the
recommendation that callers `when` it, warning-grade enforcement deferred to
the "relevant" obligation if it ever lands.

Recommendation: **the split**, with the gap stated in the docs rather than
papered over. A `[1,∞]` obligation just for this is the tail wagging the dog.

**DECISION FS-5**: (a) `Err Str` vs the `FsError` union; (b) error-in-element
vs error-at-close; (c) accept the droppable close result on the write side?

---

### FS-6 — Testability: the double story

Under FS-3's recommendation (O-P2), a fake `Fs` handler cannot mint streams,
so what *is* the test story? Three layers, cheapest first:

- **Most code shouldn't take a stream.** std is pass-only [seq-pass], and the
  conveniences (FS-7) mean application functions take `Str` or a pass —
  fakeable with a literal or `iter(list(…))`. This is the same pressure Java
  code feels toward "take a `Reader`, not a `File`", with the language
  actually enforcing the cheap version.
- **The `Fs`-member level fakes for free.** A test double
  `handler FakeFs of Fs` can implement `exists`, `delete`, `list_dir`,
  `metadata` — everything except the opens — because those return plain data.
  Restriction logic (FS-8) is testable this way… except `RestrictedFs`
  delegates to `RawFs`, whose *host* handler is the only one (`intrinsic
  handler`, FS-1). **Sub-decision: may a Salvo handler implement `RawFs`?**
  Nothing forbids it (it is an ordinary effect); recommendation: yes,
  deliberately — `RawFs` doubles as the unit-test seam for the public
  handlers, which is jimfs's role in the JVM ecosystem. That requires
  `RawHandle` (the raw stream member's payload) to be mintable by Salvo code
  — e.g. `RawFs` members trade in a plain `Long` id rather than an opaque
  host type, with the host handler owning the id→resource table *inside the
  utility class* (native maps exist there; std has none).
- **An in-memory `MemFs of Fs` in std itself** (jimfs proper) is deferred:
  it wants a map type std does not have, and its `open_read` returning a
  working `InStream` is exactly what O-P2 disallows — under the layering it
  would instead be `MemRawFs of RawFs` per the previous point, which needs
  only the id-table pattern in Salvo (assoc `List` v1).

**DECISION FS-6**: is `RawFs` officially implementable by Salvo handlers
(the id-based `RawHandle` design), or host-only (opaque type, simpler raw
members, no unit seam)?

---

### FS-7 — The v1 surface

A strawman, assuming O-R1 + O-P2 + the FS-4/FS-5 recommendations; every name
is provisional. `std/core/fs.sv` (or a new `std/io/` — first multi-module
question for std, minor).

The `Fs` effect (path-level, all fallible members returning
`Ok T | Err FsError`):

```
effect Fs {
    fn open_read(path: Str) -> Ok InStream | Err FsError => path
    fn open_write(path: Str) -> Ok OutStream | Err FsError => path      // create-or-truncate
    fn open_append(path: Str) -> Ok OutStream | Err FsError => path
    fn exists(path: Str) -> Bool => path
    fn metadata(path: Str) -> Ok FileInfo | Err FsError => path         // size: Long, is_dir: Bool
    fn list_dir(path: Str) -> Ok List<Str> | Err FsError => path        // eager; entry names, not paths
    fn create_dir(path: Str) -> Ok None | Err FsError => path           // parents too? pick one, name it clearly
    fn delete(path: Str) -> Ok None | Err FsError => path               // files and empty dirs; no recursive delete in v1
    fn rename(from: Str, to: Str) -> Ok None | Err FsError => from, to
}
```

Stream fns (free, per O-P2): `read_line(s: Mut InStream) -> Str | None`
(with FS-5(b)(iii) semantics), `read_all(s: Mut InStream) -> Ok Str | Err
FsError`, `write(s: Mut OutStream, text: Str)`, `write_line`, `flush`,
`close` (the dischargers, per side).

The pass: `linear struct Lines : Yield<self, Str>` holding the `InStream`
(legal post-phase-3: `linear struct` + same-file `close`), minted by
`fn lines(s: InStream) -> Lines` and the convenience
`fn open_lines(path: Str) [Fs] -> Ok Lines | Err FsError`.

The one-shots — written in Salvo over the members, so they are faked whenever
`Fs` is: `read_to_str(path) [Fs] -> Ok Str | Err FsError`,
`write_str(path, content) [Fs] -> Ok None | Err FsError`,
`read_lines(path) [Fs] -> Ok List<Str> | Err FsError` (eager). These are the
90% case (`Files.readString`, `File.readText`, `fs::read_to_string` — every
ecosystem grew them); no linear value ever reaches the caller.

Explicitly out of v1: seek, byte reads, recursive walk, temp files, watch,
permissions/chmod, symlink creation, stdin-as-stream (Console's, not Fs's —
the boundary should be stated when the shared-member-name decision is made).

**DECISION FS-7**: the member list above (add/cut), paths-as-`Str`,
`list_dir` eager, and where the module lives.

---

### FS-8 — Restricted handler semantics

`RestrictedFs(root: Str, raw: RawFs)`, pure Salvo. The decisions hiding in
"scoped to a root directory":

**(a) What the root check means.** Lexical resolution (normalize
`root + "/" + path`, refuse `..` escapes and absolute paths outside root) is
implementable in Salvo string code today and is v1. It is *not*
symlink-safe: a symlink inside root pointing outside escapes a lexical
check. The safe versions are `openat2(RESOLVE_BENEATH)` / `O_RESOLVE_BENEATH`
/ cap-std on the Rust side and canonicalize-then-recheck on the JVM — host
work, which under the layering belongs in the *raw* member set if wanted
(e.g. a `raw_open_read_beneath(root, rel)` member), not in Salvo.
Recommendation: **lexical in v1, documented as not symlink-safe**, with the
`RESOLVE_BENEATH`-shaped raw member recorded as the hardening path. Deciding
this consciously matters more than which is chosen: a "sandbox" whose escape
hatch is undocumented is worse than none.

**(b) Relative or rebased paths?** Does `open_read("a.txt")` under
`RestrictedFs("/data")` mean `/data/a.txt` (chroot-style rebase — WASI,
cap-std) or does the caller write full paths that are merely *checked*?
Recommendation: **rebase** — the sandboxed code is portable across roots,
which is half the point of the handler.

**(c) Refusal: distinguishable or opaque?** A blocked path can return a
dedicated `Err PathEscapes` (informative; Deno's `NotCapable`) or masquerade
as `Err NotFound` (opaque; classic capability practice, no probing the world
outside the sandbox). Recommendation: **`PathEscapes`, distinguishably** —
Salvo's restricted handler is a robustness/least-authority tool, not a
security boundary against an adversary *inside* the process (the `[RawFs]`
visibility caveat of FS-2 already concedes that), so debuggability wins.

**(d) The root's spelling.** `Str` in v1 per FS-7. The cap-std reading — the
root as an unforgeable `Dir` *value*, opens hanging off it — is the more
principled object-capability design and composes with O-P2's "the stream is
the capability"; it is also a second stream-like linear-ish type and a bigger
surface. Record as the v2 direction rather than build.

**DECISION FS-8**: (a) lexical v1 + documented caveat? (b) rebase?
(c) distinguishable refusal? (d) root as `Str`?

---

### FS-9 — Interactions to verify first (the test list)

Each of these is a place two features meet for the first time; several are
the *first exercise* of a phase-3 rule outside its own tests:

1. **O-C2 in anger**: `open_read` returning `Ok InStream | Err FsError`,
   narrowed both ways — the `Err` path closes nothing, the `Ok` path owes.
   First written linear union arm in std.
2. **`linear struct Lines` holding the stream** — first concrete linear
   field under the phase-3 marker, first same-file discharger set with a
   forward (`close(lines)` forwards into `close(stream)`).
3. **The let-bind/drive/close ergonomics** end to end, including `break` and
   early `return` paths (the all-paths analysis is what replaced the `for`
   splice; a stream is its first real workload).
4. **A linear stream live across fallible calls**: the FS-5(b)(i) refusal
   should be *observed* (a `[Throw]` call while holding a stream), confirming
   the diagnostic reads well, since users will hit it immediately.
5. **Handler-dependency wiring at two levels** (`use HostFs()` then
   `use RestrictedFs(root)`) on both backends — E1's fusion with a
   std-shipped dependency for the first time.
6. **UTF-8 strictness parity** (FS-4d): one malformed file, same `Err` on
   both backends.
7. **Effect-member deductions on fallible members**: members must declare
   deductions [effect-member-no-effects]; confirm `=> path` borrow shapes on
   every member against [effect-state-store].
8. **Whether an effect member can consume** (`close` were it ever a member,
   O-P1 worlds; also `RawFs`'s release member takes its handle) — the
   discharger definition says "same-file *fn*"; whether raw release members
   interact with the obligation at all under O-P2 (they shouldn't: the
   linear wrapper's `close` is the discharger, the raw member just gets
   called by it).

## 4. Summary of recommendations

| decision | recommendation |
|---|---|
| FS-1 seam | `intrinsic handler` over a shipped runtime class (`runtime/fs.kt`/`fs.rs`); `platform handler` recorded as the later generalization |
| FS-2 delegation | two-effect layering (`RawFs` + `Fs`), no self-dependency lift; interception (O-R2) deferred until a general customer |
| FS-3 ops location | free fns on stream types (object-capability); `Fs` members are path ops only — also mostly defuses the shared-member-name blocker |
| FS-4 streams | `InStream`/`OutStream`, always buffered, text-only v1, strict UTF-8 both backends, terminator-stripping `read_line`, no seek |
| FS-5 errors | structured `FsError` union; errors end iteration and surface at `close`; write-side `close` returns a result, droppability accepted and documented |
| FS-6 doubles | `RawFs` officially Salvo-implementable (id-based handles) as the unit seam; `MemFs` deferred (wants a map type) |
| FS-7 surface | the member list in §FS-7; paths as `Str`; eager `list_dir`; one-shot conveniences written in Salvo over the members |
| FS-8 restriction | lexical root check (documented caveat), rebased paths, distinguishable `PathEscapes`, root as `Str`; cap-std-style `Dir` value and `RESOLVE_BENEATH` raw member recorded as hardening |

The load-bearing calls are **FS-2** (layering vs interception) and **FS-3**
(where read/write live) — everything else re-derives cheaply if either flips.
Both due-before-phase-4 decisions in ROADMAP.md remain due: operator
typing/`Long` is unavoidable (metadata sizes), while the shared-member-name
decision is *softened* by FS-3's recommendation but should still be made —
`Fs` and `Console` are about to coexist in every program that logs while it
reads.
