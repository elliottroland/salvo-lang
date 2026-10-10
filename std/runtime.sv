// `runtime`: the scheduler every Salvo program runs on, `main` included —
// written once in Salvo over a small platform handler, instead of twice by
// hand in each backend (COMPLETED.md, "Design record: the runtime in Salvo").
//
// This module is **std's own** [mod-std-internal]: the rest of std imports
// it, a program cannot. Its private declarations are also where the language
// makes the exceptions a runtime needs — starting threads, holding locks,
// catching faults — that Salvo code elsewhere cannot (user guidance
// 2026-10-01). The services built on it are the modules under
// `std/runtime/` [runtime-layers]; each backend's `scheduler.rs`/
// `scheduler.kt` is the entry points generated code calls, onto this.

// [runtime-parker] One thread's park/unpark token: how a scheduler thread
// with nothing to do sleeps until another thread changes the state it is
// waiting on (runtime E3, user decision 2026-10-02). Both hosts give
// `unpark` a **token**: an `unpark` that arrives before the `park` makes the
// next `park` return at once, so recording a parker in the scheduler's state
// and parking outside its lock loses no wakeup. A park may also return with
// nobody having unparked (both hosts allow it), so a waiter parks in a loop
// that re-checks its condition.
//
// `threadsafe`: the token is unparked from any thread, while only the thread
// it belongs to may park on it.
export threadsafe platform type Parker

// [runtime-parker] The calling thread's parker.
export platform fn this_parker() [] -> Parker

// [runtime-parker] Sleeps until [p] is unparked, or returns at once if it
// already was since the last park. [p] must be the calling thread's own
// parker; parking on another thread's is a runtime bug and traps.
export platform fn park(p: Parker) [] -> None => p

// [runtime-parker] The same, giving up after [nanos] nanoseconds.
export platform fn park_nanos(p: Parker, nanos: Long) [] -> None => p, nanos

// [runtime-parker] Wakes [p]'s thread, or makes its next park return at once.
export platform fn unpark(p: Parker) [] -> None => p

// [runtime-kept-fn] Runs [body] on a new **daemon** thread: the program
// ends when `main` returns, whatever this thread is doing. The host keeps
// the fn value and runs it elsewhere — the runtime's privilege, since a fn
// value is otherwise only lent for a call [platform-fn-value]. Nothing waits
// for the thread to finish; the scheduler stops one by state, not by handle.
export platform fn start_thread(body: once () -> None) [] -> None => !body

// [runtime-kept-fn] Runs [body] inside a fault boundary: `None` when it
// returned, or the host's account of the fault that ended it. What an
// activation runs in, so a fault is the actor's death and not the thread's.
platform fn guarded(body: once () -> None) [] -> Str? => !body

// [test-recover] The same fault boundary for a body that is only lent, and
// answers: its own answer, or the fault's message in its place. Quiet — the
// host prints nothing for the fault — since the caller reports it. What
// `std.test`'s `trapped_by` is.
export platform fn trap_boundary(body: () -> Str?) [] -> Str? => body

// ===== the host [runtime-host] =====
//
// What only the host can do, behind one interface the backends implement in
// std's platform root (the runtime record): a new backend writes this handler
// and the scheduler above it is Salvo. Filled in as the port needs it.
effect RuntimeHost {
    // [addr-capability] A fresh value an outsider cannot guess: OS entropy.
    fn secure_bits() -> Long
    // A line on standard error, for the runtime's named reports.
    fn report(line: Str) -> None => line
    // [time-timer] The monotonic clock, in nanoseconds: the timeline
    // `time.tick()` reads, so a deadline and a reading agree.
    fn mono_nanos() -> Long
}

threadsafe platform handler HostRuntime() of RuntimeHost

// [mod-use] The runtime's host, bound once for the process: every function
// here reaches it without declaring it.
use HostRuntime()

// [addr-capability] The bits an addr's identity carries: from the seed, on
// the virtual runtime [test-actor].
fn fresh_bits() [] -> Long {
    if is_virtual() {
        return random_bits()
    }
    return secure_bits()
}

// [time-timer] The monotonic clock, for the services: the timeline
// `time.tick()` reads. On the virtual runtime, the clock the scheduler moves
// [test-actor].
export fn now_nanos() [] -> Long {
    if is_virtual() {
        return virtual_now()
    }
    return mono_nanos()
}

