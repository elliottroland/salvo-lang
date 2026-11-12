// [type-union-body] [fn-attached] A named union's body, as
// docs/language/Data-and-Types.md "A named union's functions" writes it: its
// fns (a `by` declaration among them) and refinements, after the obligation
// clause. As written, the fns are the body's; the lowering hoists them.

struct Circle : ToStr<self> by auto {
    radius: Int
}

struct Square : ToStr<self> by auto {
    side: Int
}

qualifier Grown<T> for T

export type Shape = Circle | Square : ToStr<self> {
    fn to_str(value: Shape) -> Str by auto

    fn area(shape: proj Shape) -> Int {
        return when shape {
            is Circle {
                3 * shape.radius * shape.radius
            }
            is Square {
                shape.side * shape.side
            }
        }
    }

    refn grow(shape: Shape) => shape: +Grown

    fn grow(shape: Shape) [] -> None => shape {}
}

fn main() {}
