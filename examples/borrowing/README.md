# Borrowing: projections, handles, and alias groups

Salvo has no `&`, no `&mut`, no lifetimes, and no explicit ownership. What it
has is one qualifier — **`proj`** — that says "this value is borrowed from
somewhere", and three answers to the question a mutable borrow raises:

1. say nothing, and hold **one handle at a time**;
2. **prove** two handles name different storage (`NotEq`), and use both;
3. **declare** that two may be the same (`canbe`), and let the callee cope.

This program is those three, plus what a read projection is and what a
mutation's *field* granularity buys. It ends up exercising four shapes the
natural Rust translation does not compile at all — they are listed below with
the rustc error each produces — and they run here because the Rust backend
renders a mutable handle as a **position** (a container plus an index,
re-materialized at each use) rather than as a reference.

Run it:

```bash
cargo run -- run --backend rust   --src examples/borrowing/salvo
cargo run -- run --backend kotlin --src examples/borrowing/salvo
```

## What it shows, section by section

1. **A read projection is a borrow.** `named(roster, "Ada")` searches and hands
   back what it found as `proj(roster) Fighter` — the element itself, not a
   copy, and the list keeps owning it. Returning the *parameter* would be a
   different sentence: a return value is owned by the caller, so returning a
   parameter moves it. Three further shapes of the same fact: a projection may
   be read anywhere but **moving** one needs `copy` (hence
   `add(names, copy(ada.name))` — the copy is written, because a copy never
   happens without the program asking), a **view** struct (`Window`) is an owned
   value that *holds* a borrow in a `proj` field and may not outlive it, and
   `filter` answers `Mut List<proj Fighter>` — a list of borrows, built without
   copying an element.
2. **A mutable projection is a handle.** Element mutability belongs to the
   **element type**, not the container handle: `Mut List<T>` is a mutable
   *container* (`add`, `remove_at`, `swap`), `List<Mut T>` is a container of
   mutable *elements*. So `get(squad, 0)!` at `T = Mut Fighter` answers
   `(proj(squad) Mut Fighter)?` — and that `Mut` is the permission to write
   through it. The section mints one where it is used, binds one and holds it
   across a **read** of its own container, lends one out of a **search loop**
   (`wounded`), and lends one from a *generic* function whose accessor the caller
   supplies (`rally_at`, over `params Locate`).
3. **Two handles, proven apart.** `duel` mutates both parameters and says
   nothing about them coinciding, so a caller owes a proof: `j is NotEq(i)` —
   `j != i`, bound to `i`'s identity. With it, two handles into one list coexist,
   a write through one leaves the other standing, and one call may take both.
   `update` and `update2` are std's wrappers for the common shapes, and both
   promise `preserve Idx`, so the reads after them stay *total* — an in-place
   write moves no boundary.
4. **An alias group.** `strike` declares `=> a canbe d`: it *means* to accept
   two handles that might be one object. No caller needs a proof, and the
   self-strike (`strike(get(squad, i), get(squad, i))`) is legal — one fighter
   spending its own energy on itself. `canbe` is symmetric and non-transitive,
   and written-only: aliasability changes what a caller may pass, so it stays in
   the signature. `rotate` is the **anchored** form —
   `=> from|to canbe in squad.members` — where each parameter may be an
   *element of a named container*: two parameters anchored in the same path may
   therefore coincide, so the n-way case costs one entry per parameter instead
   of one per pair. It also licenses what a plain `canbe` cannot, since the
   anchor is a parameter: handles passed **beside the container they came
   from**.
5. **Which field a call touches.** `spend`'s clause says `=> camp.supplies: Mut`
   — the mutation lands on that field alone, so `let banners = camp.banners`
   survives the call. `hoist` then mutates the *contents* of `banners`
   (`=> camp.banners: Mut`), which leaves that list's storage identity intact,
   so the handle sees the new banner. The other half of the distinction is
   `=> !camp.banners`: the field is **replaced**, and every handle to it falls.
   Where nothing is written, the field set is inferred from the body — the
   precision is the default, not a reward for annotating.

## The four shapes rustc refuses

Each is marked in the source with the error code it would produce. These are
not borrow-checker weaknesses that a smarter analysis lifts: they are the
exclusion rule itself, so they hold whatever rustc's version (checked against
`rustc 1.98.0`).

**§2 — a handle held across a read of its container.**

```rust
let boss = &mut squad[0];
boss.hp += 1;
let n = squad.len() as i32;   // error[E0502]: cannot borrow `squad` as immutable
boss.hp += n;                 //   because it is also borrowed as mutable
```

Salvo allows it because a *read* of a container cannot invalidate a handle into
it, and the position rendering is what lets rustc agree: the handle becomes
`let __h2 = (0) as usize;` and each use re-indexes `squad[__h2]`.

**§3 — two element handles of one container, in one call.**

