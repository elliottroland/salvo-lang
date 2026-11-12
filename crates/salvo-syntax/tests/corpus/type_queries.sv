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

// `type!(place: T)` takes a whole type, a union and a `proj` included, and a
// field for its place; `assert!` is the runtime assertion, so a re-typing
// right inside it is its condition.
fn assertions(parsing: Parsing, x: Int, o: Int?, b: Bool, s: proj Str | None) [] -> None => parsing, s {
    type!(parsing.pos: Idx(parsing.str) Int)
    type!(o: Int | None)
    type!(s: proj Str | None)
    assert!(x is Int)
    assert!(b: Bool)
}
