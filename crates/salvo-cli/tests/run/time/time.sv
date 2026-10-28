// time [time-types] [time-timer]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [time-types] [mod-import-module] The surface compiles and runs, and both
// backends print the same text.
//--- the_time_surface

import std.time

fn main() [use] -> None {
    use StdConsole()
    use DefaultTicker()
    use DefaultClock()
    let d = millis(1500)
    println("span=${d}")
    println("sum=${plus(d, seconds(1))}")
    println("scaled=${times(d, 2)}")
    println("abs=${abs(minus(seconds(1), seconds(3)))}")
    println("millis=${to_millis(d)}")
    println("units=${micros(250)} ${nanos(37)} ${minutes(2)} ${hours(1)}")
    let t0 = tick()
    let t1 = plus(t0, micros(250))
    println("ticks=${between(t0, t1)} ${between(t1, t0)}")
    let i = epoch_milli(1700000000000)
    println("epoch=${to_epoch_milli(i)} ${to_epoch_second(i)}")
    println("shifted=${to_epoch_milli(plus(i, seconds(2)))}")
    println("wall=${to_epoch_nano(now()) > 0L}")
    println("mono to wall=${to_epoch_nano(to_instant(t0)) > 0L}")
    println("round trip=${to_epoch_nano(to_instant(to_tick(i))) == to_epoch_nano(i)}")
    println("ordered=${t1 > t0} ${d == millis(1500)}")
}
//= stdout
//= span=1500ms
//= sum=2500ms
//= scaled=3s
//= abs=2s
//= millis=1500
//= units=250us 37ns 120s 3600s
//= ticks=250us -250us
//= epoch=1700000000000 1700000000
//= shifted=1700000002000
//= wall=true
//= mono to wall=true
//= round trip=true
//= ordered=true true

// [time-timer] A real deadline fires, and the fires are ordered.
//--- a_real_timer

import std.time

fn main() [use, spawn] -> None {
    use StdConsole()
    use DefaultTicker()
    let timers = spawn DefaultTimer() on pool(1)
    let started = tick()
    let first = waitfor fired: Reply<Fired> {
        timers.after(millis(50), fired)
    }
    let waited = between(started, first.at)
    println("waited enough: ${to_millis(waited) >= 50}")
    let second = waitfor again: Reply<Fired> {
        timers.after(millis(10), again)
    }
    println("in order: ${second.at > first.at}")
}
//= stdout
//= waited enough: true
//= in order: true
