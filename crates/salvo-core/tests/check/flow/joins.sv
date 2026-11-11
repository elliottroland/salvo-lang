// [qual-depend-field] [type-query] [type-assert] [test-data] A join of paths
// keeps one claim per fact. A `pos` destructured beside its `str` reads
// `Idx(str)`, the field read `a.pos` reads `Idx(a.str)`: one claim under two
// spellings, so the join of the two is the one claim, not a union of two
// (which matched each branch's value twice and refused both). Two claims
// about different values stay two. `//~ type[T]` is judged as `type!`
// judges: a union of the two spellings is not `Idx(a.str) Int`, so these
// pass only when the join merged.

struct Cursor canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

// Only the claimed overload: a call is accepted only with the claim.
fn at(str: Str, index: Idx(str) Int) [] -> Char => str, index {
    return char_at(str, index)
}

// The defect's repro: a destructured `pos`, then the field read.
fn destructured_then_read(a: Cursor, c: Bool) [] -> Char => a {
    let p = if c {
        let { str, pos } = a
        let _n = size(str)
        pos
    } else {
        a.pos
    }
    //~ type[Idx(a.str) Int]
    type?(p)
    type!(p: Idx(a.str) Int)
    return at(a.str, p)
}

// The other order.
fn read_then_destructured(a: Cursor, c: Bool) [] -> Char => a {
    let p = if c {
        a.pos
    } else {
        let { str, pos } = a
        let _n = size(str)
        pos
    }
    //~ type[Idx(a.str) Int]
    type?(p)
    return at(a.str, p)
}

// Three ways: a `when`'s arms, two spellings and a third path.
fn three_ways(a: Cursor, c: Bool, d: Bool) [] -> Char => a {
    let p = when {
        c {
            let { str, pos } = a
            let _n = size(str)
            pos
        }
        d { a.pos }
        else {
            let { pos } = a
            pos
        }
    }
    //~ type[Idx(a.str) Int]
    type?(p)
    type!(p: Idx(a.str) Int)
    return at(a.str, p)
}

// Two cursors are two values: their claims stay two, and neither holds of
// the join.
fn two_cursors_do_not_merge(a: Cursor, b: Cursor, c: Bool) [] -> Int => a, b {
    let _p = if c { a.pos } else { b.pos }
    //~ type[Idx(a.str) Int | Idx(b.str) Int]
    type?(_p)
    //~ error[type-assert:mismatch]
    type!(_p: Idx(a.str) Int)
    let v = a.pos
    if c {
        v = b.pos
    }
    //~ type[Int]
    type?(v)
    return v
}

// Two cursors each destructured: the same spelling, two values.
fn one_spelling_two_values(a: Cursor, b: Cursor, c: Bool) [] -> Int => a, b {
    let p = if c {
        let { str, pos } = a
        let _n = size(str)
        pos
    } else {
        let { str, pos } = b
        let _n = size(str)
        pos
    }
    //~ type[Idx(a.str) Int | Idx(b.str) Int]
    type?(p)
    return p
}

// Every branch's spelling names a local that ended with its branch: the
// join is about `a.str`, and another value's `str` in scope afterwards is
// not it.
fn spellings_out_of_scope(a: Cursor, b: Cursor, c: Bool) [] -> Char => a, b {
    let p = if c {
        let { str, pos } = a
        let _n = size(str)
        pos
    } else {
        let { str, pos } = a
        let _n = size(str)
        pos
    }
    //~ type[Idx(a.str) Int]
    type?(p)
    let { str, pos } = b
    //~ type[Idx(a.str) Int]
    type?(p)
    //~ error[type-assert:mismatch]
    type!(p: Idx(str) Int)
    let _d = at(str, pos)
    return at(a.str, p)
}

// An `if` statement joins a variable's states, not only values.
fn assignments_join(a: Cursor, c: Bool) [] -> Char => a {
    let v = a.pos
    if c {
        let { str, pos } = a
        let _n = size(str)
        v = pos
    }
    //~ type[Idx(a.str) Int]
    type?(v)
    let _w = at(a.str, v)
    let u = 0
    if c {
        let { str, pos } = a
        let _n = size(str)
        u = pos
    } else {
        u = a.pos
    }
    //~ type[Idx(a.str) Int]
    type?(u)
    return at(a.str, u)
}

// `?:` is the `when` it lowers to.
fn elvis_joins(a: Cursor, o: (Idx(a.str) Int)?) [] -> Char => a {
    let { str, pos } = a
    let e = o ?: pos
    //~ type[Idx(a.str) Int]
    type?(e)
    type!(e: Idx(str) Int)
    return at(str, e)
}

// An `||` of two tests joins what each narrows to.
fn or_joins(a: Cursor, i: Int) [] -> Char => a {
    let { str, pos } = a
    if i is Idx(str) || i is Idx(a.str) {
        //~ type[Idx(a.str) Int]
        type?(i)
        return at(str, i)
    }
    return at(str, pos)
}

// Returns of either spelling meet one declared return type.
fn returns_join(a: Cursor, c: Bool) [] -> Idx(a.str) Int => a {
    if c {
        let { str, pos } = a
        let _n = size(str)
        return pos
    }
    return a.pos
}

// A list literal's elements.
fn list_elements(a: Cursor) [] -> Char => a {
    let { str, pos } = a
    let _xs = List(pos, a.pos)
    //~ type[NonEmpty List<Idx(a.str) Int>]
    type?(_xs)
    let ys = List(a.pos, pos)
    //~ type[NonEmpty List<Idx(str) Int>]
    type?(ys)
    return at(str, first(ys))
}

// A `when` over a union: its arms' values join.
struct Left {}
struct Right {}

fn match_arms(a: Cursor, side: Left | Right) [] -> Char => a {
    let p = when side {
        is Left {
            let { str, pos } = a
            let _n = size(str)
            pos
        }
        is Right { a.pos }
    }
    //~ type[Idx(a.str) Int]
    type?(p)
    return at(a.str, p)
}

fn main() {}
