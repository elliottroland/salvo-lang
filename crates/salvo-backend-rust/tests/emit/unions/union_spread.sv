// A union spanning a generic arm.

//--- union_spanning_generic
// `detach<T, C>(...T | Box2<C>)` meets `Str | Int | Box2<Bool>` with
// `T = Str | Int` (the `...T` spread captures both), `C = Bool`, and answers
// the flat `Str | Int | Bool`. The callee is one generic body over the
// nested `Union2<T, Box2<C>>`; the call site rewraps the flat argument to
// nested and the nested result back to flat. [type-union-spread]
struct Box2<C> { value: C }

// The callee is a Rust generic over the nested slot (`T` one arm).
//~ emits: pub fn detach<T
//~ emits: crate::unions::Union2<T, crate::Box2<C>>
fn detach<T, C>(value: ...T | Box2<C>) -> T | C => !value {
    when value {
        is Box2<C> { return value.value }
        is T { return value }
    }
}

fn show(v: Str | Int | Bool) [Console] -> None {
    when v {
        is Str { println("str ${v}") }
        is Int { println("int ${v}") }
        is Bool { println("bool ${v}") }
    }
}

fn main() [use] {
    use StdConsole()
    let a: Str | Int | Box2<Bool> = "hi"
    let b: Str | Int | Box2<Bool> = 7
    let c: Str | Int | Box2<Bool> = Box2<Bool> { value: true }
    let d: Str | Int | Box2<Bool> = "tf"
    show(detach(a))
    show(detach(b))
    show(detach(c))
    show(detach<Str | Int, Bool>(d))
}
