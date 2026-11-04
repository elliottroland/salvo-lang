// The scheduler every Salvo program runs its actors on, `main` included:
// the tables of actors, pools and waiting frames, and the operations that
// `spawn`, a send, `replyto` and `waitfor` come down to. It is written once,
// in Salvo, over one small platform handler ([RuntimeHost]) and a few host
// types, rather than by hand in each backend.
//
// **This module is std's own.** Only std's files may import it; a program
// that tries is refused at the import. A program reaches it through the
// language forms and through [std.core.actor], whose types are handles
// into the tables here: an `eff[E]` is an index into the actor table, a
// [Pool] an index into the pool table, a [Reply] wraps a [Token]. Services
// built on the scheduler live under `std.runtime` ([std.runtime.timers],
// [std.runtime.routing], [std.runtime.streams]); they may use the actor
// forms, while this module may not, nor import them, so the dependency
// runs one way. Each backend's `scheduler.rs` / `scheduler.kt` holds the
// entry points generated code calls, which forward here. Because it sits
// beneath everything, its private declarations do what other Salvo code
// cannot: start threads, park them, keep a function value to run on
// another thread, and catch a fault.
//
// **How an actor program runs on it.**
//
// - *Spawning* adds a record to the actor table: the actor's body (its
//   handler and dispatch, as a [Body]), its pool, its mailbox bound and an
//   empty queue. The address is the record's index.
// - *Sending* ([send_dyn]) appends the message to the actor's queue and,
//   if the actor was not already waiting for a turn, puts it on its pool's
//   ready queue and wakes one idle worker of that pool. A send to a dead
//   actor drops the message. A send to a full queue parks the sender until
//   the actor takes a message out, then tries again.
// - *A worker* loops: it takes the pool's next queued task if there is
//   one, otherwise the next actor on the ready queue, and runs one
//   activation of it inside a fault boundary. The body is a linear value
//   owned by the table or by the one thread running it, which is how an
//   actor never runs twice at once. When the activation ends the body goes
//   back, and an actor with more queued goes to the back of the ready
//   queue: the actors of a pool take turns. With nothing to do, a worker
//   parks until work arrives.
// - *Replies* are [Token]s aimed at a target: an actor's continuation, a
//   frame in `waitfor`, or a task, whose body travels inside the token.
//   Answering one ([answer]) never blocks: an answer for an actor joins its
//   queue past the mailbox bound, one for a waiting frame fills its slot
//   and wakes it, and one for a task queues the task on its pool. A
//   **gated** continuation (`replyto!`) closes the actor's queue to
//   everything but that answer until it arrives.
// - *A wait* ([waiter], [await_answer]) does not idle its thread. Until its
//   answer arrives it runs the work of its own pool, except the waiting
//   actor's own messages, and parks only when there is none.
// - *Death* is an activation that faulted. The actor's queue is dropped,
//   senders parked on it are woken (their messages are then dropped), and
//   each watcher is answered with an [Exit]. A death nobody watched, and a
//   faulted task, is sent to the pool's fault sink as a [Fault], or
//   printed on standard error.
// - *`main`* is the single worker of pool 0, which has no thread of its
//   own: work placed there runs only on `main`'s thread, inside its waits,
//   and is abandoned when `main` returns. Pool threads are daemons and do
//   not keep the program alive.
//
// ```
// no-check
// actor effect Echo {
//     send fn echo(text: Str, out: Reply<Str>) => !text, !out
// }
//
// handler Echoing() for Echo {
//     mailbox { capacity: 4 }
//
//     send fn echo(text: Str, out: Reply<Str>) {
//         out.send(text)
//     }
// }
//
// // A task: it runs when the answer it was minted for arrives.
// send fn shout(out: Reply<Str>, text: Str) => !out, !text {
//     out.send("${text}!")
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let p = pool(1)                      // one worker thread, started now
//     let echo = spawn Echoing() on p      // a record in the actor table
//     let loud = waitfor out: Reply<Str> { // a waiter, and its token
//         // A mint without `on` places the task on the pool it is written
//         // in: in `main`, pool 0. The echo runs on p's thread; its answer
//         // queues the task on pool 0, which this wait then runs, here on
//         // main's thread; the task's answer ends the wait.
//         echo.echo("hi", replyto shout(out))
//     }
//     println(loud)                        // hi!
// }
// ```
//
// **When nothing can run.** The scheduler knows when no activation or task
// is running, no queue holds an entry it may deliver, and no answer is
// waiting to be picked up. At that moment it answers every [on_idle] hook.
// When it has no hook to answer, `main` is waiting, and every activation
// or task still running is parked in a wait of its own, nothing can ever
// make progress: a waiting frame prints a report and the process exits
// with status 1, rather than hang.
// A platform handler with a thread of its own (a socket listener) can
// still bring work in, so while one is open ([external_begin]) the program
// is neither idle nor stuck.
//
// ```
// no-check
// actor effect Ask {
//     send fn ask(out: Reply<Int>) => !out
// }
//
// // Dies before it answers: the reply is lost with it.
// handler Failing() for Ask {
//     mailbox { capacity: 1 }
//
//     send fn ask(out: Reply<Int>) {
//         assert!(false)
//         out.send(1)
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let asker = spawn Failing() on pool(1)
//     let n = waitfor out: Reply<Int> { asker.ask(out) }
//     println("${n}")                      // never reached
// }
// ```
//
// prints this on standard error and exits with status 1:
//
// ```text
// salvo: an uncaught fault on pool 1: salvo: assertion failed at main:10:9
// salvo: deadlock: nothing can run while main waits
// ```
//
// The first line is the fault, reported because nobody watched the actor
// (on Rust, the panic hook prints the assertion once more before it).
//
// When an actor is stuck in a wait or behind a gate, the report names it:
// `(parked in a wait: actor 3; parked gates: actor 5)`. The numbers are
// table indices. A second report covers the one wedge a wait cannot see:
// `main` sending to a full mailbox of an actor on main's own pool, which
// only `main` could drain.
//
// **The virtual runtime.** An actor test (`test actor "…"`, see the
// Testing topic) runs on a fresh scheduler with no worker threads
// ([enter_virtual]): the test's one thread serves every pool inside its
// waits, in a fixed order; time is a virtual clock that jumps to the next
// deadline only when nothing else can run; and what the runtime draws at
// random comes from the test's seed, so a run repeats exactly. Opening a
// host thread there stops the test with a report, since its work would
// arrive off the virtual clock.
//
// **Costs.**
//
// - Every operation on the tables (a send, an answer, a mint, taking or
//   finishing an activation) takes the scheduler's lock: the [Scheduler]
//   is one handler shared by the whole process. [std.runtime.routing] has a
//   lock of its own, always taken after this one, never before.
// - Each message and answer is erased to a [Dyn] on its way through a
//   queue: a heap box on the Rust backend.
// - A pool starts its threads when it is made, and they run until the
//   program ends; only a `Dedicated` pool's thread returns, once nothing
//   can run there again.
// - An actor's record is kept after it dies, so the actor table grows by
//   one record per spawn (and per proxy of a remote actor) for the life of
//   the process. Waiter records are reused: there are as many as the most
//   waits ever open at once.
// - Work run during a wait runs on the waiting thread's stack, so waits
//   nested inside that work cost stack depth.
// - A new message or answer wakes one idle worker, not all of them; a
//   death, a hook or an outside source wakes every pool, since any of them
//   may have to look again.
// @compilerrules [mod-std-internal, runtime-layers, runtime-sched, runtime-handles, waitfor-pump, actor-main-pool, actor-on-idle, test-actor, platform-threadsafe]
//
// @compilerknown

// One thread's park/unpark token: how a thread with nothing to do sleeps
// until another thread wakes it.
//
// Every thread has one ([this_parker]). [park] sleeps the thread it
// belongs to; [unpark], called from any thread, wakes it. An unpark that
// comes before the park is not lost: the next park returns at once. So a
// thread may record its parker where a waker will find it, release its
// lock, and only then park, without missing a wakeup. A park may also
// return when nobody unparked it, so always park in a loop that checks
// again what it is waiting for.
//
// It is `threadsafe`: any thread may unpark it, and it may be stored and
// shared freely. Only its own thread may park on it.
//
// ```
// no-check
// import std.runtime
//
// // Sleeps the calling thread until `ready` answers true, looking again
// // at least every millisecond.
// fn wait_until(ready: () -> Bool) [] -> None {
//     let me = this_parker()
//     while !ready() {
//         park_nanos(me, 1000000L)
//     }
// }
// ```
// @compilerrules [runtime-parker]
export threadsafe platform type Parker

// The calling thread's parker: the one [unpark] wakes this thread with.
// @compilerrules [runtime-parker]
export platform fn this_parker() [] -> Parker

// Sleeps the calling thread until [p] is unparked, or returns at once if
// it was unparked since the last park. It may also return for no reason:
// check the condition again.
//
// [p] must be the calling thread's own parker; parking on another
// thread's is a bug in the runtime, and traps.
// @compilerrules [runtime-parker]
export platform fn park(p: Parker) [] -> None => p

// [park], giving up after [nanos] nanoseconds: how a thread waits for a
// deadline and for a wakeup at once.
// @compilerrules [runtime-parker]
export platform fn park_nanos(p: Parker, nanos: Long) [] -> None => p, nanos

// Wakes the thread [p] belongs to, or, if it is not parked, makes its next
// park return at once. Safe from any thread.
// @compilerrules [runtime-parker]
export platform fn unpark(p: Parker) [] -> None => p

