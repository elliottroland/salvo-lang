# Salvo Compiler — Roadmap

What is **left** to build. Everything finished — the milestones, the four
completed sequences, the options explored and abandoned, and every closed defect
with its repro — is in [COMPLETED.md](COMPLETED.md); this document assumes it and
points into it rather than repeating it.

Consolidated 2026-09-26 (user request): the finished sections that used to fill
this file are gone, their reasoning already recorded in COMPLETED.md's decision
log, and what remains is **one sequence**.

Companion documents: [docs/language/](docs/language/) is the narrative spec
(source of truth); [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md) states every feature as a
labeled rule (`[qual-erasure]` style) with the compiler decisions under it;
`BACKEND_SPEC.<backend>.md` ([kotlin](BACKEND_SPEC.kotlin.md),
[rust](BACKEND_SPEC.rust.md)) repeats rules with backend interpretation and adds
backend-prefixed rules — load one only when working on that backend. Labels are
referenced from code and tests (`grep -rn '[rule-name]'`).

## How to read this

- **The sequence is the order of work.** Take the next unblocked item; nothing
  below step 1 is blocked on anything above it except where it says so.
- **A `DECISION` needs a language-design call before implementation**, and that
  call is the user's: state the options, the trade-offs and a recommendation,
  then wait (AGENTS.md's first invariant).
- **Standing constraints on every item.** Unsupported constructs are *errors*,
  never wrong output [backend-never-wrong]. The two backends must agree
  observably; a divergence is closed by restriction or by faithful emission.
  Backwards compatibility is not a requirement, so a rule change is a sweep of
  every example rather than a shim.
- **Before starting anything**: read COMPLETED.md's "Gotchas / lessons learned"
  for traps in the area you are touching, and its decision log for whether the
  question was already answered.

## Where we are

Everything the four sequences covered is built and running on **both backends
with identical output**: shared fate and borrow emission, linearity with a
designated `close`, effects through handler dependencies, `throw`/`try`, places
and field narrowing, deductions with refinements, the iterator reduction to
`next` (with any-name `iter fn`, `Iter` for sources and the `iter T`
placeholder — the 2026-09-27 redesign), the collections, the filesystem, actors (spawn, send, park, watch,
bridge), time, free concurrency, shareable-by-default handlers, refinement types,
group borrowing, the testing framework, the comparison/hashing capabilities,
actors across machines (`net`, the whole network sequence), one shape for
effects on both backends (every binding a handle, no fusion, no `local`), and
the first comptime slice (`comptime fn` and `by auto`, replacing `auto`: the
structural `cmp`/`eq`/`hash`/`to_str` are std functions over the fields of any
struct or the arms of any union).
Fourteen worked examples in `examples/` carry the checked-in generated code for both
targets and the output they print, three of them consuming the first dependency
(`modules/aws/`: `aws_profile`, `aws_sqs`, `aws_s3`). 1675 tests green.

## The sequence

### 0 — The runtime in Salvo (decided 2026-10-01/02, in progress)

The scheduler, routing and the host stream table are written once in Salvo
over a small `RuntimeHost` platform handler, instead of twice by hand
(`scheduler.kt` 1,994 lines, `scheduler.rs` 2,720). **[RUNTIME.md](RUNTIME.md)
is the working document**: the survey, every decision (D1–D11, platform types
in §12, the language expansions E1–E10 in §11.2), and the order of work in
**§11.5**, which is this item's sequence — take the next step there. Done so
far: steps 1 (the benchmark baseline), 2 (forged identities), 3 (the
stream table in its own runtime file), 4 (the group protocol in `net.sv`)
5 (`std/runtime.sv` as std's own module; platform `Never`) and 6
(`Deque`) and 7 (platform types, fn values at the boundary). When the sequence completes, RUNTIME.md
shrinks to what is still open, as ABI.md does.

### 0b — Three slow tests (recorded 2026-10-02, to investigate)

The user asked for these to be looked at together; they have been a problem
for a while. Measured 2026-10-02 on an M1 Pro, at a load average of 125–290
from outside the suite:

1. **`kotlinc_compiles_and_runs_every_case`** (Kotlin `codegen_tests`).
   Under `SALVO_E2E_FRESH=1 cargo nextest run` it hit nextest's 360 s
   timeout (`slow-timeout` 30 s × 12, `.config/nextest.toml`) on both of
   that day's commits; alone it passes in ~200 s. Warm, with every stamp
   hitting, it still costs ~31 s, all of it building the `KOTLIN_CASES`
   registry (each case runs parse + check + emit over std) before any stamp
   is consulted.
2. **`every_example_has_a_kotlin_case`** (same binary). Builds the whole
   registry too, only to read each case's `tag`, so it pays the same ~31 s
   in parallel with (1) for a check that needs no emission at all.
3. **The `salvo-testkit` hygiene test** (`hygiene_tests.rs`), ~10–11 s warm
   on every run.

Together they are why a warm `cargo test` takes ~1m02 against the ~15 s
budget in AGENTS.md. Leads, from the earlier records: the 2026-09-21 skip
gate fixed (1) under `SALVO_SKIP_E2E` only; the stamp stays keyed on the
generated sources (user decision 2026-09-25), so the cheap options are
those that avoid building a case whose stamp would hit — caching the
emission beside the verdict, emitting std once and sharing it across cases,
or a registry of tags that (2) can read without building anything. For the
fresh-run timeout: whether the batch kotlinc invocations scale with the
machine's free cores, and whether the driver should be split so nextest can
schedule its parts.

### 0c — Rust emitter defects (found 2026-10-02, not fixed)

Found while moving the group protocol into `std/net.sv` (RUNTIME.md §11.5
step 4). Each is refused by rustc, so none is silently wrong, but each stops
a correct program from building. std works around the first by naming.

1. **A second `let` of one name in a fn, after a loop that declared it,
   inherits the wrong move decision.** The later binding's narrowed reads
   inside a loop lower to `n.unwrap()` (a move) instead of
   `n.as_ref().unwrap()`, and rustc reports E0382. `net.sv`'s mechanisms
   name their departed node `departed` to avoid it. Repro:
   ```
   struct Node { id: Int, label: Str }
   fn take(n: Node) [Console] -> None => !n { println("took ${n.label}") }
   fn f(m: Mut Map<Int, Node>, k: Int) [Console] -> None => m: Mut {
       for id in keys(m) {
           let n = get(m, id)
           if !(n is None) { println("saw ${n.label}") }
       }
       let n = remove(m, k)
       if n is None { return }
       for i in [1, 2] { take(copy(n)) }
   }
   ```
2. **A union argument to a `send fn` through an addr is not coerced.**
   `b.go(A { x: 1 })` where `send fn go(v: A | B | None)` emits
   `__Msg_Bin::Go(A { x: 1 })` without the union wrapper (E0308); binding the
   value to an annotated `let` first works.
3. **An effect named `Box` makes a handle variable `box`**, a reserved word
   in Rust: `use Boxing()` of `effect Box` emits `let box = …`. The handle
   name needs `r#` escaping (or a mangled name) like other identifiers.
4. **Calling a fn value with its own result as the argument is E0499 on
   Rust**: `f(f(x))` for `f: (n: Int) -> Int` borrows `*f` mutably twice. A
   `let y = f(x)` first works. The emitter should hoist the inner call.

### 1 — ✅ The std reorganisation (complete 2026-09-26)

One theme: std's shape. Step (a) landed with the sitting that scheduled the rest,
and each remaining step is mechanical but wide — the sweeps are the cost, not the
design.

- **(a) ✅ `Checked<T>` in `core`, and a list write answers one** — built
  2026-09-26 [checked-type] [col-bounds] (COMPLETED.md's log). `take` is
  `detach`; the fallible `swap` hands back a `Checked<Bool>`.
- **(b) ✅ The filesystem on `Checked<T>`** — built 2026-09-26 [fs-surface]
  [checked-type] (COMPLETED.md's log). `Err Checked<FsError>` throughout; the
  bespoke wrapper, its `ignore`/`detach` and its `to_str` are gone.
- **(c) ✅ The filesystem out of `core`** — built 2026-09-26 [fs-module]
  [fs-host-split] (COMPLETED.md's log). Modules `fs`, `fs.host`, `fs.mem`,
  `fs.restricted`, unprefixed (user decision: (B1), with step 8's `std.` prefix
  left as its own slice). `DefaultFs` stays in `fs.host` rather than the surface
  module — the fusion gate reads reachable modules, so a `fs.mem` test would
  otherwise be fused by a handler it never registers.
- **(d) ✅ `core.nonempty` dissolves into the collections** — built 2026-09-26
  [qual-overload] (COMPLETED.md's log). Each `NonEmpty` sits beside its
  container; the four ordered accessors delegate through a `rename fn`, which is
  what let the module go. The recorded "`NonEmpty` constructor convention — the
  siblings" item is now *unblocked* rather than closed: `set_of(first, ...rest)
  -> +NonEmpty Set<T>` siblings are writable where they were not, and whether
  std should have them is an API question.
- **(e) ✅ `throw` out of `core`** — built 2026-09-26 [throw]
  [test-implicit-import] (COMPLETED.md's log). `import throw`, and a test annex
  gets it implicitly because the harness puts the effect there. Two defects fell
  out of the module's name: Kotlin keyword package segments now mangle
  [kt-package-keyword], and a duplicate emitted path is refused rather than
  clobbered [backend-companion].

**Step 1 is complete.**

### 2 — ✅ Actors across machines: the network sequence (complete 2026-09-27)

Eight steps, sixteen user decisions (2026-09-26), all built: COMPLETED.md's log
has one entry per step ("The network sequence, step ①" … "step ⑧") and the
round's entry ("Actors across machines — the network round"). What the
sequence left behind, by step — each a leftover, none a blocker:

- **① wire/transport**: `delay(node, d)` on `MemNetwork`; a `.test.sv` annex
  for `net` (test bodies have no `spawn`); the multi-face stateful monitor gap,
  which is why the double is two pieces.
- **② codecs**: keyed containers have no wire form; a generic `T` is refused at
  `encode`; Kotlin tuples past `Triple` have no codec.
- **③ routable addrs**: several remote senders may over-subscribe a mailbox by
  one each (the initial grant is "at least one"); a malformed remote answer is
  dropped silently.
- **④ node groups**: the partition/unreachable policy for gossip (a failed
  `deliver` → `left(n, "unreachable")`); the shared secret/TLS half of N-6 at
  the handshake; `HeartbeatNodeGroup` over a `Ddb` platform handler as the
  interop example.
- **⑥ `any`**: an `Addr<E>` answered by *spawning* a router binds as an
  ordinary `E` under `use addr` (the addr type has no room for the claim); a
  lambda's body is checked under strong availabilities; `local`/`any` on a fn
  type is reported once per lowering of the type (three times for a
  parameter), a pre-existing duplication.
- **⑦ picks**: consistent hashing for `Sharded` (today `key mod n` over the
  ordered view, so a join reshuffles most keys); a `[keyed E]` claim for a
  program relying on `Sharded`'s per-key ordering; a stub for a group whose
  replica is on another node (empty view, the stub parks); the stub is
  generated only where the module spells the protocol; two instances of an
  erased effect (`Pick<A>`, `Pick<B>`) cannot share a scope — a function per
  policy is the pattern, and lifting it means keeping a phantom on the erased
  trait plus a marker type per effect on the Rust side.
- **the tidy-up (2026-09-27)**: a node group now connects the node itself
  (2026-09-28, once handles let an actor member hand its transport to a
  spawn — COMPLETED.md, "A node group connects its node"). A mixed handler
  has no `init` block yet. The mixed servant / private-member unification
  (below) is unchanged.
- **⑧ the example**: `examples/cluster/` ships an election by host order behind
  `Leader`, not Raft. **Raft as a flagship example is still to be written** —
  terms, votes, heartbeats over `Timer`, a replicated log — and wants a
  deterministic clock across nodes first (virtual time is per process today,
  and messages between virtual nodes run on other threads). N-10's user-facing
  half — the manifest's `version` plus a lock file, so a protocol change
  without a bump fails the build — **built 2026-09-29** with the manifest
  (`salvo.lock`, [protocol-lock]).

Both groups stay **actors until the sugar pass** (section 13), which thereby
gains two concrete targets: `members()` as a plain read (the answering stub)
and `replyto` onto another actor's member (the remote mint). Recorded gap: the
deadlock graph is per program, so a wait cycle closing through a handler in
*another* program of the same node group is invisible — the same shape as the
"over types, not instances" gap, same deferred remedy.

### 2b — ✅ One shape for effects (complete 2026-09-28)

Built in four commits the day it was decided; the record — the decision, what
it took, what fell out, the shapes it replaced, the gotchas — is COMPLETED.md's
"One shape for effects" entry, the shape itself is under "Current architectural
facts worth knowing" there, and the rules are [effect-handle], [rs-handle],
[kt-handle]. The same afternoon the handle was keyed on
statefulness and named after the effect (COMPLETED.md's second 2026-09-28
entry). What it left open is in "Recorded, not scheduled": lock-free
scope-local bindings, and the erased-sibling refusal.

### 2c — After `protocol<E>`: fused emission back, effect polymorphism, `decode` as an implicit

The `actor_group`-over-`?protocol` round (COMPLETED.md, 2026-09-28 evening)
left three follow-ups the user has asked for, in this order.

- **Revisit fused effect emission** (user, 2026-09-28: "I'm happy to revisit
  the effect fusion — I personally find it more readable — but after the
  `protocol<E>` work"). §2b chose one handle parameter per effect over the
  fused `__fx` value for uniformity while the shapes were being settled; with
  the handle keyed on statefulness and named after the effect, the question
  is whether a fn declaring `[A, B]` should take one fused parameter again,
  and what that does to `[any E]`, forwarding into spawns, and the deadlock
  graph. The shapes that were deleted, and why, are in COMPLETED.md's
  "One shape for effects" entry — read it before redesigning. **DECISION**
  on the shape before code.
- **Effect polymorphism** — `fn run<T, S, E>(t: T, func: (T) [E] -> S) [E]
  -> S` (user has wanted it for fn-typed parameters; today [fn-effects]
  makes `run` inherit `Logger` from `func: (T) [Logger] -> S`, but the effect
  must be *named*, which is why std's `map`/`filter`/`reduce` take pure fn
  types). Sketch from the 2026-09-28 discussion: a *single* effect variable
  is cheap under one-shape — `run` never calls a member on `E`, it passes the
  handle through, so Rust is `fn run<T, S, E>(e: &E, …, func: impl FnMut(&E,
  T) -> S)` with no trait bound, Kotlin `fun <T, S, E> run(e: E, …)`; the
  checker treats `E` as opaque in the body ([call-resolve]: `run` cannot
  perform it, only hand it on). It must **not** be erased — `Console` and
  `Logger` are different handle structs where every `Addr<E>` is one
  `usize` — so `salvo_core::erase` needs "occurs in an effect list" as a
  reason a generic stays. A *row* (`func` performs several unknown effects)
  is where it gets expensive (no variadic generics on Rust; a bundle struct
  is the fusion shape again), so start with one variable and see whether std
  needs the row — `map(xs, x -> println(x))` needs one. To check: `[any E]`
  with `E` a variable; a lambda with an *inferred* effect set against `[E]`
  (bind when the set has one element, refuse naming the row otherwise); the
  deadlock graph prices the concrete instantiation at `run`'s call sites.
  Interacts with the fusion revisit (a fused value is one parameter whatever
  the row), so decide the two together. **DECISION**: single variable first,
  or the row.
- **`decode<T>` as an implicit** — the other intrinsic whose lowering reads
  its type argument, and the same shape as `?hash`/`?eq`: `?decode: (Bytes)
  -> T?` filled by the compiler-derived codec at the concrete call. Worth it
  beyond surface area: today `decode` in a generic body is refused by the
  *emitter* ("needs a concrete type argument"), a [backend-never-wrong] soft
  spot; as an implicit it becomes ordinary colouring ("add `?decode` to your
  signature"), and Kotlin — which cannot monomorphize — gets the codec passed
  in, which is its honest lowering anyway. After it, no user-visible
  intrinsic reads a type argument. Small; do it when `decode` is next touched.

### 2c — Comptime, the rest of the decided design (first slice built 2026-09-28; round 8 built 2026-09-29)

Seven rounds of user decisions (2026-09-28; COMPLETED.md's log, "Comptime, first
slice") and the first slice built the
same day: `comptime fn` with `<T is Struct>`/`<T is Union>` bounds and concrete comptime fns,
`by X` on obligation clauses (structs and `type` declarations) and on fn
declarations, `[for …]`/`[if …]`/`[when …]` (kinds `struct`, `union`,
`tuple`, `fn`, `opaque` = `basic` | `generic`), `v.[field]`, `[field]:` in a
literal, `refuse!`, `T.name`/`field.name`/`field.type`/`field.index`/
`field.first`/`field.last`, `core.auto` with `cmp`/`eq`/`hash`/`to_str` for
structs and unions, `auto` deleted, [interp-struct] made opt-in. Round 8
(2026-09-29) respelled it — `comptime fn/struct/type`, `<T is Struct>`,
bracketed heads, `refuse!(…)` — declared the model in `core.comptime` as
compile-time-only structs with `Type` a union the kind `when` dispatches over,
and added hover inside comptime fn bodies. What the decided design still owes,
in build order:

- **Implicits on stamped fns for generic structs** (ROADMAP §2c, decided
  "from the start", built as a refusal with the remedy named). `struct
  Wrapper<T> : Ordered<self> by auto { value: T }`: a copy meeting an opaque
  `T` **records a need** instead of failing; the stamped signature gains
  `?Ordered<T>` (deduplicated by name and type); and [group-obligation]'s match
  **ignores trailing implicit parameters** — the relaxation section 16 needs
  too, and the third customer is the obligation clause on `intrinsic type`
  below. The principle it rests on is [deduce-infer]'s: a fact inferred from
  the body, printed by the language server. `Checked<T>`'s `to_str` waits on it
  [checked-type].
- **Obligation clauses on `intrinsic type`** (decided).
  Parsed today, not swept: `intrinsic type List<T> canbe Mut : Hashed<self>,
  Ordered<self>, ToStr<self>` with the container identities written **in
  Salvo** in `core.list`/`core.set`/`core.map` (`fn eq<T>(a: List<T>, b:
  List<T>, ?Eq<T>)`), the scalars keeping their `intrinsic fn`s and declaring
  them (`intrinsic type Int : Ordered<self>, Hashed<self>, ToStr<self>`), and
  `ToStr<self>` on `List` rendering each element with its own `to_str`. Needs
  the relaxation above and recursive implicit resolution (section 6). Until
  then the **interim** structural `cmp`/`eq`/`hash` intrinsics over `List` and
  2-/3-tuples in `core.compare` stand in — they relaxed the recorded "tuple key
  refused by name" limitation on both backends, and an element struct's *own*
  `cmp` is still not consulted inside a list (the Rust backend keeps
  `Hash`/`Ord` derives on structs that have the functions for exactly this).
  Landing it deletes those intrinsics and the conditional derives.
- **`by` at a call** (ROADMAP §2c, decided): `mut_set_of(hash by auto, eq
  by auto)` stamps the implicit at the type the call binds — a third `by` site,
  emitted in the calling module once per (comptime fn, type), paired by
  [implicit-with], printed as the stamped root. The only way a tuple gets an
  identity, and the missing-tuple-identity diagnostic should name it meanwhile.
- **Typed implicit overrides** (ROADMAP §2c, decided): `to_str(xs,
  to_str: (Person) -> Str = short_name)` — an override keyed by name *and type*,
  consumed at whichever level of a resolution chain it fits (unused: error),
  also the spelling for two same-named implicits at different types; makes the
  carried identity a **tree** printed only where a level is not the canonical
  default (section 6's decision 3). Needs section 6.
- **`field.default`** (a compile-time optional of the declared `= expr`), for
  the codec slice's `from_json` — recorded, unbuilt.
- **The `json` module** (`Json` union per SERDE.md SD-1, `to_json`/`from_json`
  comptime fns spelled `by json`, tag rules per SD-4) — waits on SERDE.md's SD-1 and
  SD-4 decisions and on section 6 for container fields.
- **Docs**: a `Compile-Time-Functions.md` page exists; LANGUAGE_SPEC has
  `[comptime-*]`, `[obligation-by]`, `[fn-by]`. COMPTIME.md was deleted with the
  rounds in COMPLETED.md's log; SERDE.md's SD-3 carries a superseding note.

Settled 2026-09-29 (user decisions): a module *selector* names a module by any
**unambiguous suffix** of its path — `size@list`, `by auto`; an `import` stays
fully qualified — with a shared predicate and an ambiguity refusal
[mod-suffix]; and a
`by` name that is both a module and a comptime fn is disambiguated as `by
@auto` (the module) or `by auto@mymod` (the fn), `@import` having been
withdrawn because it confuses where a module *goes*.

Known and recorded: two `to_str` overloads that both fit a *narrowed* value
(`to_str(Manual)` beside a stamped `to_str(Source)` where `value: Source` is
narrowed to `Manual`) make `${value}` ambiguous at the interpolation while
`to_str(value)` resolves by [fn-overload-rank]; the comptime fn bodies therefore call
`to_str(value)` rather than interpolating an arm. Pre-existing
[implicit-resolve] locality, not a comptime defect.

### 3 — Recorded: the restrictive reading of a fn-typed slot's lend

Both defects of the 2026-09-25 round are closed — a bare generic struct literal
now determines its type arguments [struct-literal-arg], and a fn-typed slot whose
return is opaque now **infers** its lend [proj-infer-fn-type] (both 2026-09-26,
COMPLETED.md's log).

What the second one left open, deliberately, is the reading it chose *against*:

- **A slot could state that its fn does *not* lend.** `(c: C) -> Mut It` would
  then mean "the iterator must not borrow the container", and implicit resolution
  would refuse `core.list`'s borrowing `iter` for it — naming the candidate and
  `holds proj(c)` — while accepting `core.set`'s snapshotting one. The machinery
  is already there: resolution **already** compares a candidate's contract against
  a slot's and refuses a mismatch (probed 2026-09-26: a keep/consume mismatch is
  reported with both remedies named), so adding the lend to that comparison is
  small.
- **Why it was not taken**: the restrictive reading buys nothing observable
  today. The two things it would protect — mutating or consuming the container
  while the iterator is live, and the iterator outliving the container — are both accepted
  under *either* reading (probed), and in a generic body there is nothing an
  opaque `C` can be done to anyway. So the restriction would have cost an
  annotation on the language's most ordinary generic-iteration signature to
  express a property nothing yet depends on.
- **The trigger for revisiting**: a program that *needs* an iterator independent of
  its container — one that stores it, returns it past the container's life, or
  sends it to another thread ([actor-sendable] refuses a borrowing iterator, so a
  task or actor taking an iterator is the likely first customer). When that appears,
  the restrictive slot becomes worth stating, and the shape is above. Note the
  matching must be **directional**: a lending slot still accepts a non-lending
  candidate, so it is subtyping on the contract rather than equality.
- ✅ The spare clause **warns** (user decision 2026-09-26, built): writing
  `holds proj(c)` on an opaque-returning slot says what the signature infers, so
  it is reported the way a no-op `@place` selector is [fn-overload-at] — one
  diagnostic per clause, naming every source, and silent where the return is
  concrete.

### 4 — ✅ Project manifest and LSP source-root discovery (built 2026-09-29)

Decided (M-1…M-7) and built the same day; COMPLETED.md's log has the entry.
`salvo.toml` with `[project]`/`[build]`/`[rust]`/`[kotlin]`, discovery by
nearest ancestor with nested manifests as boundaries, every command reading it
flag > manifest > default, `backend = "*"`, per-document analysis in the LSP,
`std = true` for `std/`, and `salvo.lock` for the protocol hashes (section 2's
step-8 customer, now closed). The repo-root defect is gone: `std/`, every
`examples/*/` and `demo/` carry manifests, so the root opens as several
programs.

Left behind, none blocking: `salvo lock` as an explicit command was not added
(a build writes the file); the `--target` flag under `backend = "*"` is
per-backend for `run`/`test` (suffixed) and refused for `compile`; only a build
(`compile`, `run`, `test`) reconciles `salvo.lock` — `analyze` and the language
server read the manifest but never write the lock, so an editor session cannot
produce one. Dependencies between projects, parked here as out of scope, got
their first slice on 2026-09-29 ([manifest-deps]: `[dependencies]` + `[build]
modules`, `modules/aws/` consumed by `examples/aws_profile/`; COMPLETED.md has
the entry). **What that slice left for later**: transitive dependencies (a
dependency's own `[dependencies]` are not followed), version ranges and
resolution, a fetcher that fills `salvo_modules/` from anywhere, a dependency's
own `.svignore` and companions beyond `platform/`. The `aws` module's content is
designed (`modules/aws/DESIGN.md`) and needs three things of the compiler first —
§4b.

### 4b — What the first dependency needs of the language (decided 2026-09-29, not built)

`modules/aws/DESIGN.md` records the design (user decisions D1–D8 that day):
services generated from the public Smithy models by a `smithy-build` plugin
living in `modules/aws/codegen/`, wrapping aws-sdk-kotlin and aws-sdk-rust through
generated platform handlers (the `smithy-dafny` pattern), non-blocking by taking
a `Reply` and returning at once, streaming bodies through one std effect. The
module is a *user* of Salvo and must not extend the CLI; what it needs is three
AWS-neutral extensions, in this order. None is a **DECISION** — the calls were
made in the design sitting — but each has a shape to settle at implementation.

1. ✅ **Host dependencies of platform code** — built 2026-09-29
   ([platform-host-deps] [rs-cargo] [kt-classpath]; COMPLETED.md has the entry).
   Maven coordinates are now resolved by Gradle (2026-09-30, [kt-gradle]
   [host-tool]); left: a lock for host versions, repositories besides Maven
   Central.
2. ✅ **Host-completed continuations** — built 2026-09-29 ([platform-reply];
   COMPLETED.md has the entry). Left: a host reply to a remote-minted token (the
   typed wire path), and detecting a lost reply on Kotlin.
3. **`std.stream`** (DESIGN §5; decisions 20–27, the third sitting). Moving
   `InStream` alone broke the same-file rule, so streams move *as a layer*:
   `stream` owns `InStream`/`OutStream` and one effect `Streams` with every
   stream operation; `fs` keeps paths and mints into it. In order:
   a. ✅ **Effect prerequisites** — built 2026-09-29 ([effect-prereq]).
   b. ✅ **One handle counter** — built 2026-09-29 ([stream-handle]).
   c. ✅ **The split** — built 2026-09-29 ([stream-layer] [stream-table]
      [stream-provider]; COMPLETED.md has the entry).
   d. ✅ **Non-blocking** — built 2026-09-29 ([stream-receive]
      [stream-from-bytes] [stream-pipe]; `Reply<T canbe linear>`).
   Left from step 3: the writer pair for Salvo-produced bodies a host consumes,
   asynchronous `Lines`, a consumer-only face, and `MemStreams` for a test with
   no filesystem (DESIGN §9).
4. **Provider-checked stream handles** (DESIGN §9, decision 10 of 2026-09-29's
   second sitting). A `MemFs` stream handed to `HostByteSource` traps at
   runtime; the user wants the compiler to see it. Sketch to start from: a
   handle carries its **domain** as a provenance qualifier (`Host InStream`,
   `Mem InStream`), each handler declares the domain its handles live in, and
   at a `use` the effects in scope must agree on one — an error at the binding,
   not at the read. **DECISION** at implementation time: qualifier vs. a type
   parameter on the effect vs. a property of the handler declaration, and how a
   program that legitimately mixes domains says so. Not blocking §4b 1–3.
5. ✅ **Dot-names under a `type`** — built 2026-09-29 ([name-dot]; Kotlin nests
   the members in an `object`).
6. **Handler bundles** (recorded 2026-09-29, **DECISION**, not scheduled). A
   production composition root binds a whole collection of handlers, and the
   stream layering makes even a host filesystem four lines (`use
   HostRawStreams()`, `use DefaultStreams()`, `use HostRawFs()`, `use
   DefaultFs()`). A named bundle — one declaration that binds several handlers
   in dependency order — would make that one `use`. First customer: streams +
   fs; the shape (a declaration form, a fn that `use`s, how a bundle composes
   with `with` and interception) is the user's call when it is picked up.
7. **Program end on the JVM** (found 2026-09-30, **DECISION**: to be designed
   fully before building). A Rust program ends when `main` returns; a Kotlin
   program ends when the JVM's last non-daemon thread does, so a host library
   holding one keeps the process alive after `main` — OkHttp, under
   aws-sdk-kotlin, for 60s after its last call. The stopgap is in the aws glue
   (`salvoCloseWhenMainEnds`: a daemon thread joins `main` and closes the SDK
   client). The general fix is the Kotlin runtime ending the process when the
   Salvo program is done (`exitProcess` after `main`). To settle: what "done"
   means when actors, spawned pools or host-completed replies are still
   outstanding (today's rule on each backend, and whether the two agree);
   whether host code gets a shutdown hook (closing clients, flushing) before
   the exit; the exit status (a fault that reached the sink, a trap); and the
   cost — every emitted `main` changes, so every golden and example tree.

The second sitting's other calls (DESIGN.md's numbered table) fix the shapes
above: `Cargo.toml` only when crates are declared; a **directory of jars** for
Kotlin then (`[kotlin] libs`; since 2026-09-30 Gradle resolves `artifacts`); host-dependency version conflicts
refused; an undischarged reply reported to the fault sink where detectable;
provider-sized non-empty chunks; `Err Checked<StreamError>`; a wrong-provider
handle traps; `InStream` and `OutStream` both move.

Then, outside the compiler — **built 2026-09-29/30 for SQS and S3**: the
generator (`modules/aws/codegen`, a smithy-build plugin; `modules/aws/README.md`
has the from-scratch setup), `aws.sqs` for six operations with a recording
`FakeSqs`, `AwsConfig` with a `Credentials` union and an `endpoint` override,
`aws.s3` for `PutObject`/`GetObject` with bodies as `stream.InStream` in linear
structs and timestamps as `time.Instant`, `examples/aws_sqs` and
`examples/aws_s3` in the suite, and `modules/aws/demo/{sqs,s3}_live` run by
hand against both SDKs through local stand-ins (COMPLETED.md has both entries).
Uploads stream and require `content_length` (2026-09-30). Left, in the module
(DESIGN §9): multipart upload for a body of unknown length, Smithy unions,
paginators and waiters, more S3 operations (buckets, listing).

### 4c — Unions of literals, then the clients over them (decided 2026-09-30)

The user's order: (1) ✅ a drift test for the aws glue; (2) ✅ **unions of
literals** (built 2026-09-30, [type-literal]; left: `Byte` literals, which
need literal syntax first); (3) ✅ `aws.sqs`/`aws.s3` regenerated with
literal-union enums and one error struct per service; (4) ✅ the host ABI
written down ([platform-abi], [rs-host-abi], [kt-host-abi]), then ✅ **host code with Salvo splices** (built 2026-09-30, [host-splice]).
✅ the aws glue moved onto splices, then onto **platform templates**
(`platform/<m>.sv.kt`/`.sv.rs`, 2026-09-30), ✅ with editor support for them
(highlighting and the language server, 2026-09-30; left: a template whose
module has no `.sv` file is reported at file 0, span 0 — the embedded std — so
`salvo analyze` names it but the editor drops it; the diagnostic needs a
location of its own, the template), ✅ platform roots in the manifest, a
template-writing `platform generate` and ascription by a place (2026-09-30) —
all of which the templates' removal (2026-10-01) has since undone except the
platform roots; (5) DynamoDB.

✅ **The platform ABI** (user direction 2026-10-01; complete the same day,
COMPLETED.md): generated host projects, interfaces, adapters, boundary
checks, factories and stamps beside hand-written implementation files. The
design and its decisions (D1–D10) are in [ABI.md](ABI.md). **Next: DynamoDB.**

### 5 — Consistency passes the 2026-09-26 ambiguity round left

Three narrower questions, all downstream of "refuse to choose" (COMPLETED.md's
log for the round itself).

- **✅ Identical-signature shadowing stays an ambiguity** — confirmed
  2026-09-26 (user decision, COMPLETED.md's log). Nothing to build. What came
  out of confirming it is a **small new question**: the remedy is
  `f@<this module's path>(…)`, and there is no shorthand — `@mod` does not
  exist, and `@mod`/`@import` as *rung* keywords were rejected 2026-09-07
  because they ask the reader to know which rung a name arrived on. A `@mod`
  (or `@self`) meaning **"this file's module"** is narrower than what was
  rejected: one fixed meaning, no rung to know, and it stays short where a
  module path does not (`size@orders.pricing(xs)`). Options: leave it (the path
  is explicit and greppable, and a call needing the selector is rare by
  design); add `@mod`; or add `@self`, which already means "this handler" for a
  send [actor-self-send] and would then mean two things. Recommendation: leave
  it until a real program reads badly — the trigger is narrow enough that the
  evidence may never arrive.
- **Implicit resolution still resolves by rung**, deliberately: an implicit has no
  written call site to annotate, and two same-named types have no distinguishing
  selector at all. If that is to change it needs a spelling first.
- **The rest of the resolution-by-position rules.** The round covered calls and
  refinements. A pass over the spec should list every remaining
  resolution-by-position rule and decide each — separating *silent winners* from
  *designed shadowing* (a later `use` shadowing an earlier handler is interception
  semantics [effect-intercept], and a fn-typed local shadowing a name outright is
  the caller's explicit choice).

### 6 — Recursive implicit resolution (decided 2026-09-28, not built)

[implicit-resolve] **skips a candidate that itself needs implicits**, so
`fn eq<T>(a: List<T>, b: List<T>, ?Eq<T>)` can never be what a `Set<List<Person>>`
resolves, and the container identities have to be structural host intrinsics
(the interim ones `core.compare` carries since 2026-09-28, see section 2c). The
lift has two halves: resolution — `resolve_implicit_fn_at`'s one-line skip
becomes a recursive resolution — and emission — a filled implicit that itself
needs implicits is handed its own, so `ImplicitArg::Resolved` needs nested
arguments and both backends' adapter closures pass them. The second is the
larger.

**The six decisions, all made** (user, 2026-09-28, comptime rounds 3–7;
COMPLETED.md's comptime entry holds the argument):

1. A candidate whose own implicits cannot be filled is **an error naming the
   chain** (`cmp for (Int, Foo) needs cmp for Foo: none in scope`) unless
   another candidate resolves fully, in which case that one wins — never a
   silent non-candidate (SFINAE).
2. **Depth cap 8**, worded "resolution of `cmp` for `<type>` nests more than 8
   levels deep; pass `cmp = …` explicitly"; a chain that needs itself is refused
   at once as a cycle, with the chain printed.
3. **The carried identity is a tree** [cmp-carry], compared as one — two sets
   over `(Int, Person)` keyed by different `Person` hashes are different types —
   and **printed only where a level is not the canonical default**, so
   `Set<List<Person>>` prints bare in the ordinary case and as
   `Set<List<Person>>(hash@List(id_hash), eq@List(same_id))` when the user
   wrote the leaf. (Round 6 reversed the earlier "canonical by construction"
   choice once the typed override below could produce a non-canonical leaf.)
4. **`with` pairing propagates** [implicit-with]: `Hashed<(A, B)>` fills
   `Hashed<A>` as a pair; half a pair at any level is the existing error.
5. **std owns container identity**, in Salvo, via obligation clauses on
   `intrinsic type` (section 2c); the interim intrinsics and the Rust
   backend's conditional derives are deleted when it lands, and `xs == ys` on
   two `List<Person>` then consults `Person`'s declared `eq`.
6. Recursion applies **everywhere resolution runs**: calls, picks,
   interpolation's `to_str` lookup, and the copies inside a comptime fn.

Waiting on it: `Checked<T>`'s `to_str` [checked-type], `expect_eq` on a generic
container [interp-to-str], property testing's `?generate`, section 2c's
`intrinsic type` clauses, typed overrides and `json` over container fields.

### 7 — Qualifiers are droppable, then variance

- **Qualifiers are droppable on assignment** (user decision 2026-09-23, not
  built). A variable's type may never *widen*, but a qualifier is by definition
  something that optionally applies, so dropping one is always legal: assigning a
  plain `List<Int>` to a variable inferred as `NonEmpty List<Int>` must be
  accepted, and the variable simply stops being `NonEmpty` (the flow state already
  models exactly this — a mutating call drops claims the same way). Provenance
  qualifiers need thought: dropping one is harmless, re-*gaining* it must stay
  impossible. Where it shows up today: `analyze_tests`'
  `fate_links_merge_across_branches` and `inferred_moves_consume_arguments` had to
  annotate their `let`s when the constructors started claiming, and those
  annotations come back out.
- **An inferred type argument is not widened by the expected type** — found
  while building step 1(b), and the same invariance in a narrower place.
  `checked<T>(value: T)` binds `T` from the argument, so
  `checked(NotFound { … })` is a `Checked<NotFound>` and the expected
  `Checked<FsError>` does not widen it — not through a `let` annotation either
  (probed). std therefore names the argument at all 35 construction sites,
  `checked<FsError>(NotFound { … })`, which is explicit and costs a word. The
  declared-position widening that the old `FsError { kind: … }` relied on still
  works; what does not is *seeding a type variable* from the expectation. The
  keyed-container work seeds a callee's type parameters from the expected type
  already, so the machinery is there — the question is whether an expectation
  may pick a **supertype** for an inferred argument, which is the same question
  variance asks and should be answered with it. **A small DECISION**: widen
  (unify against the expectation's arms, so `checked(NotFound { … })` at a
  `Checked<FsError>` position infers `FsError`), or leave the explicit form as
  the one way and keep this recorded as intended.
- **DECISION — variance on generic parameters** (user direction 2026-09-23). A
  `List<NonEmpty List<Int>>` is not a `List<List<Int>>` today, so a constructor
  call cannot fill a plain type-argument position. The direction is `in`/`out`
  declaration-site variance as C# and Kotlin have it. What has to be decided with
  it: declaration-site or use-site (recommendation: declaration-site first,
  because std's containers are where it pays); what `Mut` does to it (a
  `Mut List<T>` cannot be covariant in `T` — the classic array-store hole — so the
  natural rule is that covariance holds only while the value is not `Mut`);
  whether qualifiers on a type argument are a **separate, narrower** rule worth
  pricing first (the case that raised this needs only `NonEmpty List<Int>` →
  `List<Int>` *inside* a type argument, which is qualifier-dropping at depth); and
  that the emitted Rust must not depend on it, since Rust has no variance.

### 8 — Mutating through a union arm (DECISION)

One shape is **refused on Rust and accepted on Kotlin**, which is a divergence
closed by restriction on one side and therefore a decision rather than a resting
place:

```
fn bump(o: Ok Mut List<Int> | Err Str) [] -> None {
    if o is ^Ok {
        add(o, 7)          // Kotlin: mutates the caller's list
    }                      // Rust: reported — `o` is read-only here
}
```

The parameter renders `&Union2<…>` because an arm's `Mut` is not a claim about
`o`, and **nothing can ask for the `&mut`**: a written `=> o: Mut` is refused
("a deduction may preserve or drop qualifiers, not add them"). So the question is
what a `Mut` arm means for the *parameter* that carries it.

- **(a) The arm's `Mut` makes the parameter mutable.** Cost: the Rust signature
  then disagrees with the checker's contract, which still says the parameter is a
  kept read — and two arguments naming the same place, which the checker permits
  as two reads, become two `&mut` borrows and an E0499 on a program Salvo
  accepted. Teaching the fate analysis about that is the real scope.
- **(b) Admit the deduction.** Relax "may not add qualifiers" so `=> o: Mut` is
  legal when `Mut` is present on an arm, and let inference write it from the body.
  Cost: a written deduction now means "through an arm", a new reading of the
  clause; benefit: the contract stays visible in the signature, which is what
  deductions are for.
- **(c) Keep the refusal, and refuse it in the *checker*** so both backends say
  the same thing. Cost: a Kotlin program stops compiling; benefit: one story, and
  the remedy (take the payload as its own `Mut List<T>` parameter) is one line.

**Recommendation: (b)**, with (c) as the fallback. (a) is the one to avoid: it
makes the two sides of the compiler disagree about what a signature means, which
is how the original defect happened.

### 9 — Testing, beyond the MVP (decided 2026-09-23, not built)

The framework's core is built and `salvo test` runs std's own suite. What was
deliberately cut, in the order the decisions put it:

- **`context` scopes** — decided in shape (TF-9): nesting plus per-test re-run
  initialization (`use`s and `let`s that run again for every test, so no
  cross-test state is expressible), with the test's id joining the context names.
  Three sub-calls ride the implementation: statements **preamble-only**
  (recommended, error otherwise), a preamble may do **anything a test body may**,
  and a **childless context warns**. The lowering is per-test inlining, which
  makes isolation true by construction.
- **Property testing** (TF-5): `check(runs, property)` with the generator arriving
  as the implicit **`?generate`**, so the *qualifier on the parameter type picks
  the generator* (`(text: ValidDate Str) -> …`); randomness threaded as a
  `Mut Rng` **value**, which the effect-free-resolution rule turns into a
  determinism guarantee; shrinking by replaying the generator over a shrunken draw
  stream. Two prerequisites: std needs wrapping/bit `Long` intrinsics for a
  pure-Salvo splitmix64 (parity by construction — a recommendation, not yet
  decided), and step 5's implicit lift.
- **Actor testing helpers** (TF-6): `settle(p)`/`expect_settled(p)` over
  [actor-on-idle], a recording `Probe<M>` handler, `expect_fault(target, body)`
  over [actor-watch]. Needs `spawn` added to [test-body]'s implicit powers, and
  the harness's default fault sink to record rather than print. No test scheduler
  (rejected as out of scale).
- **The blackbox tier** (TF-2's `tests/` tree): ordinary modules that `import`
  what they test and see only exports.
- **`--isolate`, `--timeout`, crash recovery** (TF-4): one process per test, a
  per-test wall-clock kill, automatic re-run of a crashed batch's remainder. The
  MVP runs everything in one process and reports a test that died (`DIED`).
- **The `std.` import prefix** (TF-8, decided 2026-09-19): `import std.time`
  rather than `import time`, `std` a reserved root, the bare spelling refused with
  the corrected path named, and the library tree moving to `lib/std`. Sequenced
  after the MVP deliberately. Step 1(c) shipped **without** the prefix (user
  decision 2026-09-26, option B1), so the filesystem's modules are `fs`,
  `fs.host`, `fs.mem`, `fs.restricted` and this step renames every top-level std
  module at once. What it drags, and why it is its own slice: module paths derive
  from the embed root, so moving the tree to `lib/std` renames `core.*` to
  `std.core.*` — five hardcoded `"core"` sites in `resolve.rs` deciding implicit
  visibility, `[std-shadow]`'s classification, the CLI embed root, and the
  emitted path of every std module in every golden and every example. **Whether
  `core` becomes `std.core` at all is the decision to take first**; the
  alternative examined in that round was a reserved `std` root mapping to the
  embedded library whatever the on-disk layout, leaving `core` unprefixed as a
  stated exception.
- **Smaller leftovers**: the annex's one-way visibility holds by construction but
  is not *checked*; the report has no `--format json` for editors; nothing
  migrates the compiler's own e2e suite onto `salvo test` (user decision: leave
  it).

### 10 — The assertion trap policy (A-6, decided 2026-09-23, not built)

Three failure classes still take the hosts' behaviour:

- **Subscript out of range** → trap with *our* message (index and length),
  replacing the hosts' two different texts. Needs care on the *place* path
  (`arr[i] = x`), where a block expression cannot stand.
- **Division by zero** → trap with our message; both hosts already trap, only the
  text differs.
- **Integer overflow** → **wrapping**, stated in the spec, with `checked_*` /
  `saturating_*` std functions for the cases that care. This is the JVM's
  behaviour today and the cheap one on Rust. **It is the only row that changes
  what existing programs compute**, so it wants its own slice and a parity test:
  today Kotlin wraps silently while Rust refuses a constant fold and panics in
  debug.

### 11 — One read, one mode: the rendering that reports a reference

Three slices landed 2026-09-23 [rs-read-mode]; what is left is the **refactor the
section is named after**. The slices work because their sites know the shape they
will get and can ask a predicate first (`owned_optional_local`,
`narrowed_borrow`). The general case cannot: a site that wants `&T` writes
`&{code}`, and if `{code}` is already a `&T` the result is `&&T`. The durable
answer is for the rendering to answer *what it produced* (`Rendered { code,
is_ref }`) so a site can decide whether to add the `&`.

The site waiting on it is **interpolation**, the most common read position in the
tree, which still clones every narrowed or `!`-ed value:

```rust
// std/test.sv's expect_trap_with, where the same `trap` two lines up borrows
format!("… got `{}`: {}", trap.as_ref().unwrap().clone(), label.clone())
```

`emit_interp_value` cannot simply switch to `emit_read`: its native branch would
take the reference happily (`format!("{}", &String)` displays), but the two
`to_str` branches in the same function write `to_str(&{arg})` and
`{place}.{field}` — three questions to the predicate in one function, which is
precisely the shape that wants the rendering to answer for itself.

Worth finishing because it is the last **systematic** copy nobody wrote,
`[copy-opt-in]` is a stated principle rather than an aspiration, and it is
measurable: `examples/*/rust/**` carries the clones, so the diff *is* the
benefit. Two recorded items are the same missing information in other clothes:
the temporary-subject `for` loop, and the copy an adapter closure makes of a
returned projection.

### 12 — Effect transformers (E3 step 4)

The last rung of the handler-control arc. A **transformer** is an effect member
that runs a fn-typed parameter with *additional* effects available — its body
having registered handlers for them. `Retry`, `Timeout` and async are all that
shape, and `try` could be re-expressed as a library transformer if it reads
better than the intrinsic.

- **The gate is already built**: step 3 made fn-type effect lists real and threads
  effects *into* a fn value instead of capturing them [fn-effects]. What remains is
  the surface.
- **No silent colouring** (user decision 2026-09-04): the ability to not resume is
  declared on the effect member, never discovered from the handler.
- **Async is explicitly not part of this arc**: it arrives later as an explicit
  effect, likely a compiler intrinsic, not as an `async`/`suspend` transform of
  the whole program.
- **Multi-shot resumption is closed on principle**: resuming twice duplicates a
  use obligation, so it cannot coexist with `Linear`. (Neither target offers it
  either.)

Two related recorded items: a **handler cannot dispatch to itself** — inside a
member the bare member name is already taken (it means the handler registered
*before* this one [effect-intercept]), so self-dispatch needs a different
spelling, and that spelling is a language call (`self.bump()`, `bump@self()`, …);
the workaround (a free fn both members call) is what std does and has cost
nothing. And **two cuts inside the effect fusion** stay *reported* rather than
mis-emitted [rs-effect-fusion]: a dependent handler using its own generic
parameters in a member signature, and a `use` whose effect instance is still
generic.

### 13 — The sugar pass (after the explicit surface, decided 2026-09-15)

Phase 5 delivered the **explicit** actor surface (tokens and reply parameters
written out); every layer of sugar above it is a later item with its own decision
surface. What is already decided, so the pass starts from a plan:

- **Per-kind `-> T`** (EU-6): plain effects unchanged forever; inside an
  `actor effect`, `fn m(a) -> T` means an implicit trailing `Reply<T>`,
  fulfil-at-every-return, and caller-side call syntax = auto-mint + gate. This is
  why `send fn` stayed explicit: the non-send forms are stated *against* it.
- **The generalized mint** (EU-7b): `replyto k(c)` resolves lexically against the
  enclosing handler, else through the effect list as a *remote* mint — a curried,
  capacity-reserved, one-shot send, with reservation at mint time so discharge
  never blocks, and mint sites contributing deadlock edges like sends.
- **Two stub readings appear here, and only here.** An answering member cannot
  have one implementation for both bindings: from an actor the call parks, from
  synchronous code it must block.
- Then the rest of the tower, each its own call: `then`/`then!`, `defer`,
  merge/join, the gate's member-set generalization.
- [fate-lambda] belongs here — no first-pass form crosses a closure. The recorded
  refinement is `move`-closure emission with hoisted clones, plus a treatment for
  captured effect-handler locals.

### 14 — Shared mutable state: `Cell` (DECISION)

Deferred until after actors deliberately, because the OTP answer is that actors
own their state and message-pass — which may remove the motivation. The full
design (a capability *qualifier* rather than a container type, the representation
per backend, why it cannot panic, the rules it drags in, and the producer case
that is its hardest customer) is recorded in COMPLETED.md's "Shared mutable state
(`Cell`)" section, moved there with this consolidation. The question to answer
first is whether shared mutable state joins the language at all.

When it is taken, it should be decided **on one table** with the rest of the
sharing story: shared-immutable versus shared-mutable, invalidation-checked
versus unchecked (today's fate links, frozen `Reg`, `canbe` groups, `Cell`) —
with the observation that `proj` is the degenerate group (a read-only member of a
singleton group under maximal invalidation sensitivity), so the two are points on
one dial rather than two features.

### 15 — Regions (designed 2026-09-10, unbuilt)

Fully designed and recorded: `effect Region` with an intrinsic handler, `Reg` as
an intrinsic provenance qualifier, regional-by-birth defaults, `reg`/`unreg`, the
freeze (dropping `Mut` makes handles duplicable, kills fate links and makes state
qualifiers permanent), and the escape rule. Kotlin erases it entirely; Rust stages
it — v1 `Rc<T>` [rs-region-rc], v2 a real arena with one mechanical lifetime per
delimiter [rs-region-arena]. The design, the R2 rejection (regions manage memory
and lifetime, never obligations) and what it retires for frozen values are in
COMPLETED.md's "Regions" section, moved there with this consolidation.

Still open when it is picked up: the exact freeze spelling (`^Mut` as an
expression, freeze-by-position, or both); cross-region operations (out of v1);
`unreg` of a deeply regional structure copying deeply; and folding D7's
`Local`/`Escaping` watch-list entry into the design.

### 16 — Composing iterators: stages over a generic source (after the redesign)

The iterator redesign (COMPLETED.md, 2026-09-27) settled §16's *spelling*: a
stage is an `iter fn`. Over a **concrete** source it works today —

```
iter fn evens(xs: List<Int>) -> Emitted Int | Finished {
    state { inner: Mut ListYield<Int> = iter(xs) }
    …
}
```

— nothing is minted until the call, per drive, so replay is free, and
`for n in doubled(evens(xs))` chains. What is **not built** is the generic
stage, in either of its two forms, both refused today at the `iter fn`:

- **over a generic source**: `iter fn evens<C>(c: C, ?Iter<C, Int>)` with
  `state { inner: iter Int = iter(c) }`;
- **over an iterator**: `iter fn doubled(it: iter Int)`, the position
  *consumed* into the hidden struct (today every `iter fn` parameter is
  borrowed and read-only).

Both need the same two things (ITER_REDUX.md, round 6): the generated `next`
carries the **forwarded implicits** (`?Yield<It, Int>` on the hidden `next`,
forwarded by whoever drives it), and the [group-obligation] match **ignores
trailing implicit parameters** when checking the hidden struct's `: Yield<self,
T>` — implicits are the callee's business, resolved at the call, not part of
the member's shape. The alternative is storing the inner `next` as a fn-valued
field, which [rs-stored-implicit] already lowers for keyed containers; the
implicit-on-`next` form is cleaner. A hand-written generic stage (`struct
MapIter<It, T, U> : Yield<self, U>` with `next(p, ?Yield<It, T>)`) hits the same
obligation gap today, so this is a prerequisite rather than a cost of the sugar.

What falls out once it lands: pipelines as values (`let pipeline = c ->
doubled(evens(c))`, passed as `total(xs, iter = pipeline)`); `zip`, `chain`,
`take` as `iter fn`s with iterator parameters and no hand-written struct; the
two-sources case (`?Iter<A, T>, ?Iter<B, T>` — two implicits named `iter` and
two named `next` at different types) as the first thing to test.

What stays parked: a stage over a **linear** iterator (`Lines`). The hidden
struct would have to be `linear` with a discharger, and the user decided
(2026-09-27) no discharger is generated — a discharger would have to be a
member of the group to be consistent — so that iterator is written by hand,
and generic code that may receive one takes the discharger as a callback
[linear-generics]. Sendability of a composed iterator is unchanged.

**Cleanup riding along**: the internal names still say "pass" (`PassDriver`,
`PassMember`, `pass_elem_ty`, `pass_driver_of`, `for_drivers`' doc) and the
dead residue of the deleted `yield fn` origin machinery is still in the tree
(`PassDriver.origin`, always `false`; `mint_machines`, `ORIGIN_PASS_PREFIX`,
`pass_or_origin_elem_ty`'s origin branch, the emitters' `if driver.origin`
arms). Rename and delete in one change, cutting by function rather than by
region (the 2026-09-10 lesson).

**Riding along, decided 2026-09-28 (comptime rounds 3 and 7):** two implicits of
**one name at different types** — `fn say_hello<T, S>(it: T | S, ?to_str: (T) ->
Str, ?to_str: (S) -> Str)` — are legal (the "two implicits named `iter` at
different types" this section already anticipates); the **override spelling** is
the typed one section 2c lists (`to_str: (Person) -> Str = f`), with two of the
same name *and* type being one binding [cmp-binder]; what is left to decide is
the **interpolation lowering** — `${it}` with `it: T | S` becomes a `when` over
the union's arms choosing the implicit per arm, which a union value's runtime arm
index permits [union-arm-identity] — and whether [interp-to-str] extends to one
`to_str` per arm.

### 17 — Recursive types (DECISION, end of the queue)

Investigated 2026-09-12; nothing needs it, and List-mediated recursion covers its
customers (trees, ASTs, JSON) meanwhile. Where it stands: nothing rejects a
recursive type, so `struct Node { value: Int, next: Node | None }` passes the
checker, runs on Kotlin and dies at rustc with E0072 — an accept/reject
divergence. Recursion **through `List<T>` already works end to end on both
backends**.

- **Step 1, the diagnostic** (a defect fix, independent of the feature): an SCC
  walk over the type graph (struct fields, union arms, alias expansions;
  `List`/array/fn-typed edges do **not** count — they indirect already) and an
  error at the declaration naming the field that closes the cycle, with the
  `List<T>` encoding as the named remedy. [type-no-cycle] landed the declaration
  refusal for the *direct* case 2026-09-25; the union-arm case is what is left.
- **Step 2, the feature**: a boxing rule for the Rust backend — where the box goes
  (minimal-edge boxing, the presumption), transparency at every use site (literals
  wrap, reads autoderef, matches need explicit derefs since box patterns are not
  stable, partial moves keep working, `Mut` paths get `&mut` via `DerefMut`,
  `copy` deep-clones), and **non-regular (polymorphic) recursion refused in
  Salvo** — a type may recurse only at its own instantiation, or Rust
  monomorphizes forever while Kotlin's erasure accepts it.
- **The semantic edges**, each a small language call: **constructibility**
  (`struct A { a: A }` has no base case — refuse cycles with no optional/union
  escape arm, recommended); **depth, not cycles** (values are acyclic, so
  `to_str`/equality/drop terminate, but each recurses per node and a 100k chain
  overflows generated `toString`/derived `Drop` — accept-and-document for v1);
  **linearity stays out** (refuse recursion + linearity in one declaration for
  v1); and an `iter fn` over a recursive subject snapshots per field, which is a
  deep copy.

## Recorded, not scheduled

Each was considered and deliberately parked. Nothing here is blocking, and
several are "revisit only if a customer appears".

- **The whole program in the host project** (user, 2026-10-01; ABI.md D4).
  The alternative to generating only the declarations the platform surface
  reaches: write the entire emitted program into the platform root as
  `*.sv.*`, so the host project is exactly the build's compilation. No
  closure to compute or keep in step with the emitter, at the cost of every
  function body in the root and a much larger checked-in diff per change.

- **Full Salvo expressions in splice holes** (user decision 2026-09-30:
  revisit if splices prove themselves at the small scale). Today a hole
  holds a closed set of emission-dependent spellings with host code only at
  the leaves; the general form would type-check Salvo embedded in host text,
  with typed escapes back to it.

- **Whether the monitor spawn of a plain-effect handler stays** (user
  decision 2026-09-28: keep the split as is for now). `spawn H()` where every
  face of `H` is a plain effect answers an `Addr<E>` and shares one locked
  instance [monitor-handler]; it was the only way to share a stateful plain
  handler between spawns until [effect-handle] made every `use` a handle and
  [spawn-inherit] hands the scope's bindings to a child. What it still buys
  is a plain-effect instance as a **first-class value** (storable, sendable
  in a message, passable in `with`); nothing in std or the examples uses it
  that way. Options if it is revisited: keep as the addr-producing form (two
  spellings for one shape, plus the by-name refusals — `on`, multi-face,
  `mailbox` — that exist to keep it honest); restrict `spawn` to actor
  effects; or restrict it and give `use` a value form, which folds into the
  pending `using` rename in [actor-use-addr]. Decide together with lock-free
  scope-local bindings below, which touches the same shape.

- **Implicit comptime fn resolution** (2026-09-28, comptime round 5, punted): an
  implicit that finds no candidate for a tuple could stamp `core.auto`'s comptime fn
  without being asked, making `mut_set_of()` over a tuple work bare. Deferred
  because it is the one place a comptime fn would apply without a `by`, which round
  1 decided against for everything else; the remedy `hash by auto, eq by auto`
  at the call (section 2c) is one line, and the missing-identity diagnostic
  should name it.

- **A `for` element of a union-typed list does not fit a parameter of that
  union** (found 2026-09-29 building §4b item 5, not fixed; unrelated to it).
  Repro: `struct A {}` `struct B {}` `type AB = A | B` `fn d(x: AB) [] -> Str
  => x { return "x" }`, then `for c in list_of<AB>(A {}, B {}) { d(c) }` —
  "no matching overload for `d(proj (A | B))`". The element is a `proj` view of
  the union, and the overload fit does not see through `proj` on a union the
  way it does on a struct. Workaround: bind the element to a typed local
  first. Likely a small fix in the arg-fits rule; needs its own test on both
  backends because the Rust borrow of a union arm is what `proj` renders.

- **Lock-free scope-local bindings** (2026-09-28, the one item left of the
  lock-free pass the one-shape decision deferred; the stateless half landed
  the same afternoon — a stateless handler, the send stub and the mixed façade
  hold no lock, [rs-handle]/[kt-handle]). A *stateful* `use` whose binding no
  spawn, task or dependent handler captures could skip the mutex; this is
  what `use local` used to select by hand. Emission-only, invisible to a
  program. The design, so it need not be re-derived:
  - *The callee never notices.* `draw() [Random]` is `fn draw(random:
    &Random)` and `random.next()` matches on the handle's arm at runtime, so
    a lock-free binding is one more arm (`Local(…)`) chosen at the `use`
    site; `draw` compiles unchanged, and the invariant that a fn's signature
    never depends on its callers holds.
  - *"Nothing captures it" is transitive.* The binding is live across every
    fn reachable from the scope, any of which may spawn a handler inheriting
    `[Random]` or mint a task capturing it. So the property is a call-graph
    fixpoint over the capture sites the checker already records (`use_deps`,
    `spawn_deps`, `task_mint_effects`) — the shape the deleted
    `handle_requirements`/`call_edges` propagation had.
  - *The handle must stay `Send + Sync`.* Spawns clone handles across
    threads and `Random` is one type for every binding, so a
    `Local(Rc<RefCell<H>>)` arm would make every `Random` `!Send`. The
    options are an `unsafe impl Send` on the local arm's wrapper justified by
    the checker's proof that a `Local` handle never reaches a spawn (a user
    call — unsafe resting on a static analysis), or a lighter single-thread
    lock. On Kotlin none of this arises: skip `__Mon_E`, bind raw.
  - *Why parked*: the saving is an uncontended mutex per call on stateful,
    never-shared bindings; the cost is a whole-program analysis plus either
    an unsafe assertion or a new lock flavour. Build it when a program shows
    that mutex mattering.
- **Eager handle cloning**: `Stamped::new(logger.clone(), clock.clone())`
  bumps two `Arc`s per `use`; fine, noted.
- **The erased-sibling refusal** (`Pick<A>` beside `Pick<B>` in one scope,
  [effect-generic-decl]) was kept through the one-shape change. Its original
  reason — two `__Has_Pick` impls colliding on one fusion struct — is gone,
  and the emitters resolve a call by instance, so lifting it is probably a
  matter of deleting `refuse_erased_sibling` and its test and restoring
  `examples/cluster/`'s `two_ids`/`shop` to inline blocks. Left because the
  two erased handles are still two handlers of one *type* in one scope
  (`__Handle_Pick`), and the cluster example reads fine as two functions;
  try it when the example next changes.

- **Unify the mixed servant with private send members** (2026-09-27). A mixed
  handler's servant emits a handler-keyed `__Msg_H` and calls its members
  directly from `resume`; a private send member of an actor handler now does
  the same through `__Priv_H` [actor-private-send]. They are one mechanism
  in two emitter paths (`emit_mixed_actor_parts` beside `emit_actor_body`);
  re-expressing the servant as "a handler with no actor faces and only private
  members" would delete the first. Churn in two emitters, three goldens and
  `examples/`, for no behaviour — do it when one of the paths next needs a
  change.

- **Pick chains** (`^Ok?: Err?: err(_)`) — recorded rather than scheduled at the
  user's call: the same program is expressible today with one pick plus a `when`,
  so a chain buys brevity and the evidence that would settle it is *a real program
  in `examples/` or `std/` that reads worse without one*. Decided and still
  standing if built: picks consume arms **in order**, and the empty pick may appear
  anywhere in the chain. Cost: `Expr::Elvis` needs `picks: Vec<ElvisPick>`, the
  checker consumes arms pick by pick, both emitters emit an `if`/`else if` chain;
  the parser's lookahead already exists.
- **Value-level parity for hash and random.** Callable `hash` lowers to each
  backend's **native** hashing, so hash *values* diverge across backends —
  accepted deliberately, on the analogy of random numbers. The shape and the
  high-level guarantees are identical (`eq(a,b)` ⇒ `hash(a) == hash(b)` within one
  execution). The possible future reversal, for hash and random *together*:
  language-defined algorithms implemented identically in both runtimes (the
  rejected sketch: FNV-1a 64 over a canonical byte encoding). Until then a program
  must not print or persist a hash value and expect cross-backend identity.
- **Platform-handler thread-safety contract — decided 2026-09-26**, as
  `threadsafe platform handler`; built as step ① of the network sequence
  (section 2). What used to be the DECISION here is in COMPLETED.md's log.
- **`on_idle`'s predicate (DECISION).** The hook fires on the strict quiescence
  condition while the deadlock report fires on a weaker one, so a program stuck
  *with a parked frame* gets the report and exit 1 where the relaxed reading would
  let it react. One line in each runtime either way; the argument for relaxing is
  consistency, the argument against is that `on_idle`'s meaning drifts toward
  "nobody can move", which is the report's job.
- **`size(Str)` outside ASCII (DECISION).** Kotlin lowers it to `String.length`
  (UTF-16 code units), Rust to `chars().count()` (code points), so
  `println("${size("a😀b")}")` prints 3 on Rust and 4 on Kotlin. Needs a decision
  about what a `Str` index *means*, then one lowering per backend. `byte_size` is
  parity-safe by construction and is what the filesystem uses.
- **Intersection types (DECISION)** — whether `Addr<A & B>`-style types join the
  language; recorded 2026-09-17 when the tuple form shipped instead.
- **`platform type`** and **`platform effect`** — deferred by decision (the
  second removed 2026-10-01); `platform handler` and, per ABI.md, `platform fn`
  are the whole interop surface until a need arises.
- **`const` bindings** — announced (user intent 2026-09-19), not designed. Its
  first customer is recorded: the shareable-handler taxonomy's rung 1 keys on "no
  mutable state", which today means "no fields, no `Mut` constructor parameters";
  `const` immutable fields would join the allowance.
- **`Deque<T>`** — the honest replacement for a linked list, and the next
  collection when a customer appears: one intrinsic type, six functions, no new
  concepts. (A representation qualifier `Linked List<T>` was examined and
  rejected: it would make the shared `List` surface worse, and Rust's
  `LinkedList` has no stable cursor API.)
- **`entries`/`values` iterators over a Map** — deferred: an entries iterator needs an
  owned `(K, V)` and Kotlin cannot copy a generic `V`, so identity-sharing would
  alias mutable values. The answer is probably the snapshot shape the key iterator
  uses, over a `List<(K, V)>`.
- **Test-suite speed** — now sequence item 0b (2026-10-02).
- **Locators through opaque anchors, branded tokens, and the bounds-check
  mitigation ladder** — the group-borrowing ladder's recorded refinements, with
  GhostCell declined on the record (a brand is a scope-bound *lifetime* and actor
  state escapes every scope) and raw pointers / `RefCell` rejected. In
  COMPLETED.md's log for 2026-09-24/25.
- **In-place writes during iteration** — refused by the driven-origins rule
  [iter-fn]. The contents-versus-replacement distinction [deduce-field] is what
  would license it, and the locator model already makes the shape renderable.
- **Field-set inference for [deduce-field]**, and qualifiers on struct fields:
  v1/v2 are written-only, so an unannotated fn keeps the conservative whole-value
  event.
- **`once` inference**, **returning/storing capture-carrying closures**,
  **exactly-once closures consuming a linear capture**, **reassignable borrowed
  locals** (accumulator bodies under a projected return), **same-call borrow/move
  (E0505 shape)**, **the internal qualifier unification**, **L5 field-disjoint
  precision**, **`NotEq` over any `?eq`-capable subject**, **link parameters**
  (with the nested-`proj` source they would make real), and **type-mention tracing
  for instantiation links** — all unforced, each recorded with its shape where its
  rule lives.
- **The linear instantiation ban misses generic effect members** — a hole in what
  shipped: `[linear-generics]` is checked in `resolve_named_call`, and an effect
  member's own generics are not covered, so a linear value can be smuggled
  through one.
- **Nested patterns** (`let ((a, b), c) = …`) are refused in a `let` as in a
  loop, and **struct patterns bind by field name only** (no `..` rest, no nested
  field pattern, no binding of a projection).
- **A generic `List<T>` cannot be interpolated** [interp-to-str] — waits on
  step 5's lift plus `implicit_args` being keyed by something an interpolation
  has.
- **Emission marks everything public**, by decision: [mod-export] is a checker
  rule. Narrowing generated visibility would buy dead-code warnings the suite
  already tolerates and needs the reachability pass to agree.
- **No re-export**, so a facade module declares wrappers; the spelling would be
  `export import a.B`, currently a targeted parse error. And **no example shows
  `export`**, because every program in `examples/` is a single file — a two-file
  example would fix that and would be the tree's first multi-module one.
- **Where dot-notation is normalized** [fn-dot] — the receiver-as-argument-0
  rewrite happens twice and never in the AST, so anything inspecting the *written*
  expression must re-derive the shift (one defect came from exactly that). Two
  shapes when it is picked up: record the normalized argument list per call span
  in a side table (cheap), or a real normalization pass after types are known
  (removes the duplication). It cannot move into `salvo-syntax`: the name must
  resolve, and an `Addr<E>` receiver is not argument 0.
- **Actor-surface prose is owed in `docs/language/`.** Every actor rule is in
  LANGUAGE_SPEC.md, and docs/language/ has the "Where work runs" and "Time"
  chapters — both of which assume vocabulary the document never introduces
  (`actor effect`, `send fn`, `spawn`, `Addr`, `replyto`/`waitfor`, `watch`).
- **Actor leftovers, each with a named trigger**: a self-send into a full own
  mailbox wedges and `k@self` deliberately contributes no deadlock edge
  (**DECISION** when it matters); the deadlock graph is over actor *types*, not
  instances, so a chain of same-protocol workers reads as a self-loop
  (stratification and the timeout form wait for observed false positives —
  writable now that `Timer` exists); the report names actors by index rather than
  by handler and member; a one-thread pool whose occupant sends into a full
  mailbox on that same pool still hangs (only the `main` case is caught); a `use`
  site does not check the `[spawn]` capability; a main-pool task whose answer
  arrives after `main`'s last wait never runs, silently; an effectful discharger
  cannot `drain` a container; there is no positional list write; `on_idle`'s
  refinements (per-pool firing, naming who is parked, a many-shot form); FC-7 host
  bridging; and the `[waitfor]` spawn-placement diagnostic still states a hazard
  the pump rule removed.
- **Multi-effect and mixed-handler leftovers**: a same-named member across two
  faces still needs `@Effect` at the call even where the parameters distinguish it
  ([effect-at] keys on the name); a dependent multi-face handler is untested;
  mixed handlers with dependencies, overloaded servant members and several faces
  were first-slice cuts; one lock behind several faces has no backend
  representation; generic-instance dependencies stay fusion-pinned (the
  representation now exists, so lifting the `handler_handle_deps` exclusion is
  engineering); a `with` item may not have dependencies of its own, and `main`'s
  platform-effect parameters cannot be captured as handles.
- **`local` inference** — the checker writing `local` for you, lifting the
  virality down local-trafficking call chains (the deduction pattern: written
  validates, unwritten infers). **Note (user, 2026-09-26): the meaning of `local`
  is itself being revisited** — a round of questions about `println`'s
  `local Console` dependency was deferred with "I might have misunderstood what
  `local` means", so re-read [use-local] and [effect-local] with the user before
  building anything here.
- **`fn qualifies@Positive`** — migrating qualifier bodies' `fn qualifies` to the
  `@`-scoped shape canonicals used. Note that the shape it would migrate *to*
  changed on 2026-09-26 [fn-attached], so this is now "should a qualifier's
  `qualifies` be declared on its subject type?" and wants re-deciding rather than
  implementing. Handler members stay put: they interact with handler state.
- **Constants: literal establishment and constant subtyping** — the refinement
  round's two remainders. Literal establishment is `listen(8080)` proving itself
  (compile-time evaluation of `qualifies`); constant subtyping is
  `InRange(10, 20)` fitting an `InRange(0, 100)` position, which needs
  per-qualifier semantics for what the constants *mean* — **DECISION**-shaped
  when it is wanted.
- **`enumerate`'s claimed `index` field and a claimed `keys` iterator** — left out of
  claim minting by design: a dependent claim on a struct field names a value the
  struct does not contain, and the map iterator walks a key snapshot. The snapshot
  form (`keys -> List<KeyOf(map) K>`) waits for step 6's variance round.
- **The `Bytes` span twin** needs same-name-different-subject value slots.
- **Binding a view of a temporary** is refused for now (user, 2026-09-11); the
  possible automation is hoisting the temporary into a fresh local, which is what
  the user writes today — not free of judgement, since the temporary then lives to
  the end of the block and a hoist inside a loop changes how often it is built.
- **A runtime file name colliding with an emitted std module** — the *silence*
  is closed (2026-09-26): emission refuses at the end when two files claim one
  path [backend-companion], which is how it was caught the second time (`throw`
  left `core`, and the runtime's `throw.kt` had been winning that path). The
  workaround is still a rename each time — `hosttime.{rs,kt}`, now
  `throwsignal.kt`. The durable fix, unscheduled: namespace the runtime under
  `salvo_rt/`, which touches every golden, every example and the documented
  `kotlinc`/`rustc` invocations, so it wants its own slice.
- **`to_str(Duration)` stops at seconds**, and there is no `to_str` for `Instant`
  or `Tick`: a wall-clock text form is a date (the calendar layer's), and a
  monotonic reading has no rendering beyond its number. **Cancellation is not in
  the timer surface** (recorded by decision), and **the wall-clock layer is
  designed but unbuilt** — `DateTime` as the calendar view of an `Instant` in a
  zone, `Period`, the two bridges, and a `WallClock` effect whose member must
  **not** be named `now`.
- **The unified test clock fakes `Ticker`; the `Clock` face is untested**, and a
  handler that waits on a *positive* deadline still wedges a `ManualTime` test —
  though it now says so, via the deadlock report. Every clock reading through the
  unified form is a round trip, which is the stance's remaining cost; the recorded
  upgrade is scheduler-owned virtual time.
- **The parked-obligation gap in the deadlock graph**: an actor gated on a token a
  *task* must discharge has a wait-for edge pointing at no effect node.
- **Array elements never narrow**, **deduction inference does not track
  bare-parameter value flow out of branch/loop tails as a move**, **module
  reachability is conservative for non-fn names**, **a private type in an
  exported signature is an opaque type and nothing checks the author meant it**,
  **struct destructuring ignores predicate-qualifier field overrides**,
  **repeating the *same* qualifier in a nested group needs an annotated
  intermediate `let`**, **`Byte` is not operator-numeric**, and **generic
  (`Ty::Var`) operands stay lenient** — each documented where its rule lives.
- **LSP**: `[symbol]` resolution is name-based over the AST rather than
  import-visibility-exact [doc-symbol-ref]; no incremental analysis; no
  `positionEncoding` negotiation for UTF-8-native clients; signature *hover*
  covers fn decls only (effect members and define fns have no `FnKey`); the
  def-site table is name-keyed, so go-to-definition on a shared effect-member
  name lands on one declaration; and hovering a fate *root* to see what derives
  from it is not built.
- **Fresh-suite speed, remaining steps toward ~10–15s**: the CLI suites (each
  spawns `salvo run`/`compile` and pays its own kotlinc), the rust codegen suite
  (a shared-runtime batch or precompiled `libcore`), and running the batched
  Kotlin programs in one JVM instead of one `kotlin` launch each. Past those the
  floor is the matrix size itself.
