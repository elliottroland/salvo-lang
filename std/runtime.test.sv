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
    let pool = new_pool_of(2)
    let next = 0
    let body = body_of(msg -> {
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
    let pool = new_pool_of(1)
    let fragile = spawn_body(copy(pool), 2, body_of(msg -> {
        let _n: Int = unerase(msg)
        let xs: List<Int> = []
        let _boom = get(xs, 0)!
    }))
    let sturdy = spawn_body(pool, 2, body_of(msg -> {
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
