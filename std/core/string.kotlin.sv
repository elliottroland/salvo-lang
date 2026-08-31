define fn size(str: Str) -> Int {
    inline: ``
    ${str}.length
    ``
}

define fn char_at(str: Str, index: Positive Int) -> Char? {
    inline: ``
    ${str}.getOrNull(${index})
    ``
}