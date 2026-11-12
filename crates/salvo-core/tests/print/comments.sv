// [fmt-comments] [fmt-blank-lines] Comments in every slot a formatter must
// print them back from, and the blank lines between statements, members and
// declarations. Parsed only (`print_tests.rs`), never checked.

// A free-standing heading over the imports.
import std.core.list
import std.core.seq // the sequences

import std.core.map

// The aliases: this run documents `Small`, the next one `Name`.
type Small = Int
// The name of a thing.
type Name = Str
type Count = Int // a trailing comment on a one-liner

// ===== A heading over a group of declarations =====

//not spaced, and kept so
struct Point {
    // the horizontal one
    x: Int // trailing on a field

    //   indented text in a comment stays indented
    y: Int
    // closing the fields
}

struct Empty {
    // nothing here but a comment
}

struct Shape {
    kind: Str

    // A heading inside a body.

    // The area, documented.
    fn area(s: Shape) -> Int {
        return 0
    }
    // closing the struct
}

// A union with a body.
type Tree = Leaf | Node {
    // the size
    fn size(t: Tree) -> Int {
        return 1 // one
    }
}

namespace Geometry {
    // A heading in a namespace.

    // documents origin
    fn origin() -> Int {
        return 0
    }
    // closing the namespace
}

effect Log {
    // documents info
    fn info(m: Str) -> None
    fn warn(m: Str) -> None // trailing on a member

    // a free-standing comment between members

    fn error(m: Str) -> None
    // closing the effect
}

handler Counting(start: Int) for Count {
    // before the state
    state {
        // documents n
        n: Int = start // trailing on a state field
        // closing the state
    }

    // before the member
    fn total() -> Int {
        return n
    }
}

fn statements(a: Int, b: Int) -> Int { // after the brace
    // leading the let
    let x = a // trailing the let
    let y = b

    // after a blank line
    x += 1
    if x > 1 {
        return 1
        // closing the then, before the `elif`
    } elif x < 0 { // after the elif's brace
        return 2
    } else { // after `else`
        // inside the else
        return 3
    }
    while x > 0 {
        x -= 1 // counting down
    } else {
        // the else of a while
        return 4
    }
    // closing the fn
}

fn lists(a: Int, b: Int) -> Int {
    let p = Point {
        x: a, // the first field
        // the second field
        y: b
    }
    let q = f(
        // the first argument
        a,
        b // the second
    )
    let r = g(
        a,
        name = b
        // closing the arguments
    )
    let t = (
        a, // the first element
        b
    )
    let u = List<
        // the element type
        Int
    >()
    let v = h(Point { x: a, y: b }, (k) -> {
        // inside a lambda
        return k // trailing in a lambda
    })
    let w = (
        k, // the first parameter
        m
    ) -> k + m
    return 0
}

fn params(
    a: Int, // the first parameter
    // the second
    b: Int
) -> Int {
    return a
}

fn arms(x: Int | Str) -> Int {
    return when x {
        // the number arm
        is Int {
            return 1 // one
        }

        // the text arm, after a blank line
        is Str {
            return 2
        }
        // closing the arms
    }
}

comptime fn describe<T is Struct>(value: T) -> Str {
    [for field in T.fields] {
        // inside a comptime loop
        print(field.name)
    }
    [if size(T.fields) > 0] {
        // inside a comptime if
        print("fields")
    } else {
        // inside its else
        print("none")
    }
    return ""
}

fn empty() -> None {
    // only a comment in this body
}

// The last comment of the file, after every declaration.
