// `canbe Mut` opts List into the language-level Mut auto-qualifier
// [type-canbe-mut]: a backend may map `Mut List<T>` to a different native
// type (Kotlin's `MutableList<T>`) or to the same one (Rust's `Vec<T>`,
// where mutability lives in the binding).
intrinsic type List<T> canbe Mut

// Constructor. The elements are stored in the new list, so they are moved: a
// variadic tail is owned, and needs no entry in the clause [deduce-syntax].
intrinsic fn list_of<T canbe linear>(...elems: T[]) [] -> List<T>

// Mutable constructor
intrinsic fn mut_list_of<T canbe linear>(...elems: T[]) [] -> Mut List<T>

// [col-by] Builds a list of [size] elements, each from its index:
// `list_by(3, i -> i * 2)` is `[0, 2, 4]`. The generator is called once per
// index, in order.
intrinsic fn list_by<T>(size: Int, init: (Int) -> T) [] -> List<T> => size, init

// Mutable variant
intrinsic fn mut_list_by<T>(size: Int, init: (Int) -> T) [] -> Mut List<T>
    => size, init

// Possibly gets the element at the given index if the list is long enough
intrinsic fn get<T>(list: List<T>, index: Int) [] -> (proj[from: list] T)? => list, index

// Adds an element to the list. The list takes ownership of `elem`, so it
// is moved; the list itself is mutated, which is why its surviving
// qualifiers are listed exhaustively [deduce-syntax].
intrinsic fn add<T canbe linear>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem

intrinsic fn first<T>(list: List<T>) [] -> proj[from: list] T? => list

// [col-nonempty] The claim that a list has at least one element, and the
// reason the qualifier machinery is worth having over a container: with it,
// `first` answers with an element instead of an optional.
//
// Only one qualifier of a given *name* is visible across the implicitly
// visible `core` modules, so `NonEmpty` is a claim about a **list**
// specifically; `Set`/`Map` and the sorted pair have no equivalent yet (see
// ROADMAP.md).
qualifier NonEmpty<T> of List<T> {
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
fn non_empty_list<T>(first: T, ...rest: T[]) [] -> List<T> as NonEmpty {
    return list_of(first, ...rest)
}

// [col-nonempty] The dividend: the same accessor, without the optional.
// Ranked above the plain `first` because it demands more of its argument
// [fn-overload-rank].
//
// It delegates to `get`, not to `first`: `first@core.list(list)` would pick
// *this* overload again — the scope selector names the module, and within it
// a `NonEmpty` argument still ranks this one first — and recurse forever.
fn first<T>(list: NonEmpty List<T>) [] -> proj[from: list] T => list {
    return get(list, 0)!
}

// Returns the number of elements in the list
intrinsic fn size<T canbe linear>(list: List<T>) [] -> Int => list

// [iter-pass] A fresh pass over the list, which is what `for x in iter(xs)`
// and every combinator walks. The pass **borrows** the list — it is a view
// with a position [proj-field] — so iterating a list by hand keeps it
// usable, and nothing is copied or consumed on the way [copy-opt-in].
fn iter<T>(list: List<T>) [] -> Mut ListYield<T> => list {
    return Mut ListYield<T> { items: list, at: 0 }
}

// [iter-protocol] The pass a list is walked by: the list plus a position in
// it. An ordinary struct with an ordinary `next` — there is no special
// container protocol, which is what "a pass is a user struct" means. The
// backends keep their native loop as a fast path for a `for` over a list
// [iter-for-native], so this shape is what *combinators* see.
struct ListYield<T> : Yield<self, proj T> canbe Mut {
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
fn next<T>(p: Mut ListYield<T>) [] -> Emitted (proj[from: p] T) | Finished => p: Mut {
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
intrinsic fn to_str<T>(list: List<T>) [] -> Str => list


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
qualifier Sorted<T> of List<T> with NonEmpty

// Returns the elements in order. The comparison is the language's, not the
// target's: `Str` compares by code point on both backends [col-sorted].
intrinsic fn sort<T>(list: List<T>) [] -> List<T> as Sorted => list

// Mutable variant, which is how a `Mut Sorted List<T>` is obtained — and so
// how `add_sorted` gets something to insert into. `mut_sort(mut_list_of())`
// is the empty sorted list.
intrinsic fn mut_sort<T>(list: List<T>) [] -> Mut List<T> as Sorted => list

// [col-sorted-list] Inserts `elem` at the position that keeps the list in
// order, which is what lets the claim survive the mutation. Equal elements
// are kept together and the new one goes *before* them (a lower bound), so
// the resulting list is identical on both backends.
//
// The list must already be sorted: inserting into an unordered list in order
// would not make it ordered, so the parameter demands the claim it returns.
// The exhaustive deduction names `Sorted` itself: a mutating function must
// state what survives [deduce-syntax], and this is the one function that
// genuinely knows the claim does — so it needs no refinement from the
// qualifier, unlike `add`, which cannot promise `NonEmpty` [qual-refn].
intrinsic fn add_sorted<T>(list: Mut Sorted List<T>, elem: T) [] -> None
    => list: Mut Sorted, !elem

// [col-sorted-list] The index of `elem`, or `None`. An honest signature only
// because the parameter is `Sorted`: over an unordered list the answer would
// be meaningless rather than merely absent. With equal elements it answers
// the **lowest** matching index, on both backends.
intrinsic fn binary_search<T>(list: Sorted List<T>, elem: T) [] -> Int?
    => list, elem
