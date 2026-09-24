# Group borrowing — the option space (working document)

**Status:** OPEN overall — whether groups join the language at all waits on
GB-5/GB-7. **The GB-1 spelling round is DECIDED** (user decisions
2026-09-24: the `canbe` entries — see GB-1's decided subsection); every
example below is written in that spelling.

Written 2026-09-24 at the user's request, with no phase attached: an
exploration of *group borrowing* — Nick Smith's mutable-aliasing borrow model,
as explained by Evan Ovadia at
<https://verdagon.dev/blog/group-borrowing> (Nick's original proposal:
<https://gist.github.com/nmsmith/cdaa94aa74e8e0611221e65db8e41f7b>) — asking
how it might be implemented in Salvo on the existing machinery: the shared-fate
analysis and the `proj` qualifier. It lays out the design as options,
trade-offs and recommendations, **and the calls are the user's** (AGENTS.md's
first invariant). The document is analysis; it does not decide.

**Propagation owed:** none of this document's *content* has propagated to
the specs — no rule labels exist for anything below. Since 2026-09-24 the
document itself is on the record: ROADMAP.md's "Decisions waiting on the
user" table lists GB-1…GB-7 as the next calls (sequenced before the
LSP-root/manifest work), and COMPLETED.md's log notes the document was
opened. The related open items (`Cell`, Regions,
mutating-through-a-union-arm) are untouched. When the user takes the calls,
the outcomes go to COMPLETED.md's decision log and the specs, and **this
document is deleted** — the DESIGN_DOC.md life cycle.

**Sources:** the blog post and gist above; ROADMAP.md's "Shared mutable state
(`Cell`)", "Regions — designed", "Mutating through a union arm (DECISION)",
"Projections and copies — leftovers" and "Recursive types" sections;
LANGUAGE_SPEC.md's Shared fate ([fate-link] [fate-poison]
[fate-field-disjoint] [fate-partial-move] [fate-move-mode]
[fate-derived-readonly] [fate-lambda]), Deductions ([deduce-syntax]
[deduce-infer] [deduce-consume] [deduce-same-call] [deduce-reapply]),
projection ([proj-type] [proj-readonly] [proj-field] [proj-infer]
[proj-anywhere] [readonly-return] [copy-opt-in] [copy-scalar-free]) and
dependent-qualifier ([qual-depend] [qual-preserve] [col-idx]) rules;
BACKEND_SPEC.rust.md's [rs-borrows] [rs-proj] [rs-borrow-locals]
[rs-narrow-mut] and its preamble ("rustc is the safety net"); std's
`core/list.sv`. Every cited label was verified to exist. Web research: the
Rust view-types experiment (rust-lang.github.io/goals/2026), the
partial-borrows IRLO threads, and Mojo's origins documentation.

## §0. The stated intent

The user's ask, numbered so the sections below can be judged against it:

1. **Explore group borrowing** as a notion: what it is, what it buys, what it
   costs.
2. **Assess how it could be implemented in Salvo on the existing machinery** —
   specifically the fate analysis (shared fate: links, poison, places) and the
   `proj()` qualifier.
3. **Include hypothetical Salvo examples.**
4. **Talk through the Rust-emission implications** in particular; flag Kotlin
   implications where they exist.

The intent is exploratory, not a commitment to build: the document is used to
map the option space and name the decisions, not to schedule work.

Where the points meet the recorded direction: (2) lands squarely on machinery
that exists and is stable (fate analysis S1–S3 + L5, `proj` phase 2b) — but
the *mutable* half of group borrowing collides with two standing rules
([proj-readonly]: every projection is read-only; [fate-poison]: mutating a
root poisons every overlapping derivation), and with three open ROADMAP items
that circle the same territory: the `Cell` **DECISION** (shared mutation
through any handle), the Regions design (duplicable handles inside a scope —
Nick's proposal even *calls* its groups "regions"), and the
mutating-through-a-union-arm **DECISION** (what a nested `Mut` means for the
handle that carries it). Those collisions are the reason the decision sections
exist. (4) collides with the Rust backend's founding posture — deductions are
the ownership contract, rendered as `&`/`&mut` [rs-borrows], with rustc
re-proving everything — because group borrowing's whole point is to accept
programs that model rejects. That collision is GB-5, and it is load-bearing.

## §1. Fixed points — already decided, inherited here

What the exploration builds on and does not reopen.

- **Salvo source has no references** (README, LANGUAGE.md): no pointers,
  no borrows, no lifetimes. The one place a borrow is *stated* is a deduction
  clause's projection entry ([deduce-syntax], [readonly-return]); everything
  else is inferred. Any group-borrowing surface must keep this — groups would
  be the second thing after `proj` that names a borrow relationship, not an
  introduction of references.
- **The fate analysis is already invalidation-based, not
  aliasing-xor-mutability.** This is the central observation of the whole
  document. Rust's model says "a mutable borrow excludes all others"; Salvo's
  says "make as many derived views as you like; a mutation event *poisons*
  the ones it overlaps" ([fate-link], [fate-poison], with place-granular
  overlap [fate-field-disjoint] and partial moves [fate-partial-move]).
  That is exactly the shape of Nick's rule — "when you might have mutated an
  object, invalidate references into its contents" — applied to *derived
  variables* instead of references. On the read side, Salvo already **is** a
  group-borrowing-style system. What it lacks is the mutable side: aliasing
  `Mut` handles, and the child-group refinement of what a mutation
  invalidates.
- **Every projection is read-only, whatever its `Mut` says**
  ([proj-readonly], user decision 2026-09-11). There is no mutable projection
  anywhere in the language: std's `get` returns `(proj(list) T)?`, and
  mutation of contents goes through container operations (`set`, `swap`,
  `add`, `remove_at`). `copy` is the escape. Group borrowing's element
  handles (`ref[r] a: Entity`, mutable) have no Salvo counterpart today —
  GB-2's question.
- **A mutation poisons prefix-overlap both ways** ([fate-poison],
  [fate-field-disjoint]): mutating `d` poisons a value derived from
  `d.rings`, and mutating `d.rings` poisons one derived from `d`. A
  **computed index may-aliases every element** — `xs[i]` and `xs[j]` are not
  distinguished. Group borrowing's child-group rule is a *relaxation* of the
  first fact (GB-3) and a workaround for the second (aliasing is permitted
  instead of disproven).
- **Copy scalars are free** ([copy-scalar-free]): `let hp = d.hp` copies the
  number; no link, no poison, no `copy` owed. One of the blog's four
  motivating reads (`hp_ref` surviving `d.damage()`) is therefore **already
  free in Salvo** — worth keeping in view when weighing what the feature
  buys.
- **A copy never happens without the program opting in** ([copy-opt-in]) and
  **a backend never emits silently wrong code** ([backend-never-wrong]).
  Both bind any lowering GB-5 proposes.
- **The Rust backend's contract**: deductions decide moved/borrowed, `Mut`
  decides `&mut` [rs-borrows]; `proj` *is* `&` [rs-proj]; borrow-mode locals
  emit real borrows because Salvo's legality happens to align with NLL
  [rs-borrow-locals]; and the preamble's posture — *"rustc is the safety net:
  generated code that violates these rules fails to compile, never silently
  misbehaves."* Group borrowing accepts programs `&`/`&mut` cannot express,
  so under this posture the lowering must change *representation*, not
  discipline (GB-5).
