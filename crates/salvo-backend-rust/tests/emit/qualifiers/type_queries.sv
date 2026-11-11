// [type-query] [assert-type] `type?(place)` and `assert!(place: T)` are the
// checker's: as statements they emit nothing, and as a value `type?` is the
// unit value.

//--- type_queries_lower_to_nothing
fn quiet(s: Str) [] -> Int => s {
    let i = size(s)
    //~ emits: let mut i: i32 = crate::std_core_string::size_platform(s);
    //~|     return i;
    type?(i)
    assert!(i: Int)
    return i
}

fn main() {
    let n = 1
    //~ emits: let mut _q: () = ();
    let _q = type?(n)
    let _m = n
}