// Runs [body] on a new **daemon** thread, one that does not keep the
// program alive.
//
// The program ends when `main` returns, whatever the thread is doing then.
//
// The host keeps the function value and runs it on the other thread, a
// privilege of the runtime's modules: everywhere else a function value is
// only lent for the call it is passed to. There is no handle to join or
// stop the thread; the scheduler ends one by changing the state it reads.
// @compilerrules [runtime-kept-fn, platform-fn-value]
export platform fn start_thread(body: once () -> None) [] -> None => !body

// Runs [body] inside a fault boundary: `None` when it returned, or the
// host's account of the fault that ended it. A fault inside is then the
// death of the work, not of the thread.
// @compilerrules [runtime-kept-fn]
platform fn guarded(body: once () -> None) [] -> Str? => !body

// Runs [body] inside a fault boundary and answers what it answered, or,
// if it faulted, the fault's message in its place. The host prints nothing
// for the fault, since the caller reports it. What [std.test]'s
// `trapped_by` is built on.
// @compilerrules [test-recover]
export platform fn trap_boundary(body: () -> Str?) [] -> Str? => body

// ===== the host =====
//
// What only the host can do, behind one interface each backend implements
// in std's platform root (`lib/platform/std/runtime.{rs,kt}`): a new
// backend writes this handler, and the scheduler above it is Salvo.
// @compilerrules [runtime-host]
effect RuntimeHost {
    // A fresh value an outsider cannot guess: OS entropy.
    // @compilerrules [net-addr-capability]
    fn secure_bits() -> Long
    // A line on standard error, for the runtime's named reports.
    fn report(line: Str) -> None => line
    // The monotonic clock, in nanoseconds: the timeline
    // `time.tick()` reads, so a deadline and a reading agree.
    // @compilerrules [time-timer]
    fn mono_nanos() -> Long
}

threadsafe platform handler HostRuntime() for RuntimeHost

// The runtime's host, bound once for the process: every function here
// reaches it without declaring it.
// @compilerrules [mod-use]
use HostRuntime()

// Fresh random bits for an addr's identity: OS entropy, or the test's
// seed on the virtual runtime.
// @compilerrules [net-addr-capability, test-actor]
fn fresh_bits() [] -> Long {
    if is_virtual() {
        return random_bits()
    }
    return secure_bits()
}

// The monotonic clock, in nanoseconds: the timeline `time.tick()` reads,
// so a timer's deadline and a reading agree. On the virtual runtime, the
// virtual clock the scheduler moves.
// @compilerrules [time-timer, test-actor]
export fn now_nanos() [] -> Long {
    if is_virtual() {
        return virtual_now()
    }
    return mono_nanos()
}

// ===== the scheduler =====
//
// The scheduler, written once, and driven by `lib/std/runtime.test.sv`.
// @compilerrules [runtime-sched]

// ---- host types

// A value with its type erased: a message, an answer or a fault report on
// its way through a queue.
//
// It is linear, so a queue cannot lose one silently: every `Dyn` is read
// back with [unerase] or dropped with [drop_dyn], and the drop is written
// where it happens. On the Rust backend it is a heap box.
export linear platform type Dyn
// Erases [v]: the value, moved into a `Dyn`.
export platform fn erase<T canbe linear>(v: T) [] -> Dyn => !v
// The value [d] holds, read back as a [T]. Reading it as a type other than
// the one it was erased from is a bug in the runtime, and traps.
export platform fn unerase<T canbe linear>(d: Dyn) [] -> T => !d
// Drops an erased value nobody is owed anything for, such as a message
// sent to an actor that has died.
export platform fn drop_dyn(d: Dyn) [] -> None => !d

// An actor's body: its handler and the dispatch that runs one activation.
//
// It is linear and has one owner at a time, the actor table or the thread
// running an activation, and it moves between them. That is how an actor
// never runs twice at once: a second thread finds no body to run. A task's
// body travels inside its reply token instead, and runs once.
export linear platform type Body
// Makes a body from [f], the function the host keeps and calls once per
// activation.
//
// [f] is told what to run: [kind] 0 for a message in [value], 1 for an
// answer to the continuation parked at [slot], 2 for a fault report (a
// `Str` reason, from which the body builds its protocol's message).
export platform fn body_of(f: (kind: Int, slot: Long, value: Dyn) -> None) [] -> Body
=>[f] !value => !f
// Runs one activation of [b] inside the fault boundary, and hands the body
// back with `None`, or with the fault that ended the activation.
platform fn activate(b: Body, kind: Int, slot: Long, value: Dyn) [] -> Ran
=> !b, !kind, !slot, !value
// Drops a body: the actor's handler and state with it. For a body whose
// actor has died, or a task that will never run.
export platform fn drop_body(b: Body) [] -> None => !b

// Grants node [from] one credit for actor [addr], on
// [pool]: a GRANT frame, staged by the routing service and sent once the
// scheduler's lock is released (`flush_frames`). Called with the lock held,
// so it must not call back into the scheduler.
// @compilerrules [net-remote-backpressure]
platform fn granted(addr: Int, pool: Int, from: Long) [] -> None => addr, pool, from

// Sends the frames the routing layer staged under the scheduler's lock.
platform fn flush_frames() [] -> None

// Ends the process with [code]: the runtime's named reports that cannot be
// recovered from (a deadlock, a wedged main pool).
platform fn exit_process(code: Int) [] -> Never => code

linear struct Ran {
    body: Body,
    fault: Str?
}

fn drop_ran(r: Ran) [] -> None => !r {
    let {body, fault} = r
    drop_body(body)
}

// A cell holding at most one linear value, taken out and put back through a
// `Mut` handle: how a body leaves the table for an activation and returns,
// and where a waiter's answer waits for it.
linear platform type Slot<T canbe linear> canbe Mut
platform fn slot_of<T canbe linear>(v: T) [] -> Mut Slot<T> => !v
platform fn slot_empty<T canbe linear>() [] -> Mut Slot<T>
platform fn slot_take<T canbe linear>(s: Mut Slot<T>) [] -> T? => s: Mut
platform fn slot_put<T canbe linear>(s: Mut Slot<T>, v: T) [] -> None => s: Mut, !v
platform fn drop_slot<T canbe linear>(s: Slot<T>) [] -> None => !s

// Where the calling thread is: the pool it works for (what an `on`-less
// spawn or mint inherits) and the actor whose activation it is inside, or
// -1 — the one actor a wait must not serve. A thread-local.
platform fn here_pool() [] -> Int
platform fn here_actor() [] -> Int
platform fn set_here(pool: Int, actor: Int) [] -> None => pool, actor

// ---- the table

// The index of `main`'s pool, 0. It has no thread of its own: `main`'s
// thread is its one worker, and runs its work only while it waits.
// @compilerrules [actor-main-pool]
export fn main_pool() [] -> Int {
    return 0
}

// What `here_actor` answers outside any frame (main's own thread, a worker
// between jobs), and inside a task's frame.
fn no_frame() [] -> Int {
    return -1
}

fn task_frame() [] -> Int {
    return -2
}

// A user message, an answer to the continuation parked at a slot, and a
// fault report to a pool's sink.
// [from] is the node a message came over the wire
// from (-1 for a local send), which is granted a credit back when it is
// dequeued.
// @compilerrules [actor-pool-fault-sink, net-remote-backpressure]
linear struct Delivered { msg: Dyn, from: Long }
linear struct Answered { slot: Long, value: Dyn }
struct Reported { reason: Str }
type Entry = Delivered | Answered | Reported

fn drop_delivered(d: Delivered) [] -> None => !d {
    let {msg, from} = d
    drop_dyn(msg)
}

fn drop_answered(a: Answered) [] -> None => !a {
    let {slot, value} = a
    drop_dyn(value)
}

fn drop_entry(e: Entry) [] -> None => !e {
    if e is Delivered d {
        drop_delivered(d)
    } elif e is Answered a {
        drop_answered(a)
    } else {
    }
}

// One actor, as the table keeps it.
linear struct ActorRec canbe Mut {
    body: Mut Slot<Body>,
    pool: Int,
    bound: Int,
    queue: Mut Deque<Entry>,
    // Beside each entry: the slot it answers, or -1 — what the gate reads
    // without touching the entries themselves.
    slots: Mut Deque<Long>,
    // User messages queued: answers and reports do not count against the
    // bound.
    user_len: Int,
    // While set, only the answer for this slot is delivered.
    gate: Long?,
    running: Bool,
    dead: Bool,
    exit_reason: Str,
    // Senders parked on a full mailbox, woken one per dequeue.
    blocked: Mut List<Parker>,
    // Tokens to answer with an `Exit` when it dies.
    // @compilerrules [actor-watch]
    watchers: Mut List<Token>,
    // Tokens aimed at it that nobody has answered.
    // @compilerrules [actor-on-idle]
    owed: Int,
    // On its pool's ready queue.
    ready: Bool,
    // A proxy of an actor on another node: a send to it is
    // handed back, to be encoded and routed.
    // @compilerrules [net-addr-routable]
    proxy: Bool
}

fn drop_actor_rec(a: ActorRec) [] -> None => !a {
    let {body, pool, bound, queue, slots, user_len, gate, running, dead, exit_reason, blocked, watchers, owed, ready, proxy} = a
    drop_slot(body)
    drain(queue, (e) -> drop_entry(e))
    drain(watchers, (t) -> drop_token(t))
}

