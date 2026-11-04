// [linear-container] [fn-variadic] Arrays hold obligations as a list does
// (user decision 2026-10-30): a constructor consumes its tail, so a list or a
// deque of obligations is built in one call; a fn may take a consumed tail of
// them and end it with `drain` or `append`; and a loop may read one.
//
// Compile-and-run data ([test-run-data]).

//--- arrays_of_obligations

linear struct Ticket { id: Int }

fn spend(t: Ticket) [] -> None => !t {
}

// Reads every ticket, then ends the array.
fn total(...ts: Ticket[]) [] -> Int => !ts {
    let n = 0
    for t in ts {
        n = n + t.id
    }
    drain(ts, spend)
    return n
}

// Moves the tickets into a list the caller owns.
fn collect(into: Mut List<Ticket>, ...ts: Ticket[]) [] -> None => into: Mut, !ts {
    into.append(ts)
}

fn main() [use] {
    use StdConsole()
    let l = List(Ticket { id: 1 }, Ticket { id: 2 }, Ticket { id: 3 })
    println("list ${size(l)}")
    drain(l, spend)

    println("total ${total(Ticket { id: 4 }, Ticket { id: 5 })}")

    let more = array_of(Ticket { id: 7 }, Ticket { id: 8 })
    let m = Mut List(Ticket { id: 6 }, ...more)
    collect(m, Ticket { id: 9 })
    println("mut list ${size(m)}")
    drain(m, spend)

    let d = deque_of(Ticket { id: 10 }, Ticket { id: 11 })
    println("deque ${size(d)}")
    drain(d, spend)
}
//= stdout
//= list 3
//= total 9
//= mut list 4
//= deque 2
