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
export noremote intrinsic type Deque<T canbe linear> canbe Mut

// Constructors, in the two shapes every collection has [col-of-nonempty].
export intrinsic fn deque_of<T canbe linear>() [] -> Deque<T>
export intrinsic fn deque_of<T canbe linear>(first: T, ...rest: T[]) [] -> Deque<T>
=> !first
export intrinsic fn mut_deque_of<T canbe linear>() [] -> Mut Deque<T>
export intrinsic fn mut_deque_of<T canbe linear>(first: T, ...rest: T[]) [] -> Mut Deque<T>
=> !first

// [col-by] A deque of [size] elements, each from its index, in order.
export intrinsic fn deque_by<T>(size: Int, init: (Int) -> T) [] -> Deque<T> => size, init
export intrinsic fn mut_deque_by<T>(size: Int, init: (Int) -> T) [] -> Mut Deque<T>
=> size, init

// Adds [elem] at the back, moving it in.
export intrinsic fn add_last<T canbe linear>(d: Mut Deque<T>, elem: T) [] -> None => d: Mut, !elem

// Adds [elem] at the front, moving it in.
export intrinsic fn add_first<T canbe linear>(d: Mut Deque<T>, elem: T) [] -> None => d: Mut, !elem

// [linear-container] Takes the front element out, or `None` when empty.
export intrinsic fn remove_first<T canbe linear>(d: Mut Deque<T>) [] -> T? => d: Mut

// [linear-container] Takes the back element out, or `None` when empty.
export intrinsic fn remove_last<T canbe linear>(d: Mut Deque<T>) [] -> T? => d: Mut

// [linear-container] Takes the element at [index] (counting from the front)
// out, or `None` past the end. O(n): for the rare out-of-order removal.
export intrinsic fn remove_at<T canbe linear>(d: Mut Deque<T>, index: Int) [] -> T?
=> d: Mut, index

// The element at [index] from the front, borrowed, or `None` past the end.
export intrinsic fn get<T canbe linear>(d: Deque<T>, index: Int) [] -> (proj(d) T)? => d, index

// The front element, borrowed, or `None` when empty.
export intrinsic fn first<T canbe linear>(d: Deque<T>) [] -> proj(d) T? => d

// The back element, borrowed, or `None` when empty.
export intrinsic fn last<T canbe linear>(d: Deque<T>) [] -> proj(d) T? => d

// The number of elements.
export intrinsic fn size<T canbe linear>(d: Deque<T>) [] -> Int => d

// [linear-container] The terminal: consumes the deque and hands every element
// to [each], front to back.
export intrinsic fn drain<T canbe linear>(d: Deque<T>, each: (x: T) -> None) [] -> None
=>[each] !x => !d, each

// The text form, `[1, 2, 3]` front to back, as a list's [interp-to-str].
export intrinsic fn to_str<T>(d: Deque<T>) [] -> Str => d

// [col-convert] The elements front to back, as a list, and a list as a deque.
export intrinsic fn to_list<T>(d: Deque<T>) [] -> List<T> => d
export intrinsic fn to_deque<T>(list: List<T>) [] -> Deque<T> => list

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
