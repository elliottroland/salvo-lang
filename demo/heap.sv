// An implementation of a binary heap qualifier for the List<T> type when T is orderable. Used as an
// exercise in a more complicated qualifier that the language should be able to support.

// Indicates that the list is organized like a succinct binary heap.
// TODO: `export` is not highlighted as a keyword in the LSP
// TODO: we need a way of saying that Heaps only make sense for ordereable types. Maybe the right answer is to
//       require each function to take a `?cmp: (T, T) -> Int` implicit parameter instead?
export qualifier Heap<T canbe ordered> of List<T> with NonEmpty

// Returns an empty List which trivially supports the heap property.
export fn empty_heap<T>() -> Mut List<T> as Heap {
    return mut_list_of()
}

fn as_heap<T>(h: Mut List<T>) -> Mut List<T> as Heap {
    return h
}

// Pushes the [elem] into the [heap], preserving the heap property.
// TODO: Need a way of asserting that the Heap qualifier still applies.
//       Maybe we should be able to say `+Heap` in the deductions as a way of explicitly "reapplying" it?
//       This is effectively what the `qualifies` function does, but as a preserving property -- running the qualifies again would be another O(n) which is unnecessary
export fn heap_push<T>(heap: Heap Mut List<T>, elem: T) => heap: Heap Mut {
    add(heap, elem)
    // The index where the value currently is
    let i = size(heap) - 1
    while i > 0 {
        // TODO: Why is val: `proj proj T?`, rather than just `proj T?`?
        let val = heap.get(i)
        if val is None {
            break
        }

        let i_parent = i / 2
        let parent = heap.get(i_parent)
        if parent is None || parent <= val {
            break
        }

        // TODO: need the ability to swap, or set specific indices.
        //       Presumably setting specific indices means we would need to _take_ from the list as well?
        // Otherwise we need to swap
        heap.swap(i, i_parent)
        i = i_parent
    }
}

// Pops the smallest element in the heap, preserving the heap property.
export fn heap_pop<T>(heap: Heap Mut List<T>) -> T? {
    if heap !is NonEmpty {
        return None
    }
    // TODO: `heap` should be `NonEmpty` here, and this should route to the NonEmpty overload
    return heap_pop(heap)
}

export fn heap_pop<T>(heap: NonEmpty Heap Mut List<T>) -> T => heap: Heap Mut {
    if heap.size() == 1 {
        return heap.remove_first()
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
        if i_child + 1 < heap.size() && heap.get(i_child)! > heap.get(i_child + 1)! {
            // TODO: Should support += syntax
            i_child = i_child + 1
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

fn remove_first<T canbe linear>(list: NonEmpty Mut List<T>) [] -> T => list: Mut {
    // TODO: Need to be able to elide the NonEmpty qualifier here to reach to the function in core.list.
    //       Does `-NonEmpty` make more sense, or `^NonEmpty` ?
    // return remove_first(-NonEmpty l)!
}