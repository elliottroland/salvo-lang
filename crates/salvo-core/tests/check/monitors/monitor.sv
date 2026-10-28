// [actor-monitor-handler] The monitor spawn (SH-3, user decision 2026-09-19):
// `spawn H(args)` where every face of `H` is a **plain** effect shares one
// instance behind a lock, answers the same `eff[E]` type an actor spawn
// answers, and is bound with `use addr` like any handle.

// The shared shapes: a plain effect with mutable state behind it (the
// monitor), and an actor that depends on it.
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    state {
        cursor: Int = 0
    }

    fn next() -> Int {
        cursor = (cursor * 31 + seed) % 100000
        return cursor
    }
}

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

// The whole surface in one function, the acceptance case: a monitor spawned
// with no `on` clause, its handle `use`-bound (calls resolve through the
// effect), and the same handle supplied to a spawned actor's dependency
// clause. An addr is freely reusable, so one handle serves both. (The actor
// spawn names its pool; only the monitor spawn omits one.)
fn a_monitor_spawn_shares_a_plain_effect_handler() [use, spawn] {
    let rng = spawn CyclicRandom(12345)
    use rng
    let _first = next()
    let drawer = spawn Drawing() with rng on pool(1)
    let _drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    let _again = next()
}

// [actor-monitor-handler] Naming `eff[E]` of a plain effect is legal since
// SH-3: it is the shared monitor's handle, in a parameter, a return, a `let`.
fn hold(handle: eff[Random]) -> Int => handle {
    return 1
}

fn a_plain_effect_addr_is_a_legal_type() [use, spawn] {
    let rng = spawn CyclicRandom(1)
    let _n = hold(rng)
}

// A monitor runs on its callers' threads, so an `on` clause has nothing to
// place and is refused by name.
fn a_monitor_spawn_refuses_a_pool() [use, spawn] {
    //~ error[actor-monitor-handler:on-pool]
    let _rng = spawn CyclicRandom(1) on pool(1)
}

// [actor-monitor-handler] The dependency restriction was lifted 2026-09-20
// (user decision): a monitor may declare deps, captured as owned handles from
// the **enclosing scope** at the spawn. With no binding in scope the
// resolution fails by name; with one, the spawn is legal.
handler Quiet() of Console {
    fn print(message: Str) -> None => message {}
    fn printerr(message: Str) -> None => message {}
    fn readln() -> Str? {
        return None
    }
}

handler Noisy(seed: Int) [Console] of Random {
    state {
        cursor: Int = 0
    }

    fn next() -> Int {
        println("drawing")
        cursor = cursor + seed
        return cursor
    }
}

fn a_monitor_dependency_with_nothing_in_scope() [use, spawn] {
    //~ error[actor-monitor-handler:missing-dep]
    let _rng = spawn Noisy(1)
}

fn a_monitor_dependency_resolved_from_scope() [use, spawn] {
    use Quiet()
    let _rng = spawn Noisy(1)
}

// A handler of several plain effects has no representation yet: one shared
// instance would sit behind several faces. Refused rather than guessed.
effect Peek {
    fn peek() -> Int
}

handler Wide(seed: Int) of Random, Peek {
    state {
        cursor: Int = 0
    }

    fn next() -> Int {
        cursor = cursor + seed
        return cursor
    }

    fn peek() -> Int {
        return cursor
    }
}

fn a_multi_face_monitor_spawn_is_refused() [use, spawn] {
    //~ error[actor-monitor-handler:several-faces]
    let _both = spawn Wide(1)
}

// [effect-handle] [actor-sendable] A handler holding a **function value**
// shares like any other (user decision 2026-09-28): a Salvo lambda captures
// by value, and both backends store a handler's callback in a form that
// crosses threads, so `Derived(step: (n: Int) -> Int)` is spawned as a
// monitor and `use`d alike.
handler Derived(step: (n: Int) -> Int) of Random {
    state {
        cursor: Int = 0
    }

    fn next() -> Int {
        cursor = step(cursor)
        return cursor
    }
}

