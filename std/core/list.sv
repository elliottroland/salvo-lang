// `canbe Mut` opts List into the language-level Mut auto-qualifier
// [type-canbe-mut]: a backend may map `Mut List<T>` to a different native
// type (Kotlin's `MutableList<T>`) or to the same one (Rust's `Vec<T>`,
// where mutability lives in the binding).
//
// [linear-container] `<T canbe linear>` is what lets a list **hold
// obligations**: `List<Reply<Int>>` is a linear type — it owes, and its
// terminal is [drain] — while `List<Int>` is an ordinary list. The
// element's declaration is the source of the linearity; a list never
// spells it.
export intrinsic type List<T canbe linear> canbe Mut

// Constructors, in two shapes [col-of-nonempty] (user decision 2026-09-23).
// Writing a *first* element establishes `NonEmpty` **by construction**, so no
// `qualifies` call is emitted [qual-ctor-predicate] and the claim costs nothing:
//
//     list_of()             // List<T>, empty
//     list_of(1, 2, 3)      // NonEmpty List<T> — the claim comes free
//     list_of(a, ...rest)   // NonEmpty List<T> — one written element is enough
//
// A **lone** `...spread` reaches neither: a spread cannot supply a required
// parameter [fn-variadic] (its length is not known here, so `NonEmpty` would be
// a guess) and the empty shape takes no arguments. Building a list out of an
// array is `map_to(mut_list_of(), iter(array), x -> x)` or a loop with `add`.
//
// The elements are stored in the new list, so they are moved: a variadic tail is
// owned, and needs no entry in the clause [deduce-syntax].
export intrinsic fn list_of<T canbe linear>() [] -> List<T>
export intrinsic fn list_of<T canbe linear>(first: T, ...rest: T[]) [] -> +NonEmpty List<T>
=> !first

// The same pair, mutable. A `Mut NonEmpty` list is a claim the next mutation
// may take away — which is exactly what a state qualifier is [qual-subject] —
// so `remove_at` strips it and `add` puts it back [qual-refn].
export intrinsic fn mut_list_of<T canbe linear>() [] -> Mut List<T>
export intrinsic fn mut_list_of<T canbe linear>(first: T, ...rest: T[]) [] -> +NonEmpty Mut List<T>
=> !first

// [col-by] Builds a list of [size] elements, each from its index:
// `list_by(3, i -> i * 2)` is `[0, 2, 4]`. The generator is called once per
// index, in order.
export intrinsic fn list_by<T>(size: Int, init: (Int) -> T) [] -> List<T> => size, init

// Mutable variant
export intrinsic fn mut_list_by<T>(size: Int, init: (Int) -> T) [] -> Mut List<T>
=> size, init

// Possibly gets the element at the given index if the list is long enough
export intrinsic fn get<T canbe linear>(list: List<T>, index: Int) [] -> (proj(list) T)? => list, index

// [qual-depend] [col-idx] The claim that an `Int` is a **valid index of one
// particular list**: `0 <= index < size(list)`, bound to that list's
// identity, so `Idx(xs) Int` and `Idx(ys) Int` are different facts. Tested
// with the filled block (`i is Idx(xs)`), stripped by any mutation of the
// list; the total [get] and [swap] overloads below consume it.
export qualifier Idx<T>(list: List<T>) of Int {
    fn qualifies(index: Int, list: List<T>) -> Bool {
        return index >= 0 && index < size(list)
    }

    // [qual-preserve] Growth keeps every existing index valid, and an
    // exchange moves no boundary: `Idx` claims survive both — the opt-back
    // from the conservative rule that any mutation of the list strips them.
    refn add(list: Mut List<T>, elem: T) => list: preserve Idx
    refn swap(list: Mut List<T>, i: Int, j: Int) => list: preserve Idx
}

