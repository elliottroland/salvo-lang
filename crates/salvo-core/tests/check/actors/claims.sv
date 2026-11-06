// [effect-claim] Delivery guarantees are claims: provenance qualifiers
// declared `for Effect`, written before the effect. A handler makes them on
// its face, a function asks for them in its effect list, a handle carries
// them, and a bare `[E]` asks nothing.
// [actor-router] A router takes the messages with `receive` and hands each
// to one of two members: the deadlock graph gives it a node of its own, so
// `Counter → Counter` is not reported.

actor effect Counter {
    send fn bump(n: Int)
    send fn total(out: Reply<Int>) => !out
}

actor effect Log {
    send fn note(what: Str) => !what
}

handler Printing() for Log {
    mailbox { capacity: 4 }

    send fn note(what: Str) {}
}

handler Counting() [Log] for Ordered ShardKeyOrdered Counter {
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

handler Spraying(a: eff[Counter], b: eff[Counter]) for Counter {
    mailbox { capacity: 16 }
    state {
        flip: Bool = false
    }
    send fn receive(m: Message<Counter>) => !m {
        flip = !flip
        if flip { forward(a, m) } else { forward(b, m) }
    }
}

// Adds, then reads the sum: right only through one ordered instance.
fn tally() [Ordered Counter] -> Int {
    bump(1)
    bump(2)
    return waitfor out: Reply<Int> { total(out) }
}

fn poke() [Counter] -> None {
    bump(3)
}

// A router claims nothing, so it satisfies `[Counter]` and not `[Ordered
// Counter]`; a handler written `for Ordered Counter` satisfies both.
fn a_router_binding_satisfies_only_bare_requirements() [use, spawn] {
    let p = pool(2)
    let log = spawn Printing() on p
    use log
    let a = spawn Counting() on p
    let b = spawn Counting() on p
    use Spraying(a, b)
    poke()
    //~ error[effect-claim:unmet]
    let _n = tally()
    use Counting()
    let _m = tally()
}

// Viral downward: a body holding `[Counter]` may not call a callee asking
// for a claim; one holding `[Ordered Counter]` may call either.
fn weak() [Counter] -> Int {
    poke()
    //~ error[effect-claim:unmet]
    return tally()
}

fn strong() [Ordered Counter] -> Int {
    poke()
    return tally()
}

// A handle carries its claims: spawning `Counting` answers `eff[Ordered
// ShardKeyOrdered Counter]`, which fits `eff[Ordered Counter]` and binds
// `Ordered Counter` under `use`; a bare `eff[Counter]` does not fit.
fn through(c: eff[Ordered Counter]) [use] -> Int => c {
    use c
    return tally()
}

fn handles() [use, spawn] {
    let p = pool(2)
    let log = spawn Printing() on p
    use log
    let c = spawn Counting() on p
    let _a = through(copy(c))
    let bare: eff[Counter] = copy(c)
    //~ error[call-resolve:no-overload]
    let _b = through(bare)
}

// A dependency's claims are asked of the binding around its construction.
effect Audit {
    fn check() -> None
}

handler Auditing() [Ordered Counter] for Audit {
    fn check() -> None {
        bump(1)
    }
}

fn audited() [use, spawn] {
    let p = pool(2)
    let log = spawn Printing() on p
    use log
    let a = spawn Counting() on p
    let b = spawn Counting() on p
    use Spraying(a, b)
    //~ error[effect-claim:unmet]
    use Auditing()
}

//--- declarations
// [effect-claim] What a claim may be, and where it may be written.
//~ error[effect-claim:state]
qualifier Loose for Effect

effect Clock {
    fn now() -> Int
}

effects Time = [Clock]

// A claim names a claim.
//~ error[effect-claim:not-a-claim]
fn not_a_claim() [NonEmpty Clock] -> None {}

// A claim is not a value's qualifier.
//~ error[effect-claim:on-value]
fn on_value(n: Ordered Int) [] -> None {}

// Not on an alias, nor in one.
//~ error[effect-alias:claim-alias]
fn on_alias() [Ordered Time] -> None {}

//~ error[effect-alias:claim]
effects Claimed = [Ordered Clock]

// Not on an effect variable.
//~ error[effect-var:claim]
fn on_var<[E]>(f: () [E] -> None) [Ordered E] => f {}

// Not in a fn type: the function calling the value says it.
//~ error[effect-claim:fn-type]
fn in_fn_type(f: () [Ordered Clock] -> None) -> None => f {}
