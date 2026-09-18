// [actor-kind] The asynchronous half of the effect surface: an actor is
// an effect handler bound with `spawn` instead of `use`, and this module
// declares the three types that binding produces or consumes. Everything
// here is `intrinsic` — an addr, a reply token and a pool are handles into the
// scheduler each backend ships in its runtime, so their representation is
// the backend's business and never Salvo's.
//
// The forms that use them are language syntax rather than functions, because
// a handler is not a value: `spawn H(args) use D(...) on POOL`
// mints an [Addr], `replyto k(captures)` mints a [Reply], and `waitfor` is
// the bridge into both — `main`'s, and any frame's that declares
// `[waitfor]`. See LANGUAGE_SPEC.md's "Actors".

// [actor-spawn-expr] A handle to a running actor, and the *only* thing a
// spawn hands back: the state behind it is the child's alone, so an `Addr` is
// the whole of what one actor knows about another.
//
// [E] is the **effect** the actor serves — the one place in the language
// where an effect appears as a type argument [effect-not-data]. That is what
// makes an actor and a locally `use`d handler interchangeable: both are
// bound to the same effect, so `use addr` and `use H()` are the same act on
// the calling side.
//
// An addr is freely copyable and never linear: sending to an actor that has
// died is a silent no-op, so holding a stale one is safe, and the monitor
// surface (`watch`) is how a death is *observed* rather than tripped over.
export intrinsic type Addr<E>

// [actor-replyto] A one-shot answer channel, minted by `replyto k(captures)`
// and consumed by sending to it. **Linear**, which is the guarantee the whole
// request/response shape rests on: an answer is delivered exactly once, on
// every path, checked where the token is held rather than at run time
// [linear-group].
//
// A `Reply<T>` is an addr with a single send member, so `send(r, v)` — or
// `r.send(v)` — is the ordinary operation it looks like. [T] is what the
// awaiting continuation receives.
export linear intrinsic type Reply<T>

// [actor-replyto] Answer a request: delivers [value] to the continuation
// [reply] was minted for, and discharges the token by consuming it. Enqueue,
// never execute — the continuation runs as its own later activation, on the
// actor that minted it — and it never blocks, because the mailbox capacity
// for the answer was reserved when the request was made.
//
// Sending to an actor that has already died is a silent no-op, as every
// send is.
export intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply, !value

// [actor-mailbox] An actor's mailbox, declared by its handler:
// `mailbox { capacity: 16 }` (user decision 2026-09-16). The block is this
// struct's literal with the type elided — `mailbox` names the slot, and the
// slot's type is this — so field names, types and diagnostics are the ordinary
// struct ones, and a future setting is a *field* here rather than new syntax.
//
// [capacity] is the number of user messages the queue holds before a send
// blocks; replies do not count against it, since their room is reserved when the
// request is made. There is no default: a bound the compiler chose would be a
// performance cliff nobody wrote.
export struct Mailbox { capacity: Int }

// [actor-spawn-expr] Where actors run: a pool of threads, named by a
// spawn's `on POOL` clause. A pool is an ordinary value, so one can be built
// once and handed to many spawns — which is how a program says "these
// actors share these threads".
//
// [main-pool] `main` has a pool of its own, of which it is the single
// worker: a spawn that writes no `on` clause runs on the pool current where
// it was written, which in `main` is that one. Work placed there runs while
// `main` waits in a `waitfor` and dies when `main` returns.
export intrinsic type Pool

// [actor-spawn-expr] A pool of [size] threads. An ordinary function, not
// syntax: `on pool(2)` is a call, and declaring `[spawn]` is what makes
// creating one a capability the caller must hold.
export intrinsic fn pool(size: Int) [spawn] -> Pool => size

// [waitfor-dedicated] A pool of exactly one thread, and the placement that
// grants the right to *block* it. It is a claim about where the handle came
// from rather than about the pool's contents, so it is a provenance
// qualifier: only [thread] mints one, and the `on` clause **consumes** it —
// which is what makes "a dedicated thread has exactly one occupant" a fact
// of the type system instead of a convention.
export provenance qualifier Dedicated of Pool

// [pool-fault-sink] Why a task or an unwatched actor died, delivered to a
// pool's fault sink.
//
// The sibling of [Exit], and separate from it for the reason the two shapes
// differ: a death notification is a one-shot answer about an *identity*, while
// a pool emits a recurring stream about work that had none. [reason] is the
// host's account of the fault, exactly as `Exit`'s is — print it, do not branch
// on it.
export struct Fault { reason: Str }

