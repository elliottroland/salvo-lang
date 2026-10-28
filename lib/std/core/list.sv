// `canbe Mut` opts List into the language-level Mut auto-qualifier:
// a backend may map `Mut List<T>` to a different native
// type (Kotlin's `MutableList<T>`) or to the same one (Rust's `Vec<T>`,
// where mutability lives in the binding).
//
// `<T canbe linear>` is what lets a list **hold
// obligations**: `List<Reply<Int>>` is a linear type — it owes, and its
// terminal is [drain] — while `List<Int>` is an ordinary list. The
// element's declaration is the source of the linearity; a list never
// spells it.
// A value platform type (ROADMAP 0.7): the host
// implements it in `lib/platform/std/core/list.{rs,kt}` and names the mutable
// kind `MutList`. The constructors stay intrinsic, since their spread
// lowering is a template over rendered arguments (`vec![…]`, `listOf<T>(…)`)
// a Salvo body cannot express for a `canbe linear` element type.
// @compilerrules [type-canbe-mut, linear-container, platform-value-type]
export iterable platform type List<T canbe linear> canbe Mut : Iter<self, ListNext<T>, T>

// Constructors, in two shapes (user decision 2026-09-23).
// Writing a *first* element establishes `NonEmpty` **by construction**, so no
// `qualifies` call is emitted and the claim costs nothing:
//
//     List()             // List<T>, empty
//     List(1, 2, 3)      // NonEmpty List<T> — the claim comes free
//     List(a, ...rest)   // NonEmpty List<T> — one written element is enough
//
// A **lone** `...spread` reaches neither: a spread cannot supply a required
// parameter (its length is not known here, so `NonEmpty` would be
// a guess) and the empty shape takes no arguments. Building a list out of an
// array is `map_to(Mut List(), iter(array), x -> x)` or a loop with `add`.
//
// The constructor **is** the type-named function: `List(...)`
// resolves to this declaration, whose canonical key is `List` (and `Mut_List`
// for the mutable pair), carried verbatim to the backend lowering. There is no
// separate lowercase constructor — the type name is the name.
//
// The elements are stored in the new list, so they are moved: a variadic tail is
// owned, and needs no entry in the clause.
// @compilerrules [col-of-nonempty, qual-ctor-predicate, fn-variadic, fn-type-named, deduce-syntax]
export intrinsic fn List<T canbe linear>() [] -> List<T>
export intrinsic fn List<T canbe linear>(first: T, ...rest: T[]) [] -> +NonEmpty List<T>
=> !first

// The same pair, mutable. A `Mut NonEmpty` list is a claim the next mutation
// may take away — which is exactly what a state qualifier is —
// so `remove_at` strips it and `add` puts it back.
// @compilerrules [qual-subject, qual-refn]
export intrinsic fn Mut List<T canbe linear>() [] -> Mut List<T>
export intrinsic fn Mut List<T canbe linear>(first: T, ...rest: T[]) [] -> +NonEmpty Mut List<T>
=> !first

// Builds a list of [size] elements, each from its index:
// `list_by(3, i -> i * 2)` is `[0, 2, 4]`. The generator is called once per
// index, in order.
// @compilerrules [col-by]
export fn list_by<T>(size: Int, init: (Int) -> T) [] -> List<T> => size, init {
    return mut_list_by(size, init)
}

// Mutable variant
export fn mut_list_by<T>(size: Int, init: (Int) -> T) [] -> Mut List<T>
=> size, init {
    let out = Mut List<T>()
    let i = 0
    while i < size {
        add(out, init(copy(i)))
        i = i + 1
    }
    return out
}

// Possibly gets the element at the given index if the list is long enough
export platform fn get<T canbe linear>(list: List<T>, index: Int) [] -> (proj(list) T)? => list, index

// [col-idx] `Idx`, the claim about an index, is `std.core.index`'s:
// one `Idx` serves every container with a `size`.