// A frame waiting in `await_answer`, and the answer once it arrives.
linear struct WaiterRec canbe Mut {
    pool: Int,
    value: Mut Slot<Dyn>,
    filled: Bool,
    parker: Parker?,
    // While the frame waits: what kind of frame it is (1 an activation or a
    // task, 2 main's own thread, 0 not waiting), and the actor whose
    // activation it is, or -1.
    waiting: Int,
    waiting_actor: Int,
    // The slot of the token minted for this wait: a record is
    // reused once its wait has ended, and an answer carrying another slot
    // (late, or forged from the wire) is not this wait's.
    // @compilerrules [waitfor-pump]
    slot: Long
}

fn drop_waiter_rec(w: WaiterRec) [] -> None => !w {
    let {pool, value, filled, parker, waiting, waiting_actor, slot} = w
    drop_slot(value)
}

// A task whose answer has arrived: its body and the answer, ready to run.
linear struct TaskRun { body: Body, value: Dyn }

fn drop_task_run(t: TaskRun) [] -> None => !t {
    let {body, value} = t
    drop_body(body)
    drop_dyn(value)
}

linear struct PoolRec canbe Mut {
    // The parkers of its threads with nothing to do.
    idle: Mut List<Parker>,
    tasks: Mut Deque<TaskRun>,
    // Actors with an entry they may be activated for, in the order they
    // became so: what a thread looking for work takes from, instead of
    // scanning the table. An entry may be stale (the actor since ran or
    // died); a taker drops it, and the actor is queued again when it next
    // has something to run.
    ready: Mut Deque<Int>,
    // The actor its uncaught faults go to, or -1.
    // @compilerrules [actor-pool-fault-sink]
    sink: Int,
    // Tokens owed to work here that belongs to no actor.
    // @compilerrules [actor-on-idle]
    owed: Int,
    // A `Dedicated` pool: made for one spawn, so nothing is
    // placed on it but what its actors place there themselves.
    // @compilerrules [actor-pool-retire]
    dedicated: Bool,
    // Nothing can run here any more: its threads return.
    // @compilerrules [actor-pool-retire]
    retired: Bool
}

fn drop_pool_rec(p: PoolRec) [] -> None => !p {
    let {idle, tasks, ready, sink, owed, dedicated, retired} = p
    drain(tasks, (t) -> drop_task_run(t))
}

// Where an answer goes: an actor's parked continuation, a
// waiting frame, or a task — whose body travels inside the token until the
// answer schedules it.
// @compilerrules [actor-replyto, task-mint]
struct ToActor { addr: Int }
struct ToWaiter { wid: Int }
linear struct ToTask { pool: Int, body: Body }
type Target = ToActor | ToWaiter | ToTask

fn drop_to_task(t: ToTask) [] -> None => !t {
    let {pool, body} = t
    drop_body(body)
}

// The scheduler's side of a [Reply]: a one-shot answer aimed at a target.
//
// The target is an actor's parked continuation, a frame waiting in
// `waitfor`, or a task, whose body travels inside the token until the
// answer schedules it. [slot] tells a waiting frame's answer from a late or
// forged one, and an actor's continuations apart. A token is linear, so
// it is answered exactly once ([answer]).
//
// [tracked] says whether it still counts as owed in what [on_idle]
// reports: a token the scheduler holds itself, for a watch or an idle
// hook, does not.
// @compilerrules [actor-replyto, actor-on-idle]
export linear struct Token {
    target: Target,
    slot: Long,
    tracked: Bool
}

// Answers a token nobody will read, so its obligation ends.
fn drop_token(t: Token) [] -> None => !t {
    answer(t, erase(0))
}

// What [waiter] answers: the token to hand to whoever will answer, and the
// waiter record [wid] to wait on with [await_answer].
export linear struct WaiterMint {
    token: Token,
    wid: Int
}

fn drop_waiter_mint(m: WaiterMint) [] -> None => !m {
    let {token, wid} = m
    drop_token(token)
}

// What a thread with work to do runs: an actor's activation or a task.
linear struct RunActor { addr: Int, pool: Int, kind: Int, slot: Long, value: Dyn, body: Body }
linear struct RunTask { pool: Int, body: Body, value: Dyn }
type Work = RunActor | RunTask
// What a worker of a retired pool is told: return.
// @compilerrules [actor-pool-retire]
struct Retire {}

fn drop_run_actor(a: RunActor) [] -> None => !a {
    let {addr, pool, kind, slot, value, body} = a
    drop_dyn(value)
    drop_body(body)
}

fn drop_run_task(t: RunTask) [] -> None => !t {
    let {pool, body, value} = t
    drop_body(body)
    drop_dyn(value)
}

struct Sent {}
struct Dead {}
linear struct Full { msg: Dyn }
linear struct Remote { msg: Dyn }

fn drop_remote(r: Remote) [] -> None => !r {
    let {msg} = r
    drop_dyn(msg)
}

fn drop_full(f: Full) [] -> None => !f {
    let {msg} = f
    drop_dyn(msg)
}

// A registered quiescence hook: the pool it reports on, and
// the token its `Idle` answers.
// @compilerrules [actor-on-idle]
linear struct IdleHook { pool: Int, token: Token }

fn drop_idle_hook(h: IdleHook) [] -> None => !h {
    let {pool, token} = h
    drop_token(token)
}

// What one step of a wait found: the answer, work to run meanwhile, nothing
// (its parker is recorded: sleep), or the deadlock report.
linear struct Got { value: Dyn }
struct Sleep {}
// Something changed under the lock (a hook fired): look again before
// sleeping, since the change may be this waiter's own answer.
struct Again {}
struct Stuck { report: Str }
type WaitStep = Got | RunActor | RunTask | Sleep | Again | Stuck

fn drop_got(g: Got) [] -> None => !g {
    let {value} = g
    drop_dyn(value)
}

effect SchedTable {
    fn new_pool(sink: Int, dedicated: Bool) -> Int => !sink, !dedicated
    fn new_actor(pool: Int, bound: Int, body: Body) -> Int => !pool, !bound, !body
    // Enqueues a message: `Sent`, `Dead` (dropped), or `Full` — the message
    // comes back and [waiter] is recorded to be woken by the next dequeue.
    fn enqueue(addr: Int, msg: Dyn, waiter: Parker) -> Sent | Dead | Full | Remote => !addr, !msg, !waiter
    fn set_proxy(addr: Int) -> None => !addr
    // A message that came over the wire from [from]: past
    // nothing — its room was granted as a credit — and granted back when
    // dequeued. `false` for an unknown or dead actor (the message dropped).
    // @compilerrules [net-addr-routable]
    fn enqueue_remote(addr: Int, msg: Dyn, from: Long) -> Bool => !addr, !msg, !from
    // The actor dies of [reason] without an activation: a proxy
    // of an actor on a node that left.
    // @compilerrules [node-exit]
    fn kill(addr: Int, reason: Str) -> None => !addr, !reason
    fn mint_actor(addr: Int, gated: Bool) -> Token => !addr, !gated
    fn mint_task(pool: Int, body: Body) -> Token => !pool, !body
    fn mint_waiter(pool: Int) -> WaiterMint => !pool
    // Delivers an answer: never blocks (an answer's room is reserved), and a
    // no-op to the dead.
    fn deliver(t: Token, value: Dyn) -> None => !t, !value
    fn watch_actor(addr: Int, t: Token) -> None => !addr, !t
    fn idle_hook(pool: Int, t: Token) -> None => !pool, !t
    // The next work on [pool]; `None` having recorded [idle] to be woken
    // when work arrives there.
    fn next_work(pool: Int, idle: Parker) -> RunActor | RunTask | Retire | None => !pool, !idle
    // Worker threads that have returned from retired pools.
    // @compilerrules [actor-pool-retire]
    fn retired_workers() -> Int
    // Waiter records in the table, free ones included.
    // @compilerrules [waitfor-pump]
    fn waiter_records() -> Int
    // One step of waiting on [wid] from a frame of kind [frame] (1 an
    // activation or task, 2 main's thread) inside [own], serving [pool].
    fn wait_step(wid: Int, pool: Int, own: Int, frame: Int, me: Parker) -> WaitStep
    => !wid, !pool, !own, !frame, !me
    // An activation finished: the body goes back, or the actor dies of
    // [fault].
    fn finish(addr: Int, body: Body, fault: Str?) -> None => !addr, !body, !fault
    fn task_done(pool: Int, fault: Str?) -> None => !pool, !fault
    fn pool_of_actor(addr: Int) -> Int => addr
    fn pool_of_waiter(wid: Int) -> Int => wid
    fn external(delta: Int) -> None => !delta
    // Room left in [addr]'s mailbox, and messages
    // queued there.
    // @compilerrules [net-remote-backpressure]
    fn room(addr: Int) -> Int => addr
    fn is_dead(addr: Int) -> Bool => addr
    fn queued(addr: Int) -> Int => addr
    // The virtual runtime: enters it with randomness from
    // [seed], leaving a fresh scheduler (every actor so far dead, silently).
    // @compilerrules [test-actor]
    fn go_virtual(seed: Long) -> None => !seed
    fn is_virtual() -> Bool
    fn virtual_now() -> Long
    // Moves the virtual clock forward to [at] (never back).
    fn advance_to(at: Long) -> None => !at
    fn random_bits() -> Long
    // A uniform double in [0, 1): from the seed on the
    // virtual runtime, from a generator seeded by OS entropy otherwise.
    // @compilerrules [std-random-default]
    fn random_unit() -> Double
    // A token answered when nothing can run: what moves virtual time.
    fn clock_hook(t: Token) -> None => !t
    // Work from any pool but [own]'s activations, for a send waiting for room.
    fn virtual_work(own: Int) -> RunActor | RunTask | None => !own
}