// [qual-depend] [col-noteq] The claim that an `Int` **differs from one
// particular other `Int`**: `j is NotEq(i)` proves `j != i`, bound to
// `i`'s identity. For two element handles of one list it is the proof that
// they cannot alias — mutation through one leaves the other standing
// [elem-distinct], and a call may take both at once — which is what the
// `update2` family stands on. Reassigning either side strips it
// [qual-depend], like any dependent claim.
export qualifier NotEq(i: Int) of Int with Idx {
    fn qualifies(j: Int, i: Int) -> Bool {
        return j != i
    }
}

// [col-idx] The **total** read: an index carrying the claim answers the
// element itself — no `None` arm, nothing to `!`. Ranked above the optional
// [get] by its qualifier [fn-overload-rank], exactly as `first` over a
// `NonEmpty` list is [col-of-nonempty].
export fn get<T canbe linear>(list: List<T>, index: Idx(list) Int) [] -> proj(list) T
=> list, index {
    // `index + 0` re-derives a plain `Int`: delegating with the claim still
    // attached re-picks this overload and recurses — the same trap
    // `first(NonEmpty)` dodges by not delegating to itself [col-of-nonempty].
    return get(list, index + 0)!
}

// [col-idx] The **total** exchange: two proven indices cannot be out of
// range, so there is no `Bool` to check — the claim did the checking
// [col-bounds]. Its own clause promises what its body's refined [swap]
// keeps: existing `Idx` claims survive [qual-preserve].
export fn swap<T>(list: Mut List<T>, i: Idx(list) Int, j: Idx(list) Int) [] -> None
=> list: Mut, list: preserve Idx, i, j {
    // The claims make failure impossible here, which is what lets the total
    // overload discharge the obligation without reading it.
    ignore(swap(list, i + 0, j + 0))
    return None
}

// [col-locate] What a **position-based** algorithm needs, as a params group
// [implicit-group]: one function turning a container and a position into the
// element's mutable handle. The caller — which knows the concrete shape —
// fills it, so the algorithm itself stays generic over *what a position is*:
// an `Idx` for a list, a key for a map, a cursor for a structure of your own.
// The `Yield` pattern, for places rather than elements.
//
// A locator is what the Rust backend renders this as [rs-loc]: position data
// crossing the closure boundary, materialized at the use site — which is why
// a generic algorithm may hand out mutable handles at all.
export params Locate<C, L, T> {
    fn at(c: C, l: L) -> proj(c) Mut T?
}

// [col-locate] The canonical `at` for a list, which is `get` under the
// group's name — the way `cmp`/`eq` have canonical implementations for the
// intrinsic types [cmp-groups]. A position for a list is its index.
export fn at<T canbe linear>(list: List<Mut T>, index: Int) [] -> proj(list) Mut T?
=> list, index {
    return get(list, index)
}

// [col-update] Applies [f] to the element at [index], **in place**: the
// callback receives the mutable element handle [proj-mut], so nothing is
// copied, moved out, or put back. An in-place write moves no boundary, so
// existing `Idx` claims survive the call [qual-preserve].
export fn update<T>(list: List<Mut T>, index: Idx(list) Int, f: (elem: Mut T) -> None) [] -> None
=> list: preserve Idx, index, f {
    f(get(list, index))
    return None
}

// [col-update] Applies [f] to the elements at [i] and [j] at once — the
// two-handle transaction. The indices must be proven apart (`j is
// NotEq(i)` [col-noteq]): two handles to one element cannot exist
// [elem-distinct], and with the proof the pair costs one `split_at_mut`
// on the Rust backend [rs-elem-mut]. Ordinary Salvo, not an intrinsic —
// the body is exactly the two mints the proof legalizes.
export fn update2<T>(list: List<Mut T>, i: Idx(list) Int, j: NotEq(i) Idx(list) Int,
                     f: (a: Mut T, b: Mut T) -> None) [] -> None
=> list: preserve Idx, i, j, f {
    f(get(list, i), get(list, j))
    return None
}

// Adds an element to the list. The list takes ownership of `elem`, so it
// is moved; the list itself is mutated, which is why its surviving
// qualifiers are listed exhaustively [deduce-syntax].
export intrinsic fn add<T canbe linear>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem

