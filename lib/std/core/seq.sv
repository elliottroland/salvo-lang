// Functions that walk an iterator: transform its elements, keep some of
// them, fold them into one value, or wrap the iterator in another that does
// so as it is driven.
//
// Every function here takes an **iterator**, not a container. A list is
// passed as `iter(xs)`, a map as `iter(m)`, and anything else a `for` loop
// can drive (the result of an `iter fn`, or a type of your own that
// declares `: Next<self, T>`) is passed as it is. The iterator's `next`
// travels with the call as an implicit parameter, so you never write it. If
// that `next` performs effects, the call performs them too, together with
// the effects of the callback you pass.
//
// The functions come in two families:
//
// - **Eager** functions run the whole pass when called. [map] and [filter]
//   answer a new list, [reduce] a single value, [to_list] and [count] what
//   their names say; [map_to] and [filter_to] put their results into a
//   collection you provide.
// - **Lazy** adaptors are named by a present participle: [mapping],
//   [filtering], [taking], [taking_while], [skipping] and [skipping_while].
//   Each answers a new iterator and does nothing until that iterator is
//   driven; every step then pulls one element from the source. They work on
//   a source that never ends, and a `break` out of the loop stops the source
//   too.
//
// ```
// let xs = List(1, 2, 3, 4, 5)
// let squares = map(xs, (x) -> x * x)                  // [1, 4, 9, 16, 25]
// let total = reduce(xs, 0, (acc, x) -> acc + x)       // 15
// for x in taking(filtering(iter(xs), (x) -> x % 2 == 1), 2) {
//     println("${x}")                                  // 1, then 3
// }
// ```
//
// [map], [filter] and [reduce] also take a `List` directly, as `map(xs, f)`
// above does: the most common case keeps the short spelling.
//
// **Iterators are advanced in place.** An eager function drives the
// iterator it is given and leaves it where it stopped, so a caller can read
// part of an iterator one way and the rest another. An adaptor takes its
// source over; [detach] ends an adaptor and answers the source, at the
// position the adaptor reached:
//
// ```
// let xs = List(1, 2, 3, 4, 5)
// let first = taking(iter(xs), 2)
// for x in first {
//     println("${x}")                                  // 1, then 2
// }
// let rest = detach(first)
// println("${count(rest)} more")                       // 3 more
// ```
//
// **Results borrow, unless the name says otherwise.** [filter] and [to_list]
// answer a list of the source's own elements rather than copies of them.
// Nothing is copied, and the list can live no longer than whatever the
// iterator walks. [filter_to] copies each element it keeps into the
// collection you give it.
// @compilerrules [seq-iterator, iter-protocol, implicit-group, seq-lazy, iter-generic-drive]

// Applies a function to every element of an iterator, in order, and
// collects the results in a new list.
//
// ```
// let words = List("a", "bb", "ccc")
// let lengths = map(iter(words), (w: Str) -> size(w))  // [1, 2, 3]
// ```
//
// @param it  the iterator; it is driven to its end
// @param f   the function applied to each element
// @returns   a new list of the results, in the iterator's order
export fn map<It canbe linear, T, U, [E]>(it: Mut It, f: (T) [E] -> U, ?Next<It, T, [E]>) -> Mut List<U> => it: Mut, f {
    let out = Mut List<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}

// The elements of an iterator that a predicate accepts, in order.
//
// The answer is a **view**: it holds the source's own elements, not
// copies, so it can live no longer than whatever the iterator walks. To
// keep copies in a collection of your own, use [filter_to].
//
// ```
// let xs = List(1, 2, 3, 4)
// let even = filter(iter(xs), (x) -> x % 2 == 0)       // [2, 4]
// ```
//
// @param it    the iterator; it is driven to its end
// @param keep  answers `true` for an element to keep
// @returns     the kept elements, borrowed from the source
// @compilerrules [copy-opt-in, proj-infer]
export fn filter<It canbe linear, T, [E]>(
    it: Mut It,
    keep: (T) [E] -> Bool,
    ?Next<It, T, [E]>
) -> Mut List<proj T> holds proj(it)
=> it: Mut, keep {
    // `holds proj(it)` is the written lend: a generic body cannot show the
    // analysis that an element of an opaque iterator is stored, so the
    // signature says it.
    let out: Mut List<proj T> = Mut List()
    for x in it {
        if keep(x) {
            add(out, x)
        }
    }
    return out
}