// [pool-fault-sink] The protocol a pool's fault sink serves. Ordinary Salvo:
// spawn a handler of it and hand the addr to [pool], and every uncaught fault
// on that pool arrives as a message.
//
// The net *under* supervision rather than a replacement for it: `watch` is
// per-actor and one-shot [actor-watch], and a scheduled task has no addr to
// watch — so a fault that nobody was watching would otherwise vanish. With no
// sink, the runtime reports it by name instead.
export actor effect Faults {
    send fn faulted(fault: Fault) => !fault
}

// [pool-fault-sink] A pool of [size] threads whose uncaught faults go to
// [sink] (user decision 2026-09-17, FC-5(a)). The overload exists so the plain
// `pool(n)` stays the simple thing it was: a sink is a choice, and where it is
// absent the runtime's named report is the default.
export intrinsic fn pool(size: Int, sink: Addr<Faults>) [spawn] -> Pool => size, sink

// [waitfor-dedicated] One fresh thread, owned by whatever is placed on it.
// This is the placement a `[waitfor]`-carrying handler needs: a wait may
// occupy its thread until the answer arrives, so it must not be a thread
// anything else was counting on. Spending the value is spending the thread —
// `on thread()` consumes it, and there is no second spawn onto the same one.
export intrinsic fn thread() [spawn] -> Dedicated Pool

// [actor-watch] Why an actor died, delivered to whoever was watching it.
//
// A death is always abnormal: an activation faulted, and the actor is gone
// with whatever it still owed. [reason] is the host's account of the fault —
// a panic message on the Rust backend, an exception's on the Kotlin one — so
// it is the one thing on this surface whose *text* is the target's rather
// than the language's. Print it in a diagnostic, do not branch on it.
export struct Exit { reason: Str }

// [actor-watch] Watch [target] for death: when it dies, the scheduler sends
// an [Exit] to [on_exit]. The whole monitor surface — one function, one
// struct — because a death notification is itself an answer, so the
// request/response machinery already carries it.
//
// The token is minted like any other (`replyto died(...)` in a handler,
// `waitfor` in `main`) and is **consumed** here: a watch is a promise to
// handle the answer, and forgetting one is the ordinary leak diagnostic
// [linear-obligation] rather than a silently dropped registration. Watching a
// actor that has *already* died answers immediately, so there is no race to
// lose between a spawn and its watch.
//
// [target] is kept, since an addr is freely copyable: watching does not spend
// the handle, and the same actor may be watched by many.
export intrinsic fn watch<E>(target: Addr<E>, on_exit: Reply<Exit>) [spawn] -> None => target, !on_exit

// [actor-on-idle] What quiescence looked like, delivered to whoever asked to
// hear about it.
//
// The two fields are the difference between *done* and *stuck*, both zero
// meaning the first: [parked_gates] counts the actors on the pool whose
// mailbox is gated on an answer that has not come, and [parked_tokens] counts
// the reply tokens aimed at work on that pool which nobody has discharged. A
// gated actor is also owed a token, so the gates are the subset of the tokens
// that block a mailbox as well.
//
// A registration the scheduler is holding — a [watch], or an [on_idle] of its
// own — is not counted: the scheduler will answer it when the event happens,
// so it is not an obligation the program has forgotten.
export struct Idle {
    parked_gates: Int,
    parked_tokens: Int
}

// [actor-on-idle] Ask to be told when the program runs out of work: the
// scheduler sends an [Idle] to [notify] the moment nothing anywhere can run —
// no activation running, every mailbox undeliverable, every queue of scheduled
// work empty — reporting the obligations still outstanding on [p].
//
// The shape is [watch]'s, for the same reasons: the token is minted like any
// other (`waitfor` in `main`, `replyto` in a handler) and **consumed** here, so
// a registration that is forgotten is the ordinary leak diagnostic; and it is
// edge-triggered and one-shot, since delivering the answer is itself work and
// ends the idleness that prompted it. Hearing about the next one means
// registering again.
//
// What it is for is sequencing: "the work I sent has settled" is otherwise a
// guess about how many messages the code under test sends. Two caveats it
// inherits from the detection rather than adding: an idle answer means what it
// says only while nothing *outside* the scheduler can inject work (a platform
// handler with a thread of its own can stale it), and a frame parked in a
// `waitfor` counts as running — so idleness does not fire while a wait is in
// flight anywhere.
//
// [p] is kept: a pool is an ordinary value, and asking about one does not
// spend it.
export intrinsic fn on_idle(p: Pool, notify: Reply<Idle>) [spawn] -> None => p, !notify