handler Scheduler() for SchedTable {
    state {
        actors: Mut List<Mut ActorRec> = Mut List()
        waiters: Mut List<Mut WaiterRec> = Mut List()
        // Waiter records whose wait has ended, to be reused: the
        // table stays as large as the most waits ever open at once.
        // @compilerrules [waitfor-pump]
        free_waiters: Mut List<Int> = Mut List()
        pools: Mut List<Mut PoolRec> = Mut List()
        idle_hooks: Mut List<IdleHook> = Mut List()
        next_slot: Long = 0
        // Frames running — activations and tasks — and, of those, the ones
        // parked in a wait; how deep main's own thread is in a wait; outside
        // sources of work.
        // @compilerrules [platform-threadsafe]
        active: Int = 0
        parked_frames: Int = 0
        main_waits: Int = 0
        externals: Int = 0
        // Worker threads that have returned.
        // @compilerrules [actor-pool-retire]
        retired: Int = 0
        // The virtual runtime: no worker threads, a clock moved by
        // the scheduler, randomness from a seed, and the hooks that move time.
        // @compilerrules [test-actor]
        virtual_mode: Bool = false
        vnow: Long = 0
        rng: Long = 1
        // Whether `rng` has been seeded outside the virtual
        // runtime, which seeds it itself.
        // @compilerrules [std-random-default]
        rng_seeded: Bool = false
        clock_hooks: Mut List<Token> = Mut List()
    }

    // Pool 0 exists from the start and belongs to `main`.
    // @compilerrules [actor-main-pool]
    init {
        add(pools, Mut PoolRec { idle: Mut List(), tasks: mut_deque_of(), ready: mut_deque_of(), sink: -1, owed: 0, dedicated: false, retired: false })
    }

    fn new_pool(sink: Int, dedicated: Bool) -> Int => !sink, !dedicated {
        add(pools, Mut PoolRec { idle: Mut List(), tasks: mut_deque_of(), ready: mut_deque_of(), sink: sink, owed: 0, dedicated: dedicated, retired: false })
        return size(pools) - 1
    }

    fn new_actor(pool: Int, bound: Int, body: Body) -> Int => !pool, !bound, !body {
        add(actors, Mut ActorRec {
            body: slot_of(body), pool: pool, bound: bound, queue: mut_deque_of(), slots: mut_deque_of(),
            user_len: 0, gate: None, running: false, dead: false, exit_reason: "",
            blocked: Mut List(), watchers: Mut List(), owed: 0, ready: false, proxy: false
        })
        return size(actors) - 1
    }

    fn enqueue(addr: Int, msg: Dyn, waiter: Parker) -> Sent | Dead | Full | Remote => !addr, !msg, !waiter {
        if addr < 0 || addr >= size(actors) {
            drop_dyn(msg)
            return Dead {}
        }
        let a = at(actors, addr) ?: unreachable!()
        if a.dead {
            drop_dyn(msg)
            return Dead {}
        }
        if a.proxy {
            return Remote { msg: msg }
        }
        if a.user_len >= a.bound {
            add(a.blocked, waiter)
            return Full { msg: msg }
        }
        let e: Entry = Delivered { msg: msg, from: to_long(-1) }
        add_last(a.queue, e)
        add_last(a.slots, to_long(-1))
        a.user_len = a.user_len + 1
        if mark_ready(a, pools, copy(addr)) {
            let pool = copy(a.pool)
            wake_pool(pools, pool)
        }
        return Sent {}
    }

    fn enqueue_remote(addr: Int, msg: Dyn, from: Long) -> Bool => !addr, !msg, !from {
        if addr < 0 || addr >= size(actors) {
            drop_dyn(msg)
            return false
        }
        let a = at(actors, addr) ?: unreachable!()
        if a.dead {
            drop_dyn(msg)
            return false
        }
        let e: Entry = Delivered { msg: msg, from: from }
        add_last(a.queue, e)
        add_last(a.slots, to_long(-1))
        a.user_len = a.user_len + 1
        if mark_ready(a, pools, copy(addr)) {
            let pool = copy(a.pool)
            wake_pool(pools, pool)
        }
        return true
    }

    fn kill(addr: Int, reason: Str) -> None => !addr, !reason {
        let a = at(actors, addr) ?: unreachable!()
        if a.dead {
            return
        }
        a.dead = true
        a.exit_reason = copy(reason)
        while remove_first(a.blocked) is Parker b {
            unpark(b)
        }
        let watchers: Mut List<Token> = Mut List()
        while remove_first(a.watchers) is Token t {
            add(watchers, t)
        }
        while remove_first(watchers) is Token t {
            deliver_to(actors, waiters, pools, t, erase(Exit { reason: copy(reason) }))
        }
        drain(watchers, (t) -> drop_token(t))
    }

    fn mint_actor(addr: Int, gated: Bool) -> Token => !addr, !gated {
        next_slot = next_slot + 1
        let slot = copy(next_slot)
        let a = at(actors, addr) ?: unreachable!()
        if gated {
            a.gate = copy(slot)
        }
        a.owed = a.owed + 1
        return Token { target: ToActor { addr: addr }, slot: slot, tracked: true }
    }

    fn mint_task(pool: Int, body: Body) -> Token => !pool, !body {
        next_slot = next_slot + 1
        let p = at(pools, pool) ?: unreachable!()
        p.owed = p.owed + 1
        return Token { target: ToTask { pool: pool, body: body }, slot: copy(next_slot), tracked: true }
    }

    fn mint_waiter(pool: Int) -> WaiterMint => !pool {
        let p = at(pools, pool) ?: unreachable!()
        p.owed = p.owed + 1
        next_slot = next_slot + 1
        let wid = reuse_waiter(waiters, free_waiters, copy(pool), copy(next_slot))
        let t = Token { target: ToWaiter { wid: copy(wid) }, slot: copy(next_slot), tracked: true }
        return WaiterMint { token: t, wid: wid }
    }

    fn deliver(t: Token, value: Dyn) -> None => !t, !value {
        deliver_to(actors, waiters, pools, t, value)
    }

    fn watch_actor(addr: Int, t: Token) -> None => !addr, !t {
        let watch = untrack(actors, waiters, pools, t)
        let a = at(actors, addr) ?: unreachable!()
        if a.dead {
            let reason = copy(a.exit_reason)
            deliver_to(actors, waiters, pools, watch, erase(Exit { reason: reason }))
            return
        }
        add(a.watchers, watch)
    }

    fn idle_hook(pool: Int, t: Token) -> None => !pool, !t {
        let hook = untrack(actors, waiters, pools, t)
        add(idle_hooks, IdleHook { pool: pool, token: hook })
        wake_all_pools(pools)
    }

    fn waiter_records() -> Int {
        return size(waiters)
    }

    fn next_work(pool: Int, idle: Parker) -> RunActor | RunTask | Retire | None => !pool, !idle {
        if (get(pools, pool) ?: unreachable!()).retired {
            retired = retired + 1
            return Retire {}
        }
        let w = take_work(actors, pools, copy(pool), -1)
        if w is RunActor ra {
            active = active + 1
            return ra
        }
        if w is RunTask rt {
            active = active + 1
            return rt
        }
        let p = at(pools, pool) ?: unreachable!()
        add(p.idle, idle)
        // [waitfor-pump] [actor-on-idle] Every frame still running is
        // parked in a wait: the scheduler may have settled, idle or
        // stuck. The hooks fire here; whether it is stuck only a waiting
        // frame can tell, so the waiting frames look again.
        if active == parked_frames && quiet(actors, waiters, pools, externals) {
            if !(size(idle_hooks) == 0) && active == 0 {
                fire_idle(actors, waiters, pools, idle_hooks)
            }
            wake_waiters(waiters)
        }
        return None
    }

    fn retired_workers() -> Int {
        return copy(retired)
    }

    fn wait_step(wid: Int, pool: Int, own: Int, frame: Int, me: Parker) -> WaitStep
    => !wid, !pool, !own, !frame, !me {
        let w = at(waiters, wid) ?: unreachable!()
        // [waitfor-pump] Booked once, on the first step: an activation or a
        // task counts out of the frames that can still proceed; main's own
        // thread records that it is inside a wait at all.
        if w.waiting == 0 {
            w.waiting = copy(frame)
            w.waiting_actor = copy(own)
            if frame == 1 {
                parked_frames = parked_frames + 1
            } else {
                main_waits = main_waits + 1
            }
        }
        let got = slot_take(w.value)
        if got is Dyn v {
            w.filled = false
            w.parker = None
            if w.waiting == 1 {
                parked_frames = parked_frames - 1
            } else {
                main_waits = main_waits - 1
            }
            w.waiting = 0
            w.waiting_actor = -1
            // The token is spent and the wait over: the record is free.
            add(free_waiters, copy(wid))
            // This thread may have been woken for work, and leaves with its
            // answer instead: the wake is passed on.
            wake_pool(pools, copy(pool))
            return Got { value: v }
        }
        // [test-actor] On the virtual runtime the one thread serves every
        // pool.
        let work = take_for(actors, pools, copy(pool), copy(own), copy(virtual_mode))
        if work is RunActor ra {
            active = active + 1
            return ra
        }
        if work is RunTask rt {
            active = active + 1
            return rt
        }
        let q = quiet(actors, waiters, pools, externals)
        // [test-actor] Nothing can run on the one thread: time moves to the
        // next deadline, before anything is called idle or stuck.
        if virtual_mode && size(clock_hooks) > 0 && q {
            fire_clock(actors, waiters, pools, clock_hooks)
            return Again {}
        }
        // [actor-on-idle] Nothing to run is exactly when the hooks fire, and
        // firing one is progress — so it comes before the deadlock report.
        if !(size(idle_hooks) == 0) && active == 0 && q {
            fire_idle(actors, waiters, pools, idle_hooks)
            return Again {}
        }
        // [test-actor] With one thread, a park could never be woken.
        if virtual_mode || (active == parked_frames && main_waits > 0 && q) {
            return Stuck { report: deadlock_report(actors, waiters, own) }
        }
        let parked = at(waiters, wid) ?: unreachable!()
        parked.parker = copy(me)
        let p = at(pools, pool) ?: unreachable!()
        add(p.idle, me)
        return Sleep {}
    }

    fn finish(addr: Int, body: Body, fault: Str?) -> None => !addr, !body, !fault {
        active = active - 1
        let a = at(actors, addr) ?: unreachable!()
        a.running = false
        if fault is None {
            slot_put(a.body, body)
            // The actor's own next entry, which could not run while it did:
            // the finishing thread looks for work again at once and takes it.
            let _again = mark_ready(a, pools, copy(addr))
            return
        }
        drop_body(body)
        let reason = copy(fault)
        a.dead = true
        a.exit_reason = copy(reason)
        a.gate = None
        a.user_len = 0
        while size(a.queue) > 0 {
            let entry = remove_first(a.queue)
            assert!(entry !is None)
            drop_entry(entry)
        }
        while size(a.slots) > 0 {
            let _s = remove_first(a.slots)
        }
        while remove_first(a.blocked) is Parker b {
            unpark(b)
        }
        let pool = copy(a.pool)
        let watchers: Mut List<Token> = Mut List()
        while remove_first(a.watchers) is Token t {
            add(watchers, t)
        }
        // [actor-pool-fault-sink] The net under supervision: a death nobody was
        // watching still reaches the pool's sink, or the named report.
        if size(watchers) == 0 {
            report_fault(actors, pools, copy(pool), copy(reason))
        }
        while remove_first(watchers) is Token t {
            deliver_to(actors, waiters, pools, t, erase(Exit { reason: copy(reason) }))
        }
        drain(watchers, (t) -> drop_token(t))
        retire_if_done(actors, pools, copy(pool))
        wake_all_pools(pools)
    }

    fn task_done(pool: Int, fault: Str?) -> None => !pool, !fault {
        active = active - 1
        if fault is Str reason {
            report_fault(actors, pools, copy(pool), copy(reason))
        }
        retire_if_done(actors, pools, pool)
    }

    fn pool_of_actor(addr: Int) -> Int => addr {
        return copy((get(actors, addr) ?: unreachable!()).pool)
    }

    fn set_proxy(addr: Int) -> None => !addr {
        let a = at(actors, addr) ?: unreachable!()
        a.proxy = true
    }

    fn pool_of_waiter(wid: Int) -> Int => wid {
        return copy((get(waiters, wid) ?: unreachable!()).pool)
    }

    fn room(addr: Int) -> Int => addr {
        let a = get(actors, addr) ?: unreachable!()
        return a.bound - a.user_len
    }

    fn is_dead(addr: Int) -> Bool => addr {
        return copy((get(actors, addr) ?: unreachable!()).dead)
    }

    fn queued(addr: Int) -> Int => addr {
        return copy((get(actors, addr) ?: unreachable!()).user_len)
    }

    fn external(delta: Int) -> None => !delta {
        externals = externals + delta
        if externals < 0 {
            externals = 0
        }
        wake_all_pools(pools)
    }

    fn go_virtual(seed: Long) -> None => !seed {
        virtual_mode = true
        vnow = 0
        rng = seed_of(seed)
        reset_all(actors, pools, idle_hooks, clock_hooks)
        active = 0
        parked_frames = 0
        main_waits = 0
        externals = 0
    }

    fn is_virtual() -> Bool {
        return copy(virtual_mode)
    }

    fn virtual_now() -> Long {
        return copy(vnow)
    }

    fn advance_to(at: Long) -> None => !at {
        if at > vnow {
            vnow = at
        }
    }

    fn random_unit() -> Double {
        if !virtual_mode && !rng_seeded {
            rng = seed_of(secure_bits())
            rng_seeded = true
        }
        let hi = lehmer(copy(rng))
        let lo = lehmer(copy(hi))
        rng = copy(lo)
        // Two 31-bit draws, each in 1..2^31-2, as one number in [0, m^2).
        let m = 2147483646L
        return to_double((hi - 1L) * m + (lo - 1L)) / (to_double(m) * to_double(m))
    }

    fn random_bits() -> Long {
        let hi = lehmer(copy(rng))
        let lo = lehmer(copy(hi))
        rng = copy(lo)
        return hi * 2147483647 + lo
    }

    fn clock_hook(t: Token) -> None => !t {
        let hook = untrack(actors, waiters, pools, t)
        add(clock_hooks, hook)
    }

    fn virtual_work(own: Int) -> RunActor | RunTask | None => !own {
        let w = take_for(actors, pools, 0, own, true)
        if w is RunActor ra {
            active = active + 1
            return ra
        }
        if w is RunTask rt {
            active = active + 1
            return rt
        }
        return None
    }
}

