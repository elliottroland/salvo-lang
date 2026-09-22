// An implementation of a binary heap qualifier for the List<T> type when T is orderable. Used as an
// exercise in a more complicated qualifier that the language should be able to support.
//
// STATUS (2026-09-22): the *ordering* half now compiles — the heap names the
// ordering it is kept by in a fn slot [cmp-carry], every function binds it with
// the bare `?cmp` binder [cmp-binder], and the comparisons in the bodies go
// through it. What is still missing is listed at the bottom of this file; until
// those land, this file does not compile as a whole, which is why it lives in
// `demo/` and no test builds it. A smaller worked example of the same shape
// *is* built and run on both backends — see
// `rustc_compiles_and_runs_a_carried_ordering` and its Kotlin twin.

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
        // 2026-09-22, which was a hover bug, not a type: HEAP_QUALIFIER.md
        // item 3.)
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
            // TODO: Should support += syntax (HEAP_QUALIFIER.md item 6).
            i_child = i_child + 1
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

// What this file still waits for, all of it tracked in HEAP_QUALIFIER.md.
// `salvo analyze` on it reports **exactly one** error as of 2026-09-22:
//
//   * item 2 / ROADMAP **D2** — the only one left: "deduction promises
//     qualifier `Heap` on `heap`, but the body may remove it". A mutator cannot
//     yet keep a claim it re-establishes, which is why `heap_push` above
//     consumes and returns instead.
//
// Two more TODOs are noted inline and cost no errors here:
//
//   * item 6 — `+=`.
//
// Item 3 (the `proj proj T?` hover) and item 5 (`!is`, used above) are both
// **fixed** (2026-09-22).
//
// The ordering itself needs nothing further: `?cmp` is bound once per
// signature, the two `heap_pop` overloads share it, and a caller that never
// names an ordering gets the canonical one for its element type.