// ===== the scheduler [runtime-sched] =====
//
// The scheduler, written once, and driven by `std/runtime.test.sv`.

// ---- host types

// An erased value: a message or an answer, whatever its type. Linear, so a
// queue that drops one says so.
export linear platform type Dyn
export platform fn erase<T canbe linear>(v: T) [] -> Dyn => !v
export platform fn unerase<T canbe linear>(d: Dyn) [] -> T => !d
// Drops an erased value nobody is owed anything for: a message to the dead.
export platform fn drop_dyn(d: Dyn) [] -> None => !d

// An actor's body: what its activations run. Owned by one holder — the
// table, or the thread running it — and moved between them, which is how
// one actor never runs twice at once. [kind] is 0 for a message and 1 for
// an answer to the continuation parked at [slot].
export linear platform type Body
export platform fn body_of(f: (kind: Int, slot: Long, value: Dyn) -> None) [] -> Body
=>[f] !value => !f
// Runs one activation of [b] inside the fault boundary, handing the body
// back with `None` or with the fault that ended the activation.
platform fn activate(b: Body, kind: Int, slot: Long, value: Dyn) [] -> Ran
=> !b, !kind, !slot, !value
export platform fn drop_body(b: Body) [] -> None => !b

// [remote-backpressure] Grants node [from] one credit for actor [addr], on
// [pool]: a GRANT frame, staged by the routing service and sent once the
// scheduler's lock is released (`flush_frames`). Called with the lock held,
// so it must not call back into the scheduler.
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

// [main-pool] The pool `main` is the single worker of: it has no thread of
// its own, and runs its work while it waits.
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
// fault report to a pool's sink [pool-fault-sink].
// [remote-backpressure] [from] is the node a message came over the wire
// from (-1 for a local send), which is granted a credit back when it is
// dequeued.
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
        discard(e)
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
    // [actor-watch] Tokens to answer with an `Exit` when it dies.
    watchers: Mut List<Token>,
    // [actor-on-idle] Tokens aimed at it that nobody has answered.
    owed: Int,
    // On its pool's ready queue.
    ready: Bool,
    // [addr-routable] A proxy of an actor on another node: a send to it is
    // handed back, to be encoded and routed.
    proxy: Bool
}

fn drop_actor_rec(a: ActorRec) [] -> None => !a {
    let {body, pool, bound, queue, slots, user_len, gate, running, dead, exit_reason, blocked, watchers, owed, ready, proxy} = a
    drop_slot(body)
    drain(queue, e -> drop_entry(e))
    drain(watchers, t -> drop_token(t))
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
    // [waitfor-pump] The slot of the token minted for this wait: a record is
    // reused once its wait has ended, and an answer carrying another slot
    // (late, or forged from the wire) is not this wait's.
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
    // [pool-fault-sink] The actor its uncaught faults go to, or -1.
    sink: Int,
    // [actor-on-idle] Tokens owed to work here that belongs to no actor.
    owed: Int,
    // [pool-retire] A `Dedicated` pool: made for one spawn, so nothing is
    // placed on it but what its actors place there themselves.
    dedicated: Bool,
    // [pool-retire] Nothing can run here any more: its threads return.
    retired: Bool
}

fn drop_pool_rec(p: PoolRec) [] -> None => !p {
    let {idle, tasks, ready, sink, owed, dedicated, retired} = p
    drain(tasks, t -> drop_task_run(t))
}

// [actor-replyto] Where an answer goes: an actor's parked continuation, a
// waiting frame, or a task — whose body travels inside the token until the
// answer schedules it [task-mint].
struct ToActor { addr: Int }
struct ToWaiter { wid: Int }
linear struct ToTask { pool: Int, body: Body }
type Target = ToActor | ToWaiter | ToTask

fn drop_to_task(t: ToTask) [] -> None => !t {
    let {pool, body} = t
    drop_body(body)
}

// [actor-replyto] A one-shot answer channel. Linear: answered exactly once.
// [tracked] says whether it still counts as owed [actor-on-idle]: a token the
// scheduler holds itself (a watch, an idle hook) does not.
export linear struct Token {
    target: Target,
    slot: Long,
    tracked: Bool
}

