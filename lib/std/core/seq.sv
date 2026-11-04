// [seq-iterator] Sequence functions over **iterators** [iter-protocol]: the subject
// is an iterator, and its `next` arrives as an implicit parameter through the
// `?Next<It, T>` spread [implicit-group] — so these work on std's container
// iterators (`map(iter(xs), f)`), on an `iter fn`'s result, and on an iterator
// type of your own the moment it declares `: Next<self, T>`.
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
// Driving is an ordinary `for`: the `?Next<It, T>` spread is the declaration
// the loop reads [iter-generic-drive], so a generic iterator is driven exactly as a
// named one is. `=> it: Mut` says the iterator is advanced **in place** and handed
// back, which is what lets a caller drive it further.

// Applies [f] to every element of [it], in order.
export fn map<It canbe linear, T, U, [E]>(it: Mut It, f: (T) [E] -> U, ?Next<It, T>) -> Mut List<U> => it: Mut, f {
    let out = Mut List<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}

// The elements of [it] that [keep] accepts, in order — as a **view**: the
// result holds borrows of the elements, so nothing is copied,
// and it lives no longer than the iterator's source. `holds proj(it)` is the
// written lend: a generic body cannot show the analysis that an
// element of an opaque iterator is stored, so the signature says it. For a list
// of your own to keep, see [filter_to].
// @compilerrules [copy-opt-in, proj-infer]
export fn filter<It canbe linear, T, [E]>(it: Mut It, keep: (T) [E] -> Bool, ?Next<It, T>) -> Mut List<proj T> holds proj(it)
=> it: Mut, keep {
    let out: Mut List<proj T> = Mut List()
    for x in it {
        if keep(x) {
            add(out, x)
        }
    }
    return out
}

// Folds [it] into a single value, starting from [init] and combining with
// [f] — the accumulator first, the element second.
export fn reduce<It canbe linear, T, A, [E]>(it: Mut It, init: A, f: (A, T) [E] -> A, ?Next<It, T>) -> A => it: Mut, init, f {
    let acc = init
    for x in it {
        acc = f(acc, x)
    }
    return acc
}

// The `List` fast paths. Same names, one less indirection — and the reason
// `map(xs, f)` still reads well on the type people map most: a backend lowers
// these to its own collection operation, and overload specificity picks them
// when the subject really is a `List`.
// @compilerrules [fn-overload-rank]
export fn map<T, U, [E]>(list: List<T>, f: (T) [E] -> U) -> Mut List<U> => list, f {
    let out = Mut List<U>()
    for x in list {
        add(out, f(x))
    }
    return out
}
export platform fn filter<T>(list: List<T>, keep: (T) -> Bool) [] -> Mut List<proj T> holds proj(list)
=> list, keep
export fn reduce<T, A, [E]>(list: List<T>, init: A, f: (A, T) [E] -> A) -> A => list, f, !init {
    let acc = init
    for x in list {
        acc = f(acc, x)
    }
    return acc
}

// ===== mapping into a collection you provide [seq-into] =====

// Applies [f] to every element of [it] and puts the results in [dest], which
// is the *first* argument because it is what the call is about — and which is
// **handed back**, so a chain can carry on from it.
//
// [add] is an implicit parameter, so [dest] is not a `List`: it is anything
// with an `add` the call site can find — the same "a function, not a trait"
// move `?Next` makes for the subject.
// @compilerrules [implicit-group]
export fn map_to<D, It, T, U>(
    dest: Mut D,
    it: Mut It,
    f: (T) -> U,
    ?add: (dest: Mut D, elem: U) -> None,
    ?Next<It, T>
) [] -> Mut D =>[add] dest: Mut, !elem => !dest, it: Mut, f {
    for x in it {
        add(dest, f(x))
    }
    return dest
}

