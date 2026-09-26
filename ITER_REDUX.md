# Iteration redux: `Iter` as a qualifier carrying its `next`

Design review of the 2026-09-26 proposal, written from a read-only session.
Nothing here is decided; the numbered items under "Gaps" are the calls that
are the user's, with a recommendation each.

## The proposal, as read

```
// The step protocol, unchanged. Used in exactly one place now: the slot list.
export params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}

// A pass: a value of some type It together with the `next` that drives it.
export qualifier Iter<It, T>(?Yield<It, T>) of It

// A source: something an `iter` mints a pass from.
export params Iterable<S, T> {
    fn iter<It>(source: S, ?Yield<It, T>) -> Iter<It, T>(?next) It
}

// The one public driver, generic over any pass — it calls what the type carries.
export fn next<It, T>(iter: Iter<It, T>(?next: inner_next) Mut It) -> Emitted T | Finished
=> iter: Mut {
    return inner_next(iter)
}

// The sugar: an implementation of `iter` that does not name its pass.
iter fn iter<S, T>(source: S) -> Emitted T | Finished { state { … } … }
```

## Verdict

Reasonable, and better than what is built, for reasons beyond the one given.
The shape is the `Heap<T>(?Ordered<T>)` shape applied to iteration, and the
heap already exercised every mechanism it needs: fn slots, group spreads into
a slot list, binder capture from an argument type, a constructor that
publishes what it resolved, erasure to a direct call. The proposal needs
**one new rule** ([qual-generic] must allow a type argument that is fixed by a
slot rather than by the `of` type), **one constructor in `core.iterator`**
(because of [qual-ctor-same-file]), and it should **drop two things from the
sketch** (the `?Yield` implicit on `iter`, and the member-level `<It>` on the
group). Details below.

One correction to the diagnosis first. Today the `: Yield<self, T>` clause is
*not* declared on the source: `desugar::expand_pass_fns` puts it on the hidden
`__Pass_<Subject>` struct, and the source stays a plain struct ([iter-fn]).
What actually reads wrong is the sugar's **signature**: `iter fn next(c:
Countdown)` says "the `next` of a `Countdown`", and a `Countdown` is not a
pass. The proposal fixes exactly that — the sugar is named for what it
declares (a minter), the step is hidden — so the motivation stands, with the
mechanism described accurately.

## What it buys (including what the proposal does not claim)

- **`T` becomes an ordinary type argument.** Today a combinator's element type
  is found by a special case: a `?Yield<It, T>` spread "reads `T` off `It`'s own
  `: Yield<self, T>` clause" ([implicit-infer], second bullet). Under `Iter<It,
  T>(?next) Mut It`, unifying the parameter against the argument's type binds
  `It`, `T` and the binder in one step. The special case is deleted, and the
  same unification types a bare lambda (`map(iter(xs), n -> n * 2)`).
- **The drive is a call to an identity the type carries**, not an overload
  scan. `for` today resolves *which* `next` at the loop, and [iter-resolve]
  grew a "match by declaration, not by type name" fix on 2026-09-25 because two
  same-named structs in two modules drove each other's `next`. With the
  identity in the type, that class of defect cannot occur: the pass was bound
  to its step where it was minted.
- **The step function need not be called `next`.** A hand-written pass names
  its step whatever it likes and binds it (`Iter(step_zip) Mut Zip<A, B>`);
  the sugar hides its step entirely. Today every pass in the program adds a
  `next` overload to one global set, and [iter-generic-drive] has to insist on
  the *name* `next` to find the implicit to drive with. The binder makes the
  name irrelevant: `for x in it` inside a generic fn calls the slot.
- **`iter fn` generalises to every producer** — the point the proposal makes,
  and the largest std win. `core.list` alone hand-writes `ListRevYield`,
  `IdxYield`, `ListEnumYield` and their minters for `reversed`, `indices`,
  `rev_indices`, `enumerate`, `enumerate_rev`; each becomes one `iter fn
  <name>(list: List<T>) -> Emitted … | Finished` with a `state` block. The
  `iter fn` in `core.range` stops being the one exception that is named `next`
  yet generates an `iter`.
- **`Yield` does one job.** It is spread into `Iter`'s slot list and nowhere
  else; the struct-obligation use (`: Yield<self, T>`) goes away. The
  [group-obligation] machinery stays for `Locate` and the rest; [group-self]
  loses its motivating customer but not its rule.
- **No runtime change.** Qualifiers erase; a bound identity lowers to a direct
  call; inside a generic fn the binder is a fn parameter, which is exactly what
  the `?Yield` spread lowers to today (`next: &mut dyn FnMut(&mut It) -> …`).
  "There is no iterator type" ([Passes](docs/language/Passes.md)) remains true:
  `Iter` is a claim on a concrete `It`, and two passes are still two unrelated
  types.

## Gaps and decisions

### 1. Who may establish `Iter` — [qual-ctor-same-file]

`+Q` may be written only in the qualifier's own file. `core.list`'s `iter`,
and a user's `iter` for `Bag` or `zip` for `Zip`, cannot write
`-> +Iter<…>(next) Mut ListYield<T>`.

**Recommendation**: the heap precedent, exactly. `core.iterator` exports the
one constructor, and every minter *keeps* what it published rather than
establishing anything:

```
// core.iterator — the only `+Iter` in the program.
export fn pass<It, T>(it: Mut It, ?Yield<It, T>) -> +Iter<It, T>(?next) Mut It => !it {
    return it
}

