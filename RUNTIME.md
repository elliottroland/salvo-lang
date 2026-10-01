# The scheduler runtime in Salvo — survey and plan

Working document (survey of 2026-10-01, read-only session). It answers the
ROADMAP item "How much of the scheduler runtime could be Salvo" (Recorded, not
scheduled): what the two hand-written schedulers contain today, which parts
can become one Salvo module over a small set of private intrinsics, what the
language and compiler would need for that, and in what order to do it.
Nothing here is decided. The items marked **DECISION** are the user's call.

Sketches of Salvo code below show the intended shape. They have not been run
through the checker.

## 1. Summary

- The two runtimes are `crates/salvo-backend-kotlin/runtime/scheduler.kt`
  (1,994 lines) and `crates/salvo-backend-rust/runtime/scheduler.rs`
  (2,720 lines). Each is also copied into std's host project
  (`std/platform/salvo/scheduler.sv.{kt,rs}`, 1,997 and 2,723 lines), so every
  host project that reaches `Reply` or a spawn carries a full copy.
- About 90% of each file is bookkeeping under one lock: tables, queues,
  counters, the frame formats, and the decisions the scheduler makes (what to
  run next, whether the program is idle or deadlocked, where a fault goes). All
  of that can be written in Salvo once.
- What has to stay in each host language is small: a lock with a condition
  variable, starting a thread, one thread-local, type erasure for message
  payloads, running an activation or task inside a fault boundary, and a
  process-wide slot holding the scheduler. Each is 5–40 lines per backend. With
  the host-facing shims (`SalvoReply`, `SalvoHostReply`, the external-source
  counter), I estimate 300–400 host lines per backend remain, down from
  2,000–2,700. The host stream table (about 200 lines) is not scheduling at
  all and should move to its own runtime file either way.
- The language can already express most of the logic. The gaps are a linear
  lock guard (the natural Salvo form of Rust's `MutexGuard`), moving a value
  out of a field of a list element, a payload type that hides its real type,
  an O(1) queue, and one real design question: how code that declares no
  effects (`send(reply, v) []`, host threads, the `Addr` codec) reaches a
  `Runtime` *effect* (§6, G4).
- Two restructurings make the port smaller, and both could be done before
  it: move the group protocol (handshake, gossip introductions, published
  names, member sharing — frames 4–9) out of the scheduler into `std/net.sv`,
  where the actors that consume it already live; and encode frames with the
  canonical wire encoding instead of hand-written byte layouts.
- The cost to expect is performance on the Rust backend, from how the
  emitter lowers collections and copies. It needs measuring before and after
  (§8). On Kotlin the generated code has the same shape as today's.

## 2. What is there today

### 2.1 Sections of each file

| Section | Kotlin lines | Rust lines | Content |
|---|---|---|---|
| Local scheduler | 1–1042 (≈1,040) | 1–1377 (≈1,380) | actor table, pools, spawn, send with back-pressure, mint / gated mint, waiters and the `waitfor` pump, worker loop, `run_job` / `run_task` with fault boundary, death and watches, fault sink, `on_idle`, deadlock and main-pool-wedge reports, the timer thread, external sources |
| Reply classes | 1717–1809 (≈90) | inside the local section | `SalvoReply` (`send`, `checked`, `hosted`), `SalvoHostReply` (Rust also reports a host reply dropped unsent) |
| Across machines | 1043–1716 (≈675) | 1378–2484 (≈1,105) | node identity, virtual nodes, proxies, routes and wire hooks, credits, typed send/reply, reply export/import, ten frame kinds encoded and decoded by hand, handshake, groups, publish, member sharing, route-stub views, `key_hash` |
| Host streams | 1810–1994 (≈185) | 2485–2720 (≈235) | `SalvoIn`/`SalvoOut` and the process stream table [stream-table]; shares only `freshHandle` with the scheduler |

The Rust file is longer mainly because of guard threading, `Option`
plumbing and the hand-written frame decoder. The two implement the same rules
line for line; the file headers say they "must stay identical on both
backends".

### 2.2 Who calls it

- **Generated code** (both emitters) calls about 45 entry points. By use
  count in the Kotlin emitter and the shipped host files: `freshHandle`,
  `send`, `addrIdentity`, `watch`, `sendWire`, `onIdle`, `importAddr`, `spawn`,
  `sendReply`, `sendHostReply`, `replyImport`, `replyExport`, `hostReply`,
  `currentPool`, `setProtocols`, `mintTask`, then one call site each for
  `mint`, `mintGated`, `waiter`, `waiterDecoder`, `awaitReply`, `pool`,
  `poolWithSink`, `thread`, `after`, `poolAt`, `newNode`, `hereNode`, and the
  network entries (`addRoute`, `setWire`, `deliverFrame`, `connected`,
  `credits`, `pending`, `setGroup`, `watchPeers`, `introduce`, `helloFrame`,
  `leaveGroup`, `peerProtocol`, `publish`, `shareMembers`, `viewSet`,
  `viewMembers`, `keyHash`, `parkBriefly`).
- **Generated actor bodies** implement `SalvoActor` (`handle`, `resume`,
  `decodeReply`) and pass decoders (`MsgDecoder`, `ReplyDecoder`) and
  *builders* (`ExitOf`, `IdleOf`, `FiredOf`, the fault-sink builder, and
  `HelloOf`/`GoneOf`/`IntroOf`/`NamedOf`/`MembersOf`). The builders exist
  only because the host runtime cannot construct a Salvo value.
- **std's intrinsics** are the typed surface over it: `core.actor` (`send`,
  `pool`, `thread`, `watch`, `on_idle`, `eq` on `Addr`), `time`
  (`fire_after`), and `net` (about 25 intrinsics, most of them private to
  `net.sv`).
