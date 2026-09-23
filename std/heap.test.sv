// [test-file] The tests of module `heap` — its **annex**: the same privacy
// boundary as `heap.sv`, so these tests reach its private declarations, and
// nothing reaches them (a production build never loads this file).
//
// `std.test` is implicitly available here [test-implicit-import], which is why
// `expect`/`expect_eq` arrive without an import line. `heap.sv`'s own
// declarations arrive the same way [test-visibility].

test "an empty heap pops nothing" {
    let heap = heap_of<Int>()
    expect(pop(heap) is None, "popping an empty heap answers None")
}

test "one element comes back out" {
    let heap = heap_of<Int>()
    push(heap, 7)
    // No `!`: `push` reports the heap non-empty [deduce-gained], so this is
    // the `NonEmpty` overload of `pop` and it answers an element.
    expect_eq(pop(heap), 7)
}

test "pops come out in order, whatever order they went in" {
    let heap = heap_of<Int>()
    push(heap, 5)
    push(heap, 1)
    push(heap, 9)
    push(heap, 3)
    // The first `pop` is the non-optional one — the heap is known non-empty
    // after a push [deduce-gained] — and popping gives the claim up, so the
    // rest are optional again.
    expect_eq(pop(heap), 1)
    expect_eq(pop(heap)!, 3)
    expect_eq(pop(heap)!, 5)
    expect_eq(pop(heap)!, 9)
    expect(pop(heap) is None, "the heap is empty again")
}

test "heapify arranges an arbitrary list" {
    let names: Mut List<Int> = [4, 2, 8, 1]
    heapify(names)
    expect_eq(pop(names)!, 1)
    expect_eq(pop(names)!, 2)
    expect_eq(pop(names)!, 4)
    expect_eq(pop(names)!, 8)
}

test "pushing keeps the heap property" {
    let heap = heap_of<Int>()
    push(heap, 4)
    push(heap, 2)
    expect_eq(pop(heap), 2)
    push(heap, 1)
    push(heap, 3)
    expect_eq(pop(heap), 1)
    expect_eq(pop(heap)!, 3)
    expect_eq(pop(heap)!, 4)
}

test "peek always returns the smallest element, or None if the heap is empty" {
    let heap = heap_of<Int>()
    expect(heap.peek() is None, "An empty heap should return None")
    heap.push(4)
    heap.push(1)
    expect_eq(heap.peek(), 1)
    heap.pop()
    expect_eq(heap.peek()!, 4)
    heap.pop()
    expect(heap.peek() is None, "A heap which has been emptied should peek -> None")
}

// [col-of-nonempty] `heap_of` is non-empty *by construction*: the element
// constructor requires a first, the claim travels through `heapify` because
// exchanging elements cannot change how many there are [qual-refn], and the
// `NonEmpty` overloads are then the ones that answer — `peek` and `pop` without
// an optional.
test "a heap built from elements is known non-empty" {
    let heap = heap_of(5, 1, 9)
    expect_eq(peek(heap), 1)
    expect_eq(pop(heap), 1)
    expect_eq(pop(heap)!, 5)
    expect_eq(pop(heap)!, 9)
}
