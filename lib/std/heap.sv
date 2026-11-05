// A binary heap, a priority queue: [push] adds an element, and [pop] takes
// out the smallest under the heap's ordering. It is not a container of its
// own but a **claim on an ordinary `List<T>`**: `Heap<T>(cmp)` says the list
// is arranged as a binary heap under the ordering `cmp`, and the functions
// here keep it that way.
//
// [push] and [pop] take O(log n), [peek] O(1), and [heapify] arranges a
// whole list in O(n). Not part of `std.core`, so it arrives by asking:
// `import std.heap`.
//
// ```
// let tasks = heap_of<Int>()
// push(tasks, 5)
// push(tasks, 1)
// push(tasks, 9)
// let first = pop(tasks)                   // 1, an Int: a push made it non-empty
// let next = pop(tasks) ?: 0               // 5; after a pop it may be empty
// let top = peek(tasks)                    // 9, as an Int?
// ```
//
// **The ordering is in the type.** A heap is ordered by its element type's
// `cmp`, found where it is made, or by the one passed as `cmp = ...`, which
// is how a max-heap is made. Heaps ordered differently are different types
// and refuse to mix, so [push] and [pop] always work in the order the heap
// was built with.
//
// ```
// fn desc(a: Int, b: Int) -> Int {
//     return cmp(b, a)
// }
//
// fn example() -> None {
//     let biggest = heap_of(3, 8, 5, cmp = desc)
//     let eight = pop(biggest)             // 8: smallest under desc
// }
// ```
//
// **Still a list.** A heap is a `Mut List<T>`, so the list functions read
// it (`size`, a `for` loop, printing), in heap order rather than sorted
// order. Changing it with anything but [push] and [pop] gives the claim up,
// and the heap functions then refuse the list until [heapify] arranges it
// again.
//
// **Known non-empty.** [heap_of] with elements, and a heap just pushed to,
// are `NonEmpty`, and [peek] and [pop] then answer the element itself
// rather than an optional.

// The claim that a list is arranged as a binary heap under the ordering
// `cmp`.
//
// Established by [heap_of] and [heapify], kept by [push] and [pop], and
// given up by any other change to the list. `Heap<T>(by_age)` and
// `Heap<T>(by_name)` are different types; a signature that accepts any
// ordering writes `Heap List<T>`.
// @compilerrules [cmp-carry]
// @compilerknown
export qualifier Heap<T>(?Ordered<T>) for List<T>

// Makes an empty heap, a list that trivially has the heap property.
//
// The ordering is the element type's `cmp`, found at the call site, or the
// one passed as `cmp = ...`; the answer's type carries it.
//
// ```
// let h = heap_of<Int>()
// push(h, 3)
// ```
// @compilerrules [cmp-binder]
export fn heap_of<T>(?Ordered<T>) -> +Heap<T>(?cmp) Mut List<T> {
    return Mut List()
}

// [heap_of] with elements: a heap of them, known non-empty.
//
// ```
// let h = heap_of(5, 1, 9)
// let one = peek(h)                        // 1, an Int
// ```
export fn heap_of<T>(first: T, ...elems: T[], ?Ordered<T>) -> Heap<T>(?cmp) NonEmpty Mut List<T> => !first, !elems {
    let list = Mut List(first, ...elems)
    heapify(list)
    return list
}

// Arranges a list as a heap, in place, and gives it the claim.
//
// O(n). The ordering is the element type's `cmp`, found at the call site,
// or the one passed as `cmp = ...`; `heapify(xs)` and
// `heapify(xs, cmp = by_name)` leave `xs` with two different types.
//
// ```
// let xs: Mut List<Int> = Mut List(4, 2, 8, 1)
// heapify(xs)
// let one = pop(xs) ?: 0                   // 1
// ```
// @compilerrules [deduce-reapply]
export fn heapify<T>(list: Mut List<T>, ?Ordered<T>)
=> list: +Heap<T>(?cmp) Mut {
    // `+Heap<T>(?cmp)` **establishes** the claim rather than keeping one: the
    // list arrives with nothing claimed about it, and this file declares
    // `Heap`, so it is the party trusted to say what a call leaves behind.
    //
    // An empty list is already a heap, so the work only exists for the
    // non-empty case — and the guard narrows `list` on the way in, which routes
    // to the overload below rather than recursing [is-narrow-guard]
    // [fn-overload-rank]. The same shape `pop` uses.
    if list is NonEmpty {
        heapify(list)
    }
}