// Folds an iterator into a single value: starting from an initial value,
// each element is combined with the result so far.
//
// ```
// let xs = List(1, 2, 3, 4)
// let total = reduce(iter(xs), 0, (acc, x) -> acc + x)       // 10
// let largest = reduce(xs, 0, (acc, x) -> if x > acc { x } else { acc })   // 4
// ```
//
// @param it    the iterator; it is driven to its end
// @param init  the result for an empty iterator, and the starting point
// @param f     combines the result so far (first) with the next element
// @returns     the last result `f` produced, or `init`
export fn reduce<It canbe linear, T, A, [E]>(
    it: Mut It,
    init: A,
    f: (A, T) [E] -> A,
    ?Next<It, T, [E]>
) -> A => it: Mut, init, f {
    let acc = init
    for x in it {
        acc = f(acc, x)
    }
    return acc
}

// [map] over a list itself: `map(xs, f)` is `map(iter(xs), f)`.
// @compilerrules [fn-overload-rank]
export fn map<T, U, [E]>(list: List<T>, f: (T) [E] -> U) -> Mut List<U> => list, f {
    // The `List` fast paths: same names, one less indirection. A backend
    // lowers these to its own collection operation, and overload specificity
    // picks them when the subject really is a `List`.
    let out = Mut List<U>()
    for x in list {
        add(out, f(x))
    }
    return out
}

// [filter] over a list itself: `filter(xs, keep)` is `filter(iter(xs),
// keep)`, and its answer borrows from `xs`.
export platform fn filter<T, [E]>(list: List<T>, keep: (T) [E] -> Bool) -> Mut List<proj T> holds proj(list)
=> list, keep

// [reduce] over a list itself: `reduce(xs, init, f)` is
// `reduce(iter(xs), init, f)`.
export fn reduce<T, A, [E]>(list: List<T>, init: A, f: (A, T) [E] -> A) -> A => list, f, !init {
    let acc = init
    for x in list {
        acc = f(acc, x)
    }
    return acc
}

// ===== mapping into a collection you provide =====

// Applies a function to every element of an iterator and adds the results
// to a collection you provide, which it hands back.
//
// The collection comes first because it is what the call is about, and it
// is handed back so that a call can feed another. It need not be a `List`:
// any type with an `add(dest, elem)` the call site can see will do.
//
// ```
// let xs = List(1, 2, 3)
// let doubled = map_to(Mut List<Int>(), iter(xs), (x) -> x * 2)   // [2, 4, 6]
// ```
//
// @param dest  the collection the results are added to
// @param it    the iterator; it is driven to its end
// @param f     the function applied to each element
// @param add   found at the call site: adds one result to `dest`
// @returns     `dest`, with the results added in the iterator's order
// @compilerrules [seq-into, implicit-group]
export fn map_to<D, It, T, U>(
    dest: Mut D,
    it: Mut It,
    f: (T) -> U,
    ?add: (dest: Mut D, elem: U) -> None,
    ?Next<It, T>
) [] -> Mut D => !dest, it: Mut, f =>[add] dest: Mut, !elem {
    for x in it {
        add(dest, f(x))
    }
    return dest
}

// Adds a **copy** of each element of an iterator that a predicate accepts
// to a collection you provide, which it hands back.
//
// Unlike [filter], whose answer borrows the source's elements, this one
// copies: the collection owns what it is given. The copy is the element
// type's own `copy`, found at the call site.
//
// ```
// let xs = List(1, 2, 3, 4, 5)
// let big = filter_to(Mut List<Int>(), iter(xs), (x: Int) -> x > 2)   // [3, 4, 5]
// ```
//
// @param dest  the collection the kept elements are added to
// @param it    the iterator; it is driven to its end
// @param keep  answers `true` for an element to keep
// @param add   found at the call site: adds one element to `dest`
// @param copy  found at the call site: copies one element
// @returns     `dest`, with the kept elements added in the iterator's order
// @compilerrules [seq-into, copy-opt-in, copy-implicit]
export fn filter_to<D, It, T>(
    dest: Mut D,
    it: Mut It,
    keep: (T) -> Bool,
    ?add: (dest: Mut D, elem: T) -> None,
    ?copy: (v: T) -> T,
    ?Next<It, T>
) [] -> Mut D => !dest, it: Mut, keep =>[add] dest: Mut, !elem {
    for x in it {
        if keep(x) {
            add(dest, copy(x))
        }
    }
    return dest
}

