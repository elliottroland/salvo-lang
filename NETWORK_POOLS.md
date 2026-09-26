# Network pools — actors across machines, the option space (working document)

**Status: OPEN** — nothing decided. Every section below ends in a
recommendation, and the calls are the user's.

**Provenance.** Written 2026-09-26 by the assistant, in a read-only session
while another agent held the repository, at the user's request to explore
extending actors to run across a network. It follows DESIGN_DOC.md's skeleton
and lays the design out as options, trade-offs and recommendations, in the
manner of CONCURRENCY.md (phase 5) and FILE_SYSTEM.md (phase 4) before it. It
does not decide anything; it exists so that the language-design calls this
would force are chosen before a line of it is built. The label prefix is `N-`.

**Propagation owed.** This session could write only this file. Nothing here
has reached ROADMAP.md (no **DECISION** rows exist yet for any `N-` question,
and "Actors" and "The spawn line" do not mention a network), COMPLETED.md's
decision log, LANGUAGE_SPEC.md, `BACKEND_SPEC.rust.md`, `BACKEND_SPEC.kotlin.md`,
or `docs/language/`. When the user decides, the outcomes go to COMPLETED.md's
log as user decisions, the open rows become plans in ROADMAP.md, new rules take
fresh labels (proposed below, none of which exist yet), and **this file is
deleted** — DESIGN_DOC.md's charter. Until then, nothing has propagated.

**Sources.** ROADMAP.md: "Actors — effect handlers bound asynchronously",
"The spawn line", "The sugar pass", "Platform handlers — the thread-safety
contract", "Decisions waiting on the user". COMPLETED.md's decision log for
phase 5 (2026-09-14 … 16), the second sequence (2026-09-17/18) and the
shareable-by-default round (2026-09-19/20). `docs/language/Concurrency.md`,
`Effects-and-Handlers.md`, `Backends.md`, `Time.md`, `Testing.md`. Rule labels
relied on, each verified to exist in LANGUAGE_SPEC.md or a backend spec:
[actor-kind], [actor-effect-kind], [actor-send-fn], [actor-sendable],
[actor-types], [actor-spawn-expr], [actor-mailbox], [actor-use-addr],
[actor-replyto], [actor-watch], [actor-on-idle], [actor-deadlock-cycle],
[actor-spawn-effect], [actor-no-closure], [free-send-fn], [task-mint],
[task-pool-inherit], [main-pool], [waitfor-pump], [waitfor-dedicated],
[waitfor-effect], [pool-fault-sink], [mixed-handler], [monitor-handler],
[use-local], [spawn-inherit], [with-clause], [effect-not-data],
[effect-handler-deps], [effect-intercept], [effect-handler-multi],
[effect-state-store], [linear-obligation], [linear-opaque], [linear-container],
[linear-group], [qual-subject], [qual-erasure], [time-timer], [time-manual],
[time-coupling], [mod-export], [platform-effect], [platform-handler],
[platform-tree], [cli-platform], [intrinsic-std-only], [backend-never-wrong],
[backend-companion], [test-decl]; and, as representational facts only,
[rs-actor], [kt-actor], [rs-fn-field], [rs-platform-handler],
[kt-platform-handler]. The user's stated intent is §0.

---

## §0. The stated intent

The user's sketch, numbered so the decisions below can be judged against it.
It is a set of *questions* more than a design, so it is used here to shape the
option space and to price the options, not to fix answers.

1. **Actors should be runnable across a network.** An actor today is an
   effect handler bound with `spawn` on a pool of threads in one process; the
   extension is that the pool's workers, or the actor itself, may be on another
   machine.
2. **Transparent to the function that uses the actor as an effect.** A
   function declaring `[Ledger]` and calling `record(entry)` must not change
   when the `Ledger` behind it moves to another node — the same guarantee
   [actor-use-addr] already gives between a local handler and an actor.
3. **Explicit where it is bound or written.** The spawn site, the `use` site,
   or the handler's author may have to say more — placement, serialization,
   failure handling — and that is acceptable; the *call* sites may not.
4. **"Somewhere on a cluster."** The user asks how a call to an effect comes
   to run on some node of a cluster rather than a named machine: placement as
   a scheduler decision, not an address the program writes.
5. **Does the platform own discovery and communication?** Whether the host
   (a `platform effect` / `platform handler`) is handed node discovery and the
   transport, and what that buys and costs.
6. **Distributed patterns.** How leader election, map/reduce and the other
   common shapes would be supported: as language features, std, or examples.
7. **What else has to be decided** about the network, and which language
   features it forces.
8. **(Steering, same day.) At what level does the network come in?** A
   *network pool* alongside a thread pool, something *inside* a thread pool, a
   thing only a *coordinating supervisor* manages, or something else. This is
   the load-bearing question and is taken first, as N-1.
9. **(Added, same day.) Versions across a fleet.** Once processes talk, some
   may run an older build than others — a rolling deploy has both live at
   once. The user wants options for **ensuring actors that talk share a version
   of the code**, a call on whether that versioning is **the user's to control
   or a hidden detail**, and a way for a program to **read** the version for
   logging and the like. Taken as N-10.

