define type List<T> {
    inline: ``
    List<${T}>
    ``

    Mut inline: ``
    MutableList<${T}>
    ``
}

define fn list<T>(...elems: T[]) -> List<T> {
    inline: ``
    listOf<${T}>(${...elems})
    ``
}

define fn mutable_list<T>(...elems: T[]) -> Mut List<T> {
    inline: ``
    mutableListOf<${T}>(${...elems})
    ``
}

define fn get<T>(list: List<T>, index: Int) -> T? {
    inline: ``
    ${list}.getOrNull(${index})
    ``
}

define fn add<T>(list: Mut List<T>, elem: T) {
    inline: ``
    ${list}.add(${elem})
    ``
}

define fn first<T>(list: List<T>) -> T? {
    inline: ``
    ${list}.firstOrNull()
    ``
}

define fn size<T>(list: List<T>) -> Int {
    inline: ``
    ${list}.size
    ``
}

define fn iter<T>(list: List<T>) -> Iter<T> {
    inline: ``
    ${list}
    ``
}