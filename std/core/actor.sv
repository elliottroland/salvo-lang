// [actor-kind] The asynchronous half of the effect surface: an actor is
// an effect handler bound with `spawn` instead of `use`, and this module
// declares the three types that binding produces or consumes. Everything
// here is `intrinsic` — an addr, a reply token and a pool are handles into the
// scheduler each backend ships in its runtime, so their representation is
// the backend's business and never Salvo's.
//
// The forms that use them are language syntax rather than functions, because
// a handler is not a value: `spawn H(args) use D(...) capacity N on POOL`
// mints an [Addr], `replyto k(captures)` mints a [Reply], and `waitfor` is
// `main`'s bridge into both. See LANGUAGE_SPEC.md's "Actors".

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
intrinsic type Addr<E>

// [actor-replyto] A one-shot answer channel, minted by `replyto k(captures)`
// and consumed by sending to it. **Linear**, which is the guarantee the whole
// request/response shape rests on: an answer is delivered exactly once, on
// every path, checked where the token is held rather than at run time
// [linear-group].
//
// A `Reply<T>` is an addr with a single send member, so `send(r, v)` — or
// `r.send(v)` — is the ordinary operation it looks like. [T] is what the
// awaiting continuation receives.
linear intrinsic type Reply<T>

// [actor-replyto] Answer a request: delivers [value] to the continuation
// [reply] was minted for, and discharges the token by consuming it. Enqueue,
// never execute — the continuation runs as its own later activation, on the
// actor that minted it — and it never blocks, because the mailbox capacity
// for the answer was reserved when the request was made.
//
// Sending to an actor that has already died is a silent no-op, as every
// send is.
intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply, !value

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
struct Mailbox { capacity: Int }

// [actor-spawn-expr] Where actors run: a pool of threads, named by a
// spawn's `on POOL` clause. A pool is an ordinary value, so one can be built
// once and handed to many spawns — which is how a program says "these
// actors share these threads".
intrinsic type Pool

// [actor-spawn-expr] A pool of [size] threads. An ordinary function, not
// syntax: `on pool(2)` is a call, and declaring `[spawn]` is what makes
// creating one a capability the caller must hold.
intrinsic fn pool(size: Int) [spawn] -> Pool => size

// [actor-watch] Why an actor died, delivered to whoever was watching it.
//
// A death is always abnormal: an activation faulted, and the actor is gone
// with whatever it still owed. [reason] is the host's account of the fault —
// a panic message on the Rust backend, an exception's on the Kotlin one — so
// it is the one thing on this surface whose *text* is the target's rather
// than the language's. Print it in a diagnostic, do not branch on it.
struct Exit { reason: Str }

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
intrinsic fn watch<E>(target: Addr<E>, on_exit: Reply<Exit>) [spawn] -> None => target, !on_exit
