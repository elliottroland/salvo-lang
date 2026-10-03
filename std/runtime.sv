// `runtime`: the scheduler every Salvo program runs on, `main` included —
// written once in Salvo over a small platform handler, instead of twice by
// hand in each backend (RUNTIME.md).
//
// This module is **std's own** [mod-std-internal]: the rest of std imports
// it, a program cannot. Its private declarations are also where the language
// makes the exceptions a runtime needs — starting threads, holding locks,
// catching faults — that Salvo code elsewhere cannot (user guidance
// 2026-10-01). It is filled in step by step (RUNTIME.md §11.5); until the
// port, the schedulers are still the backends' `scheduler.rs`/`scheduler.kt`.

// [runtime-parker] One thread's park/unpark token: how a scheduler thread
// with nothing to do sleeps until another thread changes the state it is
// waiting on (RUNTIME.md §11.2 E3, user decision 2026-10-02). Both hosts give
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
platform fn start_thread(body: once () -> None) [] -> None => !body

// [runtime-kept-fn] Runs [body] inside a fault boundary: `None` when it
// returned, or the host's account of the fault that ended it. What an
// activation runs in, so a fault is the actor's death and not the thread's.
platform fn guarded(body: once () -> None) [] -> Str? => !body

// ===== the host [runtime-host] =====
//
// What only the host can do, behind one interface the backends implement in
// std's platform root (RUNTIME.md §11.3): a new backend writes this handler
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

// [addr-capability] The bits an addr's identity carries.
fn fresh_bits() [] -> Long {
    return secure_bits()
}

// [time-timer] The monotonic clock, for the services: the timeline
// `time.tick()` reads.
export fn now_nanos() [] -> Long {
    return mono_nanos()
}

// ===== the scheduler (in progress) [runtime-sched] =====
//
// The scheduler, written once (RUNTIME.md §11.5 step 11). Built beside the
// backends' own while it grows, driven by `std/runtime.test.sv`; the emitters
// switch to it when it covers what theirs does.

// ---- host types

// An erased value: a message or an answer, whatever its type. Linear, so a
// queue that drops one says so.
export linear platform type Dyn
export platform fn erase<T canbe linear>(v: T) [] -> Dyn => !v
export platform fn unerase<T canbe linear>(d: Dyn) [] -> T => !d
// Drops an erased value nobody is owed anything for: a message to the dead.
platform fn drop_dyn(d: Dyn) [] -> None => !d

// An actor's body: what its activations run. Owned by one holder — the
// table, or the thread running it — and moved between them, which is how
// one actor never runs twice at once. [kind] is 0 for a message and 1 for
// an answer to the continuation parked at [slot].
linear platform type RtBody
platform fn body_of(f: (kind: Int, slot: Long, value: Dyn) -> None) [] -> RtBody
=>[f] !value => !f
// Runs one activation of [b] inside the fault boundary, handing the body
// back with `None` or with the fault that ended the activation.
platform fn activate(b: RtBody, kind: Int, slot: Long, value: Dyn) [] -> RtRan
=> !b, !kind, !slot, !value
platform fn drop_body(b: RtBody) [] -> None => !b

// [remote-backpressure] Grants node [from] one credit for actor [addr]: a
// GRANT frame, staged by the routing layer and sent once the scheduler's
// lock is released (`flush_frames`).
platform fn granted(addr: Int, from: Long) [] -> None => addr, from

// Sends the frames the routing layer staged under the scheduler's lock.
platform fn flush_frames() [] -> None

// Ends the process with [code]: the runtime's named reports that cannot be
// recovered from (a deadlock, a wedged main pool).
platform fn exit_process(code: Int) [] -> Never => code

linear struct RtRan {
    body: RtBody,
    fault: Str?
}

fn drop_ran(r: RtRan) [] -> None => !r {
    let {body, fault} = r
    drop_body(body)
}

// A cell holding at most one linear value, taken out and put back through a
// `Mut` handle: how a body leaves the table for an activation and returns,
// and where a waiter's answer waits for it.
linear platform type RtSlot<T canbe linear> canbe Mut
platform fn slot_of<T canbe linear>(v: T) [] -> Mut RtSlot<T> => !v
platform fn slot_empty<T canbe linear>() [] -> Mut RtSlot<T>
platform fn slot_take<T canbe linear>(s: Mut RtSlot<T>) [] -> T? => s: Mut
platform fn slot_put<T canbe linear>(s: Mut RtSlot<T>, v: T) [] -> None => s: Mut, !v
platform fn drop_slot<T canbe linear>(s: RtSlot<T>) [] -> None => !s

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
linear struct RtDelivered { msg: Dyn, from: Long }
linear struct RtAnswered { slot: Long, value: Dyn }
struct RtReported { reason: Str }
type RtEntry = RtDelivered | RtAnswered | RtReported

