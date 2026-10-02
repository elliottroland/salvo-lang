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
linear platform type Body
platform fn body_of(f: (kind: Int, slot: Long, value: Dyn) -> None) [] -> Body
=>[f] !value => !f
// Runs one activation of [b] inside the fault boundary, handing the body
// back with `None` or with the fault that ended the activation.
platform fn activate(b: Body, kind: Int, slot: Long, value: Dyn) [] -> Ran
=> !b, !kind, !slot, !value
platform fn drop_body(b: Body) [] -> None => !b

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

linear struct Delivered { msg: Dyn }
linear struct Answered { slot: Long, value: Dyn }
type Entry = Delivered | Answered

fn drop_delivered(d: Delivered) [] -> None => !d {
    let {msg} = d
    drop_dyn(msg)
}

fn drop_answered(a: Answered) [] -> None => !a {
    let {slot, value} = a
    drop_dyn(value)
}

fn drop_entry(e: Entry) [] -> None => !e {
    if e is Delivered d {
        drop_delivered(d)
    } else {
        drop_answered(e)
    }
}

// One actor, as the table keeps it.
linear struct ActorRec canbe Mut {
    body: Mut Slot<Body>,
    pool: Int,
    bound: Int,
    queue: Mut Deque<Entry>,
    // Beside each entry: the slot it answers, or -1 for a message — what the
    // gate reads without touching the entries themselves.
    slots: Mut Deque<Long>,
    // User messages queued: answers do not count against the bound.
    user_len: Int,
    // While set, only the answer for this slot is delivered.
    gate: Long?,
    running: Bool,
    dead: Bool,
    // Senders parked on a full mailbox, woken one per dequeue.
    blocked: Mut List<Parker>
}

fn drop_actor_rec(a: ActorRec) [] -> None => !a {
    let {body, pool, bound, queue, slots, user_len, gate, running, dead, blocked} = a
    drop_slot(body)
    drain(queue, e -> drop_entry(e))
}

// A frame waiting in `await_answer`, and the answer once it arrives.
linear struct WaiterRec canbe Mut {
    value: Mut Slot<Dyn>,
    parker: Parker?
}

fn drop_waiter_rec(w: WaiterRec) [] -> None => !w {
    let {value, parker} = w
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
    tasks: Mut Deque<TaskRun>
}

