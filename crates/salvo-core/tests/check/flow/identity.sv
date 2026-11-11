// [type-query] [assert-type] [qual-depend-field] Claims about one value,
// however it is spelled. `type?` prints a claim as it is spelled where the
// place was read; `assert!(place: T)` compares claims by the value they are
// about, so a destructured `str` and the field read `cursor.str` are one.

struct Cursor canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

// A field read is rooted at its sibling; a destructured local at the local
// bound beside it. The two are one identity.
fn destructured_and_read_are_one(cursor: Mut Cursor) [] -> None => cursor {
    let { str, pos } = cursor
    //~ type[Idx(str) Int]
    type?(pos)
    //~ type[Idx(cursor.str) Int]
    type?(cursor.pos)
    assert!(pos: Idx(str) Int)
    assert!(pos: Idx(cursor.str) Int)
    assert!(cursor.pos: Idx(str) Int)
    assert!(cursor.pos: Idx(cursor.str) Int)
    let _n = str.char_at(pos)
}

// A `pos` destructured alone still names `cursor.str`.
fn only_the_index_destructured(cursor: Cursor) [] -> None => cursor {
    let { pos } = cursor
    //~ type[Idx(cursor.str) Int]
    type?(pos)
    assert!(pos: Idx(cursor.str) Int)
    let _c = cursor.str.char_at(pos)
}

// The cursor's own type, and its borrowed field.
fn the_cursor_itself(cursor: Mut Cursor) [] -> None => cursor {
    //~ type[Mut Cursor]
    type?(cursor)
    assert!(cursor: Mut Cursor)
    //~ type[proj Str]
    type?(cursor.str)
    assert!(cursor.str: proj Str)
    //~ error[assert-type:mismatch]
    assert!(cursor: Cursor)
}

// Another cursor's string is another value, however it is spelled.
fn another_cursors_string(a: Cursor, b: Cursor) [] -> None => a, b {
    //~ error[assert-type:mismatch]
    assert!(a.pos: Idx(b.str) Int)
    let { str, pos } = b
    //~ error[assert-type:mismatch]
    assert!(a.pos: Idx(str) Int)
    assert!(pos: Idx(b.str) Int)
    let _c = str.char_at(pos)
}

// Two fields of one value are two values.
struct Two { a: Str, b: Str }

fn two_fields(p: Two) [] -> None => p {
    let { a, b } = p
    let i = 0
    assert!(i is Idx(a))
    //~ type[Idx(a) Int]
    type?(i)
    assert!(i: Idx(a) Int)
    assert!(i: Idx(p.a) Int)
    //~ error[assert-type:mismatch]
    assert!(i: Idx(b) Int)
    //~ error[assert-type:mismatch]
    assert!(i: Idx(p.b) Int)
    let _n = size(b)
}

// A literal's borrowed field is the string it was built from.
fn a_local_cursor(s: Str) [] -> None => s {
    let cursor = Cursor { str: s, pos: 0 as! Idx(s) }
    let { str, pos } = cursor
    //~ type[Idx(str) Int]
    type?(pos)
    assert!(pos: Idx(s) Int)
    assert!(cursor.pos: Idx(s) Int)
    // Its `str` field is `s`, and a written claim about it is rooted as a
    // read's is (an annotation's `Idx(cursor.str)` was a claim about a
    // field of `cursor` until 2026-11-11).
    assert!(pos: Idx(cursor.str) Int)
    let _j: Idx(cursor.str) Int = cursor.pos
    let _c = str.char_at(pos)
}

// A temporary has no identity: its field reads with no claim.
fn a_temporarys_field(other: Cursor) [] -> None => other {
    let _i = copy(other).pos
    //~ type[Int]
    type?(_i)
}

// Asking reads nothing: a moved value still has a type, and asking is not a
// use after the move.
fn eat<T>(v: T) [] -> None => !v {
}

fn asking_is_not_a_use(s: Str) [] -> None => !s {
    eat(s)
    //~ type[Str]
    type?(s)
    assert!(s: Str)
}

// Nor is it a read for the unused-variable warning.
fn asking_is_not_a_read() {
    //~ warning[var-unused:never-used]
    let unused = 3
    //~ type[Int]
    type?(unused)
}

// Nor is it a capture: a lambda asking about a local holds no view of it, so
// mutating the local does not poison the lambda.
fn apply(f: (Int) -> Int, v: Int) -> Int {
    return f(v)
}

fn asking_in_a_lambda_captures_nothing() -> Int {
    let xs = Mut List(1, 2)
    let f = (n: Int) -> Int {
        //~ type[NonEmpty Mut List<Int>]
        type?(xs)
        assert!(xs: Mut NonEmpty List<Int>)
        return n
    }
    let before = apply(f, 1)
    add(xs, 9)
    return before + apply(f, 1)
}

// Nor does it end a linear value.
linear struct Ticket { n: Int }

fn close(t: Ticket) [] -> None => !t {
}

fn asking_does_not_discharge() {
    //~ error[ir-linear-check:leak-at-exit]
    //~ warning[var-unused:never-used]
    let t = Ticket { n: 1 }
    //~ type[Ticket]
    type?(t)
}

fn asking_then_discharging() {
    let t = Ticket { n: 1 }
    //~ type[Ticket]
    type?(t)
    close(t)
}

fn main() {}