fn drop_delivered(d: RtDelivered) [] -> None => !d {
    let {msg, from} = d
    drop_dyn(msg)
}

fn drop_answered(a: RtAnswered) [] -> None => !a {
    let {slot, value} = a
    drop_dyn(value)
}

fn drop_entry(e: RtEntry) [] -> None => !e {
    if e is RtDelivered d {
        drop_delivered(d)
    } elif e is RtAnswered a {
        drop_answered(a)
    } else {
        discard(e)
    }
}

// One actor, as the table keeps it.
linear struct RtActorRec canbe Mut {
    body: Mut RtSlot<RtBody>,
    pool: Int,
    bound: Int,
    queue: Mut Deque<RtEntry>,
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
    watchers: Mut List<RtToken>,
    // [actor-on-idle] Tokens aimed at it that nobody has answered.
    owed: Int,
    // On its pool's ready queue.
    ready: Bool
}

fn drop_actor_rec(a: RtActorRec) [] -> None => !a {
    let {body, pool, bound, queue, slots, user_len, gate, running, dead, exit_reason, blocked, watchers, owed, ready} = a
    drop_slot(body)
    drain(queue, e -> drop_entry(e))
    drain(watchers, t -> drop_token(t))
}

// A frame waiting in `await_answer`, and the answer once it arrives.
linear struct RtWaiterRec canbe Mut {
    pool: Int,
    value: Mut RtSlot<Dyn>,
    filled: Bool,
    parker: Parker?,
    // While the frame waits: what kind of frame it is (1 an activation or a
    // task, 2 main's own thread, 0 not waiting), and the actor whose
    // activation it is, or -1.
    waiting: Int,
    waiting_actor: Int
}

fn drop_waiter_rec(w: RtWaiterRec) [] -> None => !w {
    let {pool, value, filled, parker, waiting, waiting_actor} = w
    drop_slot(value)
}

// A task whose answer has arrived: its body and the answer, ready to run.
linear struct RtTaskRun { body: RtBody, value: Dyn }

fn drop_task_run(t: RtTaskRun) [] -> None => !t {
    let {body, value} = t
    drop_body(body)
    drop_dyn(value)
}

linear struct RtPoolRec canbe Mut {
    // The parkers of its threads with nothing to do.
    idle: Mut List<Parker>,
    tasks: Mut Deque<RtTaskRun>,
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
    // Work made runnable here by activations on this pool, whose wakes wait
    // for the activation to finish.
    deferred: Int
}

fn drop_pool_rec(p: RtPoolRec) [] -> None => !p {
    let {idle, tasks, ready, sink, owed, deferred} = p
    drain(tasks, t -> drop_task_run(t))
}

// [actor-replyto] Where an answer goes: an actor's parked continuation, a
// waiting frame, or a task — whose body travels inside the token until the
// answer schedules it [task-mint].
struct RtToActor { addr: Int }
struct RtToWaiter { wid: Int }
linear struct RtToTask { pool: Int, body: RtBody }
type RtTarget = RtToActor | RtToWaiter | RtToTask

fn drop_to_task(t: RtToTask) [] -> None => !t {
    let {pool, body} = t
    drop_body(body)
}

// [actor-replyto] A one-shot answer channel. Linear: answered exactly once.
// [tracked] says whether it still counts as owed [actor-on-idle]: a token the
// scheduler holds itself (a watch, an idle hook) does not.
export linear struct RtToken {
    target: RtTarget,
    slot: Long,
    tracked: Bool
}

// Answers a token nobody will read, so its obligation ends.
fn drop_token(t: RtToken) [] -> None => !t {
    answer(t, erase(0))
}

// What a waiter's mint hands back: the token to give away, and the waiter
// to wait on.
export linear struct RtWaiterMint {
    token: RtToken,
    wid: Int
}

fn drop_waiter_mint(m: RtWaiterMint) [] -> None => !m {
    let {token, wid} = m
    drop_token(token)
}

