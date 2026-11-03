// [col-nonempty] std's generic `NonEmpty`: one `qualifies`, its `size` the
// subject's own.

//--- the_generic_claim_is_tested_at_the_subjects_size
// The test binds `T` from the subject, so the call passes the list's `size`,
// not whichever `size` would fit an unknown parameter, and names `T` — a
// caller's own `T` is never read as the qualifier's.
fn main() [use] -> None {
    use StdConsole()
    let xs = List<Int>()
    //~ emits: crate::std_core_nonempty::NonEmpty_qualifies::<Vec<i32>>(&xs, &mut |__a0: &Vec<i32>| crate::std_core_list::size_platform(__a0))
    //~ !emits: std_core_array::size
    if xs is NonEmpty {
        println("list")
    }
}
