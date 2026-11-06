// [effect-claim] Claims at run time: a handler `for Ordered Counter`, used
// and spawned, its handle carrying the claim through `use addr`, and a router
// (claiming nothing) under a bare requirement.

//--- ordered_and_bare
actor effect Counter {
    send fn add(n: Int)
    send fn total(out: Reply<Int>) => !out
}

handler Counting() for Ordered Counter {
    mailbox { capacity: 16 }
    state { sum: Int = 0 }
    send fn add(n: Int) { sum = sum + n }
    send fn total(out: Reply<Int>) => !out { out.send(sum) }
}

handler Loose(back: eff[Counter]) for Counter {
    mailbox { capacity: 16 }
    send fn sent(m: Message<Counter>) => !m { send(back, m) }
}

fn tally(ns: List<Int>) [Ordered Counter] -> Int => ns {
    for n in ns { add(n) }
    return waitfor out: Reply<Int> { total(out) }
}

fn loose(ns: List<Int>) [Counter] -> None => ns {
    for n in ns { add(n) }
}

fn via(c: eff[Ordered Counter]) [use] -> Int => c {
    use c
    return tally(List(1, 2))
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let p = pool(2)
    let c = spawn Counting() on p
    println("${via(copy(c))}")
    use Counting()
    println("${tally(List(1, 2, 3))}")
    use Loose(copy(c))
    loose(List(4))
}
//= stdout
//= 3
//= 6
