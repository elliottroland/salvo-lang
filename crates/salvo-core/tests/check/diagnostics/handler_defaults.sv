// [fn-param-default] A handler's constructor parameters take defaults with
// the fn rules. Every case builds `Span`: `lo` required, `hi` defaulted from
// it, `step` defaulted.

effect Spanned {
    fn width() -> Int
}

handler Span(lo: Int, hi: Int = lo + 10, step: Int = 1) of Spanned {
    fn width() -> Int {
        return (hi - lo) * step
    }
}

// [fn-param-default] A trailing run omitted positionally, a defaulted
// parameter anywhere filled by name — in a `use`, a `with` item and a `spawn`
// alike — and a default that reads an earlier constructor parameter.
effect Twice {
    fn twice() -> Int
}

handler Doubling [Spanned] of Twice {
    fn twice() -> Int {
        return width() * 2
    }
}

actor effect Desk {
    send fn ask(out: Reply<Int>) => !out
}

handler Desking(room: Int = 4, capacity: Int = 2) of Desk {
    mailbox { capacity: capacity }
    send fn ask(out: Reply<Int>) {
        send(out, room)
    }
}

fn omitted_or_named() [use, spawn] -> None {
    use Span(1)
    use Span(1, 5)
    use Span(1, step = 2)
    use Doubling with Span(2, step = 3)
    let _a = spawn Desking() on pool(1)
    let _b = spawn Desking(room = 9) on pool(1)
}

// [fn-param-default] Naming a constructor parameter that has no default is
// refused with the fn message; a name matching no parameter is refused too
// (it used to be dropped without a word).
fn names_a_required_parameter() [use] -> None {
    //~ error[fn-param-default:named-without-default]
    use Span(lo = 1)
    //~ error[implicit-override:no-implicit]
    use Span(1, size = 2)
}

// [fn-param-default] Omitting a constructor parameter that has no default is
// still refused, naming it; so is giving one both ways, or too many.
fn omits_a_required_parameter() [use] -> None {
    //~ error[fn-param-default:handler-missing]
    use Span()
    //~ error[fn-param-default:handler-missing]
    use Span(step = 2)
    //~ error[fn-param-default:named-and-positional]
    use Span(1, 2, hi = 3)
    //~ error[fn-param-default:handler-too-many]
    use Span(1, 2, 3, 4)
}

// [fn-param-default] A constructor parameter's default sees only the earlier
// parameters, as a fn's does.
effect Tell {
    fn tell() -> Int
}

//~ error[name-ident-resolve:unknown]
handler Teller(a: Int = b, b: Int) of Tell {
    fn tell() -> Int {
        return a + b
    }
}

// [fn-param-default] A constructor parameter's default is checked with no
// effects available: an effectful one is refused at the declaration.
fn noisy() [Console] -> Int {
    println("hi")
    return 1
}

//~ error[effect-available:callee-no-handler]
handler NoisyTeller(n: Int = noisy()) [Console] of Tell {
    fn tell() -> Int {
        return n
    }
}
