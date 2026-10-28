// A double-ended queue: O(1) at both ends, which a `List` is not
// (its `remove_first` shifts every element). The collection for a mailbox, a
// work queue or a queue of obligations (user decision 2026-10-02).
//
// `canbe Mut` like `List`, and `<T canbe linear>` like it
// too: a `Deque<Reply<Int>>` owes, and its terminal is
// [drain]. No literal (`[…]` is a `List`), and no `eq`/`hash`/`cmp`: a deque
// is a queue to work through, not a value to compare or key by.
//
// No wire form yet: neither runtime has a codec for it, so a
// message or reply carrying one is refused where it would cross a node
// rather than failing in the host compiler. Local actors hold and send
// deques freely. A codec (as a list's) is a follow-up when one is wanted.
// A value platform type (ROADMAP 0.7): the host
// implements it in `lib/platform/std/core/deque.{rs,kt}` and names the mutable
// kind `MutDeque`.
// @compilerrules [col-deque, type-canbe-mut, linear-container, net-noremote, platform-value-type]
export iterable platform type Deque<T canbe linear> canbe Mut : Iter<self, DequeNext<T>, T>

// The host's primitives, from which the constructors below are built.
platform fn empty_deque<T canbe linear>() [] -> Mut Deque<T>
platform fn deque_from<T>(first: T, rest: T[]) [] -> Mut Deque<T> => !first, !rest
// A deque as one to empty, and the end of an emptied one:
// what [drain] is written with. Ending one that still holds elements traps.
// @compilerrules [linear-container]
platform fn into_mut<T canbe linear>(d: Deque<T>) [] -> Mut Deque<T> => !d
platform fn end_empty<T canbe linear>(d: Mut Deque<T>) [] -> None => !d

// Constructors, in the two shapes every collection has.
// @compilerrules [col-of-nonempty]
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

// A deque of [size] elements, each from its index, in order.
// @compilerrules [col-by]
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

// Takes the front element out, or `None` when empty.
// @compilerrules [linear-container]
export platform fn remove_first<T canbe linear>(d: Mut Deque<T>) [] -> T? => d: Mut

// Takes the back element out, or `None` when empty.
// @compilerrules [linear-container]
export platform fn remove_last<T canbe linear>(d: Mut Deque<T>) [] -> T? => d: Mut

// Takes the element at [index] (counting from the front)
// out, or `None` past the end. O(n): for the rare out-of-order removal.
// @compilerrules [linear-container]
export platform fn remove_at<T canbe linear>(d: Mut Deque<T>, index: Int) [] -> T?
=> d: Mut, index

// The element at [index] from the front, borrowed, or `None` past the end.
export platform fn get<T canbe linear>(d: Deque<T>, index: Int) [] -> (proj(d) T)? => d, index

// The **total** read: an index proven in range by `Idx(d)`
// (`std.core.index`) answers the element itself, as a list's does.
// @compilerrules [col-idx]
export fn get<T canbe linear>(d: Deque<T>, index: Idx(d) Int) [] -> proj(d) T => d, index {
    return get_at(d, index + 0)
}

// The host's read; the proven index is in range.
platform fn get_at<T canbe linear>(d: Deque<T>, index: Int) [] -> proj(d) T => d, index

// The mutable handle to the element at [index]
// from the front, or `None` past the end — `at` for a deque, by index as a
// list's. `get` is the read-only sibling. A handle is a position: an
// in-place write through it moves nothing, while adding or removing an
// element ends it. A host primitive, as a list's.
// @compilerrules [col-locate, ref-handle, fate-poison]
export platform fn at<T canbe linear>(d: Deque<T>, index: Int) [] -> ref(d) T? => d, index

// The **total** mint: an index proven in range by
// `Idx(d)` answers the handle itself, as a list's does.
// @compilerrules [col-idx, ref-handle]
export fn at<T canbe linear>(d: Deque<T>, index: Idx(d) Int) [] -> ref(d) T => d, index {
    return at_proven(d, index + 0)
}

// The host's mint; the proven index is in range.
platform fn at_proven<T canbe linear>(d: Deque<T>, index: Int) [] -> ref(d) T => d, index

// The **total positional write**, as a list's: puts [value] at
// [index] and answers the element it displaced. `std.core.index`'s `Idx` says the
// claims survive it.
// @compilerrules [col-replace]
export platform fn replace<T canbe linear>(d: Mut Deque<T>, index: Idx(d) Int, value: T) [] -> T
=> d: Mut, index, !value

// The front element, borrowed, or `None` when empty.
export platform fn first<T canbe linear>(d: Deque<T>) [] -> proj(d) T? => d

// The back element, borrowed, or `None` when empty.
export platform fn last<T canbe linear>(d: Deque<T>) [] -> proj(d) T? => d

// The number of elements.
export platform fn size<T canbe linear>(d: Deque<T>) [] -> Int => d

// The terminal: consumes the deque and hands every element
// to [each], front to back.
// @compilerrules [linear-container]
export fn drain<T canbe linear>(d: Deque<T>, each: (x: T) -> None) [] -> None
=>[each] !x => !d, each {
    let m = into_mut(d)
    while size(m) > 0 {
        each(remove_first(m) ?: unreachable!())
    }
    end_empty(m)
}

// The text form, `[1, 2, 3]` front to back, as a list's: each
// element by its own `to_str` (user decision 2026-10-04).
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

// The elements front to back, as a list, and a list as a deque:
// copies of them.
// @compilerrules [col-convert]
export fn to_list<T>(d: Deque<T>, ?copy: (v: T) -> T) [] -> List<T> => d {
    let out = Mut List<T>()
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

// A fresh iterator front to back. Borrows the deque, like a
// list's [iter].
// @compilerrules [iter-mint]
export fn iter<T>(d: Deque<T>) [] -> Mut DequeNext<T> => d {
    return Mut DequeNext<T> { items: d, at: 0 }
}

// The pass a deque is walked by: the deque and a position.
// @compilerrules [iter-protocol]
export struct DequeNext<T> : Next<self, proj T> canbe Mut {
    items: proj Deque<T>,
    at: Int
}

export fn next<T>(p: Mut DequeNext<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}

// Walks the deque back to front, borrowing it.
// @compilerrules [col-reversed]
export iter fn reversed<T>(d: Deque<T>) -> Emitted (proj(d) T) | Finished {
    state DequeReversed {
        at: Int = size(d) - 1
    }
    let elem = get(d, at)
    if elem is None {
        return finished()
    }
    at = at - 1
    return emitted(elem)
}
