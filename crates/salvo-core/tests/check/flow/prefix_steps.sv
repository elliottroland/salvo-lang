// [is-narrow-step] [type-query] [type-assert] An `is` test of a prefix step
// (`++i is Idx(s)`) tests the stepped variable's new value, so on success it
// narrows the variable; the step happens either way, so where the test
// failed the variable is the stepped `Int`. A postfix step's value is the old
// one, which the variable no longer holds: nothing is narrowed.

fn at(s: Str, i: Idx(s) Int) [] -> Char => s, i {
    return char_at(s, (i: Int)) ?: unreachable!()
}

// Inside the body the index is claimed; after the loop (the test failed, or
// the loop never ran) it is the stepped `Int`.
fn a_while_over_a_prefix_step(s: Str) [] -> None => s {
    let i = 0 as! Idx(s)
    while ++i is Idx(s) {
        //~ type[Idx(s) Int]
        type?(i)
        type!(i: Idx(s) Int)
        let _c = at(s, i)
    }
    //~ type[Int]
    type?(i)
    type!(i: Int)
}

// The claim is the test's, made again each time round: the body's own step
// drops it, and the next test gives it back, never the last iteration's.
fn the_claim_is_made_again_each_time(s: Str) [] -> None => s {
    let i = -1
    while ++i is Idx(s) {
        //~ type[Idx(s) Int]
        type?(i)
        i++
        //~ type[Int]
        type?(i)
        //~ error[call-resolve:no-overload]
        let _c = at(s, i)
    }
    //~ type[Int]
    type?(i)
}

// A body that mutates the list the claim is about strips it; the next test
// makes it again about the list as it is then.
fn a_body_that_mutates_the_claimed_list(xs: Mut List<Int>) [] -> None => xs: Mut {
    let i = -1
    while ++i is Idx(xs) {
        //~ type[Idx(xs) Int]
        type?(i)
        let _gone = remove_last(xs)
        //~ type[Int]
        type?(i)
    }
}

// A `continue` reaches the next test, a `break` what follows the loop: a
// break taken under the claim still meets paths without it there.
fn continue_and_break(s: Str) [] -> None => s {
    let i = -1
    while ++i is Idx(s) {
        if at(s, i) == ' ' {
            continue
        }
        //~ type[Idx(s) Int]
        type?(i)
        if at(s, i) == '"' {
            break
        }
    }
    //~ type[Int]
    type?(i)
}

// `if` and a subject-less `when` read their conditions the same way; the
// `else` is the failed test's, where the step has still happened.
fn if_and_when(s: Str) [] -> None => s {
    let i = 0 as! Idx(s)
    if ++i is Idx(s) {
        //~ type[Idx(s) Int]
        type?(i)
    } else {
        //~ type[Int]
        type?(i)
    }
    //~ type[Int]
    type?(i)
    when {
        --i is Idx(s) {
            //~ type[Idx(s) Int]
            type?(i)
        }
        else {
            //~ type[Int]
            type?(i)
        }
    }
    // The guard idiom: past a failed test that leaves, the claim holds.
    if !(++i is Idx(s)) {
        return
    }
    //~ type[Idx(s) Int]
    type?(i)
}

// `elif` and `assert!` too.
fn elif_and_assert(s: Str, c: Bool) [] -> None => s {
    let i = -1
    if c {
        //~ type[Int]
        type?(i)
    } elif ++i is Idx(s) {
        //~ type[Idx(s) Int]
        type?(i)
    }
    assert!(++i is Idx(s))
    //~ type[Idx(s) Int]
    type?(i)
}

// `&&` carries the claim to its right side and into the branch; `||` does
// not, since its right side runs only when the test failed.
fn and_and_or(s: Str, c: Bool) [] -> None => s {
    let i = -1
    if ++i is Idx(s) && at(s, i) == 'a' {
        //~ type[Idx(s) Int]
        type?(i)
    }
    if ++i is Idx(s) && c {
        //~ type[Idx(s) Int]
        type?(i)
    }
    if ++i is Idx(s) || c {
        //~ type[Int]
        type?(i)
    }
    //~ type[Int]
    type?(i)
}

