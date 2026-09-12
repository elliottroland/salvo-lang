// [iter-protocol] The *pull* iteration protocol, written in Salvo rather than
// built into the compiler: a **pass** is a type declaring `: Yield<T>`, and
// its `next` reports either an element or the end of the sequence.
//
// This is what makes the iterators `yield` cannot express writable by hand —
// `zip`, `merge`, anything reading two sources at once, anything wanting
// explicit control of its state. A `yield` function is the sugar; a struct
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
qualifier Emitted<T> of T

// The end arm. A fieldless struct rather than a qualifier because there is
// nothing for it to qualify: it carries no element, and reusing `None` would
// say "absent" where the claim is "the sequence ended".
struct Finished {}

// [qual-ctor-fn] Tags a value as the element arm. The value is moved into the
// result, so nothing is kept ([] deductions); linear values may be tagged,
// since the obligation travels with them [linear-generics].
fn emitted<T canbe Linear>(value: T) [] -> T as Emitted {
    return value
}

// The end of a sequence.
fn finished() [] -> Finished {
    return Finished {}
}

// [iter-protocol] [group-obligation] What a **pass** is: a type that
// declares `: Yield<self, T>`, tied to iteration by its `next`. The
// obligation is checked at the declaring struct — a misspelled or missing
// `next` is an error where the promise is written, not a puzzling "not
// iterable" at some loop — and `for` reads the declaration rather than
// scanning overloads.
//
// The state comes *first* and the element second, and the state is an
// ordinary type parameter rather than a magic `Self`: that is what keeps this
// an ordinary group, so the same declaration also spreads as
// `?Yield<It, T>` implicit parameters — which is how a generic combinator
// reaches a source pass's `next` [implicit-group]. At an obligation the
// declaring type is written `self` [group-self].
//
// The state is taken as `Mut`: advancing a pass is a mutation of its
// position. A pass that *walks* data emits borrows of it — it declares
// `: Yield<self, Proj T>` and its `next` returns `Emitted (Proj[from: p] T)`
// [yield-proj] — so reading through it copies nothing and whoever *stores* an
// element says `copy` [copy-opt-in]; a pass that *computes* its elements
// declares `: Yield<self, T>` and owns them. A reading combinator's
// `?Yield<It, T>` accepts either. A group emits nothing on any backend
// [implicit-group].
params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}
