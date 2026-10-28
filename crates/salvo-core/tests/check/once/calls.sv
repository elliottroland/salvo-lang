// `once` fn values called, looped, and passed on (L7b).

//--- l7b_once_fns
// [fn-once] L7b: `once` on fn types means callable at most once,
// enforced by consumption — a second call, a call in a loop, and a
// call after passing the value on all error; a maybe-call is fine
// ("at most once"). Subtyping is inverted (plain fns fit `once`
// positions, never the reverse), a capture-consuming lambda is legal
// but `once`-typed, and `once` applies only to fn types.
//
// [flow-use-after-move] The uses are the IR flow pass's, naming the
// move: the second call (moved by the first), the loop's back edge, and a
// call after passing the value on (the escape rule).
fn run_once(f: once () -> None) {
    f()
}

fn run_twice_bad(f: once () -> None) {
    f()
    //~ error[flow-use-after-move:consumed]
    f()
}

fn run_in_loop_bad(f: once () -> None) {
    for _i in List(1, 2) {
        //~ error[flow-use-after-move:consumed]
        f()
    }
}

// run_maybe, once_where_once_ok, plain_where_once_ok, not_a_fn are clean.
fn run_maybe(f: once () -> None, flag: Bool) {
    if flag {
        f()
    }
}

fn consume_list(v: List<Int>) -> None => !v {
}

fn once_where_once_ok() {
    let xs = List(1, 2)
    let g = () -> { consume_list(xs) }
    run_once(g)
}

fn plain_where_once_ok() {
    let n = 5
    let g = () -> { let _m = n + 1 }
    run_once(g)
}

fn escape_rule(f: once () -> None) {
    run_once(f)
    //~ error[flow-use-after-move:consumed]
    f()
}

// [fn-once] D6 (user decision 2026-09-12): `once` is valid on any
// type now, so `not_a_fn(x: once Int)` draws no error.
fn not_a_fn(x: once Int) {
}

//--- l7b_a_once_lambda_cannot_go_where_a_plain_fn_is_expected
// A `once` lambda cannot go where a plain fn is expected — a checker
// error, and the fn with it reports no flow diagnostic.
fn run_plain(f: () -> None) {
    f()
    f()
}

fn consume_list(v: List<Int>) -> None => !v {
}

fn once_where_plain_bad() {
    let xs = List(1, 2)
    let g = () -> { consume_list(xs) }
    //~ error[call-resolve:no-overload]
    run_plain(g)
}
