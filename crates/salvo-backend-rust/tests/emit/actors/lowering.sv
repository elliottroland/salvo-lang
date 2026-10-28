// [rs-actor] What asynchronous effect handlers lower to: message enums,
// actor bodies, handles, servants and façades, parked continuations.

//--- an_actor_lowers_to_a_message_enum_and_a_body
// [actor-spawn-expr] [actor-use-addr] [actor-waitfor] The first program that
// *runs* a process: a counter handler bound asynchronously, two sends, then
// `main`'s bridge asking for the total.
//
// [rs-actor] What the lowering *is*, asserted on the generated text so a
// regression names itself: a message enum per protocol, an actor struct
// wrapping the handler, and the scheduler module carried into the output.
//~ emits: pub enum __Msg_Counter {
actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

//~ emits: impl crate::scheduler::SalvoActor for __Actor_Counting
handler Counting() of Counter {
    mailbox { capacity: 8 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    //~ emits: crate::scheduler::salvo_spawn(
    // The scheduler module is part of the program.
    //~ emits(scheduler.rs): pub trait SalvoActor
    let counter = spawn Counting() on pool(1)
    counter.bump(2)
    counter.bump(3)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    println("sum ${sum}")
}

//--- a_monitor_lowers_to_the_effects_handle
// [actor-monitor-handler] [rs-handle] The monitor spawn (SH-3, user decision
// 2026-09-19): a **plain** effect's handler shared behind a lock. One
// instance serves `main` (use-bound) and a spawned actor (supplied through
// the dependency clause), and the output is deterministic because the three
// draws are sequenced by the `waitfor`.
//
// [rs-handle] What the monitor lowering *is*, asserted on the
// generated text: the per-effect handle (`Arc<Mutex<dyn E + Send>>`)
// implementing the effect's trait by lock-and-delegate, the spawn building
// one with no scheduler call, and the handle passing into the actor's
// dependency clause as itself (cloned, not stub-wrapped).
// The handle.
//~ emits: pub struct Random {
//~|     inner: __Inner_Random,
//~ emits: Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Random>>),
// It implements the effect, each member locking and delegating.
//~ emits: pub fn next(&self) -> i32 {
//~ emits: __Inner_Random::Locked(h) => h.lock().unwrap().next(),
// A plain effect gets the handle and nothing else.
//~ !emits: __Stub_Random
//~ !emits: __Lock_
//~ !emits: __Share_
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    state {
        cursor: Int = 0
    }

    fn next() -> Int {
        cursor = (cursor * 31 + seed) % 100000
        return cursor
    }
}

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    // The monitor spawn.
    //~ emits: let mut rng: crate::Random = crate::Random::locked(crate::CyclicRandom::new(12345i32));
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng on pool(1)
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    println("actor drew ${drawn}")
    println("main drew ${next()}")
}

//--- a_mixed_handler_lowers_to_a_servant_and_a_facade
// [actor-mixed-handler] [rs-mixed] The mixed handler (SH-1): state confined to
// `send fn` members (the servant), sync members on the caller's thread (the
// façade) sending to it and waiting — one shared instance serving `main`
// and a spawned actor, behind a plain effect nobody declares `[waitfor]`
// for.
//
// [rs-mixed] What the mixed lowering *is*: the handler-keyed message enum
// and actor body (the servant), the façade struct carrying the addr and
// ctor params, the façade send lowering, and the spawn answering the handle
// over the façade.
effect Random {
    fn next() -> Int
}

// The servant: its message enum and actor body.
//~ emits: pub enum __Priv_CyclicRandom {
//~ emits: impl crate::scheduler::SalvoActor for __Actor_CyclicRandom
// The façade, implementing the effect.
//~ emits: pub struct __Fac_CyclicRandom {
//~ emits: impl crate::__Stateless_Random for __Fac_CyclicRandom {
handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        cursor = (cursor * 31 + seed) % 100000
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            // The façade send.
            //~ emits: crate::scheduler::salvo_send(self.__addr, std::boxed::Box::new(crate::__Priv_CyclicRandom::Advance(
            advance(got)
        }
    }
}

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    // The mixed spawn answers the handle over the façade.
    //~ emits: crate::Random::shared(crate::__Fac_CyclicRandom { __addr: __a, seed: __c0 })
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng on pool(1)
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    println("actor drew ${drawn}")
    println("main drew ${next()}")
}

