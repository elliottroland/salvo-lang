// the network sequence, step ⑤: actor groups
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_router_of_any
import std.time

actor effect Resizer {
    send fn resize(name: Str, w: Int, out: Reply<Str>) => !name, !out
}

handler Resizing(who: Str) of Resizer {
    mailbox { capacity: 8 }
    send fn resize(name: Str, w: Int, out: Reply<Str>) => !out {
        out.send("${who} resized ${name} to ${w}")
    }
}

// [effect-any] A hand-written router: forwards each request to the next
// member, so it promises no order between two sends — and says so.
handler RoundRobin(members: List<eff[Resizer]>) of any Resizer {
    mailbox { capacity: 16 }
    state {
        next: Int = 0
    }
    send fn resize(name: Str, w: Int, out: Reply<Str>) => !out {
        let i = next % size(members)
        next = next + 1
        let target = copy(get(members, i) ?: unreachable!())
        target.resize(name, w, out)
    }
}

// Any Resizer will do: one send, one reply.
fn thumbnail(name: Str) [any Resizer] -> Str => !name {
    return waitfor out: Reply<Str> { resize(name, 100, out) }
}

fn main() [use, spawn] {
    use StdConsole()
    let p = pool(2)
    let a = spawn Resizing("a") on p
    let b = spawn Resizing("b") on p
    use RoundRobin(List(a, b))
    println(thumbnail("cat.png"))
    println(thumbnail("dog.png"))
    println(thumbnail("owl.png"))
}
//= stdout
//= a resized cat.png to 100
//= b resized dog.png to 100
//= a resized owl.png to 100

//--- an_effect_typed_generic
actor effect Ping { send fn ping(n: Int) => !n
    send fn count(out: Reply<Int>) => !out }
handler Pinging() [Console] of Ping {
    mailbox { capacity: 4 }
    state {
        seen: Int = 0
    }
    send fn ping(n: Int) => !n {
        seen = seen + 1
        println("ping ${n}")
    }
    send fn count(out: Reply<Int>) => !out { out.send(seen) }
}

actor effect Reg<E> {
    send fn join(member: eff[E]) => !member
    send fn members(out: Reply<List<eff[E]>>) => !out
}

handler Registering<E>() of Reg<E> {
    mailbox { capacity: 8 }
    state {
        all: Mut List<eff[E]> = Mut List()
    }
    send fn join(member: eff[E]) => !member { add(all, member) }
    send fn members(out: Reply<List<eff[E]>>) => !out { out.send(copy(all)) }
}

fn main() [use, spawn] {
    use StdConsole()
    let p = pool(1)
    let reg = spawn Registering<Ping>() on p
    let a = spawn Pinging() on p
    reg.join(copy(a))
    let ms = waitfor out: Reply<List<eff[Ping]>> { reg.members(out) }
    println("members: ${size(ms)}")
    for m in ms { m.ping(7) }
    let _seen = waitfor out: Reply<Int> { a.count(out) }
}
//= stdout
//= members: 1
//= ping 7

//--- elvis_over_a_borrowed_optional
fn main() [use] {
    use StdConsole()
    let xs: List<Int> = List(3)
    let names: List<Str> = List("ann")
    let m: Mut Map<Str, Int> = Mut Map<Str, Int>()
    put(m, "a", 3)
    let a = get(xs, 0) ?: 10
    let b = get(m, "a") ?: 10
    let c = get(m, "zz") ?: 10
    let d = copy(get(names, 0) ?: get(names, 0) ?: unreachable!())
    let e = if get(names, 9) is Str s { copy(s) } else { "nobody" }
    let h = get(names, 9) ?: get(names, 0) ?: unreachable!()
    let first = get(xs, 0)
    let w = when first { is Int { first + 1 } is None { 0 } }
    let found = get(xs, 0)
    let f = found ?: 5
    let g = if found is Int v { v * 2 } else { 0 }
    println("${a} ${b} ${c} ${d} ${e} ${w} ${f} ${g} ${h}")
}
//= stdout
//= 3 3 10 ann nobody 4 3 6 ann

//--- handler_init
actor effect Registry {
    send fn register(who: eff[Worker]) => !who
    send fn count(out: Reply<Int>) => !out
}
actor effect Worker {
    send fn work(out: Reply<Str>) => !out
}