// [linear-container] Takes the **first** element out of the list, or answers
// `None` when it is empty: the way an obligation leaves a list one at a time.
// The element is *moved* out — nothing is left behind and nothing is
// duplicated, which is what makes it legal for a `List<Reply<T>>` where [get]
// (an alias) is not.
//
// The `T?` shape is the whole absence story: the `None` arm owes nothing, so
// the emptiness check *is* the union narrow [linear-union-arm]. Pairs with
// `while` for a take-until-empty loop, and with [drain] for the terminal.
export intrinsic fn remove_first<T canbe linear>(list: Mut List<T>) [] -> T? => list: Mut

// [linear-container] The same, at an index: the element at [index] is moved
// out and the elements after it shift down. `None` when the index is past the
// end.
export intrinsic fn remove_at<T canbe linear>(list: Mut List<T>, index: Int) [] -> T?
=> list: Mut, index

// [col-insert] Puts [elem] at [index], shifting the elements from there on
// up by one; [index] may be the size, which appends. Out of range (below 0 or
// past the size), nothing moves and [elem] is **handed back**, so a list of
// obligations cannot lose one [linear-container].
export intrinsic fn insert_at<T canbe linear>(list: Mut List<T>, index: Int, elem: T) [] -> T?
=> list: Mut, index, !elem

// [col-remove-range] Takes the elements at indices [from] up to (not
// including) [to] out of the list, in order, and answers them. The range is
// clamped to the list, so an out-of-range bound removes what there is: the
// primitive the `remove_front`/`remove_back` family is written over.
export intrinsic fn remove_range<T canbe linear>(list: Mut List<T>, from: Int, to: Int) [] -> Mut List<T>
=> list: Mut, from, to

// [linear-container] Exchanges the elements at [i] and [j]. **Total**: no value
// enters the list and none leaves it, which is what makes it the one positional
// write a list of obligations can have — nothing can be dropped by it.
//
// [col-bounds] Answers **`false`** when either index is out of range, and then
// nothing moved (user decision 2026-09-22). A `Bool` rather than a silent no-op
// because a swap that quietly did nothing is a reordering bug with no symptom at
// the call, and rather than the hosts' own behaviour because `Vec::swap` panics
// where a JVM list throws — the same program would fail differently on the two
// backends [backend-parity].
//
// Wrapped in a [Checked], so the answer cannot be dropped by accident (user
// decision 2026-09-26): the bug this reports is invisible at the call site, which
// is exactly the case the obligation exists for. `detach` it to read the answer,
// or `ignore` it to say that not looking was the intent. The **total** overload
// above answers plain `None` — where both indices carry `Idx` claims there is no
// failure to check.
export intrinsic fn swap<T canbe linear>(list: Mut List<T>, i: Int, j: Int) [] -> Checked<Bool>
=> list: Mut, i, j

// [linear-container] There is deliberately **no** positional write for a list
// of obligations. `replace(list, index, elem) -> T?` looks like the map's, but
// a list index can be *out of range*, and then the value written has nowhere
// to go: answering `None` would drop it (a silent leak, exactly what this
// surface exists to prevent), handing it back would make "displaced" and
// "bounced" indistinguishable, and refusing at run time is not how the rest of
// std treats an index [col-bounds]. Take the element out and add a new one, or
// key the collection with a `Map`, whose `replace` has no such hole. [swap] is
// the exchange that escapes the whole question by moving nothing in or out.

// [linear-container] The **terminal**: consumes the list and hands every
// element to [each], in order. This is how a list of obligations ends — the
// container is spent, and each element's obligation continues into the
// callback, which consumes it (`=>[each] !x`).
//
// It is a function rather than a `for`-shaped iterator because there is then no
// half-drained state to account for: a drain either happened or did not, and
// no path can drop the elements it did not reach. The callback's own effects
// travel to the caller [fn-effects], so draining into an effectful discharger
// needs no annotation here.
export intrinsic fn drain<T canbe linear>(list: List<T>, each: (x: T) -> None) [] -> None
=>[each] !x => !list, each