//--- a_mixed_handler_with_dependencies
// [actor-mixed-handler] [effect-handler-deps] [actor-spawn-inherit] [effect-with-clause] A
// mixed handler **with dependencies** (the cut lifted 2026-10-20): the
// servant's send member calls `Console`, synthesized from the spawning
// scope, and `Label`, built by the `with` clause; the façade carries
// neither.
effect Random {
    fn next() -> Int
}

effect Label {
    fn label(n: Int) -> Str
}

handler Hash() of Label {
    fn label(n: Int) -> Str {
        return "#${n}"
    }
}

handler Counted(seed: Int) [Console, Label] of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        cursor = cursor + seed
        println("servant at ${label(cursor)}")
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    // [rs-mixed] The servant is built with the dependencies; the façade
    // carries only the addr and the constructor parameters.
    //~ emits: let __h = crate::Counted::new(__c0.clone(), 
    //~ emits: crate::Label::shared(crate::Hash::new())
    //~ emits: crate::__Fac_Counted { __addr: __a, seed: __c0 }
    let rng = spawn Counted(7) with Hash()
    use rng
    println("main drew ${next()}")
    println("main drew ${next()}")
}

//--- a_dependent_spawn_hands_the_actor_its_handles
// [actor-spawn-expr] [effect-handler-deps] **A dependent spawn**: the child
// declares `[Log, Tally]` and the spawn's `with` clause supplies one of each
// kind — `Recording()` as a **construction**, built on the child, and `tally`
// as an **`Addr`**, bound to a forwarding stub. That is the binding swap
// executed: `Counting` is compiled once and neither of its member bodies can
// tell which of its two dependencies is a process.
//
// [rs-actor] [rs-handle] The shape a dependent spawn lowers to: the same
// handler struct a `use` builds, its dependencies handle fields filled from
// the clause — a construction behind its handle for `Log`, the send stub
// behind its handle for the `eff[Tally]` — and an actor body that owns the
// instance and dispatches onto its members; no provider, no view.
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

