// [type-query] [type-assert] `type?(place)` and `type!(place: T)` are the
// checker's: they lower to nothing, so a program holding them prints what it
// would without them, identically on both backends.

//--- type_queries_and_type_assertions_print_nothing
struct Cursor canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

fn main() [use] -> None {
    use StdConsole()
    let s = "hello"
    let i = 1
    println("start")
    type?(s)
    type!(s: Str)
    if i is Idx(s) {
        type?(i)
        type!(i: Idx(s) Int)
        let cursor = Cursor { str: s, pos: i }
        let { str, pos } = cursor
        type?(cursor.pos)
        type!(pos: Idx(cursor.str) Int)
        type!(cursor.pos: Idx(str) Int)
        println("at ${pos} of ${size(str)}")
    }
    // As a value it is `None`.
    let q = type?(i)
    if q is None {
        println("none")
    }
    println("end ${i}")
}
//= stdout
//= start
//= at 1 of 5
//= none
//= end 1
