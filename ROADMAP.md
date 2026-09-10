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
dependencies, `defer`, `throw`/`try` and effects on fn types (E1, E3 steps 1–3),
places and field narrowing (P1), deductions with refinements (D1, D3), dot-names
(N1), overload resolution (finalized), the std string and sequence surfaces
(S-Str, S-Seq), and the iterator reduction to `next` (R0–R5 plus the generic
drive). What is left is below, grouped by theme; **"The sequence" is the order
it will be done in**, and each themed section is tagged with the phase it belongs
to. One dependency is worth stating on its own, because it is the reason the
order is what it is: **S-IO cannot restart until L8** decides how a linear value
lives in a composite, since its result shape (`Ok InputStream | Err Str`) is
exactly what the interim refusal forbids.

## The sequence (user decision 2026-09-09)

Five phases, in this order. Each names what is in scope, the decisions that have
to be answered before it starts, and the smaller items that ride along with it
rather than being scheduled separately. Nothing outside a phase needs doing
first.

**1 — Finish the iterators.** ("Iterators", below.)

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
- The **open defect**: a qualifier applied to an already-qualified value
  flattens, so `emitted(ok("x"))` matches no arm. It is first because it is also
  what makes `Emitted (Ok T | Err E)` unwritable, and phase 4 needs exactly that
  shape.
- A **suspending loop driving a pass** — the one capability the reduction lost.
  The planner already produces the nested-pass field; only emission is missing.
- The **`?close` implicit**, so an early-stopping combinator (`take`, `first`)
  can release its source. Phase 4 needs precisely this when it stops reading a
  file part-way.
- Riding along: the `@Suppress("UNCHECKED_CAST")` polish in std's `seq.kt` (the
  repo's own "generated code is warning-free" standard currently fails there),
  the implicit-resolution collision when a user type is named like one of std's
  passes, and three compiler comments still describing `Iter<T>` as current.
- Optional, and priced as an optimization rather than a fix: **mutable origins,
  option (e)**.
- ✅ **`yield fn` is deleted** (user decision 2026-09-10): `iter fn` covers the
  same ground without a state machine, so the sugar, the `yield` keyword and
  the whole machine apparatus went — `generator.rs`, both backends' renderers,
  the per-`defer` flags, the origin mints and the Rust `iter.rs` runtime. See
  COMPLETED.md.
**2 — Finish shared fate: places and partial moves.** ("Linear types → L5".)
Field-disjoint precision on the `Place` substrate P1 built. No decision
outstanding: it is analysis engineering under decisions already made.

**3 — Finish linearity: composition and conditionality.** Three questions that
have to be answered together rather than one:

- **L8** — how an obligation travels through a container, and when a container is
  linear at all. Today storing a linear value in a composite is refused outright,
  which is what makes a linear pass uncomposable and blocks phase 4's result
  shape.
- **D7** — linearity conditioned on a use-site qualifier. Its original motivation
  (`Once Iter<T>` versus plain `Iter<T>`) died with `Iter<T>`, so the case has to
  be re-derived from L8's container question.
- **D6** — `Once` on any type, which an earlier decision (user, 2026-09-07) binds
  to D7: "we'll need to design those together".
- Also in scope, because it is a hole in what already shipped: the **linear
  instantiation ban does not cover effect members with their own generics**.

**4 — The filesystem, on an IO stream design** ("Standard library surface →
S-IO"), using linearity and iterators — which is why it follows 1 and 3. Two
decisions are due **before** it starts, because both become concrete the moment
streams exist:

- **Operator typing and numeric promotion.** Offsets and sizes are naturally
  `Long`, so "does `Int + Long` promote, and to what" stops being theoretical;
  deciding it after std has a numeric surface means churning that surface.
- **Two effects sharing a member name.** `read`/`write`/`close` on an `Fs`
  effect, the stream surface and `Console` collide, so the `println@Console(...)`
  question is due here rather than being dodged with prefixed names.

**5 — Threading: the Erlang/Gleam/OTP model.** ("Threading and concurrency",
below.) A design pass first: four questions the existing implementation forces,
starting with sendability — generated Rust holds a fn-typed field as
`Rc<dyn Fn…>`, which is every composed pass. The capture-carrying-closure hole
surfaces at `spawn`, so it is fixed here rather than left as a leftover.

**Not before then: `Cell`.** It exists to make shared mutable state
expressible, and the OTP answer is that processes own their state and
message-pass — so phase 5 may remove its motivation entirely. Deciding it
earlier spends a language-design call twice.

## Decisions waiting on the user

Every item here needs a language-design call before it can be built, and the
call is the user's (AGENTS.md's first invariant). They are listed in one place
because that is the question a session most often needs answered first; each
links to the section that states the options.