// Discards a value. Pass it where a function asks for a callback that
// consumes each value, and you have nothing to do with them.
//
// Only a value that owes nothing can be dropped: for a linear value, pass
// the function that discharges it instead (a reply's `reply`, a file's
// `close`). `drop(v)` on a linear value is a compile-time error, because
// dropping it would leave its obligation undischarged.
// @compilerrules [linear-generics]
export fn drop<T>(value: T) [] -> None => !value {
    // Deliberately *without* `canbe linear`: a linear argument is refused by
    // the instantiation ban, which is exactly the protection.
}

// ===== lazy adaptors =====
//
// Each adaptor is a struct holding the source iterator and the source's
// `next` (the `?Next` implicit, kept as a fn field), whose own `next` pulls
// from the source one element at a time. The source is moved into the
// adaptor; one over a list's `iter` borrows the list, as the pass itself
// does.
// @compilerrules [seq-lazy]

// The iterator [mapping] answers: the elements of its source, each passed
// through a function as it is reached.
// @compilerrules [seq-lazy]
export struct Mapping<It canbe linear, T, U, [E]> : Next<self, U, [E]> canbe Mut {
    src: It
    step: (s: Mut It) [E] -> Emitted T | Finished
    f: (x: T) [E] -> U
}

// Pulls the next element from the source and answers it mapped.
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

// An iterator over the results of applying a function to each element of
// another iterator. The function runs as the result is driven, once per
// element reached: the lazy form of [map].
//
// ```
// let xs = List(1, 2, 3)
// for y in mapping(iter(xs), (x) -> x * 10) {
//     println("${y}")                                  // 10, 20, 30
// }
// ```
//
// @param it  the source, which the adaptor takes over (see [detach])
// @param f   the function applied to each element
// @compilerrules [seq-lazy]
export fn mapping<It canbe linear, T, U, [E]>(
    it: It,
    f: (x: T) [E] -> U,
    ?Next<It, T, [E]>
) [] -> Mut Mapping<It, T, U, [E]> => !it, !f {
    return Mut Mapping<It, T, U, [E]> { src: it, step: copy(next), f: f }
}

// Ends a [Mapping] and answers its source, at the position it reached.
//
// This is how a caller reads on past an adaptor, and how an adaptor over a
// source that must be closed is ended: `close(detach(m))`.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, U, [E]>(it: Mapping<It, T, U, [E]>) [] -> It => !it {
    return it.src
}

// The iterator [filtering] answers: the elements of its source that a
// predicate accepts.
// @compilerrules [seq-lazy]
export struct Filtering<It canbe linear, T, [E]> : Next<self, T, [E]> canbe Mut {
    src: It
    step: (s: Mut It) [E] -> Emitted T | Finished
    keep: (x: T) [E] -> Bool
}

// Pulls elements from the source until the predicate accepts one, and
// answers it.
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

// An iterator over the elements of another iterator that a predicate
// accepts: the lazy form of [filter].
//
// ```
// let xs = List(1, 2, 3, 4)
// for x in filtering(iter(xs), (x) -> x > 2) {
//     println("${x}")                                  // 3, then 4
// }
// ```
//
// @param it    the source, which the adaptor takes over (see [detach])
// @param keep  answers `true` for an element to emit
// @compilerrules [seq-lazy]
export fn filtering<It canbe linear, T, [E]>(
    it: It,
    keep: (x: T) [E] -> Bool,
    ?Next<It, T, [E]>
) [] -> Mut Filtering<It, T, [E]> => !it, !keep {
    return Mut Filtering<It, T, [E]> { src: it, step: copy(next), keep: keep }
}

