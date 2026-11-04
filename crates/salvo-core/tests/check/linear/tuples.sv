// [linear-container] A tuple owes when any component does (user decision
// 2026-10-30): one value, ended whole, by destructuring, or by moving each
// linear component out.

linear struct Ticket { id: Int }

fn spend(t: Ticket) [] -> None => !t {
    discard(t)
}

fn issue(n: Int) [] -> (Ticket, Int) => n {
    return (Ticket { id: n }, n)
}

fn both() [] -> (Ticket, Ticket) {
    return (Ticket { id: 1 }, Ticket { id: 2 })
}

fn destructured() {
    let (t, n) = issue(1)
    let _seen = n
    spend(t)
}

// Moving every linear component out settles the tuple, and reading the rest
// is fine.
fn by_component() {
    let p = issue(1)
    let _n = p.1
    spend(p.0)
    let b = both()
    spend(b.0)
    spend(b.1)
}

fn dropped() {
    //~ error[ir-linear-check:leak-at-exit]
    let _p = issue(1)
}

fn half_moved() {
    //~ error[ir-linear-check:leak-at-exit]
    let b = both()
    spend(b.0)
}

fn ignored_in_a_pattern() {
    //~ error[ir-linear-check:leak-at-exit]
    let (_, _n) = issue(1)
}

fn copied(p: (Ticket, Int)) [] -> None => !p {
    //~ error[linear-generics:copy-linear]
    let q = copy(p)
    spend(p.0)
    spend(q.0)
}

// The customer: a map of obligations built in one call, its entries pairs.
fn main() [use] {
    let m = Map("a": Ticket { id: 3 }, "b": Ticket { id: 4 })
    drain(m, spend)
}
