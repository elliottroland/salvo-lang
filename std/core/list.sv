// `with Mut` opts List into the language-level Mut auto-qualifier
// [type-with-mut]: backends may map `Mut List<T>` to a different native
// type (see the `Mut inline:` define section).
external type List<T> with Mut

// Constructor
external fn list<T with Linear>(...elems: T[]) -> List<T>

// Mutable constructor
external fn mutable_list<T with Linear>(...elems: T[]) -> Mut List<T>

// Possibly gets the element at the given index if the list is long enough
external fn get<T>(list: List<T>, index: Int) -> T?

// Adds an element to the list
external fn add<T with Linear>(list: Mut List<T>, elem: T)

external fn first<T>(list: List<T>) -> T?

// Returns the number of elements in the list
external fn size<T with Linear>(list: List<T>) -> Int

external fn iter<T>(list: List<T>) -> Iter<T>