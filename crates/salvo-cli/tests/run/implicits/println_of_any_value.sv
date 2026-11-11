// [std-console] [implicit-param] `println` of a value that is not a `Str`
//
// std's `println<T>(value: T, ?to_str: (T) -> Str)` prints any value whose
// type has a `to_str` in scope at the call; `println(message: Str)` still
// takes a `Str`, being the more specific overload.

//--- printing_values

struct Point {
    x: Int
    y: Int
}

fn to_str(p: Point) -> Str {
    return "(${p.x}, ${p.y})"
}

struct Circle {
    radius: Int
}

struct Square {
    side: Int
}

type Shape = Circle | Square {
    fn to_str(s: proj Shape) -> Str {
        return when s {
            is Circle { "circle ${s.radius}" }
            is Square { "square ${s.side}" }
        }
    }
}

fn brief(p: Point) -> Str {
    return "<${p.x}>"
}

fn each(xs: List<Int>, f: (x: Int) [Console] -> None) [Console] -> None => xs, f {
    for x in xs {
        f(x)
    }
}

fn main() [use] -> None {
    use StdConsole()
    println(42)
    println(Point { x: 1, y: 2 })
    let shape: Shape = Square { side: 3 }
    println(shape)
    println("a plain Str")
    println(Point { x: 5, y: 6 }, to_str = brief)
    each(List(7, 8), println)
}
//= stdout
//= 42
//= (1, 2)
//= square 3
//= a plain Str
//= <5>
//= 7
//= 8
