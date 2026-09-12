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
// named one is. `=> it: Mut` says the pass is advanced **in place** and handed
// back, which is what lets a caller drive it further.

// Applies [f] to every element of [it], in order.
fn map<It, T, U>(it: Mut It, f: (T) -> U, ?Yield<It, T>) [] -> Mut List<U> => it: Mut, f {
    let out = mutable_list<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}

// The elements of [it] that [keep] accepts, in order — as a **view**: the
// result holds borrows of the elements, so nothing is copied [copy-opt-in],
// and it lives no longer than the pass's source. `[it: Mut Proj]` is the
// written lend [proj-infer]: a generic body cannot show the analysis that an
// element of an opaque pass is stored, so the signature says it. For a list
// of your own to keep, see [filter_to].
fn filter<It, T>(it: Mut It, keep: (T) -> Bool, ?Yield<It, T>) [] -> Mut List<Proj T> => it: Mut, Proj[from: it], keep {
    let out = mutable_list<Proj T>()
    for x in it {
        if keep(x) {
            add(out, x)
        }
    }
    return out
}

// Folds [it] into a single value, starting from [init] and combining with
// [f] — the accumulator first, the element second.
fn reduce<It, T, A>(it: Mut It, init: A, f: (A, T) -> A, ?Yield<It, T>) [] -> A => it: Mut, f {
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
intrinsic fn map<T, U>(list: List<T>, f: (T) -> U) [] -> Mut List<U> => list, f
intrinsic fn filter<T>(list: List<T>, keep: (T) -> Bool) [] -> Mut List<Proj T> => list, Proj[from: list], keep
intrinsic fn reduce<T, A>(list: List<T>, init: A, f: (A, T) -> A) [] -> A => list, f, !init

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
    ?add: (dest: Mut D, elem: U) -> None,
    ?Yield<It, T>
) [] -> Mut D =>[add] dest: Mut, !elem => it: Mut, f {
    for x in it {
        add(dest, f(x))
    }
    return dest
}

// The same, keeping the elements [keep] accepts rather than mapping them.
// [dest] owns what it is given, so each kept element is **copied** in — the
// `_to` name is the opt-in [copy-opt-in], and [copy] arrives as an implicit
// so the copy is the element type's own [copy-implicit].
fn filter_to<D, It, T>(
    dest: Mut D,
    it: Mut It,
    keep: (T) -> Bool,
    ?add: (dest: Mut D, elem: T) -> None,
    ?copy: (v: T) -> T,
    ?Yield<It, T>
) [] -> Mut D =>[add] dest: Mut, !elem => it: Mut, keep {
    for x in it {
        if keep(x) {
            add(dest, copy(x))
        }
    }
    return dest
}
