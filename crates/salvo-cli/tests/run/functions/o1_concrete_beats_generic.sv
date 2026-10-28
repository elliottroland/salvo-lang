// [fn-overload-rank] O1: concrete beats generic
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- effect_members_taking_fn_values

effect Apply {
    fn apply(f: (n: Int) -> Int, x: Int) -> Int => f, x
    fn once_apply(f: once () -> Int) -> Int => !f
}

handler Applying() of Apply {
    fn apply(f: (n: Int) -> Int, x: Int) -> Int => f, x {
        let y = f(x)
        return f(y)
    }

    fn once_apply(f: once () -> Int) -> Int => !f {
        return f()
    }
}

fn through(x: Int) [Apply] -> Int {
    return apply(n -> n + 1, x) + once_apply(() -> x * 10)
}

fn main() [use] {
    use StdConsole()
    use Applying()
    let k = 3
    println("${apply(n -> n * k, 2)} ${once_apply(() -> k + 1)} ${through(5)}")
}
//= stdout
//= 18 4 57

//--- the_most_specific_overload

fn describe<T>(value: T) -> Str {
    return "generic"
}

fn describe(value: Int) -> Str {
    return "concrete"
}

fn main() [use] {
    use StdConsole()
    println(describe(3))
    println(describe("text"))
}
//= stdout
//= concrete
//= generic