**How the points fall against the record.** Points 2 and 3 are the phase-5
design *extended*, not challenged: "the binding is what makes it an actor"
(COMPLETED.md, phase 5; [actor-kind]) becomes "the binding is what makes it
remote", and every call site stays a member call. Point 1 meets
[actor-sendable] head on: what may cross a *thread* seam is a structural rule
about ownership, and what may cross a *machine* seam is a rule about
representation — a stricter one (N-3). Point 4 meets [actor-spawn-expr] and
[task-pool-inherit]: placement is the `on` clause and an ambient pool, so
"somewhere on a cluster" has to be a pool-shaped thing or a new clause (N-1,
N-2). Point 5 meets [platform-effect] / [platform-handler], the two interop
paths, and the open thread-safety **DECISION** on the latter (N-4). Point 6
meets the recorded posture that supervision is "a pattern with no syntax"
(SUPERVISION.md's decision, COMPLETED.md's log); the same posture is on offer
for the distributed patterns (N-7). Point 7 surfaces failure, identity,
security, testing and versions — N-5, N-6, N-8, N-9 — which have no local
counterpart today because a process cannot half-fail. Point 9 meets the
exhaustive `when` and positional arm identity (AGENTS.md's invariant): an old
node cannot decode a message with an arm it lacks, so version compatibility in
Salvo has to be settled before decoding, which is what makes N-10 a design
question rather than a wire-format detail.

---

## §1. Fixed points — already decided, inherited here

What this phase builds on and does not reopen. Where a §0 point collides with
one, the collision is flagged; each is the reason a decision section exists.

- **An actor is an effect handler bound asynchronously** [actor-kind]: a state
  struct plus one function per member, run-to-completion on a scheduler that is
  a *library* in each backend's runtime, with no runtime in generated code and
  identical behaviour on both backends. A network runtime must keep that shape:
  it is more library in `runtime/scheduler.{rs,kt}` plus host code, never a
  daemon the generated program depends on.
- **`Addr<E>` is the whole of what one actor knows about another**
  [actor-types]: a many-shot address typed by a protocol, freely copyable,
  never linear; a send to a dead actor is a silent no-op and death is
  *observed* with `watch` [actor-watch]. **Collision with §0.1:** an addr today
  is a handle into one process's scheduler. Across a network it needs a
  routable identity and a home node (N-2), and "dead" acquires a second
  meaning — unreachable (N-5).
- **`Reply<T>` is linear and its capacity is reserved at the mint**
  [actor-replyto] [actor-types]: an answer is delivered exactly once *by the
  program*, on every path, statically. **Collision:** a network can lose the
  answer after the program sent it. Linearity states what the sender did, not
  what arrived; N-5 has to say what the waiter sees.
- **Sendability is a structural ownership rule** [actor-sendable]: no function
  values, no `proj` views, checked at the actor effect's declaration for
  payloads and at the crossing site for captures and spawn arguments; an
  `Addr`, a `Reply`, a `Pool` all cross a thread seam today because they are
  handles. **Collision with §0.1:** a machine seam needs *serializability* —
  a representation both ends agree on — and a `Pool` handle cannot cross at all
  (N-3).
- **Placement is the `on` clause, inherited when unwritten**
  [actor-spawn-expr] [task-pool-inherit] [main-pool]: `spawn H() on POOL`,
  `replyto k(...) on POOL`, and an ambient pool exists on every thread the
  language owns. `thread()` answers a `Dedicated Pool`, a *provenance
  qualifier* on `Pool` [waitfor-dedicated] [qual-subject] — the precedent for
  saying where a pool came from in its type without new grammar. **This is what
  §0.8 asks about**: whether a node is a pool (N-1).
- **The mailbox belongs to the handler** [actor-mailbox], the fault sink to the
  pool [pool-fault-sink], the watch to the observer [actor-watch]. Placement
  knobs, if any, were recorded as the *spawn site's* half of the parameter
  survey ("The spawn line": `on pool(2).throughput(50)` or `on pinned()`).
- **`use addr` is an address-to-interface adapter** [actor-use-addr]: a
  generated forwarding stub `__Stub_E` whose bodies enqueue, which is how a
  function declaring `[E]` never learns whether `E` is a local handler or an
  actor. §0.2 is this rule at a distance. The stub for an *answering* member
  (plain effect over an addr) was deliberately deferred to "The sugar pass"
  because from a process a call parks and from synchronous code it blocks;
  N-2 inherits that question, since a remote plain effect is exactly an
  answering stub.
- **A mixed handler is a servant behind a façade** [mixed-handler]: plain
  members run on the caller's thread and wait for a servant's answer, with the
  wait priced in the deadlock graph. A remote-backed plain effect has this
  shape already — the servant is on another machine — which is why N-2 can
  reuse it rather than invent a call form.
- **A task is a free `send fn` scheduled rather than called** [free-send-fn]
  [task-mint], with no state, no identity and no effects; its placement is the
  `on` clause or the ambient pool. This is the unit map/reduce would ship
  around (N-7).
- **The deadlock baseline is a static cycle check over actor effects**
  [actor-deadlock-cycle], whole-program, over types not instances. **Collision
  with §0.1:** it assumes it sees every handler. A cluster of nodes running *the
  same program* keeps that property; a cluster of different programs does not
  (N-9).
- **Interop is two declared forms** [platform-effect] [platform-handler]: a
  `platform effect` is a capability the host owns and hands to `main`; a
  `platform handler` is a host class registered with `use` for an ordinary
  Salvo effect (`HostRawFs`), generated into `platform/` by `salvo platform
  generate` [platform-tree] [cli-platform]. std's own primitives are
  `intrinsic` [intrinsic-std-only]. A platform handler is **assumed
  thread-safe** and the contract is an open **DECISION** (ROADMAP.md) — a
  network transport handler would be the second host class that shares across
  threads, so §0.5 leans on that open item (N-4).
- **Time is data, and the fake is pure Salvo** [time-coupling] [time-manual]:
  `Timer` is an actor effect, `ManualTime` a two-faced handler that advances
  virtual time, and the posture is to *pass* time rather than read it. The
  filesystem has the same shape (`MemFs`, `RestrictedFs` over `Fs`). A network
  should have one too — a `MemCluster` — or distributed programs become the
  first untestable thing in std (N-8).
- **A handler may implement several effects, one face each**
  [effect-handler-multi]: `spawn` answers one addr per face, which is how a
  public protocol and an administrative one share state. Leader election and
  membership want exactly this split (N-7).
- **Supervision is a pattern with no syntax** (user decision 2026-09-15,
  SUPERVISION.md → COMPLETED.md's log): `watch` plus ordinary handlers. The
  recorded refinement is minter attribution for pool faults. The same posture
  is the default offer for §0.6.
- **Language-design decisions belong to the user; backwards compatibility is
  not a requirement** (AGENTS.md). Every option below may change landed
  surface outright.
- **Never emit silently wrong code** [backend-never-wrong]: a construct the
  network lowering cannot handle is an error at the binding or the
  declaration, never a message that arrives as garbage.

---

## §2. What other languages teach

Grouped by family. Each entry names what it does, what is worth stealing, and
which §0 point or `N-` decision it informs. Claims were checked against the
primary documentation cited; where a system's own docs are the source, the
phrasing is theirs paraphrased.

### Erlang/OTP — location-transparent pids, explicit nodes

A distributed Erlang system is a set of *nodes* (named runtimes,
`name@host`); message passing, links and monitors "are transparent when pids
are used", while registered *names* are local to a node and must be qualified
(`{Name, Node} ! Msg`). Connections are made on first use and are transitive
by default (full mesh), a node's death removes all its connections,
`monitor_node/2` delivers `{nodedown, Node}`, and a remote spawn is
`spawn(Node, M, F, A)` — the node is an explicit argument at the *spawn*, and
nowhere at a *send*. Security is a shared "magic cookie" compared at
connection setup, explicitly documented as protection against accidental
misuse rather than an attacker, with TLS distribution as the real answer.
Delivery is best-effort with per-pair ordering; a message to a dead process is
dropped silently. (Erlang/OTP 29 system docs, "Distributed Erlang".)

What to steal: the split *pids are transparent, nodes are explicit at
spawn* is §0.2 + §0.3 verbatim, and it is the shape N-1(a) proposes. The
"send to a dead pid is a no-op, monitor to learn" pair is already Salvo's
[actor-watch]; `nodedown` is what N-5 has to add. What *not* to take: the
transitive full mesh (it does not scale past a few hundred nodes, which is why
Partisan and the like exist) and the cookie as security (N-6).

### Akka / Pekko — remoting made explicit by serialization and sharding

Akka's cluster tools are the catalogue of the patterns §0.6 asks about, each
a library over the same actor core: **Cluster Sharding** ("interact with
actors using their logical identifier, without having to care about their
physical location, which might also change over time"), **Cluster Singleton**
("one singleton actor instance among all cluster nodes or a group of nodes
tagged with a role", run on the oldest node and handed over on its departure),
distributed pub/sub and distributed data (CRDTs). Two lessons cost them years:
**serialization must be explicit** — Java serialization is disabled by
default because it was slow and unsafe, every message class needs a binding to
a serializer (Jackson, protobuf), and third-party tooling exists just to give a
compile-time guarantee that a message *is* serializable — and **split brain**
needs a resolver policy, because a singleton on both sides of a partition is
two singletons.

Informs: N-3 (serializability as a *checked* property at the declaration,
which Salvo can do at the `actor effect` where [actor-sendable] already
checks), N-7 (sharding and singleton are libraries over `Addr` + membership,
not language), N-5 (partitions are the failure mode to design for first).

### Orleans — virtual actors, the network inside the scheduler

Orleans grains are *virtual*: a grain reference is location-transparent and
always valid; "when a grain call is made, an instance of that grain is
available in memory on some server in the cluster", activated on demand,
placed by a strategy (random by default in older docs; resource-optimized
power-of-k-choices now), deactivated when idle, and reactivated on a healthy
silo after a failure. Turn-based execution (one request at a time per
activation) is the default, as Salvo's is. Placement and later *balancing*
(migration) are separate decisions with different information and costs.

This is N-1(b) — the network *inside* the pool — done as well as it has been
done. Its price is the one §0.3 accepts and §0.2 refuses to pay at call
sites: every grain call is potentially remote, so every argument and result is
serializable by construction, every call is asynchronous, and grain state
lives in a store because an activation may move. For Salvo it is the
shape-changing alternative in N-1 and the model for the "somewhere on a
cluster" of §0.4: identity-keyed placement (N-7 sharding) rather than
spawn-site placement.

### Ray — remote tasks and actors on a scheduler, data by reference

Ray's `@ray.remote` makes a function a *task* and a class an *actor*; a call
returns an object reference immediately, results live in a shared object
store and are fetched by `ray.get`, and placement groups reserve resources.
The programmer never names a node; the scheduler picks one from resource
requirements. What is worth noting for §0.6 is that map/reduce is *nothing*
in Ray: a list comprehension of task calls and a `get` on the futures, with a
reduce task taking references. It is the closest existing thing to "a free
`send fn` placed on a cluster pool, answers gathered through `Reply`" —
Salvo's task kernel [free-send-fn] with a remote pool (N-7).

### Cloud Haskell, Unison — what must be true of the *code*

Cloud Haskell brought Erlang's model to a typed language and hit the boundary
squarely: "remote processes must be expressed as static closures, messages
must satisfy serialisation constraints, and participating nodes are assumed
to share the relevant code" (CloudMicroHaskell, 2026, summarising
distributed-process). Arbitrary closures cannot be sent; the set of remotable
computations is known in advance (`remotable`) and shipped as a *name* plus
serializable environment. Unison's `Remote` ability goes the other way:
definitions are content-addressed, so a computation *can* be sent — the
recipient syncs any hashes it lacks before running it.

Salvo is closer to Cloud Haskell than to Unison and should say so: a
function value is already unsendable across a thread [actor-sendable]
[rs-fn-field], both backends compile ahead of time, and neither can load code
at runtime without a host. So the honest first pass is *one compiled program
per cluster*, with "a task is a free `send fn` known at compile time" as the
static-closure discipline already in place [free-send-fn] (N-9). Unison's
lesson survives as a check: a protocol hash exchanged at handshake, so two
builds that disagree refuse to talk instead of misreading each other.

### E, CapTP — an address as an unforgeable capability

E's vats are single-threaded event loops (an actor with a mailbox), eventual
sends (`obj <- msg()`) return promises, and **CapTP** carries references
across the network so that holding a remote reference is holding the
authority it names — unguessable "Swiss numbers" stand in for the local
pointer. Salvo's `Addr<E>` is a capability locally by construction (only a
spawn mints one, only a holder can send). Across a wire that property has to
be *re-established*, since a routable identity is a bit pattern anyone can
forge; N-6 is that question, and the two-face handler [effect-handler-multi]
is what makes least-authority addresses ordinary (`Addr<Timer>` vs
`Addr<TimerCtl>`).

### The Raft family, ZooKeeper/etcd — where consensus lives

Leader election is either implemented (Raft, Bully, ring) or *delegated* to a
coordination service that has already done it (etcd, ZooKeeper, Consul; a
Kubernetes lease). Every production system of any size does the second for
the control plane and the first only inside the coordination service itself.
For §0.6 this means the question is not "how does Salvo do Raft" but "is Raft
an example written in Salvo actors, a std module, or a `platform handler` over
the host's coordinator" — N-7, with a recommendation of the first and third.

### What Kotlin and Rust do — the two floors this must land on

Every decision has to lower into both backends with identical observable
behaviour, and the local scheduler is already a per-backend library
(`runtime/scheduler.rs`, `runtime/scheduler.kt`).

- **Kotlin/JVM.** Akka/Pekko exist but bring their own actor system, which
  Salvo does not want under its scheduler; kotlinx-rpc and gRPC-Kotlin are the
  transport-level tools; kotlinx.serialization generates codecs from annotated
  classes at compile time. Java serialization is the anti-pattern everyone
  warns about. A `platform handler` for the transport would be a Ktor or
  raw-socket class; the codecs should be *generated by the Salvo compiler*
  rather than by an annotation processor the host has to run, so both backends
  emit the same bytes (N-3).
- **Rust.** No dominant distributed actor library: `ractor_cluster` (Erlang
  style node servers over `ractor`), Coerce, Lunatic (Erlang-like WASM
  processes with distributed nodes) each ship a runtime of their own; `tonic`
  (gRPC) and raw `tokio`/std sockets are the transport tools; `serde` with
  `bincode`/`postcard` is how bytes are made, and derive-generated. The same
  conclusion: the transport is host code (a `platform handler`), the codecs
  are compiler output, and the routing table and membership are Salvo actors
  in the runtime library — so a Rust node and a Kotlin node can be members of
  one cluster, which no other language on either floor offers and which is the
  argument for doing the wire format in the compiler (N-3(a)).

---

## N-1. Where the network comes in (§0.8, §0.4, §0.1) — load-bearing

**The question.** Which existing thing grows a network: the pool, the
scheduler beneath it, a supervisor actor, or the binding? Everything else
depends on this — what an `Addr` is (N-2), what must be serializable (N-3),
who owns the transport (N-4), and what a failure looks like (N-5).

The vocabulary the language already has for "where work runs" is one word:
a **pool** [actor-spawn-expr]. `on pool(4)`, `on thread()`, and the ambient
pool [task-pool-inherit] are the only placement anything is ever given, and a
`Dedicated Pool` shows a pool's *kind* can live in a provenance qualifier
[waitfor-dedicated] without new grammar. The four options are four answers to
"is a node a pool".

### (a) A network pool beside the thread pool — a node *is* a pool

A new std constructor answers a pool whose workers are on another node (or on
a set of nodes), and `on` takes it exactly as it takes `pool(2)`:

```
fn main() [use, spawn] {
    use HostTransport("0.0.0.0:7000")          // N-4: the host owns the wire
    let cluster = join("ledger-cluster", seeds)  // std: membership, an actor
    let far = node(cluster, "ledger-2")          // Remote Pool — one named node
    let any = anywhere(cluster)                  // Remote Pool — the scheduler picks
    let ledger = spawn Ledgering() on far        // Addr<Ledger>, as ever
    ledger.record(entry)                         // the call site is unchanged
}
```

`node(...)`/`anywhere(...)` answer a **`Remote Pool`** — `Remote` a provenance
qualifier declared in std beside `Dedicated`, so [qual-subject] and
[qual-erasure] apply unchanged. The `on` clause accepts it; a `spawn` onto it
crosses a machine; the answer is an `Addr<E>` that routes. §0.4's "somewhere"
is `anywhere(cluster)`: a pool whose worker set is every member with room,
chosen by the membership actor's placement policy.

- **Cost.** A second kind of pool with different guarantees: a task or actor
  placed on a `Remote Pool` must have *serializable* arguments (N-3) and
  cannot borrow anything; the ambient-pool rule [task-pool-inherit] means an
  actor spawned remotely mints its continuations remotely (correct, and the
  same "whoever creates work pays" reading); `waitfor` across a wire is a
  network round trip that serves the *local* pool while it waits (unchanged
  in kind, different in latency).
- **Trade-offs.** Placement stays where the parameter survey put it — the spawn
  site — and is explicit, satisfying §0.3. Nothing local pays: `pool(4)` is
  unchanged, and the sendability rule is only *stricter* on `Remote Pool`
  sites, so a program with no network compiles exactly as today. The
  `Dedicated` precedent means the type system already knows how to say this.
  What it does not give is *migration*: an actor spawned on `ledger-2` lives
  and dies there (Orleans' balancing is out of scope; see (b)).

### (b) Inside the thread pool — a pool may span machines

`pool(4)` becomes a *local* pool and `pool(4, cluster)` (or a configuration
of the runtime) a pool whose workers are drawn from every node. Any spawn or
mint on such a pool may land anywhere; the scheduler decides, and may later
move an idle actor. This is Orleans, and Ray's default.

- **Cost.** Every payload on such a pool must be serializable, and the
  program cannot tell at a site whether it is paying that — so the rule has
  to be *program-wide* (every actor effect serializable) or the pool's kind
  has to be in its type anyway, which collapses (b) into (a) with a worse
  spelling. Actor state must be movable for balancing to be legal, which
  forbids linear obligations in state [linear-container] [linear-state] or
  makes them travel — a research problem. `main`'s pool [main-pool], `thread()`
  [waitfor-dedicated] and the whole `waitfor-pump` reasoning are about *a*
  thread; a spanning pool has no such thread.
- **Trade-offs.** It is the most "transparent" reading of §0.1, but it
  achieves transparency at the *spawn* site, which §0.3 does not ask for, by
  taxing every site, which §0.2 forbids. Rejected as the primary shape; its one
  real gift — "somewhere on a cluster" without naming a node — is kept as
  `anywhere(cluster)` in (a).

### (c) Only a coordinating supervisor — the network is an actor

No new pool kind. std ships an `actor effect Cluster` whose handler, written
in Salvo over a transport effect, answers `spawn_on(node, …)`/`lookup(name)`
requests with an `Addr<E>`. The language is untouched: an addr produced by
the cluster actor happens to route over the wire, and everything the program
does is send to it.

- **Cost.** A remote spawn cannot be an ordinary member call, because a
  handler is not a value [effect-not-data]: `spawn_on(node, Ledgering())` has
  nothing to pass. So either the cluster actor gets a *name* of a handler
  (a string the compiler cannot check — [backend-never-wrong] violated at the
  first typo) or `spawn` grows a form after all, which is (a) wearing a
  supervisor's coat. It also puts placement behind a request/response instead
  of an expression, so `let a = spawn …` becomes a `waitfor`.
- **Trade-offs.** The *membership, discovery and routing* halves of this are
  right and are what (a) uses underneath (`join` answers a handle to exactly
  such an actor). What is wrong is making it the spawn path. Kept as N-4's
  layering, refused as the placement surface.

### (d) Shape-changing alternative — the network at the *binding*

Neither pool nor supervisor: `use remote Ledgering() at node` — a third
binding kind after `use` (synchronous) and `spawn` (asynchronous), meaning
"construct this handler on that node and bind its effect here". Placement is a
property of the binding, and an `Addr` never appears for the remote case.

- **Cost.** It duplicates what `spawn … on` + `use addr` already compose to,
  for one case, and loses the addr — so the program cannot hand the remote
  handler to a third party, `watch` it, or put it in a `with` clause. It also
  binds a *plain* effect remotely, which is the answering-stub question the
  sugar pass owns (N-2), as a new keyword rather than as the existing form.
- **Trade-offs.** Its virtue is that it reads §0.2/§0.3 literally: the binding
  says `remote`, the calls say nothing. But (a) plus `use addr` reads the same
  way with one fewer form and keeps the handle.

### Shape-changing alternative 2 — computation moves, not actors

Unison's `Remote.transfer node`: a *frame* continues on another machine, code
and all. Not available to Salvo without runtime code loading on both backends
and a content-addressed program; recorded so that N-9's "one program per
cluster" is seen as a choice with a known alternative, not an oversight.

**Recommendation (the user's call): (a).** A node is a pool: `Remote Pool`
as a provenance qualifier, `node(cluster, name)` and `anywhere(cluster)` as
its constructors in std, the `on` clause unchanged, the membership actor of
(c) underneath. It is the smallest addition that answers §0.8 with a word the
language already has, it keeps §0.2 by never touching a call site, it puts
§0.3's explicitness exactly where the parameter survey said placement belongs,
and it leaves local programs untaxed. Migration and virtual actors (b) stay
out of the first pass and are reachable later as a *policy* of `anywhere`
(N-7, sharding).

---

## N-2. What an `Addr<E>` and a `Reply<T>` are across the wire (§0.2, §0.4)

**The question.** An addr is a handle into one process's scheduler
[actor-types]. What is it once actors live on several nodes — and what does a
`Reply<T>` become, given that its capacity was reserved in the target's queue
at the mint?

### (a) One `Addr<E>` type; locality is a runtime fact

Every `Addr<E>` is a routable identity — `(node id, local id)` — and the
runtime's send checks whether the node is this one. A local send is a queue
push as today; a remote send serializes and forwards. No change to any type,
so `use addr`, `with addr`, `watch(addr, …)`, `List<Addr<E>>` all work
unchanged, and an addr crosses the wire as a value (it is just two integers).

- **Cost.** Every addr grows a node field even in a program with no network
  (a word; negligible). More seriously, the checker cannot tell a local addr
  from a remote one, so *every* send has to satisfy the network's
  serializability rule (N-3), or the rule has to be dynamic. A dynamic rule
  is refused by [backend-never-wrong]: a payload with a function value would
  fail at the first remote send with nothing at the declaration to warn.
- **Trade-offs.** Maximal transparency (§0.2), minimal surface. It works only
  if N-3 makes serializability the rule for *every* actor effect — which is
  acceptable if the rule is cheap, and it nearly is (see N-3(a): the compiler
  generates the codecs; the author writes nothing).

### (b) A provenance qualifier: `Remote Addr<E>`

A spawn on a `Remote Pool` answers a `Remote Addr<E>`; a plain `Addr<E>` is
local. `use` and `send` accept both (the qualifier drops, [qual-subject]); the
*serializability* check fires only where a `Remote Addr` is the target, and at
the actor effect's declaration only if some spawn of a handler of it is remote
— a whole-program fact the checker already computes for the deadlock graph.

- **Cost.** Two kinds of addr the program can see, and a `List<Addr<E>>` of
  mixed provenance erases to the weaker claim, so the static precision is
  lost the moment addrs are stored — which is exactly what a registry does.
  The whole-program declaration check is brittle: adding one remote spawn
  anywhere makes a previously fine effect need serializable payloads, and the
  error appears at the declaration, far from the cause.
- **Trade-offs.** It lets a program *say* "this addr may be far", which N-5
  wants for failure handling. But the precision does not survive storage, and
  the coupling of a declaration's legality to distant spawn sites is the
  thing Salvo's per-function checking avoids elsewhere.

### (c) Shape-changing alternative — remote is a *different effect kind*

`network effect Ledger { send fn … }`: a third effect kind after `effect` and
`actor effect`, whose members are checked for serializable payloads at the
declaration and whose handlers may be spawned remotely. An `Addr<Ledger>` of
a network effect routes; an `Addr` of an actor effect never leaves the
process.

- **Cost.** A handler serves one kind; a protocol the author wants to use both
  locally and across the wire is written once as `network effect` and used
  locally at the price of a codec it never runs (zero at runtime if the
  serializer is only invoked on a remote send). Two kinds to teach, and the
  `Timer`/`Faults` protocols in std would need to pick.
- **Trade-offs.** It puts the decision where [actor-sendable] put sendability:
  *at the declaration, where the author is choosing*. The checker's job is
  local and one-directional (a `network effect` handler may go on any pool; an
  `actor effect` handler may not go on a `Remote Pool`), the diagnostics land
  where the fix is, and nothing whole-program is needed. It is heavier in
  vocabulary than (a) and lighter in rules than (b).

### The `Reply<T>` half

Under every option a `Reply<T>` that crosses the wire is a routable one-shot
`(node, actor, slot)`, and its two properties survive: linearity is static and
unchanged (the sender still discharges it exactly once), and its capacity was
reserved in the *minting* actor's own queue [actor-types], so the answer's
arrival never blocks the remote sender. What changes is that the answer may
not arrive (N-5). A `waitfor` in `main` on a remote reply is a blocking
network round trip that serves main's pool meanwhile [waitfor-pump] — the same
rule, and the reason no `await` is needed.

### The plain-effect (answering) case

§0.4 says "a call to an effect", not "a send". A *plain* effect's member
answers, and today a plain effect is never actor-backed ([actor-effect-kind]
closed that question): `use addr` binds an actor effect only. A remote plain
effect is the deferred **answering stub** of "The sugar pass" — from a
process the call must park, from synchronous code it must block — and the
mixed-handler shape [mixed-handler] is its existing spelling: a façade member
that sends to a remote servant and waits. The recommendation is to *not* open
this here: remote binding is for actor effects in the first pass, and a
program that wants a synchronous-looking remote call writes a mixed handler
over a remote addr (a few lines, priced by the deadlock graph), exactly as it
does over a local servant today. When the sugar pass lands the answering
stub, the remote case comes with it for free.

**Recommendation (the user's call): (a) for the type, with N-3(a) making
every actor effect serializable by construction, so the transparency is
real.** Fall back to (c) if N-3 decides serializability must be opt-in — then
the kind marker is the opt-in, and (b) is refused in either case because its
precision does not survive a `List`. Plain effects stay local until the
answering stub exists; mixed handlers are the bridge meanwhile.

---

## N-3. What may cross a machine, and in what bytes (§0.1, §0.5)

**The question.** [actor-sendable] says what may cross a thread: anything the
receiver can own. A machine seam needs more — a *representation* both ends
agree on, produced and consumed by generated code — and less: some things
that cross a thread today (a `Pool`, a platform handle) must not cross a
wire. Who defines the encoding, and where is the rule enforced?

What cannot cross under any option, each an error at the declaration or the
crossing site [backend-never-wrong]: a function value and a `proj` view
(already refused); a `Pool` (a set of *this* node's threads — a `Remote Pool`
is a name, and may cross); a `platform effect`'s instance (the host owns it
and hands it to `main` as a borrow — already uncapturable); an `InStream` /
`OutStream` or any other linear intrinsic handle into this process
[linear-opaque]. What *may* cross: scalars, `Str`, `Bytes`, structs, tuples,
unions, arrays, the collections, `Addr<E>`, `Reply<T>`, `Remote Pool`, `Exit`,
`Fault`, and time's `Duration`/`Instant`/`Tick` — every one a value with no
process-local meaning.

### (a) A compiler-defined canonical encoding, generated per type

The Salvo compiler emits an encoder and decoder for every type that crosses a
remote seam, on both backends, to one byte format it specifies. The format is
positional and mirrors the identity rules the checker already has: a struct
is its fields in declaration order; a union is its arm index over the
*declared* non-`None` arms in declaration order (the rule AGENTS.md forbids
changing) followed by the arm; `None` is a tag; an `Addr` is `(node, id)`. A
variable-length integer scheme (LEB128 or `postcard`'s) keeps it compact. A
**protocol hash** — over the canonical form of every actor effect and every
type reachable from one — is exchanged at the handshake, so two builds that
disagree refuse to connect (N-9).

- **Cost.** A serialization format is a specification the project now owns,
  with two implementations (the two backends' runtime libraries) that must
  agree byte for byte — the parity bar the scheduler already meets, applied
  to bytes. Schema evolution is nil in the first pass (same program on every
  node); adding it later means field tags, which is a format change.
- **Trade-offs.** The author writes **nothing**: every actor effect is
  serializable by construction, which is what makes N-2(a)'s transparency
  real, and `intrinsic` types in std get their codec in the backend like
  every other intrinsic. It is the *only* option under which a Kotlin node
  and a Rust node can be members of one cluster, since there is no shared
  third-party format between kotlinx.serialization and serde that a compiler
  can rely on without picking one anyway. It also gives std a `to_bytes<T>` /
  `of_bytes<T>` for free if wanted (N-7's map/reduce over files).

### (b) Delegate to the host — payloads are `Bytes`, codecs are platform code

The transport carries `Bytes`; an actor effect's payloads must be `Bytes` (or
a type with a user-written `encode`/`decode` pair, resolved as an implicit
like `cmp`); the host picks protobuf, JSON, whatever the shop uses.

- **Cost.** §0.2 breaks: an actor effect bound remotely has a different
  *signature* from one bound locally (or every actor effect pays `Bytes`
  payloads locally too), and the author writes codecs by hand for every
  struct. Cross-backend clusters depend on the host authors agreeing.
- **Trade-offs.** Interop with an existing wire (a protobuf service another
  team owns) is the one thing (b) does better, and it is reachable from (a)
  as a *platform effect* at the edge of the program, which is where interop
  with a foreign protocol belongs anyway. Refused as the actor transport.

### (c) Shape-changing alternative — serializability as an opt-in claim

A `serializable` marker on struct declarations (or a `: auto Wire<self>` in
the manner of `: auto Hashed<self>`), and an actor effect whose payloads
carry the claim may be spawned remotely. Locality-only effects pay nothing,
and the author states the intent once per type.

- **Cost.** Every type in every remote payload, transitively, must carry the
  marker — including std's `Exit`, `Fault`, `Fired`, the collections and the
  time types — so std marks everything anyway and the user's own structs are
  where the friction lands. It is the Akka experience: the marker is
  forgotten at the leaf and the error lands at the root.
- **Trade-offs.** It makes "this crosses machines" visible in the type, which
  is Salvo's habit (`canbe Mut`, `linear`). But nothing about a struct of
  scalars *can't* cross; the marker records a wish rather than a fact, and the
  fact ([actor-sendable]'s two refusals plus the process-local handles) is
  already checkable. If a marker is wanted for *documentation*, it can be a
  no-op refinement later.

**Recommendation (the user's call): (a).** The compiler owns the encoding,
generates the codecs, and checks serializability structurally at the actor
effect's declaration exactly where it checks sendability today (with the
process-local handles added to the refused list). The format gets a spec
section and a label (`[wire-format]`, proposed), the arm-identity and
field-order rules it depends on are already invariants, and the protocol hash
is the versioning story until schema evolution is a real need (N-9, and N-10
for what the hash is per protocol and how a mixed fleet behaves).

---

## N-4. Who owns discovery and the transport (§0.5) — benefits and costs

**The question.** Three layers have to exist: the **wire** (sockets, TLS,
framing, reconnection), **membership and discovery** (which nodes exist, how
a new one finds the others, who has left), and **routing** (given
`(node, id)`, deliver). Which of the compiler, std-in-Salvo, and the host owns
each — and specifically, does the platform get discovery and communication, as
§0.5 asks?

### (a) The platform owns the wire only; std owns membership and routing

A `platform handler` of a std `Transport` effect provides `connect`, `send
(node, Bytes)`, `listen`, and a delivery callback into the runtime — the
`HostRawFs` pattern [platform-handler], one class per backend in std's
`platform/` tree [platform-tree]. Membership (`join`, `leave`, node up/down,
seeds, gossip or a static list) and routing are **Salvo actors** in std over
that effect. Discovery of *seeds* is a constructor argument (`join(name,
seeds)`), where `seeds` may come from the host (a DNS name, a Kubernetes
service) through an ordinary platform effect the program declares.

- **Benefits.** The part that differs per deployment (sockets, TLS, how the
  first address is learned) is host code the shop can replace, which is what
  `platform` is for. The part that must be *identical on both backends* — who
  is a member, where an addr routes, what a partition does — is one Salvo
  program, tested with a `MemTransport` the way `MemFs` tests the filesystem
  (N-8), and readable by the user in std rather than buried in two runtimes.
  The deadlock graph and the idle report can see the membership actor because
  it is an actor.
- **Costs.** std grows a real distributed system (membership with failure
  detection is the hard part of every cluster library). The transport handler
  is the second host class shared across threads, so the **open
  thread-safety DECISION** (ROADMAP.md, "Platform handlers") becomes
  load-bearing: a transport is called from every pool and *must* be
  concurrent-safe, and today that is assumed rather than declared. The
  delivery path — bytes arriving on a host thread and becoming an activation
  on a pool — is a host→runtime upcall, which the scheduler has one of already
  (the timer's `fire_after` [time-timer]); it becomes a documented seam.

### (b) The platform owns discovery, membership and the wire

The host hands `main` a `platform effect Cluster` with `members()`,
`spawn_on(node, …)`, `send`, `watch_node`: the runtime of a hosted cluster
(Kubernetes, an Akka system, Erlang's `net_kernel`) does the distributed part,
Salvo does the local part.

- **Benefits.** Nothing distributed is built in std; production membership is
  whatever the shop already runs; a Kotlin build could sit on Pekko cluster
  and a Rust build on `ractor_cluster`.
- **Costs.** Parity is gone: two hosts with two membership semantics, and the
  same program behaves differently on the two backends under partition,
  which the project has refused everywhere else. A cross-backend cluster is
  impossible. The routing of an `Addr` — the thing N-2 makes a language
  value — is then the host's, so `watch` on a remote addr, the fault sink and
  the idle report all depend on host behaviour the compiler cannot check.
  And a `platform effect` is uncapturable and handed to `main` as a borrow
  [platform-effect], so every actor that needs the cluster has to be wired
  through a Salvo handler over it — which is (a) with the interesting half
  hidden.

### (c) Shape-changing alternative — the compiler owns everything

The runtime libraries grow a TCP transport and membership in Rust and Kotlin
directly, no `platform` declaration anywhere, `join(...)` an `intrinsic`.

- **Benefits.** Zero host setup: a program that says `join` clusters.
- **Costs.** TLS, proxies, service discovery and firewall policy are exactly
  the things a shop needs to change and could not; both runtimes gain a
  sockets layer the tests must exercise for real; and it contradicts the
  recorded interop stance that the host owns what is the host's. A *default*
  transport shipped in std's `platform/` tree — as `HostRawFs` is — gives the
  zero-setup experience under (a) without closing the door.

**Recommendation (the user's call): (a), with a default `HostTcpTransport`
shipped in std's `platform/` tree.** The platform gets *communication*, not
*discovery*: the wire is host code, seeds are a constructor argument the host
may compute, and membership/routing are Salvo actors so they are one
semantics on both backends and testable without a network. This makes the
platform-handler thread-safety contract the **prerequisite decision**: the
transport must declare concurrent safety, and `salvo platform generate`
should print that contract into the skeleton it writes.

---

## N-5. Partial failure — what a program sees when a node is gone (§0.7)

**The question.** A process cannot half-fail; a cluster does nothing else.
Today "dead" means "an activation faulted" [actor-watch], a send to a dead
actor is a silent no-op, a reply that will never come is named by the
idle-with-parked-gates report, and a pool's uncaught faults reach its sink
[pool-fault-sink]. Each needs a network reading: what does an unreachable
node look like, what happens to a `Reply<T>` whose target vanished, and what
does back-pressure mean over a wire?

The one thing every option shares: a **`Remote Pool` is an identity that can
die**, so it should be watchable — `watch(pool, on_exit)` as an overload of
[actor-watch]'s function, delivering an `Exit` whose reason names the node.
That is Erlang's `monitor_node` in the vocabulary the language has, and it
needs no new mechanism.

### (a) Unreachable is dead — one failure model

A node the membership actor has declared down is treated exactly as a faulted
actor: every addr homed there is dead, sends to them are silent no-ops, every
`watch` on them fires with an `Exit` naming the node, and every parked
`Reply<T>` whose *target* was there is reported by the idle report as it is
today. If the node comes back, its actors do not: it rejoins as a new member
with new identities (Erlang's rule).

- **Cost.** A partition looks like death from both sides, so two halves of a
  cluster each decide the other is gone — split brain. The program must be
  written for it (a singleton on both sides), which is why Akka's resolver
  exists; std's membership actor needs a *policy* (keep the majority side,
  keep the side with the oldest member, keep-referee) and the program has to
  pick one at `join`. False positives on slow networks kill actors that were
  fine.
- **Trade-offs.** One model, already in the language, and honest about what a
  distributed system can know. It is what OTP has shipped for thirty years.

### (b) Unreachable is a distinct state — `Exit | Unreachable`

`watch` answers a union: `Exit` for a fault, `Unreachable` for a lost
connection; sends to an unreachable actor are buffered up to the mailbox bound
and delivered on reconnection; a parked reply stays parked. The program can
distinguish "gone" from "not now".

- **Cost.** Buffering is state the runtime holds on behalf of a peer that may
  never return, so a bound and a give-up must exist anyway — at which point
  (b) degrades to (a) with a delay. The two-state model leaks into every
  supervisor: each `is Unreachable` arm has to decide something with no
  information. Reconnection with the *same* identities means the runtime
  must guarantee an actor did not fault meanwhile, which it cannot know
  across a partition.
- **Trade-offs.** Strictly more information for the program, at the cost of a
  state nobody can act on soundly. Akka went here (`Unreachable` in
  membership) and then added a resolver to force a decision — which is (a).

### (c) Shape-changing alternative — failure is an effect, not a message

A remote send may fail *synchronously*: a `send fn` on a `Remote Addr`
requires `[Throw<NetError>]` in the caller, the way a filesystem member
answers `Err FsError`. The caller handles the failure where it happens.

- **Cost.** It breaks §0.2 outright — a call site declares an effect because
  the handler behind it is far — and it is also wrong about the failure: a
  send *enqueues* and returns [actor-send-fn], so the failure the sender could
  see is only the local queue's, never delivery. Delivery failure arrives
  later, asynchronously, which is a message or a watch.
- **Trade-offs.** Right for the *plain-effect* remote call when the answering
  stub exists (N-2): a synchronous remote call that cannot complete should
  probably throw, and the mixed-handler bridge can `throw` today. Wrong for
  sends.

### Back-pressure over a wire

A local send blocks while the target's bounded mailbox is full
[actor-mailbox]; the static graph treats a send cycle as a warning
[actor-deadlock-cycle]. Across a node the sender cannot see the target's
queue. Two readings: **(i)** the sender blocks on a *credit* the receiver
grants per `(sender, target)` — the same semantics, with the reservation
travelling in the protocol — or **(ii)** the remote send never blocks and the
receiver drops or faults at the bound. (i) keeps `Reply`'s "capacity reserved
at the mint" rule true across the wire [actor-types] and keeps the deadlock
graph's edges meaningful; (ii) is simpler and is what most systems do, but it
makes the mailbox bound lie for remote senders. **Recommend (i)**, since the
whole point of a declared `capacity` is that it is true.

### The deadlock graph

Whole-program and over types [actor-deadlock-cycle], so under N-9(a) — one
program per cluster — it is *still correct*: every handler that could be on
any node is in the program, and a `Remote Pool` adds no edge kind of its own
(a remote send is a back-pressure edge under (i), a remote gated mint a
wait-for edge). What it cannot price is a node's *disappearance*, which is not
a deadlock but a fault, and (a) routes that through `watch`.

**Recommendation (the user's call): (a), with `watch` on a `Remote Pool`,
a membership policy chosen at `join`, and credit-based back-pressure (i).**
Delivery is at-most-once and in order per `(sender, target)` pair, which is
what the local scheduler gives today and what Erlang gives; anything stronger
(exactly-once, idempotent retry) is a pattern over `Reply` in N-7, not a
runtime promise.

---

## N-6. Identity, authority and the wire's security (§0.7)

**The question.** Locally an `Addr<E>` is unforgeable — only a spawn mints
one and only a holder can send — so the two-face handler
[effect-handler-multi] makes least authority a matter of which addr you were
given. On a wire an addr is a bit pattern; anyone who can reach the port can
send anything to `(node 3, actor 17)`. What re-establishes the property, and
who may join a cluster at all?

### (a) A cluster-wide shared secret; addrs are plain identities

Erlang's cookie: `join(name, seeds, secret)`, compared at handshake; inside
the cluster every node trusts every other, and an addr is `(node, id)` with
no authority of its own. TLS, if wanted, is the transport handler's business.

- **Cost.** Authority collapses to membership: any member can send to any
  actor, including an administrative face it was never given — the two-face
  split is advisory across a wire. A leaked secret is the whole cluster.
- **Trade-offs.** Trivial to implement and to reason about; matches the
  "trusted network inside, TLS at the edge" deployment almost every shop runs.
  Explicitly what Erlang's docs call protection against *accidents*, not
  attackers.

### (b) Capability addrs — unguessable identities

An addr that crosses the wire carries a random 128-bit component minted with
it (E/CapTP's Swiss number). Holding the bits *is* the authority; the runtime
drops a message whose addr it did not mint. The two-face split survives:
`Addr<TimerCtl>` has bits `Addr<Timer>` does not.

- **Cost.** Sixteen bytes per addr on the wire and in every table; a revoked
  authority needs a level of indirection (a forwarding actor) since bits
  cannot be un-given. Does not replace transport security — anyone on the
  wire can *read* a passing addr — so TLS is still needed to make the bits a
  secret.
- **Trade-offs.** It is the only option under which what the language
  promises locally ("who holds which face decides what they may do",
  Effects-and-Handlers.md) stays true across a machine, and it costs no
  syntax: the bits live inside the intrinsic. Combined with a shared secret
  or TLS for *membership*, it is defence in depth at a price of sixteen bytes.

### (c) Shape-changing alternative — authority in the type, checked at the seam

`provenance qualifier Authenticated of Request` exists as an example already;
the analogue is a qualifier on addrs — `Trusted Addr<E>` — granted by the
membership actor and *required* by the `on` clause and by sends to
administrative faces. It says statically who may do what.

- **Cost.** The qualifier is erased in the generated code [qual-erasure], so it
  disciplines *this program's* code and nothing on the wire; a foreign sender
  is unaffected. It is documentation of trust, not enforcement of it.
- **Trade-offs.** Useful as an *addition* — making a program say which addrs
  it treats as administrative — not as the mechanism.

**Recommendation (the user's call): (a) for membership plus (b) for addrs,
with TLS as the default transport's job.** Membership is a shared secret or a
certificate the host checks; every addr that crosses a wire carries an
unguessable component; the two-face guarantee therefore survives the network.
(c) can be layered on later if a program wants to state trust in its types.

---

## N-7. Leader election, map/reduce and the other patterns (§0.6)

**The question.** Which of the common distributed shapes become language,
which std, which worked examples? The recorded stance for supervision was "a
pattern with no syntax" (user decision 2026-09-15), and the actor surface has
since carried timeouts, obligation queues and death-watching as *programs*.
The test for each pattern below is whether it can be written today over N-1's
`Remote Pool` and N-4's membership actor with no new form.

### The patterns, one at a time

**Membership and node watch** — std, unavoidable (N-4). An
`actor effect Cluster` with a public face (`members(out: Reply<List<Node>>)`,
`on_change(out: Reply<Change>)`) and an administrative one (`leave`,
`set_policy`) — the two-face handler [effect-handler-multi] doing what it was
built for.

**Leader election** — an *example* over membership, and a *platform handler*
for the delegated case. Bully or a lease-based election is ~60 lines of actor
Salvo over `members`/`on_change` and `Timer` [time-timer]; Raft is a worked
example of a few hundred lines and would be the most convincing program the
language has run. The delegated case — a Kubernetes lease, etcd, ZooKeeper —
is a `platform handler` of a std `effect Leader { fn is_leader() -> Bool;
send fn on_lost(...) }`, so a program declares `[Leader]` and never learns
which of the two it got: §0.2 again. **Not** language: nothing about election
wants syntax, and a wrong built-in election is the one thing worse than none.

**Singleton** — an example: one actor spawned by the leader on
`anywhere(cluster)`, its addr published through a `Registry` actor (below),
respawned by the new leader on `Exit`. The split-brain policy of N-5(a) is
what makes it *one*.

**Registry / named addrs** — std. `actor effect Registry { send fn
register(name: Str, addr: Addr<E>); send fn lookup(name: Str, out:
Reply<Addr<E>?>) }` — except that `Addr<E>` cannot be stored heterogeneously
in one map and an effect is not a type argument anywhere but `Addr`
[effect-not-data]. So either one registry *per effect* (`Registry<E>`, a
generic actor effect, which is fine and type-safe) or a `Str`-keyed
`Map<Str, Addr<E>>` per effect inside one handler. Recommend `Registry<E>`;
it is Erlang's `global` with types. This is also the *discovery* half of §0.4:
"somewhere on a cluster" for a service that already exists is
`lookup@Registry<Ledger>("ledger")`.

**Map/reduce** — an example, and it is *already writable* modulo N-1:

```
send fn count_words(shard: Bytes, out: Reply<Map<Str, Int>>) => !shard, !out {
    out.send(word_counts(shard))
}

handler Reducing(expected: Int, done: Reply<Map<Str, Int>>) of Reducer {
    mailbox { capacity: 64 }
    totals: Mut Map<Str, Int> = {}
    seen: Int = 0
    send fn partial(counts: Map<Str, Int>) => !counts {
        merge_into(totals, counts)
        seen = seen + 1
        if seen == expected { done.send(copy totals) }
    }
}

fn main() [use, spawn] {
    let far = anywhere(join("wc", seeds))
    let result = waitfor all: Reply<Map<Str, Int>> {
        let reducer = spawn Reducing(shards.size(), all)          // local
        for shard in shards {
            send(replyto reducer.partial(...) on far, shard)      // N-7(ii): see below
        }
    }
}
```

The map step is a free `send fn` [free-send-fn] placed `on far` — a task on a
remote pool is a task (N-1(a)) — and the reduce step is an ordinary actor
collecting replies. Two seams show where the surface is thin, and both are
*decided-not-built* items rather than new questions: (i) a task placed
remotely must have serializable captures (N-3, automatic under (a)); (ii)
"mint a continuation on *another* actor" is the **remote mint** the sugar pass
owns (ROADMAP.md, "The sugar pass": `replyto` resolves lexically today, and a
target reached through the effect list is a diagnostic naming the workaround —
mint where `k` lives and pass the token). The example above writes the
workaround shape; the sugar pass makes it pretty. So map/reduce costs the
network *nothing new* beyond N-1 and N-3. A `Yield`-style pass over shards
(`for r in scatter(shards, count_words, far)`) is a std helper for later.

**Sharding / virtual actors** — a *policy* of `anywhere`, later: `anywhere
(cluster, by: key)` picks a node by consistent hash of `key`, and a `Registry`
keyed by entity id gives Orleans' "always exists" reading as a library
(`lookup_or_spawn`). Migration stays out (N-1(b)).

**Pub/sub, distributed data (CRDTs), exactly-once** — examples, when a
program wants them; each is actors plus `Reply`.

### The options for *how much* is std

**(a) Membership and `Registry<E>` in std; everything else examples.** The two
things a program cannot write itself (it needs the transport) plus the one
every distributed program needs (a name → addr). Elections, singletons,
map/reduce, sharding are `examples/cluster/`.

- **Cost.** A user wanting a leader copies an example. **Trade-offs.** std
  ships nothing it cannot test deterministically (N-8), the patterns stay
  readable programs, and a wrong pattern is the user's to fix.

**(b) A `std/cluster` with the patterns as handlers.** `LeaseLeader`,
`Singleton<E>`, `Sharded<E>`, `Scatter` as std handlers over the membership
actor.

- **Cost.** std owns the correctness of a leader election under partition;
  each handler has a policy surface. **Trade-offs.** Batteries included, and
  the deadlock graph and the `MemCluster` tests cover them.

**(c) Shape-changing alternative — a pattern *is* a spawn policy.** `spawn
Ledgering() on singleton(cluster)`, `on sharded(cluster, key)`: placement
values that carry the pattern, so the program says *what kind of actor* at the
spawn and the runtime keeps the promise.

- **Cost.** The runtime now does elections. **Trade-offs.** It is the most
  Salvo-shaped spelling — placement is the spawn site's, and the pattern is a
  placement — but it moves the hardest code below the language, where a
  program cannot read or replace it. Worth keeping as the *eventual* surface
  for `singleton`/`sharded` once (a)'s examples have proven the semantics.

**Recommendation (the user's call): (a) first**, with Raft as the flagship
example and the `Leader` effect declared in std so the delegated platform
handler and the Salvo election are interchangeable. Promote to (b) or (c) when
a second program wants the same handler.

---

## N-8. Testing a cluster without a network (§0.7)

**The question.** Every std surface with a machine underneath has a pure-Salvo
double: `MemFs` for `Fs`, `ManualTime` for `Timer` [time-manual],
`RestrictedFs` as a policy interceptor [effect-intercept]. A distributed
program with no such double is the first untestable thing the language would
ship, and the parity bar — identical output on both backends — needs a
deterministic run.

### (a) `MemCluster` — N virtual nodes in one process

A `platform handler`-free handler of `Transport` that routes `Bytes` between
virtual node ids inside one process, plus a `ManualTime`-style administrative
face: `partition(a, b)`, `heal()`, `kill(node)`, `delay(node, d)`, `step()`.
`join` over it answers a `Remote Pool` that is in truth a local pool with a
serializing seam — so the codecs *run* (a test exercises the wire format) and
the failure model *runs* (a partition fires the watches), on one machine, in
`salvo test` [test-decl].

- **Cost.** A second implementation of routing, kept honest by sharing the
  membership actor (which under N-4(a) is Salvo and identical). Determinism
  needs the scheduler to run single-threaded under test (`on main`'s pool
  [main-pool] already gives cooperative scheduling) or a seeded interleaving.
- **Trade-offs.** It is exactly the filesystem's and the timer's posture, it
  makes the two hardest things (partitions, message loss) *scriptable*, and it
  gives the codegen tests a way to assert byte-for-byte codec parity across
  backends without sockets.

### (b) Real sockets on localhost in the e2e tests

The default `HostTcpTransport`, N processes, the testkit's content cache.

- **Cost.** Ports, timing, flakiness; no way to script a partition; the
  suite's wall time (AGENTS.md's budget) pays for real handshakes. **Trade-offs.**
  It tests the host class, which (a) does not; one smoke test per backend is
  worth having *in addition*.

### (c) Shape-changing alternative — deterministic simulation as the runtime's mode

FoundationDB's discipline: the scheduler library gains a simulation mode
(seeded, single-threaded, with injected faults) that *any* program can run
under, not just clusters. `salvo test --simulate seed`.

- **Cost.** A scheduler mode is runtime work on both backends and must not
  diverge. **Trade-offs.** It would make every actor program's interleaving
  reproducible, which the deadlock graph's *warnings* (back-pressure cycles)
  currently cannot demonstrate. Bigger than this phase; recorded as the
  direction (a)'s `step()` points at.

**Recommendation (the user's call): (a), plus one localhost smoke test per
backend.** `MemCluster` ships with the membership actor, std's own cluster
tests run under `salvo test`, and every example in `examples/cluster/` prints
identical output on both backends *through the codecs*.

---

## N-9. One program or many — code identity across the cluster (§0.7)

**The question.** Cloud Haskell assumes nodes share the code; Erlang loads
code per node and lets versions differ; Unison ships code by hash. Both Salvo
backends compile ahead of time and load nothing at runtime, so what may a
cluster's members *be*?

### (a) One compiled program per cluster, roles by argument

Every node runs the same binary; `main` decides what to do from its
arguments or from membership (the leader spawns the coordinator, the rest
spawn workers). The protocol hash (N-3) is checked at handshake and a
mismatch refuses the connection with a diagnostic naming both builds.

- **Cost.** A rolling upgrade is a cluster restart, or two clusters and a
  cut-over; heterogeneous services (a Kotlin front end talking to a Rust
  storage node) are one *program* compiled twice, which is possible here and
  nowhere else, but they cannot be two programs.
- **Trade-offs.** The deadlock graph stays whole-program and correct
  [actor-deadlock-cycle]; every handler a `Remote Pool` might host is in the
  binary, so a remote spawn is a *name in a table*, never code on the wire;
  the static-closure discipline is already the language's [actor-no-closure]
  [free-send-fn]. It is the honest first pass.

### (b) Many programs sharing declared protocols

A cluster is any set of programs whose *actor effects* agree; a program
declares which effects it hosts and which it only sends to, and the hash is
per effect. Rolling upgrades add an effect version, old and new coexist.

- **Cost.** The deadlock check loses its whole-program view and degrades to a
  per-effect declaration of "may wait for" — the stratification tiers
  ROADMAP.md recorded as unbuilt. Schema evolution (field tags, defaults for
  missing fields, unknown arms) enters the wire format, which N-3(a) left out.
  A handler for a protocol nobody in *this* program spawns is dead code the
  compiler cannot see is live.
- **Trade-offs.** It is what a production system eventually needs and what
  every listed system does. It should be designed *after* (a) has shown what
  the per-effect hash has to cover.

### (c) Shape-changing alternative — a module is the unit

The manifest ROADMAP.md is about to decide ("LSP source-root discovery, and a
project manifest") could name *which modules* a node builds, so a cluster is
one source tree compiled into several binaries with different `main`s and
shared protocol modules; the hash is over the shared modules.

- **Cost.** Couples this phase to the manifest decision. **Trade-offs.** It
  gives (b)'s heterogeneity with (a)'s single source of truth, and it is the
  shape a multi-`main` repository already has (`--main` picks the entry
  point). Recorded as the likely second step once the manifest exists.

**Recommendation (the user's call): (a)**, with the protocol hash designed so
that (c) is a relaxation of it rather than a replacement. What the hash *is*,
whether the user controls it, and what a mixed fleet does during a deploy is
N-10.

---

## N-10. Versions — mixed builds during a deploy (§0.9)

**The question.** The moment two processes talk, one may be older than the
other: a rolling deploy across a fleet has old and new nodes live at once for
minutes or hours. Three things have to be decided. **How does the runtime
know** two actors share a version of the code? **Is the version the user's to
control** (a declared number, a name) or a hidden detail the compiler owns?
And **how does a program read it** — for a log line, a metric, a `Node` in a
membership answer — so that "which version answered" is never a guess.

**A fact that shapes every option: Salvo cannot decode leniently.** Protobuf's
compatibility story is "unknown fields are skipped, unknown enum values are
kept as integers". Salvo's `when` over a union is exhaustive
(Control-Flow.md), and union arm identity is positional over the declared arms
(AGENTS.md's invariant): an old node receiving a message with an arm it has no
index for has *no value to construct* — there is no "unknown arm" for an
exhaustive `when` to fall through to, and inventing one would put a case in
every `when` in the program. So compatibility in Salvo is decided **before a
byte is decoded**, per connection or per protocol, and a message that would
not decode is never sent. Every option below refuses early; they differ in
*granularity* and in *who names the version*.

### (a) Hidden, whole-program: the protocol hash is the version

N-3's hash — over the canonical form of every actor effect and every type
reachable from one — is exchanged at the handshake, and a node whose hash
differs is refused: a `join` from a new build fails with a diagnostic naming
both hashes, and the old node never sees the new one. The user writes
nothing; the compiler emits the hash as a constant both backends carry.

- **Cost.** A rolling deploy is **two clusters** until the last node flips:
  new nodes cannot join the old cluster, so they form their own, and the
  cut-over is the load balancer's problem, not the program's. Any change to
  any actor effect — even one the two nodes never use between them — splits
  the fleet. There is no user-meaningful version to print: a hash is
  sixteen hex characters that mean nothing in a log line.
- **Trade-offs.** Simplest, sound by construction, and it is the N-9(a) story
  taken literally. Right for a first pass and for programs that deploy as a
  unit. Wrong for a fleet that must stay one cluster while it upgrades.

### (b) Hidden, per-protocol: one hash per actor effect

The compiler emits a hash **per actor effect** — over its members, their
parameter types in order, and every type transitively reachable from them,
in canonical form, so a comment or a renamed local changes nothing and a
renamed *member* does. Two nodes connect if their programs agree, and the
handshake exchanges the *table* of protocol hashes. Compatibility is then
decided at the three places a protocol is reached across the wire, each
refusing rather than decoding:

- **spawn onto a `Remote Pool`**: refused (an `Exit` to the spawner's watch,
  or a synchronous error at the `spawn` if placement is resolved there) when
  the target node's hash for the handler's effect differs — the node cannot
  host a handler of a protocol it does not have;
- **`lookup@Registry<E>`**: answers only addrs whose home node agrees on
  `E`'s hash, so an old node's `Ledger` is invisible to a new node whose
  `Ledger` changed — and the answer says so (`None`, or a distinct
  `Incompatible` arm if the program should be able to log it);
- **a send through an addr obtained before a node was replaced**: the addr's
  home is gone (N-5(a): a rejoined node has new identities), so this case
  cannot arise — an addr never outlives the build that minted it.

With that, old and new nodes **coexist in one cluster** and keep talking on
every protocol that did not change, which is what a rolling deploy needs.
Membership itself (`Cluster`, `Registry<E>`, `Faults`) is a std protocol like
any other and gets the same treatment — so a std change to `Cluster`'s
members is, correctly, a flag day.

- **Cost.** The handshake carries a table instead of a hash; the registry
  and the spawn path compare hashes per effect; the diagnostics have to name
  *which* effect disagreed. Structurally identical types with different
  names hash the same (names are not in the canonical form), which is fine —
  a renamed struct is compatible — but a changed field *order* is not, which
  matches the positional encoding and may surprise a user who reorders
  fields for tidiness. Still no user-meaningful version string.
- **Trade-offs.** It costs the user nothing, it is exactly as fine-grained
  as the wire format itself (the hash covers what the codec reads, no more),
  and it turns "a mixed fleet" from a forbidden state into an ordinary one
  with a precise, local rule for what may not happen. The `Registry<E>`
  reading is the important one: *discovery* is where an old and a new
  service meet, so that is where compatibility is answered.

### (c) User-declared versions, compiler-checked against the hash

The user names versions and the compiler holds them to it. Two spellings,
not exclusive:

- **On the effect**: `actor effect Ledger version 3 { … }` — a declared
  integer (or `"1.4.0"`) beside the protocol, printed in every diagnostic and
  readable from the addr;
- **In the manifest** (ROADMAP.md, "a project manifest", the next decision):
  `version = "1.4.0"` for the program, plus a **checked-in lock file** the
  build maintains — `salvo.lock`, effect → `(declared version, protocol
  hash)`. The build **fails** when an effect's hash changed and its declared
  version did not: "`Ledger`'s protocol changed (member `record` gained a
  parameter) but it is still `version 3`; bump it or revert the change". The
  compiler has no history of its own, so the lock file *is* the history, and
  a reviewer sees the bump in the diff.

Compatibility across nodes is then by declared version — equal, or a
declared range (`version 3 accepts 2`) if a program wants to say an old
sender is fine — with the hash check guaranteeing the declaration is not a
lie about the *current* build.

- **Cost.** Ceremony on every protocol change, and a lock file to commit;
  the compiler still cannot check a *compatibility claim* (`accepts 2`)
  because it cannot see version 2's shape — unless the lock file also keeps
  the last N hashes, which is a real option and turns the lock file into a
  small schema registry. A declared range also needs the wire format to
  *tolerate* something (a missing trailing field with a default, say), which
  is the N-3 format change (d) below.
- **Trade-offs.** It gives what (a) and (b) cannot: a version a human
  recognises in a log, a review-time signal that a protocol changed, and a
  place to state intent. The lock-file check is the Salvo-shaped part — a
  version that *cannot* silently go stale — and is worth having even if the
  runtime rule stays (b)'s hash equality.

### (d) Shape-changing alternative — schema evolution in the wire format

Give N-3's format field tags and defaults (protobuf's discipline): a new
trailing field with a default decodes on an old node by omission, a removed
field is skipped, and only *incompatible* changes (a changed type, a new
union arm) bump the hash. Old and new interoperate across a wider class of
changes, and version is a compatibility *range*.

- **Cost.** The positional format becomes tagged (bigger, slower to encode);
  a struct needs a notion of "field with a default the codec may supply",
  which Salvo has for *literals* (struct defaults, `surname: Str? = None`)
  but not as a wire-level rule; and unions still cannot evolve, so the
  headline case — a protocol gaining a message kind, which is a new member
  and therefore a new arm of `__Msg_E` — is exactly the one it cannot help
  with. The exhaustiveness fact above puts a hard ceiling on what evolution
  can buy.
- **Trade-offs.** Worth designing *after* (b) has run in anger and the
  changes that actually split fleets are known. Recorded, not recommended.

### Reading the version — the surface (all options)

Whatever the mechanism, a program needs to *say* which build it is, and the
posture is [time-coupling]'s: **pass it, do not read it** where possible.

- std: `export struct Build { program: Str, protocol: Str }` — the program
  hash and, under (b), the hash of the effect asked about — with `intrinsic
  fn build() -> Build` (the constant the compiler emitted; pure, no effect,
  because it cannot vary within one process) and `fn protocol<E>() -> Str`
  (the per-effect hash, the one `<E>`-over-an-effect shape [actor-types]
  already sanctions for `watch`). Under (c) a `version: Str` field carries
  the declared one, read from the manifest, so a log line prints
  `1.4.0 (a3f9…)` and a human and a machine each get the half they need.
- Membership: `Node { id: Str, build: Build }` in the `Cluster` face's
  `members` answer, so a supervisor logs which build each member runs and a
  deploy script can wait for "all members at 1.4.0" through the same actor
  the program uses.
- Diagnostics: every refusal — a `join`, a remote `spawn`, a `lookup` —
  names both sides' versions in the `Exit`/`Fault` reason it delivers, in the
  declared form if there is one, the hash otherwise.
- **Not** a `platform effect`: the version is the compiler's fact about the
  program, not the host's about the machine. A host may add its own (image
  tag, git SHA) through an ordinary platform effect if it wants them in the
  same log line.

**Recommendation (the user's call): (b) as the mechanism, (c)'s manifest
version and lock-file check as the user-facing surface, in that order.** The
runtime rule is per-protocol hash equality, decided at handshake, remote
spawn and registry lookup, never at decode — so a mixed fleet is legal and a
rolling deploy stays one cluster. The user controls a *label*, not the
compatibility rule: the manifest's `version` is what a log prints and what a
deploy waits for, and the lock file makes it impossible to change a protocol
without bumping it. Readable through `build()`, `protocol<E>()` and the
`Build` on every `Node`. (d) waits for evidence.

---

## What else has to be decided (§0.7) — recorded, not sectioned

Questions this document surfaced that are small enough to be settled inside
the sections above or during the build, listed so the next session does not
re-discover them:

- **Where does `main` run, and how many?** Under N-9(a) every node runs `main`.
  The program ends when `main` returns [actor-waitfor]; on a *worker* node
  `main` must therefore park (`waitfor` on a `Reply<Exit>` from watching the
  cluster) or the node leaves at once. A std `serve(cluster)` that waits for
  the cluster to dissolve is the obvious helper.
- **Ordering.** Per `(sender, target)` pair, in order, at most once (N-5).
  Two senders to one target interleave arbitrarily, as locally.
- **`on_idle` across nodes** [actor-on-idle]: a pool's idleness is local;
  cluster-wide quiescence is a distributed snapshot and is out of scope. The
  hook keeps its pool argument and answers for `Remote Pool` about *that*
  node's queues, or is refused on one — decide when building N-5.
- **Faults on a remote pool** [pool-fault-sink]: `anywhere(cluster, sink)` as
  the overload, the sink an addr like any other, reports crossing the wire.
- **Timeouts.** The hand-written timeout shape (two `replyto` mints racing
  into a pending map, ROADMAP.md "Actors" item 6) is the remote call's timeout
  too; no new form. `Timer` on a remote pool is meaningless — refuse it.
- **`Dedicated` and `Remote` are both provenance qualifiers on `Pool`**: can a
  pool be both? `thread()` is local by construction and `node(...)` answers a
  set of workers elsewhere; a remote dedicated thread (`thread_on(node)`) is
  plausible and would let a `[waitfor]` handler be placed remotely
  [waitfor-dedicated]. Allow both qualifiers; the `on` clause consumes a
  `Dedicated` one as it does today.
- **What `salvo platform generate` writes** for the transport: the skeleton
  should carry the thread-safety contract (ROADMAP.md's DECISION) and the
  upcall signature the runtime expects.
- **Spec labels proposed** (none exist yet): `[remote-pool]` (N-1),
  `[addr-routable]` (N-2), `[wire-format]` and `[wire-serializable]` (N-3),
  `[cluster-transport]` and `[cluster-membership]` (N-4), `[node-exit]` and
  `[remote-backpressure]` (N-5), `[addr-capability]` (N-6),
  `[cluster-registry]` (N-7), `[mem-cluster]` (N-8), `[protocol-hash]` (N-9),
  `[protocol-version]` and `[build-info]` (N-10);
  backend rules `[rs-remote]`/`[kt-remote]` for the codec and transport
  emission.

---

## Decisions pending — the table

Load-bearing order: **N-1 first** (everything is shaped by "a node is a
pool"), then **N-3** (serializability decides whether N-2's transparency can
be real), then **N-2**, then **N-4** (which makes ROADMAP.md's platform
thread-safety DECISION a prerequisite), then N-5, N-6, N-9 and **N-10**
(which fixes what N-3's hash covers and depends on the manifest decision for
its user-facing half), and N-7/N-8 last since they are libraries and tests
over the rest.

| Label | Question | Folds | Recommendation |
|---|---|---|---|
| **N-1** | Where does the network come in: a network pool, inside the thread pool, a supervisor actor, or the binding? | §0.8, §0.4, §0.1 | **A node is a pool**: `Remote Pool` as a provenance qualifier, `node(cluster, name)` / `anywhere(cluster)` in std, `on` unchanged, membership actor underneath |
| **N-3** | What may cross a machine, and who defines the bytes? | §0.1, §0.5 | **Compiler-defined canonical encoding**, codecs generated for both backends, serializability checked structurally at the actor effect's declaration, protocol hash at handshake |
| **N-2** | What is an `Addr<E>` / `Reply<T>` across the wire; do plain effects bind remotely? | §0.2, §0.4 | **One `Addr` type, routable, locality a runtime fact**; `Reply` a routable one-shot; plain effects stay local until the sugar pass's answering stub (mixed handlers bridge meanwhile) |
| **N-4** | Who owns discovery, membership, transport? | §0.5 | **Platform owns the wire only** (a `Transport` platform handler, default `HostTcpTransport` in std's `platform/`); membership and routing are Salvo actors in std; the platform thread-safety contract becomes a prerequisite |
| **N-5** | What does a program see when a node is gone? | §0.7 | **Unreachable is dead**: `watch` on a `Remote Pool`, membership policy chosen at `join`, credit-based back-pressure so `capacity` stays true, at-most-once in-order per pair |
| **N-6** | How is authority re-established on the wire? | §0.7 | **Shared secret/TLS for membership + capability bits in every crossing addr**, so the two-face guarantee survives the network |
| **N-9** | One program or many? | §0.7 | **One compiled program per cluster**, roles by argument, protocol hash designed so a per-module relaxation (with the manifest) comes later |
| **N-10** | Mixed builds during a deploy: how is "same version" known, is it the user's or hidden, how is it read? | §0.9 | **Per-protocol hash equality as the hidden mechanism**, checked at handshake, remote spawn and registry lookup — never at decode, since exhaustive `when` forbids lenient decoding — so a mixed fleet stays one cluster; **the manifest's `version` plus a lock-file check** as the user's label (a protocol change without a bump fails the build); read through `build()`, `protocol<E>()`, and `Build` on every `Node` |
| **N-7** | Which patterns are language, std, or examples? | §0.6 | **Membership and `Registry<E>` in std; election, singleton, map/reduce, sharding as `examples/cluster/`** with Raft as the flagship; `Leader` declared in std so a platform handler and a Salvo election are interchangeable |
| **N-8** | How is a cluster tested? | §0.7 | **`MemCluster`** — N virtual nodes in one process with `partition`/`heal`/`kill`, run under `salvo test`, codecs exercised; one localhost smoke test per backend |

**Delete-when-decided.** When the user has made these calls, fold the outcomes
into COMPLETED.md's decision log, turn the questions into plans (and any
undecided ones into **DECISION** rows) in ROADMAP.md, add the rules under the
labels above to LANGUAGE_SPEC.md and the backend specs, and delete this file.
