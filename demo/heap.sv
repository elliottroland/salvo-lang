// An implementation of a binary heap qualifier for the List<T> type when T is orderable. Used as an
// exercise in a more complicated qualifier that the language should be able to support.
//
// STATUS (2026-09-22): **it compiles.** Every gap this file was written to find
// is closed — see the notes at the bottom. It still lives in `demo/` and no test
// builds it; what is tested is the same shape with a driver and asserted output
// on both backends (`rustc_compiles_and_runs_a_heap` and its Kotlin twin).

// Indicates that the list is organized like a succinct binary heap, ordered by
// the `cmp` the heap was built with: `Heap<min_by_age>` and `Heap<max_by_age>`
// are different types that refuse to mix [cmp-carry].
export qualifier Heap<T, ?cmp: (T, T) -> Int> of List<T> with NonEmpty

// Returns an empty List which trivially supports the heap property. The
// ordering arrives as an ordinary implicit parameter, and the return type
// publishes the one resolution chose [cmp-binder].
export fn empty_heap<T>(?cmp: (T, T) -> Int) -> Mut List<T> as Heap<T, ?cmp> {
    return mut_list_of()
}

// Pushes the [elem] into the [heap], preserving the heap property.
//
// An ordinary mutator: the claim is **re-established** by this function, which
// `+Heap<T, ?cmp>` is how a deduction says so [deduce-reapply]. `add`'s own
// clause strips the claim — a mutating callee must — and nothing but this
// function knows the sift below puts it back. Trusted because this is the file
// that declares `Heap`, the same party a constructor fn and a refinement trust.
export fn heap_push<T>(heap: Heap<T, ?cmp> Mut List<T>, elem: T) -> None
    => heap: +Heap<T, ?cmp> Mut, !elem {
    add(heap, elem)
    // The index where the value currently is
    let i = size(heap) - 1
    while i > 0 {
        // `val` is `proj T?` — a borrow of the element, so reading it is free
        // and moving it is refused. (It hovered as `proj proj T?` until
        // 2026-09-22, which was a hover bug rather than a type.)
        let val = heap.get(i)
        if val is None {
            break
        }

        let i_parent = i / 2
        let parent = heap.get(i_parent)
        if parent is None || cmp(parent, val) <= 0 {
            break
        }

        // A total exchange, so nothing can be dropped by it; it answers `false`
        // out of range, which cannot happen here [col-bounds].
        heap.swap(i, i_parent)
        i = copy(i_parent)
    }
}

// Pops the smallest element in the heap, preserving the heap property.
export fn heap_pop<T>(heap: Heap<T, ?cmp> Mut List<T>) -> T? {
    if heap !is NonEmpty {
        return None
    }
    // The guard narrows `heap` to `NonEmpty` on the way out of the `if`
    // [is-narrow-guard], so this routes to the overload below rather than
    // recursing into this one [fn-overload-rank].
    return heap_pop(heap)
}

// A deduction entry names qualifiers, not their arguments: keeping `Heap` keeps
// the ordering too, since the identity lives in the type and this fn could not
// have changed it [cmp-binder].
export fn heap_pop<T>(heap: NonEmpty Heap<T, ?cmp> Mut List<T>) -> T
    => heap: +Heap<T, ?cmp> Mut {
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
        if i_child + 1 < heap.size() && cmp(heap.get(i_child)!, heap.get(i_child + 1)!) > 0 {
            i_child += 1
        }

        // If the parent is already smaller than the smallest child, then the heap property is preserved
        if cmp(heap.get(i)!, heap.get(i_child)!) <= 0 {
            break
        }

        // Otherwise, we swap and proceed down the new path
        heap.swap(i, i_child)
        i = copy(i_child)
    }
    return elem
}

// **This file compiles and runs** as of 2026-09-22, which is what it was
// written to test: whether a heap — a claim on an ordinary list, kept in order
// by mutators — can be written *outside* std, in ordinary Salvo. It can, and
// every TODO it was written around is closed:
//
//   * the ordering it is kept by lives in the type, bound once per signature
//     with `?cmp` [cmp-carry] [cmp-binder], so two differently-ordered heaps are
//     two types and a caller that never names an ordering gets the canonical one
//     for its element type;
//   * `+Heap<T, ?cmp>` is how a mutator keeps a claim it re-establishes
//     [deduce-reapply], which is what lets `heap_push` and `heap_pop` take `Mut`
//     parameters instead of consuming and returning;
//   * `swap` exists, and answers `false` out of range [col-bounds];
//   * `!is` reads as a guard [is-not], and the narrowing it leaves behind routes
//     the bare `heap_pop` to the `NonEmpty` overload [fn-overload-rank];
//   * `+=` is ordinary arithmetic on a place [op-compound].
//
// The smaller worked version of the same shape, with a driver and asserted
// output on both backends, is `rustc_compiles_and_runs_a_heap` and its Kotlin
// twin. COMPLETED.md's decision log has the reasoning for each rule above.
