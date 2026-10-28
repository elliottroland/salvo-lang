// [linear-group] [linear-composite] the wrapper pass, end to end
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_linear_wrapper_pass

linear struct Ticks : Next<self, Int> canbe Mut {
    at: Int
}

fn open_lines(from: Int) -> Mut Ticks {
    return Mut Ticks { at: from }
}

fn next(l: Mut Ticks) -> Emitted Int | Finished => l: Mut {
    if l.at <= 0 {
        return finished()
    }
    let v = copy(l.at)
    l.at = l.at - 1
    return emitted(v)
}

fn close(l: Ticks) [Console] -> None => !l {
    println("closed lines")
    discard(l)
}

linear struct Take : Next<self, Int> canbe Mut {
    source: Mut Ticks,
    left: Int
}

fn take(source: Mut Ticks, n: Int) -> Mut Take => !source {
    return Mut Take { source: source, left: n }
}

fn next(t: Mut Take) -> Emitted Int | Finished => t: Mut {
    if t.left <= 0 {
        return finished()
    }
    t.left = t.left - 1
    return next(t.source)
}

fn close_take(t: Take) [Console] -> None => !t {
    close(t.source)
}

fn main() [use] {
    use StdConsole()
    let t = take(open_lines(5), 2)
    for n in t {
        println("n ${n}")
    }
    close_take(t)
    println("done")
}
//= stdout
//= n 5
//= n 4
//= closed lines
//= done
