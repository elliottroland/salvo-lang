// Actors: state that runs by itself, one message at a time, on a pool of
// threads. This module holds the types that go with them: an actor's
// address ([Addr], written `eff[E]`), one-shot answers ([Reply]), pools
// ([Pool], [pool], [thread]), and what the runtime reports back: deaths
// ([Exit], [watch], [Faults]) and quiescence ([Idle], [on_idle]). Every
// file sees it; nothing needs importing.
//
// An **actor** is a handler of an `actor effect`, bound with `spawn`
// instead of `use`. The effect is the protocol: its members are `send fn`s,
// and sending one enqueues a message and answers nothing. The handler is
// ordinary Salvo, with `state` that only the actor can reach and a
// `mailbox` that bounds how many messages may queue. `spawn H(args) on p`
// starts the actor on pool `p` and answers its address, an `eff[E]`;
// `addr.member(args)` sends to it. A member with something to answer takes
// a [Reply] for it, and whoever wants the answer mints one: `waitfor`, in
// any function, waits for it; `replyto k(captures)`, in a handler, has it
// arrive later as a message to the handler's own member `k`.
//
// ```
// // The protocol: what can be sent to a counter.
// actor effect Counter {
//     send fn add(n: Int)
//     send fn total(out: Reply<Int>) => !out
// }
//
// // `sum` belongs to the actor alone, and its members run one at a time,
// // so no lock guards it.
// handler Counting() for Ordered Counter {
//     mailbox { capacity: 16 }
//
//     state {
//         sum: Int = 0
//     }
//
//     send fn add(n: Int) {
//         sum = sum + n
//     }
//
//     send fn total(out: Reply<Int>) {
//         out.send(sum)
//     }
// }
//
// // A task: a free `send fn`, run when the answer it is waiting for
// // arrives. It has no mailbox and no state.
// send fn report(label: Str, out: Reply<Str>, n: Int) => !label, !out {
//     out.send("${label}: ${n}")
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let workers = pool(2)
//     let counter = spawn Counting() on workers
//     counter.add(2)                                  // enqueues, returns at once
//     counter.add(3)
//     let sum = waitfor out: Reply<Int> {             // waits for the answer
//         counter.total(out)
//     }
//     println("${sum}")                               // 5
//     let line = waitfor out: Reply<Str> {
//         counter.total(replyto report("total", out) on workers)
//     }
//     println(line)                                   // total: 5
// }
// ```
//
// The Concurrency topic tells the same story at more length, and
// [std.runtime] describes the scheduler underneath.
//
// **What is guaranteed.**
//
// - *One activation at a time.* An activation is one member running for
//   one message. An actor's activations never overlap, so its state needs
//   no lock, and an activation runs to its end before the actor starts
//   another.
// - *Arrival order.* Each actor has one queue, delivered in the order
//   entries arrived. Two sends made one after the other, from one thread,
//   arrive in that order; sends from different threads interleave in the
//   order they reach the queue. Answers to the actor's own continuations
//   join the same queue. The one exception is a **gated** mint,
//   `replyto! k(…)`: until that answer arrives, the actor is delivered
//   nothing else.
// - *Sends block only on a full mailbox.* A send returns once the message
//   is queued. When the mailbox already holds `capacity` messages, the
//   sender waits until the actor takes one out.
// - *Answers never block.* Sending to a [Reply] never waits, and answers do
//   not count against the mailbox: room for the answer was set aside when
//   the reply was minted.
// - *A send to a dead actor does nothing.* No error, no exception: holding
//   a stale address is safe, and [watch] is how a death is noticed.
//
// **Where the work runs.** A [Pool] is a set of worker threads, made once
// with [pool] and shared by as many actors as you place on it. Its workers
// take a queued task before a waiting activation, and each queue in arrival
// order; after each activation, an actor with more to do goes to the back
// of its pool's queue, so the actors on a pool take turns. A `waitfor`
// does not idle its thread: while it waits, the thread runs other work of
// the same pool (never the waiting actor's own messages), and the wait
// ends when its answer arrives. `main` is the one worker of a pool of its
// own, so a task minted in `main` without `on` runs only while `main`
// waits. An actor that must block its thread in a host call belongs on
// [thread], which gives it a thread of its own.
//
// **Obligations.** A [Reply] is **linear**: the program does not compile
// unless every path sends to it exactly once or hands it on, as an
// argument, a capture of a `replyto`, or an element of a linear collection
// in the actor's state that is drained before the actor lets go of it.
// [watch] and [on_idle] take a reply and keep it until their event comes,
// and a `Dedicated` pool from [thread] is spent by the `on` that uses it.
// What a faulted actor still owed is lost with it. A `waitfor` that can
// then never be answered does not hang silently: once nothing anywhere can
// run while `main` waits, the runtime prints a deadlock report naming the
// stuck actors and ends the program.
//
// **Failure.** An actor dies when an activation faults: an
// `unreachable!()` or a failed `assert!` is reached, or host code panics
// (Rust) or throws (Kotlin). A `throw` cannot cross a member, so it is not
// a way to die. On death the queue is dropped, later sends are no-ops, and
// whoever called [watch] on the actor is sent an [Exit]. A death nobody watched, and any faulted task, goes to
// the pool's fault sink ([Faults]) or, without one, is printed on standard
// error. Either way the rest of the program carries on. Supervision is
// written, not built in: a supervisor is an actor that keeps its worker's
// address private, watches it, and replaces it when it dies, so its own
// clients never hold a dead address.
//
// ```
// actor effect Doubler {
//     send fn double(n: Int, out: Reply<Int>) => !out
// }
//
// // The worker. A negative number is a bug it does not survive.
// handler Doubling() for Ordered Doubler {
//     mailbox { capacity: 4 }
//
//     send fn double(n: Int, out: Reply<Int>) {
//         let none: List<Int> = List<Int>()
//         if n < 0 {
//             let _boom = get(none, 0) ?: unreachable!()
//         }
//         out.send(n * 2)
//     }
// }
//
// actor effect Jobs {
//     send fn run(n: Int, out: Reply<Int>) => !out
// }
//
// // The supervisor. It keeps each request it forwards until the worker
// // answers, so a death costs the client an answer of -1, not an answer
// // that never comes.
// handler Supervising(workers: Pool) [spawn] for Ordered Jobs {
//     mailbox { capacity: 16 }
//
//     state {
//         child: eff[Doubler]? = None
//         pending: Mut Deque<Reply<Int>> = mut_deque_of()
//     }
//
//     send fn run(n: Int, out: Reply<Int>) {
//         if child is None {
//             let fresh = spawn Doubling() on copy(workers)
//             watch(fresh, replyto died())
//             child = fresh
//         }
//         let c = copy(child)
//         if c is None {
//             out.send(-1)
//             return
//         }
//         add_last(pending, out)
//         c.double(n, replyto doubled())
//     }
//
//     // Private members: no face declares them, so only this actor's own
//     // continuations reach them. The worker answers in the order asked.
//     send fn doubled(n: Int) {
//         if remove_first(pending) is Reply<Int> out {
//             out.send(n)
//         }
//     }
//
//     send fn died(exit: Exit) => !exit {
//         drain(pending, (out) -> send(out, -1))
//         pending = mut_deque_of()
//         child = None
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let p = pool(2)
//     let jobs = spawn Supervising(copy(p)) on p
//     let a = waitfor out: Reply<Int> { jobs.run(2, out) }
//     let b = waitfor out: Reply<Int> { jobs.run(-1, out) }
//     let c = waitfor out: Reply<Int> { jobs.run(3, out) }
//     println("${a} ${b} ${c}")                       // 4 -1 6
// }
// ```
//
// **Shutdown.** The program ends when `main` returns. Whatever is still
// queued or running then is abandoned, with no message and no `Exit`: pool
// threads do not keep a program alive. To let work finish, wait for it,
// for the answer it sends or with [on_idle].
//
// **Costs.** Sending, answering and starting or finishing an activation
// each take one lock that the whole process shares, so a message costs a
// lock round-trip and a hand-off between threads, not a call. Every
// payload is moved into its message, not copied. A pool's threads start
// when it is made and run until the program ends (a [thread] is the
// exception), so make pools once and share them rather than one per spawn.
// A thread that waits runs other work on top of its own stack; if that
// work waits too, the two nest, so long chains of nested waits cost stack
// depth.
// @compilerrules [actor-kind, actor-types, actor-spawn-expr, actor-replyto, actor-waitfor, waitfor-pump, task-mint, actor-watch]

