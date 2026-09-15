// [async-process] The asynchronous half of the effect surface: a process is
// an effect handler bound with `spawn` instead of `use`, and this module
// declares the three types that binding produces or consumes. Everything
// here is `intrinsic` — an addr, a reply token and a pool are handles into the
// scheduler each backend ships in its runtime, so their representation is
// the backend's business and never Salvo's.
//
// The forms that use them are language syntax rather than functions, because
// a handler is not a value: `spawn H(args) use D(...) capacity N on POOL`
// mints an [Addr], `replyto k(captures)` mints a [Reply], and `waitfor` is
// `main`'s bridge into both. See LANGUAGE_SPEC.md's "Asynchronous effect
// handlers".

// [async-spawn-expr] A handle to a running process, and the *only* thing a
// spawn hands back: the state behind it is the child's alone, so an `Addr` is
// the whole of what one process knows about another.
//
// [E] is the **effect** the process serves — the one place in the language
// where an effect appears as a type argument [effect-not-data]. That is what
// makes a process and a locally `use`d handler interchangeable: both are
// bound to the same effect, so `use addr` and `use H()` are the same act on
// the calling side.
//
// An addr is freely copyable and never linear: sending to a process that has
// died is a silent no-op, so holding a stale one is safe, and the monitor
// surface (`watch`) is how a death is *observed* rather than tripped over.
intrinsic type Addr<E>

// [async-replyto] A one-shot answer channel, minted by `replyto k(captures)`
// and consumed by sending to it. **Linear**, which is the guarantee the whole
// request/response shape rests on: an answer is delivered exactly once, on
// every path, checked where the token is held rather than at run time
// [linear-group].
//
// A `Reply<T>` is an addr with a single send member, so `send(r, v)` — or
// `r.send(v)` — is the ordinary operation it looks like. [T] is what the
// awaiting continuation receives.
linear intrinsic type Reply<T>

// [async-replyto] Answer a request: delivers [value] to the continuation
// [reply] was minted for, and discharges the token by consuming it. Enqueue,
// never execute — the continuation runs as its own later activation, on the
// process that minted it — and it never blocks, because the mailbox capacity
// for the answer was reserved when the request was made.
//
// Sending to a process that has already died is a silent no-op, as every
// send is.
intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply, !value

// [async-spawn-expr] Where processes run: a pool of threads, named by a
// spawn's `on POOL` clause. A pool is an ordinary value, so one can be built
// once and handed to many spawns — which is how a program says "these
// processes share these threads".
intrinsic type Pool

// [async-spawn-expr] A pool of [size] threads. An ordinary function, not
// syntax: `on pool(2)` is a call, and declaring `[spawn]` is what makes
// creating one a capability the caller must hold.
intrinsic fn pool(size: Int) [spawn] -> Pool => size
