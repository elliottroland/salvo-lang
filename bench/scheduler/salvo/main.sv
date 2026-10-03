// The scheduler benchmarks (the runtime record, D6): four workloads, each timed on
// the monotonic clock and printed as milliseconds. Run with
// `tools/bench-scheduler.sh`, which builds both backends optimised and prints
// the numbers side by side. Not part of the test suite: the output is timing.
//
//   ping-pong  one actor asking another, 10^6 round trips, actor to actor
//   fan-out    1,000 actors on one pool, 100 messages each, then every total
//   tasks      a chain of 10^5 tasks, each scheduling the next
//   timers     10^4 deadlines registered at once, counted as they fire
import time

// ===== ping-pong =====

actor effect Ponger {
    send fn ping(out: Reply<Int>) => !out
}

handler Ponging() of Ponger {
    mailbox { capacity: 16 }

    send fn ping(out: Reply<Int>) {
        out.send(1)
    }
}

actor effect Pinger {
    send fn start(n: Int, done: Reply<Int>) => !n, !done
    send fn got(n: Int, done: Reply<Int>, v: Int) => !n, !done, !v
}

handler Pinging() [Ponger] of Pinger {
    mailbox { capacity: 16 }

    send fn start(n: Int, done: Reply<Int>) {
        ping(replyto got(n, done))
    }

    send fn got(n: Int, done: Reply<Int>, v: Int) {
        discard(v)
        if n <= 1 {
            done.send(0)
        } else {
            ping(replyto got(n - 1, done))
        }
    }
}

// ===== fan-out =====

actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 128 }

    sum: Int = 0

    send fn bump(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

// ===== tasks =====

send fn hop(left: Int, out: Reply<Int>, x: Int) => !left, !out, !x {
    if left == 0 {
        out.send(x)
    } else {
        send(replyto hop(left - 1, out), x + 1)
    }
}

// ===== timers =====

actor effect Alarms {
    send fn arm(n: Int, done: Reply<Int>) => !n, !done
    send fn rang(f: Fired) => !f
}

handler Alarming() [Timer] of Alarms {
    mailbox { capacity: 16 }

    left: Int = 0
    waiting: Mut List<Reply<Int>> = mut_list_of()

    send fn arm(n: Int, done: Reply<Int>) {
        left = n
        add(waiting, done)
        let i = 0
        while i < left {
            after(millis(1), replyto rang())
            i = i + 1
        }
    }

    send fn rang(f: Fired) {
        discard(f)
        left = left - 1
        if left == 0 {
            drain(waiting, r -> send(r, 0))
            waiting = mut_list_of()
        }
    }
}

fn ms_since(start: Long) [] -> Long {
    return (monotonic_nanos() - start) / 1000000
}

fn main() [use, spawn] {
    use StdOutConsole()
    let workers = pool(2)

    let pong = spawn Ponging() on workers
    let ping = spawn Pinging() with pong on workers
    let t1 = monotonic_nanos()
    let r1 = waitfor done: Reply<Int> {
        ping.start(1000000, done)
    }
    discard(r1)
    println("ping-pong ${ms_since(t1)} ms")

    let counters: Mut List<Addr<Counter>> = mut_list_of()
    let t2 = monotonic_nanos()
    let i = 0
    while i < 1000 {
        add(counters, spawn Counting() on workers)
        i = i + 1
    }
    let round = 0
    while round < 100 {
        for c in counters {
            c.bump(1)
        }
        round = round + 1
    }
    let all = 0
    for c in counters {
        let t = waitfor out: Reply<Int> {
            c.total(out)
        }
        all = all + t
    }
    println("fan-out ${ms_since(t2)} ms (${all})")

    let t3 = monotonic_nanos()
    let r3 = waitfor out: Reply<Int> {
        send(replyto hop(100000, out) on workers, 0)
    }
    println("tasks ${ms_since(t3)} ms (${r3})")

    use DefaultTimer()
    let alarms = spawn Alarming() on workers
    let t4 = monotonic_nanos()
    let r4 = waitfor done: Reply<Int> {
        alarms.arm(10000, done)
    }
    discard(r4)
    println("timers ${ms_since(t4)} ms")
}
