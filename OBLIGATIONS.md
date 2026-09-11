# Obligations — the phase-3 option space (working document)

Status: **draft for discussion, 2026-09-11**. Nothing here is decided; every
DECISION point below is the user's call (AGENTS.md's first invariant). This
document exists to frame L8, D7 and D6 — "The sequence" phase 3 — as one
design question and lay out the options with trade-offs and recommendations.
Once the calls are made, the outcomes belong in COMPLETED.md's decision log
and the rules in LANGUAGE_SPEC.md; this file is scaffolding and should not
outlive the phase.

Sources: ROADMAP.md ("Linear types → L8", "Deductions and qualifier
reasoning → D6/D7", "The sequence"), COMPLETED.md (R4 parts 1–2, L6/L7
records), LANGUAGE_SPEC.md ([linear-group], [linear-obligation],
[linear-composite], [linear-generics], [once-fn], [group-obligation]).

---

## 1. The frame: an obligation qualifies a type and restricts how its values must be treated

The general pattern (user, 2026-09-11): **an obligation is a qualifier-shaped
thing that puts extra restrictions on how values of a type must be treated.**
`Once`, `Linear` and `Proj` are all obligations in this sense. The vocabulary
already half-exists — the D5 decision split the compiler's intrinsics into
*permissions* and *obligations* with the sub-rule "permissions are droppable,
obligations are not" — and all three fit its signature pattern:

- **non-droppable**: forgetting the qualifier would forget a restriction, so
  widening it away is unsound (`Once` may never be dropped [once-fn]; `Proj`
  cannot be treated as an owned value; `Linear` cannot even be written at a
  use site);
- **the obligated type is a *supertype* of the plain type** (user,
  2026-09-11): a `T` can always be given where a `Linear T`, `Proj T` or
  `Once T` is expected — an owned value stands in for a borrow, an
  any-number-of-times value stands in for an at-most-once one, and
  discharging a value that never demanded it is harmless. So plain
  `T <: Once T` [once-fn], and generally `T <: Obligated T` — the exact
  opposite of a permission like `Mut`, which refines and drops
  (`Mut Person <: Person`). Qualifiers narrow; obligations widen.

What distinguishes the three is *which* restriction they carry:

| obligation | restriction axis | the restriction |
|---|---|---|
| `Once` | **multiplicity** | use at most once (`[0,1]`, affine) |
| `Linear` | **multiplicity** | use exactly once (`[1,1]`), terminal use from a **discharge set** |
| `Proj` | **placement/escape** | a borrow: may not outlive its source `[from: p]`, may not sit in a struct field [proj-no-field] |

(`Reg`'s escape rule — nothing carrying a region's provenance leaves its
delimiter — is the same *shape* of restriction on the placement axis, even
though `Reg` itself is classified as provenance; worth noticing because it
means the escape machinery is shared, not because regions carry obligations —
they decidedly do not, §2.)

One further split inside the pattern does real work in §5: an obligation is
either **restraint** — negative, "you may not do X" (`Once`, `Proj`) — or a
**duty** — positive, "you must do X before this value dies" (`Linear`'s
must-discharge). Restraints can be safely *self-imposed at any use site*,
because they demand nothing of the type's author or of other holders. A duty
must be declared or derived, because someone has to guarantee it is
dischargeable on every path.

For the multiplicity axis specifically, a value's discipline is the pair
**(bounds, discharge set)**:

| bounds | name | in Salvo today |
|---|---|---|
| `[0, ∞]` | plain | every ordinary value |
| `[0, 1]` | affine | `Once` — fn types and `canbe Once` opt-ins [once-fn] |
| `[1, 1]` | linear | `: Linear<self>` declarations [linear-group] |
| `[1, ∞]` | relevant ("must use") | *does not exist* — see §5 |

The **discharge set** is which uses may be the *terminal* one. For a linear
value today it is a singleton: the designated `close`, whose own parameter is
exempt because that is where the value legitimately dies [linear-group].
Every other move merely *transfers* the obligation [linear-obligation].

The three roadmap items are then one question asked along three axes:
**what are a value's obligations a function of?**

