# Network pools — actors across machines, the option space (working document)

**Status: DECIDED (rounds 1–5, 2026-09-26)** — every question this document
opened has a user decision; see "Round 5 — the decided list" at the end, which
is what a session should read first. The sections stay as the argument trail.
Propagation is owed (below); **delete this file** once it lands.

**Decided so far (user, 2026-09-26, round 1).**

- **N-3 → (a), amended.** Salvo owns its serialization: a compiler-defined
  canonical encoding with codecs generated for both backends, so a Kotlin
  node and a Rust node can be members of one cluster. The user's amendment:
  **serializability is the default and the opt-out is a marker** — a type is
  serializable unless declared `noremote` (spelling provisional), and the
  process-local handles (`Pool`, streams, platform instances, function
  values, `proj` views) are `noremote` by construction. A struct holding a
  `noremote` field is `noremote` transitively. Option (c)'s opt-in marker is
  refused.
- **The check site.** An actor handler **may be spawned remotely when every
  `send fn` of the effects it serves has serializable parameters**; otherwise
  a spawn onto a `Remote Pool` is **an error at the spawn site**, naming the
  member and the `noremote` type that stops it. The declaration is not where
  the error lands (an actor effect with a `noremote` payload is a legal *local*
  protocol), which is N-2's whole-program worry answered: the check is local
  to the spawn expression and needs nothing from distant sites.
- **N-4 → (a).** The platform handler owns the **transport only**; membership,
  routing and everything above the wire are Salvo, in std, on both backends.
- **Recorded, not decided:** the "actor group" model, analysed as N-11 below,
  which reshapes N-1's `anywhere` and N-7's `Registry<E>` if taken.

**Round 2 (user, 2026-09-26, same day).** The `Addr<E>` / group separation is
taken, the type named **`AddrGroup<E>`** (N-11(b)). Still the user's call in
N-11b–d: the spelling of the effect-list claim that accepts a group (`[any E]`
recommended over `[remote E]`, since a local group is one and a remote single
addr is not), the `Policy` value shape, and `attach` for pure clients.

**Round 3 (user, 2026-09-26).** Guideline stated: *a minimal set of tools the
user builds from, then convenience for common cases* — leaning library-first
with primitives such as `pending(addr)`, routers generated like stubs, and a
policy interface that does not close off variations. N-11e compares
first-class vs library; N-11f gives the resulting kit, under which
`AddrGroup<E>` as a type is **withdrawn** in favour of `Registry<E>` + a
`Pick<E>` effect + the generated `route` stub, with `[any E]` / `of any E` as
the one language addition, shipped with the library. Still the user's call.
**Later the same day:** the two membership levels named **`NodeGroup`**
(nodes) and **`ActorGroup<E>`** (actors of one effect, across nodes) — N-11i,
which also answers why they take names and why both are actors.

**Provenance.** Written 2026-09-26 by the assistant, in a read-only session
while another agent held the repository, at the user's request to explore
extending actors to run across a network. It follows DESIGN_DOC.md's skeleton
and lays the design out as options, trade-offs and recommendations, in the
manner of CONCURRENCY.md (phase 5) and FILE_SYSTEM.md (phase 4) before it. It
does not decide anything; it exists so that the language-design calls this
would force are chosen before a line of it is built. The label prefix is `N-`.

**Propagation: landed 2026-09-26 (writable session).** COMPLETED.md's log has
the round's entry ("Actors across machines — the network round"); ROADMAP.md
section 2 holds the build sequence as the plan, the thread-safety DECISION is
closed there, and the manifest DECISION notes the version label as its second
customer. Spec rules land with each step — **step ① landed 2026-09-26**
([threadsafe-platform], [net-transport], [net-host], [net-mem]). This file is
the argument trail until step ⑧ lands, then it is deleted.

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

> **Decided 2026-09-26 (round 1): (a), with two amendments** — see "Decided
> so far" in the header. Serializable is the *default* and `noremote` the
> opt-out on a type; and the check lands at the **spawn site** (a handler whose
> served `send fn`s all take serializable parameters may go on a `Remote
> Pool`; otherwise the spawn is the error), not at the declaration as the
> paragraph above proposed. A `noremote` payload therefore makes a protocol
> local-only without making it illegal.

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

> **Decided 2026-09-26 (round 1): (a).** Transport only is platform; Salvo
> builds on top.

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

## N-11. Actor groups — routing to *any* member (user sketch, round 1)

**The sketch.** Code using a remote actor effect does not target one actor
but an **actor group** that runs on a cluster or thread pool. To join a group a
process must spawn at least one actor of the relevant type, which guarantees a
member exists and gives a routing rule: try the local members; if all are
busy or full, try the remote ones; if those are all busy, wait. The user
suspects this is a supervisor over a more general cluster, asks whether it
cuts off patterns such as leader election (how does everything go through one
leader?), and notes the local-only version is wanted too: run 100 copies of an
actor on a pool and route to whichever is free.

### What it is

It is a **router**: one address, many interchangeable actors behind it, a
policy choosing which gets each message. Akka has exactly this pair of words —
a *pool router* spawns its routees, a *group router* routes to routees
registered elsewhere, with round-robin, random and smallest-mailbox policies
and a cluster-aware variant that prefers local routees. Erlang's `pg` (process
groups) plus a picker is the same thing by hand; Orleans' `StatelessWorker`
grain is the auto-scaled, always-local version. So yes: it is "a supervisor
built on top", *semantically*. But it is not something a Salvo program can
write generically today, because a handler cannot be generic over an *effect*
— `Addr<E>` is the one place an effect is a type argument [effect-not-data] —
so a hand-written group is one forwarding handler **per effect**
(`handler LedgerGroup(members: List<Addr<Ledger>>) of Ledger`, every member
re-declared to forward). That is the boilerplate the compiler already
generates as `__Stub_E` [actor-use-addr], and generating a *routing* stub
beside the forwarding one is a small step. So the honest classification is:
**a runtime + std feature with no syntax**, sitting exactly where the
forwarding stub sits — provided one type question is answered (below).

### What it clarifies in N-1

The sketch says the group "runs on a cluster/thread pool", and that phrase
carries two decisions the document had folded into one word. **Placement** —
where a *member* lives — is the `on` clause and stays with the spawn site
[actor-spawn-expr]. **Routing** — which member gets a *message* — is the
group's policy and happens at the send, inside the stub. N-1's
`anywhere(cluster)` conflated them: it placed a new actor "somewhere" *and*
implied sends would find it. Under the group model `anywhere` is not needed
for sends at all — a group routes — and placement is always to a named pool
(`node(cluster, n)` or a local pool). A group is therefore **a set of
members of one effect, each placed by its own spawn, reachable by one
address**. Joining a group *is* spawning a member and registering its addr.

### What it depends on, and interacts with

- **`Reply<T>` is untouched.** A reply routes to the specific minter (N-2); only
  the request direction is group-routed. Gated mints (`replyto!`) on a member
  behave as they do on any actor.
- **Back-pressure and the routing rule.** "Busy or full" needs queue depth.
  Locally the scheduler has it; remotely it needs N-5's **credits** (i) — the
  sender's view of a remote member's free capacity — which the document
  already recommends and which the group makes load-bearing. Two thresholds
  should be kept apart: *busy* (an activation is running) and *full* (the
  mailbox is at `capacity`). Routing on *full* makes `capacity` a
  load-balancing knob rather than the back-pressure bound it was declared as
  [actor-mailbox]; routing on *busy* is the smallest-mailbox router and
  spreads work earlier. Recommend "least-loaded local, spill to least-loaded
  remote past a threshold below capacity", with the policy a value on the
  group rather than a fixed rule.
- **The deadlock graph is unchanged.** It is over types, not instances
  [actor-deadlock-cycle]; a group is many instances of one type, so a chain of
  same-protocol workers already reads as a self-loop (ROADMAP.md's recorded
  gap). Nothing new to price.
- **N-10 folds in.** Joining a group is the "remote spawn / registry lookup"
  seam where per-protocol hash equality is checked: a joiner whose hash for
  `E` differs is refused at `join`, with both versions named. A group is
  therefore also **N-7's `Registry<E>`**, since it is a named set of addrs of
  one effect — the two collapse into one std actor.
- **`watch` on a group** fires when it is *empty*, the group analogue of a
  `Remote Pool`'s exit (N-5). Members die individually and are watched
  individually by whoever spawned them, who respawns — the supervision
  pattern, no syntax.
- **`use group`** is `use addr` with a different stub; a function declaring
  `[E]` never learns whether its `E` is one actor, a group of a hundred on this
  pool, or a group across a cluster. That is §0.2 kept whole.

### What it cuts off, or costs — the flags

1. **Per-pair ordering is lost inside a group.** Today two sends from one
   frame to one actor are processed in order, and "the serialization *is* the
   mutual exclusion" holds because there is one mailbox. Across a group,
   `counter.bump(2); counter.bump(3); counter.total(out)` may run on three
   members and answer `0`, `2`, `3` or `5`. This is not a defect of the model
   — it is what "interchangeable" means — but it must be **visible in the
   type** or it silently breaks every program written for a single actor.
   Hence the type question below.
2. **State per member is disjoint.** A group of `Ledgering` is a hundred
   ledgers. The model fits *stateless workers* (transform, fetch, compute) and
   *keyed* state (route by key — sharding); it does not fit an actor whose
   state is the point. Single-actor addressing must remain the ordinary
   thing, and it does — a group is opt-in at the binding.
3. **The join rule conflates client and server.** "A process must spawn at
   least one member to join" gives a liveness guarantee (some member exists)
   and a code guarantee (the joiner compiled the handler, so N-10's hash check
   is automatic). But a *pure client* — a front end that only sends to
   `Ledger`, a node without the GPU or the database connection the handler
   needs — is forced to host one. Recommend: hosting is the *default* reading
   of `join` but a client-only attach exists; "at least one member" becomes an
   invariant of the **group** (a send to an empty group parks and the idle
   report names it) rather than of each joiner. A client with no local member
   routes remote-first trivially.
4. **Local-first is a hot-spot policy.** Every node prefers itself and spills
   only when full, so under uneven arrival one node saturates while others
   idle. Fine as a default for latency; wrong as the only policy — hence the
   policy value.
5. **A hundred copies on four threads help only if activations park.** For
   CPU-bound members, 100 on `pool(4)` buys nothing over 4; for members that
   wait on replies (IO through a remote service, a timer) it is exactly the
   right shape. Worth a sentence in the docs, not a rule.
6. **Elastic groups need replayable construction.** Orleans auto-scales
   stateless workers; a Salvo `elastic(max: 100)` policy would have to spawn
   `Working(args)` again later, so the constructor arguments must be `copy`-
   able and held by the group — [effect-state-store] territory. Recorded;
   the fixed-size group needs none of it.

### Leader election under the group model

