// [seq-iterable] Sequence functions over anything iterable
// [implicit-group]: the subject
// only has to have an `iter`, which `?Iterable<It, T>` finds at the call
// site — so `map` works on a `List<T>`, an array, a `Str` (its characters),
// an `Iter<T>` from an iterator function, and on a type of your own the
// moment you declare `fn iter` for it.
//
// **Eager**: `map` and `filter` return a `Mut List<U>` rather than a lazy
// `Iter<U>`. A lazy one would have to store the callback, and a stored
// callback cannot perform effects ([iter-effect-free] — the reason iterator
// functions may not either), which would rule out the `println` people
// actually write inside a `map`. Chaining still works: the result has an
// `iter`.
//
// Each function also has a `List` overload, declared `intrinsic` so that the
// backends lower it to their own `map`/`filter`/`fold` — the generic body
// below is the fallback, and overload specificity picks the fast path when
// the subject really is a `List` [fn-overload-rank].

// Applies [f] to every element of [xs], in order.
fn map<It, T, U>(xs: It, f: (T) -> U, ?Iterable<It, T>) [] -> [xs, f] Mut List<U> {
    let out = mutable_list<U>()
    for x in iter(xs) {
        add(out, f(x))
    }
    return out
}

// The elements of [xs] that [keep] accepts, in order.
fn filter<It, T>(xs: It, keep: (T) -> Bool, ?Iterable<It, T>) [] -> [xs, keep] Mut List<T> {
    let out = mutable_list<T>()
    for x in iter(xs) {
        if keep(x) {
            add(out, x)
        }
    }
    return out
}

// Folds [xs] into a single value, starting from [init] and combining with
// [f] — the accumulator first, the element second.
fn reduce<It, T, A>(xs: It, init: A, f: (A, T) -> A, ?Iterable<It, T>) [] -> [xs, f] A {
    let acc = init
    for x in iter(xs) {
        acc = f(acc, x)
    }
    return acc
}

// The `List` fast paths. Same names, same shapes, one less indirection: a
// backend lowers these to its own collection operation.
intrinsic fn map<T, U>(list: List<T>, f: (T) -> U) [] -> [list, f] Mut List<U>
intrinsic fn filter<T>(list: List<T>, keep: (T) -> Bool) [] -> [list, keep] Mut List<T>
intrinsic fn reduce<T, A>(list: List<T>, init: A, f: (A, T) -> A) [] -> [list, f] A

// ===== the lazy pair [seq-lazy] =====
//
// Laziness is **asked for**, not inherited (user decision 2026-09-08): the
// eager forms above stay the default, and these are the ones that hand back a
// producer. Nothing is computed until the result is driven, so an unbounded
// subject is fine — and the callback runs once per element in *every* pass,
// which is why a callback carrying mutable state is refused
// [iter-mut-param].

// Applies [f] to every element of [xs], lazily: [f] runs as the consumer
// pulls. Chain freely — the result is iterable like anything else.
fn map_lazy<It, T, U>(xs: It, f: (T) -> U, ?Iterable<It, T>) -> [xs, f] Iter<U> {
    for x in iter(xs) {
        yield f(x)
    }
}

// The elements of [xs] that [keep] accepts, lazily.
fn filter_lazy<It, T>(xs: It, keep: (T) -> Bool, ?Iterable<It, T>) -> [xs, keep] Iter<T> {
    for x in iter(xs) {
        if keep(x) {
            yield x
        }
    }
}

// ===== mapping into a collection you provide [seq-into] =====

// Applies [f] to every element of [xs] and puts the results in [dest], which
// is the *first* argument because it is what the call is about — and which is
// **handed back**, so a chain can carry on from it.
//
// [add] is an implicit parameter, so [dest] is not a `List`: it is anything
// with an `add` the call site can find — the same "a function, not a trait"
// move `?Iterable` makes for the subject [implicit-group].
fn map_to<D, It, T, U>(
    dest: Mut D,
    xs: It,
    f: (T) -> U,
    ?add: (dest: Mut D, elem: U) -> [dest: Mut] None,
    ?Iterable<It, T>
) -> [xs, f] Mut D {
    for x in iter(xs) {
        add(dest, f(x))
    }
    return dest
}

// The same, keeping the elements [keep] accepts rather than mapping them.
fn filter_to<D, It, T>(
    dest: Mut D,
    xs: It,
    keep: (T) -> Bool,
    ?add: (dest: Mut D, elem: T) -> [dest: Mut] None,
    ?Iterable<It, T>
) -> [xs, keep] Mut D {
    for x in iter(xs) {
        if keep(x) {
            add(dest, x)
        }
    }
    return dest
}
