# Concurrency — the phase-5 option space (working document)

Status: **DIRECTION SET** (user, 2026-09-15) for the model and the surface —
the effects-kernel direction, recorded in "The direction" below; C-4 and the
reworked opens in "Decisions pending" remain. First written 2026-09-14 as a
fully-open read-ahead pass for phase 5 ("Threading and concurrency — the OTP
model", ROADMAP.md); the decision sections are kept as written for the
argument trail, with supersession pointers where the direction settled them.
The remaining recommendations are recommendations; the calls are the user's
(AGENTS.md's first invariant).

Written 2026-09-14 by a read-only session, while another session works on
FILE_SYSTEM.md (phase 4). This document is modelled on FILE_SYSTEM.md's shape
— stated intent, inherited fixed points, what other languages teach, then
numbered decision sections with options, trade-offs, and a recommendation —
and DESIGN_DOC.md distils that shape from these two instances. It lays out the
design in the style OBLIGATIONS.md and FILE_SYSTEM.md used: options,
trade-offs, and recommendations, with the decisions left to the user.

**Propagation status (updated 2026-09-15, write-enabled).** The decided
outcomes are **in COMPLETED.md's decision log** ("Phase 5 designed:
asynchronous effect handlers") and **ROADMAP.md is updated** (the sequence's
phase 5, the "Threading and concurrency" section rewritten from four open
DECISIONs into the plan, the decisions-waiting table, L8's intrinsic-container
bullet). Still owed, landing **with implementation**: fresh rule labels into
LANGUAGE.md/LANGUAGE_SPEC.md and the backend specs, the examples-files
respelling sweep (Examples 1–5 of both files into the frozen grammar), and
this document's deletion once the code and specs carry its content. The
supervision design (SUPERVISION.md) is the remaining prerequisite. When the
remaining calls are made, the
outcomes must be propagated by a later session: fold the decided list into
COMPLETED.md's decision log, turn the four ROADMAP **DECISION**s (see §1) from
open questions into a plan (they stop being **DECISION**s the moment they are
decided — AGENTS.md), add the new rules to LANGUAGE_SPEC.md and the backend
specs with fresh labels, and delete this document once its outcomes live in
the log — the same charter OBLIGATIONS.md had and FILE_SYSTEM.md carries. Until
then nothing here has propagated anywhere.

Sources: ROADMAP.md ("Threading and concurrency — the OTP model (phase 5)" with
its four **DECISION**s, "Regions — designed", the phase-5 row of "Decisions
waiting on the user", the `Cell` and "Laziness, after concurrency" deferrals),
COMPLETED.md (the decision log entries on `Rc<dyn Fn…>`, linearity, and
async-as-an-explicit-effect), LANGUAGE.md ("Effects", "Non-resumption",
"Deductions", "Backends"), LANGUAGE_SPEC.md and the backend specs
([rs-fn-field], [linear-obligation], [fate-lambda], [iter-protocol],
[fn-effects], [platform-effect], [intrinsic-std-only]), FILE_SYSTEM.md (the
document shape), and the user's stated intent (below), plus a cross-language
survey (§2) verified against primary documentation.

## 0. The stated intent

The user's sketch (2026-09-14), restated so the decisions below can be judged
against it. **It is tentative** — the user asked for it to *guide* the options,
including their cost and shape-changing alternatives that might turn out
better, not to fix the answers in advance.

1. **No shared-mutable-state primitives.** No `Mutex`, no `Lock`. Everything is
   managed with the Erlang/Gleam/OTP model: processes that own their state and
   communicate by message passing.
2. **A process is spawned by an effect.** Spawning is a capability, reached
   through a specific effect rather than a free function.
3. **Mailboxes are declared at spawn, by the type/qualifier sent, each
   bounded.** A spawn states how many mailboxes the process has — one per
   message type or qualifier it accepts — and the size bound on each. In the
   general case a mailbox is backed by a blocking queue.
4. **Send takes anything in the union of the mailbox types.** `send` accepts a
   value of the union of the process's mailbox types, and can `when`-express
   over that union to route the value to the mailbox for its arm.
5. **Send blocks the sender when the queue is full.** Back-pressure is the
   default: a full bounded mailbox stalls the sender rather than growing without
   bound.
6. **Processes run on selectable thread pools.** A process — perhaps better
   called a *thread* — runs on a thread pool chosen at spawn, so the user can
   route CPU-bound, network-bound, and disk-IO work to different pools.
7. **Some message types are linear, guaranteeing a reply.** A linear message
   carries an obligation that a response comes back to the sender.
8. **Consider a "future send".** A send might yield a future that the receiver
   completes and the sender waits on — though the general case is a round trip.

Points 1, 2, and 7 fit the repo's recorded direction exactly (§1). Points 3–6
and 8 are the shape decisions, and each meets an existing rule or an open
**DECISION**; those meetings are C-1 … C-8 below.

## 1. Fixed points — already decided, inherited here

These are settled by earlier decisions or by what the implementation already
does, and phase 5 builds on top of them rather than reopening them. Where a
point in §0 collides with one, it is flagged.

- **Message ownership transfer is already ordinary consumption.** A `send` that
  moves its argument is a plain deduction list, and the existing flow analysis
  rejects use-after-send with a diagnostic that names the call. Sending a value
  *into* a process needs no new ownership machinery — it is a move like any
  other. (ROADMAP.md phase 5, "What is already in place".)
- **Linearity composes with sending for free** [linear-obligation]. "Whoever
  ends up with this handle must close it" survives a send, because a move
  transfers the obligation. So sketch point 7 (linear messages) starts from an
  obligation model that already works across a move — the open part is the
  *reply* guarantee, not the linearity (C-6).
- **Supervision has somewhere to live without new machinery: a supervisor is a
  handler.** Effects already give an interception point (a later `use` shadows
  an earlier one, and a handler may depend on the effect it implements), so a
  supervisor that restarts a child is a handler, not a new construct. (ROADMAP.md
  phase 5.)
- **Sendability is an open DECISION, and the blocker is representational, not
  notational** [rs-fn-field]. Generated Rust holds a fn-typed field as
  `Rc<dyn Fn…>` — which is *every* composed pass — and `Rc` is not `Send`. A
  value that crosses a process boundary cannot hold one as things stand. This is
  ROADMAP **DECISION 1** and it is C-4. `Sendable` is on the intrinsic-capability
  watch list (structurally inferred, asymmetric between backends, never
  user-authored), so whatever is decided is the compiler's to infer, not the
  user's to write.
- **What effects a spawned process has is an open DECISION** (ROADMAP
  **DECISION 2**). An effect reaches a function as `&mut dyn E` borrowed for the
  call, and the Rust fusion is one value per `use` scope holding borrows of the
  handlers registered in it — neither crosses a thread boundary. So a process
  cannot inherit its parent's handler set by reference; the likely answer is that
  a process opens its own `use` scope. This is C-1, and it decides whether
  `spawn` takes a handler set.
- **The execution substrate is an open DECISION** (ROADMAP **DECISION 3**). The
  JVM has threads and coroutines; Rust has threads and no runtime in std. One OS
  thread per process with a blocking `receive` is the cheapest thing that is
  *the same* on both backends, and the parity principle argues for it until
  something forces otherwise. Sketch point 6 (selectable thread pools) meets this
  directly — it is C-5.
- **Async's fate is an open DECISION** (ROADMAP **DECISION 4**; recorded
  2026-09-04). Async arrives later as an *explicit* effect, never as silent
  `async`/`suspend` colouring — and if a process blocks on `receive`, the OTP
  model may remove the need for it entirely. This is C-7, and it overlaps the
  future-send in sketch point 8 (C-6).
- **A prerequisite the checker does not yet enforce** [fate-lambda]. Returning
  or storing a capture-carrying closure is a rustc lifetime error the checker
  does not reject — and `spawn(() -> …)` is exactly that shape. The recorded
  refinement is `move`-closure emission with hoisted clones plus a treatment for
  captured effect-handler locals. It is phase-5 work, and it gates any `spawn`
  that takes a closure.
- **Regions ride along, with "a process is a region" as the null hypothesis**
  (user, 2026-09-10). The region design is decided and is built with this phase;
  how a region and a process line up is C-8.
- **`Cell` and laziness wait for after phase 5.** `Cell` exists to make shared
  mutable state expressible, and the OTP answer (point 1) may remove its
  motivation entirely — deciding it earlier spends a language-design call twice.
  Laziness was removed 2026-09-10 and reopens after this phase; its recorded
  direction ("compose functions into pipeline functions, not data") is itself
  gated on the sendability of fn-typed fields (C-4). Both stay out of scope here,
  named only where they bear on a decision.
- **The interop boundary is unchanged** [platform-effect] / [intrinsic-std-only].
  Target-language concurrency machinery — a JVM `ExecutorService`, a Rust thread
  pool — is reached the way every capability is: an `intrinsic` for std, a
  `platform effect` member for customer code. Nothing about phase 5 adds a new
  interop path.

## 2. What other languages teach

Salvo's target is the OTP model — processes that own their state and
message-pass — but the design space around that target is wide, and several
languages have already made the exact calls C-1 … C-8 ask for. The survey is
grouped by family; each entry names the pattern, any variation worth stealing,
and the sketch point or decision it informs. The two backends have their own
subsection, because whatever is chosen must encode into *both* Kotlin and Rust.

### Erlang / OTP — the origin

A process has one mailbox and **selective receive**: `receive` pattern-matches
the mailbox and leaves unmatched messages in place for a later `receive`. That
is powerful and it is a footgun — an unmatched message lingers and can pile up,
a memory leak with no type error. `gen_server` is the disciplined form:
`gen_server:call` is a synchronous request/reply built by tagging the request
with a unique reference `From = {addr, make_ref()}` so the reply can be tied
back to the exact call, and `gen_server:cast` is fire-and-forget. Supervision is
built from **links and monitors**: a supervisor is notified of a child's exit
and restarts it under a policy.

- Informs C-2 (the sketch's per-type mailbox with a `when`-routed `send` is the
  *typed, non-selective* inverse of Erlang's one-mailbox selective `receive` —
  the type system is what stops the lingering-message leak), C-6 (the `call`
  reference is exactly the reply-correlation a linear message needs), and the
  supervisor-as-handler fixed point.

### Elixir — sync/async split and demand-driven back-pressure

Elixir's `GenServer` makes the request/reply split explicit in the API:
`call` is synchronous (caller waits for a reply), `cast` is asynchronous. Its
unique variation is **`GenStage`**: back-pressure is *demand-driven* — a
consumer signals how many events it can take, and the producer sends at most
that many. Pressure propagates by pull, not by a blocked push.

- Informs C-3 directly: the sketch's block-the-sender back-pressure (point 5)
  is the push model; `GenStage` is the shape-changing alternative where the
  *receiver* sets the pace, which composes better across a pipeline of processes
  but is a larger mechanism.

### Gleam — the typed OTP relative

Gleam runs on the BEAM and puts a **type-safe layer over OTP**. Messages go
through a typed `Subject`, so the compiler catches protocol mismatches before a
message leaves the sender; there is **no selective receive** (a deliberate
omission — it is what makes the typing tractable). It is the closest existing
relative of the sketch's per-type mailboxes: a typed handle, no lingering-message
footgun, at the cost of Erlang's selective-receive flexibility.

- Informs C-2: strong precedent that "typed mailboxes, no selective receive" is
  a coherent and shippable point in the space — and that giving up selective
  receive is the price of the types, not a regression.

### Go — CSP with typed channels and `select`

Go is channels, not mailboxes: a `chan T` is a typed conduit, decoupled from any
process. A goroutine reads from as many channels as it likes, and **`select`**
waits on several at once — which is how Go expresses "receive from any of these
typed sources", the same job the sketch's multi-mailbox process does. A bounded
channel **blocks the sender when full** (an unbuffered channel is a rendezvous —
capacity zero); this is exactly sketch point 5.

- Informs C-2 (channels are the shape-changing alternative to mailboxes: the
  conduit is a first-class value separate from the process, which the sketch's
  per-type mailboxes are not) and C-3 (bounded-blocking is Go's default, and
  validates it).

### Concurrent ML — receiving as a first-class, composable value

CML (and its OCaml and Haskell descendants) makes **synchronisation first-class**:
`send` and `receive` return *events* immediately; `sync` blocks until the event
"happens"; and `choose` composes events, so "receive from A or B" is itself a
value you can pass around and build larger selections from. It is CSP made
functional — the act of receiving is data.

- Informs C-2 and C-6: if the sketch's `when`-routed receive ever needs to
  *compose* (wait on this mailbox or that reply), CML is the precedent for making
  the receive a value rather than a statement — a direction the sketch does not
  ask for but that C-2's channel alternative would open.

### Pony — reference capabilities make sending safe with no copy

Pony's actors pass messages with **no copying, no locks, no runtime overhead**,
and the type system is what makes that safe. Reference capabilities split
mutable-but-unique from immutable-but-shareable: `iso` is read/write *unique* —
sending an `iso` means the sender gives up all access, so the receiver can
mutate it safely — and `val` is immutable and freely shareable. Only `iso`,
`val`, and `tag` (opaque identity) are *sendable*; a `ref` (ordinary mutable
alias) is not.

- Informs C-4 directly: this is the capability pole of the sendability decision.
  Salvo already has the `iso` idea — a moved value the sender loses — as ordinary
  consumption (§1); Pony shows that a small sendability lattice can replace the
  `Rc`/`Arc` question with a *static* one, at the cost of a new axis in the type
  system.

### D — `receive` dispatches by message type

D's `std.concurrency` is shared-nothing message passing by default: `spawn`
returns a `Tid`, `send(tid, value)` is typed, and **`receive` is a switch-case
that dispatches the incoming value to the handler whose parameter type it
matches**. That is almost exactly sketch point 4 — a `when` over the union of
message types — but on the *receiving* side, and expressed with ordinary typed
delegates.

- Informs C-2: the strongest precedent for the sketch's routing model. D routes
  by type on receive; the sketch routes by type on send. The symmetry is worth
  noting — the type-directed dispatch can sit on either end.

### Cloud Haskell — typed unidirectional ports beside untyped addrs

`distributed-process` offers **typed channels**: a `SendPort a` / `ReceivePort a`
pair carries only values of type `a`, one direction, alongside the untyped
"send any term to an addr" primitive. It is a working demonstration that a process
can expose *several typed ports* — one per message type — which is the sketch's
"a mailbox per type" idea under another name.

- Informs C-2: precedent for many typed ports per process as an alternative to
  one union-typed mailbox. **Session types** in the Haskell literature push the
  same idea further — a channel's type encodes the *protocol*, and linear use
  ("every send has exactly one receive") is *communication safety* — which is the
  theoretical backing for C-6's linear messages.

### Concurrent Haskell — the shared-memory functional baseline (what OTP avoids)

`MVar` (a one-slot box that blocks) and **STM** (composable memory transactions,
`retry`/`orElse`) are the functional shared-memory tools. They are elegant — STM
composes where locks do not — but they are exactly the shared-mutable-state
model sketch point 1 rules out. Worth stating precisely, because "no `Mutex`"
does not mean "no good shared-memory story exists"; it means Salvo is choosing
the message-passing side of a real fork.

- Informs §1 / C-4: the contrast that justifies point 1, and the reminder that
  the `Cell` question (deferred) is this fork seen from the other side.

### OCaml 5 / Eff / Koka — effects as the thing concurrency is built *from*

Added 2026-09-14 with C-10. OCaml 5 adopted effect handlers **specifically to
build lightweight concurrency in userland**: its schedulers, fibers, and
message-passing libraries are ordinary libraries written with handlers —
`perform` passes the suspended continuation to the nearest handler, which may
store it and resume it later, and no concurrency primitive lives in the
runtime's surface. Eff and Koka both encode actors as an effect. The
freer-monad encoding from the functional literature (operations reified as a
union type, one `perform` over it, an interpreter matching the branches) makes
the correspondence textual: the operations-union *is* a mailbox union (C-2(a)),
the interpreter loop *is* a process's handler loop, and handler state is
process state. One sharp gap worth stealing from: OCaml enforces one-shot
continuations **dynamically** (resume twice → runtime exception); a language
with linear obligations can enforce it statically.

- Informs C-10 directly (the whole section), and reframes C-2/C-6/C-9: if
  performing an effect and sending to a process are one mechanism, the message
  structs, reply handles, and await are *generated* artifacts of one surface.

### Æff — asynchronous effects (Ahman & Pretnar)

Added 2026-09-15, with the direction; the survey's closest relative to what
was ultimately taken, found by asking what the finished model is *called*.
"Asynchronous Effects" (POPL 2021; core calculus **Æff**, with a higher-order
follow-up) takes algebraic effects and decouples the execution of an operation
call into **signalling** that the operation's implementation must run, and
**interrupting** the computation with the result, to which it reacts through
previously installed **interrupt handlers**. That is the kernel's decoupling,
term for term: signal ≈ `tell`; interrupt ≈ `fulfil` (a tell to a token);
installed interrupt handler ≈ a `then`-member, wired by `mint`; their promise
construct ≈ `Reply<T>`.

- What the direction adds beyond Æff: actor-style **state serialization**
  (Æff's interrupts are preemptive; ours queue as invocations, one at a time),
  the **gate** (bounded selective receive; Æff's interrupts arrive whenever),
  and **static linearity** of the tokens ([linear-obligation]; Æff's promises
  are structural). Informs the direction's name and its formal grounding.

### The two backends — where this must land

Whatever is chosen has to emit into both Kotlin and Rust, and they are
asymmetric — the same asymmetry that makes the substrate an open DECISION.

**Kotlin.** `kotlinx.coroutines` gives a `Channel<T>` that is "conceptually a
`BlockingQueue`, but with *suspending* operations instead of blocking ones".
Capacity is explicit: `RENDEZVOUS` (0 — `send` suspends until a `receive`),
a fixed buffer (`send` suspends once full — back-pressure), or `UNLIMITED`. The
`actor { }` coroutine builder is a process with a mailbox: it returns a
`SendChannel` others send to. Pool selection is **`Dispatchers`** —
`Dispatchers.Default` for CPU-bound work, `Dispatchers.IO` for blocking IO —
which is sketch point 6 already built into the platform. A `Deferred<T>` (from
`async`) is a future — sketch point 8's future-send.

- So on the JVM every sketch element has a near-native fit — but through
  *suspension* (coroutines), not blocking, which is the parity tension with Rust.

**Rust.** std has `mpsc::channel` (unbounded) and `mpsc::sync_channel(n)`, whose
sends "block until there is buffer space available" — `n = 0` is a rendezvous.
That is sketch point 5 exactly, in the standard library, blocking. `crossbeam`
adds mpmc channels and a `select!`; `tokio` adds bounded async channels
(`send().await` applies back-pressure) and a **`oneshot`** channel — a
single-value reply conduit that is precisely the future-send of point 8. But
std has **no runtime**: green threads or an async executor mean shipping a
scheduler in generated code, whereas OS threads plus `sync_channel` need none.
Sendability is enforced by the `Send`/`Sync` marker traits — `Rc` is not `Send`,
`Arc` is — which is the concrete form of C-4.

- So on Rust the blocking, OS-thread, `sync_channel` model is the one that needs
  no runtime and matches the JVM's *semantics* (if not its mechanism). The moment
  the design wants cheap processes or async receive, the two backends diverge —
  which is why C-5 and C-7 are the load-bearing calls.

## C-1. Spawn as an effect, and what effects a spawned process gets

*Settled by the 2026-09-15 direction: a process's effects are its handler
dependencies [effect-handler-deps], bound at spawn. Kept for the argument
trail.*

Sketch points 2 and (by consequence) the whole handler story. This folds
ROADMAP **DECISION 2**.

`spawn` is a capability, so it is a member of an effect — call it `Spawn` — and
a function that spawns declares `[Spawn]`, exactly as one that prints declares
`[Console]`. That much matches the effect model with no new machinery. The open
question is what handler set the *child* runs under, and it is forced by the
Rust lowering: an effect reaches a function as `&mut dyn E` borrowed for the
call, and the per-`use`-scope fusion holds *borrows* of the handlers registered
in the parent scope. A borrow cannot cross a thread boundary. So the child
cannot see the parent's handlers by reference — something has to give.

- **(a) The child opens its own `use` scope.** The spawned function starts with
  an empty handler set and must `use` whatever it needs, exactly as `main` does.
  *Cost:* none representationally — it is the model already. *Trade-off:* a child
  that needs `Console` must be given a way to make one; convenient ambient
  handlers (a logger registered once at the top) do not reach it. This is the
  recorded "likely answer".
- **(b) `spawn` takes an explicit handler set.** The parent passes the handlers
  the child starts with, by value (moved or cloned into the child), so they are
  owned on the child's thread. *Cost:* handlers must be sendable (C-4) and the
  API grows a handler-set argument. *Trade-off:* explicit and flexible, but "pass
  the handlers" is a second dependency-passing mechanism beside `use`, and a
  handler shared between parent and child is the `Cell` question (deferred)
  arriving from the other side.
- **(c) Shape-changing: handlers are values a process owns, and `spawn` is
  ordinary.** If a handler set were a first-class owned value (not a scope of
  borrows), `spawn(handlers, () -> …)` would be a plain move and (a)/(b) would
  collapse into one. *Cost:* reworks the Rust effect fusion from borrowed to
  owned at a thread boundary — the largest change of the three, and it touches
  code outside phase 5. *Benefit:* one dependency-passing mechanism, and the
  supervisor-as-handler story (a supervisor hands its child a modified handler
  set) becomes direct.

**Recommendation (the user's call):** start from (a) — it is free and it is the
model — and add (b)'s explicit hand-off only for the handlers a child provably
needs, treating (c) as the thing to reach for only if the borrowed-handler
boundary becomes the recurring obstacle. Whichever is chosen decides the *type*
of `spawn`, so it is the first call to make.

## C-2. The mailbox shape

*Settled in shape by the 2026-09-15 direction (with C-10): the protocol
surface is the effect declaration; the union, dispatch, and correlation are
generated. The ordering/bounds remnant lives on as C-3-reshaped ("Decisions
pending"). Kept for the argument trail.*

Sketch points 3 and 4: a process declares one mailbox per message type or
qualifier, and `send` takes the union and `when`-routes to the right one. This
is the load-bearing surface decision, and the survey offers four coherent
points.

- **(a) The sketch: per-type mailboxes, union-typed `send`, `when`-routed.** A
  process declares `mailbox<Request>`, `mailbox<Cancel>`, each bounded; `send`
  accepts `Request | Cancel` and `when`s to the queue for the arm. *Fit:* this
  is Salvo's grain — unions and exhaustive `when` already exist, so the routing
  is ordinary language. Closest existing relative is Gleam (typed, no selective
  receive) and D (type-directed dispatch, but on receive). *Cost:* the process
  type must express "these mailboxes with these bounds", a new type constructor;
  and per-type queues mean a message's *ordering* is only within its type, not
  across types (a `Cancel` can overtake a `Request`) — which must be stated,
  because Erlang programmers expect one ordered mailbox.
- **(b) One mailbox, `when`-on-receive (D / Erlang-typed).** A single bounded
  queue of the union; the process `when`s the union on `receive`. *Trade-off:*
  one total order across all message types (matches Erlang intuition), and one
  bound to size; but back-pressure is shared — a flood of one message type
  stalls senders of every type. No selective receive (a `when` consumes the head;
  it does not leave non-matching messages in place), which §2 shows is the right
  omission for a typed language.
- **(c) Channels, not mailboxes (Go / Cloud Haskell).** The conduit is a
  first-class typed value (`Channel<Request>`) separate from the process; a
  process reads several, `select`-style. *Shape change:* decouples the queue from
  the process, so a channel can be shared, passed, and selected over — more
  compositional, and it makes C-6's reply channel just another channel. *Cost:*
  it is a different mental model from "a process has mailboxes", and a first-class
  channel that holds fn-typed values collides head-on with C-4.
- **(d) Typed ports per process (Cloud Haskell `SendPort`/`ReceivePort`).** Like
  (a) but each mailbox is exposed as its own typed sendable handle rather than
  reached through one union `send`. *Trade-off:* a sender holds exactly the port
  it needs (least authority), but the sketch's single `when`-routed `send` — the
  part the user liked — goes away.

**Recommendation (the user's call):** (a) is the best fit for Salvo's existing
surface and is what the sketch describes; the one thing to decide with open eyes
is cross-type ordering (per-type queues give none). Keep (c) explicitly in view
as the shape-changing alternative, because if C-6 (replies) and future
composition (Concurrent ML) matter, a first-class channel value pays for itself
— at the cost of C-4 applying to channels too. **C-10 (added later the same
day) is the deepest version of this fork**: the protocol declared as an
*effect* rather than a message union, with the union, the reply plumbing, and
the dispatch all generated — C-2 and C-10 are one decision and should be made
together.

## C-3. Bounded blocking queues and back-pressure

*Reshaped by the 2026-09-15 direction: bounds and ordering now attach to the
invocation queue (per-process vs per-member), a blocking `tell` from inside a
handler needs a policy, and `fulfil` is exempt (reserved capacity). See
"Decisions pending", C-3-reshaped. Kept for the argument trail.*

Sketch points 3 and 5: mailboxes are bounded, and a full mailbox blocks the
sender.

- **(a) The sketch: bounded, block the sender (push back-pressure).** A full
  mailbox stalls `send`. *Fit:* this is `sync_channel(n)` in Rust std (sends
  "block until buffer space") and a buffered `Channel` in Kotlin (`send` suspends
  when full) — both backends have it natively, so it is the cheapest thing that
  is the same on both. *Cost:* blocking `send` can deadlock a cycle of processes
  each full and waiting on the other; that is inherent to push back-pressure and
  must be documented, not designed away.
- **(b) Unbounded.** `send` never blocks; the queue grows. *Trade-off:* no
  deadlock from back-pressure, but a slow consumer is an unbounded memory leak —
  Erlang's default and Erlang's classic outage. Rejected by the sketch, correctly.
- **(c) Shape-changing: demand-driven (Elixir `GenStage`).** The consumer signals
  how many it can take; the producer sends at most that. *Benefit:* composes
  across a pipeline without the deadlock cycles of (a), and no unbounded growth.
  *Cost:* a substantially larger mechanism — demand is a second message flowing
  the other way — and it changes `send` from "hand over a value" to "hand over a
  value when demand exists".

**Recommendation (the user's call):** (a), with the bound required at spawn
(sketch point 3) and a rendezvous (bound 0) available as the Kotlin/Rust
primitives both offer it. State the deadlock caveat in the spec. Hold (c) as a
later, opt-in pipeline construct rather than the default — it is the right tool
for streaming and the wrong default for request/reply.

## C-4. Sendability, and the `Rc` in generated code

*Unchanged by the 2026-09-15 direction, and promoted by it: now the first open
prerequisite, governing message arguments, seam-crossing captures, and tokens
alike.*

ROADMAP **DECISION 1**, met head-on. Generated Rust holds a fn-typed field as
`Rc<dyn Fn…>` [rs-fn-field] — every composed pass — and `Rc` is not `Send`. A
value crossing a process boundary as things stand may hold one. `Sendable` is a
structurally-inferred, backend-asymmetric, never-user-authored capability, so
whatever is chosen the compiler infers it.

- **(a) A rule and a diagnostic: a sent value may not hold a non-sendable
  field.** Infer sendability structurally; reject a `send` of a value that
  (transitively) holds an `Rc<dyn Fn…>`, naming the field. *Cost:* least
  machinery; the checker already reasons structurally. *Trade-off:* fn-typed
  fields become un-sendable, so a composed pass cannot be a message — which,
  after laziness was removed (2026-09-10), nothing in std needs, so the bite is
  small today and grows if pipeline-functions (deferred laziness) store fn
  fields.
- **(b) Change the representation to `Arc`.** `Arc<dyn Fn…>` is `Send` (when its
  captures are). *Cost:* an atomic refcount on *every* fn-typed field, whether it
  is ever sent or not — a whole-program tax to unblock the rare send. *Trade-off:*
  simplest to reason about; worst default cost, and it violates "pay for what you
  use".
- **(c) `Arc` only where sent, `Rc` otherwise.** Infer which fn-typed values can
  reach a `send` and make those `Arc`, the rest `Rc`. *Cost:* a sendability
  inference pass that feeds representation selection — real analysis engineering,
  but it is the "pay for what you use" answer.
- **(d) Shape-changing: a sendability lattice (Pony).** A small capability axis
  — sendable-immutable / sendable-unique / not-sendable — that the checker tracks
  and `send` requires. *Benefit:* zero-copy sends become statically safe and the
  `Rc`/`Arc` question dissolves into the lattice; Salvo already has the `iso`
  idea as ordinary consumption (§1). *Cost:* a new axis in the type system, the
  largest of the four, and it must stay *inferred* (never user-authored) to honour
  the `Sendable` watch-list constraint — Pony's is authored, so only the mechanism
  transfers, not the surface.

**Recommendation (the user's call):** (a) as the floor — a clear rule and a
diagnostic, cheap, and adequate while nothing in std sends a fn-typed field —
with (c) as the growth path if sending composed functions becomes real (it is
the same inference either way). (d) is the principled endpoint and the one to
choose if C-2(c) first-class channels land, because then sendability is pervasive
enough to deserve its own axis. This is the call most entangled with the others.

## C-5. The execution substrate: thread pools, OS threads, or a runtime

*Settled by the 2026-09-15 direction: (d) run-to-completion on a scheduler
library, with (b)'s pool selection at spawn riding along; the pool-assignment
surface is a remaining detail. Kept for the argument trail.*

ROADMAP **DECISION 3**, meeting sketch point 6 (selectable thread pools). The
backends are asymmetric: the JVM has cheap coroutines and `Dispatchers`; Rust
std has OS threads and no runtime.

- **(a) One OS thread per process, blocking `receive`.** The parity floor: no
  scheduler in generated code, identical semantics on both backends
  (`sync_channel` + `std::thread` on Rust, a blocking mailbox on the JVM). *Cost:*
  OS threads are not free, so "millions of processes" (Erlang's promise) is off
  the table; a few thousand is fine. *Trade-off:* cheapest to build and to keep
  in parity; least scalable.
- **(b) The sketch: selectable thread pools.** A process is assigned to a pool at
  spawn (CPU / IO / …). *Fit:* on the JVM this is `Dispatchers` almost exactly; on
  Rust it is an `ExecutorService`-equivalent (a thread-pool crate or a hand-rolled
  pool) reached as an `intrinsic`. *Key tension:* a pool multiplexes many tasks
  onto few OS threads — but a process that *blocks* on `receive` (C-3(a)) holds
  its pool thread for as long as it waits, so "one process per pool thread" does
  not pool at all unless the process yields. Pools and blocking-receive pull in
  opposite directions; reconciling them is the crux of this decision. The
  resolutions: keep receive blocking and let a pool be a *bounded set of OS
  threads* processes are distributed across (simple, but a blocked process ties up
  a thread), or make receive *yield* the thread — which is (c).
- **(c) Shape-changing: a green-thread / async runtime.** Receive yields; a
  scheduler multiplexes many processes onto few threads (Kotlin coroutines, a Rust
  async executor, or Loom-style virtual threads on a new enough JVM). *Benefit:*
  cheap processes and pools that actually pool; matches Erlang's scale. *Cost:* a
  scheduler shipped in generated code on at least one backend — the largest piece
  of machinery the two backends would *not* share — and it drags in C-7 (async),
  because a yielding receive is an async receive by another name.
- **(d) Shape-changing: run-to-completion processes on a scheduler library
  (Pony / Akka).** Added 2026-09-14 (user, in the session that wrote this
  document). There is **no blocking `receive` at all**: a process is an explicit
  state struct plus handler functions, and a `send` *is* scheduling a handler
  invocation — the scheduler holds a work queue of (process, message) pairs and a
  pool runs each handler to completion. Pony is the purest form (behaviours are
  the only receive there is); Akka/Actix/Orleans dispatchers, GCD serial queues,
  and SEDA are the same architecture; Erlang's own `gen_server` *API*
  (`handle_call`/`handle_cast` over explicit state) is this surface with the
  loop hidden by the behaviour. *Benefit:* pools genuinely pool — a waiting
  process occupies no thread, only a struct in a table — with **no scheduler
  compiled into user code**: the machinery is a small library (an
  `ExecutorService`-shaped pool on the JVM, a threadpool crate or a hand-rolled
  pool in Rust), identical in shape on both backends, so it threads the parity
  needle that (c) fails. The per-process serialization guarantee (one message at
  a time) is statically visible: handlers are the only code touching the state.
  This is the substrate analogue of the `yield fn` → `iter fn` move: the state
  machine a coroutine compiler would generate becomes the user's explicit state
  struct, and each step is an ordinary call [iter-protocol]-style. *Cost:* a
  handler **cannot block mid-body** — not on a reply, and dangerously not on a
  full mailbox either (a blocking `send` from inside a handler holds a pool
  thread, and cycles can wedge the pool — C-3(a)'s bound needs a policy for
  sends *from handlers*; Pony ducks this with unbounded queues plus runtime
  heuristics). Request/reply mid-handler becomes continuation-as-state — record
  what you are waiting for, return, resume in the handler for the reply — which
  is manual CPS and is Akka's well-known ergonomic sore. C-9 is the proposal to
  fix that with sugar rather than a runtime.

**Recommendation (the user's call):** ship (a) first as the parity floor, and
offer (b) as *pool selection over OS threads* — the user gets CPU/IO separation
(the stated goal) without a scheduler, accepting that a blocked process holds a
pool thread. Treat (c) as the deliberate, later step taken only when the blocking
model's thread cost is measured and found binding — and taken with C-7, since
they are the same decision. The parity principle says do not build a scheduler
until forced. **(d) changes this calculus** and is the strongest challenger to
(a): it delivers (b)'s pools *actually pooling* and (c)'s cheap processes with
library-sized machinery and full backend parity, at the price of forbidding
blocking inside handlers — evaluate it together with C-9, which is what makes
that price payable.

## C-6. Linear replies and the future-send

*Settled in shape by the 2026-09-15 direction: the linear `Reply` token
(CONCURRENCY_EXAMPLES.md, "What a `Reply<T>` is"); the future-send is subsumed
by call syntax. Kept for the argument trail.*

Sketch points 7 and 8: some messages are linear (a reply is guaranteed to come
back), and a "future send" yields a future the receiver completes and the sender
waits on. The general case is a round trip.

- **(a) Round-trip on a linear reply channel (the general case).** The sender
  includes a reply capability with the request; the receiver must fulfil it
  exactly once. Salvo already has the pieces: linearity survives a move
  [linear-obligation], so a *linear* reply handle sent with the request is an
  obligation the receiver cannot drop without a leak diagnostic — which is the
  "guaranteed reply" of point 7, for free. This is Erlang's `gen_server:call`
  reference and Akka's ask, made a type. (What the token *is* — value-borne
  routing, fulfil-as-enqueue, reserved reply capacity, the process-death edge —
  is defined in CONCURRENCY_EXAMPLES.md, "What a `Reply<T>` is".) *Cost:* the
  reply handle is a value that
  must be sendable (C-4) and, if it is a channel, is C-2(c) arriving; the
  discharge set for the linear reply must be defined (fulfil = discharge).
- **(b) Future-send: `send` returns a future.** `send` of a request yields a
  `Future<Reply>`; the receiver completes it; the sender awaits. *Fit:* this is
  `tokio::oneshot` and Kotlin `Deferred` exactly. *Trade-off:* ergonomic and it is
  the same round trip as (a) with the reply channel hidden inside the future — but
  "await a future" is a blocking (or yielding) wait, so it meets C-5/C-7: a future
  you *block* on needs no runtime; a future you *yield* on is async.
- **(c) Shape-changing: session-typed channels.** The reply is not a separate
  handle but the *next step* of the channel's protocol type (Haskell session
  types: every send has exactly one receive = communication safety). *Benefit:*
  the reply guarantee becomes a property of the channel type, checked
  structurally, no linear handle to thread. *Cost:* protocol-typed channels are a
  research-grade feature and a large addition; only worth it if C-2(c) channels
  are already the model.

**Recommendation (the user's call):** build (a) — it reuses linearity, which
already works across sends, so "guaranteed reply" costs a linear reply handle and
its discharge rule, nothing more. Offer (b) as sugar over (a) once C-5 settles
whether the wait blocks or yields (the future's wait is the same choice as
receive's). Keep (c) as the endpoint that only makes sense if channels (C-2c)
win. **Under C-5(d) this section is revised by C-9**: a future you *block* on is
forbidden inside a handler, and the awaiting sugar of C-9 — whose one-shot
continuation *is* a linear reply handle — becomes the native form of both (a)
and (b).

## C-7. Async's fate

*Dissolved by the 2026-09-15 direction: no async construct, no colouring — the
2026-09-04 record honoured by making the question moot. Kept for the argument
trail.*

ROADMAP **DECISION 4** (recorded 2026-09-04: async arrives later as an *explicit*
effect, never silent colouring). Phase 5 forces the question because a
*yielding* receive (C-5c) or a *yielding* future-wait (C-6b) is async under
another name.

- **(a) No async; receive and reply-wait block.** The OTP bet: a process blocks
  on `receive`, and that removes the need for async in the concurrency model
  entirely. *Fit:* the parity floor (C-5a). *Trade-off:* simplest; caps
  scalability at OS-thread cost.
- **(b) Async as an explicit effect, later.** If a yielding substrate (C-5c) is
  ever built, the yield is surfaced as an effect (`[Async]`-like), never as an
  ambient `suspend`/`async` colour — honouring the 2026-09-04 record. *Trade-off:*
  keeps effects honest, but it is a second concurrency mechanism beside processes,
  and the two must be designed to not overlap confusingly.
- **(c) Shape-changing: actor isolation hides it (Swift).** Swift actors make
  `await` ambient at actor boundaries. Explicitly *rejected* by the 2026-09-04
  record (no silent colouring) — listed only to mark it out of bounds.

**Recommendation (the user's call):** (a) for phase 5 — bet on blocking receive
and keep async out entirely, which is the OTP thesis and the parity floor. Revisit
(b) only if and when C-5(c) is built, and only as an explicit effect. Do not adopt
(c). This decision is downstream of C-5: decide the substrate, and async's fate
follows. **C-5(d) + C-9 add a fourth outcome**: async neither survives nor waits —
it *dissolves* into the process model, the way `iter fn` dissolved `yield fn`;
awaiting exists as sugar with no function colouring and no runtime, honouring the
2026-09-04 record by making the question moot.

## C-8. How regions integrate

Regions are already designed and ride along with this phase (user, 2026-09-10),
with **"a process is a region"** as the null hypothesis. This section only
records the integration question, since the region design itself is settled.

- **(a) A process is a region (the null hypothesis).** A process's owned state is
  exactly a region's scope-bounded data; a message that leaves the process leaves
  the region, which is precisely the sendability check (C-4). *Benefit:* the two
  features reinforce — "may not outlive the region" and "must be sendable to
  leave the process" become one rule. *Cost:* none beyond confirming the two
  scopes coincide.
- **(b) Regions are finer than processes.** A process contains several regions
  (e.g. a per-request region inside a long-lived server process). *Trade-off:*
  more expressive, but the "leaving the region = a send" equivalence weakens, and
  two scoping mechanisms must be told apart.

**Recommendation (the user's call):** confirm (a) — it is the null hypothesis and
it makes sendability and region-escape the same check. Only reach for (b) if a
concrete customer inside phase 5 needs a sub-process region; nothing recorded
does yet.

## C-9. Awaiting, as sugar over run-to-completion (depends on C-5(d))

*Settled by the 2026-09-15 direction: awaiting is call syntax (auto-mint plus
the gate); reentrancy is a per-call-site toggle. The seam checks and the
qualifier-invalidation idea remain live checker work. Kept for the argument
trail.*

Raised by the user 2026-09-14, in the session that wrote this document; not part
of the original sketch. The question: since Salvo controls code generation (as it
does for `iter fn`), can it give the *appearance* of awaiting — compiled down to
the C-5(d) model — without the runtime that async/await normally brings?

**What the mechanism is, named honestly.** "Cut the function at the seam" is
precisely what async/await *is*: C#, Kotlin `suspend`, and Rust `async fn` all
compile await by splitting the function at each await point and reifying the
locals that cross a seam into a compiler-generated state machine. That is exactly
the `yield fn` machinery this language already rejected once. The proposal is
therefore not "whether the transform exists" (it does, proven at scale, and it
needs no executor — the generated Rust is an ordinary state enum plus a `step`
function the pool invokes; Kotlin can emit the *same explicit shape* rather than
reaching for `suspend`) but whether the **`iter fn` lesson** can be applied to
it: keep the underlying model ordinary and hand-writable — a process is a state
struct plus handler functions anyone can write — and make `await` mechanical
sugar that *writes that struct and those handlers for you*, the same relationship
`iter fn` has to a hand-written pass [iter-protocol]. Theoretically this is
**one-shot delimited continuations**: `try` is already Salvo's delimiter for
`Throw`, where non-resumption was chosen precisely to avoid capturing
continuations; await is the one place a continuation is genuinely needed, and
only one-shot — captured once, resumed exactly once.

A hypothetical sketch (illustrative syntax, nothing decided; **worked examples
of all three process shapes — request-shaped, event-shaped, and the hybrid —
are in CONCURRENCY_EXAMPLES.md**). Desugared — the hand-written C-5(d) form,
continuation-as-state:

```
struct GetUser { id: Int, reply: Reply<User | Err Str> }

struct UserServer {
    db: Addr<DbQuery>,
    pending: Mut Map<Int, Reply<User | Err Str>>   // parked continuations
}

fn handle(s: Mut UserServer, msg: GetUser) [Send] {
    s.pending.put(msg.id, msg.reply)               // park: cannot block here
    send(s.db, DbQuery {id: msg.id, reply_to: self})
}

fn handle(s: Mut UserServer, msg: DbReply) [Send] {  // resume, by hand
    let reply = s.pending.take(msg.id)
    fulfil(reply, parse_user(msg.row))
}
```

Sugared — one handler, one seam, and the compiler writes `pending`, the second
handler, and the correlation:

```
fn handle(s: Mut UserServer, msg: GetUser) [Send] {
    let row = await request(s.db, DbQuery {id: msg.id})   // ← the seam
    fulfil(msg.reply, parse_user(row))
}
```

**The three calls that decide whether it is sound** (independent of codegen):

- **(i) Reentrancy — the semantic call, bigger than the transform.** While a
  handler is parked at a seam, does the process run other messages?
  *Non-reentrant* (Orleans grains): the logical handler keeps run-to-completion
  semantics end-to-end, but A-awaits-B-awaits-A deadlocks — Orleans ships
  deadlock detection because of it. *Reentrant* (Swift actors): no deadlock, the
  pool stays live, but state can change across every seam — the classic bug is
  checking an invariant before `await` and relying on it after. Salvo has a hook
  no language in this list has: the qualifier machinery already distinguishes
  claims *invalidated by mutation*; "a claim invalidated by an await" (other
  handlers ran meanwhile) is the same shape, which could make reentrancy
  *checkable* rather than documented. CONCURRENCY_EXAMPLES.md works the
  comparison concretely (Example 3 vs 3b): reentrant await wins when awaiting
  activations are independent, and buys little — while importing the
  stale-claim hazard — when they coordinate through shared state.
- **(ii) What crosses a seam becomes a field.** Locals live across an await are
  stored in the process state — so C-4 sendability and [fate-lambda] apply to
  them with full force (a local `Rc<dyn Fn…>` alive across a seam is the
  un-`Send` field problem exactly), and effect handlers reached as borrows
  cannot cross (the segment after the seam runs later, possibly on another pool
  thread) — an awaiting scope that *owns* its handlers in the process state is
  C-1(c) arriving by necessity. This is the strongest argument for a `try`-like
  **delimited scope**: it bounds the transform (only code inside pays) and gives
  the checker a precise boundary at which to enforce "everything live here is
  storable and sendable".
- **(iii) One-shot = linear.** The captured continuation must be resumed exactly
  once — twice is corruption, never is a hang. Salvo can *state* that: the
  continuation is (or rides in) a linear reply handle [linear-obligation], C-6(a)
  and this section unify, and the existing leak diagnostic becomes "this request
  can never be answered".

**Recommendation (the user's call):** treat C-9 as a package with C-5(d) — it is
what makes (d)'s no-blocking price payable, and (d) is what makes await
runtime-free. If the package is taken: awaiting only inside process handlers (or
an explicit delimited scope), never as a property of function types — no
colouring, which is what keeps the 2026-09-04 record honoured (C-7). Decide (i)
reentrancy first and deliberately; the recommendation is non-reentrant by default
(simpler invariants, matching run-to-completion's promise) with the deadlock
caveat stated — CONCURRENCY_EXAMPLES.md's Example 4 shows the cycle concretely
and lays out the static-detection options (an effect-surfaced await graph,
qualifier-tier stratification, and typed timeout/reentrant-opt-in fallbacks) —
and the qualifier-invalidation idea explored as the path to
relaxing it. Prototype (ii)'s seam check before committing surface syntax —
whether it falls out of the existing flow analysis will decide the build cost.

## C-10. Processes as effect handlers — one concept or two

*Taken as the direction (user, 2026-09-15), through the kernel formulation —
see "The direction" below. The sub-decisions (i)–(iii) resolved into the
tower; (iv)'s gate stands as designed; `defer`'s fate is an open readability
call. Kept for the argument trail.*

Raised by the user 2026-09-14, later the same session as C-9: the observation
that the C-5(d) examples — a state struct, a `handle` function per message
type, dispatch on the message's type — are structurally an effect handler,
with messages as defunctionalized operation calls. This is not an aesthetic
resemblance: the constructions are known to be inter-encodable, and the
strongest precedent went exactly this direction (OCaml 5 built its concurrency
*out of* effect handlers — see §2). The correspondence, exactly:

| Effect handlers (Salvo today) | Processes (the C-5(d) examples) |
|---|---|
| `effect UserApi { fn get_user(id: Int) -> User \| Err Str }` | The message union `GetUser \| DbReply` — defunctionalized by hand |
| An operation call: `get_user(7)` | Constructing `GetUser {id: 7, reply}` and sending it |
| The implicit continuation at the call site | `reply: Reply<…>` — the continuation reified as a linear value |
| A handler clause | `fn handle(s, msg: GetUser)` |
| Handler state (`handler H(…) of E { i: Int = 0 … }`) | The process state struct |
| `use H(...)` — bind effect to implementation, in scope | `spawn` — bind protocol to a running instance |

The honest statement of the unification: **a process is an effect handler plus
a mailbox plus serialization**. The protocol, state, and dispatch concepts are
the same; what differs is the *binding* — a `use`d handler runs synchronously
on the caller's thread inside dynamic scope, a spawned one runs asynchronously,
serialized, shared by many callers. The declaration stays one thing; the
registration decides sync or async.

**What it buys, concretely:**

- **The message structs are generated, not written.** Every message struct in
  CONCURRENCY_EXAMPLES.md is hand-defunctionalization; the compiler already
  performs this family of transform in reverse (effects → traits with methods,
  on both backends). Declaring the protocol as an `effect` and deriving the
  union + reply plumbing is the same machinery pointed the other way.
- **C-9's await stops being a construct.** Calling an effect member on a
  process-backed handler *is* the await — ordinary call syntax, continuation
  implicit, linear underneath. The sugar and the thing it sugars collapse.
- **Example 4's deadlock graph is the effect graph.** No `[Await<Q>]` effect
  needs inventing: if calling a process is performing an effect, the await
  graph is already in every signature, and the cycle check is an analysis over
  the effect lists the language is committed to keeping visible.
- **C-1 reframes into existing rules.** "What effects does a spawned process
  have" becomes "what are this handler's declared dependencies"
  [effect-handler-deps] — supplied at spawn, which is the existing
  handler-dependency rule at a distance.
- **Supervision gets real teeth.** Interception (a handler depending on the
  effect it implements; `use` shadowing binding outward) now composes across
  the scheduler boundary: retry/timeout/restart policy around a process is a
  wrapping handler — the existing mechanism, no new construct
  (CONCURRENCY_EXAMPLES.effects.md Example 4).
- **Statically linear continuations — the differentiator.** OCaml enforces
  one-shot resumption dynamically; Salvo's reified continuation is a linear
  reply handle [linear-obligation]: resume-twice is use-after-discharge,
  resume-never is a leak. This is the piece the effects literature wants and
  does not have.

**The sub-decisions the unification forces** (worked concretely in
CONCURRENCY_EXAMPLES.effects.md):

- **(i) The call/tell split.** An effect member call waits for its result;
  fire-and-forget (`Topic`'s `publish`, gen_server's `cast`) has no synchronous
  reading. The effect declaration needs a per-member asynchrony marker — a
  `tell fn` whose call returns at *enqueue*, against the default `fn` whose
  call returns at *reply*. This changes what a member's return type means, so
  it is surface, not detail.
- **(ii) Pids as values, effects as capabilities.** Effects cannot be stored
  or passed [it is a compile-time error today]; real topologies must hold and
  send process references. The resolution to evaluate: an `Addr<Protocol>` is an
  ordinary sendable *token* value, and calling through it requires a scoped
  binding (a `use addr`, or dot-call sugar `addr.member(args)` as its inline
  form) — keeping "effects are not values" intact while making the *address*
  transferable. One finding makes the token form load-bearing rather than
  convenient: scoped binding is **singular** (one handler per effect per
  scope; a later `use` shadows), so N instances of one protocol — a shard
  pool, a scatter target set — are unreachable through scope and *only*
  addressable through tokens (CONCURRENCY_EXAMPLES.effects.md, Example 5).
- **(iii) Deferred replies and continuation-members.** `NoticeFetcher` parks
  replies and must stay reactive during its own downstream call. The surface
  needs (per the literature's handler-receives-`k`): a member form that names
  its reply instead of auto-responding (`defer reply` — the reified linear
  handle), an explicit continuation-routing form for performing a call
  without parking (`fetch_batch(n) then batch_arrived` — the hand-written
  reentrant shape of C-9/Example 3b, kept visible), and the gate modifier
  (`then only member` — the same routing with the mailbox gated to that
  reply; see (iv)).
- **(iv) Serialization vocabulary — the gate.** Refined twice on 2026-09-14
  (user questions: "if plain calls desugar to `then`, is everything
  reentrant?"; "can the gate be spelled apart from the plain call?"). The
  resolution, with the modifier approach taken as **user direction
  2026-09-14**: a plain call desugars to `defer` + a cut at the seam + the
  gate modifier **`then only member(captures)`** — while a gated continuation
  is outstanding, the mailbox admits only the awaited reply; bare `then`
  leaves it open. Both end the activation and store a continuation, so
  run-to-completion is never violated; C-9(i)'s reentrancy choice is a
  per-call-site gate toggle, and the desugaring is *complete* (the iter-fn
  property holds — CONCURRENCY_EXAMPLES.effects.md, Example 1 desugared).
  The primitive out-expresses the sugar (gated-with-code-after; gated routing
  to a member), the deadlock graph's edges are exactly the *gated*
  continuations (plain and `then only`; bare `then` contributes none), and
  the gate is a bounded compiler-controlled selective receive — Gleam's
  dropped feature readmitted in the one statically-typed shape, with a
  member-*set* generalization available later if demanded. Two rules ride
  along: **at most one gated continuation may be outstanding per process**
  (a second is an error — the plain-call sugar cannot trip this, only
  hand-written `then only` can; a scatter is therefore necessarily bare
  `then`, Example 5), and one asymmetry to
  carry into the C-10(a)/(c) choice: `only` names the gate but cannot
  decompose it — the decomposition (Akka-style stash) requires reified
  messages, which exist only in the message-passing surface, so under (c)
  `then only` is a true primitive while under (a) it has a lower-level
  expansion.

**Options:**

- **(a) Adopt the unification as the surface.** Processes are declared as
  effects + handlers; `spawn` is the async binding; C-2's message unions,
  C-6's reply handles, and C-9's await become generated artifacts. *Cost:* the
  four sub-decisions above are all load-bearing surface design; the effect
  chapter grows (i)–(iii) as new forms. *Benefit:* one concept where the
  alternative is five (process, mailbox, message, send, reply) — the largest
  simplification available in this option space.
- **(b) Keep them separate.** Processes get their own surface (C-2 as drafted);
  effects stay synchronous capabilities. *Cost:* two protocol concepts (effect
  declarations and message unions) that are the same thing squinting; the
  defunctionalization is user-visible forever. *Benefit:* no changes to the
  effect chapter; each side stays simple on its own terms.
- **(c) Shape-changing middle: processes are a *lowering target*, effects the
  only surface.** Even hand-written processes are declared as handlers; the
  message-union form does not exist in the language at all, only in generated
  code. This is (a) taken to its conclusion — worth naming separately because
  it deletes C-2 rather than answering it.

**Recommendation (the user's call):** the correspondence is real, load-bearing,
and Salvo-shaped — (a) is the direction to pursue, entering through C-2 (they
are one decision: the protocol surface). Work sub-decisions (i)–(iii) on the
worked examples before committing syntax, and treat (iv) as C-9(i) restated —
decided once, worded for effects. (b) remains the fallback if (i)–(iii) turn
out to cost more surface than they delete; the test is whether
CONCURRENCY_EXAMPLES.effects.md reads *simpler* than CONCURRENCY_EXAMPLES.md
to someone who knows today's effect chapter.

## The direction (user, 2026-09-15) — the kernel and the sugar tower

Taken by the user 2026-09-15, closing the exploration C-9 and C-10 opened:
**the effect surface is the model**, resting on a four-piece kernel with
everything else a strict sugar tower above it. Worked in full, with the
derivations, in CONCURRENCY_EXAMPLES.effects.md ("The kernel, and the sugar
tower"); recorded here as the shape phase 5 builds.

**The name: asynchronous effect handlers.** The model's closest formal
relative is Ahman & Pretnar's *asynchronous effects* (the Æff calculus — see
§2), whose signal/interrupt/interrupt-handler decoupling matches the kernel
term for term; "asynchronous effect handlers" is therefore an inherited name,
not a coined one. Within Salvo's own vocabulary the noun stays **process** —
"a process is a handler bound asynchronously with `spawn`" — so the existing
effect chapter renames nothing. The full lineage, one line per piece:
algebraic effect handlers (Plotkin–Pretnar) for the protocol/handler/state
base Salvo already has; **Æff** for the tell/interrupt decoupling; active
objects with asynchronous method calls and futures (Creol/ABS/Rebeca) for the
same runtime shape in the distributed-OO tradition; run-to-completion actors
(Pony behaviours, Akka dispatch, `gen_server`'s callback surface) for the
execution substrate; the join calculus (Fournet–Gonthier; Polyphonic C#'s
chords) for the merge form; defunctionalized one-shot delimited continuations
for the compilation story. What the direction adds that none of these have
together: scheduler-serialized handler state, the gate as a bounded typed
selective receive, and statically *linear* continuation tokens — the piece the
effects literature enforces only dynamically.

**The kernel:**

1. **Process state** — a handler's fields; exclusive by scheduler
   serialization (one activation at a time).
2. **`tell`** — enqueue a member invocation on a process. The only send.
3. **`mint k(captures)`** — allocate a parked one-shot continuation targeting
   member `k`, yielding its linear `Reply<T>` [linear-obligation].
4. **The gate bit** (`only`) — bounded selective receive; named, not
   decomposed (C-10(iv)); at most one outstanding per process.

**The tower**, each layer expressible in the one below: `fulfil(r, v)` is a
tell to a token (a `Reply` is a one-shot Addr with a single tell member) ·
`then k(c)` is `mint k(c)` placed in the reply slot · `then only` is that plus
the gate · a member's `-> T` is an implicit trailing `reply: Reply<T>`
parameter — so the only primitive member kind is `tell`, and a "call member"
is a tell member carrying a token · a plain member body is fulfil-at-every-
return · `defer` is a *marker only* (announces "does not respond before
returning"; keep-or-drop is an open readability call) · caller-side call
syntax `let x = m(a)` is an auto-mint of an anonymous resume continuation
(captures = the seam-crossing locals) plus the gate — legal only for members
with a single trailing token, multi-token members take explicit mints · the
merge/join form `k@self(c, reply e1, reply e2)` is a multi-mint into generated
gather state (Example 5 of the effects file is its manual desugaring; race,
deadline, and partial-results are gather-state *policies*, deliberately not
primitives).

**What the direction settles across the decision sections** (each kept above
for the argument trail):

- **C-2 + C-10** — settled in shape: the protocol surface is the effect
  declaration; unions, dispatch, and correlation are generated. The
  ordering/bounds remnant moves into the reworked C-3 (see the table).
- **C-5** — (d): run-to-completion on a scheduler library, which the kernel
  presumes, with (b)'s pool selection at spawn riding along; the
  pool-assignment surface is a remaining detail.
- **C-9** — awaiting is call syntax; the gate is the per-call-site reentrancy
  toggle. The seam checks (everything crossing is storable and sendable) and
  the qualifier-invalidation idea remain live checker work.
- **C-6** — the linear `Reply` token as defined (CONCURRENCY_EXAMPLES.md,
  "What a `Reply<T>` is"); the future-send is subsumed by call syntax.
- **C-7** — dissolved: no async construct, no colouring; the 2026-09-04
  record honoured by making the question moot.
- **C-1** — a process's effects are its handler dependencies
  [effect-handler-deps], bound at spawn.

## The first pass (user decisions, 2026-09-15, second round)

Scope decision: the first implementation pass ships the **minimal non-sugared
surface** — explicit tokens, explicit reply parameters, no call sugar. Later
passes add layers of the tower, each with its own decision surface.

**Decided this round:**

- **C-4 = (a)**, the structural rule + diagnostic — a sent or captured value
  may not transitively hold a non-sendable field — **with (c) recorded as the
  growth point** (`Arc`-where-sent inference) for when sent closures or
  pipeline-functions become real. Governs send payloads, reply captures, and
  tokens.
- **C-3-reshaped: one arrival-order queue per process**, its bound defined at
  construction (spawn). A blocking send from *inside* a handler is **allowed**;
  its load-conditioned wait-for edges join the deadlock graph rather than being
  forbidden.
- **Deadlock statics: the effect-graph cycle check (Example 4 a/b) is the
  committed baseline**, shipping with the first pass that has gated parking;
  stratification (c) and the fallbacks (d) wait until the baseline's false
  positives are observed rather than predicted.
- **C-8 confirmed: a process is a region.** Corollary to spell out during
  implementation (owed): **handlers never cross into a spawn — construction
  crosses.** A child either constructs its own handlers (`use H(args)` in its
  init, with the constructor args subject to C-4) or holds `Addr` tokens for
  process-backed capabilities. No `fork`/duplication operation on handlers
  exists or is needed — "duplicating a handler" is re-running its constructor,
  which is already expressible.
- **Spellings** (user, with the kernel names): `tell` → **`send`** (members
  are `send fn …`; in the first pass *every* member is one, and the unmarked
  `fn` member spelling stays reserved for the later call-member sugar);
  `mint` → **`replyto`** (renamed from `reply` 2026-09-15 third round: the
  expression `replyto k(captures)` yields the `Reply<T>` — "the reply goes
  *to* `k`" — resolving the noun/verb ambiguity; compound keyword per the
  `canbe` precedent); the gated mint → **`replyto!`** (the `!`-ambiguity with
  Erlang/Rust accepted); **`fulfil` is dropped for `r.send(v)`** (user:
  consumption of the token already says the linear obligation is discharged;
  a `Reply` is a one-shot Addr with one send member, so sending to it *is* the
  operation); the **`spawn` effect is lowercase** in effect lists, like `use`
  — `fn main() [use, spawn]` is the typical entry. `Addr<T>`, `Reply<T>`, and
  dot-call stand. **`use addr` is first-pass, not sugar**: it binds an effect
  in the current scope to a generated forwarding stub over the `Addr` — legal
  anywhere, including `main`, since all first-pass members are sends (no
  parking); its value is unqualified calls and, above all, passing the
  capability *down* through ordinary effect lists (`fn drive() [Roll]`).
  What `main` can do with it is sends only — `replyto` targets a member of
  the *enclosing handler*, and `main` has none, so `main`'s only token
  source is `waitfor`. Dot-call is the inline form of the same binding.
  **Not in the first pass** (later sugar passes, each with its own
  decisions): `then`, `defer`, member `-> T`, caller-side call syntax, and
  merge/join.
- **Spawn syntax (decided this round; clause spelling settled 2026-09-15,
  third round):** `spawn H(args) [use D1(...), addr, …] capacity N on POOL`.
  Dependencies are **ordinary constructor parameters** or the spawn-site
  `use` clause; **`capacity N`** is the process's own mailbox bound
  (explicit, required, no default — it is a property of *this instance*, so
  it sits at the spawn site); **`on POOL`** takes an ordinary expression,
  and `pool(n)` is an **ordinary function** (declared `[spawn]`), not
  syntax. `spawn` and the three clause words are *contextual* — nothing is
  reserved, following the `iter fn` precedent. Read left to right: what to
  run, what it depends on, how deep its queue is, where it runs.
  * **Rejected: `spawn` as an intrinsic function with named parameters**
    (user asked 2026-09-15). Two blockers: Salvo has no named arguments at
    all (calls are positional plus variadics and `?`-implicits), and —
    load-bearing — **handlers are not values**, so `Counting()` in
    `spawn Counting()` is a *construction* only `use`/`spawn` may perform,
    not an argument. Making it a call would mean making handler
    constructions first-class, against "effects are capabilities, not
    values". The clause keywords *are* the named parameters.
  * **Rejected: `capacity` on the handler declaration** (option (E)): it
    would be meaningless for a `use`d handler, and marking handlers
    async-only re-introduces the sync/async split at the handler that C-10
    removed — Example 6's binding swap depends on the same handler working
    both ways. It could never live on an *effect*, which sync handlers share.
  * The spelling is revisitable (user, 2026-09-15) — nothing downstream
    depends on the words.
- **Prerequisite sequencing** (user): the **linearity-in-collections design
  comes first**, with this as its first customer; **[fate-lambda]** rides with
  it; and **the monitors/supervision story is designed before full
  implementation begins**, so it can inform implementation decisions rather
  than retrofit them.

- **The main boundary (decided this round): the explicit bridge.** `waitfor
  out: Reply<T> { … }` — legal only in `main` — mints a token, requires it
  consumed in the block (ordinary linearity), blocks main's real thread until
  it is sent to, and yields what was sent. Paired rule: **the program ends
  when `main` returns**; anything still running dies with it, and a program
  that means to serve says so by waiting on a shutdown token. No
  run-to-quiescence ambient semantics.

- **Child handler wiring (decided this round): the spawn-site `use` clause.**
  Handlers declare dependencies as today ([effect-handler-deps]); the spawn
  site supplies them — `spawn Roller(v) use MemFs(root), StdOutConsole() on
  pool(2)` — a restricted form (handler name + argument expressions, no
  closure): arguments evaluate in the parent and cross under C-4,
  construction happens on the child, exhaustiveness is checked at the spawn
  site like a `use` scope. In-body `use` stays legal for genuinely internal
  handlers. This is what keeps test wiring (`MemFs` for `DefaultFs`) working
  across the process boundary.

**Carried to the call-sugar pass (named question, 2026-09-15):** may an
*ordinary* (non-`send`) effect member be process-backed (`use addr` binding an
effect whose members return values — the "IO actor" pattern)? The call site
would look synchronous but park. Options sketched: allow silently
(uniformity; hidden deadlock edges), allow with the distinction carried at
the *binding site* only (a `use addr` binding is where the deadlock-graph
edges appear; call sites stay agnostic, the platform-handler precedent), or
forbid (process-backed capabilities must be all-`send` protocols; sync
effects never park). The first pass does not force this — without call sugar
an `Addr` can only back all-`send` protocols.

**Still being workshopped:** nothing — the first-pass grammar is frozen.

## Decisions pending

Reworked twice 2026-09-15: "The direction" settled the model and surface in
shape; "The first pass" settled C-4, C-3-reshaped, the deadlock baseline, C-8,
the kernel spellings, and the prerequisite sequencing. What remains:

| Question | Status / note |
|---|---|
| **Design prerequisites, sequenced** | **All resolved 2026-09-15**: 1. linearity-in-collections **DECIDED** (LINEARITY_COLLECTIONS.md; the intrinsic extension of phase 3's conditional containers); 2. [fate-lambda] **deferred to the call-sugar pass** (no first-pass form crosses a closure); 3. supervision/monitors **DECIDED** (SUPERVISION.md — death = faulted activation, `watch` + linear `Exit` token, silent no-ops to the dead + idle-with-parked-gates report, supervision as a pattern). **The decision space ahead of implementation is empty** |
| **Implementation placement** (user, 2026-09-15) | The scheduler library lives in **backend runtime files** (the emitted-support precedent); anything that turns out to need compiler-specific cooperation is **flagged to the user** before being built that way. The per-process queue bound is **explicit and required** at spawn — no default |
| **Later sugar passes** (each with its own decision surface) | member `-> T` + call syntax — including the named question: may an ordinary member be process-backed (the IO-actor pattern)?; `then`/`then!`; `defer` (or the desugared-signature rule); merge/join; the gate member-set generalization; the `use`-block spawn form |
| **Deferred checker work** | qualifier-invalidation across seams (checkable reentrancy); stratification (c) and fallbacks (d) for deadlock statics, when the baseline's false positives are real |

Load-bearing order among what is left: the three **design prerequisites**
gate implementation start (the supervision design explicitly so — user,
2026-09-15); the three **syntax items** gate the grammar freeze; the sugar
passes and checker work follow the first pass. See DESIGN_DOC.md for the
shape this document follows.
