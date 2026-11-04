// [linear-container] An array of linear elements is linear (user decision
// 2026-10-30): it is owned once, passed on, and ended whole by `drain` or
// `append`. [fn-variadic] A constructor consumes its tail.

linear struct Ticket { id: Int }

fn spend(t: Ticket) [] -> None => !t {
    discard(t)
}

// A loop may read a linear array…
fn total(...ts: Ticket[]) [] -> Int => !ts {
    let n = 0
    for t in ts {
        n = n + t.id
    }
    drain(ts, spend)
    return n
}

// …but not move its elements out: a loop can stop early.
fn spend_each(...ts: Ticket[]) [] -> None => !ts {
    //~ error[ir-linear-check:array-for]
    for t in ts {
        spend(t)
    }
}

// A consumed tail is owed like any linear parameter.
//~ error[ir-linear-check:leak-at-exit]
fn forgets(...ts: Ticket[]) [] -> None => !ts {
}

// A kept one is the caller's: nothing is owed for it, and reading is all the
// fn may do (no caller can hand it linear values).
fn count(...ts: Ticket[]) [] -> Int => ts {
    return size(ts)
}

// Moving into a list takes the whole array.
fn gather(into: Mut List<Ticket>, ...ts: Ticket[]) [] -> None => into: Mut, !ts {
    into.append(ts)
}

fn main() [use] {
    let l = Mut List(Ticket { id: 1 }, Ticket { id: 2 })
    gather(l, Ticket { id: 3 })
    drain(l, spend)
    //~ error[linear-generics:variadic]
    let _n = count(Ticket { id: 4 })
}

//--- a_constructor_consumes_its_spread
// [fn-variadic] `List(0, ...xs)` spends `xs`, as `List(x)` spends `x`.
fn main() [use] {
    let xs = array_of(1, 2)
    let _l = List(0, ...xs)
    //~ error[flow-use-after-move:consumed]
    let _n = size(xs)
}
