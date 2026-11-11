// [qual-depend-field] [type-query] [type-assert] [test-data] A join of paths
// keeps one claim per fact. A `pos` destructured beside its `str` reads
// `Idx(str)`, the field read `a.pos` reads `Idx(a.str)`: one claim under two
// spellings, so the join of the two is the one claim, not a union of two
// (which matched each branch's value twice and refused both). A claim not
// every path holds is dropped (user decision 2026-11-11): two claims about
// different values, or a claim and none, join to the plain `Int`, which is
// what using either would have to test for anyway. `//~ type[T]` is judged
// as `type!` judges: a union of the two spellings is not `Idx(a.str) Int`,
// so these pass only when the join merged.

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

// Two cursors are two values: no claim holds on both paths, so the join is
// the plain `Int`, as the variable's state over the same paths is, and an
// `Int` parameter takes it.
fn two_cursors_do_not_merge(a: Cursor, b: Cursor, c: Bool) [] -> Int => a, b {
    let _p = if c { a.pos } else { b.pos }
    //~ type[Int]
    type?(_p)
    //~ error[type-assert:mismatch]
    type!(_p: Idx(a.str) Int)
    let v = a.pos
    if c {
        v = b.pos
    }
    //~ type[Int]
    type?(v)
    return plain(_p) + v
}

fn plain(i: Int) [] -> Int => i {
    return i
}

// Two cursors each destructured: the same spelling, two values, no claim.
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
    //~ type[Int]
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

// A claim about a value nothing in scope at the join can name is dropped
// there (user decision 2026-11-11): no call could use it, so the join is the
// plain type, not a claim spelled with a local that has ended.
fn branch_locals_drop(c: Bool) [] -> Int {
    let p = if c {
        let s = "abc"
        0 as! Idx(s)
    } else {
        let t = "xyz"
        0 as! Idx(t)
    }
    //~ type[Int]
    type?(p)
    type!(p: Int)
    return p
}

// The same spelling in both branches is two values, both ended; a later
// `s` is a third, and the join is not about it.
fn one_spelling_both_ended(c: Bool) [] -> Int {
    let p = if c {
        let s = "abc"
        0 as! Idx(s)
    } else {
        let s = "abc"
        0 as! Idx(s)
    }
    let s = "other"
    let _n = size(s)
    //~ type[Int]
    type?(p)
    //~ error[type-assert:mismatch]
    type!(p: Idx(s) Int)
    return p
}

// One branch's claim is still named, the other's is not: the other arm has
// no claim, so neither has the join, in either order.
fn reachable_and_ended(a: Cursor, c: Bool) [] -> Int => a {
    let p = if c {
        a.pos
    } else {
        let s = "abc"
        0 as! Idx(s)
    }
    //~ type[Int]
    type?(p)
    let q = if c {
        let s = "abc"
        0 as! Idx(s)
    } else {
        a.pos
    }
    //~ type[Int]
    type?(q)
    return p + q
}

// Inside an optional, a list and a tuple, the dropped claim leaves the type
// around it whole.
fn nested_positions_drop(a: Cursor, c: Bool) [] -> Int => a {
    let o = if c {
        let s = "abc"
        let x: (Idx(s) Int)? = 0 as! Idx(s)
        x
    } else {
        a.pos
    }
    //~ type[Int?]
    type?(o)
    let xs = if c {
        let s = "abc"
        List(0 as! Idx(s))
    } else {
        let t = "xyz"
        List(0 as! Idx(t))
    }
    //~ type[NonEmpty List<Int>]
    type?(xs)
    let ys = if c {
        List(a.pos)
    } else {
        let s = "abc"
        List(0 as! Idx(s))
    }
    //~ type[NonEmpty List<Int>]
    type?(ys)
    let _pair = if c {
        let s = "abc"
        (size(s), 0 as! Idx(s))
    } else {
        let s = "xyz"
        (size(s), 0 as! Idx(s))
    }
    //~ type[(Int, Int)]
    type?(_pair)
    return first(xs) + first(ys) + (o ?: 0)
}

// Three ways, two claims still named and one ended: the ended path has no
// claim, so the join has none, whichever arm comes first.
fn three_ways_one_ended(a: Cursor, b: Cursor, c: Bool, d: Bool) [] -> Int => a, b {
    let p = when {
        c { a.pos }
        d { b.pos }
        else {
            let s = "abc"
            0 as! Idx(s)
        }
    }
    //~ type[Int]
    type?(p)
    let q = when {
        c {
            let s = "abc"
            0 as! Idx(s)
        }
        d { a.pos }
        else { b.pos }
    }
    //~ type[Int]
    type?(q)
    return p + q
}

