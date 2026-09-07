// [seq-iterable] [implicit-group] What "iterable" means in Salvo: not a trait a type
// implements, but a *function* the call site can find. `Iterable<It, T>` is
// a `params` group with one member — `iter` — so a function that wants to
// walk anything writes `?Iterable<It, T>` and every call fills it with the
// `iter` overload that fits: std's own for `List<T>`, `T[]`, `Str` and
// `Iter<T>`, or one the caller declared for a type of their own.
//
// A group emits nothing on any backend [implicit-group]: it is a
// declaration-side shorthand, resolved at each call site by name and type
// [implicit-resolve]. Its own module so that a program mentioning none of
// this emits nothing for it [mod-used-only] — `core.*` is implicitly
// visible, so no import is needed either way.
params Iterable<It, T> {
    fn iter(it: It) -> Iter<T>
}

// An `Iter<T>` is itself iterable — the identity that lets a chain compose
// (`filter(map(xs, f), keep)`) and lets an iterator function's result reach
// the sequence functions.
intrinsic fn iter<T>(it: Iter<T>) [] -> [it] Iter<T>
