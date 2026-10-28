// [actor-use-addr] the forwarding stub
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_stub_bound_effect

actor effect Log {
    send fn note(what: Str) => !what
    send fn count(out: Reply<Int>) => !out
}

handler Counting() of Log {
    mailbox { capacity: 8 }

    state {
        seen: Int = 0
    }

    send fn note(what: Str) {
        seen = seen + 1
    }

    send fn count(out: Reply<Int>) {
        out.send(seen)
    }
}

fn work() [Log] {
    note("a")
    note("b")
}

fn main() [use, spawn] {
    use StdConsole()
    let logger = spawn Counting() on pool(1)
    use logger
    work()
    let n = waitfor out: Reply<Int> {
        count(out)
    }
    println("noted ${n}")
}
//= stdout
//= noted 2

//--- an_inferred_lend

struct Stepped : Next<self, proj Str> canbe Mut {
    items: proj List<Str>,
    at: Int,
    step: Int
}

fn next(p: Mut Stepped) [] -> Emitted (proj(p) Str) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + p.step
    return emitted(elem)
}

fn iter(list: List<Str>, step: Int) [] -> Mut Stepped => list, step {
    return Mut Stepped { items: list, at: 0, step: copy(step) }
}

// No `holds proj(...)` written on either slot.
fn count<C, K, It, T>(c: C, k: K, ?iter: (c: C, k: K) -> Mut It, ?Next<It, T>) -> Int
=> c, k {
    let n = 0
    for _ in iter(c, k) {
        n = n + 1
    }
    return n
}

fn total<C, It, T>(c: C, ?iter: (c: C) -> Mut It, ?Next<It, T>) -> Int => c {
    let n = 0
    for _ in iter(c) {
        n = n + 1
    }
    return n
}

fn main() [use] -> None {
    use StdConsole()
    println("${count(List("a", "b", "c", "d", "e", "f"), 2)}")
    println("${total(List("x", "y", "z"))}")
    // The owning pass still fits the same slot: the lifetime it does not use
    // costs it nothing, which is why inferring the lend is free.
    println("${total(Set("p", "q"))}")
}
//= stdout
//= 3
//= 3
//= 2

//--- is_bindings_over_calls

fn drain_ints(xs: Mut List<Int>) [Console] -> None => xs: Mut {
    // statement `while`: one call per turn, so every element is seen
    while remove_first(xs) is Int n {
        println("  took ${n}")
    }
}

fn first_or(xs: Mut List<Int>, fallback: Int) [] -> Int => xs: Mut, fallback {
    // value-position `if`: one call, and the binding is that call's value
    return if remove_first(xs) is Int n { n } else { fallback }
}

fn sum_down(xs: Mut List<Int>) [] -> Int => xs: Mut {
    let total = 0
    // value-position `while`: the loop's value is its last body value
    let last = while remove_first(xs) is Int n {
        total = total + n
        n
    } else { 0 }
    discard(last)
    return total
}

fn main() [use] {
    use StdConsole()
    let a: Mut List<Int> = Mut List(1, 2, 3)
    drain_ints(a)
    println("left ${size(a)}")

    let b: Mut List<Int> = Mut List(7, 8)
    println("first ${first_or(b, 0)} then ${size(b)}")

    let c: Mut List<Int> = Mut List(1, 2, 3, 4)
    println("sum ${sum_down(c)} left ${size(c)}")

    // statement `if`: the call happens once, so the list loses exactly one
    let d: Mut List<Int> = Mut List(5, 6)
    if remove_first(d) is Int n {
        println("one ${n} left ${size(d)}")
    }
}
//= stdout
//=   took 1
//=   took 2
//=   took 3
//= left 0
//= first 7 then 1
//= sum 10 left 0
//= one 5 left 1

//--- a_task_that_inherits_effects

send fn report(out: Reply<Int>, n: Int) [Console] => !out, !n {
    println("task saw ${n}")
    out.send(n * 2)
}

send fn chain(out: Reply<Int>, n: Int) [Console] => !out, !n {
    println("chain saw ${n}")
    let inner = waitfor r: Reply<Int> {
        let k = replyto report(r)
        k.send(n + 1)
    }
    out.send(inner)
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let direct = waitfor a: Reply<Int> {
        let k = replyto report(a)
        k.send(21)
    }
    println("direct ${direct}")
    let nested = waitfor b: Reply<Int> {
        let k = replyto chain(b) on pool(2)
        k.send(10)
    }
    println("nested ${nested}")
}
//= stdout
//= task saw 21
//= direct 42
//= chain saw 10
//= task saw 11
//= nested 22

//--- the_task_kernel

actor effect Db {
    send fn query(id: Int, reply: Reply<Int>) => !reply
}

handler Rows() of Db {
    mailbox { capacity: 4 }

    send fn query(id: Int, reply: Reply<Int>) {
        reply.send(id * 10)
    }
}

// The free send fn: no return type, every parameter consumed, and it runs on a
// pool rather than in anyone's frame. Its trailing parameter is the answer.
send fn finish(label: Str, out: Reply<Str>, row: Int) => !label, !out, !row {
    out.send("${label}=${row}")
}

// An ordinary function — full features, synchronous — that wires future work
// and returns. It never parks and never blocks, and it is called identically
// from `main` and from anywhere else.
fn fetch(id: Int, out: Reply<Str>) [Db] -> None => !out {
    query(id, replyto finish("row", out))
}

fn fetch_on(id: Int, out: Reply<Str>, p: Pool) [Db] -> None => !out, p {
    query(id, replyto finish("placed", out) on p)
}

fn main() [use, spawn] {
    use StdConsole()
    let workers = pool(2)
    let db = spawn Rows() on workers
    use db

    // The continuation inherits `main`'s pool, so it runs on main's own thread
    // while main waits [waitfor-pump].
    println(waitfor a: Reply<Str> { fetch(7, a) })
    // And here it runs on the worker pool instead.
    println(waitfor b: Reply<Str> { fetch_on(4, b, workers) })
}
//= stdout
//= row=70
//= placed=40

//--- the_waitfor_package

actor effect TimerApi {
    send fn after(ms: Int, out: Reply<Int>) => !out
}

actor effect ClockApi {
    send fn now(out: Reply<Int>) => !out
}

actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

handler Timing(at: Int) of TimerApi {
    mailbox { capacity: 4 }

    send fn after(ms: Int, out: Reply<Int>) {
        out.send(at + ms)
    }
}

// The T-5 shape: a handler whose member *waits* for another actor's answer.
// It declares the capability, so every spawn of it must give it a thread.
handler Clocking() [TimerApi] of ClockApi {
    mailbox { capacity: 4 }

    send fn now(out: Reply<Int>) {
        let at = waitfor fired: Reply<Int> {
            after(0, fired)
        }
        out.send(at)
    }
}

handler Counting() of Counter {
    mailbox { capacity: 4 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

fn main() [use, spawn] {
    use StdConsole()

    let timer = spawn Timing(1000) on pool(1)
    // `thread()` is consumed here: one thread, one occupant.
    let clock = spawn Clocking() with timer on thread()
    let now = waitfor out: Reply<Int> {
        clock.now(out)
    }
    println("the clock says ${now}")

    // A single-thread pool: these activations run during the `waitfor`
    // below, on that one worker thread.
    let counter = spawn Counting() on pool(1)
    counter.bump(2)
    counter.bump(3)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    println("the solo pool's actor says ${sum}")
}
//= stdout
//= the clock says 1000
//= the solo pool's actor says 5
