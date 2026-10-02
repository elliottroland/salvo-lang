# The scheduler runtime in Salvo — survey and plan

Working document (survey of 2026-10-01, read-only session). It answers the
ROADMAP item "How much of the scheduler runtime could be Salvo" (Recorded, not
scheduled): what the two hand-written schedulers contain today, which parts
can become one Salvo module over a small set of private intrinsics, what the
language and compiler would need for that, and in what order to do it.
Nothing here is decided. The items marked **DECISION** are the user's call.

**User guidance (2026-10-01):** module-private declarations in the runtime
module may behave differently from Salvo elsewhere. The runtime is what all
Salvo code runs on, `main` included, so it is expected to contain exceptions
such as starting threads explicitly and handling locks. A privilege that is
private to the runtime module therefore needs no language decision. Only
what becomes visible outside it does.

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
  2,000–2,700. The host stream table (about 200 lines) is not scheduling,
  but most of it is not host-specific either: its logic can move to Salvo
  too, leaving only the raw I/O calls in the host (§3.4).
- The language can already express most of the logic. The gaps are a linear
  lock guard (the natural Salvo form of Rust's `MutexGuard`), moving a value
  out of a field of a list element, a payload type that hides its real type,
  and how code that declares no effects (`send(reply, v) []`, host threads,
  the `Addr` codec) reaches the runtime (§6, G4). Under the user guidance
  these are privileges of the runtime module, so none needs a language
  decision while it stays private. A run-time-initialised module constant
  would be one way to spell the process-wide slot. The runtime also needs an
  O(1) queue, which std is getting anyway (§4.2).
- Most primitives have to be intrinsics rather than platform fns, because
  they are generic, linear, take function values or are tied to generated
  code. Three or four cold, concrete ones could be platform fns (§4.1).
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
7. **Pools and their threads are never released.** Neither the language nor
   the runtime has a way to retire a pool, so every worker thread lives until
   the process ends. This matters most for `thread()`: each dedicated actor
   gets a thread of its own, and when the actor dies the thread stays parked
   on the condition variable forever. A program that keeps creating
   dedicated actors and losing them leaks one thread each time. The fix is a
   cooperative stop (§4, "Why threads need no stop or handle"): retire a
   pool once nothing can be placed on it any more, for example once a
   `Dedicated` pool's only actor is dead and its task queue is empty.

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
- The raw I/O underneath the host stream table: the host stream objects and
  the read, write, flush and close calls on them (§3.4).
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

### 3.4 The host stream table

The first draft of this document kept the table in the host. That was
wrong: it is not scheduling, but most of it is not host-specific either.
Today it is `SalvoIn`/`SalvoOut` and `SalvoStreams` in the scheduler files
(about 185 lines in Kotlin, 235 in Rust), plus the `RawStreams`
implementations in `std/platform/stream/host.{kt,rs}` (about 190 lines
each), which do the same work twice.

What needs the host:

- the host stream objects (`InputStream`/`OutputStream`, `Read`/`Write`),
  the raw read, write, flush and close calls on them, and turning their
  exceptions into fault text;
- registration from host code: `HostRawFs` opening a file and the aws S3
  glue handing over a response body pass in a host object and get a handle
  back. That entry point stays a host-facing shim, like `SalvoReply`;
- `raw_receive`'s background read, which reads on a host thread and answers
  through a hosted reply [platform-reply].

What can be Salvo:

- the table itself, handle to entry, with the trap for a handle another
  provider minted [stream-provider];
- the read-ahead buffer, line splitting on `\n` and `\r\n`, and the position
  counted in bytes handed to the reader rather than bytes read ahead;
- failure recording: once a read fails, later reads report the end and the
  close reports why, and the same for writes at flush and close;
- strict UTF-8 decoding, which `str_of_bytes` already provides.

`HostRawStreams` would then be an ordinary Salvo handler over a few host
leaf operations (§4.1 discusses whether those are intrinsics or platform
fns).

The table needs the same two privileges as the scheduler: a process-wide
slot and locking. Each entry needs its own lock, as today, so a slow read of
one stream never blocks another; that is one `Lock<S>` per entry. The user
guidance grants those privileges to the runtime module only, so the table
lives there. It already shares the handle counter (`freshHandle`) with the
scheduler. `stream.host` reaches it through the lowering of its own
intrinsics, the same bridge §5.1 describes for `core.actor`, `time` and
`net`. The alternative, granting the privileges to std's host-facing modules
(`stream.host`, `fs.host`) as well, widens the exception beyond what the
user granted.

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
| Queue | none: std's public queue (§4.2) | | |
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
- **Why threads need no stop or handle.** Nothing in today's semantics stops
  or waits for a runtime thread. Pool workers and the timer thread loop
  forever, and the program ends when `main` returns. On Kotlin that works
  only because the threads are daemon threads, so creating daemon threads is
  part of `start_thread`'s contract. On Rust the process exits when `main`
  returns and takes its threads with it. Nobody waits for a thread to
  finish, because completion is observed through replies. Neither host can
  safely stop a thread from outside: Kotlin's `Thread.stop` is gone, and
  Rust never had one. The only safe stop is cooperative: set a flag in
  `Sched`, call `signal_all`, and the worker loop sees the flag and returns
  from its body, which ends the thread. That needs scheduler state, not a
  handle, and it is how retiring a pool would work (§2.3 item 7). A handle
  would be needed only to wait for a thread to exit or to name threads in
  diagnostics, and the deadlock report names actors, not threads. If the JVM
  program-end question (ROADMAP §4b, item 7) is settled with an explicit
  `exitProcess` after `main`, it needs no handle either.
- **Timeouts take a `Long` reading** of the monotonic clock rather than
  `time.Tick`, so the runtime does not import `time`, which itself depends on
  the runtime through `fire_after`.

### 4.1 Intrinsic or platform fn

The table above writes every primitive as an `intrinsic`. A
`platform fn` [platform-fn] is the other way to put host code under a Salvo
declaration: the host writes an ordinary function in std's platform root,
with its own compiler and IDE checking it, and the program calls a generated
wrapper. Its limits decide most cases. A platform fn cannot be generic, has
no effects, and the ABI has no opaque *platform types*, so it can neither
take nor return a host object or a Salvo function value. Its wrapper also
validates the result [platform-check], which is wasted on a hot path whose
types need no checking.

So a primitive has to be an intrinsic when any of these holds:

- it is generic (`Lock<S>`, `Held<S>`, `Dyn`, `erase`/`unerase`);
- it is linear or answers a projection (`Held<S>`, `state(h)`), which the
  checker reads off the intrinsic's declaration and deduction clause;
- it takes a Salvo function value (`start_thread`, `run_guarded`);
- it is tied to the shape of generated code (`activate`, which dispatches
  into the emitted `SalvoActor` impl inside a fault boundary);
- it has no host body at all (`sched()`, or a module constant instead).

What is left can be a platform fn: concrete signatures over plain values,
off the hot path.

| Primitive | Kind | Why |
|---|---|---|
| `Lock`, `Held`, `lock`, `unlock`, `state`, `wait`, `wait_until`, `signal_all` | intrinsic | generic, linear, projection, hot path |
| `sched()` | intrinsic, or a module constant | no host body |
| `start_thread` | intrinsic | takes a fn value |
| `here` / `set_here` | either | concrete (`Here` is a struct of `Int`, `Int?`, `Bool`), but read on every activation; intrinsic preferred for the hot path |
| `Dyn`, `erase`, `unerase` | intrinsic | generic |
| `Body`, `activate` | intrinsic | tied to generated code |
| `run_guarded` | intrinsic | takes a fn value |
| `report(line: Str)` | platform fn | concrete, cold |
| `exit_with(line: Str)` | platform fn if a platform fn may return `Never`; intrinsic otherwise | cold |
| `secure_bits() -> Long` | platform fn | concrete, cold (once per spawn) |
| FNV-1a over `Bytes` (what `key_hash` reduces to after encoding the key) | platform fn, or Salvo if `Long` gains xor and wrapping multiply | concrete |
| Host stream leaf calls (§3.4) | intrinsic, unless platform types exist | they take a host stream object |

That is about a dozen intrinsics against three or four platform fns, so
platform fns save little here. What they do give is that their bodies are
ordinary host code under std's platform root, edited with host tooling,
rather than lowering strings in `intrinsics.rs`.

The host stream calls are the one case where neither form fits cleanly,
because there are no opaque platform types. The options:

- (a) `intrinsic type HostIn`/`HostOut` with intrinsic read, write, flush
  and close. Simplest, and std-only, which is fine for std.
- (b) Platform fns over `Long` handles. The host then keeps a small map from
  handle to host object, so a sliver of the table stays in the host.
- (c) **Opaque platform types** in the ABI: `platform type HostIn`, a host
  class the program holds but cannot look inside. This would also serve
  customers, for example an SDK client handle held by a platform handler's
  caller. It is an ABI and language addition, so a **DECISION** (D8).

Recommendation: (a) for the port. (c) is worth considering on its own merits
in ABI.md.

### 4.2 A queue in std

The user intends to add a good queue to std anyway, so the runtime can use
the public one, and the private `Queue` primitive goes.

**What it gives back.** In lines, little: one primitive and about ten call
sites. The value is elsewhere:

- The runtime keeps no collection of its own. Every queue it needs is the
  public one: each actor's mailbox, each pool's task queue, the staged
  outbound frames and the frames parked for a node with no route yet.
- The FIFO users that exist today become O(1). Salvo's `List` lowers to
  `Vec`/`ArrayList`, and `remove_first` is `Vec::remove(0)` on Rust
  (`runtime/seq.rs`), O(n) per pop. Three examples already use a
  `Mut List` with `remove_first` as a queue of linear obligations: `actors`
  (`Desking.waiting`), `cluster` (`pending`) and `linearity` (`queue`).
- It is the same work either way: a runtime-private queue would need the
  same intrinsic lowering as a public one.

**What the runtime needs from it:**

- `T canbe linear`: entries hold `Dyn` payloads and replies;
- O(1) push at the back and pop at the front;
- removal at an index, for the gate, which delivers the awaited reply out of
  order. O(n) is fine because it is rare;
- finding an element by predicate, or iterating with indexes, for the
  gate's search by slot and for `deliverable`;
- `size`, and a `drain` that hands each linear element to a function, for
  an actor's death, mirroring `List`'s `drain`;
- iteration (`: Iter<self, T>`), as for the other collections.

**Implementation.** An intrinsic type lowered to the host's deque,
`VecDeque` on Rust and `kotlin.collections.ArrayDeque` on Kotlin, the way
`List`, `Set` and `Map` are lowered today. Both support index removal
(`VecDeque::remove`, `ArrayDeque.removeAt`). A ring buffer written in Salvo
over `Mut List<T?>` is the alternative. It would need G2's move-out of a
slot for linear elements and its own growth logic, and would be slower than
the native deque, so it gains nothing.

**Shape, a DECISION (D3).** The name, and whether it is FIFO only (`Queue`:
push back, pop front) or double-ended (`Deque`: both ends, plus `get`).
Both hosts give double-ended at the same cost, and stack and undo uses come
up often, so I would make it a `Deque` with a `Mut` constructor in the style
of the other collections (`mut_deque_of()`). Whether it gets a literal,
equality, hashing and `to_str` follows from how `List` does them.

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
    queue: Mut Deque<Entry>,
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

The runtime module is privileged (see the user guidance at the top): its
private declarations may start threads, hold locks, read a process-wide
slot and catch faults, none of which Salvo code elsewhere can do. The
restrictions below run the other way. The runtime implements the actor
surface, so it cannot use it. Inside `std/runtime.sv`:

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
- **G4. How code that declares no effects reaches the runtime.** The request
  was for a `Runtime` effect. An effect is the natural Salvo seam, and it is
  what would let a deterministic test runtime (single thread, virtual time;
  the "scheduler-owned virtual time" upgrade recorded in ROADMAP) be bound in
  place of the threaded one. The problem is that effects are not data
  [effect-not-data], and the operations that need the runtime are declared
  without effects: `send(reply, v) []`, `Addr` equality and codecs, host
  threads completing replies, the timer thread. Options:
  - (a) **A process binding for `Runtime`**: bound once at program start, and
    performed by the runtime's functions without being declared. Under the
    user guidance this is a runtime-module privilege, not a language change,
    as long as only the runtime module performs it.
  - (b) **Data, not an effect**: the module's functions take the
    `Lock<Sched>` from the `sched()` intrinsic. A test runtime then comes
    from configuration inside `Sched` (for example a virtual-clock field)
    rather than from binding another handler.
  - (c) **Thread it through `[spawn]`**: the existing capability would carry
    the runtime handle. That covers spawns but not sends, replies or host
    threads, so it does not work alone.
  - (d) **A module-level constant initialised at run time** (not taken:
    E4's module-level `use` replaced it, and D1 allows pure constants only):
    (b) with a
    declaration instead of an intrinsic, if module-level constants are added
    (the user is considering them for a math module). Constants come in two
    kinds, and only the second helps here:
    - *Pure* constants (literals, arithmetic, immutable struct and list
      literals) are what a math module needs. The runtime would use them for
      `MAIN_POOL`, the frame kind numbers and the report strings. Tidier,
      but they remove nothing from §4.
    - *Run-time-initialised* constants, evaluated once on first read:
      `const SCHED: Lock<Sched> = lock_of(init())`. The scheduler needs this
      kind, because `init()` draws a random node id and creates the main
      pool, so its state is not a compile-time value. It replaces the
      `sched()` intrinsic. Rust lowers it as `static LazyLock<…>` (the type
      must be `Sync`, which a lock is); Kotlin as a top-level `val`, which is
      already initialised lazily when its file class loads.

    A run-time-initialised constant of a type like `Lock<S>` is a global
    mutable variable whose mutability `Mut` cannot see, which is the hidden
    state Locality rules out. Outside the runtime module the language should
    offer pure constants only, or limit the other kind to types std marks
    safe to share across threads. Inside the runtime module, under the user
    guidance, it is simply one of the module's privileges. Constants do not
    replace the lock, guard, wait and signal primitives (those are about
    blocking, not storage), the thread-local (each thread needs its own
    copy), the fault boundary, erasure or threads.

  Recommendation: start with (b), or with (d) if module-level constants land
  first, so the port does not wait on anything. A handler
  `Scheduler(l: Lock<Sched>) of Runtime` would have exactly the same bodies,
  and by the [effect-handle] predicate it binds bare, with no monitor lock,
  because it has no state field, no fn-typed parameter and no `replyto`. So
  (a) can be added later without rewriting anything, once a second runtime
  implementation is wanted, and the guidance means it no longer needs a
  language decision then either.
- **G5. The fault boundary**: private (`activate`, `run_guarded`, §4). No
  decision as long as it stays private.
- **G6. Bootstrapping rules** (§5.4): engineering. Enforced by a test.
- **G7. Intrinsic lowerings that call emitted Salvo functions**: emitter
  work in both `intrinsics.rs` files (§7).
- **G8. A queue**: std's public queue (§4.2), whose shape is D3. The
  runtime needs no private one.
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

**Superseded by §11.5** (2026-10-02). Kept for the reasoning behind the
intrinsic plan.

Each step keeps both backends passing the full suite.

1. **Benchmarks** (§8). Record today's numbers.
2. **Move the host stream table** out of `scheduler.*` into its own runtime
   file. Independent and small.
3. **The queue in std** (§4.2; needs D3). Independent of everything else
   here: an intrinsic type over `VecDeque`/`ArrayDeque`, its docs page
   section and tests, and the three examples that use a `Mut List` as a FIFO
   rewritten onto it.
4. **Move the group protocol into `net.sv`** (§3.3; needs D5). Still host
   code in the scheduler, but frames 4–9 go to a control actor and the five
   group builders disappear. Shrinks step 6.
5. **Primitives and the empty module**: the §4 intrinsics in both backends
   (and the platform fns of §4.1, if that split is taken), `std/runtime.sv`
   with `Sched`, `init` and `sched()`, and Salvo tests for `Lock`/`Held` and
   `Dyn`. G1–G3 checker work lands here.
6. **Port the scheduler**: the local scheduler and routing (frames 0–3,
   proxies, credits) together, because they share the actor table. Switch the
   emitters' lowerings, delete the host code, and move `runtime_tests.rs` to
   `runtime.test.sv`. This is the large step. It can be split by keeping
   the host wire code temporarily and having it call into the Salvo tables
   through exported runtime functions, at the price of throwaway glue.
7. **Port the stream table** (§3.4): the table, buffering, lines, positions
   and failure recording into the runtime module; `HostRawStreams` becomes a
   Salvo handler over the host leaf calls; the host keeps registration and
   the raw I/O. Needs only step 5's primitives, so it can also come before
   step 6 as a smaller first use of them.
8. **Optional: frames as canonical-encoded values** (G9).
9. **Optional: the `Runtime` effect** (G4(a)) and a deterministic test
   runtime, if wanted.
10. **Docs**: BACKEND_SPEC.kotlin.md / BACKEND_SPEC.rust.md runtime sections,
   ABI.md's host-project contents, ROADMAP (remove the "Recorded" entry),
   COMPLETED.md. The LANGUAGE_SPEC rules ([actor-*], [waitfor-pump],
   [task-mint], [time-timer], [addr-*], [stream-table]) describe behaviour
   and should not change. Their labels move from the host runtimes to
   `std/runtime.sv`.

## 10. Decisions for the user

- **D1 (G4). Decided 2026-10-02: pure constants only, for now.** The runtime
  half is superseded by E4: the scheduler and `RuntimeHost` are reached
  through module-level `use` bindings, so the runtime needs no constants.
  Run-time-initialised constants are not part of the language.
- **D2 (G2).** Settled by the user guidance while the move-out operation
  stays private to the runtime. A decision only if it is wanted as public
  std API.
- **D3 (§4.2). Decided 2026-10-01: `Deque`.** Whether it is an intrinsic
  over the host deque or a generic platform type is now open under §11 (D10).
- **D4 (G9). Decided 2026-10-02: yes.** Frames become canonical-encoded
  Salvo values; done in the port (§11.5 step 11).
- **D5 (§3.3). Decided 2026-10-02: yes.** The group protocol moves to
  `net.sv` before the port (§11.5 step 4).
- **D6. Decided 2026-10-02:** the four benchmarks of §8 on both backends,
  and the port stays within 1.5× of today's numbers on each.
- **D7 (§2.3). Decided 2026-10-02:** every finding is part of this work,
  ordered by the author; §11.5 places each one.
- **D8 (§4.1). Decided 2026-10-01: add platform types** (`platform type
  HostIn`, an opaque host class a Salvo program holds but cannot look
  inside).
- **D9 (§4.1). Answered 2026-10-01: not worth it as posed.** Splitting a few
  cold primitives into platform fns while the rest stay intrinsics is not
  the goal. The user's goal is the opposite: as few intrinsics as possible,
  because each is something a new backend must implement. §11 is the plan
  under that goal.
- **D10 (§11).** Which of the language expansions E1–E10 to take. The user
  is generally happy with all of them (2026-10-01); E9 is now rejected for
  actors in favour of E10 (§11.6). E3 is the `Parker` platform type
  (decided 2026-10-02), with `wait until` in monitors kept as a possible
  user-facing feature.
- **D11 (§11.7). Decided 2026-10-02.** The **test kind** chooses the
  runtime: an actor test kind and `proptest` set up the virtual runtime
  before running, and plain `test` stays threaded. The harness writes **one
  synthesized program per mode**, with a fresh `Sched` per virtual test.
  `proptest` **generates the seed** as an input, so a failing interleaving
  shrinks and replays like a failing value.

## 11. Platform code instead of intrinsics (user direction, 2026-10-01)

**The goal.** Every intrinsic type or function is something a new backend
has to understand and lower inside its emitter. A platform handler is
different: its requirements are stated as a generated host interface, and a
backend meets them with an ordinary host file in std's platform root. So the
aim is a `Runtime` *platform handler* that supplies what only the host can
do, behind a host interface, with the scheduler written in Salvo on top and
as few intrinsics as possible. The effect is there to give the host part an
interface, not to make the runtime reachable as an effect by programs.

Sections 4–7 assumed the opposite (intrinsics everywhere), so this section
supersedes §4's table where they differ.

### 11.1 What blocks each primitive from being platform code today

| Primitive (§4) | Why it is not platform code today | Expansion that unblocks it |
|---|---|---|
| `Lock<S>`, `Held<S>`, `state(h)`, waits | generic over Salvo state; `state(h)` returns a borrow, and borrowed platform results are refused (ABI.md D10 C5) | a monitor for the state, plus `Parker` (E3) for waiting |
| `sched()` process slot | no host body; needs a binding reachable from effect-less sites | E4 |
| `start_thread(body)` | takes a Salvo fn value | E2 |
| `here` / `set_here` | nothing: concrete struct, a platform member already | none |
| `Dyn`, `erase`, `unerase` | generic | E1 (a generic platform type) |
| `Body`, `activate` | tied to generated code's actor trait | E10 + E2 |
| `run_guarded` | takes a fn value | E2 |
| `Deque<T>` | a collection with value semantics | stays an intrinsic beside `List` (§12.3, decided) |
| `report`, `secure_bits`, monotonic clock | nothing: concrete | none |
| `exit_with` | returns `Never` | E6 |
| Host stream calls | take a host object | D8 (platform types, decided) |

### 11.2 The expansions

Each is listed with what else it would be good for, since the point is to
grow the language in ways that pay off outside the runtime too.

- **E1. Generic platform types and functions.** The host writes generic code
  over an opaque type parameter that it can only store and move: Kotlin
  `class Deque<T>`, Rust `struct Deque<T>` with the bounds the emitter
  always adds (`T: Send + 'static`). Elsewhere: host-backed containers and
  caches, typed SDK wrappers. With E1, `Deque` and even the existing
  collections could be platform types in std rather than intrinsics, which
  would be the largest single cut in what a new backend's emitter must
  know. The lock *could* be done this way too, but its `state(h)` returns
  a borrow, against C5, which is why the state is a monitor and waiting is a
  `Parker` (E3) instead.
- **E2. Function values in platform signatures**, for effect-free fn types
  (`() -> None`, `once (Bytes) -> None`). The host receives its own closure
  type (`() -> Unit`, `Box<dyn FnOnce() + Send>`). Captured handles travel
  inside the closure, so the host never supplies an effect. Elsewhere:
  callbacks into SDKs, event listeners, a host executor. This one expansion
  makes `start_thread` and `guarded(f: once () -> None) -> Str?` ordinary
  members of the runtime's platform handler.
- **E3. Waiting, by a `Parker` platform type — revised 2026-10-02 (user:
  "I like the Parker idea").** The scheduler sleeps until another thread
  changes the state in five places: a worker with nothing to run, a sender
  on a full mailbox, a `waitfor` frame, the timer thread (until the
  earliest deadline), and a remote send with no credits. A monitor
  [monitor-handler] gives mutual exclusion but cannot sleep. The danger in
  sleeping is the **lost wakeup**: if the state changes between "nothing to
  do" and going to sleep, the wakeup arrives before anyone sleeps.
  - **Chosen: a non-generic platform type**, needing no language change
    beyond platform types (D8):

    ```
    platform type Parker
    //   park(p)                      sleep until unparked
    //   park_until(p, nanos: Long)   the same, or until the deadline
    //   unpark(p)                    wake it
    // Rust: std::thread::park / Thread::unpark
    // Kotlin: java.util.concurrent.locks.LockSupport
    ```

    Both hosts give `unpark` a token: an `unpark` that arrives before the
    `park` makes the next `park` return at once, which closes the
    lost-wakeup window without holding a lock while sleeping. A wait is
    then: a member of the scheduler monitor checks the condition and, if it
    cannot proceed, records the caller's parker in the state and returns;
    the caller parks outside the lock; every member that changes the state
    unparks the recorded parkers (today's `signalAll`); the caller calls
    the member again, in a loop that also absorbs spurious wakeups. The
    Salvo side needs a few helpers for registering and waking, and each
    state change must remember to wake. A test that every member ends in a
    wake is cheap to add.
  - **Not taken for the runtime: `wait until <condition>` in monitor
    members**, lowered as a condition-variable wait on the monitor's lock.
    More concise, and useful to users (bounded buffers, latches), but it is a
    language feature: checker and both emitters, the deadlock graph
    [actor-deadlock-cycle] learning that a waiting member does not hold the
    lock, and condition variables in every backend's monitor lowering. Kept
    as a possible user-facing feature, independent of the runtime.
- **E4. A module-level binding.** `use` at module scope: bound once,
  lazily, on first use, and available to every function in the module
  without being declared.

  ```
  // std/runtime.sv
  use HostRuntime()          // the platform handler: threads, guarded, here, …
  use Scheduler()            // the monitor holding Sched

  fn send(addr: Int, msg: Dyn) [] -> None => !msg { … }   // performs both, declares neither
  ```

  - **Why it is needed.** One scheduler serves the whole process, and these
    sites must reach it without being handed it: operations declared `[]`
    (`send(reply, v)`, a send on an actor stub, `Addr` equality); generated
    code at `waitfor` and `replyto` sites in arbitrary functions (the
    `[waitfor]` capability is deleted); the `Addr` codec deep inside a
    generated `decode`; host threads (a platform handler completing a hosted
    reply from an SDK thread, the TCP reader's `externalBegin`/`End`); and
    the runtime's own calls to `RuntimeHost`.
  - **Why the alternatives fall short.** Carrying a runtime reference in
    every `Addr` and `Reply` covers sends, replies and host threads, makes
    each addr copy an `Arc` clone on Rust, and still leaves `waitfor` sites
    and decoders out. Putting it in the thread-local beside `Here` misses
    foreign host threads. Threading it through `[spawn]` covers spawns only.
    Today's `static STATE: OnceLock` / `object SalvoSched` is the honest
    model of a process-wide fact, and the language has to say it somewhere.
  - **Why `use` and not a constant.** What is bound is handlers, the
    `Scheduler` monitor and the `RuntimeHost` platform handler, and effects
    are not data [effect-not-data], so a module constant cannot hold them.
  - **Semantics.** Bound at first use, for the life of the process. Visible
    to the module's own functions only, never to importers. The bound
    handler must be shareable across threads: a monitor, a stateless
    handler, or a `threadsafe` platform handler. The initializer may read
    configuration set by the entry point before first use, which is what
    keeps a virtual runtime possible (§11.7).
  - **Limited to the runtime module (user decision 2026-10-02).** A
    module-level `use` anywhere else is an error. Widening it later is a
    separate decision, because elsewhere it would be hidden state by
    another name.
- **E5. Platform types** (D8, decided), including **linear** ones, so a
  platform type can carry an obligation the checker enforces (a host reply,
  for example).
- **E6. Platform members that return `Never`**, for `exit_with`. Small.
- **E7. `Addr<E>`, `Reply<T>` and `Pool` as Salvo types** in `core.actor`
  instead of intrinsic types: `Addr<E>` a struct over an index, with `E` as
  a phantom parameter; `Reply<T>` a linear struct; `Pool` a struct
  over an index. Host code still sees them through the generated ABI types
  (ABI.md D4). Needs phantom type parameters to be accepted, which I have not
  checked.
- **E8. A std-internal module**: `std/runtime.sv` exports to the rest of std
  but cannot be imported by programs. With it, `core.actor`'s `pool`,
  `thread`, `watch`, `on_idle` and `send`, `time`'s `fire_after` and `net`'s
  private intrinsics become ordinary Salvo functions calling the runtime,
  instead of intrinsics whose lowering calls it. That is 38 intrinsic
  declarations today (10 in `core.actor`, 25 in `net`, 3 in `time`), most of
  which would stop being intrinsics. Small, and it needs no decision while
  only std can use it.
- **E9. Closures instead of erasure — revised 2026-10-01: rejected for
  actors, kept for tasks.** The first version had the mailbox hold
  `once () -> None` thunks that capture the message and the actor's state.
  Worked through (§11.6), that has two problems. Rust can only let several
  queued thunks share one actor's state behind a runtime-checked lock, which
  costs four atomic operations per message plus cache-line traffic between
  the sending and running cores. And a sender holds only an `Addr<E>`, not
  the handler, so it cannot build a thunk over the handler's state without
  a per-actor closure inside every addr. What survives:
  - **Tasks** are closures already (`TaskBody` is a `FnOnce`) and own their
    captures, so a task continuation stays `once (T) -> None` with no
    sharing and no lock.
  - **Actors** keep today's ownership model: the runtime *moves* the body
    out of the table for an activation and back afterwards, which proves
    exclusivity with no extra cost. Two things make that platform code
    rather than intrinsics:
    - **E10. An owned, mutable fn value**: a fn type whose value is moved,
      never copied, and may mutate its captures (Rust
      `Box<dyn FnMut(Entry) + Send>`, Kotlin `(Entry) -> Unit`). The
      emitter lowers each actor handler to one such value at the spawn; the
      runtime holds it in a `Body?` field, takes it, calls it and puts it
      back. Only generated code creates one at first, so the checker rule
      for writing one in Salvo can wait.
    - **`Dyn` as a generic platform type** (E1): `platform type Dyn` with
      generic platform fns `erase<T>`/`unerase<T>` (Rust
      `fn erase<T: Any + Send>(v: T) -> Dyn`, a downcast back; Kotlin a
      cast). Message payloads and replies aimed at actors stay erased
      exactly as today.
  - The fault boundary is one platform member, `guarded(f: once () -> None)
    -> Str?`, either way.

### 11.3 What is left

With E1–E8 and E10, the runtime adds **no new intrinsics**, and
removes most of the 38 existing actor, net and time intrinsics. A new backend
then provides:

1. **The core language**, including monitors, fn values, generics, linearity, module bindings (E4) and the canonical
   wire encoding (`encode`/`decode`, which stay intrinsics: they dispatch on
   type).
2. **The platform ABI generator** for its host language: interfaces,
   adapters, platform types, fn values at the boundary.
3. **std's platform files** for it, of which the runtime's share is one
   `threadsafe platform handler` of a `RuntimeHost` effect, roughly:

```
effect RuntimeHost {
    fn start_thread(body: once () -> None) -> None      // a daemon thread (§4)
    fn guarded(body: once () -> None) -> Str?           // the fault boundary
    fn here() -> Here
    fn set_here(h: Here) -> Here
    fn monotonic_nanos() -> Long
    fn secure_bits() -> Long
    fn report(line: Str) -> None
    fn exit_with(line: Str) -> Never                    // E6
    // and `platform type Parker` with park, park_until, unpark (E3)
}
```

   plus `platform type Dyn` with generic `erase<T>`/`unerase<T>` (E1), the
   stream leaf calls over `platform type HostIn`/`HostOut`, and the `Deque`
   intrinsic beside `List` (§12.3). I estimate 100–150 host lines per backend for
   `RuntimeHost`, against 2,000–2,700 today.
4. **The language forms** (`spawn`, `replyto`, `waitfor`, `send fn` calls,
   actor handlers), which stay emitter work in any design. They lower to
   calls of Salvo runtime functions, to an owned fn value per actor (E10)
   and to task closures, which is less than lowering them to a host
   runtime's API.

What stays hand-written host code outside the interface: the host-facing
shims (`SalvoReply.hosted()`, `SalvoHostReply` and Rust's drop report,
`externalBegin`/`End`) become generated ABI code over the Salvo `Reply` type
plus a few lines per backend.

### 11.4 Costs and risks of this variant

- **More language work up front.** E2, E4 and E10 are each real features
  with checker, both-emitter and docs work. The intrinsic plan (§4) needs none of them.
- **Adapter overhead.** Every `RuntimeHost` call goes through a generated
  adapter. The hot ones (`here`, `set_here`, `guarded`, `erase`, `unerase`)
  take and answer types whose checks are empty, so the adapter should inline
  away, but that belongs in the §8 benchmarks.
- **No per-actor lock.** Keeping the move model (E10) costs nothing over
  today; the rejected thunk design would have (§11.6).
- **Order.** E1, E3, E4, E5, E8 and E10 block the port (E3's `Parker` is a
  platform type, so E5 now does too); E6 only blocks moving `exit_with` out
  of intrinsics, and can follow.

### 11.5 The sequence

This is the order of work, and it supersedes §9 (written for the intrinsic
plan). It folds in the survey findings of §2.3 (user decision D7,
2026-10-02, ordering left to the author) and the decisions of §10. Each step
keeps both backends passing the full suite.

**Before the port, in today's host runtimes:**

1. ✅ **Benchmarks** (D6), built 2026-10-02: `bench/scheduler/`, run by
   `tools/bench-scheduler.sh` (Rust `rustc -O`, Kotlin `kotlinc`). Fan-out
   is 1,000 actors × 100 messages on a 2-worker pool, then every total;
   timers are 10⁴ deadlines of 1 ms set by one actor. The baseline, medians
   of five runs on an Apple M1 Pro (rustc 1.98, kotlinc 2.4 / JRE 26):

   | | ping-pong 10⁶ | fan-out | tasks 10⁵ | timers 10⁴ |
   |---|---|---|---|---|
   | Rust | 402 ms | 211 ms | 29 ms | 58 ms |
   | Kotlin | 367 ms | 355 ms | 37 ms | 102 ms |
   | budget (1.5×) | 603 / 551 ms | 317 / 533 ms | 44 / 56 ms | 87 / 153 ms |

   Ping-pong is the noisiest (Rust 311–478 ms, Kotlin 299–464 ms across the
   five runs), so a comparison should use medians of five.
2. ✅ **Forged identities stop growing the actor table** (built
   2026-10-02) (§2.3 item 2). First
   because a remote peer can trigger it: `importAddr` answers one shared
   dead entry per process instead of minting a new one per failed identity.
   A few lines in each runtime, and worth not waiting for the port.
3. ✅ **Move the host stream table** out of the scheduler files into its own
   runtime file (built 2026-10-02: `hoststreams.rs` / `hoststreams.kt`).
4. ✅ **The group protocol moves to `net.sv`** (D5, §3.3), built
   2026-10-02. The runtime carries one CONTROL frame kind (4: to, from,
   channel, payload) to the actor listening on the channel; the handshake,
   introductions, departures and member sharing are Salvo over the
   canonical encoding; NAMED is gone (a replica shares on its node group's
   `joined`, and a node group's `subscribe` now replays the nodes it already
   knows). Left in the runtime until step 11: the route-stub views (read
   synchronously on senders' threads) and the peer protocol store behind
   the exported `peer_protocol`. The scheduler shrank from 1,994 to 1,640
   lines (Kotlin) and 2,720 to 2,160 (Rust). Three Rust emitter defects
   found on the way are ROADMAP item 0c.
   The original plan for this step: Frames 4–9 go to a
   control actor `net.sv` registers, `node_left(node)` marks a departed
   node's proxies dead, and the five group builders go. The scheduler keeps
   frames 0–3.

**The language and ABI expansions:**

5. ✅ **E8** (std-internal module) and **E6** (platform members returning
   `Never`), built 2026-10-02: `std/runtime.sv` exists (empty but for its
   doc) and a program importing it is refused [mod-std-internal]; Rust's
   platform signatures answer `-> !` for `Never` [platform-never], which
   closed a silent hole — a Rust host returning from one used to fall
   through.
6. ✅ **`Deque`** (D3) as an intrinsic beside `List` (§12.3), built
   2026-10-02 [col-deque]; the generic platform types (E1) and `Dyn` move
   after step 7, since they build on plain platform types. Built with the three examples that use a `Mut List` as a FIFO
   (`actors`, `cluster`, `linearity`) rewritten onto it, and **`Dyn`** with
   `erase`/`unerase`.
7. **E2** (effect-free fn values in platform signatures) and **E5**
   (platform types, including linear ones).
8. **E3, the `Parker`** platform type.
9. **Monitor reentrance agrees on both backends** (§2.3 item 4). Kotlin's
   `__Mon_E` refuses a re-entry through the handle with a trap naming it,
   as Rust's `Mutex` would deadlock; a member calling a sibling member
   directly is unaffected. Before the port, because the scheduler becomes a
   monitor there, and reentrance is the mistake most likely to differ.
10. **E4** (module-level `use`, runtime module only), **E7** (`Addr`,
    `Reply`, `Pool` as Salvo types) and **E10** (owned, mutable fn values,
    emitter-generated only).

**The port:**

11. **The scheduler in Salvo**: the local scheduler and routing together,
    `RuntimeHost` as the platform handler, actor bodies as E10 values moved
    in and out, payloads as `Dyn`, tasks as closures, waiting through
    parkers. Folded in:
    - **Frames 0–3 as canonical-encoded values** (D4), so no hand-written
      frame code survives the port.
    - **No table grows without bound** (§2.3 item 1): waiters live in a
      map and are removed when their wait ends; a dead actor keeps only a
      tombstone (its index and exit reason), with its body, queue and
      watchers dropped. Indexes are not reused, because a stale addr must
      keep meaning "the dead actor" [actor-watch].
    - **Capability bits from OS entropy on both backends** (§2.3 item 3),
      through `RuntimeHost.secure_bits`.
    - **`std.test`'s `trapped_by` and the runtime's `guarded` become one**
      `RuntimeHost` member (§11.7).
    - `runtime_tests.rs` moves to `std/runtime.test.sv`.
    - The step-1 benchmarks are re-run against the 1.5× budget.
12. **The route stub waits instead of polling** (§2.3 item 5): a stub whose
    pick answers `None` parks until its group's view changes, rather than
    sleeping 1 ms and asking again.
13. **Pools retire** (§2.3 item 6): a pool on which nothing can be placed
    any more (first case: a `Dedicated` pool whose one actor is dead and
    whose queues are empty) is marked retired, its workers are unparked,
    see the flag and return, which ends their threads.
14. **The stream table** onto `HostIn`/`HostOut` and the Salvo table in the
    runtime module (§3.4).

**After the port:**

15. **The virtual runtime** (D11, §11.7), with the harness writing one
    program per mode, as the actor test kind and `proptest` arrive.
16. **Docs**: BACKEND_SPEC.kotlin.md / BACKEND_SPEC.rust.md runtime
    sections, ABI.md (platform types, generic platform code, fn values at
    the boundary), LANGUAGE_SPEC rules for each expansion, ROADMAP (remove
    the "Recorded" entry), COMPLETED.md. The behaviour rules ([actor-*],
    [waitfor-pump], [task-mint], [time-timer], [addr-*], [stream-table])
    stay; their labels move to `std/runtime.sv`.
17. **Optional**: the other collections onto generic platform types.

Module-level constants (D1) are independent of this sequence.

### 11.6 Why the per-actor lock appears, and why it is avoidable

Rust lets a value be mutated through `&mut` only when the compiler can see
that nothing else can reach it at the same time. Exclusivity can be shown
two ways: by *moving* the value, so only one owner exists, or by *locking*
it, so a runtime check stands in for the proof. Moving is free; locking
costs atomic operations. The scheduler guarantees that one actor's
activations never overlap [actor-kind], but that is a fact about the
scheduler's logic, which the Rust compiler cannot see. So the question is
which of the two the design lets Rust use.

**Today: the body is moved.** The sender knows only the protocol
(`Addr<Counter>`), so a message is data:

```rust
salvo_send(addr, Box::new(__Msg_Counter::Bump(n)));     // one allocation
```

and the worker moves the body out of the table for the activation:

```rust
let mut body = s.actors[addr].body.take().unwrap();     // move out, under the scheduler lock
drop(s);                                                // release the scheduler lock
body.handle(&ctx, msg);    // a dyn call; inside, `msg.downcast::<__Msg_Counter>()`
                           // compares one type id
s = lock.lock().unwrap();
s.actors[addr].body = Some(body);                       // move back
```

The scheduler lock is taken anyway, and it is what orders memory between the
worker that ran the last activation and the one that runs the next. Proving
exclusivity costs nothing on top.

**The rejected thunk design: the state is shared.** If the mailbox holds
closures that each capture the actor's state, several of them can be
queued at once, each holding the state. That is sharing, so Rust needs a
reference-counted pointer and a lock:

```rust
let state = Arc::new(Mutex::new(Counting::new()));      // at the spawn

// per message:
let st = Arc::clone(&state);                            // atomic increment
let thunk: Box<dyn FnOnce() + Send> = Box::new(move || {
    st.lock().unwrap().bump(n);                         // atomic compare-and-swap, then a release store
});                                                     // `st` dropped after the run: atomic decrement
```

That is four atomic operations per message that today's code does not do.
Uncontended, each is roughly 5–20 ns depending on the CPU. The larger cost
is the cache line: the reference counts live in the same allocation as the
lock and the state, so a sender on one core increments a count on the
actor's line, and the worker on another core then has to pull that line
back. On a pool that is the normal case, and a cross-core line transfer is
on the order of 50–100 ns. Against today's per-message cost (an allocation
and free, the shared scheduler lock taken twice, a `notify_all`), I would
expect this to add something like 10–30% to a ping-pong round trip. That
is a guess until the §8 benchmarks run.

The design also has a gap of its own: the sender holds only an `Addr<E>`, so
it has no handle on the handler's state to capture. Closing that would need
a per-actor delivery closure carried inside every addr (making each addr
copy an `Arc` clone too), or erasing the message anyway.

**The plan instead: keep the move, without intrinsics.** The actor body
becomes an owned, mutable fn value (E10), which the runtime moves exactly as
it moves the trait object today:

```rust
// emitted at the spawn: the closure owns the handler state
let mut h = Counting::new();
let body: Box<dyn FnMut(Entry) + Send> = Box::new(move |e| match e {
    Entry::Delivered(m) => match unerase::<__Msg_Counter>(m) {
        __Msg_Counter::Bump(n) => h.bump(n),
        __Msg_Counter::Total(out) => h.total(out),
    },
    Entry::Answered(slot, v) => h.resume(slot, v),
});
```

The machine code is the same as today's: one allocation per message, a type-id
compare, an indirect call, and no extra atomics. What changes is where it is
defined: `Box<dyn FnMut>` is the host's ordinary closure type rather than a
runtime trait, and `Dyn` with `erase`/`unerase` is a generic platform type
(E1) rather than an intrinsic.

Two remaining Rust details:

- **A fault inside an activation.** Today a panicking body is lost with
  the unwind, and the actor is marked dead. The closure behaves the same:
  `guarded` catches the panic, the `Box` is dropped, and the actor is dead.
  There is no lock to poison.
- **Kotlin** has none of this. The JVM needs no proof of exclusivity, and
  the scheduler's lock already orders memory between consecutive
  activations, so Kotlin lowers E10 to a plain `(Entry) -> Unit`.

### 11.7 A virtual runtime for tests, under a module-level binding

A module-level `use` (E4) does not rule out a virtual runtime for tests:
single-threaded, deterministic, with time advanced by the scheduler rather
than by the clock. It does decide where the swap happens.

**The swap point is the layer under the scheduler, chosen at start-up.**
The Salvo `Scheduler` already has most of what a virtual runtime needs. A
`waitfor` serves its own pool while it waits, so a frame running on main can
drive work. The scheduler already knows when nothing can run ([actor-on-idle],
the deadlock report). So a virtual runtime is the same Salvo scheduler in a
different mode over a different host:

- **No worker threads.** `pool(n)` records the pool but starts nothing, and
  main's thread serves every pool while it waits.
- **Virtual time.** When nothing is runnable and a deadline is pending, the
  scheduler moves its clock to the earliest deadline and fires it, instead
  of sleeping. `monotonic_nanos` answers the virtual clock. This is the
  "scheduler-owned virtual time" upgrade the ROADMAP records, and it would
  make the hand-written `ManualTime` fake and the six-line test clock
  unnecessary.
- **Seeded randomness.** `secure_bits` answers from a seed, so addr bits
  and anything derived from them repeat run to run.
- **Parking means deadlock.** With one thread, a `park` can never be woken
  by anyone, so reaching one is reported as the deadlock instead.

The order the scheduler picks work in is already deterministic (actors in
index order, tasks before activations, insertion-ordered maps and sets), so
nothing extra is needed for repeatable interleavings.

In code, the binding's initializer picks the host from configuration the
entry point sets before first use:

```
use runtime_host()         // HostRuntime() or VirtualHost(seed), per the start-up setting
use Scheduler()
```

This is why E4's semantics must let the initializer read start-up
configuration. A module-level binding fixed at compile time would leave a
virtual runtime only as a separate build of the program.

**Per-test isolation is easier in the virtual mode than in the threaded
one.** `salvo test` runs a suite in one process today, and the threaded
scheduler is a process-wide static, so actors from one test are still
there in the next. A threaded runtime cannot safely be reset between tests,
because worker threads may still be inside it. The virtual runtime has no
threads, so between tests the harness can replace the whole `Sched` state
with a fresh one (a `reset` member of the monitor), and each test starts
from an empty scheduler.

**What a virtual test cannot use:** platform handlers that run threads of
their own (`HostTcpTransport`, a real SDK client), since their work arrives
from outside the virtual clock. The `Mem*` fakes (`MemTransport`, `MemFs`)
are what such a test uses, and they are Salvo already. In virtual mode,
`externalBegin` can report that a host thread was opened, rather than let
the test lose determinism silently.

Who chooses the mode, and how, is open: a `salvo test` flag, a manifest
setting, or a per-test annotation. That is part of D11.

**The test kind chooses (user, 2026-10-02).** `test` is the only kind
today, but `proptest` and a kind for testing actors are planned, and a test
kind can set the runtime up before its body runs. So the mode needs no flag
or setting of its own:

- an **actor test** runs on the virtual runtime with a fresh scheduler;
- a **`proptest`** runs on it too and generates the seed as one more input
  (user, 2026-10-02), so a failing interleaving shrinks and replays like a
  failing value;
- a plain **`test`** keeps the threaded runtime, as today.

One consequence for the harness. It runs a suite as one synthesized program
in one process (docs/language/Testing.md, "The runner works by writing a
Salvo program"), and the module-level binding is bound once per process.
After a plain test has started worker threads, the process cannot switch to
the virtual runtime safely, since those threads may still be inside the
threaded scheduler. Options:

- (a) **One program per mode.** The harness already writes the program, so
  it writes two: one for the threaded tests and one for the virtual ones,
  with a fresh `Sched` per virtual test. **Chosen (user, 2026-10-02).**
- (b) **Virtual runs first.** Every virtual test runs before any threaded
  one in a single program. This works, but it couples the report's order to
  the runtime mode.
- (c) **Every test virtual unless it asks otherwise.** This makes the
  threaded runtime the exception in tests, which may be right eventually,
  but changes what today's plain tests exercise.

A related precedent: `std.test` already has a private fault boundary,
`intrinsic fn trapped_by(body: () -> Str?) [] -> Str?`, which is how the
harness catches a trap. It has the same shape as the runtime's `guarded`
(§11.2, E10), so the two should be one primitive, and under §11 a
`RuntimeHost` member that `std.test` calls rather than an intrinsic of its
own.

## 12. Platform types (decided 2026-10-02)

The user agreed the whole proposal on 2026-10-02, including `Deque` staying
an intrinsic and `canbe Mut` only on linear platform types. Design for D8. When agreed, the rules move to ABI.md and LANGUAGE_SPEC.md (a
`[platform-type]` rule); this section then shrinks to a pointer. Scope as
the user set it: platform types do **not** replace the intrinsic types that
`canbe Mut` (`List`, `Map`, `Set`, the sorted collections, `Str`, `Bytes`),
which need special handling in emission. They are designed to cover what
the runtime needs, and anything else that is intrinsic today only because
the host has to provide it.

### 12.1 What the runtime needs from them

| Type | What it is | Kind (§12.2) |
|---|---|---|
| `Parker` | one thread's park/unpark token, recorded in the scheduler state while its owner sleeps | threadsafe handle |
| `Dyn` | an erased payload, made by `erase<T>` and taken apart by `unerase<T>` | linear, generic fns over it |
| `HostIn`, `HostOut` | a host stream object under the Salvo stream table (§3.4) | linear, `canbe Mut` |

Beyond the runtime, the obvious customer is a handle to a host object a
platform handler's caller holds: an SDK client, a connection, a native
buffer.

### 12.2 The proposal

- **P1. A platform type is an opaque handle.** Salvo sees a name, type
  parameters and modifiers, never contents. Declared `platform type Name<T>`
  (`export` to export it), implemented by a host class or struct of the
  **same name** in the module's implementation file [platform-tree]. The
  program refers to the host's type directly; there is no adapter around a
  value.
- **P2. Three kinds, by modifier**, because copying has to mean one thing:
  - **plain** (`platform type Client`): copying a value shares the host
    object, as every Kotlin reference does. Rust requires `Clone` and
    expects it to be cheap (an `Arc` inside). No `Mut`: a mutation the host
    makes inside the object is not a Salvo mutation, so the parameter is
    plain.
  - **`threadsafe`** (`threadsafe platform type Parker`): a plain handle
    that may be used from several threads *at the same time*. Rust adds
    `Sync`. The same word and the same trust as `threadsafe platform
    handler` [threadsafe-platform].
  - **`linear`** (`linear platform type HostIn`): exactly-once, never
    copied, consumed by a platform fn that says `=> !s` [linear-obligation].
    Rust does not require `Clone`. Because a linear value has one owner,
    it **may** declare `canbe Mut`, and a `Mut HostIn` parameter lowers to
    `&mut HostIn` on Rust, which is how a read that advances the host
    stream is written without interior mutability. `canbe Mut` on a
    non-linear platform type is refused: with copies sharing one object, a
    `Mut` permission would not mean what it means for a struct.
- **P3. Every platform type is sendable and `noremote`.** Sendable, so it
  can sit in actor state, a message or a `Reply<T>` [actor-sendable]; Rust
  requires `Send`. `noremote` is implied, since a host object has no wire
  form [noremote]; using one where a value crosses the wire is the existing
  error.
- **P4. Operations are platform fns and platform handler members.** No
  member syntax on the type: constructors and operations are ordinary
  `platform fn`s taking or answering it, reached with dot notation like any
  fn (`p.unpark()` is `unpark(p)`). Equality, hashing, ordering and `to_str`
  exist only when declared as platform fns, since comparison is a
  capability [op-equality] [cmp-groups]. `by auto` is refused, as for
  intrinsic types: there are no fields to stamp over [comptime-fields].
- **P5. Generic parameters are opaque to the host (E1).** `platform type
  Box<T canbe linear>` and `platform fn erase<T canbe linear>(v: T) -> Dyn`:
  the host may store, move and hand back a `T`, nothing more. Rust gets
  `T: Send + 'static` on every parameter, always and nothing else (which
  also gives `Any`, which is what `unerase` downcasts with). Kotlin gets a
  plain type parameter.
- **P6. Nothing to check at the boundary** [platform-check]: a
  platform-type value is opaque, so the wrappers pass it through. What a
  generic platform fn answers as a `T` is checked as the `T` it is at the
  call site, which is what the wrappers already do for a concrete type.
- **P7. Borrowed results stay refused** (ABI.md D10 C5). The runtime needs
  none: `Parker` and `Dyn` answer owned values, and the stream calls answer
  `Bytes`. The narrow relaxation worth keeping in mind for later is a
  result `proj(p)` where `p` is the only borrowed parameter, since Rust's
  lifetime elision then needs no annotation.
- **P8. The ABI files.** `salvo platform generate` writes a skeleton for a
  platform type into the implementation file (Kotlin `class Parker`, Rust
  `pub struct Parker` with the derives P2 requires). The interface file
  states the contract the host compiler can check: on Rust a static
  assertion per type (`Send`, plus `Clone` for plain and threadsafe, plus
  `Sync` for threadsafe), so a wrong host type is a host compile error
  rather than a mystery in generated code. On Kotlin the contract is
  documented, as `threadsafe` is today. A program reaching a platform type
  whose implementation is missing is the existing codegen error naming
  `salvo platform generate`.
- **P9. Available to customers**, like platform fns and handlers. The
  runtime is the first user, not the only one.

### 12.3 What this means for the sequence

- **`Deque` stays an intrinsic.** It is a collection: it needs `canbe Mut`
  with value semantics (a `copy` is a new deque, not a shared one), the
  element-conditional linearity of [linear-container], and borrowed access
  for iteration and peeking, which P2 and P7 rule out. It belongs with `List`
  and gets the same special handling. §11.5 step 6 changes from "`Deque` as
  a platform type" to "`Deque` as an intrinsic beside `List`"; E1 is still
  needed for `Dyn`. The runtime's queue operations that inspect an entry in
  place (the gate's search by slot) use `Deque`'s own iteration.
- **`Addr`, `Reply` and `Pool`** become Salvo types (E7), not platform
  types: they have no host part, and Salvo types are cheaper for a new
  backend than either.
- **The intrinsic handlers** `StdOutConsole` and `DefaultRandom` are not
  types, but they are the same kind of leftover: once platform handlers in
  std are routine, both can become platform handlers. Not part of this
  work.

### 12.4 Two weak spots

- **`unerase` on Kotlin cannot check its type argument.** A generic Kotlin
  function sees `T` erased, so `unerase<T>` is an unchecked cast, and a
  mismatch surfaces later as a `ClassCastException` where the value is used,
  not at the `unerase`. Rust's `downcast` fails at the `unerase` itself. In
  the runtime a mismatch is a compiler bug either way; making Kotlin's
  check exact would need a type token passed alongside, which generated code
  could supply if it ever matters.
- **Rust `Clone` "must be cheap" is a contract**, not something the
  compiler can check. A host type that deep-copies on `Clone` is correct
  but slow wherever Salvo copies a handle.
