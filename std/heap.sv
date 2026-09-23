// A binary heap, as a **claim on an ordinary `List<T>`** rather than a
// container of its own: `Heap<T>(?cmp)` says a list is arranged as a succinct
// binary heap under the ordering it was built with, and the mutators here keep
// it that way.
//
// Written outside std first (as `demo/heap.sv`), to find out whether a
// non-trivial qualifier could be written in ordinary Salvo at all. It could, so
// it moved in (user decision 2026-09-23) and is now the first std module with
// tests of its own: `heap.test.sv` beside it, run by `salvo test --src std`.
//
// Not part of `core`, so it arrives by asking: `import heap`.

// Indicates that the list is organized like a succinct binary heap, ordered by
// the `cmp` the heap was built with: `Heap(min_by_age)` and `Heap(max_by_age)`
// are different types that refuse to mix [cmp-carry].
export qualifier Heap<T>(?Ordered<T>) of List<T> with NonEmpty

// Returns an empty List which trivially supports the heap property. The
// ordering arrives as an ordinary implicit parameter, and the return type
// publishes the one resolution chose [cmp-binder].
export fn heap_of<T>(?Ordered<T>) -> +Heap<T>(?cmp) Mut List<T> {
    return mut_list_of()
}

export fn heap_of<T>(first: T, ...elems: T[], ?Ordered<T>) -> Heap<T>(?cmp) NonEmpty Mut List<T> {
    let list = mut_list_of(first, ...elems)
    heapify(list)
    return list
}

// Makes an arbitrary list into a heap. `+Heap<T>(?cmp)` **establishes** the
// claim rather than keeping one [deduce-reapply]: the list arrives with nothing
// claimed about it, and this file declares `Heap`, so it is the party trusted to
// say what a call leaves behind. The ordering the claim carries is the one the
// *call* resolved, so `heapify(xs)` and `heapify(xs, cmp = by_name)` hand back
// two types that refuse to mix.
export fn heapify<T>(list: Mut List<T>, ?Ordered<T>)
=> list: +Heap<T>(?cmp) Mut {
    // An empty list is already a heap, so the work only exists for the
    // non-empty case — and the guard narrows `list` on the way in, which routes
    // to the overload below rather than recursing [is-narrow-guard]
    // [fn-overload-rank]. The same shape `pop` uses.
    if list is NonEmpty {
        heapify(list)
    }
}

// The same for a list already known non-empty — `heap_of`'s path, and the
// reason `NonEmpty` can appear in the clause of a module that does not own the
// claim: arranging a list cannot change *how many* elements it has, so the
// claim is *kept*, never established. It survives the body because `swap`
// strips it (a mutating callee must) and `core.list`'s refinement puts it back
// [qual-refn] — remove that refinement and this signature stops checking,
// which is the whole mechanism in one line.
export fn heapify<T>(list: NonEmpty Mut List<T>, ?Ordered<T>)
=> list: +Heap<T>(?cmp) NonEmpty Mut {
    // Floyd's construction: sift down from the last parent to the root.
    let n = list.size()
    for i in range(n / 2 - 1, -1) {
        heapify_part(list, n, i)
    }
}

// One sift-down, used by [heapify]: pushes the element at [i] down until both
// children are no smaller than it.
//
// The `NonEmpty` in the signature is not something a sift-down needs — it works
// on any list — but a *channel* for the caller's claim: a private mutator whose
// clause is inferred hands back an exhaustive `Mut`, which would swallow it, and
// a claim this module does not own cannot be re-established here. Taking it and
// naming it keeps it, and the exchange preserves it because `core.list` says a
// swap of an *already* non-empty list leaves it non-empty
// [qual-refn-narrow] — a claim kept, not established, which is why it also
// survives the `if` below.
// One sift-down, used by [heapify]: pushes the element at [i] down until both
// children are no smaller than it.
//
// The `NonEmpty` in the signature is not something a sift-down needs — it works
// on any list — but a *channel* for the caller's claim: a private mutator whose
// clause is inferred hands back an exhaustive `Mut`, which would swallow it, and
// a claim this module does not own cannot be re-established here. Taking it and
// naming it keeps it, and the exchange preserves it because `core.list` says
// `swap` does [qual-refn].
// One sift-down, used by [heapify]: pushes the element at [i] down until both
// children are no smaller than it.
//
// The `NonEmpty` in the signature is not something a sift-down needs — it works
// on any list — but a *channel* for the caller's claim: a private mutator whose
// clause is inferred hands back an exhaustive `Mut`, which would swallow it, and
// a claim this module does not own cannot be re-established here. Taking it and
// naming it keeps it, and the exchange preserves it because `core.list` says
// `swap` does [qual-refn].
// One sift-down, used by [heapify]: pushes the element at [i] down until both
// children are no smaller than it.
//
// The `NonEmpty` in the signature is not something a sift-down needs — it works
// on any list — but a *channel* for the caller's claim: a private mutator whose
// clause is inferred hands back an exhaustive `Mut`, which would swallow it, and
// a claim this module does not own cannot be re-established here. Taking it and
// naming it keeps it, and the exchange preserves it because `core.list` says
// `swap` does [qual-refn].
fn heapify_part<T>(list: NonEmpty Mut List<T>, n: Int, i: Int, ?Ordered<T>)
=> list: NonEmpty Mut, n, i {
    let smallest = i
    let left_child = 2 * i + 1
    let right_child = 2 * i + 2

    if left_child < n && list.get(left_child)! < list.get(smallest)! {
        smallest = left_child
    }

    if right_child < n && list.get(right_child)! < list.get(smallest)! {
        smallest = right_child
    }

    if smallest != i {
        list.swap(smallest, i)
        heapify_part(list, n, smallest)
    }
}