fn a_monitor_spawn_shares_a_stored_function_value() [use, spawn] {
    let _rng = spawn Derived(n -> n + 1)
    use Derived(n -> n + 2)
    let _ok = next()
}

// ===== every binding is a handle [effect-handle] =====

// [effect-handle] A bare `use` binds shareable with no annotation anywhere: a
// stateful handler as a monitor, satisfying a callee's bare `[E]`
// requirement.
fn draw() [Random] -> Int {
    return next()
}

fn a_bare_use_of_a_stateful_handler_is_shareable() [use] {
    use CyclicRandom(7)
    let _n = draw()
}

// [effect-handle] [effect-handler-deps] A dependency of any shape binds with
// a plain `use`: an actor-effect dependency and a generic effect instance
// included.
effect Beacon {
    fn shine() -> Int
}

handler ViaDrawer [Drawer] of Beacon {
    fn shine() -> Int {
        return waitfor got: Reply<Int> {
            draw(got)
        }
    }
}

effect Store<T> {
    fn keep(value: T) -> Int => value
}

handler MemStore of Store<Int> {
    fn keep(value: Int) -> Int => value {
        return 1
    }
}

handler ViaStore [Store<Int>] of Beacon {
    fn shine() -> Int {
        return keep(3)
    }
}

fn dependencies_of_every_shape_bind_with_a_plain_use() [use, spawn] {
    use CyclicRandom(7)
    let d = spawn Drawing() on pool(1)
    use d
    use ViaDrawer()
    use MemStore()
    use ViaStore()
}

// [effect-handle] [actor-monitor-handler] The generic monitor (user decision
// 2026-09-20): a stateful handler of a generic effect *instance* shares by
// default like any other.
handler Keeping of Store<Int> {
    state {
        total: Int = 0
    }

    fn keep(value: Int) -> Int => value {
        total = total + value
        return total
    }
}

fn stash() [Store<Int>] -> Int {
    return keep(2)
}

fn a_stateful_handler_of_a_generic_instance_shares() [use] {
    use Keeping()
    let _n = stash()
}

// [actor-spawn-inherit] The v1 lexical cut is **lifted** (user decision
// 2026-09-20): a shareable construction may capture a handle for an effect
// that arrived through the enclosing *signature*: the handle threads in from
// the caller. Interception wiring in a fn that received the effects it wires.
handler Relaying [Random] of Beacon {
    fn shine() -> Int {
        return next()
    }
}

fn wire() [Random, use] {
    use Relaying()
    let lit = shine()
    let _sink = lit
}

fn a_signature_supplied_dependency_can_be_captured() [use] {
    use CyclicRandom(7)
    wire()
}

// ===== spawn-inheritance and the `with` clause (2026-09-20) =====

// [actor-spawn-inherit] [effect-with-clause] A **partial** clause (user
// decision 2026-09-20): a written item satisfies the dependency it matches,
// and the rest still inherit from the scope. One child, two dependencies,
// one of each.
handler Shining of Beacon {
    fn shine() -> Int { return 1 }
}

actor effect Asker {
    send fn ask(out: Reply<Int>) => !out
}

handler Asking() [Random, Beacon] of Asker {
    mailbox { capacity: 4 }

    send fn ask(out: Reply<Int>) {
        send(out, next() + shine())
    }
}

fn a_partial_with_clause_inherits_the_rest() [use, spawn] {
    use CyclicRandom(7)
    let _a = spawn Asking() with Shining() on pool(1)
}

// [actor-spawn-inherit] Two compatible instances in scope is an
// **ambiguity**, not a guess: the clause is how the program says which one.
handler AnyKeeping<T> of Store<T> {
    fn keep(value: T) -> Int => value { return 1 }
}