// The next value of a Lehmer generator: in 1..2^31-2, and no
// product past 2^47, so nothing overflows.
// @compilerrules [test-actor]
fn lehmer(x: Long) [] -> Long => !x {
    return x * 48271 % 2147483647
}

// A generator state from a seed: never 0, which is the one
// state a Lehmer generator cannot leave.
// @compilerrules [test-actor]
fn seed_of(seed: Long) [] -> Long => !seed {
    let s = seed % 2147483646
    if s < 0 {
        s = 0 - s
    }
    return s + 1
}

// A fresh scheduler, keeping the table's indices: every actor is
// dead, without a report or an `Exit`, so an addr or a token left from before
// is a send to the dead; every pool's queued work is dropped, and every pool
// but main's retired.
// @compilerrules [test-actor]
fn reset_all(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, idle_hooks: Mut List<IdleHook>, clock_hooks: Mut List<Token>) [] -> None
=> actors: Mut, pools: Mut, idle_hooks: Mut, clock_hooks: Mut {
    let k = 0
    while k < size(actors) {
        let a = at(actors, k) ?: unreachable!()
        k = k + 1
        a.dead = true
        a.running = false
        a.ready = false
        a.gate = None
        a.user_len = 0
        a.owed = 0
        while size(a.queue) > 0 {
            let entry = remove_first(a.queue)
            assert!(entry !is None)
            drop_entry(entry)
        }
        while remove_first(a.slots) is Long {
        }
        while remove_first(a.watchers) is Token t {
            drop_token(t)
        }
        while remove_first(a.blocked) is Parker {
        }
        if slot_take(a.body) is Body b {
            drop_body(b)
        }
    }
    let i = 0
    while i < size(pools) {
        let p = at(pools, i) ?: unreachable!()
        while remove_first(p.tasks) is TaskRun t {
            drop_task_run(t)
        }
        while remove_first(p.ready) is Int {
        }
        p.owed = 0
        if i > 0 {
            p.retired = true
        }
        i = i + 1
    }
    while remove_first(idle_hooks) is IdleHook h {
        drop_idle_hook(h)
    }
    while remove_first(clock_hooks) is Token t {
        drop_token(t)
    }
}

// Answers the hook that moves virtual time.
// @compilerrules [test-actor]
fn fire_clock(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>, hooks: Mut List<Token>) [] -> None
=> actors: Mut, waiters: Mut, pools: Mut, hooks: Mut {
    if remove_first(hooks) is Token t {
        deliver_to(actors, waiters, pools, t, erase(0))
    }
}

// The next work for a thread serving [pool] — or, on the virtual runtime
// ([any]), every pool in order.
// @compilerrules [test-actor]
fn take_for(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, pool: Int, exclude: Int, any: Bool) [] -> Work?
=> actors: Mut, pools: Mut, !pool, !exclude, !any {
    if !any {
        return take_work(actors, pools, pool, exclude)
    }
    let i = 0
    while i < size(pools) {
        let w = take_work(actors, pools, copy(i), copy(exclude))
        if w is RunActor ra {
            return ra
        }
        if w is RunTask rt {
            return rt
        }
        i = i + 1
    }
    return None
}

// A token the scheduler takes over (a watch, an idle hook):
// no longer counted as owed by the program, since the scheduler answers it.
// @compilerrules [actor-on-idle]
fn untrack(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>, t: Token) [] -> Token
=> actors: Mut, waiters: Mut, pools: Mut, !t {
    let {target, slot, tracked} = t
    if tracked {
        release(actors, waiters, pools, target)
    }
    return Token { target: target, slot: slot, tracked: false }
}

// One owed token fewer, against whatever it was owed by.
// @compilerrules [actor-on-idle]
fn release(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>, target: Target) [] -> None
=> actors: Mut, waiters: Mut, pools: Mut, target {
    if target is ToActor to {
        let a = at(actors, to.addr) ?: unreachable!()
        if a.owed > 0 {
            a.owed = a.owed - 1
        }
    } elif target is ToWaiter tw {
        let pool = copy((get(waiters, tw.wid) ?: unreachable!()).pool)
        let p = at(pools, pool) ?: unreachable!()
        if p.owed > 0 {
            p.owed = p.owed - 1
        }
    } else {
        let p = at(pools, target.pool) ?: unreachable!()
        if p.owed > 0 {
            p.owed = p.owed - 1
        }
    }
}

