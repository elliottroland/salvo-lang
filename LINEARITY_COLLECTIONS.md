# Linearity in collections — the option space (working document)

Status: **OPEN** — nothing here is decided. This is the first of the three
design prerequisites the user sequenced ahead of implementing asynchronous
effect handlers (CONCURRENCY.md, "The first pass", 2026-09-15): the
linearity-in-collections design comes first, **with the concurrency first
pass as its first customer**. The recommendations are recommendations; the
calls are the user's (AGENTS.md's first invariant).

Written 2026-09-15 by the same read-only session that wrote CONCURRENCY.md,
in the DESIGN_DOC.md shape. This document answers the question ROADMAP.md
carries as **L8** ("composition plus conditional linearity are one design
question") and that [linear-composite] explicitly defers to: the interim rule
refuses a linear value in any composite *until that question is answered*.
This is the answering.

**Propagation owed once the read-only restriction lifts.** When the user has
made the calls below: fold the outcomes into COMPLETED.md's decision log,
retire or rewrite [linear-composite] and its interim language in
LANGUAGE.md/LANGUAGE_SPEC.md, close ROADMAP.md's L8, and delete this document
once its outcomes live in the log — the OBLIGATIONS.md charter. Until then
nothing here has propagated anywhere.

Sources: LANGUAGE.md ("Linear types: values that must be used" — the
obligation model, discharge sets, the composite refusal, generics opt-in),
LANGUAGE_SPEC.md ([linear-obligation], [linear-composite] with its L6d
history, [linear-union-arm] and its `linear_settled` flow fact,
[linear-generics], [linear-static], [iter-protocol]), ROADMAP.md (L8; R4
part 2), COMPLETED.md (decision L6d — contagion, replaced 2026-09-08; O-C2,
2026-09-12), CONCURRENCY.md ("The first pass") and the two examples files
(the customer's shapes), and the survey below (verified against primary
sources).

## 0. The stated intent — the customer's shapes

No fresh user sketch this time; the intent is the set of shapes the decided
concurrency design already writes, collected so the decisions below can be
judged against them. All of them are today refused by [linear-composite].

1. **A list of parked reply tokens.**
   `waiting: Mut List<Reply<Notice | Err Str>>` — operations: `add` (park,
   obligation moves in), take-first (unpark, `T?`-shaped — the list may be
   empty), drain (shutdown: answer every parked requester), `size`.
2. **A map of linear composites.**
   `gathers: Mut Map<Int, Gather>` where `struct Gather { reply: Reply<…>,
   hits: Mut List<Hit>, outstanding: Int }` — a *struct holding a linear
   field*, keyed put, take-by-key (`Gather?`), spread-update
   (`Gather {...g, outstanding: g.outstanding - 1}`).
3. **A list of linear domain values.**
   `stock: Mut List<Notice>` with `linear struct Notice` (assign-or-return —
   the NoticeFetcher invariant): `add`, take-first, `drain` into a returning
   send.
4. **Handler-state fields holding all of the above.** Every one of these
   lives in *process state* — a handler field — so the field-level composite
   refusal bites before the collection-level one does.
5. **The compiler's own pending storage** (the future call-sugar pass parks
   generated continuations): should ride on whatever is decided here rather
   than a private mechanism, so the rules must be good enough for the
   compiler to be their heaviest user.

Out of scope, deliberately: sendability (C-4 owns what may *cross* a
process; this document owns what may be *held*), laziness/pipeline functions
(post-phase-5), and the supervision/death story (prerequisite 3 — though
LC-4 below hands it a precisely-shaped question).

## 1. Fixed points — already decided, inherited here

- **The obligation model** [linear-obligation]: every linear value must be
  moved onward on every path; moves transfer the obligation; a borrow leaves
  it with the caller; dropping is a compile-time error naming the discharge
  set. Discharge happens only in the type's own file's consuming functions;
  `discard` is legal only inside a discharger. **No implicit discharge
  exists** (`defer` was removed 2026-09-10 for exactly that).
- **Linearity is declared, not applied**: `linear struct` at the
  declaration; there is no use-site `linear`. Whatever rule emerges for
  containers must preserve this — a `List<FileHandle>`'s linearity must be a
  *consequence*, never a spelling.
