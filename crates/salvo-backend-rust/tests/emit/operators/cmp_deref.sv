// A borrowed Copy scalar an operator takes is copied out [rs-cmp-deref].

//--- a_borrowed_copy_scalar_derefs_as_an_operand
// [rs-cmp-deref] A borrowed Copy scalar is **copied out where an operator
// takes it**: Rust implements neither `&i32 == i32` (E0277) nor
// `i32::wrapping_add(&i32, …)`, so a lending call's scalar result — the
// total `get`'s — derefs as a comparison's or an arithmetic operator's
// operand. Kotlin has no references and needs nothing.
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
        // Arithmetic is a wrapping function of `i32`s [rs-op-wrap], so it
        // takes the scalar copied out the same way (the comment said it must
        // not until 2026-10-30, from before wrapping arithmetic).
        //~ emits: i32::wrapping_add((*crate::std_core_list::get::<i32>(&xs, i)), 1i32)
        println("${get(xs, i) + 1}")
    }
}
