// Actors: effect handlers bound asynchronously.
//
// An **actor effect** is an ordinary effect whose members are `send fn`s: a
// call on one *enqueues* an invocation and answers nothing. A handler of that
// effect is bound with `spawn` instead of `use`, which is the whole difference
// — the same handler, the same members, the same state, running one activation
// at a time on a scheduler instead of inline on the caller's thread.
//
// What this program is chosen to show, in the order it needs it:
//
//   1. the smallest actor: state, a send, and `main`'s bridge back
//   2. request/response: a linear reply token, minted and answered
//   3. an actor whose answer needs *another* actor — the continuation form
//   4. an actor holding a queue of obligations, drained on shutdown
//   5. death, and watching for it
//   6. the same handler bound synchronously, which is not an actor at all
//
// The sibling example `effects/` shows the synchronous half of this surface;
// nothing here changes how effects are declared or called.

// ===== 1. the smallest actor =====
//
// `send fn` says the call enqueues. It answers nothing, so a member that has
// something to say takes a `Reply<T>` for it (section 2) — and because the
// invocation crosses to another thread, its payload is consumed: the clause
// says `=> !n`.
actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

// An ordinary handler. Its state is the actor's, reachable from nowhere else,
// and every member runs to completion before the next one starts — so
// `sum = sum + n` needs no lock and cannot interleave.
handler Counting() of Counter {
    mailbox { capacity: 8 }

    sum: Int = 0

    send fn bump(n: Int) {
        sum = sum + n
    }

    // ===== 2. request/response =====
    //
    // `Reply<Int>` is a one-shot answer channel, and it is **linear**: this
    // member must send to it exactly once, on every path, or the program does
    // not compile. That is the guarantee the whole surface rests on.
    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

// ===== 3. an answer that needs another actor =====
//
// A `Ledger` cannot answer `report` by itself: the number lives in the
// counter. So it asks, and **parks a continuation** for the reply —
// `replyto reported(out)` mints a token aimed at its own `reported` member,
// carrying `out` (the original caller's token) as a capture. The activation
// then ends. When the counter answers, `reported` runs as its own later
// activation and fulfils `out`.
//
// `main` is not involved: the answer travels actor to actor.
actor effect Ledger {
    send fn report(label: Str, out: Reply<Str>) => !label, !out
    send fn reported(label: Str, out: Reply<Str>, total: Int) => !label, !out, !total
}

// The handler declares what it depends on exactly as a synchronous one does —
// `[Counter]` — and the spawn site supplies it. The child never learns whether
// its `Counter` is another actor or a local handler.
handler Bookkeeping() [Counter] of Ledger {
    mailbox { capacity: 4 }

    send fn report(label: Str, out: Reply<Str>) {
        total(replyto reported(label, out))
    }

    send fn reported(label: Str, out: Reply<Str>, total: Int) {
        out.send("${label}=${total}")
    }
}

// ===== 4. a queue of obligations =====
//
// A `Reply<T>` is linear, and a `Mut List<Reply<Str>>` is therefore linear
// too: the queue owes, and `drain` is what ends it. This is the shape a real
// actor uses to hold requests it cannot answer yet.
actor effect Desk {
    send fn ticket(out: Reply<Str>) => !out
    send fn serve(name: Str) => !name
    send fn close_up(reason: Str) => !reason
}

// The bound as a **constructor parameter**: the slot's expressions may read
// them (and nothing else — it is computed before the actor exists), which is
// how a caller chooses a per-instance queue depth without a spawn-site clause.
handler Desking(room: Int) of Desk {
    mailbox { capacity: room }

    waiting: Mut List<Reply<Str>> = mut_list_of()

    send fn ticket(out: Reply<Str>) {
        add(waiting, out)
    }

    // One obligation leaves the queue. `remove_first` *moves* it out and
    // answers `Reply<Str>?`, so the emptiness check is the ordinary narrow and
    // the `None` arm owes nothing.
    send fn serve(name: Str) {
        let next = remove_first(waiting)
        when next {
            is Reply<Str> { next.send("served ${name}") }
            is None { discard(name) }
        }
    }

    // Shutdown answers everyone still waiting, and puts a fresh queue back:
    // an activation may take state out, but it may not *return* with a hole
    // in it — the actor owns those obligations until it ends.
    send fn close_up(reason: Str) {
        drain(waiting, r -> send(r, "closed: ${reason}"))
        waiting = mut_list_of()
    }
}

// ===== 5. death =====
//
// Death is a **faulted activation** and nothing else: there is no `kill`, and
// a Salvo-level `throw` cannot cross a member boundary. `watch` is the whole
// monitor surface — one function, one `Exit` struct — because a death
// notification is itself an answer, so the reply machinery already carries it.
actor effect Fragile {
    send fn crash()
}

handler Breaking() of Fragile {
    mailbox { capacity: 1 }

    send fn crash() {
        let empty: List<Int> = []
        // Reading past the end answers `None`; asserting it is the fault.
        let boom = get(empty, 7)!
        discard(boom)
    }
}

// ===== 6. the same handler, bound synchronously =====
//
// There is no section 6 declaration, and that is the point: `use Counting()`
// binds the *same* handler inline on this thread — no mailbox, no scheduler,
// no addr — and `Counter`'s callers cannot tell the difference. Asynchrony is
// a property of the **binding**, not of the handler. See the end of `main`.

fn main() [use, spawn] {
    use StdOutConsole()

    // A pool is an ordinary value: one pool, several actors on it.
    let workers = pool(2)

    // 1 — spawn, send, and bridge back. The queue depth is not here: the
    // handler declared it (`mailbox { capacity: 8 }`), so a spawn says what to
    // run, what it depends on, and where.
    let counter = spawn Counting() on workers
    counter.bump(2)
    counter.bump(3)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    println("1. counter total is ${sum}")

    // 3 — an actor that depends on another actor. The dependency arrives as
    // an addr in the spawn's `use` clause; a construction (`use Counting()`)
    // would have been the same act.
    let ledger = spawn Bookkeeping() use counter on workers
    let line = waitfor out: Reply<Str> {
        ledger.report("counter", out)
    }
    println("3. ledger says ${line}")

    // 4 — two waiters, one served, the rest answered by the drain. Arrival
    // order is the whole ordering story: these four sends are served in the
    // order they were made, by one actor, one at a time.
    let desk = spawn Desking(8) on workers
    let first = waitfor a: Reply<Str> {
        desk.ticket(a)
        let second = waitfor b: Reply<Str> {
            desk.ticket(b)
            desk.serve("ada")
            desk.close_up("end of day")
        }
        println("4. second waiter got: ${second}")
    }
    println("4. first waiter got: ${first}")

    // 5 — watch, then kill it with a fault. The `Exit`'s reason is the host's
    // text (a panic message on Rust, an exception's on Kotlin), so this prints
    // only *that* there was one.
    let fragile = spawn Breaking() on workers
    let exit = waitfor gone: Reply<Exit> {
        watch(fragile, gone)
        fragile.crash()
    }
    println("5. it died with a reason: ${size(exit.reason) > 0}")
    // A send to a dead actor is a silent no-op, so this changes nothing.
    fragile.crash()

    // 6 — the same handler, inline. `use` binds it here, so these two bumps
    // and the total run on *this* thread, in this order, with no mailbox
    // involved. `waitfor` still works: the token is answered before
    // `total(out)` returns, so nothing ever blocks.
    use Counting()
    bump(4)
    bump(5)
    let inline = waitfor out: Reply<Int> {
        total(out)
    }
    println("6. inline total is ${inline}")

    println("done")
}