handler Registering() of Registry {
    mailbox { capacity: 8 }
    state {
        n: Int = 0
    }
    send fn register(who: eff[Worker]) => !who { n = n + 1 }
    send fn count(out: Reply<Int>) => !out { out.send(n) }
}

// Registers itself as it starts: `init` runs first, and `self@Worker` is
// this actor's own address as its Worker face.
handler Working(registry: eff[Registry], who: Str) of Worker {
    mailbox { capacity: 8 }
    state {
        started: Int = 0
    }
    init {
        started = 1
        registry.register(self@Worker)
    }
    send fn work(out: Reply<Str>) => !out { out.send("${who} (started ${started})") }
}

effect Greeter { fn greet() -> Str }
handler Greeting(name: Str) of Greeter {
    state {
        prefix: Str = ""
    }
    init { prefix = "hello, " }          // a constructor body for a plain handler
    fn greet() -> Str { return "${prefix}${name}" }
}

effect Clock { fn now() -> Int }
handler FixedClock(t: Int) of Clock { fn now() -> Int { return copy(t) } }
// A dependent handler's `init` under a `use`: the dependency is readable.
handler Stamped(who: Str) [Clock] of Greeter {
    state {
        started_at: Int = 0
    }
    init { started_at = now() }
    fn greet() -> Str { return "hello ${who} since ${started_at}" }
}

fn main() [use, spawn] {
    use StdConsole()
    let p = pool(2)
    let reg = spawn Registering() on p
    let a = spawn Working(copy(reg), "a") on p
    let b = spawn Working(copy(reg), "b") on p
    println(waitfor out: Reply<Str> { a.work(out) })
    println(waitfor out: Reply<Str> { b.work(out) })
    println("registered: ${waitfor out: Reply<Int> { reg.count(out) }}")
    use Greeting("world")
    println(greet())
    use FixedClock(42)
    use Stamped("x")
    println(greet())
}
//= stdout
//= a (started 1)
//= b (started 1)
//= registered: 2
//= hello, world
//= hello x since 42

//--- an_actor_group_across_nodes
import std.net
import std.time

actor effect Ping {
    send fn ping(out: Reply<Str>) => !out
}

handler Pinging(who: Str) of Ping {
    mailbox { capacity: 8 }
    send fn ping(out: Reply<Str>) => !out { out.send("pong from ${who}") }
}

handler Noticing(label: Str) [Console] of ActorGroupWatcher<Ping> {
    mailbox { capacity: 16 }
    send fn joined(member: eff[Ping]) => !member { println("${label}: + a Ping (local: ${eq(node_of(member), this_node())})") }
    send fn left(member: eff[Ping]) => !member { println("${label}: - a Ping") }
}

actor effect Boot {
    send fn boot(done: Reply<eff[ActorGroup<Ping>]>) => !done
}

handler Booting(at: NodeEndpoint, all: List<NodeEndpoint>, net: eff[MemNet]) [Transport, spawn] of Boot {
    mailbox { capacity: 1 }
    send fn boot(done: Reply<eff[ActorGroup<Ping>]>) => !done {
        let p = pool(1)
        let nodes = spawn StaticNodeGroup("demo", copy(all)) on p
        let pings = actor_group<Ping>(nodes)
        spawn Pinging("b") on p in pings
        done.send(pings)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let a = NodeEndpoint { host: "a", port: 1 }
    let b = NodeEndpoint { host: "b", port: 1 }
    let all = List(copy(a), copy(b))
    let net = spawn MemNetwork() on pool(1)

    use MemTransport(copy(a), copy(net))
    let pa = pool(1)
    let _connected = connect(copy(a), copy(pa))
    let nodes_a = spawn StaticNodeGroup("demo", copy(all)) on pa
    let pings_a = actor_group<Ping>(nodes_a)
    pings_a.subscribe(spawn Noticing("a") on pa)
    spawn Pinging("a") on pa in pings_a

    let pb = pool_at(new_node(), 1)
    let booter = spawn Booting(copy(b), copy(all), copy(net)) with MemTransport(copy(b), copy(net)) on pb
    let pings_b = waitfor done: Reply<eff[ActorGroup<Ping>]> { booter.boot(done) }

    let timer = spawn DefaultTimer() on pool(1)
    let _t = waitfor f: Reply<Fired> { timer.after(millis(500), f) }
    let seen_a = waitfor out: Reply<List<eff[Ping]>> { pings_a.members(out) }
    let seen_b = waitfor out: Reply<List<eff[Ping]>> { pings_b.members(out) }
    println("a's group sees ${size(seen_a)}, b's sees ${size(seen_b)}")
    for m in seen_a {
        let answer = waitfor r: Reply<Str> { m.ping(r) }
        println("  ${answer}")
    }
}
//= stdout
//= a: + a Ping (local: true)
//= a: + a Ping (local: false)
//= a's group sees 2, b's sees 2
//=   pong from a
//=   pong from b

//--- private_send_members
actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 8 }
    state {
        sum: Int = 0
    }
    send fn bump(n: Int) { sum = sum + n }
    send fn total(out: Reply<Int>) { out.send(sum) }
}

