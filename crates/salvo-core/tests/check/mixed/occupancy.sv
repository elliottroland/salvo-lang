// [actor-deadlock-cycle] [actor-mixed-handler] The occupancy edge (SH-4),
// inferred from declarations. The graph is program-wide, so every case is a
// program of its own over the shared `Random` mixed handler.

//--- servant-dependency-cycle
// [actor-deadlock-cycle] [actor-mixed-handler] A servant's dependency is an
// edge out of the servant's node: the occupancy cycle of
// `occupancy-through-back-pressure` below, with the servant reaching `Drawer`
// through its dependency list instead of a held handle. The cycle is the
// only error.
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

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Feedback() [Drawer] of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        tally = tally + 1
        draw(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

//~ error[actor-mixed-handler:occupancy-cycle]
handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

//--- occupancy-through-back-pressure
// [actor-deadlock-cycle] [actor-mixed-handler] SH-4: an actor handler
// depending on a plain effect with a mixed handler in the program may park on
// that servant, and the servant here sends back to the actor's own protocol:
// the design's upcall shape. The cycle mixes an occupancy edge with a
// back-pressure edge and is an **error**: the ungated-side downgrade does not
// apply, because an occupied activation serves nothing.
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

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Feedback(drawer: eff[Drawer]) of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        tally = tally + 1
        //~ error[actor-mixed-handler:undeclared-defer]
        drawer.draw(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

//~ error[actor-mixed-handler:occupancy-cycle]
handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

//--- acyclic-occupancy-edge
// The same dependency with no return path is an edge, not a cycle: nothing
// is reported.
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

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

fn main() [use, spawn] {
    let rng = spawn CyclicRandom(1)
    let drawer = spawn Drawing() with rng on pool(1)
    let _drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
}

//--- gated-servant-park
// [actor-deadlock-cycle] [linear-defer] A gated park in a mixed servant
// makes its sends Wait-kind, mirroring the actor logic: the servant serves
// nothing but its answer while the gate holds, so a peer whose handler
// occupies the servant's effect closes a reported cycle.
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

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Feedback(drawer: eff[Drawer]) of Random {
    mailbox { capacity: 4 }
    state {
        tally: Int = 0
    }

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        drawer.draw(replyto! settled(out))
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

//~ error[actor-mixed-handler:occupancy-cycle]
handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}
