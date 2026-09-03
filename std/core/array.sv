// Arrays (`T[]`) are a language-level type: literals (`[1, 2]`),
// indexing (`arr[i]`), and `for`-iteration are built in [type-array].
// This module supplies the *function* surface, mirroring `core.list`
// minus construction (array literals are the constructor) and mutation
// (arrays are fixed-size).
//
// The `<T with Linear>` opt-ins mirror List's [linear-generics]:
// measuring or iterating an array of linear values is fine, but taking
// an element *out* of one would duplicate the obligation, so `get` and
// `first` stay closed to linear types.

// Returns the number of elements in the array
external fn size<T with Linear>(array: T[]) [] -> [array] Int

// Possibly gets the element at the given index if the array is long enough
external fn get<T>(array: T[], index: Int) [] -> [array, index] T?

external fn first<T>(array: T[]) [] -> [array] ReadOnly[from: array] T?

external fn iter<T with Linear>(array: T[]) [] -> [array] Iter<T>