// A waiter record for a new wait of slot [slot] on [pool]: a
// freed one when there is one, else a new one. Answers its index.
// @compilerrules [actor-replyto, waitfor-pump]
fn reuse_waiter(waiters: Mut List<Mut WaiterRec>, free: Mut List<Int>, pool: Int, slot: Long) [] -> Int
=> waiters: Mut, free: Mut, !pool, !slot {
    if remove_first(free) is Int wid {
        let w = at(waiters, wid) ?: unreachable!()
        w.pool = pool
        w.filled = false
        w.parker = None
        w.waiting = 0
        w.waiting_actor = -1
        w.slot = slot
        return wid
    }
    add(waiters, Mut WaiterRec {
        pool: pool, value: slot_empty(), filled: false, parker: None, waiting: 0, waiting_actor: -1, slot: slot
    })
    return size(waiters) - 1
}

// An answer, to wherever its token was aimed.
fn deliver_to(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>,
    t: Token, value: Dyn) [] -> None
    => actors: Mut, waiters: Mut, pools: Mut, !t, !value {
    let {target, slot, tracked} = t
    if tracked {
        release(actors, waiters, pools, target)
    }
    if target is ToActor to {
        let a = at(actors, to.addr) ?: unreachable!()
        if a.dead {
            drop_dyn(value)
            return
        }
        add_last(a.slots, copy(slot))
        let e: Entry = Answered { slot: slot, value: value }
        add_last(a.queue, e)
        if mark_ready(a, pools, copy(to.addr)) {
            let pool = copy(a.pool)
            wake_pool(pools, pool)
        }
    } elif target is ToWaiter tw {
        let w = at(waiters, tw.wid) ?: unreachable!()
        if w.slot != slot || w.filled {
            drop_dyn(value)
            return
        }
        slot_put(w.value, value)
        w.filled = true
        if w.parker is Parker p {
            unpark(copy(p))
        }
    } else {
        let {pool, body} = target
        let p = at(pools, pool) ?: unreachable!()
        add_last(p.tasks, TaskRun { body: body, value: value })
        wake_pool(pools, copy(pool))
    }
}

// An uncaught fault on [pool]: a report to its sink when it
// has a live one — enqueued past the bound, since a fault report must not
// block the faulting thread — or the named report on stderr.
// @compilerrules [actor-pool-fault-sink]
fn report_fault(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, pool: Int, reason: Str) [] -> None
=> actors: Mut, pools: Mut, !pool, !reason {
    let sink = copy((get(pools, pool) ?: unreachable!()).sink)
    if sink >= 0 {
        let s = at(actors, sink) ?: unreachable!()
        if !s.dead {
            let e: Entry = Reported { reason: reason }
            add_last(s.queue, e)
            add_last(s.slots, to_long(-1))
            if mark_ready(s, pools, copy(sink)) {
                let sink_pool = copy(s.pool)
                wake_pool(pools, sink_pool)
            }
            return
        }
    }
    report("salvo: an uncaught fault on pool ${pool}: ${reason}")
}

// True when no work is queued anywhere: no actor has a
// deliverable entry, no pool a ready task, no outside source is open, and no
// waiting frame has an answer sitting in its slot (its pickup is the waiting
// thread itself, one instant away).
// @compilerrules [actor-on-idle]
fn quiet(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>, externals: Int) [] -> Bool
=> actors, waiters, pools, externals {
    if externals > 0 {
        return false
    }
    for p in pools {
        if size(p.tasks) > 0 {
            return false
        }
    }
    for a in actors {
        if !a.running && !a.dead && !(deliverable(a.slots, a.gate) is None) {
            return false
        }
    }
    for w in waiters {
        if w.waiting > 0 && w.filled {
            return false
        }
    }
    return true
}

// Fires every registered hook — quiescence is a property of
// the whole scheduler, so they all see it together — each with what its pool
// is still owed.
// @compilerrules [actor-on-idle]
fn fire_idle(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>, hooks: Mut List<IdleHook>) [] -> None
=> actors: Mut, waiters: Mut, pools: Mut, hooks: Mut {
    while remove_first(hooks) is IdleHook h {
        let {pool, token} = h
        let gates = 0
        let tokens = copy((get(pools, pool) ?: unreachable!()).owed)
        for a in actors {
            if a.pool == pool {
                tokens = tokens + a.owed
                if !(a.gate is None) && !a.dead {
                    gates = gates + 1
                }
            }
        }
        deliver_to(actors, waiters, pools, token, erase(Idle { parked_gates: gates, parked_tokens: tokens }))
    }
}

// The deadlock report: nothing can run while [own] (or main)
// waits, naming the two ways a mailbox comes to be unservable — an actor
// parked in a wait, and a gated actor.
// @compilerrules [waitfor-pump]
fn deadlock_report(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, own: Int) [] -> Str
=> actors, waiters, own {
    let occupied: Mut List<Str> = Mut List()
    for w in waiters {
        if w.waiting > 0 && w.waiting_actor >= 0 {
            add(occupied, "actor ${w.waiting_actor}")
        }
    }
    let gated: Mut List<Str> = Mut List()
    let i = 0
    for a in actors {
        if !(a.gate is None) && !a.dead {
            add(gated, "actor ${i}")
        }
        i = i + 1
    }
    let who = if own >= 0 { "actor ${own}" } else { "main" }
    let clauses: Mut List<Str> = Mut List()
    if size(occupied) > 0 {
        add(clauses, "parked in a wait: ${join(occupied, ", ")}")
    }
    if size(gated) > 0 {
        add(clauses, "parked gates: ${join(gated, ", ")}")
    }
    let detail = if size(clauses) == 0 { "" } else { " (${join(clauses, "; ")})" }
    return "salvo: deadlock: nothing can run while ${who} waits${detail}"
}

// The next work on [pool]: a ready task first, then the first
// actor with a deliverable entry — honouring its gate — other than
// [exclude], whose own activation is the one waiting.
// @compilerrules [waitfor-pump]
fn take_work(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, pool: Int, exclude: Int) [] -> Work?
=> actors: Mut, pools: Mut, !pool, !exclude {
    let p = at(pools, pool) ?: unreachable!()
    let task = remove_first(p.tasks)
    if task is TaskRun t {
        let {body, value} = t
        return RunTask { pool: pool, body: body, value: value }
    }
    while remove_first(p.ready) is Int i {
        let a = at(actors, i) ?: unreachable!()
        a.ready = false
        if i != exclude && !a.running && !a.dead {
            let at = deliverable(a.slots, a.gate)
            if at is Int k {
                let _slot = remove_at(a.slots, copy(k))
                let e = remove_at(a.queue, k)
                assert!(e !is None)
                a.running = true
                let body = slot_take(a.body) ?: unreachable!()
                return work_of(a, copy(i), e, body)
            }
        }
    }
    return None
}

// Queues actor [a] (at [addr]) on its pool's ready queue when it has an
// entry it may be activated for and is neither running nor dead; answers
// whether it was queued now, which is when a thread should be woken for it.
fn mark_ready(a: Mut ActorRec, pools: Mut List<Mut PoolRec>, addr: Int) [] -> Bool => a: Mut, pools: Mut, !addr {
    if a.ready || a.running || a.dead {
        return false
    }
    if deliverable(a.slots, a.gate) is None {
        return false
    }
    a.ready = true
    let p = at(pools, copy(a.pool)) ?: unreachable!()
    add_last(p.ready, addr)
    return true
}

// The work an entry taken from actor [a] (at [addr]) makes: a message wakes
// one blocked sender; an answer to the gate opens it; a report is kind 2,
// carrying the fault's reason (the body builds its protocol's message).
fn work_of(a: Mut ActorRec, addr: Int, e: Entry, body: Body) [] -> Work? => a: Mut, !addr, !e, !body {
    if e is Delivered d {
        a.user_len = a.user_len - 1
        let woken = remove_first(a.blocked)
        if woken is Parker b {
            unpark(b)
        }
        let {msg, from} = d
        // [net-remote-backpressure] A remote sender's message left the queue:
        // that node gets one credit back for this actor.
        if from >= 0 {
            granted(copy(addr), copy(a.pool), from)
        }
        return RunActor { addr: addr, pool: copy(a.pool), kind: 0, slot: 0, value: msg, body: body }
    }
    if e is Reported r {
        return RunActor { addr: addr, pool: copy(a.pool), kind: 2, slot: 0, value: erase(copy(r.reason)), body: body }
    }
    let {slot, value} = e
    let opens = false
    if a.gate is Long g {
        opens = g == slot
    }
    if opens {
        a.gate = None
    }
    return RunActor { addr: addr, pool: copy(a.pool), kind: 1, slot: slot, value: value, body: body }
}

// The index of the entry that may be delivered next: the head, or — while
// gated — only the awaited answer.
fn deliverable(slots: Deque<Long>, gate: Long?) [] -> Int? => slots, gate {
    if size(slots) == 0 {
        return None
    }
    if gate is None {
        return 0
    }
    let i = 0
    while i < size(slots) {
        if (get(slots, i) ?: unreachable!()) == gate {
            return copy(i)
        }
        i = i + 1
    }
    return None
}

