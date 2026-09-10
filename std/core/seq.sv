// [seq-pass] Sequence functions over **passes** [iter-protocol]: the subject
// is a pass, and its `next` arrives as an implicit parameter through the
// `?Yield<It, T>` spread [implicit-group] — so these work on std's container
// passes (`map(iter(xs), f)`), on an `iter fn`'s result, and on a pass
// type of your own the moment it declares `: Yield<self, T>`.
//
// The subject is written as the pass rather than as the container (user
// decision 2026-09-09): a container is iterated by writing its `iter`, which
// is one call more at the use site and no cross-implicit inference in the
// compiler. The `List` fast paths at the bottom keep the short spelling for
// the common case.
//
// **Eager**: `map` and `filter` return a `Mut List<U>`. Nothing here is lazy —
// the lazy pair was removed 2026-09-10 (user decision) and laziness is
// reconsidered after concurrency lands; see ROADMAP.md. A *composed* pass is
// still perfectly writable by hand, since a pass is only a struct with a
// `next` [iter-protocol].
//
//
// Driving is an ordinary `for`: the `?Yield<It, T>` spread is the declaration
// the loop reads [iter-generic-drive], so a generic pass is driven exactly as a
// named one is. `[it: Mut]` says the pass is advanced **in place** and handed
// back, which is what lets a caller drive it further.

// Applies [f] to every element of [it], in order.
fn map<It, T, U>(it: Mut It, f: (T) -> U, ?Yield<It, T>) [] -> [it: Mut, f] Mut List<U> {
    let out = mutable_list<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}

// The elements of [it] that [keep] accepts, in order.
fn filter<It, T>(it: Mut It, keep: (T) -> Bool, ?Yield<It, T>) [] -> [it: Mut, keep] Mut List<T> {
    let out = mutable_list<T>()
    for x in it {
        if keep(x) {
            add(out, x)
        }
    }
    return out
}

// Folds [it] into a single value, starting from [init] and combining with
// [f] — the accumulator first, the element second.
fn reduce<It, T, A>(it: Mut It, init: A, f: (A, T) -> A, ?Yield<It, T>) [] -> [it: Mut, f] A {
    let acc = init
    for x in it {
        acc = f(acc, x)
    }
    return acc
}

// The `List` fast paths. Same names, one less indirection — and the reason
// `map(xs, f)` still reads well on the type people map most: a backend lowers
// these to its own collection operation, and overload specificity picks them
// when the subject really is a `List` [fn-overload-rank].
intrinsic fn map<T, U>(list: List<T>, f: (T) -> U) [] -> [list, f] Mut List<U>
intrinsic fn filter<T>(list: List<T>, keep: (T) -> Bool) [] -> [list, keep] Mut List<T>
intrinsic fn reduce<T, A>(list: List<T>, init: A, f: (A, T) -> A) [] -> [list, f] A

// ===== mapping into a collection you provide [seq-into] =====

// Applies [f] to every element of [it] and puts the results in [dest], which
// is the *first* argument because it is what the call is about — and which is
// **handed back**, so a chain can carry on from it.
//
// [add] is an implicit parameter, so [dest] is not a `List`: it is anything
// with an `add` the call site can find — the same "a function, not a trait"
// move `?Yield` makes for the subject [implicit-group].
fn map_to<D, It, T, U>(
    dest: Mut D,
    it: Mut It,
    f: (T) -> U,
    ?add: (dest: Mut D, elem: U) -> [dest: Mut] None,
    ?Yield<It, T>
) [] -> [it: Mut, f] Mut D {
    for x in it {
        add(dest, f(x))
    }
    return dest
}

// The same, keeping the elements [keep] accepts rather than mapping them.
fn filter_to<D, It, T>(
    dest: Mut D,
    it: Mut It,
    keep: (T) -> Bool,
    ?add: (dest: Mut D, elem: T) -> [dest: Mut] None,
    ?Yield<It, T>
) [] -> [it: Mut, keep] Mut D {
    for x in it {
        if keep(x) {
            add(dest, x)
        }
    }
    return dest
}
