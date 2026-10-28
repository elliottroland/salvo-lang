// [actor-mixed-handler] The mixed handler (SH-1, user decision 2026-09-19): a
// plain-effect handler whose `send fn` members own the state (the servant)
// and whose sync members run on the caller's thread (the façade), sending to
// the servant and waiting on its answers. The checker surface:
// classification, confinement, the façade-send resolution, the spawn-only
// rule, the mailbox requirement, the local-send-member obligations, the
// `defer` contract point and servant parking.

// The design's own motivating handler: `Random` backed by state that lives
// behind a thread boundary, reached through a plain effect.
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        cursor = (cursor * 31 + seed) % 100000
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

// The whole checker surface in one program: the declaration checks clean
// (local send member with its clause, the façade's send + `waitfor` with no
// effect declared for the wait, SH-5(d)), the spawn answers a handle, `use`
// binds it, and the plain effect's member is callable from ordinary code
// that declares nothing.
fn draw_twice() [Random] -> Int {
    return next() + next()
}

fn a_mixed_handler_checks_clean_end_to_end() [use, spawn] {
    let rng = spawn CyclicRandom(12345)
    use rng
    let _n = draw_twice()
}

// [actor-mixed-handler] Confinement: a sync member touching a state field
// gets the rule by name, not an "unknown variable".
effect Peek {
    fn peek() -> Int
}

handler Peeking() of Peek {
    mailbox { capacity: 2 }
    state {
        hidden: Int = 0
    }

    send fn poke() {
        hidden = hidden + 1
    }

    fn peek() -> Int {
        //~ error[actor-mixed-handler:servant-state-sync]
        return hidden
    }
}

// [actor-mixed-handler] Spawn-only: `use` of a mixed handler is refused where
// the binding is chosen, naming the reason and the spelling that works.
fn a_mixed_handler_cannot_be_use_bound() [use] {
    //~ error[actor-mixed-handler:use-bound]
    use CyclicRandom(1)
    let _n = next()
}

// [actor-mixed-handler] [actor-mailbox] Send members are delivered through a
// mailbox, so a mixed handler without the slot is told to add one.
//~ error[actor-mailbox:no-capacity]
handler NoBox() of Peek {
    state {
        tally: Int = 0
    }

    send fn poke() {
        tally = tally + 1
    }

    fn peek() -> Int {
        return waitfor got: Reply<Int> {
            ask(got)
        }
    }

    send fn ask(out: Reply<Int>) => !out {
        send(out, tally)
    }
}

// [actor-mixed-handler] [actor-free-send-fn] A handler-local send member
// carries the free send fn's obligations: the clause must be written out (a
// scheduled body consumes what it is given). [copy-scalar-free] A Copy scalar
// is never consumed, so the required clause names only `why`.
handler Sloppy() of Peek {
    mailbox { capacity: 2 }
    state {
        tally: Int = 0
    }

    //~ error[actor-mixed-handler:send-clause]
    send fn poke(n: Int, why: Str) {
        tally = tally + n
    }

    fn peek() -> Int {
        return 0
    }
}

// [actor-mixed-handler] The servant's dispatch is by member name, so
// overloading a local send member is refused for now.
handler Twice() of Peek {
    mailbox { capacity: 2 }
    state {
        tally: Int = 0
    }

    //~ error[actor-mixed-handler:overloaded-send]
    send fn poke(n: Int) {
        tally = tally + n
    }

    //~ error[actor-mixed-handler:overloaded-send]
    send fn poke(s: Str) => !s {
        tally = tally + 1
    }

    fn peek() -> Int {
        return 0
    }
}

// [actor-mixed-handler] [effect-handler-deps] A mixed handler's dependencies
// are the **servant's** (the cut lifted 2026-10-20, user decision R6): a send
// member calls them, and the spawn resolves them like any actor spawn's, from
// the spawning scope here [actor-spawn-inherit].
handler Noisy() [Console] of Peek {
    mailbox { capacity: 2 }
    state {
        tally: Int = 0
    }

    send fn poke() {
        tally = tally + 1
        println("poked")
    }

    fn peek() -> Int {
        poke()
        return 0
    }
}

fn a_mixed_handlers_dependencies_belong_to_the_servant() [Console, use, spawn] {
    let _p = spawn Noisy() on pool(1)
}

// [actor-mixed-handler] The confinement half: a sync member runs on the
// caller's thread over the façade value, which carries no dependency, so it
// cannot call one.
handler Leaky() [Console] of Peek {
    mailbox { capacity: 2 }

    send fn poke() {
        print("poked")
    }

    fn peek() -> Int {
        //~ error[actor-mixed-handler:sync-member-dependency]
        print("peeked")
        return 0
    }
}

// [actor-mixed-handler] [effect-at] The ambiguity arm: a bare name that is
// both a sibling `send fn` and a dependency's member must say which it means,
// and each `@` form resolves.
effect Tick {
    fn poke() -> None
}

