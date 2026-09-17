# Free concurrency — free functions in the actor world (working document)

Status: **OPEN**, with recorded user directions (2026-09-17, this session's
conversation): the kernel-first constraint (§0.2), the two-case decomposition
(§0.3–4, FC-1/FC-2's shape), and pool selection defaulting to the current pool
(§0.5, FC-3). The load-bearing open question is **FC-4 — whether `main` runs
on a pool** — asked by the user at this document's creation; nothing is
implemented.

Written 2026-09-17, the day after phase 5 closed (all five phases of "the
sequence" complete; the phase-5 working documents retired into COMPLETED.md's
log and the `[actor-*]` rules). The design grew out of a conversation that
began at unbounded mailboxes, produced TIME.md (Timer/Clock, still open), and
arrived at the observation this document exists for: **in a highly-concurrent
program the logic concentrates in actors, and a free function is a limited
citizen** — it can send (through the stub) but cannot mint, so it cannot
participate in continuation-shaped code at all. The document lays out the
design in DESIGN_DOC.md's shape — options, trade-offs, recommendations — and
the calls are the user's (AGENTS.md's first invariant).

**Propagation owed.** Only this file is written. Still owed to a write
session: a ROADMAP.md item pointing here (adjacent to "The sugar pass — after
phase 5", whose carried watch item this design answers); COMPLETED.md log
entries when the FC directions are confirmed as decisions; and a staleness
sweep of **TIME.md**, which cites the now-deleted CONCURRENCY.md /
SUPERVISION.md and whose T-2 option 3 ("Clock backed by the timer process")
is overtaken by [actor-effect-kind]'s "a plain effect is never actor-backed".
This document is **deleted** once its outcomes live in the log and the specs
— the OBLIGATIONS.md / FILE_SYSTEM.md / CONCURRENCY.md charter.

Sources: LANGUAGE_SPEC.md's actor chapter — [actor-kind], [actor-effect-kind]
(the kind divide and its refusal list), [actor-sendable], [actor-replyto]
(the rule this design amends), [actor-waitfor] (and its paired rule: the
program ends when `main` returns), [actor-use-addr] (sends are legal from any
frame), [actor-watch], [actor-deadlock-cycle], [actor-types]; [linear-
obligation] and [linear-container]; [rs-fn-field] and [fate-lambda] as
representational facts; COMPLETED.md's phase-5 log (the rejection of full
sync/async unification and the kept "refusal list of a kind" diagnosis; the
`async` → `actor` rename and why the word `async` is deliberately absent);
ROADMAP.md "The sugar pass — after phase 5" (EU-6/EU-7b's folded content —
**context, not constraint**, per §0.2 — and its carried watch item: helpers
that need continuations in data-dependent number cannot be written outside a
handler body); the spawn-line respelling **DECISION** (the `Placement`
surface FC-3/FC-5 ride on); and the user's stated intent (below).

## 0. The stated intent

The user's direction from the 2026-09-17 conversation, numbered so the
decisions below can be judged against it.

1. **Free functions should be full citizens of the concurrent world.** Much
   of a concurrent program's logic centres on actors because only handler
   bodies can mint continuations; logic that needs no actor *state* should
   not need an actor. The ambition is the algebraic-effects property — no
   function colouring — reached not by making everything asynchronous (that
   is round 1 of the unification, already rejected) but by letting the
   **send kind extend to free functions** while ordinary functions keep the
   full synchronous feature set.
2. **Kernel first.** The sugar tower's plans (EU-6/EU-7b in ROADMAP.md) are
   tentative and **must not shape this kernel** (user, 2026-09-17). Where
   this document notes compatibility with them, it is a fact, not a design
   input.
3. **Case 1 — the minting normal function.** A normal function — return
   type, full features, synchronous — may use `replyto` when calling a
   `send fn`, targeting another free function, optionally on another pool.
   It wires future work and returns; it never parks and never blocks.
4. **Case 2 — the free `send fn`.** A free function declared `send fn`,
   with the usual limitations (the actor-member refusal list); it runs by
   being scheduled.
5. **Pool selection is optional, defaulting to the current pool** — no new
   vocabulary for "the pool I am on" (user, 2026-09-17, choosing against a
   required-`on` alternative whose "explicitness" was the same ambient read
   with ceremony, or a hand-threaded context parameter).
