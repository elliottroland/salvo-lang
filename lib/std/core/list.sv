// The list: an ordered sequence of elements, read by index and grown at the
// end. It is the collection most Salvo code reaches for first.
//
// A `List<T>` is the host's array list (a Kotlin `MutableList`, a Rust
// `Vec`). Reading by index, [size] and adding at the end are cheap (adding is
// amortized O(1)); inserting or removing anywhere else shifts every element
// after that position, so [remove_first] on a long list costs O(n). For a
// queue worked from both ends, use a `Deque` (`std.core.deque`).
//
// ```
// let xs = Mut List(30, 10, 20)
// add(xs, 40)
// let head = first(xs)                     // 30, an Int: xs is known non-empty
// let second = get(xs, 1)                  // 10, as an Int?
// let missing = get(xs, 9)                 // None: out of range
// let taken = remove_first(xs)             // 30, moved out of the list
// println("${xs} has ${size(xs)}")         // [10, 20, 40] has 3
// ```
//
// **Immutable unless asked.** `List(1, 2, 3)` builds a list nobody can
// change; `Mut List(1, 2, 3)` builds one that [add], [insert_at],
// [remove_first] and the other mutators accept. Whether the *elements* can be
// changed in place is a separate question, answered by the element type: a
// `List<Mut Counter>` has a fixed length, but each counter can be changed
// through [at] or [update].
//
// **Reads borrow, writes move.** [get], [first], [last] and a `for` loop
// answer the list's own element as a read-only view: nothing is copied, and
// the view can live no longer than the list. Write `copy(get(xs, i))` to
// keep a value of your own. Putting an element in ([add], [insert_at], the
// constructors) moves it into the list, and taking one out ([remove_first],
// [remove_at], [remove_range]) moves it back out to you.
//
// **Indices answer rather than fail.** Indices count from 0, and a negative
// index is out of range rather than counted from the back. Out of range, a
// read answers `None`, [insert_at] hands the element back and [swap] answers
// `false`; nothing here throws, and both backends behave the same.
//
// **Claims a list's type can carry.** A qualifier on the type records a
// fact the compiler has proven, and some functions here demand one so that
// they can answer outright instead of with an optional:
//
// - `NonEmpty`: a list built with at least one element, or just added to,
//   is known non-empty, and [first] then answers the element itself.
//   `xs is NonEmpty` tests it; removing an element gives it up.
// - `Idx(xs) Int`: an index proven in range for `xs`, by `i is Idx(xs)` or by
//   walking `indices(xs)`. With one, [get] and [at] answer the element
//   itself, and [swap], [replace] and [update] cannot miss. Adding,
//   swapping and writing in place keep the proof; removing gives it up.
// - [Sorted]: established by [sort] and [mut_sort], kept by [add_sorted],
//   and demanded by [binary_search].
//
// ```
// let xs = List(10, 20, 30)
// let total = 0
// for i in indices(xs) {
//     total = total + get(xs, i)           // an Int: no None to handle
// }
// let i = 5
// if i is Idx(xs) {
//     println("${get(xs, i)}")
// } else {
//     println("no element ${i}")           // this branch: 5 is out of range
// }
// ```
//
// **Lists of obligations.** A list may hold linear values, which must be
// used exactly once (see the Linear Types topic), and is then linear
// itself: it has to be drained or handed on. Its surface is the moves:
// [add] and [append] put elements in, [remove_first], [remove_last] and
// [remove_at] take them out, [swap] reorders them, and [drain] ends the list
// by handing every element to a function that consumes it. What would drop
// an element is missing on purpose: there is no `clear`, and no write at an
// unproven index.
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
// fn redeem_all() [use] -> None {
//     use StdConsole()
//     let pending = Mut List(Ticket { id: 1 }, Ticket { id: 2 })
//     add(pending, Ticket { id: 3 })
//     let one = remove_first(pending)
//     if one is Ticket {
//         redeem(one)                      // redeemed 1
//     }
//     drain(pending, redeem)               // redeemed 2, then 3
// }
// ```

// `canbe Mut` opts List into the language-level Mut auto-qualifier: a
// backend may map `Mut List<T>` to a different native type (Kotlin's
// `MutableList<T>`) or to the same one (Rust's `Vec<T>`, where mutability
// lives in the binding). A value platform type: the host implements it in
// `lib/platform/std/core/list.{rs,kt}` and names the mutable kind `MutList`.
// Only the empty mutable constructor is intrinsic; the others are written
// below in terms of it and `add`.
//
// `<T canbe linear>` is what lets a list hold obligations; the element's
// declaration is the source of the linearity, and a list never spells it.
// The body says what `std.core.nonempty`'s generic `NonEmpty` learns from a
// list's own functions: a list's author is the one who knows that appending
// makes it non-empty.

// An ordered, growable sequence of elements, read by index from 0.
//
// `List<T>` cannot be changed; `Mut List<T>` can, and is what the mutators
// take. A list whose element type is linear (a value that must be used
// exactly once) is linear itself, and ends with [drain]; `List<Int>` is an
// ordinary value. Lists compare with `==` and order with `<` element by
// element, print as `[1, 2, 3]`, and are walked by `for x in xs`.
//
// ```
// let names = List("ada", "grace")         // a NonEmpty List<Str>
// let seen: Mut List<Str> = Mut List()
// add(seen, "linus")
// println("${names} ${seen}")              // [ada, grace] [linus]
// ```
// @compilerrules [type-canbe-mut, linear-container, platform-value-type, qual-refn-type]
export iterable platform type List<T canbe linear> canbe Mut : Iter<self, ListIter<T>, T> {
    // Adding makes a list non-empty. `add` cannot say this itself: a function
    // that mutates may not promise back a qualifier it has never heard of,
    // and appending is exactly the operation that makes a list non-empty.
    // @compilerrules [deduce-syntax, qual-refn]
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty

    // Exchanging two elements of a list that is *already* non-empty leaves
    // it non-empty, which is why the parameter is written
    // `NonEmpty Mut List<T>` and not `Mut List<T>`: swapping does not *make*
    // a list non-empty, so the claim is **kept**, not established. A
    // refinement whose parameter is narrower than the declaration it refines
    // states exactly that, and applies only where the claim is already there.
    //
    // `swap`'s own exhaustive clause has to strip the claim, like every
    // mutator's; this is the owner saying what the call really leaves behind.
    // It is what lets a heap's sift-down keep the `NonEmpty` it was handed
    // (`std.heap`'s `heapify`).
    // @compilerrules [qual-refn-narrow]
    refn swap(list: NonEmpty Mut List<T>, i: Int, j: Int) => list: +NonEmpty

    // Appending to a non-empty list keeps it non-empty (the array may be
    // empty, so it establishes nothing).
    refn append(list: NonEmpty Mut List<T>, items: T[]) => list: +NonEmpty
}