| axis | today | the item |
|---|---|---|
| the **declaration** | yes — the only input | baseline |
| the **discharge set's shape** | singleton unary `close` | §3 (your stop/join and cache examples) |
| the **type arguments** | no — containers refused instead [linear-composite] | **L8**, §4 |
| a **use-site qualifier** | only `Once`, only on fn types / opt-ins | **D7 + D6**, §5 |

The unifying candidate rule, stated once so the options below can be judged
against it:

> **(U)** The obligation set of a value is computed from its **fully
> instantiated, fully qualified type** — declaration first, then type
> arguments, then use-site qualifiers — and moves transfer whatever that
> computation yields.

Everything in §3–§5 is a choice about how much of (U) to build and how each
input is *spelled*.

### The syntax question: should obligations look different from qualifiers?

The supertype observation cuts against the current surface: `Proj T` and
`Once T` are written exactly like `Mut T` and `NonEmpty T`, but they point the
other way — a qualifier *narrows* (drop it and you still have the value's
type), an obligation *widens* (drop it and you have claimed a capability
nobody granted). Today the reader learns the direction per name; nothing in
the spelling says it (user, 2026-09-11: this suggests a syntax difference
between qualifiers and obligating types).

Options:

- **O-S1 — status quo.** One prefix position for both; direction is
  vocabulary. For: the obligation class is closed and small — D5's decision 5
  keeps hard capabilities compiler-intrinsic, so users never declare one, and
  three names (`Once`, `Proj`, and `Linear` which is never written at use
  sites anyway) may not earn a syntax. Against: the two most safety-relevant
  words in a signature are visually indistinguishable from ordinary claims.
- **O-S2 — a marker on the obligation.** Same position, a distinguishing
  sigil or keyword — e.g. `Once! T` / `Proj! T`, or a word (`must Once T`).
  For: minimal grammar change, greppable, teachable ("`!` means restrictions
  apply"). Against: `!` is heavily loaded already (assert, `!is`, not — the
  adjacency-lexing rule would need a fourth case); a word is noisy at every
  use site of what 2b is about to make the *common* return shape
  (`(Proj T)?` everywhere in std).
- **O-S3 — a distinct syntactic slot.** Obligations after the type or behind
  a separator (`T where Once`, `T + Proj[from: xs]`), leaving prefix position
  purely for qualifiers. For: the direction is structural — prefix narrows,
  suffix widens — and stacking order questions (`Once Proj T`?) dissolve.
  Against: the largest churn; 2b *just* made `Proj`'s prefix **position**
  meaningful (`List<Proj T>` view vs `Proj List<T>` borrowed list, decided
  2026-09-11), and a suffix form has to re-express that distinction some
  other way.

**Timing note, whichever way it goes**: 2b's steps 2–3 are about to spread
`Proj` through every std signature. If the spelling is going to change, decide
before that sweep, not after.

No recommendation is offered with confidence here — O-S1 is the cheapest and
O-S3 the most honest, and the deciding factor is how `(Proj T)?` reads at
scale, which the 2b audit is about to show. One forward constraint whichever
way it goes: the chosen spelling should still admit a *bound variable* in the
obligation position (`q canbe Once`, then `f: q (A) -> B`) — see
"Multiplicity polymorphism, and the pipeline connection" in §5, whose
scheduled customer is the post-phase-5 pipeline surface. **DECISION §1-1**:
keep unified spelling, mark obligations in place, or move them to their own
slot — and if either of the latter two, sequence it against 2b's std sweep.

---

## 2. Fixed points — decided, not reopened here

Listed so the option space doesn't accidentally wander into them:

- **Regions manage memory, never obligations** (R2 rejected, user
  2026-09-10). No bulk discharge at a scope close — your stop/join and cache
  examples are the argument, and they hold.
- **A `close` never implies linearity** [linear-group]; obligations attach by
  declaration, never by function-name convention [qual-*].
- **`discard` is not an escape hatch** for linear values [linear-discard].
- **The two spellings stay distinct**: `: Linear<self>` on a declaration is
  the obligation; `canbe Linear` on a type parameter is permission
  [linear-generics].
- **Multi-shot resumption stays closed** — resuming twice duplicates an
  obligation.
- **Linearity is static and backend-identical** [linear-static]: no
  destructors, no runtime component. Every option below must be checkable
  by the existing all-paths consumption machinery (`owes_linear`,
  `check_linear_exit`, branch merges) or a stated extension of it.
