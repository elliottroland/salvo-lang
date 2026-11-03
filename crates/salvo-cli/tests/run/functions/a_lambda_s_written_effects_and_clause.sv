// [fn-lambda-sig] A lambda writes its effects and its deduction clause between
// its parameters and `->`, as a fn writes them after its own: `=> !b` makes
// the value consume its argument, `=> b` keeps it, and `[Console]` is the set
// its callers supply, so a pure fn can build one and hand it out. A
// parenthesized subscript (`(arr)[0]`) still reads as one.
//
// Compile-and-run data ([test-run-data]).

//--- written_effects_and_clauses_run
struct Box { v: Str }

fn eat(b: Box) -> Str => !b {
    return b.v
}

fn apply(f: (b: Box) -> Str, b: Box) -> Str => f {
    return f(b)
}

fn first(...ns: Int[]) -> Int => ns {
    return (ns)[0]
}

fn make() -> () [Console] -> None {
    return () [Console] -> println("from a made lambda")
}

fn main() [use] {
    use StdConsole()
    let consume = (b: Box) => !b -> eat(b)
    println(consume(Box { v: "moved" }))
    let keep = (b: Box) => b -> b.v
    let y = Box { v: "kept" }
    println(keep(y))
    println(apply((b) => b -> "${b.v} again", y))
    println(y.v)
    let h = make()
    h()
    let say = (s: Str) [Console] -> { println("said ${s}") }
    say("hi")
    let xs = List(1, 2, 3)
    let doubled = map(iter(xs), (n) [] -> n * 2)
    println("${doubled} ${first(7, 8)}")
}
//= stdout
//= moved
//= kept
//= kept again
//= kept
//= from a made lambda
//= said hi
//= [2, 4, 6] 7
