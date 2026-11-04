// [seq-lazy] [linear-container] A lazy adaptor over a linear source is linear
// itself (user decision 2026-10-30): it is driven where it lives, and
// `detach` ends it, answering the source to be closed.

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

fn close(l: Ticks) [] -> None => !l {
}

fn detached() {
    let t = taking(ticks(5), 2)
    for _n in t {
    }
    close(detach(t))
    let m = mapping(skipping(ticks(4), 1), (x) -> x * 10)
    for _n in m {
    }
    close(detach(detach(m)))
}

// The eager functions drive a linear source in place and hand it back.
fn counted() [] -> Int {
    let c = ticks(3)
    let n = count(c)
    close(c)
    return n
}

fn forgotten() {
    //~ error[ir-linear-check:leak-at-exit]
    let t = filtering(ticks(5), (x) -> x > 2)
    for _n in t {
    }
}

fn a_temporary_adaptor() {
    // [iter-drive-in-place] A temporary the loop would consume: nothing
    // could end it, so it must be a named place.
    //~ error[iter-drive-in-place:linear-consumed]
    for _n in taking(ticks(5), 2) {
    }
}
