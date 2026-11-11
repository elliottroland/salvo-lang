// [str-char] A string read and built one character at a time: the total
// `char_at` reads at an index a loop's `is Idx(s)` test proves, and
// `append(Mut Str, Char)` writes a character without making a string of it.

//--- reversing_and_escaping_by_character
fn reversed(s: Str) [] -> Str => s {
    let out = Mut Str()
    let i = size(s)
    while --i is Idx(s) {
        append(out, char_at(s, i))
    }
    return out
}

// Every `"` and `\` gets a `\` in front of it.
fn escaped(s: Str) [] -> Str => s {
    let out = Mut Str()
    let i = -1
    while ++i is Idx(s) {
        let c = char_at(s, i)
        if c == '"' || c == '\\' {
            append(out, '\\')
        }
        append(out, c)
    }
    return out
}

fn main() [use] -> None {
    use StdConsole()
    println(reversed("salvo"))
    println(escaped("say \"hi\" \\ bye"))
    println("<${reversed("")}>")
    // Appending a character makes a string non-empty, from empty.
    let built = Mut Str()
    append(built, 'x')
    append(built, "yz")
    println("${built} ${size(built)}")
    // An index that has not been tested still reads through the optional.
    println("${char_at("abc", 3) ?: '-'}")
}
//= stdout
//= ovlas
//= say \"hi\" \\ bye
//= <>
//= xyz 3
//= -