// The same, keeping the elements [keep] accepts rather than mapping them.
// [dest] owns what it is given, so each kept element is **copied** in — the
// `_to` name is the opt-in, and [copy] arrives as an implicit
// so the copy is the element type's own.
// @compilerrules [copy-opt-in, copy-implicit]
export fn filter_to<D, It, T>(
    dest: Mut D,
    it: Mut It,
    keep: (T) -> Bool,
    ?add: (dest: Mut D, elem: T) -> None,
    ?copy: (v: T) -> T,
    ?Next<It, T>
) [] -> Mut D =>[add] dest: Mut, !elem => !dest, it: Mut, keep {
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
// `canbe linear`: a linear argument is refused by the instantiation ban,
// which is exactly the protection — `drop` never
// discharges an obligation.
// @compilerrules [linear-generics]
export fn drop<T>(value: T) [] -> None => !value {
}

// ===== lazy adaptors [seq-lazy] =====
//
// An iterator over another is named by a **present participle** (user
// decision 2026-10-04): `map(it, f)` is a list, `mapping(it, f)` an iterator
// that maps as it is driven. Each adaptor is a struct holding the source
// iterator and the source's `next` (the `?Next` implicit, kept as a fn
// field), whose own `next` pulls from the source one element at a time.
// Nothing runs until the adaptor is driven, so an endless source costs
// nothing until then, and a `break` stops the source too. The source is moved
// into the adaptor; one over a list's `iter` borrows the list, as the pass
// itself does.

// The source's elements, each passed through a function.
// @compilerrules [seq-lazy]
export struct Mapping<It canbe linear, T, U, [E]> : Next<self, U> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    f: (x: T) [E] -> U
}

export fn next<It canbe linear, T, U, [E]>(m: Mut Mapping<It, T, U, [E]>) [E] -> Emitted U | Finished => m: Mut {
    let step = m.step
    let x = step(m.src)
    if x is Finished {
        return finished()
    }
    let f = m.f
    let y: U = f(x)
    return emitted(y)
}

// An iterator over [f] of each element of [it], computed as it is
// driven.
// @compilerrules [seq-lazy]
export fn mapping<It canbe linear, T, U, [E]>(it: It, f: (x: T) [E] -> U, ?Next<It, T>) [] -> Mut Mapping<It, T, U, [E]> => !it, !f {
    return Mut Mapping<It, T, U, [E]> { src: it, step: copy(next), f: f }
}

// Ends the adaptor and answers the iterator it wraps, at the position it
// reached: how an adaptor over an obligation ends (`close(detach(t))`), and
// how a caller reads on past it.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, U, [E]>(it: Mapping<It, T, U, [E]>) [] -> It => !it {
    return it.src
}

// The source's elements a predicate accepts.
// @compilerrules [seq-lazy]
export struct Filtering<It canbe linear, T, [E]> : Next<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    keep: (x: T) [E] -> Bool
}

export fn next<It canbe linear, T, [E]>(t: Mut Filtering<It, T, [E]>) [E] -> Emitted T | Finished => t: Mut {
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

// An iterator over the elements of [it] that [keep] accepts.
// @compilerrules [seq-lazy]
export fn filtering<It canbe linear, T, [E]>(it: It, keep: (x: T) [E] -> Bool, ?Next<It, T>) [] -> Mut Filtering<It, T, [E]> => !it, !keep {
    return Mut Filtering<It, T, [E]> { src: it, step: copy(next), keep: keep }
}

// Ends the adaptor and answers the iterator it wraps, at the position it
// reached: how an adaptor over an obligation ends (`close(detach(t))`), and
// how a caller reads on past it.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, [E]>(it: Filtering<It, T, [E]>) [] -> It => !it {
    return it.src
}

// The first [n] elements of the source.
// @compilerrules [seq-lazy]
export struct Taking<It canbe linear, T> : Next<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    left: Int
}

export fn next<It canbe linear, T>(t: Mut Taking<It, T>) [] -> Emitted T | Finished => t: Mut {
    if t.left <= 0 {
        return finished()
    }
    t.left = t.left - 1
    let step = t.step
    return step(t.src)
}

// An iterator over the first [n] elements of [it].
// @compilerrules [seq-lazy]
export fn taking<It canbe linear, T>(it: It, n: Int, ?Next<It, T>) [] -> Mut Taking<It, T> => !it, !n {
    return Mut Taking<It, T> { src: it, step: copy(next), left: n }
}

// Ends the adaptor and answers the iterator it wraps, at the position it
// reached: how an adaptor over an obligation ends (`close(detach(t))`), and
// how a caller reads on past it.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T>(it: Taking<It, T>) [] -> It => !it {
    return it.src
}

