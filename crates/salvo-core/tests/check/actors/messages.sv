// [actor-message] `Message<E>`: one arm per send member, a struct of its
// parameters, named after the effect (`Inventory.Reserve`).
// [actor-receive] `send fn receive(m: Message<E>)` answers every send
// member of `E`, and `forward(to, m)` hands a message on.
export comptime qualifier Key for Field

actor effect Inventory {
    send fn reserve(Key sku: Str, qty: Int, out: Reply<Str>) => !sku, !out
    send fn restock(qty: Int)
}

actor effect Ping {
    send fn ping()
}

// A message is a union of its arms, narrowed like any union.
fn describe(m: Message<Inventory>) [] -> Str => m {
    return when m {
        is .Reserve { "reserve ${m.qty} of ${m.sku}" }
        is Inventory.Restock { "restock ${m.qty}" }
    }
}

// A protocol of one send is that one arm.
fn ping_text(m: Message<Ping>) [] -> Str => m {
    return "${m is Ping.Ping}"
}

handler Front(back: eff[Inventory]) for Inventory {
    mailbox { capacity: 8 }
    send fn receive(m: Message<Inventory>) => !m {
        forward(back, m)
    }
}

// Answering a message by hand: its reply moves out.
handler Answers() for Inventory {
    mailbox { capacity: 8 }
    send fn receive(m: Message<Inventory>) => !m {
        when m {
            is .Reserve { send(m.out, "no ${m.sku}") }
            is .Restock { }
        }
    }
}

// All or nothing.
handler Both(back: eff[Inventory]) for Inventory {
    mailbox { capacity: 8 }
    send fn receive(m: Message<Inventory>) => !m {
        forward(back, m)
    }
    //~ error[actor-receive:both]
    send fn restock(qty: Int) {}
}

// A message holding a reply owes it.
handler Leaky() for Inventory {
    mailbox { capacity: 8 }
    //~ error[ir-linear-check:leak-at-exit]
    send fn receive(m: Message<Inventory>) => !m {
    }
}

// A comptime qualifier stands before a send fn's parameter only.
//~ error[comptime-qualifier:on-param]
fn keyed(Key k: Str) [] -> Str => !k {
    return k
}

fn main() {
    let _a = describe(Inventory.Restock { qty: 1 })
    let _b = ping_text(Ping.Ping {})
}
