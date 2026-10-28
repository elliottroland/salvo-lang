// A raw pass that owns something [linear-group] [iter-drive-in-place].

//--- a_raw_pass_is_driven_in_place_and_closed_explicitly
// [linear-group] [iter-protocol] A **raw pass that owns something**: it
// declares `: Linear<self>` beside `: Next<self, T>`, so [group-obligation]
// requires the `close` — and *driving* is what releases it (user decision
// 2026-09-09). The loop calls `close` on every exit: exhaustion, `break` and
// `return` alike. Before this the linear obligation counted as discharged by
// the move into the loop — bookkeeping — while the resource leaked.
//
// [iter-drive-in-place] [linear-group] No implicit discharge sites
// (user decision 2026-09-12): the named pass is driven where it lives —
// no loop-local, no finally — and the program's own `close(lines)` is
// the discharge.
//~ !emits(main.rs): __loop1_pass
linear struct Ticks : Next<self, Int> canbe Mut {
    at: Int
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
    discard(l)
    println("closed")
}

fn drained() [Console] -> None {
    let lines = Mut Ticks { at: 2 }
    for n in lines {
        println("n ${n}")
    }
    // Plain: the program's module declares one `close` [fn-emit-name].
    //~ emits(main.rs): close(console, lines);
    close(lines)
    println("after drain")
}

fn abandoned() [Console] -> None {
    let lines = Mut Ticks { at: 5 }
    for n in lines {
        println("m ${n}")
        break
    }
    close(lines)
    println("after break")
}

fn main() [use] {
    use StdConsole()
    drained()
    abandoned()
}
