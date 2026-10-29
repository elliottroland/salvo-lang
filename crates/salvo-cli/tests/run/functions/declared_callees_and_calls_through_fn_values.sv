// [backend-never-wrong] declared callees and calls through fn values
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- declared_callees_past_same_named_locals

struct Ctr : Next<self, Int> canbe Mut {
    n: Int
}

fn next(c: Mut Ctr) [] -> Emitted Int | Finished => c: Mut {
    if c.n >= 2 {
        return finished()
    }
    c.n = c.n + 1
    return emitted(c.n)
}

fn by_tens(c: Mut Ctr) [] -> Emitted Int | Finished => c: Mut {
    if c.n >= 2 {
        return finished()
    }
    c.n = c.n + 1
    return emitted(c.n * 10)
}

fn walk(c: Mut Ctr, d: Mut Ctr, ?next: (c: Mut Ctr) -> Emitted Int | Finished) [Console] -> None => c: Mut, d: Mut {
    for x in c {
        println("for: ${x}")
    }
    let r = next(d)
    if r is Emitted Int {
        println("call: ${r}")
    }
}

fn show(n: Int) [] -> Str => n { return "decl ${n}" }

effect Labels {
    fn label(n: Int) -> Str
}

fn label(n: Int) [] -> Str => n { return "fn ${n}" }

fn apply(f: (n: Int) -> Str, n: Int) [] -> Str => f, n { return f(n) }

fn roots(salvo: Int) [] -> Str => salvo {
    let kotlin = salvo + 1
    let java = show(kotlin)
    return java
}

handler LabelsH of Labels {
    fn label(n: Int) -> Str {
        return "member, ${label(n)}, ${apply(label, n)}"
    }
}

fn main() [use] {
    use StdConsole()
    let c = Mut Ctr { n: 0 }
    let d = Mut Ctr { n: 0 }
    walk(c, d, next = by_tens)
    let show = (n: Int) -> "local ${n}"
    println("call: ${show(5)}")
    println("dot: ${5.show()}")
    println("at: ${show@main(5)}")
    println("roots: ${roots(1)}")
    use LabelsH()
    println(label@Labels(7))
}
//= stdout
//= for: 10
//= for: 20
//= call: 10
//= call: local 5
//= dot: local 5
//= at: decl 5
//= roots: decl 2
//= member, fn 7, fn 7

//--- calls_through_fn_values

fn adder() [] -> (n: Int) -> Int { return (n: Int) -> n + 1 }

fn printer() [] -> (s: Str) [Console] -> None { return (s: Str) -> println(s) }

effect Ops {
    fn apply(f: (x: Int) -> Int, n: Int) -> Int => f
}

handler OpsH of Ops {
    fn apply(f: (x: Int) -> Int, n: Int) -> Int { return f(n) }
}

fn apply(f: (x: Int) -> Int, s: Str) [] -> Int => f { return f(100) }

fn pick(f: (x: Int) -> Int, n: Int) [] -> Int => f { return f(n) }
fn pick(f: (x: Int) -> Int, s: Str) [] -> Int => f { return f(1000) }

fn main() [use] {
    use StdConsole()
    use OpsH()
    println("computed: ${adder()(1)}")
    printer()("computed, with an effect")
    println("member: ${apply(x -> x + 1, 20)}")
    println("fn: ${apply(x -> x * 2, "s")}")
    println("pick: ${pick(x -> x + 3, 4)} ${pick(x -> x + 3, "s")}")
}
//= stdout
//= computed: 2
//= computed, with an effect
//= member: 21
//= fn: 200
//= pick: 7 1003
