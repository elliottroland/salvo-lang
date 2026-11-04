// [seq-lazy] [linear-container] A lazy adaptor over a linear iterator: driven
// where it lives, then `detach`ed for its source, which is read on and closed
// (user decision 2026-10-30).
//
// Compile-and-run data ([test-run-data]).

//--- adaptors_over_an_obligation

linear struct Ticks : Next<self, Int> canbe Mut {
    at: Int
}

fn ticks(from: Int) [] -> Mut Ticks {
    return Mut Ticks { at: from }
}

fn next(l: Mut Ticks) [] -> Emitted Int | Finished => l: Mut {
    if l.at <= 0 {
        return finished()
    }
    let v = copy(l.at)
    l.at = l.at - 1
    return emitted(v)
}

fn close(l: Ticks) [Console] -> None => !l {
    println("closed at ${l.at}")
    discard(l)
}

fn main() [use] {
    use StdConsole()
    let t = taking(ticks(5), 2)
    for n in t {
        println("took ${n}")
    }
    let rest = detach(t)
    for n in rest {
        println("rest ${n}")
    }
    close(rest)

    let m = mapping(skipping(ticks(4), 1), (x) -> x * 10)
    for n in m {
        println("mapped ${n}")
    }
    close(detach(detach(m)))
}
//= stdout
//= took 5
//= took 4
//= rest 3
//= rest 2
//= rest 1
//= closed at 0
//= mapped 30
//= mapped 20
//= mapped 10
//= closed at 0
