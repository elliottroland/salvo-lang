// Tasks and stubs: how a minted task, its inherited effects, and a `use addr`
// stub lower [rs-task].

//--- inherited_effect_is_owned
// [task-effects] A task body's effects are **inherited from the frame that
// minted it** (user decision 2026-09-26). Two shapes in one program, because
// they exercise different halves: a task minted from a frame that *binds* the
// handler, and a task that itself mints a task on another pool — which is the
// case that needs the inherited handle to be *owned*, since the second closure
// has to hold one too.
//
// [task-effects] [rs-task] The lowering: the inherited handle is an **owned**
// handle bound outside the closure and cloned inside it, so nothing borrows
// the minting frame — and a task's own effect parameters are owned handles for
// the same reason, which is what lets `chain` mint `report`.

// A task body takes the handle like any fn.
//~ emits: pub fn report(console: &crate::std_core_console::Console
send fn report(out: Reply<Int>, n: Int) [Console] => !out, !n {
    println("task saw ${n}")
    out.send(n * 2)
}

send fn chain(out: Reply<Int>, n: Int) [Console] => !out, !n {
    println("chain saw ${n}")
    let inner = waitfor r: Reply<Int> {
        // The mint clones the handle outside the closure, which owns the clone.
        //~ emits: let __e0 = console.clone();
        //~ emits: std::boxed::Box::new(move |__v| crate::report(&__e0, 
        // And nothing borrowed the frame for it.
        //~ !emits: report(&mut console
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

//--- task_mint_is_a_scheduled_closure
// [actor-free-send-fn] [task-mint] [task-pool-inherit] The **task kernel**, as a
// program: a free `send fn` that runs by being scheduled, an ordinary
// function that mints a continuation for it and returns (no park, no block),
// and the placement — inherited at one mint, written at the other. The
// Kotlin backend asserts the same output.
//
// [rs-task] The lowering: no continuation enum, no slot, no parked table —
// the closure *is* the continuation, and the pool is the one the mint chose.
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
    // An omitted `on` clause inherits the current pool.
    //~ emits: crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool()
    // The continuation is a moved one-shot closure.
    //~ emits: std::boxed::Box::new(move |__v| crate::finish(__c0, __c1, 
    // A task mint parks nothing: the closure is the continuation.
    //~ !emits: __parked.insert
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

//--- a_stub_implements_the_effect_by_sending
// [actor-use-addr] `use addr` binds an effect to a **stub** that sends to a
// process, so a function declaring `[Log]` never learns that its capability is
// a process — the point of the form, and the shape a dependent spawn will
// reuse. Same program and same expected output on the Kotlin backend.

// The forwarding stub.
//~ emits: pub struct __Stub_Log {
//~ emits: impl crate::__Stateless_Log for __Stub_Log
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
    // `use addr` builds the stub.
    //~ emits: __Stub_Log::new(
    use logger
    work()
    let n = waitfor out: Reply<Int> {
        count(out)
    }
    println("noted ${n}")
}