// Builds a list of the elements written, in order; with none, an empty list.
//
// Writing at least one element proves the list is not empty, so
// `List(1, 2, 3)` is a `NonEmpty List<Int>` and [first] answers its element
// with no test. `List()` takes its element type from where it is used
// (`let xs: List<Int> = List()`), or from `List<Int>()`.
//
// The elements are moved into the list, a spread with them: `List(a, ...xs)`
// spends the array `xs`. That is what lets a list of linear values be built
// in one call. A spread on its own, `List(...xs)`, is not accepted, since
// nothing would prove the list non-empty; [append] the array onto a
// `Mut List()` instead.
//
// ```
// let empty: List<Int> = List()
// let xs = List(1, 2, 3)
// let head = first(xs)                     // 1, an Int
// let more = List(0, ...array_of(4, 5))    // [0, 4, 5]
// ```
// @compilerrules [col-of-nonempty, fn-variadic, fn-type-named, deduce-syntax, qual-refn-type, linear-container]
export fn List<T canbe linear>() [] -> List<T> {
    // The constructor **is** the type-named function: `List(...)` resolves
    // to this declaration, whose canonical key is `List` (and `Mut_List` for
    // the mutable pair). There is no separate lowercase constructor.
    return Mut List<T>()
}

// [List] with at least one element: the answer is known non-empty.
export fn List<T canbe linear>(first: T, ...rest: T[]) [] -> NonEmpty List<T> => !first, !rest {
    // Nothing tests the claim at the call: `add` establishes it, by `List`'s
    // own refinement above, and `append` keeps it.
    return Mut List(first, ...rest)
}

// The empty one is the list's one intrinsic constructor: everything else is
// built from it with `add` and `append`.

// [List], mutable: `Mut List()` is an empty list to fill, and
// `Mut List(1, 2)` a non-empty one.
//
// On a `Mut List`, `NonEmpty` lasts until a mutation that can take it away:
// [remove_at] or [remove_first] gives it up, and [add] puts it back.
//
// ```
// let xs: Mut List<Int> = Mut List()
// add(xs, 7)
// let seven = first(xs)                    // 7, an Int: add made xs NonEmpty
// ```
// @compilerrules [qual-subject, qual-refn, backend-intrinsic, linear-container]
export intrinsic fn Mut List<T canbe linear>() [] -> Mut List<T>

// `Mut List` with at least one element: the answer is known non-empty.
export fn Mut List<T canbe linear>(first: T, ...rest: T[]) [] -> NonEmpty Mut List<T> => !first, !rest {
    let list = Mut List<T>()
    list.add(first)
    list.append(rest)
    return list
}

// Moves every element of an array onto the end of a list, in order,
// spending the array.
//
// This is how the elements of an array of linear values reach a list (the
// other way an array of them ends is its `drain`): a `for` may read such an
// array but not move its elements out, since a loop can stop early and
// leave the rest owed by nobody.
//
// ```
// let xs = Mut List(1)
// append(xs, array_of(2, 3))               // [1, 2, 3]
// ```
//
// @param list   the list to extend
// @param items  the array whose elements are moved onto the end
// @compilerrules [linear-container]
export platform fn append<T canbe linear>(list: Mut List<T>, items: T[]) [] -> None => list: Mut, !items

// Builds a list of a given size, each element computed from its index.
//
// The function is called once per index, from 0 up, and its effects are the
// call's. A size of 0 or less gives an empty list.
//
// ```
// let squares = list_by(4, (i) -> i * i)   // [0, 1, 4, 9]
// ```
//
// @param size  how many elements to build
// @param init  computes the element at each index
// @compilerrules [col-by]
export fn list_by<T, [E]>(size: Int, init: (Int) [E] -> T) -> List<T> => size, init {
    return mut_list_by(size, init)
}

// [list_by], answering a mutable list.
export fn mut_list_by<T, [E]>(size: Int, init: (Int) [E] -> T) -> Mut List<T>
=> size, init {
    let out = Mut List<T>()
    let i = 0
    while i < size {
        add(out, init(copy(i)))
        i = i + 1
    }
    return out
}

// The element at an index, or `None` when the index is out of range.
//
// The element is borrowed: a read-only view of the list's own element, so
// nothing is copied. Write `copy(get(xs, i))` to keep a value of your own,
// and use [at] to change the element in place. An index proven in range
// (`i is Idx(xs)`) picks the overload below, which answers the element
// itself.
//
// ```
// let xs = List(10, 20, 30)
// let second = get(xs, 1)                  // 10, as an Int?
// let n = get(xs, 7) ?: 0                  // 0: out of range
// ```
// @compilerknown
export platform fn get<T canbe linear>(list: List<T>, index: Int) [] -> (proj(list) T)? => list, index

// `Idx`, the claim about an index, is `std.core.index`'s: one `Idx` serves
// every container with a `size`.
// @compilerrules [col-idx]

// [get] at an index proven in range: answers the element itself, with no
// `None` to handle.
//
// A call reaches this overload whenever the index's type carries
// `Idx(list)`: after `i is Idx(xs)`, or for an index from `indices(xs)` or
// [find_first].
//
// ```
// let xs = List(10, 20, 30)
// let i = 2
// if i is Idx(xs) {
//     println("${get(xs, i) + 1}")         // 31
// }
// ```
// @compilerrules [col-idx, fn-overload-rank, col-of-nonempty]
// @compilerknown
export fn get<T canbe linear>(list: List<T>, index: Idx(list) Int) [] -> proj(list) T
=> list, index {
    // The host's read answers the element itself: going through the optional
    // [get] and `!` would read a stored `None` of a `List<T?>` as absence on a
    // backend that flattens `T??` (the JVM), and trap.
    return get_at(list, index + 0)
}

// The host's read; the proven index is in range.
// @compilerknown
platform fn get_at<T canbe linear>(list: List<T>, index: Int) [] -> proj(list) T => list, index

