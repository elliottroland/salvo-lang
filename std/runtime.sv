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

// [time-timer] What a deadline answers. `time` imports this module for
// `after_nanos` in turn; the two refer to each other.
import time.Fired
import time.Tick

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
threadsafe platform type Parker

// [runtime-parker] The calling thread's parker.
platform fn this_parker() [] -> Parker

// [runtime-parker] Sleeps until [p] is unparked, or returns at once if it
// already was since the last park. [p] must be the calling thread's own
// parker; parking on another thread's is a runtime bug and traps.
platform fn park(p: Parker) [] -> None => p

// [runtime-parker] The same, giving up after [nanos] nanoseconds.
platform fn park_nanos(p: Parker, nanos: Long) [] -> None => p, nanos

// [runtime-parker] Wakes [p]'s thread, or makes its next park return at once.
platform fn unpark(p: Parker) [] -> None => p

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

// ===== deadlines [time-timer] =====
//
// One thread serves every deadline in the program, and only while there is
// one: the **wheel**, an actor on a dedicated thread whose one activation
// fires what is due and parks until the earliest deadline left — and returns
// when none is left, so a program with no pending deadline has no running
// frame (it can be idle, and a wait can be reported stuck). While a deadline
// is pending the wheel's frame is running, which is the rule the scheduler
// has always had: a pending deadline is work on its way.
//
// Registrations go into [Deadlines], a monitor, not into the wheel's mailbox:
// the mailbox holds at most the one `run` that starts the wheel, so a burst of
// registrations never fills it. Waking is by the wheel's [Parker]: an unpark
// that lands before its park makes the park return at once, so no wakeup is
// lost between "what is earliest" and going to sleep.

effect DeadlineTable {
    // Adds a deadline; answers whether the wheel must be started (it was
    // stopped).
    fn register(at: Long, done: Reply<Fired>) -> Bool => !at, !done
    // Records the wheel's parker, as the wheel starts.
    fn wheel_parker(p: Parker) -> None => !p
    // The parker to wake after a registration, while the wheel runs.
    fn waker() -> Parker?
    // Takes every deadline due by [now].
    fn take_due(now: Long) -> Mut List<Reply<Fired>> => now
    // The earliest deadline left, or `None` having marked the wheel stopped.
    fn next_deadline() -> Long?
}

handler Deadlines() of DeadlineTable {
    ats: Mut List<Long> = mut_list_of()
    dones: Mut List<Reply<Fired>> = mut_list_of()
    running: Bool = false
    parker: Parker? = None

    fn register(at: Long, done: Reply<Fired>) -> Bool => !at, !done {
        add(ats, at)
        add(dones, done)
        if running {
            return false
        }
        running = true
        return true
    }

    fn wheel_parker(p: Parker) -> None => !p {
        parker = p
    }

    fn waker() -> Parker? {
        if running {
            return copy(parker)
        }
        return None
    }

    fn take_due(now: Long) -> Mut List<Reply<Fired>> => now {
        let due: Mut List<Reply<Fired>> = mut_list_of()
        let i = 0
        while i < size(ats) {
            if get(ats, i)! <= now {
                let _at = remove_at(ats, i)
                if remove_at(dones, i) is Reply<Fired> r {
                    add(due, r)
                }
            } else {
                i = i + 1
            }
        }
        return due
    }

    fn next_deadline() -> Long? {
        let earliest: Long? = None
        for at in ats {
            if earliest is None || at < earliest {
                earliest = copy(at)
            }
        }
        if earliest is None {
            running = false
        }
        return earliest
    }
}

// Answers every token in [due] with the reading [now].
fn answer_all(due: Mut List<Reply<Fired>>, now: Long) [] -> None => !due, now {
    drain(due, r -> send(r, Fired { at: Tick { nanos: copy(now) } }))
}

actor effect Wheel {
    send fn run()
}

handler Wheeling() of Wheel {
    mailbox { capacity: 2 }

    send fn run() {
        wheel_parker(this_parker())
        while true {
            let now = mono_nanos()
            let due = take_due(now)
            answer_all(due, now)
            let until = next_deadline()
            if until is None {
                return
            }
            let wait = until - mono_nanos()
            if wait > 0 {
                park_nanos(this_parker(), wait)
            }
        }
    }
}

use Deadlines()
use Wheeling() on thread()

// [time-timer] Answers [done] with the monotonic reading it fired at once at
// least [delay] nanoseconds have passed — a later activation, never a call inside
// this one, even for a delay of zero or less. `time.DefaultTimer` is the
// surface programs use.
export fn after_nanos(delay: Long, done: Reply<Fired>) [] -> None => delay, !done {
    let wait = delay
    if wait < 0 {
        wait = 0
    }
    if register(mono_nanos() + wait, done) {
        run()
        return
    }
    let p = waker()
    if p is Parker {
        unpark(p)
    }
}