export intrinsic fn first<T canbe linear>(list: List<T>) [] -> proj(list) T? => list

// [col-nonempty] The claim that a list has at least one element, and the
// reason the qualifier machinery is worth having over a container: with it,
// `first` answers with an element instead of an optional.
//
// Only one qualifier of a given *name* is visible across the implicitly
// visible `core` modules, so `NonEmpty` is a claim about a **list**
// specifically; `Set`/`Map` and the sorted pair have no equivalent yet (see
// ROADMAP.md).
export qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) [] -> Bool {
        return size(list) > 0
    }

    // The claim's *owner* states this, because `add` cannot: a function that
    // mutates may not promise back a qualifier it has never heard of
    // [deduce-syntax], and appending is exactly the operation that makes a
    // list non-empty [qual-refn].
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty

    // [qual-refn-narrow] And exchanging two elements of a list that is
    // *already* non-empty leaves it non-empty — which is why the parameter is
    // written `NonEmpty Mut List<T>` and not `Mut List<T>`: swapping does not
    // *make* a list non-empty, so the claim is **kept**, not established. A
    // refinement whose parameter is narrower than the declaration it refines
    // states exactly that, and applies only where the claim is already there
    // (user correction 2026-09-23).
    //
    // `swap`'s own exhaustive clause has to strip the claim, like every
    // mutator's; this is the owner saying what the call really leaves behind.
    // It is what lets a heap's sift-down keep the `NonEmpty` it was handed
    // (`std.heap`'s `heapify`).
    refn swap(list: NonEmpty Mut List<T>, i: Int, j: Int) => list: +NonEmpty
}

// [col-nonempty] The dividend: the same accessor, without the optional.
// Ranked above the plain `first` because it demands more of its argument
// [fn-overload-rank].
//
// It delegates to `get`, not to `first`: `first@core.list(list)` would pick
// *this* overload again — the scope selector names the module, and within it
// a `NonEmpty` argument still ranks this one first — and recurse forever.
export fn first<T canbe linear>(list: NonEmpty List<T>) [] -> proj(list) T => list {
    return get(list, 0)!
}

// Returns the number of elements in the list
export intrinsic fn size<T canbe linear>(list: List<T>) [] -> Int => list

// ===== the list surface written in Salvo [col-salvo] =====
//
// Everything below is ordinary Salvo over the intrinsics above (`get`, `size`,
// `add`, `insert_at`, `remove_range`), so a new backend implements none of it.

// [col-salvo] The last element, or `None` for an empty list.
export fn last<T canbe linear>(list: List<T>) [] -> proj(list) T? => list {
    return get(list, size(list) - 1)
}

// [col-salvo] Whether the list has no elements.
export fn is_empty<T canbe linear>(list: List<T>) [] -> Bool => list {
    return size(list) == 0
}

// [col-salvo] Takes the first [n] elements out of the list and answers them, in
// order — all of them when there are fewer.
export fn remove_front<T canbe linear>(list: Mut List<T>, n: Int) [] -> Mut List<T> => list: Mut, n {
    return remove_range(list, 0, n)
}

// [col-salvo] Takes the last [n] elements out of the list and answers them, in
// order — all of them when there are fewer.
export fn remove_back<T canbe linear>(list: Mut List<T>, n: Int) [] -> Mut List<T> => list: Mut, n {
    let at = size(list) - n
    if at < 0 {
        at = 0
    }
    return remove_range(list, at, size(list))
}

// [col-salvo] Takes elements off the front while [keep] accepts them, and
// answers them in order: the list is left starting at the first one it
// refused.
export fn remove_front_while<T canbe linear>(list: Mut List<T>, keep: (x: T) -> Bool) [] -> Mut List<T>
=> list: Mut, keep {
    let n = 0
    while n < size(list) && keep(get(list, n)!) {
        n = n + 1
    }
    return remove_range(list, 0, n)
}