// core.list — keeps the claim `pass` published; no `+`, no trust.
export fn iter<T>(list: List<T>) [] -> Iter Mut ListYield<T> => list {
    return pass(Mut ListYield<T> { items: list, at: 0 })
}
```

The `?next` at the `pass(…)` call resolves in `core.list`'s scope for
`(Mut ListYield<T>) -> …`, which is the step `core.list` declared — the right
locus (see 2). The generated `iter` of an `iter fn` calls the same
constructor with its hidden step. The name `pass` is a placeholder; `as_iter`
is the other candidate.

Two things to check when building it: the constructor moves a pass through a
generic parameter and must **preserve the pass's borrow of its container**
([proj-infer] — today `iter`'s lend of `list` is inferred from the struct
literal; here it has to flow through `pass`'s return); and the result type
should be writable with the type arguments *omitted* (`Iter Mut
ListYield<T>`), since `It` duplicates the subject and `T` is fixed by the
identity's return type — [qual-value-arg]'s all-or-none rule already permits
the empty list.

### 2. Drop the `?Yield<It, T>` implicit from `iter`

As sketched, `iter`'s `next` is an implicit **of `iter`**, so it resolves at
the *call site of `iter`*, for whatever `It` that call binds. Three problems:
the caller may override it (`iter(xs, next = other)`), which is a feature
nobody asked for; every pass's step must be exported and visible wherever
`iter` is called, where today it is resolved in the pass's own file; and `It`
appears in no non-implicit parameter, so resolution runs with `It` unbound —
the ambiguity [implicit-infer] accepts for container-shaped combinators,
here on the most common call in the language.

**Recommendation**: the identity is bound **where the pass is minted**, by
the constructor in 1. The minter's signature carries the *result*
(`Iter Mut ListYield<T>`, or the bare-name form `Iter(step) Mut Zip<A, B>`),
never an implicit. A pass's step is then private if its module wants it so.

### 3. The member-level `<It>` on `Iterable.iter` is new machinery

`fn iter<It>(source: S, …) -> Iter<It, T>(?next) It` quantifies `It` on the
member. Spread as `?Iterable<S, T>`, that gives an implicit whose fn type has
its own type variable — an existential return — which Salvo's fn types do not
have. The obligation form (`struct Bag : Iterable<self, Int>`) is the reason
to want it (an `iter fn`'s pass is unnameable, so the pass type cannot be a
group argument), but the obligation buys little: the `iter fn` *is* the
declaration, and a delegating container (`fn iter(bag: Bag) -> Iter Mut
ListYield<Int>`) can name its pass.

**Recommendation**: no `Iterable` group in the first cut. `for x in e`'s
third rung stays "an `iter` overload accepting `e` whose result carries
`Iter`" — the resolution [iter-resolve] already does, plus one check. A
container-shaped combinator writes what it writes today, with the claim
replacing the second spread:

```
fn total<S, It>(s: S, ?iter: (s: S) -> Iter<It, Int>(?next) Mut It) -> Int =>[iter] !s {
    let sum = 0
    for n in iter(s) { sum = sum + n }
    return sum
}
```

This does need one small extension: the `?next` binder is filled by
**capture from the resolved implicit's return type**, where [cmp-binder]
today captures from argument types only. It sits in the same repeated sweep
[implicit-infer] already runs. If a group is wanted later, `params
Iterable<S, It, T>` with `It` as a group parameter needs nothing new and
serves the delegating-container obligation; only the iter-fn source cannot
state it, and does not need to.

### 4. [qual-generic]: a type argument the `of` type does not mention

"Qualifiers can be generic, and as generic as their `of` type or less."
`Iter<It, T> of It` has `T` in the slot's type only. The rule has to admit a
type argument **determined by a slot** — instantiated by matching the `of`
type against the value *and* the slot's type against the identity. Small,
and the unification already runs both ([cmp-carry]: "a slot's type is
instantiated by matching the qualifier's `of` type against the value").

### 5. Overload conflict on `next`

[fn-overload-rank] declares "a more specific base with a smaller qualifier
set" **unrankable** against the reverse. `next(p)` with `p: Iter(next) Mut
ListYield<Int>` sees `core.list`'s `next<T>(p: Mut ListYield<T>)` (concrete
base, no qualifier) beside `core.iterator`'s `next<It, T>(iter: Iter<It,
T>(?next: inner) Mut It)` (variable base, one qualifier) — same rung
([fn-overload-scope]: both `core`), unrankable, an error at every manual
drive of a std pass.

**Recommendation**: the generic driver is the **only** thing named `next`.
std's steps take other names (`step`, or per pass — they are no longer
looked up by name), the sugar's are hidden, and user passes are told the
same in the docs. Inside the driver the alias (`?next: inner_next`) is
already what the sketch does.

### 6. Dependent element claims lose their `self`

`IdxYield<T> : Yield<self, Idx(self.items) Int>` names the pass's own field
from the clause, and `next` returns `Emitted (+Idx(p.items) Int)`; the two
are "one slot from two vantage points, matched by the field" ([qual-depend],
step 5). In `Iter<IdxYield<T>, Idx(?.items) Int>` there is no `self` to
write: a qualifier has no name for its own subject in its type arguments.

Under the sugar this is *simpler* than today, because the source parameter
names the place: `iter fn indices<T>(list: List<T>) -> Emitted (+Idx(list)
Int) | Finished`, whose generated minter returns `Iter<__Pass, Idx(list) Int>
…` — a claim about a parameter in a return type, which [qual-depend] already
allows (`binary_search -> (+Idx(list) Int)?`). Since `indices` and
`rev_indices` become `iter fn`s, std needs no spelling for the hand-written
case. **Recommendation**: accept that a *hand-written* pass cannot yet emit a
claim about its own field, and record it; the spelling, if a customer
appears, is a subject name in the slot list (`Iter<It, T>(?Yield<It, T>) of
it: It`, then `Idx(it.items)`), which is the value-slot syntax pointed the
other way.

### 7. A claim is dropped by a signature that does not mention it

[qual-erasure]: passing `Iter(step) Mut Zip<A, B>` to `fn take(p: Mut Zip<A,
B>, …)` hands the body a plain `Zip`, and `for x in p` inside is refused. Every
signature that *drives* a pass must say so — `p: Iter Mut Zip<A, B>` — where
today the struct's clause made the type a pass everywhere. One word per
signature, and the same word `Heap` costs; but it is a change in kind: "is
this a pass?" becomes a property of the value's type at this point rather
than of the struct. A struct literal never carries it (as a bare list is not a
heap), so a hand-written pass is minted through its constructor fn, and
`for x in Mut Zip { … }` is not a loop.

Drive-in-place ([iter-drive-in-place]) is unaffected: `let z = zip(xs, ys)`
infers the claimed type, the loop drives `z` where it lives, and a `take(z:
Iter Mut Zip<…>)` keeps the claim and the position. What to watch is the
**error text**: "not iterable" on a `Zip` that has a step should name the
remedy (mint through the constructor, or type the parameter `Iter`), the way
today's names the missing clause.

### 8. Unnameable passes compose badly, and generalising `iter fn` spreads it

"The pass has no name" ([iter-fn]) is a cost today; if every std producer
became an `iter fn`, nothing could store or wrap one: a `state` field needs
an annotation, and `fn iter(bag: Bag) -> Iter Mut ListYield<Int>` names
`ListYield`. **Recommendation**: keep the passes downstream code names as
structs (`ListYield`, `ArrayYield`, `StrYield`, `BytesYield`, `SetYield`,
`MapKeyYield`), and convert the ones nobody stores (`reversed`, `indices`,
`rev_indices`, `enumerate`, `enumerate_rev`, `Range`). The alternative —
inferring `state` field types — is the "all three sites gain inference
together" item already recorded under [iter-fn].

### 9. Smaller checks

- **Hidden struct naming.** Today `__Pass_<Subject>`; with several `iter fn`s
  over one source (`iter`, `reversed`, `indices` over `List<T>`) the name must
  include the fn (`__Pass_reversed_List`). Same for the hidden step.
- **Generic identities.** Every identity std binds today is attached or
  monomorphic (`cmp@Person`, `min_by_age`). `Iter(step) Mut ListYield<T>`
  binds a *generic* fn `step<T>`, instantiated per `T` — the identity domain
  compares by name and substitutes like a type argument ([cmp-carry]), which
  should suffice, but nothing has exercised it.
- **Slot contracts.** The slot type is `(Mut It) -> Emitted T | Finished =>
  it: Mut`. `ListEnumYield`'s step adds `holds proj(p)`, and the walking
  passes return `Emitted (proj(p) T)`. Matching a candidate's contract against
  a slot's is already directional (probed 2026-09-26: a lending slot accepts a
  non-lending candidate), so this should work as the `?Yield` spread does
  today; verify on both.
- **The sugar's return type still lies a little.** `iter fn iter(c) ->
  Emitted Int | Finished` reads as "`iter` returns an element" while `iter(c)`
  returns a pass. Today's form has the same mismatch in the other direction
  (`next` generating `iter`). Accept it: `iter fn` already means "the body is
  the step, the declaration is the minter". The alternative — a different
  return spelling for the sugar — is more syntax for the same fact.
- **`Iter` was the name of the deleted iterator type** (COMPLETED.md, R5). No
  conflict in the code; a reader of the history will meet both meanings.
  `Pass` would avoid it but collides with the vocabulary ("a pass is a value…").

## Migration footprint

Everything below is touched; none of it is hard, and it is all the same
change.

- `std/core/iterator.sv`: the qualifier, the constructor, the generic `next`;
  `Yield` unchanged. Every `std/core/*.sv` pass: drop `: Yield<self, T>`,
  rename the step, mint through the constructor, convert the unnamed producers
  to `iter fn`s. `seq.sv`: `?Yield<It, T>` spreads become `Iter<It, T>(?next)
  Mut It` parameters.
- Syntax: `expand_pass_fns` takes a fn name, hides the step, emits the
  constructor call; the parser accepts any name after `iter fn`.
- Checker: `find_next_driver`/`PassDriver` read the claim's slot instead of
  scanning; [iter-generic-drive] keys on the claim, not on a spread named
  `next`; [implicit-infer]'s designated-group bullet is deleted; [qual-generic]
  relaxed (4); binder capture from a resolved implicit's return (3).
- Emitters: the drive calls the bound identity or the binder parameter —
  already the two shapes the spread produces today.
- Docs: `Iteration.md`, `Passes.md`, `Implicit-Parameters.md` (the struct
  obligation section loses its example), `Dependent-Qualifiers.md` (step 5),
  README's iteration bullet; LANGUAGE_SPEC rules [iter-protocol] [iter-fn]
  [iter-resolve] [iter-generic-drive] [seq-pass] [implicit-infer] [group-self]
  [qual-generic]; `examples/iteration` and its generated output; the corpus.

## Recommended shape, in one block

```
export params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}

export qualifier Iter<It, T>(?Yield<It, T>) of It

export fn pass<It, T>(it: Mut It, ?Yield<It, T>) -> +Iter<It, T>(?next) Mut It => !it {
    return it
}

export fn next<It, T>(iter: Iter<It, T>(?next: step) Mut It) -> Emitted T | Finished
=> iter: Mut {
    return step(iter)
}

// A hand-written pass: struct, step under any name, minter keeping the claim.
struct Zip<A, B> canbe Mut { left: List<A>, right: List<B>, at: Int }
fn step_zip<A, B>(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut { … }
fn zip<A, B>(left: List<A>, right: List<B>) -> Iter(step_zip) Mut Zip<A, B> => left, right {
    return pass(Mut Zip<A, B> { left: left, right: right, at: 0 }, next = step_zip)
}

// The sugar, under any name.
iter fn reversed<T>(list: List<T>) -> Emitted (proj(list) T) | Finished {
    state { at: Int = size(list) - 1 }
    …
}

// A combinator: the claim replaces the spread; T is ordinary.
export fn map<It, T, U>(it: Iter<It, T>(?next) Mut It, f: (T) -> U) [] -> Mut List<U> => it: Mut, f {
    let out = mut_list_of<U>()
    for x in it { add(out, f(x)) }
    return out
}
```

Decisions for the user, in the order they block: 1 (constructor, and its
name), 2 (no implicit on `iter`), 3 (no `Iterable` group in the first cut),
5 (`next` is only the driver). 4, 6, 7, 8 and 9 are consequences to accept
or record once those are made.

---

# Round 2 (2026-09-26, later): answers, and a revised recommendation

The user's replies: 1 accepted with the name `make_iter`; 2 asked to
confirm the shape; 3 asked whether existential fn types are hard; 4 and 7
judged hard blockers; 5 disputed (`Mut ListYield<T>` looks *more* specific);
6 accepted; 8 asked for an example. Plus: what about `Iter<T>` as an
intrinsic type bound to a `next` by an `iter fn`?

Answering 7 changed the recommendation, so that comes first.

## The revision: the claim is declared on the pass struct, not minted

Gap 7 is real: with `Iter` an ordinary constructive qualifier, "is this a
pass?" becomes a property of a *value at a point*, dropped by any signature
or field that does not spell it. And it is worse than the first round said.
A struct that **stores** a pass (`struct Merge { a: Iter Mut ListYield<Int>,
… }`) reads the field back as a *pattern* — slot unconstrained — so `for x in
m.a` has no identity to call. The field would have to name it
(`a: Iter(next@ListYield) Mut ListYield<Int>`), which forces every step to
be exported and puts an `@`-canonical in every field type that holds a pass.
(std has no struct field holding a `Heap`, so this hole is shared with
`Heap`, not created here — but iteration would hit it immediately.)

The fix is the one precedent the language already has for a claim every
value of a type carries: **`linear struct`**. `Linear` is not minted and not
dropped; the declaration makes every value of the type wear it. Let a struct
declare `Iter` the same way — the current `:` clause, with a qualifier and
its slot filled where today it names a group:

```
export qualifier Iter<It, T>(?Yield<It, T>) of It

struct Zip<A, B> : Iter<self, (A, B)>(step_zip) canbe Mut {
    left: List<A>,
    right: List<B>,
    at: Int
}

fn step_zip<A, B>(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut { … }
```

`Zip<A, B>` then *normalises* to `Iter<Zip<A, B>, (A, B)>(step_zip) Zip<A,
B>` wherever the type is lowered — it is the type's normal form, not a claim
a value gained — so `p: Mut Zip<A, B>` is a pass in every signature and every
field, a struct literal is a pass, and [qual-erasure]'s argument-side
dropping never touches it because it is not separable from the base. The
identity check (does `step_zip` fit `(Mut Zip<A, B>) -> Emitted (A, B) |
Finished => z: Mut`?) happens at the struct, exactly where `: Yield<self,
T>` is checked today [group-obligation].

What this does to the first round's gaps:

| gap | under the constructor design | under the declared claim |
|---|---|---|
| 1 same-file constructor | `make_iter` in `core.iterator` | **gone** — nothing is minted |
| 2 implicit on `iter` | drop it; minters call `make_iter(p, next = step)` | **gone** — `iter` returns `Mut ListYield<T>`, which carries the claim by declaration |
| 4 `T` not in `of` | inferred from the identity at every mint | **declaration only**: `T` is written in the clause and checked against the step's return; use sites read it off the normal form. [qual-generic] still needs to *permit* the declaration, but no inference rides on it |
| 6 `self.items` | no spelling for hand-written passes | **gone** — `self` is available in the clause: `struct IdxYield<T> : Iter<self, Idx(self.items) Int>(step_idx)` |
| 7 erasure | every driving signature and field says `Iter`, fields name the identity | **gone** |
| 5 `next` ambiguity | unchanged | unchanged (below) |
| 3 `Iterable` | defer | defer (below) |
| 8 unnameable | keep named std structs | unchanged |

And it keeps every gain: the identity is in the type, so the drive is a call
to `step_zip` with no overload scan and no name requirement on the step; `T`
is a type argument, so `map<It, T, U>(it: Iter<It, T>(?next) Mut It, f: (T)
-> U)` binds `It`, `T` and the binder by unifying the parameter against the
argument's normal form; the sugar generalises to any name.

The `iter fn` desugar writes exactly this form on its hidden struct:

```
iter fn reversed<T>(list: List<T>) -> Emitted (proj(list) T) | Finished {
    state { at: Int = size(list) - 1 }
    …
}
// ⇒
struct __Pass_reversed_List<T> : Iter<self, proj T>(__step_reversed_List) canbe Mut {
    list: proj List<T>, at: Int
}
fn __step_reversed_List<T>(p: Mut __Pass_reversed_List<T>) -> Emitted (proj(p) T) | Finished => p: Mut { … }
fn reversed<T>(list: List<T>) [] -> Mut __Pass_reversed_List<T> => list {
    return Mut __Pass_reversed_List<T> { list: list, at: size(list) - 1 }
}
```

Callers see `Iter<__Pass…, proj T>(__step…) Mut __Pass…` — unnameable base,
as today, but drivable by anything generic over `Iter<It, T>(?next) Mut It`.

**What it costs.** One new notion: a *user-declared* qualifier a struct
always carries. Today only the compiler's own (`Linear`, via `linear struct`)
work that way, and [canbe-optin] says so explicitly. Implementation is a
normalisation step where struct types are lowered (`qualify()` already
re-normalises `Ty::Qualified` after substitution [qual-generic]), plus
teaching the qualifier-dropping unification that a declared claim is part of
the base. `is Iter` on such a value is meaningless and should be refused
like `is Mut`. Moderate, and it touches the type representation — but it
replaces the `Yield` obligation check rather than adding to it, and it
deletes `make_iter`, the implicit on `iter`, and the inference in 4.

**Honest framing.** This is today's design with the group in the clause
replaced by a qualifier that records the step's identity. The delta from what
is built is small; that is a point in its favour, not against.

## 2. The minting shape (moot under the revision)

Under the constructor design: yes — `iter` promises `Iter … Mut ListYield<T>`
and the body calls `make_iter(p, next = step_list)`. Two consequences the
first round understated: the *return type* must name the identity
(`-> Iter(step_list) Mut ListYield<T>`), because an unwritten slot in a
return type is a pattern and callers would receive an unconstrained claim;
and with steps no longer called `next` (5), the constructor's `?Yield` never
auto-resolves, so every minter writes `next = step` explicitly. Under the
declared claim none of this exists: `fn iter<T>(list: List<T>) -> Mut
ListYield<T>` is the whole signature, as today.

## 3. Existential fn types: what is actually hard

*Checking* is not hard. `?Iterable<S, T>` would spread to an implicit whose
type quantifies `It`; resolution unifies the candidate `iter` with a fresh
variable, and the body treats the result's base as opaque (it is a generic
already), driving through the binder. *Lowering* is the constraint: a Rust or
Kotlin function-typed parameter has a concrete return type, and a rank-2
closure type does not exist in either (Rust's `for<…>` quantifies lifetimes
only). The only way to render it without boxing the pass — which Passes.md
refuses — is for the **caller** to fix `It`, i.e. it is an outer generic of
the enclosing fn after all.

So the honest reading of a member-level `<It>` is **hoisting**: spreading a
group whose member declares its own type parameter adds a fresh, unwritable
generic to the spreading fn; an *obligation* (`struct Bag : Iterable<self,
Int>`) checks the member existentially, which is a one-time match with no
type to carry. That is implementable and not large — [effect-member-generics]
is the nearest precedent — but the hidden generic shows up in diagnostics and
in the all-or-none type-argument rule, and the two customers (the obligation
form, container-shaped combinators) are both served today without it: `for x
in bag` resolves `iter` by name, and a combinator writes `total<S, It>(s: S,
?iter: (s: S) -> Iter<It, Int>(?next) Mut It)`. Recommendation stands:
defer, revisit when an obligation on a source is wanted.

(The binder-capture-through-an-implicit's-return extension from round 1 is
still needed for that combinator shape, under either design.)

## 4. [qual-generic]

Not a hard blocker under either design, and under the revision it is barely
a rule change. The rule exists so applicability can be decided by unifying
the `of` type against the subject; a type argument absent from `of` would be
unconstrained. `T` **is** constrained — by the slot's type, as the user
says — so the relaxation is "every type argument is determined by the `of`
type or by a slot's type". Under the declared claim the determination
happens once, at the struct, where `T` is written and the step's return is
checked against it; nothing at a use site infers anything.

## 5. Why `Mut ListYield<T>` does not beat `Iter<It, T>(?next) Mut It`

The intuition is criterion 1 of [fn-overload-rank]: a type variable says the
least, so `ListYield<T>` beats `It`. But ranking is per slot over *all*
criteria, and criterion 3 (qualifier sets by inclusion: `{Mut, Iter}` says
more than `{Mut}`) points the other way in the same slot. The rule for that
case is explicit: "criteria pulling in opposite directions within one slot
(a more specific base with a smaller qualifier set) are unrankable" — chosen
so that one argument's gain can never pay for a loss, because that is a
guess. The two candidates are therefore ambiguous, not ordered.

Two things reduce the weight. Scope ranks first ([fn-overload-scope]), so a
*user's* `fn next(z: Mut Zip)` beats `core`'s driver from the user's module;
the conflict is between `core.list`'s step and `core.iterator`'s driver, both
in `core`. And the driver is *meant* to be what `next(p)` means for every
pass. So the recommendation stands as std-internal discipline: `next` names
the driver only; std's steps are named per pass. The alternative, if steps
should be called `next` everywhere, is a rank tweak — let a base win over a
qualifier loss when the lost qualifier is one the parameter *does not
mention* (it was going to be dropped by [qual-erasure] anyway). Defensible,
but it is a change to a rule that was made deliberately conservative.

## 8. Where an unnameable pass cannot be used

Anywhere a type must be *written*:

```
// A pass over a pass: the inner one needs a type in the state block.
iter fn pairs<T>(list: List<T>) -> Emitted (T, T) | Finished {
    state { inner: Mut ListYield<T> = iter(list) }    // fine — ListYield has a name
    …
}
// If `iter(list)` were itself an `iter fn`, `inner: ???` has no spelling.

// A struct that holds a pass.
struct Cursor { rows: Mut ListRevYield<Int> }        // needs the name

// A concrete function over one particular pass.
fn take(p: Mut Slice<Int>, count: Int) -> Int => p: Mut   // Passes.md's own example

// A union of two producers (Passes.md, "There is no iterator type").
let ys = if fast { counter(100) } else { primes(100) }    // Counter | Primes
```

Each has a generic escape (`<It>` with `Iter<It, T>(?next) Mut It`), but it
forces genericity on a concrete program, and the `state` field and struct
field have none at all until field-type inference exists. Hence: convert to
`iter fn` the producers nobody stores or wraps (`reversed`, `indices`,
`rev_indices`, `enumerate`, `enumerate_rev`, `Range`), keep the rest named.

## `Iter<T>` as an intrinsic type

Considered and, I think, a dead end in the form asked — but it points at
something.

- **As a real type**, `Iter<T>` is the deleted R5 design (`Iter<T>` →
  Kotlin `Iterable<T>`, Rust `Box<dyn SalvoPass<T>>`): one Salvo type for
  every producer means one representation, which means erasure and dynamic
  dispatch. Passes.md's "nothing is boxed behind your back" is the decision
  against it, and it was made with the costs in view.
- **As an intrinsic type whose representation is per-`iter fn`** (each
  `iter fn` supplies both the state and the step), two `Iter<Int>`s from two
  producers are one Salvo type with two Rust types. `let p = if c { a() } else
  { b() }` type-checks and cannot be lowered. Making it sound requires the
  type to carry *which* producer — a hidden state type and a step identity —
  which is `Iter<It, T>(step) It` with `It` hidden. It converges on the
  qualifier, minus the ability to name the pass.
- **Where it does have merit**: an *explicit* boxed pass — `dyn`-style, opt-in,
  for the two-producers case — is a feature a program might want and could
  be added later without touching any of this. Not now.

## Alternatives to both

- **Rename only.** Keep everything as built; change the sugar's header to
  `iter fn <name>(source) -> Emitted T | Finished`, generating `<name>` plus
  the hidden struct with `: Yield<self, T>` and a hidden `next`. Zero
  type-system change; gets the honest signature and the any-name std collapse.
  Does not get the identity in the type or `T` as an ordinary argument, and
  keeps the overload scan and [implicit-infer]'s special case. The fallback
  if the declared-claim normalisation looks too invasive.
- **The declared claim** (the revision above). Recommended.
- **The constructor design** (round 1). Sound, but 7 makes it worse to use
  than what exists.

## Revised decision list

1. `Iter` as a **declared** claim on the pass struct (`: Iter<self,
   T>(step)`), normalised into the type — the one new notion. Or the
   constructor design with `make_iter` and the field-type cost accepted.
2. `next` names the generic driver only; std steps are named per pass. Or
   the rank tweak.
3. No `Iterable` group in the first cut.
4. Which std producers become `iter fn`s (the unnamed ones), which stay
   structs (the stored ones).