// Exchanges two elements of a list, at indices proven in range.
//
// With both indices proven, the exchange cannot miss, so this overload
// answers nothing; with either unproven, the overload below answers whether
// it swapped. Every `Idx` claim on the list survives the call.
//
// ```
// let xs = Mut List(1, 2, 3)
// let i = 0
// let j = 2
// if i is Idx(xs) && j is Idx(xs) {
//     swap(xs, i, j)                       // [3, 2, 1]
// }
// ```
// @compilerrules [col-idx, col-bounds, qual-preserve]
export fn swap<T>(list: Mut List<T>, i: Idx(list) Int, j: Idx(list) Int) [] -> None
=> list: Mut, list: preserve Idx, i, j {
    // Its own clause promises what its body's refined [swap] keeps: existing
    // `Idx` claims survive. The claims make failure impossible here, which is
    // what lets the total overload discharge the obligation without reading
    // it.
    ignore(swap(list, i + 0, j + 0))
    return None
}

// Writes a value at an index proven in range and answers the element it
// displaced.
//
// Nothing written is lost: the displaced element is handed back, which is
// what makes this write safe on a list of linear values. Taking an element
// out of a `List<T?>` is `replace(xs, i, None)`. A write moves no boundary,
// so every `Idx` claim on the list survives it.
//
// ```
// let xs = Mut List(1, 2, 3)
// let i = 1
// if i is Idx(xs) {
//     let old = replace(xs, i, 20)         // 2; xs is [1, 20, 3]
// }
// ```
//
// @param index  the position, proven in range by `Idx(list)`
// @param value  the new element, moved into the list
// @returns      the element that was at `index`, moved out
// @compilerrules [col-replace, linear-container, qual-preserve]
export platform fn replace<T canbe linear>(list: Mut List<T>, index: Idx(list) Int, value: T) [] -> T
=> list: Mut, index, !value

// A locator is what the Rust backend renders `Ref` as: position data
// crossing the closure boundary, materialized at the use site, which is why
// a generic algorithm may hand out mutable handles at all.
// @compilerrules [rs-loc]

// The accessor a generic algorithm needs to change an element in place:
// given a container and a position, a handle to the element there.
//
// A function that asks for `?Ref<C, L, T>` stays generic over what a
// position is (an index for a list, a key for a map, a cursor for a
// structure of your own), and the caller, which knows the concrete
// container, supplies `at`. The [Next] group does the same for elements;
// this one does it for places.
//
// ```
// struct Counter canbe Mut {
//     n: Int
// }
//
// fn bump_at<C, L>(c: C, l: L, ?Ref<C, L, Counter>) -> None {
//     let counter = at(c, l) ?: return
//     counter.n = counter.n + 1
// }
//
// fn bump_first(counters: List<Mut Counter>) -> None {
//     bump_at(counters, 0)                 // `at` is the list's
// }
// ```
// @compilerrules [col-locate, implicit-group, ref-handle]
export fns Ref<C, L, T> {
    fn at(c: C, l: L) -> ref(c) Mut T?
}

// A handle to the element at an index, through which it can be changed in
// place, or `None` when the index is out of range.
//
// [get] reads and `at` handles: a handle is a position in the list, and
// assigning to its fields changes the element where it is, with nothing
// copied out or put back. The handle is writable when the element type is
// `Mut X`, as in a `List<Mut Counter>`. Changing an element through it moves
// nothing; reshaping the list (adding or removing) ends the handle. See the
// Mutable Handles topic for using two handles at once.
//
// ```
// struct Counter canbe Mut {
//     n: Int
// }
//
// fn bump_second(counters: List<Mut Counter>) -> None {
//     let c = at(counters, 1) ?: return
//     c.n = c.n + 1                        // the list's own counter changes
// }
// ```
// @compilerrules [col-locate, ref-handle, cmp-groups, rs-path]
// @compilerknown
export platform fn at<T canbe linear>(list: List<T>, index: Int) [] -> ref(list) T?
=> list, index

// [at] at an index proven in range: answers the handle itself, with no
// `None` to handle.
// @compilerrules [col-idx, ref-handle, fn-overload-rank]
// @compilerknown
export fn at<T canbe linear>(list: List<T>, index: Idx(list) Int) [] -> ref(list) T
=> list, index {
    return at_proven(list, index + 0)
}

// The host's mint; the proven index is in range.
// @compilerknown
platform fn at_proven<T canbe linear>(list: List<T>, index: Int) [] -> ref(list) T
=> list, index

// Changes the element at an index proven in range, in place, by handing its
// handle to a function.
//
// Nothing is copied, moved out or put back. A write in place moves no
// boundary, so every `Idx` claim on the list survives, and a run of updates
// stays total.
//
// ```
// struct Counter canbe Mut {
//     n: Int
// }
//
// fn bump_first(counters: List<Mut Counter>) -> None {
//     let i = 0
//     if i is Idx(counters) {
//         update(counters, i, (c) -> { c.n = c.n + 1 })
//     }
// }
// ```
//
// @param index  the position, proven in range by `Idx(list)`
// @param f      receives the element's handle and changes it
// @compilerrules [col-update, ref-handle, qual-preserve]
export fn update<T, [E]>(list: List<Mut T>, index: Idx(list) Int, f: (elem: Mut T) [E] -> None) -> None
=> list: preserve Idx, index, f {
    f(at(list, index))
    return None
}

// Adds an element at the end of the list, moving it in.
//
// Amortized O(1). The list is known non-empty afterwards, and every `Idx`
// claim on it survives, since growing at the end moves no element.
//
// ```
// let xs: Mut List<Str> = Mut List()
// add(xs, "a")
// add(xs, "b")                             // [a, b]
// ```
// @compilerrules [deduce-syntax]
export platform fn add<T canbe linear>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem

// Takes the first element out of the list, or answers `None` when it is
// empty.
//
// The element is moved out: nothing is left behind and nothing is copied,
// which is how a linear value leaves a list one at a time. The elements
// after it shift down, so this is O(n); a `Deque` takes from the front in
// O(1). With `?: break`, it drives a take-until-empty loop:
//
// ```
// let queue = Mut List("a", "b")
// while true {
//     let next = remove_first(queue) ?: break
//     println(next)                        // a, then b
// }
// ```
// @compilerrules [linear-container, linear-union-arm]
export platform fn remove_first<T canbe linear>(list: Mut List<T>) [] -> T? => list: Mut