6. **`waitfor` stays the marker of the blocking world** — dedicated threads,
   `main` — and is not the norm.

Point 1's literal reading ("everything async") collides with the recorded
rejection of full unification; the kind-divide framing in FC-1 is the
resolution, and the refusal list is the instrument that shows where the
boundary must sit. Point 3 collides with [actor-replyto] as written (targets
are members of the enclosing handler; the form is an error outside handler
bodies) — that amendment is FC-2. Point 5 has an unresolved edge — `main`
has no current pool — which is FC-4, this document's load-bearing question.

## 1. Fixed points — already decided, inherited here

- **Run-to-completion; a handler cannot block mid-body** [actor-kind]. This
  design adds no seams, no state machines, no mid-body waits anywhere: a
  parking free function does not exist — a free `send fn` runs to completion
  and results travel by token.
- **The kind divide sits at the declaration, with a refusal list**
  [actor-effect-kind]: kept parameters, `Mut` parameters, `proj` returns,
  non-sendable payloads — checked where the author is deciding. Full
  sync/async unification (every member a `send fn`) was **rejected** (user,
  2026-09-15; COMPLETED.md log): the synchronous features are not a
  deficiency list, they are what the sync kind *is*. FC-1 extends the kind
  divide to free functions; it must not collapse it.
- **Sendability is checked at the declaration for member payloads, at the
  site for `replyto` captures and spawn arguments** [actor-sendable]. Free
  `send fn` declarations and `Task`-arm captures get the same two checks.
- **[actor-replyto] as it stands**: `replyto k(captures)` targets a `send fn`
  member of the *enclosing handler*; captures-then-answer (the trailing
  parameter is what the token carries); outside a handler the form is an
  error naming `waitfor`; a self-parking handler may only be `spawn`ed
  (a `use`-bound instance has no mailbox for the answer). FC-2 amends the
  target set; the trailing-token convention and the linearity are unchanged.
- **[actor-waitfor]**: `main`'s explicit bridge and only token source; blocks
  main's real thread; paired rule — **the program ends when `main` returns**,
  no run-to-quiescence. FC-4 decides whether "blocks" becomes "drives".
- **Sends are legal from any frame** [actor-use-addr]: the forwarding stub's
  bodies enqueue, so a plain function with an actor effect in its list
  already sends today. Case 1 rides on this; only the *mint* is new.
- **The deadlock baseline** [actor-deadlock-cycle]: gate cycles are errors,
  back-pressure cycles warnings, nodes are actor effects. FC-6 must say what
  task bodies contribute.
- **`watch` is per-addr and one-shot** [actor-watch]; a scheduled task has
  no addr — the fault-attribution hole FC-5 exists to close.
- **Tokens are linear and live in collections** [linear-obligation]
  [linear-container]; a token in a `Task` capture is an obligation the
  task discharges or leaks — same rules, no new machinery.
- **Representational fact**: the `Task` arm's natural Rust form is
  `Box<dyn FnOnce(..) + Send>` — one-shot, moved once, `Send`-checked — which
  is *not* the [rs-fn-field] problem (that is shared, many-shot
  `Rc<dyn Fn…>` fields). Lambdas as mint targets stay deferred with
  [fate-lambda].
- **The sugar-pass folded content is tentative** (§0.2). One compatibility
  fact worth recording because it is free: [actor-replyto]'s trailing-token
  convention is the same one EU-6's `-> T` would generate, so nothing here
  pre-empts or blocks that pass in either direction.
- **The spawn-line respelling is the one open phase-5 DECISION** (`capacity`
  folding into a `Placement` on the `on` expression). FC-3's `on` clause and
  FC-5's pool fault sink belong to that same surface and should be decided
  in sight of it.

## 2. What other languages teach

### Erlang / Elixir — processes from arbitrary functions

`erlang:spawn(Fun)` makes a process out of any fun; Elixir's `Task.async(fun)`
is the disciplined form — a fire-and-forget unit whose *result comes back as
a message*, supervised via `Task.Supervisor`. The closest existing relative
of the free `send fn`: concurrency units need not be declared servers, and
results-by-message is the norm, not a workaround.

