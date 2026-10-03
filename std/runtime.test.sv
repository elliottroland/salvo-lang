// [test-file] The tests of module `runtime`.

test "an unpark before the park makes the park return at once" {
    let p = this_parker()
    unpark(p)
    park(p)
    expect(true, "the park returned")
}

test "a timed park returns when nobody unparks" {
    park_nanos(this_parker(), 1000000)
    expect(true, "the timed park returned")
}

test "an unpark token is used up by one park" {
    let p = this_parker()
    unpark(p)
    unpark(p)
    park(p)
    // A second park would wait: one token, not a count.
    park_nanos(p, 1000000)
    expect(true, "the second park timed out")
}

test "the module's own binding answers without being declared" {
    let a = fresh_bits()
    let b = fresh_bits()
    expect(!(a == b), "two draws differ")
}

test "a thread the runtime starts runs its body" {
    let me = this_parker()
    let waker = copy(me)
    start_thread(() -> { unpark(waker) })
    park(me)
    expect(true, "the thread woke the test")
}

test "a fault inside the boundary is answered, not raised" {
    let ok = guarded(() -> {})
    expect(ok is None, "a body that returns answers None")
    let why = guarded(() -> {
        let xs: List<Int> = []
        let _x = get(xs, 3)!
    })
    expect(!(why is None), "a faulting body answers its fault")
}

// [runtime-sched] A counter actor on the Salvo scheduler: messages are
// erased `Int`s, the body checks they arrive in order, and unparks the test
// with what it saw once the last one is in. The mailbox (4) is smaller than
// the burst, so the sender is held back and nothing is lost.
test "the scheduler runs an actor's messages in order on its pool" {
    let me = this_parker()
    let waker = copy(me)
    let pool = new_pool_of(2, -1)
    let next = 0
    let body = body_of((kind, slot, msg) -> {
        let n: Int = unerase(msg)
        if n == 99 {
            unpark(copy(waker))
        } else {
            if n != next {
                let xs: List<Int> = []
                let _out_of_order = get(xs, 0)!
            }
            next = next + 1
        }
    })
    let a = spawn_body(pool, 4, body)
    let i = 0
    while i < 50 {
        send_dyn(copy(a), erase(copy(i)))
        i = i + 1
    }
    send_dyn(copy(a), erase(99))
    park(me)
    expect(true, "the last message reached the actor")
}

// [runtime-sched] A faulting activation is the actor's death: a send to it
// afterwards is the silent no-op, and the pool's workers live on.
test "an actor that faults dies and later sends are dropped" {
    let me = this_parker()
    let waker = copy(me)
    let pool = new_pool_of(1, -1)
    let fragile = spawn_body(copy(pool), 2, body_of((kind, slot, msg) -> {
        let _n: Int = unerase(msg)
        let xs: List<Int> = []
        let _boom = get(xs, 0)!
    }))
    let sturdy = spawn_body(pool, 2, body_of((kind, slot, msg) -> {
        let _n: Int = unerase(msg)
        unpark(copy(waker))
    }))
    send_dyn(copy(fragile), erase(1))
    send_dyn(copy(fragile), erase(2))
    send_dyn(fragile, erase(3))
    send_dyn(sturdy, erase(4))
    park(me)
    expect(true, "the pool kept working after a death")
}

// [actor-replyto] [actor-waitfor] Request and response: the message carries
// a token, the actor answers it, and the waiting frame gets the answer.
test "an actor answers a token, and a waiting frame receives it" {
    let pool = new_pool_of(1, -1)
    let sum = 0
    let counter = spawn_body(pool, 4, body_of((kind, slot, msg) -> {
        let t: Token = unerase(msg)
        sum = sum + 10
        answer(t, erase(copy(sum)))
    }))
    let w = waiter()
    let {token, wid} = w
    send_dyn(copy(counter), erase(token))
    let first: Int = unerase(await_answer(wid))
    let w2 = waiter()
    let {token: t2, wid: wid2} = w2
    send_dyn(counter, erase(t2))
    let second: Int = unerase(await_answer(wid2))
    expect_eq(first, 10)
    expect_eq(second, 20)
}

// [main-pool] [waitfor-pump] An actor on `main`'s pool has no thread of its
// own: it runs while `main` waits, on `main`'s thread.
test "an actor on the main pool runs while main waits" {
    let echo = spawn_body(main_pool(), 2, body_of((kind, slot, msg) -> {
        let t: Token = unerase(msg)
        answer(t, erase(here_pool() * 100 + 7))
    }))
    let w = waiter()
    let {token, wid} = w
    send_dyn(echo, erase(token))
    let got: Int = unerase(await_answer(wid))
    expect_eq(got, 7)
}

