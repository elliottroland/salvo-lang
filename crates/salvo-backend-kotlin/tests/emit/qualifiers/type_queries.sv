// [type-query] [assert-type] `type?(place)` and `assert!(place: T)` are the
// checker's: as statements they emit nothing, and as a value `type?` is the
// unit value.

//--- type_queries_lower_to_nothing
fn quiet(s: Str) [] -> Int => s {
    let i = size(s)
    //~ emits: val i: Int = salvo.std.core.string.sizePlatform(s)
    //~|     return i
    type?(i)
    assert!(i: Int)
    return i
}

fn main() {
    let n = 1
    //~ emits: val _q: Unit = Unit
    let _q = type?(n)
    let _m = n
}
