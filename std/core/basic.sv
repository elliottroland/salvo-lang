internal type Byte
internal type Int
internal type Long
internal type Float
internal type Double
internal type Char
internal type Bool
internal type Any
internal type Nothing

// The iterator type driving `for`-loops and `yield` functions
internal type Iter<T>

type Number = Int | Long | Double | Float

// [internal-fn] [copy-fn] Duplicates a value: the argument is kept
// untouched (with all its qualifiers) and the result is a fresh,
// independent value with no fate links to the source. Implemented by
// each backend directly (Kotlin: identity for transitively immutable
// types, a real copy for Mut-capable ones; Rust: `.clone()`).
internal fn copy<T>(value: T) [] -> [value] T

// [internal-fn] [linear-discard] Deliberately drops a value, consuming
// it: the escape hatch for linear types — the one generic fn blessed to
// accept them. Implemented by each backend directly (Kotlin: evaluate
// and ignore; Rust: `drop`).
internal fn discard<T canbe Linear>(value: T) [] -> [] None
