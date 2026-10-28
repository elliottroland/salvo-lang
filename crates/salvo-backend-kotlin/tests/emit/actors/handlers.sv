// Actors [kt-actor]: message classes, actor bodies, monitors and mixed
// handlers.

//--- an_actor_lowers_to_message_classes_and_a_body
// [kt-actor] What the lowering *is*, asserted on the generated text: a
// sealed message class per protocol, an actor class wrapping the handler,
// and the scheduler file carried into the output.
// [actor-spawn-expr] [actor-use-addr] [actor-waitfor] The same program the
// Rust backend runs (`ACTOR` in codegen_tests.rs).
//~ emits: sealed class __Msg_Counter {
//~ emits(scheduler.kt):
actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

//~ emits: class __Actor_Counting(private val handler: Counting) : salvo.SalvoActor
handler Counting() of Counter {
    mailbox { capacity: 8 }

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
    //~ emits: salvo.SalvoSched.spawn(
    let counter = spawn Counting() on pool(1)
    counter.bump(2)
    counter.bump(3)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    println("sum ${sum}")
}

//--- a_monitor_lowers_to_a_lock_wrapper
// [kt-monitor] What the monitor lowering *is*, asserted on the generated
// text: the per-effect lock wrapper implementing the effect's interface by
// locking its `ReentrantLock` and delegating, the spawn wrapping the handler instance,
// and no send stub for a plain effect.
// [actor-monitor-handler] [kt-monitor] The monitor spawn (SH-3, user decision
// 2026-09-19), `MONITOR` in codegen_tests.rs.
//~ emits: class __Mon_Random(
//~|     private val inner: Random,
//~ emits: lock.lock()
//~|         try { return inner.next() } finally { lock.unlock() }
//~ !emits: __Stub_Random
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

fn main() [use, spawn] -> None {
    use StdConsole()
    //~ emits: __Mon_Random(CyclicRandom(seed = 12345))
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng on pool(1)
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    println("actor drew ${drawn}")
    println("main drew ${next()}")
}

//--- a_mixed_handler_with_dependencies
// [actor-mixed-handler] [effect-handler-deps] [actor-spawn-inherit] [effect-with-clause] A
// mixed handler **with dependencies** (the cut lifted 2026-10-20): the
// servant's send member calls `Console`, synthesized from the spawning
// scope, and `Label`, built by the `with` clause; the façade carries
// neither. Same program, same output on both backends.
effect Random {
    fn next() -> Int
}

effect Label {
    fn label(n: Int) -> Str
}

handler Hash() of Label {
    fn label(n: Int) -> Str {
        return "#${n}"
    }
}

// [kt-mixed] The servant class takes the dependencies; the façade carries
// only the addr and the constructor parameters.
//~ emits: class Counted(private val seed: Int, private val __dep0: 
//~ emits: class __Fac_Counted(private val __addr: Int, private val seed: Int) : Random
handler Counted(seed: Int) [Console, Label] of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        cursor = cursor + seed
        println("servant at ${label(cursor)}")
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    //~ emits: __Fac_Counted(__a, __c0)
    let rng = spawn Counted(7) with Hash()
    use rng
    println("main drew ${next()}")
    println("main drew ${next()}")
}

//--- a_mixed_handler_lowers_to_a_servant_and_a_facade
// [kt-mixed] What the mixed lowering *is*: the handler-keyed message class
// and actor body (the servant), the façade class carrying the addr and ctor
// params, the façade send, and the spawn answering the façade.
// [actor-mixed-handler] [kt-mixed] The mixed handler (SH-1), `MIXED` in
// codegen_tests.rs.
effect Random {
    fn next() -> Int
}

//~ emits: sealed class __Priv_CyclicRandom {
//~ emits: class __Actor_CyclicRandom(private val handler: CyclicRandom) : salvo.SalvoActor
//~ emits: class __Fac_CyclicRandom(private val __addr: Int, private val seed: Int) : Random {
//~ emits: salvo.SalvoSched.send(__addr, __Priv_CyclicRandom.Advance(
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

fn main() [use, spawn] -> None {
    use StdConsole()
    //~ emits: __Fac_CyclicRandom(__a, __c0)
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng on pool(1)
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    println("actor drew ${drawn}")
    println("main drew ${next()}")
}
