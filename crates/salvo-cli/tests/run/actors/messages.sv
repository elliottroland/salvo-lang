// [actor-message] [actor-receive] Messages as values: a handler that
// takes every message with `receive` and hands it on with `forward`,
// spawned and bound by `use`, and a message narrowed by its arm.

//--- receive_and_forward

actor effect Inventory {
    send fn reserve(sku: Str, qty: Int, out: Reply<Str>) => !sku, !out
    send fn restock(qty: Int)
}

handler Store() for Inventory {
    mailbox { capacity: 8 }
    state { stock: Int = 10 }
    send fn reserve(sku: Str, qty: Int, out: Reply<Str>) => !sku, !out {
        stock = stock - qty
        send(out, "${sku}: ${stock} left")
    }
    send fn restock(qty: Int) {
        stock = stock + qty
    }
}

handler Front(back: eff[Inventory]) for Inventory {
    mailbox { capacity: 8 }
    state { seen: Int = 0 }
    send fn receive(m: Message<Inventory>) => !m {
        seen = seen + 1
        forward(back, m)
    }
}

fn describe(m: Message<Inventory>) [] -> Str => m {
    return when m {
        is .Reserve { "reserve ${m.qty} of ${m.sku}" }
        is .Restock { "restock ${m.qty}" }
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let p = pool(2)
    let store = spawn Store() on p
    let front = spawn Front(copy(store)) on p
    println(waitfor out: Reply<Str> { front.reserve("apple", 3, out) })
    front.restock(5)
    println(waitfor out: Reply<Str> { front.reserve("pear", 1, out) })
    println(describe(Inventory.Restock { qty: 2 }))
    use Front(copy(store))
    println(waitfor out: Reply<Str> { reserve("fig", 2, out) })
}
//= stdout
//= apple: 7 left
//= pear: 11 left
//= restock 2
//= fig: 9 left