import std.runtime.addr_index
import std.runtime.pool_of
import std.runtime.pool_index
import std.runtime.new_pool_of
import std.runtime.routing.adopt
import std.runtime.routing.same_actor
import std.runtime.Token
import std.runtime.reply_token
import std.runtime

// Each type here is a handle into the scheduler (`std.runtime`): an addr
// and a pool are indices into its tables, a reply wraps its token, and
// their representation is each backend's business.

// The address of a running actor: what `spawn` answers, and the only way
// to reach the actor. A program writes the type as `eff[E]`; `Addr` is the
// compiler's name for it.
//
// [E] is the effect the actor serves, and an address can be used wherever
// that effect is wanted: `addr.member(args)` sends one message, and
// `use addr` binds the effect for the rest of the scope, so code written
// against `[E]` cannot tell an actor from a local handler. The state
// behind an address is the actor's alone: an address is all one actor
// knows about another.
//
// An address is a plain value: copy it, store it, send it in a message.
// It is never linear and never goes stale in a harmful way, because a send
// to an actor that has died does nothing. Sent to another node, it arrives
// as a proxy with the same type and the same sends (see [std.net]).
//
// ```
// actor effect Log {
//     send fn line(text: Str) => !text
// }
//
// handler Logging() [Console] for Ordered Log {
//     mailbox { capacity: 8 }
//
//     send fn line(text: Str) {
//         println(text)
//     }
// }
//
// // Written against the effect, not against an actor.
// fn greet(names: List<Str>) [Log] -> None {
//     for n in names {
//         line("hello, ${n}")
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let p = pool(1)
//     let log = spawn Logging() on p
//     log.line("starting")                            // one send
//     use log                                         // `Log` is now the actor
//     greet(List("ada", "bo"))
//     // The program ends when `main` returns: let the log catch up first.
//     let _idle = waitfor out: Reply<Idle> { on_idle(p, out) }
// }
// ```
// @compilerrules [actor-spawn-expr, effect-not-data, actor-use-addr]
export intrinsic type Addr<E>

