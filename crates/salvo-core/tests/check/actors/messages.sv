// [actor-message] `Message<E>`: one arm per send member, a struct of its
// parameters, named after the effect (`Inventory.Reserve`).
// [actor-sent] `send fn sent(m: Message<E>)` answers every send
// member of `E`, and `send(to, m)` hands a message on.
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
    send fn sent(m: Message<Inventory>) => !m {
        send(back, m)
    }
}

// Answering a message by hand: its reply moves out.
handler Answers() for Inventory {
    mailbox { capacity: 8 }
    send fn sent(m: Message<Inventory>) => !m {
        when m {
            is .Reserve { send(m.out, "no ${m.sku}") }
            is .Restock { }
        }
    }
}

// All or nothing.
handler Both(back: eff[Inventory]) for Inventory {
    mailbox { capacity: 8 }
    send fn sent(m: Message<Inventory>) => !m {
        send(back, m)
    }
    //~ error[actor-sent:both]
    send fn restock(qty: Int) {}
}

// A message holding a reply owes it.
handler Leaky() for Inventory {
    mailbox { capacity: 8 }
    //~ error[ir-linear-check:leak-at-exit]
    send fn sent(m: Message<Inventory>) => !m {
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

//--- send members are not overloaded
// [actor-message] Each send member is one `Message<E>` arm named after it,
// so an actor effect's send members have distinct names, and names that
// make distinct arms (user decision 2026-11-05). The same name on two faces
// is unaffected: each face has its own `Message<E>`. (A handler's private
// send members were refused overloading already,
// `actor-mixed-handler:overloaded-send`.)
actor effect Sink {
    send fn put(a: Int)
    //~ error[actor-message:overloaded-send]
    send fn put(a: Str) => !a
    send fn get_all(out: Reply<Int>) => !out
    //~ error[actor-message:arm-collision]
    send fn getAll(out: Reply<Int>) => !out
}

actor effect Ping {
    send fn ping(n: Int)
}

actor effect Pong {
    send fn ping(s: Str) => !s
}

handler Both() for Ping, Pong {
    mailbox { capacity: 4 }
    send fn ping(n: Int) {}
    send fn ping(s: Str) {}
}

//--- sent is no call target
// [actor-sent] `sent` is the form a handler takes its messages in, recognised
// by its name and its shape; it is no member a send, a servant send or a
// mint can name. A self-send of a message is `send(self@E, m)`. A member
// named `sent` of another shape is an ordinary member.
actor effect Inventory {
    send fn restock(qty: Int)
    send fn clear()
}

handler Selfish() for Inventory {
    mailbox { capacity: 8 }
    send fn sent(m: Message<Inventory>) => !m {
        when m {
            is .Restock {
                //~ error[actor-sent:called]
                sent@self(Inventory.Clear {})
                send(self@Inventory, Inventory.Clear {})
            }
            is .Clear { }
        }
    }
}

actor effect Inbox {
    send fn sent(n: Int)
}

handler Counting() for Inbox {
    mailbox { capacity: 8 }
    send fn sent(n: Int) {}
}

fn poke(i: eff[Inbox]) [] -> None => i {
    i.sent(1)
}
