// [actor-spawn-expr] [actor-waitfor] [actor-use-addr] The asynchronous
// surface's checker rules: what a `spawn` needs to be legal, where a
// `waitfor` may stand, and how an `Addr` is called. The types the forms
// produce and consume are `std.core.actor`'s ([actor-types]). A process is a
// handler bound asynchronously, so almost every rule is one `use` already
// had, moved to the spawn site.

// The effects and handlers the cases share: a `Counter` protocol with a
// request/response pair, a `Log` dependency, and handlers for both.
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

// ===== The shape that must check clean =====

// The whole first-pass surface in one function: two spawns (one supplying a
// dependency with the *other's* addr), a send through an addr, `waitfor` with
// the token consumed inside its block, and `use addr` followed by an
// unqualified call. If this ever stops checking, the feature is broken
// regardless of what the negative cases say.
fn the_first_pass_surface_checks_clean() [use, spawn] {
    let logger = spawn Printing() on pool(1)
    let counter = spawn Counting() with logger on pool(2)
    counter.bump(2)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    use counter
    bump(sum)
}

// ===== [actor-spawn-expr] The spawn =====

// [actor-spawn-effect] The capability gate: creating a process is something
// a function must declare, exactly as registering a handler is.
fn a_spawn_requires_the_spawn_capability() [use] -> Int {
    //~ error[actor-spawn-effect:missing]
    //~ error[actor-spawn-effect:callee-needs-spawn]
    let _c = spawn Printing() on pool(1)
    return 1
}

// [actor-spawn-inherit] A spawned handler **inherits** what the spawning
// scope has (user decision 2026-09-20): a declared dependency the `with`
// clause does not cover resolves from the scope and travels as a handle. The
// clause is for *overriding*, not for satisfying.
fn a_spawned_handler_inherits_the_spawning_scope() [use, spawn] {
    let printer = spawn Printing() on pool(1)
    use printer
    let _counter = spawn Counting() on pool(1)
}

// [actor-spawn-inherit] [effect-handle] A spawn inherits an actor-face
// handler bound inline with `use` like any other binding (2026-09-28): the
// binding is a handle, so the child holds the same instance the scope calls.
fn a_spawn_inherits_an_inline_binding() [use, spawn] {
    use Printing()
    let _counter = spawn Counting() on pool(1)
}

// Supplying something the handler does not depend on is a mistake, not
// generosity: the reader believes it is being used.
fn a_spawn_cannot_supply_an_unused_dependency() [use, spawn] {
    //~ error[actor-spawn-inherit:not-a-dep]
    let _counter = spawn Printing() with Printing() on pool(1)
}

// A dependency *constructed* in the clause may not itself have dependencies:
// there is no scope on the child to resolve them from. The remedy is an addr
// of a process that already serves the effect.
fn a_clause_construction_cannot_have_dependencies_of_its_own() [use, spawn] {
    //~ error[actor-spawn-expr:with-dependencies]
    //~ error[actor-spawn-inherit:missing-dep]
    //~ error[actor-spawn-inherit:not-a-dep]
    let _c = spawn Counting() with Counting() on pool(1)
}

// [actor-mailbox] The mailbox is typed where it is declared (its `capacity`
// is an `Int`, checked as any struct field's value is) and the spawn's one
// remaining clause is typed too: `on` takes a `Pool`, which `pool(n)` is the
// way to get.
handler Loud() of Log {
    //~ error[struct-decl:field-mismatch]
    mailbox { capacity: "lots" }
    send fn note(what: Str) {}
}

fn the_pool_clause_is_typed() [use, spawn] {
    //~ error[actor-spawn-expr:pool-type]
    let _b = spawn Printing() on 7
}

// A spawn's value is the child's `eff[E]` for the effect its handler
// implements, which is what makes it usable as another spawn's dependency
// and as a receiver. Checked through a *wrong* annotation, so the message
// states the type.
fn a_spawn_answers_an_addr_of_the_handlers_effect() [use, spawn] {
    //~ error[var-let-infer:annotation-mismatch]
    let _c: Int = spawn Counting() with Printing() on pool(1)
}

// ===== [actor-use-addr] Sends through an addr =====

// A dot-call through an addr resolves against the effect the process serves,
// with the receiver naming *where* the message goes rather than being the
// first argument.
fn an_addr_call_resolves_against_the_served_effect() [use, spawn] {
    let counter = spawn Counting() with Printing() on pool(1)
    //~ error[actor-use-addr:no-member]
    counter.wind_down()
}

