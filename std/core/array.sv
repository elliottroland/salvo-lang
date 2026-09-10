// Arrays (`T[]`) are a language-level type: literals (`[1, 2]`),
// indexing (`arr[i]`), and `for`-iteration are built in [type-array].
// This module supplies the *function* surface, mirroring `core.list`
// minus construction (array literals are the constructor) and mutation
// (arrays are fixed-size).
//
// The `<T canbe Linear>` opt-ins mirror List's [linear-generics]:
// measuring or iterating an array of linear values is fine, but taking
// an element *out* of one would duplicate the obligation, so `get` and
// `first` stay closed to linear types.

// Returns the number of elements in the array
intrinsic fn size<T canbe Linear>(array: T[]) [] -> [array] Int

// Possibly gets the element at the given index if the array is long enough
intrinsic fn get<T>(array: T[], index: Int) [] -> [array, index] T?

intrinsic fn first<T>(array: T[]) [] -> [array] ReadOnly[from: array] T?

// [iter-pass] A fresh pass over the array; the array is moved into it.
fn iter<T>(array: T[]) [] -> [] Mut ArrayYield<T> {
    return Mut ArrayYield<T> { items: array, at: 0 }
}

// [iter-protocol] The pass an array is walked by — `core.list`'s [ListYield]
// with an array inside. The backends keep their native loop for a `for` over
// an array [iter-for-native]; this shape is what combinators see.
struct ArrayYield<T> : Yield<self, T> canbe Mut {
    // The array being walked.
    items: T[],
    // The index of the next element to emit.
    at: Int
}

// Advances the pass, reporting the element at its position or the end of the
// array.
fn next<T>(p: Mut ArrayYield<T>) [] -> [p: Mut] Emitted T | Finished {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}