// [col-salvo] Takes elements off the back while [keep] accepts them, and
// answers them in list order: the list is left ending at the last one it
// refused.
export fn remove_back_while<T canbe linear>(list: Mut List<T>, keep: (x: T) -> Bool) [] -> Mut List<T>
=> list: Mut, keep {
    let at = size(list)
    while at > 0 && keep(get(list, at - 1)!) {
        at = at - 1
    }
    return remove_range(list, at, size(list))
}

// [col-salvo] A copy of the elements at [from] up to (not including) [to],
// clamped to the list. A copy rather than a view: a list of borrows cannot be
// answered from a generic fn on the Rust backend yet, and a slice type, which
// is what a view would really be, does not exist.
export fn sub_list<T>(list: List<T>, from: Int, to: Int, ?copy: (v: T) -> T) [] -> Mut List<T>
=> list, from, to {
    let out = mut_list_of<T>()
    let i = from
    if i < 0 {
        i = 0
    }
    while i < to && i < size(list) {
        add(out, copy(get(list, i)!))
        i = i + 1
    }
    return out
}

// [col-salvo] The index of the first element [pick] accepts, or `None`.
export fn find_first<T>(list: List<T>, pick: (x: T) -> Bool) [] -> (+Idx(list) Int)? => list, pick {
    let i = 0
    while i < size(list) {
        if pick(get(list, i)!) {
            return i
        }
        i = i + 1
    }
    return None
}

// [col-salvo] The index of the last element [pick] accepts, or `None`.
export fn find_last<T>(list: List<T>, pick: (x: T) -> Bool) [] -> (+Idx(list) Int)? => list, pick {
    let i = size(list) - 1
    while i >= 0 {
        if pick(get(list, i)!) {
            return i
        }
        i = i - 1
    }
    return None
}

// [col-salvo] The index of the first element equal to [elem], or `None`.
export fn index_of<T>(list: List<T>, elem: T, ?Eq<T>) [] -> (+Idx(list) Int)? => list, elem {
    let i = 0
    while i < size(list) {
        if eq(get(list, i)!, elem) {
            return i
        }
        i = i + 1
    }
    return None
}

// [col-salvo] The index of the last element equal to [elem], or `None`.
export fn last_index_of<T>(list: List<T>, elem: T, ?Eq<T>) [] -> (+Idx(list) Int)? => list, elem {
    let i = size(list) - 1
    while i >= 0 {
        if eq(get(list, i)!, elem) {
            return i
        }
        i = i - 1
    }
    return None
}

// [col-salvo] Whether some element equals [elem].
export fn contains<T>(list: List<T>, elem: T, ?Eq<T>) [] -> Bool => list, elem {
    let i = 0
    while i < size(list) {
        if eq(get(list, i)!, elem) {
            return true
        }
        i = i + 1
    }
    return false
}

// [col-salvo] Whether [pick] accepts some element.
export fn any<T>(list: List<T>, pick: (x: T) -> Bool) [] -> Bool => list, pick {
    return !(find_first(list, pick) is None)
}

// [col-salvo] Whether [pick] accepts every element (true for an empty list).
export fn all<T>(list: List<T>, pick: (x: T) -> Bool) [] -> Bool => list, pick {
    let i = 0
    while i < size(list) {
        if !pick(get(list, i)!) {
            return false
        }
        i = i + 1
    }
    return true
}

// [col-salvo] How many elements [pick] accepts.
export fn count<T>(list: List<T>, pick: (x: T) -> Bool) [] -> Int => list, pick {
    let n = 0
    let i = 0
    while i < size(list) {
        if pick(get(list, i)!) {
            n = n + 1
        }
        i = i + 1
    }
    return n
}

// [col-salvo] Splits the list in two by [pick]: the elements it accepts, then
// the rest, each in order. Copies the elements, as [filter_to] does, since a
// tuple cannot hold borrows.
export fn partition<T>(list: List<T>, pick: (x: T) -> Bool, ?copy: (v: T) -> T) [] -> (Mut List<T>, Mut List<T>)
=> list, pick {
    let yes = mut_list_of<T>()
    let no = mut_list_of<T>()
    let i = 0
    while i < size(list) {
        let x = get(list, i)!
        if pick(x) {
            add(yes, copy(x))
        } else {
            add(no, copy(x))
        }
        i = i + 1
    }
    return (yes, no)
}

