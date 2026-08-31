external type List<T>

// Constructor
external fn list<T>(...elems: T[]) -> List<T>

internal qualifier Mut<T> of List<T>

// Mutable constructor
external fn mutable_list<T>(...elems: T[]) -> Mut List<T>

// Possibly gets the element at the given index if the list is long enough
external fn get<T>(list: List<T>, index: Int) -> T?

// Adds an element to the list
external fn add<T>(list: Mut List<T>, elem: T)

external fn first<T>(list: List<T>) -> T?

external fn iter<T>(list: List<T>) -> Iter<T>