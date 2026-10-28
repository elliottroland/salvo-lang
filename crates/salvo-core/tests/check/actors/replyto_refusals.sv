// [actor-replyto] The two refusals the parked-continuation slice adds, both
// checker-side. A handler that parks may only be **spawned** (user decision
// 2026-09-15): bound with `use` its members run inline, so the answer would
// have no mailbox to arrive on and no dispatcher to run it — the continuation
// would silently never run. And a **remote** mint — `k` naming a member of
// another actor effect in scope — is the generalized form of a later slice,
// named rather than mis-resolved.

//--- parking_handler_use_bound
actor effect Db {
    //~ warning[copy-scalar-free:moved-scalar]
    send fn lookup(id: Int, out: Reply<Str>) => !id, !out
}

actor effect Notices {
    //~ warning[copy-scalar-free:moved-scalar]
    send fn fetch(id: Int) => !id
    //~ warning[copy-scalar-free:moved-scalar]
    send fn arrived(id: Int, text: Str) => !id, !text
}

handler Rows() of Db {
    mailbox { capacity: 8 }

    send fn lookup(id: Int, out: Reply<Str>) { out.send("r") }
}

handler Fetching() [Db] of Notices {
    mailbox { capacity: 8 }

    send fn fetch(id: Int) {
        lookup(copy(id), replyto arrived(id))
    }
    send fn arrived(id: Int, text: Str) {}
}

fn main() [use, spawn] {
    let rows = spawn Rows() on pool(1)
    use rows
    //~ error[actor-replyto:use-bound]
    use Fetching()
    fetch(9)
}

//--- remote_mint
actor effect Db {
    //~ warning[copy-scalar-free:moved-scalar]
    send fn lookup(id: Int, out: Reply<Str>) => !id, !out
    //~ warning[copy-scalar-free:moved-scalar]
    send fn arrived(id: Int, text: Str) => !id, !text
}

actor effect Ask {
    //~ warning[copy-scalar-free:moved-scalar]
    send fn go(id: Int) => !id
}

handler Rows() of Db {
    mailbox { capacity: 8 }

    send fn lookup(id: Int, out: Reply<Str>) { out.send("r") }
    send fn arrived(id: Int, text: Str) {}
}

handler Asking() [Db] of Ask {
    mailbox { capacity: 4 }

    send fn go(id: Int) {
        //~ error[actor-replyto:other-actor-member]
        lookup(copy(id), replyto arrived(id))
    }
}

fn main() [use, spawn] {
    let rows = spawn Rows() on pool(1)
    let a = spawn Asking() with rows on pool(1)
    a.go(1)
}
