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
intrinsic fn get<T>(list: List<T>, index: Int) [] -> [list, index] T?

// Adds an element to the list. The list takes ownership of `elem`, so it
// is moved; the list itself is mutated, which is why its surviving
// qualifiers are listed exhaustively [deduce-syntax].
intrinsic fn add<T canbe Linear>(list: Mut List<T>, elem: T) [] -> [list: Mut] None

intrinsic fn first<T>(list: List<T>) [] -> [list] ReadOnly[from: list] T?

// Returns the number of elements in the list
intrinsic fn size<T canbe Linear>(list: List<T>) [] -> [list] Int

intrinsic fn iter<T>(list: List<T>) [] -> [list] Iter<T>