// Whether two addresses name the same actor. `a == b` calls it.
//
// It compares identities, not representations: an actor on a node and a
// proxy for it on another node are equal. Group membership checks read it.
// @compilerrules [actor-types, actor-group]
export fn eq<E>(a: eff[E], b: eff[E]) [] -> Bool => a, b {
    return same_actor(addr_index(a), addr_index(b))
}

// A one-shot channel for answering a request with a [T]. Minted by
// `waitfor out { … }` or `replyto k(captures)`, and spent by [send].
//
// A reply is **linear**. The compiler checks that each one is sent to
// exactly once on every path, or handed on: passed to a function or a send,
// captured by a `replyto`, or kept in a linear collection in an actor's
// state and drained. That check is what makes "every request gets one
// answer" a fact rather than a hope. At run time an answer is delivered as
// a message, to the actor or waiting frame or task the reply was minted
// for; it never blocks and never counts against a mailbox.
//
// [T] may be linear itself (`Reply<InStream>`): the reply then carries
// that obligation to whoever receives the answer.
//
// ```
// actor effect Desk {
//     send fn ticket(out: Reply<Str>) => !out
//     send fn serve(name: Str) => !name
// }
//
// // Requests it cannot answer yet wait in state, as obligations.
// handler Desking() for Ordered Desk {
//     mailbox { capacity: 8 }
//
//     state {
//         waiting: Mut Deque<Reply<Str>> = mut_deque_of()
//     }
//
//     send fn ticket(out: Reply<Str>) {
//         add_last(waiting, out)
//     }
//
//     send fn serve(name: Str) {
//         if remove_first(waiting) is Reply<Str> next {
//             next.send("served ${name}")
//         }
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let desk = spawn Desking() on pool(1)
//     let got = waitfor out: Reply<Str> {
//         desk.ticket(out)
//         desk.serve("ada")
//     }
//     println(got)                                    // served ada
// }
// ```
// @compilerrules [actor-replyto, linear-group, linear-generics]
export linear intrinsic type Reply<T canbe linear>

