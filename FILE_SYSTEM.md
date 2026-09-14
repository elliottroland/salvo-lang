# The filesystem — the phase-4 option space (working document)

Status: **DECIDED** (2026-09-14, six rounds of user decisions — see §5;
handler-dependency syntax updated in place the same day, when the surface
became an effect list on the declaration — `handler DefaultFs [RawFs] of Fs`
— rather than constructor parameters of effect type;
§5.9 has the full decided list, the agreed implementation sequence, and
the propagation owed to ROADMAP.md/COMPLETED.md/the specs once the
session's read-only restriction lifts). §§1–4 are kept as written for the
argument trail, with pointers where later rounds superseded them.

Written 2026-09-12 by a read-only session, ahead of phase 4
("The filesystem, on an IO stream design", ROADMAP.md), while phase 3
(linearity: composition and conditionality) is being built. This document
lays out the decisions phase 4 needs, in the style OBLIGATIONS.md used:
options, trade-offs, and recommendations — the calls are the user's.
**Phase 3 has since landed as decided** (O-C2 union arms, `linear struct`,
same-file dischargers, no implicit discharge sites), and OBLIGATIONS.md is
deleted per its charter — its decided outcomes live in COMPLETED.md's
decision log ("Phase 3 decided" and "Phase 3 built"); references to its
sections below read against that log. **Collections (S-Col) were decided
*and built* 2026-09-12/13, before phase 4** (see the decision log) — FS-6's
"std has no map" caveats are resolved outright; that section is updated in
place.

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
  no byte-buffer surface yet. **Update 2026-09-12: collections are built**
  (`Map<K, V>` with insertion order, `canbe hashed` keys — see COMPLETED.md)
  — the "std has no map" caveats in FS-6 are resolved, and its
  recommendations simplify accordingly.

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
*Decided 2026-09-14: O-M2 — ship `platform handler`, std as its first
customer. If it grows, it becomes the first implementation session,
decoupled from the rest of phase 4; the concrete `HostRawFs`/`HostFs` use
case plus the plan for what sits on top is the forcing function for
designing platform behavior properly.*
**Built 2026-09-14** ([platform-handler]), and the "where is the class?"
question O-M2 was charged with answering came out as *the same place a
platform effect's is*: a companion in the `platform/` tree of the module
that declared the handler, with std shipping its own under
`std/platform/…` — so the runtime-file idea O-M1 rested on is what std
does, spelled as a companion rather than as a backend registry. The
restrictions that fell out: bodyless, no effect dependencies (host code
performs no Salvo effect — put a Salvo handler in between, which is what
`DefaultFs [RawFs] of Fs` is), non-generic.

---

### FS-2 — How the restricted handler reaches the default

The intent: `RestrictedFs(root)` is written in Salvo and delegates to the
default handler. Two existing rules forbid the direct reading:

- **[effect-handler-deps]: a handler may not depend on the effect it
  implements** — registering it would require itself. So
  `handler RestrictedFs(root: Str) [Fs] of Fs` is illegal as written.
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
handler DefaultFs [RawFs] of Fs { … }          // thin pass-through, written in Salvo
handler RestrictedFs(root: Str) [RawFs] of Fs { … }   // path-checks, then delegates
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
*Revised by the user 2026-09-14 — direction is O-R2; see §5.1.*

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
O-P1. *Decided 2026-09-14 (morning): O-P2, conditional on a MemFs story.
**Revised 2026-09-14 (late morning): O-P1** — seeing the O-T1 worked example
(§5.5.1), the user rejected the `[Fs, RawFs]` double declaration in
application signatures; stream ops are effect members after all. The full
revised architecture and its consequence list is §5.7.*

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
*Partially decided 2026-09-14: (b) buffering accepted with a byte-offset
proviso that reopens (c)/(f) — see §5.3. Fifth round: (a) confirmed —
two stream types, `InStream`/`OutStream`; (d) confirmed — strict UTF-8
on both backends; (e) confirmed — `read_line` strips the terminator
(`\n` and `\r\n`), `write_line` appends exactly `\n`. (c) is resolved by
§5.3.1 (per-operation, bytes in v1); (f) by §5.3 (no seek; `open_read_at`
instead).*

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
*Decided 2026-09-14: (a) the `FsError` union, (b) errors-at-close — both
"start with these and see if they bite". (c) is superseded: the user wants
`FsError` itself to carry an obligation, which gives the close result teeth
directly — options in §5.4.*

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
  utility class*.
- **An in-memory `MemFs`-equivalent is now writable in Salvo**: with
  collections built (S-Col, 2026-09-12 — see COMPLETED.md), a
  `MemRawFs of RawFs` keeps its id table as `Mut Map<Long, …>` in handler
  state, per the previous point (an `open_read` returning a working
  `InStream` remains exactly what O-P2 disallows, so the double lives at
  the `RawFs` layer, not at `Fs`). What was "deferred for want of a map"
  is now just work scheduled behind S-Col.

**DECISION FS-6**: is `RawFs` officially implementable by Salvo handlers
(the id-based `RawHandle` design), or host-only (opaque type, simpler raw
members, no unit seam)? *Reframed 2026-09-14: the user wants a full MemFs
(no real files in unit tests, streams included) — the option set moved to
§5.5, where it interacts with the FS-2 revision.*

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

`RestrictedFs(root: Str) [RawFs] of Fs`, pure Salvo. The decisions hiding in
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
*Decided 2026-09-14 (fifth round): (a) lexical, documented as not
symlink-safe; (b) rebase — "the caller shouldn't know where they are
running"; (c) distinguishable (`PathEscapes`); (d) root as `Str`, marked
to revisit (the cap-std `Dir`-value reading stays the recorded v2
direction). (b) raised a new sub-question — should `RestrictedFs` be its
own effect *type*? — analyzed in §5.10.1, recommendation: no.*

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
| FS-1 seam | *(superseded by §5's decision: O-M2 `platform handler`, built 2026-09-14)* `intrinsic handler` over a shipped runtime class (`runtime/fs.kt`/`fs.rs`); `platform handler` recorded as the later generalization |
| FS-2 delegation | two-effect layering (`RawFs` + `Fs`), no self-dependency lift; interception (O-R2) deferred until a general customer |
| FS-3 ops location | free fns on stream types (object-capability); `Fs` members are path ops only — also mostly defuses the shared-member-name blocker |
| FS-4 streams | `InStream`/`OutStream`, always buffered, text-only v1, strict UTF-8 both backends, terminator-stripping `read_line`, no seek |
| FS-5 errors | structured `FsError` union; errors end iteration and surface at `close`; write-side `close` returns a result, droppability accepted and documented |
| FS-6 doubles | `RawFs` officially Salvo-implementable (id-based handles) as the unit seam; `MemRawFs` writable once S-Col lands (`Mut Map<Long, …>` handler state) |
| FS-7 surface | the member list in §FS-7; paths as `Str`; eager `list_dir`; one-shot conveniences written in Salvo over the members |
| FS-8 restriction | lexical root check (documented caveat), rebased paths, distinguishable `PathEscapes`, root as `Str`; cap-std-style `Dir` value and `RESOLVE_BENEATH` raw member recorded as hardening |

The load-bearing calls are **FS-2** (layering vs interception) and **FS-3**
(where read/write live) — everything else re-derives cheaply if either flips.
Both due-before-phase-4 decisions in ROADMAP.md remain due: operator
typing/`Long` is unavoidable (metadata sizes), while the shared-member-name
decision is *softened* by FS-3's recommendation but should still be made —
`Fs` and `Console` are about to coexist in every program that logs while it
reads.

## 5. The 2026-09-14 responses — what landed, what opened

The user read §§1–4 and responded. Settled outright:

- **FS-3 = O-P2** (free fns on stream types), conditional on a real MemFs
  story (§5.5) — "tests should be fast, and disk IO can slow that down";
  real temp files in unit tests are not an acceptable answer.
- **FS-4(b) buffering accepted** — with the byte-offset proviso of §5.3.
- **FS-5(a) = the structured `FsError` union** and **FS-5(b) = (iii),
  errors end iteration and surface at `close`** — both explicitly "start
  with those and see if they bite".
- **FS-5(c) superseded**: rather than accept a droppable close result, the
  user wants `FsError` itself to be linear (or otherwise obligated), with
  discharge by an explicit `ignore()` or by narrowing the result to the
  non-error arm. Options in §5.4 — this was an explicit request for design
  options, not a decision.

Changed direction:

- **FS-2 → O-R2**: the user likes `RestrictedFs` *implementing `Fs` while
  depending on an(other) `Fs`* — "this is the point at which we add the
  ability for effects to overwrite and use one another" — i.e. interception
  is wanted as a language feature now, not deferred. The acyclicity concern
  is to be answered by the rule sketched in §5.1.

Not yet addressed (still open, unchanged from §3): **FS-7** (the member
list, module location), **FS-8** (restriction semantics a–d), the remaining
FS-4 sub-points (a, d, e — (c) and (f) are reopened by §5.3), and both
ROADMAP.md due-before decisions — operator typing/`Long` is now *more*
load-bearing (§5.3 puts `Long` arithmetic in user-facing position math),
and the shared-member-name decision stays softened-but-due under O-P2.
(**FS-1 was decided in the second round** — O-M2, see its decision block;
§5.9 carries the authoritative open list, §5.7–5.8 the architecture as it
now stands.)

### 5.1 Interception (O-R2) and the construct-at-use acyclicity rule

The user's proposed cycle-prevention rule: *"whenever someone `use`s a
handler, the handler had to be created in the function it was used."*

**Finding: the first half is already a theorem.** [handler-not-value] says a
handler instance is *only* produced by `use` — a handler constructor call in
any other position is an error today. So "created where used" holds by
construction; nothing can pass a pre-built handler around. The substantive
content of the proposal is therefore the **binding rule** for the new
self-dependency:

> A handler's dependency on the effect it implements binds to the instance
> in scope **strictly before this `use`** (the outward binding). All other
> dependencies keep their existing rule (available at registration, from
> the enclosing scope).

Under that rule the acyclicity argument survives in the same form
[effect-handler-deps] uses today: every dependency edge points to a `use`
that textually/dynamically precedes this one, the precedes relation is
well-founded, so no cycle can be constructed in any order. Mutual dependence
dies at the first `use` with the existing "register one before it" error
(`use A(…)` needing `B` before any `B` exists). **The rule is sufficient** —
what it needs is to be *stated* as the binds-outward clause, not a new
creation restriction.

What O-R2 actually costs (unchanged from §FS-2, now due rather than
deferred):

1. **[effect-handler-deps]'s self-ban is replaced** by the binds-outward
   clause above. A self-dependency `use` with no instance already in scope
   is the existing "register one before it" error.
2. **[use-no-dup] is relaxed to innermost-wins across nested scopes** —
   same-scope duplicates stay errors. This is the bigger change: it touches
   [effect-disambiguation] ("which instance does this member call mean" —
   answer: the innermost, per ordinary scoping), the checker's `effect_env`
   truncation, and both backends' fusion emission ([rs-effect-fusion],
   [kt-effect-fusion] — Rust's per-scope fusion must now hand the *outer*
   instance to the shadowing handler's members from a disjoint borrow).
3. **Sub-decision (decided 2026-09-14): shadowing is allowed**, including
   later-in-block over earlier-in-block (`use HostFs(); use
   RestrictedFs(root)` in one statement list is legal). The retained error
   is *registering the same handler instance twice*. Note for the spec
   text: under [handler-not-value] a handler instance exists only at its
   `use` site, so "same instance twice" cannot currently be written — the
   rule is therefore *stated* as: a `use` may shadow any earlier
   registration for the effect instance, and the duplicate-instance error
   is the future-proofing clause should handler values ever become
   nameable. What replaces [use-no-dup]'s check in practice is the
   binds-outward rule (a self-dependency binds strictly before its own
   `use`).
4. **Sub-decision (resolved 2026-09-14, second round): the `RawFs` layer
   is dead.** With FS-3 flipped to O-P1 (§5.7), there is **one effect,
   `Fs`, all the way down**: `DefaultFs` is FS-1's `platform handler` at
   the bottom; `RestrictedFs(root: Str) [Fs] of Fs` and `MemFs of Fs` are
   written in Salvo. The audit story is the handler stack at the
   composition root, not a grep — accepted with the flip.
5. **Scheduling (resolved with 4)**: interception is **load-bearing for
   phase 4** — `RestrictedFs of Fs` depending on `Fs` requires the
   binds-outward rule and shadowing, so O-R2 ships in (or before) the
   filesystem phase. ***Built 2026-09-14***, *ahead of the filesystem: the
   binds-outward clause as stated above, shadowing with the
   duplicate-instance clause retained as future-proofing, innermost-wins
   lookups through the checker and both emitters, and one shared
   compile-and-run demo (stacked interceptors, a stateful interceptor,
   block-scoped expiry, one instance of a generic effect intercepted) to
   identical stdout on both backends. The as-built record is COMPLETED.md's
   decision log ("Effect interception"); the rules are LANGUAGE_SPEC.md
   [effect-intercept] and the restated [use-no-dup].*

### 5.2 FS-3 under the MemFs requirement

O-P2 stands, but its known cost — "the double can't intercept reads" — is
now explicitly not acceptable at the "use real files" fallback level. The
resolution is §5.5; nothing else in FS-3 changes. If §5.5's options all
prove too expensive, the honest fallback is re-opening FS-3 toward O-P1
(members), which was rejected for the collision and handle-table costs —
flagged so the trade is made consciously, not by drift.
*This is what happened (2026-09-14, second round): the user judged the
`[Fs, RawFs]` double declaration in application signatures worse than
O-P1's costs and took the fallback consciously — see §5.7.*

### 5.3 Byte offsets under buffering (reopens FS-4(c)/(f))

The user's requirement: *while writing or reading, the program must be able
to compute **exact byte offsets**, usable later for targeted (byte-range)
reads.* The record-store / index-file shape. Buffering must either not get
in the way, or an opt-out must exist even if it costs the user overhead.

**Finding: buffering is not actually in the way — opacity is.** A buffered
writer delays bytes but does not change *where* they land: the logical
position (bytes accepted so far) is exactly knowable if the stream tracks
it. Same on the read side: the position of the *consumed* point is exact if
the buffered reader counts consumed bytes. The hazards are
implementation-level, and both are ours to get right in the runtime
classes: Kotlin's `BufferedReader`/`Writer` count **chars, not bytes**, so
the JVM runtime class must count encoded UTF-8 bytes itself (or buffer
below the decoder); Rust's `BufReader` makes consumed-byte counting easy.
An *unbuffered mode* as the opt-out would be the wrong tool — it does not
by itself reveal position, and position tracking works fine under
buffering — so the recommendation is to satisfy the requirement directly
and skip the opt-out.

Options for the surface (combinable):

- **O-B1 — position + seek**: `position(s) -> Long` on both stream types
  (logical, buffer-aware, byte-based) and `seek(s: Mut InStream, pos: Long)`
  (drops the read buffer, refills). Reverses FS-4(f) ("no seek in v1").
  The classic shape; smallest concept count for the user's use case
  (write, note `position()`, later seek and read).
- **O-B2 — ranged open, no seek**: `open_read_at(path, offset: Long)
  [Fs] -> Ok InStream | Err FsError` (± a `len` bound). Targeted read = a
  fresh open. Keeps streams forward-only (simpler linearity/buffering
  story, no seek-past-buffer bugs), object-capability clean; costs an
  open per record — acceptable at v1 scale, and `Fs` members are where the
  restricted handler already polices.
- **O-B3 — writes report their size**: `write`/`write_line` return the
  byte count written (`-> Long`), so offset arithmetic needs no separate
  position query on the write side. Cheap; pairs with either of the above.
  (Alternative spelling: a `byte_len(s: Str) -> Long` free fn and keep
  writes `-> None` — same information, pull instead of push.)

Recommendation: **O-B2 + O-B3 for v1, O-B1 recorded as the follow-up** if
re-opening per record proves too slow — ranged open keeps every stream
forward-only, which §5.4's linearity story and the buffering design both
appreciate, and it needs no change to the pass/`read_line` machinery.
*Decided 2026-09-14: O-B2 (`open_read_at`). O-B3 (writes return byte
counts) not yet addressed — folded into the Str/Byte sub-design below.*
Either way two consequences are fixed: (i) the **operator-typing/`Long`
decision is now unavoidable and user-facing** — offset arithmetic is the
user's code, not just metadata; (ii) **mid-codepoint offsets need a stated
policy** — an offset computed by the program lands on a boundary by
construction, but a wrong one lands mid-codepoint and strict UTF-8 (FS-4d)
turns it into `Err` — which is the correct behavior, and should be tested,
not discovered. FS-4(c) "text-only v1" survives *if* ranged reads return
`Str` under that policy; if byte-exact payloads are wanted (`read_range ->
List<Byte>`-shaped), the byte surface arrives early and FS-4(c) flips.
**Sub-decision: ranged reads return `Str` (strict UTF-8) or bytes?**

#### 5.3.1 Choosing between `Str` and `Byte` reads (opened 2026-09-14)

The user asked where the text/binary choice lives — per stream or per
operation — and what the backends permit. The backend facts first, since
they decide more than taste does:

- **The JVM's built-in split is a trap, and we are not bound by it.**
  `InputStream` (bytes) vs `Reader` (chars) cannot share a stream safely
  because `Reader` buffers and decodes ahead — byte positions are lost the
  moment a `Reader` wraps the stream. But FS-4(d) (strict UTF-8, REPORT)
  and §5.3 (byte-exact positions) *already* force the Kotlin runtime class
  to own a **byte-level buffer and do its own incremental UTF-8 decode**
  — `BufferedReader` counts chars, not bytes, so it was never usable
  as-is. Once the buffer is ours and byte-level, text and byte reads from
  the same stream are both just views over it. This is exactly Okio's
  architecture (`Buffer` is bytes; `readUtf8LineStrict()` decodes from
  it), and Rust's (`BufReader` is bytes; string reads decode).
- **`Byte` has a live parity hazard**: it lowers to Rust `u8` (0..255)
  and Kotlin `Byte` (**signed**, -128..127) today
  ([rs] `Byte`→`u8`; [kt] `Byte` keeps its name). The moment a byte value
  is user-visible (compared, interpolated, used in arithmetic), 0x80..0xFF
  diverge between backends — a [backend-never-wrong]-adjacent break.
  Options: Kotlin lowers to `UByte` (stable since 1.5; boxing and
  `toString` behave; arithmetic returns `UInt`, needs care in the
  intrinsics), or keep `Byte` and mask (`b.toInt() and 0xFF`) at every
  observable seam (fragile — a seam missed is a silent parity bug).
  **Sub-decision, owed before any byte-returning API ships.**
- **`List<Byte>` boxes on the JVM.** The honest lowering for a byte
  payload is `ByteArray`/`Vec<u8>`; whether `List<Byte>` gets a
  specialized backend rendering (a `kt-` rule) or byte payloads get their
  own type is part of the same sub-decision. Rust is indifferent
  (`Vec<u8>` either way).

The surface options:

- **O-S1 — stream-level (Python mode-style)**: `open_read` yields a text
  `InStream` (`read_line`, `read_all`); `open_read_bytes` yields a
  `ByteInStream` (`read_range`-shaped ops). Wrong-direction use is a
  compile error (the FS-4(a) argument), but the record-store use case —
  read a byte range, *then* parse text out of it — needs two opens or a
  conversion, and the type count doubles (four stream types with the
  write side).
- **O-S2 — per-operation on one stream**: one `InStream` per direction,
  byte-buffered underneath (which the backends force anyway, per above);
  `read_line`/`read_all` decode, `read_bytes(s, n)`/byte ops don't. The
  choice is the call, not the open. Okio's model. Costs: a stream meant
  as text-only cannot statically forbid byte reads (accepted — mixing is
  well-defined over one byte buffer, positions stay exact), and the
  free-fn surface on `InStream` grows.

Recommendation: **O-S2** — the runtime classes must be byte-buffered
custom code regardless, at which point stream-level segregation buys type
safety nobody asked for and blocks the mixed read-range-then-parse
pattern that motivated §5.3 in the first place. Text-only *v1 shipping*
can still hold: O-S2 with only the text ops shipped is indistinguishable
from FS-4(c), and byte ops land later without a new stream type.
*Decided 2026-09-14 (second round): O-S2, per-operation. Fifth round:
`Byte` lowers to **Kotlin `UByte`** (the Rust `u8` mapping stands);
**bytes ship in v1** — no text-only first cut ("if we need to sequence
multiple deliverables here, so be it"); **O-B3 accepted** — string writes
return the byte count written. Still open underneath: the byte-payload
lowering (`List<Byte>` → `UByteArray`/`Vec<u8>` rendering, or boxed v1)
— carried in the FS-7 draft (§5.10.2).*

### 5.4 A linear (or obligated) `FsError` — the option set

The intent: an `FsError` must not be silently droppable. Discharge is an
explicit `ignore(e)` call, or narrowing the surrounding result to the
non-error arm ("it was fine"). The user's question: does this compose with
`FsError` being a union — `type FsError = NotFound | … `— and should
something like `linear type FsError = … | …` exist?

**Finding: `linear type` collides with what aliases are.** [type-alias]
aliases expand *structurally* at use sites — no nominal identity — while
linearity is a property of a *declaration* ([linear-group]), deliberately
unwritable at use sites ([obligation-spelling]: a per-value spelling that
could be forgotten would defeat the protection). A linear alias would make
`FsError` and its written-out expansion the same type with different
obligations, which is exactly the incoherence [obligation-spelling] exists
to prevent. So `linear type` as *alias + modifier* is not a small feature —
it forces one of the real options below.

Also inherited before choosing: whatever carries the obligation obeys
[linear-composite] (**no `List<FsError>`, no error field in a struct** —
collecting errors to report at the end requires converting to a non-linear
record first) and the [linear-throw] fixed point (**holding an undischarged
error blocks calls to `[Throw]` fns**). And O-C2 [linear-union-arm] already
delivers half the user's wish for free: narrowing `Ok T | Err FsError` to
the **non-linear `Ok` arm discharges** — "confirming that the result was
not an error" costs nothing extra, in every option. The options differ only
in what the *error path* owes.

**O-L1 — per-arm `linear struct`s, union of linear arms.**
`linear struct NotFound { path: Str }` … six times; `type FsError = …`
stays an ordinary (structural) alias — the obligation lives on the arms,
which O-C2 supports today ("a union arm may be linear"; a union of *only*
linear arms owes until consumed).

- For: zero new language surface; `when` over the union stays the
  dispatch idiom, and *narrowing between arms never discharges* (all arms
  linear), so every branch must end in a consuming call — the obligation
  is airtight.
- Against: **six discharge sets**. [linear-group] dischargers are same-file
  consuming fns *of the type*, so honest coverage is `ignore(e: NotFound)`
  × 6 (overloads, mechanical but noisy) — plus the ergonomic question of
  whether `fn ignore(e: FsError)` (union-typed parameter, `when` + forward
  inside) counts as a discharger for each arm. If it does (needs a rule
  reading: "consumes a parameter whose type *includes* the arm"), the
  noise collapses to one fn; if not, six. **Sub-decision either way.**
  Handling ergonomics: `is NotFound { println(…); ignore(e) }` — the
  explicit consume appears in every arm even after handling, which is the
  point, but should be shown in LANGUAGE.md so it reads as intended rather
  than as boilerplate.

**O-L2 — one linear wrapper, non-linear payload.**
`linear struct FsError { kind: FsErrorKind }` with
`type FsErrorKind = NotFound | PermissionDenied | …` ordinary and
droppable. One discharge set (`ignore(e: FsError)`, plus e.g.
`fn message(e: FsError) -> Str => !e` as a consuming accessor for the
log-and-move-on path).

- For: works **entirely with today's machinery**, one type to document,
  one `ignore`. The "collect errors" escape is natural: a same-file
  `fn detach(e: FsError) -> FsErrorKind => !e` discharges and returns the
  droppable payload for storage in a `List<FsErrorKind>`.
- Against: dispatch is `when e.kind { … }` — one field hop before the
  union, and the obligation is *shallower* than O-L1: once `detach`ed (or
  after reading `.kind`… no — reading doesn't consume; only `detach`
  does), the payload is droppable. That is arguably the right depth: the
  obligation means "acknowledge me", not "carry me forever".

**O-L3 — nominal linear unions: `linear union FsError = NotFound | …`.**
The feature the user's `linear type` question is really asking for: a
*declared*, nominally-identified union that carries the modifier itself,
arms staying ordinary structs.

- For: the spelling matches the mental model exactly; arms stay reusable
  and non-linear elsewhere; one discharge set on the union declaration.
- Against: a **new kind of type declaration** — nominal unions do not
  exist ([type-alias] is explicit that unions-via-alias are structural),
  so this buys parser + resolver + checker + two backends' lowering work,
  *and* a semantic novelty: narrowing `FsError` to `NotFound` would land
  on a **non-linear type, which per O-C2 discharges** — silently
  defeating the obligation — so nominal linear unions need an amendment
  ("narrowing *within* a linear union keeps the obligation") that makes
  discharge-by-narrowing depend on where a type came from. Deep water for
  an error type; if nominal unions are ever wanted, they should be wanted
  for more than this.

**O-L4 — a weaker "must-use" obligation** (the user's "if linear is too
strong"). The `[1,∞]` "relevant" obligation was explicitly not built
(OBLIGATIONS.md §5, recorded in COMPLETED.md); a use-site-checked
must-look warning is a new analysis with its own false-positive budget.

- Against, decisively: linearity is *not* too strong here — consumption
  is a move, so "exactly once" costs the error path a single `ignore`
  call, which is precisely the explicitness the user asked for. Building
  a second, weaker obligation system to avoid one call per error path is
  the tail wagging the dog again. Recommend rejecting.

**Recommendation: O-L2**, with O-L1 as the runner-up if the per-arm
obligation depth is wanted (and then the union-typed-discharger
sub-decision made deliberately). Record O-L3 as "what `linear type` would
really mean" and why it was not built.
*Decided 2026-09-14: O-L2 — one `linear struct FsError { kind: FsErrorKind }`.*

**Why `List<FsError>` is refused — conditional linearity does not reach
`List`** (user question 2026-09-14). Conditional containers exist, but for
*declared structs only*: `struct Box<T canbe linear>` works because a
struct's fields are statically enumerable, so its obligation can **settle
by decomposition** — the checker sees each linear-carrying field moved out
and the shell then owes nothing [linear-generics]. `List` is an intrinsic
type whose element *count is dynamic*: the checker cannot know that every
element's obligation was discharged (which elements were removed? all of
them?), so settlement-by-decomposition has no static footing, and
tracked-container obligations are exactly the deferred roadmap-L8 design
question. Until L8, a call that would put a linear value into a `List<T>`
is refused at the store [linear-composite] regardless of any
`canbe linear` on the callee. Under O-L2 this stays a non-problem in
practice: `detach(e: FsError) -> FsErrorKind => !e` discharges and returns
the droppable payload, and `List<FsErrorKind>` aggregates freely — the
pattern to document.

Consequences to verify whichever lands (extends FS-9's list):

9. `close(s)` as an **expression statement is now refused** when close
   returns `Ok None | Err FsError` — the un-narrowed union owes
   [linear-union-arm]/[linear-obligation]. This is FS-5(c)'s missing
   teeth, delivered by the error type rather than by a special rule; the
   ergonomic floor is `if close(s) is Err e { … ignore(e) }` or a `when`.
   **Sub-decision reopened: should the *read*-side `close` return plain
   `None`** (FS-5(c)'s split, so line-reading loops end in one clean
   `close`), now that a returned error can no longer be silently
   dropped anywhere it appears? *Decided 2026-09-14 (fifth round): no
   split — the read side returns `Ok None | Err FsError`, same as the
   write side.*
10. An `FsError` **cannot be stored or aggregated** [linear-composite] —
    the `detach`/to-record pattern must exist and be documented, or test
    suites and retry loops will fight the rule immediately.
11. Holding an `FsError` across a `[Throw]` call is refused
    [linear-throw reasoning] — check the diagnostic reads well, since
    "log the error via a fallible logger" is a plausible early collision.
12. Effect members returning linear-armed unions: confirm
    [effect-member-no-effects] deduction spellings (`=> path`) compose
    with a linear `Err` arm on *members* (O-C2's tests were on free fns).

### 5.5 The MemFs option set (full in-memory tests, no disk IO)

*Resolved 2026-09-14 (second round): none of the below — FS-3 flipped to
O-P1, so `MemFs of Fs` fakes everything directly (stream members
included); see §5.7. The section and the §5.5.1 example are kept because
the example is what surfaced the `[Fs, RawFs]` double-declaration cost
that drove the flip, and O-T2's closed-union idea is referenced from
§5.7's stream-token design.*

The requirement: unit tests run against a complete in-memory filesystem —
opens, reads, writes, streams included — with no real files created. Three
architectures, differing in *which layer* the fake lives at; entangled
with §5.1's sub-decision 4 (does `RawFs` survive?).

**O-T1 — the raw seam (two effects, fake at `RawFs`)** — §FS-6's design,
unchanged: `RawFs` members trade in a plain id (`RawHandle` ≈ `Long`);
`InStream`/`OutStream` are `linear struct`s holding the id; the stream free
fns are Salvo fns declaring `[RawFs]` that call raw members with the id;
`MemRawFs of RawFs` keeps `Mut Map<Long, …>` in handler state (buildable —
S-Col landed). Streams then work *unmodified* over the fake: `read_line`
dispatches through whatever `RawFs` is in scope.

- For: everything above `RawFs` — the public handlers, the passes, the
  one-shots, the streams themselves — is exercised for real in tests;
  needs **no interception** (works under O-R1 as-is; `RestrictedFs` over
  `MemRawFs` already composes because the restricted handler's dependency
  is nominally `RawFs`).
- Against: keeps the two-effect architecture §5.1 was leaning away from;
  stream fns carry `[RawFs]` in their signatures (visible everywhere
  streams are touched); and the **id/handler mismatch hazard**: a stream
  minted under `MemRawFs` but read under `HostFs` looks up a meaningless
  id. Mitigation to design: per-handler id namespaces so a foreign id is
  a clean `Err`, not garbage.

#### 5.5.1 O-T1 worked example (added 2026-09-14, on request)

How the raw seam makes an in-memory filesystem possible: the streams are
*dumb tickets* — a `linear struct` holding nothing but an id — and every
stream operation is a Salvo free fn that hands the id to whichever `RawFs`
handler is in scope. The handler owns what the id *means* (a host file, or
an entry in a map). Swap the bottom handler and the same streams, passes,
one-shots and application code run against memory. Sketch (names
provisional, `FsError` per O-L2):

```
// std: the raw seam — members trade in ids, no stream types here
effect RawFs {
    fn raw_open_read(path: Str) -> Ok Long | Err FsErrorKind => path
    fn raw_read_line(handle: Long) -> Str | None
    fn raw_close_read(handle: Long) -> Ok None | Err FsErrorKind
    // … raw_open_read_at(path, offset), the write side, exists, …
}

// std: the public stream — a linear ticket around the id
linear struct InStream { handle: Long }

fn read_line(s: Mut InStream) [RawFs] -> Str | None {
    return raw_read_line(s.handle)
}
fn close(s: InStream) [RawFs] {          // the discharger [linear-group]
    raw_close_read(s.handle)             // (error handling per FS-5(b))
    discard(s)                           // legal only here [linear-discard]
}

// std: the public handler — path policy only, delegates down
handler DefaultFs [RawFs] of Fs {        // dependency [effect-handler-deps]
    fn open_read(path: Str) -> Ok InStream | Err FsError {
        let r = raw_open_read(path)
        when r {
            is Ok  { return ok(InStream { handle: ^r }) }
            is Err { return err(FsError { kind: ^r }) }
        }
    }
    // …
}

// std (or a test module): the in-memory bottom — pure Salvo, no disk
struct MemOpen { content: Str, at: Long }
handler MemRawFs(files: Map<Str, Str>) of RawFs {
    open:    Mut Map<Long, MemOpen> = mut_map_of()   // handler state
    next_id: Long = 0
    fn raw_open_read(path: Str) -> Ok Long | Err FsErrorKind {
        let content = get(files, path)
        if content is None { return err(NotFound { path: path }) }
        // mint an id, put(open, id, MemOpen { … }), return ok(id)
    }
    fn raw_read_line(handle: Long) -> Str | None {
        // look up open[handle], slice to next '\n', advance `at`
    }
    fn raw_close_read(handle: Long) -> Ok None | Err FsErrorKind {
        remove(open, handle)
        return ok(None)
    }
}
```

The application function is written once, against `Fs` plus the stream
fns' `[RawFs]`:

```
fn first_line(path: Str) [Fs, RawFs] -> Ok Str | Err FsError {
    let opened = open_read(path)
    if opened is Err { return opened }
    let s = ^opened                        // s: InStream, owes [linear-obligation]
    let line = read_line(s)
    close(s)                               // discharge
    when line {
        is Str  { return ok(line) }
        is None { return err(FsError { kind: UnexpectedEof { path: path } }) }
    }
}
```

And only the composition root differs between contexts:

```
fn main() [use] {
    use HostRawFs()                        // FS-1's platform handler: real files
    use DefaultFs()                        // raw supplied from scope, not written
    println(to_str(first_line("data/log.txt")))
}

fn test_first_line() [use] {
    use MemRawFs(map_of(("data/log.txt", "hello\nworld\n")))   // no disk
    use DefaultFs()                        // same public handler, for real
    // assert first_line("data/log.txt") == ok("hello")
}
```

`first_line`, `DefaultFs`, the `InStream` linearity, `read_line`, `close`
— all identical in both runs; `raw_read_line(s.handle)` dispatches to
whichever `RawFs` was registered. `RestrictedFs(root) [RawFs]` slots
into either stack the same way, so the restriction logic is also
unit-testable over memory — all without interception.

**The honest wart this example exposes (new finding): `[RawFs]` audit
dilution.** §FS-2 sold the raw layer as the trust audit — "grep `[RawFs]`
and you have the audit". But under O-P2+O-T1 the *stream fns themselves*
declare `[RawFs]`, so every function that reads a stream carries it
(`first_line` above does) — and a function with `[RawFs]` in scope can
also call `raw_open_read` directly, **bypassing `RestrictedFs`
entirely**. The sandbox stops being auditable by grep the moment stream
IO is routine. The repair, if O-T1 is chosen: **split the raw seam in
two** — `RawFs` (path-taking members: the opens, `exists`, `delete`, …)
and `RawIo` (handle-taking members only: read/write/close on ids). Stream
fns declare `[RawIo]`, which cannot open anything; `[RawFs]` appears
*only* in the public handlers, and the grep-audit story survives intact.
`MemRawFs` implements both. Costs one more effect name; changes nothing
else above.


**O-T2 — streams as closed unions (one effect, fake at `Fs`)**: under the
single-effect O-R2 architecture, `type InStream = HostInStream |
MemInStream` — `HostInStream` the opaque host-backed `linear struct`,
`MemInStream` a pure-Salvo `linear struct` (its content in its own fields).
The free fns stay free but are written in Salvo, `when`-ing over the arm
(host arm calls the intrinsic, mem arm runs Salvo code). `MemFs of Fs`
mints `MemInStream`s directly.

- For: the fake lives at the *public* effect, matching the one-effect
  architecture; no `[RawFs]` in stream signatures (the fns take the
  stream, not an effect — dispatch is data, not capability); composes
  with interception (`RestrictedFs` over `MemFs`) naturally.
- Against: the union is **closed in std** — a future third minting handler
  (an S3-backed `Fs`) cannot add an arm without editing std, which is the
  classic expression-problem corner; every read pays an arm test (cheap,
  but real); and `MemInStream`'s byte-offset support (§5.3) needs
  byte-indexable in-memory content — with no byte surface, its fields are
  `Str`+`Long` offsets and slicing intrinsics, which is a small hidden
  byte API arriving through the back door. Both linear arms are fine
  ([linear-union-arm]); `close` is an overload set or a `when`-ing
  discharger — same sub-decision as O-L1's union discharger.

**O-T3 — stream ops move onto the effect (re-open FS-3 toward O-P1)**: the
only design where a single-layer `MemFs of Fs` fakes everything with no
unions and no raw seam — and it re-imports everything O-P2 was chosen to
avoid (member-name collisions across effects, the handle table in every
handler, dynamic dispatch per read). Listed for completeness; recommend
against unless both options above fail.

**Recommendation: O-T2 if §5.1 lands on the single-effect architecture,
O-T1 if the raw layer survives.** They are the same test story at
different seams; the architecture decision (§5.1 sub-decision 4) should
pick, not testing. Either way one hazard is shared and must be tested:
a `MemFs`/`MemRawFs` file written and re-opened must round-trip byte
offsets identically to the host (§5.3's `position` arithmetic runs against
the fake in unit tests — if the fake counts chars where the host counts
bytes, tests pass and production breaks).

### 5.7 The decided architecture (2026-09-14, second round)

FS-3 flipped to **O-P1** after the §5.5.1 example made O-T1's cost
concrete: application signatures would carry `[Fs, RawFs]` (or
`[Fs, RawIo]`) everywhere a stream is read, and the user rejected the
double declaration. The architecture as now decided:

- **One effect, `Fs`**, carrying *both* the path-level members and the
  stream operations. No raw layer.
- **`DefaultFs` is the bottom**: FS-1's `platform handler`, host-backed,
  one per backend.
- **`RestrictedFs(root: Str) [Fs] of Fs` and `MemFs of Fs` are written
  in Salvo.** `RestrictedFs` is the interception customer: it depends on
  the effect it implements, under §5.1's binds-outward rule and shadowing
  — **O-R2 is load-bearing for phase 4**.
- **Streams stay linear tokens**: `linear struct InStream { handle: Long }`
  (ditto `OutStream`), minted by whichever handler answered the open, with
  the handler keeping id → its-own-representation in handler state (the
  host resource table inside `DefaultFs`'s platform class; a
  `Mut Map<Long, MemOpen>` in `MemFs`). This is FS-3's "workable version"
  of the token: the linear obligation lives *outside* the handler, the
  non-linear resource inside, so [linear-composite]/[effect-state-store]
  are satisfied. Application code declares `[Fs]` — only `[Fs]`.

What the flip pulls back in (FS-3's recorded costs, now owed):

1. **The shared-member-name decision is blocking again.** `read_line`,
   `write`, `close` as `Fs` members are the most collision-prone names in
   the language: a future `Net` wants all of them, and `close` collides
   *today* with the free-fn `close` dischargers of every linear pass
   (member-vs-free-fn resolution is exactly the [mod-collision]-adjacent
   question the `println@Console(...)` decision exists for). **A spelling
   sub-option that defuses most of it while keeping the user's call**
   (members are the dispatch seam; MemFs fakes everything): give the
   members collision-proof names (`fs_read_line`, …) and ship the public
   surface as thin std free fns — `fn read_line(s: Mut InStream) [Fs] ->
   Str | None { return fs_read_line(s) }` — which *overload* like any fn
   and keep the single-`[Fs]` signature the user wanted. Costs a
   forwarding layer in std; buys back free-fn overloading and gives the
   discharger question (next item) a clean answer. **Sub-decision: bare
   members vs wrapped members.** *Decided 2026-09-14 (third round): bare
   members, no wrapper layer — effects may share member names, and calls
   disambiguate with `@Effect` syntax (`read_line@Fs(s)`), the shape the
   ROADMAP due-before decision anticipated; that decision is hereby made.
   Backend note (user asked): the fused emissions are affected but the
   fix is mechanical, because the emitter owns every emitted name and the
   checker already records the resolved effect per call site
   ([effect-disambiguation]'s `effect_calls`). Rust: a multi-bounded
   fused generic (`__Fx: Console + Fs`) makes `__fx.close(…)` ambiguous
   (E0034) when two bounds share a member — emit UFCS
   (`Fs::close(&mut *__fx, …)`) for collided names (or always). Kotlin:
   handler objects are separate today (no ambiguity at call sites), but
   the planned uniformity fusion (`where T : Console, T : Fs`) cannot let
   one class implement two interfaces whose members collide at identical
   signatures — the emitter mangles emitted member names per effect
   (`fun fs__close(…)` on the emitted interface), which it is free to do
   since nothing outside generated code calls them. Both are emitter
   decisions ([rs-]/[kt-] rules), not language surface. The Has-trait
   proposal (§5.8.1) would supersede both answers.*
2. **The discharger question (FS-9 item 8) is live.** [linear-group]'s
   discharge set is same-file consuming *fns*; whether an effect *member*
   can be a discharger is undecided. Under wrapped members (above) it
   never arises: `close(s: InStream) [Fs]` is an ordinary same-file
   consuming fn that forwards to the member and `discard`s the shell.
   Under bare members, [linear-group] needs an amendment.
3. **Every `Fs` handler implements the full member set** — `RestrictedFs`
   writes pass-through bodies for every stream member (its policy is only
   in the opens). Mechanical noise, accepted; if it grates, "default
   member implementations on effects" is a *language* feature and a
   separate DECISION, not a phase-4 rider.
4. **Dynamic dispatch per read** — accepted with the flip.
5. **The token/interpreter lifetime hazard is now real.** A linear
   `InStream` may legally outlive the `use` scope of the handler that
   minted it (linearity permits returning it outward; registrations are
   block-scoped [effect-scope]) — a later `read_line` then dispatches to
   a *different* handler, or none. Under O-P2 this couldn't happen (the
   stream owned its resource); under O-P1 the resource lives in handler
   state. Policy to decide: document as "foreign id → clean `Err`"
   (per-handler id namespaces), or attempt a static rule (hard: it is
   exactly the region problem). Recommend the documented dynamic answer
   for v1. *Decided 2026-09-14 (third round): per-handler id namespaces,
   foreign id is a clean `Err`.*
6. **New verify items (extend FS-9)**: the `Lines` pass's `next` now
   declares `[Fs]` — confirm the pass/`for` machinery and
   `map`/`filter`/`reduce` compose with an effectful `next`; and
   [effect-member-no-effects]'s deduction spellings on members that take
   `Mut InStream` / consume `InStream` arguments.

FS-5's decided outcomes survive the flip unchanged (errors-at-close,
O-L2 `FsError`, discharge-by-`Ok`-narrowing). FS-7's member list needs
re-derivation (path ops + stream ops + `open_read_at`, and the token
fields); FS-8 is untouched (RestrictedFs's policy still lives entirely in
the opens).

### 5.8 The third round (2026-09-14, midday): linearity above the platform

Two calls landed on §5.7's list (bare members with `@Effect`
disambiguation; per-handler id namespaces) — recorded inline above — and
one objection reshaped the layering: **the user rejects linear
obligations being discharged by platform code.** Under §5.7 as written,
`close` — the discharger of a linear `InStream` — was a member whose
bottom implementation lived in the platform handler: the one operation
the type system guarantees would have its guarantee honored by a class
the Salvo checker never sees. The revision: **`RawFs` returns, but
*below* the members instead of beside them.**

```
effect RawFs { … }                    // raw ops over Long ids — nothing linear,
                                      //   fallible members return … | Err FsErrorKind
platform handler HostRawFs of RawFs   // FS-1's O-M2 target moves HERE:
                                      //   the platform trades only in plain Longs

effect Fs { … }                       // path members + stream members,
                                      //   linear InStream/OutStream tokens
handler DefaultFs [RawFs] of Fs { … }            // Salvo: mints/discharges tokens,
                                                 //   delegates to raw underneath
handler MemFs of Fs { … }                        // Salvo: pure, Mut Map state, no dep
handler RestrictedFs(root: Str) [Fs] of Fs { … }     // Salvo: interception (§5.1)
```

Why this does **not** resurrect the double-declaration problem that
killed O-T1: stream ops are *members*, so application code reaches them
through the `Fs` handler — `[Fs]` alone in every application signature —
and `DefaultFs`'s `RawFs` is a handler dependency, supplied at `use` from
the enclosing scope and never written in anyone's signature
[effect-handler-deps]. Composition roots:
`use HostRawFs(); use DefaultFs()` in `main`; `use MemFs()` in tests.
Recovered for free: the **grep-`[RawFs]` audit** (raw handles are only
reachable by declaring `[RawFs]`, which nothing but composition roots and
deliberate low-level code does), and **linearity minted and discharged
entirely in checked Salvo** — `DefaultFs`'s `close` body calls
`raw_close(s.handle)` and then `discard(s)`; the platform never holds an
obligation.

What it settles and what it needs:

- **FS-1's platform handler is `HostRawFs`**, not `DefaultFs` (which is
  now ordinary Salvo). The FS-1 decision text reads through accordingly.
- **The [linear-group] amendment is now entailed** (bare members +
  member dischargers): *consuming effect members declared in the linear
  type's file join the discharge set*; discharger status attaches to the
  **member declaration**, so every handler's implementing body — wherever
  the handler lives, including a `MemFs` in a test module — is a
  discharge context where `discard` is legal [linear-discard]. The
  member's consuming contract is its deduction: `fn close(s: InStream) ->
  Ok None | Err FsError => !s`. Forwarding composes: `RestrictedFs`'s
  `close` body discharges by forwarding into its dependency's `close`
  (the `shutdown(t) { stop(t) }` shape).
- **Sub-question: what does `RestrictedFs` depend on?** The
  user's phrasing was "Fs handlers … depending on the RawFs", but
  `RestrictedFs(root) [Fs]` (interception) is recommended over
  `RestrictedFs(root) [RawFs]`: on `Fs` it composes over *any* inner
  handler (`MemFs` in tests of the restriction logic — the §FS-2 O-R2
  motivation), and it inherits token minting from the inner handler
  instead of duplicating `DefaultFs`'s bookkeeping. On `RawFs` it is a
  second `DefaultFs` with path checks. Recommend: `[Fs]`.
  *Decided 2026-09-14 (fourth round): `[Fs]`, as recommended.*
- **Token plumbing under interception, verified by inspection**: an open
  through `RestrictedFs` forwards to the inner `Fs`, which mints the
  token in *its* namespace; subsequent reads dispatch to the innermost
  handler (`RestrictedFs`), whose stream members forward to `fs` — the
  token always lands back at its minter. The per-handler id namespace
  (decided) makes a token that *escapes* its stack a clean `Err`.
- **`@Effect` member disambiguation needs its grammar written** (the
  decision is made; the spelling is not): `read_line@Fs(s)` by analogy
  with `size@core.list(xs)` — does `@` take a bare effect name, a
  generic instance (`@Random<Int>`), a handler? One rule to draft in
  LANGUAGE_SPEC.md alongside the existing scope-selection `@`.
  *Decided 2026-09-14 (fifth round): `member@Effect(args)` with a bare
  effect name; the generic arguments are written in the effect signature
  (`next_random@Random<Int>()`) **when needed to disambiguate**, and may
  be omitted when the bare name is unambiguous.*

#### 5.8.1 The Has-trait fusion proposal (user sketch, 2026-09-14)

Prompted by the member-collision discussion in §5.7 item 1, the user
sketched an alternative emission for effect threading: the fused value
does **not implement the effect interfaces** — it implements one tiny
*accessor* interface per effect instance, and member calls go through the
accessor:

```kotlin
interface HasFs      { val fs: Fs }
interface HasConsole { val console: Console }
interface HasRandom_Int { val random_int: Random<Int> }

// private to the module that creates it — one per `use` scope
class Effects_1(
    override val fs: Fs,
    override val random_int: Random<Int>,
    override val console: Console
) : HasFs, HasRandom_Int, HasConsole

fun <Fx> something(fx: Fx, s: String)
    where Fx : HasFs, Fx : HasRandom_Int, Fx : HasConsole { fx.fs.… }
```

Analysis (this is emitter design — `kt-`/`rs-` rules, no language
surface):

- **It supersedes the UFCS/mangling answers of §5.7 item 1.** Member
  names can never collide on the fused value because the fused value has
  no effect members — only accessor properties, whose names the emitter
  mints per effect *instance* (`fs`, `random_int`). `close@Fs` vs
  `close@Net` becomes `fx.fs.close(…)` vs `fx.net.close(…)`.
- **Kotlin: it also dissolves [kt-effect-facets].** The facets machinery
  exists because one class cannot implement `Random<Int>` and
  `Random<Double>` (erasure); a fused *holder* has no such problem —
  `random_int` and `random_double` are just two properties. Adopting the
  design for the planned uniformity fusion would let the facet
  special-case retire. Erasure makes the multi-bounded generic a single
  emitted function, as the plan already noted.
- **Rust: the shape translates, with accessors as `&mut` getters** —
  `trait HasFs { fn fs(&mut self) -> &mut dyn Fs; }` — and the borrow
  discipline is unchanged from today's fusion: each member call reborrows
  the fused value for exactly the call (`HasFs::fs(&mut *__fx).close(…)`),
  the same duration argument [rs-effect-fusion] already rests on. Subset
  forwarding stays trivial for the same reason it does today (a generic
  `Fx: HasFs + HasConsole` satisfies a callee's `Fx2: HasFs` bound by
  monomorphization — no dyn upcasting anywhere). Scope *chaining* needs
  one shape verified: the inner fusion's accessor for an outer effect
  forwards through its provider field, whose `dyn` type is a per-scope
  conjunction trait (`trait __Prov_Fs_Console: HasFs + HasConsole`) —
  calls on it are supertrait method calls, which `dyn` supports; no
  dyn-to-dyn coercion is ever needed. Per the E1 precedent ("every shape
  verified by rustc before the emitter was taught to produce it"), this
  chaining shape plus a two-level `use` example should be hand-verified
  first.
- **Costs**: one accessor indirection per member call (a field read /
  `&mut` projection — negligible on both targets); a naming scheme for
  generic instances in accessor names (`random_int` — the emitter already
  keys environments by checker effect types, so the mangle has a
  canonical source); and on Rust the fusion structs remain per-scope
  generated types, now with accessor impls instead of effect impls —
  comparable emitter complexity, strictly better call-site emission.
- **Interaction to check when drafting**: fn *values* with effects in
  their type (`(s: Str) [Logger] -> Str`) — however those thread their
  effect today must stay consistent with accessor-based threading.

Recommendation: **adopt as the direction for both backends' fusion**
(Kotlin's planned uniformity pass; Rust as the evolution of
[rs-effect-fusion]), with the Rust chaining shapes rustc-verified before
any emitter work — but it is not phase-4-blocking: the current per-effect
threading works for `Fs` today, and this can land as its own emitter
milestone. **DECISION: adopt (now / later / not), and when.**
*Decided 2026-09-14 (fifth round): adopt, and **sequence it before the
filesystem work** — the Has-trait fusion is its own milestone, delivered
first.* ***Built 2026-09-14***, *same day, on both backends (with a
steering refinement: the single-effect case fuses too, for consistency).
The as-built record is COMPLETED.md's decision log ("Has-accessor effect
fusion") and the rewritten [rs-effect-fusion] / [kt-effect-fusion]
sections; [kt-effect-facets] retired unbuilt as predicted; every Rust
shape rustc-verified first, including two instances of one generic effect
through a single `dyn` provider.*

### 5.9 The open-decision list, consolidated (authoritative)

**Decided** (through the fifth round, 2026-09-14): FS-1 (O-M2
`platform handler`; target = `HostRawFs`, §5.8), FS-2 (O-R2
interception: binds-outward, shadowing, duplicate-instance error
retained; load-bearing), FS-3 (O-P1 bare members; `@Effect`
disambiguation with generics-when-ambiguous), the §5.8 layering
(linearity above the platform), `RestrictedFs(root) [Fs]`, the
Has-trait fusion (§5.8.1 — **adopted, sequenced before the filesystem
work**), FS-4 (a) `InStream`/`OutStream`, (b) buffered, (d) strict
UTF-8, (e) terminator rules, FS-5(a) `FsError` union payload, (b)
errors-at-close, O-L2 + `detach()`, both `close`s returning
`Ok None | Err FsError`, §5.3 O-B2 `open_read_at` + O-B3 byte counts,
§5.3.1 O-S2 per-operation with `Byte` → Kotlin `UByte` and **bytes in
v1**, token lifetime (per-handler id namespaces → `Err`), the
[linear-group] member-discharger amendment (entailed), FS-8 (a) lexical
+ documented, (b) rebase, (c) `PathEscapes`, (d) root as `Str`
(revisit marked).
**All remaining items must be decided before implementation starts.**

**Sixth round (2026-09-14, after midday): the last three items were
decided per their recommendations** — §5.10.1 (one `Fs` effect),
§5.10.2 (the surface as drafted, sub-questions A–F as recommended),
§5.10.3 (operator typing: numerics-only arithmetic, no `Str +`,
`Bool`-only logicals, integer widening, explicit int↔float, literal
adoption, promoted result type). **Nothing on this list remains open;
phase 4 is fully decided and implementation may be scheduled.**

The agreed sequence: **(1) the Has-trait fusion milestone (§5.8.1,
Rust shapes rustc-verified first) — ✅ built 2026-09-14, (2) the
`platform handler` mechanism (FS-1/O-M2) — ✅ built 2026-09-14
(COMPLETED.md's decision log; [platform-handler] and the two backend
rules; the host class is a companion in the declaring module's
`platform/` tree, and std's route is std shipping its own under
`std/platform/`, so `HostRawFs` needs only its two files and the
declaration), (3) the filesystem itself** — with operator typing (§5.10.3)
landing before or inside phase 4 as its prerequisite, and the
two-deliverable byte sequencing (§5.10.2 E) inside it. **Also built
2026-09-14, ahead of (2)**: the operator-typing slice, the `@Effect`
member-disambiguation grammar, and **O-R2 interception** (§5.1) — see
ROADMAP.md's S-IO list and COMPLETED.md's decision log.

**Propagation done 2026-09-14** (with the fusion milestone, once the
read-only restriction lifted): COMPLETED.md carries the decision-log
entries ("Has-accessor effect fusion" as-built; "Phase 4 fully decided"
summarizing the six rounds, pointing here for the full option record);
ROADMAP.md's phase-4/S-IO sections are rewritten to the decided plan and
both former due-before DECISIONs are closed (operator typing decided,
shared member names decided via `@Effect`; `platform handler`
un-deferred); the fusion rules are rewritten in BACKEND_SPEC.rust.md
[rs-effect-fusion] and BACKEND_SPEC.kotlin.md [kt-effect-fusion]
([kt-effect-facets] retired), and LANGUAGE_SPEC.md's
[effect-handler-deps] emission bullet updated. The remaining spec
drafting ([effect-handler-deps] binds-outward, [use-no-dup] shadowing,
[linear-group] member dischargers, the `@Effect` grammar, operator
typing) is *implementation-session work*, listed in ROADMAP.md S-IO.
This document is the phase-4 plan of record until phase 4 lands, then
retires into the decision log, OBLIGATIONS.md-style.

### 5.10 The fifth round (2026-09-14, midday): the remaining substance

Decisions recorded inline above: Has-trait fusion **adopted and sequenced
before the filesystem work**; the `@Effect` grammar (bare name, generics
in the effect signature when needed to disambiguate); O-S2 with **`Byte`
→ Kotlin `UByte`**, **bytes in v1** (sequenced deliverables if needed),
and **O-B3** (string writes return byte counts); read-side `close`
returns `Ok None | Err FsError` like the write side; FS-8 (a) lexical +
documented, (b) rebase, (c) `PathEscapes`, (d) root as `Str` (revisit
marked); FS-4 (a) two stream types, (d) strict UTF-8, (e) terminator
rules. Three items needed substance rather than a checkbox:

#### 5.10.1 Should `RestrictedFs` be its own effect type? (from FS-8(b))

The user's worry: with rebased paths, the implementer of a function
taking `[Fs]` "wouldn't know whether they are working relative to a base
folder or not." Two readings:

- **(i) One `Fs` effect (recommended).** Not knowing is the *feature* —
  the object-capability reading every precedent lands on (WASI, cap-std,
  chroot): a function receives *a* filesystem and writes paths against
  the view it was granted; whether that view is the host root or
  `/data/sandbox` is the composition root's business. This is what makes
  the sandbox *useful*: the code you restrict is by definition code that
  declares plain `[Fs]` (third-party, std one-shots, the `Lines` pass) —
  if restricted code had to declare a different effect, **you could no
  longer restrict code that wasn't written to be restricted**, which was
  the point (and `RestrictedFs` was just decided to be the interception
  customer). The convention to document: **write relative paths**;
  under `RestrictedFs` they rebase, under `DefaultFs` they resolve
  against the process CWD. An **absolute path is refused by
  `RestrictedFs` with `PathEscapes`** (the WASI/cap-std answer, and
  consistent with FS-8(c)'s distinguishable refusal), while `DefaultFs`
  accepts it — deterministic in both contexts.
- **(ii) A separate effect** (`ScopedFs` or similar) makes the base-ness
  visible in signatures, but: every std convenience is written against
  `[Fs]` and there is no effect polymorphism, so the whole surface
  duplicates; `[Fs]`-declaring code cannot run sandboxed at all; and the
  interception feature loses its first customer the day after it was
  made load-bearing.

**DECISION 5.10.1**: (i) or (ii) — recommendation (i), with the
relative-path convention and absolute-refusal rule documented in
LANGUAGE.md's fs section. *Decided 2026-09-14 (sixth round): (i) — one
`Fs` effect; relative-path convention documented; `RestrictedFs` refuses
absolute paths with `PathEscapes`.*

#### 5.10.2 FS-7 re-derived: the draft surface (for sign-off)

Everything below reflects the decided architecture (§5.8) and the fifth
round. Module: **`std/core/fs.sv`** (std stays single-directory; the
`std/io/` split can wait for a second io module). Names provisional;
**sub-questions the draft forces are bolded**.

```
// errors — O-L2
type FsErrorKind = NotFound | PermissionDenied | AlreadyExists
                 | NotADirectory | PathEscapes | InvalidUtf8
                 | StaleHandle | IoError
// each arm: struct … { path: Str }; IoError adds message: Str
// StaleHandle is the foreign-token answer (§5.7 item 5);
// InvalidUtf8 is FS-4(d)'s strict-decode failure
linear struct FsError { kind: FsErrorKind }
fn ignore(e: FsError) { discard(e) }                    // discharger
fn detach(e: FsError) -> FsErrorKind => !e { … }        // discharger, aggregation opt-out
fn to_str(e: FsError) -> Str => e { … }                 // non-consuming, [interp-to-str]

// tokens — minted by whichever handler answered the open
linear struct InStream  { handle: Long }
linear struct OutStream { handle: Long }
// per-handler id namespaces are an allocation discipline (documented),
// not a token field, until proven insufficient
struct FileInfo { size: Long, is_dir: Bool }

effect Fs {
    // path members — fallible, Err is the linear FsError
    fn open_read(path: Str) -> Ok InStream | Err FsError => path
    fn open_read_at(path: Str, offset: Long) -> Ok InStream | Err FsError => path
    fn open_write(path: Str) -> Ok OutStream | Err FsError => path    // create-or-truncate
    fn open_append(path: Str) -> Ok OutStream | Err FsError => path
    fn exists(path: Str) -> Bool => path
    fn metadata(path: Str) -> Ok FileInfo | Err FsError => path
    fn list_dir(path: Str) -> Ok List<Str> | Err FsError => path      // eager, entry names
    fn create_dirs(path: Str) -> Ok None | Err FsError => path        // creates parents (mkdir -p)
    fn delete(path: Str) -> Ok None | Err FsError => path             // files + empty dirs
    fn rename(from: Str, to: Str) -> Ok None | Err FsError => from, to

    // stream members — read side: errors end iteration, surface at close (FS-5(b)(iii))
    fn read_line(s: Mut InStream) -> Str | None => s: Mut             // None = EOF or recorded error
    fn read_all(s: Mut InStream) -> Ok Str | Err FsError => s: Mut
    fn read_bytes(s: Mut InStream, max: Int) -> Ok List<Byte> | Err FsError => s: Mut
    fn position(s: InStream) -> Long => s                             // consumed-byte position
    fn close(s: InStream) -> Ok None | Err FsError => !s              // reports recorded error

    // stream members — write side: writes are recorded, errors surface at flush/close
    fn write(s: Mut OutStream, text: Str) -> Long => s: Mut, text     // bytes written (O-B3)
    fn write_line(s: Mut OutStream, text: Str) -> Long => s: Mut, text
    fn write_bytes(s: Mut OutStream, data: List<Byte>) -> Long => s: Mut, data
    fn position(s: OutStream) -> Long => s
    fn flush(s: Mut OutStream) -> Ok None | Err FsError => s: Mut
    fn close(s: OutStream) -> Ok None | Err FsError => !s             // flushes, reports
}
```

- **Sub-question A — member overloading.** `close(InStream)` /
  `close(OutStream)` and the two `position`s overload *within one
  effect*; whether effect members may overload is currently undecided
  anywhere. Either allow it (checker resolves by argument types, exactly
  as [effect-disambiguation] already does across instances), or split
  names (`close_in`/`close_out` — ugly). Recommend: allow.
- **Sub-question B — write errors deferred.** `write`/`write_line`/
  `write_bytes` return only the count; a failed write is recorded in the
  handler and surfaces at `flush`/`close` (Go's `bufio.Writer` shape,
  symmetric with the decided read side). The alternative — every write
  returning `Ok Long | Err FsError` — makes every write in a loop
  narrow a linear-armed union. Recommend: deferred.
- **Sub-question C — `position` on both streams.** Without it the read
  side cannot compute record offsets at all (`read_line` strips `\n` vs
  `\r\n`, so byte counts are not recoverable from the returned `Str`).
  It is *not* seek — streams stay forward-only. Recommend: include.
- **Sub-question D — `create_dirs` with parents** as the only mkdir
  (the "pick one, name it clearly" answer). Recommend: yes.
- **Sub-question E — byte payload lowering.** `List<Byte>` in v1
  signatures with a specialized rendering (`kt-`: `UByteArray`; `rs-`:
  `Vec<u8>`), or boxed `List<UByte>`/`Vec<u8>`-of-boxed first and the
  rendering as a follow-up deliverable (the user allowed sequencing).
  Recommend: boxed first, specialized rendering as the second
  deliverable.
- **Sub-question F — the taxonomy** (8 arms above, `InvalidUtf8` and
  `StaleHandle` added to §FS-5's six). Recommend: as listed.

`RawFs` mirrors the surface one-to-one with `raw_`-prefixed members,
`Long` handles in place of tokens, `FsErrorKind` (non-linear) in place of
`FsError`, and no linearity anywhere — plus nothing else (the mirror
keeps `DefaultFs` a thin wrapper). The pass and one-shots as in §FS-7:
`linear struct Lines` (holding the `InStream`), `lines(s)`,
`open_lines(path) [Fs]`, `read_to_str(path) [Fs]`,
`write_str(path, content) [Fs]` (now `-> Ok Long | Err FsError` per
O-B3), `read_lines(path) [Fs]`, `next(p: Mut Lines) [Fs]`,
`close(p: Lines) [Fs]` forwarding into the stream's `close`.

**DECISION 5.10.2**: sign off the surface + sub-questions A–F.
*Decided 2026-09-14 (sixth round): signed off as drafted, with every
sub-question per its recommendation — (A) effect members may overload,
resolved by argument types; (B) write errors deferred to `flush`/`close`;
(C) `position` on both stream types; (D) `create_dirs` creates parents;
(E) boxed byte payloads first, specialized `UByteArray`/`Vec<u8>`
rendering as the second sequenced deliverable; (F) the 8-arm taxonomy.*

#### 5.10.3 Operator typing — what the decision actually is (user asked)

Current state ([deduce]-era leftover, ROADMAP "DECISION (open)"): binary
operators are typed only for `None` operands [op-no-none]; **everything
else is unchecked** — `Str * Bool` passes the checker with the result
typed as the left operand, and the backend emits it raw, so the *target*
compiler reports it (a [backend-never-wrong]-adjacent leak: Salvo's
"assumes it can see everything" stops at operators today). The `==`/`!=`
slice was decided 2026-09-12; what remains is arithmetic
(`+ - * / %`), ordering (`< <= > >=`), logical (`&& || !`), and unary
minus. The concrete sub-questions:

1. **Legal operand types per operator** — arithmetic on numeric types
   only; `+` on `Str`? (interpolation exists; recommend **no `Str +`**,
   `${}` is the concatenation story); ordering on numerics and on
   `canbe ordered` structs (already the `<` story); `&&`/`||`/`!` on
   `Bool` only (formalizing no-truthiness).
2. **Mixed-width numerics** — the phase-4 forcing case: `position(s) +
   size(line)` is `Long + Int`. Options: (a) **Kotlin-style implicit
   widening within integers** (`Int + Long → Long`), backends emit the
   conversion (`as i64` on Rust); (b) Rust-style refusal — every mix
   needs an explicit `to_long(x)`; (c) full C-style lattice including
   `Int + Double → Double`. Recommend **(a) for integer widths, explicit
   conversion for int↔float** — offset arithmetic stays writable, float
   surprises stay opt-in.
3. **Literal typing** — does the literal `1` satisfy a `Long` position
   (`offset + 1`)? Recommend: integer literals adopt the expected
   numeric type where one exists (Kotlin and Rust both do a version of
   this), so `Long` arithmetic doesn't force `to_long(1)` noise.
4. **Division/remainder** — `Int / Int` is integer division on both
   backends (no decision needed, but state it); `/ 0` behavior is
   backend-panic parity to verify, not design.
5. **Result typing** — today "left operand's type"; becomes the promoted
   type from 2.

**DECISION 5.10.3**: the slice above (1–3 and 5 are real choices), owed
before phase 4 per ROADMAP; the fs surface only *requires* integer
widening (2) and literal adoption (3).
*Decided 2026-09-14 (sixth round), per the recommendations: (1)
arithmetic on numeric operands only; **no `Str +`** (`${}` is
concatenation); ordering on numerics and `canbe ordered` structs;
`&&`/`||`/`!` on `Bool` only. (2) **implicit widening within integer
types** (`Int + Long → Long`, backends emit the conversion); **int↔float
mixing requires explicit conversion**. (3) integer literals adopt the
expected numeric type where one exists. (4) stated: `Int / Int` is
integer division on both backends; `/ 0` parity is a verify item, not
design. (5) the result type is the promoted operand type. This closes
the ROADMAP "operator typing" due-before DECISION.*
