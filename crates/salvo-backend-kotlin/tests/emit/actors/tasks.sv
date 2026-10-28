// Actor state, tasks and faces [kt-actor] [kt-task].

//--- draining_state_lowers_to_a_foreach_kotlin
// [kt-actor] [linear-container] The lowering: Kotlin needs no `mem::take`
// equivalent — objects are references, and the checker made the member assign
// a fresh list before it returned — so a drain is std's Salvo `drain` over the
// field, followed by the fresh list [platform-value-type].
// [linear-container] [linear-state] **Obligations in a collection**
// (`LINEAR_QUEUE` in codegen_tests.rs).
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
        //~ emits: salvo.std.core.list.drain(waiting, fun(
        //~ emits: waiting = mutableListOf<
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

//--- an_inherited_effect_is_captured_at_the_mint
// [task-effects] [kt-task] The inherited handler is bound to a `val` outside
// the lambda and passed as the leading argument, so what the task runs with is
// the handler registered at the mint.
// [task-effects] A task body's effects are inherited from the frame that
// minted it (user decision 2026-09-26), `TASK_EFFECTS` in codegen_tests.rs.
// A task body still takes the handler it performs through.
//~ emits: fun report(console: salvo.std.core.console.Console
send fn report(out: Reply<Int>, n: Int) [Console] => !out, !n {
    println("task saw ${n}")
    out.send(n * 2)
}

send fn chain(out: Reply<Int>, n: Int) [Console] => !out, !n {
    println("chain saw ${n}")
    let inner = waitfor r: Reply<Int> {
        let k = replyto report(r)
        k.send(n + 1)
    }
    out.send(inner)
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let direct = waitfor a: Reply<Int> {
        //~ emits: val __e0 = 
        //~ emits: report(__e0,
        let k = replyto report(a)
        k.send(21)
    }
    println("direct ${direct}")
    let nested = waitfor b: Reply<Int> {
        let k = replyto chain(b) on pool(2)
        k.send(10)
    }
    println("nested ${nested}")
}

//--- a_task_mint_lowers_to_a_scheduled_lambda
// [kt-task] The lowering, mirroring the Rust backend: the lambda is the
// continuation, and no `__Cont_` class is generated for a task mint.
// [actor-free-send-fn] [task-mint] [task-pool-inherit] The **task kernel**, as a
// program (`TASK_KERNEL` in codegen_tests.rs).
actor effect Db {
    send fn query(id: Int, reply: Reply<Int>) => !reply
}

handler Rows() of Db {
    mailbox { capacity: 4 }

    send fn query(id: Int, reply: Reply<Int>) {
        reply.send(id * 10)
    }
}

// The free send fn: no return type, every parameter consumed, and it runs on a
// pool rather than in anyone's frame. Its trailing parameter is the answer.
send fn finish(label: Str, out: Reply<Str>, row: Int) => !label, !out, !row {
    out.send("${label}=${row}")
}

// An ordinary function — full features, synchronous — that wires future work
// and returns. It never parks and never blocks, and it is called identically
// from `main` and from anywhere else.
fn fetch(id: Int, out: Reply<Str>) [Db] -> None => !out {
    // An omitted `on` clause inherits the current pool; the continuation is a
    // trailing lambda (after the answer decoder).
    //~ emits: salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), 
    //~ emits: ) { __v -> salvo.main.finish(
    // [net-addr-routable] The actor's `decodeReply` *reads* the parked table
    // for a wire reply; what a task mint must not do is *write* one.
    //~ !emits: __parked[__s] =
    query(id, replyto finish("row", out))
}

fn fetch_on(id: Int, out: Reply<Str>, p: Pool) [Db] -> None => !out, p {
    query(id, replyto finish("placed", out) on p)
}

fn main() [use, spawn] {
    use StdConsole()
    let workers = pool(2)
    let db = spawn Rows() on workers
    use db

    // The continuation inherits `main`'s pool, so it runs on main's own thread
    // while main waits [waitfor-pump].
    println(waitfor a: Reply<Str> { fetch(7, a) })
    // And here it runs on the worker pool instead.
    println(waitfor b: Reply<Str> { fetch_on(4, b, workers) })
}

//--- several_faces_lower_to_one_actor_with_a_dispatcher_each_kotlin
// [effect-handler-multi] [kt-actor] What the two faces lower to, mirroring the
// Rust backend: one class implementing both interfaces, one dispatcher per
// protocol, a `handle` that asks which protocol the message belongs to, and a
// spawn whose value is a `Pair` of the same scheduler id.
// [effect-handler-multi] Handlers of several effects (`MULTI_FACE` in
// codegen_tests.rs).
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

//~ emits: class ManualTime : Timer, TimerCtl {
//~ emits: fun __dispatch_ManualTime_Timer(__handler: ManualTime, __msg: __Msg_Timer)
//~ emits: fun __dispatch_ManualTime_TimerCtl(__handler: ManualTime, __msg: __Msg_TimerCtl)
//~ emits: is __Msg_Timer -> salvo.main.__dispatch_ManualTime_Timer(handler, msg)
//~ emits: is __Msg_TimerCtl -> salvo.main.__dispatch_ManualTime_TimerCtl(handler, msg)
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

//~ emits: class Counting : Tally, Stats {
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
    //~ emits: val __a = salvo.SalvoSched.spawn(
    //~ emits: kotlin.Pair(__a, __a)
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

