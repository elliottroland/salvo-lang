// [iter-protocol] The *pull* iteration protocol, written in Salvo rather than
// built into the compiler: a **pass** is anything with a `next`, and `next`
// reports either an element or the end of the sequence.
//
// This is what makes the iterators `yield` cannot express writable by hand —
// `zip`, `merge`, anything reading two sources at once, anything wanting
// explicit control of its state. A `yield` function is the sugar; a struct
// with a `next` is the manual form; `for` drives either, because both answer
// the same group.
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
fn emitted<T canbe Linear>(value: T) [] -> [] T as Emitted {
    return value
}

// The end of a sequence.
fn finished() [] -> [] Finished {
    return Finished {}
}

// [iter-protocol] [implicit-group] What "a pass" means: not a trait a type
// implements, but a *function* `for` can find. A type of your own becomes
// drivable by declaring one `next` — and the value has to be a pass
// (`Once`, see [once-fn]), since driving it uses it up.
//
// The state is taken as `Mut`: advancing a pass is a mutation of its
// position. A group emits nothing on any backend [implicit-group].
params Iterator<St, T> {
    fn next(st: Mut St) -> [st: Mut] Emitted T | Finished
}
