// A double-ended queue: a sequence that adds and removes at either end in
// O(1), which a `List` does not (its `remove_first` shifts every element).
// It is the collection for a work queue, a mailbox, or replies waiting to be
// sent.
//
// A `Deque<T>` is the host's ring buffer (Kotlin's `ArrayDeque`, Rust's
// `VecDeque`). Besides its two ends it reads by index from the front in
// O(1); removing from the middle with [remove_at] shifts elements and costs
// O(n).
//
// ```
// let jobs: Mut Deque<Str> = mut_deque_of("b")
// add_last(jobs, "c")
// add_first(jobs, "a")
// let next = remove_first(jobs)            // "a", as a Str?
// let back = remove_last(jobs)             // "c"
// println("${jobs}")                       // [b]
// ```
//
// **Like a list.** Reads borrow: [get], [first] and [last] answer a
// read-only view of the deque's own element, and [at] a handle to change it
// in place. Writes move: elements go in and come out by move. An index
// counts from the front and answers `None` when out of range; with
// `i is Idx(d)` proven, [get] answers the element itself and [replace] writes in
// place. `for x in d` and [reversed] walk it front to back and back to
// front, and [to_list] and [to_deque] convert.
//
// **Obligations.** A deque may hold linear values (values that must be used
// exactly once), and is then linear itself: a `Mut Deque<Ticket>` of a
// linear `Ticket` owes, and [drain] is how it ends. It has no `clear` and no
// write at an unproven index, since either would drop an element.
//
// **Not a value to compare.** A deque has no constructor shorthand, no `==`,
// no hash and no ordering: it is a queue to work through, not a value to
// compare or use as a key. It also has no wire form yet, so a message that
// would carry one to another node is refused at compile time; actors on one
// node hold and send deques freely.
// @compilerrules [col-deque, net-noremote]

// `canbe Mut` like `List`, and `<T canbe linear>` like it too. No
// `eq`/`hash`/`cmp`. No wire form: neither runtime has a codec for it, so a
// message or reply carrying one is refused where it would cross a node
// rather than failing in the host compiler; a codec (as a list's) is a
// follow-up when one is wanted. A value platform type: the host implements
// it in `lib/platform/std/core/deque.{rs,kt}` and names the mutable kind
// `MutDeque`.

// A sequence that grows and shrinks at both ends, read by index from the
// front.
//
// `Deque<T>` cannot be changed; `Mut Deque<T>` can, and is what the adding
// and removing functions take. A deque whose element type is linear is
// linear itself, and ends with [drain]. A deque prints as `[1, 2, 3]`, front
// to back.
//
// ```
// let d = deque_of(1, 2, 3)
// let front = first(d)                     // 1, as an Int?
// println("${d} ${size(d)}")               // [1, 2, 3] 3
// ```
// @compilerrules [col-deque, type-canbe-mut, linear-container, platform-value-type, qual-refn-type]
export iterable platform type Deque<T canbe linear> canbe Mut : Iter<self, DequeIter<T>, T> {
    // Growth at either end makes a deque non-empty.
    refn add_last(d: Mut Deque<T>, elem: T) => d: +NonEmpty
    refn add_first(d: Mut Deque<T>, elem: T) => d: +NonEmpty
}

// The host's primitives, from which the constructors below are built.
platform fn empty_deque<T canbe linear>() [] -> Mut Deque<T>
platform fn deque_from<T canbe linear>(first: T, rest: T[]) [] -> Mut Deque<T> => !first, !rest
// A deque as one to empty, and the end of an emptied one:
// what [drain] is written with. Ending one that still holds elements traps.
// @compilerrules [linear-container]
platform fn into_mut<T canbe linear>(d: Deque<T>) [] -> Mut Deque<T> => !d
platform fn end_empty<T canbe linear>(d: Mut Deque<T>) [] -> None => !d

// Builds a deque of the elements written, front to back; with none, an
// empty deque.
//
// Writing at least one element makes the deque known non-empty. The
// elements are moved in, a spread with them, so a deque of linear values is
// built in one call. An empty deque takes its element type from where it is
// used, or from `deque_of<Int>()`.
//
// ```
// let empty: Deque<Int> = deque_of()
// let none = deque_of<Str>()
// let d = deque_of(1, 2, 3)                // [1, 2, 3]
// ```
// @compilerrules [col-of-nonempty, qual-refn-type, linear-container]
export fn deque_of<T canbe linear>() [] -> Deque<T> {
    return empty_deque<T>()
}
// [deque_of] with at least one element: the answer is known non-empty.
export fn deque_of<T canbe linear>(first: T, ...rest: T[]) [] -> +NonEmpty Deque<T> => !first, !rest {
    // The host's `deque_from` is trusted with the claim here, in the file
    // that declares `Deque`.
    return deque_from(first, rest)
}
// [deque_of], answering a mutable deque: `mut_deque_of()` is an empty one
// to fill.
//
// ```
// let jobs: Mut Deque<Str> = mut_deque_of()
// add_last(jobs, "a")
// ```
export fn mut_deque_of<T canbe linear>() [] -> Mut Deque<T> {
    return empty_deque<T>()
}
// [mut_deque_of] with at least one element: the answer is known non-empty.
export fn mut_deque_of<T canbe linear>(first: T, ...rest: T[]) [] -> +NonEmpty Mut Deque<T> => !first, !rest {
    return deque_from(first, rest)
}

