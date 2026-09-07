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
