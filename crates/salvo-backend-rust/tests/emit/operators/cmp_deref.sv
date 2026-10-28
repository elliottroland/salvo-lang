// A borrowed Copy scalar in a comparison is copied out [rs-cmp-deref].

//--- a_borrowed_copy_scalar_derefs_in_a_comparison_only
// [rs-cmp-deref] A borrowed Copy scalar is **copied out in a comparison**:
// Rust implements `&i32 + i32` but not `&i32 == i32` (E0277), so a lending
// call's scalar result — the total `get`'s — derefs there and nowhere else.
// Kotlin has no references and needs nothing.
fn main() [use] {
    use StdConsole()
    let xs = List(10, 20, 30)
    let i = 1
    if i is Idx(xs) {
        // A borrowed Copy scalar in a *comparison*: the operand is copied out.
        //~ emits: if ((*crate::std_core_list::get::<i32>(&xs, i)) == 20i32) {
        if get(xs, i) == 20 { println("eq") }
        //~ emits: if ((*crate::std_core_list::get::<i32>(&xs, i)) < 30i32) {
        if get(xs, i) < 30 { println("lt") }
        // Arithmetic and interpolation need no deref (Rust implements
        // `&i32 + i32`), and must not grow one.
        // Arithmetic takes the scalar the same way.
        //~ emits: i32::wrapping_add((*crate::std_core_list::get::<i32>(&xs, i)), 1i32)
        println("${get(xs, i) + 1}")
    }
}
