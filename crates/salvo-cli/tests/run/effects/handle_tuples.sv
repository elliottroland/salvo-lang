// handle tuples
//
// [effect-eff-handle] `use` of a tuple of handles binds each single handle it
// holds, a nested tuple's included: each read at its path (found 2026-10-30,
// the pipeline record D3.3: a nested tuple was read by its flattened index, which
// both target compilers refused).

//--- handle_tuples
effect A {
    fn a() -> Int
}
effect B {
    fn b() -> Int
}
effect C {
    fn c() -> Int
}
handler HA() of A {
    fn a() -> Int {
        return 1
    }
}
handler HB() of B {
    fn b() -> Int {
        return 2
    }
}
handler HC() of C {
    fn c() -> Int {
        return 3
    }
}

fn flat(caps: eff[A, B]) [use] -> Int => caps {
    use caps
    return a() + b()
}

fn nested(caps: ((eff[A], eff[B]), eff[C])) [use] -> Int => caps {
    use caps
    return a() * 100 + b() * 10 + c()
}

fn main() [use] -> None {
    use StdConsole
    let ha = use HA()
    let hb = use HB()
    let hc = use HC()
    println("${flat(eff[ha, hb])}")
    println("${nested(((ha, hb), hc))}")
}
//= stdout
//= 3
//= 123