- **Backend parity rests on rejection today**: clone-vs-alias differences
  between Kotlin and Rust are unobservable *because any program that could
  observe them is rejected* ([fate-poison]'s uniform-discipline bullet).
  Group borrowing deliberately legalizes observable aliasing — two handles,
  one object, mutation through one seen through the other — so parity would
  have to rest on both backends implementing the *same aliasing semantics*
  instead (GB-5, GB-6).
- **Dependent qualifiers carry places** ([qual-depend], built 2026-09-24):
  `qualifier Idx<T>(list: List<T>) of Int`, used as
  `get(list, index: Idx(list) Int) -> proj(list) T` and
  `swap(list, i: Idx(list) Int, j: Idx(list) Int)` [col-idx]. Two facts
  matter here: the machinery for a *qualifier parameterized by a sibling
  parameter's place* exists and is fresh — the natural host for a group
  binder (GB-1) — and std's current answer to "touch two elements at once"
  is already the *central-collection pattern the blog derides*, made total:
  the container plus proven indices, `swap` as one operation.
- **Within one call, an argument may not mention what an earlier argument
  mutates or consumes** ([deduce-same-call]). `attack(nth(es, i), nth(es, j))`
  with two mutating positions is refused today at the second argument. Any
  group-call surface needs a deliberate carve-out here (GB-4).
- **Structs are trees.** Salvo values cannot hold references to each other
  (no reference types; recursive types are an unscheduled ROADMAP item; the
  exception is a `proj` field, which makes the struct a *view*
  [proj-field]). Nick's **isolation** requirement — group members may not own
  or point into one another — is therefore nearly automatic in Salvo, with
  views as the one case to rule on (GB-4).
- **Three open items own adjacent ground, and this document must not spend
  their calls**: the `Cell` **DECISION** (ROADMAP: mutation through any
  handle, no invalidation, "no state qualifiers on contents"); the Regions
  design (user decisions 2026-09-10: `Reg` values with freely duplicable
  handles, **frozen** — the read-only dual of a group); and
  mutating-through-a-union-arm (**DECISION**, recommendation (b): admit
  `=> o: Mut` for a `Mut` reachable through an arm). GB-7 maps the overlap;
  the recommendation everywhere is to decide *against* these neighbours, not
  beside them.

## §2. What other languages teach

### Nick Smith's proposal (Mojo lineage) — the source model

The model this document explores, in five parts:

1. **References to an object vs. references into its contents.** Any number
   of mutable references to an *object* may coexist; what a mutation
   invalidates is references into the object's **contents** — and only the
   contents that could have been *destroyed* by it.
2. **Child groups.** The precise boundary: anything that can be independently
   destroyed — an element of a resizable collection, the payload of a tagged
   union, the target of an owning pointer — is in a *child group*. Mutating a
   group invalidates references into its child groups, and nothing else. An
   inline field (`hp: Int`) or the collection itself (`rings`, as a whole)
   is in the parent group and survives.
3. **Groups come from local variables**, and can be unioned: `attack(a, d)`
   where both arguments come from one list — or from two separate locals —
   passes one group containing both.
4. **Group annotations on functions** (`fn attack[mut r: group Entity](ref[r]
   a, ref[r] d)`) tell the caller two things: the arguments may alias, and a
   `mut` group means "contents of everything in this group may have been
   destroyed — invalidate accordingly". **Paths** (`mut rr: group Ring =
   e.rings*`) narrow that to "only `.rings` elements changed", so an
   unrelated derived reference survives the call.
5. **Isolation**: members of a group must not own or reference each other —
   otherwise mutating one could destroy another and invalidation could not be
   tracked. This pushes data toward trees, which Salvo already enforces.

There are **no unique references** in the model; where uniqueness is needed
(swap two maybe-aliasing references), a dynamic `distinct(x, y)` check splits
the group. The gist states the reframing crisply: the aliasing-XOR-mutability
restriction is lifted from *references* and re-imposed on *groups*
("lifetime parameters" in the original).

What it informs: GB-1 (annotations and paths → deduction-clause entries),
GB-2 (mutable references to members → mutable projections), GB-3 (child-group
invalidation → a refinement of [fate-poison]), GB-4 (isolation → views rule).

### Rust — the model Salvo emits into

- **AxM is the law of the target.** Two live `&mut` into one value — or one
  `&mut` beside a live `&` — do not compile, whatever Salvo proved. Group
  borrowing's flagship program (`attack(&mut es[i], &mut es[j])`, `i` maybe
  equal to `j`) is E0499 by construction. So the question for GB-5 is never
  "how do we convince rustc" but "what representation other than `&mut` do
  group members take".
- **What Rust programmers do instead** is the *central collection* pattern:
  a `SlotMap`/`Vec` plus keys, `get_mut` one at a time, `split_at_mut` for
  provably disjoint halves. Bounds/existence checks and repeated lookups are
  the price. Salvo's `swap(list, i: Idx(list) Int, j: Idx(list) Int)`
  [col-idx] is this pattern with the checks discharged statically — the
  strongest evidence that Salvo can get much of group borrowing's *value*
  without its *model* (GB-2 option C).
- **Partial borrows / view types** (the 2026 view-types experiment;
  the IRLO "Notes on partial borrows" and "Generalized partial borrows"
  threads): Rust's own movement toward field-granular borrows across
  function calls. Relevant twice — it is the target catching up to what
  [fate-field-disjoint] already does at Salvo level, and it does *nothing*
  for element-granular aliasing, which stays out of reach of safe references.
  So even future-Rust does not make GB-5's problem go away.
- **`noalias`**: Rust references carry aliasing guarantees LLVM optimizes on.
  Group members, being aliasable, cannot — the blog is candid that the model
  may be slower where uniqueness would have enabled optimization. An
  index-based lowering (GB-5 option A) sidesteps the soundness question and
  accepts the same optimization loss.

### Vale — regions, and the naming hazard

Vale blends generational references with **region borrowing**: freeze a
region, read it check-free, mutation kept out by the type system. Salvo's
designed Regions feature (ROADMAP, user decisions 2026-09-10) took the
scope-lifetime half: `Reg` values, freely duplicable handles **after a
freeze**, no shared-fate links, state qualifiers permanent — i.e. a region is
Salvo's *immutable* many-handles story. Group borrowing is the *mutable*
many-handles story. They are duals, and Nick's proposal originally called its
groups "regions" (Evan renamed them precisely because "regions" misleads).
Two lessons: keep the names apart (Salvo already owns "region"; "group" is
free), and check whether frozen-`Reg` plus dependent-index totality already
covers the use cases before adding a third sharing axis (GB-7).

### Swift — exclusivity per access, classes alias

Swift's "law of exclusivity" is AxM enforced per *access* rather than per
reference lifetime, falling back to dynamic checks where static analysis
fails — a reminder that there is a spectrum between "prove it" and "check it
at runtime". Salvo's `Cell` sketch explicitly refuses the runtime-check end
("no panic is representable"); any group design should state the same
position or consciously move it (GB-5 option C exists to be rejected on the
record). Swift *classes* meanwhile alias freely with no invalidation at all —
the JVM/Kotlin situation — which is what makes GB-6 a parity question rather
than a Kotlin-feasibility question.

### What Kotlin and Rust do (the backends)

- **Kotlin**: objects are references; aliasing is free, invisible and
  uncheckable. Mutation through one handle is always seen through every
  other. A group-borrowed program lowers to *nothing* — the semantics are the
  JVM's native ones. The risks run the other way: any place the Kotlin
  emitter **copies** (struct spread's `.copy()` [struct-spread], any
  defensive copy) would break aliasing where Rust's lowering preserves it,
  inverting today's divergence direction. GB-6.
- **Rust**: everything in §1's fixed points. One addition: the emitter
  already has one precedent for "the checker proved it, but `&mut` cannot
  carry it" — the mutating-through-a-union-arm refusal ([rs-narrow-mut]'s
  "one refused shape"), where the honest options were mode-change, deduction
  admission, or checker-level refusal. GB-5 is that dilemma at scale.

## §2b. The running example — today's Salvo vs. a group-borrowed Salvo

The blog's `attack`, written both ways, so the decision sections have
something concrete to point at. First, what the language requires **today**
(this compiles conceptually against the current std surface):

```
struct Entity {
    hp: Int,
    energy: Int
}

// The central-collection shape: the container plus proven indices.
// One Mut handle (the list); elements are touched one operation at a time.
fn attack(entities: Mut List<Entity>,
          attacker: Idx(entities) Int,
          defender: Idx(entities) Int) -> None
=> entities: Mut {
    // Reads first — every projection dies at the first write [fate-poison].
    let a = get(entities, attacker)        // proj(entities) Entity, read-only
    let d = get(entities, defender)
    let damage = attack_power(a) - defense(d)
    let a_cost = attack_cost(a, d)
    let d_cost = defend_cost(d, a)

    // Writes second — and each write rebuilds a whole element, because a
    // projection can be neither mutated [proj-readonly] nor spread without
    // copy ([fate-derived-readonly]: a spread is a move out of a borrow).
    set(entities, attacker, Entity { ...copy(a), energy: a.energy - a_cost })
    let d2 = get(entities, defender)       // d is poisoned by the set above
    set(entities, defender,
        Entity { ...copy(d2), energy: d2.energy - d_cost, hp: d2.hp - damage })
}
```

Note what is already good: `attacker == defender` is *legal and correct*
(self-attack works — the reads happened before the writes), the indices are
total [col-idx], and nothing here can dangle. And what is bad: the
read/write phasing is forced, not chosen; two `copy`s of whole elements paid
to satisfy [proj-readonly]; a re-`get` after the first write; and the update
logic reads as bookkeeping.

The same function under a **hypothetical** group-borrowed Salvo (the
decided `canbe` spelling, mutable projections per GB-2 option A):

```
fn attack(a: Mut Entity, d: Mut Entity) -> None
=> a canbe d {
    // a and d may name the same Entity; both are mutable; reads and writes
    // interleave freely because neither can destroy the other.
    let damage = attack_power(a) - defense(d)
    let a_cost = attack_cost(a, d)
    let d_cost = defend_cost(d, a)
    a.energy = a.energy - a_cost
    d.energy = d.energy - d_cost
    d.hp = d.hp - damage
}

fn skirmish(entities: Mut List<Entity>,
            i: Idx(entities) Int, j: Idx(entities) Int) -> None
=> entities: Mut {
    // nth would be a new std fn: a *mutable* projection of one element,
    // legal only into a group position (GB-2).
    attack(nth(entities, i), nth(entities, j))
}
```

And the child-group example (GB-3), with a nested collection:

```
struct Entity {
    hp: Int,
    energy: Int,
    rings: Mut List<Ring>
}

fn attack(a: Mut Entity, d: Mut Entity) -> None
=> a canbe d {
    let rings = d.rings              // derived from d, path [.rings]
    let ring = get(d.rings, 0)       // derived through an element — child group

    damage(d, 10)                    // mutates d (and maybe a — same alias group)

    let n = size(rings)              // GB-3: legal — the list itself cannot
                                     // have been destroyed, only its contents
    print("${ring}")                 // still an error: element storage may
                                     // be gone — child-group poison stands
}
```

Today *both* post-mutation reads are errors ([fate-poison]: `damage(d, 10)`
poisons everything derived from `d`, prefix both ways). Group borrowing's
refinement legalizes the `size(rings)` read and keeps the `ring` one
poisoned. Whether that refinement is worth what it costs the Rust emission is
GB-3 × GB-5.

## §3. GB-1 — Where a group lives in the surface

**The question.** Nick's model needs two pieces of signature vocabulary: "these
parameters may alias" (the group), and "what this call may have invalidated"
(the `mut` group, narrowed by paths). Folds §0(2) against [deduce-syntax]:
Salvo already has exactly one construct whose job is "what a call does to its
parameters", and one fresh mechanism for qualifiers that name sibling
parameters [qual-depend].

**Option A — a deduction-clause entry** *(the entry shape won; its
`group(…)` head spelling below is superseded by GB-1(s)'s `canbe` — kept
for the argument trail)*. `=> group(a, d)` beside the ordinary
entries — a multi-parameter relational entry, the shape the clause's
`proj` vocabulary had until the opaque lends moved to the return type
(`-> proj(a, b) in (T)`, user decision 2026-09-24):

```
fn attack(a: Mut Entity, d: Mut Entity) -> None
=> group(a, d), a: Mut, d: Mut
```

Paths reuse the projection-entry path vocabulary (`=> .f: proj(a)` precedent):
`=> group(e.rings, ring)` says the ring parameter aliases into `e`'s `.rings`
elements and mutation through it invalidates only there — Nick's
`rr: group Ring = e.rings*`, in Salvo spelling. Inference could fill the
entry the way [deduce-infer] fills everything else: a body that passes two
parameters to a group-taking callee needs the group itself, and the fixpoint
propagates it.

- *For*: the clause is already the ownership contract and already the thing
  the Rust backend reads [rs-borrows]; hover already renders effective
  clauses; no new type syntax; the entry is naturally per-*call* information,
  which is what invalidation at the call site needs.
- *Against*: a group is arguably a fact about the *values* (two handles, one
  group identity), and a clause entry cannot be mentioned by a type — so a
  group cannot be stored, returned, or named by a struct field. That confines
  groups to call chains (parameter to parameter), which is most of the blog's
  examples but not all of Nick's model (groups from local variables).

**Option B — a dependent compiler qualifier.** A lowercase group binder in the
generics list (the reserved link-parameter sketch already makes it parse), the
group as a qualifier over the parameter types:

```
fn attack<g>(a: In(g) Mut Entity, d: In(g) Mut Entity) -> None
```

- *For*: groups become type-level and first-class — a struct field could be
  `In(g)`-typed, a local could open a group, and the machinery is
  [qual-depend]'s, which is built and fresh. The identity story mirrors
  `Heap<T, ?cmp>`: two different groups are two different types and refuse to
  mix.
- *Against*: heavier surface (every parameter annotated, a binder per
  signature — the verbosity the blog apologizes for); a *compiler* qualifier
  with new flow rules, which is the expensive kind (`proj` took a phase
  [proj-type]); and
  it decides more than the feature needs before GB-5 has proven any of it
  emittable.

**Option C (shape-changing) — no surface at all: groups are implicit.** Every
pair of same-typed kept-`Mut` parameters is assumed aliasable; the checker
applies group semantics wherever it would otherwise refuse.

- *For*: zero annotation burden; the blog's callers "don't supply groups
  explicitly" anyway.
- *Against*: silently changes the meaning (and the Rust rendering, GB-5) of
  **every existing signature** — `fn merge(dst: Mut List<Int>, src: Mut
  List<Int>)` today promises rustc two disjoint `&mut`, and would stop. It
  also inverts Salvo's stated-contract culture: [decl-explicit]'s "nothing
  the compiler cannot see is inferred" is about bodies, but the spirit is
  that contracts are visible. Aliasability *must* be opt-in per signature.

**Option D — the `proj` pattern: both, split by role** (user observation
2026-09-24). The A-vs-B dichotomy is not one `proj` itself respects: `proj`
is **already both** — a type qualifier where the *kind of value* is stated
([proj-type]: `proj Str`, `-> proj(list) T`, `items: proj List<T>`), and a
a relational statement elsewhere in the signature ([deduce-syntax]'s
`=> .f: proj(a)` entries and the `-> proj(a, b) in (T)` return annotation —
the 2026-09-24 respelling; [readonly-return]). Groups can split the
same way, because the two halves answer different audiences:

- **The qualifier marks the value kind** — "this handle may be aliased
  within its group" — wherever a type is written. Under the decided
  spelling this half largely **dissolves into `proj`**: a mutable element
  handle already names its anchor (`-> proj(list) Mut T`), and the anchor
  *is* its alias group, so no separate group qualifier is needed on
  returns. What remains type-level is the anchor, and it is what the
  *emitter* keys on: an anchored-mutable position renders store-plus-index
  under GB-5-A, precisely as `proj` renders `&` [rs-proj].
- **The clause entry states the relation** — *which* parameters may
  coincide (`=> a canbe d`), path-anchored (`=> ring canbe in e.rings`).
  This half is what call-site legality (the [deduce-same-call] carve-out)
  and caller-side invalidation key on. It is load-bearing for groups in a
  way it is not for `proj`: two `proj` parameters never need to state they
  share roots, but sharing *is* a group — the relation cannot be inferred
  from the types alone.

The division of labor mirrors `proj`'s exactly: the type says *what kind of
thing arrives*, the clause says *what this declaration relates*. And as with
`proj`, one half can be recovered from the other where obvious — a fn whose
clause groups `a` and `d` has group-qualified `a` and `d` positions without
writing the qualifier, the way [proj-infer] fills sources — so the surface
cost is close to option A's while the type-level identity B wanted exists
from day one.

- *For*: dissolves A's *Against* (a group fact **can** be mentioned by a
  type — stored, returned — when that is wanted) without B's
  every-parameter binder ceremony; follows the strongest precedent the
  language has for exactly this shape of feature; gives the emitter a
  type-level hook, which GB-5-A needs anyway (a call site must know a
  position is group-rendered without resolving the callee's clause twice).
- *Against*: two spellings to keep coherent (the checker must enforce that
  the qualifier and the entry never disagree — one funnel, like the
  `Narrowing` classification [rs-narrow-mut] shares between read and `&mut`
  unwraps); and it is more design than the flagship shapes strictly need,
  so a v1 could still ship entry-only (A) with D as its stated growth path.

### GB-1(s) — the spelling: **DECIDED** (user decisions 2026-09-24)

`canbe` is the entry's verb; the `group(…)` head is rejected (a named
construct where a stated possibility reads better, and — in the path form —
an asymmetry the reader had to be told rather than shown). The decided
grammar, all forms desugaring to binary symmetric relations:

- **The core entry**: `=> a canbe d` — the two parameters may name the same
  object. **Symmetric** (writing both directions would be noise) and
  **non-transitive** (`a canbe b, b canbe c` does not relate `a` and `c` —
  the relation is a graph; the Rust lowering merges connected components
  into one store, so the checker's extra precision costs the backend
  nothing). Exempt from the one-entry-per-parameter rule, on `preserve`'s
  precedent — `=> a canbe d, a: Mut` is two statements about `a`.
- **The path form**: `=> track canbe in lib.tracks` — the parameter may be
  an element of the named container path. The direction is in the sentence,
  which is what the `group(e.rings, ring)` head could not show.
- **The shared-anchor rule**: two parameters `canbe in` the *same* path are
  maybe-elements of one container and therefore may coincide — the mutual
  aliasing **falls out of the shared anchor**, so the container-rooted
  n-way case (the common one) costs one entry per parameter, linear in n.
- **`|` lists on both sides** (user extension 2026-09-24): `|` is Salvo's
  "one of" separator (`A | B` unions), and a `canbe` list is disjunctive —
  where the clause's *space*-separated lists (`list: Sorted Mut`) are
  conjunctive, so spaces would read against their own precedent. On the
  right, a hub: `a canbe b|c` declares a↔b and a↔c (not b↔c — the sentence
  says exactly what the rule means). On the left, plural-subject sugar:
  `a|b|c canbe in es` is the three anchored entries. `canbe in` takes path
  lists the same way (`track canbe in lib.tracks|pool.spares`). Composing
  both sides gives the **anchorless clique** in one entry —
  `a|b|c canbe a|b|c` — with reflexive pairs dropped in desugaring (a value
  trivially aliases itself); rare, and derived from the two orthogonal
  extensions rather than a special form.
- **Diagnostic vocabulary**: the connected component keeps the name —
  "alias group" / "group borrow" — so a diagnostic can say "`a` and `c` are
  in one alias group, anchored at `es`" while the surface never needs the
  word.
- **Rejected n-way shapes**, for the trail: comma lists (the comma is the
  entry separator), clique-reading right lists (the sentence would say less
  than the rule means), chained `canbe` (chain-vs-clique ambiguous under
  non-transitivity), and any subjectless set form (`canbe {a, b, c}` — the
  shape the bare `=> proj(a, b)` entry was retired for that same morning).

**Recommendation (updated by the decided spelling round):** **D as the
destination, A as the v1 subset — both spelled `canbe`.** Ship the clause
entries first (they alone carry `attack` and the path examples); the
type-level half is the `proj` anchor, which already exists — what remains
of "introduce the qualifier" is legalizing `proj Mut` where an anchor or a
`canbe` entry covers it (GB-2-A, restated). C remains the one to avoid:
aliasability must be opt-in per signature. B's binder-explicit group
identities lost their remaining motivation to the anchor simplification.

## §4. GB-2 — Mutable projections, or none

**The question.** The blog's element handles (`ref[r] a: Entity` obtained from
`entities[i]`) are *mutable views into a container*. Salvo has views —
`proj` — but [proj-readonly] makes every one of them read-only, by user
decision. Does group borrowing reopen that, and how far? Folds §0(1)/(2)
against [proj-readonly] and [iter-mut-param].

**Option A — `proj Mut` becomes legal where covered.** Restated under the
decided spelling: the combination [proj-readonly] forbids becomes legal
exactly where a `canbe` entry or a `proj` anchor covers it — no new type
syntax, since the anchor names the alias group. A new std accessor mints
it:

```
export fn nth<T>(list: Mut List<T>, index: Idx(list) Int) [] -> proj(list) Mut T
```

`skirmish` in §2b is the payoff: `attack(nth(es, i), nth(es, j))` with the
two projections aliasing legally. Mutation through such a handle is a
mutation event *on the alias group* — every member's child-group
derivations poison [fate-poison], the anchor and the `canbe` entries being
what carry that to callers.

- *For*: this is the actual feature; without it, group borrowing in Salvo is
  parameter aliasing only, and the motivating `entities[i]`/`entities[j]`
  call cannot be written.
- *Against*: it reverses a one-year-old deliberate decision
  ([proj-readonly]); it creates the first value whose mutation rights depend
  on *where it sits* (group position vs. not), a contextual rule Salvo has
  avoided; and every such handle is exactly what GB-5 cannot render as
  `&mut` — so option A's feasibility is entirely hostage to GB-5's
  representation answer. Element *removal* through the container while
  handles live must also be refused (the handles' storage would go), which
  needs the group entry to distinguish "mutates contents" from "may
  destroy contents" — a distinction today's `Mut` does not draw.

**Option B — whole-value groups only.** Groups apply to parameters whose
arguments are whole locals; no mutable projections exist. `attack(a, d)`
works when `a` and `d` are two locals (Nick's "groups can be unioned" case);
the from-one-list call does not.

- *For*: small; [proj-readonly] stands; GB-5's problem shrinks to "two
  maybe-equal locals", which has cheap lowerings.
- *Against*: two distinct locals **cannot alias in Salvo at all** — no
  references, and consumption forbids binding one value to two names with
  both live. So a whole-value group's aliasing case is *unreachable*, and
  the feature degenerates to "parameters that tolerate being passed the same
  variable twice", which [deduce-same-call] refuses today for *reads paired
  with mutations* only. Honest, but close to nothing.

**Option C (shape-changing) — no handles: grow the total-container-operation
family instead.** Keep the model Salvo already has — the container is the
one `Mut` handle; elements are addressed by proven indices [col-idx] — and
spend the effort on the operations that make §2b's "today" version pleasant:

```
export fn update<T>(list: Mut List<T>, index: Idx(list) Int, f: once (elem: T) -> T) -> None
export fn update2<T>(list: Mut List<T>, i: Idx(list) Int, j: Idx(list) Int,
                     f: once (a: T, b: T) -> (T, T)) -> None
```

`attack` becomes `update2(entities, i, j, (a, d) -> …)` with the callback
owning both elements (moved out, moved back — a real `mem::swap`-style
lowering, no aliasing anywhere; `i == j` handled by the intrinsic passing the
same element twice... which is exactly the case that needs a rule: pass it
copied, or refuse `i == j` dynamically à la `distinct`). Element mutation
in place, multi-element transactions, and even the child-group read
(`size(rings)` across a mutation) can each be an operation with an honest
deduction, and [qual-preserve] lets them keep claims precise.

- *For*: no new semantics, no GB-5 crisis — everything lowers to safe Rust
  today; it continues the exact trajectory std is already on (`swap`,
  `Idx`, `preserve`); the ergonomic gap closes for the common cases.
- *Against*: it is not group borrowing — no user-written function can take
  two aliasing handles; the vocabulary is fixed by std (though `?copy`-style
  implicits and callbacks recover a lot); `update2`'s `i == j` needs its own
  small decision. And the blog's error-message benefit ("this pointer is
  invalid *because of this mutation*") is not gained — though Salvo's poison
  diagnostics already name root and event, so that benefit is largely
  already in hand.

**Option D (shape-changing hybrid) — mutable projections under exclusivity,
plus dependent disjointness claims.** Lift [proj-readonly] but keep the
aliasing ban: a mutable projection is an **exclusive** handle — while it
lives, the root and every overlapping place are untouchable, and minting a
second overlapping one is refused at the mint (a computed index may-aliases
every element [fate-field-disjoint], so per-container that means one).
Simultaneous element handles come back through a **dependent disjointness
claim** on the [qual-depend] machinery, which is the blog's `distinct(x, y)`
escape hatch promoted to the whole model:

```
qualifier Distinct(i: Int) of Int {
    fn qualifies(j: Int, i: Int) -> Bool {
        return j != i
    }
}

fn attack_at(entities: Mut List<Entity>,
             i: Idx(entities) Int,
             j: Distinct(i) Idx(entities) Int) -> None
=> entities: Mut {
    // two exclusive mutable projections, proven apart
    let a = nth(entities, i)
    let d = nth(entities, j)
    …
}
```

This is aliasing-xor-mutability with proofs instead of aliasing with
invalidation: every accepted program is one rustc accepts, so it **defuses
GB-5 entirely** — `proj Mut X` renders `&mut X` (the mutable twin of
[rs-proj]'s rule), and the two-handles case lowers to a safe
`split_at_mut`-style helper keyed by the claim. Parity-by-rejection
[fate-poison] survives untouched, since aliasing is still never observable.

- *For*: in-place element mutation *and* the two-element shapes, with no new
  memory model, no representation fork, no parity inversion (GB-6's audits
  become unnecessary); the claim machinery is built and fresh, and the
  runtime test (`j is Distinct(i)`) is ordinary refinement narrowing.
- *Against*: `i == j` is *refused*, not meaningful — self-attack, self-transfer
  and every aliasing-means-something case are outside the model, and that is
  precisely the property groups exist to provide; every call site owes a
  proof or a branch; exclusive mutable projections are real checker work
  (mint-time exclusivity is today's poison rule inverted) and real emitter
  work (`&mut` projections with lifetime ties, the split helper); and
  `Distinct` is a claim between two *values* used as an aliasing fact about
  two *places* — sound for indices into one container, but the rule that
  keeps it from being claimed across containers needs stating.

### The worked pair, and the layering it exposes (2026-09-24)

`nth` used validly and invalidly, under the decided spelling — and the
split between the two turns out to redraw the C/D/A relationship.

```
fn skirmish(es: Mut List<Entity>, i: Idx(es) Int, j: Idx(es) Int) -> None
=> es: Mut {
    // Two handles, i maybe equal to j: legal because attack declares
    // `a canbe d` — the same-call rule stands down for the covered pair.
    attack(nth(es, i), nth(es, j))

    // One handle into an UNCOVERED Mut position: also legal, no canbe
    // needed — one handle means no aliasing can occur, and the ordinary
    // mutation-through-projection event poisons overlapping derivations
    // of `es`, exactly as `add(es, x)` would.
    // Rust: a statement-scoped `heal(&mut es[j])` — plain `&mut`.
    heal(nth(es, j))

    // A bound handle, mutated in place: legal while it is the only live
    // handle and nothing destroys element storage under it.
    let boss = nth(es, i)
    boss.hp = boss.hp + 5
}
```

```
fn sabotage(es: Mut List<Entity>, i: Idx(es) Int, j: Idx(es) Int,
            out: Mut List<Entity>) -> None
=> es: Mut, out: Mut {
    // (a) Two handles into an UNCOVERED callee: refused at the second
    // argument [deduce-same-call] — swap_hp never said its parameters may
    // alias (and its Rust rendering is two disjoint `&mut`).
    swap_hp(nth(es, i), nth(es, j))

    // (b) A second mint while the first is live, outside any covered
    // call: uncovered handles keep the exclusive discipline, and a
    // computed index may-aliases every element [fate-field-disjoint].
    let v = nth(es, i)
    let w = nth(es, j)      // error: `v` is a live mutable handle into `es`

    // (c) Element destruction under a live handle — note the Idx *claim*
    // survives (`add` preserves Idx [qual-preserve]: an index is position)
    // while the *handle* dies (a handle is storage, and `add` may move it).
    let victim = nth(es, i)
    add(es, Entity { hp: 1, energy: 1 })
    victim.hp = 0           // error: invalidated by the mutation of `es`

    // (d) Storing the handle: a view cannot outlive its anchor, and a
    // store is a move, which a projection refuses [fate-derived-readonly].
    add(out, nth(es, j))    // error: `copy` the element to keep it
}
```

The line between valid-(2) and invalid-(a)/(b) is the layering fact: **an
*uncovered* `proj Mut` handle behaves exactly as option D's exclusive
mutable projection** — one live handle per container, statement-scoped
`&mut` on Rust, the existing poison discipline doing all the work. `canbe`
coverage adds *only* the aliasing relaxation: the same-call exemption at
covered calls, simultaneous handles inside covered callees, and the
store-plus-index rendering for exactly those positions. So **D is not an
alternative to A — D is A's substrate**, and the build sequence is a
ladder: exclusive `proj Mut` first (plain `&mut` emission, no new
representation), the `canbe` relaxation on top (GB-5's store, only where
covered). D's `Distinct(i)` claims become optional rather than structural —
a covered call supersedes them wherever aliasing is tolerable, and they
remain available where a caller wants proven-disjoint handles without
paying the covered rendering.

**Recommendation (updated 2026-09-24, the user's call):** treat C, D and A
as **stages of one feature**, in that order: C's total container operations
for the immediate ergonomics; D-as-substrate (exclusive `proj Mut`, `nth`,
plain `&mut` rendering) as groups v0; A's `canbe` relaxation as the step
that pays GB-5's representation, only for covered positions. Adopting A
while emitting `&mut` remains not an option that exists.

## §5. GB-3 — The invalidation refinement (child groups vs. today's poison)

**The question.** Today, mutating `d` poisons *everything* derived from `d`
(prefix overlap both ways, [fate-poison] [fate-field-disjoint]). Nick's rule
is finer: mutation invalidates only derivations that cross a
**destroyability boundary** — a collection's elements, a union's payload, an
owning indirection — and leaves whole-object and inline-field derivations
standing (§2b's third listing). Adopt the finer rule?

Where the boundary already lives in Salvo terms: element access (`get`,
computed index — the `proj::Element` may-alias case), union-arm narrowing
([flow-place]'s narrowed reads), and nothing else — Salvo has no owning
pointers, and `T?`/unions are the "variant" case. So a link's path could
carry one extra bit per segment — *crosses-contents* — set by element and
arm segments, and poison under group semantics would fire only on overlaps
whose relative path includes such a segment. The machinery is genuinely
small on the checker side: [fate-field-disjoint]'s paths already exist;
this adds a classification, not a new analysis.

**Option A — refine poison only inside groups.** Group-annotated values get
child-group precision; everything else keeps today's rule.

- *For*: opt-in precision where the feature is asked for; no existing
  program changes meaning; the Rust emission only has to handle the new
  liveness pattern for values it is *already* representing specially
  (GB-5's group representation).
- *Against*: two poison rules in one language — hover and diagnostics must
  say which regime a variable is under; teaching cost.

**Option B — refine poison globally.** Every derived variable survives
mutations that could not have destroyed its storage: `let rings = d.rings`
survives `damage(d)` everywhere, group or no group.

- *For*: one rule, and the more precise one; the checker-side change is the
  same size as A.
- *Against*: **it breaks the borrow emission for plain code.** Today a
  borrow-mode local is a real `&` in Rust precisely because Salvo's
  legality aligns with NLL [rs-borrow-locals] — the alignment is
  load-bearing and named as such. `let rings = &d.rings;` held across
  `damage(&mut d)` is E0502: every newly-legal program is a program the
  current lowering cannot compile. So B forces GB-5's representation change
  onto *ordinary* variables, not just group members — the tail wagging the
  whole backend. (Kotlin is untouched — it aliases — but parity then
  demands Rust follow.)
- A note on what B would buy semantically: with `Str` non-Copy and scalars
  already free [copy-scalar-free], the surviving derivations are mostly
  container fields (`d.rings` across `damage(d)`) — real, but narrower than
  the blog's presentation suggests once Copy-freedom is accounted for.

**Option C — no refinement.** Groups permit parameter aliasing and mutable
projections, but any mutation through a group handle poisons all members'
derivations wholesale, today's rule.

- *For*: cheapest sound thing; GB-1/GB-2 still deliver the `attack` shapes
  (which hold no cross-mutation derivations — reread §2b: the group version
  needs no derived variables to survive anything).
- *Against*: the `size(rings)`-after-`damage` read stays refused, and the
  blog's "more programs proven correct" benefit narrows to the aliasing
  half only.

**Recommendation (the user's call):** **C first, A as the follow-on if it
bites.** The flagship examples need aliasing (GB-1/GB-2), not invalidation
precision; C ships them without touching poison at all. A is a clean
increment later. B should be rejected on the record: it converts a checker
refinement into a backend rearchitecture.

## §6. GB-4 — Isolation, claims, and the same-call rule

Housekeeping decisions the model drags in; each small, each needed before
any build. Folds Nick's isolation requirement and the collisions §1 flagged.

- **Views are not group members.** A struct with `proj` fields (a view,
  [proj-field]) held in a group could reference another member — exactly
  what isolation forbids (mutating one member could invalidate another
  *through the view*, which path-overlap on distinct roots cannot see).
  Rule: a group member's type may not be or transitively contain a view;
  refused at the group position, naming the field. Cheap: `rs-proj-struct`'s
  "is a borrowing struct" classification already computes this. Linear
  values should be refused as members too ([linear-static]'s discipline
  assumes one accountable handle; two aliasing handles to an obligation is a
  discharge-twice hazard). Recommendation: both refusals, day one.
- **State qualifiers strip across the group.** A mutation through handle `a`
  can invalidate a `NonEmpty` the caller holds via handle `d` (same object,
  maybe). The exhaustive-form rule ([deduce-syntax]: mutation forces
  exhaustive) already strips the mutated *parameter's* claims; groups need
  the stripping to hit **every member of the group**, and — mirroring
  `Cell`'s "no state qualifiers on cell contents" — the cheaper posture is:
  values may not *carry* state claims while group-aliased; provenance claims
  ride free ([qual-subject]). Dependent claims about a group member
  (`Idx(list)` where `list` joins a group) strip under the same rule
  [qual-depend], with `preserve` [qual-preserve] as the existing opt-back.
  Recommendation: strip-across-group, stated as one rule.
- **The same-call carve-out.** `attack(nth(es, i), nth(es, j))` is refused
  today: the second argument mentions `es`, which the first argument's
  position mutates [deduce-same-call]. When the callee declares
  `a canbe d`, that is not a hazard — aliasing is the point — so the rule
  must exempt arguments landing in one alias group. The exemption must be
  exactly group-shaped: `f(nth(es, i), size(es))` with only the first parameter in
  a group stays refused.
- **Element destruction while handles live.** `add`/`remove_at` on a list
  whose elements a group handle projects must be refused for the handles'
  lifetime — mutation through a group handle may *change* members but never
  destroy them (Nick's footnote 8 territory). Today's poison gives this for
  free (any `Mut` use of the container poisons the projections); if GB-3
  option A later relaxes poison, the crosses-contents bit is what keeps
  destruction poisonous while writes are not. No decision needed beyond
  GB-3; recorded so it is not lost.

## §7. GB-5 — The Rust emission (load-bearing)

**The question.** Group members mutably alias; `&mut` cannot. Under
[backend-never-wrong] and the preamble's "rustc is the safety net", what do
group members *compile to*? Every other section's viability hangs on this
answer — read this one first.

**Option A — handle-and-store lowering.** A group renders as one `&mut`
backing store plus small copyable handles (indices); every member access
re-borrows through the store for exactly one statement:

```rust
// fn attack(a: Mut Entity, d: Mut Entity) => a canbe d
pub fn attack(__g: &mut Vec<Entity>, a: usize, d: usize) {
    let damage = attack_power(&__g[a]) - defense(&__g[d]);   // two &, fine
    let a_cost = attack_cost(&__g[a], &__g[d]);
    let d_cost = defend_cost(&__g[d], &__g[a]);
    __g[a].energy -= a_cost;                                  // one &mut at a time
    __g[d].energy -= d_cost;
    __g[d].hp -= damage;
}
```

This is the central-collection workaround, generated — the source stays the
nice version, the output is the pattern Rust programmers write by hand.
Aliasing semantics are exact (`a == d` behaves identically to Kotlin's
aliasing, because there is one storage). Costs and consequences:

- **The blast radius shrank (2026-09-24, the GB-2 layering):** only
  **covered** positions pay this representation. An uncovered handle keeps
  the exclusive discipline and renders as a plain statement-scoped or
  bound `&mut` — so `nth`, `heal(nth(es, j))` and the single-handle
  read-modify-write shapes, the common cases, never touch the store; the
  store appears exactly where a `canbe` entry does.
- **Bounds checks** per access. Mitigable: the checker minted the indices
  from real element positions, so `get_unchecked` would be sound — but
  that is `unsafe`, so v1 pays the checks (they are the same checks the
  hand-written pattern pays; [col-idx]'s claims could later justify a
  narrow, audited `unsafe` splice the way std intrinsics are already
  trusted templates).
- **The group needs a store.** Members minted from one container (GB-2's
  `nth`) have one naturally. *Two locals* joining a group do not — the
  lowering must move them into a synthetic `Vec`/array at group formation
  and move them back after (or, since GB-2 option B showed two locals
  cannot alias anyway, simply keep two `&mut` for the provably-disjoint
  case: the group annotation permits aliasing, it does not require
  emitting for it when the checker can see the arguments are distinct
  places — a per-call-site specialization the fusion machinery has
  precedent for).
- **Signature bifurcation.** A group parameter is `(store, index)` — two
  Rust parameters per Salvo one, with the store shared. Mechanical, but it
  is the largest signature-shape change since effect fusion, and it
  interacts with everything that renders parameters (adapters, implicits,
  effect members, [fn-contract] conventions). Group-taking fn *values* and
  effect members are the hard corners; a v1 should refuse them
  ([backend-never-wrong]-style cuts) rather than solve them.
- **Derived variables under GB-3-A** become path handles (`(d, [.rings])`)
  re-materialized per read — every read pays index-plus-field-walk where a
  plain `&` was free. Confined to group members; another reason GB-3
  recommends C first.
- **Nested groups / paths** (`power_up_ring(e, ring)` with
  `ring canbe in e.rings`) need store *paths*, not just indices — staged
  cut for v1.
- rustc remains the safety net in the meaningful sense: the emitted code is
  safe Rust; a checker bug yields a wrong answer or a bounds panic, never
  UB.

**Option B — raw pointers, Salvo as the safety argument.** Emit `*mut T`
(or `&UnsafeCell<T>`) for group members; zero-cost, `noalias` deliberately
forfeited; soundness rests on the Salvo checker alone.

- *For*: the blog's actual promise — zero-overhead mutable aliasing; no
  signature bifurcation (a member is one pointer).
- *Against*: it abandons the backend's founding posture. Today a checker
  bug is a rustc compile error ([rs-borrows]'s closing line: "a program
  that emits but does not borrow-check is a compiler bug, not a user
  error"); under B a checker bug in exactly the newest, subtlest analysis
  is silent UB in user output. The one shipped [backend-never-wrong]
  violation (the narrowed-`Mut` clone, COMPLETED.md) was caught *because*
  the output was safe and wrong, not unsound. B should be rejected on the
  record unless a formally-argued checker core exists someday.

**Option C — `RefCell`/runtime borrow flags.** Rejected on the `Cell`
sketch's own reasoning ("no panic is representable" is that design's spine):
a reentrancy panic in generated code is [backend-never-wrong] with a delay.
Listed to be declined explicitly.

**Option D (shape-changing) — don't take the fork: GB-2-C (or GB-2-D)
instead.** No group representation exists because no aliasing handles
exist; the total container operations (`update`, `update2`, `swap`) lower
to safe Rust with `mem::take`/`mem::swap`-style bodies the intrinsic table
already knows how to ship — and GB-2-D's exclusive mutable projections
stay `&mut` with a `split_at_mut` helper for the proven-disjoint pair. This is the "spend the budget on ergonomics, not on a memory
model" answer.

**Recommendation (the user's call):** **D now; A if and when group borrowing
proper is wanted, scoped to parameters-only v1 (GB-1-A + GB-2-A's `nth`,
GB-3-C), with the fn-value/effect-member corners cut loudly.** B and C
should be recorded as rejected with reasons, so the question does not reopen
silently.

## §8. GB-6 — Kotlin implications (flags, not decisions)

Kotlin gets aliasing for free, which is exactly why it needs watching:

- **Parity inverts.** Today's divergences are "Rust clones where Kotlin
  aliases", made unobservable by rejection [fate-poison]. Group borrowing
  legalizes observation, so the burden flips: every **copy** the Kotlin
  emitter makes of a group member is now the wrong side. Known sites to
  audit: struct spread's shallow `.copy()` [struct-spread], and any
  defensive copy in intrinsic lowerings. A group member must stay one JVM
  object through its whole life.
- **Scalars in groups.** A group of `Int`s is incoherent on the JVM
  (boxed identity vs. value) and pointless anyway ([copy-scalar-free]);
  refuse Copy-scalar group members in the checker so neither backend
  meets them.
- **`update2`'s moved-out elements (GB-2-C)** are the reverse trap: Kotlin
  has no moves, so "taken out, transformed, put back" must not leave the
  JVM list observing intermediate states through other references — a
  non-issue while the container is the one `Mut` handle (the checker
  guarantees no other observer), which is another argument for C's shape.
- **Identity equality.** Salvo has no `===`; `eq` is a declared capability
  ([cmp-groups]). Groups introduce the first situation where "same object?"
  is program-observable (mutate through `a`, read through `d`) without
  being *askable*. Nick's `distinct(x, y)` needs an answer eventually
  (GB-2-C's `update2` hides it inside std; GB-2-A eventually wants it in
  the language). Flagged, not designed.

## §9. GB-7 — One sharing story, or three

**The question.** Salvo now has three overlapping answers to "more than one
handle" on the table: **`Cell`** (ROADMAP **DECISION**: mutation through any
handle, no invalidation, no state claims), **Regions** (designed: duplicable
handles after a freeze — immutable sharing), and now **groups** (aliasing
`Mut` handles with static invalidation). They partition cleanly by
mutability × discipline:

| | invalidation-checked | unchecked |
|---|---|---|
| **shared immutable** | today's fate links | frozen `Reg` (Regions) |
| **shared mutable** | **groups (this doc)** | `Cell` |

### `proj` and groups — two points on one dial

A question worth settling before the options (raised 2026-09-24): if groups
landed, could `proj` be **removed**? No — but the two are points on one
spectrum, and that reframing sharpens this whole section. The honest
restatement: **`proj` is the degenerate group** — a read-only member of a
singleton group whose root retains exclusive write, under the maximally
sensitive invalidation rule (today's prefix-both-ways poison
[fate-poison]). Groups relax each dial: read-write members, many members,
destruction-boundary invalidation (GB-3). What survives of `proj`
regardless:

- **The zero-cost tier.** `proj X` *is* `&X` [rs-proj]: no indirection,
  elided lifetimes, `noalias`. A group handle under GB-5-A is
  store-plus-index — every read pays a lookup. Re-founding `get`, `first`
  and the passes on group handles would re-price all of std's read paths;
  the [copy-opt-in] economy rides on `&`.
- **Read-only as a signature fact.** `-> proj(p) T` promises the caller
  nothing mutates through the result; a group-membership return promises
  the opposite (assume writes through it reach `p`). Different contracts,
  and the weaker one is what most APIs want to state. Even Nick's model
  keeps the polarity — group annotations are `mut` or not, and Mojo's
  origins carry mutability. The source model deletes *unique* references,
  never the read-only tier.
- **Frozen-while-borrowed.** A live projection makes its root untouchable,
  which is what gives iteration its semantics ([yield-proj]) and what the
  claim-minting passes stand on ([col-idx]: "sound because the source
  cannot be mutated while the pass lives"). A group-based `iter` would
  tolerate `set(xs, i, v)` mid-loop — a legitimate but *different*
  iteration semantics, and one that breaks the dependent-claim invariant.

And what the spectrum framing *buys* if groups land:

- **One invalidation engine.** [fate-poison]/[fate-field-disjoint]'s
  place-overlap machinery serves both regimes; GB-3's child-group rule is a
  per-regime sensitivity setting on it, not a second analysis.
- **Views are already group-shaped.** A pass is `items: proj List<T>` plus
  an `at` index — store + index, GB-5-A's representation with the store
  held as `&` because it is read-only. A mutable view struct would be the
  group version of [proj-field].
- **One surface, two polarities.** Under GB-1-D the qualifier half of
  groups and `proj` are siblings; a graded design could make `proj` the
  read-only, exclusive-root, max-sensitivity point of one feature rather
  than a second feature.

The backend is where unification stops: read-only members render `&`,
aliasing-mutable members render store-plus-index — the split is forced by
the target, not by the surface, so the Rust emitter keeps two renderings
whatever the spec calls them. The realistic end states are *both features,
separate* or *one graded feature with `proj` as a point on the dial* —
never groups alone: any groups-only design reinvents the read-only tier
within a week, as the non-`mut` group annotation, priced back down to `&`.

**Option A — treat them as one design space and decide together.** Fold this
document's GB-2/GB-5 outcome into the `Cell` decision session: several of
`Cell`'s motivating cases (two lambdas sharing an accumulator, the
`Mut`-parameter producer) are *scoped* sharing that a group could serve with
static checking instead of a sanctioned hole — and if groups can serve them,
`Cell` shrinks or disappears. Cost: the `Cell` decision, already deferred
past phase 5, waits on this too.

**Option B — keep them separate; this document answers only its own
question.** Groups are call-scoped and checked; `Cell` is store-scoped and
unchecked; different tools. Cost: three sharing features to teach if all
land; the table above becomes the documentation burden.

**Option C (shape-changing) — adopt only GB-2-C and close this question.**
The total-operation family is std surface, not a sharing model; the
2×2 table stays at two filled cells plus `Cell` pending, and group borrowing
is recorded in COMPLETED.md as explored-and-declined with this document's
reasoning (chiefly: GB-5's fork — safe-but-indexed or fast-but-unsafe — prices
the model out for a transpiler whose safety story is the target's compiler).

**Recommendation (the user's call):** **C**, with A's framing recorded: when
the `Cell` decision is eventually taken, this document's 2×2 table and the
group option should be on that table, so the sharing story is decided once,
with all four cells visible.

## Final — decisions pending before implementation (refreshed 2026-09-24)

**Decided so far** (all 2026-09-24): the GB-1 spelling round in full
(GB-1(s): `canbe` entries, symmetric, non-transitive, `canbe in` paths,
`|` lists, shared-anchor rule, "alias group" as diagnostic vocabulary);
GB-1's shape (D's split, qualifier half dissolved into the `proj` anchor);
and the GB-2 layering (D is A's substrate; C/D/A are stages of one
feature). What remains, in the order the calls are needed:

| # | Decision | Where | Recommendation (user's call) |
|---|---|---|---|
| P-1 | **Go/no-go and staging**: build the C → D → A ladder? Is C (the total container-op family) still wanted as stage 1, or start at D? And GB-7's relationship to the `Cell` decision (decide together vs. proceed) | GB-2's ladder, GB-7 | Build the ladder, C included (cheap, useful regardless); proceed without waiting on `Cell`, but put the 2×2 table + the `proj`-groups dial on that future session's table |
| P-2 | **The Rust representation for covered positions**: ratify handle-and-store (safe Rust, bounds checks accepted) and reject raw-pointer and `RefCell` lowerings on the record; two-locals group formation (synthetic store vs. per-site disjoint specialization); v1 cuts (group-taking fn values, effect members, nested store paths — refused loudly) | GB-5 | A, with the cuts; bounds checks accepted for v1, the `Idx`-justified `unsafe` splice recorded as a possible later refinement |
| P-3 | **The uncovered-handle semantics** (D's substrate): lift [proj-type]'s `Mutates` refusal for a *single* anchored handle into a `Mut` position; lift [proj-readonly]'s mutation ban for anchored handles under exclusive discipline; keep the declaration-site `proj Mut` parameter error except under `canbe` coverage | GB-2's worked pair | Yes to all three — this is the v0 that costs no representation change |
| P-4 | **`canbe` inference**: are entries written-only, or may [deduce-infer] claim one (a body forwarding two parameters into a covered callee needs the entry — does the middle fn write it, or does the fixpoint infer it)? | GB-1-A's inference note, [decl-explicit] | Written-only at first (aliasability stays visible in every signature); revisit if forwarding chains make it bite |
| P-5 | **GB-3 invalidation for v1**: confirm C — mutation through a covered handle poisons other members' derivations wholesale; the child-group refinement (A) waits until it bites | GB-3 | C for v1; reject B on the record |
| P-6 | **GB-4 ratifications**: views and linear values refused as group members; state claims strip across the alias group (provenance rides); the same-call exemption exactly coverage-shaped; Copy-scalar members refused (GB-6) | GB-4, GB-6 | Ratify all four as written |
| P-7 | **`Distinct(i)` claims**: ship them (proven-disjoint handles without the covered rendering), or drop — covered calls supersede them wherever aliasing is tolerable | GB-2-D | Drop for now; record as a later refinement of the ladder |
| P-8 | **C's details, if stage 1 ships**: `update2`'s `i == j` rule (same element passed twice, refuse dynamically, or copy) and the family's exact members | GB-2-C | Same element twice, documented — the aliasing-tolerant reading D and A also take |

P-1 governs everything; P-2 and P-3 are the load-bearing semantic/backend
pair; P-4–P-8 can be taken in any order once those stand.

### The P-round outcomes (user decisions 2026-09-24, second sitting)

- **P-2, P-3, P-4 — DECIDED as recommended**: handle-and-store for covered
  positions with the v1 cuts (B and C rejected on the record); the three
  uncovered-handle lifts; `canbe` entries written-only.
- **P-5 — explored, direction set**: GB-3-A's base rule needs **no new
  syntax** (a crosses-destroyability-boundary bit on link path segments,
  group scope from the `canbe` entries; representation-safe since covered
  code already renders path handles). The cross-call design decision is the
  **destroy-vs-write distinction in contracts** (`add` and `set` are both
  `=> list: Mut` today): v1 is an internal std-intrinsic table with user
  fns conservative — no syntax; v2 is one new preservation-family clause
  item (distinct from `preserve Idx`: index-validity and storage-stability
  are different facts) plus inference. Wait for v1's conservatism to bite.
- **P-6 — the linear refusal NARROWS (user challenge, conceded)**: group
  handles never own, so a linear value can be neither discharged nor
  duplicated through one — kept-only membership is safe, and the container
  stays the one accountable place. What remains refused is any *owning*
  position receiving one element twice, which linearity itself enforces
  and only the C family could even attempt. Views stay refused (isolation,
  not linearity).
- **P-7 — resolved via P-8**: `Distinct(i)` is an ordinary dependent
  qualifier on existing [qual-depend] machinery — **std ships it with
  `update2`**, no compiler work. The later refinement is only the
  compiler-side use (group splitting / `split_at_mut`-keyed D-handle
  lowering).
- **P-8 — corrected and detailed**: the owned-pair callback was wrong —
  `i == j` is incoherent for owned moves (one value cannot move out
  twice). The family takes **`Mut` callbacks**: `update(list, i: Idx(list)
  Int, f: (elem: Mut T) -> None)` → `f(&mut list[i])`;
  `update2(list, i: Idx(list) Int, j: Distinct(i) Idx(list) Int,
  f: (a: Mut T, b: Mut T) -> None)` → safe `split_at_mut`. No moves, so no
  linear hazard (P-6) and no `Default` bound; wholesale replacement stays
  `set`, linear discharge stays `remove_*`. v1 family: `update`, `update2`,
  the existing `swap`.
- **P-1 — answered with the ladder-value examples** (each rung the
  cheapest adequate tool: C = one atomic touch, D = one element many steps
  at `&mut` cost, A = two handles that may be one); the go/no-go itself is
  the remaining call.

### The go decision, and the build sequence (user decisions 2026-09-24, third sitting)

- **P-1 — GO.** All three rungs are built, and this document **stays open
  until every rung has landed** — plus GB-3-A (the user's P-5 call: start
  under GB-3-C, design and build the invalidation refinement before the
  document closes).
- **`nth` is retired before it ever existed** — and (user correction
  2026-09-24, probe-verified): it is not even an overload. **Element
  mutability is the element type's, not the container handle's**: the
  generic total `get` instantiated at `T = Mut Entity` *already* answers
  `proj(list) Mut Entity` today (the checker prints exactly that type;
  only [proj-readonly]'s ban — P-3's lift — refuses the mutation). The
  corrected model keeps two capabilities apart: container `Mut` is
  *structural* (`add`/`remove`/`set`/`swap`), element-type `Mut` is
  *in-place* (`List<Mut T>` hands out mutable handles) — a distinction
  Kotlin's representation (`MutableList` nesting) already makes physical.
  Consequences: the doc's examples sweep to `List<Mut Entity>` /
  `Mut List<Mut Entity>`; the C family's `list` parameters carry no
  container `Mut` at all (`update<T>(list: List<Mut T>, i: Idx(list) Int,
  f: (e: Mut T) -> None)`); Rust needs one mode extension (a kept
  `List<Mut T>` parameter renders `&mut Vec<T>` — depth-reachable lendable
  mutability, [rs-borrows] + the `type_has_mut_arm` precedent); the
  mutation-event definition gains handle-sourced events at the element
  path [fate-poison]. **P-9 collapses to one binary, still open**: a
  handle from a `Mut`-typed element is *exclusive from the mint*, or
  *mode-inferred* ([fate-move-mode]'s S2 pattern — read-mode until a
  downstream use mutates; recommended, or two read handles for a
  `cmp(a, b)` over `List<Mut Entity>` stop compiling).
- **P-6 — DECIDED as narrowed**: kept linear values are legal group
  members.
- **P-8 — DECIDED, with a resequencing**: the C family is **ordinary
  Salvo**, not intrinsics — which is possible exactly because, after
  P-3's lifts and P-9, `update` is `f(get(list, i))` and `update2` is
  `f(get(list, i), get(list, j))` under a `Distinct(i)` claim. The
  compiler work sequenced in instead: (1) a proven `Distinct` claim
  exempts a second mint from D's exclusivity — the first refinement of
  the computed-index may-alias rule [fate-field-disjoint]; (2)
  proven-disjoint handles do not poison each other; (3) a general Rust
  pair lowering (`salvo_pair_mut` in `runtime/seq.rs`: `split_at_mut`
  with index ordering, safe, `i != j` checker-guaranteed)
  [rs-runtime-source].

**The build sequence** (progress markers as steps land): ① **BUILT
2026-09-24** — [proj-mut] + [rs-elem-mut]: the acceptance and mutation
lifts, root-poisoning with the acting handle exempted, P-9's `handle_muts`
mode table, the `get_mut` splice and captured-index virtual bindings,
`List<Mut T>` parameters arriving `&mut`; the v1 cut (bound mints only
from a direct `get(place, i)!`) reported loudly. ① P-3's lifts + P-9 (the
D substrate) →
② `Distinct` in std + the three compiler pieces → ③ the C family as plain
std code → ④ `canbe` entries + the covered store rendering (A) →
⑤ GB-3-A. The document folds into COMPLETED.md's log, spec rules and the
ROADMAP plan as each step lands, and is deleted after ⑤, per its charter.
