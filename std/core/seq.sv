// [seq-pass] Sequence functions over **passes** [iter-protocol]: the subject
// is a pass, and its `next` arrives as an implicit parameter through the
// `?Yield<It, T>` spread [implicit-group] — so these work on std's container
// passes (`map(iter(xs), f)`), on a `yield` function's result, and on a pass
// type of your own the moment it declares `: Yield<self, T>`.
//
// The subject is written as the pass rather than as the container (user
// decision 2026-09-09): a container is iterated by writing its `iter`, which
// is one call more at the use site and no cross-implicit inference in the
// compiler. The `List` fast paths at the bottom keep the short spelling for
// the common case.
//
// **Eager**: `map` and `filter` return a `Mut List<U>`. The lazy pair below
// returns a pass instead, and it is asked for by name [seq-lazy].
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

// ===== the lazy pair [seq-lazy] =====
//
// Laziness is **asked for**, not inherited (user decision 2026-09-08): the
// eager forms above stay the default, and these hand back a pass that computes
// as it is driven. Nothing runs until something calls `next` on the result, so
// an unbounded source is fine.
//
// Each is a **composed pass**: a struct holding the source pass and the
// callback, plus the `next` that discharges its `Yield` obligation. That is
// the manual form the reduction makes ordinary — there is no lazy *type* to
// return, only a struct of one's own [iter-protocol].

// A pass applying [f] to every element of [source] as it is pulled.
struct MapPass<It, T, U> : Yield<self, U> canbe Mut {
    // The pass being mapped, advanced in place as this one is.
    source: Mut It,
    // Applied to each element as it arrives.
    f: (T) -> U,
    // The source's `next`, captured where it could be resolved: at the call
    // that built this pass [implicit-group]. A composed pass stores the
    // function rather than asking for an implicit of its own, so driving it —
    // and mapping over the result again — stays an ordinary call.
    step: (it: Mut It) -> [it: Mut] Emitted T | Finished
}

// Applies [f] to every element of [it], lazily: [f] runs as the consumer
// pulls. Chain freely — the result is a pass like any other.
fn map_lazy<It, T, U>(it: Mut It, f: (T) -> U, ?Yield<It, T>) [] -> [] Mut MapPass<It, T, U> {
    return Mut MapPass<It, T, U> { source: it, f: f, step: copy(next) }
}

fn next<It, T, U>(pass: Mut MapPass<It, T, U>) [] -> [pass: Mut] Emitted U | Finished {
    let advance = copy(pass.step)
    let step = advance(pass.source)
    when step {
        is Emitted {
            let f = copy(pass.f)
            return emitted(f(step))
        }
        is Finished {
            return finished()
        }
    }
}

// A pass keeping the elements of [source] that [keep] accepts.
struct FilterPass<It, T> : Yield<self, T> canbe Mut {
    // The pass being filtered, advanced in place as this one is.
    source: Mut It,
    // Decides which elements survive.
    keep: (T) -> Bool,
    // The source's `next` [implicit-group].
    step: (it: Mut It) -> [it: Mut] Emitted T | Finished
}

// The elements of [it] that [keep] accepts, lazily.
fn filter_lazy<It, T>(it: Mut It, keep: (T) -> Bool, ?Yield<It, T>) [] -> [] Mut FilterPass<It, T> {
    return Mut FilterPass<It, T> { source: it, keep: keep, step: copy(next) }
}

fn next<It, T>(pass: Mut FilterPass<It, T>) [] -> [pass: Mut] Emitted T | Finished {
    let advance = copy(pass.step)
    let keep = copy(pass.keep)
    let going = true
    while going {
        let step = advance(pass.source)
        when step {
            is Emitted {
                if keep(step) {
                    return emitted(step)
                }
            }
            is Finished {
                going = false
            }
        }
    }
    return finished()
}

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
