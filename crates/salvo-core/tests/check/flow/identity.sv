// [type-query] [type-assert] [test-data] [qual-depend-field] Claims about one
// value, however it is spelled. `type?` prints a claim as it is spelled where
// the place was read; `type!(place: T)` compares claims by the value they are
// about, so a destructured `str` and the field read `cursor.str` are one. A
// `//~ type[T]` annotation is judged as `type!` judges, at the `type?`.

struct Cursor canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

// A field read is rooted at its sibling; a destructured local at the local
// bound beside it. The two are one identity, for the annotation too.
fn destructured_and_read_are_one(cursor: Mut Cursor) [] -> None => cursor {
    let { str, pos } = cursor
    //~ type[Idx(str) Int]
    type?(pos)
    //~ type[Idx(cursor.str) Int]
    type?(pos)
    //~ type[Idx(cursor.str) Int]
    type?(cursor.pos)
    //~ type[Idx(str) Int]
    type?(cursor.pos)
    type!(pos: Idx(str) Int)
    type!(pos: Idx(cursor.str) Int)
    type!(cursor.pos: Idx(str) Int)
    type!(cursor.pos: Idx(cursor.str) Int)
    let _n = str.char_at(pos)
}

// Two cursors, each destructured into a local `str` in its own block: the
// index from the first is about `a.str` (the join spells it so), and in the
// second block, where `str` is the other cursor's, an `Idx(str)` is not it.
// The annotation names the value.
fn two_cursors_one_spelling(a: Cursor, b: Cursor) [] -> None => a, b {
    let _p = if size(a.str) > 0 {
        let { str, pos } = a
        let _c = size(str)
        pos
    } else {
        let { str, pos } = a
        let _c = size(str)
        pos
    }
    //~ type[Idx(a.str) Int]
    type?(_p)
    if size(b.str) > 0 {
        let { str, pos } = b
        //~ type[Idx(a.str) Int]
        type?(_p)
        //~ error[type-assert:mismatch]
        type!(_p: Idx(str) Int)
        type!(_p: Idx(a.str) Int)
        let _d = str.char_at(pos)
    }
}

// A `pos` destructured alone still names `cursor.str`.
fn only_the_index_destructured(cursor: Cursor) [] -> None => cursor {
    let { pos } = cursor
    //~ type[Idx(cursor.str) Int]
    type?(pos)
    type!(pos: Idx(cursor.str) Int)
    let _c = cursor.str.char_at(pos)
}

// The cursor's own type, and its borrowed field.
fn the_cursor_itself(cursor: Mut Cursor) [] -> None => cursor {
    //~ type[Mut Cursor]
    type?(cursor)
    type!(cursor: Mut Cursor)
    //~ type[proj Str]
    type?(cursor.str)
    type!(cursor.str: proj Str)
    //~ error[type-assert:mismatch]
    type!(cursor: Cursor)
}

// Another cursor's string is another value, however it is spelled.
fn another_cursors_string(a: Cursor, b: Cursor) [] -> None => a, b {
    //~ error[type-assert:mismatch]
    type!(a.pos: Idx(b.str) Int)
    let { str, pos } = b
    //~ error[type-assert:mismatch]
    type!(a.pos: Idx(str) Int)
    type!(pos: Idx(b.str) Int)
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
    type!(i: Idx(a) Int)
    type!(i: Idx(p.a) Int)
    //~ error[type-assert:mismatch]
    type!(i: Idx(b) Int)
    //~ error[type-assert:mismatch]
    type!(i: Idx(p.b) Int)
    let _n = size(b)
}

// A literal's borrowed field is the string it was built from.
fn a_local_cursor(s: Str) [] -> None => s {
    let cursor = Cursor { str: s, pos: 0 as! Idx(s) }
    let { str, pos } = cursor
    //~ type[Idx(str) Int]
    type?(pos)
    type!(pos: Idx(s) Int)
    type!(cursor.pos: Idx(s) Int)
    // Its `str` field is `s`, and a written claim about it is rooted as a
    // read's is (an annotation's `Idx(cursor.str)` was a claim about a
    // field of `cursor` until 2026-11-11).
    type!(pos: Idx(cursor.str) Int)
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
    type!(s: Str)
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
        type!(xs: Mut NonEmpty List<Int>)
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
//--- a_keyed_identity
// An annotation may write a keyed type's identities; judging it records
// nothing of them (a written identity is a reference of the program's, and
// the program's own, written below, must still be recorded).
import keyed.Key
import keyed.key_hash
import keyed.key_eq
import keyed.keyed

fn a_keyed_identity_in_an_annotation() [] -> Int {
    let u = keyed()
    //~ type[Mut Set<Key>(key_hash, key_eq)]
    type?(u)
    return size(u)
}

fn the_program_writes_them_after() [] -> Int {
    let v: Mut Set<Key>(key_hash, key_eq) = keyed()
    return size(v)
}

fn main() {}

// FILE: keyed.sv
export struct Key { x: Int }

export fn key_hash(v: Key) [] -> Long => v { return hash(v.x) }
export fn key_eq(a: Key, b: Key) [] -> Bool => a, b { return a.x == b.x }

export fn keyed() [] -> Mut Set<Key>(key_hash, key_eq) {
    return Mut Set(hash = key_hash, eq = key_eq)
}