// [col-salvo] Reverses the list in place.
export fn reverse<T canbe linear>(list: Mut List<T>) [] -> None => list: Mut {
    let i = 0
    let j = size(list) - 1
    while i < j {
        ignore(swap(list, copy(i), copy(j)))
        i = i + 1
        j = j - 1
    }
}

// [iter-mint] A fresh iterator over the list, which is what `for x in iter(xs)`
// and every combinator walks. The iterator **borrows** the list — it is a view
// with a position [proj-field] — so iterating a list by hand keeps it
// usable, and nothing is copied or consumed on the way [copy-opt-in].
export fn iter<T>(list: List<T>) [] -> Mut ListYield<T> => list {
    return Mut ListYield<T> { items: list, at: 0 }
}

// [iter-protocol] The pass a list is walked by: the list plus a position in
// it. An ordinary struct with an ordinary `next` — there is no special
// container protocol, which is what "an iterator is a user struct" means. The
// backends keep their native loop as a fast path for a `for` over a list
// [iter-for-native], so this shape is what *combinators* see.
export struct ListYield<T> : Yield<self, proj T> canbe Mut {
    // The list being walked — borrowed, not owned: an iterator is a position in
    // someone else's data. A `proj` field makes the struct a view of
    // whatever each literal stores there [proj-field]; `iter` above lends
    // its `list`, which the checker reads off its body [proj-infer].
    items: proj List<T>,
    // The index of the next element to emit.
    at: Int
}

// Advances the iterator, reporting the element at its position or the end of the
// list. Out of range is the end: [get] answers `None` past the last index, so
// the bound is read rather than remembered.
export fn next<T>(p: Mut ListYield<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}

// [col-reversed] Walks the list back to front. An **iterator**, not a copy —
// Kotlin's `reversed()` answers a fresh list; this borrows and emits
// projections like [iter] does, so `for x in reversed(xs)` costs nothing per
// element. A caller wanting the reversed *list* writes `to_list(reversed(xs))`.
//
// [iter-fn] An `iter fn`: the declaration is the minter, the `state` block is
// the iterator's own field, and the body is its `next` — the compiler writes
// the struct, whose type is spelled `iter (proj T)` [iter-type]. `list` is
// borrowed by the iterator [proj-field], which is what lets it emit
// projections of it.
export iter fn reversed<T>(list: List<T>) -> Emitted (proj(list) T) | Finished {
    state {
        // The index of the next element to emit, counting down; `-1` is the
        // end, which [get] reports as `None` like any out-of-range index.
        at: Int = size(list) - 1
    }
    let elem = get(list, at)
    if elem is None {
        return finished()
    }
    at = at - 1
    return emitted(elem)
}

// [col-idx] Walks the **indices** of the list, front to back — and every
// emitted `Int` carries the claim: `for i in indices(xs)` makes `get(xs, i)`
// total. Sound because the iterator borrows the list [proj-field], so nothing
// can shrink it while the loop runs [proj-infer].
//
// The `+Idx` is [deduce-reapply]'s establishment in a return-type arm: this
// file declares `Idx`, and the bounds test above the emit is the proof the
// trust rests on. [qual-depend] The claim names the parameter; in the
// generated struct's obligation it names the struct's own borrowed field, and
// a `for` binds it to the *source* list's identity, so the claim reads
// `Idx(xs)` in the loop body.
export iter fn indices<T>(list: List<T>) -> Emitted (+Idx(list) Int) | Finished {
    state {
        // The index the next turn emits.
        at: Int = 0
    }
    if at >= size(list) {
        return finished()
    }
    let index = at.copy()
    at = at + 1
    return emitted(index)
}