// The source's elements while a predicate accepts them.
// @compilerrules [seq-lazy]
export struct TakingWhile<It canbe linear, T, [E]> : Next<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    keep: (x: T) [E] -> Bool,
    done: Bool
}

export fn next<It canbe linear, T, [E]>(t: Mut TakingWhile<It, T, [E]>) [E] -> Emitted T | Finished => t: Mut {
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

// An iterator over the elements of [it] up to the first one
// [keep] refuses (which is consumed from the source and not emitted).
// @compilerrules [seq-lazy]
export fn taking_while<It canbe linear, T, [E]>(it: It, keep: (x: T) [E] -> Bool, ?Next<It, T>) [] -> Mut TakingWhile<It, T, [E]>
=> !it, !keep {
    return Mut TakingWhile<It, T, [E]> { src: it, step: copy(next), keep: keep, done: false }
}

// Ends the adaptor and answers the iterator it wraps, at the position it
// reached: how an adaptor over an obligation ends (`close(detach(t))`), and
// how a caller reads on past it.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, [E]>(it: TakingWhile<It, T, [E]>) [] -> It => !it {
    return it.src
}

// The source's elements after the first few.
// @compilerrules [seq-lazy]
export struct Skipping<It canbe linear, T> : Next<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    left: Int
}

export fn next<It canbe linear, T>(t: Mut Skipping<It, T>) [] -> Emitted T | Finished => t: Mut {
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

// An iterator over the elements of [it] after its first [n].
// @compilerrules [seq-lazy]
export fn skipping<It canbe linear, T>(it: It, n: Int, ?Next<It, T>) [] -> Mut Skipping<It, T> => !it, !n {
    return Mut Skipping<It, T> { src: it, step: copy(next), left: n }
}

// Ends the adaptor and answers the iterator it wraps, at the position it
// reached: how an adaptor over an obligation ends (`close(detach(t))`), and
// how a caller reads on past it.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T>(it: Skipping<It, T>) [] -> It => !it {
    return it.src
}

// The source's elements from the first one a predicate refuses.
// @compilerrules [seq-lazy]
export struct SkippingWhile<It canbe linear, T, [E]> : Next<self, T> canbe Mut {
    src: It,
    step: (s: Mut It) -> Emitted T | Finished,
    skip: (x: T) [E] -> Bool,
    started: Bool
}

export fn next<It canbe linear, T, [E]>(t: Mut SkippingWhile<It, T, [E]>) [E] -> Emitted T | Finished => t: Mut {
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

// An iterator over the elements of [it] from the first one [skip]
// refuses.
// @compilerrules [seq-lazy]
export fn skipping_while<It canbe linear, T, [E]>(it: It, skip: (x: T) [E] -> Bool, ?Next<It, T>) [] -> Mut SkippingWhile<It, T, [E]>
=> !it, !skip {
    return Mut SkippingWhile<It, T, [E]> { src: it, step: copy(next), skip: skip, started: false }
}

// Ends the adaptor and answers the iterator it wraps, at the position it
// reached: how an adaptor over an obligation ends (`close(detach(t))`), and
// how a caller reads on past it.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, [E]>(it: SkippingWhile<It, T, [E]>) [] -> It => !it {
    return it.src
}

// Drives [it] to its end, answering its elements as a list — a
// view of a borrowing source's elements, as [filter]'s answer is. A
// container's own `to_list` is more specific, and wins.
// @compilerrules [seq-lazy, fn-overload-rank]
export fn to_list<It canbe linear, T>(it: Mut It, ?Next<It, T>) [] -> Mut List<proj T> holds proj(it) => it: Mut {
    let out: Mut List<proj T> = Mut List()
    for x in it {
        add(out, x)
    }
    return out
}

// Drives [it] to its end, answering how many elements it had. A
// `count` of the program's own over a list is more specific and wins.
// @compilerrules [seq-lazy]
export fn count<It canbe linear, T>(it: Mut It, ?Next<It, T>) [] -> Int => it: Mut {
    let n = 0
    for _x in it {
        n = n + 1
    }
    return n
}
