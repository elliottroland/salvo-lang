# Group borrowing: findings, alternatives, and the committed design

Status: **Parts 1–6 are the research trail (exploration and rejected
alternatives); Part 7 is the committed design** (user decisions, this
session). Parts 1–2 record two verified defects in the current
`canbe`/virtual-place implementation and survey Nick Smith's "group
borrowing" proposal plus incremental fixes. Parts 3–6 develop and critique
a purpose-built group container, including a rejected `Rc`-based storage
variant and an accepted swap-placeholder refinement. **Part 7 states the
final design: `ref(c)` — a provenance qualifier naming the container a
mutable handle borrows from — unifying today's `proj Mut` and the
abandoned `Reg`, with no region scope.** Part 7 also keeps, compactly, the
superseded approaches (region/swap/`Reg`) and why each was dropped — the
reasoning the earlier §§7a–7j worked through across this session. Part 8 is
the implementation plan (v1a → v1b → v2). See `ROADMAP.md` item 15, which
this design **retires** (region is not needed).


## Part 1: two verified gaps in the current model

Both of the following were reproduced by compiling and *running* generated
code on both backends (`rustc`, `kotlinc 2.4.0` + `java`), not just reading
the spec. Repro sources are inline below; they are small enough to retype
into a scratch workspace.

### 1a. A mutable handle cannot chain through a map lookup

`core.map`'s `get` (`std/core/map.sv:71,129`) returns `proj(map) V` — an
ordinary read-only projection — with no `__loc` locator twin the way
`List`/`Deque` indexing has (`[rs-loc]`, `BACKEND_SPEC.rust.md`). The v1 cut
of `[rs-elem-mut]` requires a bound mutable handle to be minted from a
direct `get(place, i)!` over a *pure place*; a map lookup's result is not
such a place, so chaining a further index through it is refused outright:

```
struct Item canbe Mut { tag: Str }

let m: Map<Str, NonEmpty List<Mut Item>> = map_of(("a", list_of(Mut Item { tag: "x" }, Mut Item { tag: "y" })))
let it = get(get(m, "a")!, 1)!   // mutable handle, chained through a map get
it.tag = "z"
```

Compiling this to Rust fails with:

```
main.sv: a mutable lend from something that is not a place [rs-loc]
```

Binding the map lookup to an intermediate `let` first does not help: it only
produces a **read-only** `&Vec<Item>` (confirmed in generated output), with
no re-indexable position to mint a further mutable handle from. So
`map.get("something")[i].field.get("key2")` — the shape the user asked
about — is correctly identified as unreachable today: not a syntax
inconvenience, but a structural gap. Lists and deques get a locator face;
maps do not.

This is **refused, not wrong** — the compiler says no, loudly, exactly as
`[backend-never-wrong]` demands. It is a *coverage* gap (maps are
second-class in the position/locator model), not a soundness bug.

### 1b. Plain `canbe` with two different anchors silently miscompiles — a real backend-parity bug

This one is not a refusal. It compiles, runs, and produces a **wrong
answer** on Rust while Kotlin produces the right one — a confirmed,
reproducible divergence of exactly the kind `[backend-parity]` exists to
prevent, caught here for the first time.

Repro:

```
struct Item canbe Mut { name: Str }

fn f(a: Mut Item, d: Mut Item) -> None => a canbe d, a: Mut, d: Mut {
    a.name = "changed-a"
    d.name = "changed-d"
}

fn main() [use] {
    use StdOutConsole()
    let list1: List<Mut Item> = list_of(Mut Item { name: "p" }, Mut Item { name: "q" })
    let list2: List<Mut Item> = list_of(Mut Item { name: "r" }, Mut Item { name: "s" })

    let a = get(list1, 0)!
    let b = get(list2, 1)!
    f(a, b)
    println("${get(list1, 0)!.name} ${get(list2, 1)!.name}")
}
```

Expected output (and what Kotlin actually prints, verified with `kotlinc
2.4.0` + `java`): `changed-a changed-d`.

Rust's actual output (verified with `rustc`, no error, no warning):
**`changed-a s`** — `list2[1]` is never touched. `d.name = "changed-d"` is
silently redirected onto `list1`.

Root cause, read directly out of `crates/salvo-core/src/mut_lends.rs`'s
`covered_fns`: for the **plain** `canbe` form (`a canbe d`, as opposed to
the **anchored** form, `a canbe in lib.tracks`), the function collects every
covered parameter name into one list and assumes a single shared container
exists for all of them — the generated Rust signature becomes `f(__anchor:
&mut Vec<Item>, __c0: usize, __c1: usize)`. Nothing validates, at the call
site, that the arguments bound to the covered parameters actually came from
the *same* container before collapsing them onto one synthesized `&mut`.
The emitted call just reuses whichever container the first covered
argument's handle-minting call indexed into:

```rust
let __h1: usize = get_platform__loc(&list1, 0).expect(...);
let __h2: usize = get_platform__loc(&list2, 1).expect(...);
{ let __c3 = __h1; let __c4 = __h2; crate::f(&mut list1, __c3, __c4) };   // list2 silently discarded
```

`__h2` was computed as an index into `list2`, then silently reinterpreted
as an index into `list1`. The **anchored** form already has the right
instinct here — its own code comment states "several distinct anchors in
one clause share no storage, so there is no one container to index:
reported at the call site" — but that check only runs for the anchored
form, which has a named path to verify against. The plain form has no
anchor path to check and so has nothing to refuse against.

**This is a defect to fix regardless of any larger redesign** — it is pure
unsoundness in the current implementation, not a known-and-accepted
restriction. The fix within the current architecture is bounded: either (a)
require the plain `canbe` form to prove, per call site, that every covered
argument was minted from the same container before collapsing to one
anchor — refusing the call otherwise, the way the anchored form already
does — or (b) extend the covered-position rendering to carry **one anchor
per distinct container actually observed at the call site**, with a position
pair per anchor pairing (this is closer to correct, but complicates the
generated signature, since the number of anchors a given `canbe`-covered
function needs is then a call-site fact, not a function fact). Either fix
is small and should land independent of anything below.

### Why both gaps point at the same root cause

Both findings trace back to the same property of the current design: a
mutable handle's *validity* is represented as **a position (index/path) plus
an implicit, syntactically-inferred claim about which container it indexes
into** — the container is not carried as part of the handle's own identity
at the type level, it is reconstructed from the call site's syntax (which
`get(...)` call produced it) each time the checker or emitter needs to know.
This works cleanly when every handle's container is still lexically visible
and unambiguous (the overwhelming common case, and the one the worked
`examples/borrowing/` suite exercises). It breaks down exactly where that
reconstruction becomes either impossible (map lookups have no indexable
identity to reconstruct — finding 1a) or ambiguous/unchecked (two
syntactically-independent handles being silently assumed to share a
container with no verification — finding 1b). Both are symptoms of
*container identity being implicit rather than tracked*, which is the
"virtual place" strategy's central economy: it is cheap exactly because it
does not reify a group/container concept as a first-class value the checker
carries around — until it needs one, at which point (1a, 1b) it has nothing
to fall back on.

## Part 2: alternative designs

### Researched: Group Borrowing (Nick Smith, via Evan Ovadia's write-up)

