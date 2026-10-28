// Consumption: a written `!p` moves the argument, and the IR flow pass
// reports a later use [flow-use-after-move]; branches, loops and `when` arms
// merge what they consumed. Each program is its own: they share names.

//--- use_after_consume_is_an_error
// [deduce-consume] A written `!p` consumes the identifier argument and later
// uses are errors — the IR flow pass's, naming the move
// [flow-use-after-move]; reassignment revives it; kept
// parameters are unaffected. Consumption on an always-exiting branch does not
// leak past the branch. A Copy scalar is the exception [copy-scalar-free]:
// moving one is a copy, so the same program over `Int`s compiles.
fn add(a: Str, b: Str) -> Str => !a, !b {
    return "${a}${b}"
}

fn main() [use] {
    use StdConsole
    let a = "1"
    let b = "2"
    let _c = a.add(b)
    //~ error[flow-use-after-move:consumed]
    println("${a}")
}

//--- use_after_consume_over_ints_is_a_copy
// [copy-scalar-free] The same over `Int`s: moving one is a copy, and the
// written `!a, !b` only warns that it changes nothing.
//~ warning[copy-scalar-free:moved-scalar]
//~ warning[copy-scalar-free:moved-scalar]
fn add(a: Int, b: Int) -> Int => !a, !b {
    return a + b
}

fn main() [use] {
    use StdConsole
    let a = 1
    let b = 2
    let _c = a.add(b)
    println("${a}")
}

//--- reassignment_revives_and_kept_parameters_stay
// Reassignment revives the variable; kept parameters (`[a]`) are
// never consumed; consumption inside an always-exiting `if` branch
// never reaches the code after the `if`.
fn add(a: Int, b: Int) -> Int {
    return a + b
}

fn double(a: Int) -> Int => a {
    return a + a
}

fn main() [use] {
    use StdConsole
    let a = 1
    let b = 2
    let _c = a.add(b)
    a = 5
    println("${a}")
    let _d = double(a)
    println("${a}")
    let n = 0
    let _found = while n < 5 {
        if n == 3 {
            break add(n, 10)
        }
        n++
        n
    }
}

//--- consumption_in_a_fall_through_branch_is_conservative
// Consumption in a *fall-through* branch is conservative: the value
// is maybe-moved after the `if`, so using it is an error.
fn consume(text: Str) -> None => !text {
}

fn main() [use] {
    use StdConsole
    let s = "x"
    let flag = true
    if flag {
        consume(s)
    }
    //~ error[flow-use-after-move:consumed]
    println("${s}")
}

//--- inferred_moves_consume_arguments
// [deduce-consume] A *declared* consuming parameter is enforced like an
// inferred one used to be (second checking round): a callee that consumes
// its parameter moves it, so the caller's argument is consumed with the
// annotation in the signature.
//
// NOTE (deduction-defaults change): deductions are no longer inferred from
// the body, so `give_back`'s move (it returns `list`) has to be declared
// with `=> !list` — it can no longer be inferred from the `return list`
// alone. The test's point (a parameter's move at the callee reaches the
// caller's argument with no annotation *at the call site*) still holds.
fn give_back(list: List<Str>) -> List<Str>
=> !list {
    return list
}

fn main() [use] {
    use StdConsole
    let strings = List("a", "b")
    give_back(strings)
    //~ error[flow-use-after-move:consumed]
    println("${strings.size()}")
}

//--- rebinding_through_the_return_value_keeps_it_usable
// Rebinding through the return value keeps it usable.
fn give_back(list: List<Str>) -> List<Str>
=> !list {
    return list
}

fn main() [use] {
    use StdConsole
    let strings: List<Str> = List("a", "b")
    strings = give_back(strings)
    println("${strings.size()}")
}

//--- consumption_survives_narrowing_and_loop_back_edges
// [deduce-consume] Flow-analysis edge cases: consuming an `is`-narrowed
// variable inside its own narrowed branch survives the narrowing restore,
// and loop bodies are re-checked with their exit state so back-edge flows
// (use early, consume late) surface. Consume-then-revive per iteration
// stays clean.
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }
}

fn consume(strings: List<Str>) -> None => !strings {
}

