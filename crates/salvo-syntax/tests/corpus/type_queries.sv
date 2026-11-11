// [type-query] [assert-type] Asking for a place's type, and asserting it,
// as docs/language/Control-Flow.md "Finding out a type" writes them.

struct Parsing canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

fn step(parsing: Mut Parsing) [] -> None => parsing: Mut {
    let { str, pos } = parsing
    type?(pos)
    type?(parsing.pos)
    assert!(pos: Idx(str) Int)
    assert!(pos: Idx(parsing.str) Int)
    pos++
    assert!(pos: Int)
    if pos is Idx(str) {
        assert!(pos: Idx(str) Int)
        parsing.pos = pos
    }
    let _shown = type?(parsing)
    assert!(_shown: None)
}

fn arms(v: Ok Int | Str) [] -> None => v {
    assert!(v: Ok Int | Str)
    if v is Ok {
        assert!((v: -Ok) is Int)
    }
}

fn main() {}