Source: [verdagon.dev/blog/group-borrowing](https://verdagon.dev/blog/group-borrowing)
(Aug 28 2025) — Nick Smith's proposal (Mojo community), explained by Evan
Ovadia (Vale). Summarized here for the parts relevant to Salvo; the full
article has more motivation and caveats than are reproduced below.

**The core mechanism.** Every local variable introduces a **group** — a
compile-time region identifier, not a runtime value. A function signature
names which of its reference parameters share a group:

```
fn attack[mut r: group Entity](ref[r] a: Entity, ref[r] d: Entity):
    ...
```

The call site does **not** have to declare that its two arguments alias —
the compiler infers the group from where the arguments came from:

```
fn main():
    entities = List(Entity(10, 10), Entity(12, 7))
    attack(entities[rand() % len(entities)], entities[rand() % len(entities)])
```

This compiles and is sound with **zero annotation at the call site**,
including the case where both random picks happen to be the same index.
Compare this to Salvo today, where the equivalent call needs the function to
have been written with `canbe` (or the anchored `canbe in` form naming the
specific container) — the *caller* needs the *callee* to have opted in by
name.

**The invalidation rule — the real engine of the system.** A plain,
coarse rule ("mutating through a reference invalidates every other reference
into the same group") is refined by **child groups**: a value only forms a
new child group if it could be independently destroyed — i.e. it sits
behind something ownership-shaped (a `Variant`/union arm, a collection
element, a `Box`). A reference to `entity.hp` (a plain `Int` field) is *not*
in a child group, because nothing can delete an `Int` out from under its
parent struct; a reference to `entity.rings[0]` *is*, because `remove`/`add`
on the list could independently destroy it. So:

- Mutating through one handle into a group **never invalidates another live
  handle into the same group's non-child contents** — e.g. two `Entity`
  references stay valid across a call that mutates one of them, as long as
  neither held a reference into the *other's* collection-shaped fields.
- Mutating only invalidates references into **child groups** — things that
  could have been deleted by the mutation.

This is strictly finer than Salvo's rule for ordinary (non-covered)
handles today, which invalidates "everything derived from the mutated
root" on any mutation of it, with the one escape being the already-built
field-identity narrowing (`e.hp: Mut` vs `e: Mut`, `docs/language/
Mutable-Handles.md` "Which field a call touches"). Salvo's field-level
narrowing is actually a **restricted, named-ahead-of-time instance** of the
same idea child groups generalize automatically: both are answering "can
this specific sub-part have been destroyed," but Salvo's version needs the
author to spell out which field in a deduction clause, where group borrowing
derives it structurally from the field's *type shape* (collection/variant
boundary) with no clause entry required.

**Soundness anchor: mutual isolation.** Every item that could end up in one
group must be *mutually isolated* — no item in the group may contain a
reference to, or transitively own, another item in the same group. This is
what makes "mutating through one handle can't delete another handle's
target" provable at all: an `Entity` cannot hold a reference to another
`Entity`. The article is explicit that this pushes program data shapes
toward trees, "similar to how Rust's borrow checker does," though with much
more relaxed external-access rules than Rust gives a tree-shaped structure.

**Where groups come from, concretely:**
1. Each local variable forms its own group.
2. Multiple groups can be unioned (passing two different locals to one
   function that wants them in the same group).
3. Collections/variants *inside* a group recursively form child groups.

**Costs and open questions the article itself raises** (not resolved,
included here because they matter for any Salvo adoption decision):
- **No unique/exclusive references exist at all** in this model — there is
  no Rust-style `&mut` that *guarantees* no other reference exists. This
  trades away a capability Salvo's `NotEq`-proven and default
  (single-handle) cases currently *do* get for free: real exclusivity,
  usable for `noalias`-style codegen hints. The article proposes three
  unproven mitigations (emit `noalias` only when an argument is the sole
  occupant of its group; a special "guaranteed distinct" group variant;
  detect dynamically-proven distinctness via `if`/`assert` and emit
  `noalias` there) — none implemented, none benchmarked.
- **The author states outright that the proposal is unimplemented and
  unproven** ("this is all theoretical... let me know if you have any
  improvements"). There is no reference implementation, no compiler, and no
  empirical performance data to check Salvo's own backend-parity bar
  against.
- A genuinely open design question in the article itself: whether an
  **immutable parent group with a mutable child group** composes soundly
  (needed for exactly Salvo's `=> from|to canbe in squad.members, squad:
  Mut` anchored shape) is explicitly marked as *not designed for* in Nick's
  original proposal — Evan and Nick "tossed around the idea" and believe it
  works but it is unverified.

**What adopting this would mean for Salvo, concretely.** This is the
most structurally different option on this list, because it is not a
backend-rendering change — it would mean promoting "group" to a first-class
concept the *checker* tracks (today, as Part 1 shows, "which container" is
reconstructed from syntax on demand, never carried as a value). That is a
large checker change, not a backend change, and it would answer finding 1b
by construction (groups are the container-identity record findings 1a/1b
show Salvo currently lacks) and extend naturally to finding 1a (a map's
values could form a child group the same way a list's elements do, once
groups exist as a concept — maps stop being special-cased). It would also
likely let `canbe`/`NotEq`/the anchored form **collapse into one mechanism**
(group inference, with no per-pair annotation needed) rather than Salvo's
current three-tier menu — a genuine simplification in the spirit of
earlier discussion in this session, at the cost of being unproven technology
with no implementation to borrow from and real unresolved tension with
`noalias`/zero-cost codegen, which Salvo's current proven-disjoint
(`NotEq` → `split_at_mut`) path gets for free today.

### Other alternatives, for completeness

**A. Fix-in-place: validate container identity at `canbe` call sites.**
The minimal, scoped fix to 1b described in Part 1 — require every
plain-`canbe` call to prove its covered arguments share a container, refuse
otherwise. Does not touch 1a. Does not change the model's expressiveness —
it only closes the soundness hole. This should happen regardless of what
else is decided; it is not really an "alternative," it is a bug fix.

**B. Give maps a locator face, closing 1a without a model change.**
Add a `__loc`-equivalent to `core.map`'s `get` — something that answers a
stable key-handle (or, since Rust's `HashMap` has no stable "position" the
way a `Vec` index is stable, this likely has to be a different mechanism:
e.g. render a mutable map-value handle as a *re-looked-up-by-key* virtual
place, `map.get_mut(&key).unwrap()`, re-executed at each use the same way
`squad[__hN]` is re-indexed today). This is a direct extension of the
existing virtual-place strategy rather than a replacement for it — same
economy, same backend-only cost, closes exactly gap 1a, leaves 1b to fix (A)
and leaves the "implicit container identity" root cause from Part 1 in
place for any future gap of this shape. Risk: Rust's hash-map API makes
"re-look-up by key" a real runtime cost per use (unlike a `Vec` index,
which is free), so this would be the first case where a virtual place is
not free — worth costing out before committing.

**C. Reify container identity as a lightweight checker-side value (a
narrow version of "groups" without the full group-borrowing apparatus).**
Between (B) and full group borrowing: give the checker's existing `Place`
type (`crates/salvo-core/src/place.rs` — already a root-plus-steps path,
see the nested-path answer earlier in this session) the job of being the
*single source of truth* for "which container does this handle index into,"
and require every handle-consuming operation (including `canbe` coverage)
to go through it rather than re-deriving the answer from call-site syntax.
This would catch 1b directly (two handles into different containers would
have provably different root `Place`s, so the covered-call check in `A`
falls out of this for free) and would make a future "maps form child
places" extension (closing 1a with less special-casing than B) a smaller
step, since the Place abstraction already models containment via `Step`.
This is architecturally the smallest change of any option that actually
fixes the *root cause* rather than patching each symptom — it uses
machinery already in the codebase, rather than importing an unproven
external model. The tradeoff against full group borrowing: it keeps
Salvo's current coarse invalidation rule (mutating a root poisons
*everything* derived from it, modulo the already-existing field-level
carve-out) rather than gaining child-group-style fine invalidation through
collections automatically — so it fixes the two found bugs without buying
the broader ergonomic win (e.g. still no free "two live handles into
different elements of the same list, no proof, no declaration" case
analogous to group borrowing's zero-annotation `attack(entities[i],
entities[j])`).

**D. Do nothing to the model; only patch (A) and leave (1a) as a
documented restriction.** The conservative option. Correct, cheap, ships
fastest, leaves both the map gap and the "implicit container identity"
root cause for the next person to hit a third symptom of. Worth stating
plainly as the baseline every other option should be measured against,
since fixing a confirmed soundness bug (1b) is non-optional regardless of
which larger path, if any, is chosen for the rest.

## Summary table (Part 2 options)

| Option | Fixes 1a (maps) | Fixes 1b (soundness) | New mechanism? | Implementation risk |
|---|---|---|---|---|
| A — validate `canbe` call sites | No | Yes | No | Low, should happen regardless |
| B — map locator face | Yes | No (needs A too) | No (extends virtual places) | Low–medium; map re-lookup may not be free |
| C — reify `Place` as source of truth | Yes (future-proofs it) | Yes | Small (uses existing `Place`) | Medium |
| Group borrowing (full, Part 2) | Yes (by construction) | Yes (by construction) | Yes, large | High; unimplemented upstream, open composability question (mut child of immut parent), no `noalias` story |
| D — do nothing beyond A | No | Yes | No | None |

This table predates Parts 3–6 below, which develop one option — a
purpose-built group container — in enough depth to warrant its own
comparison; see "Overall summary," after Part 6, for how it stacks up
against the options above.

## Part 3: a purpose-built group container, instead of generalizing virtual places

A different cut than Part 2's options: instead of generalizing the existing
index/path rendering to cover more cases (maps, deeper nesting), introduce
**one new runtime data structure that a group's members move into**, modeled
on Rust's `slotmap`/`generational-arena` family. Members get a small `Copy`
`Handle` (an index, or index+generation) instead of a reference; every
access goes through the group's own `get`/`get_mut`, which is a fresh,
momentary borrow per call — the same "re-materialize instead of bind"
discipline Salvo's virtual places already use, except the container is
purpose-built for it rather than a repurposed `Vec`.

```rust
pub struct Group<T> { slots: Vec<Option<T>> /* + generation, for staleness */ }

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Handle { index: usize /* + generation */ }

impl<T> Group<T> {
    pub fn insert(&mut self, value: T) -> Handle { ... }
    pub fn get(&self, h: Handle) -> Option<&T> { ... }
    pub fn get_mut(&mut self, h: Handle) -> Option<&mut T> { ... }
    pub fn remove(&mut self, h: Handle) -> Option<T> { ... }
}
```

This closes finding 1b by construction (two entities moved into one group
share one storage — there is no second anchor to lose track of) and
finding 1a without special-casing maps (a map lookup only has to happen
once, at insertion; every later access is a `Handle` lookup, as cheap and
chainable as list indexing). It also gets a free, well-understood
staleness story: a generational `Handle` captured before a `remove` fails
its `get` cleanly instead of aliasing whatever later value reused the slot
— a runtime backstop under the compile-time checker proof, not a
replacement for it.

Hand-rolled runtime code of this shape already has a direct precedent in
the codebase: the staged, designed-but-unbuilt regions feature's Rust v2
(`[rs-region-arena]`, `ROADMAP.md` item 15) is "a real arena (hand-rolled in
emitted `core/`...), `Reg T` → `&'r T`, one mechanical lifetime per
delimiter" — the same "ship a hand-rolled container, give values a cheap
handle into it" strategy, for a different motivating problem (region
lifetime management rather than aliasing). Kotlin needs none of this —
objects alias natively — so a group container is Rust-only machinery, the
same shape `Reg`/regions already are (erased on one backend, real structure
on the other, [qual-erasure]'s pattern generalized).

### What this costs that today's virtual places don't

1. **An extra layer of indirection and allocation.** `Vec<Option<T>>` (or a
   slot map with a free list) versus a plain `Vec<T>` — one more allocation,
   one more pointer-chase per access, a discriminant tax per slot unless
   packed carefully.
2. **Generational staleness detection, if included, costs a counter per slot
   and a check per access** — cheap, but new runtime behavior with no
   existing Salvo analogue: today a stale handle is a purely compile-time
   checker concern; this adds a runtime backstop, which needs its own
   panic/`Option` convention consistent with `!`'s existing behavior.
3. **Items that join a group leave their original container** — this is
   the real design question, below.

## Part 4: automatic vs. explicit group membership

The costs above (indirection, allocation, generational checks) are accepted
as reasonable. The open question is narrower and sharper: **should entering
and leaving a group happen automatically, inferred by the compiler at call
sites that need it, or should it be an explicit, syntax-visible operation
the program writes?**

### Worked example

```
struct Entity canbe Mut {
    hp: Int,
    rings: Mut List<Ring>
}

fn attack(a: Mut Entity, d: Mut Entity) -> None => a canbe d, a: Mut, d: Mut {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
}
```

Two call sites in the same program:

```
// Call site 1: same container. Already works correctly today, zero-cost.
let squad: Mut List<Mut Entity> = mut_list_of(...)
attack(get(squad, i)!, get(squad, j)!)

// Call site 2: different containers. Today: finding 1b — silently wrong.
let squad_a: Mut List<Mut Entity> = mut_list_of(...)
let squad_b: Mut List<Mut Entity> = mut_list_of(...)
attack(get(squad_a, i)!, get(squad_b, j)!)
```

### Option (a): automatic, inferred group formation

The checker decides on its own, per call site, whether `attack`'s `canbe`
pair needs a group — and if the two arguments don't share a root container,
it synthesizes one invisibly: move both entities into ad hoc shared storage,
call `attack` through `Handle`s, move the (possibly mutated) results back
into their original slots afterward. Source text at both call sites above
stays identical to today; nothing in `attack`'s declaration changes.

**Why this costs more than it looks like.** The detection step — "do these
two arguments provably share a container" — is exactly the aliasing-
provenance analysis finding 1b showed was missing, so this does not avoid
building that analysis; it requires it, plus new move-synthesis machinery
on top. And the move-in/move-out pair (two moves each way) is paid
*silently*: call site 1 above stays free, call site 2 pays four hidden
moves, and nothing in `attack`'s own signature — the one place Salvo
normally makes a function's cost legible — can show which case a given
call is in, because the fact that decides it (do the arguments share a
container) lives entirely on the caller's side. This is a sharper version
of the already-known "signature-change-free contract drift" problem
([deduce-infer]'s revisit note, `LANGUAGE_SPEC.md` ~6633): there, at least
the callee's own body determines the inferred fact, so hovering the
function shows it. Here the decisive fact is call-site-local, so there is
no signature anywhere that would ever show it.

### Option (b): explicit, syntax-visible opt-in

Entering and leaving a group are themselves ordinary function calls, in the
same register as the already-designed-but-unbuilt `reg`/`unreg` (regions,
`ROADMAP.md` item 15) — move a value in, get a `Handle` back; hand a
`Handle` back, get the value out. Spelling is provisional; the shape is the
point:

```
group entities: Group<Entity> = Group<Entity> {}

let ha = enter(entities, remove_at(squad_a, i)!)
let hb = enter(entities, remove_at(squad_b, j)!)

attack(entities, ha, hb)        // attack's signature now names the group

leave(entities, ha, squad_a, i)  // move the (possibly mutated) entity back
leave(entities, hb, squad_b, j)
```

`enter`/`leave` get ordinary deduction clauses — `enter` consumes its value
argument and produces a `Handle`; `leave` consumes a `Handle` and the
group, and (depending on which variant of Part 5 is chosen) either hands
the value back owned or writes it into a caller-supplied destination. No
new checker machinery is needed beyond what already tracks every other
move in the language; the fixpoint, shared-fate analysis and move-tracking
just see two more ordinary move events.

### Why (b) is the stronger choice

1. **Cost is visible exactly where Salvo already makes cost visible**: the
   signature and the call site. `attack(entities, ha, hb)` taking a group
   plus two handles tells a reader this call needs pooled storage — no
   call-site-dependent branching in what the generated code does, and
   nothing invisible to find later.
2. **It composes with the existing deduction vocabulary** rather than
   requiring a new inference pass whose only job is deciding *whether* to
   run a transformation. `enter`/`leave` are two more moves to an analysis
   that already understands moves.
3. **It gives the author a real choice automatic synthesis cannot**: group
   membership can be short-lived (one call) or long-lived (construct a
   group once, run many `canbe`-covered calls against the pooled set,
   leave at the end) — a persistent-pooling strategy option (a)'s per-call
   synthesis model has no way to express, since it only ever reasons about
   one call at a time.
4. **The real cost is paid exactly where the programmer decided it was
   worth paying, and nowhere else.** A same-container `canbe` call (call
   site 1) is untouched — no group, no handles, today's zero-cost rendering.
   The different-container case (call site 2), currently silently *wrong*,
   becomes correct and requires writing `enter`/`leave` explicitly — a
   smaller, more honest ask than it sounds, and the same trade the language
   already makes for `copy(...)`: "a copy never happens without the program
   opting in" (`docs/language/Deductions-and-Ownership.md`, Projections),
   applied here to group membership instead of to copying.

The cost of (b), stated plainly: the different-container case requires
writing something, where (a) would require writing nothing. Given that the
different-container case is also the rare one — and the one currently
broken — this reads as the right place to ask for an explicit opt-in.

**Recommendation for discussion:** pursue (b). Record (a) as the rejected
alternative, kept here for the reasoning rather than discarded outright,
since a future, better-scoped version of automatic detection (e.g. limited
to a lint that *suggests* wrapping in a group rather than silently doing
it) might still be worth revisiting once (b)'s explicit form exists and has
real usage to learn from.

## Part 5: can `Rc` manage the group without moving values?

Explored as an alternative to Part 3's slot map, specifically to avoid
Part 4's cost that items must physically leave their source container.
The idea: instead of a `Vec<Option<T>>` a group owns outright, give each
member an `Rc<RefCell<T>>` (or `Rc<Cell<T>>` for Copy-ish payloads), and let
"entering a group" mean cloning an `Rc` handle rather than moving the
value — the original container could, in principle, keep its own `Rc`
pointing at the same heap allocation.

**This fails before the panic-risk question even arises: a plain
container's elements aren't behind a pointer to begin with, so there is
nothing to wrap in place.** Verified directly with `rustc`:

```rust
let mut squad: Vec<Entity> = vec![Entity { hp: 10 }, Entity { hp: 20 }];
let handle: Rc<RefCell<Entity>> = Rc::new(RefCell::new(squad[0]));
```

```
error[E0507]: cannot move out of index of `Vec<Entity>`
```

A `Vec<T>` stores its elements **inline, by value** — `squad[0]` is not a
pointer to an `Entity`, it *is* the `Entity`'s bytes, embedded in the
vector's own backing buffer. `Rc::new(RefCell::new(...))` needs to *move* a
`T` into a fresh, independent heap allocation; there is no way to move
`squad[0]` into it without also emptying `squad[0]`, which is exactly the
"source container loses the element" cost Part 5 was proposed to avoid in
the first place. The compiler refuses even the attempt.

The only way to make this compile is to **clone** the element in, which
was also verified directly: cloning `squad[0]` into the `Rc<RefCell<>>` and
mutating through the handle (`hp: 10 → 15`) leaves `squad[0]` completely
unaffected (`hp` stays `10`). That is not aliasing — it is an ordinary
copy wearing an `Rc<RefCell<>>` costume. Getting *real* aliasing this way
would require the container to store `Rc<RefCell<Entity>>` as its element
type from the moment each `Entity` is constructed — i.e. `List<Mut
Entity>` would have to become `List<Rc<RefCell<Entity>>>` (or every
`Entity` individually heap-allocated and reference-counted from birth) —
not something `enter` could arrange opportunistically at the point a value
joins a group. That is a far larger, more invasive change to how Salvo's
mutable containers are represented on the Rust backend than anything else
in this document proposes, and it would apply to every value that might
*ever* need to join a group, not just the ones that actually do.

**Even setting the above aside, the scheme also carries the panic risk
already found and rejected once for a different feature.** `COMPLETED.md`'s
"E1a — Ownership strategy" section rejected `Rc<RefCell<...>>` for handler
dependencies (its own option B2) with a finding that transfers here
unchanged: wrapping shared mutable state in `RefCell` **turns a
compile-time question into a runtime panic**, breaking backend parity by
construction, because *Rust temporaries live to the end of the statement*
— the concrete counterexample recorded there, `println("${bump()}
${bump()}")`, panics with "already borrowed" with no cycle anywhere in the
program. The same failure mode applies directly to a group built on
`Rc<RefCell<Entity>>`: `attack(get_handle(group, ha), get_handle(group,
hb))` — two `.borrow_mut()` calls live for the duration of one statement —
would panic at runtime with "already borrowed" if `ha == hb` (the exact
self-aliasing case `canbe` exists to support, per its own doc: "including
the self-strike case"), while Kotlin's native aliasing runs the same
program fine. That is a backend-parity break by the project's own existing
definition — exactly the class of bug Part 1 of this document is about,
reintroduced by the "fix."

Two consequences follow directly, mirroring the rejected handler option:

- **Cycle/overlap detection is not sufficient to save this**, for the same
  reason recorded against B2: the overlap is created *per call*, by two
  live `.borrow_mut()` guards coexisting for one statement's duration, not
  by any structural cycle the checker could statically rule out ahead of
  time the way it rules out recursive ownership.
- **Every access through the group becomes a *runtime* question** (did
  this `.borrow()`/`.borrow_mut()` panic) **where today it is a
  *compile-time* one** (did the checker's own proof/declaration permit
  this aliasing) — a strictly worse trust model than either the slot-map
  option or today's `canbe` rendering, both of which keep the question
  entirely at compile time.

There's a second, independent problem beyond the panic risk: **`Rc` is not
`Send`**, flagged as a phase-5 blocker in three unrelated places in this
codebase already (`[rs-fn-field]`'s fn-typed struct fields, the actor
survey's `SalvoProcess: Send` requirement, and the regions design's own v1
staging, `[rs-region-rc]`, which already carries this exact flag). A group
built on `Rc` would inherit the same restriction — it could never cross an
actor/thread boundary, which is a real limitation for a general-purpose
aliasing mechanism to carry silently.

`Rc` alone (no `RefCell`, immutable payload) sidesteps the panic risk but
then gives up mutation entirely, which is the one thing a group exists to
support — not a usable middle ground for this problem.

**Conclusion: rejected, on three independent grounds**: it cannot even be
constructed in place for a plain container's elements (verified above —
this is the most basic objection, and would hold even if the other two
did not); it reintroduces the structurally identical panic-risk failure the
project already rejected once for a different feature; and it would carry
`Rc`'s existing `!Send` restriction into a general-purpose aliasing
mechanism. The slot map from Part 3 is the right tool for "cheap handle,
no reference," specifically because a `Handle` is a plain `Copy` value with
*no* runtime borrow-tracking to panic and no pointer-identity requirement
on the source container — the checker's compile-time proof is the only
thing deciding validity, same as today.

## Part 6: swap-with-placeholder instead of true removal

A different, more promising angle on the same cost Part 4 flags (items
leaving their source container): instead of `remove_at` — which shrinks
the list, shifts every later element down one slot, and reallocates on
growth — have `enter` swap the moved value out for a cheap placeholder
left in its original slot, and have `leave` swap the real value back in.
Rust already has the exact primitive for this: `std::mem::replace(&mut
list[i], placeholder)`, which trades a value for another value of the same
type with no shift, no reallocation, and no shrinking — an O(1) in-place
exchange.

This is not a new idiom for the codebase to adopt — it is already the
documented behavior of one of std's own list functions. `core/list.sv`'s
`[col-replace]` rule is, in its own words, "the **total positional write**:
puts `value` at `index` and answers the element it displaced" — precisely
`mem::replace`'s contract, just not yet named that way outside the doc
comment. `enter`/`leave` under this scheme would compile to calls into
that same primitive rather than to `remove_at`/`insert_at`'s shift-the-rest
behavior:

```
let placeholder = Entity { hp: 0, energy: 0, rings: list_of() }  // or a type-level default
let (ha, displaced) = enter(group, squad_a, i, placeholder)      // mem::replace under the hood
...
leave(group, ha, squad_a, i)                                      // mem::replace back
```

### What this buys

1. **No reallocation, no shift, no shrink** on either `enter` or `leave` —
   strictly cheaper than `remove_at`'s current behavior for this specific
   use, and it removes the asymmetry Part 4 flagged (the source container
   staying "logically the same size" the whole time a member is on loan,
   rather than momentarily shrinking and later growing back).
2. **It composes cleanly with the slot map from Part 3**: the group's own
   storage is unaffected by this choice — this is purely about what
   `enter`/`leave` do to the *source* container, independent of what kind
   of container the group itself is.
3. **It is a smaller ask of the backend than it looks**: `mem::replace` is
   already how `[col-replace]`'s total positional write is implemented
   (confirmed in `std/core/list.sv`), so `enter`/`leave` would be a new
   *source-level* pair of functions calling an *already-shipping* backend
   primitive, not new backend machinery.

### What it costs, and the one hard question it raises

- **A placeholder value has to exist for every type that can enter a
  group.** For a struct with no sensible "empty" state (an `Entity` has no
  natural placeholder `hp`/`rings` — `0`/`list_of()` is a choice, not a
  fact about the domain), this needs either (a) the caller to supply one
  explicitly at every `enter` call (shown above — correct, but asks the
  author for a value that is never actually used, which is friction), or
  (b) wrapping the slot in `Option<T>` so the placeholder is `None` rather
  than a fabricated `T` (no fabricated-value problem, but now every access
  to the source container's slot — including ones that never touch the
  loaned-out member — pays an `Option` unwrap it didn't pay before,
  spreading a cost outward from the one slot that's actually on loan to
  every read of that collection type).
- **The source container's element type must change to `Option<T>` (or
  must already be one) for option (b) to work at all** — this is not free
  on the type-system side either: `List<Entity>` and `List<Entity>` with
  "some slots might be mid-loan" are different contracts, and today's
  `List<T>` has no notion of a slot that's temporarily absent. Making this
  work without changing the container's declared element type pushes
  toward option (a) (caller-supplied placeholder), which is simpler to
  reason about but asks something of every `enter` call.
- **`canbe linear` types cannot have a placeholder constructed for them at
  all**, by the language's own existing rule: a `linear` type's whole point
  is that no value of it may be conjured except through its own
  constructors with a tracked must-use obligation (`docs/language/
  Linear-Types.md`) — there is no "cheap default" a linear type could ever
  offer, by design. So this scheme would need to refuse `enter` on a
  `canbe linear`-opted container element outright, which `core.list`'s own
  precedent already anticipates (`remove_first`/`remove_at` are typed
  `<T canbe linear>` specifically *because* removing a linear element is a
  real, trackable move, not a placeholder swap — the existing functions
  already distinguish "can be removed because it's linear and tracked" from
  what a placeholder-swap would need, which is "can be removed because
  something harmless can stand in its place," and those are different
  properties of a type). This is a real, fixed-in-advance restriction to
  record, not a bug to fix later: a group would need its own `canbe`-style
  opt-in deciding whether a type supports the placeholder scheme, separate
  from whether it supports being removed at all.
- **The window between `enter` and `leave` is observable from the source
  container's side**, which is new: anyone still holding a plain
  (non-handle) reference into `squad_a[i]` during the loan sees the
  placeholder, not the real entity — correctly reflecting that the real
  value is elsewhere, but a new kind of "stale-looking but not actually
  invalidated" state the checker would need to reason about (today, a
  handle into a mutated root is simply poisoned; here, a *read* of the
  slot during a loan is not poisoned, it legitimately returns a different,
  valid value — the placeholder — which is a new flow fact, not a
  violation).

**Overall assessment:** this is a real improvement over plain move-out
(Part 4's unqualified version) specifically for non-linear types with a
natural or acceptable placeholder, and it reuses a primitive the backend
already ships (`[col-replace]`/`mem::replace`) rather than inventing one.
It is not a free win: it needs either a caller-supplied placeholder (asks
something of every `enter` call) or an `Option<T>`-shaped container (asks
something of the collection's own declared type), and it needs an explicit
carve-out for `canbe linear` element types, which cannot participate at
all. Worth pursuing as a refinement of (b) from Part 4 — not as a
replacement for the plain-move version, but as a cheaper default for the
common case where a placeholder is acceptable, with plain move-out (or an
outright `enter` refusal) remaining the answer for `canbe linear` payloads.

## Overall summary (updated after Parts 3–6)

| Option | Fixes 1a / 1b | New mechanism? | Values leave source container? | Risk / cost |
|---|---|---|---|---|
| A — validate `canbe` call sites | 1b only | No | N/A | Low; ship regardless |
| B — map locator face | 1a only (needs A for 1b) | No | No | Low–medium; map re-lookup may not be free |
| C — reify `Place` as source of truth | Both | Small | No | Medium |
| Full group borrowing (Part 2) | Both, by construction | Large | No (groups are implicit, from locals) | High; unimplemented upstream, no `noalias` story |
| Group container, automatic (Part 4a) | Both | Medium–large | Yes, invisibly | High; cost invisible at every signature, requires the same aliasing-provenance analysis as C plus move-synthesis on top |
| Group container, explicit (Part 4b) | Both | Medium | Yes, visibly (`enter`/`leave`) | Medium; composes with existing deduction machinery, cost visible in signatures |
| ↳ with `Rc`/`RefCell` storage (Part 5) | — | — | — | **Rejected**: can't even alias a plain container's inline elements in place (`E0507`, verified), and even if storage were restructured to allow it, reintroduces the runtime-panic class of backend-parity bug the project already rejected once (E1a/B2), plus `Rc: !Send` |
| ↳ with swap-placeholder `enter`/`leave` (Part 6) | — | — | No (placeholder stands in) | Medium; reuses `[col-replace]`'s existing `mem::replace` primitive; needs a placeholder story per type and refuses `canbe linear` elements |

Where this leaves the decision: (A) is unconditional — it is a correctness
fix, not a design choice. Beyond that, the real fork is between **(C)**,
which fixes both findings with the smallest change by generalizing
machinery the checker already has, and **the explicit group container
(Part 4b, refined by Part 6's placeholder scheme)**, which costs more to
build but is the only option on this list that also gives the *language* a
new, visible capability — pooling arbitrary values from different
containers for aliasing, on purpose, as a named operation — rather than
only closing the two bugs this document started from. Full, unmodified
group borrowing (Part 2) and the automatic-synthesis variant (Part 4a) are
both set aside: the former for being unproven upstream with no `noalias`
answer, the latter for making cost invisible at exactly the place Salvo's
design has consistently chosen to make cost visible.

## Part 7: the committed design — `ref(c)`, a container-named mutable handle

**Status: committed (user decisions, this session).** This Part is the
single, forward statement of the design the session converged on. The
dead-ends it replaced — a `region { }` scope, a `Reg` qualifier, element
swap with `Placeholder<T>` — are kept compactly at the end ("Superseded
approaches, and why") because the reasoning is worth not re-deriving; they
are not part of the design. `ROADMAP.md` item 15 ("Regions") is **retired**,
not built: the one job a region scope was load-bearing for (reconciling
checked-out elements) does not exist once nothing is moved out of a
container.

### The one idea

A mutable element handle — the thing `get(list, i)!` produces, that
`canbe`/`NotEq`/group-borrowing v1 are about — is today spelled `proj Mut`,
which conflates two orthogonal facts: *provenance* ("a borrow of a known
source, re-materialized not owned, shares the source's fate") and
*permission* ("read-only, unless `Mut`"). The implementation already keeps
them apart — `FateLink` carries the source (`root_id` + `path`), and the
`Mut`-ness lives on the type, not the link — so the conflation is only at
the surface.

**`ref(c)` names the provenance directly: a handle into container `c`.** Its
permission is a separate qualifier on top: `ref(c) Mut T` is a mutable
handle into `c`, `ref(c) T` a read-only one. The container `c` is a
dependent slot (a place), the `Idx(c)`/`KeyOf(c)` shape. This unifies
`proj Mut` and the abandoned `Reg` into one concept — there is no second
handle kind — and it reifies the "which container" fact that `proj Mut` left
implicit (reconstructed from call-site syntax, the root cause of findings
1a/1b).

### What it buys

- **Self-aliasing is free and automatic.** Two `ref(c) Mut` handles with the
  *same* `c` may alias — self-strike (`i == j` at runtime) included — because
  they are two positions into one container, rendered as the shared-anchor
  path the Rust backend already emits for anchored `canbe`
  (container once + `usize` positions; `salvo_pair_mut` for a proven-distinct
  pair). Two `ref` handles with *different* `c` are refused: different
  containers cannot alias (which is correct, and more honest than the
  abandoned region design, which would have co-located them and called that
  aliasing — a relation Kotlin's two objects cannot share). The aliasing
  decision is `root_id` equality, a checker fact, independent of the
  backend.
- **No `canbe` clause needed.** `fn attack(a: ref(c) Mut Entity, b: ref(c)
  Mut Entity)` says the two parameters share container `c` and so may alias,
  in the signature, uniformly with how `(list)` and `?cmp` already appear —
  replacing the pairwise `canbe` relation with one that *names the shared
  container*.
- **Generic and custom containers, by construction.** A container is
  handle-able exactly when it offers an accessor `at(c, l) -> ref(c) Mut T?`
  (the `Locate` bundle, renamed `Ref` under this vocabulary). `List` ships
  `at` = indexing; a map ships `at` = by-key lookup (**closing finding 1a**,
  no map special case); a struct ships `at` = field selection; a custom
  container ships its own. One ordinary function per container, nothing
  region-aware.

### How a handle is written, lowered, and spelled

- **Written (the mint):** an ordinary function returning a handle derived
  from the container — what `core.list` already ships (`[col-locate]`),
  re-spelled with `ref`:
  ```
  fn at<T>(list: List<Mut T>, index: Int) -> ref(list) Mut T? => list, index {
      return get(list, index)
  }
  ```
  The only rules are `proj`/`ref`'s existing ones: the result must derive
  from the container parameter ([proj-infer]'s "must derive from a named
  source"), and `L` is whatever the container indexes by. No region, no
  handle type to construct, no lifetime to name.
- **Lowered (Rust):** a *bound* handle is a plain position (`usize` for a
  list) — `let __h2: usize = …; squad[__h2].hp = …` — re-materialized by
  indexing the live container at each use ([rs-loc]/[rs-elem-mut]). **No
  closure is stored in the handle.** The accessor `at` appears only
  transiently at the mint (to turn a locator into the position) and, for a
  generic container, as a *statically-resolved* `at` call per use (like
  `cmp`/`eq` for a generic — "nothing materialized", [implicit-resolve]). A
  `&mut dyn FnMut` locator closure appears **only** at a type-erased `?at`
  consumer boundary, as a parameter, never as a stored field — the existing
  `?Locate` rendering. The common case (concrete container, bound handle) is
  a bare `usize`, zero overhead, identical to today. Kotlin erases all of
  it — the handle is the element reference.
- **Spelled (`?at`):** `at` is a *nameable* capability, exactly like `?cmp`
  (confirmed against `Sorted<T>(?cmp)`/`add_sorted`: a capability is in scope
  only when carried by a qualifier on a parameter or declared as a standalone
  implicit — never free from `T`). The `Ref` group is the `params` bundle
  around it. But in the common case it is **not written** — resolved by name
  like `?cmp` ([cmp-carry]).

### `ref(c)` relates to today's handles by subtyping — it does not fracture the model

`ref(c) Mut T <: Mut T <: T` by [qual-erasure]: dropping `ref` forgets *which*
container, which only ever loses aliasing permission (sound). So a function
written against plain `Mut`/`proj Mut` parameters accepts `ref(c)` arguments
untouched, and `attack`'s two aliasing handles need the *caller* in the
know, not the callee — the permission is established at the call site from
the shared `ref(c)`, mirroring how `canbe` is caller-visible API today. The
old three-rung ladder (one-at-a-time / `NotEq` / `canbe`) is unchanged for
non-`ref` code; `ref(c)` is a fourth route to the same permission, additive.

### The two defaulting rules (settled, this session)

Both are `[cmp-carry]`/[implicit-resolve] — "an unwritten slot is resolved
by its name" — applied to `?at`.

- **Rule 1 (mint):** `-> ref(c)` records *this function* as the handle's
  accessor (inferred self; the common case, nothing written). `-> ref(c,
  ?at)` is permitted and then names a *different* accessor that must be **in
  scope** — so a mint that **delegates** (returns a handle another accessor
  produced) must take that accessor in, exactly as `sort -> +Sorted<T>(?cmp)`
  is supplied by its `?Ordered<T>` parameter: `fn borrow_via<C, L, T>(c: C,
  l: L, ?Ref<C, L, T>) -> ref(c, ?at) Mut T? => c, l { return at(c, l) }`.
  You cannot publish `?at` from nothing. (In/out split: `ref(c, ?at)` in a
  *return* publishes a capability and must have it in scope; `ref(c) Mut` on
  a *parameter* receives it from the caller's resolution — the `+Sorted(?cmp)`
  vs `Sorted(?cmp)` split, exactly.)
- **Rule 2 (consumer):** two bare `ref(c)` parameters default to **one shared
  accessor** (and one container), so they may alias — the 99% case, nobody
  passing two different locators into one collection. The rare genuine case
  is named: `a: ref(c, ?at) Mut Entity, b: ref(c, ?at: at2) Mut Entity`;
  mismatched accessors that aren't distinguished are an ordinary type error
  (the accessors don't unify, as `cmp@Person` ≠ `by_age` makes two
  `SortedSet` types distinct), fixed the usual way (alias one, or name the
  second). **Sound because the default errs toward *may-alias*** — the
  conservative direction, routed through the existing one-at-a-time/split
  machinery; the *unsafe* "cannot alias" conclusion is only ever reached by
  an explicit `?at: at2` claim, never the default.

### Net surface

```
// mint: self, nothing written
fn at<T>(list: List<Mut T>, index: Int) -> ref(list) Mut T? => list, index { … }

// consumer, common case: two handles, one container, inferred shared at
fn attack(a: ref(c) Mut Entity, b: ref(c) Mut Entity) { a.hp -= 1; b.hp -= 2 }

// consumer, rare case: two different accessors into one container, named
fn f(a: ref(c, ?at) Mut Entity, b: ref(c, ?at: at2) Mut Entity) { … }

// mint delegating: the published ?at must be taken in (?Ref bundle)
fn borrow_via<C, L, T>(c: C, l: L, ?Ref<C, L, T>) -> ref(c, ?at) Mut T?
=> c, l { return at(c, l) }

// a map: at = by-key lookup, which is what brings maps into the model (1a)
fn at<K, V>(m: Map<K, Mut V>, key: K) -> ref(m) Mut V? => m, key { return get(m, key) }
```

### Why no region, checked against the hard (generic) case

Three jobs a scope might be thought to own, each already covered:

1. **Re-materialize a generic handle** — the accessor is a function-identity
   the checker threads (resolved statically at a known-type use; the existing
   `?at` closure only at an erased boundary). A backend concern, no scope.
2. **Escape / lifetime safety** — the fate link: `ref(c)` carries `root_id`
   → `c`, so any structural mutation or move of `c` poisons every handle into
   it ([fate-poison]/[proj-mut]: "destruction under a live handle needs no new
   rule"); a handle over a temporary is already refused ([proj-anywhere]).
   No scope.
3. **Aliasing permission across a generic boundary** — `root_id` equality; a
   generic container parameter is one `var` with one stable id, so two
   handles off the same `c` share it whether `c` is concrete or generic. A
   checker fact. No scope.

Region was load-bearing only for swap reconciliation, which this design does
not do. So the whole `region`/`Reg`/effect apparatus is unnecessary.

### Settled decisions (user, 2026-10-08)

- **Terminology:** `ref(c)` replaces `proj Mut` for a mutable element
  handle. **Plain read-only `proj` stays** exactly as it is (an owned
  view's borrow, `[proj-readonly]`) — `ref(c)` is only the
  element-handle-into-a-named-container case. So the split is `proj`
  (read-only borrow of a view) vs. `ref(c)` (handle into container `c`,
  mutable when it carries `Mut`).
- **`get` reads, `at` handles (user, 2026-10-08).** `get(list, i)` always
  returns `proj(list) T` — a *read-only* projection — **including when `T`
  is `Mut X`**: `get` on a `List<Mut X>` is `proj(list) Mut X`, which is
  immutable *from `get`'s perspective* (a plain `proj` carrying `Mut` is no
  longer a handle and does not satisfy a `Mut` position; the `Mut` just
  rides along as part of the element type). `at(list, i)` always returns
  `ref(list) T` — the container-named handle — so `at` on a `List<Mut T>`
  is `ref(list) Mut T`, the mutable handle. There is **no mutable `get`
  overload** and no per-container overload explosion: `get` keeps its one
  read-only signature, `at` (the `Locate`/`Ref` member) is the sole mint
  for handles, and the user picks `get` for a reading or `at` to mutate.
  Consequence: every mutating mint in examples/docs moves from
  `get(squad, i)!` to `at(squad, i)!` — making "I am taking a mutable
  handle" visible at the call site, which is the point.
- **`canbe` is retired**, replaced by `ref(c)` signatures. The anchored
  `canbe in squad.members` form is subsumed — and is in fact the *natural*
  case: the container is a parameter the callee holds, so it is named
  directly, `fn something<T>(c: List<Mut T>, a: ref(c) Mut T)`, no more
  exotic than writing `let a = c.at(20)` in that scope. So retiring drops
  no capability; the sweep rewrites `examples/borrowing/`, the `canbe`
  codegen tests, and the `Mutable-Handles.md`/`LANGUAGE_SPEC.md` prose.
- **`Locate` → `Ref` group rename:** folded into v1a (pure vocabulary).

### Settled decisions (user, 2026-10-10): storage paths, `NotSame`, no `?at`

- **A handle is a storage path.** On a backend with ownership a `ref(c)`
  re-materializes by walking a **path** from `c` — field steps, union
  arms, list/`Deque` indices, map *slots* — computed once, when it is
  minted, by running `at`'s body. Never by replaying `at`. A path exists for
  every `at`, because a `ref(c)` result must derive from `c` and mutable
  storage is built only from those steps. So what a handle can do never
  depends on how `at` is written. The path type belongs to the **(container
  type, element type)** pair, derived from the type definitions, not to an
  accessor, so handles from different accessors of one container compare
  and split. The worked example is Part 9.
- **`?at` is removed from `ref(c)`.** With paths typed by the container,
  the accessor identity carries nothing a backend needs. Rules 1–2 above
  (`ref(c, ?at)`, `?at: at2`, `borrow_via`) are **superseded**: `ref(c)` is
  the whole spelling.
- **`NotSame` is the proof that two handles differ** (`b is NotSame(a)`): a
  claim about *handles*, so it is independent of the accessor. Rust
  compares paths and Kotlin compares references (`!==`); two `NotSame`
  handles may be live at once (split where the paths diverge). Whether
  `NotEq` on indices survives beside it is open (Part 9).
- **Naming.** A `ref`-returning fn renders on each backend as *its* `ref`,
  under its own name: Rust's `at` returns the path, Kotlin's the
  element. There is no separate natural face beside it.

### Superseded approaches, and why (kept for the reasoning, not the design)

The session reached `ref(c)` by eliminating each of these; recorded so they
are not re-proposed. (The full exploration was §§7a–7j and Parts 3–6.)

- **Element swap into a region (`reg_from`/`unreg_into`/`Placeholder<T>`,
  the first committed design).** `reg_from` moved an element out of its slot,
  leaving a placeholder. **Fatal, user-found:** two checkouts of the *same*
  slot via runtime indices that coincide give the second caller the
  placeholder, not an alias — silent corruption, exactly the self-aliasing
  case the feature exists for. Abandoned; its scope existed only to reconcile
  these checkouts, so the scope went with it.
- **Reg the container, keep a separate `Reg` qualifier + `region { }` scope.**
  Fixed the swap gap (nothing moved out) but left two overlapping handle
  kinds (`Reg` and `proj Mut`) and a scope whose only remaining jobs were
  already done by `proj Mut`'s fate links (escape) or non-existent (swap).
  Collapsed into `ref(c)` — one concept, no scope.
- **Forwarding placeholder (a slot that points to the region entry).**
  Rejected on the same ground as Part 5's `Rc`: `List<T>` is `Vec<T>`,
  elements inline by value, so a forwarding marker can't be a `T` without
  changing `Vec<T>` → `Vec<T | Forwarded>` and taxing every read of every
  container. Not viable without redefining what a list is.
- **Full group borrowing / automatic call-site synthesis (Part 2 / §7 Part
  4a).** Unproven upstream, no `noalias` story; the auto-synthesis variant
  made cost invisible exactly where Salvo makes it visible. Set aside.
- **`region`/`Reg`/`Placeholder` generally.** None survives: no scope, no new
  qualifier, no swap, no placeholder, no `Rc`. The capability the whole
  exploration was reaching for — pooled mutable aliasing of handles from a
  container, on purpose — is expressed by `ref(c)` directly, because the
  handle model already had the pieces (`proj Mut` + fate-link container
  identity + the anchored-`canbe` shared-anchor lowering) once the swap idea
  and its gap were abandoned.

## Part 8: implementation plan

> **Superseded in part (2026-10-10):** v2's "re-materialize through the
> resolved `at`" and every `?at` form below are replaced by storage paths
> (Part 7's 2026-10-10 decisions; Part 9).

**Status (2026-10-10): storage paths (Part 9) and `NotSame` are built** —
generic containers and effect members' single face remain (ROADMAP 15).
Earlier the same day: **v1a, v1b and v2 for `List` and `Map` were built**
(COMPLETED.md, "The `ref(c)` rework: done"). What remains of v2 — generic
and custom containers, the named `?at` forms — is ROADMAP item 15.

Re-planned for the `ref(c)` design (this session), superseding the earlier
`region`/swap plan. **Steps 0, 1, 2 already landed** (committed): Step 1 the
plain-`canbe` two-container soundness fix (`[GB-fix-1b]`), Step 2 the
map-chain confirmed-refusal guard, Step 0 `: Params<self>` on `intrinsic
type` (useful on its own; its original `Placeholder` motivation is dropped).
The `Placeholder`/`reg_from`/`Region`/`region { }` steps of the old plan are
**deleted**. What remains builds `ref(c)` in three additive stages, each
`cargo build`-clean, `cargo test`-green, kotlinc/rustc e2e where applicable.

### v1a — the rename+split: `proj Mut` → `ref(c)`, and `Locate` → `Ref`

Pure surface/vocabulary; **no backend change, no new capability.** It only
renames what the language already produces and clears the `proj Mut`
category error so later stages have correct vocabulary.

- Introduce `ref(c)` as the provenance spelling for the element-handle case:
  the result of `get` on a `List<Mut T>` is `ref(list) Mut T` where it was
  `proj(list) Mut T`. `proj` without a source stays the owned-view borrow it
  is today.
- Rename the `Locate` group to `Ref` (member stays `at`), updating `std`,
  examples, corpus, and the `[col-locate]`/`[rs-loc]` spec rules.
- Rule 1's mint-self inference (`-> ref(c)` self-binds the accessor) and the
  in/out publishing split.
- Verification: behavior-neutral — every program compiles identically; the
  only diffs are spellings in diagnostics/hover and the renamed group. Sweep
  the examples and snapshots deliberately (expect `proj Mut` → `ref` text
  diffs in goldens).

### v1b — `ref(c)` as the aliasing signature (concrete containers)

The aliasing win for `List`/arrays/`Deque`, reusing the anchored-`canbe`
shared-anchor lowering that already exists.

- Accept `fn f(a: ref(c) Mut T, b: ref(c) Mut T)`: two parameters sharing
  `c` may alias; different `c` refused. Decide by `root_id` equality at the
  call site (the `[GB-fix-1b]` check generalized from "same root" to "the
  `ref(c)` names it").
- Rule 2's consumer defaulting (bare `ref(c)` → shared `?at`; explicit
  `?at`/`?at: at2` for the rare split).
- Lower to the existing container-once-plus-`usize`-positions anchor path
  (`salvo_pair_mut` for proven-distinct, one `&mut` + two indices for the
  possibly-coinciding case) — no new backend machinery.
- **`canbe` decision applies here:** if retired, rewrite `examples/borrowing/`
  and the `canbe` codegen tests to `ref(c)` signatures in this step
  (verifying `ref(c)` subsumes the anchored `canbe in` form first); if kept,
  `ref(c)` lands beside `canbe`.
- Verification: the Part-1b repro and the `examples/borrowing/` cases run
  byte-identical on both backends; self-strike (`i == j`) is correct.

### v2 — `at` on the handle: generic and custom containers (maps, structs)

The one genuinely new backend piece: lift the "known indexable type" cut so
re-materialization is `at(c, position)` for any container.

- A `ref(c)` over a generic `C` re-materializes through its resolved `at`
  (static call at a known-type use; the existing `?at`-closure parameter at
  an erased boundary — now *named*, `ref(c, ?at)`, not anonymous).
- Ship `at` for `Map` (by-key) — **closes finding 1a**; the Step 2
  confirmed-refusal guard flips to an accept-and-run test. Ship `at` for
  arrays/`Deque` if not already, and document the recipe for a user struct's
  `at`.
- Purely *additive*: v1a/v1b programs are unchanged; v2 only removes a
  refusal (the type-erased-locator cut), so nothing regresses.
- Verification: a map-chaining program and a custom-container (`Grid`-style)
  program compile and run identically on both backends.

### Sequencing notes

- v1a is vocabulary-only and lands first; it is the safe starting point (no
  backend change) and gives v1b/v2 correct terminology.
- v1b carries the `canbe` decision; confirm `ref(c)` subsumes anchored
  `canbe in` before retiring `canbe`.
- v2 is backend-last, as every feature in this codebase is built
  (checker/Kotlin first, Rust real-borrow machinery last).
- Retire `ROADMAP.md` item 15 as part of v1a (it describes a design — region —
  that is not being built).

## Part 9: storage paths, worked through (2026-10-10)

The design the 2026-10-10 decisions settle, worked through on a custom
container whose path crosses a field, a map key and a list index. The Salvo
half type-checks today and runs on Kotlin (`bench 1` / `Cy 7`, without the
`NotSame` block); the Rust half is the target rendering, written by hand.

```
struct Player canbe Mut { name: Str, goals: Int }
struct Team   canbe Mut { coach: Str, roster: List<Mut Player> }
struct League canbe Mut { teams: Map<Str, Mut Team>, bench: List<Mut Player> }

fn at(l: League, team: Str, n: Int) [] -> ref(l) Mut Player? => l, team, n {
    if n < 0 { return at(l.bench, -n - 1) }       // .bench[i]
    let t = at(l.teams, team)                      // .teams{slot}
    if t is None { return None }
    return at(t.roster, n)                         // .teams{slot}.roster[i]
}

fn trade(l: League, a: ref(l) Mut Player, b: ref(l) Mut Player) -> None { … }

let star = at(league, "red", 1)!
star.goals = star.goals + 1
println("bench ${size(league.bench)}")             // a read between writes
trade(league, star, star)                          // self-trade
let sub = at(league, "", -1)!
if sub is NotSame(star) { score(star, sub) }       // two live handles
```

Rust, the target:

```rust
#[derive(Clone, Copy, PartialEq)]
pub enum LeaguePlayerPath { Teams(usize /* slot */, TeamPlayerPath), Bench(usize) }
#[derive(Clone, Copy, PartialEq)]
pub enum TeamPlayerPath { Roster(usize) }

impl LeaguePlayerPath {
    pub fn walk<'a>(&self, l: &'a mut League) -> &'a mut Player {
        match *self {
            LeaguePlayerPath::Teams(s, p) => p.walk(&mut l.teams[s]),
            LeaguePlayerPath::Bench(i) => &mut l.bench[i],
        }
    }
}

// `at` itself: the body runs once, at the mint, and answers the path.
pub fn at(l: &League, team: &String, n: i32, hash: HashFn<String>, eq: EqFn<String>)
    -> Option<LeaguePlayerPath> { … }

let __h1 = crate::at(&league, &"red".into(), 1, …).expect(…);
__h1.walk(&mut league).goals += 1;               // each use walks the path
pub fn trade(l: &mut League, a: &LeaguePlayerPath, b: &LeaguePlayerPath) { … }

// Two live handles: split where the paths diverge — the same list by
// `split_at_mut`, the same map by a slot split, different fields natively.
impl League {
    pub fn walk_pair(&mut self, a: &LeaguePlayerPath, b: &LeaguePlayerPath)
        -> Option<(&mut Player, &mut Player)> { … }
}
if __h4 != __h1 {                                  // NotSame
    let (p, q) = league.walk_pair(&__h1, &__h4).expect("salvo: handles proven distinct");
    crate::score(p, q);
}
```

Recorded consequences and open points:
- **The path types are the Rust backend's own declarations, not IR
  enums.** An IR enum is a Salvo union, with Salvo semantics and a Kotlin
  rendering; a path is how *one* backend spells `ref(c)` [core-layers]. The
  IR already carries what the backend reads to build paths: places with
  field steps, the bodies of the mints, `FnDecl::borrows`/`ref_anchors`
  (which parameter a `ref` is a handle into, so its type). A generic `C`
  gets the path type as an extra Rust type parameter `P`, which the backend
  adds the way it adds lifetimes.
- **New host primitive**: `Map::pair_mut(slot, slot)`, the slab's
  `split_at_mut`.
- **Recursive containers** need a recursive path (boxed or `Vec` steps), an
  allocation per handle; they wait on the recursive-types defect anyway.
- **Open: `NotEq` beside `NotSame`.** For borrowing, `NotSame` subsumes the
  index proof (on a list, paths are indices, so the runtime test is the
  same). What `NotEq` alone gives is a proof *before* any handle exists,
  as on `update2`'s signature — which `NotSame` on two `ref(list)`
  parameters says equally well. Retiring it would also retire the
  minting-index identity machinery ([elem-distinct]'s `elem_idx`,
  `live_distinct_pairs`). The user's call.
