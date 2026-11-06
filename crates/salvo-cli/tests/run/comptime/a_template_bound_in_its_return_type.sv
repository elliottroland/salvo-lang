// [fn-by] The type a declaration stamps at is where the template's bound
// parameter stands in its signature: `fn describe() -> Shown<Point> by
// describe` against `comptime fn describe<T is Struct>() -> Shown<T>`
// stamps at `Point`, with no parameter to read it from.

//--- a_template_bound_in_its_return_type
struct Shown<T> {
    text: Str
}

comptime fn describe<T is Struct>() [] -> Shown<T> {
    let out: Mut Str = Mut Str("${T.name}:")
    [for field in T.fields] {
        append@string(out, " ${field.name}")
    }
    return Shown<T> { text: out }
}

struct Point {
    x: Int,
    y: Int
}

fn describe() -> Shown<Point> by describe

fn main() [use] -> None {
    use StdConsole()
    let s: Shown<Point> = describe()
    println(s.text)
}
//= stdout
//= Point: x y
