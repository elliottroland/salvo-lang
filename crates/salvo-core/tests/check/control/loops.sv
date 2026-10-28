// Loop control the checker refuses.

//--- break_outside_a_loop_is_an_error
// [loop-while-value]
fn f() -> None {
    //~ error[expr-escape:break-outside-loop]
    break
}