- **Purely static, both backends** [linear-static]: no runtime component, no
  destructors. This is load-bearing against one whole family of solutions
  (Mezzo's, §2).
- **[linear-composite] is explicitly interim** (user decision 2026-09-08,
  replacing L6d): a linear value lives only in a local, a parameter, a
  return value — or a union arm. Refused *at the store*. The rule's own text
  defers to L8; this document is L8's answer. The history matters: **L6d was
  contagion** — a composite holding a linear component became linear itself —
  and it was replaced not because contagion was wrong but because
  conditional linearity ("a `Box<T>` is linear exactly when `T` is") was
  undesigned. Any revival must bring that design.
- **[linear-union-arm] (O-C2) is the carve-out that works**: a union value
  *is* the value; the un-narrowed union owes; narrowing to a non-linear arm
  discharges (the `linear_settled` flow fact, joined across branches). `T?`
  falls out — which is exactly the shape every take-from-collection wants to
  return.
- **[linear-generics]**: `<T canbe linear>` opts a *function's handling* in;
  the std collection surface is already audited and opted where sound
  (`list`, `mut_list_of`, `add`, `size` are callable with linear `T`) — the
  *store* is what [linear-composite] refuses. `get` deliberately stays out
  (returns an alias — would duplicate the obligation); `copy` refuses linear
  values; variadic positions refuse them (untracked).
- **Partial moves through fields already work**: consuming `p.tags` marks
  `p` partially moved; restoring the field makes it whole. The
  handler-state story (LC-4) leans on this machinery as it stands.
- **The customer's decided context** (CONCURRENCY.md): `Reply<T>` is linear;
  handler state is process state, exclusive by serialization; C-4(a) is the
  sendability rule; the supervision/death design is sequenced *after* this
  one and before implementation.

## 2. What other languages teach

### Rust — the API-shaped answer, minus the guarantee

Rust is affine, not linear: dropping is always allowed (`mem::forget` is
safe), so Rust never had to solve *this document's hardest case* — a
collection dropped while still holding obligations — because `Drop` runs and
affinity forgives. What Rust does teach is that **moving out of a collection
is entirely an API-design problem**: you cannot move out through an index
(E0507); you move out through operations that leave the container whole —
`Vec::remove`/`pop`/`swap_remove`, `HashMap::remove`, `Option::take`,
`mem::take`/`replace` (swap a placeholder in), `drain` (consume a range by
iteration), `into_iter` (consume the whole container). Every one of these
returns the value and never leaves a hole. The lesson transfers wholesale;
the missing piece — refusing the still-full drop statically — is ours to
add.

### Vault — adoption and focus

Fähndrich & DeLine (PLDI 2002): a linear object **adopted** by a container
becomes freely aliasable (its individual identity leaves the static
discipline); **focus** temporarily recovers linear access to one adoptee at
a time. The insight to keep: *per-element static tracking is abandoned at
the container boundary and recovered per-access* — the obligations
rank-collapse into the container, and an access is a scoped loan. Vault kept
this static; its price was the focus discipline's complexity.

### Mezzo — adoption and abandon, the runtime pole

Mezzo's version of the same idea checks the recovery (**abandon**) at
*runtime* — the unique-owner policy "relaxed and enforced in part at
runtime". It marks the pole [linear-static] rules out: if per-element
recovery from a shared container fundamentally needs a dynamic check, Salvo
must instead shape the API so the check is never needed (take-by-move
returning `T?` — the emptiness/absence check *is* the union narrow, already
static).

### Austral — strict linearity, consume-and-return APIs

A strictly linear systems language in production shape: temporary access
without consumption is **borrowing** (scoped, statically delimited);
everything else consumes and returns. Confirms that a fully static, strictly
linear discipline over container-like APIs is livable — and that its
ergonomics depend on exactly the take/return/borrow trio this document has
to specify.

### Linear Haskell — consumption propagates through structure

Multiplicity polymorphism: consuming a list *linearly* means consuming each
element linearly — a fold over a list of linear values discharges them all,
and the type of the fold says so. This is **contagion done right**: the
container's linearity is a consequence of its element's, and *consuming
iteration is the natural terminal*. The direct ancestor of LC-1's
recommendation.

## LC-1. The ownership model: what is a `List<Reply<T>>`?

The core call. Three poles:

- **(a) Conditional contagion — the container is linear when its element
  is.** L6d revived, now *with* the conditional-linearity design it lacked:
  a generic type opts in per parameter (`struct List<T canbe linear>` — the
  existing [linear-generics] phrase, extended from functions to type
  declarations), and an instantiation with a linear `T` **is a linear
  type**: `List<Reply<X>>` owes; `List<Int>` does not. Same for `Box<T>`,
  `Map<K, V canbe linear>` (values only — LC-3), and user types. Linearity
  stays declared-not-applied: the *element's* declaration is the source, the
  container's is a conditional carrier, and no use-site spelling exists.
  *Cost:* the checker learns instantiation-dependent linearity; the leak
  diagnostics must name the container's terminal (LC-2's `drain`) rather
  than a discharge set in the element's file.