// The **total** read: an index carrying the claim answers the
// element itself — no `None` arm, nothing to `!`. Ranked above the optional
// [get] by its qualifier, exactly as `first` over a
// `NonEmpty` list is.
// @compilerrules [col-idx, fn-overload-rank, col-of-nonempty]
export fn get<T canbe linear>(list: List<T>, index: Idx(list) Int) [] -> proj(list) T
=> list, index {
    // The host's read answers the element itself: going through the optional
    // [get] and `!` would read a stored `None` of a `List<T?>` as absence on a
    // backend that flattens `T??` (the JVM), and trap.
    return get_at(list, index + 0)
}

// The host's read; the proven index is in range.
platform fn get_at<T canbe linear>(list: List<T>, index: Int) [] -> proj(list) T => list, index

// The **total** exchange: two proven indices cannot be out of
// range, so there is no `Bool` to check — the claim did the checking.
// Its own clause promises what its body's refined [swap]
// keeps: existing `Idx` claims survive.
// @compilerrules [col-idx, col-bounds, qual-preserve]
export fn swap<T>(list: Mut List<T>, i: Idx(list) Int, j: Idx(list) Int) [] -> None
=> list: Mut, list: preserve Idx, i, j {
    // The claims make failure impossible here, which is what lets the total
    // overload discharge the obligation without reading it.
    ignore(swap(list, i + 0, j + 0))
    return None
}

// The **total positional write**: puts [value] at [index] and
// answers the element it displaced. The index is proven (`Idx(list) Int`), so
// there is no out-of-range case to answer, and the displaced element is handed
// back — which is what makes it safe for a list of obligations:
// nothing written is dropped. Taking an element out of a
// `List<T?>` is `replace(xs, i, None)`. A write moves no boundary, so
// existing `Idx` claims survive it.
// The host writes it, and `std.core.index`'s `Idx` says the claims survive.
// @compilerrules [col-replace, linear-container, qual-preserve]
export platform fn replace<T canbe linear>(list: Mut List<T>, index: Idx(list) Int, value: T) [] -> T
=> list: Mut, index, !value

// What a **position-based** algorithm needs, as a fns group:
// one function turning a container and a position into the
// element's mutable handle. The caller — which knows the concrete shape —
// fills it, so the algorithm itself stays generic over *what a position is*:
// an `Idx` for a list, a key for a map, a cursor for a structure of your own.
// The `Next` pattern, for places rather than elements.
//
// A locator is what the Rust backend renders this as: position data
// crossing the closure boundary, materialized at the use site — which is why
// a generic algorithm may hand out mutable handles at all. `at`
// mints a `ref` handle into its container; `get` (read-only `proj`) reads.
// @compilerrules [col-locate, implicit-group, rs-loc, ref-handle]
export fns Ref<C, L, T> {
    fn at(c: C, l: L) -> ref(c) Mut T?
}

// The canonical `at` for a list: the mint of a
// element handle, `ref(list) T` (writable when `T` is `Mut X`; the handle's
// permission follows the element's kind) — the way `cmp`/`eq` have
// canonical implementations for the intrinsic types. A position
// for a list is its index. `get` is the read-only sibling (`proj`); `at` is
// the handle. (On Rust a `ref` result renders as the backend's
// position: the index, bounds-checked.) A handle is never recovered
// from a `get`'s borrow: positions are preserved, not rediscovered, so the
// mint is a host primitive beside the read.
// @compilerrules [col-locate, ref-handle, cmp-groups, rs-path]
export platform fn at<T canbe linear>(list: List<T>, index: Int) [] -> ref(list) T?
=> list, index

// The **total** mint: an index carrying the claim
// answers the handle itself — no `None` arm, nothing to `!`. Ranked above the
// optional [at] by its qualifier, as the total [get] is.
// @compilerrules [col-idx, ref-handle, fn-overload-rank]
export fn at<T canbe linear>(list: List<T>, index: Idx(list) Int) [] -> ref(list) T
=> list, index {
    return at_proven(list, index + 0)
}

// The host's mint; the proven index is in range.
platform fn at_proven<T canbe linear>(list: List<T>, index: Int) [] -> ref(list) T
=> list, index