// A step on the right of `&&` moves the index the left side claimed: the
// claim does not survive it, in the branch or after.
fn a_step_after_the_claim(s: Str, n: Int) [] -> None => s, n {
    let i = 0
    if i is Idx(s) && i++ < n {
        //~ type[Int]
        type?(i)
    }
    //~ type[Int]
    type?(i)
    if ++i is Idx(s) && ++i < n {
        //~ type[Int]
        type?(i)
    }
    // And on the right of `||`, whose `else` both sides reached.
    let j = 0
    if !(j is Idx(s)) || j++ > n {
        return
    }
    //~ type[Int]
    type?(j)
}

// A right side may be skipped: where it wrote a place the left side
// narrowed, what follows the condition has neither the left side's claim
// nor the claims the write dropped. Here the mutation of `ys` drops only
// the claim about `ys`, and the `else` is reached when `i is Idx(xs)`
// failed.
fn a_skipped_right_side(xs: List<Int>, ys: Mut List<Int>, p: Slot) [] -> None => xs, ys: Mut, p {
    let i = 5
    assert!(i is Idx(ys))
    if i is Idx(xs) && remove_last(ys) is Int {
        //~ type[Idx(xs) Int]
        type?(i)
    } else {
        //~ type[Int]
        type?(i)
    }
    let j = 5
    assert!(j is Idx(ys))
    if !(j is Idx(xs)) || remove_last(ys) is Int {
        //~ type[Int]
        type?(j)
    } else {
        //~ type[Idx(xs) Int]
        type?(j)
    }
    assert!(p.i is Idx(ys))
    if p.i is Idx(xs) && remove_last(ys) is Int {
        //~ type[Idx(xs) Int]
        type?(p.i)
    } else {
        //~ type[Int]
        type?(p.i)
    }
}

struct Slot { i: Int }

// A claim on an arm the left side picked: the join cannot see it, so the
// state after a skipped right side that wrote the variable is the state
// before without its claims.
fn a_claim_on_an_arm(o: Int?, ys: Mut List<Int>) [] -> None => o, ys: Mut {
    let q = o ?: 0
    assert!(q is Idx(ys))
    let r: (Idx(ys) Int)? = q
    if r is Int && remove_last(ys) is Int {
        //~ type[Int]
        type?(r)
    } else {
        if r is Int {
            //~ type[Int]
            type?(r)
        }
    }
}

// A field fact an `elif` condition invalidates: no earlier fact about the
// field comes back in its branch.
struct Three canbe Mut { v: Int | Str | Bool }

fn bump(t: Mut Three) [] -> Bool => t: Mut {
    t.v = "s"
    return true
}

fn an_invalidated_field_fact(t: Mut Three) [] -> None => t: Mut {
    if t.v is Str {
        return
    } elif t.v is Bool {
        return
    } elif bump(t) {
        //~ type[Int | Str | Bool]
        type?(t.v)
    }
}

// An `elif` condition runs after the earlier ones failed, in their facts: a
// step there ends them, for its branch, the later ones and past a guard.
fn a_step_in_an_elif(s: Str, n: Int) [] -> None => s, n {
    let i = 0
    if !(i is Idx(s)) {
        return
    } elif ++i > n {
        //~ type[Int]
        type?(i)
        return
    }
    //~ type[Int]
    type?(i)
    let j = 0
    if !(j is Idx(s)) {
        return
    } elif j > n {
        //~ type[Idx(s) Int]
        type?(j)
        return
    }
    //~ type[Idx(s) Int]
    type?(j)
}