// [heapify] of a list known non-empty, which stays known non-empty.
// @compilerrules [qual-refn]
export fn heapify<T>(list: NonEmpty Mut List<T>, ?Ordered<T>)
=> list: +Heap<T>(?cmp) NonEmpty Mut {
    // `heap_of`'s path, and the reason `NonEmpty` can appear in the clause of
    // a module that does not own the claim: arranging a list cannot change
    // *how many* elements it has, so the claim is *kept*, never established.
    // It survives the body because `swap` strips it (a mutating callee must)
    // and `std.core.list`'s refinement puts it back; remove that refinement
    // and this signature stops checking, which is the whole mechanism in one
    // line.
    //
    // Floyd's construction: sift down from the last parent to the root.
    let n = list.size()
    for i in range(n / 2 - 1, -1) {
        heapify_part(list, n, i)
    }
}

// One sift-down, used by [heapify]: pushes the element at [i] down until both
// children are no smaller than it.
// @compilerrules [qual-refn-narrow, qual-refn]
fn heapify_part<T>(list: NonEmpty Mut List<T>, n: Int, i: Int, ?Ordered<T>)
=> list: NonEmpty Mut, n, i {
    // The `NonEmpty` in the signature is not something a sift-down needs (it
    // works on any list) but a *channel* for the caller's claim: a private
    // mutator whose clause is inferred hands back an exhaustive `Mut`, which
    // would swallow it, and a claim this module does not own cannot be
    // re-established here. Taking it and naming it keeps it, and the exchange
    // preserves it because `std.core.list` says a swap of an *already*
    // non-empty list leaves it non-empty: a claim kept, not established,
    // which is why it also survives the `if` below.
    let smallest = i
    let left_child = 2 * i + 1
    let right_child = 2 * i + 2

    if left_child < n && (list.get(left_child) ?: unreachable!()) < (list.get(smallest) ?: unreachable!()) {
        smallest = left_child
    }

    if right_child < n && (list.get(right_child) ?: unreachable!()) < (list.get(smallest) ?: unreachable!()) {
        smallest = right_child
    }

    if smallest != i {
        // In range by construction (`smallest` is `i`, a child index, or one
        // below `n`), so there is nothing to check [col-bounds].
        ignore(list.swap(smallest, i))
        heapify_part(list, n, smallest)
    }
}

// The smallest element under the heap's ordering, borrowed, or `None` when
// the heap is empty.
//
// O(1): the smallest element is always at the front. On a heap known to be
// `NonEmpty`, the overload below answers the element itself.
//
// ```
// let h = heap_of<Int>()
// let none = peek(h)                       // None
// push(h, 4)
// let four = peek(h)                       // 4, an Int
// ```
export fn peek<T>(heap: Heap List<T>) -> proj(heap) T? => heap {
    return heap.get(0)
}

// [peek] of a heap known non-empty: answers the element itself.
export fn peek<T>(heap: NonEmpty Heap List<T>) -> proj(heap) T => heap {
    return heap.get(0) ?: unreachable!()
}

// Adds an element to the heap, keeping the heap property.
//
// O(log n). The element is moved in, and the heap keeps its claim and is
// known non-empty afterwards, so a [pop] straight after answers an element
// rather than an optional.
//
// ```
// let h = heap_of<Int>()
// push(h, 7)
// let seven = pop(h)                       // 7, an Int
// ```
// @compilerrules [deduce-reapply]
export fn push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) -> None
=> heap: +Heap NonEmpty Mut, !elem {
    // An ordinary mutator: the claim is **re-established** by this function,
    // which `+Heap<T>(?cmp)` is how a deduction says so. `add`'s own clause
    // strips the claim (a mutating callee must), and nothing but this
    // function knows the sift below puts it back. Trusted because this is the
    // file that declares `Heap`, the same party a constructor fn and a
    // refinement trust.
    add(heap, elem)
    // The index where the value currently is
    let i = size(heap) - 1
    while i > 0 {
        // `val` is `proj T?` — a borrow of the element, so reading it is free
        // and moving it is refused.
        assert!(i is Idx(heap))
        let val = heap.get(i)

        let i_parent = (i - 1) / 2
        assert!(i_parent is Idx(heap))
        let parent = heap.get(i_parent)
        if parent <= val {
            break
        }

        // Nothing can be dropped by an exchange, and out of range cannot happen
        // here — so the answer is `ignore`d rather than read [col-bounds].
        heap.swap(i, i_parent)
        i = i_parent
    }
}

