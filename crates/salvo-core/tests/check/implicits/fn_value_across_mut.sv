// A written fn value across a `Mut` difference.

//--- a_written_fn_value_does_not_fit_across_mut
// [fn-erased-variance] A difference the backends render stays invariant
// (option (a), not chosen): a `count(xs: List<Int>)` does not fill a
// `(Mut List<Int>) -> Int`, as an override or by name — `&Vec` is not
// `&mut Vec` — and the messages are the ones it always had.
fn count(xs: List<Int>) [] -> Int => xs { return size(xs) }
fn measure_all(xs: Mut List<Int>, ?measure: (xs: Mut List<Int>) -> Int) [] -> Int => xs: Mut { return measure(xs) }
fn apply_mut(f: (xs: Mut List<Int>) -> Int, xs: Mut List<Int>) [] -> Int => f, xs: Mut { return f(xs) }
fn main() [use] {
    use StdConsole()
    let ys = Mut List(1, 2)
    //~ error[implicit-override:does-not-fit]
    println("${measure_all(ys, measure = count)}")
    //~ error[call-resolve:no-overload]
    println("${apply_mut(count, ys)}")
}
