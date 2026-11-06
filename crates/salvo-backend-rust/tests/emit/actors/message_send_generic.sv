// [actor-sent] A message `send` over a protocol only a type parameter names
// cannot be lowered — a generic fn has no protocol to switch on — so it is
// refused, never emitted as the reply's `send`, its overload of the name.
// A handler `for E` is built per protocol and sends freely.
actor effect Inventory {
    send fn restock(qty: Int)
    send fn clear()
}

//~ refused[rs-ir:unsupported-node]
fn relay<E>(to: eff[E], m: Message<E>) [] -> None => to, !m {
    send(to, m)
}

fn main() [use, spawn] {
    let s = spawn Quiet() on pool(1)
    relay(s, Inventory.Clear {})
}

handler Quiet() for Inventory {
    mailbox { capacity: 4 }
    send fn restock(qty: Int) {}
    send fn clear() {}
}
