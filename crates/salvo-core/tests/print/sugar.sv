// [ast-print] [syntax-lowered] One use of each piece of sugar the parser
// keeps as written, in each position a formatter must print it back from:
// a statement, a condition, a call argument, a lambda body. Parsed only
// (`print_tests.rs`), never checked.

use Logging() on pool(1)

// A doc comment survives the printing.
fn as_qual(x: Int, xs: List<Int>) -> None {
    let a = x as! Positive
    let b = x as? Positive
    if x as? Positive is Int {
        print(x as! Positive)
    }
    let f = (y) -> y as! Positive
    xs.each((y) -> {
        let c = y as? Positive NonZero
    })
}

fn compound(p: Mut Point, xs: List<Int>) -> None {
    let n = 1
    n += 2
    n -= 1 + 2
    n *= -3
    n /= n as! Positive
    n **= 2
    p.x += 1
    if n > 0 {
        p.y -= n
    }
    xs.each((y) -> {
        n += y
    })
}

fn is_forms(s: Str, x: Int | Str | None, xs: List<Int | Str>) -> Bool {
    let a = x !is Str
    let b = s is "json" | "text"
    let c = x !is Int | Str
    if x !is None && s is "a" | "b" | "c" {
        print(x !is Int, s !is "a" | "b")
    }
    while x !is Int {
        return false
    }
    let f = (y) -> y !is Int | Str
    assert!(x as? NonNegative !is None)
    return !(x !is Str) || x is Int | None
}

fn pairs(m: Map<Str, Int>) -> None {
    let a = Map("a": 1, "b": 2 + 3)
    let b = Mut Map<Str, Int>("x" + "y": m.size(), "z": -1)
    f(g("k": (x: -Ok)), h((1, 2)))
    if Map("a": 1).size() > 0 {
        print(List(("a", 1)))
    }
    let c = (k) -> Map(k: k as! Positive, "n": n !is Int)
    m?.merge("a": 1)
}

fn shorthand(x: Int, y: Int) -> Point {
    let p = Point { x, y }
    let q = Point { x, y: y + 1, ...p }
    let r = Mut Point { x: x, y }
    f(Point { x, y })
    if (p == Point { x, y }) {
        print(Point { x })
    }
    let g = (x) -> (Point { x, y: 0 })
    return { x: x, y }
}

fn terminal(xs: List<Int>, c: Bool) -> Int {
    for x in xs ...
    while c ...
    if c ... else {
        return 0
    }
    try ...
    let n = 1
    n += 1
    xs.each((y) -> ...)
    if y !is Int ...
    return n
}

fn terminal_empty(xs: List<Int>) -> None {
    for x in xs ...
}

fn terminal_nested(xs: List<Int>) -> None {
    for x in xs {
        if x > 0 ...
        print(x)
    }
    let f = (y) -> {
        while y > 0 ...
        y -= 1
    }
}

fn main() [use] {
    waitfor out ...
    out.send(1)
}

fn use_on(n: Int) [use, spawn] {
    use Counting(0) on pool(2)
    use Counting(n) with Db(), log on pool(n + 1)
    let c = use Counting(0) on pool(1)
    let d = use Counting(0) with Db() on main_pool()
    use Counting(0)
    f(use Counting(1) on pool(1))
}

send fn trailing(n: Int) [spawn] {
    counter.total() replyto got(n)
    counter.total() replyto got(n) replyto done()
    counter.add(n, n + 1) replyto! got(n) on pool(1)
    if n > 0 {
        total(n) replyto got(n)
    }
    (a() replyto b()) replyto c()
}

fn plural<T>(xs: List<T>, ?a: (T) -> Bool, ?b: (T) -> Bool, ?c: (T) -> Bool, ?d: (T) -> Bool) -> None
=> a|b|c with d, xs {}

fn plural_and_one<T>(xs: List<T>, ?a: (T) -> Bool, ?b: (T) -> Bool, ?c: (T) -> Bool) -> None => a with c, b with c {}

// A namespace keeps its items, and its docs.
export namespace Shapes {
    struct Circle {
        r: Int
    }

    namespace Inner {
        fn area(c: Circle) -> Int {
            return c.r * c.r
        }
    }
}

namespace Rest ...

fn last() -> None {}

namespace Deeper ...

struct Leaf {}

// The comptime forms, which a formatter meets in a template as written.
comptime fn describe<T is Struct>(t: T) -> Str {
    [let keys = filter(T.fields, (f) -> f.name !is "id" && f.type is <E> List<E>)]
    let out = Mut List<Str>()
    [for field in keys] {
        let [field]: [field.type] = t.[field]
        [if field.default !is None && field.index > 1 + 2 * 3] {
            add(out, "${field.name}")
        } else {
            refuse!("no default for ${field.name}")
        }
        [when field.type.kind] {
            is Struct {
                add(out, [describe@main(t.[field])])
            }
            is Union {
                [when t.[field]] {
                    [arm] {
                        add(out, "${arm.name}")
                    }
                }
            }
            is Tuple {}
            is FnType {}
            is Opaque {}
        }
        [when field.type] {
            is <E> List<E> {
                f([...for e in E.fields] { 1, 2 })
            }
            is * {}
        }
        [when field.name] {
            is "a" | "b" {
                let r = [field.type] { [...for g in T.fields] { [g]: 0 } }
            }
            is "c" {}
        }
    }
    [for field in T.fields] ...
    [if T.fields is None] ... else {
        return "none"
    }
    return show@[site](out)
}