// The facts kept after a guard on every path drop the same way.
fn guard_facts_drop(c: Bool, i: Int) [] -> Int {
    if c {
        let s = "abc"
        if !(i is Idx(s)) {
            return 0
        }
    } else {
        let t = "xyz"
        if !(i is Idx(t)) {
            return 1
        }
    }
    //~ type[Int]
    type?(i)
    return i
}

// A variable's state after branches drops the same way.
fn assignments_drop(c: Bool) [] -> Int {
    let v = 0
    if c {
        let s = "abc"
        v = 0 as! Idx(s)
    } else {
        let t = "xyz"
        v = 0 as! Idx(t)
    }
    //~ type[Int]
    type?(v)
    return v
}

// A claim spelled with a branch's local but about a value still in scope is
// kept, respelled with a name that reaches it.
fn reachable_claim_kept(c: Bool) [] -> Char {
    let s = "abc"
    let p = if c {
        let t = s
        0 as! Idx(t)
    } else {
        let u = s
        0 as! Idx(u)
    }
    //~ type[Idx(s) Int]
    type?(p)
    return at(s, p)
}

// A claim on one path and none on the other: the join is the plain `Int`,
// in either order (it was `Idx(a.str) Int | Int`, and the `a.pos` branch was
// refused as matching both arms).
fn claim_and_none(a: Cursor, c: Bool) [] -> Int => a {
    let p = if c { a.pos } else { 0 }
    //~ type[Int]
    type?(p)
    type!(p: Int)
    let q = if c { 0 } else { a.pos }
    //~ type[Int]
    type?(q)
    let r = when {
        c { 0 }
        else {
            let { str, pos } = a
            let _n = size(str)
            pos
        }
    }
    //~ type[Int]
    type?(r)
    return plain(p) + q + r
}

// Three paths, grouped every way: the claim survives only where every path
// holds it, so each grouping and each order gives the same join.
fn three_paths_any_grouping(a: Cursor, b: Cursor, c: Bool, d: Bool) [] -> Int => a, b {
    let p1 = if c {
        a.pos
    } else {
        if d {
            b.pos
        } else {
            let s = "abc"
            0 as! Idx(s)
        }
    }
    //~ type[Int]
    type?(p1)
    let p2 = if c {
        if d { a.pos } else { b.pos }
    } else {
        let s = "abc"
        0 as! Idx(s)
    }
    //~ type[Int]
    type?(p2)
    let p3 = if c {
        let s = "abc"
        0 as! Idx(s)
    } else {
        if d { b.pos } else { a.pos }
    }
    //~ type[Int]
    type?(p3)
    let p4 = if c {
        a.pos
    } else {
        if d { a.pos } else { b.pos }
    }
    //~ type[Int]
    type?(p4)
    let p5 = if c {
        if d { a.pos } else { a.pos }
    } else {
        b.pos
    }
    //~ type[Int]
    type?(p5)
    let p6 = when {
        c { b.pos }
        d { 0 }
        else { a.pos }
    }
    //~ type[Int]
    type?(p6)
    let p7 = if c {
        if d { 0 } else { a.pos }
    } else {
        a.pos
    }
    //~ type[Int]
    type?(p7)
    return p1 + p2 + p3 + p4 + p5 + p6 + p7
}

// Three paths that all hold one claim, under any spelling and any grouping,
// keep it.
fn three_paths_one_claim(a: Cursor, c: Bool, d: Bool) [] -> Char => a {
    let p = if c {
        a.pos
    } else {
        if d {
            let { str, pos } = a
            let _n = size(str)
            pos
        } else {
            a.pos
        }
    }
    //~ type[Idx(a.str) Int]
    type?(p)
    let q = if c {
        if d { a.pos } else { a.pos }
    } else {
        let { pos } = a
        pos
    }
    //~ type[Idx(a.str) Int]
    type?(q)
    let _w = at(a.str, q)
    return at(a.str, p)
}

// Arms of different shapes are not compared: the lone `Int` arm keeps its
// claim beside a `Str`.
fn different_shapes_keep(a: Cursor, c: Bool) [] -> Int => a {
    let _p = if c { a.pos } else { "none" }
    //~ type[Idx(a.str) Int | Str]
    type?(_p)
    let _q = if c { a.pos } else { None }
    //~ type[(Idx(a.str) Int)?]
    type?(_q)
    return 0
}

// The facts after a guard on every path: two claims about two values keep
// neither, one claim on both keeps it.
fn guard_facts_two_claims(a: Cursor, b: Cursor, c: Bool, i: Int) [] -> Int => a, b {
    if c {
        if !(i is Idx(a.str)) {
            return 0
        }
    } else {
        if !(i is Idx(b.str)) {
            return 1
        }
    }
    //~ type[Int]
    type?(i)
    return i
}