// Builds a deque of a given size, each element computed from its index,
// front to back.
//
// The function is called once per index, from 0 up. A size of 0 or less
// gives an empty deque.
//
// ```
// let d = deque_by(3, (i) -> i * 10)       // [0, 10, 20]
// ```
// @compilerrules [col-by]
export fn deque_by<T, [E]>(size: Int, init: (Int) [E] -> T) -> Deque<T> => size, init {
    return mut_deque_by(size, init)
}
// [deque_by], answering a mutable deque.
export fn mut_deque_by<T, [E]>(size: Int, init: (Int) [E] -> T) -> Mut Deque<T> => size, init {
    let d = empty_deque<T>()
    let i = 0
    while i < size {
        add_last(d, init(copy(i)))
        i = i + 1
    }
    return d
}

// Adds an element at the back, moving it in.
//
// O(1) amortized. The deque is known non-empty afterwards, and every `Idx`
// claim on it survives, since no existing element moves.
export platform fn add_last<T canbe linear>(d: Mut Deque<T>, elem: T) [] -> None => d: Mut, !elem

// Adds an element at the front, moving it in.
//
// O(1) amortized. The deque is known non-empty afterwards. The new element
// is at index 0, so every other element is now one index further from the
// front; `Idx` claims survive, since an index that was in range still is.
export platform fn add_first<T canbe linear>(d: Mut Deque<T>, elem: T) [] -> None => d: Mut, !elem

// Takes the front element out, or answers `None` when the deque is empty.
//
// O(1). The element is moved out, which is how a linear value leaves a
// deque one at a time. With `?: break`, it drives a take-until-empty loop:
//
// ```
// let jobs = mut_deque_of("a", "b")
// while true {
//     let job = remove_first(jobs) ?: break
//     println(job)                         // a, then b
// }
// ```
// @compilerrules [linear-container]
export platform fn remove_first<T canbe linear>(d: Mut Deque<T>) [] -> T? => d: Mut

// Takes the back element out, or answers `None` when the deque is empty.
//
// O(1); the element is moved out.
// @compilerrules [linear-container]
export platform fn remove_last<T canbe linear>(d: Mut Deque<T>) [] -> T? => d: Mut

// Takes the element at an index, counting from the front, out of the deque,
// or answers `None` when the index is out of range.
//
// O(n): the elements around it shift to close the gap. For the occasional
// out-of-order removal; the ends are what a deque is fast at.
// @compilerrules [linear-container]
export platform fn remove_at<T canbe linear>(d: Mut Deque<T>, index: Int) [] -> T?
=> d: Mut, index

// The element at an index from the front, borrowed, or `None` when the index
// is out of range.
//
// O(1). An index proven in range (`i is Idx(d)`) picks the overload below,
// which answers the element itself.
//
// ```
// let d = deque_of(10, 20)
// let second = get(d, 1)                   // 20, as an Int?
// ```
// @compilerknown
export platform fn get<T canbe linear>(d: Deque<T>, index: Int) [] -> (proj(d) T)? => d, index

// [get] at an index proven in range: answers the element itself, with no
// `None` to handle.
//
// ```
// let d = deque_of(10, 20)
// let i = 1
// if i is Idx(d) {
//     println("${get(d, i) + 1}")          // 21
// }
// ```
// @compilerrules [col-idx]
// @compilerknown
export fn get<T canbe linear>(d: Deque<T>, index: Idx(d) Int) [] -> proj(d) T => d, index {
    return get_at(d, index + 0)
}

// The host's read; the proven index is in range.
// @compilerknown
platform fn get_at<T canbe linear>(d: Deque<T>, index: Int) [] -> proj(d) T => d, index

// A handle to the element at an index from the front, through which it can
// be changed in place, or `None` when the index is out of range.
//
// [get] reads and `at` handles, as on a list. A write through the handle
// moves nothing, while adding or removing an element ends it. The handle is
// writable when the element type is `Mut X`.
// @compilerrules [col-locate, ref-handle, fate-poison]
// @compilerknown
export platform fn at<T canbe linear>(d: Deque<T>, index: Int) [] -> ref(d) T? => d, index

// [at] at an index proven in range: answers the handle itself, with no
// `None` to handle.
// @compilerrules [col-idx, ref-handle]
// @compilerknown
export fn at<T canbe linear>(d: Deque<T>, index: Idx(d) Int) [] -> ref(d) T => d, index {
    return at_proven(d, index + 0)
}

// The host's mint; the proven index is in range.
// @compilerknown
platform fn at_proven<T canbe linear>(d: Deque<T>, index: Int) [] -> ref(d) T => d, index

