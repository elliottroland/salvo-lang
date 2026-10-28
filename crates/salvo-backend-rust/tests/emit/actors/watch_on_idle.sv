// `watch` and `on_idle` as Salvo over the core token.

//--- watch_is_salvo_over_the_core_token
// [actor-watch] [actor-on-idle] [runtime-handles] `watch` and `on_idle` are
// Salvo: the program calls `std.core.actor`'s fns, which take the core's token
// out of the reply token (`take_local()`) and register it; the scheduler
// builds `Exit` and `Idle` itself, so no builder travels with the call.
actor effect Counter {
    send fn bump(n: Int) => !n
    send fn crash()
}

handler Counting() of Counter {
    mailbox { capacity: 8 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
    }

    // A faulted activation is what death *is* — there is no `kill`, and a
    // Salvo-level throw cannot cross a member boundary.
    send fn crash() {
        let xs: List<Int> = List(1)
        sum = sum + (get(xs, 9) ?: unreachable!())
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let c = spawn Counting() on pool(1)
    c.bump(2)
    let died = waitfor out: Reply<Exit> {
        //~ emits(main.rs): crate::std_core_actor::watch(c, out)
        //~ !emits(main.rs): salvo_watch(
        //~ emits(std/core/actor.rs): .take_local()
        watch(c, out)
        c.crash()
    }
    println("died with a reason: ${size(died.reason) > 0}")
    let again = waitfor out: Reply<Exit> {
        watch(c, out)
    }
    println("late watch answered: ${size(again.reason) > 0}")
    // Sends to the dead are silent no-ops, so this changes nothing and the
    // program ends normally.
    c.bump(3)
    println("done")
}

//--- on_idle_is_salvo_over_the_core_token
// [actor-watch] [actor-on-idle] [runtime-handles] `watch` and `on_idle` are
// Salvo: the program calls `std.core.actor`'s fns, which take the core's token
// out of the reply token (`take_local()`) and register it; the scheduler
// builds `Exit` and `Idle` itself, so no builder travels with the call.
actor effect Desk {
    send fn ask(out: Reply<Str>) => !out
}

// Parks every token it is given and answers none, which is legal because the
// obligation lives in state [linear-state] — and is exactly the shape a
// quiescence report has to be able to name.
handler Desking() of Desk {
    mailbox { capacity: 4 }

    state {
        waiting: Mut List<Reply<Str>> = Mut List()
    }

    send fn ask(out: Reply<Str>) {
        add(waiting, out)
    }
}

actor effect Client {
    send fn go(desk: eff[Desk]) => !desk
    send fn answered(word: Str) => !word
}

// The gate: while the continuation is outstanding this actor serves nothing
// else, so its mailbox is stalled for good.
handler Clienting() of Client {
    mailbox { capacity: 4 }

    send fn go(desk: eff[Desk]) {
        desk.ask(replyto! answered())
    }

    send fn answered(word: Str) {
        discard(word)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let p = pool(1)
    let desk = spawn Desking() on p
    let client = spawn Clienting() on p
    //~ emits(main.rs): crate::std_core_actor::on_idle(
    //~ !emits(main.rs): salvo_watch(
    //~ emits(std/core/actor.rs): .take_local()
    let settled = waitfor i: Reply<Idle> { on_idle(p, i) }
    println("settled: gates ${settled.parked_gates}, tokens ${settled.parked_tokens}")
    client.go(desk)
    let stuck = waitfor i: Reply<Idle> { on_idle(p, i) }
    println("stuck: gates ${stuck.parked_gates}, tokens ${stuck.parked_tokens}")
}
