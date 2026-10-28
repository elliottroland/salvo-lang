// [fn-value-select] [linear-discard] `drop` as the uniform callback
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- drop_as_a_consuming_callback

linear struct Handle : Next<self, Int> canbe Mut {
    at: Int
}

fn open_handle(from: Int) -> Mut Handle {
    return Mut Handle { at: from }
}

fn next(h: Mut Handle) -> Emitted Int | Finished => h: Mut {
    if h.at <= 0 {
        return finished()
    }
    let v = copy(h.at)
    h.at = h.at - 1
    return emitted(v)
}

fn close(h: Handle) -> None => !h { discard(h) }

struct Counter : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Counter) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Next<It, Int>) -> Int => !it =>[end] !x {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    end(it)
    return sum
}

fn main() [use] {
    use StdConsole()
    let h = open_handle(3)
    println("linear ${drain(h, close)}")
    let c = Mut Counter { at: 3 }
    println("plain ${drain(c, drop)}")
}
//= stdout
//= linear 6
//= plain 6