// Wakes one idle thread of [pool]: one new piece of work needs one worker,
// and waking every idle thread only has the rest find nothing and park
// again. A thread that finds nothing records itself idle under the lock, and
// a woken waiting frame that leaves with its answer instead passes the wake
// on, so work is never left with every thread asleep.
fn wake_pool(pools: Mut List<Mut PoolRec>, pool: Int) [] -> None => pools: Mut, !pool {
    let p = at(pools, pool) ?: unreachable!()
    if remove_first(p.idle) is Parker w {
        unpark(w)
    }
}

// Wakes every idle thread of [pool].
fn wake_every(pools: Mut List<Mut PoolRec>, pool: Int) [] -> None => pools: Mut, !pool {
    let p = at(pools, pool) ?: unreachable!()
    while remove_first(p.idle) is Parker w {
        unpark(w)
    }
}

// Wakes every frame parked in a wait, to look at the
// scheduler again.
// @compilerrules [waitfor-pump]
fn wake_waiters(waiters: Mut List<Mut WaiterRec>) [] -> None => waiters: Mut {
    for w in waiters {
        if w.waiting > 0 && w.parker is Parker p {
            unpark(copy(p))
        }
    }
}

// Retires [pool] when nothing can run on it any more: a
// `Dedicated` pool — nobody holds it but the spawn that consumed it — whose
// actors are all dead, with no task queued and no token owed to work there.
// Its threads are woken, see the flag and return.
// @compilerrules [actor-pool-retire]
fn retire_if_done(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, pool: Int) [] -> None
=> actors, pools: Mut, !pool {
    let p = at(pools, pool) ?: unreachable!()
    if !p.dedicated || p.retired || size(p.tasks) > 0 || p.owed > 0 {
        return
    }
    for a in actors {
        if a.pool == pool && !a.dead {
            return
        }
    }
    p.retired = true
    wake_every(pools, pool)
}

// Wakes every idle thread of every pool: something global changed (a death,
// a hook, an outside source), which any of them may have to look at.
fn wake_all_pools(pools: Mut List<Mut PoolRec>) [] -> None => pools: Mut {
    let i = 0
    while i < size(pools) {
        wake_every(pools, copy(i))
        i = i + 1
    }
}

use Scheduler()

// ---- the surface

// The index of [a] in the actor table.
//
// An address and a pool are indices into this runtime's tables, and these
// four functions are the only way between the two. Only std can call them,
// since a program cannot import this module, so a program cannot forge an
// address or a pool. They are what lets [std.core.actor] and [std.net] be
// written in Salvo over this module. Each costs nothing: a cast on Rust,
// the value itself on Kotlin.
// @compilerrules [runtime-handles]
export intrinsic fn addr_index<E>(a: eff[E]) [] -> Int => a
// The address at [index] in the actor table, as an `eff[E]`.
export intrinsic fn addr_of<E>(index: Int) [] -> eff[E] => index
// The index of [p] in the pool table.
export intrinsic fn pool_index(p: Pool) [] -> Int => p
// The pool at [index] in the pool table, as a `Pool`.
export intrinsic fn pool_of(index: Int) [] -> Pool => index
// The scheduler's [Token] inside [r]: what [std.core.actor]'s `watch` and
// `on_idle` register. `None` for a reply minted on another node, which has
// no token here.
// @compilerrules [runtime-handles]
export intrinsic fn reply_token<T canbe linear>(r: Reply<T>) [] -> Token? => !r

// Makes a pool of [n] worker threads, started now, and answers its index.
// Uncaught faults on it go to the actor at [sink], or, when [sink] is -1,
// are printed on standard error. On the virtual runtime no threads start.
// @compilerrules [runtime-sched]
export fn new_pool_of(n: Int, sink: Int) [] -> Int => n, !sink {
    return start_pool(n, sink, false)
}

// Makes a `Dedicated` pool, of one thread, and answers its index. Its
// thread returns once the pool's actors are dead, no task is queued there
// and no reply is owed to work there.
// @compilerrules [actor-pool-retire, waitfor-dedicated]
export fn new_dedicated_pool() [] -> Int {
    return start_pool(1, -1, true)
}

// How many waiter records the scheduler has made: as many as the most
// waits ever open at once, since a finished wait's record is reused. For
// tests of the runtime.
// @compilerrules [waitfor-pump]
export fn waiter_record_count() [] -> Int {
    return waiter_records()
}

// How many worker threads have returned from retired pools. For tests of
// the runtime.
// @compilerrules [actor-pool-retire]
export fn retired_worker_count() [] -> Int {
    return retired_workers()
}

fn start_pool(n: Int, sink: Int, dedicated: Bool) [] -> Int => n, !sink, !dedicated {
    let id = new_pool(sink, dedicated)
    // [test-actor] The virtual runtime's one thread serves every pool.
    if is_virtual() {
        return id
    }
    let i = 0
    while i < n {
        start_thread(() -> { serve_pool(copy(id)) })
        i = i + 1
    }
    return id
}

// Adds an actor to the table and answers its index: [body] runs its
// activations on [pool], and its mailbox holds [bound] messages.
// @compilerrules [runtime-sched]
export fn spawn_body(pool: Int, bound: Int, body: Body) [] -> Int => !pool, !bound, !body {
    return new_actor(pool, bound, body)
}

// Sends [msg] to the actor at [addr]: what a send comes down to.
//
// While the mailbox is full, the calling thread parks and tries again after
// the next message is taken out. A send to a dead actor drops the message.
// So does a send to a proxy, which needs [send_or_back]: an erased message
// has no encoding to send over the wire.
// @compilerrules [runtime-sched]
export fn send_dyn(addr: Int, msg: Dyn) [] -> None => !addr, !msg {
    let back = send_or_back(addr, msg)
    if back is Dyn d {
        drop_dyn(d)
    }
}

// [send_dyn], except that a send to a proxy hands [msg] back, for the
// caller to encode and route to the node the actor is on.
//
// Two waits for room are special. On the virtual runtime the one thread
// makes the room itself, running other work until the mailbox has some.
// And `main` sending to a full mailbox of an actor on main's own pool would
// wait forever, since only `main` could drain it: that is reported, and the
// process exits with status 1.
// @compilerrules [net-addr-routable, actor-main-pool, test-actor]
export fn send_or_back(addr: Int, msg: Dyn) [] -> Dyn? => !addr, !msg {
    let r = enqueue(copy(addr), msg, this_parker())
    while r is Full full {
        // [test-actor] One thread: the room is made by running the work
        // there is, here, until the mailbox has some.
        if is_virtual() {
            make_room(copy(addr))
            let {msg: again} = full
            r = enqueue(copy(addr), again, this_parker())
            continue
        }
        // [actor-main-pool] Blocking here would be a guaranteed wedge: the only
        // thread that could drain a main-pool mailbox is main, sending.
        if here_pool() == main_pool() && here_actor() == no_frame() && pool_of_actor(copy(addr)) == main_pool() {
            report("salvo: deadlock: the main pool's actor ${addr} has a full mailbox and the only thread that could drain it is the one sending: the main pool has one worker, `main` itself, and it serves work only inside a `waitfor` — send fewer messages before waiting, raise the handler's `mailbox` capacity, or place the actor on a pool of its own")
            exit_process(1)
        }
        park(this_parker())
        let {msg: back} = full
        r = enqueue(copy(addr), back, this_parker())
    }
    // The loop leaves only on `Sent`, `Dead` or `Remote`; the checker
    // does not narrow past a `while`, so the `Full` case is spelled, and
    // unreachable.
    if r is Full full {
        drop_full(full)
        return None
    }
    if r is Remote remote {
        let {msg: back} = remote
        return back
    }
    return None
}

// Runs one piece of work so a full mailbox can drain, or reports
// that nothing can.
// @compilerrules [test-actor]
fn make_room(addr: Int) [] -> None => !addr {
    let w = virtual_work(here_actor())
    if w is RunActor ra {
        run_actor(ra)
        return
    }
    if w is RunTask rt {
        run_task(rt)
        return
    }
    report("salvo: deadlock: actor ${addr} has a full mailbox and nothing can run to drain it: an actor test runs every pool on one thread")
    exit_process(1)
}

// Marks the actor at [addr] a proxy, standing for an actor on another node:
// sends to it are handed back by [send_or_back], to be routed.
// @compilerrules [net-addr-routable]
export fn mark_proxy(addr: Int) [] -> None => !addr {
    set_proxy(addr)
}

// Mints a token for a continuation of the actor at [addr]: what
// `replyto k(…)` in a handler comes down to. With [gated] (`replyto!`), the
// actor is delivered nothing else until the token is answered.
// @compilerrules [actor-replyto]
export fn mint(addr: Int, gated: Bool) [] -> Token => !addr, !gated {
    return mint_actor(addr, gated)
}

// Mints a token for a task: what `replyto k(…)` aimed at a free `send fn`
// comes down to. The answer queues [body] on [pool], to run once.
// @compilerrules [task-mint]
export fn mint_task_on(pool: Int, body: Body) [] -> Token => !pool, !body {
    return mint_task(pool, body)
}

// Mints a token for the calling frame to wait for, on its current pool:
// the first half of `waitfor`. [await_answer] is the second.
// @compilerrules [actor-waitfor]
export fn waiter() [] -> WaiterMint {
    return mint_waiter(here_pool())
}

// Answers [t] with [value], spending it: what sending to a reply comes
// down to. It never blocks. An answer for an actor that has died is dropped.
// @compilerrules [actor-replyto]
export fn answer(t: Token, value: Dyn) [] -> None => !t, !value {
    deliver(t, value)
}