// What a thread with work to do runs: an actor's activation or a task.
linear struct RtRunActor { addr: Int, pool: Int, kind: Int, slot: Long, value: Dyn, body: RtBody }
linear struct RtRunTask { pool: Int, body: RtBody, value: Dyn }
type RtWork = RtRunActor | RtRunTask

fn drop_run_actor(a: RtRunActor) [] -> None => !a {
    let {addr, pool, kind, slot, value, body} = a
    drop_dyn(value)
    drop_body(body)
}

fn drop_run_task(t: RtRunTask) [] -> None => !t {
    let {pool, body, value} = t
    drop_body(body)
    drop_dyn(value)
}

struct RtSent {}
struct RtDead {}
linear struct RtFull { msg: Dyn }

fn drop_full(f: RtFull) [] -> None => !f {
    let {msg} = f
    drop_dyn(msg)
}

// [actor-on-idle] A registered quiescence hook: the pool it reports on, and
// the token its `Idle` answers.
linear struct RtIdleHook { pool: Int, token: RtToken }

fn drop_idle_hook(h: RtIdleHook) [] -> None => !h {
    let {pool, token} = h
    drop_token(token)
}

// What one step of a wait found: the answer, work to run meanwhile, nothing
// (its parker is recorded: sleep), or the deadlock report.
linear struct RtGot { value: Dyn }
struct RtSleep {}
// Something changed under the lock (a hook fired): look again before
// sleeping, since the change may be this waiter's own answer.
struct RtAgain {}
struct RtStuck { report: Str }
type RtWaitStep = RtGot | RtRunActor | RtRunTask | RtSleep | RtAgain | RtStuck

fn drop_got(g: RtGot) [] -> None => !g {
    let {value} = g
    drop_dyn(value)
}

effect SchedTable {
    fn new_pool(sink: Int) -> Int => !sink
    fn new_actor(pool: Int, bound: Int, body: RtBody) -> Int => !pool, !bound, !body
    // Enqueues a message: `RtSent`, `RtDead` (dropped), or `RtFull` — the message
    // comes back and [waiter] is recorded to be woken by the next dequeue.
    fn enqueue(addr: Int, msg: Dyn, waiter: Parker) -> RtSent | RtDead | RtFull => !addr, !msg, !waiter
    // [addr-routable] A message that came over the wire from [from]: past
    // nothing — its room was granted as a credit — and granted back when
    // dequeued. `false` for an unknown or dead actor (the message dropped).
    fn enqueue_remote(addr: Int, msg: Dyn, from: Long) -> Bool => !addr, !msg, !from
    // [node-exit] The actor dies of [reason] without an activation: a proxy
    // of an actor on a node that left.
    fn kill(addr: Int, reason: Str) -> None => !addr, !reason
    fn mint_actor(addr: Int, gated: Bool) -> RtToken => !addr, !gated
    fn mint_task(pool: Int, body: RtBody) -> RtToken => !pool, !body
    fn mint_waiter(pool: Int) -> RtWaiterMint => !pool
    // Delivers an answer: never blocks (an answer's room is reserved), and a
    // no-op to the dead.
    fn deliver(t: RtToken, value: Dyn) -> None => !t, !value
    fn watch_actor(addr: Int, t: RtToken) -> None => !addr, !t
    fn idle_hook(pool: Int, t: RtToken) -> None => !pool, !t
    // The next work on [pool]; `None` having recorded [idle] to be woken
    // when work arrives there.
    fn next_work(pool: Int, idle: Parker) -> RtWork? => !pool, !idle
    // One step of waiting on [wid] from a frame of kind [frame] (1 an
    // activation or task, 2 main's thread) inside [own], serving [pool].
    fn wait_step(wid: Int, pool: Int, own: Int, frame: Int, me: Parker) -> RtWaitStep
    => !wid, !pool, !own, !frame, !me
    // An activation finished: the body goes back, or the actor dies of
    // [fault].
    fn finish(addr: Int, body: RtBody, fault: Str?) -> None => !addr, !body, !fault
    fn task_done(pool: Int, fault: Str?) -> None => !pool, !fault
    fn pool_of_actor(addr: Int) -> Int => addr
    fn external(delta: Int) -> None => !delta
    // [remote-backpressure] Room left in [addr]'s mailbox, and messages
    // queued there.
    fn room(addr: Int) -> Int => addr
    fn is_dead(addr: Int) -> Bool => addr
    fn queued(addr: Int) -> Int => addr
}

