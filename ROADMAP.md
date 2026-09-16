# Salvo Compiler — Roadmap

What is left to build, with the decisions and plans already made about each
item. The record of what is *already* built — and the reasoning that got it
there — is [COMPLETED.md](COMPLETED.md); this document assumes it and points
into it rather than repeating it. The two replaced PROGRESS.md (2026-09-09).

Companion documents: [LANGUAGE.md](LANGUAGE.md) is the narrative spec (source of
truth); [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md) states every feature as a labeled
rule (`[qual-erasure]` style) with the compiler decisions under it;
`BACKEND_SPEC.<backend>.md` ([kotlin](BACKEND_SPEC.kotlin.md),
[rust](BACKEND_SPEC.rust.md)) repeats rules with backend interpretation details
and adds backend-prefixed rules (`kt-…`, `rs-…`) — load it only when working on
that backend. Labels are referenced from compiler code and tests
(`grep -rn '[rule-name]'`); backend-prefixed labels may only be referenced from
that backend's crate. Keep all of these in sync when adding or changing
features. Detailed feature mechanics live in the specs; these two documents keep
the decision log, the plan, and the hard-won operational knowledge.

## How to read this

- **Items marked DECISION need a language-design call before implementation**,
  and that call is the user's: state the options, the trade-offs and a
  recommendation, then wait (AGENTS.md's first invariant). Everything else is
  engineering under decisions already made.
- **Standing constraints on every item here.** Unsupported constructs are
  *errors*, never wrong output ([backend-never-wrong]). The two backends must
  agree observably; a divergence is closed by restriction or by faithful
  emission, never by silently cloning mutable data (the backend-parity
  principle, in COMPLETED.md). Backwards compatibility is not a requirement, so
  a rule change is a sweep of every example rather than a shim.
- **Section names in quotes point into COMPLETED.md** unless the section is in
  this file. The two documents were one until 2026-09-09, so an "above" or
  "below" in moved text may mean the other document.
- **"The sequence" is the agreed order of work** (user decision 2026-09-09).
  Pick from the current phase; the themed sections below carry the detail and are
  tagged with the phase they belong to.
- **Before starting anything**: read COMPLETED.md's "Gotchas / lessons learned"
  for traps in the area you are touching, and its decision log for whether the
  question was already answered.

## Where we are

The arcs that are *complete*: shared fate and borrow emission (S1–S3), must-use
linearity with a designated `close` (L6, L7a–d), the effects arc through handler
dependencies, `throw`/`try` and effects on fn types (E1, E3 steps 1–3),
places and field narrowing (P1), deductions with refinements (D1, D3), dot-names
(N1), overload resolution (finalized), the std string and sequence surfaces
(S-Str, S-Seq), the iterator reduction to `next` (R0–R5 plus the generic
drive), the collections (S-Col) and **the filesystem (S-IO, phase 4 —
complete 2026-09-14, bytes and worked example included)**. What is left is
below, grouped by theme; **"The sequence" is the order it will be done in**,
and each themed section is tagged with the phase it belongs to. **Phase 5
(threading) is under way**: its design is settled (CONCURRENCY.md,
SUPERVISION.md — the decision space is empty), the
scheduler library is built, and the surface is landing in slices — actors
spawn, send, park continuations and bridge on both backends today, dependent
handlers included, request/response no longer needs `main` in the loop, a
actor's death is both watchable and — where a topology could deadlock —
reported before it runs, and a handler may park a queue of obligations in its
own state.
See "Threading and concurrency" for what remains.

## The sequence (user decision 2026-09-09)

Five phases, in this order. Each names what is in scope, the decisions that have
to be answered before it starts, and the smaller items that ride along with it
rather than being scheduled separately. Nothing outside a phase needs doing
first.

**1 — Finish the iterators.** ("Iterators", below.) **✅ Complete 2026-09-10**
— every item below landed, was answered, or was retired; what the phase leaves
behind is the recorded cuts under "Iterators", each a diagnostic. Phase 2 is
next.

- ✅ **A generic function over *containers*** landed 2026-09-10: `?iter` as an
  implicit whose result determines the pass type, which is the only way the shape
  can work for an `iter fn` subject (its pass has no name to write). See
  COMPLETED.md. **std stays pass-only** (user decision 2026-09-10): a source need
  not have a container behind it, so anything new in `std/` — S-IO's streams
  included — takes a pass and lets the caller write `iter(c)`.
- ✅ **`iter fn` with a `state { … }` block** landed 2026-09-09 — a hand-written
  `next` whose pass struct is generated, which is the answer to "the generated
  state machine is a lot of code": most producers need no machine at all. The
  generated pass holds only as much of the subject as the body reads (nothing, a
  snapshot per field, or the whole value). It also closed a
  `for`-over-a-container emission defect and lifted the effectful-`next` cut. See
  COMPLETED.md.
- ✅ **The qualifier-flattening defect** closed 2026-09-10: `emitted(ok("x"))`
  builds straight into an `Emitted (Ok Str | Err Str)` arm, which is what makes
  `Emitted (Ok T | Err E)` writable at all — phase 4 needs exactly that shape.
  It also closed a latent emission gap on the same path (a group over plain
  arms, `Emitted (Str | Int)`, emitted one wrap where two were needed). See
  COMPLETED.md.
- ✅ **"A suspending loop driving a pass" is retired, not built** (2026-09-10).
  The item described a `for` inside a `yield fn` body, driving a nested pass out
  of a machine slot — and `yield fn`, the planner it named and the slot
  machinery all went with the deletion earlier the same day. Nothing suspends
  any more: a `for` inside an `iter fn`'s `next` is an ordinary loop, and the
  capability the cut actually cost — *lazily* interleaving with an inner pass,
  as a flatten does — is written by holding that pass in `state`. Checking that
  found the defect below instead. See COMPLETED.md.
- ✅ **Mutation through a narrowed place on Rust** closed 2026-09-10 — the
  worst class of bug this repo has had: silently wrong output rather than a
  diagnostic. See COMPLETED.md.
- ✅ **The `?close` implicit is answered, not built** (2026-09-10): an
  early-stopping combinator already releases its source. `[iter-drive-in-place]`
  plus the `?Linear<It>` spread — option (a), chosen over R0's sketch — put the
  release on the type that owns the resource, and a `for` over a pass the fn
  owns releases it on every exit. Phase 4 therefore has what it needs to stop
  reading a file part-way. See COMPLETED.md.
- ✅ **The riding-along polish landed 2026-09-10**: emitted Kotlin carries
  `@Suppress("UNCHECKED_CAST")` where a generic payload read needs it
  ([kt-suppress-cast] — std's `seq.kt` compiles warning-free again), implicit
  resolution applies the overload scope ladder before declaring ambiguity
  (a program's own `ListYield` + `next` no longer collides with std's), and
  the stale `Iter<T>` compiler comments are gone — with a dead
  `once_position` arm (a latent bug for a user struct named `Iter`) removed
  along the way. See COMPLETED.md.
- ✅ **Mutable origins, option (e) is closed** (2026-09-10): the per-field
  snapshot already landed with `iter fn` `state` blocks, and the same-file
  restriction is now lifted — the CLI and LSP expand `iter fn`s program-wide,
  so a subject declared in another file snapshots per field too. Generic
  subjects and assignment-through-the-subject keep the whole-value fallback,
  as recorded. See COMPLETED.md.
- ✅ **`yield fn` is deleted** (user decision 2026-09-10): `iter fn` covers the
  same ground without a state machine, so the sugar, the `yield` keyword and
  the whole machine apparatus went — `generator.rs`, both backends' renderers,
  the per-`defer` flags, the origin mints and the Rust `iter.rs` runtime. See
  COMPLETED.md.
**2b — Copies only by opt-in.** **✅ Complete 2026-09-11** — `proj` everywhere,
views, `?copy`, the std audit, `iter fn` passes borrowing their subject, and
the `=>` deduction respelling; leftovers under "Projections and copies —
leftovers". Phase 3 is next.

**2 — Finish shared fate: places and partial moves.** ("Linear types → L5".)
**✅ Complete 2026-09-10** — both halves: field-disjoint poison
[fate-field-disjoint] and partial moves [fate-partial-move], both on the
`Place` substrate P1 built. Phase 3 is next.

**3 — Finish linearity: composition and conditionality.** **✅ Complete
2026-09-12** — decided and built in one day (all calls and the build record
in COMPLETED.md's decision log, "Phase 3 decided" and "Phase 3 built").
The lowercase obligation keywords, `linear struct` + the same-file
discharge model, linear union arms (the fallible open), no implicit
discharge sites (loop splicing deleted; the consuming-callback pattern +
std's `drop`), conditional containers with settle-by-decomposition,
`once` on any type (D6), the effect-member and fn-value instantiation
bans, and generic-fn-value instantiation from the expected type — all
with acceptance tests on both backends. Leftovers, recorded under
"Linear types" below: the generic wrapper-pass loop-emission gap, bare
generic struct literals not inferring type args, and intrinsic
containers (`List<linear T>`) deferred by decision.

**4 — The filesystem, on an IO stream design** ("Standard library surface →
S-IO"), using linearity and iterators — which is why it follows 1 and 3.
**✅ Complete 2026-09-15** — decided in six rounds of user decisions and built
across two days: the Has-accessor effect fusion, the operator-typing slice, the
`@Effect` grammar, O-R2 interception, the `platform handler` mechanism,
effect-member overloading, the [linear-group] member-discharger amendment,
then `core.fs` + `core.hostfs`, `MemFs`, `RestrictedFs`, the byte payload
(`Byte` unsigned on both backends, then **`Bytes`** as std's buffer
[bytes-type]), the fill-a-buffer reads and the copy one-shots [fs-read-to],
and `examples/files/`. FILE_SYSTEM.md, the plan of record, has **retired into
COMPLETED.md's decision log** as its charter said; the as-built rules live in
LANGUAGE_SPEC.md ([fs-surface] … [fs-v1-cuts], [byte-value], [bytes-type],
[fs-read-to]). **Phase 5 is next**, and nothing from phase 4 is left open.
Leftovers found on the way are under "S-IO" below.

**S-Col — collections rode before phase 4 and landed 2026-09-12**, the same
day it was decided: `Set`/`Map`/`SortedSet`/`SortedMap`, collection literals,
universal struct `==`, `canbe hashed`/`canbe ordered`, and the
`*_of`/`*_by`/`to_*` conventions. See COMPLETED.md. The in-memory test
filesystem (`MemFs`) was to be its first internal customer and now has the
collections it needs.

**5 — Threading: asynchronous effect handlers.** ("Threading and concurrency",
below.) Designed (user decisions 2026-09-14/15) and **being built**: the
scheduler library, the whole surface's syntax, its types and its checker rules
are in, and actors **run on both backends with identical output** —
including a **dependent** handler whose dependencies its spawn clause supplies,
a request/response chain that never passes through `main`, and (2026-09-16) a
**death watch** plus the **static deadlock baseline**.
What is left of the agreed eight-item sequence: **propagation** (item 8) —
the worked `examples/actors/`, the examples-file respelling, and the working
documents' retirement.
**The sugar pass leaves the phase** (user decision 2026-09-15;
"The sugar pass — after phase 5"): phase 5 ships the explicit surface, and
call syntax, `then`, `defer` and merge/join become later items with their own
decision surfaces. [fate-lambda] goes with them — no first-pass form crosses
a closure.

**Not before then: `Cell`.** It exists to make shared mutable state
expressible, and the OTP answer is that actors own their state and
message-pass — so phase 5 may remove its motivation entirely. Deciding it
earlier spends a language-design call twice.

**Not before then either: laziness** (user decision 2026-09-10). std's lazy pair
was **removed** the same day rather than carried through four phases as a design
constraint — see "Laziness, after concurrency" below for the direction to take
when it is picked up.

## Decisions waiting on the user

Every item here needs a language-design call before it can be built, and the
call is the user's (AGENTS.md's first invariant). They are listed in one place
because that is the question a session most often needs answered first; each
links to the section that states the options.

| Question | Due | Where |
|---|---|---|
| `Cell` — whether shared mutable state joins the language at all | after phase 5 | "Shared mutable state" |
| **D2** — `+Q` in a function's own deduction list (needs an establishment rule) | unscheduled | "Deductions and qualifier reasoning" |
| **D4** — predicate `is` on a union subject (needs qualifiers over unions) | unscheduled | "Deductions and qualifier reasoning" |
| **`size(Str)` outside ASCII** — what a `Str` index means (code points, UTF-16 units, bytes), then one lowering per backend | unscheduled | "Open defects" |
| **Recursive types** — the Rust boxing rule, regular-recursion-only, constructibility, depth semantics | unscheduled, end of the queue | "Recursive types" |

(**No phase-5 rows remain.** The four original DECISIONs, the
supervision/monitors story, and the three questions the build itself surfaced
— self-sends, sendability's content, and the sequencing of the effect
unification against the rest of the phase — were all decided 2026-09-14/15;
see "The sequence" phase 5, "Threading and concurrency", "The sugar pass"
below, and COMPLETED.md's decision log. Phase 5 is engineering from here.)

One further proposal is **deferred by decision** rather than waiting:
`platform type`. (`platform handler` was un-deferred 2026-09-14 and built the
same day — see "Effects"; the `defers`-block proposal went with `defer`
itself, 2026-09-10 — see "`defer` is deleted".)

## Open defects

Bugs found and reproduced, not yet fixed. Each carries a repro small enough to
paste and a root cause, so picking one up needs no re-investigation. Closed ones
move to COMPLETED.md with their repro intact.

**Five open.** (Closed in the sessions before this one, with repros and
root causes in COMPLETED.md: the retagged-lambda deref-in-cast miss (E0606)
and the adapter's silent clone of a returned projection; a tuple-array type
`(Str, Int)[]` misparsed as an effect list, an effect member hijacking a
same-named fn-typed local, and variadic args moving out of their caller's
locals. The broken `Int[n] { … }` array generator was closed by **deleting
the form** — user decision 2026-09-13. Mixing a plain argument with a
`...spread` in one variadic call is now **supported** rather than refused,
which also let std's `non_empty_list` go back to being ordinary Salvo. A
user-declared variadic of a *primitive* element type no longer breaks on
Kotlin: a variadic parameter is an ordinary `Array<T>` there, not a `vararg`
[kt-variadic]. **Closed 2026-09-15**: an effect member whose name is also a
std fn — one loud half inside std's own emission and one *silently wrong
output* half through `@module` — and consuming a handler's stored values,
which Rust silently cloned and Kotlin shared. Both in COMPLETED.md.)

- **The Kotlin case driver costs ~17s on every run, cached or skipped**
  (found 2026-09-15 while adding the scheduler runtime tests; pre-existing,
  and the reason a warm `cargo test` is ~30s against AGENTS.md's ~5s
  budget). Repro:

  ```bash
  # identical timings, three ways — the cache and the skip both no-op:
  cargo test -p salvo-backend-kotlin --test codegen_tests            # 18.3s
  cargo test -p salvo-backend-kotlin --test codegen_tests            # 18.2s
  SALVO_SKIP_E2E=1 cargo test -p salvo-backend-kotlin --test codegen_tests  # 17.7s
  cargo nextest run -p salvo-backend-kotlin --test codegen_tests
  #   PASS [ 16.827s] kotlinc_compiles_and_runs_every_case
  ```

  Root cause: `kotlinc_compiles_and_runs_every_case` builds **every**
  `KOTLIN_CASES` entry up front (`cases = KOTLIN_CASES.iter().map(|f| f())`),
  and each case function runs the whole pipeline — parse + check + emit over
  `std` — before any gate is consulted. So the cost is paid whether or not
  `kotlinc` is available, whether or not the stamps hit, and whether or not
  `SALVO_SKIP_E2E` is set. It is *not* toolchain time: the 17s survives
  skipping.

  Two fixes, in increasing order of value: (1) consult the availability /
  skip gate **before** building the cases — restores `SALVO_SKIP_E2E`'s
  documented ~4s and costs three lines; (2) make the stamp key cheap enough
  to check without emitting (hash the `.sv` source plus an emitter-version
  token instead of the generated files), which is what would restore the warm
  budget. (2) is a testkit design change and wants its own think: the current
  key's virtue is that it cannot go stale, and a source-plus-version key
  trades that for speed. The Rust backend's per-test runner does not have the
  problem (its cases build one at a time, inside their own gate).

- **A whole-valued `Double`/`Float` interpolates differently per backend**
  (found 2026-09-14 while testing the operator slice; pre-existing). Repro:

  ```
  fn main() [use] -> None {
      use StdOutConsole
      let d = 2.0
      println("${d}")     // Rust: "2"   Kotlin: "2.0"
  }
  ```

  Root cause: Rust's `Display` for `f64` drops the trailing `.0` where
  Kotlin's `toString` keeps it; fractional values agree (`8.25` both). A
  [backend-never-wrong]-grade parity break in the printed *text*. The fix
  belongs in the interpolation lowering [interp-to-str]: a Salvo-emitted
  float formatter (or a `format!("{:?}")`-shaped rendering on Rust, which
  keeps the `.0`) — decide once, test with whole, fractional, negative-zero
  and very large values on both backends.

- **A container operation inside a *generic* function is a backend
  divergence** (reproduced 2026-09-13). Repro:

  ```
  fn collect_one<T>(elem: T) [] -> Set<T> => !elem {
      let s: Mut Set<T> = mut_set_of()
      add(s, elem)
      return s
  }
  ```

  Kotlin compiles and runs it (erased generics, `LinkedHashSet` takes
  anything); Rust fails with a raw rustc **E0599** — "the method `insert`
  exists for struct `SalvoSet<T>`, but its trait bounds were not satisfied" —
  because the emitted signature carries only the bounds Salvo knows
  (`T: Clone`), while `insert` needs `T: Hash + Eq`. No Salvo diagnostic, so
  this is a [backend-never-wrong] violation.
  Root cause: **there is no way to write the bound.** Key eligibility
  [col-key-eligible] is checked where a container type is *instantiated*, and
  a type parameter is deliberately allowed through there ("checked at the
  instantiation") — but a generic fn body is a use site with no instantiation
  in sight, and a type parameter accepts only `canbe linear`
  ("only `linear` is supported in a type-parameter `with` clause").
  The fix is the same mechanism `sort`/`binary_search`/`add_sorted` need — see
  the C-6 decision under "Standard library surface" — and it cuts both ways:
  the checker could refuse the unbounded body with a Salvo error, and the Rust
  emitter could put `T: Hash + Eq` (or `T: Ord`) on the signature and make the
  program work. Until then, container operations belong in non-generic code.

- **A recursive struct is an undiagnosed backend divergence** (reproduced
  2026-09-12). Repro: `struct Node { value: Int, next: Node | None }` plus
  any use. The checker accepts it on both backends; Kotlin compiles and runs
  (`data class` fields are references); Rust emits `pub next: Option<Node>`
  and fails downstream with rustc E0072 — a raw target-compiler error, no
  Salvo diagnostic. Same through a union arm (`type Tree = Int | Branch`).
  Root cause: no cycle check anywhere over the type graph — nothing ever
  admitted or refused recursive types. Fix: the SCC walk + declaration-site
  diagnostic described under "Recursive types" step 1 (`List`/array edges
  are not cycle edges — recursion through `List<T>` works end to end today
  and stays legal). The full feature is separate and unscheduled; the
  diagnostic is owed regardless.

- **A bare inline collection literal does not determine a callee's type
  parameter** (reproduced 2026-09-12). Repro: `to_set([1, 2])` reports
  "cannot infer type argument `T`"; binding it first (`nums = [1, 2]` then
  `to_set(nums)`) works, as does annotating. Root cause: argument type
  inference runs before the literal is given an expected type, so the
  literal's element type is still unknown when the substitution is solved —
  the same shape as the recorded "bare generic struct literals not inferring
  type args" leftover under "Linear types", and probably one fix.

## Linear types

**Phases 2 and 3.** The arc's completed stages, the backend-parity principle they rest on, and the
`Place` substrate are in COMPLETED.md ("Roadmap: toward full linear types"
and "Roadmap: place-based flow analysis"). What is left:

### L5 — Places and partial moves (phase 2) — ✅ complete 2026-09-10

Both halves are built. **Poison** [fate-field-disjoint]: a fate link carries
the projection path out of its root, an event carries the path it hit, and
poison fires only where the two overlap. **Moves** [fate-partial-move]: a move
of a projection records it as moved out of its root rather than consuming the
whole variable, so a disjoint field stays readable, reading the moved field
back is an error naming it, a whole-value use is refused, and reassigning the
place revives it. Both rest on `Place::overlaps` — the relation P1 built for
narrowing — which is why the phase cost a fraction of the "place lattice
instead of per-variable states" it was budgeted as. See COMPLETED.md for what
it took, the evidence that forced it, and the Rust alignment.

Settled along the way, and worth not re-opening: **partial moves need no
signature notation**. Rust's model is that ownership is all-or-nothing per
parameter — a borrowed parameter refuses a move out of it (E0507), an owned one
may be partially moved because the caller already surrendered the whole value
— so the state is function-local. Salvo already implemented exactly this, so
the binary kept/moved deduction stands. `proj[from: p]` is a different
axis (the **return** channel, where the borrowed place escapes), and a
place-parameterized deduction (`[p: -tags]`) is deliberately **not** wanted: it
is viral, and passing the field rather than the struct (`f(p.tags)`) says the
same thing with no notation.

**Not L5: field smart-casting.** Place-based *type narrowing* (reads of
`h.field` narrowed by `h.field is T`) is a separate feature from place-based
ownership — it was roadmap phase **P1**, done 2026-09-03, and it is the
substrate both halves of L5 stand on.

### L7 remainders — recorded, none forced

The L7 milestone landed in four pieces (L7a `<T canbe linear>`, L7b `once` fn
types, L7c derived returns, L7d fn-type contracts). What it recorded and did not
build:

- **The internal qualifier unification** (phase 2 of the parameterized-qualifier
  design: folding links/poison/`consumed_by` into parameterized qualifiers on
  the narrowed type, with per-qualifier join directions). Unforced — L7c and L7d
  landed on links alone. The dividend would be diagnostics carrying LSP
  related-information spans from the qualifier's parameters.
- **`once` inference**: a callee that calls its fn-typed parameter at most once
  does not auto-promote it to `once`; written only [once-fn]. The same
  written-validates / unwritten-infers pattern deductions use, when it is taken.
- **Per-parameter written contracts beyond kept/moved/quals** on fn types
  [fn-contract].
- **Accumulator bodies in a derived-return fn** (`best = person; …; return best`)
  are rejected by the provenance validation: the recorded refinement is
  reassignable borrowed locals [readonly-return].

### The linear instantiation ban misses generic effect members (phase 3)

`[linear-generics]` refuses instantiating an unconstrained type parameter with a
linear type — checked in `resolve_named_call` on the resolved substitution — but
an **effect member's own generics** are not covered, so a linear value can be
smuggled through one. A hole in what already shipped rather than a missing
feature, and it belongs with phase 3 because that is when the obligation rules
are being reopened anyway.

### L8 — Composition and conditional linearity — ✅ complete 2026-09-12

Answered together with D6 and D7 and built the same day (phase 3; the
decision set and build record are in COMPLETED.md's log). Union arms carry
obligations and settle by narrowing [linear-union-arm]; conditional
containers are `struct Box<T canbe linear>` with settle-by-decomposition,
and concrete linear fields take the `linear struct` marker
[linear-generics] [linear-composite]; the wrapper pass (lazy `take` over a
linear source) runs on both backends. What the phase leaves open:

- **Generic wrapper-pass loop emission.** A `for` in a *caller* over a
  generic conditional pass (`Take<It>` instantiated at the call) does not
  fill the pass's `next` implicit at the loop's emission — rustc E0061
  (`next__5(&mut t)` missing the callback argument). Checker-clean;
  refused only by the target compiler, so loud, never wrong. Repro: the
  step-6 generic wrapper probe; the concrete-wrapper e2e
  (`wrapper-pass`) covers the semantics meanwhile.
- **Bare generic struct literals do not infer type arguments**:
  `Box { item: open_lines(n) }` needs `Box<Lines> { … }` written.
  Pre-existing; surfaced by the conditional-container work.
- ✅ **Intrinsic containers** — decided 2026-09-15 and **built 2026-09-16**
  (phase 5 item 7): `List`/`Map` values opt in with `canbe linear` on the type
  declaration, take-by-move answers `T?`, `drain(container, each)` is the
  terminal, `Set`/keys are refused because dedup is dropping, and handler state
  owns obligations across activations [linear-container] [linear-state]. What
  it left open is under phase 5 item 7 in "The sequence".


## Effects

E1 (handler dependencies, and the Rust fusion behind them), E3 steps 1–3
(`throw`/`try`, effects on fn types — and `defer`, since deleted) and the six ownership strategies
explored on the way are in COMPLETED.md ("Roadmap: effects"). What is left:

### E3 step 4 — effect transformers, and async as the second one

The general prize, and the last rung of the handler-control arc. A
**transformer** is an effect member that runs a fn-typed parameter with
*additional* effects available — its body having registered handlers for them.
`Retry`, `Timeout` and async are all that shape, and `try` could be re-expressed
as a library transformer if it reads better than the intrinsic.

- **The gate is already built.** Step 3 made fn-type effect lists real and
  threads effects *into* a fn value instead of capturing them [fn-effects], so
  the mechanism a transformer needs exists. What remains is the surface.
- **No silent colouring** (user decision 2026-09-04): the ability to not resume
  is declared on the effect member, never discovered from the handler — which is
  forced anyway by a user fn being emitted once whatever handlers flow in.
- **Async is explicitly not part of this arc**: it arrives later as an explicit
  effect, likely a compiler intrinsic, not as an `async`/`suspend` transform of
  the whole program.
- **Multi-shot resumption is closed on principle**, not for want of a mechanism:
  resuming twice duplicates a use obligation, so it cannot coexist with `Linear`.
  (Neither target offers it either — a Kotlin `Continuation` throws on a second
  resume, a Rust `Future` cannot be cloned.)

### Two effects sharing a member name — ✅ built 2026-09-14

Decided during the phase-4 rounds and built the same day (COMPLETED.md
decision log): member names recur across effects [effect-member-overload],
bare calls resolve by availability, and `member@Effect(args)` picks
explicitly [effect-at] — `close@Fs(h)`, `h.close@Net()`, instance pinned by
the call's type arguments (`next_random@Random<Int>()`). One leftover,
deliberate: the LSP def-site table is name-keyed, so go-to-definition on a
*shared* member name lands on one declaration (last collected) — worth a
keyed table if it ever grates.

### `platform type` — still deferred; `platform handler` shipped 2026-09-14

Both were recorded when the interop redesign landed (user decision
2026-09-05): a `platform handler` is a host implementation of an *ordinary*
Salvo effect, constructed by `use`; a `platform type` is the type-aliasing
gap the user accepted, with platform structs sketched as the eventual answer
("let's leave platform type until a need arises"). `platform handler` was
un-deferred when the need arose (`HostRawFs of RawFs`, FS-1 resolved as O-M2)
and **built 2026-09-14** — see COMPLETED.md's decision log and
[platform-handler]. `platform type` remains deferred; the two `platform`
declarations are the interop surface until it is picked up.

### A handler cannot dispatch to itself — recorded gap (found 2026-09-14)

A handler member may not call **another member of its own effect**. Repro:

```
handler Twice of Counter {
    fn bump() -> Int {
        return bump() + bump()      // error: a handler member cannot call
    }                               // `bump`, a member of `Counter` — the
}                                   // effect its own handler implements
```

Root cause, and why it is a design question rather than a bug: inside a
handler member the effect environment is the handler's *dependencies*
[effect-handler-deps], and the bare member name is already taken — declaring
the effect it implements means the handler registered **before** this one
[effect-intercept]. So self-dispatch needs a *different* spelling, and that
spelling is a language call (`self.bump()`, `bump@self()`, …). Neither remedy
the general "no handler" diagnostic used to name is available in a member
either (a member may not declare effects, and may not `use`), so the
diagnostic now says what is actually wrong and names the workaround.

**The workaround is a function**: move the shared logic into an ordinary fn
that both members call, passing what it needs. That is what std does today.

It bit in phase 4 twice and the workaround held both times: `MemFs` shares
logic between its members through free fns in its own module
(`mem_find_newline`, `mem_append`, and — once the `read_to` family arrived and
each fill member wanted its returning sibling — `mem_read_line`,
`mem_read_all`, `mem_read_bytes`). It reads fine and cost nothing but a
parameter list, so this stays a **DECISION** without a deadline: the spelling, and whether a self-call
may be recursive at all (a member calling itself is unbounded recursion the
checker would not diagnose).

### Two cuts inside the effect fusion

Both are *reported* rather than mis-emitted [rs-effect-fusion]:

- a dependent handler using its own generic parameters in a member signature;
- a `use` whose effect instance is still generic.

Each needs the fusion to derive type arguments it does not derive today. They
used to be **divergences** (Kotlin accepted them, because generics erase);
since 2026-09-14 Kotlin refuses a still-generic fused effect set too, in its
own words — a fused class has no type parameters to spell a `Store<T>`
property with, and before that the leak surfaced as a kotlinc "unresolved
reference 'T'" [kt-effect-fusion].

## Iterators

**Phase 1 — ✅ complete 2026-09-10.** The reduction to `next` is complete (R0–R5, plus `for` over a generic pass and
the generic-effect lift); COMPLETED.md holds the phase notes, the two prototypes
that shaped them, and the `Iter<T>` design they replaced. What remains recorded
here is not work but the accepted cuts:

### Known cuts, each reported rather than mis-emitted

The list the flip left behind ("Known cuts and gaps" under "R5 part 2 as built"
in COMPLETED.md). Every one of them is a diagnostic today, so none can produce
wrong output [backend-never-wrong]:

- **A callback handed *onward*** to another storing fn stays borrowed on Rust —
  the "does this fn store its callback?" predicate is deliberately
  non-transitive — and rustc reports the lifetime.
- **A *generic* `next` that performs effects** cannot be driven by `for` on
  either backend: its effects live on a fn value the caller supplied, so the
  handlers would have to reach through the implicit rather than being threaded
  per turn. The non-generic case works as of 2026-09-09.
- **A linear pass cannot be composed** — the L8 casualty above, repeated here
  because it is the shape people will try: a wrapper pass over
  `open_lines("a.txt")` stores its source, and storing a linear value in a
  composite is the interim refusal.

(Two entries left this list 2026-09-10, fixed rather than cut: the
same-name-pass implicit collision, and the unchecked-cast warnings in
generated Kotlin. See COMPLETED.md.)

Two open *questions* the iterator work forwarded to the qualifier roadmap
rather than answering: **D6** (`once` on any type — the I2b collision forces it
in a narrower form; see COMPLETED.md) and **D7** (qualifier-conditional
linearity). Both are under "Deductions and qualifier reasoning" below.

### An early-stopping combinator closing its source — answered, not built

Recorded when R5 shipped, **closed 2026-09-10**: the `?close` implicit R0
sketched (option (d)) was never needed. `[iter-drive-in-place]` plus the
`?Linear<It>` spread — option (a), the design chosen instead — put the release on
the *type that owns the resource* rather than on every combinator: a `for` over a
pass the function **owns** releases it on every exit, `break` and `return` alike,
through the resolved `close` for a concrete pass or the implicit one for a
generic pass. Verified on both backends for all four shapes; a hand-written
`while` driver, the one shape no loop can help with, is caught by linearity
("`h` still owns a linear value when it goes out of scope"). See COMPLETED.md.

What the item pointed at that *is* still open is a **lazy** `take` — one that
returns a wrapper pass instead of draining — and it belongs to L8, not here: a
wrapper has to store its source, and storing a linear value in a composite is
the interim refusal [linear-composite].

### `defer` is deleted, and the `defers`-block proposal with it (user, 2026-09-10)

`defer` is **gone from the language**: it was the partial solution to a problem
linearity already solves in full — an obligation discharged on every path — and
it carried its own complexity (a body checked once but applied at every exit,
the facts it relied on having to survive to each of them, a rule against
control flow or a throw leaving it, and two unrelated lowerings). Releasing is
now written on each path, and the checker names the path you missed. See
COMPLETED.md for what the removal took out and what it cost.

With it goes the **`defers { … }` block proposal** deferred on 2026-09-08 (a
`Defer` effect, cleanup registered into a *caller's* scope, all of it running at
the end of a named block). Its "for" case — cleanup visible in signatures, and
registering cleanup on someone else's scope — is not lost, but it is now a
proposal to *add* a feature rather than to generalize one, so it starts from
scratch if a customer appears. The strongest argument against it stands and is
worth keeping: what made `defer` cheap was having **zero runtime
representation**, and a dynamic queue costs an allocation and brings the capture
question back.

**What still uses the machinery underneath.** Both emitters keep their
exit-splice path — Rust splices at each exit [rs-exit-splice], Kotlin wraps in
`try`/`finally` [kt-exit-finally] — because the release a `for` owes a pass it
owns needs exactly that. It is now compiler-internal rather than a language
feature, which is the right side of the line: the compiler owns the value, so it
may own its lifetime.

## Shared mutable state (`Cell`)

**Deliberately not before phase 5** (user decision 2026-09-09): the OTP model has
actors own their state and message-pass, so it may remove this item's
motivation entirely — and deciding it earlier would spend the same
language-design call twice.

An idea developed 2026-09-03 while looking for a way to keep *immutable*
effects testable (a recording double needs state). It stands on its own
merits and is **not** tied to that use case — most of the patterns below
have nothing to do with effects. Open **DECISION**.

### The problem it addresses (and what it unlocks)

Salvo's mutation rule is about the **handle**: you may mutate through a
path only if that path is `Mut`, which is exclusive. Several ordinary
patterns need the opposite — mutation through a *shared* path:

- two lambdas appending to one accumulator (today the first one to mutate
  a capture *consumes* it, so the second is an error and the original is
  dead afterwards — verified: "`total` cannot be used here: it was
  consumed (moved) by a lambda that captures and mutates it");
- memoization / lazy initialization behind an immutable handle;
- counters, metrics, id generators shared by several holders;
- a stateful handler of an effect whose other handlers want to be shared;
- **a producer writing to a collection its caller keeps** — refused outright
  since 2026-09-08 [iter-mut-param], and the case with the sharpest
  requirements of the five (see "Producers: the `Mut`-parameter case (option
  C)" below).

### The proposal: a capability qualifier, not a container type

`Cell` joins the intrinsic capability qualifiers (`Mut`, `Linear`,
`once`, `proj`) rather than arriving as a std generic type
`Cell<T>`. The family fits exactly — each intrinsic qualifier exists
because it needs "a representation choice, a flow rule, a subtyping
direction or a restricted position that no user declaration could
supply", and `Cell` needs the first three:

- `Mut T` — mutation permitted, only through *this* handle.
- `Cell T` — mutation permitted through *any* handle.

**Benefits over a container type:**

- **No wrapper noise.** `count = count + 1` and `if count > 3`, rather
  than `set(count, get(count) + 1)` and `if get(count) > 3`. Reads and
  assignments keep ordinary syntax; only the *permission* differs.
- **It inherits machinery instead of adding surface**: `canbe Cell`
  opt-in on declarations, qualifier erasure, overload selection, and
  D1's stripping rule — where `Cell`, being a capability rather than a
  claim about contents, is never stripped (like `Mut` and provenance).
- **Family membership is the documentation.** "Capability qualifiers say
  what you may do with a handle" already exists as a concept; a std
  container with its own API is a second thing to learn.
- Danger stays visible in the type either way: `Cell Int` at every use
  site, greppable, opt-in — unlike interior mutability hidden inside an
  ordinary type.

### Representation, and why it cannot panic

- **Copyable contents → Rust `Cell<T>`**: `get`/`set` only, no borrow
  guard exists, so no runtime check and no panic is *representable*
  (verified: a shared id generator, `ids: 1 2 3`).
- **Collections → Rust `RefCell<T>`**: a guard exists, but the only
  operations are std primitives whose define templates the compiler
  controls (`${list}.push(${value})`), so no Salvo code ever runs inside
  the borrow (verified: a recording double shared by a capturing logger
  *and* used directly, all three writes recorded).
- Kotlin: a plain mutable field. No parity gap — both backends accept the
  same programs.

**The rule that keeps this true:** a cell may be mutated by assignment
and by *standard-library* primitives, but never lent to a user-defined
`Mut` parameter. Handing `&mut` into user code is what puts a borrow
guard around a user call, which is precisely where B2's reentrancy panics
came from (see E1a). One sentence to teach: "you can mutate a cell; you
cannot hand its insides to a function you wrote."

### Rules it drags in (the real design work)

- **No state qualifiers on cell contents.** A claim like `NonEmpty` is
  about contents, and contents can change through a handle the compiler
  is not looking at — so `qualifier … of Cell …` must be rejected for
  *state* claims. Provenance claims are fine (they are about where the
  handle came from). This is the one genuine soundness rule, enforced at
  the declaration.
- **No shared-fate links.** Reads copy rather than lend, so
  `let v = count` is an independent value: no link, no poison. Simpler
  than the field case, and a direct consequence of "no handle into the
  contents".
- **Linear contents need `replace`.** Overwriting a cell holding a
  `canbe linear` value would silently drop an obligation; `replace(cell,
  v) -> T` hands the old value back and transfers the obligation, while
  plain assignment over linear contents stays an error.
- **Deductions say nothing.** A fn taking a `Cell` and writing it needs
  no `Mut`, so its signature cannot report the write — acceptable only
  because the first rule leaves no claim worth preserving.

### The concession being accepted

`Cell` is a sanctioned hole in "no hidden shared mutable state": two
holders can surprise each other, and the ownership analysis stops helping
inside a cell. That is the price of shared mutable state in any language
with an ownership discipline; what makes it defensible is that it is
visible in the type rather than hidden behind an ordinary one.

### Producers: the `Mut`-parameter case (option C)

Added 2026-09-08 with the decision that refused it for now
([iter-mut-param]; the divergence that forced that is in COMPLETED.md, under
"Defects found and closed").
A producer taking `sink: Mut List<Int>` and appending to it as it yields is
the pattern, and it is the most demanding customer this roadmap has:

- **The handle outlives the call.** A producer's parameters are captured by a
  *factory* that may mint a pass at any later time, so this is not "two
  holders in one scope" — it is a handle stored for an unbounded period, which
  is what makes `Rc<RefCell<…>>` (rather than a scoped `&mut`) the only Rust
  shape that works. Every other case on the list above is at least *nameable*
  within one scope.
- **It multiplies.** A factory mints many passes, each capturing the same
  cell, and they can be alive at once (`zip(p, p)`). So the borrow discipline
  has to hold between *passes*, not just between a producer and its caller —
  and that is exactly where a `RefCell` would panic at run time, the outcome
  "Representation, and why it cannot panic" is written to avoid.
- **It makes replayability a question rather than a promise.** `Iter<T>` is a
  factory whose contract is that a second `for` starts from the beginning
  [iter-protocol]. Sharing a cell with the caller keeps the *elements*
  replayable while the side effect accumulates, so two loops over one factory
  stop being interchangeable. Whether that is acceptable is a language call,
  not a representation detail.

**If `Cell` lands, this is the acceptance case to run first**, because it
exercises the two hard parts together: a cell captured for longer than any
scope, and several live holders derived from one capture. The narrower version
— a producer returning `once Iter<T>`, where exactly one pass exists — needs no
`Cell` at all and may be answerable with **shared fate** ([fate-link]) once a
pass *is* the state machine (roadmap I2c); that is recorded under "Producer
parameters: `Mut` refused" and is the cheaper thing to try first.

### Relationship to E1

`Cell` is **not load-bearing** for effects. E1's chosen strategy (B9
handler fusion) keeps handler members able to mutate dependencies they
receive as parameters, so recording test doubles need no interior
mutability and effects need no immutable/mutable distinction. `Cell`
therefore stands on the lambda/memoization/counter cases, which are real
limitations today, and can land independently of the effects work if it is
wanted at all.

## Deductions and qualifier reasoning

D1 (exhaustive and delta deductions) and D3 (refinements) are built; D5
(qualifier subjects) and D8 (a producer's effects) were decided and landed. See
COMPLETED.md. What is left:

### D2 — Qualifier asserts (`+Q`)

Still deferred for a *function's own* deduction list (user decision
2026-09-02: not even in the grammar there — the parser reports "adding
qualifiers in a deduction (`+Qual`) is not supported yet" and names this
item). `+Q` asserts that the body *establishes* `Q`, which is what
`-> T as Q` does for return values — the parameter-position analogue. A
predicate qualifier cannot be proven statically (that means reasoning
about the algorithm), so establishment is trust (like `as Q`) or a runtime
`qualifies` check.

**D3 gave `+Q` exactly one home, and it is not this one** (2026-09-06):
inside a `refn` [qual-refn], where it is the *qualifier author's* claim
about someone else's call rather than a claim about your own body. That
distinction is what kept D2 deferred through D3's implementation, and it
is also why an inferred deduction may only re-establish a qualifier the
parameter *declares* [qual-refn-infer] — letting inference add a new one
would have implemented D2 by the back door, without ever deciding its
establishment rule.

- **DECISION D2a** — whether `+Q` and `as Q` unify into one notion of
  "this function establishes a qualifier", and whether establishment is
  trusted, runtime-checked, or restricted to fns declared in the
  qualifier's own file (as `as Q` is today). D3's answer for refinements
  was *trusted*, which is the precedent but not the decision: a
  refinement is written by the party that owns the claim's meaning, and a
  function asserting `+Q` about its own body is not.

### D4 — Predicate `is` on union subjects (and qualifiers over unions)

Motivated by an analysis of the `is` keyword (2026-09-03): `is` has one
grammar and two evidence sources — union-arm identity, statically known
[is-narrowing], and a runtime `qualifies` call [is-qualifies]. There is
no parse ambiguity (one `Expr::Is` node; both forms are
`subject is Qual* [Type] [binding]`), but `is_info` picks between them
by the *subject's shape*: a `Ty::Union` subject **always** takes the
arm-matching path. Consequence: a predicate qualifier can never be
tested against a union-typed value. With `let x: Int | Str`,
`x is Positive` matches no arm and reports "this check can never
succeed"; the workaround is to narrow first (`x is Int && x is Positive`
works, because the second test sees a non-union subject). The predicate
form is shadowed by the union form, and the shadow is invisible in the
surface syntax.

Fixing it is not a checker patch — the narrowed type it should produce
is a union whose arms carry a qualifier, so it needs qualifiers over
unions in general (today [qual-union-arm] binds a qualifier to a single
arm, and only an explicitly parenthesized group can be qualified
[qual-group]).

- **DECISION D4a — semantics of `x is Q` on a union subject.** Which
  arms participate (recommendation: those whose qualifier-stripped type
  satisfies `Q`'s `of` type), and what the check *is*: a conjunction of
  the arm/tag test and the `qualifies` call (recommended — it is what
  the two-step workaround does today), or `qualifies` alone.
  Then-type: the participating arms with `Q` added.
- **DECISION D4b — the else branch.** A failed predicate proves nothing
  ([is-qualifies] already records "no else information"), so the
  remaining set must *keep* the participating arms — unlike a pure arm
  test, which subtracts them. That asymmetry is the load-bearing
  difference and it propagates: a `when` whose arms are predicate checks
  can never be exhaustive. Decide whether predicate checks are allowed
  in `when` arms at all (recommendation: allow only when the arms are
  exhaustive on tags alone, otherwise reject with a message pointing at
  `if`/`elif`).
- **D4c — qualifiers over unions.** Decide the shape of the narrowed
  type: per-arm `Q A | Q B` (recommended — preserves [qual-union-arm]
  and existing arm identity) versus a qualified group `Q (A | B)`
  ([qual-group], which changes wrapper identity). Per-arm keeps the
  positional-arm invariant that the checker and both emitters share.
- **D4d — mixed checks.** `x is Positive Int` on a union: base-type arm
  test *plus* the `qualifies` call, one lowering.
- **Backend work.** `is_tests` and `predicate_tests` are separate side
  tables and each emitter lowers one of them; a union subject with a
  predicate needs a *combined* lowering (tag test `&&` qualifies call)
  in both, and checker and emitters must agree exactly, per the
  invariant. Effects on the `qualifies` fn stay subject to
  [is-qualifies-effects] at every such site.
- Sequencing: independent of D1–D3, but it shares the "what does a
  qualifier mean over a composite type" question with D3's refinements;
  do D4c's decision before either.
- Not in scope: renaming `is`. The two readings are opposites in
  *feel* — "already attached" versus "may be attached" — but both are
  "test whether this holds now, and refine if it does"; conferring a
  qualifier is what `-> T as Q` does [qual-ctor-fn]. User decision
  2026-09-03: keep one `is`, revisit only if D4's rules prove confusing
  in practice.

### D6 — `once` on any type — ✅ complete 2026-09-12

Decided yes and built with phase 3 (COMPLETED.md's log): the position gate
is gone, `once` is valid on any type, and on data it drops at consuming
positions (`once T <: T` for non-fn bases — a plain-typed holder consumes
at most once anyway) while fn types keep the strict never-drop rule
[once-fn]. The `once`-inference residual (auto-promoting a callee that
calls its fn param once) stays not-built, as recorded.

### D7 — Qualifier-conditional linearity — ✅ closed 2026-09-12, no surface

Decided with phase 3: covered by conditional containers ([linear-generics]
— the type-argument axis is the re-derivation of the need), no use-site
`linear` spelling; revisit only if a customer appears that a container
cannot express.


### Related, independent: `!is` in expressions

Negated checks (`if x !is Str { … }`) — sugar over `!(x is Str)` with the
same fact propagation, and no binding form (a failed test binds nothing:
`x !is T name` is a parse error). Lexing (user decision 2026-09-02): `!`
binds *adjacently* — `x!` (assert) requires no space before `!`, `!is`
requires no space between, `!x` (not) requires no space after, and a
floating `!` (space on both sides) is a syntax error.

The disambiguating clause, refining "not preceded *and* followed by
alphanumerics": the left neighbour must count `)` and `]` as
value-endings, or `names.first()!is Str` stays ambiguous — and
`first()!` is real, common code (the M2 demo interpolates
`${names.first()!}`). So:

> `!` may not be *directly* preceded by a value-ending token
> (identifier, literal, `)`, `]`) **and** directly followed by an
> operand-starting token (identifier, literal, `(`, `[`) or the keyword
> `is`. Whitespace on one side resolves it.

This keeps `x!.field`, `x!)`, `x!,` legal (the following token cannot
start an operand), rejects `x!is T` and `a!b`, and leaves `x! is T`
(assert then test) and `x !is T` (negated test) as the two spellings.

## Projections and copies — leftovers (phase 2b complete 2026-09-11)

Phase 2b — `proj` everywhere, views, `?copy`, the std audit, the `iter fn`
borrow, and the `=>` deduction respelling — is **built**, and so are the
follow-ons: **`proj` as a type qualifier landed 2026-09-12** (option A),
and **capturing lambdas are views + written lend entries take precedence
over the instantiation fallback landed the same day** (user decision; both
in COMPLETED.md's decision log with the soundness hole and the two emitter
defects they closed). The rules live in LANGUAGE_SPEC.md ([proj-type]
[lambda-view] [proj-anywhere] [proj-readonly] [proj-field] [proj-infer]
[yield-proj] [copy-implicit] [copy-scalar-free] [deduce-syntax]) and
BACKEND_SPEC.rust.md ([rs-proj]). What stays open:

- **Type-mention tracing for instantiation links (refinement, recorded
  2026-09-12).** With no written entry, a `proj`-holding instantiation
  still links its result to *every* kept argument — including a container
  the substituted type never mentions (`keep_all(iter(words), tags)` links
  to `tags`). The remedy today is the written `=> proj[from: it]` entry,
  which now takes precedence; the refinement would link only kept
  arguments whose substituted type contains the projection. Build it if
  over-linking bites where the entry is unavailable (an unannotatable
  callee).
- **Contract stability (revisit).** With unmentioned parameters inferred
  [deduce-syntax], editing a body to consume or lend a parameter changes what
  callers may do with no signature change; the old exhaustive list made moves
  visible at a glance. Accepted for now (the hover shows the effective clause;
  the diagnostics name the cause). If it bites, the remedy is an opt-in
  "exhaustive" marker or a lint that asks for `!p` to be written.
- **Re-pointing entries are declared, not verified.** `=> v.items: proj[from:
  other]` links the caller's `v` to `other` at the call, and the emitter ties
  the lifetimes; the *body* is trusted to perform the re-pointing. The
  inference in `lends.rs` is return-centric — extending it to assignments
  into a parameter's `proj` fields is the verification.
- **Binding a view of a temporary (user, 2026-09-11 — refuse for now).** A
  derived-return or lending call whose borrowed argument is a temporary may be
  used within its statement — `map(iter(list(4, 5)), f)`, `for x in
  iter(list(1, 2))` — but binding, returning or storing the view is an error
  ("cannot bind a view of a temporary … bind that argument with `let`
  first"). Rust exposed it (E0716); Kotlin would have run it. **Possible later
  automation**: hoist the temporary into a fresh local that lives as long as
  the view (`let __src = list(1, 2); let p = slice(__src)`) — exactly what the
  user writes today. Not free of judgement: the temporary then lives to the
  end of the block (observable only through destructors/regions, which Salvo
  does not yet have), and a hoist inside a loop or lambda changes how often
  it is built. Revisit once regions land.
- **Reserved extension — link parameters (user sketch, 2026-09-11).** If the
  conservative fallback of [proj-infer] ever bites (two sources, read
  separately, through a fn value or effect member), the per-field explicit
  form is a lowercase name in the generics list — `[name-casing]` already
  makes it parse:
  ```
  struct Pair<T, U, a, b> { first: proj[from: a] T, second: proj[from: b] U }
  fn get_pair<T, U>(ts: List<T>, us: List<U>, i: Int) -> Pair<T, U, ts, us> => ts, us
  ```
  with Rust emitting one lifetime per link parameter (today every `proj`
  field shares one `'s`, which rustc unifies to the shortest source — sound,
  less flexible). Elision would follow Rust's. Not built; the `.f:
  proj[from: a]` entries cover per-field precision wherever there is a body.
- **Accumulator bodies** (`best = person; …; return best` under a projected
  return) are out of scope: reassignable borrowed locals are a recorded
  refinement of [readonly-return].

## Standard library surface

S-Str (a mutable string and the string function surface) and S-Seq (the sequence
functions over any pass) landed 2026-09-06; **S-Col — collections** landed
2026-09-12 (`Set`/`Map`/`SortedSet`/`SortedMap`, collection literals, universal
struct `==`, `canbe hashed`/`canbe ordered`, the `*_of`/`*_by`/`to_*`
conventions); see COMPLETED.md for all three, including how C-7 — the Set/Map
pass design — was answered by prototype (snapshot passes owning a `List<T>`,
after borrowing passes and `copy()` were both tried and failed). What is left:

### S-Col leftovers — recorded, none forced

- **Reachability precision.** Name-based reachability now pulls `core.set`,
  `core.map` and the Rust `collections.rs` runtime into *every* program,
  because `to_set`/`to_map`/`list_of` name each other across modules: a
  hello-world went from ~300 to 569 lines of (allowed, dead) generated code.
  Harmless — both backends tolerate unused items — but the fix is real:
  reachability should walk the checker's *resolved* call targets rather than
  matching names, which needs the checker to record them. Deliberately not
  done with the collections work; it is a separate pass over `reach.rs`.
- **`entries` and `values` passes over a Map** are deferred. A Map pass
  yields **keys** (Python's `for k in d` precedent) because an entries pass
  would need an owned `(K, V)` and Kotlin cannot copy a generic `V`, so
  identity-sharing would alias mutable values and break backend parity. The
  answer is probably the same snapshot shape the key pass uses, over a
  `List<(K, V)>`, once tuples-of-generics are exercised enough to trust.
- **Comparator functions for keys** stay out (intrinsic ordering only, user
  decision 2026-09-12), and **float fields still bar `canbe hashed`**. The
  latter is the one worth revisiting: it is a consequence of Salvo owning
  float equality, and the future precision-specified comparison the user
  asked for is where a hashable float would come from.
- **`Deque<T>`** is the honest replacement for the linked list the user
  asked about, and the recommendation was to build neither in v1. Both
  targets ship and are proud of `ArrayDeque`/`VecDeque`, and every workload
  people reach for a linked list for (queues, BFS frontiers, sliding
  windows, LRU order) is served better by one: one intrinsic type, six
  functions (`push_front`/`push_back`, `pop_front`/`pop_back`, `peek` at
  both ends), no new concepts. Recorded as the *next* collection, when a
  customer appears — phase 5's mailboxes may be it. (A representation
  qualifier `Linked List<T>` was examined and rejected: it would make the
  shared List surface *worse*, since `get(list, i)` becomes O(n), and
  Rust's `LinkedList` has no stable cursor API so the O(1) middle insertion
  that justifies the representation is unreachable. Salvo-proper `struct
  Node<T> { value: T, next: Node<T> | None }` is blocked on the Rust boxing
  rule — see "Recursive types".)

### S-Col C-6 leftovers — the claims std does *not* ship

C-6 landed 2026-09-13: `NonEmpty`, `Sorted` and `Distinct` over `List<T>`,
with `non_empty_list`, the optional-dropping `first` overload, `sort`,
`mut_sort`, `add_sorted` and `binary_search` — and, once **qualifier
overloading** landed the same day (user decision), `NonEmpty` over `Set`,
`Map`, `SortedSet` and `SortedMap` too, in `core.nonempty`, with the
refinements on `add`/`put` and the `min`/`max`/`first_key`/`last_key`
overloads that drop the optional. See COMPLETED.md. What is still not there:

- **The `NonEmpty` overloads that need no claim of their own**: a `fold` with
  no seed, and `first`-like accessors on the unordered containers (a `Set` has
  no `first` to specialize, since it has no index).
- **`Distinct` from `to_list` over a `SortedSet`.** That `to_list` lives in
  `core.sorted` and a constructor must sit beside its qualifier
  [qual-ctor-same-file], so it returns a plain list while `core.set`'s mints
  the claim. Either move the qualifier somewhere both can see, or relax the
  same-file rule for `as Q` on an `intrinsic`.
- **A `Sorted` list cannot be *tested*.** No `qualifies`, because deciding it
  compares elements — which needs the type-parameter bound the generic
  container defect under "Open defects" also wants. With that bound, `Sorted`
  could gain an `is_sorted` predicate and stop being mint-only.
- **`Sorted` over the other containers** would be meaningless (the sorted pair
  *is* the representation), but `Distinct of Set<T>` is tautological and
  `NonEmpty` is the only claim that generalized. Worth remembering before
  reaching for overloading again: the mechanism is general, the claims are not.

### S-IO — the filesystem (phase 4) — ✅ complete 2026-09-15

Decided in six rounds of user decisions and built the same day, item by item:
the Has-accessor effect fusion, the operator-typing slice, the `@Effect`
member-disambiguation grammar, O-R2 interception, the `platform handler`
mechanism, effect-member overloading, the [linear-group] member-discharger
amendment, `core.fs` + `core.hostfs` with std's two host files, `MemFs`,
`RestrictedFs`, one overload set for members and fns, the byte payload (twice:
`List<Byte>` first, then std's own `Bytes`), the fill-a-buffer reads, the chunk
pass and the copy one-shots, and `examples/files/`. **The record is
COMPLETED.md's decision log** (six entries, newest "`Bytes`, `read_to` and the
copy one-shots"); **the as-built rules are LANGUAGE_SPEC.md's** [fs-surface],
[fs-token], [fs-errors-at-close], [fs-bytes], [fs-read-to], [fs-host-split],
[fs-double], [fs-restricted], [fs-v1-cuts], [byte-value] and [bytes-type]. FILE_SYSTEM.md — the plan of record, and the full option
record — **has retired into COMPLETED.md** as its charter said, so nothing
below points at it any more.

The historical outline of an `Fs` effect (2026-09-06, deferred so that IO
streams could be designed first) was superseded by what shipped: its two
findings — errors returned rather than thrown, and stream operations as
*members* so a double can fake them — are both in the built surface
[fs-surface].

#### The byte payload — ✅ answered and built 2026-09-15

The question the first byte deliverable left open (Kotlin boxing `List<Byte>`,
with no `UByteArray` rendering available under erased generics) was **decided
by the user: a `Bytes` type of std's own** — option (b) of the three — with the
fill-a-buffer reads and the copy one-shots on top of it. Built the same day;
see COMPLETED.md's decision log ("`Bytes`, `read_to` and the copy one-shots")
and the rules [bytes-type], [fs-read-to], [kt-bytes].

### Leftovers found while building the fs (none blocking)

- **Reachability is name-based, so `core.fs` is linked by programs that never
  open a file**: it declares a `next` and a `to_str`, and `reach.rs` pulls in
  every module declaring a *used name* [mod-used-only]. The consequence is
  emitted-but-dead code (the surface, plus the 8-arm union wrappers), not
  wrong behavior — and the dependent handler was moved to `core.hostfs`
  precisely so the *fusion* is not dragged in with it. The fix, when it is
  worth it: resolve fn-name usage through the checker's `call_fn` instead of
  `name_origins`, which needs reachability to run after checking.
- **`rename` is a keyword**, so the member is `rename_path` (both on `Fs` and
  in `RawFs`). Making `rename` contextual in a member position is a parser
  change nobody has asked for; the name deviates from the signed-off member
  list deliberately.
- **A pass's type must be *visible* for `for` to drive it**: importing the
  `next` alone is not enough (`import fs.Lines` was needed in a scratch
  program before `for line in p` resolved), which reads as a missing import
  of something the value already has. Invisible in std (`core.*` is
  implicit); worth a better diagnostic, or a rule that the subject's own
  declaration is enough.
- **`size(Str)` disagrees between the backends outside ASCII** (found
  2026-09-14, pre-existing): Kotlin lowers it to `String.length` (UTF-16 code
  units) and Rust to `chars().count()` (code points), so a string containing a
  non-BMP character — an emoji — makes the same program print different
  numbers, and `char_at` indexes differently with it. `byte_size` was added
  beside it for the filesystem's offsets and is parity-safe by construction,
  but `size`/`char_at` need a **decision**: what a `Str` index means (code
  points, UTF-16 units, or bytes), and then one lowering per backend that says
  so. Repro: `println("${size("a😀b")}")` prints 3 on Rust and 4 on Kotlin.
- **A colliding call types its arguments without expected types**
  [effect-available]: the lead-candidate machinery belongs to the fn path, so
  inside a call whose name is both an available member and a fn, a lambda that
  needs its parameter type from the position falls back to
  [type-unknown-lenient]. Both backends still emit working code for the cases
  tried (a field read through such a lambda runs identically on both), but the
  checker is not proving it. The fix is a lead pool shared by both kinds.
- **Kotlin's `USELESS_CAST` is suppressed rather than avoided**
  [kt-suppress-cast]: the emitter could skip the payload cast where kotlinc's
  smart cast already types it, but knowing exactly when is subtle, and the
  annotation costs nothing.


## Threading and concurrency — asynchronous effect handlers (phase 5)

**Designed** (user decisions 2026-09-14/15; the four **DECISION**s this
section used to carry are all answered — the argument trails live in
CONCURRENCY.md, the decided summary in COMPLETED.md's decision log). The
intended shape (user, 2026-09-09) was the Erlang/Gleam/OTP model; the design
pass landed somewhere better: **the effect surface is the model**. An actor
is an effect handler bound asynchronously — a state struct plus one function
per member, run-to-completion on a small scheduler library (no runtime in
generated code, full backend parity). Named *asynchronous effect handlers*
after Ahman & Pretnar's Æff, its closest formal relative.

**The documents**: CONCURRENCY.md (option space → the direction → the frozen
first pass → remaining opens), CONCURRENCY_EXAMPLES.md and
CONCURRENCY_EXAMPLES.effects.md (worked examples, the kernel and sugar tower,
the deadlock example), DESIGN_DOC.md (the template). They stay alive until
implementation lands, as FILE_SYSTEM.md did for phase 4.
**LINEARITY_COLLECTIONS.md has retired** into COMPLETED.md's decision log per
its charter: item 7 carries its content in code and in [linear-container] /
[linear-state]. SUPERVISION.md stays until item 8 — the runtime files cite its
section numbers, so its retirement is a sweep rather than a delete.

**The four answers, one line each**: sendability = structural rule +
diagnostic (C-4(a), `Arc`-where-sent inference as growth); a spawned actor's
effects = handler dependencies supplied at the spawn site
([effect-handler-deps] at a distance — construction crosses, handlers never
do); substrate = run-to-completion on pools (`on pool(n)`, one arrival-order
queue per actor, **bound explicit and required**); async **dissolves** (no
colouring — the 2026-09-04 record honoured by making the question moot).

**First-pass grammar (frozen)**: `send fn` members with explicit `Reply<T>`
parameters (linear, statically one-shot); `replyto k(captures)` /
`replyto! k(captures)` (the gate: bounded selective receive, one outstanding
per actor); `r.send(v)` discharges; `spawn H(args) use Handler(...), addr
on pool(n)`; lowercase `[use, spawn]`; `use addr`; `waitfor` as main's
explicit bridge, and **the program ends when `main` returns**. Deadlock
baseline: the effect-graph cycle check — **built 2026-09-16**, with gate cycles
as errors and blocking-send cycles as warnings. The sugar tower (`-> T` + call
syntax, `then`/`then!`, `defer`, merge/join, the gate member-set
generalization) comes in later passes, each with its own decision surface.

**What remains before the build: nothing — the decision space is empty.**
The supervision/monitors design was the last prerequisite and is **decided**
(user, 2026-09-15; SUPERVISION.md — death = a faulted activation, `watch`
with a linear `Exit` token as the whole monitor surface, dead-target
sends/fulfils as silent no-ops plus the idle-with-parked-gates runtime
report, supervision as a pattern with no syntax). Its opening requirement
(LC-4's dying-with-obligations) is answered there.

**The implementation**, per the first-pass plan in CONCURRENCY.md. **A
program spawns, sends, parks continuations, bridges, and watches its
children die — on both backends, with identical output** (2026-09-15/16; the
build records and what each slice cost are in COMPLETED.md's decision log). Everything landed the same day: the
scheduler library, the declaration forms, the expression forms, the types, the
checker rules, the emitters' first cut — then the respelling sweep, the
`actor effect` kind, the forwarding stub, **dependent-handler spawns**, and
**`replyto` / `@self`**. The smallest running program is the counter in both
backends' codegen tests: `spawn Counting() capacity 8 on pool(1)`,
`counter.bump(2)`, `waitfor out: Reply<Int> { counter.total(out) }`, printing
`sum 5` from Kotlin and Rust alike; the one that shows what the surface is
*for* is the fetcher beside it, which parks a continuation for a database
actor's answer and fulfils `main`'s token from inside its own activation.
As-built rules: LANGUAGE_SPEC.md's "Asynchronous effect handlers"
([actor-kind] … [actor-types], [actor-watch], [actor-deadlock-cycle]) plus
[linear-opaque], [rs-actor] and [kt-actor].

**The agreed sequence for the rest of phase 5** (user decisions 2026-09-15,
after reading EFFECT_UNIFICATION.md's plan): the two surface changes that
document decided go **first**, because both touch landed surface and every
later slice adds sites to them; the emitters follow; **the call-member sugar
pass and the rest of the sugar tower leave phase 5 altogether** (below, "The
sugar pass — after phase 5"). Every item here is a *diagnostic* in the
compiler today, so its own output is the work list.

1. ✅ **The respelling sweep — done 2026-09-15.** `Pid<E>` → `Addr<E>` and
   `self.k(…)` → `k@self(…)`, both landed with their spec rules
   ([actor-types], [actor-self-send], [actor-use-addr]) and the old spellings
   now plain parse errors. What it cost and what it simplified is in
   COMPLETED.md's log; the one thing worth carrying: `@self` as a *selector*
   deleted the rule that a handler member may not declare a variable named
   `self`, because nothing can shadow a selector.
2. ✅ **`actor effect` and its refusal list — done 2026-09-15.** The kind
   marker, `send fn` requiring it, the declaration-site refusals (kept
   parameters, `Mut` parameters, `proj` returns, non-sendable payloads) and the
   binding gate on `spawn` / `use addr` / `Addr<E>` — which **closes
   CONCURRENCY.md's carried named question**: a plain effect is never
   actor-backed. Sendability landed with it, as decided. Rules:
   [actor-effect-kind], [actor-sendable]. The emitters' gates now key on the
   kind rather than on "has send members".
3. ✅ **The forwarding stub — done 2026-09-15.** `__Stub_E` beside each async
   effect, implementing it by sending to an addr; `use addr` binds one exactly
   as a handler instance is bound, in both backends and in both fusion modes.
   The refactor that made it cheap is the one item 4 needs: each backend's
   `use` path now takes the *expression* that builds an instance
   (`emit_fusion_instance` / `bind_effect_instance`) instead of a handler
   declaration, so nothing downstream knows which kind it got. Verified by a
   compile-and-run case per backend, with `[Log]` travelling down an ordinary
   effect list into a function that never learns it is an actor
   ([actor-use-addr], [rs-actor], [kt-actor]).
4. ✅ **Dependent-handler spawns — done 2026-09-15.** Every realistic handler
   needed it (`Counting [Log]`, std's `DefaultFs [RawFs]`), and it landed as
   the survey predicted: one **checker table** (`spawn_dep_items` — which
   clause item satisfied which declared dependency, which `check_spawn`
   already knew while matching), then each backend's own shape for "the child
   owns its environment". Rust emits a **generic flat provider** `__Prov_H<__D0,
   …>` beside the handler, `__Proc_H` holds it, and `handle` builds the
   existing `__Deps_H` view over it ([rs-actor]); Kotlin repeats the
   handler's carrier type parameter on `__Proc_H` and has the *spawn site*
   build an `__Fx_N` from the clause ([kt-actor]). Both make a clause item
   into an instance the way item 3's refactor made one — a construction is
   `D(args)`, an addr is the forwarding stub — so a dependency swaps between a
   local handler and an actor with no change to the child. Verified by a
   compile-and-run case per backend with one dependency supplied as a
   construction and another as an addr, identical output on both, plus the
   written-order swap and a plain-effect dependency with a constructor
   argument. **The two "not emitted yet" refusals in `emit_spawn` are gone**;
   what `emit_spawn` still refuses is a generic handler. Two defects surfaced
   on the way and were **both fixed the same day** — an effect member name
   colliding with a std fn, and consuming a handler's stored values; each
   turned out to have a *silently wrong output* half, and both records are in
   COMPLETED.md.
5. ✅ **`replyto` and `@self` emission — done 2026-09-15.** The slice the phase
   was blocked on: **request/response now works without `main` in the loop**.
   Three design points were settled first (user decisions, D5-a/b/c):
   * **The address**: a generated `__addr` field on any handler of an `async
     effect`, written by `__Proc_H` from the activation's `SalvoCtx`. Chosen
     over threading `SalvoCtx` into the member (which would land on the
     effect *trait*, and so on the stub, the fusion and `__Impl_H`) and over a
     runtime thread-local. Its absence doubles as the **self-send's
     discriminator**, which is what lets `k@self(…)` have both its readings
     without compiling the member twice.
   * **The parked table**: `__parked: slot → __Cont_E` on the same handler,
     with `__Cont_E` beside `__Msg_E` carrying each target member's parameters
     minus the trailing answer. On the handler because the *mint* happens in a
     member body, which cannot see the actor struct — and this left
     `SalvoProcess::resume`'s decided signature untouched, which was the point.
   * **`replyto` under a `use` binding**: refused **statically**, at the `use`
     site — a handler that parks may only be spawned. Bound synchronously its
     mint targets a *local* instance with no mailbox and no dispatcher, so the
     continuation would silently never run. The gate is syntactic per handler
     (effects propagate, so a `use` site cannot know which members a scope
     reaches); it does not touch Example 6's binding swap, which is a spawn
     clause.
   * **Remote mints stay out** (user decision 2026-09-15): `replyto` resolves
     lexically only in this pass. The generalized mint (EU-7b) makes the mint
     itself send-like — capacity reserved in the *target's* queue, so it can
     block and contributes its own wait-for edge — and belongs with the sugar
     pass; a target reached through the effect list is a diagnostic naming the
     workaround, which needs nothing new (a token is a linear value: mint it
     where `k` lives and pass it).

   Verified by three compile-and-run cases per backend with identical output:
   a fetcher parking a continuation for a database's answer and only then
   fulfilling `main`'s token (`got row 7`); the gate, whose ordering *is* the
   assertion (`reply R, user late`); and both readings of `k@self` answering
   the same thing. **All four "not emitted yet" refusals are gone** — including
   the two whose text still named the pre-item-1 `self.k(…)` spelling.
6. ✅ **`watch` and the deadlock baseline — done 2026-09-16.** The monitor
   surface and the static check, both on both backends with identical output.
   Three design points were settled first (user decisions, D6-a/b/c): `watch`
   as an `intrinsic fn` in `core.actor` rather than a member of the
   capability-only `spawn` effect; `Exit` a plain struct whose value the
   **watch site** builds, since the runtime cannot construct a Salvo struct
   (and a `resume`-side special case would have been silently wrong the moment
   a program fulfilled a `Reply<Exit>` itself); and the cycle check's two
   severities — a gate cycle is an error, a **blocking-send cycle a warning**
   (the user's amendment: the compiler already has a warning severity, so the
   honest edge set is the full one). Interception is exempt, which no design
   document had anticipated: a handler of `E` declaring `[E]` is an `E → E`
   edge by construction and can never deadlock, because the dependency binds
   outward. A `[spawn]`-propagation defect surfaced and was fixed the same day.
   Rules: [actor-watch], [actor-deadlock-cycle], [actor-spawn-effect]; the
   record is in COMPLETED.md's log. **Two gaps recorded, not closed**:
   * **A self-send into a full own mailbox wedges the actor**, and
     `k@self(…)` deliberately contributes no edge — warning on every self-send
     would drown the form, and the runtime's idle report cannot see it (a
     blocked sender is not idle). The alternatives when it bites: exempt a
     self-send from the queue bound in both runtimes (a semantics call), or
     warn at the form. **DECISION** when it matters.
   * **The graph is over actor types, not instances**, so a chain of
     same-protocol workers reads as a self-loop, and a handler's declared
     dependencies stand in for what its members reach. Stratification (a tier
     qualifier on an addr) and the fallbacks (a timeout form, a per-edge
     reentrant opt-in) stay unbuilt until the false positives are *observed*.
7. ✅ **Linearity in collections — done 2026-09-16.** L8's answer, built: a
   container is linear exactly when its element type is, so
   `waiting: Mut List<Reply<Str>>` in handler state works — `add` parks a
   token, `remove_first` answers one, `drain` is the terminal, and a container
   that is never drained is a leak naming `drain`. Five build-time decisions
   (D7-a…e, user 2026-09-16) settled the surface; the one that changed the
   design document's sketch is the terminal — a **consuming callback**
   (`drain(list, each)`) rather than `for x in drain(list)`, since a `for`
   cannot consume a linear temporary [iter-drive-in-place] and no implicit
   discharge site exists. Rules: [linear-container], [linear-state], a
   rewritten [linear-composite], [rs-state-take], [rs-linear-move],
   [kt-linear-container]; the record is in COMPLETED.md's log. **Three things
   left behind**, below: the effectful-discharger gap, the list's missing
   positional write, and bare obligations in state.

   * **An effectful discharger cannot drain a container.** A lambda performs
     only the effects its *type* declares [fn-effects], and `drain`'s callback
     type is pure — so `close(s: InStream) [Fs]`, or anything that logs, cannot
     fill it, which makes a `List<InStream>` undrainable. Reply tokens are
     unaffected (`send(r, v)` is pure), which is why the slice shipped. Two
     candidate answers, both **DECISION**s: a `for`-driven terminal (a
     `let`-bound drain pass, which needs a rule for what discharges a pass that
     stopped early), or letting a *pure* callback position accept an effectful
     argument by widening the call's own requirements (effect polymorphism at
     the call site, which is E3 step 4's neighbourhood). Worth doing when a
     real program wants it; the workaround meanwhile is a discharger that takes
     the effect's *handler* as data, or draining into a plain list first.
   * **No positional list write**: `replace(list, i, v)` would have to answer
     `None` for an out-of-range index and drop the value it was handed. Take an
     element out and add a new one, or key the collection with a `Map` (whose
     `replace` has no such hole). Add it if a customer appears, with a
     `Ok T | Err T`-shaped answer.
   * **A bare obligation in handler state is refused**, naming the container:
     taking it out would leave a hole nothing could fill, so its obligation
     would have no reachable discharge. It is also what keeps LC-4 sound on
     Rust, where a non-`Default` field has no representable temporarily-empty
     state. A `linear struct Gather` living *in* a map is the shape that works,
     and is what the examples use.
8. **Propagation and retirement.** The COMPLETED.md entries (including
   EFFECT_UNIFICATION.md's round-1 rejection as an explored-and-abandoned
   option); CONCURRENCY.md's pending table (the named question closes);
   `CONCURRENCY_EXAMPLES.md`'s `Reply<T>` definition reworded to
   reservation-at-mint-in-the-target; the examples-file respelling done
   **once**, here, rather than four times on the way (`capacity N` is
   required, and those files' spawns predate it); a worked
   `examples/actors/`; then the working documents retire into the decision
   log per their charters — with the sugar pass's decided content folded into
   this file first, so nothing open lives outside ROADMAP.md.
   **LINEARITY_COLLECTIONS.md is already gone** (item 7 carried its content
   into code, the spec rules and the decision log). SUPERVISION.md is ready to
   follow — its S-1…S-4 content is in [actor-watch] and in the log — but the
   two runtime files and their tests cite its section numbers, so its
   retirement is a small sweep; CONCURRENCY.md, the two examples files and
   EFFECT_UNIFICATION.md follow it.

**The leftovers the checker slice found are all closed** (2026-09-15) — the
record, with what each taught, is in COMPLETED.md's decision log; the last of
them, self-sends, was a user decision the same day (`k@self(…)`, respelled to
`k@self(…)` by item 1 above).

**Deferred out of the phase**: [fate-lambda] moves to the call-sugar pass —
no first-pass form crosses a closure (spawn-`use` arguments, `replyto`
captures, and `waitfor`'s token are all *values*). The recorded refinement
(`move`-closure emission with hoisted clones, treatment for captured
effect-handler locals) is unchanged, just re-scheduled.

**Regions rode along as planned** (user, 2026-09-10, confirmed 2026-09-15):
an actor **is** a region; sendability and region-escape are one check.

**What was already in place carried its weight**: send-as-move was ordinary
consumption; linearity survived sends [linear-obligation] and became the
reply-token guarantee; supervision-as-handler became interception across the
scheduler boundary (CONCURRENCY_EXAMPLES.effects.md, Example 4).

## The sugar pass — after phase 5 (user decision 2026-09-15)

Deferred out of the phase deliberately: phase 5 delivers the **explicit**
surface (tokens and reply parameters written out), and every layer of sugar
above it becomes a later item with its own decision surface. What is already
*decided* about it, so the pass starts from a plan rather than a blank page
(EFFECT_UNIFICATION.md, EU-6 and EU-7b):

- **Per-kind `-> T`** (EU-6 = (a)): plain effects unchanged forever; inside an
  `actor effect`, `fn m(a) -> T` means an implicit trailing `Reply<T>`,
  fulfil-at-every-return, and caller-side call syntax = auto-mint + gate. This
  is why item 2 above keeps `send fn` rather than making it implicit: the
  non-send forms are stated *against* it — `send fn m(a)` is no completion,
  `fn m(a) -> T` a call member, `fn m(a) -> None` the acknowledged form.
- **The generalized mint** (EU-7b): `replyto k(c)` resolves lexically against
  the enclosing handler, else through the effect list as a *remote* mint — a
  curried, capacity-reserved, one-shot send. Reservation happens at mint time
  in the token's target, so discharge never blocks; mint sites contribute
  deadlock edges like sends. A bare `k` naming both an enclosing member and an
  in-scope async member is **refused**, naming `k@self` and `k@E`; a remote
  mint where no actor exists is refused, naming `waitfor`. `replyto!` and
  self-targets stay lexical to handler bodies.
- **Two stub readings appear here, and only here.** An answering member cannot
  have one implementation for both bindings: from an actor the call parks,
  from synchronous code it must block — which `main` may do and an actor may
  not (EU-2, and point 2 of the unification's stated intent). The first pass
  has one stub precisely because nothing answers.
- Then the rest of the tower, each its own call: `then`/`then!`, `defer`,
  merge/join, the gate's member-set generalization.
- **Watch item, carried**: a helper that must create self-targeted or gated
  continuations in *data-dependent number* still cannot be written outside a
  handler body. If that bites, the recorded shape to revisit is EU-7(a)'s
  narrow running-in entry.


## Laziness, after concurrency (user decision 2026-09-10)

std's lazy pair (`map_lazy`/`filter_lazy`, composed passes that computed as they
were driven) was **removed** 2026-09-10 rather than carried along, and the
question reopens after phase 5. The reason for removing it now: laziness was
touching three unsettled decisions at once — L8 (a composed pass stores its
source, so a linear one is refused), sendability (`Rc<dyn Fn…>` in a fn-typed
field is not `Send`), and the shape of the combinator surface itself — and it was
the *least* settled of the four, so it was the one to take off the table.

**The direction to try when it is picked up** (the user's, stated with the
removal): standard laziness *couples data to the functions over it*, and the two
should stay separate. What is wanted instead is a good way to **compose
functions — `iter fn`s included — into pipeline functions**, which then mint a
fresh pass from data supplied independently. So `map`-then-`filter` would build a
*function*, not a wrapped data structure, and the data arrives at the end.

What the existing implementation already contributes, so this is not a blank
page:

- **An `iter fn` is already "a function that mints a pass"**, and its pass is
  unnameable by design — which is exactly the shape a pipeline function wants to
  return. `?iter` as an implicit (2026-09-10) is already the mechanism for "take
  the data, mint the pass" in a *generic* function.
- **A pass is only a struct with a `next`** [iter-protocol], so a pipeline that
  does need state has somewhere to put it without new language surface.
- **Effects on fn types** [fn-effects] already thread a callback's effects to
  whoever calls the value, which a pipeline of effectful steps needs.

Questions to answer with it, all of them consequences of composing functions
rather than data:

- **What composes, and how it is spelled.** Two `(T) -> U` steps compose
  obviously; an `iter fn` (subject → pass) composed with a step is a different
  arrow, and a filter changes the *count* of elements rather than their type.
  Whether all three are one notion or three is the first call.
- **Where the state lives.** If a pipeline function is a value, and a stage needs
  per-run state, the state must be minted per drive rather than captured once —
  which is the replay property `iter fn` already has (it copies its subject at
  the mint) and the thing a stored composed pass got wrong.
- **Does it dissolve the L8 casualty or inherit it?** A pipeline that holds only
  *functions* stores no source, so "can you lazily `map` over a file's lines?"
  may become yes without widening the composite rule at all. That is the
  strongest argument for this direction and it should be tested first.
- **Sendability.** If a pipeline is a value holding fn-typed fields, phase 5's
  `Rc`-is-not-`Send` question applies to it directly; if it is a *function*, it
  may not.

## Regions — designed (user decisions 2026-09-10), built with phase 5

Raised by the user: model Vale-style regions as effects — a scope explicitly
opens a region, functions declare that they use the caller's region, the way
`try`/`throw` works. Designed across one session (this supersedes the first
write-up of the same day; the decision record is in COMPLETED.md). The design
calls are made; the build is scheduled **into phase 5**, where its customers
live.

### What transfers from Vale, and what does not

Vale's regions exist to remove *generational-reference* runtime checks; that
motivation does not transfer — Salvo's safety is static. What transfers is
**region-scoped data** (a value that may not outlive a scope) and
**scope-wide immutability** as a fact the checker can use. Said plainly: a
region is a lifetime with a coarser grain and a friendlier name, and it
spends part of the "no lifetimes in the source" premise deliberately — one
binder per scope instead of a lifetime per value.

### The decided reading: R1 — values may not outlive their region

R1 (region-scoped data plus scope immutability) is the design. R2 — the
region *owns cleanup obligations* and bulk-discharges them at close — is
**rejected** (user, 2026-09-10): linearity cleanup stays explicit, per path,
because discharge can be a *choice* (`stop` vs `join` on a thread handle),
can need context the scope does not hold (`remove(cache, handle)`), and
discharge functions can use effects, which an implicit close has no business
supplying (the first two are the examples recorded under L8). Regions manage
**memory and lifetime, never obligations** — linear values are exempt below —
which also avoids the `defer` trap ("a block whose end runs cleanup") by
construction rather than by rule.

### The design

- **`effect Region`, with an intrinsic handler.** `Region` is an ordinary
  effect whose members are `reg` and `unreg`; the `region { … }` delimiter
  registers the **intrinsic handler** for its scope. This keeps "an effect is
  a capability with a handler" true — `Throw` remains the single handler-less
  exception — while regions inherit the full effect machinery: `[Region]` in
  effect lists, outward propagation, "no delimiter above you" diagnostics,
  innermost-wins. No labelled regions in v1 (precedent: no labelled throws).
  `main` may open `region { }` but may not declare `[Region]` — no caller.
- **`Reg`, an intrinsic provenance qualifier**, marks membership — provenance
  because mutation can never invalidate where a handle came from. Merely
  *holding* a `Reg T` needs no effect entry (having one proves a region is
  open below you); `[Region]` is declared by whoever calls `reg`/`unreg` or
  constructs into the caller's region, per ordinary effect rules. The
  register/region double reading of `reg` is intentional; spec prose must
  keep the bare word "register" for handlers and `use`.
- **Transitive through projections**: a projection of a `Reg` value is `Reg`
  (`node.name` on a `Reg Node` is `Reg Str`), so inner tags carry no
  information and are rejected (`Reg List<Reg Node>` is an error; write
  `Reg List<Node>`). Whether propagation-through-projection becomes a
  *general* per-qualifier property (L8 wants something adjacent for
  obligations) is **deferred until more examples exist** (user, 2026-09-10).
  It cannot be uniform: `Authenticated Request` must not project to
  `Authenticated Str`.
- **Inverted defaults — regional by birth.** Every value constructed in
  region context (lexically inside `region { }`, or in the body of a fn
  declaring `[Region]`) is `Reg`. `reg(v)` moves an outside value in — a
  consuming deduction, no copy. `unreg(v)` takes a copy out:
  `fn unreg<T>(value: Reg T) [Region] -> T` — the copy is built into => value
  the function, since duplicable handles mean exclusivity can never be
  proven; it **elides when the argument is a fresh construction**, which is
  also the opt-out-at-construction spelling (`unreg(Summary { … })`). `copy`
  respects ambient placement (a copy made in region context is `Reg`);
  `unreg` is the override.
- **Exemptions.** Copy scalars (`Int`, `Bool`, …) are never `Reg` — no
  lifetime to manage, nothing to tag. Linear values are implicitly
  un-regional: a `: Linear` construction in region context is an ordinary
  value under the existing per-path discharge rules (see the R2 rejection).
- **`Mut` interplay — the freeze.** A `Reg` value with a `Mut` handle follows
  today's rules unchanged (exclusive handle, fate links, deductions) — this
  is how anything is *built* inside a region, and it matches the arena
  reality (allocation hands back exclusive access). The **freeze** is
  dropping the `Mut` — widening (`^Mut`) or moving into a non-`Mut` position —
  after which the value gets the regional treatment: handles freely
  duplicable, **no shared-fate links**, and **state qualifiers permanent**
  (nothing can ever mutate a frozen value, so `Reg NonEmpty List<Int>` never
  loses `NonEmpty` — the exact mirror of `Cell`'s "no state qualifiers on
  contents").
- **The escape rule.** Nothing carrying a region's provenance may escape its
  delimiter — as the block's value, by `return` or `break`-with-value, or by
  storage into an outer variable or literal. The existing consumption/flow
  analysis is the machinery; the diagnostic names the escape event and the
  remedy ("`unreg(v)` to take a copy out"). Ordinary locals declared in the
  block are untouched — the region delimits only its members, and both
  disciplines coexist in one scope with the qualifier saying which one a
  value is under.

### What it retires, for frozen `Reg` values

| today | frozen `Reg` value |
|---|---|
| returning a kept parameter's projection is a move / needs `copy` | legal — the return borrows the region, not the parameter |
| derived returns (`proj[from: p]`, generated lifetimes) | unnecessary — projections are `Reg` automatically |
| shared fate: links, root mutation poisons derivatives | no links exist; nothing can mutate a frozen root |
| storing one value in two literals consumes it at the first | handles duplicate freely |
| re-test (`is NonEmpty`) after every mutating call | claims are permanent |

Honest framing, kept from the first write-up: this is a **second axis**, not
a reduction of the first. Deductions, `Mut`, shared fate and `Linear` all
remain, unchanged, for un-regional values. The D7 watch-list entry
`Local`/`Escaping` is this idea under a smaller name; fold it into this
design when phase 5 picks it up.

### Backend lowering, staged

- **Kotlin**: erased entirely — `region { }` is a plain block, `Reg T` is
  `T`, `reg`/`unreg` are identity/copy. The same story as deductions
  [qual-erasure].
- **Rust v1 [rs-region-rc]**: `Reg T` → `Rc<T>`, `reg` → `Rc::new`, `unreg` →
  clone-out. Zero lifetimes in generated signatures — the first write-up's
  concern that regions put `'r` everywhere is answered by staging, not
  denied. The escape rule is enforced *semantically* from day one even
  though `Rc` would not dangle, deliberately, so v2 is a pure representation
  swap. Flag: `Rc` is not `Send` — the same phase-5 blocker as
  [rs-fn-field].
- **Rust v2 [rs-region-arena]**: a real arena (hand-rolled in emitted
  `core/` while output stays a single `rustc` invocation), `Reg T` → `&'r T`,
  one mechanical lifetime per delimiter, `[Region]` fns get `'r` threaded
  like an effect parameter. The recorded hard part is drop glue for regional
  collections (a `Reg List` owns a heap buffer that must not leak past the
  region).

### Why phase 5 (user, 2026-09-10)

An actor in the OTP model *is* a region: a private heap, bulk-freed on
death, with "leaving the region requires move/copy" as the sendability rule.
The null hypothesis for the phase-5 design session is that `region { }` is
the **sequential special case of an actor** — one that runs inline and dies
at the brace — giving one concept instead of two. The escaping-closure fix
[fate-lambda] is already a phase-5 prerequisite, and R1 is its notation.
Deciding regions standalone earlier would spend part of the same design call
twice — the reasoning that already deferred `Cell`.

### Still open when phase 5 picks this up

- The exact freeze spelling: `^Mut` as an expression, freeze-by-position
  only, or both.
- Cross-region operations (Vale's "read an outer region while building an
  inner one") — deliberately out of v1.
- `unreg` of a deeply regional structure must copy deeply — same per-backend
  rules as `copy`, including its refuse-rather-than-diverge cases.
- Folding D7's `Local`/`Escaping` watch-list entry into this design.

## Recursive types — unscheduled, after the sequence

Investigated 2026-09-12 (probes against that evening's debug binary; raised by
the collections design's linked-list question, which it outgrew). Deliberately **at
the end of the queue**: nothing in phases 3–5 needs it (user, 2026-09-12). Its
customers are trees, ASTs and JSON-shaped data — and, until it is built,
List-mediated recursion (below) covers them.

### Where things stand today: a hole, not a rule

Nothing in the checker or resolver rejects a recursive type; no spec rule
mentions them. The consequences, all verified by probe:

- `struct Node { value: Int, next: Node | None }` **passes the checker on both
  backends**. Kotlin emits `data class Node(val value: Int, val next: Node?)`
  — compiles and runs, references are free indirection. Rust emits
  `pub next: Option<Node>` and dies downstream with rustc's E0072 ("recursive
  type has infinite size"), with no Salvo diagnostic. The same happens for
  recursion through a union arm (`type Tree = Int | Branch`,
  `Branch { left: Tree, right: Tree }` → `Union2<i32, Branch>`, E0072).
  This accept/reject divergence is the open defect recorded above.
- **Recursion through `List<T>` already works end to end on both backends**
  (probe: `struct Tree { value: Int, kids: List<Tree> }` with a recursive
  `total` — compiled under rustc, ran, correct output). `Vec` is heap
  indirection, so the shape is representable today. Trees are therefore
  *usable now* under this encoding; only direct field and union-arm recursion
  is broken.
- `check.rs` was already written defensively: its type-walking predicates
  carry cycle guards ("recursive struct: already being checked"; the
  depth-guarded transitive-linearity walk), so the checker survives recursive
  declarations even though nothing admits them.

### Step 1 — the diagnostic (a defect fix, independent of the feature)

Close the [backend-never-wrong] hole now or with the feature, but decide it is
owed: an SCC walk over the type graph (struct fields, union arms, alias
expansions; `List`/array/fn-typed edges do **not** count as cycle edges —
they indirect already) and an error at the declaration naming the field that
closes the cycle, with the `List<T>` encoding as the named remedy. Cheap,
and honest whichever way the feature decision goes.

### Step 2 — the feature: a boxing rule for the Rust backend

Kotlin needs nothing. Rust needs compiler-inserted indirection, and the
design questions are:

- **Where the box goes** — minimal-edge boxing (box only the field/arm edges
  that close a cycle, which is rustc's own hint) versus boxing every
  recursive-type field. Minimal is the presumption. Placement interacts with
  the union representation: for `Tree = Int | Branch` the box can wrap the
  arm payload (`Union2<i32, Box<Branch>>`) or the field inside `Branch`, and
  the choice lands on every generated arm accessor and match.
- **Transparency at every use site.** A boxed field must behave exactly like
  an unboxed one: literals wrap (`Box::new`), reads autoderef, union matches
  see through the box (box patterns are not stable Rust, so emitted matches
  need explicit derefs), partial moves out of a boxed field keep working
  (they do, through `Box`), `Mut` paths get `&mut` via `DerefMut`, `copy`
  deep-clones (`Box<T: Clone>`). **Precedent that this is tractable**: the
  emitter already renders fn-typed fields differently from fn-typed values —
  `Rc` wrap at stores, clone at reads [rs-fn-field] — and the recorded
  drop-conversion machinery shows wrap/unwrap at checker-known sites is
  established. New spec rule on the Rust side (`rs-box`-shaped), nothing on
  Kotlin's.
- **Non-regular (polymorphic) recursion must be refused in Salvo.**
  `struct Node<T> { next: Node<List<T>> | None }` monomorphizes to infinitely
  many types on Rust while Kotlin's erasure accepts it — a second silent
  divergence hiding behind the first, currently surfacing (if at all) as
  Rust's recursion-limit error. The rule: a type may recurse only at its own
  instantiation.

### The semantic edges (each small, each a language call)

- **DECISION — constructibility.** `struct A { a: A }` has no base case: no
  value of it can ever be built. Refuse cycles with no optional/union escape
  arm at the declaration (recommended), or let them exist vacuously.
- **DECISION — depth, not cycles.** Actual *cyclic* values appear
  unconstructible — a cycle needs aliasing plus mutation through the alias,
  which ownership refuses (and boxed Rust representation could not hold one)
  — so `to_str`/equality/drop always terminate. But each recurses per node:
  a 100k-node chain overflows the stack in Kotlin's generated
  `toString`/`equals` and Rust's derived `Debug` and `Drop` (a known real
  Rust wart). Accept-and-document (recommended for v1) or emit iterative
  drop glue for recursive types.
- **Linearity stays out, together.** A recursive `linear struct` (a chain of
  obligations) would ask the discharge analysis to walk a runtime-sized
  structure at compile time; refuse recursion + linearity in the same
  declaration for v1. The existing cycle guards keep the *predicates*
  terminating meanwhile.
- **`iter fn` snapshot note**: an `iter fn` over a recursive subject
  snapshots per field — a deep copy, correct but O(n) where O(1) is assumed;
  document when the feature lands.

### Verification shapes, when picked up

The three probes above (direct field, union arm, `List`-mediated), each
compiled *and run* on both backends; a match through a boxed union arm; a
partial move out of a boxed field; `copy` of a recursive value; the
polymorphic-recursion refusal; and the constructibility refusal. The probes
are re-writable in minutes (they were built against `tmp/`, not kept).

## Consolidated leftovers

Small recorded remainders, each also noted in its own milestone section or spec
rule, collected here for findability. They are not a queue: nothing here is
blocking, and several are "revisit only if a customer appears".

- **Fresh-suite speed, remaining steps toward ~10–15s** (goal set by the
  user 2026-09-12; the kotlinc batching landed the same day — see
  COMPLETED.md decision log — bringing a fresh run to ~50s, ~55–70s
  since the interception case joined the batch). What is left,
  in impact order: the CLI suites (`run_tests`, `platform_tests`,
  `analyze_tests`: ~16s max single test; each spawns `salvo run`/`compile`
  which pays its own kotlinc), the rust codegen suite (~4.8s max under
  contention; a shared-runtime batch or precompiled `libcore` would cut
  it), and running the batched Kotlin programs in one JVM instead of one
  `kotlin` launch each (~0.4s per program). Past those, the floor is the
  matrix size itself — trimming compile-and-run cases whose behavior the
  goldens already pin.

- **`once` inference**: a callee calling its fn param at most once does
  not auto-promote to `once`; written only [once-fn]. Same
  written-validates/unwritten-infers pattern as deductions when taken.
- **Returning/storing capture-carrying closures**: rustc lifetime error
  the checker does not reject; the recorded refinement is
  `move`-closure emission with hoisted clones, pending a treatment for
  captured effect-handler locals [fate-lambda].
- **Exactly-once closures**: a `once` lambda may not consume a *linear*
  capture (the closure would inherit the obligation) [linear-lambda];
  supporting it means linear fn values.
- **Reassignable borrowed locals** (accumulator bodies in derived-return
  fns: `best = person; …; return best`) are rejected by provenance
  validation; supporting them is the recorded [readonly-return]
  refinement.
- **Same-call borrow/move (E0505 shape)**: one call that passes a
  borrow-emitted local *and* moves its root is checker-legal
  (left-to-right model) but rustc-rejected — loud, rare
  [rs-borrow-locals].
- **Internal qualifier unification** and **L5 field-disjoint precision** are
  both still unforced — see "L7 remainders" and "L5" above.

- `salvo lsp`: go-to-definition [lsp-definition] and doc-comment hover
  [doc-comment] landed, including nested declarations — struct fields,
  handler state, effect/handler/qualifier members — and, since 2026-09-11,
  `params` groups, names reached through an `import`, a predicate
  qualifier's short `qualifies` body [doc-qualifies-body], and fate links
  at a variable's *declaration* as well as its uses. Still open: `[symbol]`
  resolution is name-based over the AST rather than
  import-visibility-exact [doc-symbol-ref]. Also still open:
  incremental analysis if workspaces outgrow
  re-check-everything-per-keystroke, and a `positionEncoding` negotiation
  for UTF-8-native clients. Signature *hover* still covers fn decls only
  — effect members and define fns have no `FnKey` (go-to-definition does
  reach effect members, via `def_refs`). Not built, and unforced: hovering
  a fate *root* to see what derives from it (the reverse direction).
- **A generic `List<T>` cannot be interpolated** [interp-to-str]: an opaque
  `T` has no text form, so std's list renderer cannot reach its elements.
  The fix is composing an element `?to_str` at the interpolation site, and
  it needs two existing limits lifted: `resolve_implicit_fn` skips
  candidates that themselves take implicit parameters, and `implicit_args`
  is keyed by *call* spans, which an interpolation does not have. Unforced —
  the remedy is a `to_str` of your own — but it is the natural next step if
  interpolation of generic containers is wanted.
- **Predicate `is` on a union subject** is now roadmap phase **D4**, not
  a leftover: a `Ty::Union` subject always takes the arm-matching path,
  so `x is Positive` on `Int | Str` errors ("this check can never
  succeed") instead of calling `qualifies` — narrow first
  (`x is Int && x is Positive`). Lifting it requires qualifiers over
  unions [is-qualifies] [qual-union-arm].
- Struct destructuring ignores predicate-qualifier field overrides
  (deliberate: bindings get the declared type; direct accesses get the
  override + cast).
- Constructing a nested qualified union group in one expression works for
  *distinct* qualifiers as of 2026-09-10 (`emitted(ok(x))`); repeating the
  **same** one (`ok(ok("yes"))` into `Ok (Ok Str | Err Int) | …`) still needs an
  annotated intermediate `let`, and cannot be fixed by a rule — a flat
  qualifier list deduplicates, so `Ok Ok Str` *is* `Ok Str`. The no-arm
  diagnostic names the workaround [qual-group].
- Deduction inference does not track bare-parameter value flow out of
  branch/loop tails as a move (documented leniency in [deduce-infer]).
- **Array elements never narrow** ([flow-place] narrows variables, field chains
  and tuple positions): an unknown index may alias any element, so a constant
  one is not treated specially either — the expectation it would set is the
  reason.
- Module reachability is name-based and conservative: a local variable
  shadowing a std fn name still pulls that std module in (harmless
  extra output, never a missing module).
- **Operator typing — ✅ decided and built 2026-09-14** ([op-arith]
  [op-order] [op-bool] [op-promote] [op-convert] [lit-adopt]; the
  `==`/`!=` slice was 2026-09-12's). One deliberate exclusion carried
  forward: **`Byte` is not operator-numeric** until the byte surface and
  the `UByte` lowering land with the filesystem work — its arithmetic
  would diverge (signed on the JVM, unsigned on Rust) today. Generic
  (`Ty::Var`) operands stay lenient like `Unknown`, a documented leftover
  matching the equality slice.