// Ends a [Filtering] and answers its source, at the position it reached.
//
// This is how a caller reads on past an adaptor, and how an adaptor over a
// source that must be closed is ended: `close(detach(f))`.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, [E]>(it: Filtering<It, T, [E]>) [] -> It => !it {
    return it.src
}

// The iterator [taking] answers: the first few elements of its source.
// @compilerrules [seq-lazy]
export struct Taking<It canbe linear, T, [E]> : Next<self, T, [E]> canbe Mut {
    src: It
    step: (s: Mut It) [E] -> Emitted T | Finished
    left: Int
}

// Answers the source's next element, or the end once enough have been
// taken. The source is not touched after that.
export fn next<It canbe linear, T, [E]>(t: Mut Taking<It, T, [E]>) [E] -> Emitted T | Finished => t: Mut {
    if t.left <= 0 {
        return finished()
    }
    t.left = t.left - 1
    let step = t.step
    return step(t.src)
}

// An iterator over the first elements of another iterator, at most a given
// number of them. The source is not read past them, so `taking` is how a
// loop over a source that never ends is made to stop.
//
// ```
// let xs = List(1, 2, 3, 4)
// for x in taking(iter(xs), 2) {
//     println("${x}")                                  // 1, then 2
// }
// ```
//
// @param it  the source, which the adaptor takes over (see [detach])
// @param n   how many elements to emit at most; none if it is 0 or less
// @compilerrules [seq-lazy]
export fn taking<It canbe linear, T, [E]>(it: It, n: Int, ?Next<It, T, [E]>) [] -> Mut Taking<It, T, [E]> => !it, !n {
    return Mut Taking<It, T, [E]> { src: it, step: copy(next), left: n }
}

// Ends a [Taking] and answers its source, at the position it reached.
//
// This is how a caller reads on past an adaptor, and how an adaptor over a
// source that must be closed is ended: `close(detach(t))`.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, [E]>(it: Taking<It, T, [E]>) [] -> It => !it {
    return it.src
}

// The iterator [taking_while] answers: the elements of its source for as
// long as a predicate accepts them.
// @compilerrules [seq-lazy]
export struct TakingWhile<It canbe linear, T, [E]> : Next<self, T, [E]> canbe Mut {
    src: It
    step: (s: Mut It) [E] -> Emitted T | Finished
    keep: (x: T) [E] -> Bool
    done: Bool
}

// Answers the source's next element if the predicate accepts it, and the
// end from the first one it refuses.
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

// An iterator over the elements of another iterator up to the first one a
// predicate refuses. That element is read from the source but not emitted,
// so a source resumed with [detach] continues after it.
//
// ```
// let xs = List(1, 2, 5, 3)
// for x in taking_while(iter(xs), (x) -> x < 4) {
//     println("${x}")                                  // 1, then 2
// }
// ```
//
// @param it    the source, which the adaptor takes over (see [detach])
// @param keep  answers `true` while elements should be emitted
// @compilerrules [seq-lazy]
export fn taking_while<It canbe linear, T, [E]>(
    it: It,
    keep: (x: T) [E] -> Bool,
    ?Next<It, T, [E]>
) [] -> Mut TakingWhile<It, T, [E]>
=> !it, !keep {
    return Mut TakingWhile<It, T, [E]> { src: it, step: copy(next), keep: keep, done: false }
}

// Ends a [TakingWhile] and answers its source, at the position it reached.
//
// This is how a caller reads on past an adaptor, and how an adaptor over a
// source that must be closed is ended: `close(detach(t))`.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, [E]>(it: TakingWhile<It, T, [E]>) [] -> It => !it {
    return it.src
}

// The iterator [skipping] answers: the elements of its source after the
// first few.
// @compilerrules [seq-lazy]
export struct Skipping<It canbe linear, T, [E]> : Next<self, T, [E]> canbe Mut {
    src: It
    step: (s: Mut It) [E] -> Emitted T | Finished
    left: Int
}

