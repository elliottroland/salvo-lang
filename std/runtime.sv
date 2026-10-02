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

// [runtime-sched] An erased value: a message or an answer, whatever its
// type. Linear, so a queue that drops one says so.
export linear platform type Dyn
export platform fn erase<T canbe linear>(v: T) [] -> Dyn => !v
export platform fn unerase<T canbe linear>(d: Dyn) [] -> T => !d
// Drops an erased value the program no longer owes anything for: a message
// to an actor that died.
platform fn drop_dyn(d: Dyn) [] -> None => !d

// [runtime-sched] An actor's body: what an activation runs, handed one
// message at a time. Owned by one holder — the table, or the thread running
// it — and moved between them, which is how one actor never runs twice at
// once.
linear platform type Body
// A body from a Salvo fn value (the runtime's privilege: kept, and run on
// other threads [runtime-kept-fn]).
platform fn body_of(f: (msg: Dyn) -> None) [] -> Body
=>[f] !msg => !f
// Runs one activation of [b] on [msg] inside the fault boundary, handing the
// body back with `None`, or with the fault that ended the activation.
platform fn activate(b: Body, msg: Dyn) [] -> Ran => !b, !msg

linear struct Ran {
    body: Body,
    fault: Str?
}

fn drop_ran(r: Ran) [] -> None => !r {
    let {body, fault} = r
    drop_body(body)
}
platform fn drop_body(b: Body) [] -> None => !b

// [runtime-sched] A cell holding at most one linear value, which can be
// taken out and put back through a `Mut` handle: how a body leaves the table
// for an activation and returns after it.
linear platform type Slot<T canbe linear> canbe Mut
platform fn slot_of<T canbe linear>(v: T) [] -> Mut Slot<T> => !v
platform fn slot_take<T canbe linear>(s: Mut Slot<T>) [] -> T? => s: Mut
platform fn slot_put<T canbe linear>(s: Mut Slot<T>, v: T) [] -> None => s: Mut, !v
platform fn drop_slot(s: Slot<Body>) [] -> None => !s

// One actor, as the table keeps it. Its body moves out of the slot for an
// activation and back after it.
linear struct ActorRec canbe Mut {
    body: Mut Slot<Body>,
    pool: Int,
    bound: Int,
    queue: Mut Deque<Dyn>,
    running: Bool,
    dead: Bool,
    // Senders parked on a full mailbox, woken one per dequeue.
    blocked: Mut List<Parker>
}

fn drop_actor_rec(a: ActorRec) [] -> None => !a {
    let {body, pool, bound, queue, running, dead, blocked} = a
    drop_slot(body)
    drain(queue, d -> drop_dyn(d))
}

// What a worker runs: one message for one actor, with its body.
linear struct Job {
    addr: Int,
    msg: Dyn,
    body: Body
}

fn drop_job(j: Job) [] -> None => !j {
    let {addr, msg, body} = j
    drop_dyn(msg)
    drop_body(body)
}

effect SchedTable {
    fn new_pool() -> Int
    fn new_actor(pool: Int, bound: Int, body: Body) -> Int => !pool, !bound, !body
    // Enqueues [msg] for [addr]: `Sent`, `Dead` (the message is dropped), or
    // `Full` — the message comes back and [waiter] is recorded to be woken
    // by the next dequeue.
    fn enqueue(addr: Int, msg: Dyn, waiter: Parker) -> Sent | Dead | Full => !addr, !msg, !waiter
    // The next job on [pool], or `None` having recorded [idle] to be woken
    // by the next enqueue there.
    fn next_job(pool: Int, idle: Parker) -> Job? => !pool, !idle
    // An activation finished: the body goes back, or the actor dies of
    // [fault].
    fn finish(addr: Int, body: Body, fault: Str?) -> None => !addr, !body, !fault
}

struct Sent {}
struct Dead {}
linear struct Full { msg: Dyn }

fn drop_full(f: Full) [] -> None => !f {
    let {msg} = f
    drop_dyn(msg)
}

handler Scheduler() of SchedTable {
    actors: Mut List<Mut ActorRec> = mut_list_of()
    // Per pool, the parkers of its workers with nothing to do.
    idle: Mut List<Mut List<Parker>> = mut_list_of()

    fn new_pool() -> Int {
        add(idle, mut_list_of())
        return size(idle) - 1
    }

    fn new_actor(pool: Int, bound: Int, body: Body) -> Int => !pool, !bound, !body {
        add(actors, Mut ActorRec {
            body: slot_of(body), pool: pool, bound: bound, queue: mut_deque_of(),
            running: false, dead: false, blocked: mut_list_of()
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
        if size(a.queue) >= a.bound {
            add(a.blocked, waiter)
            return Full { msg: msg }
        }
        add_last(a.queue, msg)
        let pool = copy(a.pool)
        wake_all(idle, pool)
        return Sent {}
    }

    fn next_job(pool: Int, idle_parker: Parker) -> Job? => !pool, !idle_parker {
        let i = 0
        while i < size(actors) {
            let a = get(actors, i)!
            if a.pool == pool && !a.running && !a.dead && size(a.queue) > 0 {
                let msg = remove_first(a.queue)!
                a.running = true
                let woken = remove_first(a.blocked)
                if woken is Parker {
                    unpark(woken)
                }
                let body = slot_take(a.body)!
                return Job { addr: copy(i), msg: msg, body: body }
            }
            i = i + 1
        }
        let ps = get(idle, pool)!
        add(ps, idle_parker)
        return None
    }

    fn finish(addr: Int, body: Body, fault: Str?) -> None => !addr, !body, !fault {
        let a = get(actors, addr)!
        a.running = false
        if fault is None {
            slot_put(a.body, body)
            if size(a.queue) > 0 {
                let pool = copy(a.pool)
                wake_all(idle, pool)
            }
            return
        }
        drop_body(body)
        a.dead = true
        while remove_first(a.queue) is Dyn d {
            drop_dyn(d)
        }
    }
}

// Wakes every idle worker of [pool].
fn wake_all(idle: Mut List<Mut List<Parker>>, pool: Int) [] -> None => idle: Mut, pool {
    let ps = get(idle, pool)!
    while remove_first(ps) is Parker p {
        unpark(p)
    }
}

use Scheduler()

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

// A worker's loop: run what its pool has, sleep when there is nothing.
fn serve_pool(pool: Int) [] -> None => !pool {
    while true {
        let job = next_job(copy(pool), this_parker())
        if job is Job j {
            let {addr, msg, body} = j
            let ran = activate(body, msg)
            let {body: back, fault} = ran
            finish(addr, back, fault)
        } else {
            park(this_parker())
        }
    }
}