// Answers a request: delivers [value] to whoever [reply] was minted for,
// and spends the reply. `reply.send(value)` is the same call.
//
// It never blocks: the answer is queued for its receiver (an actor's
// continuation, a frame in `waitfor`, or a task), which runs it later, and
// room for it was set aside when the reply was minted. If the receiver has
// died, the answer is dropped.
//
// ```
// let n = waitfor out: Reply<Int> {
//     send(out, 4)
// }
// println("${n}")                                     // 4
// ```
// @compilerrules [actor-replyto]
// @compilerknown
export intrinsic fn send<T canbe linear>(reply: Reply<T>, value: T) [] -> None => !reply, !value

// One message of the actor effect [E], as a value, written `message[E]`: a
// union with one arm per `send fn` of `E`, each arm a struct of the member's
// parameters. `message[A, B]` is the messages of both, the union of the two.
// This declaration is the compiler's internal name for it; a program writes
// `message[…]`, as it writes `eff[…]` for an `Addr`.
//
// A `send fn` is shorthand for a message: `reserve(sku, 1, out)` sends
// `Inventory.Reserve { sku: sku, qty: 1, out: out }`. The arms are named
// after the effect and the member in CamelCase, and are tested like any
// union's: `is Inventory.Reserve`, or `is .Reserve` where the subject's type
// says which effect. A handler may take every message of an effect at once
// with `send fn sent(m: message[E])` instead of writing each send member,
// and hand a message on with [send]; that is how a router is written
// without knowing the protocol it routes.
//
// ```
// actor effect Inventory {
//     send fn reserve(sku: Str, qty: Int, out: Reply<Str>) => !sku, !out
//     send fn restock(qty: Int)
// }
//
// fn describe(m: message[Inventory]) [] -> Str => m {
//     return when m {
//         is .Reserve { "reserve ${m.qty} of ${m.sku}" }
//         is .Restock { "restock ${m.qty}" }
//     }
// }
// ```
//
// **Linear when its parameters are.** An arm holding a `Reply` owes it: the
// message is ended by sending it on (`send(to, m)`) or by moving the reply out
// and answering it.
// @compilerrules [actor-message]
// @compilerknown
export intrinsic type Message<E>

// Sends message [m] to the actor [to], as if its member had been called
// there: `send(to, m)` for an `Inventory.Reserve` is `to.reserve(m.sku,
// m.qty, m.out)`. The message is moved: the receiver answers its replies.
//
// The other `send`: where the one above delivers a value to whoever a
// `Reply` was minted for, this one delivers a message to an actor. A
// handler takes every message of a face with `send fn sent(m: message[E])`,
// and this is how it hands one on.
//
// ```
// handler Front(back: eff[Inventory]) for Inventory {
//     mailbox { capacity: 8 }
//     send fn sent(m: message[Inventory]) => !m {
//         send(back, m)
//     }
// }
//
// actor effect Inventory {
//     send fn reserve(sku: Str, out: Reply<Str>) => !sku, !out
// }
// ```
// @compilerrules [actor-message, actor-sent]
// @compilerknown
export intrinsic fn send<E>(to: eff[E], m: message[E]) [] -> None => to, !m

