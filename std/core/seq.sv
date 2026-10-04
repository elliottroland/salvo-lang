// [seq-iterator] Sequence functions over **iterators** [iter-protocol]: the subject
// is an iterator, and its `next` arrives as an implicit parameter through the
// `?Yield<It, T>` spread [implicit-group] — so these work on std's container
// iterators (`map(iter(xs), f)`), on an `iter fn`'s result, and on an iterator
// type of your own the moment it declares `: Yield<self, T>`.
//
// The subject is written as the iterator rather than as the container (user
// decision 2026-09-09): a container is iterated by writing its `iter`, which
// is one call more at the use site and no cross-implicit inference in the
// compiler. The `List` fast paths at the bottom keep the short spelling for
// the common case.
//
// **Eager**: `map` and `filter` return a `Mut List<U>`. The lazy adaptors at
// the bottom answer iterators, and say so in the name: a present participle
// (`mapping`, `filtering`, `taking`, …) is an iterator over another
// [seq-lazy]. Each is an ordinary struct with a `next` [iter-protocol].
//
//
// Driving is an ordinary `for`: the `?Yield<It, T>` spread is the declaration
// the loop reads [iter-generic-drive], so a generic iterator is driven exactly as a
// named one is. `=> it: Mut` says the iterator is advanced **in place** and handed
// back, which is what lets a caller drive it further.

// Applies [f] to every element of [it], in order.
export fn map<It, T, U>(it: Mut It, f: (T) -> U, ?Yield<It, T>) [] -> Mut List<U> => it: Mut, f {
    let out = mut_list_of<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}

// The elements of [it] that [keep] accepts, in order — as a **view**: the
// result holds borrows of the elements, so nothing is copied [copy-opt-in],
// and it lives no longer than the iterator's source. `holds proj(it)` is the
// written lend [proj-infer]: a generic body cannot show the analysis that an
// element of an opaque iterator is stored, so the signature says it. For a list
// of your own to keep, see [filter_to].
export fn filter<It, T>(it: Mut It, keep: (T) -> Bool, ?Yield<It, T>) [] -> Mut List<proj T> holds proj(it)
=> it: Mut, keep {
    let out = mut_list_of<proj T>()
    for x in it {
        if keep(x) {
            add(out, x)
        }
    }
    return out
}

// Folds [it] into a single value, starting from [init] and combining with
// [f] — the accumulator first, the element second.
export fn reduce<It, T, A>(it: Mut It, init: A, f: (A, T) -> A, ?Yield<It, T>) [] -> A => it: Mut, f {
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
export intrinsic fn map<T, U>(list: List<T>, f: (T) -> U) [] -> Mut List<U> => list, f
export intrinsic fn filter<T>(list: List<T>, keep: (T) -> Bool) [] -> Mut List<proj T> holds proj(list)
=> list, keep
export intrinsic fn reduce<T, A>(list: List<T>, init: A, f: (A, T) -> A) [] -> A => list, f, !init

// ===== mapping into a collection you provide [seq-into] =====

// Applies [f] to every element of [it] and puts the results in [dest], which
// is the *first* argument because it is what the call is about — and which is
// **handed back**, so a chain can carry on from it.
//
// [add] is an implicit parameter, so [dest] is not a `List`: it is anything
// with an `add` the call site can find — the same "a function, not a trait"
// move `?Yield` makes for the subject [implicit-group].
export fn map_to<D, It, T, U>(
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
export fn filter_to<D, It, T>(
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

// The uniform **consuming callback** for values that owe nothing: where a
// combinator takes an `end: (x: T) -> None` that consumes (the pattern that
// replaced the implicit release — a linear caller passes the type's own
// discharger), a non-linear caller iterators `drop`. Deliberately *without*
// `canbe linear`: a linear argument is refused by the instantiation ban
// [linear-generics], which is exactly the protection — `drop` never
// discharges an obligation.
export fn drop<T>(value: T) [] -> None => !value {
}

// ===== lazy adaptors [seq-lazy] =====
//
// An iterator over another is named by a **present participle** (user
// decision 2026-10-04): `map(it, f)` is a list, `mapping(it, f)` an iterator
// that maps as it is driven. Each adaptor is a struct holding the source
// iterator and the source's `next` (the `?Yield` implicit, kept as a fn
// field), whose own `next` pulls from the source one element at a time.
// Nothing runs until the adaptor is driven, so an endless source costs
// nothing until then, and a `break` stops the source too. The source is moved
// into the adaptor; one over a list's `iter` borrows the list, as the pass
// itself does.

// [seq-lazy] The source's elements, each passed through a function.
export struct Mapping<It, T, U> : Yield<self, U> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    f: (x: T) -> U
}

export fn next<It, T, U>(m: Mut Mapping<It, T, U>) [] -> Emitted U | Finished => m: Mut {
    let step = m.step
    let x = step(m.src)
    if x is Finished {
        return finished()
    }
    let f = m.f
    let y: U = f(x)
    return emitted(y)
}

// [seq-lazy] An iterator over [f] of each element of [it], computed as it is
// driven.
export fn mapping<It, T, U>(it: It, f: (x: T) -> U, ?Yield<It, T>) [] -> Mut Mapping<It, T, U> => !it, !f {
    return Mut Mapping<It, T, U> { src: it, step: copy(next), f: f }
}

// [seq-lazy] The source's elements a predicate accepts.
export struct Filtering<It, T> : Yield<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    keep: (x: T) -> Bool
}

export fn next<It, T>(t: Mut Filtering<It, T>) [] -> Emitted T | Finished => t: Mut {
    let step = t.step
    let keep = t.keep
    while true {
        let x = step(t.src)
        if x is Finished {
            return finished()
        }
        if keep(x) {
            return emitted(x)
        }
    }
    return finished()
}

// [seq-lazy] An iterator over the elements of [it] that [keep] accepts.
export fn filtering<It, T>(it: It, keep: (x: T) -> Bool, ?Yield<It, T>) [] -> Mut Filtering<It, T> => !it, !keep {
    return Mut Filtering<It, T> { src: it, step: copy(next), keep: keep }
}

// [seq-lazy] The first [n] elements of the source.
export struct Taking<It, T> : Yield<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    left: Int
}

