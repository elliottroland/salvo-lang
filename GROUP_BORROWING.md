# Group borrowing: findings and alternatives

Status: **research note, not a decision document.** It records two verified
defects in the current `canbe`/virtual-place implementation, traces them to
a shared root cause, and surveys alternative designs — including Nick
Smith's "group borrowing" proposal (via Kelsey Hightower... no — via Evan
Ovadia's write-up) — that could replace or extend it. Nothing here is
committed; it exists so the two live findings aren't lost, and so the next
design pass starts from evidence instead of from scratch.

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

## Summary table

| Option | Fixes 1a (maps) | Fixes 1b (soundness) | New mechanism? | Implementation risk |
|---|---|---|---|---|
| A — validate `canbe` call sites | No | Yes | No | Low, should happen regardless |
| B — map locator face | Yes | No (needs A too) | No (extends virtual places) | Low–medium; map re-lookup may not be free |
| C — reify `Place` as source of truth | Yes (future-proofs it) | Yes | Small (uses existing `Place`) | Medium |
| Group borrowing | Yes (by construction) | Yes (by construction) | Yes, large | High; unimplemented upstream, open composability question (mut child of immut parent), no `noalias` story |
| D — do nothing beyond A | No | Yes | No | None |

Recommendation for discussion, not a decision: ship (A) immediately since it
is a correctness fix with no design risk; evaluate (C) as the next step
because it fixes both findings using machinery the checker already has,
before considering whether full group borrowing's finer invalidation rule
and zero-annotation aliasing are worth importing an unproven external model
for.