// Takes the element at an index out of the list, or answers `None` when the
// index is out of range.
//
// The element is moved out and the elements after it shift down, which is
// O(n) in the number after it.
//
// ```
// let xs = Mut List("a", "b", "c")
// let b = remove_at(xs, 1)                 // "b"; xs is [a, c]
// ```
// @compilerrules [linear-container]
export platform fn remove_at<T canbe linear>(list: Mut List<T>, index: Int) [] -> T?
=> list: Mut, index

// Puts an element at an index, shifting the elements from there on up by
// one, or hands it back when the index is out of range.
//
// The index may be the size, which appends. Below 0 or past the size,
// nothing moves and the element comes back, so a list of linear values
// cannot lose one.
//
// ```
// let xs = Mut List("a", "c")
// let fit = insert_at(xs, 1, "b")          // None; xs is [a, b, c]
// let back = insert_at(xs, 9, "z")         // "z": out of range
// ```
//
// @returns  `None` when the element went in; the element itself when the
//           index was out of range
// @compilerrules [col-insert, linear-container]
export platform fn insert_at<T canbe linear>(list: Mut List<T>, index: Int, elem: T) [] -> T?
=> list: Mut, index, !elem

// Takes the elements from one index up to (not including) another out of
// the list, and answers them as a new list, in order.
//
// The range is clamped to the list, so a bound out of range removes what
// there is, and an empty range removes nothing. [remove_front] and
// [remove_back] are written over it.
//
// ```
// let xs = Mut List(1, 2, 3, 4, 5)
// let mid = remove_range(xs, 1, 3)         // [2, 3]; xs is [1, 4, 5]
// ```
//
// @param from  the first index to remove
// @param to    the index after the last one to remove
// @compilerrules [col-remove-range]
export platform fn remove_range<T canbe linear>(list: Mut List<T>, from: Int, to: Int) [] -> Mut List<T>
=> list: Mut, from, to

// [swap] at indices not proven in range: answers, wrapped in a `Checked`,
// whether it swapped.
//
// The answer is `false` when either index is out of range, and then
// nothing moved. A swap moves no value in or out of the list, so it is the
// one reordering a list of linear values allows.
//
// A swap that silently did nothing would be a bug with no symptom, so the
// answer is a [Checked], which cannot be dropped by accident: `detach` it
// to read the answer, or `ignore` it to say that not looking is intended.
//
// ```
// let xs = Mut List(1, 2)
// let swapped = detach(swap(xs, 0, 9))     // false: nothing moved
// ignore(swap(xs, 0, 1))                   // [2, 1]
// ```
// @compilerrules [linear-container, col-bounds]
export fn swap<T canbe linear>(list: Mut List<T>, i: Int, j: Int) [] -> Checked<Bool>
=> list: Mut, i, j {
    // A `Bool` rather than a silent no-op because a swap that quietly did
    // nothing is a reordering bug with no symptom at the call, and rather
    // than the hosts' own behaviour because `Vec::swap` panics where a JVM
    // list throws: the same program would fail differently on the two
    // backends [backend-parity]. Wrapped in a `Checked` because the bug this
    // reports is invisible at the call site, which is exactly the case the
    // obligation exists for.
    return checked(swap_at(list, i, j))
}

// The host's exchange, answering whether both indices were in range.
platform fn swap_at<T canbe linear>(list: Mut List<T>, i: Int, j: Int) [] -> Bool => list: Mut, i, j

// There is deliberately **no** positional write for a list at an unproven
// index. `replace(list, index, elem) -> T?` looks like the map's, but a
// list index can be *out of range*, and then the value written has nowhere
// to go: answering `None` would drop it (a silent leak, exactly what this
// surface exists to prevent), handing it back would make "displaced" and
// "bounced" indistinguishable, and refusing at run time is not how the rest
// of std treats an index. The proven-index [replace] escapes the question,
// as [swap] does by moving nothing in or out.
// @compilerrules [linear-container, col-bounds]

// Consumes the list and hands every element to a function, in order.
//
// This is how a list of linear values ends: the list is spent, and each
// element goes on to the function, which consumes it. A drain either
// happens or it does not, so no path can stop halfway and leave elements
// owed by nobody, which is why this is a function rather than a loop. The
// function's effects are the call's.
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
// fn redeem_all(tickets: List<Ticket>) [Console] -> None => !tickets {
//     drain(tickets, redeem)
// }
// ```
//
// @param each  receives each element, moved in, in list order
// @compilerrules [linear-container, fn-effects]
export fn drain<T canbe linear, [E]>(list: List<T>, each: (x: T) [E] -> None) -> None
=> !list, each =>[each] !x {
    let m = into_mut(list)
    reverse(m)
    while size(m) > 0 {
        each(remove_last(m) ?: unreachable!())
    }
    end_empty(m)
}

// A list as one to empty, and the end of an emptied one:
// what [drain] is written with. Ending one that still holds elements traps.
// @compilerrules [linear-container]
platform fn into_mut<T canbe linear>(list: List<T>) [] -> Mut List<T> => !list
platform fn end_empty<T canbe linear>(list: Mut List<T>) [] -> None => !list

// Takes the last element out of the list, or answers `None` when it is
// empty.
//
// O(1): nothing shifts. Like [remove_first], the element is moved out.
//
// ```
// let xs = Mut List(1, 2, 3)
// let three = remove_last(xs)              // 3; xs is [1, 2]
// ```
// @compilerrules [linear-container]
export platform fn remove_last<T canbe linear>(list: Mut List<T>) [] -> T? => list: Mut

// The first element, borrowed, or `None` when the list is empty.
//
// On a list known to be `NonEmpty` (built with elements, or just added to)
// the overload below answers the element itself.
//
// ```
// let xs: List<Int> = List()
// let none = first(xs)                     // None
// let ys = List(1, 2)
// let one = first(ys)                      // 1, an Int
// ```
export platform fn first<T canbe linear>(list: List<T>) [] -> proj(list) T? => list

// [first] of a list known non-empty: answers the element itself.
// @compilerrules [col-nonempty, fn-overload-rank]
export fn first<T canbe linear>(list: NonEmpty List<T>) [] -> proj(list) T => list {
    // It delegates to `get`, not to `first`: `first@core.list(list)` would
    // pick *this* overload again (the scope selector names the module, and
    // within it a `NonEmpty` argument still ranks this one first) and recurse
    // forever.
    return get(list, 0) ?: unreachable!()
}