```rust
duel(&mut squad[i], &mut squad[j]);
// error[E0499]: cannot borrow `squad` as mutable more than once at a time
//   help: use `.split_at_mut(position)` to obtain two mutable
//         non-overlapping sub-slices
```

The proof is what buys it in Salvo, and rustc's own suggestion is what the
backend emits: `salvo_pair_mut(&mut squad[..], __h4, __h5)`, a `split_at_mut`.

**§4 — two handles that may be the same object.**

```rust
strike(&mut squad[i], &mut squad[i]);
// error[E0499]: cannot borrow `squad` as mutable more than once at a time
```

The same error, and here no restructuring of the *signature* helps: `&mut`
cannot express "these may coincide", so a Rust programmer splits the call site
in two and writes the aliasing case as a second body —

```rust
if i == j {
    let f = &mut squad[i]; f.energy -= 1; f.hp -= 2;   // the aliasing case
} else {
    let (lo, hi) = if i < j { (i, j) } else { (j, i) };
    let (a, b) = squad.split_at_mut(hi);               // …and the plumbing
    if i < j { strike(&mut a[lo], &mut b[0]); } else { strike(&mut b[0], &mut a[lo]); }
}
```

`=> a canbe d` is one function for both cases, and the emission is exact rather
than defensive: the covered positions become one shared anchor plus a position
each, so when they alias they index the same storage.

**§5 — a handle into one field, across a call that mutates another.**

```rust
let banners = &camp.banners;   // a handle into one field
spend(&mut camp, 3);           // error[E0502]: cannot borrow `camp` as mutable
println!("{}", banners.len()); //   because it is also borrowed as immutable
```

Rust splits borrows by field *within* a function, never across a call: `spend`
takes the whole struct. The deduction clause is how Salvo says where the write
lands, and a surviving handle renders as a **virtual place** — the path
re-materialized per use, sound precisely because survival means that field was
untouched.

Two shapes this does *not* claim, checked rather than assumed: a function that
searches a collection and returns a mutable reference to what it found compiles
in Rust (`fn wounded(squad: &mut Vec<Fighter>) -> Option<&mut Fighter>` with an
`iter_mut` loop, and the best-so-far variant too), and a caller-supplied
lending accessor is expressible with a higher-ranked bound
(`F: for<'a> Fn(&'a mut Vec<Fighter>, usize) -> Option<&'a mut Fighter>`). What
Rust refuses is *using* either result alongside the container, which is §2.

## What to look for in the generated code

- **A mutable handle is a position, not a reference.** `let boss = get(squad, 0)!`
  becomes `let __h2 = (0) as usize;` (plus the bounds check `!` asked for), and
  every use re-indexes `squad[__h2]`. That is what makes the `size(squad)` in
  the middle legal.
- **A lender is emitted twice.** `wounded` gets its read face
  (`-> Option<&Fighter>`) *and* `wounded__loc(squad: &Vec<Fighter>) -> Option<usize>`,
  where the `for` became an indexed loop and `return f` became `return Some(__li0)`.
  The mutating call site takes the locator:
  `heal({ let __l3 = wounded__loc(&squad).expect(…); &mut squad[__l3] })`.
- **A covered pair has a different signature.** `strike` is
  `strike(__anchor: &mut Vec<Fighter>, __c0: usize, __c1: usize)` — one borrow,
  two positions — while the *proven* pair keeps two `&mut Fighter` parameters
  (`duel`) and is fed by a `split_at_mut`. The anchored form needs no
  synthesized anchor at all, because its anchor is a parameter:
  `rotate(squad: &mut Squad, __c1: usize, __c2: usize)`, indexing
  `squad.members[__c1]`.
- **A view carries a lifetime, and only a view does.** `Window<'s>` with
  `roster: &'s Vec<Fighter>`, `window(roster: &Vec<Fighter>) -> Window<'_>`,
  and `peek<'s>(w: &Window<'s>) -> Option<&'s Fighter>` — the three signatures
  that mention one, all because the `proj` field made the borrow part of the
  type. Nothing else in the file does: the deduction analysis decided moves and
  borrows without needing to name a lifetime.
- **`filter` builds a list of borrows.** Its instantiation is literal about it:
  `filter::<ListYield<Fighter>, &Fighter>(&mut pass, …)` — the element type
  *is* `&Fighter`, so no element was copied to build the result.
- **The surviving field handle disappeared.** `let banners = camp.banners` has
  no binding in the Rust at all: its one use reads `camp.banners` where it
  stands — a *virtual place* — which is exactly the object the Kotlin binding
  holds. That equality is why the rule is allowed to spare the handle.
- **Kotlin needed none of it.** `kotlin/main.kt` has no positions and no
  locators, keeps `val banners = camp.banners` as an ordinary binding, and the
  one `copy` in the source vanished (`names.add(ada.name)` — duplicating a
  reference to an immutable string *is* a copy). Objects are references and
  aliasing is native; every rule on this page exists to keep Rust honest while
  both backends print the same bytes.
