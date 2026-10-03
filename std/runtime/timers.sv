// `runtime.timers`: the deadline wheel, a service built on the runtime's core
// [runtime-layers] — an actor on a dedicated thread, which the core never
// calls (RUNTIME.md §5.4).

import runtime
import time.Fired
import time.Tick

// [time-timer] One thread serves every deadline in the program, and only while there is
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
    // [test-actor] Forgets every deadline, for a fresh virtual runtime: the
    // tokens are kept, never answered, since what they answer is gone.
    fn clear() -> None
}

handler Deadlines() of DeadlineTable {
    ats: Mut List<Long> = mut_list_of()
    dones: Mut List<Reply<Fired>> = mut_list_of()
    running: Bool = false
    parker: Parker? = None
    forgotten: Mut List<Reply<Fired>> = mut_list_of()

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

    fn clear() -> None {
        while remove_first(dones) is Reply<Fired> r {
            add(forgotten, r)
        }
        while remove_first(ats) is Long {
        }
        running = false
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
            let now = now_nanos()
            let due = take_due(now)
            answer_all(due, now)
            let until = next_deadline()
            if until is None {
                return
            }
            let wait = until - now_nanos()
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
    // [test-actor] On the virtual runtime there is no wheel: the scheduler
    // moves the clock to the earliest deadline when nothing can run.
    if virtual_runtime() {
        if register(now_nanos() + wait, done) {
            arm_clock()
        }
        return
    }
    if register(now_nanos() + wait, done) {
        run()
        return
    }
    let p = waker()
    if p is Parker {
        unpark(p)
    }
}

// [test-actor] Asks the scheduler to run [advance] once nothing can run.
fn arm_clock() [] -> None {
    on_clock(mint_task_on(main_pool(), body_of((kind, slot, value) -> {
        drop_dyn(value)
        advance()
    })))
}

// [test-actor] Moves the virtual clock to the earliest deadline and fires
// what is due there, arming the clock again while any deadline is left.
fn advance() [] -> None {
    let until = next_deadline()
    if until is Long at {
        set_virtual_now(at)
        let now = now_nanos()
        let due = take_due(copy(now))
        answer_all(due, now)
        if next_deadline() is Long {
            arm_clock()
        }
    }
}

// [test-actor] Forgets every pending deadline, for a fresh virtual runtime.
export fn reset_timers() [] -> None {
    clear()
}