// The number of elements in the list.
export platform fn size<T canbe linear>(list: List<T>) [] -> Int => list

// ===== the list surface written in Salvo =====
//
// Everything below is ordinary Salvo over the intrinsics above (`get`, `size`,
// `add`, `insert_at`, `remove_range`), so a new backend implements none of it.
// @compilerrules [col-salvo]

// The last element, borrowed, or `None` when the list is empty.
// @compilerrules [col-salvo]
export fn last<T canbe linear>(list: List<T>) [] -> proj(list) T? => list {
    return get(list, size(list) - 1)
}

// Whether the list has no elements.
// @compilerrules [col-salvo]
export fn is_empty<T canbe linear>(list: List<T>) [] -> Bool => list {
    return size(list) == 0
}

// Takes the first elements out of the list, a given number of them, and
// answers them as a new list, in order.
//
// When the list is shorter, all of it is taken; a count of 0 or less takes
// nothing.
//
// ```
// let xs = Mut List(1, 2, 3, 4, 5)
// let front = remove_front(xs, 2)          // [1, 2]; xs is [3, 4, 5]
// ```
// @compilerrules [col-salvo]
export fn remove_front<T canbe linear>(list: Mut List<T>, n: Int) [] -> Mut List<T> => list: Mut, n {
    return remove_range(list, 0, n)
}

// Takes the last elements out of the list, a given number of them, and
// answers them as a new list, in order.
//
// When the list is shorter, all of it is taken.
//
// ```
// let xs = Mut List(1, 2, 3, 4, 5)
// let back = remove_back(xs, 2)            // [4, 5]; xs is [1, 2, 3]
// ```
// @compilerrules [col-salvo]
export fn remove_back<T canbe linear>(list: Mut List<T>, n: Int) [] -> Mut List<T> => list: Mut, n {
    let at = size(list) - n
    if at < 0 {
        at = 0
    }
    return remove_range(list, at, size(list))
}

// Takes elements off the front of the list for as long as a predicate
// accepts them, and answers them as a new list, in order.
//
// The list is left starting at the first element the predicate refused.
//
// ```
// let xs = Mut List(1, 2, 7, 3)
// let small = remove_front_while(xs, (x: Int) -> x < 5)   // [1, 2]; xs is [7, 3]
// ```
// @compilerrules [col-salvo]
export fn remove_front_while<T canbe linear, [E]>(list: Mut List<T>, keep: (x: T) [E] -> Bool) -> Mut List<T>
=> list: Mut, keep {
    let n = 0
    while n < size(list) && keep(get(list, n) ?: unreachable!()) {
        n = n + 1
    }
    return remove_range(list, 0, n)
}

// Takes elements off the back of the list for as long as a predicate
// accepts them, and answers them as a new list, in list order.
//
// The list is left ending at the last element the predicate refused.
//
// ```
// let xs = Mut List(7, 3, 8, 9)
// let big = remove_back_while(xs, (x: Int) -> x > 5)      // [8, 9]; xs is [7, 3]
// ```
// @compilerrules [col-salvo]
export fn remove_back_while<T canbe linear, [E]>(list: Mut List<T>, keep: (x: T) [E] -> Bool) -> Mut List<T>
=> list: Mut, keep {
    let at = size(list)
    while at > 0 && keep(get(list, at - 1) ?: unreachable!()) {
        at = at - 1
    }
    return remove_range(list, at, size(list))
}

// A copy of the elements from one index up to (not including) another, as
// a new list.
//
// The range is clamped to the list, so a bound out of range copies what
// there is, and an empty range copies nothing. Each element is copied with
// its type's `copy`, found at the call site.
//
// ```
// let xs = List(1, 2, 3, 4)
// let mid = sub_list(xs, 1, 3)             // [2, 3]
// let all = sub_list(xs, -2, 99)           // [1, 2, 3, 4]
// ```
// @compilerrules [col-salvo]
export fn sub_list<T>(list: List<T>, from: Int, to: Int, ?copy: (v: T) -> T) [] -> Mut List<T>
=> list, from, to {
    // A copy rather than a view: a list of borrows cannot be answered from a
    // generic fn on the Rust backend yet, and a slice type, which is what a
    // view would really be, does not exist.
    let out = Mut List<T>()
    let i = from
    if i < 0 {
        i = 0
    }
    while i < to && i < size(list) {
        add(out, copy(get(list, i) ?: unreachable!()))
        i = i + 1
    }
    return out
}

// The index of the first element a predicate accepts, or `None`.
//
// The index answered is proven in range, so [get] at it answers the element
// itself. O(n): the elements are tried front to back until one is accepted.
//
// ```
// let xs = List(5, 8, 3, 8)
// let hit = find_first(xs, (x) -> x > 6)
// if hit is Int {
//     println("${hit}: ${get(xs, hit)}")   // 1: 8
// }
// ```
// @compilerrules [col-salvo]
export fn find_first<T, [E]>(list: List<T>, pick: (x: T) [E] -> Bool) -> (Idx(list) Int)? => list, pick {
    for i in indices(list) {
        if pick(get(list, i)) {
            return i
        }
    }
    return None
}

// The index of the last element a predicate accepts, or `None`.
//
// Like [find_first], from the back; the index is proven in range.
// @compilerrules [col-salvo]
export fn find_last<T, [E]>(list: List<T>, pick: (x: T) [E] -> Bool) -> (Idx(list) Int)? => list, pick {
    for i in rev_indices(list) {
        if pick(get(list, i)) {
            return i
        }
    }
    return None
}

// The index of the first element equal to a value, or `None`.
//
// Equality is the element type's `==`, found at the call site. O(n); the
// index answered is proven in range.
//
// ```
// let xs = List("a", "b", "a")
// let at = index_of(xs, "b")               // 1
// ```
// @compilerrules [col-salvo]
export fn index_of<T>(list: List<T>, elem: T, ?Eq<T>) [] -> (Idx(list) Int)? => list, elem {
    for i in indices(list) {
        if eq(get(list, i), elem) {
            return i
        }
    }
    return None
}