// Answers a token nobody will read, so its obligation ends.
fn drop_token(t: Token) [] -> None => !t {
    answer(t, erase(0))
}

// What a waiter's mint hands back: the token to give away, and the waiter
// to wait on.
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
// [pool-retire] What a worker of a retired pool is told: return.
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

// [actor-on-idle] A registered quiescence hook: the pool it reports on, and
// the token its `Idle` answers.
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
    // [addr-routable] A message that came over the wire from [from]: past
    // nothing — its room was granted as a credit — and granted back when
    // dequeued. `false` for an unknown or dead actor (the message dropped).
    fn enqueue_remote(addr: Int, msg: Dyn, from: Long) -> Bool => !addr, !msg, !from
    // [node-exit] The actor dies of [reason] without an activation: a proxy
    // of an actor on a node that left.
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
    // [pool-retire] Worker threads that have returned from retired pools.
    fn retired_workers() -> Int
    // [waitfor-pump] Waiter records in the table, free ones included.
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
    // [remote-backpressure] Room left in [addr]'s mailbox, and messages
    // queued there.
    fn room(addr: Int) -> Int => addr
    fn is_dead(addr: Int) -> Bool => addr
    fn queued(addr: Int) -> Int => addr
    // [test-actor] The virtual runtime: enters it with randomness from
    // [seed], leaving a fresh scheduler (every actor so far dead, silently).
    fn go_virtual(seed: Long) -> None => !seed
    fn is_virtual() -> Bool
    fn virtual_now() -> Long
    // Moves the virtual clock forward to [at] (never back).
    fn advance_to(at: Long) -> None => !at
    fn random_bits() -> Long
    // [random-default] A uniform double in [0, 1): from the seed on the
    // virtual runtime, from a generator seeded by OS entropy otherwise.
    fn random_unit() -> Double
    // A token answered when nothing can run: what moves virtual time.
    fn clock_hook(t: Token) -> None => !t
    // Work from any pool but [own]'s activations, for a send waiting for room.
    fn virtual_work(own: Int) -> RunActor | RunTask | None => !own
}