- **Aliases owe nothing**: fate-linked derivations are not obligation
  holders [linear-obligation]. Phase 2b sharpens this — see §6.3.

---

## 3. Axis one: the shape of the discharge set

Your two 2026-09-10 examples each break the current singleton-`close` shape
in a different direction, and they are independent:

**(a) Alternatives** — a thread handle is discharged by `stop` *or* `join`.
**(b) Context** — a cache handle is discharged by `remove(cache, handle)`:
the discharge takes *another value* besides the dying one.

### What the mechanism already almost supports

Discharge is already "a move into the designated member, whose implementation
is exempt". Generalizing the *designation* from one member to a set, and the
*exemption* from "the `close` parameter" to "the consumed self-typed parameter
of any set member", is mechanically small. The design work is the spelling
and the group semantics — [group-obligation] currently requires **every**
member of a group to be satisfied, which is the wrong connective for
alternatives (a thread handle satisfies `stop` and `join`; a file satisfies
only `close`).

### Options

**O-D1 — keep `Linear<It>` as the group, list discharges at the type.**
The group keeps requiring nothing beyond membership; the *type* names its
discharge set:

```
struct Thread : Linear<self by stop, join> { … }
struct Lines  : Linear<self> { … }              // sugar: by close
```

Each named fn must be a visible overload that **consumes** a `self`-typed
parameter; other parameters are ordinary (the cache case works with no
further rule — the call site satisfies them like any call). Default `by
close` preserves every existing declaration.

- For: one group, one obligation concept; the default keeps today's surface;
  alternatives and context both fall out of "named consuming overloads".
- Against: `by` is new syntax inside the obligation clause; the group's
  `close` member becomes a default rather than a requirement, which bends
  [group-obligation]'s "every member satisfied" for this one group.

**O-D2 — the group itself declares alternatives.** Extend `params` with an
any-of marker (`params Linear<It> { any fn … }`) so groups can state
disjunctions generally.

- For: the mechanism stays in the group where [group-obligation] lives;
  reusable if another group ever wants "any of".
- Against: heavier — a new group-semantics feature for exactly one customer;
  and it still can't express *per-type* sets (Thread's set differs from
  File's, but they'd share one group declaration). Effectively forces one
  group per discharge-set shape, which multiplies designated groups.

**O-D3 — implicit: any visible consuming fn over the type discharges.**

- Rejected on the fixed points: this is discharge-by-convention, exactly what
  "a `close` never implies linearity" exists to prevent, and it silently
  widens whenever anyone writes a consuming fn.