- Informs FC-1 (the kind exists and is idiomatic at scale) and FC-5
  (`Task.Supervisor`: anonymous work still gets a supervision home —
  theirs is per-task, FC-5's is per-pool).

### Swift / Kotlin — inheritance of execution context

Swift's `Task { }` inherits the enclosing actor context and priority;
`Task.detached` opts out explicitly. Kotlin's coroutine builders launch into
a scope whose dispatcher they inherit unless one is passed. Both ecosystems
independently chose **inherit-by-default, explicit to override** for exactly
FC-3's question.

- Informs FC-3 directly: the default the user chose is the one both
  mainstream structured-concurrency systems converged on.

### GCD (libdispatch) — the explicit-always pole

`dispatch_async(queue, block)` names the queue at every submission; serial
queues are the actor analogue, concurrent queues the pool analogue. Livable,
proven — and noisy enough that Swift's rebuild of the model on top of it
(above) made inheritance the default.

- Informs FC-3 as the rejected pole's best case: explicit-always works, and
  its successor still walked away from it.

### JavaScript / Kotlin `runBlocking` / Rust `block_on` — the main bridge that pumps

JS's main thread is a single-threaded executor: awaiting drains the
microtask queue. Kotlin's `runBlocking` *runs an event loop on the blocked
thread* — the coroutines it hosts execute on the thread that appears blocked.
Rust's current-thread executors (`block_on`) drive queued tasks while
waiting on the future.

- Informs FC-4(a) exactly: three independent designs answer "what does the
  entry thread do while it waits" with "it becomes the pool", and it is the
  established shape of the sync→async bridge.

### The two backends

The `Task` arm is `Box<dyn FnOnce(T) + Send>` scheduled on the existing
scheduler library (Rust) and a lambda submitted to the pool (Kotlin/JVM) —
both are the runtime's native currency; no new machinery beyond one more
work-item kind in a scheduler both backends already share the shape of.

## FC-1. The free `send fn` — the kind divide extended to functions

§0.1 and §0.4. The declaration kind, spelled with the keyword the language
already has:

```
send fn parse_row(out: Reply<User>, row: Row) [Log] {
    out.send(user_from(row))
}
```

Rules, all inherited rather than invented: the actor-member refusal list
applies at the declaration (no kept parameters, no `Mut` parameters, no
`proj` returns, all parameters sendable [actor-sendable]); **no return
type** — send-kind answers nothing, results travel by `Reply` parameters;
the body is ordinary Salvo with an ordinary effect list; invocation is a
**schedule** (a work item on a pool), never a call. What it does *not* have,
and why that is coherent: no mailbox (so no gate — a gate is a mailbox
policy — and no serialization — there is no state to serialize), no addr, no
identity beyond the token that targets it.

- **(a) The `send` marker on free functions** (the user's case 2 — taken as
  direction). One keyword, one refusal list, one invocation semantics,
  shared with members. *Cost:* a function kind the resolver and emitters
  must carry; overloading interaction (may a `send fn` overload a plain
  `fn`? — recommend refusing: the kinds differ in what a call *means*).
- **(b) No marker: any function of the right shape may be targeted.**
  Structural ("takes only sendable params, returns nothing") rather than
  declared. *Cost:* violates the kind-at-the-declaration principle
  [actor-effect-kind] this repository just paid to establish; the refusal
  list fires at use sites far from the author. Rejected by precedent.
- **(c) Lambdas as targets.** The expressive endpoint (ad-hoc continuations
  inline). *Cost:* [fate-lambda] head-on — capture-carrying closures crossing
  a schedule boundary is the exact deferred problem. Deferred with it, not
  decided here.

**Recommendation (user direction, to confirm):** (a), with (c) explicitly
deferred and (b) recorded as rejected.

## FC-2. The mint — `replyto` targeting free `send fn`s

§0.3. The amendment to [actor-replyto], and the whole of case 1:

```
fn fetch_user(id: Int, out: Reply<User>) [Db] {     // ordinary fn: full features
    db.query(id, replyto parse_row(out))            // mint toward a free send fn
}                                                    // returns normally — no park
```

- **Targets**: a `send fn` — a member of the enclosing handler (unchanged) or
  a free `send fn`. Targeting a plain `fn` is refused at the mint: a normal
  function runs by being *called*, and a fulfilled token must never run
  arbitrary synchronous code on the fulfiller's thread inside its activation
  budget (the callback anti-pattern, refused by the kind).
- **Legality**: because a free target needs no enclosing handler, the mint
  becomes legal **in any function** — which deletes [actor-replyto]'s
  "outside a handler the form is an error naming `waitfor`" for free targets
  (self-targets keep the rule: `main` and free functions still have no
  members). The "parking handler may only be `spawn`ed" gate also keeps its
  exact scope: it is about *self*-targeting mints; a `use`-bound handler
  body minting toward a free `send fn` is legal — the answer needs no
  mailbox.
- **Resolution**: a bare `k` resolves lexically against the enclosing
  handler's members first (existing rule), else as an ordinary function
  reference by the language's scope-precedence resolution; an ambiguity is
  refused naming `k@self` and the module-qualified form — the same shape
  [actor-self-send]'s selector family already provides. (Compatible with,
  but not shaped by, EU-7b's tentative lexical-then-effect-list plan.)
- **The token**: `Reply<T>` grows a second internal arm —
  `Mailbox(addr, member, captures)` | `Task(pool, one-shot closure)` — while
  `r.send(v)` stays one uniform surface operation; the fulfiller never
  learns which it holds. `Task` needs **no capacity reservation and no
  deadlock edge at the mint** (no mailbox to fill, no gate to cycle) —
  a strict simplification relative to member mints.
- **Captures**: positional, checked for sendability at the site
  [actor-sendable], captures-then-answer with the trailing-token convention
  unchanged. A linear value in the captures is an obligation the target body
  must discharge — [linear-obligation] does the rest.
- **Direct invocation** (`parse_row(out, row) on p` as a fire-and-forget
  statement) is *derivable* — a mint plus immediate self-discharge — so the
  kernel ships targets-only and the spelling is a separate, deferrable
  decision.

**Recommendation (user direction, to confirm):** as stated; the one genuinely
open sub-question is bare-name resolution order, where the recommendation is
lexical-member-first with ambiguity refused.

## FC-3. Pool selection — inherit by default, `on` to override

§0.5, decided in conversation (user, 2026-09-17) after one full reversal,
recorded so the argument is auditable:

- **Required-`on`-always** was considered and rejected: "explicit" placement
  either threads pool values through every signature on the path (the
  context parameter this design's Q&A had already rejected) or reads the
  ambient pool through a `current_pool()` accessor — *the same ambient read
  the default performs, with ceremony and new vocabulary*. The capacity
  precedent ("explicit and required at spawn — no default") does not
  transfer: capacity has no principled default, placement does —
  **run where the work that created you runs**, which is also what keeps a
  process's continuations on the pool its author budgeted (the same
  locality argument that rejects a dedicated task pool).
- **The rule**: `on POOL` is optional at the mint (and at direct invocation,
  if that ships); omitted, the continuation runs on **the pool current at
  the mint site**, resolved *at mint* and captured into the token — the
  fulfiller's pool is irrelevant; whoever creates work pays for it. Where no
  current pool exists, `on` is required with a diagnostic that says so —
  though FC-4(a), if taken, makes that case nearly vanish.
- **Implementation**: the worker thread knows its pool (its own handle in
  the scheduler library); zero per-call cost, no signature change, nothing
  in the type system.
- Rejected placements, for the record: **callee's pool** (ill-defined — the
  fulfiller is unknown at mint and can differ per token; and it lets clients
  spend a shared service's budget) and **a dedicated task pool** (breaks
  CPU/IO budgeting; a global noisy neighbour).