fn an_addr_call_checks_its_arguments() [use, spawn] {
    let counter = spawn Counting() with Printing() on pool(1)
    //~ error[call-resolve:no-overload]
    counter.bump("two")
}

// `use addr` binds the effect in scope, so its members are callable
// unqualified, and an addr is *not* consumed by binding it, because an addr
// is freely copyable.
fn use_addr_binds_the_effect_and_keeps_the_addr() [use, spawn] {
    let counter = spawn Counting() with Printing() on pool(1)
    use counter
    bump(1)
    counter.bump(2)
}

// `use` of something that is neither a handler nor an addr says so, naming
// both forms.
fn use_of_a_plain_value_is_refused() [use, spawn] {
    let n = 1
    //~ error[actor-use-addr:not-a-handle]
    use n
}

// ===== [actor-waitfor] The bridge =====

// [actor-waitfor] A wait needs no capability (SH-5(d), user decision
// 2026-09-19): occupancy is inferred by the deadlock graph and reported by
// the runtime, and a call occupying its frame until it returns is what a call
// is. Any function may wait; callers declare nothing.
fn helper() [use, spawn] -> Int {
    let counter = spawn Counting() with Printing() on pool(1)
    return waitfor out: Reply<Int> {
        counter.total(out)
    }
}

fn caller() [use, spawn] -> Int {
    return helper()
}

// And a handler whose member waits spawns on any placement: the pump rule
// [waitfor-pump] means its wait serves the pool it runs on, so the old
// dedicated-thread grant priced a hazard that no longer exists. `thread()`
// remains as placement one may *want*.
actor effect Audit {
    send fn audit(what: Str) => !what
}

handler Slow() [Counter] of Audit {
    mailbox { capacity: 2 }

    send fn audit(what: Str) {
        let sum = waitfor out: Reply<Int> {
            total(out)
        }
        discard(what)
        discard(sum)
    }
}

fn a_waiting_handler_spawns_on_any_pool() [use, spawn] {
    let counter = spawn Counting() with Printing() on pool(1)
    let _slow = spawn Slow() with counter on pool(2)
    let _dedicated = spawn Slow() with counter on thread()
}

// [waitfor-dedicated] A dedicated thread has exactly one occupant, and the
// `on` clause is what spends it: reusing the value is the ordinary
// use-after-move diagnostic rather than a rule of its own (user refinement
// 2026-09-17).
fn an_on_clause_consumes_a_dedicated_pool() [use, spawn] {
    let own = thread()
    let _a = spawn Printing() on own
    //~ error[flow-use-after-move:consumed]
    let _b = spawn Printing() on own
}

// …while a plain `Pool` is shared by many spawns.
fn a_plain_pool_is_shared() [use, spawn] {
    let many = pool(2)
    let _a = spawn Printing() on many
    let _b = spawn Printing() on many
}

// [actor-use-addr] `use H(args) on POOL`, the sugar (SH-7, user decision
// 2026-09-19): `let __a = spawn H(args) on POOL` then `use __a`, in one line.
// The spawn half wants `[spawn]`, the binding half is the ordinary `use
// addr`.
fn use_on_pool_is_spawn_plus_bind() [use, spawn] {
    use Printing() on pool(1)
    note("hello")
}

fn use_on_pool_wants_the_spawn_capability() [use] {
    //~ error[actor-spawn-effect:missing]
    //~ error[actor-spawn-effect:callee-needs-spawn]
    use Printing() on pool(1)
}

// [actor-spawn-expr] An actor `spawn` must name its pool (user decision
// 2026-10-18): `use` covers the this-scope binding, so `spawn` is reserved
// for "on a thread". An omitted `on` is refused, naming both remedies.
fn a_spawn_must_name_its_pool() [use, spawn] {
    //~ error[actor-spawn-expr:no-pool]
    let counter = spawn Counting() with Printing()
    counter.bump(2)
    let _sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
}

// Its binding is a reply token; anything else is a mistake about what the
// form does.
fn waitfor_binds_a_reply_token() [use, spawn] {
    //~ error[actor-waitfor:binding-type]
    let _n = waitfor out: Int {
        let _x = 1
    }
}

// The token is linear, so "the block must consume it" needs no rule of its
// own, and the leak names the discharger, `send`.
fn a_waitfor_that_never_sends_its_token_leaks() [use, spawn] {
    //~ error[ir-linear-check:leak-at-exit]
    let _sum = waitfor out: Reply<Int> {
        let _x = 1
    }
}

