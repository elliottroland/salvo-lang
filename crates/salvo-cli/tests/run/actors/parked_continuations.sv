// [actor-replyto] [actor-self-send] parked continuations
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- obligations_in_a_collection

actor effect Desk {
    send fn ticket(out: Reply<Str>) => !out
    send fn serve(name: Str) => !name
    send fn close_up(reason: Str) => !reason
}

handler Desking() of Desk {
    mailbox { capacity: 8 }

    state {
        waiting: Mut List<Reply<Str>> = Mut List()
    }

    send fn ticket(out: Reply<Str>) {
        add(waiting, out)
    }

    // One obligation leaves the queue, and the `None` arm owes nothing.
    send fn serve(name: Str) {
        let next = remove_first(waiting)
        when next {
            is Reply<Str> { next.send("served ${name}") }
            is None { discard(name) }
        }
    }

    // The terminal: every parked token is answered, and the state is whole
    // again before the activation ends.
    send fn close_up(reason: Str) {
        drain(waiting, r -> send(r, "closed: ${reason}"))
        waiting = Mut List()
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let desk = spawn Desking() on pool(1)
    let first = waitfor a: Reply<Str> {
        desk.ticket(a)
        let second = waitfor b: Reply<Str> {
            desk.ticket(b)
            desk.serve("ada")
            desk.close_up("end of day")
        }
        println("second ${second}")
    }
    println("first ${first}")
}
//= stdout
//= second closed: end of day
//= first served ada

//--- a_handler_of_several_effects

// [effect-handler-multi] One actor, one mailbox, one owner of the state, two
// typed faces: the public protocol and the test-control one. Least authority
// falls out of the types — a holder of `timer` cannot name `advance`.
actor effect Timer {
    send fn after(millis: Int, out: Reply<Str>) => !out
}

actor effect TimerCtl {
    send fn advance(millis: Int) => !millis
    send fn pending(out: Reply<Int>) => !out
}

handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 8 }

    state {
        waiting: Mut List<Reply<Str>> = Mut List()
        now: Int = 0
    }

    send fn after(millis: Int, out: Reply<Str>) {
        add(waiting, out)
    }

    send fn advance(millis: Int) {
        now = now + millis
        let at = now
        drain(waiting, r -> send(r, "fired at ${at}"))
        waiting = Mut List()
    }

    send fn pending(out: Reply<Int>) {
        out.send(size(waiting))
    }
}

// The synchronous half of the same shape: a public face and an admin face over
// one piece of state, bound by one `use`.
effect Tally {
    fn bump(n: Int) -> None => !n
}

effect Stats {
    fn total() -> Int
}

handler Counting() of Tally, Stats {
    state {
        sum: Int = 0
    }

    fn bump(n: Int) {
        sum = sum + n
    }

    fn total() -> Int {
        return sum
    }
}

fn count() [Tally, Stats] -> Int {
    bump(2)
    bump(3)
    return total()
}

fn main() [use, spawn] {
    use StdConsole()
    let (timer, ctl) = spawn ManualTime() on pool(1)
    let idle = waitfor c: Reply<Int> { ctl.pending(c) }
    println("pending ${idle}")
    let fired = waitfor f: Reply<Str> {
        timer.after(10, f)
        ctl.advance(10)
    }
    println(fired)
    let left = waitfor c: Reply<Int> { ctl.pending(c) }
    println("pending ${left}")
    use Counting()
    println("total ${count()}")
}
//= stdout
//= pending 0
//= fired at 10
//= pending 0
//= total 5

//--- a_quiescence_hook

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
    let settled = waitfor i: Reply<Idle> { on_idle(p, i) }
    println("settled: gates ${settled.parked_gates}, tokens ${settled.parked_tokens}")
    client.go(desk)
    let stuck = waitfor i: Reply<Idle> { on_idle(p, i) }
    println("stuck: gates ${stuck.parked_gates}, tokens ${stuck.parked_tokens}")
}
//= stdout
//= settled: gates 0, tokens 0
//= stuck: gates 1, tokens 1

//--- a_parked_continuation

actor effect Db {
    send fn lookup(id: Int, out: Reply<Str>) => !id, !out
}

actor effect Notices {
    send fn fetch(id: Int, out: Reply<Str>) => !id, !out
    send fn arrived(out: Reply<Str>, text: Str) => !out, !text
}

handler Rows() of Db {
    mailbox { capacity: 4 }

    send fn lookup(id: Int, out: Reply<Str>) {
        out.send("row ${id}")
    }
}

handler Fetching() [Db] of Notices {
    mailbox { capacity: 4 }

    send fn fetch(id: Int, out: Reply<Str>) {
        lookup(id, replyto arrived(out))
    }

    send fn arrived(out: Reply<Str>, text: Str) {
        out.send("got ${text}")
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let p = pool(2)
    let rows = spawn Rows() on p
    let fetcher = spawn Fetching() with rows on p
    let answer = waitfor out: Reply<Str> {
        fetcher.fetch(7, out)
    }
    println(answer)
}
//= stdout
//= got row 7

//--- a_gated_continuation

actor effect Echo {
    send fn ping(out: Reply<Str>) => !out
}

actor effect Trace {
    send fn start()
    send fn arrived(text: Str) => !text
    send fn note(what: Str) => !what
    send fn report(out: Reply<Str>) => !out
}

handler Echoing() of Echo {
    mailbox { capacity: 4 }

    send fn ping(out: Reply<Str>) {
        out.send("R")
    }
}

handler Tracing() [Echo] of Trace {
    mailbox { capacity: 4 }

    state {
        steps: Mut List<Str> = Mut List()
    }

    send fn start() {
        ping(replyto! arrived())
    }

    send fn arrived(text: Str) {
        add(steps, "reply ${text}")
    }

    send fn note(what: Str) {
        add(steps, "user ${what}")
    }

    send fn report(out: Reply<Str>) {
        out.send(join(steps, ", "))
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let p = pool(3)
    let echo = spawn Echoing() on p
    let tracer = spawn Tracing() with echo on p
    tracer.start()
    tracer.note("late")
    let got = waitfor out: Reply<Str> {
        tracer.report(out)
    }
    println(got)
}
//= stdout
//= reply R, user late

//--- both_readings_of_a_self_send

actor effect Steps {
    send fn begin(n: Int, out: Reply<Str>) => !n, !out
    send fn again(n: Int, out: Reply<Str>) => !n, !out
}

handler Stepping() of Steps {
    mailbox { capacity: 4 }

    state {
        steps: Mut List<Str> = Mut List()
    }

    send fn begin(n: Int, out: Reply<Str>) {
        add(steps, "begin ${n}")
        again@self(n + 1, out)
    }

    send fn again(n: Int, out: Reply<Str>) {
        add(steps, "again ${n}")
        out.send(join(steps, ", "))
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let s = spawn Stepping() on pool(1)
    let spawned = waitfor out: Reply<Str> {
        s.begin(1, out)
    }
    println("spawned ${spawned}")
    use Stepping()
    let inline = waitfor out: Reply<Str> {
        begin(1, out)
    }
    println("inline ${inline}")
}
//= stdout
//= spawned begin 1, again 2
//= inline begin 1, again 2

//--- a_death_watch

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
//= stdout
//= died with a reason: true
//= late watch answered: true
//= done
