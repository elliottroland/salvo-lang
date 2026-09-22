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

// Constructor. The elements are stored in the new list, so they are moved: a
// variadic tail is owned, and needs no entry in the clause [deduce-syntax].
export intrinsic fn list_of<T canbe linear>(...elems: T[]) [] -> List<T>

// Mutable constructor
export intrinsic fn mut_list_of<T canbe linear>(...elems: T[]) [] -> Mut List<T>

// [col-by] Builds a list of [size] elements, each from its index:
// `list_by(3, i -> i * 2)` is `[0, 2, 4]`. The generator is called once per
// index, in order.
export intrinsic fn list_by<T>(size: Int, init: (Int) -> T) [] -> List<T> => size, init

// Mutable variant
export intrinsic fn mut_list_by<T>(size: Int, init: (Int) -> T) [] -> Mut List<T>
    => size, init

// Possibly gets the element at the given index if the list is long enough
export intrinsic fn get<T>(list: List<T>, index: Int) [] -> (proj[from: list] T)? => list, index

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

// [linear-container] There is deliberately **no** positional write for a list
// of obligations. `replace(list, index, elem) -> T?` looks like the map's, but
// a list index can be *out of range*, and then the value written has nowhere
// to go: answering `None` would drop it (a silent leak, exactly what this
// surface exists to prevent), handing it back would make "displaced" and
// "bounced" indistinguishable, and refusing at run time is not how the rest of
// std treats an index [col-bounds]. Take the element out and add a new one, or
// key the collection with a `Map`, whose `replace` has no such hole.

// [linear-container] The **terminal**: consumes the list and hands every
// element to [each], in order. This is how a list of obligations ends — the
// container is spent, and each element's obligation continues into the
// callback, which consumes it (`=>[each] !x`).
//
// It is a function rather than a `for`-shaped pass because there is then no
// half-drained state to account for: a drain either happened or did not, and
// no path can drop the elements it did not reach. The callback's own effects
// travel to the caller [fn-effects], so draining into an effectful discharger
// needs no annotation here.
export intrinsic fn drain<T canbe linear>(list: List<T>, each: (x: T) -> None) [] -> None
    =>[each] !x => !list, each

export intrinsic fn first<T>(list: List<T>) [] -> proj[from: list] T? => list

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
}

// Requiring a first element establishes the claim **by construction**, so no
// `qualifies` call is emitted [qual-ctor-predicate].
export fn non_empty_list<T>(first: T, ...rest: T[]) [] -> List<T> as NonEmpty {
    return list_of(first, ...rest)
}

// [col-nonempty] The dividend: the same accessor, without the optional.
// Ranked above the plain `first` because it demands more of its argument
// [fn-overload-rank].
//
// It delegates to `get`, not to `first`: `first@core.list(list)` would pick
// *this* overload again — the scope selector names the module, and within it
// a `NonEmpty` argument still ranks this one first — and recurse forever.
export fn first<T>(list: NonEmpty List<T>) [] -> proj[from: list] T => list {
    return get(list, 0)!
}

// Returns the number of elements in the list
export intrinsic fn size<T canbe linear>(list: List<T>) [] -> Int => list

// [iter-pass] A fresh pass over the list, which is what `for x in iter(xs)`
// and every combinator walks. The pass **borrows** the list — it is a view
// with a position [proj-field] — so iterating a list by hand keeps it
// usable, and nothing is copied or consumed on the way [copy-opt-in].
export fn iter<T>(list: List<T>) [] -> Mut ListYield<T> => list {
    return Mut ListYield<T> { items: list, at: 0 }
}

// [iter-protocol] The pass a list is walked by: the list plus a position in
// it. An ordinary struct with an ordinary `next` — there is no special
// container protocol, which is what "a pass is a user struct" means. The
// backends keep their native loop as a fast path for a `for` over a list
// [iter-for-native], so this shape is what *combinators* see.
export struct ListYield<T> : Yield<self, proj T> canbe Mut {
    // The list being walked — borrowed, not owned: a pass is a position in
    // someone else's data. A `proj` field makes the struct a view of
    // whatever each literal stores there [proj-field]; `iter` above lends
    // its `list`, which the checker reads off its body [proj-infer].
    items: proj List<T>,
    // The index of the next element to emit.
    at: Int
}

// Advances the pass, reporting the element at its position or the end of the
// list. Out of range is the end: [get] answers `None` past the last index, so
// the bound is read rather than remembered.
export fn next<T>(p: Mut ListYield<T>) [] -> Emitted (proj[from: p] T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
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
export qualifier Sorted<T, ?cmp: (T, T) -> Int> of List<T> with NonEmpty

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

// [qual-refn] The claim's owner states what that insert does to it, because the
// primitive cannot: a function that mutates may not promise back a qualifier it
// has never heard of [deduce-syntax], and inserting at the lower bound is
// exactly the operation that keeps a list ordered. Sound because the position is
// computed with `cmp` — the ordering the claim names, which is why the claim
// carries it. Module-scoped [qual-refn-scope], and `Sorted` has no `qualifies`
// to put it beside, so it is written here rather than in the qualifier's body.
refn insert_sorted_by<T>(list: Mut List<T>, elem: T, cmp: (T, T) -> Int)
    => list: +Sorted

// The **lowest** index that ties with [elem] under [cmp], or `None`. A tie is
// `cmp(a, b) == 0`, never the host's equality: that is the only test
// consistent with the ordering the `Sorted` claim names [col-membership].
intrinsic fn search_sorted_by<T>(list: List<T>, elem: T, cmp: (T, T) -> Int) [] -> Int?
    => list, elem, cmp

// Returns the elements in order, and **publishes the ordering** it sorted by:
// the result is `Sorted<T, ?cmp>` for whatever `cmp` resolution found here, so
// the two functions below are computed with the ordering the list actually
// carries [cmp-carry]. The comparison is the language's, not the target's: a
// hand-written `cmp@Person` is what sorts a `List<Person>`, and `Str` compares
// by code point on both backends [col-sorted].
export fn sort<T>(list: List<T>, ?Ordered<T>) [] -> List<T> as Sorted<T, ?cmp> => list {
    return sort_by(list, cmp)
}

// Mutable variant, which is how a `Mut Sorted List<T>` is obtained — and so
// how `add_sorted` gets something to insert into. `mut_sort(mut_list_of())`
// is the empty sorted list.
export fn mut_sort<T>(list: List<T>, ?Ordered<T>) [] -> Mut List<T> as Sorted<T, ?cmp>
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
export fn add_sorted<T>(list: Mut Sorted<T, ?cmp> List<T>, elem: T) [] -> None
    => list: Mut Sorted, !elem {
    insert_sorted_by(list, elem, cmp)
}

// [col-sorted-list] The index of `elem`, or `None`. An honest signature only
// because the parameter is `Sorted`: over an unordered list the answer would
// be meaningless rather than merely absent. With equal elements it answers
// the **lowest** matching index, on both backends — and "matching" is a tie
// in the ordering the claim names, `cmp(a, b) == 0` [col-membership].
export fn binary_search<T>(list: Sorted<T, ?cmp> List<T>, elem: T) [] -> Int?
    => list, elem {
    return search_sorted_by(list, elem, cmp)
}
