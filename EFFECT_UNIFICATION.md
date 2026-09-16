# Effect unification — every member a `send fn` (working document)

Status: **DECIDED** (user, 2026-09-15, rounds 4–6) — see "The decisions" at
the end of the document for the decided list and what remains as
propagation. The trail: round 1 proposed a full sync/async unification,
**rejected** ("not a promising way to go"); round 2 surfaced the divide at
the effect declaration (`actor effect`); round 3 generalized the mint and
dissolved the running-in entry; round 4 decided EU-5, EU-6, and EU-7b;
round 5 settled the remaining details and revised the self-directed
spelling to **`@self`** (the selector form) in place of the in-flight
`self.m`; round 6 renamed `Pid<E>` → **`Addr`** (the token's name kept clear
of OS process IDs). **Round 7 — the spawn-line respelling — is OPEN**
(options and a recommendation under "Round 7"). Superseded sections are
kept for the argument trail, each with a pointer.

Written 2026-09-15 by a read-only session, while another session implements
the phase-5 first pass (the all-`send` explicit surface). This document is the
only file this session may write. It follows DESIGN_DOC.md's shape: stated
intent, fixed points, what other languages teach, the proposal stated
precisely, benefits and costs, then numbered decision sections (`EU-`) with
options, trade-offs, and recommendations.

**Propagation owed (this session is read-only outside this file):** the
decisions below have propagated **nowhere** yet — and the first-pass
session has meanwhile **landed `self.m`** (and built [actor-send-fn] in its
per-member shape), so two of the outcomes are now changes to shipped
surface rather than coordination items. That is an ordinary outcome under
the no-backwards-compatibility invariant (AGENTS.md): the old spelling
simply stops being the language, and every affected site is rewritten in
the same change. Everything owed is gathered as **"The plan"** at the end
of this document: the `self.m` → `@self` respelling and the marker
relocation first, the sugar-pass items with their pass, and the
COMPLETED.md / CONCURRENCY.md / spec propagation — after which this
document is deleted, per the working-document charter.

Sources: CONCURRENCY.md ("The direction", "The first pass", "Carried to the
call-sugar pass", C-9, C-10), CONCURRENCY_EXAMPLES.effects.md ("The kernel,
and the sugar tower", Example 6), CONCURRENCY_EXAMPLES.md ("What a `Reply<T>`
is"), LANGUAGE.md ("Effects", "Handlers with dependencies", "Implicit
parameters", "Throwing"), LANGUAGE_SPEC.md ([actor-send-fn], [actor-use-addr],
[actor-types], [actor-replyto], [actor-waitfor], [effect-decl],
[deduce-syntax], [decl-explicit], [effect-not-a-type], [implicit-fn-only],
[linear-obligation], [backend-never-wrong]), BACKEND_SPEC.rust.md
([rs-effects], [rs-effect-fusion], [rs-throw-controlflow], [rs-fn-field]),
and the user's stated intent (below).

## 0. The stated intent (user, 2026-09-15)

Restated and numbered so the analysis can be judged against it. The user asked
for benefits and costs, not a decision — the sketch guides the options.

1. **Every effect member is a `send fn`**, and the normal function forms are
   syntactic sugar over it:

   ```
   fn say_hello(name: Str) -> Str
   // equivalent to:
   send fn say_hello(name: Str, ?reply: Reply<Str>)

   fn do_something()
   // equivalent to:
   send fn do_something(?reply: Reply<None>)

   // no sync-sugar equivalent:
   send fn send_something()

   // no sync-sugar equivalent:
   send fn multiple_replies(value: Str, ?reply1: Reply<Str>, ?reply2: Reply<Int>)
   ```

   Of the forms with no sugar equivalent, only the multi-reply one cannot be
   called from synchronous code without bridging.

2. **The sync/async difference lies with the construction and invocation, not
   the handler.** Every handler is written one way. In a synchronous context,
   the `Reply<T>` passed to the member is a *different implementation* that
   requires synchronous evaluation; in an asynchronous context it mints as the
   phase-5 design has it.

3. **The question:** what are the benefits and costs of this approach?

