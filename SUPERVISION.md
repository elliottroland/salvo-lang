# Supervision and process death — the option space (working document)

Status: **DECIDED** (user, 2026-09-15) — all four recommendations accepted as
written and noted as fitting the broader design: S-1 death = a faulted
activation, no `kill`; S-2 `watch(addr, on_exit: Reply<Exit>)` as the whole
monitor surface; S-3 obligations lost with the corpse, dead-target
sends/fulfils as silent no-ops, the idle-with-parked-gates runtime report;
S-4 supervision as a pattern over `watch` + interception + `spawn`, no
syntax, std handler later. §§0–2 and the section argument trails kept as
written. This was the last design prerequisite: **the phase-5 decision space
ahead of implementation is now empty.** Propagation: decision-log entry in
COMPLETED.md and ROADMAP updated 2026-09-15 (same session); spec rules land
with implementation; this document folds into the log and is deleted per the
charter when the code carries its content.

Written 2026-09-15, bootstrapped in the session that designed phase 5, in the
DESIGN_DOC.md shape. Two questions were handed to this document by the
earlier work and are its charter: **LC-4's opening requirement** (a process
dies while holding parked obligations — LINEARITY_COLLECTIONS.md) and the
**`Reply` definition block's deferred edge** (a dead process's unfulfilled
tokens hang their requesters — CONCURRENCY_EXAMPLES.md, "What a `Reply<T>`
is").

**Propagation owed.** When decided: COMPLETED.md decision-log entry, the
ROADMAP phase-5 plan's step 1 closed, spec rules with implementation, this
document deleted per the charter.

Sources: CONCURRENCY.md ("The direction", "The first pass"),
LINEARITY_COLLECTIONS.md (LC-4), CONCURRENCY_EXAMPLES.md ("What a `Reply<T>`
is"; Example 3's `Shutdown`), LANGUAGE.md ("Effects", "Non-resumption",
"Linear types"), LANGUAGE_SPEC.md ([effect-member-no-effects],
[linear-obligation], [linear-static], [platform-effect]), and the survey
below.

## 0. The stated intent

No fresh user sketch; the intent is assembled from what the decided design
already requires of this document:

1. **Answer LC-4's question**: a process dies holding parked obligations
   (`waiting` lists, `Gather` maps, its own state's handles) — what happens,
   and what does the language promise?
2. **Answer the orphan-token edge**: requesters gated on a reply from a dead
   process hang. What notices, and what recovers?
3. **Cash in the supervisor-as-handler fixed point** (ROADMAP, phase 5;
   demonstrated as interception in CONCURRENCY_EXAMPLES.effects.md
   Example 4): supervision should be a pattern over existing machinery, not
   a construct.
4. **Stay in the kernel's vocabulary**: monitors, exits, and restarts should
   be expressible as sends, tokens, and spawns if at all possible — the
   whole design has profited every time a new need collapsed into the
   kernel.
5. **Graceful stop is already a user-level pattern** (Example 3's `Shutdown`
   member: `stopping` flag, drain, park the completion) and should stay one.

## 1. Fixed points — already decided, inherited here

- **Death by `throw` is impossible by construction.** Effect members may not
  declare effects [effect-member-no-effects], so a `send fn` can never carry
  `[Throw<M>]` — a member body may use `try`/`throw` *internally*, but
  nothing can propagate out of an activation as an exception. There is no
  Salvo-level "uncaught exception kills the process". This dramatically
  narrows what death *is* (S-1).
- **Linearity covers code paths, not crashes** [linear-obligation]
  [linear-static]: the leak diagnostics force drain-on-shutdown wherever the
  code path exists; they cannot speak about a process that stops executing.
  LC-4's guarantee is already worded accordingly: "the process owes until it
  ends" — this document decides what *ends* means.
- **`r.send(v)` is enqueue, never execute**; reply capacity is reserved at
  park time; `fulfil` contributes no deadlock edges. Whatever death does, it
  must not break these.
- **The program ends when `main` returns** (the `waitfor` bridge decision).
  Process death semantics sit *inside* a program lifetime that is already
  defined.
- **Interception across the scheduler works today** (a handler depending on
  the effect it implements, wrapped via `use` shadowing) — the supervisor's
  mechanism exists; only the death *signal* is missing.
- **Platform handlers are the real fault source**: a `platform handler`'s
  host code can fail in ways Salvo cannot see [platform-effect]; generated
  code can hit backend runtime faults (arithmetic, stack overflow) even
  though std's surface answers `None` for bounds. The fault set is small but
  not empty, and it is *backend-shaped*.

## 2. What other languages teach

### Erlang/OTP — links, monitors, and the supervision tree

Two primitives: **links** (bidirectional; a death propagates as an exit
signal that kills the linked process unless it traps exits) and **monitors**
(unidirectional; death arrives as an ordinary `'DOWN'` message carrying a
unique ref and a reason). Modern practice runs almost entirely on monitors —
links exist to build supervisors. A **supervisor** is a process that spawns
children, monitors them, and restarts per a declared strategy
(`one_for_one`, `one_for_all`, `rest_for_one`) under a restart-intensity
budget (max N restarts in T seconds, else the supervisor itself dies —
escalation). The tree shape is the fault-tolerance story: failure is
isolated at the leaf and escalates only when restarting stops helping.

- The monitor's `'DOWN'` message with its one-shot ref is *exactly* a linear
  `Reply<Exit>` — the kernel already has the shape (informs S-2).

### Akka — parental supervision and DeathWatch

Every actor has a parent that *must* decide its failure policy
(resume/restart/stop/escalate); any actor can `watch` any other and receive
`Terminated` as a message. Confirms the two-layer split: watching
(notification, universal) vs. supervising (policy, structural). Akka Typed
moved supervision *into the behavior* (`Behaviors.supervise(...)
.onFailure(restart)`) — policy as an ordinary wrapper, which is precisely
the interception shape.

### Pony — the "death is impossible" pole

Pony behaviours cannot raise: errors are values, partial functions must be
handled at the boundary, and an actor simply cannot crash from within. No
supervision machinery exists because the language removed its cause. Salvo
is *closer to this pole than to Erlang's* — [effect-member-no-effects]
already makes Salvo-level crashes inexpressible — but platform handlers and
backend faults keep Salvo off the pole's absolute point.

### Orleans — reactivation, not restart

A grain that fails is simply re-activated on the next call; state is
whatever was persisted. No tree, no policy — liveness by reincarnation.
Interesting as the zero-ceremony pole, but it presumes a persistence layer
Salvo does not have and hides the failure from the caller, which is against
the house grain.

### Rust / Kotlin — what the backends give the scheduler

Rust: a panic unwinds the thread; `catch_unwind` at the activation boundary
converts it to a value; `JoinHandle` reports it. Kotlin: exceptions at the
activation boundary are caught the same way; structured concurrency's
`supervisorScope` isolates child failure. **Both backends can cheaply give
the scheduler "this activation died, with a reason string"** — the
per-activation catch is the one primitive the runtime layer must implement,
identically shaped on both.

## S-1. What is death?

The fault inventory, given §1's fixed points: (i) **backend runtime faults**
in an activation (arithmetic, stack overflow, a bug in generated code);
(ii) **platform-handler failures** (host exceptions crossing back into
generated code); (iii) **explicit self-stop** (a graceful, user-level
pattern already — Example 3); (iv) **external kill** (does not exist; would
have to be added). There is no (v): Salvo-level uncaught throws cannot
happen.

- **(a) Fault-only death, first pass.** An activation that faults kills its
  process: the scheduler's per-activation catch (Rust `catch_unwind`, Kotlin
  `try` at the dispatch site) marks the process dead with a reason. No
  `kill`; graceful stop stays a pattern. *Cost:* nothing new in the surface;
  one catch in the runtime layer. *Consequence:* death is always *abnormal*
  — a Salvo program with no faults and no platform handlers cannot
  experience it, which keeps the common case clean.
- **(b) Add `kill(addr)`.** *Benefit:* supervisors can terminate misbehaving
  children. *Cost:* preemption — the kernel's serialization promise means a
  kill can only land *between* activations, so a looping activation is
  unkillable anyway; and a kill is an implicit mass-drop of the child's
  obligations, the very thing linearity exists to make loud.
- **(c) Death also on activation deadline** (a watchdog per activation).
  *Cost:* timers in the scheduler's hot path and a semantic cliff (was it
  slow or dead?); defer.

