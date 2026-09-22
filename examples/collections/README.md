# collections

The four everyday collections — `List`, `Set`, `Map`, and the sorted pair
`SortedSet`/`SortedMap` — and the decisions around them.

Run it:

```bash
cargo run -- run --backend rust   --src examples/collections/salvo
cargo run -- run --backend kotlin --src examples/collections/salvo
```

## What to look for

**The literals, and where a brace is ambiguous.** `[1, 2]` is a list, `{1, 2}`
a set, `{"k": 1}` a map — each sugar for the matching constructor. A brace
whose first entry is `name:` is a *struct* literal, which came first and stays;
that is why a map key is an expression rather than a bare name. An empty
literal takes its type from the position it is in, and `Mut` on the position
asks for a mutable collection.

**Two orderings, both the language's.** A `Set` or `Map` iterates in
**insertion** order on either backend — overwriting a key keeps its position,
removing one leaves the rest in place. Rust's own hash containers have no
order at all (theirs is unspecified and seeded per process), so the backend
ships an ordered implementation to match Kotlin's `LinkedHashMap`. The sorted
pair is ordered by its **keys** instead, and is a *separate type* rather than a
qualifier on `Set`/`Map`: sortedness changes how a collection behaves, and a
qualifier can be dropped on the way into a function that relied on it.

**What may be a key.** A key has to be hashable — the intrinsic key types, or a
struct with a `hash` and an `eq`. `: auto Hashed<self>` is the one-token way
to get both, generated from the fields; the sorted collections need a `cmp`
instead (`: auto Ordered<self>`), which is a slightly different bar. The
clause is validated where it is written, so a mutable struct or a float field is
refused at the declaration rather than at some distant `Set<Point>`. Once a
struct has the pair, it is a key **by value**: two equal points are the same key,
as the third section shows.

**Comparison is a capability.** `==` is `eq(a, b)` and `<` is `cmp(a, b) < 0`, so
a type of your own has them exactly when the functions exist — generated with
`default`, or hand-written and `@`-scoped to the type. `Note` here asks only for
equality. Comparing two *different* struct types is an error rather than a quiet
`false`.

**Generating and converting.** `list_by(n, i -> …)` builds from a size and a
rule; `to_set` dedups a list; `to_map` comes in two forms — a list of pairs, or
a list plus a rule.

**Claims a list can carry.** This is where the qualifier machinery pays off
over a container: a claim travels in the type, so a function demands it rather
than re-checking it.

- `NonEmpty` is the one with a predicate, so it can be *tested* with `is`. It
  earns its keep on `first`, which drops the optional. It also arrives by
  refinement: `add` cannot promise it (a mutating function may not promise back
  a qualifier it has never heard of), so the qualifier says it on `add`'s
  behalf.
- `Sorted` is established by construction only — `sort` and `mut_sort` mint it
  — because deciding whether a list happens to be sorted means comparing
  elements. It is what makes `binary_search` honest: over an unordered list the
  answer would be meaningless, not merely absent. `add_sorted` is the insert
  that *keeps* the claim, placing the element where the order survives.
  The claim also **names the ordering it was sorted by**, since orderings are
  plural: `sort(words, cmp = by_len)` publishes `by_len` into the claim, and
  the search and the insert read it out of the list's type rather than
  resolving an ordering of their own. Two lists sorted differently are two
  types, and a signature that takes either writes `Sorted List<T>` — an
  argument nobody writes is unconstrained.
- `Distinct` comes from a set, the one thing a set can honestly promise about
  the list it converts to. `count_unique` demands it and needs no duplicate
  check.

Note that `Sorted` names a *different mechanic* from the `SortedSet`/
`SortedMap` types: those are a representation, this is an erased claim about
an otherwise ordinary list — which is why a `Sorted List` still reaches the
whole list surface.

**Iterating.** A set yields its elements. A **map yields its keys**, and a
value is reached with `get`, which borrows rather than copying — the same
reading Python takes for `for k in d`.

## What it deliberately does not show

`to_map` is called on a variable rather than on a literal directly:
`to_map([…], rule)` does not currently infer the callee's type parameter from
a bare literal argument (the diagnostic names the remedies — bind it, annotate
the result, or nest a call). Recorded in ROADMAP.md.