**Recommendation (user direction, to confirm):** as stated. Swift and Kotlin
(§2) both converged on the same default.

## FC-4. Does `main` run on a pool? — the load-bearing open DECISION

Asked by the user at this document's creation. Strictly, the kernel does
**not** require it — but the pool-less alternative has a failure mode that
must be chosen with open eyes. The scenario: a free function containing an
ambient (no-`on`) mint is called from an actor — works — and the same
function is called from `main` — no current pool exists at the moment the
mint executes. What then?

- **(a) `main` is a pool: the single worker of its own single-thread pool,
  which `waitfor` drives.** The ambient pool then exists on every thread the
  language owns, and the question dissolves: an ambient mint from `main` (or
  from anything `main` calls) lands on the main pool, and its continuations
  run **while `main` waits in `waitfor`** — "blocks main's real thread"
  becomes "drives the main pool until the token is sent to", which is
  `runBlocking` / `block_on` / the JS event loop, an established shape (§2).
  *Consequences, stated rather than discovered:* work queued on the main
  pool runs only during `waitfor` waits and dies at `main`'s return — which
  is not a new rule, it is exactly the paired rule [actor-waitfor] already
  states; a `main` that computes forever without waiting starves its own
  pool (its residents only — other pools run concurrently). *Sub-decision
  riding along:* may actors be **spawned onto the main pool**? Allowing it
  falls out for free and yields genuinely single-threaded cooperative
  programs (attractive for constrained targets); the caveat is the same
  starvation note. *Cost:* `waitfor`'s implementation becomes a pump loop
  instead of a condvar wait — small, runtime-only, both backends.
