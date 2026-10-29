// [effect-member-overload] [fn-param-default] [fn-variadic] A member call is
// selected by the call's own rule (PIPELINE.md B3.5, census D15, D16, D17,
// D25): defaults, named arguments and a variadic tail fill a member as they
// fill a fn, two available effects' members are ranked by the arguments, and
// a lambda is typed by the candidate that leads.
//
// Compile-and-run data ([test-run-data]).

//--- defaults_named_and_variadic_members

effect Log {
    fn log(msg: Str, level: Int = 1) -> Str => msg
    fn total(first: Int, ...rest: Int[]) -> Int
}

handler Printing of Log {
    fn log(msg: Str, level: Int = 1) -> Str => msg {
        return "${msg}@${level}"
    }
    fn total(first: Int, ...rest: Int[]) -> Int {
        let sum = first
        for r in rest {
            sum = sum + r
        }
        return sum
    }
}

fn main() [use] {
    use StdConsole()
    use Printing()
    println(log("plain"))
    println(log("given", 2))
    println(log("named", level = 3))
    println("${total(1)} ${total(1, 2, 3)}")
}
//= stdout
//= plain@1
//= given@2
//= named@3
//= 1 6

//--- two_effects_ranked_by_the_arguments

effect Ints {
    fn put(x: Int) -> Str
}

effect Strs {
    fn put(x: Str) -> Str => x
}

handler IntsH of Ints {
    fn put(x: Int) -> Str { return "int ${x}" }
}

handler StrsH of Strs {
    fn put(x: Str) -> Str => x { return "str ${x}" }
}

fn both() [Ints, Strs] -> Str {
    return "${put(1)}, ${put("s")}"
}

fn main() [use] {
    use StdConsole()
    use IntsH()
    use StrsH()
    println(both())
}
//= stdout
//= int 1, str s

//--- a_member_beside_std_fns_types_the_lambda
// [fn-overload-rank] A member leads on the footing of the module's own fns,
// above std's: `map(3, x -> x + 1)` types the lambda by the member, where
// before a member colliding with std's `map` left nothing to type it.

effect Ops {
    fn map(n: Int, f: (x: Int) -> Int) -> Int => f
}

handler OpsH of Ops {
    fn map(n: Int, f: (x: Int) -> Int) -> Int => f { return f(n) }
}

fn main() [use] {
    use StdConsole()
    use OpsH()
    println("${map(7, x -> x * 3)}")
    let xs = map(List(1, 2), x -> x + 1)
    println("${size(xs)}")
}
//= stdout
//= 21
//= 2

//--- a_member_passed_by_name
// [effect-member-overload] [fn-value-select] A member is a value by its bare
// name at a fn-typed position (R10 D3, PIPELINE.md B3.6): the position's type
// picks it over a fn of the name, and it runs through the instance in scope.
// `m@E` passed as an argument is the same value.

struct Point { x: Int }

effect Fmt {
    fn show(p: Point) -> Str => p
}

handler Brackets of Fmt {
    fn show(p: Point) -> Str => p { return "[${p.x}]" }
}

fn show(n: Int) -> Str => n { return "<${n}>" }

fn on_point(f: (p: Point) -> Str, p: Point) -> Str => f, p { return f(p) }

fn on_int(f: (n: Int) -> Str, n: Int) -> Str => f, n { return f(n) }

fn main() [use] {
    use StdConsole()
    use Brackets()
    let p = Point { x: 4 }
    println(on_point(show, p))
    println(on_int(show, 5))
    println(on_point(show@Fmt, p))
}
//= stdout
//= [4]
//= <5>
//= [4]
