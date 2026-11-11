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

// An index joined from two spellings of one claim — a `pos` destructured
// beside its `str`, and the field read `cursor.pos` — is one claim, so it
// reads through the total overload, the only one there is.

//--- a_joined_index_reads_totally
struct Cursor {
    str: proj Str
    pos: Idx(str) Int
}

fn read_char(str: Str, index: Idx(str) Int) [] -> Char => str, index {
    return char_at(str, index)
}

fn by_if(cursor: Cursor, parts: Bool) [] -> Char => cursor {
    let p = if parts {
        let { str, pos } = cursor
        let _n = size(str)
        pos
    } else {
        cursor.pos
    }
    return read_char(cursor.str, p)
}

fn by_when(cursor: Cursor, n: Int) [] -> Char => cursor {
    let p = when {
        n == 0 { cursor.pos }
        n == 1 {
            let { str, pos } = cursor
            let _n = size(str)
            pos
        }
        else {
            let { pos } = cursor
            pos
        }
    }
    let { str } = cursor
    return read_char(str, p)
}

fn by_assignment(cursor: Cursor, parts: Bool) [] -> Char => cursor {
    let p = cursor.pos
    if parts {
        let { str, pos } = cursor
        let _n = size(str)
        p = pos
    }
    return read_char(cursor.str, p)
}

fn by_return(cursor: Cursor, parts: Bool) [] -> Idx(cursor.str) Int => cursor {
    if parts {
        let { str, pos } = cursor
        let _n = size(str)
        return pos
    }
    return cursor.pos
}

fn returned(cursor: Cursor) [] -> Str => cursor {
    let i = by_return(cursor, true)
    let j = by_return(cursor, false)
    return "${read_char(cursor.str, i)}${read_char(cursor.str, j)} at ${i + j}"
}

fn main() [use] -> None {
    use StdConsole()
    let text = "salvo"
    let cursor = Cursor { str: text, pos: 3 as! Idx(text) }
    println("${by_if(cursor, true)}${by_if(cursor, false)}")
    println("${by_when(cursor, 0)}${by_when(cursor, 1)}${by_when(cursor, 2)}")
    println("${by_assignment(cursor, true)}${by_assignment(cursor, false)}")
    println(returned(cursor))
}
//= stdout
//= vv
//= vvv
//= vv
//= vv at 6

// A claim about a value that ended with its branch is dropped at the join:
// the joined index is a plain `Int`, and it lowers and runs as one, alone,
// beside a claim still named, in an optional and in a list.

//--- a_claim_nothing_names_is_dropped_at_the_join
struct Cursor {
    str: proj Str
    pos: Idx(str) Int
}

fn plain(i: Int) [] -> Str => i {
    return "${i}"
}

fn ended(c: Bool) [] -> Int {
    let p = if c {
        let s = "abc"
        2 as! Idx(s)
    } else {
        let t = "xy"
        1 as! Idx(t)
    }
    return p
}

fn mixed(cursor: Cursor, c: Bool) [] -> Int => cursor {
    let p = if c {
        cursor.pos
    } else {
        let s = "abc"
        0 as! Idx(s)
    }
    return p
}

fn nested(cursor: Cursor, c: Bool) [] -> Int => cursor {
    let o = if c {
        let s = "abc"
        let x: (Idx(s) Int)? = 2 as! Idx(s)
        x
    } else {
        cursor.pos
    }
    let xs = if c {
        List(cursor.pos)
    } else {
        let s = "abc"
        List(1 as! Idx(s))
    }
    return (o ?: 0) * 10 + first(xs)
}

fn main() [use] -> None {
    use StdConsole()
    let text = "salvo"
    let cursor = Cursor { str: text, pos: 3 as! Idx(text) }
    println("${plain(ended(true))} ${plain(ended(false))}")
    println("${mixed(cursor, true)} ${mixed(cursor, false)}")
    println("${nested(cursor, true)} ${nested(cursor, false)}")
}
//= stdout
//= 2 1
//= 3 0
//= 23 31

// A claim not every path holds is dropped at the join: two cursors' indexes,
// or an index and a plain `Int`, join to a plain `Int` that an `Int`
// parameter takes, however the paths are grouped.

//--- a_claim_not_every_path_holds_is_dropped
struct Cursor {
    str: proj Str
    pos: Idx(str) Int
}

fn plain(i: Int) [] -> Str => i {
    return "${i}"
}

fn either(a: Cursor, b: Cursor, c: Bool) [] -> Str => a, b {
    return plain(if c { a.pos } else { b.pos })
}

fn or_zero(a: Cursor, c: Bool) [] -> Str => a {
    let p = if c { a.pos } else { 0 }
    return plain(p)
}

fn grouped(a: Cursor, b: Cursor, n: Int) [] -> Int => a, b {
    let p = if n == 0 {
        a.pos
    } else {
        if n == 1 { b.pos } else { 7 }
    }
    let q = if n != 2 {
        if n == 0 { a.pos } else { b.pos }
    } else {
        7
    }
    return p * 10 + q
}

fn main() [use] -> None {
    use StdConsole()
    let s = "salvo"
    let t = "kiwi"
    let a = Cursor { str: s, pos: 3 as! Idx(s) }
    let b = Cursor { str: t, pos: 1 as! Idx(t) }
    println("${either(a, b, true)} ${either(a, b, false)}")
    println("${or_zero(a, true)} ${or_zero(a, false)}")
    println("${grouped(a, b, 0)} ${grouped(a, b, 1)} ${grouped(a, b, 2)}")
}
//= stdout
//= 3 1
//= 3 0
//= 33 11 77

// A written union of two claims is kept, and so is a join declared that
// type: both arms are one runtime `Int`.

//--- a_written_union_of_two_claims_runs
struct Cursor {
    str: proj Str
    pos: Idx(str) Int
}

fn show(a: Cursor, b: Cursor, x: Idx(a.str) Int | Idx(b.str) Int) [] -> Str => a, b, x {
    return "${x}"
}

fn either(a: Cursor, b: Cursor, c: Bool) [] -> Str => a, b {
    let y: Idx(a.str) Int | Idx(b.str) Int = if c { a.pos } else { b.pos }
    return "${show(a, b, y)} ${show(a, b, b.pos)}"
}

fn main() [use] -> None {
    use StdConsole()
    let s = "salvo"
    let t = "kiwi"
    let a = Cursor { str: s, pos: 3 as! Idx(s) }
    let b = Cursor { str: t, pos: 1 as! Idx(t) }
    println(either(a, b, true))
    println(either(a, b, false))
}
//= stdout
//= 3 1
//= 1 1