export fn next<It, T>(t: Mut Taking<It, T>) [] -> Emitted T | Finished => t: Mut {
    if t.left <= 0 {
        return finished()
    }
    t.left = t.left - 1
    let step = t.step
    return step(t.src)
}

// [seq-lazy] An iterator over the first [n] elements of [it].
export fn taking<It, T>(it: It, n: Int, ?Yield<It, T>) [] -> Mut Taking<It, T> => !it, !n {
    return Mut Taking<It, T> { src: it, step: copy(next), left: n }
}

// [seq-lazy] The source's elements while a predicate accepts them.
export struct TakingWhile<It, T> : Yield<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    keep: (x: T) -> Bool,
    done: Bool
}

export fn next<It, T>(t: Mut TakingWhile<It, T>) [] -> Emitted T | Finished => t: Mut {
    if t.done {
        return finished()
    }
    let step = t.step
    let x = step(t.src)
    if x is Finished {
        t.done = true
        return finished()
    }
    let keep = t.keep
    if keep(x) {
        return emitted(x)
    }
    t.done = true
    return finished()
}

// [seq-lazy] An iterator over the elements of [it] up to the first one
// [keep] refuses (which is consumed from the source and not emitted).
export fn taking_while<It, T>(it: It, keep: (x: T) -> Bool, ?Yield<It, T>) [] -> Mut TakingWhile<It, T>
=> !it, !keep {
    return Mut TakingWhile<It, T> { src: it, step: copy(next), keep: keep, done: false }
}

// [seq-lazy] The source's elements after the first few.
export struct Skipping<It, T> : Yield<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    left: Int
}

export fn next<It, T>(t: Mut Skipping<It, T>) [] -> Emitted T | Finished => t: Mut {
    let step = t.step
    while t.left > 0 {
        t.left = t.left - 1
        let x = step(t.src)
        if x is Finished {
            return finished()
        }
    }
    return step(t.src)
}

// [seq-lazy] An iterator over the elements of [it] after its first [n].
export fn skipping<It, T>(it: It, n: Int, ?Yield<It, T>) [] -> Mut Skipping<It, T> => !it, !n {
    return Mut Skipping<It, T> { src: it, step: copy(next), left: n }
}

// [seq-lazy] The source's elements from the first one a predicate refuses.
export struct SkippingWhile<It, T> : Yield<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    skip: (x: T) -> Bool,
    started: Bool
}

export fn next<It, T>(t: Mut SkippingWhile<It, T>) [] -> Emitted T | Finished => t: Mut {
    let step = t.step
    if t.started {
        return step(t.src)
    }
    let skip = t.skip
    while true {
        let x = step(t.src)
        if x is Finished {
            return finished()
        }
        if !skip(x) {
            t.started = true
            return emitted(x)
        }
    }
    return finished()
}

// [seq-lazy] An iterator over the elements of [it] from the first one [skip]
// refuses.
export fn skipping_while<It, T>(it: It, skip: (x: T) -> Bool, ?Yield<It, T>) [] -> Mut SkippingWhile<It, T>
=> !it, !skip {
    return Mut SkippingWhile<It, T> { src: it, step: copy(next), skip: skip, started: false }
}

// [seq-lazy] Drives [it] to its end, answering its elements as a list — a
// view of a borrowing source's elements, as [filter]'s answer is. A
// container's own `to_list` is more specific, and wins [fn-overload-rank].
export fn to_list<It, T>(it: Mut It, ?Yield<It, T>) [] -> Mut List<proj T> holds proj(it) => it: Mut {
    let out = mut_list_of<proj T>()
    for x in it {
        add(out, x)
    }
    return out
}

// [seq-lazy] Drives [it] to its end, answering how many elements it had. A
// `count` of the program's own over a list is more specific and wins.
export fn count<It, T>(it: Mut It, ?Yield<It, T>) [] -> Int => it: Mut {
    let n = 0
    for _x in it {
        n = n + 1
    }
    return n
}
