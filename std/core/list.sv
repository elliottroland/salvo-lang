// `with Mut` opts List into the language-level Mut auto-qualifier
// [type-with-mut]: backends may map `Mut List<T>` to a different native
// type (see the `Mut inline:` define section).
external type List<T> with Mut

// Constructor. The elements are stored in the new list, so they are moved
// (an empty deduction list moves every parameter) [decl-explicit].
external fn list<T with Linear>(...elems: T[]) [] -> [] List<T>

// Mutable constructor
external fn mutable_list<T with Linear>(...elems: T[]) [] -> [] Mut List<T>

// Possibly gets the element at the given index if the list is long enough
external fn get<T>(list: List<T>, index: Int) [] -> [list, index] T?

// Adds an element to the list. The list takes ownership of `elem`, so it
// is moved; the list itself is mutated, which is why its surviving
// qualifiers are listed exhaustively [deduce-syntax].
external fn add<T with Linear>(list: Mut List<T>, elem: T) [] -> [list: Mut] None

external fn first<T>(list: List<T>) [] -> [list] ReadOnly[from: list] T?

// Returns the number of elements in the list
external fn size<T with Linear>(list: List<T>) [] -> [list] Int

external fn iter<T>(list: List<T>) [] -> [list] Iter<T>
