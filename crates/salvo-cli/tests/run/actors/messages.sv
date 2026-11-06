// [actor-message] [actor-receive] Messages as values: a handler that
// takes every message with `receive` and hands it on with `forward`,
// spawned and bound by `use`, and a message narrowed by its arm.

//--- receive_and_forward

actor effect Inventory {
    send fn reserve(sku: Str, qty: Int, out: Reply<Str>) => !sku, !out
    send fn restock(qty: Int)
}

handler Store() for Ordered Inventory {
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

//--- a_generic_relay
// [actor-router] A handler serving its own type parameter, built once per
// protocol it is used at: spawned and bound by `use`.

actor effect Inventory {
    send fn reserve(sku: Str, qty: Int, out: Reply<Str>) => !sku, !out
    send fn restock(qty: Int)
}

handler Store() for Ordered Inventory {
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

handler Relay<E>(back: eff[E]) for E {
    mailbox { capacity: 8 }
    send fn receive(m: Message<E>) => !m {
        forward(back, m)
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let p = pool(2)
    let store = spawn Store() on p
    let front = spawn Relay(copy(store)) on p
    println(waitfor out: Reply<Str> { front.reserve("apple", 3, out) })
    use Relay(copy(store))
    println(waitfor out: Reply<Str> { reserve("fig", 2, out) })
}
//= stdout
//= apple: 7 left
//= fig: 5 left

//--- messages_through_a_namespace_import
// [actor-message] [mod-namespace] The arms are a namespace named after the
// effect: `import shop.Inventory` brings the effect and its messages, and a
// namespace the program writes under the name holds its own types beside them.
import shop.Inventory
import shop.Store

fn describe(m: Message<Inventory>) [] -> Str => m {
    return when m {
        is .Restock { "restock ${m.qty}" }
        is Inventory.Clear { "clear" }
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let m = Inventory.Restock { qty: 2 }
    println(describe(m))
    let n = Inventory.Note { text: "noted" }
    println(n.text)
    let s = spawn Store() on pool(1)
    forward(s, Inventory.Restock { qty: 3 })
    let _ = waitfor r: Reply<Int> { send(r, 1) }
}
// FILE: shop.sv
export actor effect Inventory {
    send fn restock(qty: Int)
    send fn clear()
}

export handler Store() [Console] for Ordered Inventory {
    mailbox { capacity: 4 }
    send fn restock(qty: Int) { println("restock ${qty}") }
    send fn clear() {}
}

export namespace Inventory {
    export struct Note { text: Str }
}
//= stdout
//= restock 2
//= noted
//= restock 3