- **(b) `main` stays pool-less.** Lexically-in-`main` ambient mints are
  compile errors naming `on` (the checker knows `main`'s body). But a mint
  *inside a free function* cannot be checked per-caller without whole-program
  path analysis whose diagnostics appear and disappear with distant callers
  — so the honest form is a **runtime** named error ("mint without `on`
  outside a pool"), the class the existing "all actors idle while `main`
  waits" report belongs to. *Cost:* a function's correctness now depends on
  caller identity — it works from actors and dies from `main` — which is
  precisely the property signatures exist to prevent. This is the argument
  (a) exists to delete.
- **(c) A designated default pool for pool-less mints.** No error, no main
  pool: ambient mints from `main` land on a runtime-chosen pool. *Cost:*
  hidden placement — work runs somewhere no line of the program names.
  Rejected on Locality.

**Recommendation (the user's call):** (a). It is the smallest mechanism that
makes "called from `main`" and "called from an actor" indistinguishable to a
free function — the uniformity this whole design is for — and it upgrades
`waitfor` from a special-cased block into the same bridge every mainstream
runtime ships. Decide the main-pool-actors sub-decision at the same time
(recommend: allow); and note (a) supersedes FC-3's "required where no pool
exists" edge for `main` (it keeps applying to future host/FFI threads).

## FC-5. Fault attribution — the pool fault sink

A scheduled free `send fn` that faults has no addr to `watch`
[actor-watch]. From the 2026-09-17 conversation (the user proposed
watch-on-pools; refined jointly): death-watch and pool faults have different
*shapes* — `watch` is a one-shot linear token for the death of an identity;
a pool has no lifecycle and emits a recurring **stream** — so the pool's
fault handler should be an **addr**, not a watch token:

```
let sink = spawn FaultLogger() capacity 64 on pool(1)
let p    = pool(4, faults: sink)      // every uncaught fault on p → a message
```

- **(a) The sink addr at pool creation** (recommended): recurring faults as
  ordinary messages to an ordinary actor; default (no sink) = a named
  runtime report, sibling to "all actors idle while `main` waits". Catches
  task faults *and* unwatched actor faults — the safety net under
  supervision, which OTP also has. Belongs to the same surface as the
  spawn-line `Placement` DECISION.
- **(b) Attribute a task's fault to its minting actor** (structured
  concurrency): well-defined (every task chain bottoms out at an owning
  activation) but heavier — parent tracking per token — and recoverable
  supervision for anonymous work has no demonstrated customer yet. The
  recorded refinement if (a) proves too coarse for *recovery* rather than
  diagnosis.
- **(c) Silence plus the idle report.** A faulted task's lost obligations
  already surface as gated-waiter hangs the runtime's idle report names.
  Insufficient alone: the fault itself vanishes. Rejected as the only
  mechanism.

Per-addr `watch` is untouched — it remains the supervision pattern's
foundation; the sink is the net beneath it, not its replacement.

## FC-6. What the statics see

[actor-deadlock-cycle]'s nodes are actor effects; its edges come from
handler bodies. Two questions arrive with tasks:

- **Edges from task bodies.** A free `send fn` can send to actors; those
  sends can close back-pressure cycles. The graph can trace task bodies
  whole-program ([call-resolve]: every mint site names its target function
  statically) and attribute a task's sends to every actor whose mints reach
  it — conservative, same coarseness the type-level graph already accepts.
  The alternative — tasks contribute nothing — under-reports a real cycle
  class. Recommend the conservative tracing.
- **The honest gap: obligations parked in tasks.** An actor gated on a
  token that a *task* must discharge has a wait-for edge pointing at no
  effect node — the graph cannot see it. The net underneath is unchanged
  and stated: [linear-obligation] refuses the token being dropped, and the
  runtime idle report names the gated waiter if the task's chain dies.
  Record the gap; do not pretend the graph closes it.

## FC-7. Host interop (adjacent, deferrable)

Recorded from the conversation because the route defines what was undefined,
not because it blocks the kernel: **pure functions** stay plainly callable
from the host (ordinary Rust fns / Kotlin functions). **Effectful functions**
need generated shims — a sync bridge (enqueue, block the calling thread;
legal only on non-pool threads, runtime-checked; this is `waitfor`'s shape
exported) and an async bridge (the host's native future, completed by a
token) — plus **handler wiring at the export**, for which the spawn-site
`use` clause is the existing spelling. Defer the whole section until a host
caller exists; nothing in FC-1…FC-6 forecloses any of it.

## Worked example — the shape end to end

```
send fn parse_row(out: Reply<User>, row: Row) {      // FC-1: the free send fn
    out.send(user_from(row))
}

fn fetch_user(id: Int, out: Reply<User>) [Db] {      // FC-2: ordinary fn mints
    db.query(id, replyto parse_row(out))             // FC-3: inherits this pool
}

actor effect Db { send fn query(id: Int, reply: Reply<Row>) }

fn main() [use, spawn] {
    let p  = pool(2)
    let db = spawn PgDb() capacity 64 on p
    use db                                            // [actor-use-addr]

    let user = waitfor out: Reply<User> {
        fetch_user(7, out)                            // FC-4: legal from main —
    }                                                 //   ambient mint lands on
    ...                                               //   the main pool under (a)
}
```

`fetch_user` is one function, fully synchronous, called identically from
`main`, from any actor, and from another task; the continuation runs where
its creator ran unless an `on` says otherwise; every obligation in flight is
a linear token the checker already tracks.

## Decisions pending

| Label | Question | Status / recommendation |
|---|---|---|
| **FC-1** | The free `send fn` kind and its refusal list | User direction (2026-09-17), to confirm; lambdas-as-targets deferred with [fate-lambda]; refuse send/plain overloading |
| **FC-2** | `replyto` targeting free `send fn`s; the `Task` token arm | User direction, to confirm; open sub-question: bare-name resolution order (recommend lexical-member-first, ambiguity refused) |
| **FC-3** | Pool selection | User direction: optional `on`, default = mint-site pool; edge case owned by FC-4 |
| **FC-4** | **Does `main` run on a pool?** | **OPEN — the load-bearing call.** Recommend (a): main as its own single-thread pool, `waitfor` drives it; sub-decision: actors on the main pool (recommend allow) |
| **FC-5** | Fault attribution for tasks | Recommend the pool fault sink (addr at pool creation), on the spawn-line `Placement` surface; minter-attribution recorded as the refinement |
| **FC-6** | Statics: task edges and the parked-obligation gap | Recommend conservative whole-program tracing; the gap recorded, netted by linearity + the idle report |
| **FC-7** | Host→Salvo bridging | Deferred until a customer exists; shims + export wiring sketched |

Load-bearing order: **FC-4 first** — it decides FC-3's edge case, the
failure story of every ambient mint, and `waitfor`'s semantics; then
**FC-1 + FC-2 together** (the kind and the mint are one surface); FC-3 is
decided modulo FC-4; FC-5/FC-6 land with implementation; FC-7 waits for a
customer. Suggested rule labels when this lands: `[free-send-fn]`,
`[task-mint]`, `[task-pool-inherit]`, `[main-pool]`, `[pool-fault-sink]`.