// Writes a value at an index proven in range and answers the element it
// displaced.
//
// Nothing written is lost, so this is safe on a deque of linear values. A
// write moves no boundary, so every `Idx` claim on the deque survives it.
//
// ```
// let d = mut_deque_of(1, 2, 3)
// let i = 0
// if i is Idx(d) {
//     let old = replace(d, i, 10)          // 1; d is [10, 2, 3]
// }
// ```
// @compilerrules [col-replace]
export platform fn replace<T canbe linear>(d: Mut Deque<T>, index: Idx(d) Int, value: T) [] -> T
=> d: Mut, index, !value

// The front element, borrowed, or `None` when the deque is empty.
export platform fn first<T canbe linear>(d: Deque<T>) [] -> proj(d) T? => d

// The back element, borrowed, or `None` when the deque is empty.
export platform fn last<T canbe linear>(d: Deque<T>) [] -> proj(d) T? => d

// The number of elements in the deque.
export platform fn size<T canbe linear>(d: Deque<T>) [] -> Int => d

// Consumes the deque and hands every element to a function, front to back.
//
// This is how a deque of linear values ends: the deque is spent, and each
// element goes on to the function, which consumes it. The function's
// effects are the call's.
//
// ```
// linear struct Ticket {
//     id: Int
// }
//
// fn redeem(t: Ticket) [Console] -> None => !t {
//     println("redeemed ${t.id}")
// }
//
// fn redeem_all(queue: Deque<Ticket>) [Console] -> None => !queue {
//     drain(queue, redeem)
// }
// ```
// @compilerrules [linear-container]
export fn drain<T canbe linear, [E]>(d: Deque<T>, each: (x: T) [E] -> None) -> None
=>[each] !x => !d, each {
    let m = into_mut(d)
    while size(m) > 0 {
        each(remove_first(m) ?: unreachable!())
    }
    end_empty(m)
}

// The text form of a deque, `[1, 2, 3]` front to back, as a list's: what
// `"${d}"` prints.
//
// Each element is rendered by its own `to_str`, found at the call site.
// @compilerrules [interp-to-str]
export fn to_str<T>(d: Deque<T>, ?to_str: (x: T) -> Str) [] -> Str => d {
    let out = Mut Str("[")
    let i = 0
    for x in d {
        if i > 0 {
            append(out, ", ")
        }
        append(out, to_str(x))
        i = i + 1
    }
    append(out, "]")
    return out
}

// The elements of a deque as a new list, front to back.
//
// Each element is copied with its type's `copy`, found at the call site.
//
// ```
// let xs = to_list(deque_of(1, 2))         // [1, 2]
// ```
// @compilerrules [col-convert]
export fn to_list<T>(d: Deque<T>, ?copy: (v: T) -> T) [] -> List<T> => d {
    let out = Mut List<T>()
    for x in d {
        add(out, copy(x))
    }
    return out
}
// The elements of a list as a new deque, front to back.
//
// Each element is copied with its type's `copy`, found at the call site.
//
// ```
// let xs = List(1, 2)
// let d = to_deque(xs)                     // [1, 2]
// ```
export fn to_deque<T>(list: List<T>, ?copy: (v: T) -> T) [] -> Deque<T> => list {
    let out = empty_deque<T>()
    for x in list {
        add_last(out, copy(x))
    }
    return out
}

// A fresh iterator over the deque, front to back.
//
// The iterator borrows the deque, as a list's does, so nothing is copied.
// A `for` loop over a deque needs no `iter`; it is what the functions of
// `std.core.seq` take.
// @compilerrules [iter-mint]
export fn iter<T>(d: Deque<T>) [] -> Mut DequeIter<T> => d {
    return Mut DequeIter<T> { items: d, at: 0 }
}

// The iterator [iter] answers over a deque: the deque, borrowed, and a
// position in it.
// @compilerrules [iter-protocol]
export struct DequeIter<T> : Next<self, proj T> canbe Mut {
    // The deque being walked, borrowed.
    items: proj Deque<T>,
    // The index of the next element to emit.
    at: Int
}

// Answers the element at the iterator's position and moves past it, or the
// end once every element has been emitted.
export fn next<T>(p: Mut DequeIter<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}

// Walks a deque back to front, borrowing it.
//
// ```
// let d = deque_of(1, 2, 3)
// for x in reversed(d) {
//     println("${x}")                      // 3, 2, 1
// }
// ```
// @compilerrules [col-reversed]
export fn reversed<T>(d: Deque<T>) [] -> Mut DequeReversedIter<T> => d {
    return Mut DequeReversedIter<T> { items: d, at: size(d) - 1 }
}

// The iterator [reversed] answers over a deque: the deque, borrowed, and
// the index of the next element to emit, counting down.
export struct DequeReversedIter<T> : Next<self, proj T> canbe Mut {
    // The deque being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    items: proj Deque<T>,
    // The index of the next element; `-1` is the end.
    at: Int
}

// Answers the element at the walk's position and moves one back, or the
// end once the front element has been emitted.
export fn next<T>(p: Mut DequeReversedIter<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at - 1
    return emitted(elem)
}