| Question | Due | Where |
|---|---|---|
| **L8** — how an obligation travels through a container, and when a container is linear | phase 3 | "Linear types" |
| **D7** — qualifier-conditional linearity (with D6) | phase 3 | "Deductions and qualifier reasoning" |
| **D6** — `Once` on any type (with D7) | phase 3 | "Deductions and qualifier reasoning" |
| Operator typing rules — legal operand types, numeric promotion, `Bool` for `&&`/`\|\|` | before phase 4 | "Consolidated leftovers" |
| Two effects declaring one member name: keep the error, or add `println@Console(...)` | before phase 4 | "Effects" |
| **S-IO** — the stream design the filesystem is the first customer of | phase 4 | "Standard library surface" |
| Sendability, per-process effect scopes, OS threads versus a runtime, async's fate | phase 5 | "Threading and concurrency" |
| `Cell` — whether shared mutable state joins the language at all | after phase 5 | "Shared mutable state" |
| **D2** — `+Q` in a function's own deduction list (needs an establishment rule) | unscheduled | "Deductions and qualifier reasoning" |
| **D4** — predicate `is` on a union subject (needs qualifiers over unions) | unscheduled | "Deductions and qualifier reasoning" |

Two further proposals are **deferred by decision** rather than waiting: `defer`
as an effect with a `defers` block, and `platform handler` / `platform type`.

## Open defects

Bugs found and reproduced, not yet fixed. Each carries a repro small enough to
paste and a root cause, so picking one up needs no re-investigation. Closed ones
move to COMPLETED.md with their repro intact.

### A qualifier applied to an already-qualified value flattens, so `emitted(ok("x"))` matches no arm

Found 2026-09-07 while answering whether a fallible producer needs `Throw`
support [iter-protocol]. Minimal repro:

```
fn b(flag: Bool) -> Emitted (Ok Str | Err Str) | Finished {
    if flag {
        return emitted(ok("x"))    // ERROR
        // no arm of `Emitted (Ok Str | Err Str) | Finished` accepts a value
        // of type `Emitted Ok Str`
    }
    return finished()
}
```

**Root cause** (localized by two probes, both of which *pass*, so the fault is
in the combination and not in either half):

```
fn d(flag: Bool) -> Emitted (Str | Int) | Finished { ... return emitted("x") }   // fine
fn c() -> Ok Str | Err Str { return ok("x") }                                    // fine
```

`Ty::Qualified` holds a **flat** qualifier list, and `qualify` appends. So
`emitted(ok("x"))` — a qualifier applied to an already-qualified value —
produces `Qualified { quals: [Ok, Emitted], base: Str }`, which is
indistinguishable from "two qualifiers on a `Str`" and is *not* `Emitted`
applied to `Ok Str`. Matching it against the arm
`Qualified { quals: [Emitted], base: Union[Ok Str, Err Str] }` therefore
compares `Str` against `Ok Str | Err Str`, and a plain value never subtypes a
constructive qualifier [qual-constructive] — hence no arm accepts it. The
rendering ("Emitted Ok Str", no parentheses) is the same ambiguity showing
through.

So the earlier guess in this slot — "`Coercion::WrapUnion` cannot chain" — was
wrong, though a nested wrap *is* still needed for emission once matching is
fixed: the value would be a bare `Str` that has to be wrapped into the inner
union's wrapper before the outer arm's.

**Fix, when it is picked up**: either nest `Ty::Qualified` (a representation
change with wide reach) or add a targeted rule where the expected arm is
`Q (union)` — split the value's qualifier list into `Q` and the rest, and test
whether the remainder fits the union — plus the inner wrap in both emitters.

**Workaround, and it is one line**: bind the union first, so the value already
has the arm's type.

```
let good: Ok Str | Err Str = ok("x")
return emitted(good)
```

Verified end to end on both backends with that binding — see
`a_fallible_pass_yields_a_result` in each backend's codegen tests. Nothing is
blocked by it.

## Linear types

**Phases 2 and 3.** The arc's completed stages, the backend-parity principle they rest on, and the
`Place` substrate are in COMPLETED.md ("Roadmap: toward full linear types"
and "Roadmap: place-based flow analysis"). What is left:

### L5 — Places and partial moves (phase 2)

Track paths (`x.field`, tuple/array elements), not just whole variables:
destructuring consumes its source; moving a field out leaves the struct
partially unusable. This is the largest analysis change (place lattice
instead of per-variable states).

- **L5a — answered by shared fate (2026-09-01):** partial moves exist
  as *move-mode bindings* at whole-variable granularity (L1/S2); the
  question left for L5 is only *field-disjoint precision* (using one
  field while another is moved/borrowed), a refinement with no current
  use case. Revisit only if whole-variable poison proves too coarse in
  practice.
- **Not L5: field smart-casting.** Place-based *type narrowing* (reads
  of `h.field` narrowed by `h.field is T`) is a separate feature from
  place-based ownership — it was roadmap phase **P1**, done 2026-09-03.
  L5 inherits its `Place` substrate: the projection type (fields *and*
  elements), the prefix/overlap relations, and per-root fact storage.

