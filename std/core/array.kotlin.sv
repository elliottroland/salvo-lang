// Kotlin defines for core.array [backend-define-inline]. `T[]` maps to
// `Array<T>` (see the emitter's array rendering), which is not itself
// `Iterable`, hence `asIterable()` for `iter`.

define fn size<T>(array: T[]) -> Int {
    inline: ``
    ${array}.size
    ``
}

define fn get<T>(array: T[], index: Int) -> T? {
    inline: ``
    ${array}.getOrNull(${index})
    ``
}

define fn first<T>(array: T[]) -> T? {
    inline: ``
    ${array}.firstOrNull()
    ``
}

define fn iter<T>(array: T[]) -> Iter<T> {
    inline: ``
    ${array}.asIterable()
    ``
}
