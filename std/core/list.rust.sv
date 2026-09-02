// Rust defines for core.list [backend-define-inline] [rs-borrows].
//
// `List<T>` maps to `Vec<T>`; there is deliberately no `Mut inline:`
// section [type-with-mut]: Rust expresses mutability through bindings
// and references, not through a different type.

define type List<T> {
    inline: ``
    Vec<${T}>
    ``
}

define fn list<T>(...elems: T[]) -> List<T> {
    inline: ``
    vec![${...elems}]
    ``
}

define fn mutable_list<T>(...elems: T[]) -> Mut List<T> {
    inline: ``
    vec![${...elems}]
    ``
}

define fn get<T>(list: List<T>, index: Int) -> T? {
    inline: ``
    ${list}.get((${index}) as usize).cloned()
    ``
}

define fn add<T>(list: Mut List<T>, elem: T) {
    inline: ``
    ${list}.push(${elem})
    ``
}

define fn first<T>(list: List<T>) -> T? {
    inline: ``
    ${list}.first()
    ``
}

define fn size<T>(list: List<T>) -> Int {
    inline: ``
    (${list}.len() as i32)
    ``
}

define fn iter<T>(list: List<T>) -> Iter<T> {
    inline: ``
    ${list}.clone()
    ``
}
