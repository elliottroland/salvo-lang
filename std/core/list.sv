// `canbe Mut` opts List into the language-level Mut auto-qualifier
// [type-canbe-mut]: a backend may map `Mut List<T>` to a different native
// type (Kotlin's `MutableList<T>`) or to the same one (Rust's `Vec<T>`,
// where mutability lives in the binding).
intrinsic type List<T> canbe Mut

// Constructor. The elements are stored in the new list, so they are moved
// (an empty deduction list moves every parameter) [decl-explicit].
intrinsic fn list<T canbe Linear>(...elems: T[]) [] -> [] List<T>

// Mutable constructor
intrinsic fn mutable_list<T canbe Linear>(...elems: T[]) [] -> [] Mut List<T>

// Possibly gets the element at the given index if the list is long enough
intrinsic fn get<T>(list: List<T>, index: Int) [] -> [list, index] (Proj[from: list] T)?

// Adds an element to the list. The list takes ownership of `elem`, so it
// is moved; the list itself is mutated, which is why its surviving
// qualifiers are listed exhaustively [deduce-syntax].
intrinsic fn add<T canbe Linear>(list: Mut List<T>, elem: T) [] -> [list: Mut] None

intrinsic fn first<T>(list: List<T>) [] -> [list] Proj[from: list] T?

// Returns the number of elements in the list
intrinsic fn size<T canbe Linear>(list: List<T>) [] -> [list] Int

// [iter-pass] A fresh pass over the list, which is what `for x in iter(xs)`
// and every combinator walks. The pass **borrows** the list — it is a view
// with a position [proj-pass-field] — so iterating a list by hand keeps it
// usable, and nothing is copied or consumed on the way [copy-opt-in].
fn iter<T>(list: List<T>) [] -> [list] Proj[from: list] Mut ListYield<T> {
    return Mut ListYield<T> { items: list, at: 0 }
}

// [iter-protocol] The pass a list is walked by: the list plus a position in
// it. An ordinary struct with an ordinary `next` — there is no special
// container protocol, which is what "a pass is a user struct" means. The
// backends keep their native loop as a fast path for a `for` over a list
// [iter-for-native], so this shape is what *combinators* see.
struct ListYield<T> : Yield<self, Proj T> canbe Mut {
    // The list being walked — borrowed, not owned: a pass is a position in
    // someone else's data, which is why it may hold a `Proj` field where an
    // ordinary struct may not [proj-pass-field].
    items: Proj List<T>,
    // The index of the next element to emit.
    at: Int
}

// Advances the pass, reporting the element at its position or the end of the
// list. Out of range is the end: [get] answers `None` past the last index, so
// the bound is read rather than remembered.
fn next<T>(p: Mut ListYield<T>) [] -> [p: Mut] Emitted (Proj[from: p] T) | Finished {
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
intrinsic fn to_str<T>(list: List<T>) [] -> [list] Str