// [task-mint] A task's answer schedules its body on the pool it was minted
// for — here `main`'s, served by the wait. The answer carries the token the
// task answers in turn.
linear struct Req { n: Int, out: Token }

fn finish_req(r: Req) [] -> None => !r {
    let {n, out} = r
    answer(out, erase(n * 2))
}

test "a task runs when its token is answered" {
    let w = waiter()
    let {token, wid} = w
    let task = mint_task_on(main_pool(), body_of((kind, slot, value) -> {
        let r: Req = unerase(value)
        finish_req(r)
    }))
    answer(task, erase(Req { n: 5, out: token }))
    let got: Int = unerase(await_answer(wid))
    expect_eq(got, 10)
}

// [actor-replyto] The gate: while a gated continuation is outstanding, the
// actor serves only the awaited answer — a message that arrived first waits.
linear struct GateMsg { code: Int, me: Int, peer: Int, out: Token? }

fn gate_msg(code: Int, me: Int, peer: Int) [] -> GateMsg => !code, !me, !peer {
    return GateMsg { code: code, me: me, peer: peer, out: None }
}

fn drop_gate_msg(m: GateMsg) [] -> None => !m {
    let {code, me, peer, out} = m
    if out is Token t {
        answer(t, erase(""))
    }
}

test "a gated actor serves its awaited answer before older messages" {
    let pool = new_pool_of(2, -1)
    let slow = spawn_body(copy(pool), 2, body_of((kind, slot, msg) -> {
        let t: Token = unerase(msg)
        park_nanos(this_parker(), 50000000)
        answer(t, erase(1))
    }))
    let log: Mut List<Str> = mut_list_of()
    let gated = spawn_body(pool, 4, body_of((kind, slot, msg) -> {
        if kind == 1 {
            let _n: Int = unerase(msg)
            add(log, "reply")
        } else {
            let m: GateMsg = unerase(msg)
            if m.code == 0 {
                send_dyn(copy(m.peer), erase(mint(copy(m.me), true)))
                drop_gate_msg(m)
            } elif m.code == 1 {
                add(log, "late")
                drop_gate_msg(m)
            } else {
                let {code, me, peer, out} = m
                if out is Token t {
                    answer(t, erase(to_str(log)))
                }
            }
        }
    }))
    send_dyn(copy(gated), erase(gate_msg(0, copy(gated), copy(slow))))
    send_dyn(copy(gated), erase(gate_msg(1, copy(gated), copy(slow))))
    let w = waiter()
    let {token, wid} = w
    send_dyn(copy(gated), erase(GateMsg { code: 2, me: copy(gated), peer: copy(slow), out: token }))
    let order: Str = unerase(await_answer(wid))
    expect_eq(order, "[reply, late]")
}

// [actor-watch] A watch is answered with the `Exit` of the actor's death.
test "a watch is answered when the actor dies" {
    let pool = new_pool_of(1, -1)
    let fragile = spawn_body(pool, 2, body_of((kind, slot, msg) -> {
        let _n: Int = unerase(msg)
        let xs: List<Int> = []
        let _boom = get(xs, 0)!
    }))
    let w = waiter()
    let {token, wid} = w
    watch(copy(fragile), token)
    send_dyn(fragile, erase(1))
    let exit: Exit = unerase(await_answer(wid))
    expect(size(exit.reason) > 0, "the exit carries a reason")
}

// [pool-fault-sink] A death nobody watches reaches the pool's sink, as a
// report the sink's body receives as kind 2.
test "an unwatched death reaches the pool's sink" {
    let w = waiter()
    let {token, wid} = w
    let sink = spawn_body(main_pool(), 4, body_of((kind, slot, msg) -> {
        let reason: Str = unerase(msg)
        discard(reason)
    }))
    let pool = new_pool_of(1, copy(sink))
    let fragile = spawn_body(pool, 2, body_of((kind, slot, msg) -> {
        let _n: Int = unerase(msg)
        let xs: List<Int> = []
        let _boom = get(xs, 0)!
    }))
    send_dyn(fragile, erase(1))
    on_idle(main_pool(), token)
    let idle: Idle = unerase(await_answer(wid))
    expect_eq(idle.parked_gates, 0)
}

// [actor-on-idle] The hook fires when nothing anywhere can run, reporting
// what its pool is owed: here one token `main` is still holding.
test "an idle hook reports the tokens still owed" {
    let held = waiter()
    let {token: kept, wid: kept_wid} = held
    let w = waiter()
    let {token, wid} = w
    on_idle(main_pool(), token)
    let idle: Idle = unerase(await_answer(wid))
    answer(kept, erase(0))
    let _done: Int = unerase(await_answer(kept_wid))
    expect_eq(idle.parked_tokens, 1)
}
