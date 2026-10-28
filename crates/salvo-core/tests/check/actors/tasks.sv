// [actor-free-send-fn] [task-mint] The task kernel: a free `send fn` runs by
// being scheduled, and `replyto` toward one mints a detached task.

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
// [actor-free-send-fn] A **free `send fn`** is the send kind extended to a
// function (user decision 2026-09-17, FC-1(a)): it runs by being scheduled,
// answers nothing, and the refusal list is checked at the declaration, the
// same list an `actor effect`'s members carry, because the reason is the
// same.
send fn finish(label: Str, out: Reply<Int>, total: Int) => !label, !out {
    out.send(total)
}

//~ error[actor-send-fn:return-type]
send fn finish_returning(out: Reply<Int>, total: Int) -> Int => !out {
    out.send(total)
    return 1
}

// A scheduled body outlives the frame that minted it.
//~ error[actor-free-send-fn:kept-param]
send fn finish_keeping(label: Str, out: Reply<Int>, total: Int) => label, !out {
    out.send(total)
}

// Mutating across the boundary would share.
//~ error[actor-free-send-fn:mut-param]
send fn finish_mutating(xs: Mut List<Int>, out: Reply<Int>, total: Int) => !xs, !out {
    out.send(total)
}

// [task-effects] An *ordinary* effect is not refused here: it is inherited
// from the frame that mints (2026-09-26). What stays refused is the pair that
// belongs to a frame rather than to a body: a task has no scope to register
// a handler in.
//~ error[actor-free-send-fn:effect]
send fn finish_using(out: Reply<Int>, total: Int) [use] => !out {
    out.send(total)
}

// A mint carries no type arguments.
//~ error[actor-free-send-fn:generic]
send fn finish_generic<T>(out: Reply<T>, value: T) => !out, !value {
    out.send(value)
}

// [task-effects] A task body's effects are **inherited from the frame that
// mints it** (user decision 2026-09-26): the mint resolves them against the
// minting scope, exactly as a `spawn` resolves a synthesized dependency
// [actor-spawn-inherit]. What the checker owes is the failure case, because
// the remedy differs.
effect Notes {
    fn note(m: Str) -> None => m
}

handler Quiet() of Notes {
    fn note(m: Str) -> None => m {
    }
}

send fn finish_noting(label: Str, total: Int) [Notes] => !label {
    note(label)
}

// Bound in the minting scope: inherited, and clean.
fn a_bound_handler_is_inherited() [use] -> None {
    use Quiet()
    let k = replyto finish_noting("done")
    k.send(1)
}

// Nothing in scope: the mint says so, and names the remedies.
fn the_mint_is_where_the_handler_has_to_be() [] -> None {
    //~ error[task-effects:missing]
    let k = replyto finish_noting("done")
    k.send(1)
}

// [actor-free-send-fn] It runs by being **scheduled**, so it is neither
// callable nor a value: a call would run it in this frame on this thread,
// which is the callback anti-pattern the kind exists to refuse.
fn a_free_send_fn_is_not_called(out: Reply<Int>) [] -> None => !out {
    //~ error[actor-free-send-fn:called]
    finish("x", out, 1)
}

fn a_free_send_fn_is_not_a_value() [] -> None {
    //~ error[actor-free-send-fn:not-a-value]
    let f = finish
    discard(f)
}

// [task-mint] The mint that makes a free function a citizen of the
// concurrent world (FC-2): `replyto` may target a free `send fn`, and the
// mint is then legal in **any** function: the answer needs no mailbox,
// because the continuation is a detached task.
send fn finish_labelled(label: Str, out: Reply<Str>, total: Int) => !label, !out {
    out.send("${label}=${total}")
}

fn fetch_labelled(out: Reply<Str>) [Counter] -> None => !out {
    total(replyto finish_labelled("count", out))
}

// The trailing parameter is the answer, the rest are captures.
fn fetch_short(out: Reply<Str>) [Counter] -> None => !out {
    //~ error[actor-replyto:capture-count]
    total(replyto finish_labelled(out))
}

// [task-mint] A **gate** is a mailbox policy, and a task has no mailbox:
// there is nothing to hold back while the answer is outstanding.
send fn finish_plain(out: Reply<Int>, total: Int) => !out {
    out.send(total)
}

fn fetch_gated(out: Reply<Int>) [Counter] -> None => !out {
    //~ error[task-mint:gated-free-send]
    total(replyto! finish_plain(out))
}

// [task-pool-inherit] Placement: `on POOL` is optional at a mint and
// defaults to the pool current where the mint runs (FC-3: whoever creates
// work pays for it). A **member** mint takes no `on` clause at all: the
// answer arrives on the actor's own mailbox.
fn fetch_inherited(out: Reply<Int>) [Counter] -> None => !out {
    total(replyto finish_plain(out))
}

fn fetch_placed(out: Reply<Int>) [Counter, spawn] -> None => !out {
    total(replyto finish_plain(out) on pool(2))
}

actor effect Audit {
    send fn audit(what: Str) => !what
}

handler Fetching() [Counter] of Audit {
    mailbox { capacity: 2 }

    send fn audit(what: Str) {
        //~ error[actor-replyto:own-member-on]
        //~ error[actor-spawn-effect:callee-needs-spawn]
        total(replyto arrived(what) on pool(1))
    }
    send fn arrived(what: Str, sum: Int) => !what {
        discard(what)
        discard(sum)
    }
}

// A task whose body waits needs no placement either (SH-5(d)): the wait
// serves the pool it runs on [waitfor-pump], and `on thread()` remains as a
// choice, not a grant.
send fn slow(out: Reply<Int>, total: Int) => !out {
    out.send(total)
}

fn fetch_slow(out: Reply<Int>) [Counter] -> None => !out {
    total(replyto slow(out))
}

fn fetch_slow_on_a_thread(out: Reply<Int>) [Counter, spawn] -> None => !out {
    total(replyto slow(out) on thread())
}