What it inherits, and why the sequencing worked out: place-based *ownership*
(using one field while another is moved or borrowed) rides on the substrate P1
built for narrowing — the `Place` type, the prefix/overlap relations, the
per-root fact storage — and the element-projection variant is already there for
ownership's benefit. The substrate was designed and validated under the monotone
feature first, which is what P1-before-L5 was for.

### L7 remainders — recorded, none forced

The L7 milestone landed in four pieces (L7a `<T canbe Linear>`, L7b `Once` fn
types, L7c derived returns, L7d fn-type contracts). What it recorded and did not
build:

- **The internal qualifier unification** (phase 2 of the parameterized-qualifier
  design: folding links/poison/`consumed_by` into parameterized qualifiers on
  the narrowed type, with per-qualifier join directions). Unforced — L7c and L7d
  landed on links alone. The dividend would be diagnostics carrying LSP
  related-information spans from the qualifier's parameters.
- **`Once` inference**: a callee that calls its fn-typed parameter at most once
  does not auto-promote it to `Once`; written only [once-fn]. The same
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

### L8 — Composition and conditional linearity (phase 3, opened 2026-09-08)

Answered together with D6 and D7 (see "The sequence"). Deferred deliberately, and the iterator reduction is what makes it concrete.
`Linear` became a designated obligation group in R4 (see "Decided (user,
2026-09-08): `Linear` becomes a compiler-known obligation group" in
COMPLETED.md), and with it
came an **interim refusal**, built in R4 part 2 and now what
`[linear-composite]` says: storing a linear value in a composite — struct or
handler-state field, type argument, array/tuple/union component — is an error
*at the store* rather than making the composite linear. What L8 has to answer is
therefore not "should containers be refused" but "how does an obligation travel
through one, and when is a container linear at all".

Two things to consider together when this is picked up, both recorded from the
decisions and the R0 prototype rather than guessed:

- **A linear pass cannot be composed.** `map_lazy(open_lines("a.txt"), f)` is
  refused, because a composed pass stores its source. So "can you `map` over a
  file's lines?" is currently *no*, and it is the test case to design against:
  it is the canonical reason to want lazy sequences at all. The user accepted
  the interim uncomposability (2026-09-08) rather than widening the rule early.
- **The fallible-open shape.** S-IO settled on `Ok InputStream | Err Str`, and a
  linear value in a union arm is exactly what the interim rule refuses — so
  S-IO needs either an exception for union arms or a different result shape.

