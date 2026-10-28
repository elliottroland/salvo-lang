// [actor-send-fn] Send members: what they may declare. A `send fn` is a
// *message*: sending it enqueues an invocation and answers nothing, so a
// written return type is an error naming the shape that does carry an answer —
// an `out: Reply<T>` parameter. Checked on the effect's declaration and on a
// handler's implementation of it, because either can be where the mistake is
// written.
actor effect Totals {
    //~ error[actor-send-fn:return-type]
    send fn total() -> Int
}

actor effect Bumps {
    send fn bump(n: Int)
}

handler Returning() of Bumps {
    mailbox { capacity: 8 }

    //~ error[actor-send-fn:return-type]
    send fn bump(n: Int) -> Int {
        return n
    }
}

// The legal shape: no return type, and a reply travels as a parameter. This
// must stay *clean* — it is the first-pass surface every later slice builds
// on.
handler Counting() of Bumps {
    mailbox { capacity: 8 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
    }
}

// [actor-spawn-effect] `[spawn]` is accepted where a body may create a process
// — on a function, and on a handler's dependency list (a supervisor spawns its
// children). (Its refusal on a fn *type* is `member_tests.rs`'s: it is
// reported four times.)
handler Supervising() [spawn] of Bumps {
    mailbox { capacity: 8 }

    send fn bump(n: Int) {}
}

fn start() [use, spawn] {
    let _n = 0
}