// Applies [f] to the element at [index], **in place**: the
// callback receives the mutable element handle, so nothing is
// copied, moved out, or put back. An in-place write moves no boundary, so
// existing `Idx` claims survive the call.
// @compilerrules [col-update, ref-handle, qual-preserve]
export fn update<T>(list: List<Mut T>, index: Idx(list) Int, f: (elem: Mut T) -> None) [] -> None
=> list: preserve Idx, index, f {
    f(at(list, index))
    return None
}

// Adds an element to the list. The list takes ownership of `elem`, so it
// is moved; the list itself is mutated, which is why its surviving
// qualifiers are listed exhaustively.
// @compilerrules [deduce-syntax]
export platform fn add<T canbe linear>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem

// Takes the **first** element out of the list, or answers
// `None` when it is empty: the way an obligation leaves a list one at a time.
// The element is *moved* out — nothing is left behind and nothing is
// duplicated, which is what makes it legal for a `List<Reply<T>>` where [get]
// (an alias) is not.
//
// The `T?` shape is the whole absence story: the `None` arm owes nothing, so
// the emptiness check *is* the union narrow. Pairs with
// `while` for a take-until-empty loop, and with [drain] for the terminal.
// @compilerrules [linear-container, linear-union-arm]
export platform fn remove_first<T canbe linear>(list: Mut List<T>) [] -> T? => list: Mut

// The same, at an index: the element at [index] is moved
// out and the elements after it shift down. `None` when the index is past the
// end.
// @compilerrules [linear-container]
export platform fn remove_at<T canbe linear>(list: Mut List<T>, index: Int) [] -> T?
=> list: Mut, index

// Puts [elem] at [index], shifting the elements from there on
// up by one; [index] may be the size, which appends. Out of range (below 0 or
// past the size), nothing moves and [elem] is **handed back**, so a list of
// obligations cannot lose one.
// @compilerrules [col-insert, linear-container]
export platform fn insert_at<T canbe linear>(list: Mut List<T>, index: Int, elem: T) [] -> T?
=> list: Mut, index, !elem

// Takes the elements at indices [from] up to (not
// including) [to] out of the list, in order, and answers them. The range is
// clamped to the list, so an out-of-range bound removes what there is: the
// primitive the `remove_front`/`remove_back` family is written over.
// @compilerrules [col-remove-range]
export platform fn remove_range<T canbe linear>(list: Mut List<T>, from: Int, to: Int) [] -> Mut List<T>
=> list: Mut, from, to

// Exchanges the elements at [i] and [j]. **Total**: no value
// enters the list and none leaves it, which is what makes it the one positional
// write a list of obligations can have — nothing can be dropped by it.
//
// Answers **`false`** when either index is out of range, and then
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
// @compilerrules [linear-container, col-bounds]
export fn swap<T canbe linear>(list: Mut List<T>, i: Int, j: Int) [] -> Checked<Bool>
=> list: Mut, i, j {
    return checked(swap_at(list, i, j))
}

// The host's exchange, answering whether both indices were in range.
platform fn swap_at<T canbe linear>(list: Mut List<T>, i: Int, j: Int) [] -> Bool => list: Mut, i, j

// [linear-container] There is deliberately **no** positional write for a list
// of obligations. `replace(list, index, elem) -> T?` looks like the map's, but
// a list index can be *out of range*, and then the value written has nowhere
// to go: answering `None` would drop it (a silent leak, exactly what this
// surface exists to prevent), handing it back would make "displaced" and
// "bounced" indistinguishable, and refusing at run time is not how the rest of
// std treats an index [col-bounds]. Take the element out and add a new one, or
// key the collection with a `Map`, whose `replace` has no such hole. [swap] is
// the exchange that escapes the whole question by moving nothing in or out.