actor effect StoreAsker {
    send fn ask_store(out: Reply<Int>) => !out
}

handler StoreAsking<T>() [Store<T>] of StoreAsker {
    mailbox { capacity: 4 }

    send fn ask_store(out: Reply<Int>) {
        send(out, 0)
    }
}

fn an_ambiguous_inherited_dependency_is_refused() [use, spawn] {
    use AnyKeeping<Int>()
    use AnyKeeping<Str>()
    //~ error[actor-spawn-inherit:ambiguous-dep]
    let _a = spawn StoreAsking() on pool(1)
}

// [effect-with-clause] A `with` item is a **private instance** (user decision
// 2026-09-20), and it may supply the **self-dependency**: an interceptor then
// wraps the instance the clause names instead of the one in scope.
effect Greeter {
    fn greet(name: Str) -> Str => name
}

handler Plain of Greeter {
    fn greet(name: Str) -> Str => name { return "plain" }
}

handler Formal of Greeter {
    fn greet(name: Str) -> Str => name { return "formal" }
}

handler Loud [Greeter] of Greeter {
    fn greet(name: Str) -> Str => name {
        return greet(name)
    }
}

fn a_with_clause_may_override_the_self_dep() [use] {
    use Plain
    use Loud with Formal()
    let said = greet("x")
    let _sink = said
}

// [effect-with-clause] An item the handler does not depend on is a mistake,
// and a clause item with dependencies *of its own* has no scope to resolve
// them: both named where they are written.
handler Watching [Beacon] of Beacon {
    fn shine() -> Int { return shine() }
}

fn a_with_clause_refuses_a_useless_or_dependent_item() [use] {
    use CyclicRandom(7)
    use Shining
    //~ error[effect-with-clause:has-dependencies]
    //~ error[effect-with-clause:not-a-dep]
    use Watching with Relaying(), Shining()
}

//--- lock-cycle
// [actor-monitor-handler] [actor-deadlock-cycle] Waits under a monitor's lock
// are priced, not refused (option (b), user decision 2026-09-20):
// dep-bearing shareable handlers get graph nodes, and a cycle through held
// locks is reported before it runs, naming the locks.
effect Ping {
    fn ping() -> Int
}

effect Pong {
    fn pong() -> Int
}

handler BasePing of Ping {
    fn ping() -> Int { return 1 }
}

handler BasePong of Pong {
    fn pong() -> Int { return 2 }
}

//~ error[actor-mixed-handler:occupancy-cycle]
handler Pinger [Pong] of Ping {
    state {
        a: Int = 0
    }
    fn ping() -> Int {
        a = pong()
        return a
    }
}

handler Ponger [Ping] of Pong {
    state {
        b: Int = 0
    }
    fn pong() -> Int {
        b = ping()
        return b
    }
}

fn main() [use] {
    use BasePing()
    use BasePong()
    use Pinger()
    use Ponger()
    let got = pong()
    let _sink = got
}

//--- stateless-holds-no-lock
// [effect-handle] [actor-deadlock-cycle] The same shape with **stateless**
// handlers holds no lock: a stateless handler's members run on the caller's
// thread with nothing shared held, so there is no `H's lock` node and no
// cycle to report (2026-09-28: the graph's lock nodes follow the same
// statefulness predicate the emitters key the handle's lock on).
effect Ping {
    fn ping() -> Int
}

effect Pong {
    fn pong() -> Int
}

handler BasePing of Ping {
    fn ping() -> Int { return 1 }
}

handler BasePong of Pong {
    fn pong() -> Int { return 2 }
}

handler Pinger [Pong] of Ping {
    fn ping() -> Int { return pong() + 1 }
}

handler Ponger [Ping] of Pong {
    fn pong() -> Int { return ping() + 1 }
}

fn main() [use] {
    use BasePing()
    use BasePong()
    use Pinger()
    use Ponger()
    let got = pong()
    let _sink = got
}