// The dependencies are handle fields.
//~ emits: __dep0: crate::Log,
//~ emits: __dep1: crate::Tally,
// The actor body owns the handler and dispatches onto it.
//~ emits: std::boxed::Box::new(crate::__Actor_Counting::new(__h))
//~ emits: crate::__Stateless_Counter::bump(&mut *__handler, n)
// No provider machinery remains.
//~ !emits: __Prov_
//~ !emits: __Deps_
//~ !emits: __Impl_
handler Counting() [Log, Tally] of Counter {
    mailbox { capacity: 8 }

    send fn bump(n: Int) {
        // Members reach their dependencies through the fields.
        //~ emits: self.__dep0.note(
        note("bumped ${n}")
        //~ emits: self.__dep1.tick(n)
        tick(n)
    }

    send fn report(out: Reply<Str>) {
        dump(out)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let tally = spawn Summing() on pool(1)
    // The clause's two kinds, in the handler's declaration order.
    //~ emits: crate::Counting::new(crate::Log::locked(crate::Recording::new()), crate::Tally::shared(crate::__Stub_Tally::new(tally)))
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

//--- draining_state_lowers_to_a_take
// [linear-container] [linear-state] **Obligations in a collection**: an actor
// parks reply tokens in `Mut List<Reply<Str>>` state, answers them one at a
// time with `remove_first`, and `drain`s the rest on shutdown — putting a
// fresh list back, because an activation may not leave its state with a hole.
//
// [rs-state-take] [linear-container] The lowering the case rests on: taking a
// container out of handler state is `std::mem::take` (a field behind
// `&mut self` cannot be moved, and cloning it would duplicate every
// obligation in it), and the drain hands each element to the callback by
// value.
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
        // [net-addr-routable] A `send(reply, v)` with a payload that has a wire form
        // takes the typed path, which receives the token by value all the same:
        // a taken obligation is moved, not cloned.
        //~ emits: let mut next_1 = next.unwrap();
        //~|             crate::scheduler::salvo_reply_wire::<String>(next_1, 
        let next = remove_first(waiting)
        when next {
            is Reply<Str> { next.send("served ${name}") }
            is None { discard(name) }
        }
    }

    // The terminal: every parked token is answered, and the state is whole
    // again before the activation ends.
    send fn close_up(reason: Str) {
        // The drain of a state field is a take.
        //~ emits: drain::<crate::scheduler::SalvoReply>(std::mem::take(&mut self.waiting), 
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

//--- several_faces_lower_to_one_actor_with_a_dispatcher_each
// [effect-handler-multi] Handlers of several effects: T-4(a)'s own
// shape — one actor with a public `Timer` face and a `TimerCtl` admin face over
// one mailbox and one piece of state, whose `spawn` answers one addr per face —
// beside the synchronous version of the same pattern, where one `use` binds
// both faces.
//
// [effect-handler-multi] [rs-actor] What the two faces lower to: one trait impl
// per face over one struct, one dispatcher per protocol, a `handle` that asks
// each protocol in turn (one mailbox carries both), and a spawn whose value is
// a tuple of the same scheduler index — the addrs differ only in their Salvo
// type.
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

// One trait impl per face.
//~ emits: impl crate::__Stateful_Timer for ManualTime {
//~ emits: impl crate::__Stateful_TimerCtl for ManualTime {
// One dispatcher per protocol.
//~ emits: pub fn __dispatch_ManualTime_Timer(__handler: &mut crate::ManualTime, mut __msg: crate::__Msg_Timer)
//~ emits: pub fn __dispatch_ManualTime_TimerCtl(__handler: &mut crate::ManualTime, mut __msg: crate::__Msg_TimerCtl)
// The delivery asks each protocol in turn.
//~ emits: match msg.downcast::<crate::__Msg_Timer>()
//~ emits: match msg.downcast::<crate::__Msg_TimerCtl>()
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

// The synchronous handler's faces.
//~ emits: impl crate::__Stateful_Tally for Counting {
//~ emits: impl crate::__Stateful_Stats for Counting {
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
    // The spawn answers one addr per face.
    //~ emits: let __a = crate::scheduler::salvo_spawn(
    //~ emits: (__a, __a)
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

//--- a_parked_continuation_lowers_to_a_slot_table
// [actor-replyto] **The shape the slice exists for: `main` is not in the
// loop.** A fetcher process asks a database process for a row, parking a
// continuation for the answer; the database replies *to the fetcher*, whose
// continuation runs and only then fulfils `main`'s `waitfor` token.
//
// [rs-actor] [actor-replyto] The lowering: a continuation enum beside the
// message enum; the two generated fields on the handler; a `__dispatch`
// factored out of `handle` so member invocation lives in one place; and a
// `resume` that pops the slot, casts the answer to the target member's
// *trailing* parameter type, and dispatches.
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

// The continuation enum, with its captures.
//~ emits: pub enum __Cont_Fetching {
//~ emits: Arrived(crate::scheduler::SalvoReply),
// The handler's generated fields.
//~ emits: __addr: Option<usize>,
//~ emits: __parked: std::collections::HashMap<u64, __Cont_Fetching>,
// The activation writes its own address.
//~ emits: self.handler.__addr = Some(_ctx.addr);
// `resume` dispatches the parked continuation.
//~ emits: let Some(__cont) = self.handler.__parked.remove(&slot)
//~ emits: *value.downcast::<String>().expect("the awaited answer")
handler Fetching() [Db] of Notices {
    mailbox { capacity: 4 }

    send fn fetch(id: Int, out: Reply<Str>) {
        // The mint parks a continuation.
        //~ emits: crate::scheduler::salvo_mint(self.__addr
        //~ emits: self.__parked.insert(__s,
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

//--- the_remaining_process_cuts_are_errors
// [backend-never-wrong] What this slice still refuses, and does not
// mis-emit: a **generic** handler as a process. (The dependent-handler
// refusal that used to live here is gone — the feature landed.)
actor effect Counter {
    send fn bump(n: Int) => !n
}

handler Counting<T>(seed: T) of Counter {
    mailbox { capacity: 8 }

    send fn bump(n: Int) {}
}

fn main() [use, spawn] {
    //~ refused[rs-actor:spawn-generic-handler]
    let c = spawn Counting(1) on pool(1)
}