// A mutation on the right ends the claim about the list it mutates; a
// field path the right side steps loses its fact too.
fn writes_on_the_right(xs: Mut List<Int>, scan: Mut Scan, n: Int) [] -> None => xs: Mut, scan: Mut, n {
    let i = 0
    if i is Idx(xs) && remove_last(xs) is Int {
        //~ type[Int]
        type?(i)
    }
    if scan.pos is Idx(scan.str) && ++scan.pos < n {
        //~ type[Int]
        type?(scan.pos)
    }
}

// A binding the right side steps is handed on as the step left it.
fn a_binding_stepped_on_the_right(s: Str, o: Int?, n: Int) [] -> None => s, o, n {
    let q = o ?: 0
    assert!(q is Idx(s))
    let r: (Idx(s) Int)? = q
    if r is Int k && ++k < n {
        //~ type[Int]
        type?(k)
        let _u = k
    }
    if r is Int k && n > 0 {
        //~ type[Idx(s) Int]
        type?(k)
        let _u = k
    }
}

// A local stored as an optional and narrowed to its `Int` keeps that
// storage when its step is tested.
fn an_optional_local(s: Str, o: Int?) [] -> None => s, o {
    let x = o
    if x is Int {
        while ++x is Idx(s) {
            //~ type[Idx(s) Int]
            type?(x)
            let _c = at(s, x)
        }
        //~ type[Int]
        type?(x)
    }
}

// A `while`'s `else` runs when the first test failed: the step happened.
fn a_while_else(s: Str) [] -> None => s {
    let i = -1
    while ++i is Idx(s) {
        //~ type[Idx(s) Int]
        type?(i)
    } else {
        //~ type[Int]
        type?(i)
    }
}

// A postfix step's value is the old index: the variable has moved on, so it
// is not narrowed.
fn a_postfix_step_does_not_narrow(s: Str) [] -> None => s {
    let i = -1
    while i++ is Idx(s) {
        //~ type[Int]
        type?(i)
        //~ error[call-resolve:no-overload]
        let _c = at(s, i)
    }
    if i-- is Idx(s) {
        //~ type[Int]
        type?(i)
    }
}

// A claim about another string does not hold.
fn a_claim_about_another_string(s: Str, t: Str) [] -> None => s, t {
    let i = -1
    while ++i is Idx(s) {
        type!(i: Idx(s) Int)
        //~ error[type-assert:mismatch]
        type!(i: Idx(t) Int)
        //~ error[call-resolve:no-overload]
        let _c = at(t, i)
    }
}

// A field path narrows as a variable does.
struct Scan canbe Mut {
    str: proj Str
    pos: Int
}

fn a_field_path(scan: Mut Scan) [] -> None => scan: Mut {
    while ++scan.pos is Idx(scan.str) {
        //~ type[Idx(scan.str) Int]
        type?(scan.pos)
        type!(scan.pos: Idx(scan.str) Int)
        let _c = at(scan.str, scan.pos)
    }
    //~ type[Int]
    type?(scan.pos)
}

// The demo's shape: a cursor whose index is claimed, destructured, and
// stepped by the loop's test.
struct Cursor canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

fn next_word(cursor: Mut Cursor) [] -> Int => cursor {
    let { str, pos } = cursor
    let n = 0
    while ++pos is Idx(str) {
        //~ type[Idx(str) Int]
        type?(pos)
        type!(pos: Idx(cursor.str) Int)
        if at(str, pos) == ' ' {
            break
        }
        n++
    }
    return n
}

// The field itself keeps its declared claim, so a step of it is refused, as
// `cursor.pos = cursor.pos + 1` is, test or no test.
fn the_claimed_field_is_not_stepped(cursor: Mut Cursor) [] -> None => cursor: Mut {
    //~ error[var-no-widen:assign-mismatch]
    if ++cursor.pos is Idx(cursor.str) {
        let _c = at(cursor.str, cursor.pos)
    }
}

fn main() {}