handler Scheduler() of SchedTable {
    actors: Mut List<Mut ActorRec> = mut_list_of()
    waiters: Mut List<Mut WaiterRec> = mut_list_of()
    // [waitfor-pump] Waiter records whose wait has ended, to be reused: the
    // table stays as large as the most waits ever open at once.
    free_waiters: Mut List<Int> = mut_list_of()
    pools: Mut List<Mut PoolRec> = mut_list_of()
    idle_hooks: Mut List<IdleHook> = mut_list_of()
    next_slot: Long = 0
    // Frames running — activations and tasks — and, of those, the ones
    // parked in a wait; how deep main's own thread is in a wait; outside
    // sources of work [threadsafe-platform].
    active: Int = 0
    parked_frames: Int = 0
    main_waits: Int = 0
    externals: Int = 0
    // [pool-retire] Worker threads that have returned.
    retired: Int = 0
    // [test-actor] The virtual runtime: no worker threads, a clock moved by
    // the scheduler, randomness from a seed, and the hooks that move time.
    virtual_mode: Bool = false
    vnow: Long = 0
    rng: Long = 1
    // [random-default] Whether `rng` has been seeded outside the virtual
    // runtime, which seeds it itself.
    rng_seeded: Bool = false
    clock_hooks: Mut List<Token> = mut_list_of()

    // [main-pool] Pool 0 exists from the start and belongs to `main`.
    init {
        add(pools, Mut PoolRec { idle: mut_list_of(), tasks: mut_deque_of(), ready: mut_deque_of(), sink: -1, owed: 0, dedicated: false, retired: false })
    }

    fn new_pool(sink: Int, dedicated: Bool) -> Int => !sink, !dedicated {
        add(pools, Mut PoolRec { idle: mut_list_of(), tasks: mut_deque_of(), ready: mut_deque_of(), sink: sink, owed: 0, dedicated: dedicated, retired: false })
        return size(pools) - 1
    }

    fn new_actor(pool: Int, bound: Int, body: Body) -> Int => !pool, !bound, !body {
        add(actors, Mut ActorRec {
            body: slot_of(body), pool: pool, bound: bound, queue: mut_deque_of(), slots: mut_deque_of(),
            user_len: 0, gate: None, running: false, dead: false, exit_reason: "",
            blocked: mut_list_of(), watchers: mut_list_of(), owed: 0, ready: false, proxy: false
        })
        return size(actors) - 1
    }

    fn enqueue(addr: Int, msg: Dyn, waiter: Parker) -> Sent | Dead | Full | Remote => !addr, !msg, !waiter {
        if addr < 0 || addr >= size(actors) {
            drop_dyn(msg)
            return Dead {}
        }
        let a = at(actors, addr)!
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
        let a = at(actors, addr)!
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
        let a = at(actors, addr)!
        if a.dead {
            return
        }
        a.dead = true
        a.exit_reason = copy(reason)
        while remove_first(a.blocked) is Parker b {
            unpark(b)
        }
        let watchers: Mut List<Token> = mut_list_of()
        while remove_first(a.watchers) is Token t {
            add(watchers, t)
        }
        while remove_first(watchers) is Token t {
            deliver_to(actors, waiters, pools, t, erase(Exit { reason: copy(reason) }))
        }
        drain(watchers, t -> drop_token(t))
    }

    fn mint_actor(addr: Int, gated: Bool) -> Token => !addr, !gated {
        next_slot = next_slot + 1
        let slot = copy(next_slot)
        let a = at(actors, addr)!
        if gated {
            a.gate = copy(slot)
        }
        a.owed = a.owed + 1
        return Token { target: ToActor { addr: addr }, slot: slot, tracked: true }
    }

    fn mint_task(pool: Int, body: Body) -> Token => !pool, !body {
        next_slot = next_slot + 1
        let p = at(pools, pool)!
        p.owed = p.owed + 1
        return Token { target: ToTask { pool: pool, body: body }, slot: copy(next_slot), tracked: true }
    }

    fn mint_waiter(pool: Int) -> WaiterMint => !pool {
        let p = at(pools, pool)!
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
        let a = at(actors, addr)!
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
        if get(pools, pool)!.retired {
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
        let p = at(pools, pool)!
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
        let w = at(waiters, wid)!
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
        let parked = at(waiters, wid)!
        parked.parker = copy(me)
        let p = at(pools, pool)!
        add(p.idle, me)
        return Sleep {}
    }

    fn finish(addr: Int, body: Body, fault: Str?) -> None => !addr, !body, !fault {
        active = active - 1
        let a = at(actors, addr)!
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
            drop_entry(remove_first(a.queue)!)
        }
        while size(a.slots) > 0 {
            let _s = remove_first(a.slots)
        }
        while remove_first(a.blocked) is Parker b {
            unpark(b)
        }
        let pool = copy(a.pool)
        let watchers: Mut List<Token> = mut_list_of()
        while remove_first(a.watchers) is Token t {
            add(watchers, t)
        }
        // [pool-fault-sink] The net under supervision: a death nobody was
        // watching still reaches the pool's sink, or the named report.
        if size(watchers) == 0 {
            report_fault(actors, pools, copy(pool), copy(reason))
        }
        while remove_first(watchers) is Token t {
            deliver_to(actors, waiters, pools, t, erase(Exit { reason: copy(reason) }))
        }
        drain(watchers, t -> drop_token(t))
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
        return copy(get(actors, addr)!.pool)
    }

    fn set_proxy(addr: Int) -> None => !addr {
        let a = at(actors, addr)!
        a.proxy = true
    }

    fn pool_of_waiter(wid: Int) -> Int => wid {
        return copy(get(waiters, wid)!.pool)
    }

    fn room(addr: Int) -> Int => addr {
        let a = get(actors, addr)!
        return a.bound - a.user_len
    }

    fn is_dead(addr: Int) -> Bool => addr {
        return copy(get(actors, addr)!.dead)
    }

    fn queued(addr: Int) -> Int => addr {
        return copy(get(actors, addr)!.user_len)
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

// [test-actor] The next value of a Lehmer generator: in 1..2^31-2, and no
// product past 2^47, so nothing overflows.
fn lehmer(x: Long) [] -> Long => !x {
    return x * 48271 % 2147483647
}

// [test-actor] A generator state from a seed: never 0, which is the one
// state a Lehmer generator cannot leave.
fn seed_of(seed: Long) [] -> Long => !seed {
    let s = seed % 2147483646
    if s < 0 {
        s = 0 - s
    }
    return s + 1
}

// [test-actor] A fresh scheduler, keeping the table's indices: every actor is
// dead, without a report or an `Exit`, so an addr or a token left from before
// is a send to the dead; every pool's queued work is dropped, and every pool
// but main's retired.
fn reset_all(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, idle_hooks: Mut List<IdleHook>, clock_hooks: Mut List<Token>) [] -> None
=> actors: Mut, pools: Mut, idle_hooks: Mut, clock_hooks: Mut {
    let k = 0
    while k < size(actors) {
        let a = at(actors, k)!
        k = k + 1
        a.dead = true
        a.running = false
        a.ready = false
        a.gate = None
        a.user_len = 0
        a.owed = 0
        while size(a.queue) > 0 {
            drop_entry(remove_first(a.queue)!)
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
        let p = at(pools, i)!
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

// [test-actor] Answers the hook that moves virtual time.
fn fire_clock(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>, hooks: Mut List<Token>) [] -> None
=> actors: Mut, waiters: Mut, pools: Mut, hooks: Mut {
    if remove_first(hooks) is Token t {
        deliver_to(actors, waiters, pools, t, erase(0))
    }
}

// The next work for a thread serving [pool] — or, on the virtual runtime
// ([any]), every pool in order [test-actor].
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

// [actor-on-idle] A token the scheduler takes over (a watch, an idle hook):
// no longer counted as owed by the program, since the scheduler answers it.
fn untrack(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>, t: Token) [] -> Token
=> actors: Mut, waiters: Mut, pools: Mut, !t {
    let {target, slot, tracked} = t
    if tracked {
        release(actors, waiters, pools, target)
    }
    return Token { target: target, slot: slot, tracked: false }
}

// [actor-on-idle] One owed token fewer, against whatever it was owed by.
fn release(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>, target: Target) [] -> None
=> actors: Mut, waiters: Mut, pools: Mut, target {
    if target is ToActor to {
        let a = at(actors, to.addr)!
        if a.owed > 0 {
            a.owed = a.owed - 1
        }
    } elif target is ToWaiter tw {
        let pool = copy(get(waiters, tw.wid)!.pool)
        let p = at(pools, pool)!
        if p.owed > 0 {
            p.owed = p.owed - 1
        }
    } else {
        let p = at(pools, target.pool)!
        if p.owed > 0 {
            p.owed = p.owed - 1
        }
    }
}

// [actor-replyto] An answer, to wherever its token was aimed.
// [waitfor-pump] A waiter record for a new wait of slot [slot] on [pool]: a
// freed one when there is one, else a new one. Answers its index.
fn reuse_waiter(waiters: Mut List<Mut WaiterRec>, free: Mut List<Int>, pool: Int, slot: Long) [] -> Int
=> waiters: Mut, free: Mut, !pool, !slot {
    if remove_first(free) is Int wid {
        let w = at(waiters, wid)!
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

fn deliver_to(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>,
    t: Token, value: Dyn) [] -> None
    => actors: Mut, waiters: Mut, pools: Mut, !t, !value {
    let {target, slot, tracked} = t
    if tracked {
        release(actors, waiters, pools, target)
    }
    if target is ToActor to {
        let a = at(actors, to.addr)!
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
        let w = at(waiters, tw.wid)!
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
        let p = at(pools, pool)!
        add_last(p.tasks, TaskRun { body: body, value: value })
        wake_pool(pools, copy(pool))
    }
}

// [pool-fault-sink] An uncaught fault on [pool]: a report to its sink when it
// has a live one — enqueued past the bound, since a fault report must not
// block the faulting thread — or the named report on stderr.
fn report_fault(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, pool: Int, reason: Str) [] -> None
=> actors: Mut, pools: Mut, !pool, !reason {
    let sink = copy(get(pools, pool)!.sink)
    if sink >= 0 {
        let s = at(actors, sink)!
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

// [actor-on-idle] True when no work is queued anywhere: no actor has a
// deliverable entry, no pool a ready task, no outside source is open, and no
// waiting frame has an answer sitting in its slot (its pickup is the waiting
// thread itself, one instant away).
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

// [actor-on-idle] Fires every registered hook — quiescence is a property of
// the whole scheduler, so they all see it together — each with what its pool
// is still owed.
fn fire_idle(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>, hooks: Mut List<IdleHook>) [] -> None
=> actors: Mut, waiters: Mut, pools: Mut, hooks: Mut {
    while remove_first(hooks) is IdleHook h {
        let {pool, token} = h
        let gates = 0
        let tokens = copy(get(pools, pool)!.owed)
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

// [waitfor-pump] The deadlock report: nothing can run while [own] (or main)
// waits, naming the two ways a mailbox comes to be unservable — an actor
// parked in a wait, and a gated actor.
fn deadlock_report(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, own: Int) [] -> Str
=> actors, waiters, own {
    let occupied: Mut List<Str> = mut_list_of()
    for w in waiters {
        if w.waiting > 0 && w.waiting_actor >= 0 {
            add(occupied, "actor ${w.waiting_actor}")
        }
    }
    let gated: Mut List<Str> = mut_list_of()
    let i = 0
    for a in actors {
        if !(a.gate is None) && !a.dead {
            add(gated, "actor ${i}")
        }
        i = i + 1
    }
    let who = if own >= 0 { "actor ${own}" } else { "main" }
    let clauses: Mut List<Str> = mut_list_of()
    if size(occupied) > 0 {
        add(clauses, "parked in a wait: ${join(occupied, ", ")}")
    }
    if size(gated) > 0 {
        add(clauses, "parked gates: ${join(gated, ", ")}")
    }
    let detail = if size(clauses) == 0 { "" } else { " (${join(clauses, "; ")})" }
    return "salvo: deadlock: nothing can run while ${who} waits${detail}"
}

// [waitfor-pump] The next work on [pool]: a ready task first, then the first
// actor with a deliverable entry — honouring its gate — other than
// [exclude], whose own activation is the one waiting.
fn take_work(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, pool: Int, exclude: Int) [] -> Work?
=> actors: Mut, pools: Mut, !pool, !exclude {
    let p = at(pools, pool)!
    let task = remove_first(p.tasks)
    if task is TaskRun t {
        let {body, value} = t
        return RunTask { pool: pool, body: body, value: value }
    }
    while remove_first(p.ready) is Int i {
        let a = at(actors, i)!
        a.ready = false
        if i != exclude && !a.running && !a.dead {
            let at = deliverable(a.slots, a.gate)
            if at is Int k {
                let _slot = remove_at(a.slots, copy(k))
                let e = remove_at(a.queue, k)!
                a.running = true
                let body = slot_take(a.body)!
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
    let p = at(pools, copy(a.pool))!
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
        // [remote-backpressure] A remote sender's message left the queue:
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
        if get(slots, i)! == gate {
            return copy(i)
        }
        i = i + 1
    }
    return None
}

// Wakes every idle thread of [pool].
// Wakes one idle thread of [pool]: one new piece of work needs one worker,
// and waking every idle thread only has the rest find nothing and park
// again. A thread that finds nothing records itself idle under the lock, and
// a woken waiting frame that leaves with its answer instead passes the wake
// on, so work is never left with every thread asleep.
fn wake_pool(pools: Mut List<Mut PoolRec>, pool: Int) [] -> None => pools: Mut, !pool {
    let p = at(pools, pool)!
    if remove_first(p.idle) is Parker w {
        unpark(w)
    }
}

// Wakes every idle thread of [pool].
fn wake_every(pools: Mut List<Mut PoolRec>, pool: Int) [] -> None => pools: Mut, !pool {
    let p = at(pools, pool)!
    while remove_first(p.idle) is Parker w {
        unpark(w)
    }
}

// [waitfor-pump] Wakes every frame parked in a wait, to look at the
// scheduler again.
fn wake_waiters(waiters: Mut List<Mut WaiterRec>) [] -> None => waiters: Mut {
    for w in waiters {
        if w.waiting > 0 && w.parker is Parker p {
            unpark(copy(p))
        }
    }
}

// [pool-retire] Retires [pool] when nothing can run on it any more: a
// `Dedicated` pool — nobody holds it but the spawn that consumed it — whose
// actors are all dead, with no task queued and no token owed to work there.
// Its threads are woken, see the flag and return.
fn retire_if_done(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, pool: Int) [] -> None
=> actors, pools: Mut, !pool {
    let p = at(pools, pool)!
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

// [runtime-handles] An addr and a pool **are** indices into this runtime's
// tables; these four are the only way between the two, and they are std's
// alone (a program cannot import `runtime`), so a program still cannot forge
// a handle. What lets `core.actor` and `net` be Salvo over this module
// (ROADMAP 0.2).
export intrinsic fn addr_index<E>(a: Addr<E>) [] -> Int => a
export intrinsic fn addr_of<E>(index: Int) [] -> Addr<E> => index
export intrinsic fn pool_index(p: Pool) [] -> Int => p
export intrinsic fn pool_of(index: Int) [] -> Pool => index
// [runtime-handles] The core's token inside a reply token, taken out of it:
// what `core.actor`'s `watch` and `on_idle` register. `None` for a token
// minted on another node, which has no local half.
export intrinsic fn reply_token<T canbe linear>(r: Reply<T>) [] -> Token? => !r

// [runtime-sched] A pool of [n] worker threads; uncaught faults on it go to
// [sink] (an actor), or to the named report when it is -1.
export fn new_pool_of(n: Int, sink: Int) [] -> Int => n, !sink {
    return start_pool(n, sink, false)
}

// [pool-retire] [waitfor-dedicated] A `Dedicated` pool: one thread, which returns once its
// actors are dead and nothing is left to run there.
export fn new_dedicated_pool() [] -> Int {
    return start_pool(1, -1, true)
}

// [waitfor-pump] Waiter records in the scheduler's table: as many as the
// most waits ever open at once.
export fn waiter_record_count() [] -> Int {
    return waiter_records()
}

// [pool-retire] Worker threads that have returned from retired pools.
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

// [runtime-sched] An actor on [pool] whose mailbox holds [bound] messages.
export fn spawn_body(pool: Int, bound: Int, body: Body) [] -> Int => !pool, !bound, !body {
    return new_actor(pool, bound, body)
}

// [runtime-sched] Sends [msg] to [addr], waiting while its mailbox is full;
// a send to the dead is the silent no-op, and so is an untyped send to a
// proxy, which has no encoding to route.
export fn send_dyn(addr: Int, msg: Dyn) [] -> None => !addr, !msg {
    let back = send_or_back(addr, msg)
    if back is Dyn d {
        drop_dyn(d)
    }
}

// [addr-routable] The same, but a send to a proxy hands [msg] back, for the
// caller to encode and route.
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
        // [main-pool] Blocking here would be a guaranteed wedge: the only
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

// [test-actor] Runs one piece of work so a full mailbox can drain, or reports
// that nothing can.
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

// [addr-routable] Marks [addr] a proxy: sends to it are handed back.
export fn mark_proxy(addr: Int) [] -> None => !addr {
    set_proxy(addr)
}

// [actor-replyto] A token answering [addr]'s continuation; [gated], the
// actor serves nothing else until it is answered.
export fn mint(addr: Int, gated: Bool) [] -> Token => !addr, !gated {
    return mint_actor(addr, gated)
}

// [task-mint] A token whose answer schedules [body] on [pool].
export fn mint_task_on(pool: Int, body: Body) [] -> Token => !pool, !body {
    return mint_task(pool, body)
}

// [actor-waitfor] A token answering a frame that will wait for it.
export fn waiter() [] -> WaiterMint {
    return mint_waiter(here_pool())
}

// [actor-replyto] Answers [t] with [value].
export fn answer(t: Token, value: Dyn) [] -> None => !t, !value {
    deliver(t, value)
}

// [actor-watch] Answers [t] with an `Exit` when [addr] dies — at once if it
// already has.
export fn watch(addr: Int, t: Token) [] -> None => !addr, !t {
    watch_actor(addr, t)
}

// [actor-on-idle] Answers [t] with an `Idle` the moment nothing anywhere can
// run, reporting what [pool] is still owed.
export fn on_idle(pool: Int, t: Token) [] -> None => !pool, !t {
    idle_hook(pool, t)
}

// [threadsafe-platform] A host thread that may send at any moment is open,
// or closed: while one is open the program is neither idle nor stuck.
export fn external_begin() [] -> None {
    // [test-actor] Work arriving from a host thread would arrive off the
    // virtual clock, so an actor test may not open one.
    if is_virtual() {
        report("salvo: an actor test opened a host thread (a platform handler that reads or listens on a thread of its own): the virtual runtime runs everything on one thread, so its work would not be deterministic — use an in-memory fake (`MemTransport`, `MemFs`)")
        exit_process(1)
    }
    external(1)
}

// [test-actor] Enters the virtual runtime, as a fresh scheduler: what an
// actor test runs on.
export fn enter_virtual(seed: Long) [] -> None => !seed {
    go_virtual(seed)
}

// [random-default] A uniform double in [0, 1): what `random.DefaultRandom`
// answers. Repeatable from the seed in an actor test [test-actor].
export fn random_double() [] -> Double {
    return random_unit()
}

// [test-actor] Whether this is the virtual runtime.
export fn virtual_runtime() [] -> Bool {
    return is_virtual()
}

// [test-actor] Moves the virtual clock forward to [at].
export fn set_virtual_now(at: Long) [] -> None => !at {
    advance_to(at)
}

// [test-actor] Answers [t] once nothing can run: how the timer service moves
// virtual time.
export fn on_clock(t: Token) [] -> None => !t {
    clock_hook(t)
}

export fn external_end() [] -> None {
    external(-1)
}

// [waitfor-pump] Waits for [wid]'s answer, serving this thread's pool
// meanwhile — everything on it but the waiting actor's own activations — and
// reporting the deadlock when nothing anywhere can ever answer it.
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

// ---- for the host's wire layer [addr-routable]

// A token for actor [addr]'s continuation at [slot], decoded off the wire:
// owed on the node that minted it, so not tracked here.
export fn token_to_actor(addr: Int, slot: Long) [] -> Token => !addr, !slot {
    return Token { target: ToActor { addr: addr }, slot: slot, tracked: false }
}

// The same for a waiter.
export fn token_to_waiter(wid: Int, slot: Long) [] -> Token => !wid, !slot {
    return Token { target: ToWaiter { wid: wid }, slot: slot, tracked: false }
}

// [addr-routable] What a token is, for its wire form: kind (0 an actor, 1 a
// waiter, 2 a task), the actor or waiter index, and the slot. A task's body
// comes out with it — the routing layer keeps it until the answer returns.
export linear struct Exported {
    kind: Int,
    id: Int,
    slot: Long,
    body: Body?
}

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

// [addr-routable] A message that came over the wire, into [addr]'s mailbox.
export fn deliver_remote(addr: Int, msg: Dyn, from: Long) [] -> Bool => !addr, !msg, !from {
    return enqueue_remote(addr, msg, from)
}

// [node-exit] A proxy whose node left dies, firing its watches.
export fn kill_actor(addr: Int, reason: Str) [] -> None => !addr, !reason {
    kill(addr, reason)
}

// [remote-backpressure] Room left in [addr]'s mailbox.
export fn mailbox_room(addr: Int) [] -> Int => addr {
    return room(addr)
}

// [actor-group] Messages queued for [addr] — a local member's load.
export fn mailbox_queued(addr: Int) [] -> Int => addr {
    return queued(addr)
}

// [main-pool] The pool the calling thread works for.
export fn current_pool() [] -> Int {
    return here_pool()
}

// [actor-watch] Whether [addr] has died: a sender waiting on credits stops.
export fn mailbox_dead(addr: Int) [] -> Bool => addr {
    return is_dead(addr)
}

// [addr-capability] Fresh capability bits, for an identity the host mints.
export fn identity_bits() [] -> Long {
    return fresh_bits()
}

// [addr-routable] The pool actor [addr] runs on.
export fn actor_pool(addr: Int) [] -> Int => addr {
    return pool_of_actor(addr)
}

// [addr-routable] The pool waiting frame [wid] waits on.
export fn waiter_pool(wid: Int) [] -> Int => wid {
    return pool_of_waiter(wid)
}

// [addr-routable] An actor that does nothing with what it is sent: the
// table entry a proxy, or the shared dead entry, stands on.
export fn spawn_inert() [] -> Int {
    return spawn_body(0, 0, body_of((kind, slot, value) -> drop_dyn(value)))
}