// A claim that a binding of an effect is one instance taking sends in the
// order they were made, with one state behind them: what a single actor
// gives, and what a router over a group does not.
//
// A claim is written before the effect. A handler that gives the guarantee
// says so on its face (`for Ordered Counter`); a function that relies on it
// asks for it in its effect list (`[Ordered Counter]`), and a handle carries
// it (`eff[Ordered Counter]`, what spawning such a handler answers). A bare
// `[Counter]` assumes nothing about order or instances, so any binding
// does. Nothing is granted: the compiler takes the handler's word.
//
// ```
// actor effect Counter {
//     send fn add(n: Int)
//     send fn total(out: Reply<Int>) => !out
// }
//
// handler Counting() for Ordered Counter {
//     mailbox { capacity: 16 }
//     state { sum: Int = 0 }
//     send fn add(n: Int) { sum = sum + n }
//     send fn total(out: Reply<Int>) => !out { out.send(sum) }
// }
//
// // Adds then reads: right only if the read sees the adds.
// fn tally(ns: List<Int>) [Ordered Counter] -> Int => ns {
//     for n in ns { add(n) }
//     return waitfor out: Reply<Int> { total(out) }
// }
// ```
// [Ordered] implies [ShardKeyOrdered]: one instance takes every key's sends
// in order, so a binding claiming `Ordered` satisfies `[ShardKeyOrdered E]`
// too.
// @compilerrules [effect-claim, qual-imply]
export provenance qualifier Ordered for ShardKeyOrdered Effect

// A claim that sends with the same shard key reach the same instance, in
// the order they were made: what [Ordered] gives, and what a router by key
// (`Sharded`) gives over many instances. The key is the one the binding
// was built with: the claim cannot name it.
// @compilerrules [effect-claim]
export provenance qualifier ShardKeyOrdered for Effect

// An actor's mailbox settings, written in its handler as
// `mailbox { capacity: N }`.
//
// `capacity` is how many messages may wait in the queue. A send to a full
// mailbox waits until the actor takes a message out, which is how a fast
// sender is slowed to its receiver's pace. Answers and fault reports do not
// count: their room is set aside elsewhere. Every handler of an actor
// effect must declare it, because only its author knows the traffic; there
// is no default. The expression may read the handler's constructor
// parameters (`mailbox { capacity: room }`), which is how a caller chooses
// the size per instance.
//
// ```
// actor effect Queue {
//     send fn put(item: Str) => !item
// }
//
// handler Queueing(room: Int) for Ordered Queue {
//     mailbox { capacity: room }
//
//     send fn put(item: Str) {
//     }
// }
//
// fn main() [use, spawn] {
//     let q = spawn Queueing(64) on pool(1)
//     q.put("job")
// }
// ```
// @compilerrules [actor-mailbox]
// @compilerknown
export struct Mailbox { capacity: Int }

// A set of worker threads that actors and tasks run on: what a spawn's
// `on` clause names.
//
// A pool is an ordinary value, so one can be made once and given to many
// spawns: "these actors share these threads". Its workers take queued
// tasks first, then actors with messages, in arrival order, each actor one
// activation per turn. Make one with [pool], or a one-thread pool of its
// own for a single actor with [thread]. A pool belongs to this process: it
// has no wire form and cannot be sent to another node.
// @compilerrules [actor-spawn-expr, net-noremote, waitfor-pump]
export noremote intrinsic type Pool

// A pool of [size] worker threads.
//
// The threads start now and serve the pool until the program ends; they
// never keep the program alive past `main`. Creating a pool is part of
// running actors, so it needs the `[spawn]` capability. An uncaught fault
// on it is printed on standard error; give it a sink with the other
// overload to handle faults yourself.
//
// ```
// fn start() [spawn] -> Pool {
//     return pool(4)
// }
// ```
// @compilerrules [actor-spawn-expr]
// @compilerknown
export fn pool(size: Int) [spawn] -> Pool => size {
    let p = new_pool_of(size, -1)
    adopt(copy(p))
    return pool_of(p)
}

