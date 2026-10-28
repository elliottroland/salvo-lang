// [effect-handler-multi] Handlers of **several effects** — the checker's half
// (T-4(a), user decision 2026-09-17): one handler, one piece of state, one
// face per effect it implements.
//
// What is checked here is the accounting the feature needs: every member of
// every face implemented, one signature where one method implements two faces,
// the faces all of one kind and each named once, a `spawn` answering one addr
// per face, and a `use` binding all of them. The emission of all this lives in
// the backends' tests, where it is compiled and run.
//
// The two-faced pair T-4 is named for: a public protocol and an admin one,
// both `actor effect`s, plus a synchronous pair beside them.

actor effect Timer {
    send fn after(millis: Int, out: Reply<Str>) => !out
}

actor effect TimerCtl {
    send fn advance(millis: Int)
    send fn pending(out: Reply<Int>) => !out
}

effect Tally {
    fn bump(n: Int) -> None
}

effect Stats {
    fn total() -> Int
}

// The shape the step exists for: one actor, one mailbox, two typed faces, and
// a `spawn` that answers one addr per face — so a holder of the public face
// cannot name the admin one.
handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 8 }

    state {
        now: Int = 0
    }

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired at ${now}")
    }

    send fn advance(millis: Int) {
        now = now + millis
    }

    send fn pending(out: Reply<Int>) {
        out.send(now)
    }
}

fn main() [spawn] -> None {
    let (timer, ctl) = spawn ManualTime() on pool(1)
    let n = waitfor c: Reply<Int> { ctl.pending(c) }
    let s = waitfor f: Reply<Str> {
        timer.after(1, f)
        ctl.advance(1)
    }
    discard(n)
    discard(s)
}

// The addrs are typed by their own face, which is where least authority comes
// from: the public one cannot reach the admin protocol's members ("no member
// named `advance`").
fn public_face_only() [spawn] -> None {
    let (timer, _ctl) = spawn ManualTime() on pool(1)
    //~ error[actor-use-addr:no-member]
    timer.advance(1)
}

// A single face still answers a bare `eff[E]`: the tuple is what *several*
// faces produce, so nothing about the one-face spawn changes. Asserted by
// *using* the value as an addr, since a tuple has no members to send to.
handler Ticking() of Timer {
    mailbox { capacity: 4 }

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired")
    }
}

fn one_face() [spawn] -> None {
    let timer = spawn Ticking() on pool(1)
    let s = waitfor f: Reply<Str> { timer.after(1, f) }
    discard(s)
}

// Conformance: a face whose member nobody implements is an error *here*,
// naming the member ("does not implement `TimerCtl.pending(Reply<Int>)`") —
// it used to be the target compiler's missing-trait-method complaint, which
// put a Salvo mistake in a foreign diagnostic.
//~ error[effect-handler-multi:missing-member]
handler Half() of Timer, TimerCtl {
    mailbox { capacity: 4 }

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired")
    }

    send fn advance(millis: Int) {}
}

// The faces must be of one **kind** ("implements effects of two kinds"): a
// handler is bound one way or the other, and a handler mixing an `actor
// effect` with a plain one is refused — the mixed handler
// [actor-mixed-handler] is a different construct, classified by shape, with
// every face plain.
//~ error[actor-mixed-handler:two-kinds]
handler Mixed() of Timer, Stats {
    mailbox { capacity: 4 }

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired")
    }

    fn total() -> Int {
        return 0
    }
}

// Naming a face twice says nothing new and would make a `spawn` answer the
// same addr twice ("already implements `Stats`").
//~ error[effect-handler-multi:face-twice]
handler Twice() of Stats, Stats {
    fn total() -> Int {
        return 0
    }
}

// A `use` binds **every** face, which is what makes the synchronous
// public-face/admin-face pair writable: one instance, two effect lists that
// reach it.
handler Counting() of Tally, Stats {
    state {
        sum: Int = 0
    }

    fn bump(n: Int) {
        sum = sum + n
    }

    fn total() -> Int {
        return sum
    }
}

fn work() [Tally, Stats] -> Int {
    bump(2)
    return total()
}

fn bound_by_one_use() [use] -> None {
    use Counting()
    discard(work())
}

// A multi-face handler **constructed in a spawn's `with` clause** is refused
// ("implements several effects", "spawn's `with` clause"): the child owns
// what a clause builds, and one instance cannot be two of its dependencies.
// Two addrs of the same actor are the shape that works.
actor effect Worker {
    send fn go(n: Int) => n
}

handler Working() [Tally] of Worker {
    mailbox { capacity: 4 }

    send fn go(n: Int) {
        bump(n)
    }
}

fn built_in_a_clause() [spawn] -> None {
    //~ error[actor-spawn-expr:with-several-faces]
    let w = spawn Working() with Counting() on pool(1)
    discard(w)
}

// One method may implement a same-named member of two faces — when the
// signatures are the same, which is the only case in which "one method
// implements both" means anything.
effect Health {
    fn status() -> Str
}

effect Admin {
    fn status() -> Str
}

handler Serving() of Health, Admin {
    fn status() -> Str {
        return "ok"
    }
}

// Different *parameters* are what overloading distinguishes, so a handler may
// implement both with two members.
effect Ping {
    fn ping(n: Int) -> Str => n
}

effect Pong {
    fn ping(m: Str) -> Str => !m
}

handler Both() of Ping, Pong {
    fn ping(n: Int) -> Str {
        return "int"
    }

    fn ping(m: Str) -> Str
    => !m {
        return m
    }
}

//--- indistinguishable members
// …and is refused where overloading cannot tell them apart ("whose
// signatures differ in what overloading cannot see"): same parameters,
// different return type, so no method can answer for both (T-4(a)'s
// refinement, and the one case the decision names as rejected).
effect Health {
    fn status() -> Str
}

effect Admin {
    fn status() -> Int
}

handler Serving() of Health, Admin {
    //~ error[effect-handler-multi:member-conflict]
    fn status() -> Str {
        return "ok"
    }
}