//--- watch_and_on_idle_are_salvo_over_the_core_token_kotlin_watch
// [actor-watch] [actor-on-idle] [runtime-handles] `watch` and `on_idle` are
// Salvo: the program calls `std.core.actor`'s fns, which take the core's token
// out of the reply token (`takeLocal()`) and register it; the scheduler
// builds `Exit` and `Idle` itself, so no builder travels with the call.
// (`WATCH` in codegen_tests.rs; `..._on_idle` below is `ON_IDLE`.)
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
        //~ emits: watch(c, out)
        //~ !emits: SalvoSched.watch
        //~ emits(std/core/actor.kt): .takeLocal()
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

//--- watch_and_on_idle_are_salvo_over_the_core_token_kotlin_on_idle
// [actor-watch] [actor-on-idle] [runtime-handles] The `on_idle` half of
// `watch_and_on_idle_are_salvo_over_the_core_token_kotlin`.
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
    //~ emits: onIdle(p, i)
    //~ !emits: SalvoSched.watch
    //~ emits(std/core/actor.kt): .takeLocal()
    let settled = waitfor i: Reply<Idle> { on_idle(p, i) }
    println("settled: gates ${settled.parked_gates}, tokens ${settled.parked_tokens}")
    client.go(desk)
    let stuck = waitfor i: Reply<Idle> { on_idle(p, i) }
    println("stuck: gates ${stuck.parked_gates}, tokens ${stuck.parked_tokens}")
}

//--- a_parked_continuation_lowers_to_a_slot_table_kotlin
// [kt-actor] [actor-replyto] The lowering, mirroring the Rust backend's: a
// continuation class beside the message class, the two generated fields on the
// handler — **not** `private`, since `__Actor_H` is a different class and
// Kotlin's class-level `private` does not reach across one — a `__dispatch`
// factored out of `handle`, and a `resume` that pops the slot and casts the
// answer to the target member's trailing parameter type.
// (`REPLYTO_CHAIN` in codegen_tests.rs.)
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

//~ emits: sealed class __Cont_Fetching {
//~ emits: class Arrived(val out: salvo.SalvoReply) : __Cont_Fetching()
//~ emits: var __addr: Int? = null
//~ emits: val __parked: MutableMap<Long, __Cont_Fetching> = kotlin.collections.mutableMapOf()
//~ emits: handler.__addr = ctx.addr
//~ emits: val c = handler.__parked.remove(slot) ?: return
//~ emits: value as String
handler Fetching() [Db] of Notices {
    mailbox { capacity: 4 }

    send fn fetch(id: Int, out: Reply<Str>) {
        //~ emits: salvo.SalvoSched.mint(__addr!!)
        //~ emits: __parked[__s] =
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

//--- a_stub_implements_the_effect_by_sending_kotlin
// [actor-use-addr] The forwarding stub: the same program the Rust backend
// runs (`ADDR_STUB` in codegen_tests.rs).
//~ emits: class __Stub_Log(private val addr: Int) : Log
actor effect Log {
    send fn note(what: Str) => !what
    send fn count(out: Reply<Int>) => !out
}

handler Counting() of Log {
    mailbox { capacity: 8 }

    state {
        seen: Int = 0
    }

    send fn note(what: Str) {
        seen = seen + 1
    }

    send fn count(out: Reply<Int>) {
        out.send(seen)
    }
}

fn work() [Log] {
    note("a")
    note("b")
}

fn main() [use, spawn] {
    use StdConsole()
    let logger = spawn Counting() on pool(1)
    //~ emits: __Stub_Log(
    use logger
    work()
    let n = waitfor out: Reply<Int> {
        count(out)
    }
    println("noted ${n}")
}

//--- a_dependent_spawn_hands_the_child_its_dependencies
// [kt-actor] [kt-handle] The shape: the handler holds its dependencies as
// fields typed by the effect interfaces, the actor class is the same for
// every handler, and the spawn site passes the clause's instances as
// trailing constructor arguments — a construction for one dependency, a
// forwarding stub for the addr.
// [actor-spawn-expr] Dependent-handler spawns (`DEP_SPAWN` in
// codegen_tests.rs).
actor effect Log {
    send fn note(what: Str) => !what
    send fn dump(out: Reply<Str>) => !out
}

actor effect Tally {
    send fn tick(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

actor effect Counter {
    send fn bump(n: Int) => !n
    send fn report(out: Reply<Str>) => !out
}

handler Recording() of Log {
    mailbox { capacity: 1 }

    state {
        last: Str = "none"
    }

    send fn note(what: Str) {
        last = what
    }

    send fn dump(out: Reply<Str>) {
        out.send(copy(last))
    }
}

handler Summing() of Tally {
    mailbox { capacity: 8 }

    state {
        sum: Int = 0
    }

    send fn tick(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

//~ emits: class Counting(private val __dep0: Log, private val __dep1: Tally) : Counter
//~ emits: class __Actor_Counting(private val handler: Counting) : salvo.SalvoActor
handler Counting() [Log, Tally] of Counter {
    mailbox { capacity: 8 }

    send fn bump(n: Int) {
        note("bumped ${n}")
        tick(n)
    }

    send fn report(out: Reply<Str>) {
        dump(out)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let tally = spawn Summing() on pool(1)
    // [actor-mailbox] The instance is built into a local first, so the spawn
    // can read the bound off it before handing it over.
    //~ emits: val __h = Counting(__Mon_Log(Recording()), __Stub_Tally(tally))
    //~ emits: __h.__mailboxCapacity, __Actor_Counting(__h)
    let counter = spawn Counting() with Recording(), tally on pool(1)
    counter.bump(2)
    counter.bump(3)
    let last = waitfor out: Reply<Str> {
        counter.report(out)
    }
    println("last ${last}")
    let sum = waitfor out: Reply<Int> {
        tally.total(out)
    }
    println("sum ${sum}")
}
