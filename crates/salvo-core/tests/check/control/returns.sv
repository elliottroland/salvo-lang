// [fn-must-return] A fn with a non-`None` return type must return on all
// paths; `if/else` and `when` where every branch returns count, yield-based
// iterator fns are exempt.

//--- missing_return_is_an_error
//~ error[fn-must-return:missing-return]
fn sign(x: Int) -> Int {
    if x < 0 {
        return -1
    }
}

//--- every_path_returning_is_clean
// All-paths-return via if/else, a `yield fn` (its body produces elements,
// not a return value) and None-returning fns are fine.
fn sign(x: Int) -> Int {
    if x < 0 {
        return -1
    } else {
        return 1
    }
}
struct Nums {
    limit: Int
}
iter fn iter(n: Nums) -> Emitted Int | Finished {
    state NumsIter {
        at: Int = 0
    }
    if at >= n.limit {
        return finished()
    }
    at = at + 1
    return emitted(copy(at))
}
fn nothing(x: Int) {
    let _y = x
}
