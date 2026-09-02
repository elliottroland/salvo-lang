// Rust defines for core.array [backend-define-inline] [rs-borrows].
// `T[]` maps to `Vec<T>` (the same rendering as `List<T>`), so these
// mirror the `core.list` defines exactly.

define fn size<T>(array: T[]) -> Int {
    inline: ``
    (${array}.len() as i32)
    ``
}

define fn get<T>(array: T[], index: Int) -> T? {
    inline: ``
    ${array}.get((${index}) as usize).cloned()
    ``
}

define fn first<T>(array: T[]) -> T? {
    inline: ``
    ${array}.first()
    ``
}

define fn iter<T>(array: T[]) -> Iter<T> {
    inline: ``
    ${array}.clone()
    ``
}