fn main() [use] {
    use StdConsole
    let strings = List("a", "b")
    if strings is NonEmpty {
        consume(strings)
    }
    // The consumption in the `is`-narrowed branch reaches the code after
    // the `if`...
    //~ error[flow-use-after-move:consumed]
    println("${strings.size()}")
    let s = List("x")
    let i = 0
    while i < 3 {
        // ...and the loop's second pass flags the use-before-consume on the
        // back edge.
        //~ error[flow-use-after-move:consumed]
        println("${s.size()}")
        //~ error[flow-use-after-move:consumed]
        consume(s)
        i++
    }
}

//--- consume_then_revive_in_a_loop_is_clean
// Consume-then-revive inside the body is clean across iterations.
fn consume(strings: List<Str>) -> None => !strings {
}

fn main() [use] {
    use StdConsole
    let s = List("x")
    let i = 0
    while i < 3 {
        consume(s)
        s = List("y")
        i++
    }
    println("${s.size()}")
}

//--- when_branches_merge_consumption
// [deduce-consume] `when` branches merge like `if` branches: consumption
// of the *subject* inside an arm survives the arm's narrowing restore and
// reaches the code after the `when` (any-fall-through-path rule), while a
// consuming arm that always exits contributes nothing.
// `Ok`/`Err` and their constructors come from `std.core.result`.
fn consume(v: Ok Str) -> None => !v {
}

fn main() [use] {
    use StdConsole
    let result: Ok Str | Err Str = ok("x")
    when result {
        is Ok {
            consume(result)
        }
        is Err {
        }
    }
    // Consumed in one fall-through arm -> unusable after the `when`.
    //~ error[flow-use-after-move:consumed]
    println("${result}")
}

//--- when_arm_that_consumes_and_exits_contributes_nothing
// The consuming arm always exits -> usable after the `when`.
fn consume(v: Ok Str) -> None => !v {
}

fn main() [use] {
    use StdConsole
    let result: Ok Str | Err Str = ok("x")
    when result {
        is Ok {
            consume(result)
            return
        }
        is Err {
        }
    }
    println("${result}")
}

//--- if_branch_merge_matrix_both_consume
// [deduce-consume] The `if`/`else` merge matrix: consumed on every
// fall-through path, consumed on the only fall-through path (other exits),
// and consumed on an always-exiting path only (clean after).
// Both branches consume -> consumed after.
fn consume(strings: List<Str>) -> None => !strings {
}

fn both(flag: Bool) {
    let a = List("a")
    if flag {
        consume(a)
    } else {
        consume(a)
    }
    //~ error[flow-use-after-move:consumed]
    let _n = a.size()
}

//--- if_branch_merge_matrix_one_exits
// One branch consumes, the other exits: the only fall-through path
// consumed it -> consumed after.
fn consume(strings: List<Str>) -> None => !strings {
}

fn one_exits(flag: Bool) {
    let b = List("b")
    if flag {
        consume(b)
    } else {
        return
    }
    //~ error[flow-use-after-move:consumed]
    let _n = b.size()
}

//--- if_branch_merge_matrix_consuming_branch_exits
// The consuming branch always exits -> clean after.
fn consume(strings: List<Str>) -> None => !strings {
}

fn consuming_exits(flag: Bool) {
    let c = List("c")
    if flag {
        consume(c)
        return
    }
    let _n = c.size()
}

//--- for_loop_back_edge
// [deduce-consume] `for` loops re-check their body with the exit state
// like `while` loops do: the back edge surfaces use-early/consume-late.
fn consume(strings: List<Str>) -> None => !strings {
}

fn main() [use] {
    use StdConsole
    let s = List("x")
    for _i in List(1, 2, 3).iter() {
        //~ error[flow-use-after-move:consumed]
        println("${s.size()}")
        //~ error[flow-use-after-move:consumed]
        consume(s)
    }
}

//--- std_add_consumes_its_element
// [decl-explicit] std's `add` takes ownership of the element: the list
// owns it now. Before bodyless declarations became explicit this was
// inferred as *kept*, so the program below compiled on Kotlin and was
// rejected by rustc — a backend divergence the checker now catches.
struct Handle {
    id: Int
}

fn main() [use] -> None {
    use StdConsole
    let xs = Mut List(Handle {id: 1})
    let h = Handle {id: 2}
    add(xs, h)
    //~ error[flow-use-after-move:consumed]
    println("${h.id}")
}
