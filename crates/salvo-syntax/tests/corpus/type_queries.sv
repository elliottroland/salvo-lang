// [type-query] [type-assert] Asking for a place's type, and asserting it,
// as docs/language/Control-Flow.md "Finding out a type" writes them.

struct Parsing canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

fn step(parsing: Mut Parsing) [] -> None => parsing: Mut {
    let { str, pos } = parsing
    type?(pos)
    type?(parsing.pos)
    type!(pos: Idx(str) Int)
    type!(pos: Idx(parsing.str) Int)
    pos++
    type!(pos: Int)
    if pos is Idx(str) {
        type!(pos: Idx(str) Int)
        parsing.pos = pos
    }
    let _shown = type?(parsing)
    type!(_shown: None)
}

fn arms(v: Ok Int | Str) [] -> None => v {
    type!(v: Ok Int | Str)
    if v is Ok {
        assert!((v: -Ok) is Int)
    }
}

fn main() {}
