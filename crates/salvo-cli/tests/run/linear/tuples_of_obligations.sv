// [linear-container] A tuple owes when a component does (user decision
// 2026-10-30): it is destructured, or its linear components are moved out —
// and a map of obligations is built in one call, its entries pairs.
//
// Compile-and-run data ([test-run-data]).

//--- tuples_of_obligations

linear struct Ticket { id: Int }

fn spend(t: Ticket) [] -> None => !t {
    discard(t)
}

fn issue(n: Int) [] -> (Ticket, Int) => n {
    return (Ticket { id: n }, n * 10)
}

fn main() [use] {
    use StdConsole()
    let (t, n) = issue(1)
    println("destructured ${t.id} ${n}")
    spend(t)

    let p = issue(2)
    println("by component ${p.0.id} ${p.1}")
    spend(p.0)

    let m = Mut Map("a": Ticket { id: 3 }, "b": Ticket { id: 4 })
    println("map ${size(m)}")
    drain(m, spend)

    let pairs = List(issue(5), issue(6))
    drain(pairs, (e) => !e -> {
        let (ticket, _) = e
        spend(ticket)
    })
    println("done")
}
//= stdout
//= destructured 1 10
//= by component 2 20
//= map 2
//= done
