// [type-query] [type-assert] Types across loops. The checker walks a loop
// body again when the first walk changed what it knows; a `type?` in the
// body reports what the last walk saw, once.

// A condition that narrows the index: inside, it is claimed until the step;
// after the loop, it is what the body left or what came in.
fn a_while_narrowing_the_index(s: Str) [] -> None => s {
    let i = 0
    while i is Idx(s) {
        //~ type[Idx(s) Int]
        type?(i)
        i++
        //~ type[Int]
        type?(i)
    }
    //~ type[Int]
    type?(i)
}

// The first walk sees the claim the index came in with; the second, the
// back edge's stepped index, and only that is reported.
fn the_last_walk_is_reported(s: Str, n: Int) [] -> None => s, n {
    let i = 0 as! Idx(s)
    let k = 0
    while k < n {
        //~ type[Int]
        type?(i)
        i++
        k++
    }
    //~ type[Int]
    type?(i)
}

// A loop that does not change the claim keeps it, inside and after.
fn a_loop_that_keeps_the_claim(s: Str, n: Int) [] -> None => s, n {
    let i = 0 as! Idx(s)
    let k = 0
    while k < n {
        //~ type[Idx(s) Int]
        type?(i)
        k++
    }
    //~ type[Idx(s) Int]
    type?(i)
    type!(i: Idx(s) Int)
    let _ = i
}

// A `continue` path reaches the back edge too: a step on it is seen at the
// top of the next iteration.
fn a_step_before_continue(s: Str, xs: List<Int>) [] -> None => s, xs {
    let i = 0 as! Idx(s)
    for x in xs {
        //~ type[Int]
        type?(i)
        if x > 0 {
            i++
            continue
        }
    }
}

// A claim given back before the `continue` is kept on every path.
fn a_claim_given_back_before_continue(s: Str, n: Int) [] -> None => s, n {
    let i = 0 as! Idx(s)
    let k = 0
    while k < n {
        k++
        //~ type[Idx(s) Int]
        type?(i)
        if k > 2 {
            i++
            //~ type[Int]
            type?(i)
            assert!(i is Idx(s))
            continue
        }
    }
    //~ type[Idx(s) Int]
    type?(i)
}

// A loop may run no times: what its body asserted does not hold after it.
fn after_a_for_that_may_not_run(s: Str, xs: List<Int>) [] -> None => s, xs {
    let i = 5
    for _x in xs {
        assert!(i is Idx(s))
        //~ type[Idx(s) Int]
        type?(i)
    }
    //~ type[Int]
    type?(i)
}

// Nested loops: the inner body is walked inside each walk of the outer one,
// and still reported once.
fn nested_loops(s: Str, n: Int) [] -> None => s, n {
    let i = 0 as! Idx(s)
    let k = 0
    while k < n {
        k++
        let j = 0
        while j < n {
            j++
            //~ type[Int]
            type?(i)
        }
        i++
    }
}

// A type assertion's verdict is the last walk's too: the first walk sees the
// claim the index came in with, the last the stepped index.
fn an_assertion_in_a_loop_is_judged_on_the_last_walk(s: Str, n: Int) [] -> None => s, n {
    let i = 0 as! Idx(s)
    let k = 0
    while k < n {
        type!(i: Int)
        //~ error[type-assert:mismatch]
        type!(i: Idx(s) Int)
        i++
        k++
    }
}

// An element of a borrowed list is a borrow.
fn the_elements_of_a_borrowed_list(xs: List<Str>) [] -> None => xs {
    for s in xs {
        //~ type[proj Str]
        type?(s)
        type!(s: proj Str)
        //~ error[type-assert:mismatch]
        type!(s: Str)
        let _ = s
    }
}

fn main() {}
