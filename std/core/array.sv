// Arrays (`T[]`) are a language-level type: literals (`[1, 2]`),
// indexing (`arr[i]`), and `for`-iteration are built in [type-array].
// This module supplies the *function* surface, mirroring `core.list`
// minus construction (array literals are the constructor) and mutation
// (arrays are fixed-size).
//
// The `<T canbe linear>` opt-ins mirror List's [linear-generics]:
// measuring or iterating an array of linear values is fine, but taking
// an element *out* of one would duplicate the obligation, so `get` and
// `first` stay closed to linear types.

// Returns the number of elements in the array
intrinsic fn size<T canbe linear>(array: T[]) [] -> Int => array

// Possibly gets the element at the given index if the array is long enough
intrinsic fn get<T>(array: T[], index: Int) [] -> (proj[from: array] T)? => array, index

intrinsic fn first<T>(array: T[]) [] -> proj[from: array] T? => array

// [iter-pass] A fresh pass over the array — a view of it with a position:
// the array is borrowed, not moved [proj-field] [proj-infer].
fn iter<T>(array: T[]) [] -> Mut ArrayYield<T> => array {
    return Mut ArrayYield<T> { items: array, at: 0 }
}

// [iter-protocol] The pass an array is walked by — `core.list`'s [ListYield]
// with an array inside. The backends keep their native loop for a `for` over
// an array [iter-for-native]; this shape is what combinators see.
struct ArrayYield<T> : Yield<self, proj T> canbe Mut {
    // The array being walked — borrowed [proj-field].
    items: proj (T[]),
    // The index of the next element to emit.
    at: Int
}

// Advances the pass, reporting the element at its position or the end of the
// array.
fn next<T>(p: Mut ArrayYield<T>) [] -> Emitted (proj[from: p] T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}