// Answers [t] with an [Exit] when the actor at [addr] dies, or at once if
// it already has. The token no longer counts as owed.
// @compilerrules [actor-watch]
export fn watch(addr: Int, t: Token) [] -> None => !addr, !t {
    watch_actor(addr, t)
}

// Answers [t] with an [Idle] the moment nothing anywhere can run,
// counting what [pool] is still owed. The token no longer counts as owed.
// @compilerrules [actor-on-idle]
export fn on_idle(pool: Int, t: Token) [] -> None => !pool, !t {
    idle_hook(pool, t)
}

// Records that a host thread which may bring work in at any moment (a
// socket listener, a stream reader) has opened. While one is open, the
// program is neither idle nor stuck. Pair each with [external_end]. On the
// virtual runtime this is refused: the test stops with a report.
// @compilerrules [platform-threadsafe, test-actor]
export fn external_begin() [] -> None {
    // [test-actor] Work arriving from a host thread would arrive off the
    // virtual clock, so an actor test may not open one.
    if is_virtual() {
        report("salvo: an actor test opened a host thread (a platform handler that reads or listens on a thread of its own): the virtual runtime runs everything on one thread, so its work would not be deterministic — use an in-memory fake (`MemTransport`, `MemFs`)")
        exit_process(1)
    }
    external(1)
}

// Enters the virtual runtime, with randomness from [seed]: what an actor
// test runs on.
//
// The scheduler starts afresh. Every actor so far is dead, silently (no
// report, no `Exit`), so an address or token left over is a send to the
// dead; queued work and hooks are dropped; every pool but main's is
// retired; the clock is at zero.
// @compilerrules [test-actor]
export fn enter_virtual(seed: Long) [] -> None => !seed {
    go_virtual(seed)
}

// A uniform double in [0, 1): what `random.DefaultRandom` answers. From a
// generator seeded with OS entropy, or, in an actor test, from the test's
// seed, so a run repeats.
// @compilerrules [std-random-default, test-actor]
export fn random_double() [] -> Double {
    return random_unit()
}

// Whether this is the virtual runtime, as in an actor test.
// @compilerrules [test-actor]
export fn virtual_runtime() [] -> Bool {
    return is_virtual()
}

// Moves the virtual clock forward to [at], in nanoseconds; an [at] in the
// past changes nothing.
// @compilerrules [test-actor]
export fn set_virtual_now(at: Long) [] -> None => !at {
    advance_to(at)
}

// Answers [t] once nothing can run: how the timer service moves virtual
// time to its next deadline.
// @compilerrules [test-actor]
export fn on_clock(t: Token) [] -> None => !t {
    clock_hook(t)
}

// Records that a host thread opened with [external_begin] has closed.
// @compilerrules [platform-threadsafe]
export fn external_end() [] -> None {
    external(-1)
}

// Waits for the answer to the token minted with [wid], and answers it: the
// second half of `waitfor`.
//
// The thread does not idle meanwhile. It runs the work of its own pool,
// everything there but the waiting actor's own activations, nested on its
// stack, and parks only when there is none. When nothing anywhere can ever
// answer, it prints the deadlock report and the process exits with
// status 1.
// @compilerrules [waitfor-pump]
export fn await_answer(wid: Int) [] -> Dyn => !wid {
    let pool = here_pool()
    let own = here_actor()
    let frame = if own == no_frame() { 2 } else { 1 }
    while true {
        let step = wait_step(copy(wid), copy(pool), copy(own), copy(frame), this_parker())
        if step is Got g {
            let {value} = g
            return value
        }
        if step is RunActor ra {
            run_actor(ra)
        } elif step is RunTask rt {
            run_task(rt)
        } elif step is Stuck s {
            report(copy(s.report))
            exit_process(1)
        } elif step is Sleep {
            park(this_parker())
        }
    }
    // Unreachable: the loop returns.
    return unerase(erase(0))
}

fn run_actor(ra: RunActor) [] -> None => !ra {
    flush_frames()
    let {addr, pool, kind, slot, value, body} = ra
    let saved_pool = here_pool()
    let saved_actor = here_actor()
    set_here(pool, copy(addr))
    let ran = activate(body, kind, slot, value)
    set_here(saved_pool, saved_actor)
    let {body: back, fault} = ran
    finish(addr, back, fault)
}

fn run_task(rt: RunTask) [] -> None => !rt {
    let {pool, body, value} = rt
    let saved_pool = here_pool()
    let saved_actor = here_actor()
    set_here(copy(pool), task_frame())
    let ran = activate(body, 1, 0, value)
    set_here(saved_pool, saved_actor)
    let {body: done, fault} = ran
    drop_body(done)
    task_done(pool, fault)
}

// A worker's loop: run what its pool has, sleep when there is nothing.
fn serve_pool(pool: Int) [] -> None => !pool {
    set_here(copy(pool), no_frame())
    while true {
        let w = next_work(copy(pool), this_parker())
        if w is RunActor ra {
            run_actor(ra)
        } elif w is RunTask rt {
            run_task(rt)
        } elif w is Retire {
            return
        } else {
            park(this_parker())
        }
    }
}

// ---- for the host's wire layer
// @compilerrules [net-addr-routable]

// A token for the continuation at [slot] of the actor at [addr], decoded
// from a frame that came over the wire. It is owed on the node that minted
// it, so it does not count as owed here.
export fn token_to_actor(addr: Int, slot: Long) [] -> Token => !addr, !slot {
    return Token { target: ToActor { addr: addr }, slot: slot, tracked: false }
}

// A token for the waiting frame [wid], decoded from a frame that came over
// the wire, as [token_to_actor] is.
export fn token_to_waiter(wid: Int, slot: Long) [] -> Token => !wid, !slot {
    return Token { target: ToWaiter { wid: wid }, slot: slot, tracked: false }
}

// A token taken apart for its wire form.
//
// [kind] is 0 for an actor's continuation, 1 for a waiting frame and 2 for
// a task; [id] is the actor or waiter index, or a task's pool; [slot] is
// the token's slot. A task's [body] comes out with it, and the routing
// layer keeps it until the answer comes back.
// @compilerrules [net-addr-routable]
export linear struct Exported {
    kind: Int,
    id: Int,
    slot: Long,
    body: Body?
}

// Takes [t] apart as an [Exported], for a token about to cross the wire.
export fn export_token(t: Token) [] -> Exported => !t {
    let {target, slot, tracked} = t
    if target is ToTask tt {
        let {pool, body} = tt
        return Exported { kind: 2, id: pool, slot: slot, body: body }
    }
    let kind = 0
    let id = 0
    if target is ToActor to {
        id = copy(to.addr)
    } elif target is ToWaiter tw {
        kind = 1
        id = copy(tw.wid)
    }
    return Exported { kind: kind, id: id, slot: slot, body: None }
}

fn drop_exported(e: Exported) [] -> None => !e {
    let {kind, id, slot, body} = e
    if body is Body b {
        drop_body(b)
    }
}

// Queues a message that came over the wire from node [from] for the actor
// at [addr]. It is not held to the mailbox bound, since the sender already
// had room granted as a credit; one credit goes back to [from] when it is
// taken out. `false` when the actor is unknown or dead, and the message is
// dropped.
// @compilerrules [net-addr-routable, net-remote-backpressure]
export fn deliver_remote(addr: Int, msg: Dyn, from: Long) [] -> Bool => !addr, !msg, !from {
    return enqueue_remote(addr, msg, from)
}

// Kills the actor at [addr] with [reason], with no activation: a proxy
// whose node has left. Its watchers are sent an [Exit].
// @compilerrules [node-exit]
export fn kill_actor(addr: Int, reason: Str) [] -> None => !addr, !reason {
    kill(addr, reason)
}

// How many more messages the mailbox of the actor at [addr] holds before
// a send waits.
// @compilerrules [net-remote-backpressure]
export fn mailbox_room(addr: Int) [] -> Int => addr {
    return room(addr)
}

// How many messages are queued for the actor at [addr]: a local group
// member's load, as a route selector reads it.
// @compilerrules [actor-group]
export fn mailbox_queued(addr: Int) [] -> Int => addr {
    return queued(addr)
}

// The index of the pool the calling thread works for: 0 on `main`'s
// thread.
// @compilerrules [actor-main-pool]
export fn current_pool() [] -> Int {
    return here_pool()
}

// Whether the actor at [addr] has died: how a sender waiting for credits
// knows to stop.
// @compilerrules [actor-watch]
export fn mailbox_dead(addr: Int) [] -> Bool => addr {
    return is_dead(addr)
}

// Fresh random bits for an identity: what makes a wire address hard to
// guess. From the seed on the virtual runtime.
// @compilerrules [net-addr-capability]
export fn identity_bits() [] -> Long {
    return fresh_bits()
}

// The index of the pool the actor at [addr] runs on.
// @compilerrules [net-addr-routable]
export fn actor_pool(addr: Int) [] -> Int => addr {
    return pool_of_actor(addr)
}

// The index of the pool the waiting frame [wid] belongs to.
// @compilerrules [net-addr-routable]
export fn waiter_pool(wid: Int) [] -> Int => wid {
    return pool_of_waiter(wid)
}

// Adds an actor that drops whatever it is sent, and answers its index: the
// table entry a proxy, or the one shared entry for forged addresses,
// stands on.
// @compilerrules [net-addr-routable]
export fn spawn_inert() [] -> Int {
    return spawn_body(0, 0, body_of((kind, slot, value) -> drop_dyn(value)))
}