handler Ambiguous() [Tick] of Peek {
    mailbox { capacity: 2 }

    send fn poke() {
    }

    send fn relay() {
        //~ error[actor-mixed-handler:send-or-member]
        poke()
    }

    fn peek() -> Int {
        return 0
    }
}

handler SaysSelf() [Tick] of Peek {
    mailbox { capacity: 2 }

    send fn poke() {
    }

    send fn relay() {
        poke@self()
    }

    fn peek() -> Int {
        return 0
    }
}

handler SaysTick() [Tick] of Peek {
    mailbox { capacity: 2 }

    send fn poke() {
    }

    send fn relay() {
        poke@Tick()
    }

    fn peek() -> Int {
        return 0
    }
}

// [actor-mixed-handler] One face for now: the façade value behind several
// effect types has no representation yet.
handler Wide(seed: Int) of Random, Peek {
    mailbox { capacity: 2 }
    state {
        cursor: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        cursor = cursor + seed
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }

    fn peek() -> Int {
        return 0
    }
}

fn a_multi_face_mixed_spawn_is_refused() [use, spawn] {
    //~ error[actor-mixed-handler:several-faces]
    let _both = spawn Wide(1) on pool(1)
}

// ===== [linear-defer] The rung-4 contract point =====

// [linear-defer] A reply that leaves the activation any way but the
// discharge must be declared `defer`: here by forwarding to another actor,
// and by passing it on. The diagnostic names the declaration.
actor effect Oracle {
    send fn divine(out: Reply<Int>) => !out
}

handler Forwarding(oracle: eff[Oracle]) of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        tally = tally + 1
        //~ error[actor-mixed-handler:undeclared-defer]
        oracle.divine(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn stash(keep: Reply<Int>) [] -> None => !keep {
    send(keep, 0)
}

handler Latch() of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        tally = tally + 1
        //~ error[actor-mixed-handler:undeclared-defer]
        stash(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

// And with the declaration, the escape is legal, plus the upper bound: a
// declared `defer` whose body answers in-frame anyway is fine (SH-10(b)).
handler DeclaredForwarding(oracle: eff[Oracle]) of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        oracle.divine(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

handler Prompt() of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        send(out, tally)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

// [actor-mixed-handler] [actor-self-send] A servant member reaches a sibling
// `send fn` by bare call (user decision 2026-09-19), a self-enqueue, and
// `@self` is the explicit spelling, in send members and in sync members
// alike. The whole chain checks clean.
handler Chain() of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn step(out: Reply<Int>) => defer out {
        tally = tally + 1
        relay(out)
    }

    send fn relay(out: Reply<Int>) => defer out {
        deliver@self(out)
    }

    send fn deliver(out: Reply<Int>) => !out {
        send(out, tally * 10)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            step@self(got)
        }
    }
}

// [linear-defer] A reply riding a bare sibling call's payload leaves the
// activation, so the escape hook fires exactly as it does for an addr send:
// the member must declare `defer`.
handler UndeclaredChain() of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn step(out: Reply<Int>) => !out {
        tally = tally + 1
        //~ error[actor-mixed-handler:undeclared-defer]
        relay(out)
    }

    send fn relay(out: Reply<Int>) => !out {
        send(out, tally * 10)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            step(got)
        }
    }
}

// The bare-call resolution reaches `send fn` siblings only: a sync member
// named bare from a send member stays unresolved, as sibling members always
// were outside this rule.
handler SyncChain() of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn step(out: Reply<Int>) => !out {
        //~ error[call-resolve:unknown-fn]
        tally = tally + glance()
        send(out, tally)
    }

    fn glance() -> Int {
        return 1
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            step(got)
        }
    }
}

// [linear-defer] [actor-replyto] Parking inside a mixed handler: a servant
// member may mint a continuation on another of its own send members,
// provided the captured reply is declared `defer`. The whole rung-4 shape
// checks clean.
handler Delphi() of Oracle {
    mailbox { capacity: 4 }
    state {
        n: Int = 0
    }

    send fn divine(out: Reply<Int>) {
        n = n + 7
        send(out, n)
    }
}

handler Parking(oracle: eff[Oracle]) of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        oracle.divine(replyto settled(out))
    }

    send fn settled(out: Reply<Int>, drawn: Int) => !out {
        send(out, drawn + tally)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

// [linear-defer] The capture hook still holds: parking a received reply in a
// continuation lets its answer outlive the activation, so it must be
// declared `defer`, and the diagnostic names the declaration.
handler UndeclaredParking(oracle: eff[Oracle]) of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        tally = tally + 1
        //~ error[actor-mixed-handler:undeclared-defer]
        oracle.divine(replyto settled(out))
    }

    send fn settled(out: Reply<Int>, drawn: Int) => !out {
        send(out, drawn + tally)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}
