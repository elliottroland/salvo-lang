// [col-deque] A double-ended queue: O(1) at both ends, which a `List` is not
// (its `remove_first` shifts every element). The collection for a mailbox, a
// work queue or a queue of obligations (user decision 2026-10-02).
//
// `canbe Mut` like `List` [type-canbe-mut], and `<T canbe linear>` like it
// too [linear-container]: a `Deque<Reply<Int>>` owes, and its terminal is
// [drain]. No literal (`[…]` is a `List`), and no `eq`/`hash`/`cmp`: a deque
// is a queue to work through, not a value to compare or key by.
//
// [noremote] No wire form yet: neither runtime has a codec for it, so a
// message or reply carrying one is refused where it would cross a node
// rather than failing in the host compiler. Local actors hold and send
// deques freely. A codec (as a list's) is a follow-up when one is wanted.
// [platform-value-type] A value platform type (ROADMAP 0.7): the host
// implements it in `std/platform/core/deque.{rs,kt}` and names the mutable
// kind `MutDeque`.
export platform type Deque<T canbe linear> canbe Mut

// The host's primitives, from which the constructors below are built.
platform fn empty_deque<T canbe linear>() [] -> Mut Deque<T>
platform fn deque_from<T>(first: T, rest: T[]) [] -> Mut Deque<T> => !first, !rest
// [linear-container] A deque as one to empty, and the end of an emptied one:
// what [drain] is written with. Ending one that still holds elements traps.
platform fn into_mut<T canbe linear>(d: Deque<T>) [] -> Mut Deque<T> => !d
platform fn end_empty<T canbe linear>(d: Mut Deque<T>) [] -> None => !d

// Constructors, in the two shapes every collection has [col-of-nonempty].
export fn deque_of<T canbe linear>() [] -> Deque<T> {
    return empty_deque<T>()
}
// A linear element travels alone: no obligation can sit in a variadic tail.
export fn deque_of<T canbe linear>(first: T) [] -> Deque<T> => !first {
    let d = empty_deque<T>()
    add_last(d, first)
    return d
}
export fn deque_of<T>(first: T, ...rest: T[]) [] -> Deque<T> => !first, !rest {
    return deque_from(first, rest)
}
export fn mut_deque_of<T canbe linear>() [] -> Mut Deque<T> {
    return empty_deque<T>()
}
export fn mut_deque_of<T canbe linear>(first: T) [] -> Mut Deque<T> => !first {
    let d = empty_deque<T>()
    add_last(d, first)
    return d
}
export fn mut_deque_of<T>(first: T, ...rest: T[]) [] -> Mut Deque<T> => !first, !rest {
    return deque_from(first, rest)
}

// [col-by] A deque of [size] elements, each from its index, in order.
export fn deque_by<T>(size: Int, init: (Int) -> T) [] -> Deque<T> => size, init {
    return mut_deque_by(size, init)
}
export fn mut_deque_by<T>(size: Int, init: (Int) -> T) [] -> Mut Deque<T> => size, init {
    let d = empty_deque<T>()
    let i = 0
    while i < size {
        add_last(d, init(copy(i)))
        i = i + 1
    }
    return d
}

// Adds [elem] at the back, moving it in.
export platform fn add_last<T canbe linear>(d: Mut Deque<T>, elem: T) [] -> None => d: Mut, !elem

// Adds [elem] at the front, moving it in.
export platform fn add_first<T canbe linear>(d: Mut Deque<T>, elem: T) [] -> None => d: Mut, !elem

// [linear-container] Takes the front element out, or `None` when empty.
export platform fn remove_first<T canbe linear>(d: Mut Deque<T>) [] -> T? => d: Mut

// [linear-container] Takes the back element out, or `None` when empty.
export platform fn remove_last<T canbe linear>(d: Mut Deque<T>) [] -> T? => d: Mut

// [linear-container] Takes the element at [index] (counting from the front)
// out, or `None` past the end. O(n): for the rare out-of-order removal.
export platform fn remove_at<T canbe linear>(d: Mut Deque<T>, index: Int) [] -> T?
=> d: Mut, index

// The element at [index] from the front, borrowed, or `None` past the end.
export platform fn get<T canbe linear>(d: Deque<T>, index: Int) [] -> (proj(d) T)? => d, index

// The front element, borrowed, or `None` when empty.
export platform fn first<T canbe linear>(d: Deque<T>) [] -> proj(d) T? => d

// The back element, borrowed, or `None` when empty.
export platform fn last<T canbe linear>(d: Deque<T>) [] -> proj(d) T? => d

// The number of elements.
export platform fn size<T canbe linear>(d: Deque<T>) [] -> Int => d

// [linear-container] The terminal: consumes the deque and hands every element
// to [each], front to back.
export fn drain<T canbe linear>(d: Deque<T>, each: (x: T) -> None) [] -> None
=>[each] !x => !d, each {
    let m = into_mut(d)
    while size(m) > 0 {
        each(remove_first(m)!)
    }
    end_empty(m)
}

// The text form, `[1, 2, 3]` front to back, as a list's [interp-to-str]: each
// element by its own `to_str` (user decision 2026-10-04).
export fn to_str<T>(d: Deque<T>, ?to_str: (x: T) -> Str) [] -> Str => d {
    let out = mut_str("[")
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

// [col-convert] The elements front to back, as a list, and a list as a deque:
// copies of them.
export fn to_list<T>(d: Deque<T>, ?copy: (v: T) -> T) [] -> List<T> => d {
    let out = mut_list_of<T>()
    for x in d {
        add(out, copy(x))
    }
    return out
}
export fn to_deque<T>(list: List<T>, ?copy: (v: T) -> T) [] -> Deque<T> => list {
    let out = empty_deque<T>()
    for x in list {
        add_last(out, copy(x))
    }
    return out
}

// [iter-mint] A fresh iterator front to back. Borrows the deque, like a
// list's [iter].
export fn iter<T>(d: Deque<T>) [] -> Mut DequeYield<T> => d {
    return Mut DequeYield<T> { items: d, at: 0 }
}

// [iter-protocol] The pass a deque is walked by: the deque and a position.
export struct DequeYield<T> : Yield<self, proj T> canbe Mut {
    items: proj Deque<T>,
    at: Int
}

export fn next<T>(p: Mut DequeYield<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}

// [col-reversed] Walks the deque back to front, borrowing it.
export iter fn reversed<T>(d: Deque<T>) -> Emitted (proj(d) T) | Finished {
    state {
        at: Int = size(d) - 1
    }
    let elem = get(d, at)
    if elem is None {
        return finished()
    }
    at = at - 1
    return emitted(elem)
}
