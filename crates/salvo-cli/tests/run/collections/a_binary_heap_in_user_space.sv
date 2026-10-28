// [deduce-reapply] [cmp-carry] a binary heap in user space
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_heap

// [cmp-carry] A binary heap in **user space**: a claim on an ordinary list,
// carrying the ordering it is kept by, with mutators that keep the claim.
export qualifier Heap<T>(?cmp: (T, T) -> Int) of List<T>

export fn empty_heap<T>(?cmp: (T, T) -> Int) [] -> +Heap<T>(?cmp) Mut List<T> {
    return Mut List()
}

// [deduce-reapply] **Establishing** the claim rather than keeping one: `list`
// arrives an ordinary list and leaves a `Heap<T>(?cmp)`, which a function in the
// qualifier's own file may say about a parameter. The ordering it establishes the
// claim *under* is the one the call resolved, so the caller's heap is ordered by
// what it asked for.
export fn heapify<T>(list: Mut List<T>, ?Ordered<T>) [] -> None
=> list: +Heap<T>(?cmp) Mut {
    let i = size(list) / 2
    while i > 0 {
        i -= 1
        let at = copy(i)
        while at < size(list) {
            let child = at * 2 + 1
            if child >= size(list) {
                break
            }
            let smaller = copy(child)
            if child + 1 < size(list) && cmp(list.get(child + 1) ?: unreachable!(), list.get(child) ?: unreachable!()) < 0 {
                smaller += 1
            }
            if cmp(list.get(at) ?: unreachable!(), list.get(smaller) ?: unreachable!()) <= 0 {
                break
            }
            ignore(list.swap(at, smaller))
            at = copy(smaller)
        }
    }
}

// [deduce-reapply] `add` strips the claim — a mutating callee must — and this
// function is the one that knows the sift puts it back.
export fn heap_push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) [] -> None
=> heap: +Heap<T>(?cmp) Mut, !elem {
    add(heap, elem)
    let i = size(heap) - 1
    while i > 0 {
        let parent = (i - 1) / 2
        if cmp(heap.get(parent) ?: unreachable!(), heap.get(i) ?: unreachable!()) <= 0 {
            break
        }
        ignore(heap.swap(i, parent))
        i = copy(parent)
    }
}

export fn heap_pop<T>(heap: Heap<T>(?cmp) Mut List<T>) [] -> T?
=> heap: +Heap<T>(?cmp) Mut {
    if heap !is NonEmpty {
        return None
    }
    return heap_pop(heap)
}

export fn heap_pop<T>(heap: NonEmpty Heap<T>(?cmp) Mut List<T>) [] -> T
=> heap: +Heap<T>(?cmp) Mut {
    let last = size(heap) - 1
    ignore(heap.swap(0, last))
    let least = heap.remove_at(last) ?: unreachable!()
    let i = 0
    while i < size(heap) {
        let child = i * 2 + 1
        if child >= size(heap) {
            break
        }
        let smaller = copy(child)
        if child + 1 < size(heap) && cmp(heap.get(child + 1) ?: unreachable!(), heap.get(child) ?: unreachable!()) < 0 {
            smaller += 1
        }
        if cmp(heap.get(i) ?: unreachable!(), heap.get(smaller) ?: unreachable!()) <= 0 {
            break
        }
        ignore(heap.swap(i, smaller))
        i = copy(smaller)
    }
    return least
}

fn by_last_digit(a: Int, b: Int) [] -> Int => a, b {
    return cmp(a % 10, b % 10)
}

// The binder again, on a concrete element type: this drains *either* heap
// below, each in the ordering its own type carries [cmp-binder].
fn drain_heap(heap: Heap<Int>(?cmp) Mut List<Int>) [Console] -> None
=> heap: +Heap<Int>(?cmp) Mut {
    let out = Mut Str()
    while heap_pop(heap) is Int n {
        append(out, "${n} ")
    }
    println("${out}")
}

fn main() [use] {
    use StdConsole()

    let h = empty_heap<Int>()
    heap_push(h, 5)
    heap_push(h, 3)
    heap_push(h, 9)
    heap_push(h, 1)
    heap_push(h, 7)
    println("root ${first(h) ?: unreachable!()}")
    drain_heap(h)

    // A list nobody claimed anything about, made a heap by a function that
    // establishes the claim [deduce-reapply] — and then read by the same
    // surface, which only accepts a heap.
    let raw: Mut List<Int> = Mut List(8, 2, 6, 4)
    heapify(raw)
    heap_push(raw, 3)
    drain_heap(raw)

    // A second heap under an ordering of the program's own: the claim carries
    // it, so the same functions pop in *that* order.
    let byDigit = empty_heap<Int>(cmp = by_last_digit)
    heap_push(byDigit, 25)
    heap_push(byDigit, 13)
    heap_push(byDigit, 41)
    drain_heap(byDigit)
}
//= stdout
//= root 1
//= 1 3 5 7 9 
//= 2 3 4 6 8 
//= 41 13 25 