- **(b) A dedicated linear container** (`Held<T>` or similar): ordinary
  `List`/`Map` stay linear-free forever; one new std type carries the
  audited API. *Benefit:* zero blast radius on existing collections.
  *Cost:* a parallel collection surface that grows forever (the customer
  needs list *and* map shapes on day one), and the "which one do I use"
  question lands on every user; the generated pending storage would bake in
  a second-class citizen.
- **(c) Adoption/focus:** the container stays non-linear; obligations
  transfer to a scope/region object and accesses are focus-loans.
  *Benefit:* containers unchanged. *Cost:* a new ownership mechanism of
  research weight, and Mezzo's experience says the recovery step tends
  toward a runtime check, which [linear-static] forbids.

**Recommendation (the user's call):** (a). It is Linear Haskell's model, it
answers L8 as one rule ("a composite is linear exactly when a component
is"), it needs no new spelling, and the union-arm precedent already
established the pattern of linearity-as-consequence. (b) remains the
fallback if (a)'s checker cost surprises.

## LC-2. The API: how obligations enter, move through, and leave

Whichever container model wins, the operations are the design. The rule that
generates all of them: **no operation may implicitly drop an element, and no
operation may duplicate one.**

- **In:** `add(list, v)` / `put(map, k, v)` move the value in; the
  obligation joins the container's. (`put` over an existing key would drop
  the old value — so on linear-valued maps, **`put` returns the displaced
  `V?`**, and the void-returning form does not exist for them.)
- **Out, the workhorse:** take-by-move returning the [linear-union-arm]
  shape — `take_first(list) -> T?`, `take(map, k) -> V?`, `set(list, i, v)
  -> T` (displace). The `None` arm owes nothing (O-C2); the absence check
  *is* the narrow, statically. No `get` for linear elements (an alias would
  duplicate the obligation — already the [linear-generics] status quo).
- **Reading without consuming:** non-consuming, view-shaped iteration
  (`for x in list` over proj'd elements) is legal — a borrow leaves the
  obligation in place, matching "lambdas may read but not swallow". What a
  view may *do* is read-only; passing a viewed element to a consuming call
  is refused as any borrow-consume is.
- **The terminal:** `drain(list)` consumes the container and returns a
  linear pass [iter-protocol]; **a `for` that drives a linear pass runs it
  to `Finished` by construction, so `for x in drain(list)` is the
  container's discharge context**, with each element's obligation landing in
  the loop body to be discharged individually. `clear` does not exist for
  linear elements (it is a mass drop). An `into`-style whole-container move
  to another owner is just a move.
  * *Flag:* this rests on "`for` always exhausts". If a `break`/early-exit
    construct exists or is ever added, breaking out of a linear drive must
    be an error (the pass still owes) — the same shape as returning
    mid-obligation, and the flow analysis already speaks that language.
- **Neutral:** `size`, `is_empty` — fine, touch no elements.

**Recommendation (the user's call):** exactly this surface, audited into
std under the existing `<T canbe linear>` regime — it is Rust's proven API
shape plus the one thing Rust could not give (the static still-full-drop
refusal, which falls out of LC-1(a): a linear `List` that is never drained
or moved onward is an ordinary leak diagnostic naming `drain`).

## LC-3. Which containers participate

- **`List<T>` and `Map<K, V>` values: yes** — the customer's shapes.
- **`Set<T>`, `SortedSet<T>`, and map *keys*: no.** Insertion into a set
  deduplicates — inserting a duplicate silently drops one of the two values,
  which is a hidden discard no API reshaping can fix (returning the
  displaced element on collision would make set-insert order-dependent in a
  way `==`-based dedup cannot honestly express). Keys additionally get
  compared and retained internally. Refused at the instantiation, same
  diagnostic shape as today.
- **Arrays and tuples: not in the first round.** Arrays sit next to the
  variadic boundary (untracked positions) and tuples already have the
  union-arm alternative for the two-things case; neither is a customer shape.
  Extend later if demanded.
- **Struct fields — the composite proper:** a struct holding a linear-typed
  field must itself be declared `linear struct` (an error at the declaration
  otherwise, naming the field): contagion is *spelled*, not inferred,
  preserving declared-not-applied at the declaration level. Its discharge
  set is as today (its file's consumers), and **destructuring is a natural
  discharger**: taking a `Gather` apart moves `reply` out (obligation
  continues per-field) and ends the struct's own obligation. The spread
  update (`Gather {...g, outstanding: …}`) is a destructure-and-rebuild —
  legal, obligation flows through.

**Recommendation (the user's call):** as listed. The set/key refusal is the
one worth saying loudly in LANGUAGE.md, because it is semantic (dedup *is*
dropping), not an implementation fence.

## LC-4. Handler state — where the customer actually lives

Every shape in §0 sits in a handler field, and handler state is unlike a
local: it persists *across activations*, and no static analysis can know
what a process holds at an arbitrary future point. The honest options:

- **(a) The process owns the obligations; activations must keep state
  whole.** A handler field may hold linear values and linear containers
  (per LC-1/LC-3). Within one activation, the existing partial-move
  machinery governs: take from a field, and either discharge what you took
  or restore the field before returning — **an activation must leave every
  state field whole at return** (this is today's rule for `Mut` parameters,
  applied to the handler's implicit self). Across activations, the
  obligations rest with the process — and the guarantee honestly weakens
  from "every path discharges" to "**the process owes until it ends, and
  what end-of-life does with parked obligations is the supervision
  design's first question**" (prerequisite 3, deliberately sequenced next).
- **(b) Refuse linear values in handler state.** Keeps the strong
  guarantee; kills the entire customer (`waiting`, `gathers`, `stock`, the
  generated pending storage) — the concurrency design as decided cannot be
  built.
- **(c) Per-process discharge sets** — require every handler holding linear
  state to declare a terminal member that drains it, checked at… some
  notion of process end that does not exist yet. This is (a) plus
  machinery that cannot be designed before the supervision story exists;
  premature.

**Recommendation (the user's call):** (a), with the weakened-guarantee
sentence written into LANGUAGE.md verbatim rather than discovered — and the
question it hands forward ("a process died holding obligations: what
happens?") recorded as the opening requirement of the supervision design.
Note what (a) preserves: within any single activation the discipline is as
strong as ever, and `Shutdown`-style drain paths are still *forced* by the
leak diagnostics whenever the code path exists (the NoticeFetcher shutdown
example — the drain loop is compulsory, not stylistic).

## LC-5. Generics and the opt-in boundary

LC-1(a) extends `canbe linear` from functions to type declarations. The
follow-on rules, stated so they are chosen rather than discovered:

- A type parameter not opted in still refuses linear instantiation — the
  default is unchanged; `struct Pair<A, B>` holds no handles unless it says
  `<A canbe linear>`.
- An opted parameter *inside* the declaration is treated as linear
  everywhere it appears (fields carrying it make the type conditionally
  linear; the body's functions handle it under the existing
  [linear-generics] rules).
- Std audit: `List` and `Map` opt their element/value parameters in; `Set`
  and key positions do not (LC-3). The audited-function list from
  [linear-generics] extends with the LC-2 surface (`take_first`, `take`,
  displacing `set`/`put`, `drain`).
- The conditional-linearity *judgment* is purely structural: an
  instantiation is linear iff any type argument in an opted position is
  linear (transitively — `List<Gather>` where `Gather` is linear-by-field).
  No annotations at use sites, ever.

**Recommendation (the user's call):** as stated; it is the smallest rule
set that makes LC-1(a) precise.

## Decisions pending

| # | Question | Recommendation (user's call) |
|---|---|---|
| LC-1 | Ownership model for holding containers | Conditional contagion (a): opted containers are linear when an element type is |
| LC-2 | The API surface | Take-by-move (`T?`), displacing writes, `drain`+`for` as the discharge terminal, views read-only; no `get`, no `clear` |
| LC-3 | Which containers | `List` + `Map` values yes; `Set`/keys refused (dedup is dropping); arrays/tuples deferred; struct fields require declared `linear struct`, destructuring discharges |
| LC-4 | Handler state | Process owns the obligations; activations leave fields whole; end-of-life handed to the supervision design as its first requirement |
| LC-5 | Generics opt-in for types | `canbe linear` on type parameters; structural conditional-linearity judgment; std audit as listed |

Load-bearing order: **LC-1 first** (everything else is phrased inside it),
then **LC-4** (it decides whether the concurrency customer is buildable and
hands the supervision design its opening question), with LC-2/LC-3/LC-5 as
the surface that follows. The one flag to resolve during implementation
rather than decision: the `for`-exhausts assumption in LC-2's terminal
(early-exit constructs must refuse linear drives). See DESIGN_DOC.md for the
shape this document follows.