fn guard_facts_one_claim(a: Cursor, c: Bool, i: Int) [] -> Char => a {
    if c {
        if !(i is Idx(a.str)) {
            return 'x'
        }
    } else {
        let { str } = a
        if !(i is Idx(str)) {
            return 'y'
        }
    }
    //~ type[Idx(a.str) Int]
    type?(i)
    return at(a.str, i)
}

// An `||` of tests about two values narrows to neither claim.
fn or_two_values(a: Cursor, b: Cursor, i: Int) [] -> Int => a, b {
    if i is Idx(a.str) || i is Idx(b.str) {
        //~ type[Int]
        type?(i)
        return plain(i)
    }
    return 0
}

// A loop's value joins its paths the same way: a `break` with one claim and
// an `else` with another keep neither.
fn loop_value_joins(a: Cursor, b: Cursor, n: Int) [] -> Int => a, b {
    let i = 0
    let p = while i++ < n {
        if i == 2 {
            break a.pos
        }
        b.pos
    } else {
        a.pos
    }
    //~ type[Int]
    type?(p)
    return plain(p)
}

// Inside a type argument, two claims both still named keep neither.
fn lists_of_two_values(a: Cursor, b: Cursor, c: Bool) [] -> Int => a, b {
    let xs = if c { List(a.pos) } else { List(b.pos) }
    //~ type[NonEmpty List<Int>]
    type?(xs)
    let ys = if c { List(a.pos) } else { List(a.pos) }
    //~ type[NonEmpty List<Idx(a.str) Int>]
    type?(ys)
    return plain(first(xs)) + first(ys)
}

// A claim on constants is a claim too: kept where every path holds it,
// dropped where one does not.
fn constant_claims(c: Bool, n: InRange(0, 100) Int, m: InRange(0, 100) Int, k: InRange(0, 5) Int) [] -> Int => n, m, k {
    let p = if c { n } else { 0 }
    //~ type[Int]
    type?(p)
    let q = if c { n } else { m }
    //~ type[InRange(0, 100) Int]
    type?(q)
    let r = if c { n } else { k }
    //~ type[Int]
    type?(r)
    return plain(p) + q + r
}

// A written union of two claims is not a join: a parameter declared so keeps
// both arms, a value with either claim is passed to it, and a local takes it
// whole.
fn written_union(a: Cursor, b: Cursor, x: Idx(a.str) Int | Idx(b.str) Int) [] -> Int => a, b, x {
    //~ type[Idx(a.str) Int | Idx(b.str) Int]
    type?(x)
    let _y = x
    //~ type[Idx(a.str) Int | Idx(b.str) Int]
    type?(_y)
    return 0
}

fn forward_written_union(a: Cursor, b: Cursor, x: Idx(a.str) Int | Idx(b.str) Int) [] -> Int => a, b, x {
    return written_union(a, b, x)
}

fn pass_written_union(a: Cursor, b: Cursor, c: Bool) [] -> Int => a, b {
    let _i = written_union(a, b, a.pos)
    // The plain join fits a written `Int`: the claims go.
    let z: Int = if c { a.pos } else { b.pos }
    //~ type[Int]
    type?(z)
    // Arms of different shapes, each fitting its arm of the written type.
    let _o: (Idx(a.str) Int)? = if c { a.pos } else { None }
    //~ type[(Idx(a.str) Int)?]
    type?(_o)
    return written_union(a, b, b.pos) + z
}

// A join never reads the type it is checked against (user decision
// 2026-11-11): the join of two claims is the plain `Int`, which fits no arm
// of a written union of the claims, however the paths nest.
fn join_under_written_union(a: Cursor, b: Cursor, c: Bool, d: Bool) [] -> Int => a, b {
    //~ error[type-union:no-arm]
    //~ error[var-let-infer:annotation-mismatch]
    let _y: Idx(a.str) Int | Idx(b.str) Int = if c { a.pos } else { b.pos }
    //~ error[type-union:no-arm]
    //~ error[var-let-infer:annotation-mismatch]
    let _w: Idx(a.str) Int | Idx(b.str) Int = when {
        c { b.pos }
        d { a.pos }
        else { b.pos }
    }
    //~ error[type-union:no-arm]
    //~ error[var-let-infer:annotation-mismatch]
    let _n: Idx(a.str) Int | Idx(b.str) Int = if c { if d { a.pos } else { b.pos } } else { a.pos }
    //~ error[type-union:no-arm]
    //~ error[var-let-infer:annotation-mismatch]
    let _v: Idx(a.str) Int | Idx(b.str) Int = if c { a.pos } else { 0 }
    return 0
}

fn main() {}
