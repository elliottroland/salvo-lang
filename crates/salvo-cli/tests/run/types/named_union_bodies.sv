// [type-union-body] A named union's body declares fns on it, as a struct
// body does [fn-attached]: they travel with the union across modules, meet
// its obligations (a hand-written fn, `by` on one body fn, `by` on the
// clause), and are named by its selector at a call and as a value.

//--- imported_with_the_union
// FILE: shapes.sv
export struct Circle : ToStr<self> by auto { r: Int }
export struct Square : ToStr<self> by auto { side: Int }

export type Shape = Circle | Square : Ordered<self> by auto, ToStr<self> {
    fn to_str(value: Shape) -> Str by auto

    fn area(shape: proj Shape) -> Int {
        return when shape {
            is Circle { 3 * shape.r * shape.r }
            is Square { shape.side * shape.side }
        }
    }
}
// FILE: main.sv
import shapes.Shape
import shapes.Circle
import shapes.Square

fn min_of<T>(a: T, b: T, ?Ordered<T>) -> T => !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn main() [use] {
    use StdConsole()
    let a: Shape = Circle { r: 2 }
    let b: Shape = Square { side: 3 }
    println("${area(a)} ${b.area()} ${area@Shape(a)}")
    println("${a} ${b}")
    println("${cmp@Shape(a, b)} ${a < b}")
    let small: Shape = min_of<Shape>(Circle { r: 1 }, Square { side: 9 })
    println(to_str@Shape(small))
    let f = area@Shape
    println("${f(b)}")
}
//= stdout
//= 12 9 12
//= Circle { r: 2 } Square { side: 3 }
//= -1 true
//= Circle { r: 1 }
//= 9

//--- generic_optional_scalar_and_namespaced_unions
// A body on a generic union, on `A?`, on a union of scalars, and on a union
// declared in a namespace.
struct A { x: Int }
struct Some<T> { value: T }
struct Nothing {}

namespace Shapes {
    struct Sq { side: Int }
    struct Ci { r: Int }
    type Shape = Shapes.Sq | Shapes.Ci {
        fn area(s: proj Shapes.Shape) -> Int {
            return when s {
                is .Sq { s.side * s.side }
                is .Ci { 3 * s.r * s.r }
            }
        }
    }
}

type Opt<T> = Some<T> | Nothing {
    fn get_or<T>(o: Opt<T>, other: T) -> T => !o, !other {
        return when o {
            is Some<T> { o.value }
            is Nothing { other }
        }
    }
}

type MaybeA = A? {
    fn x_or(m: proj MaybeA, d: Int) -> Int {
        return m?.x ?: d
    }
}

type SI = Str | Int {
    fn describe(v: proj SI) -> Str {
        return when v {
            is Str { "s" }
            is Int { "i" }
        }
    }
}

fn main() [use] {
    use StdConsole()
    let s: Shapes.Shape = Shapes.Sq { side: 4 }
    println("${s.area()} ${area(s)}")
    let o: Opt<Int> = Some<Int> { value: 5 }
    println("${o.get_or(1)}")
    let n: Opt<Int> = Nothing {}
    println("${get_or@Opt(n, 2)}")
    let m: MaybeA = None
    let k: MaybeA = A { x: 3 }
    println("${m.x_or(9)} ${x_or@MaybeA(k, 9)}")
    let v: SI = 4
    let w: SI = "a"
    println("${v.describe()} ${describe(w)}")
}
//= stdout
//= 16 16
//= 5
//= 2
//= 9 3
//= i s