handler Scheduler() of SchedTable {
    actors: Mut List<Mut RtActorRec> = mut_list_of()
    waiters: Mut List<Mut RtWaiterRec> = mut_list_of()
    pools: Mut List<Mut RtPoolRec> = mut_list_of()
    idle_hooks: Mut List<RtIdleHook> = mut_list_of()
    next_slot: Long = 0
    // Frames running — activations and tasks — and, of those, the ones
    // parked in a wait; how deep main's own thread is in a wait; outside
    // sources of work [threadsafe-platform].
    active: Int = 0
    parked_frames: Int = 0
    main_waits: Int = 0
    externals: Int = 0

    // [main-pool] Pool 0 exists from the start and belongs to `main`.
    init {
        add(pools, Mut RtPoolRec { idle: mut_list_of(), tasks: mut_deque_of(), ready: mut_deque_of(), sink: -1, owed: 0, deferred: 0 })
    }

    fn new_pool(sink: Int) -> Int => !sink {
        add(pools, Mut RtPoolRec { idle: mut_list_of(), tasks: mut_deque_of(), ready: mut_deque_of(), sink: sink, owed: 0, deferred: 0 })
        return size(pools) - 1
    }

    fn new_actor(pool: Int, bound: Int, body: RtBody) -> Int => !pool, !bound, !body {
        add(actors, Mut RtActorRec {
            body: slot_of(body), pool: pool, bound: bound, queue: mut_deque_of(), slots: mut_deque_of(),
            user_len: 0, gate: None, running: false, dead: false, exit_reason: "",
            blocked: mut_list_of(), watchers: mut_list_of(), owed: 0, ready: false
        })
        return size(actors) - 1
    }

    fn enqueue(addr: Int, msg: Dyn, waiter: Parker) -> RtSent | RtDead | RtFull => !addr, !msg, !waiter {
        if addr < 0 || addr >= size(actors) {
            drop_dyn(msg)
            return RtDead {}
        }
        let a = get(actors, addr)!
        if a.dead {
            drop_dyn(msg)
            return RtDead {}
        }
        if a.user_len >= a.bound {
            add(a.blocked, waiter)
            return RtFull { msg: msg }
        }
        let e: RtEntry = RtDelivered { msg: msg, from: to_long(-1) }
        add_last(a.queue, e)
        add_last(a.slots, to_long(-1))
        a.user_len = a.user_len + 1
        if mark_ready(a, pools, copy(addr)) {
            let pool = copy(a.pool)
            wake_for(pools, pool)
        }
        return RtSent {}
    }

    fn enqueue_remote(addr: Int, msg: Dyn, from: Long) -> Bool => !addr, !msg, !from {
        if addr < 0 || addr >= size(actors) {
            drop_dyn(msg)
            return false
        }
        let a = get(actors, addr)!
        if a.dead {
            drop_dyn(msg)
            return false
        }
        let e: RtEntry = RtDelivered { msg: msg, from: from }
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
        let a = get(actors, addr)!
        if a.dead {
            return
        }
        a.dead = true
        a.exit_reason = copy(reason)
        while remove_first(a.blocked) is Parker b {
            unpark(b)
        }
        let watchers: Mut List<RtToken> = mut_list_of()
        while remove_first(a.watchers) is RtToken t {
            add(watchers, t)
        }
        while remove_first(watchers) is RtToken t {
            deliver_to(actors, waiters, pools, t, erase(Exit { reason: copy(reason) }))
        }
        drain(watchers, t -> drop_token(t))
    }

    fn mint_actor(addr: Int, gated: Bool) -> RtToken => !addr, !gated {
        next_slot = next_slot + 1
        let slot = copy(next_slot)
        let a = get(actors, addr)!
        if gated {
            a.gate = copy(slot)
        }
        a.owed = a.owed + 1
        return RtToken { target: RtToActor { addr: addr }, slot: slot, tracked: true }
    }

    fn mint_task(pool: Int, body: RtBody) -> RtToken => !pool, !body {
        next_slot = next_slot + 1
        let p = get(pools, pool)!
        p.owed = p.owed + 1
        return RtToken { target: RtToTask { pool: pool, body: body }, slot: copy(next_slot), tracked: true }
    }

    fn mint_waiter(pool: Int) -> RtWaiterMint => !pool {
        let p = get(pools, pool)!
        p.owed = p.owed + 1
        add(waiters, Mut RtWaiterRec {
            pool: pool, value: slot_empty(), filled: false, parker: None, waiting: 0, waiting_actor: -1
        })
        let wid = size(waiters) - 1
        next_slot = next_slot + 1
        let t = RtToken { target: RtToWaiter { wid: copy(wid) }, slot: copy(next_slot), tracked: true }
        return RtWaiterMint { token: t, wid: wid }
    }

    fn deliver(t: RtToken, value: Dyn) -> None => !t, !value {
        deliver_to(actors, waiters, pools, t, value)
    }

    fn watch_actor(addr: Int, t: RtToken) -> None => !addr, !t {
        let watch = untrack(actors, waiters, pools, t)
        let a = get(actors, addr)!
        if a.dead {
            let reason = copy(a.exit_reason)
            deliver_to(actors, waiters, pools, watch, erase(Exit { reason: reason }))
            return
        }
        add(a.watchers, watch)
    }

    fn idle_hook(pool: Int, t: RtToken) -> None => !pool, !t {
        let hook = untrack(actors, waiters, pools, t)
        add(idle_hooks, RtIdleHook { pool: pool, token: hook })
        wake_all_pools(pools)
    }

    fn next_work(pool: Int, idle: Parker) -> RtWork? => !pool, !idle {
        let w = take_work(actors, pools, copy(pool), -1)
        if w is None {
            let p = get(pools, pool)!
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
        active = active + 1
        return w
    }

    fn wait_step(wid: Int, pool: Int, own: Int, frame: Int, me: Parker) -> RtWaitStep
    => !wid, !pool, !own, !frame, !me {
        let w = get(waiters, wid)!
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
            // This thread may have been woken for work, and leaves with its
            // answer instead: the wake is passed on.
            wake_pool(pools, copy(pool))
            return RtGot { value: v }
        }
        let work = take_work(actors, pools, copy(pool), copy(own))
        if work is RtRunActor ra {
            active = active + 1
            return ra
        }
        if work is RtRunTask rt {
            active = active + 1
            return rt
        }
        // [actor-on-idle] Nothing to run is exactly when the hooks fire, and
        // firing one is progress — so it comes before the deadlock report.
        let q = quiet(actors, waiters, pools, externals)
        if !(size(idle_hooks) == 0) && active == 0 && q {
            fire_idle(actors, waiters, pools, idle_hooks)
            return RtAgain {}
        }
        if active == parked_frames && main_waits > 0 && q {
            return RtStuck { report: deadlock_report(actors, waiters, own) }
        }
        let parked = get(waiters, wid)!
        parked.parker = copy(me)
        let p = get(pools, pool)!
        add(p.idle, me)
        return RtSleep {}
    }

    fn finish(addr: Int, body: RtBody, fault: Str?) -> None => !addr, !body, !fault {
        active = active - 1
        let a = get(actors, addr)!
        a.running = false
        if fault is None {
            slot_put(a.body, body)
            let again = mark_ready(a, pools, copy(addr))
            // The finishing thread looks for work again at once and takes
            // one item itself; the rest made runnable during the activation
            // (deferred by `wake_for`), and this actor's own next entry,
            // wake one thread each now.
            let pool = copy(a.pool)
            let p = get(pools, pool)!
            let extra = copy(p.deferred) - 1
            if again {
                extra = extra + 1
            }
            p.deferred = 0
            while extra > 0 {
                wake_pool(pools, copy(pool))
                extra = extra - 1
            }
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
        let watchers: Mut List<RtToken> = mut_list_of()
        while remove_first(a.watchers) is RtToken t {
            add(watchers, t)
        }
        // [pool-fault-sink] The net under supervision: a death nobody was
        // watching still reaches the pool's sink, or the named report.
        if size(watchers) == 0 {
            report_fault(actors, pools, copy(pool), copy(reason))
        }
        while remove_first(watchers) is RtToken t {
            deliver_to(actors, waiters, pools, t, erase(Exit { reason: copy(reason) }))
        }
        drain(watchers, t -> drop_token(t))
        wake_all_pools(pools)
    }

    fn task_done(pool: Int, fault: Str?) -> None => !pool, !fault {
        active = active - 1
        if fault is Str reason {
            report_fault(actors, pools, pool, copy(reason))
        }
    }

    fn pool_of_actor(addr: Int) -> Int => addr {
        return copy(get(actors, addr)!.pool)
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
}

// [actor-on-idle] A token the scheduler takes over (a watch, an idle hook):
// no longer counted as owed by the program, since the scheduler answers it.
fn untrack(actors: Mut List<Mut RtActorRec>, waiters: Mut List<Mut RtWaiterRec>, pools: Mut List<Mut RtPoolRec>, t: RtToken) [] -> RtToken
=> actors: Mut, waiters: Mut, pools: Mut, !t {
    let {target, slot, tracked} = t
    if tracked {
        release(actors, waiters, pools, target)
    }
    return RtToken { target: target, slot: slot, tracked: false }
}

// [actor-on-idle] One owed token fewer, against whatever it was owed by.
fn release(actors: Mut List<Mut RtActorRec>, waiters: Mut List<Mut RtWaiterRec>, pools: Mut List<Mut RtPoolRec>, target: RtTarget) [] -> None
=> actors: Mut, waiters: Mut, pools: Mut, target {
    if target is RtToActor to {
        let a = get(actors, to.addr)!
        if a.owed > 0 {
            a.owed = a.owed - 1
        }
    } elif target is RtToWaiter tw {
        let pool = copy(get(waiters, tw.wid)!.pool)
        let p = get(pools, pool)!
        if p.owed > 0 {
            p.owed = p.owed - 1
        }
    } else {
        let p = get(pools, target.pool)!
        if p.owed > 0 {
            p.owed = p.owed - 1
        }
    }
}

// [actor-replyto] An answer, to wherever its token was aimed.
fn deliver_to(actors: Mut List<Mut RtActorRec>, waiters: Mut List<Mut RtWaiterRec>, pools: Mut List<Mut RtPoolRec>,
    t: RtToken, value: Dyn) [] -> None
    => actors: Mut, waiters: Mut, pools: Mut, !t, !value {
    let {target, slot, tracked} = t
    if tracked {
        release(actors, waiters, pools, target)
    }
    if target is RtToActor to {
        let a = get(actors, to.addr)!
        if a.dead {
            drop_dyn(value)
            return
        }
        add_last(a.slots, copy(slot))
        let e: RtEntry = RtAnswered { slot: slot, value: value }
        add_last(a.queue, e)
        if mark_ready(a, pools, copy(to.addr)) {
            let pool = copy(a.pool)
            wake_for(pools, pool)
        }
    } elif target is RtToWaiter tw {
        let w = get(waiters, tw.wid)!
        slot_put(w.value, value)
        w.filled = true
        if w.parker is Parker p {
            unpark(copy(p))
        }
    } else {
        let {pool, body} = target
        let p = get(pools, pool)!
        add_last(p.tasks, RtTaskRun { body: body, value: value })
        wake_pool(pools, copy(pool))
    }
}

// [pool-fault-sink] An uncaught fault on [pool]: a report to its sink when it
// has a live one — enqueued past the bound, since a fault report must not
// block the faulting thread — or the named report on stderr.
fn report_fault(actors: Mut List<Mut RtActorRec>, pools: Mut List<Mut RtPoolRec>, pool: Int, reason: Str) [] -> None
=> actors: Mut, pools: Mut, !pool, !reason {
    let sink = copy(get(pools, pool)!.sink)
    if sink >= 0 {
        let s = get(actors, sink)!
        if !s.dead {
            let e: RtEntry = RtReported { reason: reason }
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
fn quiet(actors: Mut List<Mut RtActorRec>, waiters: Mut List<Mut RtWaiterRec>, pools: Mut List<Mut RtPoolRec>, externals: Int) [] -> Bool
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
fn fire_idle(actors: Mut List<Mut RtActorRec>, waiters: Mut List<Mut RtWaiterRec>, pools: Mut List<Mut RtPoolRec>, hooks: Mut List<RtIdleHook>) [] -> None
=> actors: Mut, waiters: Mut, pools: Mut, hooks: Mut {
    while remove_first(hooks) is RtIdleHook h {
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
fn deadlock_report(actors: Mut List<Mut RtActorRec>, waiters: Mut List<Mut RtWaiterRec>, own: Int) [] -> Str
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
fn take_work(actors: Mut List<Mut RtActorRec>, pools: Mut List<Mut RtPoolRec>, pool: Int, exclude: Int) [] -> RtWork?
=> actors: Mut, pools: Mut, !pool, !exclude {
    let p = get(pools, pool)!
    let task = remove_first(p.tasks)
    if task is RtTaskRun t {
        let {body, value} = t
        return RtRunTask { pool: pool, body: body, value: value }
    }
    while remove_first(p.ready) is Int i {
        let a = get(actors, i)!
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
fn mark_ready(a: Mut RtActorRec, pools: Mut List<Mut RtPoolRec>, addr: Int) [] -> Bool => a: Mut, pools: Mut, !addr {
    if a.ready || a.running || a.dead {
        return false
    }
    if deliverable(a.slots, a.gate) is None {
        return false
    }
    a.ready = true
    let p = get(pools, copy(a.pool))!
    add_last(p.ready, addr)
    return true
}

// The work an entry taken from actor [a] (at [addr]) makes: a message wakes
// one blocked sender; an answer to the gate opens it; a report is kind 2,
// carrying the fault's reason (the body builds its protocol's message).
fn work_of(a: Mut RtActorRec, addr: Int, e: RtEntry, body: RtBody) [] -> RtWork? => a: Mut, !addr, !e, !body {
    if e is RtDelivered d {
        a.user_len = a.user_len - 1
        let woken = remove_first(a.blocked)
        if woken is Parker b {
            unpark(b)
        }
        let {msg, from} = d
        // [remote-backpressure] A remote sender's message left the queue:
        // that node gets one credit back for this actor.
        if from >= 0 {
            granted(copy(addr), from)
        }
        return RtRunActor { addr: addr, pool: copy(a.pool), kind: 0, slot: 0, value: msg, body: body }
    }
    if e is RtReported r {
        return RtRunActor { addr: addr, pool: copy(a.pool), kind: 2, slot: 0, value: erase(copy(r.reason)), body: body }
    }
    let {slot, value} = e
    let opens = false
    if a.gate is Long g {
        opens = g == slot
    }
    if opens {
        a.gate = None
    }
    return RtRunActor { addr: addr, pool: copy(a.pool), kind: 1, slot: slot, value: value, body: body }
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
fn wake_pool(pools: Mut List<Mut RtPoolRec>, pool: Int) [] -> None => pools: Mut, !pool {
    let p = get(pools, pool)!
    if remove_first(p.idle) is Parker w {
        unpark(w)
    }
}

// Wakes every idle thread of [pool].
fn wake_every(pools: Mut List<Mut RtPoolRec>, pool: Int) [] -> None => pools: Mut, !pool {
    let p = get(pools, pool)!
    while remove_first(p.idle) is Parker w {
        unpark(w)
    }
}

// Wakes [pool]'s idle threads for new work there — unless the calling
// thread is an activation on that pool, which looks for work the moment it
// finishes: the wake waits for that, so the two do not race for one item.
fn wake_for(pools: Mut List<Mut RtPoolRec>, pool: Int) [] -> None => pools: Mut, !pool {
    if here_pool() == pool && here_actor() >= 0 {
        let p = get(pools, pool)!
        p.deferred = p.deferred + 1
        return
    }
    wake_pool(pools, pool)
}

// [waitfor-pump] Wakes every frame parked in a wait, to look at the
// scheduler again.
fn wake_waiters(waiters: Mut List<Mut RtWaiterRec>) [] -> None => waiters: Mut {
    for w in waiters {
        if w.waiting > 0 && w.parker is Parker p {
            unpark(copy(p))
        }
    }
}

// Wakes every idle thread of every pool: something global changed (a death,
// a hook, an outside source), which any of them may have to look at.
fn wake_all_pools(pools: Mut List<Mut RtPoolRec>) [] -> None => pools: Mut {
    let i = 0
    while i < size(pools) {
        wake_every(pools, copy(i))
        i = i + 1
    }
}

use Scheduler()

// ---- the surface

// [runtime-sched] A pool of [n] worker threads; uncaught faults on it go to
// [sink] (an actor), or to the named report when it is -1.
export fn new_pool_of(n: Int, sink: Int) [] -> Int => n, !sink {
    let id = new_pool(sink)
    let i = 0
    while i < n {
        start_thread(() -> { serve_pool(copy(id)) })
        i = i + 1
    }
    return id
}

// [runtime-sched] An actor on [pool] whose mailbox holds [bound] messages.
export fn spawn_body(pool: Int, bound: Int, body: RtBody) [] -> Int => !pool, !bound, !body {
    return new_actor(pool, bound, body)
}

// [runtime-sched] Sends [msg] to [addr], waiting while its mailbox is full;
// a send to the dead is the silent no-op.
export fn send_dyn(addr: Int, msg: Dyn) [] -> None => !addr, !msg {
    let r = enqueue(copy(addr), msg, this_parker())
    while r is RtFull full {
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
    // The loop leaves only on `RtSent` or `RtDead`; the checker does not narrow
    // past a `while`, so the `RtFull` case is spelled, and unreachable.
    if r is RtFull full {
        drop_full(full)
    }
}

// [actor-replyto] A token answering [addr]'s continuation; [gated], the
// actor serves nothing else until it is answered.
export fn mint(addr: Int, gated: Bool) [] -> RtToken => !addr, !gated {
    return mint_actor(addr, gated)
}

// [task-mint] A token whose answer schedules [body] on [pool].
export fn mint_task_on(pool: Int, body: RtBody) [] -> RtToken => !pool, !body {
    return mint_task(pool, body)
}

// [actor-waitfor] A token answering a frame that will wait for it.
export fn waiter() [] -> RtWaiterMint {
    return mint_waiter(here_pool())
}

// [actor-replyto] Answers [t] with [value].
export fn answer(t: RtToken, value: Dyn) [] -> None => !t, !value {
    deliver(t, value)
}

// [actor-watch] Answers [t] with an `Exit` when [addr] dies — at once if it
// already has.
export fn watch(addr: Int, t: RtToken) [] -> None => !addr, !t {
    watch_actor(addr, t)
}

// [actor-on-idle] Answers [t] with an `Idle` the moment nothing anywhere can
// run, reporting what [pool] is still owed.
export fn on_idle(pool: Int, t: RtToken) [] -> None => !pool, !t {
    idle_hook(pool, t)
}

// [threadsafe-platform] A host thread that may send at any moment is open,
// or closed: while one is open the program is neither idle nor stuck.
export fn external_begin() [] -> None {
    external(1)
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
        if step is RtGot g {
            let {value} = g
            return value
        }
        if step is RtRunActor ra {
            run_actor(ra)
        } elif step is RtRunTask rt {
            run_task(rt)
        } elif step is RtStuck s {
            report(copy(s.report))
            exit_process(1)
        } elif step is RtSleep {
            park(this_parker())
        }
    }
    // Unreachable: the loop returns.
    return unerase(erase(0))
}

fn run_actor(ra: RtRunActor) [] -> None => !ra {
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

fn run_task(rt: RtRunTask) [] -> None => !rt {
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
        if w is RtRunActor ra {
            run_actor(ra)
        } elif w is RtRunTask rt {
            run_task(rt)
        } else {
            park(this_parker())
        }
    }
}

// ---- for the host's wire layer [addr-routable]

// A token for actor [addr]'s continuation at [slot], decoded off the wire:
// owed on the node that minted it, so not tracked here.
export fn token_to_actor(addr: Int, slot: Long) [] -> RtToken => !addr, !slot {
    return RtToken { target: RtToActor { addr: addr }, slot: slot, tracked: false }
}

// The same for a waiter.
export fn token_to_waiter(wid: Int, slot: Long) [] -> RtToken => !wid, !slot {
    return RtToken { target: RtToWaiter { wid: wid }, slot: slot, tracked: false }
}

// [addr-routable] What a token is, for its wire form: kind (0 an actor, 1 a
// waiter, 2 a task), the actor or waiter index, and the slot. A task's body
// comes out with it — the routing layer keeps it until the answer returns.
export linear struct RtExported {
    kind: Int,
    id: Int,
    slot: Long,
    body: RtBody?
}

export fn export_token(t: RtToken) [] -> RtExported => !t {
    let {target, slot, tracked} = t
    if target is RtToTask tt {
        let {pool, body} = tt
        return RtExported { kind: 2, id: pool, slot: slot, body: body }
    }
    let kind = 0
    let id = 0
    if target is RtToActor to {
        id = copy(to.addr)
    } elif target is RtToWaiter tw {
        kind = 1
        id = copy(tw.wid)
    }
    return RtExported { kind: kind, id: id, slot: slot, body: None }
}

fn drop_exported(e: RtExported) [] -> None => !e {
    let {kind, id, slot, body} = e
    if body is RtBody b {
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
