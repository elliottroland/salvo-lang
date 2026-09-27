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

---

# Round 3 (2026-09-26, later still): `for` over a step call, `iter fn` as a fn kind

Correction taken: `linear` is a keyword modifier on the struct, not an
always-on `Linear` qualifier, and the move away from that was deliberate
("always on" and "qualifier" are at odds). Round 2's declared-claim
recommendation loses its precedent and is withdrawn.

## The proposal, as read

1. **`for` loops over a call that returns `Emitted T | Finished`.** The base
   form is `for i in next(list_yield) { … }`: the subject is a *step call*,
   re-invoked each turn until `Finished`.
2. An **`iter fn`** is a fn kind whose declared return is `Emitted T |
   Finished` but whose call mints a compiler-built pass and drives its
   compiler-built `next`. `for i in iter(my_struct)` is the shorthand where
   the pass goes unnamed. Any name, any parameters; the hidden struct and
   `next` are named from the fn.
3. **Canonicity is declared on the struct**, as a group obligation:
   `: Yield<self, T>` makes `next` canonical (`for i in list_yield`);
   `params Iter<It, T> { iter fn iter(it: It) -> Emitted T | Finished }` and
   `: Iter<self, T>` make `iter` canonical (`for i in my_struct`). Declaring
   both is an error; extra non-canonical `next`s and `iter fn`s are fine.
4. **`iter` marks a fn type**, like `once` marks one: `?Iter<C, Int>` spreads to
   `?iter: iter (C) -> Emitted Int | Finished`, and `for i in collection`
   inside `total<C>(collection: C, ?Iter<C, Int>)` drives through it.

## Verdict

It works, and it is the best of the three rounds: it keeps today's pass model
whole (named structs, `Yield` obligation, `self` in the clause, drive in
place), gets the any-name `iter fn` and the honest signature, and needs
**no qualifier, no constructor, no type normalisation, no [qual-generic]
change, and no generic `next` driver** — so round 1's gaps 1, 2, 4, 5, 6 and
7 do not arise. Four rules need pinning down, one of which decides whether
`total(xs)` works for a `List`.

### Rule A — the step-call form: arguments are evaluated once

`for i in f(args)` where `f` returns `Emitted T | Finished` is new semantics
for `for`: the subject call is the loop's condition, re-evaluated per turn.
The rule that makes it coherent is **arguments evaluate once, before the
loop**, into places the per-turn call receives: `for i in next(p)` steps `p`
where it lives ([iter-drive-in-place] falls out for free), and `for i in
next(iter(xs))` binds the temporary to a hidden local first (a *linear*
temporary refused there, as today). Two consequences to state:

- The step fn must be **re-callable on the same arguments**: a parameter it
  consumes (`=> !x`) cannot be handed to it twice, so that is an error at the
  loop naming the parameter — the general form of today's "`next` must take
  `Mut It`".
- A **non-call** subject of type `Emitted T | Finished` (`for i in (if c {
  next(a) } else { next(b) })`) is an error: a union is not iterable, and
  only a call can be re-invoked. No ambiguity with the value forms, since no
  value of that type is otherwise a `for` subject.

This is a `while let Emitted i = f(args)` with once-evaluated arguments, and
it is what lets a **non-canonical** step or minter be driven at all
(`for i in step_back(p)`, `for i in reversed(xs)`) — the point of allowing
several per type.

### Rule B — what a bare call to an `iter fn` yields

