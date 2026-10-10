# Borrowing: projections, handles, and shared containers

Salvo has no `&`, no `&mut`, no lifetimes, and no explicit ownership. What it
has is two qualifiers — **`proj`**, which says "this value is borrowed from
somewhere" and is read-only, and **`ref(c)`**, a handle into container `c`,
mutable when it carries `Mut` — and three answers to the question a mutable
borrow raises:

1. say nothing, and hold **one handle at a time**;
2. **prove** two handles name different storage (`NotEq`), and use both;
3. **name the container** two handles share (`ref(c)`), and let the callee
   cope with their being the same element.

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
2. **A `ref` is a handle.** Element mutability belongs to the **element
   type**, not the container handle: `Mut List<T>` is a mutable *container*
   (`add`, `remove_at`, `swap`), `List<Mut T>` is a container of mutable
   *elements*. `get` reads and `at` handles: `get(squad, 0)!` answers a
   read-only `proj(squad) Mut Fighter`, while `at(squad, 0)!` answers
   `ref(squad) Mut Fighter` — the handle, whose `Mut` is the permission to
   write through it. The section mints one where it is used, binds one and
   holds it across a **read** of its own container, lends one out of a
   **search loop** (`wounded`, `-> (ref(squad) Mut Fighter)?`), and lends one
   from a *generic* function whose accessor the caller supplies (`rally_at`,
   over `params Ref`).
3. **Two handles, proven apart.** `duel` mutates both parameters and says
   nothing about them coinciding, so a caller owes a proof: `j is NotEq(i)` —
   `j != i`, bound to `i`'s identity. With it, two handles into one list coexist,
   a write through one leaves the other standing, and one call may take both.
   `update` and `update2` are std's wrappers for the common shapes, and both
   promise `preserve Idx`, so the reads after them stay *total* — an in-place
   write moves no boundary.
4. **Handles that share a container.** `strike(c: List<Mut Fighter>, a:
   ref(c) Mut Fighter, d: ref(c) Mut Fighter)` *means* to accept two handles
   that might be one element: both are positions in `c`. No caller needs a
   proof, and the self-strike (`strike(squad, at(squad, i)!, at(squad, i)!)`)
   is legal — one fighter spending its own energy on itself. The caller must
   hand in handles minted from the container it passes as `c`; a handle into
   another list is refused. Any number of parameters may name one container —
   one `ref(members)` each, not one relation per pair — and the container may
   be a field path at the call: `rotate(team.members, at(team.members, i)!,
   at(team.members, j)!)` passes handles **beside the container they came
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
`let __h2: usize = …at__loc(&squad, 0i32).expect(…);` and each use re-indexes `squad[__h2]`.

**§3 — two element handles of one container, in one call.**

```rust
duel(&mut squad[i], &mut squad[j]);
// error[E0499]: cannot borrow `squad` as mutable more than once at a time
//   help: use `.split_at_mut(position)` to obtain two mutable
//         non-overlapping sub-slices
```

The proof is what buys it in Salvo, and rustc's own suggestion is what the
backend emits: `salvo_pair_mut(&mut squad[..], __h7, __h8)`, a `split_at_mut`.

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

`ref(c)` is one function for both cases, and the emission is exact rather
than defensive: the container is passed once and each handle as a position in
it, so when they alias they index the same storage.

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

- **A mutable handle is a position, not a reference.** `let boss = at(squad, 0)!`
  becomes `let __h2: usize = …__loc(&squad, 0i32).expect(…);` (the bounds check `!` asked for), and
  every use re-indexes `squad[__h2]`. That is what makes the `size(squad)` in
  the middle legal.
- **A lender is emitted twice.** `wounded` gets its read face
  (`-> Option<&Fighter>`) *and* `wounded__loc(squad: &Vec<Fighter>) -> Option<usize>`,
  where the `for` became an indexed loop and `return f` became `return Some(__li1)`.
  The mutating call site takes the locator:
  `match at(&*squad, l) { Some(__l1) => Some(&mut squad[__l1]), None => None }`.
- **`ref(c)` parameters are positions.** `strike` is
  `strike(c: &mut Vec<Fighter>, __c1: usize, __c2: usize)` — one borrow of the
  container it names, two positions, indexing `c[__c1]` — while the *proven*
  pair keeps two `&mut Fighter` parameters (`duel`) and is fed by a
  `split_at_mut`. At the call the positions are computed first, so the
  container's `&mut` is the only borrow: `rotate(&mut team.members, __c11,
  __c12)`.
- **Reads are borrows.** `get(squad, 0)!.hp` renders as a plain `&Fighter`
  from `get_platform`, not a position: only `at` mints a handle.
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
