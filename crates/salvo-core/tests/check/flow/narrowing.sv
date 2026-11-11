// [type-query] [type-assert] What a place's type is, step by step, as the
// checker's flow facts say: `//~ type[T]` asserts the `type` diagnostic a
// `type?(place)` reports, judged as `type!` judges; `type!(place: T)` checks
// the same thing in the program itself, and reports only when it is wrong.

// An `is` test narrows inside the branch, and only there.
fn narrowing_by_is(v: Int | Str) [] -> None => v {
    //~ type[Int | Str]
    type?(v)
    if v is Str {
        //~ type[Str]
        type?(v)
        type!(v: Str)
    }
    //~ type[Str | Int]
    type?(v)
    type!(v: Str | Int)
}

// The guard idiom narrows the rest of the scope.
fn a_guard_narrows_what_follows(v: Int | Str) [] -> None => v {
    if v !is Int {
        return
    }
    //~ type[Int]
    type?(v)
}

// `T?` and `T | None` are one type, in either order.
fn an_optional(o: Int?) [] -> None => o {
    //~ type[None | Int]
    type?(o)
    type!(o: Int?)
    type!(o: None | Int)
    if o is Int {
        //~ type[Int]
        type?(o)
    }
}

// `assert!(x is Q)` narrows for good; `type!(x: T)` checks and narrows
// nothing.
fn an_assertion_narrows(s: Str, i: Int) [] -> None => s, i {
    //~ type[Int]
    type?(i)
    assert!(i is Idx(s))
    //~ type[Idx(s) Int]
    type?(i)
    type!(i: Idx(s) Int)
}

// A step leaves the claim behind; an `is` test gives it back.
fn a_step_drops_the_claim(s: Str) [] -> None => s {
    let i = 0 as! Idx(s)
    //~ type[Idx(s) Int]
    type?(i)
    i++
    //~ type[Int]
    type?(i)
    type!(i: Int)
    if i is Idx(s) {
        //~ type[Idx(s) Int]
        type?(i)
    }
}

// A prefix step's value is the new one, without the claim.
fn a_prefix_step(s: Str) [] -> None => s {
    let i = 0 as! Idx(s)
    let _j = ++i
    //~ type[Int]
    type?(_j)
}

// A branch that steps the tested variable hands no fact on.
fn a_branch_that_steps(s: Str) [] -> None => s {
    let i = 0 as! Idx(s)
    if i is Idx(s) {
        i++
    } else {
        return
    }
    //~ type[Int]
    type?(i)
}

// Two paths with claims of one name about different strings keep neither.
fn paths_with_different_claims(s: Str, t: Str, c: Bool) [] -> None => s, t {
    let i = 0 as! Idx(t)
    let k = 0 as! Idx(s)
    //~ type[Idx(s) Int]
    type?(k)
    if c {
        i = k
    }
    //~ type[Int]
    type?(i)
    let _ = i
}

// A mutation strips every claim about the value mutated.
fn a_claim_lost_by_mutation(xs: Mut List<Int>) [] -> None => xs: Mut {
    let i = 0
    if i is Idx(xs) {
        //~ type[Idx(xs) Int]
        type?(i)
        let _gone = remove_last(xs)
        //~ type[Int]
        type?(i)
    }
    //~ type[Mut List<Int>]
    type?(xs)
    type!(xs: Mut List<Int>)
}

// A call that may mutate a cursor strips the claims read from it.
struct Cursor canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

fn touch(cursor: Mut Cursor) [] -> None => cursor: Mut {
    let _ = cursor
}

fn a_claim_lost_by_a_call(cursor: Mut Cursor) [] -> None => cursor: Mut {
    let i = cursor.pos
    //~ type[Idx(cursor.str) Int]
    type?(i)
    touch(cursor)
    //~ type[Int]
    type?(i)
    let _ = i
}

// A field narrowing is the place's, and a mutation of its claim's subject
// strips it too.
struct Slot { i: Int }

fn a_place_narrowing(xs: Mut List<Int>, p: Slot) [] -> None => xs: Mut, p {
    if p.i is Idx(xs) {
        //~ type[Idx(xs) Int]
        type?(p.i)
        let _gone = remove_last(xs)
        //~ type[Int]
        type?(p.i)
    }
}

fn main() {}