Not cut off — **not provided**, and the reason is instructive. A group
routes to *any* member; a leader is *exactly one* member everything routes to.
Those are two **policies over the same mechanism**: `any` (stateless work),
`by key` (sharding: the member owning the key, by consistent hash), and
`leader` (singleton: the elected member). Under `leader` the group's picker
consults a `Leader` effect (N-7) — a Salvo election over membership and
`Timer`, or a platform handler over etcd/a Kubernetes lease — and the
"at least one member per joiner" rule turns out to be exactly what Raft
wants: every node runs a candidate, one is elected, the rest are cold
standbys that route to it. What the mechanism cannot give is what nothing
can: a *single* leader across a partition (N-5's policy at `join`) and the
*old leader's state* on failover — the new leader has its own state unless
the program replicated it, which is Raft's log and the program's job. So
N-7(c)'s `singleton`/`sharded` placement values become **group policies**, and
the three patterns unify under one addr type. That is a better shape than the
document had.

### The type question — the one thing to decide

Is a group an `Addr<E>`, or its own type? Three options.

- **(a) A group *is* an `Addr<E>`**; a single actor is a group of one.
  Maximal transparency, zero new vocabulary — and flag 1 becomes a silent
  hazard: no type says whether two sends are ordered, so every `Addr<E>` loses
  the ordering guarantee or the checker has to know at each site which kind it
  holds, which storage (`List<Addr<E>>`) destroys, as N-2(b) found.
- **(b) `AddrGroup<E>`, a second intrinsic type in `core.actor`.** Copyable, never
  linear, serializable, bound with `use` like an addr (the stub differs by one
  pick call), and *not* an `Addr<E>` — so `Addr<E>` keeps its per-pair
  ordering, a group promises none, and a program that stores an addr where a
  group is wanted (or the reverse) is told so. `watch(group, …)` and
  `join(group, addr)` are ordinary `[spawn]` functions; the policy is a
  constructor argument (`group<E>(any)`, `group<E>(by_key)`,
  `group<E>(leader)`). Conversion is one-way and explicit: `join` puts an addr
  in; nothing takes one out.
- **(c) Shape-changing — a provenance qualifier, `Any Addr<E>`.** Says "this
  addr may route anywhere" in the type without a second type; erased in the
  generated code [qual-erasure], survives stores as the *weaker* claim. The
  trouble is the direction of the claim: a qualifier is dropped freely
  [qual-subject], so an `Any Addr<E>` stored in a `List<Addr<E>>` comes out
  looking ordered — the hazard of (a), reached by erasure.

**Recommendation (the user's call): (b).** An `AddrGroup<E>` beside `Addr<E>`, no
syntax, the routing stub generated beside the forwarding one, `join` as the
registration and the place N-10's per-protocol check runs, policies as values
(`any` first; `by_key` and `leader` as N-7's examples mature into them), and
the join rule relaxed so a client need not host. It subsumes N-1's
`anywhere` for sends (placement stays `on POOL`), replaces N-7's `Registry<E>`,
and gives the local hundred-workers case with the same word as the cluster
case. The ordering flag is the reason for the second type, and it is the whole
cost.

> **Decided 2026-09-26 (round 2): (b), named `AddrGroup<E>`.** The user took
> the separation and asked four questions, answered in N-11b–d below: what a
> function sees, how it knows about ordering, how a policy is given, and what
> a pure client writes.

### N-11b. What a function sees — and the `[any E]` claim (round 2)

**The question.** A function declaring `[E]` never learns what is behind `E`
(§0.2). Then how does it know whether two sends are ordered? A function
written `bump(2); bump(3); total(out)` is *correct* against an `Addr<Counter>`
and *wrong* against an `AddrGroup<Counter>`, and nothing in its body says
which it assumed.

The answer has to be in the **effect list**, because that is the one place a
function states what it needs from a binding, and there is a precedent for a
weakening there: `[local E]` [use-local] says "I need less than a shareable
binding and claim fewer rights". The user's suggestion — `remote` as the other
end of `local`'s spectrum — is the right *place*; what remains is what the
word means and which way the default points.

**Two orthogonal weakenings, not one spectrum.** `local` weakens *seam
rights*: the binding need not be shareable, and the function may not pass it
across a seam. The group question weakens *identity*: the binding need not be
one instance, and the function may not assume per-pair ordering or shared
state between its sends. A single remote actor reached through an `Addr` keeps
ordering (N-5: in order per pair), so "remote" as a location is *not* what a
group takes away; location is a runtime fact under N-2(a) and never in a type.
The property a group removes is *one-ness*.

```
fn tally(items: List<Int>) [Counter] -> Int {          // needs ONE Counter
    for n in items { bump(n) }                          // relies on order
    return waitfor out: Reply<Int> { total(out) }
}

fn thumbnail(img: Image) [any Resizer] -> Image {      // ANY Resizer will do
    return waitfor r: Reply<Image> { resize(img, 100, r) }   // one send, one reply
}
```

- **The default is the strong claim.** Bare `[E]` keeps meaning what every
  program written so far assumed: one instance, sends in order, a shareable
  binding. So binding an `AddrGroup<Counter>` where `[Counter]` is required is
  **an error at the `use` site**, and existing programs cannot break by having
  a group bound under them:

  ```
  let counters = group<Counter>(any)
  use counters
  tally([1, 2, 3])
  // error: `tally` declares `[Counter]`, one ordered instance, but `Counter`
  //        is bound to an `AddrGroup<Counter>` here. Declare `[any Counter]`
  //        in `tally` if any member may take each send, or bind one `Addr`.
  ```

- **`[any E]` is the opt-in**: "each send may go to a different instance; I
  assume no order between my sends and no state shared across them." It
  accepts *every* binding — a group, an addr, a handler, a monitor — since a
  single instance is a group of one and gives more than was asked. It is
  **viral downward** exactly as `local` is: a function declaring `[any E]` may
  call only functions declaring `[any E]` for that effect (they might rely on
  order); a function declaring `[E]` may call either. Same direction as
  `local`: the weaker claim is satisfied by the stronger binding, and a body
  cannot hand a callee more than it was given. The spec already records
  `local`'s virality as "an accepted cost, to be lifted later by inference";
  `any` has a cleaner inference target — *at most one send to `E` per path*
  makes `any` provable — but that is later.
- **Combining with `local`.** `[local any E]` would mean "any binding
  whatsoever, no rights at all". A group handle is always shareable, so a
  group never *needs* `local`; the combination is legal but pointless, and
  refusing it as redundant (as a fn type refuses `local`) is the tidy call.
- **Effect kinds unchanged.** `any` is not a new effect kind; `Resizer` is an
  ordinary `actor effect`. What the routing stub does with it is the
  binding's business.

**The word.** Three spellings for the same claim, the user's call:
`[any E]` (reads "any Resizer will do", says what is true — a local group of
a hundred on `pool(4)` is `any`, not remote); `[remote E]` (the user's, reads
as `local`'s opposite, but a remote single `Addr` is *not* this and a local
group *is*, so it names the wrong property); `[grouped E]` (exact, clumsy).
**Recommend `any`**, with the doc stating the `local`/`any` pair as two axes
rather than one spectrum.

### N-11c. Specifying the policy (round 2)

The policy is a **value passed when the group is made**, not syntax, so a
future policy is a new value rather than new grammar (the `Mailbox`-as-struct
precedent [actor-mailbox]):

```
// core.actor (sketch)
export intrinsic type AddrGroup<E>
export union Policy = Any | ByKey | Leader
export struct Any    { prefer_local: Bool = true, spill_at: Double = 0.5 }
export struct ByKey  {}                     // the key is marked on the protocol
export struct Leader {}                     // consults the `Leader` effect in scope

export intrinsic fn group<E>(policy: Policy) [spawn] -> AddrGroup<E>          // local, unnamed
export intrinsic fn group<E>(name: Str, in: Cluster, policy: Policy) [spawn] -> AddrGroup<E>
export intrinsic fn join<E>(g: AddrGroup<E>, member: Addr<E>) [spawn] -> None => g, member
export intrinsic fn watch<E>(g: AddrGroup<E>, on_empty: Reply<Exit>) [spawn] -> None => g, !on_empty
```

- **`Any`**: least-loaded local member first; spill to the least-loaded
  remote member when local queues pass `spill_at` of `capacity` (the
  busy/full distinction from the flags); if every member is at capacity the
  send **blocks**, as a send to one full mailbox does. `prefer_local: false`
  is round-robin over the whole membership.
- **`ByKey`** (later): the member that owns a key by consistent hash, so
  keyed state is partitioned rather than duplicated. The key has to be found
  in every message, and the Salvo-shaped way to say which parameter is a
  **qualifier on the parameter type** in the protocol — a claim, read by the
  stub emitter like any qualifier and erased in the output [qual-erasure]:

  ```
  export qualifier Key of T          // core.actor: "route by me"

  actor effect Shard {
      send fn put(k: Key Str, v: Bytes) => !k, !v
      send fn get(k: Key Str, out: Reply<Bytes?>) => !k, !out
  }
  ```

  `group<Shard>(ByKey {})` is refused if some member of `Shard` has no `Key`
  parameter, or two. The key type needs `hash` (`Hashed`, [cmp-auto]) — the
  implicit the collections already resolve.
- **`Leader`** (later): the picker asks the `Leader` effect in scope at the
  group's creation — `group<Ledger>(Leader {})` requires `[Leader]` on the
  creating function, exactly as any dependency — so a Salvo election over
  membership and `Timer`, and a platform handler over etcd or a Kubernetes
  lease, are interchangeable behind it (N-7). Every joiner hosts a candidate;
  sends go to the elected one; on `Exit` of the leader the picker re-asks.

Group *names* are cluster-scoped (`"resizers"` in cluster `"images"`): the
same name from two nodes is one group, which is how a client attaches (below),
and the N-10 per-protocol hash is checked at `group`/`attach`/`join` for that
name — a joiner whose `Resizer` differs is refused there, naming both builds.

### N-11d. Hosting node vs pure client (round 2)

The **hosting** version, which is the default reading of the sketch: a node
that joins a group by spawning members, and then also uses the group.

```
import net.*

actor effect Resizer {
    send fn resize(img: Image, width: Int, out: Reply<Image>) => !img, !width, !out
}

handler Resizing() of Resizer {
    mailbox { capacity: 4 }
    send fn resize(img: Image, width: Int, out: Reply<Image>) => !img, !width, !out {
        out.send(scale(img, width))
    }
}

fn thumbnail(img: Image) [any Resizer] -> Image {
    return waitfor r: Reply<Image> { resize(img, 100, r) }
}

fn main(args: List<Str>) [use, spawn] {
    use HostTcpTransport("0.0.0.0:7000")          // N-4: the wire, host code
    let cluster = join("images", seeds(args))     // membership: a std actor
    let resizers = group<Resizer>("resizers", cluster, Any {})

    let workers = pool(4)
    for i in range(0, 16) {                        // host 16 members: they park on IO
        join(resizers, spawn Resizing() on workers)
    }

    use resizers                                   // binds `Resizer` for [any Resizer]
    let t = thumbnail(load("a.png"))               // may run here or on any node
    serve(cluster)                                 // park until the cluster dissolves
}
```

The **pure client**: a node that sends but hosts nothing. It attaches to the
name instead of creating or joining, gets the same `AddrGroup<Resizer>`, and
its `Any` policy has no local members to prefer — so it spills immediately,
which is the right behaviour for a client.

```
fn main(args: List<Str>) [use, spawn] {
    use HostTcpTransport("0.0.0.0:0")            // an ephemeral port: it only calls out
    let cluster = join("images", seeds(args))
    let resizers = attach<Resizer>("resizers", cluster)   // AddrGroup<Resizer>, no members of ours

    use resizers
    println(size(thumbnail(load("a.png"))))
    // no `serve`: main returns, the node leaves, nothing here was hosting anything
}
```

Three things to read off the pair. **The consumer is identical** —
`thumbnail` and its `[any Resizer]` are the same text on both nodes, which is
§0.2. **The difference is three lines at the binding** — `group` + `join` loop
+ `serve` against `attach` — which is §0.3. And **the invariant moved from the
joiner to the group**: `attach` to a name nobody has hosted yet answers a
group with no members, a send to it parks until a member joins, and if none
ever does the idle report names the parked sender — so the sketch's liveness
guarantee is kept where it is checkable (the group) instead of forced on a
node that has nothing to host. `attach` is refused for a *local* unnamed group
(nothing to attach to), which is the only asymmetry.

**The `noremote` error, for completeness** (N-3 as decided):

```
noremote struct Canvas { surface: GpuSurface }    // process-local by declaration

actor effect Painter { send fn paint(c: Canvas, out: Reply<Image>) => !c, !out }
handler Painting() of Painter { … }

let far = node(cluster, "gpu-2")
spawn Painting() on far
// error: `Painting` cannot be spawned on a Remote Pool: `Painter.paint`
//        takes `Canvas`, declared `noremote`. Spawn it on a local pool, or
//        make `Canvas` serializable.
```

The same handler on `pool(2)` compiles as it does today; joining a *cluster*
group with it is refused at `join` for the same reason, since a group's
members may be reached from any node.

### N-11e. What first-class support buys — the library alternative (round 3)

**The question.** Could a program get this without an intrinsic `AddrGroup<E>`
and a new word in the effect list? Mostly yes, and it is worth being exact
about which parts are library, which are a small std intrinsic, and which
cannot be had without the compiler.

First, what the proposal actually adds: **one intrinsic type** (`AddrGroup<E>`,
beside `Addr<E>`), **std functions** (`group`, `join`, `attach`, an overload of
`watch`), a **generated routing stub** beside the forwarding stub the compiler
already emits per actor effect, and **one contextual word** in an effect list
(`any`, beside `local`). No new statement forms; `noremote` belongs to N-3.

**The library version.** A handler of an actor effect may be bound with `use`
[actor-kind] (the mailbox is inert), and a stateful one binds as a monitor
[use-local], so a router is an ordinary handler, one per effect:

```
handler ResizerGroup(members: List<Addr<Resizer>>) of Resizer {
    mailbox { capacity: 1 }                   // inert under `use`
    next: Int = 0
    send fn resize(img: Image, width: Int, out: Reply<Image>) => !img, !width, !out {
        let m = members[next % members.size()]
        next = next + 1
        m.resize(img, width, out)             // forward; the reply routes to the minter
    }
}

fn main() [use, spawn] {
    let workers = pool(4)
    let members = [spawn Resizing() on workers, spawn Resizing() on workers, …]
    use ResizerGroup(members)                 // `thumbnail` sees `[Resizer]`, as ever
    thumbnail(load("a.png"))
}
```

That is round-robin over a fixed set, local or remote (an `Addr` is
serializable, so `members` may come from a `Registry<Resizer>` lookup on a
cluster). Transparency to the consumer is *unchanged* — `use ResizerGroup(…)`
is one line, as `use resizers` is. Naming and client attach are a std
`Registry<E>` actor (N-7) either way — a *library*, since membership is std
under N-4. So four things remain, and they are the whole value of baking it
in:

1. **Genericity.** No handler can be generic over an *effect*
   [effect-not-data], so the router is rewritten per protocol — a dozen
   mechanical lines that are exactly the forwarding stub the compiler already
   generates. Cheap to add as a generated stub; **or** the alternative pays
   more broadly: a general *delegating handler* form (`handler Router<E>
   (members: List<Addr<E>>) of E { … }`, every member forwarded through a
   pick) would give logging, retry and routing interceptors generically, and
   is a bigger feature than `AddrGroup` — a **DECISION**-shaped alternative,
   recorded.
2. **Load-aware routing.** "Least-loaded" needs a member's queue depth, and a
   library cannot see a mailbox: there is no `pending(addr)` and no
   `try_send`, so user-land policy is round-robin, random or hash. This is a
   **std intrinsic**, not a type — `intrinsic fn pending<E>(a: Addr<E>) ->
   Int`, backed locally by the scheduler and remotely by N-5's credits — and
   it is worth adding whichever way the rest goes.
3. **A hop-free dynamic router.** Members join and die. A router that must
   *receive* `joined`/`left` and the `Exit` of a watched member has to be an
   actor (a monitor cannot receive messages), so every send takes an extra
   hop through one mailbox whose `capacity` becomes the group's bound — a
   serialization point in front of the thing that exists to remove one. The
   runtime's stub has no queue of its own: the pick happens on the sender's
   thread against a shared membership table. This is the one **performance**
   property a library cannot reach.
4. **The ordering check.** `[any E]` is a static guarantee that a function
   written against one instance is never bound to many. No library provides
   it; a router handler bound under `[Counter]` compiles and `tally` answers
   nonsense. It is also the one part with a *cost* — a viral annotation — and
   the repository's habit for such checks (the deadlock graph's
   stratification, minter attribution) has been to **add them when the false
   negative is observed**.

**Options, then.**

- **(a) Library first.** Ship `Registry<E>` (needs a small checker allowance:
  a generic *effect* parameter used only inside `Addr<E>`, as `watch<E>`
  already does for a function), `pending(addr)`, and `examples/cluster/`
  with a hand-written router per effect. Add the generated stub when the
  boilerplate or the router hop is *observed* to matter; add `[any E]` when
  an ordering bug is. Cost: every early program writes routers by hand and
  has no ordering guard; the router-hop bottleneck is real from day one for
  any dynamic group.
- **(b) The proposal as written.** `AddrGroup<E>` + `[any E]` now. Cost: a
  type and a viral annotation before either has a program to justify it; the
  library shape is never tried.
- **(c) Shape-changing — the delegating handler instead.** Build genericity
  over effects (1) as the feature; groups, interceptors and retries all
  become library handlers; `pending` covers (2); (3) is accepted as a hop or
  the delegating handler is allowed to be *stateless-and-lockless* so a
  `use` of it costs nothing; (4) is deferred. Bigger, more general, later.

**Recommendation (the user's call): (a), with (b)'s stub as the planned
second step and (4) explicitly deferred.** The value of first-class support
is real but narrow — the hop-free pick (3) and the ordering guard (4) — and
both are the kind the repository has preferred to add after a program wants
them. What should land now regardless is the enabling std: `Registry<E>`,
`pending(addr)`, and the per-effect router as a worked example, because
those are the library floor under every option. Keep `AddrGroup<E>` and `any`
as the names for when the stub arrives; the section above is the design.

### N-11f. A minimal kit, and where a policy lives (round 3, user guideline)

**The guideline.** Introduce a minimal set of tools the user can build from,
then add convenience for the common cases. The user leans library-first with a
few primitives (`pending(addr)` among them), wants routers *generated* as the
stubs are, and is concerned that a fixed policy list (`Any`, `ByKey`, `Leader`)
artificially limits what a user can do — singleton-within-a-key, scatter to
several and merge, hedge two leaders and take the first answer, are all still
"`any E`". Two questions were asked on the way: what `Registry<E>` is *for*,
and whether `[any E]`'s virality is a cost at all.

**What `Registry<E>` is for.** It is the membership half of a group with the
routing half removed: a cluster-wide *name → set of `Addr<E>`*, which is what
a node needs to reach members it did not spawn (the pure client), the actor
that observes `join`/`leave` and members' `Exit`s so the set stays true, and
the seam where N-10's per-protocol hash is compared. It is generic over the
*effect* because that is what keeps the answer typed — `members(name) ->
List<Addr<Resizer>>` — where a `Str`-keyed registry of "some addr" cannot be
written at all [effect-not-data]. One registry instance per protocol. It needs
one small checker allowance: a generic parameter of a **declaration** (here an
actor effect) standing for an effect, used only inside `Addr<E>`, which
`watch<E>` already has for a *function*. With an `AddrGroup<E>` type the
registry is inside it; without, it is the std actor every router is built
over. Either way it is the one piece a user cannot write, because it is the
thing the transport delivers *to*.

**Virality as a feature.** The user's reading is right and sharper than the
document's: `[any E]` is a statement about the *guarantee the effect gives*,
and a hand-written router **hides** the loss of ordering — `handler
ResizerGroup(...) of Resizer` bound with `use` satisfies a bare `[Resizer]`
and `tally` compiles against it and answers nonsense. So the claim wants to
exist on **both sides**, and this is the small language addition that makes a
library router honest:

- **Requirement side**: `[any E]` in an effect list — "I assume no order and
  no shared state between my sends to `E`" — accepting any binding, viral
  downward as `local` is (N-11b).
- **Binding side**: a handler declares the weaker guarantee in its `of`
  clause — `handler ResizerGroup(...) of any Resizer` — and a `use` of it
  binds `any Resizer`, which satisfies **only** `[any Resizer]`. A generated
  router stub declares `of any E` automatically. A handler that forwards to
  many members and omits `any` is telling a lie the checker cannot see;
  acceptable, and the one gap.

This is a contextual word and one checker rule mirroring `local`'s, no type,
and it carries the whole ordering guard on its own. **Recommend shipping it
with the library**, not after: without it the library is unsafe in exactly the
way the user identified. Spelling: `any` (says what is true; a `List<Addr<E>>`
of one is `any`), `volatile` (the user's; in C it means "re-read from memory",
which is a different loss and may mislead), `some`. Recommend `any`.

**Where a policy lives — the flexibility cut.** The policies the user lists
fall into two kinds by *what they need to see*, and the line between them is
where the generic/protocol-specific boundary already is in the language:

1. **Pick one member, without reading the message.** `Any`, least-loaded,
   round-robin, `ByKey` (the key is the one thing the protocol *exposes*
   generically, through the `Key` qualifier), `Leader`, singleton-within-a-key
   (ByKey where the owner is the key's elected leader — still a pick), prefer
   local, prefer a zone. Everything here is a function of the **membership
   view** and nothing else, so it can be **generic over `E`** and written
   once for every protocol.
2. **Read the message, or the replies.** Scatter to several and merge; hedge
   two and take the first; retry on a member's death; dedupe. Each has to
   *see* the payload (to fan it out — a payload is consumed by a send, so
   fanning out means `copy`) and/or *receive replies* (a `Reply<T>` is
   linear, so two members cannot answer one token: the router mints its own
   tokens, collects, and answers the caller's once — which makes the router
   an **actor**, since something has to receive). How to merge two `Image`s
   or which of two `Ledger` answers wins is **domain code**, so these are
   inherently per-protocol and are *not* limited by any generic interface:
   they are handlers `of any E` the user writes, with the sugar pass's
   remote mint making the collect step pretty.

So the flexible interface is: **the generic layer is a pick over a view; the
protocol-specific layer is a handler.** The proposal's mistake was to make
`Policy` a closed union of *values*; the minimal kit makes it a *handler of a
std effect*:

```
// core.actor — the kit (sketch)
export struct ActorView<E> { addr: Addr<E>, pending: Int, local: Bool }
export struct ActorGroupView<E> { actors: List<ActorView<E>>, key: Long? }   // key: hash of the `Key` param, if any

export effect Pick<E> {
    fn choose(view: ActorGroupView<E>) -> Addr<E>?      // None: nothing eligible — the send parks
}

export intrinsic fn pending<E>(a: Addr<E>) [] -> Int => a          // scheduler locally, credits remotely
export actor effect Registry<E> { … }                              // name → members, join/leave, Exit tracking
export intrinsic fn route<E>(r: Addr<Registry<E>>) [use] -> ...    // binds the generated stub: `of any E`
```

A policy is a handler of `Pick<E>`: stateless (`Hashed` key → member) binds
bare; stateful (round-robin's cursor) binds as a monitor — the pick runs on
the **sender's thread under a brief lock, not a queue**, which keeps the
hop-free property (N-11e's item 3) for every pick that is not itself an actor;
and one with dependencies (`handler LeaderPick<E> [Leader] of Pick<E>`)
captures them as owned handles at construction [use-local], which is how
`Leader` stops being a built-in policy and becomes a nine-line handler over an
effect a platform handler can also serve. `Any`, `ByKey`, `Leader` ship in
std *as handlers*, and the user's variations are more handlers, or (kind 2)
handlers `of any E`. Nothing is closed.

```
handler LeastLoaded<E>() of Pick<E> {
    fn choose(view: ActorGroupView<E>) -> Addr<E>? {
        return min_by(view.actors, m => m.pending)?.addr     // local-first is a filter before it
    }
}

fn main() [use, spawn] {
    let reg = attach<Resizer>("resizers", cluster)          // Addr<Registry<Resizer>>
    use LeastLoaded<Resizer>()                              // the policy, an ordinary `use`
    use route(reg)                                          // the generated stub: binds `any Resizer`
    thumbnail(load("a.png"))                                // [any Resizer]
}
```

And the concern about "does the pick see the message?" is answered by the cut:
a *generic* pick sees exactly what every protocol exposes — members, load,
locality, and the `Key` hash — and anything that must see more is by
definition not generic and is written as a handler for that protocol. That is
the same line the language draws between `Addr<E>` (generic over the effect)
and a handler (specific to it).

**What this changes upstream.** `AddrGroup<E>` as a *type* is no longer
needed: a group is `Addr<Registry<E>>` plus a `Pick<E>` in scope plus the
generated stub, and "what crosses the wire" is the registry's addr, which is
an ordinary addr. The stub is the one generated artifact (the user's wish),
`[any E]` / `of any E` the one language addition, `pending` / `Registry<E>` /
`Pick<E>` the std kit, and `Any`/`ByKey`/`Leader` become convenience handlers
in std — the guideline applied. Sections N-11b–e stand as the argument trail;
where they say `AddrGroup<E>` read "the stub bound by `route`".

**Recommendation (the user's call):** this kit as the first pass — `Registry<E>`,
`pending`, `Pick<E>` with three std handlers, the generated `route` stub, and
`[any E]` + `of any E` shipped together with it; scatter/hedge/retry as
`examples/cluster/` handlers `of any E`.

### N-11g. Under the hood — how `ActorGroupView` is built, and the `Registry` in use (round 3)

**Is `ActorGroupView` read from the `Registry`?** Indirectly, and the indirection
is the point. A pick runs on the sender's thread at every send, so it cannot
*ask* the registry — a request/response per send would be the hop the stub
exists to avoid, and a `waitfor` on a hot path. The stub reads a **local
mirror** of the registry's member set instead, and the registry's job is to
keep every node's mirror current. Three layers, bottom up, and only the
bottom one is an intrinsic:

1. **A per-node name table in the runtime** — the one primitive under all of
   it, and where N-10's check physically happens:

   ```
   export intrinsic fn publish<E>(name: Str, a: Addr<E>) [spawn] -> None => name, a
   export intrinsic fn resolve<E>(node: Node, name: Str) [spawn] -> Addr<E>?  => node, name
   ```

   `resolve<Registry<Resizer>>(n, "resizers")` asks node `n`'s table for the
   addr published under that name **and** `Registry<Resizer>`'s protocol hash;
   a node whose hash differs answers `None`, so an incompatible build is
   invisible rather than misread. This is the only `Str → addr` table in the
   design, and it is typed at the call by `<E>`, which is what [effect-not-data]
   allows and a Salvo `Map<Str, Addr<?>>` cannot be.

2. **`Registry<E>` — one replica per node, gossiping.** Not a cluster
   singleton (which would be the leader problem it exists to serve) but the
   Erlang `pg` shape: each node that creates or attaches to a name spawns a
   local `Registering<E>` and publishes it; replicas find their peers with
   `resolve` over `members(cluster)` and exchange member sets (an add-wins set
   over addrs — a member is `(node, id, bits)`, so removal is exact); a node's
   departure, reported by `Cluster`, removes every addr homed there. Reads are
   local; a join is visible cluster-wide after one gossip round.

   ```
   export actor effect Registry<E> {
       send fn join(member: Addr<E>) => !member
       send fn leave(member: Addr<E>) => !member
       send fn members(out: Reply<List<Addr<E>>>) => !out
       send fn subscribe(w: Addr<Changes<E>>) => !w        // push, for mirrors and supervisors
       send fn merge(peer: Set<Addr<E>>) => !peer          // gossip, replica to replica
   }
   export actor effect Changes<E> {
       send fn joined(member: Addr<E>) => !member
       send fn left(member: Addr<E>) => !member
   }
   ```

   Written in Salvo. The handler holds `members: Mut Set<Addr<E>>`,
   `watchers: Mut List<Addr<Changes<E>>>`, declares `[spawn]` to `watch` each
   joiner — `watch(member, replyto died(member))`, a continuation with a
   capture, so a member's `Exit` becomes a `left` — and depends on `[Cluster]`
   for node changes. It is a plain actor: nothing here is new to the language.

3. **The mirror and the stub.** The stub is `use`-bound and cannot receive
   messages, so `route(reg)` does two things: it spawns a tiny local
   `Mirroring<E>` actor of `Changes<E>` that `subscribe`s to the replica, and
   it binds a `View<E>` **monitor** [monitor-handler] the mirror writes into —
   an addr set behind a lock. The generated stub depends on `[View<E>,
   Pick<E>]`: per send it takes a snapshot of the set, decorates each addr with
   `pending(addr)` (the scheduler's queue depth for a local member, the
   credit balance for a remote one — N-5(i)) and `local`, computes `key` if
   the protocol marks one, and hands the `ActorGroupView<E>` to `choose`. So the
   registry is *behind* the view by one push, never in the send path, and the
   monitor read is a brief lock — the same cost a stateful pick already pays.
   A later intrinsic (a lock-free snapshot held by the runtime) is an
   optimisation, not a design change.

**The `Registry` used directly.** It is worth a program's while when the
policy needs the *set* rather than a pick — a scatter is the cleanest case,
and it is also the kind-2 handler N-11f promised: per-protocol, `of any E`,
an actor because it must receive.

```
// Scatter a `Search.query` to every member, merge the hits, answer once.
handler Scattering(reg: Addr<Registry<Search>>) [spawn] of any Search {
    mailbox { capacity: 16 }
    members: Mut List<Addr<Search>> = []

    send fn query(q: Str, out: Reply<List<Hit>>) => !q, !out {
        reg.members(replyto fan_out(q, out))          // ask the replica: one local hop
    }

    send fn fan_out(q: Str, out: Reply<List<Hit>>, ms: List<Addr<Search>>) => !q, !out, !ms {
        let gather = spawn Gathering(ms.size(), out) on pool_of_self()   // collects, answers `out` once
        for m in ms {
            m.query(copy q, replyto gather.partial(...))   // the sugar pass's remote mint; today: mint in Gathering
        }
    }
}

fn main(args: List<Str>) [use, spawn] {
    use HostTcpTransport("0.0.0.0:7000")
    let cluster = join("search", seeds(args))
    let reg = attach<Search>("indexes", cluster)      // Addr<Registry<Search>> — a local replica
    join(reg, spawn Indexing(shard(args)) on pool(2)) // host one shard here …
    use Scattering(reg)                               // … and bind the fan-out as `any Search`
    let hits = waitfor out: Reply<List<Hit>> { query("salvo actors", out) }   // [any Search]
    serve(cluster)
}
```

`attach` and `join` here are std functions over the primitives: `attach`
spawns the local replica, publishes it, and peers it (`resolve` per cluster
member); `join(reg, addr)` is `reg.join(addr)`. A supervisor that wants to
know when the set changes `subscribe`s with its own `Changes<Search>` face,
and a health page calls `members`. None of these needs the stub or a `Pick`
— which is the guideline holding: the registry is the tool, `route` + `Pick`
is the convenience over it.

### N-11h. The basic example — join, list the members, notice a drop (round 3)

Two levels, because there are two kinds of member. **Nodes** are members of a
`Cluster`; **actors** of one protocol are members of a `Registry<E>`. Both
have the same three verbs — a `members` question, a `subscribe` for changes,
and a `Changes`-shaped protocol the notifications arrive on — so learning one
is learning the other.

**Level 1 — nodes.** A process joins, prints who is there, and prints a line
whenever a node arrives or drops. The printing is an ordinary actor of the
std `NodeChanges` protocol; it depends on `[Console]`, which it inherits from
`main`'s scope at the spawn [spawn-inherit].

```
import net.*

// std (sketch): what `join` answers, and what a subscriber receives
//   export struct Node { id: Str, host: Str, build: Build }
//   export actor effect Cluster {
//       send fn members(out: Reply<List<Node>>) => !out
//       send fn subscribe(w: Addr<NodeChanges>) => !w
//       send fn leave() 
//   }
//   export actor effect NodeChanges {
//       send fn joined(n: Node) => !n
//       send fn left(n: Node, why: Str) => !n, !why      // "left" or "unreachable" (N-5)
//   }

handler Announcing() [Console] of NodeChanges {
    mailbox { capacity: 32 }
    send fn joined(n: Node) => !n {
        println("+ ${n.id} at ${n.host} (${n.build.version})")
    }
    send fn left(n: Node, why: Str) => !n, !why {
        println("- ${n.id}: ${why}")
    }
}

fn main(args: List<Str>) [use, spawn] {
    use StdOutConsole()
    use HostTcpTransport("0.0.0.0:7000")                 // the wire: host code (N-4)
    let cluster = join("demo", seeds(args))               // Addr<Cluster>: a std actor on this node

    let now = waitfor out: Reply<List<Node>> { cluster.members(out) }
    println("${now.size()} members:")
    for n in now { println("  ${n.id} at ${n.host}") }

    cluster.subscribe(spawn Announcing() on pool(1))      // from here on, changes print themselves
    serve(cluster)                                        // park until this node is told to leave
}
```

Run three copies of it on three machines (or `MemCluster` under `salvo test`,
N-8) and each prints the other two, then `+ node-3 …` when the third starts,
then `- node-2: unreachable` when the second is killed. `serve` is a `waitfor`
on the cluster's own exit, which is how `main` stays alive without polling
[waitfor-pump].

**Level 2 — actors of one protocol.** Same program, one level down: this node
hosts a `Ping` actor, registers it under a name, lists every `Ping` in the
cluster, and prints when one drops — which happens when its actor faults *or*
its node goes (the registry turns both into `left`).

```
actor effect Ping {
    send fn ping(out: Reply<Str>) => !out
}

handler Pinging(me: Str) of Ping {
    mailbox { capacity: 8 }
    send fn ping(out: Reply<Str>) => !out { out.send("pong from ${me}") }
}

handler Noticing() [Console] of Changes<Ping> {
    mailbox { capacity: 32 }
    send fn joined(a: Addr<Ping>) => !a { println("+ a Ping at ${node_of(a).id}") }
    send fn left(a: Addr<Ping>)   => !a { println("- a Ping at ${node_of(a).id} is gone") }
}

fn main(args: List<Str>) [use, spawn] {
    use StdOutConsole()
    use HostTcpTransport("0.0.0.0:7000")
    let cluster = join("demo", seeds(args))

    let pings = attach<Ping>("pings", cluster)            // Addr<Registry<Ping>>: this node's replica
    join(pings, spawn Pinging(args[0]) on pool(1))        // host one, register it

    let all = waitfor out: Reply<List<Addr<Ping>>> { pings.members(out) }
    println("${all.size()} pings")
    for p in all {                                        // ask each — local or remote, same call
        let answer = waitfor r: Reply<Str> { p.ping(r) }
        println("  ${answer}")
    }

    pings.subscribe(spawn Noticing() on pool(1))
    serve(cluster)
}
```

`attach` gives this node a replica of the `"pings"` registry (spawned locally,
published in the node's name table, peered with the other nodes' replicas —
N-11g); `join(pings, addr)` is `pings.join(addr)`, and the replica `watch`es
the addr so a fault becomes a `left`. The `for` loop is the whole
transparency claim in four lines: `p.ping(r)` is the same call for the local
`Pinging` and the two remote ones, and the reply comes back across the wire
to `main`'s `waitfor`. `node_of(addr) -> Node` is a std accessor over the
routable identity (N-2), for exactly this kind of log line.

Nothing above uses `route`, `Pick` or `[any E]`: those enter only when a
program wants to *send to whichever `Ping` is free* instead of to a
particular one — the convenience layer over the two registries shown here.

### N-11i. Names, and why the two groups are actors (round 3, user questions)

**Renamed (user, 2026-09-26):** the two levels are **`NodeGroup`** (members
are `Node`s; was `Cluster`) and **`ActorGroup<E>`** (members are `Addr<E>`s
serving one effect, on any node; was `Registry<E>`), with `NodeChanges` and
`ActorChanges<E>` as their notification protocols. Earlier sections keep the
old words as the argument trail; read `Cluster` as `NodeGroup` and
`Registry<E>` as `ActorGroup<E>` throughout.

**Why a string name at `join` and `attach`?** Two different reasons, and
neither is load-bearing in the same way.

- *The node group's name* is a **guard**: the same program deployed twice on
  one network (staging and prod, or two tenants) must not gossip into one
  membership, and the protocol hash cannot tell them apart because it is the
  same build. The seeds alone are not enough either — a stale seed list
  points at the wrong deployment silently. Erlang solves this with the cookie
  plus the node name; here the name is compared at the transport handshake
  beside N-10's hash and N-6's secret, so joining the wrong group is refused
  by name rather than discovered by behaviour. It could be folded into the
  secret (one opaque value) or into the manifest (ROADMAP's pending
  decision), which would remove the string from the call. **Recommend keeping
  it as a separate, printable value**: it is what a log line and the
  `- node-2: unreachable` diagnostic name, and a secret should not be printed.
- *The actor group's name* is a **rendezvous** with a second job. A pure
  client that spawned nothing has no addr to start from, so the group must be
  findable by something agreed before any addr exists — and the only such
  things are the effect (its protocol hash) and a name. The effect alone
  would do if there were **one group per effect per node group**; the name
  exists so there can be several — two independent `Search` groups over two
  indexes, a `Resizer` group for thumbnails and one for originals. So the
  minimal spelling is `attach<Ping>(nodes)` — **the name defaults to the
  effect's own qualified name**, which is what the hash already identifies —
  and `attach<Ping>("pings-b", nodes)` is the overload for a second group of
  the same effect. That is the guideline applied: the type is the tool, the
  name the convenience for the rarer case. The examples above should read
  `attach<Ping>(nodes)`.

**Why is `ActorGroup<E>` an actor and not a plain effect?** Because it must
**receive**, and only an actor can. Three things arrive at a group replica
asynchronously, none of them from a caller on this thread: `merge` from peer
replicas on other nodes; the `Exit` of a member it `watch`es (a `watch` takes a
`Reply<Exit>`, and a continuation minted with `replyto` parks on a member of an
actor — a plain-effect handler has nowhere to park it [actor-replyto]); and
node-down from the `NodeGroup`, which it `subscribe`s to. A plain effect's
handler runs on its *caller's* thread and can be reached only by a call, so a
monitor cannot receive a message [monitor-handler] and a peer on another node
could not reach it at all — a remote send needs an `Addr<E>`, and only an
actor effect has one [actor-effect-kind]. The `NodeGroup` is an actor for the
same reason: transport events (a connection dropped, a hello from a new node)
arrive from the host's thread as *upcalls*, the way the timer's fire does
[time-timer], and an upcall is a send.

The cost is on the **read** side: `members` is a `waitfor` (a local hop, one
activation, cheap) where a plain `members() -> List<Addr<E>>` would read
nicer. The mixed-handler shape [mixed-handler] cannot fix that here — its plain
façade would make the whole handler serve a *plain* effect, and then the peers
lose their addr; and a handler cannot mix effect kinds across faces
[effect-handler-multi]. So the design's answer is the one N-11g already has:
the actor is the tool, and the **`View<E>` monitor** that `route` maintains
(a mirror written by an `ActorChanges<E>` subscriber, read under a brief lock
on the caller's thread) is the synchronous read for the hot path — exposable
as a plain `effect Members<E> { fn members() -> List<Addr<E>> }` for programs
that want the sync read without `route`. When the sugar pass lands the
answering stub, `members()` on an addr becomes a plain call anyway and the
question dissolves.

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
  `[protocol-version]` and `[build-info]` (N-10), `[actor-group]`,
  `[group-policy]`, `[group-key]` and `[any-effect]` (N-11), `[noremote]` (N-3 as decided);
  backend rules `[rs-remote]`/`[kt-remote]` for the codec and transport
  emission.

---

## Round 4 (user, 2026-09-26) — P-1 decided here, N-1 and N-9 re-examined

**Taken outright:** P-2 (effect-typed generic parameters on declarations),
N-2(a), N-5 including credits, N-6, N-11b (`any`, shipped with the library),
N-7, N-8, and the build sequence. Three items needed more: P-1 (the user asked
to decide it now — options below), N-1 (is remote spawn needed at all?), and
N-9/N-10 (does the per-protocol hash make one-program-per-group redundant?).
N-11c needs a rename and worked examples.

### P-1. The platform-handler thread-safety contract — the options

ROADMAP.md's section states the divergence: Kotlin binds the raw host
instance, Rust wraps it in the per-effect lock adapter because the generated
trait takes `&mut self`; a host that honours the assumption behaves the same
on both, a host that violates it races on Kotlin and is accidentally
serialized on Rust. The DECISION has three parts: **where** the contract is
declared, **what** it asserts, and **what the undeclared case means**.

- **Where.** (i) A marker on the `platform handler` declaration, in the
  position `linear` and `provenance` take: `threadsafe platform handler
  HostTcpTransport of Transport`. The host class is what is or is not
  thread-safe, so the claim belongs on the declaration that names it, and
  `salvo platform generate` prints the contract as a comment block above the
  class it writes, so the implementer signs it. (ii) On the effect — refused:
  two handlers of one effect can differ. (iii) In the manifest — refused: it
  is a fact about a class, not a project.
- **What.** (i) Whole-handler: every member may be entered concurrently from
  any thread; the host synchronizes internally. (ii) Per-member claims —
  refused for now: nothing in the language reads per-member concurrency, and
  the emission difference below is per handler.
- **Undeclared.** (i) Keep today's assumption — refused, it is the unvalidated
  thing this decision exists to remove. (ii) **Serialize on both backends**:
  Kotlin's binding goes behind `__Mon_E` (`synchronized`) to match Rust's
  lock; a non-conforming host then works identically everywhere at the cost of
  a lock on hosts that did their own finer-grained synchronization — the cost
  the 2026-09-20 decision refused, now confined to hosts that *did not say*
  they are safe. (iii) An error — refused: every platform handler would have
  to decide before the concept exists in the program.
- **The declared emission.** `threadsafe` takes ROADMAP's option (a): Rust emits
  the effect's members as **`&self`** for that handler and shares it as
  `Arc<H>`; Kotlin binds the raw instance as today. rustc then checks half the
  contract for free (a `RefCell` in the host no longer compiles; a `Mutex` or
  atomics do), and the Kotlin host stays on trust — a strictly smaller gap.
  Hosts that are plain forwarders to a thread-safe library (`HostRawFs` over
  the OS, the transport over sockets) declare `threadsafe`.

**Recommendation (the user's call): `threadsafe` on the declaration, whole-handler,
undeclared = serialized on both backends, declared = `&self`/raw.** The
transport is declared `threadsafe`; `HostRawFs` is reviewed and, if its open-file
table is guarded, declared `threadsafe` too — otherwise it takes the lock on both
backends and the divergence is closed by making Kotlin match Rust.

### N-1 re-examined — is remote spawn needed?

The user's question: treating a node as a `Remote Pool` implies spawning
actors on it from another node; is that required or beneficial for anything?

**What it was for**, and what replaces each without it:

| Use | With remote spawn | Without |
|---|---|---|
| Place work on a node with a resource (GPU, a local database) | `spawn Painting() on node(nodes, "gpu-2")` | That node spawns `Painting()` locally at start-up and joins `ActorGroup<Painter>`; or it hosts a `GpuHost` actor with `send fn start_painter(args, out: Reply<Addr<Painter>>)` — a program-written, typed spawner |
| A coordinator places map tasks | free `send fn` `on far` | sends to a worker group (`[any Mapper]`); the workers were spawned by their nodes |
| Singleton respawn on failover | the new leader spawns on some node | every node hosts a candidate from the start; `Elected<E>` routes to the current leader's own (N-11c) |
| Elastic scale-out | spawn more where there is room | the node with room spawns more of its own and joins — a `NodeChanges`/load-driven local decision |

**What remote spawn costs**, which the document had not priced: a `spawn`
answers an `Addr` synchronously, but the identity is allocated by the
*target's* scheduler — so a remote spawn is either a **round trip** (the
spawn becomes a `waitfor`, which `spawn … on far` does not look like) or a
**pre-allocated identity** the target must honour (a protocol of its own).
Constructor arguments must be serializable (a third check site). And the
target must have the handler compiled in, which is the one reason N-9's
one-program rule existed.

**And the user's own model never had it**: "to join a group a process must
spawn at least one of its own actors" — *its own*, locally. Remote spawn was
imported from N-1's first draft, not from the sketch.

**Recommendation (the user's call): withdraw remote spawn, and with it
`Remote Pool`, `node(...)`, `thread_on`, and N-1 as a question.** Every actor
is spawned by the node that hosts it, on that node's pools; the network comes
in at the **`ActorGroup<E>`**, and what crosses the wire is addrs and
messages, never construction. Consequences, all simplifications:

- The `noremote` check moves from "the spawn site" to **the crossing site it
  always was**: an `Addr<E>` is serializable iff every payload of `E` is, so
  `Addr<Painter>` (with a `noremote` `Canvas`) is itself `noremote`, and
  `attach<Painter>(nodes)` is refused — *"`Painter.paint` takes `Canvas`,
  declared `noremote`; a group of `Painter` cannot span nodes"* — at the
  binding, as the user asked, with nothing whole-program. Local groups of it
  (`group<Painter>()`, unnamed, this node only) stay legal.
- Tasks stay local; the ambient-pool rule [task-pool-inherit] is untouched;
  `Timer` on a remote pool is no longer a question; `watch` on a node is
  `NodeChanges.left`.
- N-5 loses "watch a `Remote Pool`" and keeps everything else; N-2's routable
  `Addr` is unchanged.
- §0.4's "somewhere on a cluster" is answered entirely by groups: a *call*
  runs somewhere because the group's `Pick` sent it there.

### N-9/N-10 — does the per-protocol hash retire one-program-per-group?

**Yes, as a requirement.** N-9's one-program rule bought two things: every
handler a remote spawn might name is in the binary — moot without remote
spawn — and the deadlock graph sees every handler in the group. The
per-protocol hash (N-10(b)) is the actual compatibility rule: two nodes may
talk on `E` iff their canonical `E` hashes agree, and that holds for two
*different* programs that import the same protocol module (`import
protocols.ledger`) as well as for one program deployed twice. So **N-9 is
withdrawn as a decision**; "one program" becomes the common deployment, and a
node group of several programs sharing protocol modules is legal from the
start. What N-9 leaves behind is one recorded gap: the deadlock graph
[actor-deadlock-cycle] is per program, so a wait cycle that closes through a
handler in *another* program is invisible. That is the same shape as the
already-recorded "graph over types, not instances" gap, with the same deferred
remedy (stratification tiers on addrs), and is noted rather than solved.

N-10 stands as decided in shape — (b)'s mechanism, (c)'s manifest label and
lock file when the manifest lands — with one clarification the withdrawal
makes possible: the hash is now checked at exactly **two** seams, the
`NodeGroup` handshake (the table of hashes, so `NodeChanges.joined` can carry
a node's `Build`) and **`attach`/`join`** on an `ActorGroup<E>` (this
protocol's hash, so an incompatible replica is invisible rather than merged).

### N-11c — the std picks, renamed and shown in use

`Any` collides with the `any` keyword (N-11b), so the std handlers are named
for what they do: **`LeastLoaded`** (was `Any`), **`Sharded`** (was `ByKey`),
**`Elected`** (was `Leader`). Each is a handler of `Pick<E>`; the user picks one
with an ordinary `use` before `use route(group)`. Recall the kit:

```
export struct ActorView<E>    { addr: Addr<E>, pending: Int, local: Bool }
export struct ActorGroupView<E> { actors: List<ActorView<E>>, key: Long? }   // key: hash of the `Key` argument, if the member has one
export effect Pick<E>      { fn choose(view: ActorGroupView<E>) -> Addr<E>? }  // None: nothing eligible — the send parks
```

**`LeastLoaded<E>(prefer_local: Bool = true)`** — the default. Local members
under `capacity` first, by `pending`; then remote by `pending`; `None` when
every member is full, which is the ordinary blocking send.

**`Sharded<E>`** — the member that owns the key. The protocol marks which
argument is the key with the std qualifier `Key`, a claim on the parameter type
read by the stub emitter and erased like any qualifier [qual-erasure]; the
stub hashes that argument (`Hashed`, [cmp-auto]) into `view.key`. Consistent
hashing over the sorted member set so a join moves ~1/n of the keys; state per
key lives on one member, which is what makes a `Mut Map` in the handler
correct.

```
actor effect Inventory {
    send fn reserve(sku: Key Str, qty: Int, out: Reply<Bool>) => !sku, !qty, !out
    send fn stock(sku: Key Str, out: Reply<Int>) => !sku, !out
}

handler Stocking() of Inventory {                 // ordinary: one shard's worth of state
    mailbox { capacity: 32 }
    levels: Mut Map<Str, Int> = {}
    send fn reserve(sku: Key Str, qty: Int, out: Reply<Bool>) => !sku, !qty, !out {
        let have = levels.get(sku) ?: 0
        if have >= qty { levels.put(sku, have - qty); out.send(true) } else { out.send(false) }
    }
    send fn stock(sku: Key Str, out: Reply<Int>) => !sku, !out { out.send(levels.get(sku) ?: 0) }
}

fn checkout(cart: List<Line>) [any Inventory] -> Bool {          // `any`: each send may go to a different shard
    for line in cart {
        let ok = waitfor r: Reply<Bool> { reserve(line.sku, line.qty, r) }
        if !ok { return false }
    }
    return true
}

fn main(args: List<Str>) [use, spawn] {
    use HostTcpTransport("0.0.0.0:7000")
    let nodes = join("shop", seeds(args))
    let stock = attach<Inventory>(nodes)
    join(stock, spawn Stocking() on pool(2))      // this node owns a slice of the keyspace
    use Sharded<Inventory>()                      // the policy…
    use route(stock)                              // …and the stub, binding `any Inventory`
    println("${checkout(cart(args))}")
    serve(nodes)
}
```

`checkout` is honest about ordering: two `reserve`s for different SKUs land
on different shards and are unordered, which `[any Inventory]` states. Two for
the *same* SKU land on the same member in order — `Sharded` gives per-key
ordering, a stronger guarantee than `any` claims, and a program that relies on
it is relying on the policy rather than the type; if that matters, a
`[keyed E]` claim is a later refinement. `use Sharded<Inventory>()` is refused
if `Inventory` has a member with no `Key` argument or with two.

**`Elected<E>`** — the one member the current leader hosts. `Elected` is a
handler `[Leader]`: it depends on the std `Leader` effect and captures it as an
owned handle at construction [use-local], so a Salvo election and a platform
handler over etcd or a Kubernetes lease are interchangeable behind it.

```
// std (sketch)
export effect Leader {
    fn leader() -> Node?                          // None: an election is in progress
}
export actor effect LeaderChanges { send fn elected(n: Node) => !n; send fn lost() }

export handler Elected<E>() [Leader] of Pick<E> {
    fn choose(view: ActorGroupView<E>) -> Addr<E>? {
        let l = leader() ?: return None                                // no leader yet: park
        return first(view.actors, m => node_of(m.addr) == l)?.addr    // the leader's member, or park
    }
}
```

Used for a singleton — one `Sequencing` hands out ids, every node hosts a
candidate, sends go to the leader's:

```
actor effect Sequencer { send fn next(out: Reply<Long>) => !out }

handler Sequencing() of Sequencer {
    mailbox { capacity: 64 }
    n: Long = 0
    send fn next(out: Reply<Long>) => !out { n = n + 1; out.send(n) }
}

fn fresh_id() [any Sequencer] -> Long {
    return waitfor r: Reply<Long> { next(r) }
}

fn main(args: List<Str>) [use, spawn] {
    use HostTcpTransport("0.0.0.0:7000")
    let nodes = join("ids", seeds(args))
    use LeaseLeader(nodes, millis(1500))          // std, in Salvo: oldest-live-node with a timer lease —
                                                  // or `use HostEtcdLeader("…")` and nothing else changes
    let seq = attach<Sequencer>(nodes)
    join(seq, spawn Sequencing() on pool(1))      // every node hosts a candidate
    use Elected<Sequencer>()
    use route(seq)
    println("${fresh_id()}")
    serve(nodes)
}
```

Two things to read off it. **Failover state is the program's**: the new
leader's `Sequencing` starts at `0` unless the program replicated `n` — which
is Raft's log and the flagship example's job — so `Elected` gives *one active
member*, not *one continuous state*. And **split brain is N-5's policy at
`join`**: under a partition each side's `Leader` may answer a different node
until the node group's policy (majority, oldest) declares one side dead. The
`Leader` effect's `None` is what makes the gap visible — `choose` parks
rather than guessing.

---

## Where this stands after round 3 — the outstanding decisions

> **Superseded by round 4** (above): P-1 has options and a recommendation;
> P-2, N-2, N-5, N-6, N-11b, N-7, N-8 and the sequence are **taken**; **N-1 and
> N-9 are recommended withdrawn** (no remote spawn — the network enters at
> `ActorGroup<E>`; per-protocol hash is the compatibility rule); N-11c is
> renamed (`LeastLoaded`/`Sharded`/`Elected`) and shown in use. The table below
> is kept as the round-3 trail. **Still the user's call after round 4:** P-1's
> recommendation, the N-1 withdrawal, the N-9 withdrawal, N-11c's handlers as
> shown, and N-10's manifest half (waits on the manifest DECISION). The build
> sequence loses step ③'s `Remote Pool`/`node`/remote spawn and keeps its
> routable addrs and credits; the crossing-site `noremote` check lands with
> step ⑤'s `attach`.

**Taken (user, 2026-09-26, rounds 1–3).** Salvo owns serialization, default-on
with `noremote` as the opt-out, checked at the spawn site (N-3). The platform
owns the transport only (N-4). Addresses and groups are separate things, and
the group is **not a type**: a library kit — `NodeGroup`, `ActorGroup<E>`,
`pending(addr)`, a `Pick<E>` effect with std handlers, a generated `route`
stub — plus the `[any E]` / `of any E` claim (N-11, rounds 2–3). Both groups
stay **actors until the sugar pass**, which now has a concrete target: make
`waitfor out { g.members(out) }` read as `members()` (the answering stub) and
`replyto` onto another actor's member writable (the remote mint). The names
stay: the node group's as a printed guard, the actor group's defaulting to the
effect with an overload for several groups of one effect. The layering is
**transport → `NodeGroup` → `ActorGroup<E>` → `route` + `Pick<E>`**, built in
that order, library first.

**Still the user's call**, in dependency order, each with the recommendation
the sections above argue for:

| # | Decision | Recommendation | Depends on |
|---|---|---|---|
| **P-1** | **Platform-handler thread-safety contract** (ROADMAP's open DECISION) — now a prerequisite: the transport is called from every pool | Decide the surface; `salvo platform generate` prints it into the skeleton | — |
| **P-2** | **Effect-typed generic parameters on declarations.** `ActorGroup<E>`, `ActorChanges<E>`, `Pick<E>`, `ActorGroupView<E>`, `ActorView<E>` are all generic over an *effect*, used only inside `Addr<E>`. Today only a *function* may do this (`watch<E>`) [effect-not-data]. A real language rule | Allow `<E>` on effects, handlers and structs when every use is inside `Addr<E>`; refuse any other position with the existing diagnostic. Instances are per effect, as generic effects already are | — |
| **N-1** | A node is a pool: `Remote Pool` provenance qualifier, `node(nodes, name)` in std. **`anywhere` is withdrawn** — groups route, placement is always to a named node's pool or a local one | Take (a) minus `anywhere`; `thread_on(node)` recorded for a remote `Dedicated` | P-1 |
| **N-2** | One `Addr<E>` type, routable `(node, id, bits)`, locality a runtime fact; plain effects stay local until the answering stub | Take (a); the spawn-site check from N-3 removes the whole-program worry that made (c) tempting | N-3 ✅ |
| **N-5** | Unreachable is dead; `watch` on a `Remote Pool`; partition policy chosen at `join`; **credit-based back-pressure** — now load-bearing, since `pending(addr)` for a remote member *is* the credit balance | Take (a) + (i); at-most-once, in order per pair | N-2 |
| **N-6** | Node-group name + shared secret/TLS at handshake; unguessable bits in every crossing addr so the two-face guarantee survives | Take (a)+(b); TLS in the default transport | N-2 |
| **N-9** | One compiled program per node group; protocol hash at handshake | Take (a); design the hash so a per-module relaxation follows the manifest | — |
| **N-10** | Per-protocol hash equality checked at handshake, remote spawn and `join`/`attach`; never at decode. Manifest `version` + lock file as the user's label; `build()`, `protocol<E>()`, `Build` on every `Node` | Take (b) mechanism + (c) surface | the **manifest** DECISION (ROADMAP) |
| **N-11b** | The spelling of the weaker claim | `any` (over `volatile`, `some`); ship `[any E]` + `of any E` **with** the library, since a hand-written router otherwise hides the lost ordering (user: the virality is the point) | — |
| **N-11c** | `Key` qualifier on a protocol parameter for `ByKey`; `Pick<E>::choose` answering `Addr<E>?` with `None` = park | Take as sketched; `Any`, `ByKey`, `Leader` as std handlers, `Leader` a std effect | P-2 |
| **N-7** | What is std vs example, re-cut by the kit | std: the kit above + `Leader` effect + `serve`, `node_of`; examples: election (Raft flagship), singleton, map/reduce, scatter, hedge | N-11 |
| **N-8** | The double | `MemTransport` (a `Transport` handler with `partition`/`heal`/`kill`/`delay`), so `NodeGroup` and `ActorGroup` run unchanged over it under `salvo test`; one localhost smoke test per backend | N-4 ✅ |

**Build sequence, once decided** (the library-first order the user endorsed):
① P-1 contract and the `Transport` platform effect with `MemTransport` and
`HostTcpTransport`; ② codecs + protocol hash on both backends (N-3, N-9,
N-10's mechanism); ③ `Remote Pool`, routable `Addr`/`Reply`, `node`, remote
spawn with the spawn-site check (N-1, N-2, N-5's credits, N-6's bits);
④ `NodeGroup` with `watch` and the partition policy; ⑤ P-2 and `ActorGroup<E>`
with `ActorChanges<E>`, `attach`/`join`, `pending`; ⑥ `[any E]` / `of any E`
and the hand-written router example; ⑦ `Pick<E>`, the three std handlers, the
generated `route` stub with its `View<E>` mirror; ⑧ `examples/cluster/`
(Raft, scatter, map/reduce) and the manifest half of N-10 when the manifest
lands. Each step runs on `MemTransport` under `salvo test` before it touches a
socket.

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
| **N-3** | What may cross a machine, and who defines the bytes? | §0.1, §0.5 | ✅ **Decided (a), amended**: compiler-defined encoding, codecs for both backends; serializable **by default**, `noremote` the opt-out; the check at the **spawn site** |
| **N-2** | What is an `Addr<E>` / `Reply<T>` across the wire; do plain effects bind remotely? | §0.2, §0.4 | **One `Addr` type, routable, locality a runtime fact**; `Reply` a routable one-shot; plain effects stay local until the sugar pass's answering stub (mixed handlers bridge meanwhile) |
| **N-4** | Who owns discovery, membership, transport? | §0.5 | ✅ **Decided (a)**: platform owns the **transport only**; membership and routing are Salvo actors in std; the platform thread-safety contract becomes a prerequisite |
| **N-5** | What does a program see when a node is gone? | §0.7 | **Unreachable is dead**: `watch` on a `Remote Pool`, membership policy chosen at `join`, credit-based back-pressure so `capacity` stays true, at-most-once in-order per pair |
| **N-6** | How is authority re-established on the wire? | §0.7 | **Shared secret/TLS for membership + capability bits in every crossing addr**, so the two-face guarantee survives the network |
| **N-9** | One program or many? | §0.7 | **One compiled program per cluster**, roles by argument, protocol hash designed so a per-module relaxation (with the manifest) comes later |
| **N-10** | Mixed builds during a deploy: how is "same version" known, is it the user's or hidden, how is it read? | §0.9 | **Per-protocol hash equality as the hidden mechanism**, checked at handshake, remote spawn and registry lookup — never at decode, since exhaustive `when` forbids lenient decoding — so a mixed fleet stays one cluster; **the manifest's `version` plus a lock-file check** as the user's label (a protocol change without a bump fails the build); read through `build()`, `protocol<E>()`, and `Build` on every `Node` |
| **N-7** | Which patterns are language, std, or examples? | §0.6 | **Membership and `Registry<E>` in std; election, singleton, map/reduce, sharding as `examples/cluster/`** with Raft as the flagship; `Leader` declared in std so a platform handler and a Salvo election are interchangeable |
| **N-8** | How is a cluster tested? | §0.7 | **`MemCluster`** — N virtual nodes in one process with `partition`/`heal`/`kill`, run under `salvo test`, codecs exercised; one localhost smoke test per backend |
| **N-11** | Actor groups: one address, many interchangeable members, routed by policy — local and remote | user sketch, round 1 | ✅ **`AddrGroup<E>` decided (round 2)** as a second intrinsic type beside `Addr<E>` (ordering is the reason for the type); no syntax, routing stub generated beside the forwarding one, `join` = spawn a member + register (and the N-10 check site); subsumes N-1's `anywhere` for sends and N-7's `Registry<E>`. **Open (N-11b–d):** the effect-list claim — **`[any E]`** recommended, the strong bare `[E]` staying the default so no existing program breaks; `Policy` as a std value (`Any {}` first, `ByKey {}` via a `Key` qualifier on the protocol, `Leader {}` via the `Leader` effect); `attach<E>(name, cluster)` for a pure client, the "at least one member" invariant moved onto the group. **N-11e (round 3):** first-class vs library — recommend **library first** (`Registry<E>`, a `pending(addr)` intrinsic, a hand-written router per effect as the example), the generated stub as the planned second step, `[any E]` deferred until an ordering bug is observed; what only first-class support buys is the hop-free pick and the ordering guard. **N-11f (round 3, user guideline "minimal tools, then convenience"):** the kit — `Registry<E>` (membership half; typed name → members; the N-10 seam), `pending(addr)`, a `Pick<E>` **effect** whose handlers are the policies (generic picks over a membership view; `Any`/`ByKey`/`Leader` as std handlers, user variations as more), a generated `route` stub declaring `of any E`, and **`[any E]` + `of any E` shipped with the library** since a hand-written router otherwise *hides* the lost ordering; message-reading policies (scatter, hedge, retry) are per-protocol handlers `of any E`. `AddrGroup<E>` as a type is no longer needed |

**Delete-when-decided.** When the user has made these calls, fold the outcomes
into COMPLETED.md's decision log, turn the questions into plans (and any
undecided ones into **DECISION** rows) in ROADMAP.md, add the rules under the
labels above to LANGUAGE_SPEC.md and the backend specs, and delete this file.

---

## Round 5 (user, 2026-09-26) — the decided list

All calls taken. The naming rule the user asked to hold throughout: **node
groups and actor groups are kept visibly distinct** — `NodeGroup` / `Node` /
`NodeChanges` on one side, `ActorGroup<E>` / `ActorGroupView<E>` /
`ActorChanges<E>` / `ActorView<E>` on the other; nothing is called plain "group",
"cluster" or "registry" in the final surface. **`Addr<E>` keeps its name**
(re-asked and re-declined here, 2026-09-26): the 2026-09-15 reasons stand —
*address* is the actor model's own word and is now literally true of a
routable `(node, id, bits)` — and a new one appeared: `ActorView<E> { addr:
Addr<E>, pending, local }` is a view of the *actor* behind a handle, and
naming the handle `Actor<E>` would blur the handle/actor distinction that
makes stale addrs safe to hold. `ActorRef<E>` remains the alternative if the
family is ever unified.

**Decided (user decisions, 2026-09-26):**

1. **Serialization is Salvo's** (N-3): a compiler-defined canonical encoding,
   codecs generated for both backends so a Kotlin node and a Rust node share a
   node group; serializable **by default**, **`noremote`** the opt-out on a
   type, transitive through fields; process-local handles are `noremote` by
   construction. Checked at the **crossing site**: an `Addr<E>` is
   serializable iff every payload of `E` is, so `attach<E>(nodes)` is refused
   for an `E` with a `noremote` payload, naming the member and the type.
2. **The platform owns the transport only** (N-4): a `Transport` effect with a
   `HostTcpTransport` platform handler in std's `platform/` tree and a
   `MemTransport` double; everything above the wire is Salvo in std.
3. **P-1 — `threadsafe platform handler H of E`**: a marker on the handler
   declaration, whole-handler claim; **undeclared = serialized on both
   backends** (Kotlin gains the `synchronized` wrapper to match Rust's lock);
   declared = Rust `&self` members over `Arc<H>`, Kotlin raw. `salvo platform
   generate` prints the contract into the skeleton. The transport is
   `threadsafe`; `HostRawFs` is reviewed for it.
4. **P-2 — effect-typed generic parameters on declarations**: effects,
   handlers and structs may take `<E>` standing for an effect when every use
   is inside `Addr<E>`; any other position keeps [effect-not-data]'s error.
5. **No remote spawn** (N-1 withdrawn): every actor is spawned by the node
   that hosts it, on its own pools. `Remote Pool`, `node(...)`, `anywhere`,
   `thread_on` do not exist. The network enters at `ActorGroup<E>`; what
   crosses the wire is addrs and messages, never construction.
6. **One `Addr<E>`** (N-2(a)): routable `(node, id, bits)`, locality a runtime
   fact; `Reply<T>` a routable one-shot with its reservation unchanged. Plain
   effects bind locally until the sugar pass's answering stub; mixed handlers
   bridge meanwhile.
7. **Failure** (N-5): unreachable is dead; node departure arrives as
   `NodeChanges.left(n, why)`; a partition policy is chosen at `join`;
   **credit-based back-pressure** across the wire so `capacity` stays true and
   `pending(addr)` on a remote member is the credit balance; at-most-once,
   in order per `(sender, target)` pair.
8. **Authority** (N-6): node-group name plus shared secret / TLS at the
   handshake; unguessable bits in every addr that crosses the wire, so the
   two-face guarantee survives the network.
9. **Compatibility is per protocol** (N-9 withdrawn, N-10 taken): a
   canonical hash per actor effect, exchanged in the `NodeGroup` handshake and
   compared at `attach`/`join`; never at decode (exhaustive `when` forbids
   lenient decoding). One program per node group is the common deployment,
   not a rule; several programs sharing protocol modules are legal. The
   manifest's `version` plus a lock file (a protocol change without a bump
   fails the build) is the user-facing label, landing with the manifest
   DECISION; read through `build()`, `protocol<E>()`, and `Build` on every
   `Node`. Recorded gap: the deadlock graph is per program.
10. **Two membership levels, both actors until the sugar pass** (N-11i):
    `NodeGroup` (members `Node`; `join(name, seeds)`; name a printed guard)
    and `ActorGroup<E>` (members `Addr<E>` on any node; one gossiping
    replica per node; `attach<E>(nodes)`, name defaulting to the effect,
    `attach<E>(name, nodes)` for several groups of one effect; `join(group,
    addr)`; `members`, `subscribe`; `ActorChanges<E>`). The sugar pass's
    targets: `members()` as a plain read (the answering stub), `replyto` onto
    another actor's member (the remote mint).
11. **The kit, library first** (N-11e/f): `pending(addr)`; `Pick<E>` as an
    **effect** — `fn choose(view: ActorGroupView<E>) -> Addr<E>?`, `None`
    parks — with std handlers **`LeastLoaded`**, **`Sharded`** (key marked by
    the `Key` qualifier on a protocol parameter) and **`Elected`**
    (`[Leader]`, a std effect served by a Salvo election or a platform
    handler); a generated **`route(group)` stub** declaring `of any E`, fed by a
    `View<E>` monitor mirrored from `ActorChanges<E>`; message-reading policies
    (scatter, hedge, retry) are per-protocol handlers `of any E`. No
    `AddrGroup` type.
12. **`[any E]` and `of any E`** (N-11b): the weaker claim — no ordering, no
    shared state between sends — on both the requirement and the binding,
    viral downward like `local`, bare `[E]` keeping the strong meaning so no
    existing program changes. Shipped **with** the library.
13. **std vs examples** (N-7): std ships the kit, `Leader`, `serve(nodes)`,
    `node_of(addr)`; `examples/cluster/` ships election (Raft flagship),
    singleton, map/reduce, scatter, hedge.
14. **Testing** (N-8): `MemTransport` with `partition`/`heal`/`kill`/`delay`;
    both groups run unchanged over it under `salvo test`; one localhost smoke
    test per backend.
15. **Build sequence**: ① P-1 + `Transport` (`MemTransport`, `HostTcpTransport`)
    → ② codecs + protocol hash → ③ routable `Addr`/`Reply`, credits, addr
    bits → ④ `NodeGroup` → ⑤ P-2, `ActorGroup<E>`, `attach`/`join`,
    `pending`, the crossing-site `noremote` check → ⑥ `[any E]`/`of any E` and
    the hand-written router example → ⑦ `Pick<E>`, the three std handlers,
    `route` → ⑧ examples and N-10's manifest half. Every step on
    `MemTransport` before a socket.

16. **Membership mechanisms are handlers of `NodeGroup`** (asked 2026-09-26,
    after the list; settled in three passes the same day). The examples wrote
    `join("shop", seeds(args))` as if a seed list plus gossip were the only way
    a node finds its group — **read every such line above as `spawn
    GossipNodeGroup("shop", seeds, …) on pool(1)`; the examples are kept as
    the trail.** The first fix was a `NodeDiscovery` effect
    answering seeds; the second gave it `announce`/`nodes`/`authoritative()` so
    a heartbeat store (every node writes itself into DynamoDB with a TTL and
    reads the full list back) could be expressed. That `authoritative()` flag
    was the tell: a plain handler steering a std actor's algorithm through a
    boolean means the algorithm *is* the mechanism. **Gossip is one membership
    mechanism among several**, so:

    - **There is one actor effect, `NodeGroup`**, with the surface the examples
      already used — `members`, `subscribe`, `leave`, and `NodeChanges` with
      `joined(n)` / `left(n, why)` — and **its handlers are the mechanisms**.
      `NodeDiscovery` is deleted; `join(...)` is an ordinary spawn.
    - **Death detection belongs to the handler**, because it is
      mechanism-specific: connection drop plus a partition policy under
      gossip, TTL lapse under a heartbeat table, a ping under a static list.
      `left(n, why)` is the common output.
    - **The handshake stays common** (name, secret, protocol hashes — N-6,
      N-10): it happens in the transport when two nodes first speak, whichever
      mechanism introduced them.

    ```
    // std (sketch)
    export struct NodeEndpoint { … }                 // where the transport can dial: host+port for TCP,
                                                     // a virtual id for MemTransport — known BEFORE contact
    export struct Node { id: Str, endpoint: NodeEndpoint, build: Build }   // known AFTER the handshake

    export actor effect NodeGroup {
        send fn members(out: Reply<List<Node>>) => !out
        send fn subscribe(w: Addr<NodeChanges>) => !w
        send fn leave()
    }
    export actor effect NodeChanges {
        send fn joined(n: Node) => !n
        send fn left(n: Node, why: Str) => !n, !why
    }

    // the mechanisms — each a handler of NodeGroup, each spawned on the node it serves
    export handler GossipNodeGroup(name: Str, seeds: List<NodeEndpoint>, split: SplitPolicy)
        [Transport] of NodeGroup { … }                          // N-4's design; N-5's partition policy is ITS argument
    export handler StaticNodeGroup(name: Str, all: List<NodeEndpoint>) [Transport, Timer] of NodeGroup { … }   // fixed fleet, ping for death
    export handler MemNodeGroup(name: Str) [Transport] of NodeGroup { … }                      // the MemTransport double
    // a shop's own, ~40 lines:
    //   handler HeartbeatNodeGroup(name: Str, table: Str, ttl: Duration) [Ddb, Timer, Ticker] of NodeGroup
    //     — writes (me, now) at start and every ttl/3; scans rows younger than ttl on a Timer;
    //       diffs into joined/left; no gossip, and no partition policy: a node that cannot reach
    //       the table is itself the one that has left.

    fn main(args: List<Str>) [use, spawn] {
        use HostTcpTransport("0.0.0.0:7000")
        let nodes = spawn HeartbeatNodeGroup("shop", "shop-nodes", seconds(30)) on pool(1)   // Addr<NodeGroup>
        let stock = attach<Inventory>(nodes)                                                    // unchanged from here down
        …
    }
    ```

    **`NodeEndpoint` vs `Node`.** An endpoint is what a mechanism can know
    before any contact (DNS knows addresses, not builds); a `Node` is minted
    by the handshake and carries identity and `Build`. Every mechanism deals
    in endpoints on the way in and answers `Node`s.

    **What it costs.** A `NodeGroup` handler is a real distributed program —
    gossip especially — so writing one is a higher bar than writing a
    `seeds()` function was. That is honest: the heartbeat handler *is* a timer
    loop with liveness semantics, and a seed-list disguise would have hidden
    them. A shop that only wants a different seed *source* under gossip
    passes the list to `GossipNodeGroup` from wherever it likes (a `[Resolver]`
    lookup, a file read with `Fs`), which needs no new handler. The
    heartbeat model is also a genuinely different failure story from
    gossip's — two nodes that can both reach the store agree, and there is no
    split brain to adjudicate — which is exactly why it is a handler and not
    a flag.

    **Layering, final**: **transport → `NodeGroup` (one handler per
    mechanism) → `ActorGroup<E>` → `route` + `Pick<E>`**. For the sequence:
    step ④ builds `MemNodeGroup` and `StaticNodeGroup` (poll and diff, the
    small path) and then `GossipNodeGroup`; `HeartbeatNodeGroup` over a
    `Ddb` platform effect is the interop example in `examples/cluster/`. The
    round-5 naming rule holds: every mechanism is `<Mechanism>NodeGroup`, the
    address type is `NodeEndpoint`, and nothing on this level is called
    "discovery", "seeds" or "cluster".

**Propagation owed** (this was a read-only session): a decision-log entry in
COMPLETED.md for the round; a ROADMAP.md section holding the sequence above
as the plan, the platform thread-safety DECISION closed by item 3, the manifest
DECISION noted as gating item 9's label; LANGUAGE_SPEC.md rules under
`[noremote]`, `[wire-format]`, `[protocol-hash]`, `[threadsafe-platform]`,
`[effect-generic-decl]`, `[addr-routable]`, `[remote-backpressure]`,
`[addr-capability]`, `[node-group]`, `[actor-group]`, `[any-effect]`,
`[group-pick]`, `[group-key]`, `[mem-transport]` (final names at
propagation), with `[rs-remote]`/`[kt-remote]` in the backend specs; and
`docs/language/Concurrency.md` gaining a "Across machines" page. Then delete
this file.
