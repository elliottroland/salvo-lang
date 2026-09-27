// [iter-protocol] The *pull* iteration protocol, written in Salvo rather than
// built into the compiler: an **iterator struct** is a type declaring
// `: Yield<self, T>`, and its `next` reports either an element or the end of
// the sequence. A **source** is a type an `iter` mints an iterator from,
// declared with `: Iter<self, T>`.
//
// This is what makes `zip`, `merge`, anything reading two sources at once,
// anything wanting explicit control of its state, writable by hand. An
// `iter fn` is the sugar (the compiler writes the struct) [iter-fn]; a struct
// with a `next` is the manual form; `for` drives either, because both answer
// the same declaration.
//
// Own module so a program that never iterates by hand emits nothing for it
// [mod-used-only]; `core.*` is implicitly visible, so no import is needed.

// [qual-constructive] The element arm of a `next` result. A qualifier rather
// than a struct so the element keeps its own type — `Emitted Int` *is* an
// `Int`, tagged — and so a sequence of optionals still has a distinguishable
// end: `Emitted None | Finished` has two arms where `None | None` would
// have one. Erased in generated code [qual-erasure].
export provenance qualifier Emitted<T> of T

// The end arm. A fieldless struct rather than a qualifier because there is
// nothing for it to qualify: it carries no element, and reusing `None` would
// say "absent" where the claim is "the sequence ended".
export struct Finished {}

// [qual-ctor-fn] Tags a value as the element arm. The value is moved into the
// result, so nothing is kept ([] deductions); linear values may be tagged,
// since the obligation travels with them [linear-generics].
export fn emitted<T canbe linear>(value: T) [] -> +Emitted T {
    return value
}

// The end of a sequence.
export fn finished() [] -> Finished {
    return Finished {}
}

// [iter-protocol] [group-obligation] What an **iterator struct** is: a type
// that declares `: Yield<self, T>`, tied to iteration by its `next`. The
// obligation is checked at the declaring struct — a misspelled or missing
// `next` is an error where the promise is written, not a puzzling "not
// iterable" at some loop — and `for` reads the declaration rather than
// scanning overloads.
//
// The state comes *first* and the element second, and the state is an
// ordinary type parameter rather than a magic `Self`: that is what keeps this
// an ordinary group, so the same declaration also spreads as
// `?Yield<It, T>` implicit parameters — which is how a generic combinator
// reaches a source iterator's `next` [implicit-group]. At an obligation the
// declaring type is written `self` [group-self].
//
// The state is taken as `Mut`: advancing an iterator is a mutation of its
// position. An iterator that *walks* data emits borrows of it — it declares
// `: Yield<self, proj T>` and its `next` returns `Emitted (proj(p) T)`
// [yield-proj] — so reading through it copies nothing and whoever *stores* an
// element says `copy` [copy-opt-in]; one that *computes* its elements
// declares `: Yield<self, T>` and owns them. A reading combinator's
// `?Yield<It, T>` accepts either. A group emits nothing on any backend
// [implicit-group].
export params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}

// [iter-group] [iter-type] What a **source** is: a type an `iter` mints an
// iterator from. `iter T` is the placeholder for that iterator — "a `Mut` type
// declaring `: Yield<self, T>`" — one type across both members, so the second
// member is `Yield`'s `next` over it: the two groups are one protocol stated
// from two sides. The pair is filled **together** (`iter with next`): a caller
// may override the minter, never the step alone, since the step belongs to
// whatever the minter answers [implicit-with].
//
// A type joins by declaring `: Iter<self, T>` and either an `iter fn iter(s:
// self) -> Emitted T | Finished` — the compiler writes the struct and both
// members — or a fn `iter` of its own returning an iterator struct. Declaring
// both `: Iter` and `: Yield` on one type is refused: `for x in s` would have
// two answers.
//
// Spread as `?Iter<C, T>`, the group brings in `iter: (C) -> iter T` and
// `next: (iter T) -> Emitted T | Finished` over a **hidden** type parameter
// for the iterator struct, so a combinator over a generic *source* writes
// `for x in c` and the loop mints with the one and drives with the other.
export params Iter<C, T> => iter with next {
    fn iter(collection: C) -> iter T
    fn next(iterator: iter T) -> Emitted T | Finished => iterator: Mut
}
