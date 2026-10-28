// return-type overloads and fn values
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [fn-overload-return] Overloads that differ only in their return type,
// chosen by where each value goes: a `let`, `!`, `?:`, a struct field, an
// argument and a value `if` branch. Neither target overloads by return
// type, so each gets a name of its own from [fn-emit-name]'s sixth level.
//--- return_type_overloads
fn read_as(s: Str) [] -> Int? => s { return parse_int(s) }
fn read_as(s: Str) [] -> Bool? => s {
    if s == "true" { return true }
    if s == "false" { return false }
    return None
}

struct Opts { n: Int?, b: Bool? }

fn want_int(n: Int?) [] -> Int => n { return n ?: 0 }

fn main() [use] -> None {
    use StdConsole()
    let a: Int? = read_as("12")
    let b: Bool? = read_as("true")
    let c: Int = read_as("7") ?: unreachable!()
    let d: Int = read_as("x") ?: 5
    let o = Opts { n: read_as("3"), b: read_as("false") }
    let e = want_int(read_as("9"))
    let f: Bool? = if c > 3 { read_as("true") } else { None }
    println("${a ?: 0} ${b ?: false} ${c} ${d} ${o.n ?: 0} ${o.b ?: true} ${e} ${f ?: false}")
}
//= stdout
//= 12 true 7 5 3 false 9 true

// [fn-value] A function passed by name and one filling an implicit are one
// rule: the constructor's gained claim (`-> +Valid Range`) reaches both, a
// function with implicits of its own (`twice`, `?double`) is passed by name
// with them filled as it fills an implicit, and so is a stamped generic `eq`
// for `Box<Str>`.
//--- a_fn_value_is_one_rule_by_name_and_as_an_implicit
struct Range { min: Int, max: Int }

qualifier Valid of Range

fn validate(o: Range) [] -> +Valid Range => !o {
    return o
}

fn by_name<V>(o: Range, f: (o: Range) -> V) [] -> V =>[f] !o => !o, f {
    return f(o)
}

fn by_implicit<V>(o: Range, ?validate: (o: Range) -> V) [] -> V =>[validate] !o => !o {
    return validate(o)
}

fn width(r: Valid Range) [] -> Int => r {
    return r.max - r.min
}

fn double(n: Int) [] -> Int {
    return n * 2
}

fn twice<T>(x: T, ?double: (n: T) -> T) [] -> T => x {
    return double(double(x))
}

fn apply(n: Int, f: (n: Int) -> Int) [] -> Int => f {
    return f(n)
}

fn as_implicit(n: Int, ?twice: (n: Int) -> Int) [] -> Int {
    return twice(n)
}

struct Box<T> : Eq<self> by auto {
    item: T
}

fn same(a: Box<Str>, b: Box<Str>, f: (x: Box<Str>, y: Box<Str>) -> Bool) [] -> Bool => a, b, f {
    return f(a, b)
}

fn main() [use] -> None {
    use StdConsole()
    let a: Valid Range = by_name(Range { min: 1, max: 5 }, validate)
    let b: Valid Range = by_implicit(Range { min: 2, max: 9 })
    println("${width(a)} ${width(b)}")
    println("${apply(3, twice)} ${as_implicit(3)}")
    println("${same(Box { item: "x" }, Box { item: "x" }, eq)} ${same(Box { item: "x" }, Box { item: "y" }, eq)}")
}
//= stdout
//= 4 7
//= 12 12
//= true false