actor effect Report {
    send fn report(label: Str, out: Reply<Str>) => !label, !out
}

// Two private members: `twice` is reached by a self-send, `reported` is a
// continuation target with two captures.
handler Reporting(counter: eff[Counter]) of Report {
    mailbox { capacity: 8 }
    state {
        asked: Int = 0
    }
    send fn report(label: Str, out: Reply<Str>) {
        twice@self(1)
        counter.total(replyto reported(label, out))
    }
    send fn twice(n: Int) => !n {
        asked = asked + n * 2
    }
    send fn reported(label: Str, out: Reply<Str>, total: Int) => !label, !out, !total {
        out.send("${label}: total ${total}, asked ${asked}")
    }
}

// The same shape on a dependent handler (`[Console]`), which dispatches
// through the fused form.
handler LoudReporting(counter: eff[Counter]) [Console] of Report {
    mailbox { capacity: 8 }
    send fn report(label: Str, out: Reply<Str>) {
        note@self(copy(label))
        counter.total(replyto reported(label, out))
    }
    send fn note(label: Str) => !label {
        println("asking for ${label}")
    }
    send fn reported(label: Str, out: Reply<Str>, total: Int) => !label, !out, !total {
        out.send("${label}: ${total}")
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let p = pool(2)
    let c = spawn Counting() on p
    c.bump(3)
    c.bump(4)
    let r = spawn Reporting(copy(c)) on p
    println(waitfor out: Reply<Str> { r.report("first", out) })
    println(waitfor out: Reply<Str> { r.report("second", out) })
    let loud = spawn LoudReporting(c) on p
    println(waitfor out: Reply<Str> { loud.report("third", out) })
}
//= stdout
//= first: total 7, asked 2
//= second: total 7, asked 4
//= asking for third
//= third: 7

//--- least_loaded_and_elected_routes
import std.net
import std.time

actor effect Sequencer {
    send fn next(out: Reply<Str>) => !out
}

handler Sequencing(who: Str) of Sequencer {
    mailbox { capacity: 8 }
    state {
        n: Int = 0
    }
    send fn next(out: Reply<Str>) => !out {
        n = n + 1
        out.send("${who}#${n}")
    }
}

actor effect Boot {
    send fn boot(done: Reply<eff[Sequencer]>) => !done
}

handler Booting(at: NodeEndpoint, all: List<NodeEndpoint>, net: eff[MemNet]) [Transport, spawn] of Boot {
    mailbox { capacity: 1 }
    send fn boot(done: Reply<eff[Sequencer]>) => !done {
        let p = pool(1)
        let nodes = spawn StaticNodeGroup("ids", copy(all)) on p
        let seq = actor_group<Sequencer>(nodes)
        let mine = spawn Sequencing("b") on p in seq
        done.send(mine)
    }
}

fn fresh_id() [any Sequencer] -> Str {
    return waitfor out: Reply<Str> { next(out) }
}

fn main() [use, spawn] {
    use StdConsole()
    let a = NodeEndpoint { host: "a", port: 1 }
    let b = NodeEndpoint { host: "b", port: 1 }
    let all = List(copy(a), copy(b))
    let net = spawn MemNetwork() on pool(1)
    use MemTransport(copy(a), copy(net))
    let pa = pool(1)
    let _connected = connect(copy(a), copy(pa))
    let nodes = spawn StaticNodeGroup("ids", copy(all)) on pa
    let seq = actor_group<Sequencer>(nodes)
    spawn Sequencing("a") on pa in seq

    let pb = pool_at(new_node(), 1)
    let booter = spawn Booting(copy(b), copy(all), copy(net)) with MemTransport(copy(b), copy(net)) on pb
    let remote = waitfor done: Reply<eff[Sequencer]> { booter.boot(done) }
    let timer = spawn DefaultTimer() on pool(1)
    let _t = waitfor f: Reply<Fired> { timer.after(millis(300), f) }
    let seen = waitfor out: Reply<List<eff[Sequencer]>> { seq.members(out) }
    println("members: ${size(seen)}")

    // Least loaded, local first: every id comes from a's member.
    use LeastLoaded<Sequencer>(true)
    use any copy(seq)
    println("${fresh_id()} ${fresh_id()}")

    // Elected: the leader is b's node, so ids come from b's member.
    use StaticLeader(node_of(remote))
    use Elected<Sequencer>()
    use any seq
    println("${fresh_id()} ${fresh_id()}")
}
//= stdout
//= members: 2
//= a#1 a#2
//= b#1 b#2

//--- a_sharded_route
import std.net
import std.time

actor effect Inventory {
    send fn reserve(sku: Key Str, qty: Int, out: Reply<Str>) => !sku, !qty, !out
}

handler Stocking(shard: Str) of Inventory {
    mailbox { capacity: 32 }
    state {
        served: Int = 0
    }
    send fn reserve(sku: Key Str, qty: Int, out: Reply<Str>) => !sku, !qty, !out {
        served = served + qty
        out.send("${shard} took ${sku} (${served} so far)")
    }
}

fn checkout(skus: List<Str>) [any Inventory, Console] -> None {
    for sku in skus {
        let line = waitfor out: Reply<Str> { reserve(copy(sku), 1, out) }
        println(line)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let a = NodeEndpoint { host: "a", port: 1 }
    let net = spawn MemNetwork() on pool(1)
    use MemTransport(copy(a), copy(net))
    let p = pool(1)
    let _connected = connect(copy(a), copy(p))
    let nodes = spawn StaticNodeGroup("shop", List(copy(a))) on p
    let stock = actor_group<Inventory>(nodes)
    spawn Stocking("s1") on p in stock
    spawn Stocking("s2") on p in stock

    use Sharded<Inventory>()
    use any stock
    checkout(List("apple", "pear", "apple", "fig", "pear"))
}
//= stdout
//= s1 took apple (1 so far)
//= s2 took pear (1 so far)
//= s1 took apple (2 so far)
//= s1 took fig (3 so far)
//= s2 took pear (2 so far)

//--- spawn_in_group
import std.net
import std.time

actor effect Ping { send fn ping(out: Reply<Str>) => !out }
actor effect Admin { send fn stop() }

handler Pinging(who: Str) of Ping {
    mailbox { capacity: 4 }
    send fn ping(out: Reply<Str>) => !out { out.send("pong from ${who}") }
}
handler TwoFaced(who: Str) of Ping, Admin {
    mailbox { capacity: 4 }
    send fn ping(out: Reply<Str>) => !out { out.send("pong from ${who}") }
    send fn stop() {}
}

fn main() [use, spawn] {
    use StdConsole()
    let a = NodeEndpoint { host: "a", port: 1 }
    let net = spawn MemNetwork() on pool(1)
    use MemTransport(copy(a), copy(net))
    let p = pool(1)
    let _c = connect(copy(a), copy(p))
    let nodes = spawn StaticNodeGroup("demo", List(copy(a))) on p
    let pings = actor_group<Ping>(nodes)
    let one = spawn Pinging("one") on p in pings
    let (two, _admin) = spawn TwoFaced("two") on p in pings
    let timer = spawn DefaultTimer() on pool(1)
    let _t = waitfor f: Reply<Fired> { timer.after(millis(100), f) }
    let members = waitfor out: Reply<List<eff[Ping]>> { pings.members(out) }
    println("members: ${size(members)}")
    println(waitfor out: Reply<Str> { one.ping(out) })
    println(waitfor out: Reply<Str> { two.ping(out) })
}
//= stdout
//= members: 2
//= pong from one
//= pong from two

// [time-coupling] The three postures, and the unified test clock in particular:
// a measurement taken across virtual time is exact, because one clock is behind
// both the deadline and the reading.
//--- the_coupling_postures

import std.time

// Time as data: no effect, so nothing to fake.
fn verdict(started: Tick, at: Tick, budget: Duration) [] -> Str {
    if between(started, at) > budget {
        return "late"
    }
    return "in time"
}

// A scripted clock: readings from the constructor, half a second apart.
handler SteppingTicker(step: Duration) of Ticker {
    state {
        at: Long = 0
    }

    fn tick() -> Tick {
        at = at + step.nanos
        return Tick {nanos: at}
    }
}

// The unified test clock: a reading is a deadline of zero, so the answer is the
// timer's own virtual now. The timer arrives as a value, not a dependency — a
// handler with dependencies of its own cannot be built in a spawn `with` clause.
handler TestTicker(timer: eff[Timer]) of Ticker {
    fn tick() -> Tick {
        let fired = waitfor answer: Reply<Fired> {
            timer.after(nanos(0), answer)
        }
        return fired.at
    }
}

actor effect Sleeper {
    send fn nap(wait: Duration, out: Reply<Str>) => !wait, !out
    send fn woke(started: Tick, out: Reply<Str>, f: Fired) => !started, !out, !f
}

handler Napping() [Timer, Ticker] of Sleeper {
    mailbox { capacity: 8 }

    send fn nap(wait: Duration, out: Reply<Str>) {
        after(wait, replyto woke(tick(), out))
    }

    send fn woke(started: Tick, out: Reply<Str>, f: Fired) {
        send(out, "napped ${elapsed(started)}, fired at ${to_millis(between(Tick {nanos: 0}, f.at))}ms")
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    println("data: ${verdict(Tick {nanos: 0}, Tick {nanos: 1000000000}, millis(1500))}")
    use SteppingTicker(millis(500))
    let started = tick()
    println("scripted: ${to_millis(elapsed(started))} ${to_millis(elapsed(started))}")

    let p = pool(1)
    let (timer, ctl) = spawn ManualTime() on p
    let sleeper = spawn Napping() with timer, TestTicker(timer) on thread()
    let napped = waitfor answer: Reply<Str> {
        sleeper.nap(seconds(2), answer)
        waitfor settled: Reply<Idle> {
            on_idle(p, settled)
        }
        ctl.advance(seconds(2))
    }
    println("unified: ${napped}")
}
//= stdout
//= data: in time
//= scripted: 500 1000
//= unified: napped 2s, fired at 2000ms

// [time-manual] Virtual time, in pure Salvo, through a two-face handler.
//--- manual_time

import std.time

actor effect Sleeper {
    send fn nap(wait: Duration, done: Reply<Str>) => !wait, !done
    send fn woke(done: Reply<Str>, f: Fired) => !done, !f
}

handler Napping() [Timer] of Sleeper {
    mailbox { capacity: 8 }

    send fn nap(wait: Duration, done: Reply<Str>) => !wait, !done {
        after(wait, replyto woke(done))
    }

    send fn woke(done: Reply<Str>, f: Fired) => !done, !f {
        send(done, "woke at ${to_millis(between(Tick {nanos: 0L}, f.at))}ms")
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let p = pool(1)
    let (timer, ctl) = spawn ManualTime() on p
    let sleeper = spawn Napping() with timer on p
    let answer = waitfor result: Reply<Str> {
        sleeper.nap(seconds(2), result)
        waitfor settled: Reply<Idle> {
            on_idle(p, settled)
        }
        ctl.advance(seconds(2))
    }
    println(answer)
    // Virtual time *accumulates*: the second nap measures from where the first
    // advance left `now`, so a one-second deadline answers at three seconds.
    let second = waitfor later: Reply<Str> {
        sleeper.nap(seconds(1), later)
        waitfor drained: Reply<Idle> {
            on_idle(p, drained)
        }
        ctl.advance(seconds(1))
    }
    println(second)
}
//= stdout
//= woke at 2000ms
//= woke at 3000ms