// The index of the last element equal to a value, or `None`.
//
// Like [index_of], from the back.
// @compilerrules [col-salvo]
export fn last_index_of<T>(list: List<T>, elem: T, ?Eq<T>) [] -> (Idx(list) Int)? => list, elem {
    for i in rev_indices(list) {
        if eq(get(list, i), elem) {
            return i
        }
    }
    return None
}

// Whether some element equals a value, by the element type's `==`.
//
// O(n). For many membership tests, a `Set` answers each in O(1).
//
// ```
// let xs = List("a", "b")
// let has = contains(xs, "b")              // true
// ```
// @compilerrules [col-salvo]
export fn contains<T>(list: List<T>, elem: T, ?Eq<T>) [] -> Bool => list, elem {
    let i = 0
    while i < size(list) {
        if eq(get(list, i) ?: unreachable!(), elem) {
            return true
        }
        i = i + 1
    }
    return false
}

// Whether a predicate accepts some element; `false` for an empty list.
//
// The predicate is asked front to back, and not again once it accepts one.
// @compilerrules [col-salvo]
export fn any<T, [E]>(list: List<T>, pick: (x: T) [E] -> Bool) -> Bool => list, pick {
    return !(find_first(list, pick) is None)
}

// Whether a predicate accepts every element; `true` for an empty list.
//
// The predicate is asked front to back, and not again once it refuses one.
//
// ```
// let xs = List(1, 2, 3)
// let positive = all(xs, (x) -> x > 0)     // true
// ```
// @compilerrules [col-salvo]
export fn all<T, [E]>(list: List<T>, pick: (x: T) [E] -> Bool) -> Bool => list, pick {
    let i = 0
    while i < size(list) {
        if !pick(get(list, i) ?: unreachable!()) {
            return false
        }
        i = i + 1
    }
    return true
}

// How many elements a predicate accepts.
//
// ```
// let xs = List(1, 2, 3)
// let big = count(xs, (x) -> x > 1)        // 2
// ```
// @compilerrules [col-salvo]
export fn count<T, [E]>(list: List<T>, pick: (x: T) [E] -> Bool) -> Int => list, pick {
    let n = 0
    let i = 0
    while i < size(list) {
        if pick(get(list, i) ?: unreachable!()) {
            n = n + 1
        }
        i = i + 1
    }
    return n
}

// Splits a list in two by a predicate: the elements it accepts, then the
// rest, each in list order.
//
// The elements are copied into the two new lists with their type's `copy`,
// found at the call site, since a tuple cannot hold borrows.
//
// ```
// let xs = List(1, 2, 3, 4)
// let (even, odd) = partition(xs, (x: Int) -> x % 2 == 0)  // [2, 4] and [1, 3]
// ```
// @compilerrules [col-salvo]
export fn partition<T, [E]>(list: List<T>, pick: (x: T) [E] -> Bool, ?copy: (v: T) -> T) -> (Mut List<T>, Mut List<T>)
=> list, pick {
    let yes = Mut List<T>()
    let no = Mut List<T>()
    let i = 0
    while i < size(list) {
        let x = get(list, i) ?: unreachable!()
        if pick(x) {
            add(yes, copy(x))
        } else {
            add(no, copy(x))
        }
        i = i + 1
    }
    return (yes, no)
}

// Reverses the list in place.
//
// Elements are exchanged, never copied, so a list of linear values can be
// reversed too. To walk a list backwards without changing it, use
// [reversed].
//
// ```
// let xs = Mut List(1, 2, 3)
// reverse(xs)                              // [3, 2, 1]
// ```
// @compilerrules [col-salvo]
export fn reverse<T canbe linear>(list: Mut List<T>) [] -> None => list: Mut {
    let i = 0
    let j = size(list) - 1
    while i < j {
        ignore(swap(list, copy(i), copy(j)))
        i = i + 1
        j = j - 1
    }
}

// A fresh iterator over the list, front to back.
//
// The iterator borrows the list: it is a position in it, so nothing is
// copied or consumed and the list stays usable. A `for` loop over a list
// needs no `iter`; it is what the functions of `std.core.seq` take.
//
// ```
// let xs = List(1, 2, 3, 4)
// for x in taking(iter(xs), 2) {
//     println("${x}")                      // 1, then 2
// }
// ```
// @compilerrules [iter-mint, proj-field, copy-opt-in]
export fn iter<T>(list: List<T>) [] -> Mut ListIter<T> => list {
    return Mut ListIter<T> { items: list, at: 0 }
}

// The iterator [iter] answers: a list, borrowed, and a position in it.
//
// An ordinary struct with an ordinary `next`: there is no special container
// protocol.
// @compilerrules [iter-protocol, iter-for-native]
export struct ListIter<T> : Next<self, proj T> canbe Mut {
    // The list being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    items: proj List<T>
    // The index of the next element to emit.
    at: Int
}

// The backends keep their native loop as a fast path for a `for` over a
// list, so the `ListIter` shape is what *combinators* see. A `proj` field
// makes the struct a view of whatever each literal stores there; `iter`
// lends its `list`, which the checker reads off its body.

// Answers the element at the iterator's position and moves past it, or the
// end once the position is past the last element.
export fn next<T>(p: Mut ListIter<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    // Out of range is the end: [get] answers `None` past the last index, so
    // the bound is read rather than remembered.
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}

// Walks a list back to front, borrowing it.
//
// An iterator, not a reversed copy: `for x in reversed(xs)` costs nothing
// per element beyond the walk, and the list is unchanged. For a reversed
// list of your own, write `to_list(reversed(xs))`, or [reverse] a `Mut List`
// in place.
//
// ```
// let xs = List(1, 2, 3)
// for x in reversed(xs) {
//     println("${x}")                      // 3, 2, 1
// }
// ```
// @compilerrules [col-reversed, proj-field]
export fn reversed<T>(list: List<T>) [] -> Mut ListReversedIter<T> => list {
    return Mut ListReversedIter<T> { items: list, at: size(list) - 1 }
}

// The iterator [reversed] answers: the list, borrowed, and the index of the
// next element to emit, counting down.
export struct ListReversedIter<T> : Next<self, proj T> canbe Mut {
    // The list being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    items: proj List<T>
    // The index of the next element to emit; `-1` is the end.
    at: Int
}

// Answers the element at the walk's position and moves one back, or the
// end once the first element has been emitted.
export fn next<T>(p: Mut ListReversedIter<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    // `-1` reads as `None` like any out-of-range index.
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at - 1
    return emitted(elem)
}