// A pool of exactly one thread, made by [thread] for one spawn.
//
// The qualifier records where the pool came from, and the `on` clause
// spends it: a second spawn onto the same `Dedicated Pool` is a
// use-after-move error, so the actor placed there is the thread's only
// occupant. That is what makes it the place for an actor that blocks its
// thread (a blocking host API, say): it can stall nobody but itself.
// @compilerrules [waitfor-dedicated]
// @compilerknown
export provenance qualifier Dedicated for Pool

// Why a task, or an actor nobody watched, died: what a pool's fault sink
// receives.
//
// [reason] is the host's account of the fault, a panic message on the Rust
// backend and an exception's on the Kotlin one, so its text differs between
// backends. Print it; do not branch on it.
// @compilerrules [actor-pool-fault-sink]
// @compilerknown
export struct Fault { reason: Str }

// The protocol a pool's fault sink serves: spawn a handler of it and pass
// its address to [pool], and every uncaught fault on that pool arrives as a
// `faulted` message.
//
// It catches what [watch] cannot. A watch is per actor and fires once; a
// task has no address to watch, and an actor nobody watched dies unseen.
// The sink hears of both. An actor with a watcher is reported to its
// watchers only. A report is queued past the sink's mailbox bound, so a
// fault never blocks the thread it happened on.
//
// ```
// actor effect Parser {
//     send fn parse(text: Str) => !text
// }
//
// handler Parsing() [Console] for Ordered Parser {
//     mailbox { capacity: 8 }
//
//     send fn parse(text: Str) {
//         let n = parse_int(text) ?: unreachable!()
//         println("parsed ${n}")
//     }
// }
//
// handler Reporting() [Console] for Ordered Faults {
//     mailbox { capacity: 8 }
//
//     send fn faulted(fault: Fault) {
//         println("a parser died: ${fault.reason}")
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let sink = spawn Reporting() on pool(1)
//     let workers = pool(2, sink)
//     let parser = spawn Parsing() on workers
//     parser.parse("12")                              // parsed 12
//     parser.parse("twelve")                          // a parser died: …
//     parser.parse("13")                              // dropped: it is dead
//     let _settled = waitfor out: Reply<Idle> { on_idle(workers, out) }
// }
// ```
// @compilerrules [actor-pool-fault-sink, actor-watch]
// @compilerknown
export actor effect Faults {
    // One uncaught fault on the pool.
    send fn faulted(fault: Fault) => !fault
}

// A pool of [size] worker threads whose uncaught faults are sent to
// [sink] instead of printed. See [Faults] for an example.
// @compilerrules [actor-pool-fault-sink]
// @compilerknown
export intrinsic fn pool(size: Int, sink: eff[Faults]) [spawn] -> Pool => size, sink

// One fresh thread, as a pool for a single spawn: `spawn H() on thread()`.
//
// The `on` clause spends the `Dedicated` pool this answers, so nothing else
// can ever be placed on that thread. Use it for an actor that blocks its
// thread for long stretches: on a shared pool it would hold up the other
// actors there, on its own it holds up only itself. The thread ends once
// the actor is dead and nothing is left to run or owed there.
//
// ```
// actor effect Files {
//     send fn read(path: Str, out: Reply<Str>) => !path, !out
// }
//
// // Stands in for a handler whose members call a blocking host API.
// handler Reading() for Ordered Files {
//     mailbox { capacity: 4 }
//
//     send fn read(path: Str, out: Reply<Str>) {
//         out.send("contents of ${path}")
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let files = spawn Reading() on thread()
//     let text = waitfor out: Reply<Str> { files.read("notes.txt", out) }
//     println(text)                                   // contents of notes.txt
// }
// ```
// @compilerrules [waitfor-dedicated, actor-pool-retire]
export intrinsic fn thread() [spawn] -> Dedicated Pool

// Why an actor died: what [watch] sends to whoever was watching it.
//
// A death is always a fault: an activation failed, and the actor is gone
// with whatever it still owed. [reason] is the host's account of the fault,
// a panic message on the Rust backend and an exception's on the Kotlin
// one, so its text differs between backends. Print it; do not branch on it.
// @compilerrules [actor-watch]
export struct Exit { reason: Str }

