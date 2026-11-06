// [actor-message] [actor-sent] Messages as values: a handler that
// takes every message with `sent` and hands it on with `send`,
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
    send fn sent(m: Message<Inventory>) => !m {
        seen = seen + 1
        send(back, m)
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
    send fn sent(m: Message<E>) => !m {
        send(back, m)
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
        is .Done { "done" }
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let m = Inventory.Restock { qty: 2 }
    println(describe(m))
    let n = Inventory.Note { text: "noted" }
    println(n.text)
    let s = spawn Store() on pool(1)
    send(s, Inventory.Restock { qty: 3 })
    let _ = waitfor r: Reply<Int> { s.done(r) }
}
// FILE: shop.sv
export actor effect Inventory {
    send fn restock(qty: Int)
    send fn clear()
    send fn done(out: Reply<Int>) => !out
}

export handler Store() [Console] for Ordered Inventory {
    mailbox { capacity: 4 }
    send fn restock(qty: Int) { println("restock ${qty}") }
    send fn clear() {}
    send fn done(out: Reply<Int>) => !out { out.send(1) }
}

export namespace Inventory {
    export struct Note { text: Str }
}
//= stdout
//= restock 2
//= noted
//= restock 3

//--- a_message_known_to_be_one_arm
// [actor-sent] `send(to, m)` sends the member `m` is a message of. A value
// already known to be one arm (a literal, or `m` after `is .Restock`) is
// sent as that member, with no switch over the arms it cannot be (which
// narrowed a `Clear` to a `Restock` and read a field it does not have).
actor effect Inventory {
    send fn restock(qty: Int)
    send fn clear()
    send fn report(out: Reply<Str>) => !out
}

handler Store() for Ordered Inventory {
    mailbox { capacity: 8 }
    state { log: Str = "" }
    send fn restock(qty: Int) { log = "${log}restock ${qty}; " }
    send fn clear() { log = "${log}clear; " }
    send fn report(out: Reply<Str>) => !out { out.send(copy(log)) }
}

fn pick(m: Message<Inventory>) [] -> Message<Inventory> => !m {
    return m
}

fn main() [use, spawn] {
    use StdConsole()
    let s = spawn Store() on pool(1)
    send(s, Inventory.Clear {})
    send(s, Inventory.Restock { qty: 3 })
    let m = pick(Inventory.Restock { qty: 4 })
    when m {
        is .Restock { send(s, m) }
        is .Clear { }
        is .Report { send(s, m) }
    }
    println(waitfor out: Reply<Str> { send(s, Inventory.Report { out: out }) })
}
//= stdout
//= clear; restock 3; restock 4; 