// [col-idx] The same indices, back to front: `size(list) - 1` down to `0` —
// **the founding example of the refinement-types design**: the descending
// loop whose body needs no `!`.
export iter fn rev_indices<T>(list: List<T>) -> Emitted (+Idx(list) Int) | Finished {
    state {
        at: Int = size(list) - 1
    }
    if at < 0 {
        return finished()
    }
    let index = at.copy()
    at = at - 1
    return emitted(index)
}

// [col-enumerate] What [enumerate] and [enumerate_rev] emit: an element and
// the index it sits at. A **view struct**, not a tuple: the element part is a
// borrow of the walked list, a tuple literal cannot *store* a projection
// ([fate-derived-readonly] — the store is a move, and the remedy `copy` would
// charge every step), and a `proj` **field** is exactly the declared lend the
// tuple lacks [proj-field].
export struct Enumerated<T> {
    index: Int,
    elem: proj T
}

// [col-enumerate] Walks the list front to back, pairing each element with its
// index: `for pair in enumerate(xs)` sees `0/first`, `1/second`, … — the loop
// that wants positions without writing index arithmetic. The element is
// borrowed, the index is the pair's own — `holds proj(list)` says the emitted
// view borrows the list.
export iter fn enumerate<T>(list: List<T>) -> Emitted Enumerated<T> | Finished holds proj(list) {
    state {
        // The index of the next element to emit.
        at: Int = 0
    }
    let elem = get(list, at)
    if elem is None {
        return finished()
    }
    // Copied, not linked: the emitted index must survive `at`'s step below,
    // the same detach `core.range`'s next does.
    let index = at.copy()
    at = at + 1
    return emitted(Enumerated<T> { index: index, elem: elem })
}

// [col-enumerate] The same pairs, back to front: `enumerate_rev(xs)` sees
// `size-1/last` down to `0/first` — the descending index loop with the
// element already in hand.
export iter fn enumerate_rev<T>(list: List<T>) -> Emitted Enumerated<T> | Finished holds proj(list) {
    state {
        at: Int = size(list) - 1
    }
    let elem = get(list, at)
    if elem is None {
        return finished()
    }
    let index = at.copy()
    at = at - 1
    return emitted(Enumerated<T> { index: index, elem: elem })
}

// The text form of a list, for string interpolation [interp-to-str]:
// `[1, 2, 3]`, elements separated by `, ` and rendered by their own native
// text form. An `intrinsic` rather than Salvo code because rendering the
// *elements* is the backend's own formatting, which is also why the element
// type must render natively — a list of structs needs a `to_str` of your own.
export intrinsic fn to_str<T>(list: List<T>) [] -> Str => list


// [col-sorted-list] The claim that a list's elements are in order. A *state*
// qualifier over `List<T>`, and a different mechanic from the `SortedSet` /
// `SortedMap` **types** [col-sorted]: those are a representation, this is an
// erased claim about a list that is otherwise an ordinary list. One name,
// two mechanics, and that is on purpose — a `Sorted List<T>` still reaches
// the whole list surface, where a `SortedSet` deliberately does not.
//
// It has no `qualifies`, so it cannot be *tested* with `is Sorted`: deciding
// whether a list happens to be sorted needs the elements compared, which
// only a backend can do over an unconstrained `T`. It is established by
// construction instead — `sort`/`mut_sort` — and preserved by `add_sorted`.
// [cmp-carry] The claim **names the ordering it is sorted by**: `Sorted` alone
// says nothing about *which* order the elements are in, and an `add_sorted`
// bound to a different `cmp` than the sort used would insert at a position
// that is a lower bound for one ordering and nonsense for the other. The slot
// makes the two different types, so they refuse to mix.
export qualifier Sorted<T>(?cmp: (T, T) -> Int) of List<T> with NonEmpty {
    // [qual-refn] The claim's owner states what the insert below does to it,
    // because the primitive cannot: a function that mutates may not promise back
    // a qualifier it has never heard of [deduce-syntax], and inserting at the
    // lower bound is exactly the operation that keeps a list ordered. Sound
    // because the position is computed with `cmp` — the ordering the claim
    // names, which is why the claim carries it.
    refn insert_sorted_by<T>(list: Mut List<T>, elem: T, cmp: (T, T) -> Int)
    => list: +Sorted
}

