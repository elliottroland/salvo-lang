// [is-narrow-step] An `is` test of a prefix step narrows the stepped
// variable, end to end: a string-literal reader steps its index in the
// loop's test and reads each character through a total read, which only
// the claim the test gives makes callable. A postfix step tests the old
// value, so its body sees the index one further on.

//--- a_string_reader_over_a_claimed_cursor
struct Cursor canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

// Only the total read: every call below compiles because the index is
// claimed where it is made.
fn read_char(str: Str, index: Idx(str) Int) [] -> Char => str, index {
    return char_at(str, (index: Int)) ?: unreachable!()
}

// Reads the string literal at the cursor (`"…"`, with `\` escaping the next
// character), and moves the cursor past it when something follows.
fn next_str(cursor: Mut Cursor) [] -> Str? => cursor: Mut {
    let { str, pos } = cursor
    if str.read_char(pos) != '"' {
        return None
    }
    let value = Mut Str()
    let escape_next = false
    let closed = false
    while ++pos is Idx(str) {
        let char = str.read_char(pos)
        if escape_next {
            escape_next = false
        } elif char == '"' {
            closed = true
            break
        } elif char == '\\' {
            escape_next = true
            continue
        }
        value.append("${char}")
    }
    if !closed {
        return None
    }
    if ++pos is Idx(str) {
        cursor.pos = pos
    }
    return value
}

fn show(read: Str?) [] -> Str => read {
    return when read {
        is Str { "<${read}>" }
        is None { "unterminated" }
    }
}

fn main() [use] -> None {
    use StdConsole()
    let text = "\"ab\\\"c\" \"\" \"tail"
    let cursor = Mut Cursor { str: text, pos: 0 as! Idx(text) }
    let more = true
    while more {
        println(show(next_str(cursor)))
        let { str, pos } = cursor
        more = false
        while ++pos is Idx(str) {
            if str.read_char(pos) == '"' {
                cursor.pos = pos
                more = true
                break
            }
        }
    }
    // The test reads the new value with a prefix step and the old one with
    // a postfix step; the step happens whether it holds or not.
    let i = -1
    while ++i is Idx(text) {
        if i > 2 {
            break
        }
        println("prefix ${i}: ${text.read_char(i)}")
    }
    let j = 0
    while j++ is Idx(text) {
        println("postfix ${j}")
        if j > 2 {
            break
        }
    }
    let k = 15
    while ++k is Idx(text) {
        println("never")
    }
    println("after: ${i} ${j} ${k}")
}
//= stdout
//= <ab"c>
//= <>
//= unterminated
//= prefix 0: "
//= prefix 1: a
//= prefix 2: b
//= postfix 1
//= postfix 2
//= postfix 3
//= after: 3 3 16

//--- a_stepped_optional_local
// A local stored as an optional, narrowed to its `Int`, stepped in the
// test: the narrowing keeps the local's optional storage.
// Rust only: Kotlin emits a step of a narrowed optional local as an
// assignment to the `val` its narrowing reads (ROADMAP §0bg).
// BACKENDS: rust
fn read_char(str: Str, index: Idx(str) Int) [] -> Char => str, index {
    return char_at(str, (index: Int)) ?: unreachable!()
}

fn start() [] -> Int? {
    return 12
}

fn main() [use] -> None {
    use StdConsole()
    let text = "0123456789abcdef"
    let o = start()
    if o is Int {
        while ++o is Idx(text) {
            println("${o}: ${text.read_char(o)}")
        }
        println("after: ${o}")
    }
}
//= stdout
//= 13: d
//= 14: e
//= 15: f
//= after: 16
