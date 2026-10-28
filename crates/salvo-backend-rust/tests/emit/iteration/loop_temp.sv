// A loop over a temporary container hoists the temporary [rs-loop-temp].

//--- a_loops_temporary_subject_is_hoisted
// [rs-loop-temp] Iterating a **temporary** container: the loop *binds* the
// pass, so a Rust temporary the subject borrows dies too early (E0716) while
// Kotlin's reference does not. The subject's temporaries are hoisted into
// locals in front of the loop — rustc's own suggestion — including through a
// view that holds the borrow (`filter(iter(…), …)`).
fn main() [use] -> None {
    use StdConsole()
    // The temporary is bound before the loop…
    //~ emits: let mut __tmp1 = vec![1i32, 2i32];
    // …and the pass borrows the local.
    //~ emits: let mut __pass_3: crate::std_core_list::ListNext<'_, i32> = crate::std_core_list::iter::<i32>(&__tmp1);
    for x in iter(List(1, 2)) {
        println("a ${x}")
    }
    // Exactly one hoist: the `filter(…)` loop iterates the returned list
    // *natively* [iter-for-native], where the temporary lives to the end of
    // the `for` statement and needs nothing, and the place subject (`xs`) is
    // already a local.
    //~ emits 1: let mut __tmp
    for x in filter(iter(List(3, 4, 5)), (n: Int) -> { return n > 3 }) {
        println("b ${x}")
    }
    let xs: List<Int> = List(9)
    for x in xs {
        println("c ${x}")
    }
}