// `indices` and `rev_indices`, whose indices carry `Idx`, are
// `std.core.index`'s: establishing the claim is trusted only where it is
// declared.
// @compilerrules [col-idx]

// An element of a list together with its index: what [enumerate] and
// [enumerate_rev] emit.
//
// The element is borrowed from the list being walked; the index is the
// value's own.
// @compilerrules [col-enumerate, fate-derived-readonly, proj-field]
export struct Enumerated<T> {
    // The element's index in the list.
    index: Int
    // The element, borrowed.
    elem: proj T
}

// A view struct, not a tuple: the element part is a borrow of the walked
// list, a tuple literal cannot *store* a projection (the store is a move,
// and the remedy `copy` would charge every step), and a `proj` **field** is
// exactly the declared lend the tuple lacks.

// Walks a list front to back, pairing each element with its index.
//
// The loop that wants positions without index arithmetic. Each pair is an
// [Enumerated]: `pair.index` and `pair.elem`, the element borrowed.
//
// ```
// let xs = List("a", "b")
// for pair in enumerate(xs) {
//     println("${pair.index}: ${pair.elem}")   // 0: a, then 1: b
// }
// ```
// @compilerrules [col-enumerate]
export fn enumerate<T>(list: List<T>) [] -> Mut ListEnumerateIter<T> => list {
    return Mut ListEnumerateIter<T> { items: list, at: 0, step: 1 }
}

// The pairs of [enumerate], back to front: from the last index down to 0.
//
// ```
// let xs = List("a", "b")
// for pair in enumerate_rev(xs) {
//     println("${pair.index}: ${pair.elem}")   // 1: b, then 0: a
// }
// ```
// @compilerrules [col-enumerate]
export fn enumerate_rev<T>(list: List<T>) [] -> Mut ListEnumerateIter<T> => list {
    return Mut ListEnumerateIter<T> { items: list, at: size(list) - 1, step: -1 }
}

// The iterator [enumerate] and [enumerate_rev] answer: the list, borrowed,
// the index of the next element, and the direction.
export struct ListEnumerateIter<T> : Next<self, Enumerated<T>> canbe Mut {
    // The list being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    items: proj List<T>
    // The index of the next element to emit.
    at: Int
    // `1` front to back, `-1` back to front.
    step: Int
}

// Answers the next element with its index and moves on, or the end. The
// emitted pair borrows the list.
// @compilerrules [col-enumerate]
export fn next<T>(p: Mut ListEnumerateIter<T>) [] -> Emitted Enumerated<T> | Finished holds proj(p) => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    // Copied, not linked: the emitted index must survive `at`'s step below.
    let index = p.at.copy()
    p.at = p.at + p.step
    return emitted(Enumerated<T> { index: index, elem: elem })
}

// The text form of a list, `[1, 2, 3]`: what `"${xs}"` prints.
//
// Elements are separated by `, ` and each is rendered by its own `to_str`,
// found at the call site, so a list of your own structs prints once the
// struct has a `to_str`.
//
// ```
// let xs = List("a", "b")
// println("${xs}")                         // [a, b]
// ```
// @compilerrules [interp-to-str]
export fn to_str<T>(list: List<T>, ?to_str: (x: T) -> Str) [] -> Str => list {
    let out = Mut Str("[")
    let i = 0
    for x in list {
        if i > 0 {
            append(out, ", ")
        }
        append(out, to_str(x))
        i = i + 1
    }
    append(out, "]")
    return out
}

// The claim that a list's elements are in order, under a named ordering.
//
// A `Sorted List<T>` is an ordinary list that the compiler knows to be
// sorted; it reaches every list function, and the claim costs nothing at
// run time. [sort] and [mut_sort] establish it, [add_sorted] keeps it while
// adding, and [binary_search] demands it. Any other change to the list gives
// it up. There is no `is Sorted` test: finding out whether a list happens to
// be sorted would mean comparing its elements, so the claim comes only from
// sorting.
//
// The claim **names the ordering** the list is sorted by, `Sorted<T>(cmp)`.
// A list sorted by one ordering is a different type from one sorted by
// another, so [add_sorted] and [binary_search] always work in the order the
// list is actually in. A signature that accepts any ordering writes the
// claim without one: `xs: Sorted List<T>`.
//
// This is a claim about a list, not a collection of its own: for a set or
// map kept in order as it changes, use `SortedSet` or `SortedMap`
// (`std.core.sorted`).
//
// ```
// fn by_len(a: Str, b: Str) -> Int {
//     return cmp(size(a), size(b))
// }
//
// fn example() -> None {
//     let nums = sort(List(40, 10, 30))                  // [10, 30, 40]
//     let at = binary_search(nums, 30)                   // 1
//     let live: Mut Sorted List<Int> = mut_sort(List(10, 30))
//     add_sorted(live, 20)                               // [10, 20, 30]
//     let words = sort(List("pear", "fig", "Apple"), cmp = by_len)   // [fig, pear, Apple]
//     let kiwi = binary_search(words, "kiwi")            // 1: found by length
// }
// ```
// @compilerrules [col-sorted-list, col-sorted, cmp-carry]
// @compilerknown
export qualifier Sorted<T>(?cmp: (T, T) -> Int) for List<T> {
    // The claim's owner states what the insert below does to it, because the
    // primitive cannot: a function that mutates may not promise back a
    // qualifier it has never heard of, and inserting at the lower bound is
    // exactly the operation that keeps a list ordered. Sound because the
    // position is computed with `cmp`, the ordering the claim names, which is
    // why the claim carries it.
    // @compilerrules [qual-refn, deduce-syntax]
    refn insert_sorted_by<T>(list: Mut List<T>, elem: T, cmp: (T, T) -> Int)
    => list: +Sorted
}

// ===== the ordering-taking primitives =====
//
// The three operations below are ordinary Salvo over these, which is what
// lets them take the ordering as an **implicit** parameter: no
// `intrinsic fn` takes one, since its lowering is a template over rendered
// arguments. So the binding happens in Salvo and each primitive is handed the
// comparator as an ordinary fn-typed argument. Private to this module: what a
// program sees is the four functions below.
//
// The Kotlin side finds the lower bound by a scan (`indexOfFirst`), the Rust
// side by `partition_point`; both land on the same index.
// @compilerrules [col-sorted-list]