**Recommendation: O-D1.** The set belongs to the *type* (Thread's is not
File's), the default keeps the migration at zero, and the checker change is
localized: designation, the exemption predicate, and the leak diagnostic
naming the set ("`stop` or `join`") instead of `close`.

**DECISION §3-1**: discharge-set spelling — O-D1 / O-D2 / keep singleton.
**DECISION §3-2** (only if O-D1): must every `by` entry consume exactly one
`self`-typed parameter (recommended: yes, and it must be by-move), and is a
generic discharge (`fn recycle<T>(pool: Mut Pool<T>, h: Handle<T>)`) allowed
from day one?

---

## 4. Axis two: L8 — obligations through containers

The interim rule [linear-composite] refuses the store outright. Its two
recorded casualties: a linear pass cannot be composed (no lazy `take`, no
wrapper over `open_lines`), and S-IO's `Ok InputStream | Err Str` is
unwritable. The question is the travel rule. The options are ordered from
smallest lift to largest; they are **cumulative**, not exclusive — each keeps
the previous ones' semantics.

### O-C1 — status quo (refusal)

Not an endpoint: phase 4 is blocked on it. Listed only as the fallback if a
larger option stalls mid-phase.

### O-C2 — union arms stop being containers

The checker already believes this for *inferred* unions: `ty_own_linear`
counts "a union with a linear arm" as linear, because a union value **is**
the value — one handle, not a box holding one (R4 part 2's "one subtlety
worth keeping"). O-C2 makes the *written* form legal too:
`fn open(path: Str) -> Ok InputStream | Err Str`.

Semantics to fix (the actual decision):

- A value of `Ok InputStream | Err Str` **owes** while un-narrowed.
- Narrowing to the linear arm: the narrowed value owes; discharge as normal.
- Narrowing to a **non-linear arm discharges the obligation** — an `Err Str`
  never held the handle. This is the one new rule, and it is what makes the
  fallible-open shape usable: the `Err` path closes nothing.
- Branch merges already do the right thing (consumed-on-all-paths), because
  the union case survived R4 for exactly this reason.
- `T?` (i.e. `T | None`) falls out for free — an optional handle, `None`
  owes nothing. This is also the shape a *lookup* of a linear value wants.

Cost: small — mostly deleting the union-arm refusal site and adding the
narrow-to-non-linear-arm discharge, plus emitter agreement checks (arm
identity is untouched; positional arms already reify). **This option alone
unblocks phase 4**, and it is separable from everything below.

### O-C3 — conditional linearity for containers (declared)

The travel rule proper: a container is linear **exactly when a linear type
reaches one of its fields**, and its obligation is its own — discharged by
its own discharge set. Two sub-choices:

**Spelling — declared, not inferred.** A generic struct that may hold a
linear `T` says so, and says how the *container* discharges:

```
struct Box<T canbe Linear> : Linear<self by unbox> {
    item: T
}
fn unbox<T canbe Linear>(box: Box<T>) -> T   // consumes box, hands the obligation on
```

`Box<Lines>` is linear; `Box<Int>` is plain — the obligation clause is
**conditional on the instantiation**: it holds iff some type argument that
reaches a field is itself linear. `canbe Linear` on the parameter stays pure
permission; the `: Linear<self …>` clause is still the only obligation
spelling, so the fixed-point split survives. (The alternative — inferring
containment and making any struct with a linear field silently linear — is
the pre-R4 contagion L6d, rejected once already: a container nobody declared
a discharge for has no legal death, which is precisely the hole the R4 sweep
found in the `discard`-a-`List<FileHandle>` tests.)

**What discharge means for a container: hand the contents on, don't bulk-free.**
`unbox` returning `T` transfers the obligation to the caller — consistent
with "regions never bulk-discharge". A container's discharge fn may of course
*itself* close contents it can reach (`fn close(b: Box<Lines>) { close(b.item) }`
— legal, ordinary code); what the language never does is free contents
implicitly.

**Intrinsic containers** (`List<T>`, arrays, tuples): the same rule needs a
home without a struct declaration to carry it. Options: (i) `List<T>` gains a
conditional obligation with drain-style discharges (`remove` hands an element
out — note it is your cache example verbatim); (ii) intrinsic containers stay
refused for linear elements in v1, only user structs and unions open up.
Recommendation: **(ii) first** — nothing in the recorded casualties needs a
`List<Lines>`, the wrapper-pass case needs exactly one field, and (i) can
follow as pure std/audit work once the struct rule is proven.

**What this buys**: the wrapper pass. A composed pass stores its source pass
as a field; under O-C3 the wrapper is linear iff the source is, its discharge
is a `close` that closes the source, and `[iter-drive-in-place]`'s
release-on-every-exit already knows how to drive that. Lazy `take` becomes
writable. (Whether *std's* combinators become lazy again is a phase-4/laziness
question — the recorded direction is pipelines-of-functions — but the
language stops forbidding the shape.)

**O-C3 is elided multiplicity polymorphism** (observation 2026-09-11, while
working through the pipeline connection in §5). What "conditional by default"
implicitly says is: *the container's obligation is a variable bound to its
element's obligation*. Written with an explicit qualifier parameter (made-up
syntax, same as §5's `compose`):

```
struct Box<T ~ q> : q<self by unbox> {   // q captures T's obligation;
    item: T                              // Box re-emits it, with its own discharge
}
```

O-C3's surface is this with `q` inferred — the way Rust elides the lifetime
that is almost always meant, and the way deductions are unwritten-infers /
written-validates. Three consequences, none requiring the explicit form to
be built now:

- **Define the checker's model in these terms** — obligations(Box<T>) is a
  function of obligations(T), computed at instantiation — so that if a
  container ever needs a *non-default* relationship (forwarding only one of
  two parameters' obligations, or absorbing one into its own discharge), the
  explicit parameter is an unveiling, not a redesign.
- **O-C2 slots in as the degenerate case**: a union does not *forward* its
  arm's obligation — the value **is** the arm, so there is no container-level
  obligation to derive and no discharge set of its own. One model covers
  both, with the union as identity.
- **The same relationship already exists on the placement axis**: 2b's
  `List<Proj T>` view is a container whose escape restriction derives from
  its elements' (the view is fate-linked to the source). It is being handled
  ad hoc there, correctly; if a third axis-through-container case appears,
  "container obligations derive from element obligations, per axis" is the
  general rule all three are instances of.

The explicit spelling stays unbuilt for the same reason as §5's `q canbe
Once`: no customer needs a non-default relationship yet, and the elided form
loses nothing — it is the same semantics with the variable named by the
compiler instead of the author.

### O-C4 — obligations as parameterized qualifiers (the internal unification)

The L7 remainder ("folding links/poison/`consumed_by` into parameterized
qualifiers on the narrowed type") taken now, making `Linear` a qualifier the
*checker* manipulates on types, so containment falls out of type composition
instead of a bespoke predicate.

- For: (U) verbatim; diagnostics gain related-information spans; D7 becomes
  trivial afterwards.
- Against: the largest refactor of the checker's hottest tables, taken as a
  *prerequisite* rather than a dividend; L7c/d proved links alone suffice.
  Recorded as unforced then, and O-C2+O-C3 do not force it either.

**Recommendation: O-C2 + O-C3 (with intrinsic containers deferred), skip
O-C4 this phase.** O-C2 is the phase-4 unblocking move and is near-free;
O-C3 is the real design and stays within the declared-obligation discipline.

**DECISION §4-1**: accept O-C2's semantics — written linear union arms legal,
narrowing to a non-linear arm discharges?
**DECISION §4-2**: O-C3's conditionality spelling — is `: Linear<self by …>`
on a generic struct *conditional by default* (holds iff instantiation makes a
field linear), or is there an explicit marker (`: Linear<self> when T`)?
Recommendation: conditional by default — an unconditional container of
non-linear things being linear is expressible by not being generic, and a
`when` clause is notation for a distinction with no second customer.
**DECISION §4-3**: intrinsic containers — deferred (recommended) or in scope?

---

## 5. Axis three: D6 + D7 — obligations at the use site

L8 conditions the obligation on a type argument; D7 conditions it on a
use-site qualifier; D6 asks whether the affine qualifier may appear on any
type at all. In the §1 frame: may **bounds** be attached where a value is
*used*, not only where its type is *declared*?

The asymmetry worth keeping in view: tightening an upper bound (`[0,∞]` →
`[0,1]`, i.e. `Once`) **restricts the holder** and can never be unsound —
the value's author loses nothing when a user promises to use it less. Adding
a lower bound (`[0,∞]` → `[1,1]`, D7's conditional linearity at a use site)
**imposes on every path** and changes what code downstream must do. That
asymmetry suggests splitting the pair:

### D6 — `Once T` anywhere: recommend **yes, as the upper-bound story**

- Semantically complete already: inverted variance (`T <: Once T`), never
  droppable, enforcement is [deduce-consume]; `once_position` +
  `has_auto_once` just stop gating positions.
- It is the first obligation a user attaches to someone else's type — but an
  upper bound is a *self-restriction*, so the "should not attach to someone's
  type" principle [once-fn] is not actually violated: nothing is demanded of
  the type's author or of other holders.
- Vocabulary payoff: the affine/linear pair completes, and "at most once /
  exactly once / applied / declared" becomes a teachable 2×2.
- Residual from L7: `Once` *inference* (a callee calling its fn param once
  auto-promoting) stays not-built — written-validates, as recorded.

### D7 — use-site linearity: recommend **no new surface this phase**

Its motivating case died with `Iter<T>`; the ROADMAP says re-derive it from
L8's container question — and O-C3 *is* the re-derivation: `Wrapper<T>`
linear exactly when `T` is covers the "conditionally linear" need that
survives, through the type-argument axis rather than a use-site qualifier.
What D7 would add beyond that is a plain type made linear at a use site
(`Linear Handle` written on a local?), and:

- `Linear` at a use site is exactly what [linear-group] rejects ("a
  per-value qualifier that could be forgotten would defeat the protection");
- no recorded example needs it — the thread and cache handles are declared
  linear; the pass is conditionally linear via its source.

Keep D7 open as a question with zero surface: if a customer appears, the
machinery O-C3 builds (obligation computed from the instantiated type) is
the mechanism it would ride on.

**DECISION §5-1**: D6 — `Once` valid on any type (recommended: yes)?
**DECISION §5-2**: D7 — accept "covered by O-C3, no use-site `Linear`,
revisit on a customer" (recommended)?

### Other potential obligations (the survey asked for)

Held against the (bounds, discharge) frame to see whether the design being
chosen accommodates them *without* being built now:

| candidate | shape | verdict |
|---|---|---|
| **relevant / "must use at least once"** (`[1,∞]`, Rust's `#[must_use]`) | lower bound, no discharge set — any use satisfies | fits the frame; warning-grade, not error-grade. Do not build now; note that [unused-var] already covers the local-variable half. |
| **commit-or-rollback** (transactions) | `[1,1]` with discharge set `{commit, rollback}` | **is** O-D1, second customer after stop/join. No new design. |
| **pooled/recycled handles** (`return_to(pool, h)`) | `[1,1]`, context-carrying discharge | is the cache example. O-D1 covers it. |
| **"must be awaited"** (futures, phase 5) | `[1,1]`, discharge `{await, cancel}` | O-D1 again — evidence the set shape earns its keep before phase 5. |
| **typestate** ("open before read, read before close") | *ordered* protocol, not multiplicity | **out of scope, deliberately**: obligations here are counts + terminal sets, never state machines. Worth writing down as a boundary so the feature doesn't creep toward session types. |
| **`Sendable`** (phase 5 watch list) | capability, not obligation | unaffected; different axis. |
| **`Reg`** (regions) | provenance, not obligation | fixed point: regions never carry obligations. |

The pattern: everything with a real customer is (bounds, discharge-set) —
which is the argument that §3 + §4 are the whole general mechanism, and no
"obligation metalanguage" is needed (matching the D5-era rejection of
user-authored checker rules).

### Future obligations: other languages, and what OTP brings (surveyed 2026-09-11)

Candidates filtered through §1's two tests (non-droppable; `T <: Obligated T`)
and the restraint/duty split. Nearly everything lands on the two existing
axes, which is evidence for the frame; the one new axis anything would
introduce is *order*.

**More multiplicity:**

- **Relevant `[1,∞]`** — Rust `#[must_use]`, C#'s unawaited-`Task` warning.
  Warning-grade everywhere it exists; already in the table above.
- **Multiplicity polymorphism** — Linear Haskell's `a %m -> b`: abstraction
  over the bound itself, so one combinator serves `Once` and plain callbacks.
  The pressure that arrives *after* phase 3 makes bounds real;
  `<T canbe Linear>` is the coarse (worst-case, not polymorphic) version.
  Known next step, deliberately not taken now — expanded in full below
  ("Multiplicity polymorphism, and the pipeline connection"), because the
  laziness/pipeline direction is its scheduled customer.

**More placement:**

- **Rust `Pin`** — "may never be moved again". A restraint, self-imposable,
  passes both tests. Its customer is self-referential state, which arrives
  with suspension — if phase 5 keeps async, this may follow it in.
- **Swift `@escaping`'s inverse / the `Local`/`Escaping` watch-list entry** —
  "may not outlive this scope"; shares the escape machinery with `Reg`.

**A discriminating negative: uniqueness is not an obligation.** Clean's
unique types / Pony's `iso` fail the supertype test in the telling direction:
a plain `T` cannot stand where `Uniq T` is expected (the callee relies on
no-aliases), and uniqueness may be safely forgotten. Narrows and drops →
qualifier/permission, where the D7 watch list already has it. The frame
classifies it correctly without help.

**What OTP (phase 5) brings** — mostly second customers for §3's discharge
sets, plus one boundary test:

| candidate | shape | note |
|---|---|---|
| **reply token** (`gen_server`'s `From`; Gleam's `Subject`) | `[1,1]`, discharge `{reply}` | the best future customer: a duty on a value the *runtime* mints — no reply is a caller timeout, double reply is a bug |
| **monitor reference** | `[1,1]`, discharge `{demonitor, receive_DOWN}` | stop/join shape verbatim |
| **process handle** | `[1,1]`, discharge `{stop, join}` | already recorded under L8 |
| **send-is-move** | ordinary consumption | already covered; no new obligation |
| **supervision** ("a spawned child must end up linked or supervised") | placement *duty* — "must be stored somewhere specific" | neither axis expresses it; likely better served by API shape (spawn only through a supervisor) than by the type system. The case to check the frame against, not a recommendation |
| **session-typed channels** ("send `Ping`, then receive `Pong`, then close") | *ordered* obligations | the typestate boundary's first real test. Position: keep the boundary; exactly-once reply tokens carry most of the practical load |

Internal note: the `Init` watch-list entry ("must initialize before use") is
the only existing candidate that is a mini-typestate rather than a count or a
placement — it sits on the excluded side of the boundary, and should be
recognized as such when it comes up.

### Multiplicity polymorphism, and the pipeline connection

The duplication problem that appears the moment use-bounds become part of
types: every higher-order function must choose what bound to demand of its
function argument, and every choice is wrong for someone. Suppose D6 lands
and `Once` is general; now write `compose`:

```
fn compose<A, B, C>(f: (A) -> B, g: (B) -> C) -> (A) -> C {
    return (a: A) -> C { g(f(a)) }
}
```

A caller holding `Once` fns cannot use it — passing a `Once (A) -> B` where
a plain fn is expected is exactly the widening obligations forbid (the body
might call `f` twice; nothing stops it). So a second overload appears:

```
fn compose<A, B, C>(f: Once (A) -> B, g: Once (B) -> C) -> Once (A) -> C
```

The combinator surface doubles (a mixed `f`/`g` is a third signature), and
the bodies are *identical*: the bound is the only thing varying, and nothing
abstracts over it.

**What Linear Haskell does**: a multiplicity on the arrow (`a %1 -> b` uses
its argument exactly once, `a %Many -> b` is unrestricted), and then a
multiplicity *variable* — `map :: (a %m -> b) -> [a] %m -> [b]`: however the
callback treats its element is how `map` treats the list. One signature,
both worlds, and the input/output bound relationship is *expressed*, not
just tolerated. Idris 2 and Granule generalize further (quantitative/graded
types — multiplicities as a semiring, including `0` for erased
compile-time-only arguments); the variable is the core move.

**A made-up Salvo spelling** — a *qualifier parameter*, reusing `canbe`:

```
fn compose<A, B, C, q canbe Once>(f: q (A) -> B, g: q (B) -> C) -> q (A) -> C {
    return (a: A) -> C { g(f(a)) }
}

let plain = compose(double, stringify)      // q = nothing: plain (Int) -> Str
let once  = compose(consume_token, format)  // q = Once:    Once (Int) -> Str
let mixed = compose(consume_token, double)  // q = Once — join goes upward
```

Three properties, each falling out of machinery Salvo already has:

1. **The body checks at the worst case** (`q = Once`, so `compose` may call
   `f` and `g` at most once each) — precisely the `<T canbe Linear>` pattern
   [linear-generics]: check under the strongest assumption, and every
   instantiation is sound with no per-instantiation re-checking.
2. **Mixed arguments unify upward, and §1's supertype rule is why that is
   sound.** One `Once` and one plain argument forces `q = Once`; treating
   the plain `g` as `Once` is safe because `T <: Once T` — self-imposing a
   restraint demands nothing of anyone. Obligations widening is what makes
   the join direction well-defined (a permission like `Mut` could not do
   this — its joins go the other way).
3. **The output carrying `q` is the whole point.** Without propagation the
   choice is: always return `Once` (needlessly restrictive for plain inputs
   — the composition can never be called twice) or always plain (unsound
   for `Once` inputs). The worst-case *parameter* side exists today; the
   propagation is the missing feature.

**O-C3 is already this, in container form**: "`Box<T>` is linear iff `T` is"
is an obligation flowing through an instantiation, with the type parameter
itself as the multiplicity variable (§4 develops this identification — the
elided-variable reading is the recommended way to *model* O-C3 in the
checker). The variable form is needed only when the bound varies
*independently* of the type — `compose`'s `A`, `B`, `C` say nothing about
whether the fns are `Once`.

**The pipeline connection (the flag this subsection exists for).** The
recorded post-phase-5 laziness direction — "compose functions, `iter fn`s
included, into pipeline functions" (ROADMAP "Laziness, after concurrency") —
is a combinator surface over fn values. A pipeline stage can already *become*
`Once` today: a lambda that consumes a linear capture is `Once`-typed
[once-fn]. The moment such a stage enters a pipeline, the pipeline's own
multiplicity must derive from its stages' — and that derivation *is*
multiplicity polymorphism. So the pressure arrives with the pipeline design,
not before. Two consequences for sequencing:

- **Nothing in phase 3 should foreclose it.** The `q canbe Once` spelling is
  a reason to keep the `canbe` clause shape as-is: it has this generalization
  sitting in reserve. If DECISION §1-1 changes the obligation syntax, check
  the chosen spelling still admits a bound *variable*.
- **Nothing in phase 3 should build it.** Its only firm customer is the
  pipeline surface, two phases away; qualifier parameters now would be
  surface without a caller.

Scope caveat: Linear Haskell also uses `%m` to relate a *data structure's*
consumption to its elements' (the `map` signature above). Salvo answers
those questions structurally — conditional containers for linearity, `Proj`
views for borrowing — so the fn-type case is likely the only place the
variable form is ever needed: a much smaller feature than the Haskell paper
ships.

---

## 6. Shapes to design against (acceptance cases)

1. **S-IO fallible open** — `fn open(path: Str) -> Ok InputStream | Err Str`;
   `when` on it; `Err` path closes nothing. Needs O-C2 only.
2. **Thread handle** — `struct Thread : Linear<self by stop, join>`; a fn
   that stops on one branch and joins on the other is clean; one that does
   neither on some path errors naming *both*. Needs O-D1.
3. **Cache handle** — discharge `remove(cache, handle)` where the call site
   must have the cache in scope. Needs O-D1's context rule.
4. **Wrapper pass / lazy `take`** — a generated or hand-written pass storing
   a linear source pass; `for` over it releases on every exit through the
   wrapper's `close`. Needs O-C3 + the [iter-drive-in-place] plumbing.
5. **Optional handle** — `let h: InputStream? = …` and the `None` branch owes
   nothing. Falls out of O-C2.
6. **View of linear elements** (the 2b interaction) — `List<Proj T>` where
   `T` is linear: a view stores *borrows*, and aliases owe nothing
   [linear-obligation], so a view of linear values should owe nothing and
   never discharge anything. State it as a rule and test it, because it is
   the first time "borrowed" and "linear" compose in a container.
7. **The smuggling hole** — an effect member's own generics accepting a
   linear type argument [linear-generics leftover]. Not a decision; closes
   with whatever `resolve` gains for O-C3's instantiation reasoning.

Each should exist as a test on both backends before the phase closes, the
refusals as reject-cases with diagnostics naming the discharge set.

---

## 7. The recommended bundle, in one place

| item | call | recommendation |
|---|---|---|
| §1-1 obligation syntax split | DECISION | no confident recommendation; decide before 2b's std sweep spreads `Proj` |
| §3-1 discharge sets | DECISION | O-D1: `: Linear<self by stop, join>`, default `by close` |
| §3-2 discharge signature rule | DECISION | exactly one consumed self-typed param, by move; generic discharges allowed |
| §4-1 union arms | DECISION | legal; narrowing to a non-linear arm discharges |
| §4-2 conditional containers | DECISION | declared clause, conditional by default on instantiation |
| §4-3 intrinsic containers | DECISION | defer; user structs + unions first |
| §5-1 D6 `Once` anywhere | DECISION | yes |
| §5-2 D7 use-site linearity | DECISION | no surface; covered by conditional containers; revisit on a customer |
| O-C4 qualifier unification | — | skip this phase (unforced) |
| effect-member generics hole | engineering | close alongside, no decision needed |

Suggested build order once decided, each step leaving the tree green:
**O-C2** (unblocks phase 4, smallest) → **O-D1** (reshapes the group; thread
+ cache tests) → **O-C3** (conditional containers; wrapper-pass test) →
**D6** (position-gate removal) → the smuggling hole → spec sweep
([linear-group], [linear-composite], [linear-generics], [once-fn] all
change; [group-obligation] gains the `by` note) and the documents.
