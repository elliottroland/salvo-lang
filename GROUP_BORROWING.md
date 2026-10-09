# Group borrowing: findings, alternatives, and the committed design

Status: **Parts 1–6 are the research trail (exploration and rejected
alternatives); Part 7 is the committed design** (user decision, this
session). Parts 1–2 record two verified defects in the current
`canbe`/virtual-place implementation and survey Nick Smith's "group
borrowing" proposal plus incremental fixes. Parts 3–6 develop and critique
a purpose-built group container, including a rejected `Rc`-based storage
variant and an accepted swap-placeholder refinement. Part 7 states the
final design: `region { }` as a compiler-intrinsic delimiter, `Reg` as a
provenance qualifier, and `Placeholder<T>` as the swap-value mechanism.
Part 8 is the implementation plan. See `ROADMAP.md` item 15, which this
design replaces.

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

## Part 7: the committed design — `region { }`, `Reg`, and `Placeholder<T>`

**Status: committed (user decision, this session).** Supersedes Part 2's
unmodified group borrowing, Part 4's automatic-synthesis option, and
Part 5's `Rc`-based storage. Builds on, and **replaces**, the
designed-but-unbuilt Regions feature recorded at `ROADMAP.md` item 15 and
`COMPLETED.md`'s "Regions" section — this design was arrived at
independently, inspired by Regions' vocabulary (`reg`/`unreg`, `Reg` as a
provenance qualifier, a scope-owned handler) without resurrecting Regions'
own freeze/escape mechanics, which predate group borrowing's slot-based
proof that free *mutable* aliasing can be sound without first freezing a
value. Item 15 should be retired in favor of this design, not implemented
alongside it — see Part 8 for the concrete roadmap edit.

### The design, stated plainly

- **`region { }` is a compiler-intrinsic delimiter, like `try { }`.**
  Not a `use`-registered effect handler — Salvo owns it the same way it
  owns `try`. Nesting is permitted syntactically; the innermost enclosing
  `region { }` is unambiguously the one any `reg`/`unreg` call inside it
  targets, mirroring `try`'s innermost-wins rule for `throw` exactly.
- **An outer region's `Reg T` value, merely *used* inside an inner
  region, materializes automatically — it does not move, and the outer
  handle stays valid both during and after the inner block.** This is
  not a new mechanism: it is the same call-boundary coercion every
  qualifier gets (`Qual T <: T`, `[qual-erasure]`), except that unlike a
  state qualifier (free to drop — it never had a runtime representation),
  dropping `Reg` costs one registry read, because a `Reg T` handle *is* a
  position into the region's registry under the hood. Subtyping
  (`Reg T <: T`, so a `Reg T` is accepted anywhere `T` is expected) is
  free to state; materializing it is not free to *run*, and the IR needs
  a node for that read (`ExprKind::Widen` is currently scoped to numeric
  promotion only — check whether to extend it or add a sibling node
  before building Step 6). The outer `Reg T` binding is untouched by
  this: nothing was removed from the registry, only read, so it remains
  exactly as valid afterward as it was before the inner block opened.