// Watches [target]: when it dies, an [Exit] is sent to [on_exit].
//
// The reply is spent here and kept until the death, so a watch cannot be
// forgotten: an unused reply is a compile error, as any is. It fires once.
// Watching an actor that has already died answers at once, so a spawn
// followed by a watch has no race to lose. The address is not spent: an
// actor can be watched by many, and the watcher keeps sending to it.
//
// In `main`, mint the reply with `waitfor`; in a handler, with
// `replyto k()`, where `k` is a member taking the `Exit` (see the module's
// supervisor example).
//
// ```
// actor effect Fragile {
//     send fn crash()
// }
//
// handler Breaking() for Ordered Fragile {
//     mailbox { capacity: 1 }
//
//     send fn crash() {
//         let none: List<Int> = List<Int>()
//         let _boom = get(none, 7) ?: unreachable!()
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let fragile = spawn Breaking() on pool(1)
//     let exit = waitfor gone: Reply<Exit> {
//         watch(fragile, gone)
//         fragile.crash()
//     }
//     println("died: ${size(exit.reason) > 0}")       // died: true
//     fragile.crash()                                 // a no-op now
// }
// ```
//
// @param target   the actor to watch; kept
// @param on_exit  answered with the `Exit` when [target] dies
// @compilerrules [actor-watch, linear-obligation]
export fn watch<E>(target: eff[E], on_exit: Reply<Exit>) [spawn] -> None => target, !on_exit {
    // A token minted on another node has no local half to register: the
    // watch is dropped, as it always was.
    if reply_token(on_exit) is Token t {
        watch(addr_index(target), t)
    }
}

// What [on_idle] reports: the obligations still outstanding when the
// program ran out of work. Both zero means the work is done; anything else
// means it is stuck.
//
// A watch or an idle hook waiting for its event is not counted: the
// runtime will answer it when the event happens.
// @compilerrules [actor-on-idle]
export struct Idle {
    // Actors on the pool held by a gated mint (`replyto!`) whose answer
    // has not come, so their mailbox is closed to everything else.
    parked_gates: Int,
    // Replies aimed at work on the pool (an actor's continuation, a task,
    // a waiting frame) that nobody has answered. Every gated actor is also
    // owed one, so this includes the gates.
    parked_tokens: Int
}

// Asks to be told when the program runs out of work: once nothing anywhere
// can run, an [Idle] is sent to [notify] counting what is still owed on
// [p].
//
// "Nothing can run" means no activation or task running, no mailbox with a
// message it may deliver, and no scheduled work queued, on any pool, so a
// pool that is quiet while another still has work for it does not count as
// idle. An activation or task waiting in a `waitfor` counts as running, so
// the answer does not come while one is waiting; `main`'s own wait does
// not count, which is what lets `main` wait for the answer.
//
// It fires once: delivering the answer is itself work, which ends the
// idleness. To hear about the next time, register again. The reply is
// spent here, so a hook cannot be forgotten. The answer is only as good as
// the scheduler's view: a platform handler with a thread of its own can
// add work it never saw.
//
// What it is for is sequencing ("everything I sent has been handled") and
// tests, without counting messages.
//
// ```
// actor effect Log {
//     send fn line(text: Str) => !text
// }
//
// handler Logging() [Console] for Ordered Log {
//     mailbox { capacity: 8 }
//
//     send fn line(text: Str) {
//         println(text)
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let p = pool(2)
//     let log = spawn Logging() on p
//     log.line("one")
//     log.line("two")
//     let idle = waitfor out: Reply<Idle> { on_idle(p, out) }
//     println("owed: ${idle.parked_tokens}")          // owed: 0
// }
// ```
//
// @param p       the pool whose outstanding obligations the answer counts;
//                kept
// @param notify  answered with the `Idle`
// @compilerrules [actor-on-idle]
export fn on_idle(p: Pool, notify: Reply<Idle>) [spawn] -> None => p, !notify {
    if reply_token(notify) is Token t {
        on_idle(pool_index(p), t)
    }
}
