// Indices proven to be in range. `Idx(xs) Int` is an `Int` that the
// compiler knows is a valid index of the container `xs`, so reading `xs` at
// it cannot fail and answers the element itself rather than an optional.
//
// A container's plain `get(xs, i)` answers `T?`, `None` when `i` is out of
// range. Testing `i is Idx(xs)` once puts the fact in `i`'s type, and from
// then on `get(xs, i)` picks the **total** overload, which answers a `T`.
// The iterators [indices] and [rev_indices] answer indices that already
// carry the claim, so a loop over positions needs no test at all:
//
// ```
// let xs = List(10, 20, 30)
// let total = 0
// for i in indices(xs) {
//     total = total + get(xs, i)                       // an Int, not an Int?
// }
//
// let i = 2
// if i is Idx(xs) {
//     println("${get(xs, i)}")                         // 30
// }
// ```
//
// **The claim is about one container.** `Idx(xs) Int` and `Idx(ys) Int` are
// different facts, even when the lists have the same size. Another name
// for the same list (an alias) is still `xs`; a copy of it is a different
// list.
//
// **Changing the container takes the claim away**, conservatively: after any
// call that may modify `xs`, an `Idx(xs)` index is a plain `Int` again and
// must be tested anew. The exceptions are the changes that cannot move the
// boundary: adding to a list or a deque (growth keeps every index valid),
// swapping two elements, and writing in place with `replace`. Those keep
// every `Idx` claim.
//
// ```
// let xs: Mut List<Int> = Mut List(1, 2, 3)
// let i = 1
// assert!(i is Idx(xs))
// add(xs, 4)                                           // keeps the claim
// println("${get(xs, i)}")                             // 2
// ```
//
// One `Idx` serves every container with a `size`: `List`, `Deque` and the
// integer buffers. The claim exists only at compile time; at run time an
// `Idx(xs) Int` is an `Int`, and the total `get` reads without a bounds
// check of its own.
// @compilerrules [col-idx]

// The claim that an `Int` is a valid index of one particular container:
// `0 <= index < size(c)`.
//
// Tested with `i is Idx(xs)` (or `assert!(i is Idx(xs))`), produced by
// [indices] and [rev_indices], and consumed by the total `get`, `swap` and
// `replace` of each container. Any change to the container strips it, except
// the ones listed here as keeping it.
//
// It works for any container with a `size` function, found where the claim
// is tested, so the type reads `Idx(xs) Int` whatever `xs` is. A container
// of linear values (values that must be used exactly once) keeps its claims
// too.
// @compilerrules [qual-depend, col-idx]
export qualifier Idx<C canbe linear>(c: C) for Int {
    // Whether `0 <= index < size(c)`.
    //
    // @param size  found where the claim is tested: the container's size
    fn qualifies(index: Int, c: C, ?size: (c: C) -> Int) -> Bool {
        return index >= 0 && index < size(c)
    }

    // The changes that keep every `Idx` claim on the container, because they
    // cannot move the boundary: growth keeps every existing index valid, and
    // an exchange or a write in place moves nothing. Every other change to
    // the container strips the claims. Each container's refinements live
    // here, with the claim, since only the claim's owner may say that a call
    // keeps it.
    // @compilerrules [qual-preserve, qual-refn-scope]
    refn add<T>(list: Mut List<T>, elem: T) => list: preserve Idx
    refn append<T>(list: Mut List<T>, items: T[]) => list: preserve Idx
    refn swap<T>(list: Mut List<T>, i: Int, j: Int) => list: preserve Idx
    // The total write keeps them too.
    // @compilerrules [col-replace]
    refn replace<T>(list: Mut List<T>, index: Idx(list) Int, value: T) => list: preserve Idx
    // Growth at either end of a deque keeps every index in range: at the
    // front it is the *element* an index names that moves, not the bound.
    // @compilerrules [col-deque]
    refn add_last<T>(d: Mut Deque<T>, elem: T) => d: preserve Idx
    refn add_first<T>(d: Mut Deque<T>, elem: T) => d: preserve Idx
    refn replace<T>(d: Mut Deque<T>, index: Idx(d) Int, value: T) => d: preserve Idx
    // A buffer's length never changes.
    // @compilerrules [col-buffer]
    refn replace(buf: Mut IntBuffer, index: Idx(buf) Int, value: Int) => buf: preserve Idx
    refn clear(buf: Mut IntBuffer, fill: Int) => buf: preserve Idx
    refn replace(buf: Mut LongBuffer, index: Idx(buf) Int, value: Long) => buf: preserve Idx
    refn clear(buf: Mut LongBuffer, fill: Long) => buf: preserve Idx
}

// Every index of a list, front to back, each carrying the claim that it is
// in range: `for i in indices(xs)` makes `get(xs, i)` answer the element.
//
// The iterator borrows the list, so nothing can shrink the list while the
// loop runs, which is what makes the claim sound. In the loop body the
// claim reads `Idx(xs)`, naming the list the loop walks.
//
// ```
// let xs = List(5, 6)
// let digits = 0
// for i in indices(xs) {
//     digits = digits * 10 + get(xs, i)
// }
// // digits is 56
// ```
// @compilerrules [col-idx, proj-field, proj-infer, qual-depend, deduce-reapply]
export fn indices<T>(list: List<T>) [] -> Mut IndicesIter<T> => list {
    // The `+Idx` on `next` is an establishment in a return-type arm,
    // trusted in the file that declares `Idx` and only there, which is why
    // the indices live here rather than in `std.core.list`; the bounds test
    // in `next` is the proof the trust rests on. The claim names the
    // iterator's borrowed field (`Idx(p.list)`), and a `for` binds it to the
    // *source* list's identity, so it reads `Idx(xs)` in the loop body.
    return Mut IndicesIter<T> { list: list, at: 0, step: 1 }
}

// The iterator [indices] and [rev_indices] answer: the list, borrowed, the
// index the next turn emits, and the direction.
export struct IndicesIter<T> : Next<self, Idx(self.list) Int> canbe Mut {
    // The list whose indices are emitted, borrowed.
    // @compilerrules [proj-field, proj-infer]
    list: proj List<T>
    // The index the next turn emits.
    at: Int
    // `1` front to back, `-1` back to front.
    step: Int
}

// Advances: the next index, proven in range, or the end.
// @compilerrules [col-idx, deduce-reapply]
export fn next<T>(p: Mut IndicesIter<T>) [] -> Emitted (+Idx(p.list) Int) | Finished => p: Mut {
    if p.at < 0 || p.at >= size(p.list) {
        return finished()
    }
    let index = p.at.copy()
    p.at = p.at + p.step
    return emitted(index)
}

// Every index of a list, back to front, from `size(list) - 1` down to `0`,
// each carrying the claim that it is in range, as for [indices].
//
// ```
// let xs = List(1, 2, 3)
// let digits = 0
// for i in rev_indices(xs) {
//     digits = digits * 10 + get(xs, i)                // no check needed
// }
// // digits is 321
// ```
// @compilerrules [col-idx]
export fn rev_indices<T>(list: List<T>) [] -> Mut IndicesIter<T> => list {
    // The founding example of the refinement-types design: the descending
    // loop whose body needs no `!`.
    return Mut IndicesIter<T> { list: list, at: size(list) - 1, step: -1 }
}
