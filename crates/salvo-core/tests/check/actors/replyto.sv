// [actor-replyto] The mint: what a `replyto` may target, how its captures
// and its token's type are read off the target, and which handlers may then
// be bound with `use`.

actor effect Counter {
    send fn bump(n: Int)
    send fn total(out: Reply<Int>) => !out
}

actor effect Log {
    send fn note(what: Str) => !what
}

handler Printing() of Log {
    mailbox { capacity: 4 }

    send fn note(what: Str) {}
}

handler Counting() [Log] of Counter {
    mailbox { capacity: 16 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
        note("bumped")
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

// `replyto` targets a member of the handler it is written in, so it is
// illegal outside one, and the diagnostic names `main`'s alternative
// (`waitfor`).
fn replyto_is_illegal_outside_a_handler() [use, spawn] {
    //~ error[actor-replyto:outside-member]
    let _r = replyto somewhere()
}

// [task-mint] A `replyto` whose target exists as a free `fn`, just not a
// `send fn`, gets a diagnostic naming that fix specifically, rather than
// falling through to "no member" (inside a handler) or "legal only inside a
// handler member" (anywhere else): both are true but neither says what to
// change.
actor effect Describe {
    send fn go(out: Reply<Int>) => !out
}

handler Describing() [Counter] of Describe {
    mailbox { capacity: 1 }

    send fn go(out: Reply<Int>) {
        //~ error[task-mint:plain-fn-target]
        go@self(replyto describe(1))
    }
}

fn describe(n: Int) -> Str {
    return "n"
}

fn a_plain_fn_target_outside_any_handler() [use, spawn] {
    //~ error[task-mint:plain-fn-target]
    let _r = replyto describe(1)
}

// The target member's parameters are the captures *plus one*: the trailing
// parameter is the answer the token carries, which is what types the
// `Reply<T>`. Writing the wrong number of captures says how many are wanted.
actor effect Capture {
    send fn go()
    send fn arrived(id: Int, sum: Int)
}

handler Capturing() [Counter] of Capture {
    mailbox { capacity: 1 }

    send fn go() {
        //~ error[actor-replyto:member-capture-count]
        total(replyto arrived())
    }

    send fn arrived(id: Int, sum: Int) {}
}

// And with the captures written, the token's type comes from that trailing
// parameter: `arrived(id: Int, sum: Int)` minted with one capture is a
// `Reply<Int>`, which is what `total` takes.
actor effect Typed {
    send fn go()
    send fn arrived(id: Int, sum: Int)
    send fn wrong(id: Int, text: Str) => !text
}

handler Typing() [Counter] of Typed {
    mailbox { capacity: 1 }

    send fn go() {
        total(replyto arrived(7))
    }

    send fn arrived(id: Int, sum: Int) {}

    send fn wrong(id: Int, text: Str) {}
}

actor effect Mistyped {
    send fn go()
    send fn wrong(id: Int, text: Str) => !text
}

handler Mistyping() [Counter] of Mistyped {
    mailbox { capacity: 1 }

    send fn go() {
        //~ error[effect-member-call:arg-mismatch]
        total(replyto wrong(7))
    }

    send fn wrong(id: Int, text: Str) {}
}

// [actor-replyto] The asynchronous forms stop at a **closure**: a
// continuation belongs to the handler that minted it, and a lambda is not
// one.
actor effect Lambda {
    send fn go()
    send fn arrived(sum: Int)
}

handler Lambdaing() [Counter] of Lambda {
    mailbox { capacity: 1 }

    send fn go() {
        //~ error[actor-replyto:outside-member]
        let _f = () -> total(replyto arrived())
    }

    send fn arrived(sum: Int) {}
}

// [actor-replyto] A handler that **parks** may only be spawned (user decision
// 2026-09-15, D5-c). Bound with `use`, its member bodies run inline on the
// caller's thread: the mint would target a member of a *local* instance,
// which has no mailbox for the answer to arrive on and no dispatcher to run
// it, so the continuation would silently never run.
//
// The gate is syntactic per *handler*, not per member, because effects
// propagate: a fn declaring `[Counter]` may call any member, so a `use` site
// cannot know which ones a scope will reach.
actor effect Waiting {
    send fn ask()
    send fn got(n: Int)
}

handler Parking() [Counter] of Waiting {
    mailbox { capacity: 1 }

    send fn ask() {
        total(replyto got())
    }
    send fn got(n: Int) {}
}

fn a_parking_handler_may_not_be_used_synchronously() [use, spawn] {
    let c = spawn Counting() with Printing() on pool(1)
    use c
    //~ error[actor-replyto:use-bound]
    use Parking()
    ask()
}

// [actor-replyto] Which handlers park is read off their text before any body
// is checked, so a `use` written *before* the handler is refused in the one
// walk that checks it.
actor effect LaterWaiting {
    send fn ask_later()
    send fn got_later(n: Int)
}

fn a_parking_handler_declared_after_its_use_is_refused() [use, spawn] {
    let c = spawn Counting() with Printing() on pool(1)
    use c
    //~ error[actor-replyto:use-bound]
    use LaterParking()
    ask_later()
}

handler LaterParking() [Counter] of LaterWaiting {
    mailbox { capacity: 1 }

    send fn ask_later() {
        total(replyto got_later())
    }
    send fn got_later(n: Int) {}
}

// …and spawning it is fine, which is what makes the refusal a *binding* rule
// rather than a restriction on the form.
fn a_parking_handler_may_be_spawned() [use, spawn] {
    let c = spawn Counting() with Printing() on pool(1)
    let a = spawn Parking() with c on pool(1)
    a.ask()
}

// [actor-replyto] A **remote** mint, `k` naming a member of another `actor
// effect` in scope rather than of the enclosing handler, is the generalized
// form and a later slice: it makes the mint itself send-like, since capacity
// must be reserved in the *target's* queue. Named, with the workaround that
// needs nothing new: the handler that owns `k` mints the token and passes it.
actor effect Remote {
    send fn ask_remote()
}

handler RemoteAsking() [Counter] of Remote {
    mailbox { capacity: 1 }

    send fn ask_remote() {
        //~ error[actor-replyto:other-actor-member]
        total(replyto bump())
    }
}

fn a_remote_mint_target_is_refused_by_name() [use, spawn] {
    let c = spawn Counting() with Printing() on pool(1)
    let a = spawn RemoteAsking() with c on pool(1)
    a.ask_remote()
}