// Returns a projection of the smallest element (by the heap's [cmp]) in the heap, or None
// if the heap is empty.
export fn peek<T>(heap: Heap List<T>) -> proj(heap) T? {
    return heap.get(0)
}

export fn peek<T>(heap: NonEmpty Heap List<T>) -> proj(heap) T {
    return heap.get(0)!
}

// Pushes the [elem] into the [heap], preserving the heap property.
//
// An ordinary mutator: the claim is **re-established** by this function, which
// `+Heap<T>(?cmp)` is how a deduction says so [deduce-reapply]. `add`'s own
// clause strips the claim — a mutating callee must — and nothing but this
// function knows the sift below puts it back. Trusted because this is the file
// that declares `Heap`, the same party a constructor fn and a refinement trust.
export fn push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) -> None
=> heap: +Heap NonEmpty Mut, !elem {
    add(heap, elem)
    // The index where the value currently is
    let i = size(heap) - 1
    while i > 0 {
        // `val` is `proj T?` — a borrow of the element, so reading it is free
        // and moving it is refused. (It hovered as `proj proj T?` until
        // 2026-09-22, which was a hover bug rather than a type.)
        let val = heap.get(i) ?: break

        let i_parent = (i - 1) / 2
        let parent = heap.get(i_parent) ?: break
        if parent <= val {
            break
        }

        // A total exchange, so nothing can be dropped by it; it answers `false`
        // out of range, which cannot happen here [col-bounds].
        heap.swap(i, i_parent)
        i = i_parent
    }
}

// Pops the smallest element in the heap, preserving the heap property.
export fn pop<T>(heap: Heap<T>(?cmp) Mut List<T>) -> T? {
    if heap !is NonEmpty {
        return None
    }
    // The guard narrows `heap` to `NonEmpty` on the way out of the `if`
    // [is-narrow-guard], so this routes to the overload below rather than
    // recursing into this one [fn-overload-rank].
    return pop(heap)
}

// A deduction entry names qualifiers, not their arguments: keeping `Heap` keeps
// the ordering too, since the identity lives in the type and this fn could not
// have changed it [cmp-binder].
export fn pop<T>(heap: NonEmpty Heap<T>(?cmp) Mut List<T>) -> T
=> heap: +Heap<T>(?cmp) Mut {
    if heap.size() == 1 {
        return remove_first@core.list(heap)!
    }
    // Swap them, so that we don't have to shift everything
    heap.swap(0, heap.size() - 1)
    let elem = heap.remove_at(heap.size() - 1)!
    let i = 0
    while i < heap.size() {
        let i_child = i * 2 + 1

        // If there are no children, then we're good
        if i_child >= heap.size() {
            break
        }

        // We want to compare with the smallest of the two child indices, because if we swap this child will become the root
        if i_child + 1 < heap.size() && heap.get(i_child + 1)! < heap.get(i_child)! {
            i_child += 1
        }

        // If the parent is already smaller than the smallest child, then the heap property is preserved
        if heap.get(i)! <= heap.get(i_child)! {
            break
        }

        // Otherwise, we swap and proceed down the new path
        heap.swap(i, i_child)
        i = i_child
    }
    return elem
}

// The language rules this module leans on, each closed while it was written as
// `demo/heap.sv` (COMPLETED.md's decision log has the reasoning for each):
//
//   * the ordering it is kept by lives in the type, bound once per signature
//     with `?cmp` [cmp-carry] [cmp-binder], so two differently-ordered heaps are
//     two types and a caller that never names an ordering gets the canonical one
//     for its element type;
//   * `+Heap<T>(?cmp)` is how a mutator keeps a claim it re-establishes
//     [deduce-reapply], which is what lets `push` and `pop` take `Mut`
//     parameters instead of consuming and returning;
//   * `swap` exists, and answers `false` out of range [col-bounds];
//   * `!is` reads as a guard [is-not], and the narrowing it leaves behind routes
//     the bare `pop` to the `NonEmpty` overload [fn-overload-rank];
//   * `+=` is ordinary arithmetic on a place [op-compound].