// Discards the elements still to be skipped, if any, and answers the
// source's next element.
export fn next<It canbe linear, T, [E]>(t: Mut Skipping<It, T, [E]>) [E] -> Emitted T | Finished => t: Mut {
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

// An iterator over the elements of another iterator after its first few.
// The skipped elements are read from the source when the result is first
// driven, not when `skipping` is called.
//
// ```
// let xs = List(1, 2, 3, 4)
// for x in skipping(iter(xs), 2) {
//     println("${x}")                                  // 3, then 4
// }
// ```
//
// @param it  the source, which the adaptor takes over (see [detach])
// @param n   how many elements to skip; none if it is 0 or less
// @compilerrules [seq-lazy]
export fn skipping<It canbe linear, T, [E]>(
    it: It,
    n: Int,
    ?Next<It, T, [E]>
) [] -> Mut Skipping<It, T, [E]> => !it, !n {
    return Mut Skipping<It, T, [E]> { src: it, step: copy(next), left: n }
}

// Ends a [Skipping] and answers its source, at the position it reached.
//
// This is how a caller reads on past an adaptor, and how an adaptor over a
// source that must be closed is ended: `close(detach(s))`.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, [E]>(it: Skipping<It, T, [E]>) [] -> It => !it {
    return it.src
}

// The iterator [skipping_while] answers: the elements of its source from
// the first one a predicate refuses.
// @compilerrules [seq-lazy]
export struct SkippingWhile<It canbe linear, T, [E]> : Next<self, T, [E]> canbe Mut {
    src: It
    step: (s: Mut It) [E] -> Emitted T | Finished
    skip: (x: T) [E] -> Bool
    started: Bool
}

// On the first call, discards elements until the predicate refuses one
// and answers that one; after that, answers the source's elements as they
// come.
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

// An iterator over the elements of another iterator from the first one a
// predicate refuses, that one included. The predicate is not asked again
// after it first refuses.
//
// ```
// let xs = List(1, 2, 5, 3)
// for x in skipping_while(iter(xs), (x) -> x < 4) {
//     println("${x}")                                  // 5, then 3
// }
// ```
//
// @param it    the source, which the adaptor takes over (see [detach])
// @param skip  answers `true` for a leading element to discard
// @compilerrules [seq-lazy]
export fn skipping_while<It canbe linear, T, [E]>(
    it: It,
    skip: (x: T) [E] -> Bool,
    ?Next<It, T, [E]>
) [] -> Mut SkippingWhile<It, T, [E]>
=> !it, !skip {
    return Mut SkippingWhile<It, T, [E]> { src: it, step: copy(next), skip: skip, started: false }
}

// Ends a [SkippingWhile] and answers its source, at the position it
// reached.
//
// This is how a caller reads on past an adaptor, and how an adaptor over a
// source that must be closed is ended: `close(detach(s))`.
// @compilerrules [seq-lazy, linear-container]
export fn detach<It canbe linear, T, [E]>(it: SkippingWhile<It, T, [E]>) [] -> It => !it {
    return it.src
}

// Drives an iterator to its end and answers its elements as a list.
//
// Like [filter]'s, the answer holds the source's own elements, not copies,
// and can live no longer than whatever the iterator walks. A container's
// own `to_list`, where it has one, is chosen over this one.
//
// ```
// let xs = List(1, 2, 3, 4)
// let rest = skipping(iter(xs), 1)
// let tail = to_list(rest)                             // [2, 3, 4]
// ```
// @compilerrules [seq-lazy, fn-overload-rank]
export fn to_list<It canbe linear, T, [E]>(
    it: Mut It,
    ?Next<It, T, [E]>
) -> Mut List<proj T> holds proj(it) => it: Mut {
    let out: Mut List<proj T> = Mut List()
    for x in it {
        add(out, x)
    }
    return out
}

// Drives an iterator to its end and answers how many elements it had.
//
// ```
// let xs = List(1, 2, 3, 4)
// let big = count(filtering(iter(xs), (x) -> x > 1))   // 3
// ```
// @compilerrules [seq-lazy]
export fn count<It canbe linear, T, [E]>(it: Mut It, ?Next<It, T, [E]>) -> Int => it: Mut {
    let n = 0
    for _x in it {
        n = n + 1
    }
    return n
}