- **Joining the *inner* region's own registry — so a value can alias
  with other inner-region values — is a second, explicit, visible
  step: `.reg()` (or equivalent call syntax), never synthesized.**
  Composing the two existing operations (`reg` reading the materialized
  value, as above, then placing it in the registry currently in scope —
  which `reg`'s innermost-wins resolution already makes the *inner* one)
  produces a new handle, independent of the outer one, scoped to the
  inner region:
  ```
  fn act<T>(list: List<T>) [Region] -> None {
      region {
          let t: Reg T = reg_get(list, 0)!
          do_something(t)              // materializes; t (outer) still valid after
          region {
              // t: T here, from a type-checking perspective (materialized on use);
              // the outer `t` is still a valid Reg T, untouched, once this block ends
              let t2 = t.reg()         // explicit: materializes, then reg()s into
                                       // *this* block's registry -- a real, visible move
          }
      }
  }
  ```
  This needs no suspend/restore/poisoning machinery on the outer binding
  at all — an earlier draft of this design proposed poisoning the outer
  name for the inner block's duration and restoring it at block-close,
  which turned out to be unnecessary once materialize-on-use was
  separated from registry-join: the outer handle was never going to stop
  being valid, because reading it never consumes it.
- **`Region` is an effect with two members**, in the same register as
  `Throw`:
  ```
  export effect Region {
      fn reg<T>(value: T) -> Reg T => !value
      fn unreg<T>(handle: Reg T) -> T => !handle
  }
  ```
  `reg`/`unreg` are ordinary consuming/producing calls — no new deduction
  machinery, no new move-tracking rule. A function that calls either
  declares `[Region]`, propagated exactly like any other effect
  requirement; `region { }` satisfies it for its body the way `try { }`
  satisfies `[Throw<M>]`.
- **`Reg` is a provenance qualifier, not a replacement type.** `Reg Mut
  Entity` *is* a `Mut Entity` for every purpose other than the checker's
  aliasing proof: field reads, field writes, method calls, and passing it
  anywhere a `Mut Entity` is expected all work exactly as they do on a
  plain handle today — confirmed against `docs/language/Mutable-
  Handles.md`'s own `heal`/`attack` examples, which write `e.hp = e.hp +
  10` directly, no accessor call. `Reg T <: T` falls straight out of the
  general qualifier-erasure rule (`[qual-erasure]`, `Qual T <: T`) with no
  special case — `Reg` is a qualifier, not an obligation (`proj`/`once`/
  `linear`), so it subtypes the permissive direction like every other
  qualifier; there is nothing to special-case or flag here.
- **`attack` needs no `canbe` at all — but only when both handles share
  one region.** Two `Reg`-tagged arguments into the *same* running region
  are provably safe to alias by construction — there is only one registry
  once both values are `reg`'d, so there is no second anchor for the
  checker to lose track of (closing finding 1b by construction, not by
  validation). **Two `Reg` handles from *different* regions are refused
  outright, with no exception** — every `Reg` qualifier named in one
  function signature must resolve to the same region, the same
  conservative default two plain `Mut` parameters with no `canbe` already
  get today; there is no shared registry between two unrelated regions,
  so there is nothing to prove the aliasing permission from. `canbe`
  remains exactly what it is today, for the no-registry, declared-trust
  case; `Reg Mut T, Reg Mut T` into one region is the registry-backed
  route to the same permission, and a function does not need to declare
  both:
  ```
  fn attack(a: Mut Entity, d: Mut Entity) -> None => a: Mut, d: Mut {
      a.energy = a.energy - 1
      d.hp = d.hp - 2
  }
  ```
  called as `attack(ha, hb)` where `ha, hb: Reg Mut Entity` — the checker
  accepts this because both arguments carry `Reg` into the same region,
  independent of whether `attack` itself ever mentions `canbe`.
- **Intrinsic types declare their obligations the same way user types do
  — no special-cased built-in list.** `: Params<self>` already exists for
  `struct`/`type` declarations (`struct Point : Ordered<self> by auto,
  Hashed<self> by auto`); extending it to `intrinsic type` means
  `Placeholder` (and every other obligation an intrinsic type satisfies)
  is a declared, checked claim rather than an implicit, unconditional
  one:
  ```
  intrinsic type Int : Ordered<self>, ToStr<self>, Hashed<self>, Placeholder<self>
  ```
  Unlike a user struct, there is nothing to `by auto`-stamp — the
  implementation is a hand-written `intrinsic fn`, so the declaration is
  purely a checked claim (matching `: Ordered<self>` *without* `by auto`
  on a user struct: a promise the compiler verifies against existing
  function declarations, not a body it generates). This is new parser/
  checker surface — `: Params<self>` support for `intrinsic type` does
  not exist today — and is sequenced as its own step (Part 8, Step 0)
  before `Placeholder<T>` is built, since `Placeholder<self>` on `Int`
  presupposes it. It also fixes a standing inconsistency as a side
  effect: today `Int`'s `cmp`/`hash`/`to_str` support is unconditional on
  importing `core` at all, regardless of use, where a user type's
  `: Params<self>` claim is declared and checked — bringing intrinsic
  types onto the same footing closes that gap for every obligation, not
  only `Placeholder`.
- **Swapping is explicit and reusable, never baked into existing
  functions.** `remove_at`'s meaning does not change. A new `params`
  bundle supplies the placeholder a swap needs, following the same
  caller-fills-the-concrete-shape idiom already used by `Locate`, `Yield`,
  `Ordered`, `Eq`:
  ```
  export params Placeholder<T> {
      fn placeholder() -> T
  }

  fn reg_from<T>(list: Mut List<T>, index: Idx(list) Int, ?Placeholder<T>) [Region] -> Reg T {
      return reg(replace(list, index, placeholder()))
  }

  fn unreg_into<T>(list: Mut List<T>, index: Idx(list) Int, handle: Reg T) [Region] -> None {
      replace(list, index, unreg(handle))
      return None
  }
  ```
  `replace` is already `core.list`'s `[col-replace]` — "the total
  positional write: puts `value` at `index` and answers the element it
  displaced" — so `reg_from`/`unreg_into` need no new backend primitive,
  only a new `params` bundle and two small library functions composing
  existing ones. A type with no sensible placeholder simply never gets a
  `Placeholder<T>` instance written for it, and `reg_from` fails to
  resolve at the call site with an ordinary missing-implicit diagnostic —
  not a new failure mode.
- **Invalidation is minimized by reusing the dependent-qualifier
  growth-preservation rule**, not by inventing a new one. `Idx` already
  survives a list's `add`, `swap`, and `replace` because none of those
  move an existing index's boundary (`docs/language/Dependent-
  Qualifiers.md`). `reg_from`'s swap is exactly a `replace` at a fixed
  index, so by the same reasoning: **a handle or claim into the source
  container survives `reg_from` unless it specifically names the slot
  that was swapped.** A handle to a *different* element is untouched —
  the container's shape never changed, only one slot's contents did. This
  is the same "storage identity, not whole-container" principle
  `Mutable-Handles.md`'s `e.hp: Mut` vs `e: Mut` field narrowing already
  uses, applied to `reg`/`unreg` instead of to an ordinary mutating call.
- **Closing a `region { }` block automatically reconciles anything still
  checked out.** The block already has to walk everything tagged with its
  scope to enforce an escape rule (Regions' existing escape-rule
  machinery is the right starting point for this walk, even though its
  freeze semantics are not being reused); the natural extension is that
  anything still holding a live `Reg T` handle when the block ends is
  swapped back via the same `unreg_into`-shaped call the author would
  have written by hand, removing the need to remember a matching
  `unreg_into` on every exit path (including early `return`/`throw` paths
  through the block).

### Worked example (final, corrected)

```
export effect Region {
    fn reg<T>(value: T) -> Reg T => !value
    fn unreg<T>(handle: Reg T) -> T => !handle
}

export params Placeholder<T> {
    fn placeholder() -> T
}

fn reg_from<T>(list: Mut List<T>, index: Idx(list) Int, ?Placeholder<T>) [Region] -> Reg T {
    return reg(replace(list, index, placeholder()))
}

fn unreg_into<T>(list: Mut List<T>, index: Idx(list) Int, handle: Reg T) [Region] -> None {
    replace(list, index, unreg(handle))
    return None
}

struct Entity canbe Mut {
    hp: Int,
    energy: Int
}

params Placeholder<Entity> {
    fn placeholder() -> Entity { return Entity { hp: 0, energy: 0 } }
}

// No `canbe`: aliasing permission comes from the call site passing two
// Reg-tagged handles into the same region, not from anything declared here.
fn attack(a: Mut Entity, d: Mut Entity) -> None => a: Mut, d: Mut {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
}

fn main() [use] {
    use StdOutConsole()

    let squad_a: Mut List<Mut Entity> = mut_list_of(Entity { hp: 30, energy: 4 })
    let squad_b: Mut List<Mut Entity> = mut_list_of(Entity { hp: 8, energy: 9 })

    region {
        let ha = reg_from(squad_a, 0)
        let hb = reg_from(squad_b, 0)

        attack(ha, hb)   // two Reg Mut Entity handles into one region:
                          // the case that silently miscompiled under
                          // plain `canbe` with two different anchors (1b)

        unreg_into(squad_a, 0, ha)
        unreg_into(squad_b, 0, hb)
        // anything still checked out when the block ends is swapped back
        // automatically here
    }

    println("${get(squad_a, 0)!.energy} ${get(squad_b, 0)!.hp}")
}
```

Field access on `a`/`d` inside `attack` is plain field syntax throughout —
`a.energy`, `d.hp` — with no accessor call of any kind. The only `get(...)`
in the whole example is the ordinary `List::get` at the final `println`,
unrelated to `Reg`.

## Part 7a: an open defect in the committed design — double-checkout of one slot (user-found, 2026-10-08)

**Status: open soundness gap in Part 7, needs a design decision before
Steps 3–7 build on `reg_from`/`unreg_into`.** Found by the user.

### The gap

`reg_from(list, i)` is `reg(replace(list, i, placeholder()))`: it swaps the
real element *out* of `list[i]` and leaves a `Placeholder<T>` value sitting
in that slot. The handle it answers captures the real element. Now suppose
the program checks out the *same slot twice* — not with a statically-equal
index, but with two **runtime** indices that happen to coincide (a random
draw, an unknown index, two searches that land on the same element):

```
region {
    let ha = reg_from(squad, i)   // squad[i] := placeholder, ha captures the real entity
    let hb = reg_from(squad, j)   // j == i at runtime: squad[i] is ALREADY the placeholder
    attack(ha, hb)                // ha and hb do NOT alias — hb is the bogus placeholder
    unreg_into(squad, i, ha)
    unreg_into(squad, j, hb)      // writes the placeholder-derived value back over squad[i]
}
```

When `j == i`:

- `ha` holds the real entity; `hb` holds a **fabricated placeholder**
  (`Entity { hp: 0, … }`), not the entity. The two handles name different
  values — yet the program's intent was two handles to one element, which
  is *exactly the self-aliasing case* `canbe`/regions exist to serve.
- Mutations through `ha` and `hb` go to different places; `attack(ha, hb)`
  silently does the wrong thing.
- At block close (or the explicit `unreg_into`s), both handles are swapped
  back into index `i`. One write clobbers the other, and the placeholder's
  fabricated field values can overwrite the real entity's data.

This is **silent corruption, not a refusal** — the same class of bug as
finding 1b, reintroduced by the swap mechanism in the very case the feature
is for.

### Why Part 7's safety argument misses it

Part 7's invalidation bullet says a `reg_from` invalidates "a handle or
claim that **specifically names the slot that was swapped**," reusing the
`Idx` growth-preservation rule. That rule is a **compile-time** fact: it
fires only when the checker can see that two indices denote the same slot
(a literal `0` twice, or the same variable). The double-checkout gap lives
precisely where the checker *cannot* see it — two independent runtime
indices that might be equal. The feature's reason to exist (unknown indices
that might coincide) is the exact case the static rule cannot cover, so the
second `reg_from` type-checks and the collision only shows up at runtime,
as a placeholder.

Generational handles (Part 3, left out of Step 7's v1) do **not** fix this:
they detect a stale handle into the *group's* storage after a `remove`;
here both handles are fresh and the defect is in what the second one
*captured* from the source list — a placeholder, because the slot was
already emptied.

### Options (for the user to decide)

- **A. Runtime guard in the slot: `replace` into an already-checked-out
  slot is detected.** Make `reg_from` leave a *distinguished* placeholder
  (or set an `Option`/tombstone/"on loan" marker on the slot) and have a
  second `reg_from` on a slot already marked on-loan **fail** rather than
  swap again. The failure mode has to match the language's existing
  conventions — most naturally the same `!`/optional discipline (`reg_from`
  answers an optional, `None` when the slot is already on loan), or a
  `throw`. Pro: closes the gap for the real (runtime-index) case, cheap
  (one check per `reg_from`). Con: adds a runtime failure where the model
  has otherwise kept the question compile-time; needs a decision on *what*
  the failure is (optional vs. throw vs. panic) and whether an on-loan
  slot is representable without changing the container's element type (the
  same `Option<T>`-vs-fabricated-placeholder tension Part 6 already
  flagged).

- **B. Make the slot *alias* on second checkout instead of re-swapping.**
  A second `reg_from` of an already-on-loan slot returns a handle into the
  **same region entry** the first checkout created, rather than minting a
  fresh one — so `ha` and `hb` genuinely alias, which is the intended
  semantics. This is the most *correct* answer (it makes the double-
  checkout behave exactly as `canbe` self-aliasing should) but the most
  expensive: it requires the slot to remember *which region entry* it was
  moved into (a back-reference from source slot → region handle), so the
  second `reg_from` can find and reuse it. That back-reference is new state
  the current design does not carry, and it has to survive on the source
  container, not the region.

- **C. Refuse the shape at compile time: forbid two `reg_from`s from one
  container when the indices are not proven distinct.** Mirror the existing
  one-handle-at-a-time / `NotEq` discipline: two `reg_from(list, i)` /
  `reg_from(list, j)` into the same list are refused unless `j is NotEq(i)`
  proves them apart (distinct slots, so no collision) — the same proof the
  checker already understands. Pro: no new runtime mechanism, reuses
  `NotEq`, and it is honest (the model already asks for `NotEq` to put two
  list handles in one call). Con: it *removes* the headline capability —
  the whole point was to allow the possibly-coinciding case; option C
  turns "unknown indices that might coincide" back into "prove them apart
  first," which is what regions were meant to improve on. It closes the
  hole by refusing exactly the programs the feature was for.

- **D. Accept the restriction and document it: `reg_from` requires the
  caller to not double-check-out, undefined/placeholder result if they do.**
  The do-nothing option, rejected on sight for the same reason 1b was: a
  silent-wrong-answer is exactly what `[backend-never-wrong]` forbids. Not
  viable as stated; listed only as the baseline the others must beat.

**Recommendation for discussion: A, with the failure as an optional
(`reg_from` answers `(Reg T)?`, `None` on an already-on-loan slot), unless
the self-aliasing-should-just-work intent is strong enough to pay for B.**
A is the smallest change that satisfies `[backend-never-wrong]` (it turns
the silent corruption into a visible, handleable `None`), reuses the `!`
discipline already everywhere in the language, and leaves the common
single-checkout case untouched and zero-cost. B is the "most right" answer
and worth it if the double-checkout-aliases-correctly behavior is
considered essential rather than a corner — but it is materially more
machinery (source-slot → region-entry back-references) and should be a
deliberate choice, not a default. C is a clean fallback if neither runtime
cost is acceptable, at the price of the feature's main selling point. This
is a language-design call and is left to the user.

## Part 7b: proposed resolution — reg the *container*, not the element (user direction, 2026-10-08)

**Status: proposed, supersedes Part 7's element-swap mechanism
(`reg_from`/`unreg_into`/`Placeholder<T>`) pending user confirmation.** The
user's direction: self-aliasing is a *must-have*, and the cleanest way to
get it is to stop moving elements out of containers at all. Instead of
`reg`-ing elements taken from a container, **`reg` the container once** and
gate element access through region-controlled accessors that answer
`Reg T` without removing anything.

### Why this is the right shape

The double-checkout gap (Part 7a) exists only because `reg_from` *empties
the source slot* — so a second access to the same slot finds a placeholder
instead of the element. If nothing is ever removed, there is no placeholder,
no emptied slot, and **two accesses at the same index are simply two
positions into one container** — which on Rust is `&mut regged[i]`
re-materialized per use (`[rs-elem-mut]`), and on Kotlin is native
reference aliasing. Self-aliasing stops being a special case that needs a
mechanism and becomes the ordinary rendering the committed design already
uses for `canbe` and `NotEq`. **The hard case the feature exists for solves
itself**, with no back-reference (Part 7a option B), no tombstone (option
1 the user raised), and no runtime on-loan marker (option A).

Option 1 the user raised — a placeholder that *forwards* to the region
entry — was checked against the backend and **rejected on the same ground
Part 5's `Rc` was**: `List<T>` is `Vec<T>` (`std/platform/core/list.rs`:
`pub type List<T> = Vec<T>`), elements stored inline by value, so a
forwarding placeholder cannot be a `T` — it would force `Vec<T>` into
`Vec<T | Forwarded>`, changing the representation of every list and taxing
every read of every container, regged or not. Reg-the-container reaches the
same goal (self-aliasing on double access) without touching `Vec<T>`'s
representation at all.

### The design, stated plainly

- **`Reg` becomes a *dependent* provenance qualifier `Reg(C)`**, not a bare
  one: a `Reg(squad) Mut Entity` is "a mutable-element handle whose source
  is the regged container `squad`." This is the `Idx(list)`/`KeyOf(map)`
  shape (`docs/language/Dependent-Qualifiers.md`), reused — the container
  is a value slot filled by a place, and the claim is bound to that place's
  fate-root identity. It subtypes to the bare handle (`Reg(C) Mut T <: Mut
  T <: T`, `[qual-erasure]`) for every non-aliasing use, exactly as Part 7
  intended, so field access stays plain (`a.hp = …`).
- **Aliasing permission is decided by the container, not a `canbe`
  clause.** Two `Reg(C)`-tagged arguments with the **same** `C` may alias —
  there is one `&mut Vec` and two positions into it, the self-strike case
  included. Two `Reg` arguments with **different** containers are refused:
  they name different `Vec`s and genuinely cannot alias (which also
  corrects Part 7's quiet error — co-locating two entities from `squad_a`
  and `squad_b` in one region and calling that "aliasing" was inventing a
  relationship the backends cannot honestly share; Kotlin's two objects
  have nothing to alias). So `attack(a: Mut Entity, d: Mut Entity)` still
  takes two handles with **no `canbe`**, when both carry `Reg(C)` for one
  `C` — the Part 7 property is kept, now resting on container identity the
  checker actually tracks.
- **Access is monadic / gated**, as the user put it: `reg` hands back a
  region-scoped view of the container, and the only way to get a `Reg(C) T`
  is through an accessor the region controls (`reg_get(view, i)`), so the
  region sees every mint and the escape rule (Part 7's reused machinery)
  keeps `Reg` values from outliving the block. No value leaves its
  container; `squad[i]` always holds the real `Entity`.
- **`Placeholder<T>`, `reg_from`, `unreg_into` are dropped.** They existed
  only to move elements out and back; with the container regged in place,
  there is nothing to swap, so Step 0's `Placeholder` groundwork and
  Step 3/4's swap functions fall away. (Step 0's `: Params<self>` on
  `intrinsic type` work already landed and is independently useful — it
  stays; only its *motivation* changes.)

### Worked example (option 2)

```
export effect Region {
    // reg a container, get a region-scoped view; accessors below mint Reg(C) handles
    fn reg<C>(container: C) -> RegView<C> => !container
}

fn attack(a: Mut Entity, d: Mut Entity) -> None => a: Mut, d: Mut {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
}

fn main() [use] {
    use StdOutConsole()
    let squad: Mut List<Mut Entity> = mut_list_of(Entity { hp: 30, energy: 4 },
                                                  Entity { hp: 8, energy: 9 })
    region {
        let view = reg(squad)                 // the *container* joins the region
        let a = reg_get(view, i)!             // Reg(squad) Mut Entity
        let d = reg_get(view, j)!             // Reg(squad) Mut Entity, j may equal i
        attack(a, d)                          // same container -> may alias, no canbe,
                                              // self-strike (j == i) is just two
                                              // positions into one &mut Vec
    }
    println("${get(squad, 0)!.energy} ${get(squad, 0)!.hp}")
}
```

### Honest costs and open questions (for the user)

1. **`Reg(C)` is a dependent qualifier, which is more than Part 7's bare
   provenance qualifier** — it carries a container place and the checker
   compares those places to decide aliasing. The machinery exists (`Idx`,
   `KeyOf`), but this is more checker work than a plain mint-only
   qualifier, and the "same container" test is place-identity, with the
   same fate-root-aliasing subtleties `Idx` already handles.
2. **Shape mutation of a regged container during the region** (`add`,
   `remove_at`) could dangle a `Reg(C)` handle. The region must either
   forbid shape changes on a regged container for the block's duration
   (simplest, and consistent with the one-handle discipline) or preserve
   `Reg` only across `[col-replace]`-style in-place writes the way `Idx`
   survives them. **Open: which, and is forbidding shape change too
   strict?**
3. **Cross-container aliasing is now *impossible*, not merely explicit.**
   If any real use case needs two genuinely-different containers' elements
   to alias (the original "call site 2"), reg-the-container cannot express
   it — but that case is physically un-aliasable on Kotlin anyway, so the
   honest answer is that it was never a sound capability. **Open: confirm
   no real workload needs it** before closing the door.
4. **A `RegView<C>` wrapper vs. tagging the container in place.** The
   example shows a `RegView<C>` value; an alternative is to leave `squad`
   as-is and let `reg_get(squad, i)` within a `region { }` block mint
   `Reg(squad)` directly, with no view value. **Open: view value or
   block-scoped tag** — the latter is lighter but needs the checker to know
   "inside this region, `squad` is regged."

### Recommendation

Adopt option 2 (reg the container). It is the only option that makes
self-aliasing — the stated must-have — *free and automatic* rather than a
mechanism bolted on, it removes the double-checkout gap by construction, it
avoids both the inline-storage wall (option 1) and the per-slot runtime
marker (Part 7a option A), and it models the feature more honestly (same
container may alias, different containers cannot). The cost is a dependent
`Reg(C)` qualifier instead of a bare one and a decision on shape-mutation
during a region — both bounded, both reusing machinery already in the
checker. If adopted, Part 8 is re-planned: Steps 3–4 (`Placeholder`, swap
functions) are deleted, and the new Step 3 is "`Reg(C)` as a dependent
provenance qualifier + the container-aliasing rule," before the delimiter
(Step 6) and backend (Step 7). **This is a language-design call; it is the
user's to confirm.**

## Part 7c: custom `Reg` containers via a materialization bundle (user direction, 2026-10-08)

**Status: proposed refinement of 7b.** The user's direction: `reg(list)`
answers `Reg List<T>`, element access is qualifier-overloaded
(`fn get<T>(list: Reg List<T>, index: Int) [Region] -> (Reg T)?`), and the
`Region` effect should give users the tools to make their *own* containers
reg-able — which means a user must be able to supply the materialization
logic that turns a position in their container into a `Reg T` within a
region.

### The one thing a custom container must supply: a `Locate`-shaped accessor

Salvo already has the exact primitive this needs. `core.list`'s
`params Locate<C, L, T> { fn at(c: C, l: L) -> proj(c) Mut T? }`
(`std/core/list.sv:116`) is "the caller supplies how to find a mutable
element of container `C` at location `L`," and its own doc comment says
what it compiles to: *"a locator is what the Rust backend renders this as
[rs-loc]: position data crossing the closure boundary, materialized at the
use site — which is why a generic algorithm may hand out mutable handles at
all."* That **is** the materialization logic the user is asking to provide.
A position re-materialized on each use is exactly how `Reg T` already has
to be rendered on Rust (`[rs-elem-mut]`), and it is identity on Kotlin.

So the recipe for a custom `Reg` container is the same shape as making a
type iterable (write `next`) or locatable (write `at`): **implement the
materialization accessor, and the `Region` machinery lifts it to mint
`Reg` handles.** No per-container compiler support, no built-in list
special case — `List` is just the container that ships the canonical `at`
(`[col-locate]`), the way the scalars ship the canonical `cmp`.

### How the lift works — `Region` provides the wrapper, the user provides `at`

```
// core.region (sketch; spelling provisional)

// A container is reg-able when it offers a materialization accessor —
// the Locate bundle, reused.
export params Regable<C, L, T> {
    fn at(c: C, l: L) -> proj(c) Mut T?
}

export effect Region {
    // reg a container; its elements become reg-able for the block.
    fn reg<C>(container: C) -> Reg C => !container
}

// The lifted accessor: indexing a *regged* container yields Reg handles.
// Generic over any reg-able container, delegating to the user's `at`.
export fn get<C, L, T>(c: Reg C, l: L, ?Regable<C, L, T>) [Region] -> (Reg(c) T)?
=> c {
    // `at` produces the position; the region tags it as its own.
    // On Kotlin this is identity; on Rust the position is re-materialized
    // per use against the live container.
    return reg_handle(at(c, l))
}
```

A user's custom container reaches all of this by one declaration plus one
function — nothing region-specific to learn beyond "write `at`":

```
struct Grid<T> canbe Mut { cells: List<Mut T>, width: Int }

// The materialization accessor: a position in a Grid is a row/col pair.
fn at<T>(g: Grid<T>, pos: (Int, Int)) -> proj(g) Mut T?
=> g, pos {
    return get(g.cells, pos.0 * g.width + pos.1)
}

region {
    let rg = reg(grid)                 // Reg Grid<T>
    let a = get(rg, (r1, c1))!         // Reg(rg) Mut T, via the user's `at`
    let d = get(rg, (r2, c2))!         // Reg(rg) Mut T; may be the same cell
    attack(a, d)                       // same regged container -> may alias
}
```

### The aliasing rule, made precise: `Reg(C)`, keyed on the regged container

7b's open point and the user's `get` signature meet here. If `get`
answered a bare `(Reg T)?`, the result would lose its tie to `rg`, and the
checker could not tell "two handles off the *same* regged container" (may
alias) from "two off *different* regged containers" (cannot). So the result
is the **dependent** form `Reg(c) T` — the container place rides along, the
`Idx(list)`/`KeyOf(map)` pattern (`docs/language/Dependent-Qualifiers.md`)
reused once more. The rule is then clean and local:

- two `Reg(c) T` arguments with the **same** `c` (same fate-root place) may
  alias — one container, two positions, self-strike included;
- two `Reg` arguments with **different** `c` are refused — different
  containers cannot alias;
- `Reg(c) T <: T` by `[qual-erasure]`, so `attack(a: Mut Entity, d: Mut
  Entity)` still needs **no `canbe`** — the permission comes from both
  arguments sharing a regged container, exactly 7b's rule, now carried by
  the qualifier rather than by region identity.

(This supersedes Part 7's coarser "same *region*" rule, which would have
wrongly let two different containers in one region alias — the quiet error
7b already called out.)

### What the user writes vs. what the region supplies

| Supplied by | What |
|---|---|
| **User** (per custom container) | one `at(c: C, l: L) -> proj(c) Mut T?` — "find the element at this position"; nothing region-aware |
| **`core.region`** | the `Region` effect, `reg`, the lifted `get`/`at` that tags `at`'s result as `Reg(c)`, and the `region { }` delimiter + escape rule |
| **`core.list`** (and kin) | the canonical `at` for the built-ins, so `reg(list)` works out of the box |

The division is the point: the user supplies only the *cheap, re-runnable
position accessor* (what the backend needs to re-materialize a handle), and
the region supplies all the *provenance and aliasing* machinery. The user
never writes anything that moves a `T`, constructs a placeholder, or
reasons about aliasing — the gap-prone parts of Part 7 are gone because
nothing is moved out of the container at all.

### Open questions (for the user)

1. **Is `Regable` a distinct bundle, or literally `Locate` reused?** They
   are the same shape. Reusing `Locate` outright is less surface and means
   every already-`Locate`-able container is automatically reg-able; a
   distinct `Regable` bundle allows a container to be reg-able without
   being `Locate`-able or vice versa. **Lean: reuse `Locate`** unless a
   reason to separate them appears.
2. **Does `reg` take ownership of the container for the block** (`=>
   !container`, so the original name is unusable until the region ends and
   hands it back), or borrow it? Ownership is simplest for the escape rule
   and matches "the container is on loan to the region"; borrowing lets
   non-`Reg` reads continue but reopens the shape-mutation question from
   7b. **Lean: own it for the block.**
3. **Shape mutation of a regged container** (`add`/`remove`): forbidden for
   the block (simplest; `reg` owning the container gives this for free), or
   preserved across in-place writes only. Same open point as 7b item 2.
4. **`L` (the locator type) is generic** — `Int` for a list, `(Int, Int)`
   for a grid, a key for a map. This is what finally brings maps into the
   model (finding 1a): a map's `at` is a by-key lookup, so `reg(map)` +
   `get(rm, key)` closes 1a through the same mechanism, no map special
   case. **Confirm** maps should ride this path rather than a bespoke one.

### Recommendation

Adopt 7b + 7c together: reg the container, make reg-ability a user-supplied
`Locate`-shaped `at`, overload `get` by the `Reg` qualifier, and key
aliasing on the dependent `Reg(c)`. It gives the user exactly the extension
point asked for (write one accessor, get a custom `Reg` container), closes
the double-checkout gap by construction (nothing is moved out), closes
finding 1a for free (maps are just a container with a by-key `at`), and
reuses three mechanisms the language already has (`Locate`, dependent
qualifiers, qualifier-overloaded `get`). Part 8 re-plans as in 7b, with the
new Step 3 being "`Reg(c)` dependent qualifier + the `Region` effect +
the lifted `get`/`at` over `Locate`," then the delimiter and backend.

## Part 7d: `Reg` vs. the existing `proj Mut` — what each is, and how they relate (user question, 2026-10-08)

**Status: reconciliation, must be settled before building 7b/7c.** `proj
Mut` and `Reg` are both "a handle into storage someone else owns," so the
region design has to say exactly how they relate rather than quietly
introduce a second, overlapping thing.

### What a `proj Mut` is *today* (grounded in `[proj-mut]`, LANGUAGE_SPEC.md ~7305)

A **`proj Mut X` is a mutable element handle**: the result of minting a
handle into an element of a `List<Mut X>` (`get(list, i)!` →
`(proj(list) Mut X)?`), the thing `canbe`/`NotEq`/group-borrowing v1 are
all about. Precisely:

- **It is a projection carrying `Mut`.** `proj` alone is read-only
  ([proj-readonly]); the 2026-09-24 narrowing ([proj-mut], P-3 of the
  group-borrowing ladder) made a projection *that carries `Mut`* satisfy a
  kept `Mut X` position and be mutable in place. Element mutability is the
  **element type's** (`List<Mut X>`), not the container handle's.
- **It shares fate with its source** and participates in poison: mutating
  through it is a mutation event on its roots; a sibling handle into the
  same container falls unless a live `NotEq` proves them apart
  ([elem-distinct]); the acting handle survives (its storage did not move).
- **It cannot be a top-level parameter** written `proj Mut` ([proj-readonly]'s
  declaration half) — a `proj` position already *accepts* `proj Mut`
  arguments, so writing the `Mut` on the parameter is an error; the handle
  travels into functions as a plain `Mut X` parameter.
- **On Rust it is a *position*** — `container + index`, re-materialized per
  use ([rs-elem-mut]/[rs-loc]), mode inferred per binding
  (`Checked::handle_muts`). On Kotlin it is the element reference. It has
  **no runtime wrapper**; `proj` lives in the lowered type and on the fate
  link (`FateLink.borrowed && !held`).

### The difference between `proj Mut` and `Reg`

They are the **same runtime thing** — a re-materialized position on Rust,
an identity on Kotlin — and differ only in **what the checker knows about
aliasing**:

| | `proj Mut X` (today) | `Reg(c) X` (proposed 7b/7c) |
|---|---|---|
| What it is | a mutable element handle | a mutable element handle |
| Rust rendering | position, `[rs-loc]`/`[rs-elem-mut]` | **the same** position, `[rs-loc]` |
| Kotlin rendering | element reference | the same reference |
| Runtime wrapper | none | none |
| Minted by | `get(list, i)!` on a `List<Mut X>` | `get(reg_container, l)!` inside a `region { }` |
| Default aliasing of two of them into one container | **refused** — one handle at a time, unless `NotEq` proves them apart or the callee declares `canbe` | **allowed** — same regged container `c` means they may alias, self-strike included, no `canbe` |
| Who carries the "which container" fact | reconstructed from call-site syntax (the gap Part 1 found) | **carried on the qualifier**, `Reg(c)`, the dependent form |

So the one-line answer: **`Reg(c) X` is a `proj Mut X` that additionally
records its container as part of its type, which is exactly what lets two
of them alias without a `canbe` clause.** `proj Mut` left "which container"
implicit (reconstructed from syntax — the root cause of findings 1a/1b);
`Reg(c)` reifies it. Everything else — the rendering, the poison behavior,
the no-wrapper erasure — is identical.

### How they must fit together (the constraint on the region design)

Because they lower to the same handle, the region design must **not** make
`Reg` a parallel, incompatible handle kind. Two requirements fall out:

1. **`Reg(c) X <: proj Mut X <: Mut X <: X`.** A `Reg` handle must be
   usable anywhere a mutable element handle or a plain `Mut X` is expected —
   so `attack(a: Mut Entity, d: Mut Entity)` takes `Reg` arguments with no
   signature change ([qual-erasure] gives the last two steps; the first is
   new: `Reg` dropping to a bare `proj Mut` is dropping the
   container-identity record, which is sound — forgetting *which* container
   only loses aliasing permission, never gains it). This is what keeps the
   feature from fracturing the handle model.
2. **Inside a `region { }`, minting a handle from a regged container yields
   `Reg(c)`; outside, the same `get` yields today's bare `proj Mut`.** The
   qualifier-overloaded `get` from 7c is exactly this: `get(c: Reg C, l)`
   answers `(Reg(c) T)?`, while `get(list: List<Mut T>, i)` keeps answering
   `(proj(list) Mut T)?`. The two coexist; `Reg` is the strictly-more-
   informed overload, selected when the container is regged.

### Consequence for `canbe` / `NotEq`

`proj Mut` keeps its three-rung ladder unchanged (say nothing → one at a
time; `NotEq` → proven apart; `canbe` → declared alias) for the non-region
case — nothing about today's handles changes. `Reg(c)` adds a **fourth
route to the same permission**: put the container in a region, and two
handles off it alias by construction, no proof and no `canbe`. That is the
ergonomic win over `canbe` (the callee no longer has to opt in) — and it is
*additive*: `Reg` is a more-informed `proj Mut`, not a replacement for it.
A function written against plain `Mut`/`proj Mut` parameters accepts `Reg`
arguments untouched (requirement 1), so the region feature does not split
the ecosystem into reg-aware and non-reg-aware code.

### Open question this raises (for the user)

- **Does `Reg(c)` survive being passed into a function typed for plain `Mut
  X`?** By requirement 1 it is *accepted* (subtyping), but inside that
  function the parameter is a bare `Mut X` / `proj Mut` — the container
  identity is dropped at the boundary, so two such parameters revert to the
  one-at-a-time / `canbe` discipline *inside* the callee. That is correct
  and conservative (the callee never promised region semantics), and it is
  why `attack` taking two `Reg` handles that *do* alias still needs the
  **caller** to be in the region: the permission is established at the call
  site from the shared `Reg(c)`, not inside `attack`. **Confirm this is the
  intended boundary** — it mirrors how `canbe` is caller-visible API, and
  it means a function *can* opt into keeping the region knowledge by
  writing its parameters `Reg(c)` explicitly when it needs two aliasing
  handles in its own body.

## Part 7e: the collapse — drop `region`/`Reg`, extend `proj Mut` instead (user direction, 2026-10-08)

**Status: proposed, supersedes the `Region`-effect framing of 7b/7c.** The
user's direction: do not carry both `proj Mut` and `Reg`; unify them. And
the sharper observation — **if nothing is checked out (no swap, no move),
is the `region { }` scope and `Region` effect needed at all?** Worked
through below: **no.** Every job the scope had is either gone (there is no
checkout to reconcile) or already done by `proj Mut`'s fate links. So the
whole `region`/`Reg`/effect apparatus collapses into a small *extension of
the handle rule we already have*.

### What the scope was for, and why each job is already covered

The `region { }` scope + `Region` effect had three jobs across the designs:

1. **Own the swap/slot machinery and reconcile checkouts at close**
   (Part 7's `reg_from`/`unreg_into`). **Gone** under 7b: nothing is moved
   out of the container, so there is nothing to check in. This job does not
   exist anymore.
2. **The escape rule — a handle must not outlive the point where aliasing
   was proven safe.** **Already done by `proj Mut`.** `[proj-mut]` states
   it outright: *"Destruction under a live handle needs no new rule — any
   structural `Mut` use of the container poisons the handles it lent."* A
   `proj Mut` shares fate with its container ([fate-poison]); reshaping the
   container (`add`/`remove`/`set`/`swap`) or moving it poisons every
   handle into it, automatically, with no scope. A `proj Mut` physically
   cannot outlive its container's structural stability — the guarantee a
   region scope was going to re-add, `proj Mut` has had since 2026-09-24.
3. **Mark a container "regged" so `get` mints the aliasing-capable
   handle.** This only existed because `Reg` was conceived as a *separate*
   qualifier needing a switch. Fold the aliasing idea into `proj Mut` and
   there is nothing to switch on — every `proj Mut` is already the handle;
   the only question is the *rule* for when two of them may alias.

So the scope added nothing that `proj Mut` + fate links don't already
provide. It was load-bearing only in the swap design (job 1), which 7b
deleted. **Dropping `region { }` and the `Region` effect loses no safety.**

### The container identity `Reg(c)` wanted is already tracked — on the fate link

`Reg(c)` was going to add "which container" to the *type*. But `[proj-type]`
shows the type prints only `proj Mut X` — the **source is carried on the
fate link**, not the type: `FateLink { root_id, root_name, path, borrowed,
… }`. The checker *already knows* which container every `proj Mut` came
from — it is the exact `root_id` the Step 1 fix (`[GB-fix-1b]`) compares.
So the information `Reg(c)` proposed to reify is not missing; it is already
there, on the link, for every mutable element handle in the language. There
is nothing new to carry.

### What is actually missing, and all that needs to change

Only the **aliasing-permission rule**. Today two `proj Mut` handles into one
container are refused unless `NotEq` proves them apart or the callee
declares `canbe` (the three-rung ladder). The group-borrowing goal —
self-aliasing of handles into one container, with no `canbe` — is a
**fourth rung added to `proj Mut` directly**:

> **Two `proj Mut` handles with the same fate-root container may be passed
> as two arguments that are allowed to alias (self-strike included),
> provided the callee's parameters permit aliasing.** No swap, no region,
> no new qualifier — the permission is read off the shared `root_id` the
> links already carry.

The one honest question is *how the callee says "these two may alias"* —
and this is where the user's "apply `Reg(c)`'s ideas to `proj Mut`" lands.
Two sub-options:

- **E1 — keep `canbe`, drop everything else.** The callee opts in with the
  existing `=> a canbe d` clause (already built, already shipping). The
  only *new* work is what the user actually wanted from regions: make the
  **caller** side ergonomic so two same-container handles satisfy a
  `canbe` callee with zero caller ceremony — which, per the Step 1 fix,
  *already works* for the same-container case (`attack(get(sq,i)!,
  get(sq,j)!)` is accepted today when `attack` declares `canbe`). Under
  E1 there is **nothing left to build**: the double-checkout gap never
  arises (no swap), cross-container is correctly refused ([GB-fix-1b]), and
  self-aliasing within one container already compiles. Regions were solving
  a problem that `proj Mut` + `canbe` + the 1b fix already solve.
- **E2 — add a caller-inferred alias set (the real ergonomic gain over
  `canbe`).** If the goal is to drop the callee's `canbe` *too* — i.e. let
  an ordinary `fn attack(a: Mut Entity, d: Mut Entity)` with **no** clause
  take two aliasing handles purely because the caller passed two
  same-container `proj Mut` — then the new rule is: *at the call site, two
  mutable-element handles with the same fate-root may be passed to any two
  `Mut` parameters, and the backend renders them as two positions into one
  `&mut` (the `canbe` lowering) regardless of whether the callee declared
  it.* This is the one genuinely new capability, and it is a **call-site**
  rule on `proj Mut`, still with no scope, no effect, no `Reg` qualifier.
  Its cost is the one `canbe` was invented to make visible: aliasing stops
  being part of the callee's signature. (The reason `canbe` is caller-
  visible API today — [canbe-entry], "aliasability is written, never
  inferred" — is exactly this trade; E2 reverses that decision, so it is a
  DECISION, not a given.)

### What this means concretely

- **Delete** from the plan: the `Region` effect, `region { }` delimiter,
  `Reg` qualifier (bare or dependent `Reg(c)`), `Placeholder<T>`,
  `reg_from`/`unreg_into`, the `RegView`/`reg`-the-container surface, and
  the escape-rule re-implementation. None are needed.
- **Keep**: `proj Mut` exactly as it is, its fate-link container tracking,
  the Step 1 call-site shared-root check, and `canbe`/`NotEq`.
- **The custom-container goal from 7c still works, better**: a user type is
  already "reg-able" the moment it offers a `Locate`-shaped `at` —
  `at(c, l) -> proj(c) Mut T?` *is* the mutable-element-handle minting
  surface, no region needed. `update`/`update2` over `Locate` already give
  callers the aliasing transaction (`update2` takes two `NotEq`-proven
  handles). Maps join by shipping a by-key `at` — closing finding 1a as a
  pure library addition, no scope.
- **The user's "materialization logic" question answers itself**: there is
  no `Reg T` to materialize *out of*, because the handle never left the
  container. `proj Mut`'s re-materialization ([rs-loc], position per use)
  is the only materialization, and it is already how the backend works.

### Recommendation

Collapse to **E1 now, E2 only if the caller-no-`canbe` ergonomics are
judged worth reversing "aliasability is written."** E1 is essentially
*already done* — `proj Mut` + the shipped `canbe`/`NotEq` + the Step 1 fix
cover self-aliasing (same container), proven-disjoint (`NotEq`), and the
cross-container refusal, with no region machinery and no second handle
concept. That directly satisfies "I would rather not have both a `proj Mut`
and a `Reg`": there is only `proj Mut`. The region feature, in the end, was
a more elaborate route to a permission the handle model could express
directly once the swap idea (and its gap) was abandoned. E2 is the only
part that is genuinely new, is purely a call-site rule on `proj Mut`, and
should be taken only as a deliberate reversal of the `canbe`-is-visible
decision — the user's call. **This supersedes 7a–7d's `Region` framing;
ROADMAP item 15 ("Regions") can be retired rather than built.**

## Part 7f: E2's signature form — `proj(c) Mut` names the shared container, and the runtime answer (user direction, 2026-10-08)

**Status: proposed, this is the concrete form of 7e's E2.** The user's
form: instead of a `canbe` relation, name the shared container in the
parameter types —

```
fn attack(a: proj(c) Mut Entity, b: proj(c) Mut Entity)
```

— the same way `?cmp` and `(list)` dependent-slot parameters already appear
in signatures today, with the added meaning that `c` **is passed through on
the Rust backend** and that the two parameters sharing `c` **may alias each
other**. This is better than `canbe`: it says *which* container the handles
share, not merely that two of them might coincide, and the "which
parameters may alias" fact is read off the shared `c` name rather than a
separate relation.

### Why this is a reskin of machinery that already lowers correctly

A mutable element handle on Rust is **already** `(container, position)`,
and two handles into one container passed to one call **already** lower to
*container once + two positions* ([rs-elem-mut]): `salvo_pair_mut(&mut
squad[..], __h7, __h8)` for the proven-distinct case, and the anchored-
`canbe` form renders `f(c: &mut Vec<_>, __c1: usize, __c2: usize)` — the
container borrowed once, the handles as `usize` positions into it. So

```
fn attack(a: proj(c) Mut Entity, b: proj(c) Mut Entity)
```

lowers to exactly

```
fn attack(c: &mut Vec<Entity>, a_pos: usize, b_pos: usize)
```

where `a` means `c[a_pos]`, `b` means `c[b_pos]`, and aliasing `a`/`b` is
the existing `salvo_pair_mut` split (or, when the checker cannot prove them
distinct, a single `&mut` with two indices — the self-strike-safe case
`canbe` renders today). **No new runtime mechanism** — `proj(c) Mut` is a
signature-level spelling of the container-anchor the backend already
threads for anchored `canbe`. The gain is entirely in the surface: the
anchor is named in the type, uniformly with `(list)`/`?cmp`, instead of
living in a `canbe … in …` clause.

### The runtime question answered: `at` is a *construction*-time input, not stored in the handle

The user's precise question — *do we only need `?at` when constructing a
`proj(c) Mut T`, or must a separate `?at` be passed at `attack` too?* The
answer is **construction-time only**, and `at` is **not** stored in the
handle:

- A handle, once it exists, is `(c, position)` where `position` is **owned
  data** ([rs-loc]: "a locator is owned data, which is what lets it pass
  through closures"). Re-materializing it is `&mut c[position]` — for a
  `List` that is `Vec` indexing; it needs **no `at`**. `at`'s only job is to
  *compute* the `position` from a locator `l` at the mint
  (`match at(&*c, l) { Some(p) => Some(&mut c[p]), None => None }`), which
  is the `?Locate`/`?at` idiom already rendered as a locator closure
  parameter at the mint site.
- Therefore `attack` **does not** need its own `?at`. It receives `c` (the
  `&mut` container) and two already-computed positions; it re-materializes
  by indexing `c` directly. `at` was consumed entirely at the call site
  that *built* the two handles, before `attack` was called.
- The one subtlety: for a **custom** container, "re-materialize by indexing
  `c`" is not `c[position]` syntactically — it is whatever that container's
  own element access is. So the *position type* must encode enough to
  re-index the specific container, and the backend needs the container's
  re-materialization form (its `at`/indexing) available where `attack`'s
  body dereferences `a`. For `List`/arrays this is built in (`Vec`
  indexing). For a user container, this is the open question below.

### Open question (for the user): custom-container re-materialization inside the callee

For `List`, `attack`'s body re-materializes `a` as `c[a_pos]` with no help.
For a **generic or custom** container `C` with a user-defined `at`,
`attack`'s body cannot know how to turn `(c, position)` back into `&mut
element` without the container's accessor. Two ways to resolve it, and this
is the real design fork left:

- **F1 — `proj(c) Mut` is only allowed on *concrete, backend-known
  containers* (`List`, arrays, `Deque`) in a callee that dereferences the
  handle.** Then re-materialization is always built-in indexing and no
  `at` crosses into `attack`. Custom containers get the aliasing benefit
  only through the generic `update`/`update2`-over-`Locate` transaction
  shape (the callback receives already-materialized `Mut T` handles, so the
  caller's `at` did the work). Simpler; covers the motivating examples;
  custom containers alias via the existing `Locate` transaction rather than
  via a `proj(c)` signature. **Lean.**
- **F2 — a `proj(c) Mut` parameter over a generic `C` additionally
  threads the container's `?at`/`?Locate` into the callee**, so `attack`'s
  body can re-materialize a generic handle. This is strictly more general
  (a user-written `attack` over an arbitrary reg-able container) but puts an
  implicit locator parameter on the signature — the cost `?cmp`/`?at`
  already pay elsewhere, now on every `proj(c) Mut`-generic function. More
  machinery, and the implicit `at` is exactly the "separate `?at` passed at
  `attack`" the user asked whether we need — under F2 we do, under F1 we do
  not.

### Recommendation

Adopt E2 in the `proj(c) Mut` signature form (7f), with **F1** for v1:
`proj(c) Mut` names the shared container, lowers to the existing
container-once-plus-positions anchor, needs no `at` inside the callee for
the concrete containers, and reuses the anchored-`canbe` backend path
wholesale — so the build is "accept the signature syntax, resolve `c` to a
shared-anchor the way the anchored `canbe` already does, and drop the
`canbe` clause as the way to express it." `at` stays a construction-time
input at the mint, never stored in the handle, never passed to `attack`.
F2 (generic custom containers dereferenced inside a callee) is a later
extension if a real case needs it; it is the only part that would require
threading `?at` through a `proj(c) Mut` call. This keeps "only `proj Mut`,
no `Reg`" (7e) and gives E2 a concrete, buildable shape. **The F1/F2 fork
is the user's to confirm.**

## Part 7g: the general design — `ref(c, at)`, and why region is not needed even here (user direction, 2026-10-08)

**Status: proposed; this is the general case the user asked for, designed
first, with v1 derived from it at the end.** The user's direction: do not
over-simplify to concrete containers (F1) — maps, structs, and future
custom containers must get mutable handles too — so design the generic case
and work backward. Two of the user's observations are the foundation.

### Observation 1: `proj Mut` is a category error; split provenance from permission → `ref(c)`

`proj` today conflates two orthogonal things:

- **provenance** — "this is a borrow of a known source; it shares the
  source's fate, is re-materialized not owned, cannot be moved out";
- **permission** — "this is read-only" (a plain `proj` fails a `Mut`
  position).

`proj Mut` (group-borrowing v1) kept the first and *inverted* the second,
which reads as a contradiction ("a read-only borrow you can mutate"). The
implementation already keeps them separate: `FateLink` carries provenance
(`root_id` + `root_name` + `path`) and the borrow's nature (`borrowed` /
`held`), and **the `Mut`-ness is not on the link at all — it is on the
type**. So the surface conflation is the only problem.

**Proposed rename+split: `ref(c)` is the provenance qualifier — "a handle
into container `c`" — and the permission is just whatever qualifier it
carries on top.** `ref(c) Mut T` is a mutable handle into `c`; `ref(c) T`
is a read-only one. This surfaces the distinction the internals already
make (`ref(c)` *is* the link's `root_id` + `path`; `Mut` is the type
qualifier), and it dissolves the "`proj Mut` is read-only-but-mutable"
contradiction: `ref` says nothing about permission, so there is nothing to
contradict. `proj` without a source stays the owned-view borrow it is
today; `ref(c)` is the element-handle-into-a-named-container case that
`proj Mut` was straining to name. (Terminology to confirm: `ref(c)` vs.
keeping `proj(c)`; the user floated `ref`. The point is the *split*, not
the exact word.)

### Observation 2: a handle must carry its re-materialization, not a bare position → `ref(c, at)`

A bare position (`usize`) is meaningless without *also* knowing the
operation that turns it back into `&mut element`: a `List` re-indexes
`c[i]`, a map does `c.get_mut(&key)`, a struct does `c.field`, a custom
container does whatever its accessor does. In the position-only model that
operation is implicit in the container *type* — which is exactly why
`[rs-loc]` records a **type-erased locator and refuses** a "bare generic
container with no index" (BACKEND_SPEC.rust.md, the GB-5 cut). That refusal
*is* the over-simplification the user wants gone.

**Proposed: the handle carries its own re-materialization — `ref(c, at)`.**
`at` is the accessor (`at(c, l) -> ref(c) Mut T?`, the `Locate` shape),
encoded on the handle alongside the container, so re-materializing is
`at(c, position)` for **any** container, generic included — no "known
indexable type" requirement, no type-erased-locator cut. This is the user's
answer to the earlier F1/F2 fork: take F2 (generic), but make it clean by
putting `at` *on the ref* rather than threading a separate implicit `?at`
to every consumer. A consumer like `attack(a: ref(c) Mut Entity, b: ref(c)
Mut Entity)` receives `c` and two handles that each already know how to
re-materialize themselves; it needs no `?at` of its own.

### The honest question the user asked: does the generic case bring region back? No.

Three jobs a scope might be thought to own in the generic case, each
checked:

1. **Re-materialize a generic handle.** Solved by Observation 2 — the ref
   carries `at`. This is a *backend* mechanism (what data the handle is at
   runtime), not a scope: `ref(c, at)` lowers to the container plus a
   position plus the accessor, exactly as the anchored-`canbe` form already
   lowers to container-plus-positions ([rs-elem-mut], the covered-anchor
   path), with `at` added for the non-`List` case. No scope.
2. **Escape / lifetime safety.** Still the fate link: `ref(c)` carries
   `root_id` pointing at `c`, so any structural mutation or move of `c`
   poisons every handle into it ([fate-poison] / [proj-mut]'s "destruction
   under a live handle needs no new rule"), and a handle over a *temporary*
   `c` is already refused ([proj-anywhere]). Carrying `at` for
   re-materialization does not touch the link, so the guarantee is
   unchanged. No scope.
3. **Aliasing permission across a generic boundary.** `root_id` is a `u32`
   per variable (`check.rs`: `root_id: var.id`), compared by equality; a
   generic container parameter is still one `var` with one stable id, so
   two handles minted from the same `c` share `root_id` and "may alias"
   falls out of the equality check whether `c`'s type is concrete or
   generic. A checker fact, not a runtime one. No scope.

So the generic case needs **no scope, no `Region` effect, no `Reg`
qualifier** — the same conclusion as 7e, now checked against the *harder*
generic case rather than only the concrete one. Region was load-bearing
only for swap reconciliation (Part 7, deleted). **This is the answer to "I
want to know now if region is required": it is not, even for arbitrary
custom containers.**

### The general design, stated

- **`ref(c)` is a provenance qualifier**: a handle into container `c`,
  `c` a dependent slot (a place), the `Idx(c)`/`KeyOf(c)` pattern. Its
  permission is a separate qualifier (`ref(c) Mut T` vs `ref(c) T`). `ref(c)
  Mut T <: Mut T <: T` by `[qual-erasure]` (dropping `ref` forgets the
  container — only ever loses aliasing permission, sound).
- **A container becomes handle-able by providing `at(c, l) -> ref(c) Mut
  T?`** — the `Locate` bundle, reused. `List` ships `at` = indexing; a map
  ships `at` = by-key lookup (closing finding 1a); a struct ships `at` =
  field selection (the user's "index structs"); a custom container ships
  its own. One accessor per container, nothing else.
- **The handle carries `(c, position, at)`** on the backend; `at` is the
  container's accessor, `position` what `at` computed from the locator at
  the mint. Re-materialization is `at(c, position)`, uniform across
  containers. On Kotlin all of this erases — the handle is the reference.
- **`fn f(a: ref(c) Mut T, b: ref(c) Mut T)` is the aliasing signature**:
  two parameters sharing `c` may alias (self-strike included), decided by
  `root_id` equality at the call site; different `c` is refused. This
  replaces `canbe` with a form that *names the shared container*, and
  lowers to the existing shared-anchor path (container once + positions),
  plus `at` for non-`List` containers.
- **No `canbe`, no `region`, no `Reg`, no swap, no `Placeholder`.** The
  three-rung ladder for *un-shared* handles (one-at-a-time / `NotEq` /
  `canbe`) can remain as-is for back-compat, or `canbe` can be retired in
  favor of `ref(c)` signatures outright (user's call — a sweep, not a
  shim, per the no-compat invariant).

### Working backward to v1

The general design above is the target; v1 is a *subset that is sound and
useful on its own*, extended later without rework:

- **v1a — the rename+split (`proj Mut` → `ref(c) Mut`), no new capability.**
  Pure surface/terminology: introduce `ref(c)` as the spelling for the
  element-handle case, keep today's behavior exactly (concrete containers,
  position-only, the existing `[rs-loc]` lowering). This clears Observation
  1's category error and gives the later steps a correct vocabulary to
  build on. Fully buildable with no backend change — it renames what
  `get(list, i)!` already produces.
- **v1b — `ref(c) Mut` in signatures as the aliasing form**, lowering to
  the anchored-`canbe` shared-anchor path that already exists, for the
  concrete containers (`List`, arrays, `Deque`). This is E2/7f's F1,
  reached through `ref(c)` syntax — the aliasing win, no `at`-on-handle yet.
- **v2 — `at` on the handle (`ref(c, at)`)**, which lifts the "known
  indexable type" cut and brings maps, structs and custom containers in.
  This is the one piece that is genuinely new backend machinery (the
  handle grows an accessor), and it is where the generic case lands. It is
  *additive*: v1a/v1b programs are unchanged; v2 only *removes a refusal*
  (the type-erased-locator cut), so nothing regresses.

Each v-step ends green and useful; the generic case is reached by removing
a cut, never by rework.

### Recommendation and the one thing to confirm

Adopt the general `ref(c, at)` design as the target, build v1a → v1b → v2.
This gives the user everything asked for: generic containers (v2), no
region (checked against the hard case above), one handle concept (`ref`,
not `proj Mut` + `Reg`), and maps/structs indexable (v2's `at`). The one
genuine confirmation needed: **the terminology** — `ref(c)` vs. keeping
`proj(c)` for the split, and whether `canbe` is retired in favor of
`ref(c)` signatures or kept beside them. Both are the user's call; neither
changes the mechanism. **Region stays retired (item 15); this is its
replacement, and it needs no scope.**

## Part 7h: how `ref(c, at)` is written, lowered, and spelled — three concrete answers (user questions, 2026-10-08)

**Status: concretization of 7g, grounded in the existing `Locate`/`[rs-loc]`
machinery.** Three user questions about whether the design is actually
writable and sane in Rust.

### Q1 — How does a user *write* an `at`?

An `at` is an ordinary function returning a handle derived from the
container — the pattern `core.list` already ships (`std/core/list.sv:122`,
`[col-locate]`). Under the `ref` rename, the return qualifier is `ref(c)`
instead of `proj(c)`:

```
// list: a position is an index (what std already writes, re-spelled)
fn at<T>(list: List<Mut T>, index: Int) -> ref(list) Mut T?
=> list, index {
    return get(list, index)
}

// map: a position is a key — this is what closes finding 1a
fn at<K, V>(m: Map<K, Mut V>, key: K) -> ref(m) Mut V?
=> m, key {
    return get(m, key)        // the map's own by-key mutable lookup
}

// a user struct: a position is a field selector of the user's choosing
struct Grid<T> canbe Mut { cells: List<Mut T>, width: Int }
fn at<T>(g: Grid<T>, pos: (Int, Int)) -> ref(g) Mut T?
=> g, pos {
    return get(g.cells, pos.0 * g.width + pos.1)
}
```

The rules a user follows are the ones `proj`/`ref` already impose, nothing
new: the result must be *derived from the container parameter* (checked —
`[proj-infer]`'s "must derive from a named source"), and the locator type
`L` is whatever the container indexes by (`Int`, a key, a tuple). A
container is "handle-able" exactly when such an `at` is in scope for it,
the same way a type is comparable when `cmp` is in scope — and `at` can be
the canonical member of the `Locate` group so `?at`/`?Locate` resolves it
by name for generic algorithms (`[implicit-resolve]`). **The user writes
one ordinary function; there is no region, no handle type to construct, no
lifetime to name.**

### Q2 — What does a handle look like in Rust? Does it hold a closure?

**No — a bound handle is a plain position; the handle does not carry a
closure.** This is already true today and does not change. From `[rs-loc]`:

- A **bound** handle is a `usize` (or the container's position type), and
  every use re-materializes by indexing the live container:
  `let __h2: usize = …; squad[__h2].hp = …`. No closure, no stored
  accessor, no lifetime — just position data, which is why it survives
  reads of the container (where a bound `&mut` would be E0502).
- `at` appears only **transiently at the mint**, to turn a locator into
  that position: `match at(&*squad, l) { Some(__l1) => Some(&mut squad[__l1]) }`.
  Afterward only the `usize` remains.
- Re-materialization for `List` is `squad[pos]` — the **container's type**
  supplies the indexing, so no `at` is stored. For a **generic** `C`,
  re-materialization is `C::at(c, pos)` — `at` is resolved from the *type*
  per use (`[implicit-resolve]`, a static call, "nothing materialized"),
  exactly as `cmp`/`eq` resolve for a generic. **Still not stored in the
  handle** — it is a statically-known function applied at each use, not a
  value carried around.
- A closure (`&mut dyn FnMut(&C, &L) -> Option<pos>`) appears **only** when
  `at` must cross a *type-erased* boundary — a `?at` passed into a generic
  algorithm that does not know `C` concretely — and even there it is a
  **parameter at the boundary**, not a field of a persistent handle. This
  is the existing `?Locate` rendering (`&mut |c, k| …get_platform__loc(c, k)`),
  unchanged.

So the Rust handle is: **`(container-in-scope, position)`**, with
re-materialization being container indexing (concrete) or a static `at`
call (generic), and a closure only at an erased `?at` boundary. The
"`ref(c, at)`" notation is about *which `at` the checker threads*, not about
stuffing a closure into every handle. The common case (concrete container,
bound handle) is a bare `usize`, zero overhead — identical to today.

### Q3 — Does `ref(c)` include `?at` as a named function, or is it hidden?

**Hidden by default; nameable only when the user needs a non-canonical
accessor** — the `?cmp`/`Ordered` precedent exactly:

- The common spelling is just `ref(c) Mut T`, with **no `at` written**. The
  checker resolves `at` for `c`'s container type by name, the way `${p}`
  resolves `to_str` or `a < b` resolves `cmp` — the accessor is implicit,
  so `fn attack(a: ref(c) Mut Entity, b: ref(c) Mut Entity)` mentions no
  `at` at all.
- `at` becomes *visible* only in the two places implicits already surface:
  (a) a **generic** algorithm over an arbitrary handle-able container
  carries it as `?Locate<C, L, T>` / `?at` in its signature (the caller's
  resolution fills it), the same `?cmp` rides on; (b) a container with
  **more than one** sensible accessor, or a caller that wants a specific
  one, names it with the selector syntax (`by`/`@`), as orderings do
  (`SortedSet<Str>(by_len)`).
- So `ref(c, at)` as *written in 7g* is the **internal** form — "a handle
  into `c` whose accessor is `at`"; the **surface** is `ref(c) Mut T` and
  the `at` is inferred, exactly as `Idx(list)` carries a `size` implicit
  the user never writes. The user sees `ref(c)`; the checker tracks the
  `at`.

### Consequence for the plan

These three answers confirm v1a/v1b need **no** handle-representation change
(bound handles stay `usize`, `at` stays implicit and canonical for `List`),
and v2's only new work is: thread a resolved `at` into the generic
re-materialization site (the static-call case) and extend the existing
`?at`-closure boundary rendering to the `ref(c)` consumer position. No
handle grows a stored closure in any case; the generic case adds a static
`at` call per use, the erased case reuses the closure-parameter rendering
that already exists. **Writable (Q1: an ordinary function), cheap (Q2: a
position, not a closure), and quiet (Q3: `at` inferred like `cmp`).**

## Part 7i: correction and refinement — `?at` is nameable, carried by the qualifier, and self-referential at the mint (user questions, 2026-10-08)

**Status: corrects Part 7h's Q2/Q3 overstatement and resolves the
self-reference the user spotted.** Checked against `core.list`/`core.compare`
(`Sorted<T>(?cmp)`, `add_sorted`, `sort`).

### Correction to 7h: a capability is not free from `T` — it is carried, and must be nameable

7h said `at` resolves "from the type per use, nothing threaded," which
overstated it. The ground truth from `Sorted<T>(?cmp)`:

- A capability (`cmp`, `at`) is available in a body **only when it is in
  scope**, and it gets there two ways: **carried by a qualifier on a
  parameter** — `fn add_sorted(list: Mut Sorted<T>(?cmp) List<T>) { …
  insert_sorted_by(list, elem, cmp) }`, where `cmp` is nameable in the body
  *because the `Sorted<T>(?cmp)` qualifier carries it* — or **declared as a
  standalone implicit**, `fn sort<T>(list: List<T>, ?Ordered<T>)`, for an
  *unqualified* parameter. It is **not** available merely from `T`.
- So the user's Q3 is right: a `Heap<T>(?cmp)` / `Sorted<T>(?cmp)` parameter
  is exactly what puts `cmp` in scope; drop the `(?cmp)` and `<` on the
  `T`s would not resolve. The qualifier's `?cmp` slot is a
  **function-identity slot** — it carries *which* ordering, erased at
  runtime, tracked by the checker (`[cmp-carry]`).
- Therefore the user's Q1 is right and 7h's Q3 was loose: **`?at` must be
  nameable**, precisely like `?cmp` — the body refers to it to pass it
  onward (as `add_sorted` passes `cmp`), and the handle must record *which*
  `at` re-materializes it. It is not anonymous. The `Locate` group is the
  `params` group around it; under the `ref` vocabulary it should be renamed
  to match (e.g. `params Ref<C, L, T> { fn at(c: C, l: L) -> ref(c) Mut
  T? }`), so a parameter carrying a handle's accessor reads
  `ref(c, ?at) T`, the `Sorted<T>(?cmp)` shape exactly.

### The self-reference the user spotted (Q2): `at`'s own return type names *itself* as the locator

Writing

```
fn at<T>(list: List<T>, index: Idx(list) Int) -> ref(list, ?at) T
```

the `?at` in the **return type** cannot mean "an `?at` passed into this
function" (the `Heap<T>(?cmp)` reading, where `cmp` flows *in* and is
*consumed*). Here `at` is **itself** the accessor the resulting handle will
be re-materialized through — the capability flows *out*, bound to the very
function producing it. This is a real asymmetry with `?cmp`:

| | `Sorted<T>(?cmp)` | `ref(c, ?at)` |
|---|---|---|
| direction | capability flows **in**, consumed by the body | accessor flows **out**, re-invoked later |
| what fills the slot | an ordering passed by the caller | the mint's own accessor function |
| at the declaring fn | `?cmp` is a parameter the fn receives | `?at` in `at`'s own return is **`at` itself** |

So `?at` has two distinct readings depending on position, and conflating
them (as a bare `?at` would) is the confusion the user flags:

1. **At the mint (`at`'s own return type)**: the accessor is *this
   function*. Writing `?at` here to mean "me" is circular and misleading.
   **Proposed: it is inferred, not written** — a function whose return type
   is `ref(c) …` has *itself* recorded as that handle's accessor by the
   checker, with no slot written. The author writes `-> ref(list) T` (or
   `-> ref(list) Mut T?`), and "the locator is `at`" is a fact the checker
   attaches, the same way a `+Sorted` return records "sorted by the `cmp`
   this body used" without the author restating it. A delegating accessor
   (`fn at2(…) -> ref(c) T { return at(c, l2) }`) records *its* callee's
   accessor transitively — the return carries whichever `at` actually
   produced the handle.
2. **At a consumer (a function receiving a handle whose accessor it does
   not know)**: here `?at` *is* a carried capability in the `?cmp` sense —
   `fn use_two<C, L, T>(a: ref(c, ?at) Mut T, b: ref(c, ?at) Mut T)` names
   the accessor the handles carry so the body (and the backend) can
   re-materialize them, exactly as `add_sorted` names the `cmp` its
   `Sorted` parameter carries. This is the nameable, `?cmp`-shaped use —
   and it is where Q1's "must be nameable" bites.

**Proposed syntax resolution:** `?at` is written **only where it is carried
in** (the consumer, reading 2), never in a mint's own return (reading 1,
inferred). This mirrors the existing split precisely: `sort` writes
`-> +Sorted<T>(?cmp)` because it is *publishing* an ordering resolution it
performed (the `?cmp` there is "the one I resolved," already a mild version
of self-reference, `[cmp-carry]`), while `add_sorted` writes `Sorted<T>(?cmp)`
on its *parameter* because it is *receiving* one. The `ref` case is the same
distinction, sharper: a mint publishes its accessor (inferred, unwritten); a
consumer receives it (`ref(c, ?at)`, written). If an explicit spelling for
the mint is ever wanted, it should be a *publishing* form (`-> ref(list)
by at`, naming the accessor the way `by @auto` names a stamp source), never
a bare `?at` that reads as an in-parameter.

### Consequence for the plan

- The `Locate` group is renamed to the `ref` vocabulary (its member stays
  `at`), and this rename is folded into v1a (it is pure vocabulary, like
  the `proj Mut` → `ref` rename itself).
- The checker already records "which function produced this handle" on the
  fate link (`root_id` + the producing call); recording "which `at`
  re-materializes it" is the same shape — a function-identity the mint
  publishes and a consumer's `ref(c, ?at)` parameter receives, erased on
  both backends like every other function-identity slot (`[cmp-carry]`).
- No handle stores a closure (7h Q2 stands): the accessor is a
  function-identity the checker threads, materialized as a static call at
  a known-type use and as the existing `?at`-closure parameter only at an
  erased consumer boundary — which is now *named* (`ref(c, ?at)`) rather
  than anonymous, closing the gap the user identified.

**To confirm:** the two-reading resolution (mint infers its accessor and
publishes it; consumer names it with `ref(c, ?at)`), and the `Locate` →
`Ref` group rename. Both are the user's call.

## Part 7j: the two defaulting rules — mint-self and consumer-same-`at` (user direction, 2026-10-08)

**Status: settled with the user, this session; both are the `?cmp`
mechanism (`[cmp-carry]`) applied to `?at`, and both are sound.**

### Rule 1 (mint): `-> ref(c)` is self, `-> ref(c, ?at)` is an explicit override

- `fn at(…) -> ref(c)` records **this function** as the handle's accessor
  (inferred self) — the common case, nothing written.
- `fn f(…) -> ref(c, ?at)` is *permitted*, and then `?at` means **some
  `?at` in scope that is not `f` itself** — a function handing back a handle
  it re-materializes through a *different* accessor than itself (a wrapper
  that borrows via a helper's `at`). Resolved exactly as `sort`'s
  `-> +Sorted<T>(?cmp)` resolves its `?cmp` from scope.
- The asymmetry (a bare `-> ref(c)` self-binds; a written `?at` does not)
  is unusual but accepted — "so is `ref`" (user). It reads cleanly: writing
  the slot means "not me, that one."

### Rule 2 (consumer): omitted `?at` defaults to "the same accessor as every other `ref(c)` here"

- Two bare `ref(c)` parameters are assumed to share **one** accessor (and
  one container), so they may alias — the 99% case, where the compiler is
  doing the `at` call and nobody passes two *different* locators into one
  collection. Nothing is written.
- The rare genuine case — two *different* accessors into one container — is
  named: `a: ref(c, ?at) Mut Entity, b: ref(c, ?at: at2) Mut Entity`. If
  the handles passed actually differ in accessor but both parameters are
  bare `ref(c)`, it is an **ordinary type error** (the accessors don't
  unify, exactly as `cmp@Person` ≠ `by_age` makes two `SortedSet` types
  distinct, `[cmp-carry]`), and the fix is the usual one — alias one so the
  two names denote one handle, or name the second accessor.

This is `[cmp-carry]`/`[implicit-resolve]` verbatim: *"an unwritten slot is
resolved by its name"* already governs `?cmp`; Rule 2 is that rule for
`?at`, with the per-signature default being "the one `at` all the bare
`ref(c)`s share."

### Why Rule 2 is sound: the default errs toward *may-alias*, the safe direction

The one thing a defaulting rule must not do is let the checker wrongly
conclude two handles **cannot** alias (that would permit two live `&mut`
into one storage — corruption). Rule 2 cannot do this:

- Assuming "same `at`" makes two handles into one container **may-alias**.
  Two handles into one container that may alias are handled by the exact
  conservative machinery that already exists — one-at-a-time unless split
  (`salvo_pair_mut`) or declared aliasing — so "may alias" is never
  unsound; it is the *conservative* assumption. Worst case it refuses a
  program that was actually fine (two provably-disjoint accessors), caught
  as a plain type error with the alias/annotate fix.
- The *unsafe* direction — concluding two handles are disjoint when they
  are not — is only ever reached by an **explicit** `?at: at2` claim that
  the two accessors guarantee disjoint storage, which is a deliberate,
  checked statement (and the accessors' disjointness is the author's
  claim, the same trust `NotEq`/`+Q` already rest on), never the default.

So the default is safe by construction: omitting the slot can only
over-restrict, never corrupt. This matches the `Sorted` precedent's own
soundness note — "an `add_sorted` under a different `cmp` than the sort used
inserts at a position that is a lower bound for one ordering and nonsense
for the other" is exactly the hazard the *identity check* prevents, and the
identity check is what Rule 2 keeps for the explicit case while defaulting
the common one.

### Net surface

```
// mint: self, nothing written
fn at<T>(list: List<Mut T>, index: Int) -> ref(list) Mut T? => list, index { … }

// consumer, common case: two handles, one container, inferred shared at
fn attack(a: ref(c) Mut Entity, b: ref(c) Mut Entity) { a.hp -= 1; b.hp -= 2 }

// consumer, rare case: two different accessors into one container, named
fn f(a: ref(c, ?at) Mut Entity, b: ref(c, ?at: at2) Mut Entity) { … }

// mint delegating through another accessor: explicit override
fn borrow_via<T>(g: Grid<T>, p: Pos) -> ref(g, ?at) Mut T? => g, p { return at(g, p) }
```

Both rules confirmed; folded into v1a (vocabulary + defaulting) and v1b
(the aliasing signature). No new mechanism beyond `[cmp-carry]` extended to
`?at`.

## Part 8: implementation plan

**Note (2026-10-08): Part 8 below is the plan for Part 7's element-swap
design. If Part 7b (reg the container) is adopted, Steps 3–4 are replaced
as described in Part 7b's recommendation; Steps 0 (done), 1 (done), 2
(done), 6 (the delimiter) and 7 (backend) survive with `reg_from`/`Placeholder`
references rewritten to the container-gated accessors. The step list is
kept as-is until the user confirms the direction.**

Ordered so each step is independently useful and independently verifiable
(`cargo build` warning-free, `cargo test`, kotlinc/rustc e2e where
applicable), per AGENTS.md's workflow. Steps 1–2 are correctness fixes
that should land regardless of anything else here; steps 3+ build the
committed design incrementally, library-first before touching parser/
checker machinery, so there is a working, testable slice at every point
rather than one large change landing at the end.

**Re-grounded against the current codebase (this session): the Rust
backend has since moved from an AST emitter to IR-based emission
(`ad49db0c`, `05258e67`).** `crates/salvo-core/src/mut_lends.rs`'s
`covered_fns` — Step 1's original target — is now dead code with zero
callers; the real logic lives in `crates/salvo-ir/src/build/decls.rs`'s
`may_alias` (building `IR::MayAlias`) and
`crates/salvo-backend-rust/src/ir_emit/{decls,body}.rs`'s `covered`/
`covered_args`. **Finding 1b was re-verified directly against this
current pipeline** (compiled and run with `rustc`, same wrong output,
`changed-a s`) — the bug and its root cause (one `anchor_done: bool`
instead of per-anchor tracking, in `body.rs`'s `covered_args`) are
unchanged by the migration; only the file path moved. Step 1 below is
corrected accordingly.

### Step 0 — `: Params<self>` on `intrinsic type` declarations

New prerequisite (user decision, this session): intrinsic types should
declare their obligations the same way user types do, rather than
getting unconditional, undeclared support for `cmp`/`hash`/`to_str`.
Sequenced first because `Placeholder<self>` on `Int` (Step 3) presupposes
it, and because it is checker/parser surface independent of everything
else in this plan.

- Parser: accept `: Params<self>, Params<self>, …` after an `intrinsic
  type` declaration's name, reusing the existing grammar for `struct`/
  `type`'s obligation list (`struct Point : Ordered<self> by auto`).
  `by auto` is **not** accepted on an `intrinsic type` — there is no body
  to stamp, since the implementation is always a hand-written
  `intrinsic fn`; a `by auto` on an `intrinsic type` obligation should be
  a parse or check error naming the reason (no auto-generation target).
- Checker: for each declared obligation, verify matching `intrinsic fn`
  declarations already exist for the type, the same way a user struct's
  bare `: Ordered<self>` (without `by auto`) is checked against a
  hand-written `cmp` — reuse that checking path rather than writing a
  parallel one.
- Apply it to `Int`, `Long`, `Float`, `Double`, `Bool`, `Char`, `Byte`,
  `Str` for their existing `Ordered`/`ToStr`/`Hashed` support, confirming
  the declared-and-checked form matches today's actual (unconditional)
  support with no behavior change — this is the regression test for the
  step: every program that compiles today using `Int`'s `cmp`/`hash`/
  `to_str` must still compile identically once the obligation is declared
  rather than implicit.
- Verification: a scalar type importing `core` without ever using
  `cmp`/`hash`/`to_str` should (per `[mod-used-only]`'s existing
  dead-code discipline) not change what gets emitted — confirm this
  holds before and after the declaration is added, since the point of
  this step is to make support *declared*, not to change what is emitted
  for programs that already compile.

### Step 1 — Fix finding 1b: validate `canbe` call-site anchors (Part 2, option A)

Independent of everything else; ships first because it is a confirmed
soundness bug, not a design choice. Corrected to target the current
IR-based pipeline (see the re-grounding note above), not the dead
`mut_lends.rs` code this step originally named.

- `crates/salvo-backend-rust/src/ir_emit/body.rs`'s `covered_args`: the
  **plain** `canbe` form's rendering uses a single `anchor_done: bool`,
  set once and never re-checked against which container a *later*
  covered argument's own anchor names — so a second covered argument
  from a different container than the first is silently discarded,
  because only the first argument's anchor is ever emitted
  (`&mut {anchor}`), and later covered arguments just contribute their
  position, assumed (wrongly) to index into that same first anchor.
  Fix: track the anchor expression *per covered argument* (not one
  shared boolean), and refuse the call — naming both arguments' distinct
  anchors — when two covered, non-anchored-form arguments resolve to
  different containers. This mirrors `decls.rs`'s `covered` function,
  which already renders the **anchored** form's `MayAlias::In` case
  correctly (a real, named path, not a shared boolean); the plain
  `MayAlias::Params` case is the one needing the fix.
- New test: the exact repro in Part 1b (`list1`/`list2`, two `Mut Item`
  handles, `canbe` with no anchor) must now be a **compile error**, not a
  silent miscompile. Add it to the Rust backend's codegen test corpus
  (`crates/salvo-backend-rust/tests/codegen_tests.rs` and a snapshot) so
  a regression here fails loudly, and re-run the exact repro through
  `rustc`/`kotlinc`+`java` by hand once the fix lands, the same way every
  other finding in this document was verified empirically, not just
  asserted from a diagnostic string.
- Verification: existing `canbe`/`examples/borrowing/` suite (same-
  container cases, and the anchored `canbe in` form) must still pass
  unchanged — this fix narrows acceptance only for the previously-
  miscompiling case, nothing else.

### Step 2 — Document finding 1a as a known restriction (no code change yet)

Map-chaining (`get(get(m, k)!, i)!`) stays refused under `[rs-elem-mut]`'s
v1 cut until Step 6 gives maps a path into the region/`Reg` story, or
until Part 2's option B (map locator face) is picked up independently.
Add the repro from Part 1a to the diagnostics test corpus as a *confirmed-
refusal* test (asserting the existing error message, so a future change
that silently starts accepting or silently miscompiling it is caught).

### Step 3 — `params Placeholder<T>` in std, built on Step 0

Smallest piece of the committed design otherwise, and fully testable in
isolation. The built-in-vs-user-written question this step originally
flagged is resolved by Step 0's design: there is no special-cased
built-in list at all — a scalar type gets `Placeholder` support exactly
when, and only when, it declares `: Placeholder<self>` and a matching
`intrinsic fn placeholder() -> T` exists, the same declared-and-checked
path every other obligation now takes. Whether std declares `Placeholder`
on `Int`/`Bool`/`Str` out of the box is now an ordinary library-content
question (does std want to ship these instances), not a language-design
one — ship them if convenient, since a program that never imports or
uses them pays nothing either way per `[mod-used-only]`.

- Add `export params Placeholder<T> { fn placeholder() -> T }` to
  `core`, likely `core/basic.sv` or a new small module (check against
  `[mod-used-only]`'s dead-code-emission rule from the existing `Ok`/`Err`
  precedent — a program that never uses `Placeholder` should not emit it).
- If shipping built-in instances for scalars: declare `intrinsic type Int
  : Placeholder<self>` (and similarly for `Bool`, `Str`, etc.) per Step 0,
  with `intrinsic fn placeholder() -> Int => 0` (and so on) — using the
  now-general declared-obligation mechanism, not a separate carve-out.
- Verification: a `params Placeholder<Entity>` instance resolving at a
  call site that needs it; a missing instance producing the ordinary
  missing-implicit diagnostic (no new diagnostic code needed).

### Step 4 — `replace`-based `reg_from`/`unreg_into` as ordinary library functions, *without* `Region` yet

Decouples the swap-mechanics from the effect/qualifier machinery so each
can be tested on its own. Write `reg_from`/`unreg_into` first against a
**stub** signature that does not yet require `[Region]` or produce `Reg T`
— e.g. a plain `swap_in<T>(list, index, ?Placeholder<T>) -> T` /
`swap_out<T>(list, index, value: T) -> None` pair that just does the
`replace` dance with no qualifier involved — to validate the `[col-replace]`
reuse and the `Idx`-preservation claim from Part 7 independently of
`Region`'s existence.

- Verification: a handle/claim into a *different* list index survives
  `swap_in`/`swap_out` on another index — write this as an explicit test
  of the growth-preservation argument from Part 7, mirroring the existing
  `Idx`-survives-`add`/`swap`/`replace` tests in `core.index`'s own
  corpus.

### Step 5 — `effect Region` and the `Reg` provenance qualifier, no delimiter yet

Add `Region`'s two members as an ordinary effect, and `Reg` as an ordinary
provenance qualifier, **before** touching the parser for `region { }`.
This is testable through today's `use`-based effect machinery as a
scaffolding step, even though the final design does not keep `use
Region()` as the real spelling:

- `export effect Region { fn reg<T>(value: T) -> Reg T => !value; fn
  unreg<T>(handle: Reg T) -> T => !handle }`.
- `export provenance qualifier Reg<T> of T` (mint-only, no body — same
  shape as `Ok`/`Err`/`Authenticated`).
- Verification: `Reg T <: T` falls out of `[qual-erasure]` automatically
  — write a test confirming a `Reg Mut Entity` argument is accepted
  wherever a plain `Mut Entity` is expected (field read, field write,
  passing to a function typed for the bare `Mut Entity`), with **no**
  `get`/accessor call needed, confirming the "qualifier, not a replacement
  type" property directly.
- Verification: two `Reg`-tagged handles passed to a function with no
  `canbe` clause (`attack` from Part 7's worked example) must be
  **accepted** without an aliasing proof — this is the one genuinely new
  checker rule in this step (today, two `Mut` parameters with no `canbe`
  default to "one handle at a time"; this step adds the exception "unless
  both carry `Reg` into the same region"). Write both the positive test
  (same region, accepted) and the negative test — **settled (user
  decision, this session): two `Reg` handles from different regions in
  one signature are refused outright, with no exception; every `Reg`
  named in a signature must resolve to the same region.** This is now a
  known-answer test, not an open design check — write it as a refusal
  test alongside the positive one before moving on.

### Step 6 — `region { }` as a compiler-intrinsic delimiter

The parser/checker work, modeled directly on `try { }`'s existing
implementation (`Expr::Try` and its handling across
`crates/salvo-syntax/src/{ast,parser,desugar,visit,visit_mut}.rs`, and
whatever marks it a delimiter in `salvo-core`'s checker) rather than
designed from scratch.

- New `Expr::Region` (or equivalent) AST node, parsed the same way `try`
  is; a block, no parameters, no return-type annotation needed (unlike
  `try`, which evaluates to `Ok T | Thrown M`, `region { }` evaluates to
  whatever its body's tail expression is — check this against `try`'s own
  typing rule rather than assuming).
- **Nesting is permitted; materializing an outer `Reg T` on use inside an
  inner region is automatic, costs one registry read, and leaves the
  outer handle untouched (settled, user decision, this session —
  supersedes an earlier draft's suspend/restore-on-the-outer-binding
  idea, which turned out to be unnecessary).** This is checker work, not
  new vocabulary: a free-variable scan over the inner `region { }`
  block's body (same shape as the existing lambda-capture analysis,
  `[fate-lambda]`) finds every reference to an outer-region `Reg T`
  value, and each such reference's *type* becomes `T` for type-checking
  purposes inside the inner block — materializing via whatever IR node
  Step 0's groundwork settled on (`ExprKind::Widen`, extended past
  `[op-promote]`'s numeric-only scope, or a sibling node — decide which
  before writing this). No poisoning of the outer binding, no implicit
  `unreg`: the outer `Reg T` name remains valid and unchanged, both
  during and after the inner block, because reading through it never
  removes anything from the outer registry. Joining the *inner*
  region's own registry (so a value can alias with other inner-region
  values) is a **separate, explicit, visible operation** — `t.reg()` or
  equivalent call syntax — composing the same materialize step with an
  ordinary `reg` call, which `reg`'s own innermost-wins resolution
  already routes to the *inner* region. Test both: a plain read/call-
  boundary use of an outer `Reg T` inside an inner region (materializes,
  no explicit syntax, outer handle still usable after); and an explicit
  `t.reg()` producing an inner-region handle independent of the outer
  one (real move, inner-region membership, outer handle still intact
  and still itself a valid `Reg T` of the outer region afterward).
- `region { }` satisfies `[Region]` for its body, exactly as `try { }`
  satisfies `[Throw<M>]` — reuse the existing effect-satisfaction checker
  logic rather than writing a parallel path.
- Automatic reconciliation at block close (Part 7's last bullet): walk
  everything tagged `Reg` within the block's own registry at the closing
  brace and swap each one back via the `unreg_into`-shaped operation,
  covering early-exit paths (`return`, `throw`, `break` out of an
  enclosing loop) the same way the existing escape-rule flow analysis
  already walks a block's members on every exit path. This is the
  heaviest single piece of new checker work in the whole plan — budget
  it accordingly, and prototype it against a *single* exit path (fall-
  through) before tackling early-exit paths.
- Verification: Part 7's full worked example, compiled and run on both
  backends with `rustc`/`kotlinc`+`java`, exactly as every other finding
  in this document was verified — the two-different-containers `attack`
  call must now produce the correct result on Rust (`changed-a changed-d`-
  equivalent), closing finding 1b not just by validation (Step 1) but by
  the committed design making the previously-broken case *correct* rather
  than merely *refused*.

### Step 7 — Rust backend: the region's runtime representation

Only after Steps 1–6 are checker-complete and tested on the Kotlin side
(where `region { }`, `Reg`, `reg`/`unreg` should erase to approximately
nothing — plain blocks, identity functions, same as `Reg`/Regions'
original erasure story and `[qual-erasure]` generally).

- Hand-roll the group/slot storage in `runtime/` (Part 3's `Group<T>`/
  `Handle` shape), following the existing precedent of
  `salvo_pair_mut`/`runtime/seq.rs` and the staged-but-unbuilt
  `[rs-region-arena]` — reuse design intent from both rather than
  starting fresh.
- Decide generational staleness checking in or out for v1 (Part 3's cost
  #2) — recommend **out** for the first working version (plain `Vec<Option<T>>`,
  no generation counter), since the compile-time checker proof is already
  the thing making this sound; add generational detection later as a
  pure runtime hardening pass if real usage shows it's worth the cost,
  rather than building it into the first cut.
- Verification: Part 7's worked example's generated Rust, inspected by
  hand the way every other backend finding in this document was (read the
  actual emitted code, don't just trust the description) — confirm
  `attack`'s two `Reg Mut Entity` parameters render as handles into the
  one region's storage with no bound `&mut` spanning more than one
  statement, matching the virtual-place discipline every other mutable
  handle in the language already follows.

### Sequencing notes

- **Step 0 is the one piece of new parser/checker surface before Step 6**:
  `: Params<self>` on `intrinsic type` is small and self-contained, but
  it is real new syntax acceptance and a new checking path, not a pure
  library addition — budget it as such, and land it (and its regression
  test confirming `Int`/etc.'s existing support is unchanged) before
  Step 3 depends on it.
- Steps 1–2 have no dependency on anything else (including Step 0) and
  should land immediately.
- Steps 3–4 (library-only, Step 3 depending on Step 0) can be built and
  tested with **zero further** parser/checker changes, which is
  deliberate: they validate the `Placeholder`/`replace`-reuse parts of
  the design cheaply before the remaining compiler-internals work starts.
- Step 5 (effect + qualifier) is also parser-free — `effect`/`provenance
  qualifier` are existing declaration forms — and is the right place to
  settle the "two `Reg` handles, no `canbe`, accepted" checker rule in
  isolation, before the delimiter adds nesting complexity on top.
- Step 6 (the delimiter) is the one step that genuinely needs new parser
  and new checker machinery, and should not start until Step 5's core
  aliasing rule is proven out, since getting that rule wrong is cheaper to
  fix without a delimiter and nesting rule layered on top of it.
- Step 7 (Rust codegen) is backend-only and should not start until Steps
  1–6 are solid on the checker/Kotlin side, consistent with how every
  other feature in this codebase is built (checker and Kotlin first, since
  Kotlin's erasure makes mistakes cheap to see and fix; Rust's real
  lifetime/borrow machinery last, where mistakes are expensive to unwind).
- Before starting implementation, update `ROADMAP.md` item 15 ("Regions
  (designed 2026-09-10, unbuilt)") to point here instead of describing the
  old freeze/escape design — this document's Part 7 is a replacement, not
  an addition, and leaving item 15's old description in place would leave
  two conflicting designs on record, which `AGENTS.md`'s own documentation
  rules say to avoid ("do not leave an item in both").
