// Rust defines for core.string [backend-define-inline].

define fn size(str: Str) -> Int {
    inline: ``
    (${str}.chars().count() as i32)
    ``
}

define fn char_at(str: Str, index: Positive Int) -> Char? {
    inline: ``
    ${str}.chars().nth((${index}) as usize)
    ``
}
