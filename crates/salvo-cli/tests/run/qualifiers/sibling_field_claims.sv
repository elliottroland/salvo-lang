// [qual-depend-field] A struct field's claim about a sibling field, end to
// end: a cursor holds a string and an index proven for it, and a small
// tokenizer reads every character through the total overload — the claim
// rooted at each field read, at the destructured locals, and re-established
// after each step by an `is` test.

//--- a_tokenizer_over_a_claimed_cursor
struct Cursor canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

// The optional read and the total one, ranked by the claim.
fn read_char(str: Str, index: Int) [] -> Char? => str, index {
    return char_at(str, index)
}

fn read_char(str: Str, index: Idx(str) Int) [] -> Char => str, index {
    let c = read_char(str, (index: Int))
    assert!(c is Char)
    return c
}

fn kind(c: Char) [] -> Int => c {
    if c >= '0' && c <= '9' {
        return 1
    } elif c == ' ' {
        return 0
    }
    return 2
}

// Moves the cursor one character on, when there is one: the destructured
// `pos` steps out of its claim, the `is` test against the destructured `str`
// gives it back, and only then may it be stored in the field.
fn step(cursor: Mut Cursor) [] -> Bool => cursor: Mut {
    let { str, pos } = cursor
    pos++
    if pos is Idx(str) {
        cursor.pos = pos
        return true
    }
    return false
}

fn main() [use] -> None {
    use StdConsole()
    let text = "let x1 = 42 + y"
    let cursor = Mut Cursor { str: text, pos: 0 as! Idx(text) }
    let word = Mut Str()
    let last = kind(cursor.str.read_char(cursor.pos))
    let more = true
    while more {
        let c = cursor.str.read_char(cursor.pos)
        let k = kind(c)
        if k != last {
            if last != 0 {
                println("[${word}]")
            }
            word.clear()
            last = k
        }
        word.append("${c}")
        more = step(cursor)
    }
    if last != 0 {
        println("[${word}]")
    }
    let { str, pos } = cursor
    println("last: ${str.read_char(pos)} at ${pos}")
}
//= stdout
//= [let]
//= [x]
//= [1]
//= [=]
//= [42]
//= [+]
//= [y]
//= last: y at 14
