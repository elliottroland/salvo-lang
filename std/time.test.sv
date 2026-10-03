// [test-actor] The timer and the clocks on the virtual runtime: an actor
// test waits for no deadline, and every reading is exact.

actor effect Napper {
    send fn nap(wait: Duration, out: Reply<Tick>) => !wait, !out
    send fn woke(out: Reply<Tick>, f: Fired) => !out, !f
}

handler Napping(timer: Addr<Timer>) of Napper {
    mailbox { capacity: 4 }

    send fn nap(wait: Duration, out: Reply<Tick>) {
        timer.after(wait, replyto woke(out))
    }

    send fn woke(out: Reply<Tick>, f: Fired) {
        send(out, f.at)
    }
}

test actor "an hour's timer fires at once, exactly an hour on" {
    let timer = spawn DefaultTimer() on pool(1)
    use DefaultTicker()
    let started = tick()
    let fired = waitfor answer: Reply<Fired> {
        timer.after(hours(1), answer)
    }
    expect_eq(between(started, fired.at).nanos, hours(1).nanos)
    expect_eq(between(started, tick()).nanos, hours(1).nanos)
}

test actor "deadlines fire in order, each at its own time" {
    let timer = spawn DefaultTimer() on pool(1)
    let napper = spawn Napping(timer) on pool(2)
    use DefaultTicker()
    let started = tick()
    let long = waitfor answer: Reply<Tick> {
        napper.nap(minutes(5), answer)
    }
    let short = waitfor answer: Reply<Tick> {
        napper.nap(seconds(3), answer)
    }
    expect_eq(between(started, long).nanos, minutes(5).nanos)
    expect_eq(between(long, short).nanos, seconds(3).nanos)
}

test actor "each actor test starts its clock at zero" {
    use DefaultTicker()
    expect_eq(tick().nanos, 0L)
}