**Recommendation (the user's call):** (a). Death = a faulted activation,
detected at the dispatch boundary, reason recorded. No `kill` in the first
pass — a supervisor that wants a child gone asks it to stop (the `Shutdown`
pattern) and stops *forwarding* to it either way (interception makes the
child unreachable, which is the useful half of kill without the drop).

## S-2. The monitor surface

- **(a) `watch` as a kernel-shaped member: death is a one-shot token.**
  The spawn effect gains one member: `send fn watch(p: Addr<…>, on_exit:
  Reply<Exit>)` — `Exit` a plain struct (reason, and whether it was a fault
  or `main`-end cleanup). The notification is `on_exit.send(exit)` by the
  scheduler when the process dies. Erlang's `'DOWN'`+ref, as a linear token:
  one-shot by construction, the *watcher's* obligation machinery forces
  handling it (a dropped `watch` registration is a leak diagnostic — you
  cannot silently forget you were watching). Watching an already-dead addr
  answers immediately. *Cost:* the scheduler keeps per-process watcher
  lists; the `Exit` token is state the corpse must not orphan (the runtime,
  not the process, owns fulfilling it — so it survives the death it
  reports).
- **(b) Links (bidirectional death propagation) as a primitive.** *Cost:* a
  second primitive, exit-trapping semantics, and a kill mechanism by the
  back door (a linked death kills you). Erlang itself mostly builds on
  monitors now. Defer; if wanted later, a link is two watches plus a policy
  handler.
- **(c) No monitors; supervisors poll.** Rejected on arrival — polling in a
  message-passing design is the tell that a primitive is missing.

**Recommendation (the user's call):** (a), and it is the whole surface — one
member, one struct, the token machinery doing the enforcement. Note the
pleasing closure: the death notification is *itself* a `Reply`, so the
kernel's four pieces still suffice; nothing new enters the model.

## S-3. The corpse: obligations, queue, and stale tokens

What death does with what the process held. The honest baseline, stated
rather than discovered:

- **Parked obligations and state are lost.** Statically-checked linearity
  cannot survive a crash [linear-static]; the language's promise weakens
  exactly as LC-4 worded it, and the *monitor is the recovery mechanism* —
  the watcher learns, and whatever redelivery/reopen/compensation applies is
  its logic (the NoticeFetcher's DDB redelivery is the worked example: the
  notices reappear server-side; the supervisor respawns; the world heals at
  the domain level, not the language level).
- **The queue is dropped; sends to a dead addr are silent no-ops** (Erlang's
  rule, and the only composable one — a send that could error on a dead
  target would make *every* send fallible and every sender death-aware;
  monitoring is the opt-in for callers who care).
- **`r.send(v)` to a token whose *target* died: also a no-op** — same rule,
  same reason (fulfil is a send).
- **Requesters gated on a dead callee** would hang silently — the one place
  a no-op is not acceptable. Two mitigations, complementary: the runtime
  knows a gated process's awaited token died with its holder **if** token
  residency is traceable (it is not, cheaply — tokens are ordinary values);
  so instead: **(i)** the scheduler detects *global* idle-with-parked-gates
  — all queues empty, no activation running, ≥1 process (or `main`) gated —
  and reports it as a runtime deadlock/orphan error naming the parked
  processes (quiescence detection reused as a *diagnostic*, not as
  termination semantics); **(ii)** anything that must survive its callee's
  death watches it (S-2) — the supervisor pattern's whole point.

**Recommendation (the user's call):** all four bullets as stated, with (i)'s
idle-with-parked-gates report built into the first-pass scheduler — it is
cheap (the scheduler already knows), it converts the silent hang into a
named error, and it catches orphaned `waitfor` in `main` for free.

## S-4. Supervision itself

- **(a) A pattern, not a construct** — and std ships it *later*. A
  supervisor is an interceptor (Example 4's shape) plus `watch` plus
  `spawn`: it holds the child's addr privately, forwards, and on `Exit`
  respawns and re-forwards — clients hold the *supervisor's* addr and never
  observe the death (the Erlang registered-name indirection, obtained
  structurally). Restart strategies, intensity budgets, escalation: handler
  logic, written in Salvo, no language surface. First pass ships `watch`
  and the pattern documented; a std `Supervisor` handler waits for real
  usage to shape it.
- **(b) A language-level supervision tree** (declared child specs,
  strategies as syntax). Everything OTP learned says the *mechanism* is
  small and the *policy* is a library; baking policy into syntax is the
  wrong layer, and Akka Typed's drift from parental-magic toward
  wrapper-supervision corroborates.

**Recommendation (the user's call):** (a). The first pass needs exactly one
new thing total across S-1–S-4: the `watch` member and its `Exit` token;
everything else is scheduler behaviour and documentation.

## Decisions pending

| # | Question | Recommendation (user's call) |
|---|---|---|
| S-1 | What is death | Faulted activation only, caught at dispatch; no `kill` (ask-to-stop + stop-forwarding covers it) |
| S-2 | The monitor surface | `watch(addr, on_exit: Reply<Exit>)` — one member, one struct; death notification is itself a linear token |
| S-3 | The corpse | Obligations lost (monitor = recovery); dead-target sends and fulfils are silent no-ops; idle-with-parked-gates runtime report |
| S-4 | Supervision | A pattern over `watch` + interception + `spawn`; std handler later; no syntax |

Load-bearing order: **S-1 first** (everything else names "death"), then
**S-2/S-3 together** (the token and the no-op rule interlock), with **S-4**
falling out. The scheduler work all sits in the backend runtime files per
the placement decision — nothing here appears to need compiler cooperation
beyond the `watch` member's std declaration, and anything that turns out to
is flagged per the user's standing instruction. See DESIGN_DOC.md for the
shape this document follows.
