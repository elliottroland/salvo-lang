// Sets: an unordered collection with no duplicate elements, which
// **iterates in insertion order** [col-insertion-order] — the same on every
// backend, so printing a set or writing one to a file is deterministic
// (Kotlin's `LinkedHashSet`; a runtime helper on Rust, whose own `HashSet`
// has no order to speak of).
//
// `canbe Mut` opts Set into the language-level `Mut` auto-qualifier
// [type-canbe-mut]: a backend may map `Mut Set<T>` to a different native
// type (Kotlin's `MutableSet<T>`) or to the same one.
//
// [col-key-eligible] An element type must be **hashable**: the intrinsic
// key types (`Int`, `Long`, `Str`, `Char`, `Bool`) in this version. `Double`
// and `Float` are deliberately excluded — Rust's `f64` is neither `Eq` nor
// `Hash`, so a float-keyed set is not representable on both backends
// [backend-parity]. A struct joins in by having a `hash` and an `eq` — one
// token with `: auto Hashed<self>`, or hand-written and `@`-scoped
// [cmp-auto] [cmp-canonical].
//
// Own module rather than part of `core.list` so a program that never uses a
// set emits no code for it [mod-used-only].
export intrinsic type Set<T>(?hash: (T) -> Long, ?eq: (T, T) -> Bool) canbe Mut

// Constructor. The elements are stored in the new set, so they are moved: a
// variadic tail is owned, and needs no entry in the clause [deduce-syntax].
// A duplicate element is dropped — the last one wins, as it does in a set
// literal [col-duplicate-keys], so the result may be smaller than the
// argument list.
export intrinsic fn set_of<T>(...elems: T[], ?Hashed<T>) [] -> Set<T>(?hash, ?eq)

// Mutable constructor
export intrinsic fn mut_set_of<T>(...elems: T[], ?Hashed<T>) [] -> Mut Set<T>(?hash, ?eq)

// [col-by] Builds a set from [size] generated elements. Duplicates collapse,
// so the result may hold fewer than [size].
export intrinsic fn set_by<T>(size: Int, init: (Int) -> T, ?Hashed<T>) [] -> Set<T>(?hash, ?eq)
=> size, init

// Mutable variant
export intrinsic fn mut_set_by<T>(size: Int, init: (Int) -> T, ?Hashed<T>) [] -> Mut Set<T>(?hash, ?eq)
=> size, init

// [col-convert] The elements of [list] as a set, in first-appearance order;
// duplicates collapse.
export intrinsic fn to_set<T>(list: List<T>, ?Hashed<T>) [] -> Set<T>(?hash, ?eq) => list

// Adds an element to the set, reporting whether it was *new*: `false` means
// an equal element was already there and the set is unchanged. The set
// takes ownership of [elem], so it is moved; the set itself is mutated,
// which is why its surviving qualifiers are listed exhaustively
// [deduce-syntax].
export intrinsic fn add<T>(set: Mut Set<T>, elem: T) [] -> Bool => set: Mut, !elem

// Removes an element, reporting whether it was there. [elem] is only read —
// hashed and compared — so it is kept, not moved: removing by a value you
// still hold is the normal case. The surviving elements keep their order
// [col-insertion-order].
export intrinsic fn remove<T>(set: Mut Set<T>, elem: T) [] -> Bool => set: Mut, elem

// Whether the set holds an element equal to [elem].
export intrinsic fn contains<T>(set: Set<T>, elem: T) [] -> Bool => set, elem

// Returns the number of elements in the set
export intrinsic fn size<T>(set: Set<T>) [] -> Int => set

// The text form of a set, for string interpolation [interp-to-str]:
// `{1, 2, 3}` in insertion order — the set *literal* that would build it,
// which is the same shape `to_str` of a list follows [col-to-str]. An
// `intrinsic` rather than Salvo code because rendering the *elements* is
// the backend's own formatting.
export intrinsic fn to_str<T>(set: Set<T>) [] -> Str => set


// [col-distinct] The claim that a list holds no duplicates. Mint-only: there
// is no `qualifies`, because deciding it needs the elements compared, which
// only a backend can do over an unconstrained `T` — so it is established by
// construction and never tested.
//
// It lives here, in `core.set`, rather than beside `List`: a constructor must
// be declared in the same file as the qualifier it claims
// [qual-ctor-same-file], and a *set* is the thing that can honestly promise
// it. (`to_list` over a `SortedSet` is in `core.sorted` and so cannot mint
// it, which is why that one returns a plain list.)
export qualifier Distinct<T> of List<T> with NonEmpty, Sorted

// [col-insertion-order] The elements as a list, in insertion order — which
// is also what makes a set iterable, below. A set holds no duplicates, so
// neither does the list [col-distinct].
export intrinsic fn to_list<T>(set: Set<T>) [] -> +Distinct List<T> => set

// Reads an element **owned** out of a snapshot the pass owns.
//
// Not `copy(get(items, i))`: copying a value of an unconstrained generic
// type is unsupported on the Kotlin backend by construction — with erased
// generics it cannot tell an immutable `Str` from a `Mut List` at runtime,
// so it refuses rather than aliasing [kt-copy]. This read is sound where
// `copy` cannot be: an element of a set is a **key**, and keys are the
// immutable intrinsic types [col-key-eligible], for which sharing the
// reference *is* the copy. Kept out of `core.list` deliberately — as a
// general `List` accessor the same lowering would alias a mutable element.
export intrinsic fn snapshot_at<T>(items: List<T>, index: Int) [] -> T? => items, index

// [iter-pass] A fresh pass over the set's elements, in insertion order.
//
// Unlike a list's pass, which is a *position in* the data [proj-field], this
// one walks a **snapshot**: a hash set has no index to walk, and the
// alternative — holding the target's native iterator in an intrinsic pass —
// needs a borrowing intrinsic type neither backend has machinery for
// (see COMPLETED.md). The consequences are worth knowing: `iter` copies
// the elements out (O(n) once, then O(1) a step), and a set mutated while a
// pass over it is live keeps yielding what it held at the mint — on both
// backends alike, rather than being a race on one and a refusal on the
// other.
export fn iter<T>(set: Set<T>) [] -> Mut SetYield<T> => set {
    return Mut SetYield<T> { items: to_list(set), at: 0 }
}

// [iter-protocol] The pass a set is walked by: a snapshot of its elements
// plus a position in it. The snapshot is **owned** by the pass — the
// structural difference from `ListYield`, whose `items` is a projection of
// someone else's list — and so are the elements it emits: a pass that
// *computes* its elements owns them, where one that *walks* data borrows
// them [iter-protocol].
export struct SetYield<T> : Yield<self, T> canbe Mut {
    // The elements, in insertion order, owned by this pass.
    items: List<T>,
    // The index of the next element to emit.
    at: Int
}

// Advances the pass, reporting the element at its position or the end of the
// set.
export fn next<T>(p: Mut SetYield<T>) [] -> Emitted T | Finished => p: Mut {
    let elem = snapshot_at(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}
