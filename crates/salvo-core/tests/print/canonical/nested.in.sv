// [fmt-width] A unit inside a unit: the statements of a block the width
// breaks, and a construct of the statement around them printed after the
// block, which the width breaks too, in one printing (`nested.out.sv`).
// Parsed only (`print_tests.rs`), never checked.
fn f(ok: Bool, done: Bool) {
    if ok {
        a_function_with_a_long_name(the_first_argument, the_second_argument, the_third_argument_xxxxxxxxxxxxxxxxxxxxxxxx)
    } elif first_condition_of_the_test(ok) && second_condition_of_the_test(done) && third_condition_of_the_test(ok) && more(ok) {
        return
    }
    if ok {
        println("a string longer than the width on a line of its own too is never broken, and the call around it is not broken either")
    } elif first_condition_of_the_test(ok) && second_condition_of_the_test(done) && third_condition_of_the_test(ok) && more(ok) {
        return
    }
    let r = xs.map((x) -> { return a_function_with_a_long_name(the_first_argument, the_second_argument, the_third_argument_xxxxxxxxxx) }).first_of_them(the_first_argument_of_first, the_second_argument_of_first)
}
