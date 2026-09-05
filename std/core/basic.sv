intrinsic type Byte
intrinsic type Int
intrinsic type Long
intrinsic type Float
intrinsic type Double
intrinsic type Char
intrinsic type Bool
intrinsic type Any
intrinsic type Nothing

// The iterator type driving `for`-loops and `yield` functions
intrinsic type Iter<T>

type Number = Int | Long | Double | Float

// [intrinsic-fn] [copy-fn] Duplicates a value: the argument is kept
// untouched (with all its qualifiers) and the result is a fresh,
// independent value with no fate links to the source. Implemented by
// each backend directly (Kotlin: identity for transitively immutable
// types, a real copy for Mut-capable ones; Rust: `.clone()`).
intrinsic fn copy<T>(value: T) [] -> [value] T

// [intrinsic-fn] [linear-discard] Deliberately drops a value, consuming
// it: the escape hatch for linear types — the one generic fn blessed to
// accept them. Implemented by each backend directly (Kotlin: evaluate
// and ignore; Rust: `drop`).
intrinsic fn discard<T canbe Linear>(value: T) [] -> [] None
