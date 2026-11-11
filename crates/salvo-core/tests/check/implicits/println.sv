// [std-console] [implicit-resolve] std's `println<T>(value: T, ?to_str:
// (T) -> Str)` beside `println(message: Str)`: a value prints when its type
// has a `to_str` in scope at the call, and a type with none is the fill's
// error, naming the `?to_str` position nothing fills.

struct Plain {
    x: Int
}

struct Shown {
    x: Int
}

fn to_str(s: Shown) -> Str {
    return "shown ${s.x}"
}

fn prints() [Console] -> None {
    println(1)
    println(Shown { x: 1 })
    println("a Str: the non-generic overload")
    //~ error[implicit-fit:no-fill]
    println(Plain { x: 1 })
    let u: Int | Str = 1
    //~ error[implicit-fit:no-fill]
    println(u)
}

// [fn-value-select] `println` named as a value is two functions now: a
// position with a fn type picks one, and an unannotated `let` cannot.
fn each(xs: List<Int>, f: (x: Int) [Console] -> None) [Console] -> None => xs, f {
    for x in xs {
        f(x)
    }
}

fn as_value() [Console] -> None {
    each(List(1, 2), println)
    let say: (message: Str) [Console] -> None = println
    say("hi")
    //~ error[fn-value-select:overloaded]
    let _g = println
}