fn drop_pool_rec(p: PoolRec) [] -> None => !p {
    let {idle, tasks} = p
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
export linear struct Token {
    target: Target,
    slot: Long
}

// What a waiter's mint hands back: the token to give away, and the waiter
// to wait on.
export linear struct WaiterMint {
    token: Token,
    wid: Int
}

fn drop_waiter_mint(m: WaiterMint) [] -> None => !m {
    let {token, wid} = m
    answer(token, erase(0))
}

// What a thread with work to do runs: an actor's activation or a task.
linear struct RunActor { addr: Int, kind: Int, slot: Long, value: Dyn, body: Body }
linear struct RunTask { pool: Int, body: Body, value: Dyn }
type Work = RunActor | RunTask

fn drop_run_actor(a: RunActor) [] -> None => !a {
    let {addr, kind, slot, value, body} = a
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

fn drop_full(f: Full) [] -> None => !f {
    let {msg} = f
    drop_dyn(msg)
}

// What one step of a wait found: the answer, work to run meanwhile, or
// nothing (its parker is recorded: sleep).
linear struct Got { value: Dyn }

fn drop_got(g: Got) [] -> None => !g {
    let {value} = g
    drop_dyn(value)
}
struct Sleep {}
type WaitStep = Got | RunActor | RunTask | Sleep

effect SchedTable {
    fn new_pool() -> Int
    fn new_actor(pool: Int, bound: Int, body: Body) -> Int => !pool, !bound, !body
    // Enqueues a message: `Sent`, `Dead` (dropped), or `Full` — the message
    // comes back and [waiter] is recorded to be woken by the next dequeue.
    fn enqueue(addr: Int, msg: Dyn, waiter: Parker) -> Sent | Dead | Full => !addr, !msg, !waiter
    fn mint_actor(addr: Int, gated: Bool) -> Token => !addr, !gated
    fn mint_task(pool: Int, body: Body) -> Token => !pool, !body
    fn mint_waiter() -> WaiterMint
    // Delivers an answer: never blocks (an answer's room is reserved), and a
    // no-op to the dead.
    fn deliver(t: Token, value: Dyn) -> None => !t, !value
    // The next work on [pool], skipping [exclude]'s activations; `None`
    // having recorded [idle] to be woken when work arrives there.
    fn next_work(pool: Int, exclude: Int, idle: Parker) -> Work? => !pool, !exclude, !idle
    // One step of waiting on [wid], serving [pool] meanwhile.
    fn wait_step(wid: Int, pool: Int, exclude: Int, me: Parker) -> WaitStep
    => !wid, !pool, !exclude, !me
    // An activation finished: the body goes back, or the actor dies of
    // [fault].
    fn finish(addr: Int, body: Body, fault: Str?) -> None => !addr, !body, !fault
    fn pool_of_actor(addr: Int) -> Int => addr
}

handler Scheduler() of SchedTable {
    actors: Mut List<Mut ActorRec> = mut_list_of()
    waiters: Mut List<Mut WaiterRec> = mut_list_of()
    // [main-pool] Pool 0 exists from the start and belongs to `main`.
    pools: Mut List<Mut PoolRec> = mut_list_of()
    next_slot: Long = 0

    init {
        add(pools, Mut PoolRec { idle: mut_list_of(), tasks: mut_deque_of() })
    }

    fn new_pool() -> Int {
        add(pools, Mut PoolRec { idle: mut_list_of(), tasks: mut_deque_of() })
        return size(pools) - 1
    }

    fn new_actor(pool: Int, bound: Int, body: Body) -> Int => !pool, !bound, !body {
        add(actors, Mut ActorRec {
            body: slot_of(body), pool: pool, bound: bound, queue: mut_deque_of(), slots: mut_deque_of(), user_len: 0,
            gate: None, running: false, dead: false, blocked: mut_list_of()
        })
        return size(actors) - 1
    }

    fn enqueue(addr: Int, msg: Dyn, waiter: Parker) -> Sent | Dead | Full => !addr, !msg, !waiter {
        if addr < 0 || addr >= size(actors) {
            drop_dyn(msg)
            return Dead {}
        }
        let a = get(actors, addr)!
        if a.dead {
            drop_dyn(msg)
            return Dead {}
        }
        if a.user_len >= a.bound {
            add(a.blocked, waiter)
            return Full { msg: msg }
        }
        let e: Entry = Delivered { msg: msg }
        add_last(a.queue, e)
        add_last(a.slots, to_long(-1))
        a.user_len = a.user_len + 1
        let pool = copy(a.pool)
        wake_pool(pools, pool)
        return Sent {}
    }

    fn mint_actor(addr: Int, gated: Bool) -> Token => !addr, !gated {
        next_slot = next_slot + 1
        let slot = copy(next_slot)
        if gated {
            let a = get(actors, addr)!
            a.gate = copy(slot)
        }
        return Token { target: ToActor { addr: addr }, slot: slot }
    }

    fn mint_task(pool: Int, body: Body) -> Token => !pool, !body {
        next_slot = next_slot + 1
        return Token { target: ToTask { pool: pool, body: body }, slot: copy(next_slot) }
    }

    fn mint_waiter() -> WaiterMint {
        add(waiters, Mut WaiterRec { value: slot_empty(), parker: None })
        let wid = size(waiters) - 1
        next_slot = next_slot + 1
        let t = Token { target: ToWaiter { wid: copy(wid) }, slot: copy(next_slot) }
        return WaiterMint { token: t, wid: wid }
    }

    fn deliver(t: Token, value: Dyn) -> None => !t, !value {
        let {target, slot} = t
        deliver_to(actors, waiters, pools, target, slot, value)
    }

    fn next_work(pool: Int, exclude: Int, idle: Parker) -> Work? => !pool, !exclude, !idle {
        let w = take_work(actors, pools, copy(pool), exclude)
        if w is None {
            let p = get(pools, pool)!
            add(p.idle, idle)
            return None
        }
        return w
    }

    fn wait_step(wid: Int, pool: Int, exclude: Int, me: Parker) -> WaitStep
    => !wid, !pool, !exclude, !me {
        let w = get(waiters, wid)!
        let got = slot_take(w.value)
        if got is Dyn v {
            w.parker = None
            return Got { value: v }
        }
        let work = take_work(actors, pools, copy(pool), exclude)
        if work is RunActor ra {
            return ra
        }
        if work is RunTask rt {
            return rt
        }
        w.parker = copy(me)
        let p = get(pools, pool)!
        add(p.idle, me)
        return Sleep {}
    }

    fn finish(addr: Int, body: Body, fault: Str?) -> None => !addr, !body, !fault {
        let a = get(actors, addr)!
        a.running = false
        if fault is None {
            slot_put(a.body, body)
            if size(a.queue) > 0 {
                let pool = copy(a.pool)
                wake_pool(pools, pool)
            }
            return
        }
        drop_body(body)
        a.dead = true
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
    }

    fn pool_of_actor(addr: Int) -> Int => addr {
        return copy(get(actors, addr)!.pool)
    }
}

// [actor-replyto] An answer, to wherever its token was aimed.
fn deliver_to(actors: Mut List<Mut ActorRec>, waiters: Mut List<Mut WaiterRec>, pools: Mut List<Mut PoolRec>,
    target: Target, slot: Long, value: Dyn) [] -> None
    => actors: Mut, waiters: Mut, pools: Mut, !target, !slot, !value {
    if target is ToActor to {
        let a = get(actors, to.addr)!
        if a.dead {
            drop_dyn(value)
            return
        }
        add_last(a.slots, copy(slot))
        let e: Entry = Answered { slot: slot, value: value }
        add_last(a.queue, e)
        let pool = copy(a.pool)
        wake_pool(pools, pool)
    } elif target is ToWaiter tw {
        let w = get(waiters, tw.wid)!
        slot_put(w.value, value)
        if w.parker is Parker p {
            unpark(copy(p))
        }
    } else {
        let {pool, body} = target
        let p = get(pools, pool)!
        add_last(p.tasks, TaskRun { body: body, value: value })
        wake_pool(pools, copy(pool))
    }
}

// [waitfor-pump] The next work on [pool]: a ready task first, then the first
// actor with a deliverable entry — honouring its gate — other than
// [exclude], whose own activation is the one waiting.
fn take_work(actors: Mut List<Mut ActorRec>, pools: Mut List<Mut PoolRec>, pool: Int, exclude: Int) [] -> Work?
=> actors: Mut, pools: Mut, !pool, !exclude {
    let p = get(pools, pool)!
    let task = remove_first(p.tasks)
    if task is TaskRun t {
        let {body, value} = t
        return RunTask { pool: pool, body: body, value: value }
    }
    let i = 0
    while i < size(actors) {
        let a = get(actors, i)!
        if a.pool == pool && i != exclude && !a.running && !a.dead {
            let at = deliverable(a.slots, a.gate)
            if at is Int k {
                let _slot = remove_at(a.slots, copy(k))
                let e = remove_at(a.queue, k)!
                a.running = true
                let body = slot_take(a.body)!
                return work_of(a, copy(i), e, body)
            }
        }
        i = i + 1
    }
    return None
}

// The work an entry taken from actor [a] (at [addr]) makes: a message wakes
// one blocked sender; an answer to the gate opens it.
fn work_of(a: Mut ActorRec, addr: Int, e: Entry, body: Body) [] -> Work? => a: Mut, !addr, !e, !body {
    if e is Delivered d {
        a.user_len = a.user_len - 1
        let woken = remove_first(a.blocked)
        if woken is Parker b {
            unpark(b)
        }
        let {msg} = d
        return RunActor { addr: addr, kind: 0, slot: 0, value: msg, body: body }
    }
    let {slot, value} = e
    let opens = false
    if a.gate is Long g {
        opens = g == slot
    }
    if opens {
        a.gate = None
    }
    return RunActor { addr: addr, kind: 1, slot: slot, value: value, body: body }
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
fn wake_pool(pools: Mut List<Mut PoolRec>, pool: Int) [] -> None => pools: Mut, !pool {
    let p = get(pools, pool)!
    while remove_first(p.idle) is Parker w {
        unpark(w)
    }
}

use Scheduler()

// ---- the surface

// [runtime-sched] A pool of [n] worker threads.
export fn new_pool_of(n: Int) [] -> Int => n {
    let id = new_pool()
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
// a send to the dead is the silent no-op.
export fn send_dyn(addr: Int, msg: Dyn) [] -> None => !addr, !msg {
    let r = enqueue(copy(addr), msg, this_parker())
    while r is Full full {
        park(this_parker())
        let {msg: back} = full
        r = enqueue(copy(addr), back, this_parker())
    }
    // The loop leaves only on `Sent` or `Dead`; the checker does not narrow
    // past a `while`, so the `Full` case is spelled, and unreachable.
    if r is Full full {
        drop_full(full)
    }
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
    return mint_waiter()
}

// [actor-replyto] Answers [t] with [value].
export fn answer(t: Token, value: Dyn) [] -> None => !t, !value {
    deliver(t, value)
}

// [waitfor-pump] Waits for [wid]'s answer, serving this thread's pool
// meanwhile — everything on it but the waiting actor's own activations.
export fn await_answer(wid: Int) [] -> Dyn => !wid {
    let pool = here_pool()
    let own = here_actor()
    while true {
        let step = wait_step(copy(wid), copy(pool), copy(own), this_parker())
        if step is Got g {
            let {value} = g
            return value
        }
        if step is RunActor ra {
            run_actor(ra)
        } elif step is RunTask rt {
            run_task(rt)
        } elif step is Sleep {
            park(this_parker())
        }
    }
    // Unreachable: the loop returns.
    return unerase(erase(0))
}

fn run_actor(ra: RunActor) [] -> None => !ra {
    let {addr, kind, slot, value, body} = ra
    let saved_pool = here_pool()
    let saved_actor = here_actor()
    let pool = actor_pool(copy(addr))
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
    set_here(pool, -1)
    let ran = activate(body, 1, 0, value)
    set_here(saved_pool, saved_actor)
    let {body: done, fault} = ran
    drop_body(done)
}

// The pool [addr] runs on.
fn actor_pool(addr: Int) [] -> Int => addr {
    return pool_of_actor(addr)
}

// A worker's loop: run what its pool has, sleep when there is nothing.
fn serve_pool(pool: Int) [] -> None => !pool {
    set_here(copy(pool), -1)
    while true {
        let w = next_work(copy(pool), -1, this_parker())
        if w is RunActor ra {
            run_actor(ra)
        } elif w is RunTask rt {
            run_task(rt)
        } else {
            park(this_parker())
        }
    }
}