// And its value is the token's payload, so `waitfor out: Reply<Int>` is an
// `Int`, checked through a wrong annotation.
fn a_waitfor_answers_its_tokens_payload() [use, spawn] {
    let counter = spawn Counting() with Printing() on pool(1)
    //~ error[var-let-infer:annotation-mismatch]
    let _sum: Str = waitfor out: Reply<Int> {
        counter.total(out)
    }
}

// ===== The leftovers the first slice closed =====

// [actor-use-addr] A receiver is any **place** whose type is an addr, not
// just a variable: a field chain (a registry of children, a supervisor's
// state) and an array element both work.
struct Registry {
    counter: eff[Counter]
}

fn an_addr_place_of_any_shape_is_a_receiver() [use, spawn] {
    let counter = spawn Counting() with Printing() on pool(1)
    let r = Registry { counter: counter }
    r.counter.bump(1)
    let many: eff[Counter][] = array_of(r.counter)
    many[0].bump(2)
}

// [actor-spawn-expr] [actor-waitfor] The asynchronous forms stop at a
// **closure**: a function value's body runs wherever it is called, a fn type
// cannot declare `spawn`, and `waitfor` needs call sites something can
// enumerate. Each diagnostic says so in the closure's terms.
fn a_spawn_stops_at_a_lambda() [use, spawn] {
    //~ error[actor-spawn-effect:in-lambda]
    //~ error[actor-spawn-effect:callee-needs-spawn]
    let _f = () -> spawn Printing() on pool(1)
}

fn a_waitfor_stops_at_a_lambda() [use, spawn] {
    let counter = spawn Counting() with Printing() on pool(1)
    //~ error[actor-waitfor:in-lambda]
    let _f = () -> waitfor out: Reply<Int> {
        counter.total(out)
    }
}

// [effect-member-overload] An overloaded send member reached through an addr
// is chosen by the call's rule (PIPELINE.md B3.7, D24 (a)): the arguments
// typed once, the overloads ranked by them — by arity here, by type below.
actor effect Sink {
    send fn put(a: Int)
    send fn put(a: Int, b: Int)
}

handler Dropping() of Sink {
    mailbox { capacity: 1 }

    send fn put(a: Int) {}
    send fn put(a: Int, b: Int) {}
}

fn arity_settles_the_overload() [use, spawn] {
    let s = spawn Dropping() on pool(1)
    s.put(1)
    s.put(1, 2)
}

actor effect TiedSink {
    send fn put(a: Int)
    send fn put(a: Str) => !a
}

handler TiedDropping() of TiedSink {
    mailbox { capacity: 1 }

    send fn put(a: Int) {}
    send fn put(a: Str) {}
}

// Before B3.7 a same-arity pair was refused ("several overloads take 1
// argument"): arity was all a send could tell them apart by.
fn a_same_arity_overload_is_chosen_by_type() [use, spawn] {
    let s = spawn TiedDropping() on pool(1)
    s.put(1)
    s.put("one")
    //~ error[call-resolve:no-overload]
    s.put(true)
}

// ===== [waitfor-infer] The binder's type =====

// [waitfor-infer] The type may be omitted: the binder is passed to a send,
// and that member's `Reply<T>` parameter says what is being waited for (user
// decision 2026-09-21). The `bump(sum)` below only resolves if `sum` is an
// `Int`.
fn a_waitfor_infers_its_binder_type() [use, spawn] {
    let logger = spawn Printing() on pool(1)
    let counter = spawn Counting() with logger on pool(2)
    let sum = waitfor out {
        counter.total(out)
    }
    use counter
    bump(sum)
}

// [waitfor-infer] Nothing to read it off: the block never passes the token to
// a `Reply<T>` parameter, so the type cannot be told, and the diagnostic
// names both remedies (write it out, or send the token).
fn a_waitfor_with_no_send_cannot_be_inferred() [use, spawn] {
    let logger = spawn Printing() on pool(1)
    let counter = spawn Counting() with logger on pool(2)
    //~ error[waitfor-infer:no-reply-param]
    let _sum = waitfor out {
        counter.bump(1)
    }
}

//--- waitfor-is-no-declarable-effect
// The word is gone from effect lists everywhere, with a **parse** error that
// names the deletion rather than claiming the name never existed.
//~ error[waitfor-effect:declared]
fn helper() [use, spawn, waitfor] -> Int {
    return 1
}