- **Host code** calls `externalBegin`/`externalEnd` (`std/platform/net.*`, the
  TCP reader), `freshHandle` (the stream hosts), and `SalvoReply.hosted()` /
  `SalvoHostReply.send` (platform handlers completing replies).

### 2.3 Divergences and defects found during the survey

None of these is fixed here. They are recorded because a single Salvo
implementation removes the first three by construction, and the rest should go
on the ROADMAP whether or not the port happens.

1. **A host reply dropped unsent is reported on Rust only** (`impl Drop for
   SalvoHostReply`). Documented as best-effort (user decision 2026-09-29), and
   it stays host-specific after a port, since only Rust has a drop.
2. **Addr capability bits differ in strength.** Kotlin draws them from
   `SecureRandom`; Rust's `random_u64` hashes a counter under `RandomState`,
   and its comment says "not a cryptographic RNG". [addr-capability] is the
   same rule on both backends with different guarantees.
3. **Lock reentrance differs.** Kotlin's `ReentrantLock` lets a thread that
   already holds the lock take it again; Rust's `Mutex` deadlocks. Today's code
   is careful never to do it (COMPLETED.md's "Codecs must run with the
   scheduler lock released"), but the difference is latent: a mistake hangs
   Rust and passes on Kotlin.
4. **The waiter table never shrinks.** Every `waitfor` adds a `WaiterState`
   that is never removed, on both backends, so a long-running loop around a
   `waitfor` grows without bound. The actor table also keeps every dead
   actor.
5. **A forged identity grows the actor table.** `importAddr` answers a new
   dead entry (`deadEntry()` / `dead_entry`) for each identity that fails the
   bits check, so a peer sending forged addrs can grow the table without
   limit.
6. **The route stub busy-waits.** When a pick answers `None`, the stub calls
   `parkBriefly` (a 1 ms sleep) and asks again.

## 3. What moves, what stays

### 3.1 Moves to Salvo (written once)

- All tables and their record types: actors (including proxy fields, credits
  and the parked-gate state), waiters, pools, idle hooks, timers, exported
  tasks, routes, parked frames.
- Every scheduling decision: `deliverable` (with the gate), `pick`, the
  task-before-activation rule, `quiet`, `idle`, `stuck`, `release`/`untrack`
  accounting, `parked` (the `Idle` payload), `fire_idle`.
- The worker loop, the `waitfor` pump, `run_job` and `run_task` *around* the
  fault boundary, death, watch delivery, the fault sink, and the timer
  thread's loop.
- The diagnostics: the deadlock report and the main-pool wedge report, as
  one set of strings.
- Routing: identity, proxies, `importAddr`, credits and GRANT/OPEN, typed
  send and reply, reply export and import, staging frames under the lock and
  flushing them after it is released.
- Constructing `Exit`, `Idle`, `Fired` and `Fault` directly, which removes
  every builder parameter from the runtime API and from generated code.

### 3.2 Stays in each host language

- The primitives of §4.
- The host-facing classes: `SalvoReply` (`send`, `checked`, `hosted`),
  `SalvoHostReply` (with Rust's drop report), `externalBegin`/`externalEnd`.
  These become thin shims that call into the Salvo runtime.
- The `SalvoActor` interface that generated actor bodies implement, and the
  generated decoders.
- The `Addr`/`Reply` codecs in `wire.rs`/`wire.kt`, which call into the
  runtime for identity.
- The host stream table, moved to its own file (`streams.rs`/`streams.kt`).
- `key_hash` (FNV-1a), unless Salvo gains `Long` xor and wrapping multiply.
  It is 10 lines and gains nothing from moving.

### 3.3 Moves to `std/net.sv` instead (proposal)

Frames 4–9 (HELLO, ACK, LEAVE, INTRO, NAMED, MEMBERS), the per-node group
state, the peer protocol tables, `published`, `peerNames` and the route-stub
`views` are consumed only by `NodeGroup`/`ActorGroup` handlers that are
already Salvo in `net.sv`. The scheduler needs only frames 0–3 (MSG, REPLY,
GRANT, OPEN) plus one hook: hand every other frame to a registered control
actor. LEAVE also marks the departed node's proxies dead, so it needs one
runtime call (`node_left(node)`). This removes about half of the
across-machines section from the runtime before any port, and it is the part
whose builders (`HelloOf`, `GoneOf`, `IntroOf`, `NamedOf`, `MembersOf`) cost
the most plumbing. Whether the handshake belongs to the scheduler or to
`net` is a structural choice the user should confirm (D5 in §10).

## 4. The private primitives

All of these live in the runtime module and are not exported. Each backend
lowers them in its `intrinsics.rs` [intrinsic-fn], as with every other std
intrinsic.

| Primitive | Salvo signature (sketch) | Rust lowering | Kotlin lowering |
|---|---|---|---|
| Lock | `intrinsic type Lock<S>`; `intrinsic fn lock_of<S>(s: S) [] -> Lock<S> => !s` | `Arc<(Mutex<S>, Condvar)>` | class holding `ReentrantLock`, its `Condition` and `S` |
| Held lock | `linear intrinsic type Held<S>`; `lock(l) -> Held<S>`; `unlock(h) => !h` | `MutexGuard<S>` | a token; `lock()` / `unlock()`; `lock` asserts `!isHeldByCurrentThread` so reentry fails on both backends (§2.3 item 3) |
| State under the lock | `intrinsic fn state<S>(h: Held<S>) [] -> proj(h) Mut S => h` | `&mut *guard` | the field |
| Waiting | `wait(h) -> Held<S> => !h`; `wait_until(h, mono_nanos: Long) -> Held<S> => !h`; `signal_all(h)` | `cv.wait`, `wait_timeout`, `notify_all` | `await`, `awaitNanos`, `signalAll` |
| The process slot | `intrinsic fn sched() [] -> Lock<Sched>` | `static OnceLock`, initialised by calling the Salvo `init()` | `object` with `by lazy` |
| Threads | `intrinsic fn start_thread(body: once () -> None) [] -> None` | `std::thread::spawn` | `Thread { }` with `isDaemon = true` |
| Thread-local | `intrinsic fn here() [] -> Here`; `intrinsic fn set_here(h: Here) [] -> Here` (answers the previous) | `thread_local!` `RefCell<Here>` | `ThreadLocal<Here>` |
| Erased payload | `linear intrinsic type Dyn`; `erase<T canbe linear>(v: T) -> Dyn => !v`; `unerase<T canbe linear>(d: Dyn) -> T => !d` | `Box<dyn Any + Send>`, `downcast` | `Any?`, a cast |
| Activation | `intrinsic type Body`; `intrinsic fn activate(body: Body, addr: Int, entry: Entry) [] -> Ran Body \| Faulted Str` | `Box<dyn SalvoActor>`, dispatch inside `catch_unwind`, panic payload to text | `SalvoActor`, dispatch inside `try`/`catch (Throwable)` |
| Task | `intrinsic fn run_guarded(task: once (Dyn) -> None, value: Dyn) [] -> Str?` | `catch_unwind` | `try`/`catch` |
| Queue | `linear intrinsic type Queue<T canbe linear>` with `push`, `pop_front`, `remove_at`, `index_of_first`, `size`, `clear` | `VecDeque` | `ArrayDeque` |
| Reports | `intrinsic fn report(line: Str) [] -> None`; `intrinsic fn exit_with(line: Str) [] -> Never` | `eprintln!`, `process::exit(1)` | `System.err.println`, `exitProcess(1)` |
| Capability bits | `intrinsic fn secure_bits() [] -> Long` | OS entropy (fixes §2.3 item 2) | `SecureRandom` |

Notes on the choices:

- **Why a linear guard rather than a closure** (`locked(l, s -> …)`): the
  existing code releases and retakes the lock in the middle of `run_job`,
  `run_task` and the `waitfor` pump, and Rust's `cv.wait(guard)` consumes the
  guard and returns a new one. A linear `Held<S>` that `wait` consumes and
  returns is the same shape, so the port is a translation, and linearity
  checks that every path unlocks [linear-obligation]. Because `state(h)` is
  a projection of `h`, the checker also refuses to use a handle on the
  state after `unlock(h)` or across `wait(h)`, which Rust enforces by hand
  today and Kotlin does not enforce at all.
- **Why `activate` and `run_guarded` instead of a generic `guarded(f)`**: a
  generic fault boundary needs a `once` lambda that moves the actor body into
  its capture. If move-captures into `once` lambdas work already, one generic
  `guarded<R>(f: once () -> R) -> Ok R | Faulted Str` replaces both. Either
  way the fault boundary stays private. A public way to catch a fault would
  contradict "death is a faulted activation" [actor-watch].
- **A private `Queue` needs no decision.** A public `Deque` in std would
  (D3). Salvo's `List` lowers to `Vec`/`ArrayList`, and `remove_first` is
  `Vec::remove(0)` (O(n), `runtime/seq.rs`), which is wrong for a mailbox.
- **Timeouts take a `Long` reading** of the monotonic clock rather than
  `time.Tick`, so the runtime does not import `time`, which itself depends on
  the runtime through `fire_after`.

## 5. The Salvo layer

### 5.1 Placement

`std/runtime.sv` (and `std/runtime/wire.sv` for routing), outside `core`, so
nothing imports it implicitly and no program can name it. It imports
`core.actor` for `Exit`, `Idle`, `Fault` and `Reply`.

The typed surface stays where it is: `core.actor`'s `pool`, `watch`,
`on_idle` and `send`, `time`'s `fire_after`, and `net`'s intrinsics keep
their declarations. Only their *lowering* changes, from a call into the host
runtime to a call of the emitted runtime function. This needs no new
visibility rule: the emitter writes host code and can name an emitted private
function directly. Two compiler consequences follow:

- Module reachability must keep `std/runtime.sv` alive whenever
  `needs_scheduler` is set, because no Salvo code calls it.
- A runtime module named `runtime` emits `runtime.kt`/`runtime.rs`. No
  existing runtime file has that name, but [backend-companion] is the check
  that would catch a collision.

### 5.2 Data model (sketch)

```
struct ActorState canbe Mut {
    pool: Int,
    bound: Int,
    node: Long,
    bits: Long,
    remote: RemoteRef?,
    credits: Int,
    granted: Int,
    queue: Mut Queue<Entry>,
    user_len: Int,
    gate: Long?,
    running: Bool,
    dead: Bool,
    exit_reason: Str?,
    watchers: Mut List<Reply<Exit>>,   // typed: no ExitOf builder
    owed: Int,
    body: Body?,                       // taken out while an activation runs
}

struct Delivered { msg: Dyn, from: Long? }
struct Answered { slot: Long, value: Dyn }
struct AnsweredRaw { slot: Long, bytes: Bytes }
type Entry = Delivered | Answered | AnsweredRaw

struct Here { pool: Int, actor: Int?, frame: Bool }
```

The watchers, idle hooks and timers all hold one reply type each
(`Reply<Exit>`, `Reply<Idle>`, `Reply<Fired>`), so they stay typed in Salvo.
The runtime builds the payload itself (`send(w, Exit { reason })`), which is
what removes the builders. Mailbox entries are the only place that needs
`Dyn`, because one queue holds messages of several protocols.

Two tables get simpler:

- **Tasks**: today a `Target.Task` carries the body in a shared mutable cell
  (`Arc<Mutex<Option<TaskBody>>>` in Rust) so that exporting the token over
  the wire can move it aside. In Salvo the body goes into a
  `Mut Map<Long, PendingTask>` keyed by slot at mint, and the target holds
  only `(pool, slot)`. The slot is already unique, so the cell disappears.
- **Waiters**: a `Mut Map<Long, WaiterState>` removed on `leave_wait`, which
  fixes the unbounded growth in §2.3 item 4.

### 5.3 Two representative functions (sketch)

`send`, with back-pressure and the main-pool wedge:

```
fn send(addr: Int, msg: Dyn) [] -> None => !msg {
    let h = lock(sched())
    while blocked(state(h), addr) {
        if here().pool == MAIN_POOL && actor(state(h), addr).pool == MAIN_POOL {
            exit_with(main_pool_wedge(addr))
        }
        h = wait(h)
    }
    let a = actor(state(h), addr)
    if a.dead {
        discard(msg)            // a send to the dead is a silent no-op
    } else {
        push(a.queue, Delivered { msg: msg, from: None })
        a.user_len = a.user_len + 1
    }
    signal_all(h)
    unlock(h)
}
```

`run_job`, which releases the lock for the activation itself:

```
fn run_job(h: Held<Sched>, addr: Int, at: Int) [] -> Held<Sched> => !h {
    let a = actor(state(h), addr)
    let entry = remove_at(a.queue, at)!
    let staged = dequeued(state(h), addr, entry)   // user_len, gate, credit frame
    let body = take(a.body)!                        // see G2
    let pool = a.pool
    a.running = true
    state(h).active = state(h).active + 1
    signal_all(h)
    unlock(h)                                       // `a` is dead from here on
    flush(staged)

    let saved = set_here(Here { pool: pool, actor: addr, frame: true })
    let outcome = activate(body, addr, entry)
    set_here(saved)

    let h2 = lock(sched())
    finish(state(h2), addr, outcome)   // body back, or death + watches + sink
    signal_all(h2)
    h2
}
```

The `waitfor` pump becomes a loop over one step function that decides under
the lock and acts outside it: answer is in, run a task, run a job, fire the
idle hooks, report the deadlock, or wait. That is the structure today's loop
already has.

### 5.4 The rules the runtime module must keep

The runtime implements the actor surface, so it cannot use it. Inside
`std/runtime.sv`:

- no `spawn`, `send fn`, `replyto`, `waitfor`, actor effects, `Timer`, or
  any std function that uses them;
- no stateful handler: [effect-handle] would put it behind its own monitor
  lock, so the runtime would take a second lock under its own;
- no call back into the runtime while `Held<Sched>` is live (frames and
  codec work happen after `unlock`, as today);
- no `throw`: a fault inside the runtime is a runtime bug and should end the
  process, not be caught at an activation boundary.

These could be checked by the checker for this one module (a flag on the
source file, like `is_std`) or by a test that inspects the module's resolved
calls. A test is enough to start with.

## 6. What the language and compiler need

Each gap says whether it is engineering inside std (no decision) or a
**DECISION**.

- **G1. A linear guard with a projected state** (§4). Private intrinsics.
  The checker work is projecting `proj(h) Mut S` out of a *linear* value
  and refusing it after `h` is consumed. I have not checked whether the
  checker accepts a projection from a linear intrinsic type today.
- **G2. Moving a value out of a field of a list element.** `run_job` takes
  the actor body out of `actors[addr].body`, and death clears a queue that
  holds linear payloads. [linear-state] allows taking handler state out if
  it is put back, but it is not clear that a field reached through
  `get(list, i)!` allows a move-out. If not, a `take<T canbe linear>(slot:
  Mut T?) -> T?` is needed. Private first; making it public std API would be
  a **DECISION**.
- **G3. An erased payload type** (`Dyn`). Private intrinsic. Making it linear
  forces the runtime to `discard` messages explicitly when an actor dies,
  which states the existing behaviour (messages to the dead are dropped)
  instead of hiding it.
- **G4. How code that declares no effects reaches a `Runtime` effect.
  DECISION.** The request was for a `Runtime` effect. An effect is the
  natural Salvo seam, and it is what would let a deterministic test runtime
  (single thread, virtual time; the "scheduler-owned virtual time" upgrade
  recorded in ROADMAP) be bound in place of the threaded one. The problem is
  that effects are not data [effect-not-data], and the operations that need
  the runtime are declared without effects: `send(reply, v) []`, `Addr`
  equality and codecs, host threads completing replies, the timer thread.
  Options:
  - (a) **A std-only process binding**: `Runtime` is bound once at program
    start, and any std function may perform it without declaring it. That is
    a new kind of binding and works against Locality, though only inside std.
  - (b) **Data, not an effect**: the module's functions take the
    `Lock<Sched>` from `sched()`. No language change. A test runtime then
    comes from configuration inside `Sched` (for example a virtual-clock
    field) rather than from binding another handler.
  - (c) **Thread it through `[spawn]`**: the existing capability would carry
    the runtime handle. That covers spawns but not sends, replies or host
    threads, so it does not work alone.

  Recommendation: write the module functions first under (b). A handler
  `Scheduler(l: Lock<Sched>) of Runtime` would have exactly the same bodies,
  and by the [effect-handle] predicate it binds bare, with no monitor lock,
  because it has no state field, no fn-typed parameter and no `replyto`. So
  (a) can be added later without rewriting anything, and only when a second
  runtime implementation is actually wanted.
- **G5. The fault boundary**: private (`activate`, `run_guarded`, §4). No
  decision as long as it stays private.
- **G6. Bootstrapping rules** (§5.4): engineering. Enforced by a test.
- **G7. Intrinsic lowerings that call emitted Salvo functions**: emitter
  work in both `intrinsics.rs` files (§7).
- **G8. A queue**: private `Queue<T>`, no decision. A public `Deque` is a
  **DECISION** (std API shape).
- **G9. Frames as canonical-encoded values**: the wire encoding is already
  big-endian integers, `u32` lengths and a tag byte per union arm
  (`runtime/wire.rs` header), so the hand-written layouts could become
  `type Frame = Msg | Answer | Grant | Open | Control`, and the decoder could
  become `decode<Frame>(bytes)` plus a `when`. This changes the bytes on the
  wire. Nothing outside the repository depends on them, but it is a protocol
  change and so a **DECISION** (light).
- **G10. Integer widths.** An addr is `Int` in Salvo and `usize` in
  generated Rust; node ids and bits are `u64` in Rust and `Long` in Kotlin.
  The Salvo module would use `Int` and `Long` throughout, so the Rust
  emitter's `Addr` representation changes to `i32` (or a newtype over it).
  Engineering.

## 7. Emitter and build changes

- Every `SalvoSched.x(...)` / `salvo_x(...)` in generated code becomes a call
  into the emitted runtime module. The shapes generated code relies on stay
  the same: actor bodies implementing `SalvoActor`, decoders, the typed
  send/reply split. The builder arguments are removed from `watch`,
  `on_idle`, `fire_after`, `pool` with a sink and the net calls.
- `needs_scheduler` emits the runtime module (and its primitives file)
  instead of including `scheduler.rs`/`scheduler.kt`.
- The host project's ABI (`std/platform/salvo/scheduler.sv.*`, ABI.md
  D2–D4) shrinks to the shims and the primitive types, plus whatever of the
  emitted runtime module the platform code reaches (`SalvoReply`'s methods).
- Every checked-in example tree that contains `scheduler.rs`/`scheduler.kt`
  changes (examples `actors`, `borrowing`, `cluster`, `qualifiers`,
  `throw-and-release`, `time`, at least), as do the goldens that include
  generated runtime code.
- `runtime_tests.rs` (15 Rust tests, 3 Kotlin) drive the host API with
  hand-written `SalvoActor`s. They would move to `std/runtime.test.sv`, run by
  `salvo test` on both backends, which also makes them parity tests for
  free.

## 8. Costs

**Speed.** The hot paths are `send` (lock, push, signal), `run_job` (lock,
pop, unlock, dispatch, lock) and reply delivery. In Salvo these lower to the
same host operations as long as:

- the mailbox is a real deque (G8);
- list access through `get(list, i)!` compiles to an index plus a check, and
  nothing on the hot path is copied: watch for `clone()` of `Str` reasons and
  of `Here` on Rust;
- union values do not allocate more than today's entries do. Rust unions are
  enums. Kotlin union arms are wrapper objects, and Kotlin entries are already
  objects today.

Dispatch through a `Runtime` handle, if G4(a) is taken, is one virtual call
per runtime operation, small next to the mutex.

Nothing was measured in this session. Proposed gate: before starting, add a
benchmark program run on both backends — one-to-one ping-pong (10⁶
round trips), fan-out to 1,000 actors, a chain of 10⁵ tasks, and 10⁴ timers —
record today's numbers in COMPLETED.md, and require the port to stay within an
agreed factor (1.5× is a reasonable starting point) on each.

**Readability when debugging.** A stack trace through the scheduler goes
through generated code instead of hand-written code. Whether doc comments are
carried into the emitted host source affects how readable that is.

**Semantics drift during the port.** The deadlock and idle predicates are
subtle (COMPLETED.md records two hangs found in this code). Porting one
function at a time, with the existing tests running at each step, keeps
that risk contained. The order is in §9.

**What gets cheaper later.** The ROADMAP items that are "one line in each
runtime" become one line: the `on_idle` predicate DECISION, the
parked-obligation gap in the deadlock graph, scheduler-owned virtual time, and
the JVM program-end question (ROADMAP §4b step 7), whose "what counts as done"
rule would be stated once.

## 9. Build sequence (draft)

Each step keeps both backends passing the full suite.

1. **Benchmarks** (§8). Record today's numbers.
2. **Move the host stream table** out of `scheduler.*` into its own runtime
   file. Independent and small.
3. **Move the group protocol into `net.sv`** (§3.3; needs D5). Still host
   code in the scheduler, but frames 4–9 go to a control actor and the five
   group builders disappear. Shrinks step 5.
4. **Primitives and the empty module**: the §4 intrinsics in both backends,
   `std/runtime.sv` with `Sched`, `init` and `sched()`, and Salvo tests for
   `Lock`/`Held`, `Queue` and `Dyn`. G1–G3 checker work lands here.
5. **Port the scheduler**: the local scheduler and routing (frames 0–3,
   proxies, credits) together, because they share the actor table. Switch the
   emitters' lowerings, delete the host code, and move `runtime_tests.rs` to
   `runtime.test.sv`. This is the large step. It can be split by keeping
   the host wire code temporarily and having it call into the Salvo tables
   through exported runtime functions, at the price of throwaway glue.
6. **Optional: frames as canonical-encoded values** (G9).
7. **Optional: the `Runtime` effect** (G4(a)) and a deterministic test
   runtime, if wanted.
8. **Docs**: BACKEND_SPEC.kotlin.md / BACKEND_SPEC.rust.md runtime sections,
   ABI.md's host-project contents, ROADMAP (remove the "Recorded" entry),
   COMPLETED.md. The LANGUAGE_SPEC rules ([actor-*], [waitfor-pump],
   [task-mint], [time-timer], [addr-*]) describe behaviour and should not
   change. Their labels move from the host runtimes to `std/runtime.sv`.

## 10. Decisions for the user

- **D1 (G4).** Does `Runtime` start as an effect with a std-only process
  binding, or as data (`sched()`) with the effect added later?
  Recommendation: data first; the bodies are the same either way.
- **D2 (G2).** If a move-out operation is needed, is it private to the
  runtime or public std API?
- **D3 (G8).** A private queue, or a public `Deque` in std?
- **D4 (G9).** Change the frame format to the canonical encoding?
- **D5 (§3.3).** Move the group protocol (frames 4–9 and their state) out of
  the scheduler into `net.sv`?
- **D6.** The speed budget for the port (§8): which factor, and on which
  benchmarks.
- **D7 (§2.3).** Which of the survey findings go on the ROADMAP now: waiter
  and actor table growth, forged-identity growth, the weaker Rust capability
  bits, the reentrance difference, the route-stub busy wait.