// Answers the elements of [list] in the order [cmp] puts them in. A stable
// sort on both backends, so equal elements keep their relative order.
platform fn sort_by<T>(list: List<T>, cmp: (T, T) -> Int) [] -> Mut List<T>
=> list, cmp

// Inserts [elem] at the **lower bound** for [cmp] — before any element that
// ties with it — which is the position that keeps the list ordered.
platform fn insert_sorted_by<T>(list: Mut List<T>, elem: T, cmp: (T, T) -> Int) [] -> None
=> list: Mut, !elem, cmp

// The **lowest** index that ties with [elem] under [cmp], or `None`. A tie is
// `cmp(a, b) == 0`, never the host's equality: that is the only test
// consistent with the ordering the `Sorted` claim names.
// @compilerrules [col-membership]
platform fn search_sorted_by<T>(list: List<T>, elem: T, cmp: (T, T) -> Int) [] -> Int?
=> list, elem, cmp

// A sorted copy of a list, carrying the claim that it is sorted and by
// which ordering.
//
// The ordering is the element type's `cmp`, found at the call site, or the
// one passed as `cmp = ...`; the answer is `Sorted<T>(cmp)` for exactly that
// ordering. The sort is stable: equal elements keep their order. The
// comparison is Salvo's own on both backends: a `cmp` you wrote for a struct
// is what sorts a list of it, and strings compare by code point.
//
// ```
// let xs = sort(List(3, 1, 2))             // [1, 2, 3]
// let at = binary_search(xs, 2)            // 1
// ```
// @compilerrules [cmp-carry, col-sorted]
export fn sort<T>(list: List<T>, ?Ordered<T>) [] -> +Sorted<T>(?cmp) List<T> => list {
    return sort_by(list, cmp)
}

// [sort], answering a mutable list: how a `Mut Sorted List<T>`, which
// [add_sorted] inserts into, is obtained.
//
// `mut_sort(List<Int>())` is an empty sorted list to fill.
//
// ```
// let live: Mut Sorted List<Int> = mut_sort(List(30, 10))
// add_sorted(live, 20)                     // [10, 20, 30]
// ```
export fn mut_sort<T>(list: List<T>, ?Ordered<T>) [] -> +Sorted<T>(?cmp) Mut List<T>
=> list {
    return sort_by(list, cmp)
}

// Inserts an element into a sorted list, at the position that keeps it
// sorted.
//
// The list keeps its `Sorted` claim. An element equal to some already there
// goes before them, so the result is the same on both backends. The
// position is found with the ordering the list was sorted by, carried in its
// type, not with one resolved at this call. The elements after the position
// shift up, so this is O(n).
//
// ```
// let live: Mut Sorted List<Int> = mut_sort(List(10, 30))
// add_sorted(live, 20)                     // [10, 20, 30]
// ```
// @compilerrules [col-sorted-list, cmp-binder, deduce-syntax, qual-refn]
export fn add_sorted<T>(list: Mut Sorted<T>(?cmp) List<T>, elem: T) [] -> None
=> list: Mut Sorted, !elem {
    // The exhaustive deduction names `Sorted` itself: a mutating function must
    // state what survives, and this is the one function that genuinely knows
    // the claim does, so it needs no refinement from the qualifier, unlike
    // `add`, which cannot promise `NonEmpty`.
    insert_sorted_by(list, elem, cmp)
}

// Finds an element in a sorted list and answers its index, or `None`.
//
// Only a sorted list is accepted: over an unordered one the answer would
// mean nothing. An element matches when the list's ordering ties it with
// the one asked for (`cmp(a, b) == 0`), not by `==`; with several matches,
// the lowest index is answered, on both backends. The index answered is
// proven in range, so [get] at it answers the element itself. The Rust
// backend searches in O(log n); the Kotlin backend currently scans, O(n).
//
// ```
// let xs = sort(List(30, 10, 20))
// let found = binary_search(xs, 20)
// if found is Int {
//     println("${get(xs, found)}")         // 20
// }
// ```
// @compilerrules [col-sorted-list, col-membership]
export fn binary_search<T>(list: Sorted<T>(?cmp) List<T>, elem: T) [] -> (Idx(list) Int)?
=> list, elem {
    // The host's search answers an index in range or nothing; the test is
    // what proves it here, since `Idx` is established only in `std.core.index`.
    let found = search_sorted_by(list, elem, cmp) ?: (return None)
    if found is Idx(list) {
        return found
    }
    return None
}

// Whether two lists hold equal elements in the same order: what `a == b`
// means for lists.
//
// Elements compare by their own type's `==`, found at the call site, so a
// `List<Person>` compares by the `eq` declared for `Person`. Lists of
// different lengths are never equal.
//
// ```
// let same = List(1, 2) == List(1, 2)      // true
// ```
// @compilerrules [op-unique-shape, col-hashed-ordered, implicit-recursive]
export fn eq<T>(a: List<T>, b: List<T>, ?eq: (x: T, y: T) -> Bool) [] -> Bool => a, b {
    // An operator on a type is declared in the type's own file. Ordinary Salvo
    // over the implicits of the element types: resolving one of these for
    // `List<Person>` fills the element's in turn. (The list's `hash`, not an
    // operator, is in `std.core.compare`.)
    if size(a) != size(b) {
        return false
    }
    for p in enumerate(a) {
        let j = p.index
        if j is Idx(b) {
            if !eq(p.elem, get(b, j)) {
                return false
            }
        }
    }
    return true
}

// Compares two lists element by element: what `<`, `<=`, `>` and `>=` mean
// for lists.
//
// The first pair of elements that differ decides, by the element type's
// `cmp`; when one list is a prefix of the other, the shorter comes first.
// The answer is negative, zero or positive, as for any `cmp`.
//
// ```
// let before = List(1, 2) < List(1, 3)     // true
// let prefix = List(1, 2) < List(1, 2, 0)  // true
// ```
export fn cmp<T>(a: List<T>, b: List<T>, ?cmp: (x: T, y: T) -> Int) [] -> Int => a, b {
    for p in enumerate(a) {
        let j = p.index
        if j is Idx(b) {
            let c = cmp(p.elem, get(b, j))
            if c != 0 {
                return c
            }
        } else {
            return 1
        }
    }
    if size(a) < size(b) {
        return -1
    }
    return 0
}