// Takes the smallest element under the heap's ordering out of the heap, or
// answers `None` when it is empty.
//
// O(log n). The element is moved out, and the heap keeps its claim. On a
// heap known to be `NonEmpty`, the overload below answers the element itself;
// after a pop the heap may be empty, so the next pop answers an optional.
//
// ```
// let h = heap_of(5, 1, 9)
// let one = pop(h)                         // 1, an Int
// let five = pop(h) ?: 0                   // 5
// ```
export fn pop<T>(heap: Heap<T>(?cmp) Mut List<T>) -> T? => heap: Mut Heap {
    if heap !is NonEmpty {
        return None
    }
    // The guard narrows `heap` to `NonEmpty` on the way out of the `if`
    // [is-narrow-guard], so this routes to the overload below rather than
    // recursing into this one [fn-overload-rank].
    return pop(heap)
}

// [pop] of a heap known non-empty: answers the element itself.
// @compilerrules [cmp-binder]
export fn pop<T>(heap: NonEmpty Heap<T>(?cmp) Mut List<T>) -> T
=> heap: +Heap<T>(?cmp) Mut {
    // A deduction entry names qualifiers, not their arguments: keeping `Heap`
    // keeps the ordering too, since the identity lives in the type and this
    // fn could not have changed it.
    if heap.size() == 1 {
        return remove_first(heap) ?: unreachable!()
    }
    // Swap them, so that we don't have to shift everything. Both indices are in
    // range (the one-element heap returned above), so the answer is `ignore`d.
    ignore(heap.swap(0, heap.size() - 1))
    let elem = heap.remove_at(heap.size() - 1) ?: unreachable!()
    // The root is a valid index (the early return above handled the
    // one-element heap, and `remove_at` left at least one element), so from
    // here every read in the sift is the total `get` [col-idx] — no `!`
    // anywhere.
    let i = 0
    if i !is Idx(heap) { return elem }
    while i < heap.size() {
        // If there are no children, then we're done.
        let i_left = i * 2 + 1
        if i_left !is Idx(heap) { break }

        // We want to compare with the smallest of the two child indices,
        // because if we swap this child will become the root. Both candidates
        // carry the `Idx` claim — `i_left` from the guard above, `i_right`
        // from the `is Idx` test — so the `if`-expression keeps it.
        let i_right = i_left + 1
        let i_child = if i_right is Idx(heap) && heap.get(i_right) < heap.get(i_left) {
            i_right
        } else {
            i_left
        }

        // If the parent is already smaller than the smallest child, then the
        // heap property is preserved
        if heap.get(i) <= heap.get(i_child) {
            break
        }

        // Otherwise, we swap and proceed down the new path — the **total**
        // exchange, since both indices carry the claim, so there is no answer to
        // check and it preserves the claims [qual-preserve].
        heap.swap(i, i_child)
        i = i_child
    }
    return elem
}

// The language rules this module leans on, each closed while it was written
// as `demo/heap.sv` (COMPLETED.md's decision log has the reasoning for each).
// It was written outside std first, to find out whether a non-trivial
// qualifier could be written in ordinary Salvo at all; it could, so it moved
// in, the first std module with tests of its own (`heap.test.sv`, run by
// `salvo test --src lib`).
//
//   * the ordering it is kept by lives in the type, bound once per signature
//     with `?cmp`, so two differently-ordered heaps are two types and a
//     caller that never names an ordering gets the canonical one for its
//     element type;
//   * `+Heap<T>(?cmp)` is how a mutator keeps a claim it re-establishes,
//     which is what lets `push` and `pop` take `Mut` parameters instead of
//     consuming and returning;
//   * `swap` exists, and answers `false` out of range;
//   * `!is` reads as a guard, and the narrowing it leaves behind routes the
//     bare `pop` to the `NonEmpty` overload;
//   * `+=` is ordinary arithmetic on a place.
// @compilerrules [cmp-carry, cmp-binder, deduce-reapply, col-bounds, is-not, fn-overload-rank, op-compound]