Where this meets the record: point 1's first half **is Collapse 1** of the
decided kernel (a member's `-> T` is an implicit trailing `Reply<T>` — the
sugar-tower row), currently scheduled as a *later sugar pass* and scoped to
process protocols. The new content is making the collapse **retroactive over
the entire effect chapter** — today's `Console`, `Fs`, `Random<T>` included —
plus point 2's caller-side story. Point 2 is the strongest possible answer to
the **named question carried to the call-sugar pass** ("may an ordinary
member be process-backed?"): not merely *may* — an ordinary member *is* the
send form, so process-backing is the general case and synchronous invocation
the specialization.

## 0b. Round 2 — the proposal rejected; the divide surfaced (user, 2026-09-15)

The user read round 1 and turned the direction around. Three thoughts,
restated and numbered so the new sections can be judged against them:

1. **Hiding the sync/async difference is a mistake.** Sync effects can keep
   arguments; actor effects cannot. That is something the user of the
   language needs to reckon with *when deciding which effects to use* — a
   design-time choice, not a diagnosed-at-the-binding surprise. (Round 1's
   EU-3 classification, accepted as fact — but promoted to surfaced language
   law rather than erased by a unification.)
2. **An effect with both sync and async members is not reasonable**: the same
   handler state would be reachable from two contexts/threads. The one case
   that might work — sync members that only *read* the state — is of doubtful
   usefulness; and it is worth adding that it is also unsound without
   synchronization (a synchronous reader on another thread can observe state
   mid-activation, which is precisely what scheduler serialization exists to
   prevent), which strengthens the conclusion. Therefore the kind is declared
   **on the effect** — `actor effect` versus plain `effect` — governing how
   its members are interpreted and limited. This is EU-5.
3. **The open question:** a function's effect list says which actor effect
   members are *available to call*. Could it also specify which actor effect
   the function is **running in** — licensing `replyto` / `replyto!` for that
   effect's members in functions outside the handler body, and, once effect
   polymorphism wires effect lists through to lambdas, in lambdas too? This
   is EU-7.

What this settles and opens: EU-1 is rejected in its (a) form — the outcome
is (b)-shaped but stronger, with the kind made *explicit* rather than left
emergent; EU-2 is mooted (no unified sync `Reply` exists; the
[actor-use-addr] forwarding stub remains the async mechanism); EU-3 transforms
into a declaration rule (the async-effect restriction list, EU-5); EU-4's
context rule stands for the async side, with §0b.3 as a signature-carried
extension of where the explicit continuation forms reach (EU-7). One round-1
finding survives intact and becomes load-bearing: the sync-only feature list
(keep deductions, `Mut` parameters, `proj` returns, non-sendable payloads) is
now the *content* of what `actor effect` refuses.

## 1. Fixed points — already decided, inherited here

- **Collapse 1 already exists, as sugar, on the process side.** "A member's
  `-> T` is an implicit trailing `reply: Reply<T>` parameter — the only
  primitive member kind is `tell`" (the kernel; CONCURRENCY_EXAMPLES.effects.md).
  The first pass deliberately ships without it, and [actor-send-fn] reserves
  the unmarked `fn` member spelling for the later call-member sugar. §0.1 asks
  to promote this sugar to the *definition* of every effect member.
- **The named question is already open, with three sketched options**
  (CONCURRENCY.md, "Carried to the call-sugar pass"): allow an ordinary member
  to be process-backed silently; allow with the distinction carried at the
  *binding site* only; or forbid. §0 is a fourth stance that subsumes the
  first two: the ordinary member is *defined as* process-shaped, and the
  binding site carries the distinction by construction.
- **`fulfil` is enqueue, unconditionally** ("What a `Reply<T>` is", rule 2;
  Example 6's first honesty note). The invariant it protects: no user code
  runs on the fulfiller's stack, so per-process serialization holds. §0.2's
  synchronous `Reply` — discharge delivers the value to the caller *now* —
  collides with the unconditional form of this rule. EU-2 resolves the
  collision (the invariant survives restated; the sync token has no queue and
  no process to protect).
- **Member deductions are the contract, and they encode borrows**
  [deduce-syntax] [decl-explicit]. A member's written clause decides parameter
  modes in generated code: kept plain → `&T`, kept `Mut` → `&mut T`, consumed
  → owned. And a **send payload always crosses the seam and so is always
  consumed** ([actor-send-fn]'s deduction bullet). Collision: a member that
  *keeps* a parameter (`fn println(message: Str) -> None => message`) has no
  faithful send reading — this is EU-3's subject, and it is the largest single
  cost of §0.1.
- **The synchronous lowering is the performance baseline** [rs-effects]
  [rs-effect-fusion] [kt-effect-fusion]. An effect member call today is one
  virtual call through `&mut dyn E` with borrowed arguments, zero allocation.
  Whatever the unification means, `println` must keep compiling to that
  ([backend-never-wrong]'s cousin: never emit silently *slow* code for the
  program that asked for nothing new). EU-2's erasure option is what pays
  this.
- **C-4 is decided as the structural rule**: a sent or captured value may not
  transitively hold a non-sendable field. It applies to send payloads, reply
  captures, and tokens — and therefore bounds which members can have an async
  reading (EU-3).
- **Run-to-completion, seams, and the bridge are decided.** A handler
  activation cannot block; parking (the seam transform) exists only in handler
  member bodies; `waitfor` is main's explicit blocking bridge, and **main's
  only token source is `waitfor`** ([actor-waitfor]; CONCURRENCY.md "The first
  pass"). No colouring, ever (user decision 2026-09-04; C-7 dissolved). These
  jointly produce the *context residue* of EU-4: §0.2's "the caller
  determines" is bounded by what the calling frame can afford.
- **`use pid` is a generated forwarding stub** [actor-use-addr] — a binding of
  an effect to stub member bodies that talk to the `Pid`. This is the
  implementation shape that makes a caller-side sync/async split feasible
  without touching call sites: the *stub* is where a different `Reply`
  strategy would live.
- **Effects are not values; handlers are not values; `use` is singular**
  [effect-not-a-type]. The unification must not disturb this — and does not:
  it changes what a member *means*, not what an effect *is*.
- **Implicit parameters must be function-typed** [implicit-fn-only]. §0.1's
  `?reply: Reply<Str>` spelling collides: `Reply<T>` is a data type, not a fn
  type, and implicits resolve *by name to a function*. The kernel's own
  resolution — the implicit trailing token is its own rule (call syntax legal
  only for members with a single trailing token), not a `?` parameter — avoids
  the collision and is assumed hereafter. Noted once; not load-bearing.
- **Throw is special today and stays interesting here.** Non-resumption emits
  no trait at all [rs-throw-controlflow]; `throw` returns `Nothing`. Under the
  unification `throw` reads as a member holding a `Reply<Nothing>` — a token
  that *can never be sent to*, since no value of `Nothing` exists — which is a
  pleasingly exact account of "never comes back", but the unwind machinery
  (`try`/`Thrown`) is orthogonal and untouched. Flagged in the costs; no
  decision needed now.
- **Example 6's binding swap is this unification's decided kernel.** For
  all-`send` protocols, one handler already works under both bindings, with
  the swap changing timing tightness, not shape, and the deadlock graph keyed
  on bindings, not bodies. §0 asks to extend the swap from all-`send`
  protocols to call-shaped ones.

## 2. What other languages teach

### ABS / Creol — active objects: the caller literally decides

The closest precedent for §0.2. In ABS an interface method is declared once;
the **caller** picks the invocation: `o.m(e)` is a synchronous call, `o!m(e)`
is an asynchronous send yielding a future the caller may `await` or `get`.
One declaration, two invocation forms, caller's choice — §0's shape exactly.
Their accumulated finding: synchronous *cross-object* calls reintroduce the
blocking hazards the model exists to remove (a sync call into another active
object holds the caller's cog), so the discipline is async-by-default across
boundaries with sync reserved for self/local calls. Informs EU-4 directly:
even where "caller decides" is the surface, *which context may decide which
way* ends up ruled.

### Erlang / Gleam — the sync form as a library over send+reply

`gen_server:call` is not a primitive: it is a client wrapper that mints a
reference, sends `{From = {pid, ref}, Request}`, and selectively receives the
tagged reply — §0.2's "different `Reply` implementation in a sync context",
existing as *convention* rather than language. Gleam's `process.call` is the
same wrapper, typed: mint a `Subject`, send it inside the request, receive on
it with a timeout. Proof that a synchronous surface over an asynchronous
kernel is buildable and pleasant. The caveat that transfers: both *block the
calling process* — cheap there because processes are green and blocking one
is free. Salvo's run-to-completion handlers have no thread to block; the
equivalent of "block this process" is the park/gate, which exists only at
seams in handler bodies. Informs EU-4's context rule.

### Pony — the counter-pole: the split marked at the declaration

Pony marks the divide where §0 refuses to: `fun` is synchronous, `be`
(behaviour) is asynchronous, chosen by the *declarer*, and behaviours cannot
return values — request/reply is hand-built from promises. The cost is
visible: protocol authors choose for every caller, and one capability often
ships as two declarations (a sync `fun` for local use, a `be` wrapping it).
§0 is the anti-Pony, and Pony is the evidence for why one might want to be.

### Kotlin / C# — what happens when "caller decides" meets deep call stacks

Kotlin's `suspend` is caller-flexibility bought with colouring: any function
that *might* park marks itself, transitively — rejected outright for Salvo
(2026-09-04). The bridge at the boundary is `runBlocking` — block a real
thread — which is exactly `waitfor`. The instructive failure is C#'s
sync-over-async (`.Result`, `.Wait()`): letting arbitrary synchronous frames
block on asynchronous work at *runtime* discretion produced a decade of
deadlock literature (blocked pool threads waiting on continuations scheduled
onto the same starved pool). Informs EU-4's central claim: the context rule
must be **static**. A unified surface whose "sync invocation" can be reached
from a pool thread is the C# bug reintroduced.

### Rust — one token, two consumption modes

`tokio::oneshot::Receiver<T>` — already the recorded model for `Reply`'s
backend shape — offers both `blocking_recv()` (sync context) and `.await`
(async context) on **one token type**: the mode is the *consumer's* choice,
checked by context (calling `blocking_recv` inside a runtime panics — a
dynamic check Salvo can make static). Direct precedent for §0.2's "different
implementation": it is not two token types, it is two consumption disciplines
over one token. Informs EU-2.

## 3. The proposal, stated precisely

*Superseded: the §0 proposal was rejected in round 2 (§0b). Kept for the
argument trail. The §3 table's ack/no-ack distinction (`Reply<None>` versus
no reply at all) survives, relocated inside the async kind — see EU-6.*

The correspondence, in the decided vocabulary (implicit trailing token by
rule, not `?`-parameter — see §1 on [implicit-fn-only]):

| Surface form | Ground form | Sync reading | Async reading |
|---|---|---|---|
| `fn m(a: A) -> T` | `send fn m(a: A, reply: Reply<T>)`, fulfil at every return | call, value returned | call syntax: auto-mint + gate (park), per the tower |
| `fn m(a: A)` | `send fn m(a: A, reply: Reply<None>)`, fulfilled at return | call, returns when done | park until **acknowledged** |
| `send fn m(a: A)` | itself (primitive) | — (no completion to wait for) | enqueue, fire-and-forget |
| `send fn m(a, r1: Reply<X>, r2: Reply<Y>)` | itself (primitive) | — (bridging only) | explicit mints |

Two readings of "ground form", and the whole of EU-1/EU-2 is choosing
between them:

- **Semantic grounding (erasure).** The send form is what the member *means*;
  the compiler is licensed to specialize. A synchronously-bound call never
  materializes a token: the "sync `Reply`" is the native return path — the
  callee's `return v` *is* the discharge, the caller's binding of the result
  *is* the receipt. Zero-cost by construction; the token exists at sync call
  sites only in the spec's account of them.
- **Representational grounding (reification).** Every call really constructs
  a `Reply` value, and sync/async are two implementations behind it. Uniform,
  and uniformly paid for: every `println` allocates or at best branches.

Note what the second row surfaces: the unification gives `fn do_something()`
versus `send fn send_something()` a **principled semantic difference** —
identical argument lists, but the former carries a completion contract
(`Reply<None>` = "tell me when it is done") and the latter none. Today that
distinction exists only across the sync/async divide; unified, it is a real
axis *within* one surface, visible in every declaration.

And what the binding side means. A protocol has one handler text; the
**binding** decides execution:

| Binding | Member body runs | `-> T` call means | Decided already? |
|---|---|---|---|
| `use H(args)` (local) | inline, caller's stack | plain call, value back now | today's semantics |
| `spawn H(args)` + `use pid` | as activations, serialized | park (in a handler) / block (in main, via the bridge) | Example 6 for all-`send`; the named question for `-> T` |

## 4. What it buys

*Superseded with §0 (§0b). Of the seven benefits: 2 (the named question) is
delivered differently by EU-5; 1 survives only within the async kind
(Example 6's swap — a local test binding of an async-effect handler); 3's
spec economy is partially kept by EU-6 (Collapse 1 scoped to actor effects);
5, 6, 7 lapse or defer. Kept for the argument trail.*

1. **One handler form; the binding decides.** Example 6's swap — production
   process, test-local handler; local stats promoted to a shared hub —
   currently works only for protocols written in the explicit all-`send`
   shape. Unified, it works for call-shaped protocols *as naturally written*:
   `ScriptedDb` implements `fn fetch_batch(count: Int) -> List<Notice>` with
   a plain `return`, and the same text backs a spawned process. The
   MemFs-for-DefaultFs test story extends across the scheduler boundary
   without anyone writing `Reply` plumbing for the common case.
2. **The named question dissolves rather than being answered.** "May an
   ordinary member be process-backed?" assumed ordinary members and send
   members are different kinds. If the ordinary member *is* a send member,
   the question inverts into EU-3's real content: which members have a
   *synchronous* reading — statically visible in the declaration.
3. **Spec economy.** One member kind, one call semantics, one meaning for
   `-> T` everywhere. Collapse 1 stops being a sugar-tower row that applies
   in one chapter and becomes the definition both chapters share. The effect
   chapter's opening story ("interfaces passed around") survives untouched as
   the erased specialization — a reader meets `Reply` only when they meet
   processes, the same way `T?` readers meet `T | None` only when they care.
4. **No colouring, structurally.** Sync/async is a property of a *binding*,
   never of a function type or a call site. Function values with effect lists,
   higher-order effect inheritance, interception — all unchanged. This honors
   the 2026-09-04 record more thoroughly than the status quo, which keeps two
   member kinds whose difference a reader must track.
5. **`waitfor` stops being a special form and becomes the desugaring.**
   `let x = get_user(7)` in main = mint + tell + block = exactly `waitfor
   out: Reply<T> { users.get_user(7, out) }`. One bridge mechanism, spelled
   automatically in the common case, still writable explicitly for the
   multi-token cases. (Example 1 and Example 5 of the effects file already
   *assume* this — `main` does `let u = get_user(7)` and "parks until
   gathered" — so the direction's own examples want it.)
6. **Interception and supervision become definitionally uniform.** Example
   4's `RetryOnce` — a synchronous policy handler wrapping a process — is
   today a pleasant surprise; unified, it is the null case: every handler in
   a `use` chain is the same kind of thing, and whether a link in the chain
   crosses the scheduler is per-binding. The deadlock analysis stays keyed on
   bindings, not bodies — already the decided property, now total.
7. **An interop dividend, later.** A `platform handler` is a host
   implementation of an ordinary effect. If ordinary members are ground-send,
   a host implementation may be *asynchronous* — a callback-shaped host API
   (JVM NIO, a JS-style completion handler) fulfils the `Reply` when the host
   calls back — without changing the Salvo-side declaration or its callers.
   Not first-pass, not designed here; named because it falls out of the
   grounding and nothing else offers it.

## 5. What it costs — and where the divide survives

*Superseded with §0 (§0b) — and vindicated by it: §5.2's classification is
the reason round 2 surfaces the divide instead of hiding it, and its
sync-only feature list becomes EU-5's refusal list verbatim. §5.1's context
residue survives as the boundary stated in EU-7. Kept for the argument
trail.*

The honest headline: **the unification relocates the sync/async divide; it
does not delete it.** The user's conjecture — the difference lies with
construction and invocation, not the handler — is *almost* exactly right, and
the residue is precisely characterizable. Two places the divide survives, one
invariant that must be restated, and three smaller costs:

1. **The context residue (EU-4).** "The caller determines" holds at the
   *binding*, but what a process-backed `-> T` call can *do* depends on the
   calling frame: in a handler member it parks (the seam machinery exists
   there); on main's real thread it blocks (the bridge); in an ordinary
   function reached from a handler activation it can do **neither** — parking
   mid-arbitrary-stack is the multi-frame continuation capture both backends
   lack in parity, and blocking is the C# pool-wedge. A uniform surface makes
   an illegal-in-context call *look* fine until the checker says otherwise;
   the diagnostics carry what the syntax no longer does. This is bounded and
   static (EU-4's rule), but it is real, and it is the part of §0.2 that
   cannot be engineered away without colouring or a runtime.
2. **The member classification (EU-3).** Not every sync member has an async
   reading. A send payload is always consumed and must be sendable [C-4], a
   reply value crosses a boundary, and an activation shares no stack with its
   caller. So members that **keep** parameters (`=> message` — a borrow in
   generated code), take **`Mut` parameters** (in-place mutation of the
   caller's value: `read_to(s, buf: Mut Bytes)`), return **projections**
   (`proj[from: …]` — the result borrows the argument), traffic in
   **non-sendable payloads** [rs-fn-field], or return **`Nothing`** (throw)
   have no process reading. Audit note: **today's std effects live almost
   entirely in this class** — `Console.println` keeps its message, `Fs` is
   built on `Mut` buffers and linear stream tokens with keep-deductions. So
   "every handler can be a process" is false for the effects that exist; the
   unification's value is *prospective* (protocols designed inside the
   both-ways core) plus *definitional* (one meaning), not a retrofit of std.
   The mitigation is that the classification is **decidable from the
   declaration alone** — the deduction clause is the contract [deduce-syntax]
   — so it can be surfaced precisely (EU-3).
3. **The invariant carve-out.** "Fulfil is enqueue, unconditionally" becomes
   conditional on token kind — a sync discharge delivers on the caller's
   stack. The invariant survives restated: **a discharge never runs another
   process's code on the discharger's stack.** The sync token belongs to no
   process (its holder and its awaiter are the same frame), so there is no
   serialization to protect; Example 6's causal-order note is unaffected
   because a *local binding inside a process* still uses real (enqueueing)
   tokens — the erased token exists only where no process is involved at
   all. This must be worded carefully in the spec, but nothing breaks.
4. **A zero-cost proof obligation, permanently.** The erasure (EU-2(a)) is a
   compiler promise: synchronously-bound calls compile to exactly today's
   output — trait call, borrowed args where the contract keeps them, plain
   return. Every future change to the member lowering carries this
   obligation. (The reification alternative pays runtime cost on the most
   common operation in the language instead; §5.4 is the cheaper debt.)
5. **Throw stays a carve-out.** `Reply<Nothing>` gives the unification a
   true *account* of non-resumption but not an implementation of it: unwind
   to the `try` delimiter is control flow [rs-throw-controlflow], not token
   traffic, and a process-backed `Throw` means "the activation faults"
   (SUPERVISION.md's territory), not "a reply never comes". One effect
   remains special; the unification changes its *story*, not its machinery.
6. **Churn timing.** The first pass is in flight and deliberately all-send
   explicit. Adopting the unification *as semantics* costs the in-flight work
   nothing (it is the call-sugar pass's design, made earlier and larger), but
   respelling LANGUAGE.md's effect chapter around the grounding is real spec
   work, and doing it before the sugar pass exists risks documenting forms
   the checker refuses ([backend-never-wrong]'s spirit: the spec should not
   run ahead of the diagnostics).

## EU-1. The ground truth: definition, sugar, or correspondence

*Settled by round 2 (§0b): (a) rejected — the divide is surfaced at the
declaration (EU-5). The outcome is (b)-shaped but stronger: the kind is
explicit, not emergent. Kept for the argument trail.*

The load-bearing decision; the others assume its answer.

- **(a) Unification as definition.** Every effect member *is* a send member;
  `fn m(a) -> T` is defined as `send fn m(a, reply: Reply<T>)` with
  fulfil-at-every-return; the synchronous surface is the erased
  specialization (EU-2(a)). *Benefit:* §4 in full — one member kind, the
  binding swap for call-shaped protocols, the named question dissolved.
  *Cost:* §5 in full — the context rule must be designed (EU-4), the
  classification surfaced (EU-3), the spec re-grounded.
- **(b) The status quo trajectory.** Collapse 1 stays scoped to process
  protocols; ordinary effects stay primitively synchronous; the named
  question gets one of its three original answers at the sugar pass. *Cost:*
  two member kinds forever, with a correspondence the examples already lean
  on but the language never states; the Example 6 swap stays bounded at
  all-`send` protocols; `waitfor` stays a special form. *Benefit:* no spec
  churn, no new checker obligations, the sync world stays exactly as simple
  as it is.
- **(c) Shape-changing middle: correspondence without identity.** Sync
  members stay primitive, but every sync member declares a *canonical send
  form* the compiler derives, and bridging stubs are generated in both
  directions (sync→process: mint/park-or-block; process→sync: a generated
  wrapper handler that calls and fulfils). *Benefit:* the swap works without
  re-grounding the spec. *Cost:* two kinds *plus* a generated correspondence
  — more total machinery than (a), with the model of neither; the bridging
  rules of EU-3/EU-4 are all still needed, just described twice.

**Recommendation (the user's call):** (a), adopted as the *semantics of the
call-sugar pass* rather than as new work — it is Collapse 1 given the whole
language instead of one chapter, and (c) demonstrates that the hard parts
(EU-3, EU-4) are owed under any option that delivers the swap, so they are
not a reason to prefer (b). Sequence it after the first pass lands (§5.6).

## EU-2. What the synchronous `Reply` is

*Mooted by round 2 (§0b): with the unification rejected there is no
synchronous `Reply` to define; the [actor-use-addr] forwarding stub remains
the async-side mechanism. Kept for the argument trail.*

§0.2 proposes "a different implementation that requires synchronous
evaluation". Three shapes:

- **(a) Erased.** No runtime token exists at synchronously-bound call sites;
  the model's `Reply` is the return path itself. The discharge rule
  "fulfil-at-every-return" is exactly today's `return`; linearity is exactly
  "every path returns a value" — both already checked. *Cost:* the invariant
  rewording (§5.3) and the permanent zero-cost obligation (§5.4). *Benefit:*
  `println` compiles as today, to the instruction.
- **(b) Reified, two implementations.** Every call constructs a token;
  sync tokens deliver-in-place, async tokens enqueue — Rust's oneshot with
  `blocking_recv` vs `.await`, made a value. *Benefit:* one uniform runtime
  story, simplest to specify. *Cost:* allocation/branching on every effect
  call in the language — a whole-program tax to unify a path the compiler
  can specialize statically; violates "pay for what you use".
- **(c) Stub-mediated (where the design already is).** Tokens materialize
  only in the [actor-use-addr] forwarding stub: a Pid-backed binding generates
  member bodies that mint/park (handler context) or mint/block (main), while
  local bindings keep the native convention. *Observation:* (c) is (a) seen
  from the implementation side — the "different implementation" of §0.2 is
  the *stub*, not the token.

**Recommendation (the user's call):** (a)+(c), which are one design: erasure
as the semantic account, the forwarding stub as its mechanism. The user's
instinct that the difference lives in "how it is constructed and invoked" is
this option — with the refinement that in the sync case the `Reply` is not a
different *object* but the absence of one.

## EU-3. The classification: which members cross, and where it is checked

*Transformed by round 2 (§0b.1): the classification stops being a diagnosis
at binding sites and becomes declaration-level law — the async-effect
marker's refusal list (EU-5). Kept for the argument trail.*

From §5.2: members partition into **both-ways** (payloads consumed and
sendable; a single trailing reply; result owned), **sync-only** (keep
deductions, `Mut` parameters, `proj` returns, non-sendable payloads,
`Nothing`), and **async-only** (reply-less sends, multi-token members). The
partition is decidable from the effect declaration — the deduction clause is
the contract. The decision is where it surfaces:

- **(a) Structural, diagnosed at the binding site.** `spawn H(…)` /
  `use pid` of an effect with sync-only members is an error naming the member
  and the reason ("`println` keeps `message`; a send payload is always
  consumed"). *Benefit:* zero new surface; the C-4 precedent exactly.
  *Cost:* drift discovered late — a protocol developed against a local test
  binding compiles for months, then the first production `spawn` reveals a
  keep-deduction added in week two. The failure is at the *user* of the
  protocol, not its author.
- **(b) Declared on the effect.** A marker (spelling open — the `canbe`
  family is the precedent) states "this effect must remain process-backable",
  checked *where the effect is declared*: every member inside the both-ways
  core, or an error at the declaration. *Benefit:* the Example 6 swap becomes
  a stated contract, protected against drift; the author owns the promise.
  *Cost:* a new declaration form; unmarked effects still need (a)'s check
  anyway.
- **(c) Shape-changing: forbid sync-only members in effects entirely** —
  every effect is process-backable by construction; borrowing capabilities
  become something else (functions with handler-typed implicits, or a
  separate "local interface" construct). *Benefit:* the classification
  vanishes. *Cost:* rules out today's std wholesale (`Fs`'s `Mut` buffers,
  `Console`'s kept message) and forces copies or API redesign onto purely
  local programs — the tail wagging the dog.

**Recommendation (the user's call):** (a) as the floor — it is the C-4 move
and costs nothing — with (b) available as an opt-in assertion for protocols
whose backability is a design promise (std's future process-facing effects
would carry it). Reject (c): the sync-only class is not a defect, it is
where in-place mutation and borrowing legitimately live.

## EU-4. The context rule: where a unified call may park or block

*Reshaped by round 2: the rule stands for calls on actor effects (park at
member-body seams, block via the bridge in `main`), and §0b.3's running-in
entry (EU-7) is a signature-carried extension of where the explicit
continuation forms reach — the implicit-seam boundary stated here survives
verbatim. Kept for the argument trail.*

The residue of §5.1, and the decision that keeps the unification honest. A
process-backed `-> T` call needs its frame to wait; the frames differ:

- **(a) Lexical contexts, enumerated.** Call syntax on a process-backed
  binding is legal (i) directly in a handler member body — it parks, the
  seam/gate machinery of the decided direction; (ii) directly in `main` (and
  main-line real-thread code) — it blocks, desugaring to `waitfor`. Anywhere
  else — an ordinary function, wherever it may be called from — a
  process-backed *call-shaped* capability cannot be exercised, and passing
  one down through an effect list into a plain `fn` is refused at the
  `use pid`/pass-down point when the function's body performs a waiting
  call. All-`send` protocols stay legal everywhere (first-pass status quo:
  no waiting, no restriction). *Benefit:* static, colouring-free, and
  exactly the contexts the decided machinery already covers; the ABS/Kotlin
  lesson applied. *Cost:* "caller determines" has a footnote; the diagnostic
  must teach the model ("this call waits; `drive` can be reached from a
  handler activation, which cannot wait — bind the effect to a local handler
  here, or lift the call into the member body").
- **(b) Blocking-permissive.** Ordinary functions may exercise waiting calls
  by blocking the current thread, wherever they run. *Benefit:* no
  restriction to explain. *Cost:* a handler activation reaching such a
  function wedges a pool thread — the C# deadlock class, load-conditioned
  and unreproducible in tests. Rejected by the run-to-completion decision's
  own logic; listed to mark it out of bounds.
- **(c) Shape-changing: a delimited waiting scope.** A `try`-like block
  (C-9(ii)'s "strongest argument" resurfacing) inside which ordinary code may
  wait, compiled by the seam transform, legal only where a seam can exist.
  *Benefit:* extends (a)(i) beyond the member body's top level without
  colouring — the boundary is explicit and the checker's storability rules
  apply at it. *Cost:* a new block form and the multi-frame reification it
  implies; C-9 deferred exactly this. Not first-sugar-pass material.

**Recommendation (the user's call):** (a), stated in the spec as the rule
that *completes* §0.2: the handler is written once; the **binding** decides
sync or async; the **context** decides what waiting is available — park,
block, or neither — and the checker enforces the triple statically. (c) is
the recorded growth path if (a)'s restriction bites in practice.

## Decisions pending (round 1 — superseded)

*Superseded by round 2 (§0b); the decided list is "The decisions" at the
end of the document. Kept, with the closing paragraph, for the argument
trail.*

| # | Question | Answers | Recommendation (the user's call) |
|---|---|---|---|
| EU-1 | Is the send form the *definition* of every effect member? | §0.1; the named question; Collapse 1's scope | Yes — as the call-sugar pass's semantics, after the first pass lands |
| EU-2 | What is the synchronous `Reply`? | §0.2 | Erased at sync bindings; materialized only in the `use pid` forwarding stub |
| EU-3 | Where is process-backability checked? | §5.2 | Structural diagnosis at binding sites; opt-in declared assertion for protocols that promise it |
| EU-4 | Where may a unified call wait? | §5.1 | Handler bodies park, main blocks, plain functions neither; static, enumerated, colouring-free |

Load-bearing order: EU-1 first (the others assume its answer), then EU-2 and
EU-3 (independent), then EU-4 (needs EU-3's classification to state which
calls wait). None of it gates the in-flight first pass, whose all-`send`
surface is the ground either way; all of it lands with (and largely *is*)
the call-member sugar pass.

The direct answer to §0.3, in one paragraph: the benefits are real and mostly
already latent in the decided direction — one handler form with the binding
deciding execution (the Example 6 swap extended to call-shaped protocols),
the named question dissolved, one meaning for `-> T`, `waitfor` reduced to a
desugaring, and a later async-interop dividend — at near-zero runtime cost if
the sync form is the erased specialization. The costs are two places the
divide genuinely survives relocation: a **member classification** (borrowing,
`Mut`-parameter, projection, and non-sendable members have no async reading —
which is most of today's std, so the unification is prospective, not a
retrofit) and a **context rule** (parking exists only at handler-body seams,
blocking only on real threads, so where a sync-looking call is *legal* still
depends on the frame — the one thing "the caller determines" cannot
determine without colouring or a runtime). Both are statically checkable and
both are owed under *any* design that delivers the handler swap, which is the
strongest argument that the unification is the right frame: it pays the same
costs once, for the whole language, and gets the definitional simplification
in return.

## EU-5. The effect kind: `actor effect`, and what it forbids

Folds §0b.1 and §0b.2. The divide is declared, not inferred and not hidden.

- **(a) An effect-level marker (the user's proposal).** `actor effect E
  { … }` declares a process protocol; plain `effect` stays exactly today's.
  The marker governs the members: inside an actor effect every member is
  asynchronous — payloads always cross the seam and are always consumed
  ([actor-send-fn]'s deduction rule), sendability is required (C-4, checked
  at the declaration where types are concrete and at instantiation where
  generic), and the sync-only features are refused *by name at the
  declaration*: keep deductions (`=> message`), `Mut` parameters, `proj`
  returns, non-sendable payload types. Inside a plain effect, `send fn` is
  refused symmetrically. *Cost:* one new declaration form, and the in-flight
  [actor-send-fn] rule moves house — "is this member asynchronous" is
  answered by the effect header, not per member (coordination with the
  first-pass session owed). *Benefit:* the reader knows the kind from the
  first line; the diagnostics name a law ("`Mut` parameter in an `async
  effect`") rather than an emergent property; and §0b.1's "reckon with it
  when choosing" has a place to happen — the declaration.
- **(b) Kind by inference — no marker.** `send fn` stays per-member and a
  checker rule refuses *mixed* effects. §0b.2's safety with no new syntax.
  *Cost:* the kind is emergent — which member "made" this effect async is a
  diff-archaeology question, the natural diagnostic is the weaker "adding
  this member makes the effect mixed", and EU-6's two readings of
  `fn m(a) -> T` would hang off an inference — which is exactly the hidden
  difference §0b.1 rejects, one level up.
- **(c) Shape-changing: a separate declaration kind** (`process E` /
  `protocol E`), not an `effect` at all. *Benefit:* maximal separation.
  *Cost:* duplicates everything the C-10 direction unified — handlers,
  `use`, interception, dependencies, generics — into two constructs that
  share all machinery but the name; and Example 6's swap (one handler text,
  local or spawned binding) would straddle two worlds. Against
  orthogonality.

**Sub-decisions under (a):**

- **Spelling.** `actor effect` (the user's proposal) is honest — the
  direction's own name is "asynchronous effect handlers", and it does not
  reopen C-7: the 2026-09-04 record rejected async as a *function colour*,
  and this word marks a *declaration kind*, not a call site or a fn type.
  Alternatives if wanted: `process effect`, `send effect`. User's call.
- **Does `send fn` survive inside?** Recommend yes. In the first pass it is
  every member anyway; at the sugar pass it becomes the meaningful half of
  an axis: `send fn m(a)` = fire-and-forget, no completion; `fn m(a) -> T` =
  call member (EU-6). [decl-explicit] keeps this crisp — a plain member must
  declare its return type and a send member may not have one, so no bare
  form is ever ambiguous between the two.
- **Bindings follow the kind.** `spawn` and `use pid` require an async
  effect; a sync effect can never be process-backed. **The carried named
  question resolves to "forbid" — with a sanctioned spelling for the
  want:** the IO-actor pattern is written by declaring the protocol `async`
  in the first place. Example 6's swap survives *within* the kind — a local
  `use` of an async-effect handler stays legal (test wiring; members run
  inline; replies still enqueue, so causal order is binding-independent as
  decided) — while sync→async *promotion* is deliberately gone: §0b.1, you
  reckon with the choice when you design the effect.
- **Handlers and dependencies.** A handler's kind follows its effect:
  `handler H of AsyncE` may be spawned or `use`d; `handler H of SyncE` may
  only be `use`d. Dependency lists mix freely in both directions — an async
  handler constructing sync handlers at spawn (the decided spawn-site `use`
  clause), a sync handler holding `Pid`s and telling through them (sends
  wait for nothing, so no context problem arises).
- **std is untouched.** `Console`, `Fs`, `Random` remain plain effects;
  round 1's retrofit cost (§5.2's audit) vanishes by construction.

**Recommendation (the user's call):** (a), with `send fn` kept inside, and
the [actor-send-fn] relocation coordinated with the in-flight session.

## EU-6. What `-> T` means, per kind — Collapse 1, scoped

With the kind declared, the call-member sugar pass gets a clean landing:

- **(a) Two readings, keyed by the declaration kind** — §0b.2's "governs how
  the members are interpreted", applied to the sugar pass. In a plain
  effect, `fn m(a) -> T` is today's synchronous member, unchanged forever.
  In an actor effect it is the call member: implicit trailing `Reply<T>`,
  fulfil-at-every-return, call syntax = auto-mint + gate, per the decided
  tower. The reader is never ambushed, because the effect's first line
  states the kind. This is Collapse 1 adopted exactly where it is true and
  nowhere it is not — round 1's spec-economy benefit at none of its
  grounding cost.
- **(b) No `-> T` in actor effects, ever** — the first-pass explicit-token
  surface is final. *Cost:* Example 1's clean form never exists; every
  call-shaped protocol is written in continuation style forever. The sugar
  pass dies.
- **(c) A distinct marker for async call members** (a different arrow, a
  keyword) so the member shows its reading without the header. *Cost:*
  syntax for a distinction the header already makes; noise on every line.

Worth pinning under (a): within an actor effect the full axis is `send fn
m(a)` = no completion; `fn m(a) -> T` = call member; `fn m(a) -> None` = the
acknowledged form — completion signal, no payload. That is round 1 §3's
`do_something`/`send_something` distinction, relocated inside the kind where
it belongs.

**Recommendation (the user's call):** (a), landing at the sugar pass.

## EU-7. The running-in entry (§0b.3)

*Superseded in the main by round 3 (user question, 2026-09-15): the
generalized mint — EU-7b — delivers remote minting through the ordinary
effect list with no new entry, and the residue (self-targets, the gate) is
served lexically plus by passing tokens as parameters. Kept for the argument
trail; the soundness analysis (why minting needs no continuation capture)
and the implicit-seam boundary at the end carry over unchanged.*

The question restated: `[E]` in a function's effect list says E's members
are available to call. Can a list also say the function is **running in** an
activation of a handler of actor effect E — licensing `replyto k(…)` /
`replyto! k(…)` for E's members in that function, and later in lambdas, once
effect polymorphism threads effect lists through fn values?

**Why the mechanism is sound before any design choice:** `replyto` does not
park the current frame. It allocates a parked invocation of a *member* and
yields the linear token ([actor-replyto]) — the resume point is a member,
never the current stack. So licensing it outside the member body needs no
continuation capture at all; it needs only the process identity (self
address + slot allocation), which a capability entry can thread exactly as
effect dependencies already thread. And an effect-list entry is the honest
colouring this language already chose: `[use]` and `[spawn]` are body
capabilities carried in lists and refused on fn types.

**A baseline that answers part of the want with no new surface:** handlers
already contain private fns (Example 2's `flush`) and private send members
(Example 6's `ensure_fetching`, `batch_arrived`). Whether a *plain private
fn* of a handler may mint — its enclosing handler is lexically unambiguous —
is a first-pass-adjacent detail worth settling regardless. Recommend: yes.
Everything below is about *free* functions, reusable across handlers.

- **(a) The entry, on named functions.** Spelling candidates: `[in E]`
  (reads "runs in E"), `[of E]` (echoes the handler's `of` clause),
  `[self E]`. Grants: `replyto`/`replyto!` targeting **E's members**;
  whether it also grants self-tells (enqueue an invocation of an own member
  without a token) is a sub-question — today's surface reaches self only
  through minted tokens. Propagation: implicit inside the members and
  private fns of any `handler … of E`; a function declaring the entry is
  callable only from a context carrying it — ordinary effect-list
  discipline, statically checked, no dynamic lookup. Scope limit worth
  stating: the targets are the *effect's* members — a free function cannot
  see handler-private members, so continuation routing that lands on a
  private member stays in the handler (or the target is promoted into the
  effect). *Benefits:* handler logic factors into ordinary functions;
  helpers shared across the handlers of one protocol — both sides of
  Example 6's swap use the same helper; the deadlock graph stays static,
  since gated mints in helpers are attributed through the entry, in
  signatures. *Costs:* a new effect-list entry kind in a frozen grammar (so:
  next pass, or a user-approved amendment); the **one-outstanding-gate rule
  loses locality** — a member body plus its helpers can each mint gated, so
  the check becomes interprocedural over the entry, a conservative static
  rule (at most one `replyto!` reachable per activation), or a runtime
  backstop; and it *is* colouring — the sanctioned, effect-list kind, but
  one more thing signatures carry, to be spent deliberately.
- **(b) Don't add it.** Minting stays in member bodies and private fns (the
  baseline); free helpers compute values and members do the routing.
  *Cost:* continuation-heavy handlers (Examples 3 and 5) stay monoliths at
  the routing layer, and no cross-handler helper library can exist.
- **(c) Shape-changing: reify the context as a value** — a `Self<E>`-like
  token parameter helpers take explicitly, minting as an operation on it.
  *Benefit:* no new entry kind; flows through lambdas today as ordinary
  capture. *Cost:* slot-allocation rights become storable, sendable data —
  the escape problem everywhere at once, on a many-shot token; it is
  "effects as values" arriving through the back door, against
  [effect-not-a-type]'s whole design. Rejected on those grounds.

**The lambda extension** (the user's "more opportunities"): once effect
polymorphism exists, a combinator can accept `(x: T) [in E] -> …` and a
lambda that mints — generic scatter, retry, and timeout shapes become
library code. The soundness edge to pin *now*: the capability is tied to the
current activation's process, so a lambda carrying it **must not outlive the
activation** (not stored, not sent, not returned) — or it would mint against
a context that has moved on. That is a fate/linearity constraint of exactly
[fate-lambda]'s shape, and the reason (a) restricts to named functions until
that machinery lands.

**The boundary that keeps EU-7 honest** — the part of round 1's EU-4 that
survives: the entry extends the *explicit* continuation forms to helpers; it
cannot extend the *implicit* one. Call syntax's auto-mint (sugar pass)
targets an anonymous resume continuation whose body is "the rest of the
member activation" — and the rest of a helper plus the rest of its caller is
a multi-frame capture, the machinery both backends lack and C-9 rejected. So
the line is: **explicit continuation style travels down the call graph with
the entry; implicit seams stay in member bodies.** The same relationship
`then` had to the plain call in the exploration — one line apart in power,
different in what the compiler must reify.

**Recommendation (the user's call):** the baseline (private fns mint)
regardless; (a) as the design, spelled `[in E]` unless the `of` echo is
preferred, landing after the first pass unless the user chooses to amend the
frozen grammar; the lambda extension deferred to the effect-polymorphism
work with the outlives-constraint recorded here so it is designed in, not
retrofitted.

## EU-7b. Round 3 — the generalized mint: is the entry needed at all? (user, 2026-09-15)

The user's question, verbatim in substance: do we even need to specify
*which* effect we're running in? Surely any effect we can name we can mint
replies for — not `replyto!`, admittedly, but any normal `replyto` should
work.

**The reframing that makes it true.** A `Reply<T>` minted toward member `k`
is a **one-shot, partially-applied invocation of `k`** — captures pre-bound,
the reply value as the trailing argument, capacity reserved, send-once. For
anyone who can already *call* `k` through their effect list, minting toward
it grants no new authority: the token is a curried send. So the general rule
falls out of the existing scoping machinery:

> `replyto k(captures)` resolves `k` like a member call — **lexically**
> against the enclosing handler's members first (the first-pass rule,
> unchanged: private send members live in no effect and self is never in
> scope), otherwise **through the effect list** against a member of an
> `actor effect` in scope. The former is the classic self-continuation; the
> latter is a *remote mint*: a token that, when sent to, enqueues
> `k(captures…, value)` on whatever process backs that binding.

A plain `[E]` in the effect list is all a remote mint needs — exactly the
user's "any effect we can name". No running-in entry.

**What it buys beyond deleting EU-7's entry:**

- **Direct reply routing — pipelines without hops.** `a(x, replyto
  other.k(c))` sends a request whose reply lands on a *third* process,
  without transiting the requester. Today that topology requires the third
  process to have minted and shipped the token itself; generalized, anyone
  who can name both parties can wire them. This is Erlang's
  `gen_server:reply`-from-elsewhere, made constructible rather than only
  forwardable — and it composes with the decided delegation story (tokens
  already travel; now they can also be *born* pointing anywhere reachable).
- **The lambda story is rescued, not deferred.** EU-7's soundness edge — a
  lambda carrying a running-in capability must not outlive the activation —
  existed only because self-mints depend on the current activation's
  process identity. A remote mint depends on nothing about the current
  activation: it is a curried send over a scope binding, supplied at the
  lambda's call site like any fn-type effect list. So once effect
  polymorphism threads effect lists through fn values, minting inside
  lambdas is clean *by construction* for remote targets — no outlives fence
  needed. (Self-mints in lambdas stay out, and lose their motivation: see
  the residue.)
- **The kind marker earns more keep.** Only `actor effect` members are
  mintable-toward (a sync member has no queue to land on) — EU-5's
  declaration-level kind is exactly the rule that makes this a one-line
  check.

**The residue — what "running in" still uniquely named, and how it is
served without the entry:**

1. **Self-targets.** Inside `handler NoticeFetcher of NoticeSource`,
   `replyto batch_arrived()` targets a private send member that belongs to
   *no effect*; and even `NoticeSource`'s own members are not self-bound in
   scope — a dependency on the effect you implement means the *outer*
   handler ([effect-intercept]'s outward binding), never self. So
   self-targeting is lexical, stays lexical, and is confined to handler
   bodies (plus private fns, per EU-7's baseline).
2. **The gate.** `replyto!` gates *my* mailbox awaiting *my* member's
   invocation — self-only by nature, as the user noted. It stays lexical
   with (1), and the one-outstanding-gate check stays local, dissolving
   EU-7(a)'s interprocedural-gate cost.
3. **Factoring is served by token-passing, not by the entry.** Tokens are
   ordinary linear values: a handler body mints its self-targets and hands
   them to helpers as parameters (`fn helper(r: Reply<T>, …)`), and
   linearity rides through the move [linear-obligation]. What genuinely
   cannot be factored out: a helper that must *create* self-targeted or
   gated continuations in data-dependent number (a scatter loop minting one
   self-continuation per item). If that bites in practice, the entry can be
   revisited as a narrow addition — recorded as the watch item, not built.

**Soundness: one invariant to preserve, one consequence to accept.**
"Sending to a `Reply` never blocks — capacity was reserved at mint time"
(CONCURRENCY_EXAMPLES.md, "What a `Reply<T>` is") must survive the
generalization. A self-mint reserves trivially in its own process; a
**remote mint must reserve a slot in the target's bounded queue** — so the
*mint* is send-like: it may block when the target queue is full, and it
contributes a load-conditioned wait-for edge **at the mint site**, exactly
as a blocking send from a handler does under the decided C-3 shape. Sending
to the token stays non-blocking wherever it happens. The deadlock graph
stays static and binding-keyed: remote-mint edges read off the effect
bindings like call edges do. Riding along unchanged: linearity (the token
is linear regardless of target), the dead-process rule (a token to a dead
process is a silent no-op, per the supervision design), and typing (`k`
must be a send member whose parameters are the captures plus a trailing
`T`).

**Sub-questions the generalization opens** (details, not blockers):

- **A remote mint through a *local* binding of an actor effect.** Example
  6's test wiring binds `DbApi` to a handler constructed on the hosting
  process; a token minted toward its member should enqueue on the hosting
  process (fulfil-is-enqueue is unconditional), which falls out — but a
  local async binding in a context with *no* process (main, before any
  spawn) has no queue to reserve in; refusing the mint there, with a
  diagnostic naming `waitfor` as main's token source, is the consistent
  answer. To be pinned when the mint's checker slice lands.
- **Vocabulary.** "Mint" was named for slot allocation in the requester;
  generalized, the one concept is "construct a one-shot, capacity-reserved
  member invocation" — the spec should define `Reply<T>` that way once,
  with self and remote as the two resolutions of the target.

**Recommendation (the user's call):** adopt the generalized mint as EU-7's
answer — `replyto` resolves lexically-then-by-effect-list, `replyto!` stays
lexical to handler bodies, no running-in entry — with the data-dependent
self-mint case recorded as the watch item that would reopen a narrow entry,
and the reserve-at-mint rule written into the `Reply` definition when this
propagates.

## The decisions (user, 2026-09-15, round 4)

- **EU-5 = (a): `actor effect`, with `send fn` kept inside.** The
  effect-level kind marker as proposed, carrying the whole of option (a):
  the refusal list (keep deductions, `Mut` parameters, `proj` returns,
  non-sendable payloads refused inside an actor effect; `send fn` refused in
  a plain effect), bindings following the kind (`spawn` / `use pid` require
  an actor effect — **CONCURRENCY.md's carried named question closes as
  "forbid, with the async kind as the sanctioned spelling"**), and std
  untouched. `send fn` is kept **so that the non-send (sugar) member forms
  can be stated against it** (the user's stated reason): within an async
  effect the axis is `send fn m(a)` = no completion, `fn m(a) -> T` = call
  member, `fn m(a) -> None` = the acknowledged form.
- **EU-6 = (a): two readings of `fn m(a) -> T`, keyed by the declared
  kind.** Plain effects unchanged forever; the async reading (implicit
  trailing `Reply<T>`, fulfil-at-every-return, call syntax = auto-mint +
  gate) lands at the call-member sugar pass.
- **EU-7b = agreed: the generalized mint, no running-in entry.**
  `replyto k(c)` resolves lexically against the enclosing handler, else
  through the effect list as a remote mint — a curried, capacity-reserved,
  one-shot send, with reservation at mint time (mint-site deadlock edges,
  send-like) and non-blocking discharge preserved. `replyto!` and
  self-targets stay lexical to handler bodies. The watch item stands:
  data-dependent self-minting in helpers is what would reopen a narrow
  entry.
- **New input, same round: `self.m` syntax.** The parallel session is adding
  `self.m` inside actor effect handlers; the user notes it can disambiguate
  self-directed mints if need be. *(Spelling revised in round 5, below: the
  selector form `@self` replaces the dot form.)* Recorded for the `replyto`
  checker slice either way: an explicit self form gives the resolution rule
  a disambiguator with the existing selector shape ([effect-at] /
  [fn-overload-at] — a tie is an error and the call says which it means).
  If the self form is also a self-*tell*, EU-7's self-tell sub-question is
  answered by the in-flight surface.

### Round 5 (user, 2026-09-15) — the details settled, and the `@self` spelling

- **The self-directed spelling is `@self`, not `self.m`.** Revising the
  round-4 note: the selector form — `m@self(args)` for a tell,
  `replyto m@self(c)` / `replyto! m@self(c)` for mints — replaces the
  in-flight dot form. It reuses the existing "the call says which it means"
  machinery: `k@E` picks an effect's member, `k@self` picks the enclosing
  handler's — one selector family for every disambiguation, where `self.m`
  would have added a second, dot-shaped mechanism beside it. The parallel
  session has **already landed `self.m`**, so this is a respelling of
  shipped surface — ordinary under the no-backwards-compatibility invariant
  (AGENTS.md), and it is **item 1 of "The plan" below**: the old form
  becomes a plain parse error and every site the first-pass session wrote
  is rewritten in the same change. One spelling detail for the
  implementation: after `@`, a capitalized name is an effect and a
  lowercase path is a module ([effect-at]) — `self` is lowercase, so the
  parser must treat `self` as a contextual selector keyword (with whatever
  rule keeps a module named `self` from colliding; to be pinned in the
  slice).
- **Private fns of a handler may mint self-targets** (EU-7's baseline):
  yes. Their enclosing handler is lexically unambiguous, and the spelling
  is the same `@self` form as in member bodies.
- **No silent lexical win.** When a bare `k` in `replyto k(c)` names both
  an enclosing-handler member and an in-scope async-effect member, the call
  is refused, naming `k@self` and `k@E` as the remedies — the
  [fn-overload-at] tie rule, applied to mint targets.
- **A remote mint in a context with no process is refused**, with the
  diagnostic naming `waitfor` as main's token source.

### Round 6 (user, 2026-09-15) — DECIDED: `Pid<E>` → `Addr`

The user's concern: `Pid` is the operating system's word — anything
interacting with the OS may need *real* process IDs (a `platform effect`
wrapping process management would want the name), so the protocol token
should not sit on it. The same argument disqualifies `Process<E>` and
`Proc<E>`, and weakens `Port<E>` (OS ports shadow the same way). Candidates,
judged against the design's own definition of the token — "a many-shot
**address** typed by a protocol", with `Reply<T>` "a one-shot address typed
by a single value" (CONCURRENCY_EXAMPLES.md, "What a `Reply<T>` is"):

| Candidate | For | Against |
|---|---|---|
| `Address<E>` | The spec's own defining word; the token pair becomes self-describing (Address = many-shot, `Reply` = one-shot); no OS shadow | A popular *domain* noun — user programs model postal/shipping addresses, so shadowing of a core name would actually get exercised |
| `Inbox<E>` | "Send to its inbox" reads exactly right; rare as a domain noun | Mildly conflates the token with the queue it routes to — capacity lives with the process, not the token |
| `Handle<E>` | Neutral, familiar PL word | Says nothing about messaging; the spec prose already uses "handle" for linear stream tokens — an in-house overload |
| `Sender<E>` | Names the capability precisely: a many-shot send capability (Rust `mpsc` lineage) | Reads as the sending *party* rather than the destination; asymmetric beside `Reply` |
| `Recipient<E>` | Names the far end correctly | Long; email-flavoured |
| `Actor<E>` | Industry-recognizable | Introduces a second noun beside the decided "process" |
| `Subject<E>` | Gleam's precedent (typed `Subject`) | Collides with Salvo's own prose — "`when` subject", "`for` subject" |

**Recommendation was `Address<E>`** — naming the thing what its own spec
already calls it, with the token pair self-describing — with the noted
domain-noun shadowing cost. Runner-up: `Inbox<E>`.

**Decided (user, 2026-09-15): `Addr`** — the abbreviated form of the
recommendation (the user wrote `Addr<R>`; the parameter is unchanged — the
effect the process serves [actor-types] — the letter is presentational).
It keeps the address reading, stays short in signature-heavy code
(`List<Addr<ShardApi>>`), and drops most of the domain-noun collision:
`Addr` is rarely a model name where `Address` is common. `Pid<E>` is landed
surface ([actor-types], `core.process`), so the rename joins **plan item
1's respelling sweep**: the intrinsic type and its spec rules, every
signature, test and snapshot, [actor-use-addr]'s label refreshed, and the
prose renamed with it (`use pid` → binding an effect to an *addr*; the
spawn clause's `pid` metavariable in [actor-spawn-expr]).

### Round 7 (user, 2026-09-15) — OPEN: respelling the spawn line

The user finds the frozen spawn line cumbersome — multiple keyword clauses
run together on one line:

```
spawn NoticeFetcher(25, seconds(5)) use MemFs(root), StdOutConsole() capacity 16 on pool(2)
```

— and asks for a better spelling, multi-line welcome. The spelling was
explicitly marked revisitable when frozen ("nothing downstream depends on
the words"). The constraints the *decided* record imposes on any respelling:
`spawn` stays a form, not a call (handler constructions are not values —
the named-parameter function shape is already rejected [actor-spawn-expr]);
the clause keywords are the named parameters Salvo does not otherwise have;
`capacity` and `on` are required with no defaults; the `use` clause stays a
restricted form (constructions and `Addr`s, no closures).

- **(a) Continuation-line clauses — layout only.** Each clause may start on
  its own line; the form ends after the `on` clause. Exact precedent: the
  deduction clause is legal "on the same line or the next" [deduce-syntax].
  *Benefit:* fixes the scanning problem (the `use` list running into
  `capacity` with only a keyword as the boundary) at zero grammar cost;
  every frozen word survives. *Cost:* the keyword count is untouched.
- **(b) Fold `capacity` into the `on` expression.** A `Placement` value
  built by ordinary functions — `pool(2).queue(16)`, or `io.queue(16)` over
  a shared pool — with `on` taking a `Placement` and a bare `Pool` refused,
  so "capacity explicit, required, no default" survives as a *type* rule
  rather than a keyword. *Benefit:* three clauses become two (`use`, `on`);
  the bound stays labeled — by a function name, which is how Salvo labels
  arguments without named parameters; a `Placement` is an ordinary
  shareable value (`let small = io.queue(16)` reused across spawns).
  *Cost:* the queue bound conceptually belongs to the process, and the
  chain visually attaches it to the pool; one new `core.process` type.
- **(c) The spawn block.** Clauses one per line inside braces:

  ```
  let fetcher = spawn NoticeFetcher(25, seconds(5)) {
      use ddb, SystemTimer()
      capacity 16
      on pool(2)
  }
  ```

  Principled reading: the block is the **child's init preamble** — `use`
  inside it registers the child's handlers, which is exactly what the
  spawn-site clause already means ("a spawn is a `use` whose dependencies
  come from its own clause" [actor-spawn-expr]). Aligns with the recorded
  later-sugar latitude ("the `use`-block spawn form", CONCURRENCY.md
  pending table), and has room to grow if supervision options ever join
  the spawn site. *Costs:* a braced block that is neither code nor a
  struct literal — a new block kind admitting clause lines only; and a
  call to make on whether the inline form survives beside it (two
  grammars for one form is its own cumbersomeness).
- **(d) Rejected for the record: the bracketed dependency list** from the
  exploration files (`spawn NoticeFetcher(25) [DbApi: db, Timer: clock]`),
  mirroring the handler's declared `[DbApi, Timer]`. The labeled form
  introduces colon-pair syntax existing nowhere else; the positional form
  is fragile against declaration reordering; `use` already says the right
  thing and keeps the swap-with-local-binding symmetry legible.

**Recommendation (the user's call): (b) + (a)** —
`spawn H(args) use deps on io.queue(16)`, breakable across lines:

```
let fetcher = spawn NoticeFetcher(25, seconds(5))
    use ddb, SystemTimer()
    on io.queue(16)
```

The smallest change that attacks both complaints: one clause keyword gone,
the line length solved by a rule with existing precedent. Hold (c) in
reserve for when the clause set grows. Whichever is chosen joins **plan
item 1's respelling sweep** — `spawn` is landed surface, so the change is
an outright respelling with the usual full-repo rewrite.

### The plan — what this document hands to implementation

The two former coordination items became work items when the first pass
landed under them; they head the list. In order:

1. **Respell `self.m` → `m@self`, and rename `Pid<E>` → `Addr<E>`**
   (rounds 5–6). Both forms are already in the tree, so these are ordinary
   syntax changes under the no-backwards-compatibility invariant. The
   respell: the parser accepts `m@self(args)` for tells and
   `replyto m@self(c)` / `replyto! m@self(c)` for mints, with `self` a
   contextual selector keyword after `@` (lowercase would otherwise read as
   a module path — [effect-at]); the dot form becomes a **plain parse
   error**, no transitional accept (the `canbe` precedent — nothing outside
   this repository writes Salvo). The rename: the `core.process` intrinsic
   type, its [actor-types] rule, [actor-use-addr]'s label refreshed, every
   signature, diagnostic text, test and snapshot, and the spec prose that
   says "pid" (including [actor-spawn-expr]'s clause metavariable). One
   sweep for both: `grep -rn 'self\.\|Pid\b\|pid'` across crates/, std/,
   examples/ and the spec files; sites that cannot be rewritten confidently
   are listed for the user, per AGENTS.md.
2. **Move the member-kind question to the effect header** (EU-5). The
   `actor effect` marker; `send fn` legal only inside one and refused in a
   plain effect; the refusal list (keep deductions, `Mut` parameters,
   `proj` returns, non-sendable payloads) checked at the declaration;
   `spawn` / `use pid` requiring an actor effect — which closes
   CONCURRENCY.md's carried named question. [actor-send-fn]'s spec text
   moves with it; the diagnostics name the law ("`Mut` parameter in an
   `actor effect`").
3. **With the call-member sugar pass** (EU-6 + EU-7b). The per-kind `-> T`
   reading (plain effects untouched; async: implicit trailing `Reply<T>`,
   fulfil-at-every-return, call syntax = auto-mint + gate). The generalized
   mint: `replyto k(c)` resolves lexically then through the effect list;
   refuse-on-tie naming `k@self` / `k@E`; a remote mint in a no-process
   context refused, naming `waitfor`; reservation at mint time in the
   token's target, mint-site deadlock edges, non-blocking discharge.
4. **Propagation, then deletion.** The COMPLETED.md decision-log entry
   (rounds 1–5, including the round-1 rejection as an explored-and-abandoned
   option); CONCURRENCY.md's pending table updated (the named question
   closes; the sugar-pass row gains items 3's shape); the `Reply<T>`
   definition in CONCURRENCY_EXAMPLES.md reworded to
   reservation-at-mint-in-the-target (self as the special case); fresh spec
   labels with each landing slice — then this document is deleted, its
   record living in the log.

Watch item, carried (would reopen a narrow decision, nothing else does):
a helper that must create self-targeted or gated continuations in
data-dependent number still cannot be written outside the handler body; if
that bites in practice, a narrow running-in entry is the recorded shape to
revisit (EU-7(a)).

The answer to §0b and round 3, in one paragraph: surfacing the divide at the
effect declaration keeps everything round 1 found true and drops everything
it strained for. The sync-only feature list stops being a diagnosis and
becomes the `actor effect` refusal list, checked where the author is
deciding; mixed effects are refused outright, which one shared-state
argument suffices to justify; the carried named question closes ("a sync
effect is never a process — declare the protocol async if you want one");
Example 6's swap survives where it is sound (within the async kind) and
disappears where it was illusory (promotion); and std is untouched. Round 3
then shrank the one new capability to nearly nothing: a `Reply` is a
curried, capacity-reserved, one-shot send, so any member reachable through
the ordinary effect list is mintable-toward with no new entry — direct
reply routing between third parties falls out, lambdas get minting for free
once effect polymorphism lands (no outlives fence, since a remote mint owes
nothing to the current activation) — and what remains genuinely self-bound
(`replyto!`, private continuation members) stays lexical to the handler
body, where the first pass already put it.