`let p = iter(c)` is used today (Passes.md: "or hold the pass and drive it
yourself"; `take(p, 2)` then `take(p, 9)`). Under the proposal the declared
return is `Emitted T | Finished`, which is not what a call in expression
position produces. Decide:

- **(i)** an `iter fn` may be called only as a `for` subject or filled into
  an `iter`-kinded position — staged driving is lost; or
- **(ii)** a call in expression position **mints the pass** (today's
  behaviour), whose type is the hidden struct, inferred and unnameable; the
  hidden struct declares `: Yield<self, T>` and its `next` is named `next`
  (distinct by argument type, so several `iter fn`s over one subject do not
  collide), so `for i in p`, `for i in next(p)` and `map(p, f)` all work.

Recommendation: (ii). It is what exists, and it makes the `iter` arrow's
meaning exact: "a call yields a pass whose `next` yields this".

### Rule C — what fills an `iter (C) -> Emitted T | Finished` position

This is the decision that matters for std. Two readings:

- **Strict**: only an `iter fn` fits (kinds must match, so a plain `fn iter(x)
  -> Emitted T | Finished` — a *step* fn — does not, and neither does a plain
  fn returning a pass). Then `?Iter<List<Int>, Int>` has **no candidate**,
  because `core.list`'s `iter` is a plain `fn iter<T>(list: List<T>) -> Mut
  ListYield<T>` — and it must stay one, since `ListYield` has to be nameable
  (round 2, §8). `total(xs)` would fail for the most common container.
- **Semantic**: the position is filled by **any fn whose call yields a pass
  emitting `T`** — an `iter fn`, or a plain fn returning a `P` with `P :
  Yield<self, T>`. The `iter` arrow is then exactly "∃ pass type. `(C) -> Mut
  It` with `It : Yield<self, T>`", spelled without the existential, and an
  `iter fn` is one way of being that.

Recommendation: semantic, for both the implicit position and the `: Iter<self,
T>` obligation. It keeps `iter fn` pure sugar (a minter with an anonymous
pass), lets std's named passes fill `?Iter`, and makes the obligation
satisfiable by a delegating container without a wrapper pass (`fn iter(bag:
Bag) -> Mut ListYield<Int>`). The match rule for [group-obligation] is one
line: a return type `Emitted T | Finished` on an `iter fn` member is satisfied
by a return type `P` whose canonical `Yield` emits `T`.

### Rule D — lowering: the `iter` arrow is a pair plus a hidden generic

Neither backend has a fn type with an existential return, and boxing the pass
is refused. So an `iter`-kinded parameter lowers to **two** parameters and
**one hidden type variable** on the enclosing fn:

```
fn total<C>(collection: C, ?Iter<C, Int>) -> Int
// is, after desugaring,
fn total<C, __It>(collection: C, ?iter: (c: C) -> Mut __It, ?next: (it: Mut __It) -> Emitted Int | Finished) -> Int
```

That is **exactly the 2026-09-10 container-shaped combinator**
(`total<C, It>(c: C, ?iter: (c: C) -> Mut It, ?Yield<It, Int>)`), which
already compiles and runs on both backends — with `It` hidden. So the `iter`
arrow reduces to a shape that exists; what is new is the hiding:

- `__It` is inferred from the resolved minter's return (the [implicit-infer]
  repeated sweep does this today), never written; an explicit type-argument
  list at the call names the visible generics only.
- `for i in collection` in the body: the subject's type `C` has an
  `iter`-kinded implicit **named `iter`** over it → mint with it, drive with
  the paired `next`. `for i in reversed(collection)` with a second implicit
  `?reversed: iter (C) -> …` is Rule A applied to a fn value. Naming, not
  shape, decides canonicity in generic code too — consistent with the
  obligation.
- Calling `iter(collection)` in the body yields a `Mut __It`, and passing it
  to `map(…, ?Yield<It, T>)` forwards the paired `next` by name and type
  [implicit-forward] — so a generic source function can reach every std
  combinator.
- Rendered generics gain `__It` in both targets (Rust monomorphises it,
  Kotlin erases it); only the emitters see it.
- The same lowering serves an `iter`-kinded **positional** parameter (`fn
  drive<C>(c: C, f: iter (C) -> Emitted Int | Finished)`, called as `drive(xs,
  iter)` or `drive(xs, reversed)`); a lambda can never fill one (it has no
  pass). Allow it — it is the same machinery — or restrict to implicits for
  the first cut.

Rejected alternative: lower an `iter`-kinded value as a **push** function
(`(C, (T) -> ControlFlow)`), which needs no hidden type. `break` fits;
`return` from the body, effects in the body and staged driving do not, and
the emitters already refuse early return inside lambdas. The pair is the
honest shape.

### On "`iter` is a qualifier of the fn type"

It is a **kind**, not a qualifier: it changes what a call *means* (mint, then
drive) rather than bounding uses as `once` does, and it has no `qualifies`,
no `is`, no erasure story. `send fn` is the nearer precedent — a fn kind with
its own call semantics. Worth spelling it as a kind in the docs so nobody
looks for `is iter`.

### Smaller points

- **`: Iter<self, T>, Yield<self, T>` is an error** — agreed, with the
  framing "two answers to `for i in s`" (drive `s`, or mint from `s`) rather
  than "two canonical `next`s"; the `Iter` member's `next` is on the hidden
  struct, not on `s`. Today's [iter-resolve] ordering ("pass before `iter`")
  becomes unnecessary.
- **Multi-parameter `iter fn`s** (`iter fn chunks<T>(list: List<T>, size:
  Int)`) capture every non-subject argument into the pass — owned, or `proj`
  by the existing tier scan — and the hidden struct and `next` are keyed by
  the fn name (`__Pass_chunks_List`). Only the one-parameter `iter` named by
  the obligation is canonical.
- **The obligation check** gains a kind: `Yield`'s `fn next` member is not
  satisfied by an `iter fn next`, and (under strict Rule C) `Iter`'s member
  not by a plain step fn. Under semantic Rule C the second half is the
  "yields a pass emitting `T`" match instead.
- **Effects** on a step call or an `iter fn` are ordinary effects per turn,
  as today.
- **What stays from round 1**: hidden names keyed by fn (§9), the `state`
  annotation cost (§8), the sugar's return type reading as an element (§9,
  accepted).
- **What is not gained** relative to the qualifier design: the drive still
  finds `next` through the obligation rather than an identity in the type
  (not a scan — [iter-protocol] reads the declaration), and `T` is read off
  the resolved `next`/minter, which is [implicit-infer]'s ordinary first
  bullet. Neither is a cost worth a qualifier.

## Decisions for the user

1. Rule B: (ii) a bare call to an `iter fn` mints the pass. (Recommended.)
2. Rule C: semantic filling — any pass-minting fn fits an `iter` arrow and
   the `Iter` obligation. (Recommended; strict breaks `total(xs)`.)
3. Rule D: `iter`-kinded parameters positional as well as implicit, or
   implicits only in the first cut.
4. Spelling: `iter (C) -> Emitted T | Finished` as a fn kind, documented
   beside `send fn`.

---

# Round 4 (2026-09-26, later): `iter T` as the pass placeholder in the group

## The proposal, as read

```
params Iter<C, T> {
    fn iter(collection: C) -> iter T
    fn next(iterator: iter T) -> Emitted T | Finished
}
```

`iter T` is a type placeholder standing for "the pass this implementation
mints, emitting `T`" — one type across both members. An implementation is
either two fns with a concrete struct filling `iter T`, or a single `iter fn
iter(collection: C) -> Emitted T | Finished`, which generates both. On spread,
`iter T` is exactly the hidden `__It` of round 3's lowering.

## Verdict

Better than round 3, and it is the shape to build. The pair that round 3 hid
behind an `iter` arrow is now **visible in the group**, so the lowering is
literal: a `?Iter<C, T>` spread yields two implicits because every group
spread yields one per member [implicit-group], and the only new thing is a
placeholder type. `iter fn` becomes an *implementation strategy* for a
two-member obligation rather than a kind with its own call semantics, which
removes round 3's Rule C (what fills an arrow) and Rule D (the desugar into a
pair) as separate decisions. Rule A (the step-call form of `for`) and Rule B
(a bare `iter(c)` mints the pass) stand unchanged.

What `iter T` is: an **associated type with no name** — fixed per
implementation, opaque per spread. The nearest precedents are `self` in an
obligation (a placeholder resolved per declaring type) and `proj T` (a type
former written as a prefix). It is a type former, not a qualifier: no
`qualifies`, no `is iter`, and it erases to the concrete pass type.

## Rules to pin down

### 1. What `iter T` requires of the type that fills it

Two readings for the obligation `struct Bag : Iter<self, Int>` satisfied by
hand:

- **Shape**: any `P` with an `iter(bag) -> P` and a `next(p: Mut P) -> Emitted
  Int | Finished` overload. Then a `P` without `: Yield<self, Int>` satisfies
  `Iter` but is not itself a pass — `for x in iter(bag)` (value form) would
  have no canonical `next` to drive with. Two ways to be a pass, inconsistent
  with [iter-protocol]'s "the tie is declared, never inferred from a method
  name".
- **Declared** (recommended): `iter T` denotes **a type declaring `: Yield<self,
  T>`** — a std pass, a user pass, or the hidden struct of an `iter fn`. The
  `next` member is then satisfied by construction (it *is* `Yield`'s member),
  and `Iter`'s listing it is what makes the spread carry it.

Under the declared reading the two groups are one protocol stated twice, and
that is worth saying in `core/iterator.sv`: `Yield<It, T>` is the pass
obligation and the pass-shaped spread; `Iter<C, T>` is the source obligation
and the source-shaped spread, whose second member is `Yield`'s with `It =
iter T`. (If groups could spread inside groups — `params Iter<C, T> { fn
iter(collection: C) -> iter T ; ?Yield<iter T, T> }` — the duplication would
vanish, but that is machinery nobody else needs.)

A walking pass emits `proj T` (`ListYield<T> : Yield<self, proj T>`); the
match of `iter T` against it should accept the projection, as a reading
combinator's `?Yield<It, T>` does today.

### 2. `Mut`

The sketch has `fn next(iterator: iter T)`. Advancing mutates, and every
`next` in std is `(p: Mut P) => p: Mut`. Either write it —
`fn iter(collection: C) -> Mut iter T` and `fn next(iterator: Mut iter T) ->
Emitted T | Finished => iterator: Mut` — or define `iter T` as inherently a
mutable position. Recommend writing it, for parity with `Yield` and so
`let p = iter(c)` has a type the checker treats as `Mut` for the same reason
it does today.

### 3. Where `iter T` may be written

Inside a group's member signatures, and nowhere else in the first cut. In
the body of a fn that spreads `?Iter<C, T>`, `let p = iter(c)` has type `Mut
iter T` by inference and needs no annotation. Allowing it in a *signature*
(`fn f<C>(c: C, ?Iter<C, Int>) -> Mut iter Int`) would leak the hidden
generic into a public type — expressible today as `f<C, It>(…) -> Mut It`
with the generic visible — and is a possible later sugar, not a need.

### 4. Filling the pair: resolve `next` by the pass's declaration, not by scope

This is the one place the ordinary rule needs a variant. [implicit-resolve]
is local: a call gets the `next` visible where it is written. But the `next`
paired with `iter T` is the *canonical* one of whatever concrete pass `iter`
returned — and that pass may be a hidden struct from another module, whose
generated `next` is not in the caller's scope by name (today's [iter-fn]:
"the generated pass type inherits its visibility", which is about the type,
not about an unqualified `next` at a call site three modules away).

Recommendation: once `iter T` is bound to a concrete `P`, the `?next` implicit
is filled from **`P`'s own `Yield` obligation**, wherever the call is — the
rule `for` already follows ([iter-protocol]: "`for` reads the declaration").
`?iter` itself: `C`'s declared `Iter` obligation when it has one, else
ordinary resolution by name (which is how an intrinsic `List`, with no
obligation, reaches `core.list`'s exported `iter`), and `iter = other`
overrides either [implicit-override]. Overriding `next` alone should be
refused — it is not a choice the caller has, given `iter T` — which is a
`with`-style tie ([implicit-with]: `iter with next`) and could be written as
one in the group.

### 5. Lowering

Per `?Iter<C, T>` spread: one hidden type parameter and two implicit fn
parameters, exactly the 2026-09-10 shape (`total<C, It>(c: C, ?iter: (c: C)
-> Mut It, ?Yield<It, Int>)`) with `It` hidden. Hidden generics render in
both targets (Rust monomorphises, Kotlin erases); an explicit type-argument
list at a call names visible generics only. `for i in c` in the body mints
with the spread's `iter` and drives with the `next` **paired by `iter T`** —
structural, not by convention. `iter(c)` in the body yields a `Mut iter T`,
and handing it to `map(…, ?Yield<It, T>)` forwards the paired `next` by name
and type [implicit-forward]. Nothing here is new to the emitters.

### 6. Two sources in one fn

`fn merge<A, B, T>(a: A, b: B, ?Iter<A, T>, ?Iter<B, T>)` produces two
implicits named `iter` and two named `next`, at different types, and two
hidden generics. Calls in the body pick by argument type (`iter(a)` vs
`iter(b)`; `next(p)` by which `iter T` `p` has), and the call site fills each
by its own `C`. This should work as two `?Yield` spreads over different `It`s
would today, but nothing has exercised two same-named implicits in one
signature; it is the first thing to test.

### 7. Carried over

- **Rule A** (round 3): `for i in f(args)` re-invokes a step call per turn,
  arguments evaluated once; a consuming parameter is an error at the loop; a
  non-call subject of union type is an error.
- **Rule B**: a bare `iter(c)` mints the pass; its type is the hidden struct,
  inferred — or, in generic code, `Mut iter T`.
- `: Iter<self, T>, Yield<self, T>` is an error ("two answers to `for i in
  s`").
- Multi-parameter `iter fn`s (`iter fn chunks(list, size)`) capture
  non-subject arguments into the pass; hidden struct and `next` keyed by fn
  name; only the one-parameter `iter` named by the obligation is canonical.
- `state` annotations, hidden names in diagnostics, the sugar's return type
  reading as an element: unchanged costs.

## Decisions for the user

1. `iter T` denotes a type declaring `: Yield<self, T>` (declared reading).
2. `Mut` written explicitly in both members.
3. `iter T` legal only inside group members for the first cut.
4. `next` paired with `iter T` is filled from the pass's declaration; `iter`
   from `C`'s obligation, else by scope; `iter with next`.

---

# Round 5 (2026-09-26, later): what an `iter fn` call is typed as

Settled by the user: every `iter T` declares `: Yield<self, T>`; `iter with
next` on the group; and the obligation-fulfilling declarations are **imported
with the struct**, which covers round 4 §4 — the hidden `next` travels with
the hidden struct, so ordinary scope resolution finds it and the
"by declaration" variant is withdrawn. The `with` tie is what stops a caller
overriding `next` without `iter`. In the fully explicit form `f<C, It, T>(c:
C, ?iter: (C) -> Mut It, ?next: (Mut It) -> Emitted T | Finished)`, `iter T`
is simply what fills `It`.

## The question: the type of `iter(c)` when `iter` is an `iter fn`

The call yields the hidden struct — a real type with a real `: Yield<self, T>`
obligation and a generated `next`, so `for`, `next(p)`, and `map(p, f)` all
work by the existing rules. What it lacks is a **spelling**, and that is what
`iter T` supplies: **`Mut iter T` is the language's name for an anonymous pass
type.** One concept, three places:

| where | `iter T` is |
|---|---|
| a group member (`fn iter(collection: C) -> Mut iter T`) | the associated placeholder, one type across the members |
| a fn body spreading `?Iter<C, T>` | the opaque hidden generic `It` |
| the type of an `iter fn` call (`let p = iter(bag)`) | the concrete anonymous struct, displayed as `Mut iter Int` |

The declared return of the `iter fn` stays the step's (`Emitted T |
Finished`); the *call's* type is `Mut iter T`. That is the mismatch already
accepted in round 1 §9, now with a name for the other side.

Rules that follow:

- **Two anonymous passes never unify.** `iter Int` from `reversed` and `iter
  Int` from `iter` are distinct types that print alike; a mismatch
  diagnostic names the minting fn (`iter Int (from reversed)`), the way two
  same-named structs in two modules would need their module.
- **As a `let` annotation, `iter T` is a pattern** ([cmp-carry]'s rule for
  annotations): `let p: Mut iter Int = iter(bag)` asserts "a pass of Int" and
  the variable keeps the concrete type. A *named* pass fits it too
  (`let p: Mut iter Int = iter(xs)` over a `ListYield<Int>`) — the pattern
  means "declares `: Yield<self, Int>`", and nothing is widened.
- **In a signature or a struct field it is refused** in the first cut (an
  existential there is boxing, or a hidden generic that leaks); the
  remedy named is a visible generic (`<It>`) or the concrete pass name.
- **In a `state` block it can be allowed**, because a `state` field has an
  initializer to fill the pattern from — which closes round 1 §8 for the
  case that mattered:
  ```
  iter fn pairs<T>(list: List<T>) -> Emitted (T, T) | Finished {
      state { inner: Mut iter (proj T) = iter(list) }
      …
  }
  ```
  The field's type is the initializer's concrete type; the annotation is
  checked as a pattern. Worth doing in the same change, since it is the
  pattern rule applied to a third site rather than new machinery.

## Round 5a: should `iter T` imply `Mut`?

Yes. The argument is not just that every use happens to be `Mut`; it is
entailed. A type filling `iter T` declares `: Yield<self, T>`, whose `next`
takes `Mut self`, so the type must `canbe Mut` — and advancing is the *only*
operation a pass has, so a non-`Mut` pass has nothing it can do. A spelling
that allows a useless form is worse than one that does not.

So `iter T` is a type former whose expansion is `Mut <pass>`:

- `fn iter(collection: C) -> iter T` and `fn next(iterator: iter T) -> Emitted
  T | Finished => iterator: Mut` in the group; `let p = iter(bag)` has type
  `iter Int`; `state { inner: iter (proj T) = iter(list) }`.
- **`Mut iter T` is refused** as a duplicate ([qual-no-dup]), with the hint
  that `iter T` is already mutable — the same way `Mut Mut` is.
- **The deduction stays written.** `=> iterator: Mut` is a contract, not a
  type, and a bodiless member's clause is its whole contract; implying it
  would be the first implied deduction in the language. One word in one
  place in `core/iterator.sv`.
- The hidden struct of an `iter fn` gets `canbe Mut`, as it does today.

This is not an always-on qualifier on a value's type: `iter T` *denotes* a
`Mut`-qualified type, the way `T?` denotes `T | None`. Nothing is gained by a
value, and nothing can be dropped.

---

# Round 6 (2026-09-27): does this compose?

Short answer: the design does not stand in the way, and `iter T` plus the
`state` pattern make the **concrete** case read well; the **generic** case
(a lazy `map`/`filter` over *any* pass) needs two small additions, both
independent of the redesign and both absent today — which is why ROADMAP §16
is parked. Nothing here reopens §16's *decision* (composing functions, data
supplied last); it is the spelling that decision was waiting for.

## What "composition" means here

A stage is a pass that owns another pass and re-emits from it. Two shapes:

```
// Over a source: mint the inner pass in state. Works under round 5 as written.
iter fn evens<C>(c: C, ?Iter<C, Int>) -> Emitted Int | Finished {
    state { inner: iter Int = iter(c) }
    …                                   // drive `inner`, skip odd ones
}

// Over a pass: take the position itself.
iter fn doubled(it: iter Int) -> Emitted Int | Finished {
    let x = next(it)
    when x {
        is Finished { return finished() }
        is Emitted  { return emitted(x * 2) }
    }
}

for n in doubled(evens(xs)) { … }       // xs → evens → doubled, one element at a time
```

The first stage is §16's "pipeline of functions": nothing is minted until
`evens(xs)` is called, per drive, so replay is free. The second is what makes
stages chain.

## What the second shape needs

**1. An `iter fn` parameter may be a pass, moved in.** Today the subject is
*borrowed* and **read-only** ([iter-fn]: "a `Mut` subject is refused —
advancing never writes through it"), so an `iter fn` cannot drive what it is
handed. Rule: a parameter of type `iter T` (or any declared pass type) is
**consumed** into the hidden struct (`=> !it`) and drivable in the body. A
pass is a position; the stage owns the position. Non-pass parameters keep
today's borrow-or-copy tiers.

**2. `iter T` in a parameter is a hoisted generic.** Round 5 refused `iter T`
in signatures because in a *return* position it is an existential. In a
*parameter* position it is universal — "any pass emitting `T`" — which is a
generic, monomorphisable, and the same hoisting `?Iter<C, T>` already does:
`doubled(it: iter Int)` is `doubled<It>(it: It, ?Yield<It, Int>)` with `It`
hidden. Refine the rule: **parameter → hoisted generic with its `?Yield`
spread; return of a plain fn → refused; struct field → refused; `state`
field → pattern; `iter fn` call type → concrete.**

Two occurrences in one signature (`chain(a: iter T, b: iter T)`) must be
**two** generics — `chain(iter(xs), reversed(xs))` is the point — where
inside a *group* `iter T` is one type across the members. State the
difference: in a group it names the group's pass; in a fn each occurrence is
its own. A stage that needs two parameters of the *same* pass type writes
`<It>`.

**3. The generated `next` carries the forwarded implicit.** `doubled`'s hidden
`__next` calls `next(p.it)` on an opaque `It`, so it needs `?Yield<It, Int>`
— forwarded from whoever drives it, which a `for` does like any call
[implicit-forward]. The [group-obligation] match ("parameter types equal,
positionally") must then **ignore trailing implicit parameters** when
checking the hidden struct's `: Yield<self, Int>`: implicits are the
callee's business, resolved at the call, not part of the member's shape. The
alternative — storing the inner `next` as a fn-valued field, minted with the
forwarded implicit — is what [rs-stored-implicit] already lowers for keyed
containers, and avoids touching the obligation check; the implicit-on-`next`
form is cleaner and keeps the pass a plain struct. Either works; the first
is recommended.

This is the same gap a **hand-written** generic stage hits today (`struct
MapPass<It, T, U> : Yield<self, U>` with a `next(p, ?Yield<It, T>)`), so it
is a §16 prerequisite rather than a cost of this redesign.

## What falls out

- **Pipelines as values.** `let pipeline = c -> doubled(evens(c))` infers a
  lambda whose return is a concrete anonymous pass; its type cannot be
  *written*, but it can be passed where a lambda can — `total(xs, iter =
  pipeline)` fills `?Iter<List<Int>, Int>`'s `iter` by override
  [implicit-override]. Nothing stores a source until it is called.
- **`zip`, `chain`, `take`** are `iter fn`s with pass parameters and no
  hand-written struct: `iter fn zip<A, B>(a: iter A, b: iter B) -> Emitted (A,
  B) | Finished`, two hoisted generics and two forwarded `next`s — round 4 §6's
  same-named-implicits test, again first.
- **Borrowed elements through a stage.** `evens` over a walking pass receives
  `proj T` and re-emits it; whether the stage's `Emitted` is `proj` of the
  inner pass (which borrows the list) is the `holds proj(it)` machinery
  `filter` already uses, not an iteration question.

## What does not fall out

- **A stage over a linear pass** (`Lines`): the hidden struct owns a linear
  value, so it must itself be `linear` with a generated discharger, and the
  desugar does none of that. This is §16's "L8 casualty" question; the
  redesign neither helps nor hurts it, and it should stay parked until the
  above lands.
- **Sendability** of a composed pass — §16's last question — is unchanged.

## Decisions this adds

1. An `iter fn` parameter of pass type is consumed and drivable.
2. `iter T` in a parameter position is a hoisted generic (fresh per
   occurrence in a fn; shared within a group).
3. The obligation match ignores trailing implicits (or: store the inner
   `next` as a field).

## Round 6a: `iter T` as the return type of a plain fn with a body

The case: `core.range`'s pattern — one fully specified constructor and
overloads that fill in defaults and delegate:

```
export iter fn range(start: Int, end: Int, step: Int) -> Emitted Int | Finished {
    state { i: Int = start }
    …
}
export fn range(start: Int, end: Int) -> iter Int { return range(start, end, step_for(start, end)) }
export fn range(end: Int) -> iter Int { return range(0, end) }
```

Round 5 refused `iter T` in a return position as an existential. That was
too coarse: the existential is only a problem where the compiler does **not**
know the concrete type — a bodiless declaration, a fn *type*, a field. Here
the body returns `range(start, end, step)`, whose type is one concrete
anonymous struct, so the annotation can be a **pattern filled from the
body's returns**, exactly as a `let` annotation is — and the fn's real return
type, the one callers unify against and `for` drives, is the concrete one.
Rust's `-> impl Iterator<Item = i32>` is the same device: opaque to the
reader, concrete to the compiler. There is precedent inside Salvo for
reading part of a return type off the body already: a return's *lend* is
inferred [proj-infer].

Refined rule for `iter T` in return position:

- **A fn with a body**: a pattern. Every return path must produce the *same*
  concrete pass type; two different anonymous passes (`if c { range(3) }
  else { evens(xs) }`) is an error naming both origins, with the remedy
  "fold the case into one `iter fn`, or name the pass". Callers see the
  concrete type; hover and docs print `iter Int`.
- **A group member**: the associated placeholder, filled per implementation
  (round 4). Same reading, one level up.
- **A bodiless fn** (`intrinsic`), an **effect member** (implemented per
  handler, dispatched through a trait, so no single concrete type exists),
  a **fn type** (lambda or fn-typed parameter), a **struct field**: refused,
  with the remedy "name the pass or make the position generic".

Two consequences for `core.range`: the `Range` struct disappears (the
`iter fn`'s three scalar parameters are the state), and the [mod-export]
note — `Range` and `next` exported so a private pass would not strand the
constructors — becomes moot, since the hidden struct and its `next` travel
with `range`. The ROADMAP item about `core.range` being emitted into every
program because every program uses the name `next` should shrink with it:
nothing names that `next` any more.

One thing to check when building: inference from the body means the return
type is known only after the body is checked, so a recursive `iter Int` fn
(or a cycle of them) needs either an "opaque until checked" placeholder or a
refusal naming the cycle. The delegating overloads are not recursive; a
refusal is fine for the first cut.

---

# Terminology (user decision 2026-09-27): "pass" → "iterator struct"

"Pass" is not specific enough; the type a `next` advances is an **iterator
struct**. `iter T` then reads as it should — "an iterator of `T`" — and the
group names line up (`Iter`, `iter fn`, `iter T`, iterator struct).

Sweep when the redesign lands (the term is everywhere the protocol is):

- `docs/language/Passes.md` (title and page), `Iteration.md`,
  `Implicit-Parameters.md`, `Dependent-Qualifiers.md`, `Linear-Types.md`,
  `Concurrency.md` where they say "pass"; `README.md`'s iteration bullet;
  `tools/sync-wiki.sh` afterwards.
- LANGUAGE_SPEC.md prose, and the labels that carry the word: `[iter-pass]`,
  `[seq-pass]` — renaming a label means `grep -rn` for every reference in
  code and tests, per AGENTS.md.
- Code names: `PassDriver`, `for_drivers`, `expand_pass_fns`, `__Pass_…`
  (the hidden struct prefix — `__Iter_…` or `__Iterator_…`), and the
  diagnostics that say "pass" ("a `for` cannot consume a linear pass", "is
  not iterable … declare `: Yield<self, T>` on it to make it a pass").
- std comments (`core/iterator.sv`, `list.sv`, `array.sv`, `set.sv`,
  `map.sv`, `string.sv`, `bytes.sv`, `fs`'s `Lines`), and the names
  `*Yield` if they are to follow (`ListYield` → `ListIter`? — a separate
  call; the current names are fine and the user has not asked).
- ROADMAP.md §16 and COMPLETED.md's prose are not archives (AGENTS.md):
  update the term there too, with the decision logged.

A *value* of an iterator struct is **an iterator** (user, 2026-09-27) —
matches `iter T`, and "pass" leaves the vocabulary entirely.

---

# Round 7 (2026-09-27): linearity

Two different questions, and the guess is right on both.

## Linear elements: yes, as today

`emitted<T canbe linear>` already tags a linear element, and a `next` hands
its element over by value, so an iterator of linear things works now. `iter
T` adds nothing and needs nothing: the opt-in lives on the type *parameter*
(`iter fn drain_handles<T canbe linear>(…) -> Emitted T | Finished`), where
`canbe linear` is already written, and `iter T` with such a `T` is just the
hidden `It` over that `T`. In `?Iter<C, T>` and `?Yield<It, T>` spreads the
same holds.

## A linear iterator: `iter T` cannot say it, deliberately

`iter T` denotes "`Mut`, declares `: Yield<self, T>`", and in a *generic*
position (a parameter, a spread) it is a hidden `It` with **no `canbe
linear`** and no spelling to add one. That is the right restriction, not a
gap: a linear iterator has to be **discharged**, and generic code can only
discharge an opaque `It` if someone hands it the discharger. The language
already has the honest shape for that — `drain<It canbe linear>(it: Mut It,
end: (x: It) -> None, ?Yield<It, Int>) => !it =>[end] !x` [linear-generics],
a consuming callback the caller fills with `close` or `drop`. `iter T` is
sugar for the *common* case, and the common case has no discharger to
thread. So:

- `doubled(it: iter Int)` refuses a linear argument, naming the explicit form.
- `total<C>(c: C, ?Iter<C, Int>)` refuses at resolution when `C`'s `iter`
  mints a linear iterator ("this position cannot discharge it"), for the same
  reason.

In a **concrete** position — the type of an `iter fn` call, a return or
`let` pattern, a `state` field — linearity flows with the concrete type.
`iter Str` may well *be* linear; the display does not say so, the checker
knows, and the ordinary rules apply: a `for` refuses a linear *temporary*
(bind it first) and the owner discharges after the loop.

## What the redesign makes possible: a generated discharger

The interesting case is an **`iter fn` that holds a linear value** — in its
`state`, or as a consumed iterator parameter of *concrete* type:

```
iter fn lines(path: Str) [Fs] -> Emitted Str | Finished {
    state { s: InStream = open_read(path) }
    …
}
let it = lines("a.txt")     // an `iter Str`, linear because its state is
for line in it { … }
close(it)                    // generated: closes `s`
```

Today this is refused at the struct: a field of linear type makes the struct
a resource, so it must be `linear struct` with a discharger in its own file
[linear-composite] [linear-group], and the desugar writes neither. It can:
when a hidden struct holds a linear value whose type is **concrete** and
whose discharge set has **exactly one** member, generate `linear struct` and
`fn close(p: __Iter_…) => !p { close(p.s) }` beside it (named after the held
type's discharger, exported with the `iter fn`). Two held linear values, or a
discharge set with several members (`stop`/`join`), or a linear value of
*hidden* type (`state { inner: iter Str = iter(c) }` under a `?Iter<C, Str>`
spread): refuse at the declaration — "write the iterator struct by hand" —
which is [linear-composite]'s own "the casualty that remains" (the composed
linear pass) stated with its boundary.

That settles ROADMAP §16's L8 question as a **partial yes**: a *concrete*
stage over a *concrete* linear source (`lines`, `upper_lines(path)` holding
an `InStream`) is one `iter fn` with a generated `close`; a *generic* stage
over a possibly-linear source stays the explicit `<It canbe linear>` +
callback form, because nothing else could know how to close it. The
pipeline-of-functions hope in §16 — "a pipeline holding only functions stores
no source" — is true of the *pipeline* and false of the *iterator it mints*,
which is where the obligation lives and where it is discharged.

## Round 7a: no generated discharger (user decision 2026-09-27)

The generated `close` is **rejected**: a discharger would have to be a member
of the group for the model to stay consistent, and it is not one. An `iter fn`
whose state would hold a linear value, or that would consume a linear
iterator, is refused at the declaration with "write the iterator struct by
hand". Round 7's "what the redesign makes possible" section is superseded by
this; the analysis of `iter T` and linearity above it stands.

# Summary: every use of `iter T`, and what it stands for

`iter T` is one thing everywhere — **"a `Mut` type declaring `: Yield<self,
T>`"** — read in whichever way the position allows: as a *placeholder* in a
group, as a *hidden generic* in a signature, as a *pattern* where there is a
value to fill it from, and as the *concrete anonymous struct* that an `iter
fn` mints. Each row gives the sugar and the form a user writes when the
sugar does not apply (a linear iterator, a struct that must be named, two
parameters of the same iterator type).

### 1. Group member — the associated placeholder

```
params Iter<C, T> => iter with next {
    fn iter(collection: C) -> iter T
    fn next(iterator: iter T) -> Emitted T | Finished => iterator: Mut
}
```
Desugared: the iterator struct is a **group parameter**.
```
params Iter<C, It, T> => iter with next {
    fn iter(collection: C) -> Mut It
    fn next(iterator: Mut It) -> Emitted T | Finished => iterator: Mut
}
```
One `iter T` per group: both members name the same `It`.

### 2. Obligation on a struct

```
struct Bag : Iter<self, Int>
```
Desugared: name the iterator struct in the obligation, and declare it.
```
struct Bag : Iter<self, BagIter, Int>
struct BagIter : Yield<self, Int> canbe Mut { … }
fn next(p: Mut BagIter) -> Emitted Int | Finished => p: Mut { … }
fn iter(bag: Bag) -> Mut BagIter { … }
```
Satisfied either by those three declarations or by one `iter fn iter(bag:
Bag) -> Emitted Int | Finished` (row 8).

### 3. Spread in a signature — a hidden generic

```
fn total<C>(c: C, ?Iter<C, Int>) -> Int
```
Desugared: the 2026-09-10 container-shaped combinator, `It` visible.
```
fn total<C, It>(c: C, ?iter: (c: C) -> Mut It, ?Yield<It, Int>) -> Int => iter with next
```
Inside the body `let p = iter(c)` is `Mut It` either way. A linear iterator
does not fit this position under either spelling; the linear-capable form is
row 10.

### 4. Parameter of a fn or `iter fn` — a hidden generic, fresh per occurrence

```
iter fn doubled(it: iter Int) -> Emitted Int | Finished
fn first_two(it: iter Int) -> Int
```
Desugared:
```
iter fn doubled<It>(it: Mut It, ?Yield<It, Int>) -> Emitted Int | Finished   // `it` consumed into the state
fn first_two<It>(it: Mut It, ?Yield<It, Int>) -> Int => it: Mut               // driven in place
```
`chain(a: iter T, b: iter T)` is `chain<A, B>(a: Mut A, b: Mut B, ?Yield<A, T>,
?Yield<B, T>)` — **two** generics. Two parameters that must be the *same*
iterator struct are written with one `<It>`; `iter T` cannot say that.

### 5. The type of an `iter fn` call — the concrete anonymous struct

```
let p = iter(bag)        // p : iter Int
```
Desugared: the hand-written struct of row 2, and `let p = iter(bag)` typed
`Mut BagIter`. The anonymous struct is real (it has the obligation and the
`next`) and unnameable; `iter Int` is how it prints.

### 6. `let` annotation — a pattern

```
let p: iter Int = iter(bag)
```
Desugared: `let p: Mut BagIter = iter(bag)`, or no annotation. A named
iterator struct fits the pattern too (`let p: iter Int = iter(xs)` over a
`ListIter<Int>`); nothing is widened.

### 7. `state` field — a pattern (it has an initializer)

```
iter fn evens<C>(c: C, ?Iter<C, Int>) -> Emitted Int | Finished {
    state { inner: iter Int = iter(c) }
    …
}
```
Desugared: with a concrete source, name the struct — `state { inner: Mut
ListIter<Int> = iter(xs) }`; with a generic source, the explicit `<It>` of
row 3 and `state { inner: Mut It = iter(c) }`.

### 8. `iter fn` — the minter, the struct and the `next` in one declaration

```
iter fn range(start: Int, end: Int, step: Int) -> Emitted Int | Finished {
    state { i: Int = start }
    …
}
```
Desugared:
```
struct RangeIter : Yield<self, Int> canbe Mut { start: Int, end: Int, step: Int, i: Int }
fn next(p: Mut RangeIter) -> Emitted Int | Finished => p: Mut { … }   // the body, fields via `p.`
fn range(start: Int, end: Int, step: Int) -> Mut RangeIter {
    return Mut RangeIter { start: start, end: end, step: step, i: start }
}
```
Non-scalar parameters are `proj` or owned by the existing tier scan; a
parameter of iterator type is owned (row 4). Any name, any arity; only the
one named by an `Iter` obligation is canonical for `for x in c`.

### 9. Return type of a fn with a body — a pattern

```
fn range(end: Int) -> iter Int { return range(0, end) }
```
Desugared: `fn range(end: Int) -> Mut RangeIter { … }` over the hand-written
struct of row 8. All return paths must produce one concrete iterator struct.

### 10. Linear — always by hand

There is no `iter T` spelling for a linear iterator, and no generated
discharger. A concrete one is the written-out struct with its death in its
own file:
```
linear struct Lines : Yield<self, Str> canbe Mut { s: InStream, … }
fn next(l: Mut Lines) [Fs] -> Emitted Str | Finished => l: Mut { … }
fn lines(path: Str) [Fs] -> Mut Lines { … }
fn close(l: Lines) [Fs] => !l { close(l.s) }
```
Generic code that may receive one takes the discharger as a callback
[linear-generics]:
```
fn drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Yield<It, Int>) -> Int => !it =>[end] !x
```
An `iter fn` whose state would hold a linear value, or whose iterator
parameter is linear, is refused at the declaration naming this row.

### Refused positions (no sugar; the replacement is named)

| position | why | write instead |
|---|---|---|
| return type of a bodiless fn (`intrinsic`) | no body to fill the pattern | the struct's name |
| effect member return | implemented per handler, one trait type needed | the struct's name |
| fn *type* (lambda, fn-typed parameter) | a return the callee chooses, existential | `<It>` on the enclosing fn |
| struct field | existential storage is boxing | the struct's name, or `<It>` on the struct |
| `Mut iter T` | `Mut` is implied | `iter T` |

---

# Plan step (user decision 2026-09-27): the summary goes into the docs

The "every use of `iter T`" table above becomes a section of the iteration
docs when the redesign lands — `docs/language/Iteration.md`, or the page that
replaces `Passes.md` under the new name — rather than staying here. The
framing the user wants is the one the table already has: **every use of
`iter T` lowers to something the user could write explicitly**, so the page
shows the sugar and the written-out form side by side, row by row, and the
written-out form is the specification. LANGUAGE_SPEC.md gets the rule
(`[iter-type]` or similar) with the four readings — placeholder, hidden
generic, pattern, concrete anonymous struct — and points at the page for the
table; the page carries the examples.

Order within the redesign's plan: after the spec rules and before the std
rewrite, so the std rewrite can be checked against the page. Then
`tools/sync-wiki.sh`.

---

# Clarification to row 10 (user, 2026-09-27): linear *elements* are fine

Row 10 is about the **iterator struct** being linear. An iterator of linear
*elements* — `iter T` with `T canbe linear` — is allowed, because nothing
about it needs a discharger: `next` hands each element over by value, the
obligation travels with the element to whoever receives it, and the hidden
struct holds no linear value between turns (the element is minted and handed
out in one `next`). `emitted<T canbe linear>` already exists for exactly this.

So the boundary is: **the hidden struct may never be linear** — no linear
`state` field, no consumed linear iterator parameter, no `proj` of a linear
subject that would make the struct a resource — and everything else `iter T`
does is available with `T canbe linear`:

```
iter fn handles<T canbe linear>(pool: Pool<T>) -> Emitted T | Finished { … }   // fine: each T leaves on its turn
fn drain<T canbe linear>(it: iter T, end: (x: T) -> None) -> Int =>[end] !x    // fine: iterator not linear, elements are
```

Row 10's hand-written form is for the struct that owns a resource (`Lines`
over an `InStream`); the generic form with the `end` callback is for code
that may receive such a struct. Both unchanged.