And two spellings that must stay distinct, decided with the group
(2026-09-08): `: Linear` on a *type declaration* is the obligation ("provides a
`close`"), while `canbe Linear` on a *type parameter* is permission ("may be
instantiated with a linear type"). Conditional linearity is where the second
grows teeth — `Wrapper<T>` being linear exactly when `T` is — and it is also
where the composite refusal can be lifted without making every container
linear. The presence of a `close` function never implies either.

## Effects

E1 (handler dependencies, and the Rust fusion behind them), E3 steps 1–3
(`defer`, `throw`/`try`, effects on fn types) and the six ownership strategies
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

### DECISION — two effects may not share a member name

`Symbols::effect_of_fn` maps a member name to one effect, so declaring `emit` on
both `Logger` and `Metrics` is a declaration error today, and the message
already promises the syntax that would fix it: `println@Console(...)`. Lifting
it means a multimap plus every member path — `check_effect_call`, deduction
inference, refinements, the LSP — so it is a milestone of its own rather than an
easy win. The call to make is whether the collision stays an error
([mod-collision]'s reasoning) or the disambiguation form ships.

### Deferred by decision: `platform handler` and `platform type`

Both recorded when the interop redesign landed (user decision 2026-09-05):
a `platform handler` is a host implementation of an *ordinary* Salvo effect,
constructed by `use`; a `platform type` is the type-aliasing gap the user
accepted, with platform structs sketched as the eventual answer ("let's leave
platform type until a need arises"). `platform effect` is the whole interop path
until one of them has a customer.

### Two cuts inside the Rust effect fusion

Both are live divergences — Kotlin accepts them, because generics erase — and
both are *reported* rather than mis-emitted [rs-effect-fusion]:

- a dependent handler using its own generic parameters in a member signature;
- a `use` whose effect instance is still generic.

Each needs the fusion to derive type arguments it does not derive today.

## Iterators

**Phase 1.** The reduction to `next` is complete (R0–R5, plus `for` over a generic pass and
the generic-effect lift); COMPLETED.md holds the phase notes, the two prototypes
that shaped them, and the `Iter<T>` design they replaced. What is left:

### Known cuts, each reported rather than mis-emitted

The list the flip left behind ("Known cuts and gaps" under "R5 part 2 as built"
in COMPLETED.md). Every one of them is a diagnostic today, so none can produce
wrong output [backend-never-wrong]:

- **A callback handed *onward*** to another storing fn stays borrowed on Rust —
  the "does this fn store its callback?" predicate is deliberately
  non-transitive — and rustc reports the lifetime.
- **An `iter fn` with a *generic* subject is refused on Kotlin** [iter-fn], and
  only when it reaches the whole-subject tier: the mint copies the subject, and
  the Kotlin `copy` lowering [kt-copy] cannot decide whether a value typed by a
  type variable holds mutable data. A body that only reads plain fields of it
  avoids the copy entirely — but the per-field snapshot needs the field types,
  which today means a non-generic subject declared in the same file, so the two
  limits reinforce each other. Lifting either means a structural copy Kotlin can
  generate for a generic struct, or making the snapshot type-aware (which is the
  same idea as a per-field snapshot, which the `iter fn` desugaring already does
  where it can [iter-fn]).
- **A *generic* `next` that performs effects** cannot be driven by `for` on
  either backend: its effects live on a fn value the caller supplied, so the
  handlers would have to reach through the implicit rather than being threaded
  per turn. The non-generic case works as of 2026-09-09.
- **A user type named like one of std's passes collides.** Implicit resolution
  matches by name and does not module-qualify a nominal type, so a program
  declaring its own `ListYield` plus `next` makes the `next` ambiguous. Worth
  fixing when someone hits it; the demos were renamed instead.
- **Kotlin's generic union arm reads warn** (`unchecked cast of 'Any?' to 'T'`)
  in std's `seq.kt`, and the same class of warning appears on a nested-union
  bind in user code. A `when` arm over a generic union must use star
  projections, so the element read casts. Cosmetic, but it is in code the author
  cannot edit: `@Suppress("UNCHECKED_CAST")` on the emitted function is the fix,
  and the first thing to do in a polish pass.
- **A linear pass cannot be composed** — the L8 casualty above, repeated here
  because it is the shape people will try: `map_lazy(open_lines("a.txt"), f)`
  stores its source, and storing a linear value in a composite is the interim
  refusal.

Two open *questions* the iterator work forwarded to the qualifier roadmap
rather than answering: **D6** (`Once` on any type — the I2b collision forces it
in a narrower form; see COMPLETED.md) and **D7** (qualifier-conditional
linearity). Both are under "Deductions and qualifier reasoning" below.

### An early-stopping combinator cannot close its source

Recorded when R5 shipped and still open: a combinator that abandons its source
early has no way to release it. A generated machine has a `close` when its body
defers, and nothing in a hand-written driving loop calls it — `map`/`filter`/
`reduce` are safe because they *drain*, and a drained body discharges its own
defers (R0 finding 3). So `take`, `first`, and anything else that stops early
need the `?close` implicit R0 sketched and nothing has built. It is the same
shape as `?Yield`: an optional member resolved at the call site, which is
precisely why rendering (B) was chosen over a bound.

### Deferred: `defer` as an effect with a `defers` block (user, 2026-09-08)


The proposal: a special `defers { … }` block in which a `defer` action is
available, a `Defer` effect for functions that register into an enclosing one,
and all deferred work running at the end of the named block — `Throw`/`try`'s
shape, applied to cleanup.

- **For**: cleanup becomes visible in signatures, and "register cleanup on *my
  caller's* scope" becomes expressible, which is impossible today (a `defer`
  inside a callee runs at the callee's block end).
- **Against**: `defer` currently has **zero runtime representation** — it is a
  splice, "exactly the code written at each of those points" [defer], which is
  why there is no capture question and why it can discharge a linear obligation
  on every path. A dynamic queue costs an allocation and brings the capture
  question back.
- **The iterator argument for it is gone.** Its strongest motivation was
  collapsing the generated machine's per-site flags and giving the release path
  one shape; under the reduction that machinery is confined to generated code
  and stops being language complexity. The restriction floated earlier — that a
  producer may not use an *outer* `defers` block — also dissolves: `next` is an
  ordinary call whose caller is alive for the whole loop.
- **If it is taken**: splice when the registrations in a `defers` block are
  static (all of today's code), and use a queue only where a `[Defer]` function
  actually registers into someone else's block, so existing code keeps its
  current properties.

## Shared mutable state (`Cell`)

**Deliberately not before phase 5** (user decision 2026-09-09): the OTP model has
processes own their state and message-pass, so it may remove this item's
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
`Once`, `ReadOnly`) rather than arriving as a std generic type
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
  `canbe Linear` value would silently drop an obligation; `replace(cell,
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
([iter-mut-param]; the divergence that forced that is under "Open defects").
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
— a producer returning `Once Iter<T>`, where exactly one pass exists — needs no
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

### D6 — `Once` on any type (phase 3, opened 2026-09-07)

`Once` is specified as *fn-type only* ([once-fn], "the language-level
call-multiplicity qualifier"). The iterator rework generalizes it to a
**use-multiplicity** qualifier and applies it to `Iter<T>`, with the fn
case as the instance where using means calling (user decision
2026-09-07; see "Roadmap: iterators — Salvo-level pull iterators" in
COMPLETED.md). The
generalization was accepted; the *scope* was deliberately left narrow.

- **Open question: is `Once` valid on any type?** Nothing in its
  semantics is iterator- or fn-specific — it is an obligation-side
  qualifier the compiler owns, never droppable, with inverted variance
  ([qual-*]: permissions drop, obligations do not), and enforcement is
  the existing consumption machinery [deduce-consume]. So `Once
  FileHandle` or `Once Ticket` would already mean something coherent:
  "use this at most once".
- **Why it was not opened up in the same step** (user decision
  2026-09-07): shipping a general affine qualifier as a side effect of
  an iterator change is how a language surface grows by accident. The
  position list stays explicit — fn types and `Iter<T>` — and widens on
  demand.
- **What to weigh when it comes up**: `Once T` (at most once) sits next
  to `canbe Linear` (exactly once) and `Mut`/`ReadOnly`; a general
  `Once` makes the affine/linear pair complete and user-reachable,
  which is a bigger vocabulary decision than it looks. Also note
  `Once` is *applied* at use sites while `Linear` is *declared*
  ([linear-canbe], "linearity is declared, not applied") — a general
  `Once` would be the first obligation a user can attach to someone
  else's type.

### D7 — Qualifier-conditional linearity (phase 3, opened 2026-09-07)

**Its stated motivation is stale and the case has to be re-derived**: the split
this was written for — `Once Iter<T>` (a pass, which must be drained or closed)
versus plain `Iter<T>` (a factory, which holds nothing) — died with `Iter<T>` in
R5. What survives is the general question, and it is the same machinery question
L8 asks along a different axis: L8 conditions the obligation on a *type
argument*, D7 on a *use-site qualifier*. Answer them together, with D6.

Today linearity is a property of a *declaration*: `canbe Linear` opts a
type in, and every value of it carries the obligation [linear-canbe]
[linear-obligation]. There is no way to say **"the qualified form carries
the obligation, the plain form does not."**

The iterator rework is the first concrete need. `Once Iter<T>` (a pass)
holds a position and may hold a resource, so it must be drained or
closed; plain `Iter<T>` (a factory) holds nothing and needs no disposal.
`canbe Linear` on the `Iter` declaration cannot express that split, since
it would burden the factory too.

- **Not blocking**: the mandatory `close` is *compiler-injected* on every
  exit out of a `for` (decision 4 of the iterator roadmap), so the
  no-leak half is covered without linearity. `Once` covers the no-replay
  half. This item is about making the obligation *visible and checked in
  the language* rather than injected.
- **What it would take**: linearity conditioned on a use-site qualifier —
  i.e. the obligation set becomes a function of the qualified type, not
  of the declaration. The existing all-paths obligation machinery
  (`owes_linear`, `check_linear_exit`, `merge_fallthrough`) would not
  change shape; what changes is which values enter it.
- **Relation to D6**: if `Once` generalizes to any type, this is the
  natural companion — the pair "at most once" (applied) and "exactly
  once" (currently declared) would want the same application mechanism.
  **Decide them together** (user, 2026-09-07: "I think we'll need to design
  the 'Once on any type' (D6) and the 'optional Linear' (D7) together").
  The 2026-09-07 iterator work took the narrow road instead — `canbe Once`,
  an opt-in on the declaration — precisely so that this pair stays a single
  deliberate design rather than an accumulation of special cases.
- **Payoff beyond iterators**: it is the general shape of
  "borrowed handle versus owned resource" without lifetimes — the same
  question `ReadOnly[from: p]` answers for derived returns [readonly-return].


The predicate/constructive split [qual-predicate] [qual-constructive] is
an *evidence* axis — how a value acquires a fact. It says nothing about
what the fact is *about*, which is why `NonEmpty` is legitimately both
(one state claim, two evidence routes [qual-ctor-predicate]). Three
independent axes were identified:

| Axis | Values | Status |
|---|---|---|
| Evidence | predicate (runtime `qualifies`) / constructive (mint-only) | in the spec |
| **Subject** | **state (contents) / provenance (the handle)** | **D5** |
| Authority | user-declarable / compiler intrinsic | implicit |

One combination is impossible and explains the shape of the intrinsics:
a **predicate capability cannot exist** — no inspection of the bits
reveals whether you are *permitted* to write, so `Mut` has no
`qualifies` and never could. Capability/provenance implies constructive;
constructive does not imply capability (`Sorted` minted by `sort()` is
mint-only *state*).

Decided (user, 2026-09-03):

1. **Qualifier declarations gain a subject axis**: *state* (default,
   today's semantics) and *provenance*. Option B of the explored set.
2. **Provenance is a content-independent claim about where the handle
   came from** — mint-only (no `qualifies` body; `is Q` on a non-union
   subject stays a compile error, as [qual-constructive] already says).
   The payoff: a provenance tag **survives an unlisted mutation**.
   D1's stripping rule is sound only for claims about contents —
   [deduce-syntax] says so in as many words ("mutation can invalidate a
   caller's *state* predicates") — so exempting provenance needs no new
   soundness argument. Without this, `Authenticated Request` loses its
   tag to any logger declaring `[req: Mut]`, and D3 refinements would be
   the only escape: per-qualifier-per-function boilerplate for a fact
   that is blanket-true.
3. **Provenance is droppable** (forgetting provenance is safe) and
   **survives being stored into another value**.
4. **Provenance composes without a `with` declaration**, mirroring the
   `Mut` auto-qualifier [type-canbe-mut]: it is orthogonal to every
   claim about contents. Tags stack freely, including several over one
   base (`Authenticated EnvironmentId Str`). `with` [qual-with] keeps
   its original job — two *state* claims co-applying, where explicit
   compatibility is the whole point. Without this rule
   `Ok EnvironmentId Str` would be inexpressible (per-tag
   `qualifier Ok<T> of T with EnvironmentId` does not scale, and
   `Ok (EnvironmentId Str)` is the nested-qualified case [qual-generic]
   calls discouraged) — a newtype that cannot be returned in a `Result`
   is half a newtype.
5. **Hard capabilities stay compiler intrinsics** (`Mut`, `Linear`,
   `Once`, `ReadOnly[from: p]`). Each needs something no user can write:
   a per-backend representation choice (`Mut` is *not* erased —
   `MutableList`, `&mut`), a rule in the flow analysis, a non-standard
   subtyping direction (`Once` inverts it), or a restricted syntactic
   position (`Linear` never at a use site; `ReadOnly` return-only).
   User-authored versions would need a metalanguage for checker rules —
   rejected as out of proportion to Simplicity/Verifiability.
6. **Vocabulary.** What users declare is *state* or *provenance*. What
   the compiler owns are *permissions* and *obligations*, and the
   sub-rule is **permissions are droppable, obligations are not**:
   `Mut Person <: Person` (`map` returns `List<T>` while knowing
   `Mut List<T>` internally), while `Once` may never be dropped and
   `Linear` cannot be written at all.

**Motivating example (record this one — it is more legible than
`Authenticated`): the wrapper/newtype pattern.** The Kotlin habit of
`data class Environment(val id: Id)` with a nested
`data class Id(val value: String)` exists so that an incomplete refactor
is a type error instead of a silently mis-wired `String`. As a Salvo
provenance qualifier (`qualifier EnvironmentId of Str` plus a
constructor), it is content-independent (any string can be an id),
mint-only, and not runtime-testable — and it delivers the protection,
because a bare `Str` does not subtype `EnvironmentId Str`: swapping two
tagged arguments fails to compile in both positions. Only widening is
open, which is the approved droppability, and is the same hole Kotlin
has whenever the target parameter is `String`.

Two properties of the erased design worth knowing before revisiting it:

- **In union position the tag is reified.** `Ok Str | Err Str` already
  works — positional wrappers give arms a physical identity even when
  they erase to the same type, and `is Err Str` matches precisely
  [is-precise]. So `EnvironmentId Str | DeploymentId Str` discriminates
  at runtime; erasure applies to a value in a parameter, not to a value
  in a union.
- **What erasure actually costs** is identity, not usability: two tags
  over `"prod"` compare equal and collide as keys in one map. Display,
  interpolation, base-type operations and (static) overloading all work
  for free — the `toString` override the Kotlin pattern needs exists
  only to undo the boxing, which Salvo never does.

7. **Provenance stays erased** (user decision 2026-09-03, D5a settled):
   uniform with [qual-erasure], so the backends are untouched by D5.
   Reification (a wrapper type per tag per backend) was declined: it
   needs wrap/unwrap insertion at every widening site, double wrapping
   inside unions, and messier interop for std fns taking `Str`, while
   the only benefit — distinct equality and map keys — is already
   reachable with a one-field struct, which *is* the nominal flavor of
   this pattern and needs no new feature. The decisive cost is two
   lowering models for one concept, doubling the checker/emitter
   agreement surface. Kotlin's own tool for this job
   (`@JvmInline value class`) is likewise unboxed except in
   generic/nullable/collection positions, so the erased design is the
   idiomatic trade, not an exotic one.
Namespacing (the former D5b) outgrew this section and is now its own
roadmap item, **N1** — dot-names apply to structs as well as qualifiers,
so it is a naming feature independent of the subject axis.

What landed (rule [qual-subject]):

- **Syntax** (user decision 2026-09-03, confirmed after implementation):
  `provenance qualifier Q of T`, a prefix modifier matching the existing
  `intrinsic`/`external` shape. Plain `qualifier` stays state
  (`QualSubject::State` is the AST default), so nothing existing changed;
  there is deliberately no `state` keyword — one keyword for the
  non-default is enough, and the default is documented. Revisitable if it
  reads badly in practice (there is no compatibility to preserve).
- **Stripping exemption (the payoff)**: `QualEffect::removal_set` now
  takes a provenance predicate and filters the removal set, so a
  provenance tag survives any call. The *inference* side matches
  (`remove_quals` filters, `restrict_to` re-admits the parameter's
  declared provenance quals) so inferred signatures never claim a removal
  that cannot happen. Deduce works from a program-wide provenance name
  set — the deduction machinery is qualifier-name keyed throughout, and
  [mod-collision] already rejects one name declared by two visible
  modules; the checker uses the precise per-file scope.
- **Mint-only**: a `provenance` qualifier with a body is an error naming
  both halves (no `qualifies`, no field overrides). `is Q` on a non-union
  provenance value needed no new code — provenance is bodiless, so
  [qual-constructive]'s existing message fires.
- **Free composition**: the pairwise `with` check in `validate_quals`
  skips any pair where either side is provenance.
- **No backend work**: provenance erases like every qualifier
  [qual-erasure]. The only emitter-visible consequence is *which
  overload* the checker picked.
- Verified end to end on both backends with one program that makes the
  distinction observable in program output: a `Mut List<Int>` carrying a
  state tag and one carrying a provenance tag both go through `add`
  (`[list: Mut]`), then dispatch — `trusted 3` (provenance survived),
  `plain 3` (state stripped), `checked 2` (state intact without
  mutation), identical under `kotlinc` and `rustc`.

Implementation sketch (no backend work — provenance is erased, so
emitters are untouched under D5a-as-recommended):

- `QualifierDecl` gains the subject kind (parser + AST); the checker's
  `validate_quals` enforces the provenance legality rules (no
  `qualifies` body, no `with` needed, `is` on a non-union subject
  rejected — the last one is already [qual-constructive] behavior).
- The removal-set computation in the deduction machinery skips
  provenance tags when a call strips state qualifiers; the *contagious*
  exhaustiveness rule [deduce-syntax] narrows accordingly.
- Stacking validation stops requiring pairwise `with` when either side
  is provenance.

**Future intrinsic capabilities (watch list, in order of how soon they
force themselves on us).** `Sendable`/thread-safety the moment
concurrency lands — structurally inferred, trusted-by-audit for
externals (the `canbe Linear` pattern), and asymmetric between backends,
so it cannot be user-authored. That moment is **phase 5**; see "Threading
and concurrency" for what it collides with today. Then `Uniq`/`Shared` if sharing arrives
(`Uniq` is the precondition for in-place mutation of a shared type),
`Local`/`Escaping` (the conservative refusal of escaping closures
becomes a contract), `Init` for two-phase initialization
(`MaybeUninit`/`lateinit`), and `Const` if compile-time evaluation
lands. Note that `Uniq` and `Local` are facts the fate analysis
*already computes*: they are the user-facing side of the recorded
"internal qualifier unification" leftover, which is the strongest reason
to set the vocabulary up deliberately now.

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

## Standard library surface

S-Str (a mutable string and the string function surface) and S-Seq (the sequence
functions over any pass) landed 2026-09-06; see COMPLETED.md. What is left:

### S-IO — streams, then the filesystem (phase 4)

**Phase 4**, and two decisions are due before it starts: operator typing and
numeric promotion (offsets and sizes are `Long`), and whether two effects may
share a member name (`read`/`write`/`close` collide across `Fs`, the stream
surface and `Console`). See "The sequence".

An `Fs` effect was designed in outline (effect + `intrinsic handler
DefaultFs`, a `File` struct, linear `InputStream`/`OutputStream` as
`intrinsic type … canbe Linear`, errors in the return type because
[effect-member-no-effects] forbids a member from declaring `[Throw<M>]`).
**Deferred by the user 2026-09-06**: IO *streams* should be designed
properly first, with the filesystem as their first customer, rather than
the other way round. Two findings from the outline worth keeping for when
it resumes:

- Errors cannot use `Throw` at all — an effect member may not declare
  effects — so every fallible member returns `Ok T | Err Str`. **Blocked as of
  2026-09-08**: storing a linear value in a composite — a union arm included —
  is now an interim *error* (see "`Linear` becomes a compiler-known obligation
  group"), so this shape needs either an exception for union arms or a different
  result shape before S-IO restarts. That makes
  `Ok InputStream | Err Str` the normal shape, and a **linear value inside
  a union arm** the interaction to verify first ([linear-composite] says
  composites are contagious, but nothing exercises it).
- Whether stream operations are *members* of the effect or free
  `intrinsic fn`s is a testability question, not a plumbing one: only
  members can be faked by a double.

## Threading and concurrency — the OTP model (phase 5)

The intended shape (user, 2026-09-09): the Erlang/Gleam/OTP model — processes
that own their state, message passing, supervision. Nothing is designed yet, so
what follows is not a plan but the list of questions the *existing*
implementation forces, written down so the answers are chosen rather than
discovered mid-phase. All four are **DECISION**s.

- **Sendability, and the `Rc` in generated code.** `Sendable` is already on the
  intrinsic-capability watch list under D7 ("the moment concurrency lands":
  structurally inferred, asymmetric between backends, never user-authored). The
  concrete blocker is representational rather than notational: generated Rust
  holds a fn-typed field as `Rc<dyn Fn…>` [rs-fn-field] — which is *every*
  composed pass, so `map_lazy`'s result — and `Rc` is not `Send`. Either a value
  that crosses a process boundary may not hold one (a rule, and a diagnostic), or
  the representation becomes `Arc`, or fn-typed fields go away again. Phase 1's
  decisions about pass composition should be taken with one eye on this.
- **What effects a spawned process has.** An effect reaches a function as
  `&mut dyn E` borrowed for the call, and the Rust fusion is one value per `use`
  scope holding borrows of the handlers registered in it. Neither crosses a
  thread boundary, so a process cannot inherit its parent's handler set by
  reference. The likely answer — a process starts its own `use` scope — has to be
  stated, because it decides whether `spawn` takes a handler set, and because a
  handler *shared* between processes is the `Cell` question arriving from the
  other side.
- **Processes: OS threads or a runtime.** The JVM has threads and coroutines;
  Rust has threads and no runtime in std. A green-thread model needs a scheduler
  inside generated code on at least one backend, which would be the largest piece
  of machinery the two backends do not share. One OS thread per process with a
  blocking `receive` is the cheapest thing that is *the same* on both, and the
  parity principle is the reason to prefer it until something forces otherwise.
- **Whether async survives at all.** Recorded 2026-09-04: async arrives later as
  an *explicit* effect, never as silent `async`/`suspend` colouring. If a process
  blocks on `receive`, the OTP model may remove the need for it entirely — worth
  answering before building either, since the two designs overlap.

**A prerequisite that will surface immediately**: returning or storing a
capture-carrying closure is a rustc lifetime error the checker does not reject
[fate-lambda] — and `spawn(() -> …)` is exactly that shape. The recorded
refinement is `move`-closure emission with hoisted clones, plus a treatment for
captured effect-handler locals. It is phase-5 work rather than a leftover.

**What is already in place, and should not be re-invented.** Message *ownership
transfer* is ordinary consumption: a `send` that moves its argument is a plain
deduction list, and the flow analysis already rejects use-after-send with a
diagnostic that names the call. Linearity composes with it for free — "whoever
ends up with this handle must close it" survives a send, because moves transfer
the obligation [linear-obligation]. Effects give supervision somewhere to live
without new machinery: a supervisor is a handler.

## Consolidated leftovers

Small recorded remainders, each also noted in its own milestone section or spec
rule, collected here for findability. They are not a queue: nothing here is
blocking, and several are "revisit only if a customer appears".


- **`Once` inference**: a callee calling its fn param at most once does
  not auto-promote to `Once`; written only [once-fn]. Same
  written-validates/unwritten-infers pattern as deductions when taken.
- **Returning/storing capture-carrying closures**: rustc lifetime error
  the checker does not reject; the recorded refinement is
  `move`-closure emission with hoisted clones, pending a treatment for
  captured effect-handler locals [fate-lambda].
- **Exactly-once closures**: a `Once` lambda may not consume a *linear*
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
  handler state, effect/handler/qualifier members. Still open: `[symbol]`
  resolution is name-based over the AST rather than
  import-visibility-exact [doc-symbol-ref]. Also still open:
  incremental analysis if workspaces outgrow
  re-check-everything-per-keystroke, and a `positionEncoding` negotiation
  for UTF-8-native clients. Signature *hover* still covers fn decls only
  — effect members and define fns have no `FnKey` (go-to-definition does
  reach effect members, via `def_refs`).
- **Predicate `is` on a union subject** is now roadmap phase **D4**, not
  a leftover: a `Ty::Union` subject always takes the arm-matching path,
  so `x is Positive` on `Int | Str` errors ("this check can never
  succeed") instead of calling `qualifies` — narrow first
  (`x is Int && x is Positive`). Lifting it requires qualifiers over
  unions [is-qualifies] [qual-union-arm].
- Struct destructuring ignores predicate-qualifier field overrides
  (deliberate: bindings get the declared type; direct accesses get the
  override + cast).
- Constructing a *nested* qualified union group in one expression
  (`ok(ok("yes"))` into `Ok (Ok Str | Err Int) | …`) needs an annotated
  intermediate `let`; single-level coercion only (errors, never mis-emits).
- Deduction inference does not track bare-parameter value flow out of
  branch/loop tails as a move (documented leniency in [deduce-infer]).
- **Array elements never narrow** ([flow-place] narrows variables, field chains
  and tuple positions): an unknown index may alias any element, so a constant
  one is not treated specially either — the expectation it would set is the
  reason.
- Module reachability is name-based and conservative: a local variable
  shadowing a std fn name still pulls that std module in (harmless
  extra output, never a missing module).
- **DECISION (open, for the user):**
  - Binary operators are typed only for `None` [op-no-none]: everything
    else is unchecked (`Str * Bool` passes, result typing is just the
    left operand's type). Decide the operator typing rules — legal
    operand types per operator, numeric promotion, `Bool` for `&&`/`||`.
