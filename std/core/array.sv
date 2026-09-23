// Arrays (`T[]`) are a language-level type: indexing (`arr[i]`) and
// `for`-iteration are built in [type-array]. Since 2026-09-13 the bracket
// literal `[1, 2]` builds a **List**, not an array [col-literal], so this
// module supplies the constructor too — arrays are fixed-size, so there is
// nothing to mutate but element assignment.
//
// The `<T canbe linear>` opt-ins mirror List's [linear-generics]:
// measuring or iterating an array of linear values is fine, but taking
// an element *out* of one would duplicate the obligation, so `get` and
// `first` stay closed to linear types.

// Builds an array from [elems]. The variadic tail **is** the array, which is
// what makes this the constructor rather than a copy: a `...spread` argument
// arrives as the whole array already [fn-variadic]. Arrays exist for the
// variadic boundary, so a program that never spreads rarely needs one — a
// `List` is the ordinary sequence [col-literal].
export intrinsic fn array_of<T canbe linear>(...elems: T[]) [] -> T[]

// Builds an array of [size] elements, each from its index: `array_by(3, i ->
// i * 2)` is `[0, 2, 4]`. The generator is called once per index, in order
// [col-by].
export intrinsic fn array_by<T>(size: Int, init: (Int) -> T) [] -> T[] => size, init

// Returns the number of elements in the array
export intrinsic fn size<T canbe linear>(array: T[]) [] -> Int => array

// Possibly gets the element at the given index if the array is long enough
export intrinsic fn get<T>(array: T[], index: Int) [] -> (proj(array) T)? => array, index

export intrinsic fn first<T>(array: T[]) [] -> proj(array) T? => array

// [iter-pass] A fresh pass over the array — a view of it with a position:
// the array is borrowed, not moved [proj-field] [proj-infer].
export fn iter<T>(array: T[]) [] -> Mut ArrayYield<T> => array {
    return Mut ArrayYield<T> { items: array, at: 0 }
}

// [iter-protocol] The pass an array is walked by — `core.list`'s [ListYield]
// with an array inside. The backends keep their native loop for a `for` over
// an array [iter-for-native]; this shape is what combinators see.
export struct ArrayYield<T> : Yield<self, proj T> canbe Mut {
    // The array being walked — borrowed [proj-field].
    items: proj (T[]),
    // The index of the next element to emit.
    at: Int
}

// Advances the pass, reporting the element at its position or the end of the
// array.
export fn next<T>(p: Mut ArrayYield<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}
