// The deadline service behind [std.time]'s timers: one thread that answers
// every pending deadline in the program when it comes due, and runs only
// while there is one.
//
// This is std's own module, as everything under `std.runtime` is: a program
// cannot import it. Programs set timers through [std.time], whose
// [DefaultTimer] handler hands each `after` to [after_nanos] here:
//
// ```no-check
// // In std.time.
// export handler DefaultTimer() for Ordered Timer {
//     mailbox { capacity: 64 }
//
//     send fn after(wait: Duration, done: Reply<Fired>) => !wait, !done {
//         after_nanos(wait.nanos, done)
//     }
// }
// ```
//
// **What a deadline promises.** [after_nanos] answers its reply with a
// `Fired` once at least the delay has passed on the monotonic clock, so a
// deadline never moves when the wall clock is adjusted. `Fired.at` is the
// clock's reading when the deadline was found due: never earlier than the
// deadline, and later by however long the thread took to wake. The answer
// is always a later activation of whoever waits on the reply, never a call
// made inside [after_nanos], even for a delay of zero. There is no
// cancellation: a deadline nobody wants any more still fires, into a
// continuation that finds its work already done.
//
// **One thread for every deadline.** The deadlines are kept in a table
// (`Deadlines`) and served by the **wheel**, an actor on a thread of its
// own. Its one activation answers what is due, sleeps until the earliest
// deadline left, and returns when none is left. There is no thread per
// timer and no polling.
//
// **A pending deadline is work on its way.** While a deadline is pending the
// wheel's frame is running, so the program is not idle: an `on_idle` hook
// does not fire, and a `waitfor` on the reply is not reported as a deadlock.
// With no deadline pending the wheel has no frame, so an idle program can
// be reported idle, and a wait nothing can answer can be reported stuck.
//
// **Costs.** Registering a deadline appends it to the table, under the
// table's lock, and wakes the wheel if it is asleep or starts it if it has
// stopped. Each time the wheel wakes it scans every pending deadline, so a
// wake costs time linear in the number pending.
//
// **In an actor test there is no wheel.** On the virtual runtime (see
// [std.test.actor]) a deadline goes into the same table, and the scheduler,
// once nothing else can run, moves the virtual clock to the earliest
// deadline and answers everything due there. `Fired.at` is then exactly the
// deadline. [reset_timers] empties the table for each new actor test.
// @compilerrules [runtime-layers, time-timer, test-actor]

import std.runtime
import std.time.Fired
import std.time.Tick

// Registrations go into `Deadlines`, a monitor, not into the wheel's
// mailbox: the mailbox holds at most the one `run` that starts the wheel,
// so a burst of registrations never fills it. Waking is by the wheel's
// `Parker`: an unpark that lands before its park makes the park return at
// once, so no wakeup is lost between "what is earliest" and going to sleep.
// The deadlines and their tokens are two index-aligned lists, since a list
// of obligations can only be reached by `remove_at`.
// @compilerrules [time-timer, runtime-parker, linear-container]

// The table of pending deadlines and the wheel's running state.
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
    // Forgets every deadline, for a fresh virtual runtime: the
    // tokens are kept, never answered, since what they answer is gone.
    // @compilerrules [test-actor]
    fn clear() -> None
}

handler Deadlines() for DeadlineTable {
    state {
        ats: Mut List<Long> = Mut List()
        dones: Mut List<Reply<Fired>> = Mut List()
        running: Bool = false
        parker: Parker? = None
        forgotten: Mut List<Reply<Fired>> = Mut List()
    }

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
        let due: Mut List<Reply<Fired>> = Mut List()
        let i = 0
        while i < size(ats) {
            if (get(ats, i) ?: unreachable!()) <= now {
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
        while remove_first(ats) is Long {}
        running = false
    }
}

// Answers every reply in [due] with a `Fired` at the reading [now].
fn answer_all(due: Mut List<Reply<Fired>>, now: Long) [] -> None => !due, now {
    drain(due, (r) -> send(r, Fired { at: Tick { nanos: copy(now) } }))
}

// The wheel: the actor that serves the deadlines, on a thread of its own.
actor effect Wheel {
    send fn run()
}

// The wheel's one activation: answers what is due, sleeps until the
// earliest deadline left, and returns once none is.
handler Wheeling() for Ordered Wheel {
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

// Answers a reply with a `Fired` once at least a delay has passed on the
// monotonic clock.
//
// Returns at once: the deadline is registered and the reply is answered
// later, as a separate activation, never inside this call. A delay of zero
// or less fires as soon as the wheel runs. `Fired.at` is the reading when
// the deadline was found due, which is never before it. Programs reach this
// through [std.time]'s [DefaultTimer].
//
// ```no-check
// // Inside std: wait 10 ms.
// let fired = waitfor done: Reply<Fired> {
//     after_nanos(10000000L, done)
// }
// ```
//
// @param delay  how long to wait, in nanoseconds; less than 0 counts as 0
// @param done   answered once, when the deadline comes due
// @compilerrules [time-timer]
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

// Asks the virtual runtime's scheduler to run [advance] once nothing else
// can run.
// @compilerrules [test-actor]
fn arm_clock() [] -> None {
    on_clock(mint_task_on(main_pool(), body_of((kind, slot, value) -> {
        drop_dyn(value)
        advance()
    })))
}

// Moves the virtual clock to the earliest deadline and fires
// what is due there, arming the clock again while any deadline is left.
// @compilerrules [test-actor]
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

// Forgets every pending deadline, for a fresh virtual runtime.
//
// Called before each actor test, by [begin_actor_test], so that a deadline
// an earlier test left behind does not fire into the next one. The
// forgotten replies are kept and never answered, since whatever waited on
// them is gone.
// @compilerrules [test-actor]
export fn reset_timers() [] -> None {
    clear()
}