// The **terminal**: consumes the list and hands every
// element to [each], in order. This is how a list of obligations ends — the
// container is spent, and each element's obligation continues into the
// callback, which consumes it (`=>[each] !x`).
//
// It is a function rather than a `for`-shaped iterator because there is then no
// half-drained state to account for: a drain either happened or did not, and
// no path can drop the elements it did not reach. The callback's own effects
// travel to the caller, so draining into an effectful discharger
// needs no annotation here.
// @compilerrules [linear-container, fn-effects]
export fn drain<T canbe linear>(list: List<T>, each: (x: T) -> None) [] -> None
=>[each] !x => !list, each {
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

// Takes the **last** element out, or `None` when empty.
// @compilerrules [linear-container]
export platform fn remove_last<T canbe linear>(list: Mut List<T>) [] -> T? => list: Mut

export platform fn first<T canbe linear>(list: List<T>) [] -> proj(list) T? => list

// The claim that a list has at least one element, and the
// reason the qualifier machinery is worth having over a container: with it,
// `first` answers with an element instead of an optional.
//
// Only one qualifier of a given *name* is visible across the implicitly
// visible `std.core` modules, so `NonEmpty` is a claim about a **list**
// specifically; `Set`/`Map` and the sorted pair have no equivalent yet (see
// ROADMAP.md).
// @compilerrules [col-nonempty]
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

// The dividend: the same accessor, without the optional.
// Ranked above the plain `first` because it demands more of its argument.
//
// It delegates to `get`, not to `first`: `first@core.list(list)` would pick
// *this* overload again — the scope selector names the module, and within it
// a `NonEmpty` argument still ranks this one first — and recurse forever.
// @compilerrules [col-nonempty, fn-overload-rank]
export fn first<T canbe linear>(list: NonEmpty List<T>) [] -> proj(list) T => list {
    return get(list, 0) ?: unreachable!()
}

// Returns the number of elements in the list
export platform fn size<T canbe linear>(list: List<T>) [] -> Int => list

// ===== the list surface written in Salvo [col-salvo] =====
//
// Everything below is ordinary Salvo over the intrinsics above (`get`, `size`,
// `add`, `insert_at`, `remove_range`), so a new backend implements none of it.

// The last element, or `None` for an empty list.
// @compilerrules [col-salvo]
export fn last<T canbe linear>(list: List<T>) [] -> proj(list) T? => list {
    return get(list, size(list) - 1)
}

// Whether the list has no elements.
// @compilerrules [col-salvo]
export fn is_empty<T canbe linear>(list: List<T>) [] -> Bool => list {
    return size(list) == 0
}

// Takes the first [n] elements out of the list and answers them, in
// order — all of them when there are fewer.
// @compilerrules [col-salvo]
export fn remove_front<T canbe linear>(list: Mut List<T>, n: Int) [] -> Mut List<T> => list: Mut, n {
    return remove_range(list, 0, n)
}

// Takes the last [n] elements out of the list and answers them, in
// order — all of them when there are fewer.
// @compilerrules [col-salvo]
export fn remove_back<T canbe linear>(list: Mut List<T>, n: Int) [] -> Mut List<T> => list: Mut, n {
    let at = size(list) - n
    if at < 0 {
        at = 0
    }
    return remove_range(list, at, size(list))
}

// Takes elements off the front while [keep] accepts them, and
// answers them in order: the list is left starting at the first one it
// refused.
// @compilerrules [col-salvo]
export fn remove_front_while<T canbe linear>(list: Mut List<T>, keep: (x: T) -> Bool) [] -> Mut List<T>
=> list: Mut, keep {
    let n = 0
    while n < size(list) && keep(get(list, n) ?: unreachable!()) {
        n = n + 1
    }
    return remove_range(list, 0, n)
}

// Takes elements off the back while [keep] accepts them, and
// answers them in list order: the list is left ending at the last one it
// refused.
// @compilerrules [col-salvo]
export fn remove_back_while<T canbe linear>(list: Mut List<T>, keep: (x: T) -> Bool) [] -> Mut List<T>
=> list: Mut, keep {
    let at = size(list)
    while at > 0 && keep(get(list, at - 1) ?: unreachable!()) {
        at = at - 1
    }
    return remove_range(list, at, size(list))
}

// A copy of the elements at [from] up to (not including) [to],
// clamped to the list. A copy rather than a view: a list of borrows cannot be
// answered from a generic fn on the Rust backend yet, and a slice type, which
// is what a view would really be, does not exist.
// @compilerrules [col-salvo]
export fn sub_list<T>(list: List<T>, from: Int, to: Int, ?copy: (v: T) -> T) [] -> Mut List<T>
=> list, from, to {
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

// The index of the first element [pick] accepts, or `None`.
// @compilerrules [col-salvo]
export fn find_first<T>(list: List<T>, pick: (x: T) -> Bool) [] -> (Idx(list) Int)? => list, pick {
    for i in indices(list) {
        if pick(get(list, i)) {
            return i
        }
    }
    return None
}

// The index of the last element [pick] accepts, or `None`.
// @compilerrules [col-salvo]
export fn find_last<T>(list: List<T>, pick: (x: T) -> Bool) [] -> (Idx(list) Int)? => list, pick {
    for i in rev_indices(list) {
        if pick(get(list, i)) {
            return i
        }
    }
    return None
}

// The index of the first element equal to [elem], or `None`.
// @compilerrules [col-salvo]
export fn index_of<T>(list: List<T>, elem: T, ?Eq<T>) [] -> (Idx(list) Int)? => list, elem {
    for i in indices(list) {
        if eq(get(list, i), elem) {
            return i
        }
    }
    return None
}

// The index of the last element equal to [elem], or `None`.
// @compilerrules [col-salvo]
export fn last_index_of<T>(list: List<T>, elem: T, ?Eq<T>) [] -> (Idx(list) Int)? => list, elem {
    for i in rev_indices(list) {
        if eq(get(list, i), elem) {
            return i
        }
    }
    return None
}

// Whether some element equals [elem].
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

// Whether [pick] accepts some element.
// @compilerrules [col-salvo]
export fn any<T>(list: List<T>, pick: (x: T) -> Bool) [] -> Bool => list, pick {
    return !(find_first(list, pick) is None)
}

// Whether [pick] accepts every element (true for an empty list).
// @compilerrules [col-salvo]
export fn all<T>(list: List<T>, pick: (x: T) -> Bool) [] -> Bool => list, pick {
    let i = 0
    while i < size(list) {
        if !pick(get(list, i) ?: unreachable!()) {
            return false
        }
        i = i + 1
    }
    return true
}

// How many elements [pick] accepts.
// @compilerrules [col-salvo]
export fn count<T>(list: List<T>, pick: (x: T) -> Bool) [] -> Int => list, pick {
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

// Splits the list in two by [pick]: the elements it accepts, then
// the rest, each in order. Copies the elements, as [filter_to] does, since a
// tuple cannot hold borrows.
// @compilerrules [col-salvo]
export fn partition<T>(list: List<T>, pick: (x: T) -> Bool, ?copy: (v: T) -> T) [] -> (Mut List<T>, Mut List<T>)
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

// A fresh iterator over the list, which is what `for x in iter(xs)`
// and every combinator walks. The iterator **borrows** the list — it is a view
// with a position — so iterating a list by hand keeps it
// usable, and nothing is copied or consumed on the way.
// @compilerrules [iter-mint, proj-field, copy-opt-in]
export fn iter<T>(list: List<T>) [] -> Mut ListNext<T> => list {
    return Mut ListNext<T> { items: list, at: 0 }
}

// The pass a list is walked by: the list plus a position in
// it. An ordinary struct with an ordinary `next` — there is no special
// container protocol, which is what "an iterator is a user struct" means. The
// backends keep their native loop as a fast path for a `for` over a list,
// so this shape is what *combinators* see.
// @compilerrules [iter-protocol, iter-for-native]
export struct ListNext<T> : Next<self, proj T> canbe Mut {
    // The list being walked — borrowed, not owned: an iterator is a position in
    // someone else's data. A `proj` field makes the struct a view of
    // whatever each literal stores there; `iter` above lends
    // its `list`, which the checker reads off its body.
    // @compilerrules [proj-field, proj-infer]
    items: proj List<T>,
    // The index of the next element to emit.
    at: Int
}

// Advances the iterator, reporting the element at its position or the end of the
// list. Out of range is the end: [get] answers `None` past the last index, so
// the bound is read rather than remembered.
export fn next<T>(p: Mut ListNext<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}

// Walks the list back to front. An **iterator**, not a copy —
// Kotlin's `reversed()` answers a fresh list; this borrows and emits
// projections like [iter] does, so `for x in reversed(xs)` costs nothing per
// element. A caller wanting the reversed *list* writes `to_list(reversed(xs))`.
//
// An `iter fn`: the declaration is the minter, the `state` block
// names the iterator struct and declares its own field, and the body is its
// `next` — the compiler writes the struct. `list` is borrowed by the iterator,
// which is what lets it emit projections of it.
// @compilerrules [col-reversed, iter-fn, proj-field]
export iter fn reversed<T>(list: List<T>) -> Emitted (proj(list) T) | Finished {
    state ListReversed {
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

// [col-idx] `indices` and `rev_indices`, whose indices carry `Idx`, are
// `std.core.index`'s: establishing the claim is trusted only where it is declared.

// What [enumerate] and [enumerate_rev] emit: an element and
// the index it sits at. A **view struct**, not a tuple: the element part is a
// borrow of the walked list, a tuple literal cannot *store* a projection
// (— the store is a move, and the remedy `copy` would
// charge every step), and a `proj` **field** is exactly the declared lend the
// tuple lacks.
// @compilerrules [col-enumerate, fate-derived-readonly, proj-field]
export struct Enumerated<T> {
    index: Int,
    elem: proj T
}

// Walks the list front to back, pairing each element with its
// index: `for pair in enumerate(xs)` sees `0/first`, `1/second`, … — the loop
// that wants positions without writing index arithmetic. The element is
// borrowed, the index is the pair's own — `holds proj(list)` says the emitted
// view borrows the list.
// @compilerrules [col-enumerate]
export iter fn enumerate<T>(list: List<T>) -> Emitted Enumerated<T> | Finished holds proj(list) {
    state ListEnumerate {
        // The index of the next element to emit.
        at: Int = 0
    }
    let elem = get(list, at)
    if elem is None {
        return finished()
    }
    // Copied, not linked: the emitted index must survive `at`'s step below,
    // the same detach `std.core.range`'s next does.
    let index = at.copy()
    at = at + 1
    return emitted(Enumerated<T> { index: index, elem: elem })
}

// The same pairs, back to front: `enumerate_rev(xs)` sees
// `size-1/last` down to `0/first` — the descending index loop with the
// element already in hand.
// @compilerrules [col-enumerate]
export iter fn enumerate_rev<T>(list: List<T>) -> Emitted Enumerated<T> | Finished holds proj(list) {
    state ListEnumerateRev {
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

// The text form of a list, for string interpolation:
// `[1, 2, 3]`, elements separated by `, ` and rendered by their own native
// text form. An `intrinsic` rather than Salvo code because rendering the
// *elements* is the backend's own formatting, which is also why the element
// type must render natively — a list of structs needs a `to_str` of your own.
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


// The claim that a list's elements are in order. A *state*
// qualifier over `List<T>`, and a different mechanic from the `SortedSet` /
// `SortedMap` **types**: those are a representation, this is an
// erased claim about a list that is otherwise an ordinary list. One name,
// two mechanics, and that is on purpose — a `Sorted List<T>` still reaches
// the whole list surface, where a `SortedSet` deliberately does not.
//
// It has no `qualifies`, so it cannot be *tested* with `is Sorted`: deciding
// whether a list happens to be sorted needs the elements compared, which
// only a backend can do over an unconstrained `T`. It is established by
// construction instead — `sort`/`mut_sort` — and preserved by `add_sorted`.
// The claim **names the ordering it is sorted by**: `Sorted` alone
// says nothing about *which* order the elements are in, and an `add_sorted`
// bound to a different `cmp` than the sort used would insert at a position
// that is a lower bound for one ordering and nonsense for the other. The slot
// makes the two different types, so they refuse to mix.
// @compilerrules [col-sorted-list, col-sorted, cmp-carry]
export qualifier Sorted<T>(?cmp: (T, T) -> Int) of List<T> {
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

// Returns the elements in order, and **publishes the ordering** it sorted by:
// the result is `Sorted<T>(?cmp)` for whatever `cmp` resolution found here, so
// the two functions below are computed with the ordering the list actually
// carries. The comparison is the language's, not the target's: a
// hand-written `cmp@Person` is what sorts a `List<Person>`, and `Str` compares
// by code point on both backends.
// @compilerrules [cmp-carry, col-sorted]
export fn sort<T>(list: List<T>, ?Ordered<T>) [] -> +Sorted<T>(?cmp) List<T> => list {
    return sort_by(list, cmp)
}

// Mutable variant, which is how a `Mut Sorted List<T>` is obtained — and so
// how `add_sorted` gets something to insert into. `mut_sort(Mut List())`
// is the empty sorted list.
export fn mut_sort<T>(list: List<T>, ?Ordered<T>) [] -> +Sorted<T>(?cmp) Mut List<T>
=> list {
    return sort_by(list, cmp)
}

// Inserts `elem` at the position that keeps the list in
// order, which is what lets the claim survive the mutation. Equal elements
// are kept together and the new one goes *before* them (a lower bound), so
// the resulting list is identical on both backends.
//
// The list must already be sorted: inserting into an unordered list in order
// would not make it ordered, so the parameter demands the claim it returns.
// The ordering is **captured from the claim** rather than
// resolved here, so the insert position is computed with the ordering the list
// was sorted by — a list sorted by one `cmp` cannot be added to under another,
// because the two are different types.
//
// The exhaustive deduction names `Sorted` itself: a mutating function must
// state what survives, and this is the one function that
// genuinely knows the claim does — so it needs no refinement from the
// qualifier, unlike `add`, which cannot promise `NonEmpty`.
// @compilerrules [col-sorted-list, cmp-binder, deduce-syntax, qual-refn]
export fn add_sorted<T>(list: Mut Sorted<T>(?cmp) List<T>, elem: T) [] -> None
=> list: Mut Sorted, !elem {
    insert_sorted_by(list, elem, cmp)
}

// The index of `elem`, or `None`. An honest signature only
// because the parameter is `Sorted`: over an unordered list the answer would
// be meaningless rather than merely absent. With equal elements it answers
// the **lowest** matching index, on both backends — and "matching" is a tie
// in the ordering the claim names, `cmp(a, b) == 0`.
// @compilerrules [col-sorted-list, col-membership]
export fn binary_search<T>(list: Sorted<T>(?cmp) List<T>, elem: T) [] -> (Idx(list) Int)?
=> list, elem {
    // The host's search answers an index in range or nothing; the test is
    // what proves it here, since `Idx` is established only in `std.core.index`.
    let found = search_sorted_by(list, elem, cmp) ?: return None
    if found is Idx(list) {
        return found
    }
    return None
}

// What `==` and `<`
// mean on a list, declared beside the type (an operator on a type is declared
// in the type's own file, user decision O1-D, 2026-10-27): element by
// element, **through the elements' own identities** — a `List<Person>`
// compares by `Person`'s declared `eq` and orders by its `cmp`. Ordinary
// Salvo over the implicits of the element types (ROADMAP §0j step 8);
// resolving one of these for `List<Person>` fills the element's in turn. (The
// list's `hash`, not an operator, is in `std.core.compare`.)
// @compilerrules [op-unique-shape, col-hashed-ordered, implicit-recursive]
export fn eq<T>(a: List<T>, b: List<T>, ?eq: (x: T, y: T) -> Bool) [] -> Bool => a, b {
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