// ===== the ordering-taking primitives =====
//
// [col-sorted-list] The three operations below are ordinary Salvo over these,
// which is what lets them take the ordering as an **implicit** parameter: no
// `intrinsic fn` takes one, since its lowering is a template over rendered
// arguments. So the binding happens in Salvo and each primitive is handed the
// comparator as an ordinary fn-typed argument. Private to this module: what a
// program sees is the four functions below [mod-private].

// Answers the elements of [list] in the order [cmp] puts them in. A stable
// sort on both backends, so equal elements keep their relative order.
intrinsic fn sort_by<T>(list: List<T>, cmp: (T, T) -> Int) [] -> Mut List<T>
=> list, cmp

// Inserts [elem] at the **lower bound** for [cmp] — before any element that
// ties with it — which is the position that keeps the list ordered.
intrinsic fn insert_sorted_by<T>(list: Mut List<T>, elem: T, cmp: (T, T) -> Int) [] -> None
=> list: Mut, !elem, cmp

// The **lowest** index that ties with [elem] under [cmp], or `None`. A tie is
// `cmp(a, b) == 0`, never the host's equality: that is the only test
// consistent with the ordering the `Sorted` claim names [col-membership].
intrinsic fn search_sorted_by<T>(list: List<T>, elem: T, cmp: (T, T) -> Int) [] -> Int?
=> list, elem, cmp

// Returns the elements in order, and **publishes the ordering** it sorted by:
// the result is `Sorted<T>(?cmp)` for whatever `cmp` resolution found here, so
// the two functions below are computed with the ordering the list actually
// carries [cmp-carry]. The comparison is the language's, not the target's: a
// hand-written `cmp@Person` is what sorts a `List<Person>`, and `Str` compares
// by code point on both backends [col-sorted].
export fn sort<T>(list: List<T>, ?Ordered<T>) [] -> +Sorted<T>(?cmp) List<T> => list {
    return sort_by(list, cmp)
}

// Mutable variant, which is how a `Mut Sorted List<T>` is obtained — and so
// how `add_sorted` gets something to insert into. `mut_sort(mut_list_of())`
// is the empty sorted list.
export fn mut_sort<T>(list: List<T>, ?Ordered<T>) [] -> +Sorted<T>(?cmp) Mut List<T>
=> list {
    return sort_by(list, cmp)
}

// [col-sorted-list] Inserts `elem` at the position that keeps the list in
// order, which is what lets the claim survive the mutation. Equal elements
// are kept together and the new one goes *before* them (a lower bound), so
// the resulting list is identical on both backends.
//
// The list must already be sorted: inserting into an unordered list in order
// would not make it ordered, so the parameter demands the claim it returns.
// The ordering is **captured from the claim** [cmp-binder] rather than
// resolved here, so the insert position is computed with the ordering the list
// was sorted by — a list sorted by one `cmp` cannot be added to under another,
// because the two are different types.
//
// The exhaustive deduction names `Sorted` itself: a mutating function must
// state what survives [deduce-syntax], and this is the one function that
// genuinely knows the claim does — so it needs no refinement from the
// qualifier, unlike `add`, which cannot promise `NonEmpty` [qual-refn].
export fn add_sorted<T>(list: Mut Sorted<T>(?cmp) List<T>, elem: T) [] -> None
=> list: Mut Sorted, !elem {
    insert_sorted_by(list, elem, cmp)
}

// [col-sorted-list] The index of `elem`, or `None`. An honest signature only
// because the parameter is `Sorted`: over an unordered list the answer would
// be meaningless rather than merely absent. With equal elements it answers
// the **lowest** matching index, on both backends — and "matching" is a tie
// in the ordering the claim names, `cmp(a, b) == 0` [col-membership].
export fn binary_search<T>(list: Sorted<T>(?cmp) List<T>, elem: T) [] -> (+Idx(list) Int)?
=> list, elem {
    return search_sorted_by(list, elem, cmp)
}
