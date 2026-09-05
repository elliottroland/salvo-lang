# Salvo Compiler — Progress & Plan

Status snapshot as of 2026-09-03: both backends (Kotlin, Rust) work
end-to-end; the post-M8 phase added developer tooling (`salvo analyze`,
the `salvo lsp` language server, a VS Code extension) and a flow-sensitive
ownership analysis (use-after-consume from declared *and* inferred
deductions, uniform across types, branch- and loop-aware). The
shared-fate arc (S1, L2, S2, L3, L4, S3) **and L6 must-use linearity**
are complete: move-mode bindings emit real moves, borrow-mode bindings
and loops emit real borrows, read-only pipelines over kept parameters
are clone-free — and `canbe Linear` types carry a use obligation
(forgetting to close/commit is a compile error, `discard` is the
explicit escape hatch, enforcement is purely static and identical on
both backends). This document is the handoff point for continuing
development: it records what is built, the key design decisions, known
limitations, and the plan for what's next. **The L7 milestone is
complete** — L7a (`<T canbe Linear>`), L7b (`Once` fn types), L7c
(derived returns `ReadOnly[from: p]`), and L7d (fn-type contracts:
named fn-type parameters with standard deduction lists, applied at
fn-value calls, inherited by lambdas, carried by named fns, emitted as
real Rust modes). Small recorded remainders: fn-type *effect* lists
parse but are not yet enforced, `Once` inference, the internal
qualifier unification (nothing forces it), and L5 field precision only
if whole-variable granularity proves too coarse.

**E3 step 1 landed 2026-09-04: `defer { ... }` on both backends.** The
first slice of handler control beyond "always resumes" starts with the
piece the rest of it depends on: a way to run code on the way out of a
block. Four user decisions shaped it (all 2026-09-04): **block-only
syntax** (`defer { ... }`, consistent with every other body form),
**block scope** (not function scope — a `defer` in a loop body runs per
iteration), **splice-at-exit semantics**, and **no control flow out of a
deferred body**.

Splice-at-exit is the load-bearing one, and it *dissolved* the open
question the roadmap recorded ("does a deferred body capture by move or by
reference?"). `defer S` means: `S` is checked and emitted as if written at
every exit of the enclosing block — its end, and each
`return`/`break`/`continue` leaving it. There is no closure, so there is
nothing to capture, and `defer { close(f) }` discharges the linear
obligation exactly as writing `close(f)` at each exit would
[linear-obligation]. The proposed Rust lowering in the roadmap sketch (a
`Drop` guard) was dropped for a concrete reason: `close(f)` consumes the
handle, so the guard must own `f` from the `defer` onward — which makes
`f` unusable for the rest of the block, i.e. exactly the code the feature
exists to enable (a `&mut` capture trades that for `E0499` at the next
use).

Lowerings: Kotlin wraps the rest of the block in `try { … } finally {
body }`, where nesting gives LIFO for free and `try` being an *expression*
keeps value-position blocks working [kt-defer-finally]; Rust splices the
body — rendered once at the `defer`, re-indented at each exit
[rs-defer-splice]. Verified end to end: the same demo (LIFO at a block
end, an early `return`, `continue`/`break` out of a loop body, a linear
handle released on both paths of a two-exit fn) prints byte-identical
output under `kotlinc` and `rustc`.

Checker design worth remembering: the body is checked **once**, in the
flow state at the `defer` statement (snapshot/restored, so nothing is
consumed *there*), and the *effect* of running it is replayed at each
exit — the values it consumes are consumed there, so a manual `close(h)`
plus a deferred one is a use-after-move, reported once per `defer`. The
facts the single check relied on are recorded with it and verified at every
exit: if a call in between takes the value away or invalidates a narrowing
the body used, that exit is an error naming the remedy. Without that check
a deferred `!!`/unwrap could be emitted for a fact that no longer holds
[backend-never-wrong].

One divergence is accepted and recorded for future work: Kotlin's
`finally` also runs while an *unexpected* exception unwinds (a panic out of
a std define), where the Rust splice does not. Salvo has no `catch`, so
side effects during a crash are not part of a program's meaning; tightening
it means catching the abort signal specifically once `abort` exists.

Probed by hand on both backends beyond the test demo (identical output
except where noted): a `defer` in a value-position block, a `defer` nested
inside a deferred body, a deferred call through an *effect* member with the
handler registered by `use`, a `defer` inside a lambda block body passed to
a fn-typed parameter — and a `defer` in an *iterator* body, which is the
one that differs: Rust's eager collection ([rs-iter-vec]) runs the deferred
prints before the consumer sees any element while Kotlin's lazy sequence
interleaves them. That cut predates `defer` (a plain `println` after a
`yield` diverges the same way) and is already recorded under
[rs-iter-vec].

**The widening check `^` landed 2026-09-05 (user design).** The dual of
`is`: boolean-valued, same places, same runtime test, but a successful check
reads the subject with the named qualifiers **removed** rather than added —
`list ^ Mut` without `Mut`, `outcome ^ Ok` without `Ok`. A `when` branch head
may be `^ Qual` too, which is the form that motivated it: `Ok (Ok Int | Err
Str)` is a claim *about* a union, and `when o { ^ Ok { when o { … } } }`
reaches the inner union with no intermediate binding and no repeated type.

Decisions taken with it (all user, 2026-09-05): nothing to remove is an
**error** (`^` removes a known claim; testing for one is `is`), qualifier
**sequences** are allowed, there is **no binding form** (the subject itself
reads widened), and droppability comes from **one exclusion list** —
`types::qual_drop_block`, which `Qual T <: T` now reads as well, so the two
cannot drift as intrinsics are added. That list is `Once` (restricts rather
than refines), `Linear` (carries a use obligation) and `ReadOnly` (the value
is derived from another); `ReadOnly` cannot be *written* in source today, so
it is unreachable from a `^` and the list carries it for the day that
changes.

**The lowering needed a materialization the design did not anticipate, and
finding it was the whole value of running the demo.** Qualifiers are erased,
so widening looked like a pure typing act: emit `is`'s test and change the
type. The emitted code compiled — and was **wrong**. Where the check peels a
wrapper arm (`Ok (A | B)` → `A | B`), a nested `when` on the subject still
scrutinized the *outer* wrapper, whose arm 0 is the one the outer test had
already taken, so the second inner branch was dead code: `wrapped(0)`
returning `err("zero")` printed `value zero` instead of `error zero`. The
generated union's `Display` impl had been masking it, since printing either
arm produced plausible output.

The fix is to materialize the peel: the widened value is bound to a
**shadowing local** at the top of the branch — `let mut nested =
nested.u1().clone();` in Rust, `val nested = nested.value as Union2<Int,
String>` in Kotlin — so reads and nested `when`s see the inner value.
Shadowing (rather than a fresh name) is what avoids rewriting every read;
Kotlin warns about it, which is the price. Rust also saves and restores the
binding *kind* around the branch, since the shadow is owned where the outer
binding may be a borrow. Checker-side this needed a physical-view override on
the variable (`LocalVar::widened`), because `repr_of` reads the variable's
declared type and a root narrow could not previously change it.

Two cuts, both reported: `^` on a *projection* (`p.result ^ Ok`) needs a
plain variable to shadow, and a `^` matching **more than one arm** cannot be
one widened view (each arm peels a different wrapper position) — the latter
rejected in the checker, so it reads as a language rule rather than a codegen
failure.

**The subject-less `when` landed 2026-09-05 (user design).** `when` was
already the exhaustive construct; it is now exhaustive in *two* ways.
With a subject it branches on a union's arms and takes **no `else`**
(unchanged, but now stated that way and enforced with a diagnostic that
names the other form instead of reporting a missing `is`). Without a
subject it is a condition chain — bare boolean branch heads — whose
**`else` is mandatory** [when-condition]:

```
let label = when {
    n < 0 { "negative" }
    n == 0 { "zero" }
    else { "positive" }
}
```

**The mandatory `else` is the entire reason the form exists**, and it is
worth being explicit about why, because the feature otherwise looks like
`if`/`elif` with different punctuation. An `if` chain without an `else`
folds `None` into its value [if-else-none], so "a chain of conditions that
always produces a value" is a property the reader has to verify by
inspection; this form is the one the grammar guarantees. That framing is
also what kept the implementation small: `Expr::WhenCond` is checked by
`check_if` itself, so narrowing, accumulated exclusions, the value join,
`Nothing`-tail dropping and [fn-must-return] all came for free and cannot
drift from `if`'s behaviour.

Decisions taken with it (all user, 2026-09-05): **bare branch heads** (no
`->`; consistent with `if cond { }` and with every other body form),
**`is`/`^` allowed in the heads** (they are boolean-valued, so they narrow
their branch — note this buys *no* arm-exhaustiveness, so `is` heads that
visibly cover a union still need the `else`; the subject form is how you
ask for that check), an **`else`-only `when` is a parse error** (it decides
nothing — write the block), and conditions are **required to be `Bool`**
everywhere, not just here.

That last one closed a standing gap rather than adding strictness for this
feature: `[if-bool]` had said "no truthiness" since M0 and *nothing
enforced it*. It is now `[cond-bool]`, checked on `if`/`elif`, `while` and
the new branch heads, per **leaf** of a `&&`/`||`/`!` condition so the
diagnostic lands on the operand that is wrong, and lenient on `Unknown`
and `Nothing` [type-unknown-lenient]. `Bool?` is rejected too — which way
`None` should decide is exactly the guess the rule refuses to make, and
guessing is how Kotlin/Rust divergence gets in. Nothing in `std/`, the
corpus, the inline test sources or the specs had to change: the whole
suite went green on the new rule unmodified, which says the language was
already being written as if the rule existed.

Lowerings: Kotlin has the same construct, so the shape survives —
`cond -> { … }` arms closed by `else -> { … }`, an *expression* in value
position with no `else null` filler [kt-when-cond]. Rust has no
subject-less `match`, so it emits the `if`/`else if`/`else` chain it is,
reusing `if`'s statement and value paths [rs-when-cond]; a
`match () { () if cond => … }` was considered and rejected (a scrutinee
that means nothing, reading worse than the chain the source already is).
Verified end to end: one demo — value and statement position, `is` heads
narrowing their branch and the `else`, a chain returning from every
branch, and one nested inside a subject `when`'s arm — printing
byte-identical output under `kotlinc` and `rustc`.

**E3 step 3 landed 2026-09-04: effects on fn types, threaded into the
value.** Fn-type effect lists were parsed and dropped; now they are part of
the type and mean **a requirement the caller of the value supplies**, not a
capability the value carries (user decision). So a lambda body performs the
effects *its type* declares rather than whatever its enclosing scope has,
and both backends pass the effect *into* the closure as a leading parameter
instead of capturing it.

**The fusion's structural cut is lifted.** The one program shape Rust lost
to Kotlin — an effect-using fn value passed to a callee that needs an effect
too — compiles and runs: with nothing captured, the closure holds no borrow
to overlap the call's own. Its codegen-error test is now a compile-and-run
test, and the same source runs identically on both backends.

**The user added an inference rule that removed the annotation tax:** a fn
*inherits* the effects of its fn-typed parameters ("we only give the
parameter `f` to `twice` so that it can call it"). So
`fn run_it(f: (s: Str) [Logger] -> Str)` needs no list of its own — while its
*callers* must still supply `Logger`, which is mechanically necessary, since
that is where the value comes from at run time. Inheritance reaches through
qualifiers (`Once () [Logger] -> None`), optionals and unions, and it is part
of the callee's contract at call sites, so it had to be added to
`check_callee_effects` as well as to the fn's own environment.

**Two sub-items dissolved rather than shipped.** "Forbid escape" is
unnecessary: a fn value carries no capability, so storing or returning one is
safe and the error surfaces where it is *called* without its effects — the
same shape as `defer`'s capture question in step 1. And a `use` in a fn
type's effect list is rejected instead of represented: registering a handler
is local to a body, so a lambda may `use` exactly when the function
containing it may.

Decided with it: **variance** (fewer effects fit where more are expected — a
pure lambda is handed the effect and ignores it, never the reverse) and
**inference** for un-annotated lambdas (from a visible body, which
[decl-explicit] permits). Where a fn type *is* written, its list drives
emission too: a pure lambda in an effectful position must still take the
parameters the caller passes, which a hand-written Rust probe made obvious
before any code was generated.

The sweep cost four test programs (`ONCE_DEMO` and `FUSION_MIXED_DEMO` on
both backends) and nothing in `std` — no std function takes a fn-typed
parameter. Two bugs fell out: a bare *identifier* argument was checked
without its expected type, so a named fn passed by value never saw the fn
type it had to adapt to; and the emitters' effect lookups searched
outermost-first, so a lambda's own effect parameter was shadowed *by* the
enclosing fn's value instead of the other way round.

**E3 step 2 landed 2026-09-04: `abort` + the intrinsic `try` on both
backends.** Non-resumption works end to end: a fn that may abort declares
`[Abort<M>]` and keeps its own return type, `abort(m)` returns `Nothing` so
the frames in between stay silent, and `try { ... }` — a compiler
intrinsic, not an effect — yields `Ok T | Aborted M`. Four user decisions
shaped the open questions (all 2026-09-04): **`M` is the union** of the
body's message types (chosen for consistency with `if`/`when` branch types,
with generated code wrapping only when a union is present), a **`try` whose
body cannot abort is an error** rather than `Aborted None`, **`main` may not
declare `[Abort<M>]`**, and `Abort`/`Aborted` are **declared in std**
(`std/core/abort.sv`) with the compiler knowing only their names.

The roadmap said to hand-write and run the three deciding Rust shapes first;
that paid for itself twice. It confirmed `?` on `ControlFlow` is stable and
that a may-abort call inside a loop stays a loop (no trampoline — the reason
CPS-splitting was rejected for this rung). And it showed the sketch's
closure lowering for `try` is the wrong shape: a closure would capture the
fn's effect parameters, so **`try` is a labelled block** instead
([rs-try-label]) — no captures at all. The price is that `?` cannot be used
inside a `try` body, so a may-abort call there becomes an inline `match`
that breaks the label. The same `match` form is what lets a deferred release
run on the abort path, since `?` returns without running the splice — which
is the third shape, and the concrete payoff of building `defer` first.

Kotlin diverges in mechanism, as expected: the JVM's unwinding *is* the
propagation, so `abort` throws a generated stack-trace-less
`salvo.AbortSignal` and an intermediate frame does nothing at all. One
consequence was not obvious and is worth remembering: **the aborted arm has
to be chosen at the `catch`, not at the throw.** Rust wraps a message into
the delimiter's union arm at the propagation site; the JVM has no such site,
and a throwing frame cannot know which `try` will catch it. So the signal
carries the Salvo type name of the message as a `tag` and the delimiter
dispatches on it, with an `else -> throw __signal` rethrow for a signal from
outside its set ([kt-abort-signal]). Comparing Salvo type *names* rather
than JVM classes keeps it erasure-proof.

Verified by compiling and running the same program on both: the value path,
the abort path with a linear handle released by `defer` *on it*, two message
types meeting at one delimiter (`Aborted (Str | Int)`), a may-abort call
inside a loop, and a nested delimiter that does not swallow the outer abort
— byte-identical stdout under `rustc` and `kotlinc`.

Two latent bugs fell out of the work, both fixed. `TokenKind::symbol()`
carried a hand-maintained copy of the keyword list and `unreachable!()`d on
anything missing from it, so *any diagnostic* mentioning `defer` or `try`
panicked the compiler; it now resolves through `KEYWORDS`, the single source
of truth. And the Rust backend only hoisted effect-reaching call arguments
under the *fusion*, so two effectful calls in one expression (`outer(inner(1))`,
or the natural `report(try { … })`) emitted `E0499` — a live parity
divergence, since Kotlin accepts it. Hoisting is now per-call and general
([effect-args-hoisted]).

**Type arguments joined the "no trust" list (2026-09-04).** A generic
call's type arguments must be *determined* — by the arguments, an explicit
list, or the expected type ([call-type-args], user decision A). What the
compiler knows must be visible at the call, so Salvo does not look forward
to a later use the way rustc does; and because the checker now always has
the arguments, `define fn` templates interpolate them as `${T}`
([backend-define-generics]), which is how Kotlin's list constructors get
their element type.

**The checker no longer takes anything on trust (2026-09-03).** Members
must be declared, not assumed: unresolved calls and dot-calls, calls on
non-fn values, fields on non-structs, `[]` on non-arrays, and `for` over
non-iterables are all errors ([call-resolve], [field-resolve],
[index-resolve], [iter-resolve]). Target-language features are reached by
declaring them; generics are opaque for want of bounds. The remaining
leniency is inference alone ([type-unknown-lenient]).

**General-leftover sweep (2026-09-02, this session).** Eleven of the
non-ownership leftovers were worked through; four turned out to be
*stale* (the capability was already there, just untested — now pinned
with tests), the rest were implemented. Landed: precedence-aware binary
rendering in the Kotlin emitter; iterator-body `return` retargeting
through value-position lowerings (`StmtCtx` is now emitter state, with
lambda bodies as the barrier); aliased imports of mangled qualified
overloads; the new `[mod-collision]` rule (non-fn name collisions are
errors, not last-win); binding widening in `unify` (with the deliberate
no-occurs-check rationale recorded); type-directed dispatch for
unchecked call sites, ambiguity now a codegen error; the new
`[effect-member-generics]` rule (member generics bind per call; Kotlin
renders them on the interface, Rust rejects them loudly); effect
environments keyed by checker `Ty` on both backends (new
`Checked::fn_effects`); and the new `[lsp-definition]` rule
(go-to-definition via `ModuleScope::def_sites` + `Checked::def_refs`).
Two real bugs fell out of the probes: a stray `eprintln!` debug print in
the checker, and the Rust emitter rendering fn-type `let` annotations as
`impl FnMut(...)` (invalid Rust). Four items were escalated as
**DECISION**s for the user; three came back decided and are now
implemented (std array functions; optionals rejected at operators and in
interpolation), leaving `when` on non-identifier subjects and the wider
operator-typing rules open.

**E1 handler dependencies landed on both backends 2026-09-04.** A handler
may now declare a dependency as a **constructor parameter of effect type**
([effect-handler-deps]) — declared on the *handler*, per the user's
interface-design objection to putting it on the effect. What the checker
does: member bodies get the dependency's effect in their environment (so
`ConsoleLogger.log` may call `println`), the `use` site resolves each
dependency from the enclosing scope and reports one that is missing
("register one before it"), dependencies are excluded from the constructor
*argument* count (the compiler supplies them, so `use ConsoleLogger()`
takes none), and a handler depending on the effect it implements is
rejected outright. Resolved instances land in a new `Checked::use_deps`
table.

Kotlin emits it by **injection**: the dependency stays the `private val` it
already was, member bodies resolve that effect to the *field* rather than a
leading parameter (an `override` signature must match the interface), and
the `use` site passes the handler from scope —
`ConsoleLogger(console)` — with callers of the outer effect never
mentioning it. Verified end to end with kotlinc.

The Rust backend fuses instead (same session, see below): capturing the
dependency in the handler is exactly B1's exclusivity trap (`E0499`) where
Kotlin shares freely, so the handler keeps no reference and the fusion hands
the dependency in per call ([rs-effect-fusion]).

**Parity bug found and fixed 2026-09-04: handler state stores.** Chasing
the last E1 item (validating handler bodies against their member contracts)
turned up a *demonstrated* backend divergence, not just a missing check.
This program was accepted with no errors and printed **2 on Kotlin, 1 on
Rust**:

```
effect Sink {
    fn keep(list: Mut List<Int>) -> [list: Mut] None    // promises it back
    fn size_kept() -> [] Int
}
handler Bin of Sink {
    held: Mut List<Int> = mutable_list()
    fn keep(list: Mut List<Int>) -> [list: Mut] None { held = list }
    fn size_kept() -> [] Int { return size(held) }
}
// caller: keep(xs); add(xs, 2); size_kept()
```

Two defects, both needed for the divergence. First, `held = list` was
treated as a *fate link* rather than a store, so the link outlived the call
that made it — Kotlin represents that (aliasing), Rust cannot, so it
silently cloned mutable data, which the parity principle explicitly forbids
as a strategy. Second, handler member bodies were **never validated**
against their declared lists, because the deduction pass collected only
`Item::Fn`; the contract said "kept" while the body moved.

Fixed on both sides: a handler state field is now marked as such on its
`LocalVar`, and assigning into one is a store in both the checker and the
deduction walker ([effect-state-store]); and handler members are validated
against their written lists ([deduce-infer]), outside the fixpoint, which is
sound because members' *bodies* never feed other fns' contracts. The
program above is now rejected at the member's own declaration ("deduction
promises `list` back to the caller, but the body moves it"), and the
honest version — the member declaring `-> []`, moving the list — prints 2
on both backends.

Known remaining gap, recorded in BACKEND_SPEC.rust.md under [rs-effects]:
the Rust backend derives effect *member* parameter modes from the default
kept rule rather than from the member's deductions, so a moved member
parameter still emits as `&mut` plus a clone. That is sound — the checker
consumed the caller's value, so nothing can observe the copy — but it is a
missed optimization and the natural companion to the fusion work.

**Dependency cycles need no check (verified 2026-09-04).** Chasing the
prerequisite the fusion's soundness rests on — a DAG — turned up that the
availability rule already provides it: a dependency must be registered
*before* its dependent, so a cycle cannot be constructed in any order
(tested both ways round; each order fails on the first `use`). One less
pass to write, and the reason is worth remembering: an ordering requirement
on registration is an acyclicity guarantee.

**E1 is complete: the Rust fusion landed 2026-09-04.** Both backends now
run handler dependencies to the same output. Rust builds one *fusion* per
`use` scope which owns the registered handler, implements every effect in
scope, and hands a dependent handler's member its dependency from a
disjoint field borrow. The full emission — and the reasoning that each
shape rests on — is under [rs-effect-fusion]; the parts worth carrying
away, because they cost the most to re-derive:

- **A fused fn parameter must be a Sized generic, not `dyn`.** A fn needing
  `[Console, Logger]` takes `__fx: &mut __Fx` with `__Fx: Console + Logger`;
  only a fn needing exactly one effect keeps `&mut dyn E`. The reason is
  *subset forwarding*: a fn must be able to hand its fused value to a callee
  needing fewer effects, and `&mut dyn Conj_A_B_C` → `&mut dyn Conj_A_B` is
  not expressible — trait upcasting reaches supertraits only, and a
  conjunction of two is not a supertrait of a conjunction of three. The
  originally recorded plan preferred the `dyn` form for code size; it does
  not work, and this was found by compiling it.
- **Nested scopes chain, they do not rebuild flat** (revises the
  2026-09-04 decision, which was recorded before implementation). Flat
  rebuilding over the outer scope's *handler locals* fails twice: effects
  inherited from a fn's parameter are not locals at all (there is only one
  fused value), and an inner fusion re-borrowing the same locals makes the
  **outer** fusion unusable after the inner block — the very case nesting
  exists to allow. Chaining through a single
  `__outer: &'a mut dyn <E | Conj>` field keeps what the decision wanted
  (dependency threading is identical at every depth) and makes lexical
  nesting equal borrow nesting. It also yields a small theorem: a
  dependency is *always* in `__outer` and never a sibling field, because it
  had to be registered first — the acyclicity guarantee doing a second job.
- **`dyn` in that one field is load-bearing**: it keeps monomorphization
  finite when a recursive fn registers a handler and recurses (the fusion
  type maps to itself instead of nesting forever).
- **The fusion is generic over the handler it owns** (`__H`), which is what
  lets a *generic* handler be fused without re-deriving its type arguments
  from the effect instance.
- **Dependent handler bodies move into a generated `__Impl_H` trait**
  (`&mut self` kept, so `self.state` still works). It is only ever a bound,
  never `dyn`, so its methods may be generic — which is how a two-dependency
  member gets a Sized fused value, via a per-handler `__Deps_H` adapter over
  the provider.
- **Two new hazards appear only under the fusion**, both because one value
  now carries every effect: member calls need UFCS
  (`Random::<i32>::next_random(recv)`) or they are ambiguous, and an argument
  that itself reaches the fused value must be hoisted into a temporary or
  the call borrows it twice (`E0499`).

The gate is program-wide and narrow: a program where no handler declares a
dependency keeps the per-effect `&mut dyn` parameters untouched, which is
why none of the existing goldens or e2e tests moved.

**Two pre-existing defects surfaced while probing the fusion** (both
reproduced with the fusion *off*, so neither was caused by it). The first is
fixed; the second needs a language call.

**Fixed 2026-09-04 — type arguments must be determined ([call-type-args],
user decision A).** The symptom was a backend divergence:
`let xs = mutable_list()` emitted `val xs = mutableListOf()`, which kotlinc
rejects, while rustc infers `vec![]` from a *later* `add`. Diagnosis showed
the template was not the problem — the **checker** never determined the
element type either (an unbound callee generic became `Ty::Unknown` in the
result), so no `define fn` could have interpolated one. Worse, the same hole
swallowed real mistakes: `add(xs, 1)` followed by `add(xs, "two")` on that
list was accepted with no error at all.

Now a generic call's type arguments must be determined — by an explicit
list, by the arguments, or by the **expected type** (a `let` annotation, the
enclosing fn's return type, or a concrete parameter the result flows into,
which now reaches nested calls). One that appears in the *result* type and
that nothing determines is an error naming both remedies. Salvo deliberately
does not look forward to a later use the way rustc does: what the compiler
knows must be visible at the call. Nothing in the repository needed
rewriting — every existing `mutable_list()`/`list()` was already annotated
or given elements.

**Also landed with it: `define fn` templates interpolate type arguments**
([backend-define-generics], user request "I want the templating to be
consistent"). `${T}` now resolves against the define's own type parameters
using the checker's new `call_type_args` table, exactly as `define type`
templates already did. std's Kotlin list defines use it
(`mutableListOf<${T}>(${...elems})`), so the element type is *always*
spelled out rather than left to kotlinc's context. Rust's templates keep
`vec![]` deliberately — rustc infers there, and pinning it would churn
output for nothing.

**And a third hole fell out of the work: handler state initializers were
never type-checked.** `held: Mut List<Int> = "definitely not a list"` was
accepted, because the handler item only validated the field's *type* and
skipped its default expression (struct fields have always been checked).
That is also why the emitters had no types for it, which is how the gap
surfaced: the `${T}` in a state field's `mutable_list()` had nothing to
interpolate. State initializers are now checked against the declared type
like struct defaults [effect-handler].

**Next effects slice: E3 steps 1–3 landed 2026-09-04; step 4 (effect
transformers) is what remains.** Handler *control* beyond "always resumes":
`defer`, then `abort` returning `Nothing` with a compiler-intrinsic `try`
yielding `Ok T | Aborted M`, then effects on fn types threaded into the
value (which lifted the fusion's structural cut) — all three **done**, see
the entries above. Step 4 is transformers: an effect member that runs a
fn-typed parameter with *additional* effects available. The gate step 3 was
supposed to build for it is in place, so the remaining question is the
surface, not the mechanism. Async is explicitly *not* part of this arc: no
silent `async`/`suspend` colouring, and async arrives later as an explicit
effect.

- **Still open: two effects cannot share a member name.**
  `Symbols::effect_of_fn` maps a member name to *one* effect, so declaring
  `emit` on both `Logger` and `Metrics` resolves every `emit` call to
  whichever was collected last (`no handler for effect Metrics<Logger>`),
  and there is no syntax to disambiguate — `emit<Logger>(…)` parses as
  *member* type arguments, not as an effect selection. Both backends fail
  identically and loudly. Deciding this needs a language call: reject the
  collision at declaration ([mod-collision]'s reasoning), or add a
  disambiguation form.

**Two** cuts remain inside the fusion, both reported
([backend-never-wrong]), and both about type arguments the fusion does not
derive: a dependent handler using its own generic parameters in a member
signature, and a `use` whose effect instance is still generic. Kotlin
accepts both (generics erase), so each is a live divergence, documented
under [rs-effect-fusion].

The third — structural, and the only program shape the fusion *lost* to
Kotlin — was **an effect-using function value passed to a callee that needs
one too**, where the closure's captured borrow overlapped the call's own.
**Lifted 2026-09-04 by E3 step 3**, exactly as predicted here: the fused
value is passed *into* the closure instead of captured ([fn-effects]).

**E1 prerequisite landed 2026-09-04: effects and handlers are not data.**
The first buildable slice of the effects arc, and a live
[backend-never-wrong] violation on its own: an effect type in a data
position (struct field, parameter, return, `let` annotation, alias) and a
handler constructor call outside `use` were both accepted by the checker
and emitted **invalid Rust** — a bare trait (`E0782`) and a non-existent
constructor (`E0423`) — while Kotlin happened to render both correctly.
Now rejected with diagnostics naming the legitimate positions and the
`use` remedy ([effect-not-data], [handler-not-value]). The two positions
that legitimately name an effect (a fn's effect list, a handler's `of`
clause) resolve through their own path and are untouched, verified by
compiling and running an ordinary effect program on both backends. E1 will
*re-admit* exactly one of the rejected shapes — a handler constructor
parameter of effect type, its dependency — together with the fusion
emission that can render it.

**Effects arc: E1 strategy settled (user decisions 2026-09-03/04).**
Effect-to-effect dependencies will use **handler fusion (B9)**: effect
interfaces stay dependency-free so user fns are emitted once; a handler's
dependencies are its constructor parameters of effect type (already
parseable today); one *fusion* per `use` scope holds the handlers and
hides dependencies by passing them from its own fields — in Rust via
disjoint field borrows, which is what lets a *shared* dependency work at
all; nested scopes rebuild flat on Kotlin and chain on Rust (revised
during implementation — see E1a); facets are Kotlin-only, since erasure
forbids one class implementing `Random<Int>` and `Random<Double>`.
Mechanism and rationale: [rs-effect-fusion], [kt-effect-fusion]; six
explored alternatives and why they failed: E1a. Consequences worth
knowing: no exclusivity, no runtime failure mode, no duplication of user
code — and neither an immutable/mutable effect distinction nor `Cell` is
needed for testability, since a handler member may mutate a dependency it
receives as a parameter. `Cell` (shared mutable state as an intrinsic
capability qualifier) therefore stands or falls on its own cases —
accumulators across lambdas, memoization, counters — written up in its own
roadmap section.

**Next arc: place-based flow analysis.** **P1 is done** (see the decision
log): flow state is keyed by *places*, `is` narrows field chains *and*
tuple positions (`t.0`, new syntax added in the same session), and
invalidation rides on the fate analysis's event set. **P2** (`when` on
field subjects) was decided *against* — `when` stays variable-only. What
still rides on the substrate: **L5**'s field-disjoint ownership.

**Effect-member deductions reach inference too (2026-09-03).** A follow-up
to the above, from a user question about why the Rust output was still
sound: the checker enforces an effect member's declared deduction list at
the call site (`check_effect_call` builds a contract and runs
`apply_call_contract`), but `deduce.rs` did not — it resolves callees
through `call_fn`, a span→`FnKey` map, and a member has no `FnKey` because
the handler is chosen at run time. So the two disagreed about the same
call: the body could not use an argument a member had taken, while the
*inferred* contract still told callers it was kept.

Nothing miscompiled, for an instructive reason: both under-claimed at once.
Deduce said "kept" and the Rust backend emitted `&String` for the member
too (member deductions do not drive its parameter modes yet), so the
emitted program borrowed end to end — and rustc only rejects
*over*-claiming (use-after-move, moving out of a borrow), never
under-claiming. Fixing one side alone would have produced the over-claim it
does catch (`E0507`), which is why the two halves are worth keeping in
mind together.

The fix is a second lookup path in `deduce.rs`: `effect_member_contract`
finds the member through the checker's recorded `effect_calls` instance
plus the callee name, then applies its written list through the same
`apply_callee` loop resolved calls use ([call-resolve]). Verified: the
example now errors at the caller ("`text` ... was consumed (moved)"), and
with the caller corrected both backends compile and run — Rust emits
`fn forward(sink, s: String)` with `sink.take(&s)`, an owned parameter lent
to a borrowing member. Still open (E1-adjacent): the Rust backend deriving
*member* parameter modes from their deductions, and validating each handler
body against the member's contract.

**Salvo assumes it can see everything (user decision 2026-09-03).** The
checker's interop leniency is gone: a call, field read, subscript or `for`
subject that no declaration justifies is now an error
([call-resolve], [field-resolve], [index-resolve], [iter-resolve]).
Target-language features are reached by *declaring* them — `external type`
for the type, `external fn` + `define` for anything you do with it — and
dot-notation still reads like a method call because it *is* a call to a
declared function. Generics fall under the same rule: with no bounds,
nothing about a `T` is knowable, so `value.name` inside `fn f<T>(value: T)`
is an error rather than a promise about future call sites.

Six holes closed, all of which the compiler used to accept silently and
hand to the target compiler: an unresolved bare call (`nowhere()`), an
unresolved dot-call (`text.shout()`), calling a value of known non-fn type
(`n()` on an `Int`), a field on a non-struct (an opaque `external type`, a
generic, an `Int`), `[]` on a non-array, and `for` over a non-iterable.
Each produced code the target compiler rejected — `E0618: expected
function` from rustc, `unresolved reference 'n'` from kotlinc — which was
loud but pointed at generated code the author never wrote, and in Kotlin's
case named a symbol that *does* exist in the Salvo source. That is not what
[backend-never-wrong] asks for.

What remains lenient is **inference, not visibility**
([type-unknown-lenient], rewritten): a type the checker could not work out
stays `Ty::Unknown` so one mistake yields one diagnostic. The one
unresolved *callee* kind left is an effect member, whose handler is chosen
at run time — [deduce-infer]'s lenient borrow now names only that.

Cost of the change: nothing. **No test needed the leniency** — the only
failure in 331 was a deduction test whose premise was a call to
`unknown_interop`, rewritten to pin the effect-member case it actually
covers. Making generics opaque broke nothing either.

**Why (user rationale, 2026-09-03):** the rules the compiler imposes should
be *easy to understand and predictable*. Strictness is welcome on that
basis — one rule for dot-calls beats a rule plus a silent interop
exception — provided the diagnostic makes the problem obvious and, where
possible, the language offers a straightforward remedy (the precedents are
`copy` for shared fate and `discard` for linear obligations). That is the
standard the new diagnostics are held to: each names the remedy
(`external fn` for a missing member, `get(collection, index)` for a
subscript, "rebuild the tuple" for an element write) — and where no remedy
exists, it says *that* instead of suggesting a useless one: a type
parameter's diagnostic explains that nothing is known about a `T` rather
than pointing at an accessor that could not help.

The emitters' method-call fallbacks went with it (user decision, same
session): both `emit_call`s used to render an unresolved dot-call as a
native method call, which was the *mechanism* for interop and is now
unreachable from valid source. Rather than leave a path that silently
guesses, each is a codegen error naming the internal inconsistency — the
checker guarantees resolution, so arriving there means a table lost an
entry without reporting it. Fallbacks stay only where a table may
legitimately have no entry (an `Unknown`-typed expression still has to
render).

**Tuple indexing added 2026-09-03 (user decision).** `t.0` reads a tuple
element by constant position ([expr-tuple-index]) — the projection P1a had
asked to narrow but which the language could not express. It is a
`Proj::Index` place, so it narrows, invalidates and merges exactly like a
field, in any combination (`p.pair.0`, `t.1.0`). Decisions inside it:
elements are **read-only** (qualifiers, `Mut` among them, cannot apply to a
tuple [qual-union-arm], so there is nothing to assign through — the error
names the rebuild remedy), out-of-range and non-tuple bases are **errors**
rather than lenient `Unknown` (the index is program text, so it cannot be
interop-dependent), and no numeric suffixes.

The subtle part was lexical: numbers own their decimal point, so `t.0.1`
would lex as `t` and the float `0.1`. Rather than un-parse a float in the
parser — which cannot recover `.0.10` from an `f64` — the *lexer* now
refuses a fraction directly after `.`, where nothing else in the grammar
can put a numeric literal (paths and dot-calls take identifiers, spread is
one `...` token). `t.0.1` is then two ordinary index tokens. Kotlin maps
the projection onto `Pair`/`Triple` components ([kt-tuple-component]);
Rust uses its own `t.0` ([rs-tuple-index]).

**P1 landed 2026-09-03: flow analysis is keyed by places, and fields
narrow.** `is` now narrows a *place* — a variable or a field chain out of
one — so after `if p.surname is Str` the field itself reads as `Str`
([flow-place]), which is what the optional-strictness rules
([interp-no-none], [op-no-none]) had been forcing into the `is Str name`
binding form. Three user decisions shaped it:

- **P1a — which projections narrow: field chains** (`h.a.b`). Array
  elements never narrow: an unknown index may alias any element, and
  restricting to constant indices would invite the expectation that
  `arr[i]` narrows too. The `Place` type carries `Field`, `Index` and
  `Element` projections from the start so place-based *ownership* (L5)
  reuses it. The user asked for constant *tuple* indices as well —
  which turned up a gap: Salvo had no tuple element access at all (`.`
  required an ident, so `t.0` was a parse error; tuples were
  destructure-only). **The user added it the same day** — see the
  tuple-indexing entry above — so constant indices narrow too.
- **P1b — kept-immutable calls preserve narrowing.** Only a call that
  keeps the value *mutably* (a `Mut` parameter) invalidates
  ([flow-place-invalidate]); a read-only call cannot mutate, so a
  logging call no longer costs you the fact. This reads the same
  deduction facts D1 established, so it is precision without new
  machinery — and it keeps the rule consistent with D5's reason for
  exempting provenance qualifiers.
- **P2 — `when` stays variable-only** (against the recommendation): field
  subjects keep the [when-union-subject] error, since `if … is` covers
  them now. That dropped the arc's top-ranked motivation; the
  strictness-gap customer and L5's shared substrate carried it.

Design notes worth keeping: place facts live *inside* the root's
`LocalVar` (a `place_narrows` list keyed by projection path), which is
why `snapshot_narrows`/`restore_narrows`/`merge_fallthrough` stayed the
single source of truth (the S1 gotcha) and why an event on a root
invalidates everything below it for free. A fact survives a join only if
every fall-through path agrees on it exactly — falling back to the
declared type is always sound. Restoring a branch's narrows must *not*
resurrect a fact the branch invalidated, the dual of the
consumed-stays-consumed rule. The four duplicated "mutation through a
projection" provenance loops collapsed into one `fate_mutation_through`,
so no invalidation site can be forgotten.

The implementation also found a real **backend-parity bug** the feature
would have shipped: Kotlin refuses to smart-cast a *property*, and
`canbe Mut` struct fields emit as `var`, so a narrowed nullable field
read produced Kotlin that did not compile ("smart cast to 'String' is
impossible"). Narrowed nullable field reads now emit `!!`
([kt-narrow-field-assert]) — the assert can never fire, since the
checker invalidates the fact on any mutation.

**Bodyless declarations are explicit (user decision 2026-09-03).** No
inference for anything the compiler cannot see: `external`/`internal` fns
must declare effects, deductions, *and* return type; effect members must
declare return type and deductions; and every `define fn` must match an
external one-to-one, taking its signature from that external
[decl-explicit]. This removed the last inference-from-nothing guess (and
with it the `Mut`-parameter proxy D1 needed), and fixed a real ownership
bug it had been hiding: std's `add` did not consume its element, so
`add(xs, h)` then using `h` compiled on Kotlin and was rejected by rustc.
New roadmap sections: **E1** effect-to-effect dependencies (the effect
declares them, handlers mirror exactly) and **E2** heuristics for
validating external declarations against their define templates.

**Backwards compatibility is not a requirement (user decision
2026-09-03).** The language is experimental and its features are still
being worked out, so compatibility machinery would only get in the way:
syntax and rules change outright, with no deprecation periods, no
grammars that accept both spellings, and no version gates. The
obligation that replaces it is a *sweep*: every affected example gets
rewritten in the same change (`std/`, test corpora, inline `.sv` sources
in Rust tests, the spec documents, README, and the syntax references in
this file — it is a handoff document, not an archive, so stale syntax in
prose is a bug and the change belongs in this decision log instead).
Anything that cannot be rewritten confidently — ambiguous intent under
the new rules, `experiments/` sketches of unimplemented features, or a
site that merely *looks* like the changed construct — is flagged for the
user to update by hand. A transitional *error* naming the replacement is
permitted but not expected (it is a diagnostic, not compatibility);
still accepting the old form never is — and the `canbe` rename's own
transitional error was removed on the user's call, so a plain parse
error is the default. Recorded as an invariant in AGENTS.md with a step
in the task workflow.

**Qualifier subjects landed 2026-09-03 (roadmap D5).** Qualifiers now say
what their claim is *about*: `qualifier Q of T` is a **state** claim about
the value's contents (the default, unchanged), `provenance qualifier Q of
T` is a claim about where the handle came from ([qual-subject]).
Provenance is exempt from D1's stripping — the rule that a mutating call
invalidates unlisted qualifiers is sound only for claims about contents —
which is the whole point: an `Authenticated Request` no longer loses its
tag to a logger that takes `Mut`. Provenance is mint-only (no body, so no
`qualifies` and no field overrides), droppable, survives storage, and
composes without `with`. It erases like every qualifier, so no backend
changed; the visible consequence is which overload the checker picks. The
compiler's own capability qualifiers (`Mut`, `Linear`, `Once`,
`ReadOnly`) stay intrinsic — each needs a representation choice, a flow
rule, a subtyping direction or a restricted position that no user
declaration could supply.

**Dot-names landed 2026-09-03 (roadmap N1).** Structs and qualifiers can
be declared `Ns.Name` where `Ns` is a struct in the same file
([name-dot]), giving the Kotlin wrapper-type idiom (`Environment.Id`)
without nested declarations — Kotlin emits a nested class, Rust flattens
to `EnvironmentId`. Casing became a language rule ([name-casing]): types
uppercase, values lowercase, module paths lowercase (so an uppercase
`.sv` file or directory name is a compile-time error). That rule is what
makes `Environment.Id { … }` decidable against `person.name`, and it
retired the parser's old "lowercase after `is` means a binding"
heuristic. Nothing visible may carry the concatenated spelling, checked
scope-wide because Rust flattening and overload mangling share it. No
existing Salvo source violated the casing rule.

**`canbe` replaces `with` at opt-in sites (user decision 2026-09-03).**
Auto-qualifiers on struct and type declarations and the per-type-parameter
linear opt-in are now spelled `canbe` (`struct Person canbe Mut`,
`external type List<T> canbe Mut`, `struct FileHandle canbe Linear`,
`fn hold<T canbe Linear>`), which states the optionality the clause
actually carries [canbe-optin]. `with` keeps exactly one job: qualifier
*compatibility* (`qualifier Old of Person with Surname` [qual-with]) —
co-application of two qualifiers, which `canbe` would misdescribe; no
better word was found, so the overload is gone but `with` stays. The two
clauses are simply independent syntax now — a transitional rename
diagnostic (and the test pinning it) was implemented and then removed on
the user's call, since nothing outside this repository writes Salvo yet.
Labels renamed with the
syntax: `[type-with-mut]` → `[type-canbe-mut]`, `[linear-with]` →
`[linear-canbe]`; AST field `FnDecl.generic_with` → `generic_canbe`
(uniform snapshot churn). The same analysis produced roadmap **D4**:
`is` on a union subject always means arm identity, so a *predicate*
qualifier can never be tested against a union-typed value — fixing that
needs qualifiers over unions, and `is` itself stays as-is (user decision
2026-09-03).

**D1 landed 2026-09-02: deductions are exhaustive by default.** A
confirmed unsoundness — qualifiers surviving calls that invalidate them,
reproduced with a `clear` that emptied a list while the caller kept
believing `NonEmpty` — is fixed. `[list: NonEmpty Mut]` now means *only*
those apply afterwards (including dropping qualifiers the callee never
declared), `[list: -Q]` is the delta form for "everything else
preserved", `[list: Nothing]` is moved, and a parameter the body mutates
may use neither keep-all nor a delta. D2 holds `+Q` asserts and their
possible unity with constructive qualifiers; D3 covers refinements — the
general answer to D1's accepted over-strictness, after analysis showed
blanket qualifier *polymorphism* cannot be sound.

Companion documents: LANGUAGE.md is the narrative spec (source of truth);
LANGUAGE_SPEC.md states every feature as a labeled rule (`[qual-erasure]`
style) with the compiler decisions under it; BACKEND_SPEC.<backend>.md
(`BACKEND_SPEC.kotlin.md`, `BACKEND_SPEC.rust.md`) repeats rules with
backend interpretation details and adds backend-prefixed rules (`kt-…`,
`rs-…`) — load it only when working on that backend. Labels are referenced
from compiler code and tests (`grep -rn '\[rule-name\]'`); backend-prefixed
labels may only be referenced from that backend's crate. Keep all of these
in sync when adding or changing features. Detailed feature mechanics live
in those specs; this file keeps the decision log, the plan, and the
hard-won operational knowledge.

## How to build and test

```bash
cargo build                 # workspace build, no warnings
cargo test                  # 386 tests; includes twenty-seven kotlinc and twenty-five rustc
                            # compile+run tests (skipped gracefully when the
                            # toolchain is not on PATH)
INSTA_UPDATE=always cargo test   # accept/update insta snapshots after intended changes

# End-to-end:
cargo run -- compile --src ./some_dir --target ./out        # --backend defaults to kotlin
cargo run -- compile --backend rust --src ./some_dir --target ./out_rs
cargo run -- compile --src ./some_dir --target ./out --emit-ast             # user-module AST dump
cargo run -- compile --src ./some_dir --target ./out --emit-ast=core.list   # one module's AST

# Type-check without generating code [cli-analyze]:
cargo run -- analyze --src ./some_dir                       # text diagnostics, exit 1 on errors
cargo run -- analyze --src ./some_dir --format json         # machine-readable diagnostics
cargo run -- analyze --src ./some_dir --backend kotlin      # also parse kotlin define files

# Language server over stdio [cli-lsp] (point your editor's LSP client at it):
cargo run -- lsp

# Regenerate the VS Code extension's TextMate grammar [cli-lang]
# (a test fails if the checked-in copy is stale):
cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json

# Verify generated Kotlin manually (the CLI prints the entry point):
kotlinc $(find out -name '*.kt') -d classes && kotlin -cp classes salvo.main.MainKt
# Verify generated Rust manually (the CLI prints the exact command):
rustc --edition 2021 out_rs/main.rs -o program && ./program
```

## Workspace layout

```
crates/
├── salvo-cli/            # binary "salvo": clap CLI, backend registry, embeds std/ via include_dir,
│                         #   analysis pipeline (analysis.rs), LSP server (lsp.rs), tm-grammar (lang.rs)
├── salvo-syntax/         # lexer, parser, AST, spans, diagnostics (no deps)
│   └── tests/corpus/     # LANGUAGE.md-example .sv files + insta snapshots
├── salvo-core/           # SourceSet, Program, Symbols + resolve.rs/types.rs/check.rs/deduce.rs/reach.rs
├── salvo-backend/        # Backend trait, BackendRegistry, BackendError
├── salvo-backend-kotlin/ # Kotlin emitter (emit.rs) + golden/kotlinc tests
└── salvo-backend-rust/   # Rust emitter (emit.rs) + golden/rustc tests
std/                      # stdlib: core/ (basic, string, list, console) + random.sv
                          #   (+ .kotlin.sv/.rust.sv defines next to each module)
vscode/                   # VS Code extension: LSP client + generated TextMate grammar
```

Adding another backend = new crate implementing `salvo_backend::Backend`,
register it in `salvo-cli/src/main.rs`, write `*.<name>.sv` define files
next to the std modules, and add a `BACKEND_SPEC.<name>.md`. Std embedding
already filters define files per backend at load time
(`SourceSet::classify`).

## History (condensed)

The milestone-by-milestone detail that used to live here has been folded
into the spec documents; what follows is the decision log — the choices
that still shape the code, and where to look for the mechanics.

- **M0+M1 — CLI + parser.** Hand-written lexer + recursive-descent parser
  (deliberate: newline-terminated statements, template/interpolation
  lexer modes, struct-literal-vs-block ambiguity, and speculative parses
  make grammar generators a poor fit). Tokens carry `newline_before`;
  parser recovers at item/statement level. String interpolation re-lexes
  `${...}` fragments with spans shifted back into the file.
- **M2 — Kotlin codegen, end-to-end verified** (kotlinc compiles the
  output; tests assert exact stdout). Founding invariant
  [backend-never-wrong]: unsupported constructs are codegen *errors*,
  never silently wrong code. Remaining deliberate cuts: multi-spread
  struct literals, early `return` inside lambdas, tuples beyond
  Pair/Triple.
- **M3 — Typechecker + unions.** `Ty` model (`Qualified` with sorted qual
  sets, `Union` flattened/deduped in declaration order), per-file scopes,
  and the side-table architecture (`Checked`: `expr_ty`, `repr_ty`,
  `coerce`, `is_tests`, `call_fn`, …) the emitters consult. Two founding
  decisions: the checker is *lenient* (anything untypable is
  `Ty::Unknown` and passes through — then justified by Kotlin interop,
  narrowed twice since: to inferred types only, and finally to inference
  alone once members had to be declared [call-resolve]), and union arm
  identity is *positional over the declared type's non-`None` arms*
  [union-arm-identity]. Kotlin unions lower to generated sealed wrappers
  (`UnionN`), `None` arms to outer nullability.
- **M4 — Qualifiers.** User decision replacing the old spec: the `as`
  effect and value-level `as` expressions were removed; constructive
  qualifiers are built exclusively through constructor functions
  (`fn ok<T>(value: T) -> T as Ok`), same-file rule, simple return types
  [qual-ctor-fn] [qual-ctor-same-file] [qual-ctor-simple]. Predicate
  qualifiers lower to `{Q}_qualifies` calls at `is` sites; struct-field
  overrides cast+assert. Overloads identical after erasure get
  deterministic `__Qual` name mangling. Qualified union groups
  (`Ok (A | B)`) wrap whole-arm first, then coerce as the bare union.
- **M5 — Effects.** The checker owns effect semantics: per-fn effect
  environments seeded from declared lists, grown by `use`, truncated at
  block boundaries; handler generics *inferred by unification* from `use`
  constructor args; effect-member disambiguation via explicit type args →
  argument types → expected type. The emitters prefer the checker's
  effect tables and fall back to string-keyed environments only in
  unchecked contexts (see "Current architectural facts").
- **M6 — Deductions + loops-as-values.** Loops are expressions
  [while-value] (body tail / `break value` / `else` join; no `else` means
  optional). Deductions (`deduce.rs`): user decision — a deduction list
  is interpreted *relative to the callee's declared parameter qualifiers*
  (a call removes exactly `declared − kept`); inference is a
  whole-program fixpoint from an optimistic start (facts only removed →
  terminates); written lists may be stricter than the body but never
  looser [deduce-syntax] [deduce-infer].
- **M7 — Polish.** Only reachable modules are emitted [mod-used-only]
  (name-usage reachability, deliberately conservative); per-module Kotlin
  packages + generated imports [kt-package] [kt-imports]; define coverage
  checked upfront for `core.*` and at reference sites
  [backend-external]; backend-native companion files copy verbatim
  [backend-companion]. Decision under [type-array]: `T[]` stays
  `Array<T>` in Kotlin (no primitive-array specialization without
  profiling data).
- **M8 — Rust backend.** The point of the design: deductions drive
  ownership [rs-borrows] — omitted parameter = moved (by value), kept =
  borrowed (`&mut` when `Mut`); expressions emit owned by default
  (borrowed reads clone); no emitted signature returns a reference, so no
  lifetimes exist anywhere. User decision: `Mut` generalized to a
  language-level qualifier any type opts into with `canbe Mut`
  [type-canbe-mut]; backends map it per type (`Mut inline:` define
  sections). Unions are generated enums; effects are traits with
  `&mut dyn` threading; iterators are *eager* (`Iter<T>` = `Vec<T>`,
  documented divergence [rs-iter-vec]); `WrapOption` coercion added
  because optionals are physical in Rust and transparent in Kotlin
  [type-nullable]. Crate layout: main-declaring module is the crate root
  with `#[path]` mounts [rs-crate].

### Post-M8 — tooling and flow analysis (decision log)

- **Optionals never reach operators or interpolation (user decisions
  2026-09-02)** — `[op-no-none]`, `[interp-no-none]`. Both were parity
  holes, not just strictness gaps: Kotlin compares against `null` and
  prints `null`, Rust rejects the `Option` (`Display` unimplemented, type
  mismatch on comparison). The checker now errors on a possibly-`None`
  operand of `+ - * / %` and `< > <= >= == !=`, and on interpolating a
  possibly-`None` value; `None` itself is rejected in both positions.
  Remedy is narrowing (`is` / `when`, taking the binding for a
  non-variable place) or `!`. Deliberately *not* covered: `&&`/`||`,
  because operand typing beyond `None` (numeric towers, promotion, `Bool`
  requirements) is a separate open decision, and value-position `&&` goes
  through a different path than `analyze_cond`. Two pieces of evidence
  that the leniency was costing us: the loops demo's Kotlin and Rust
  sources had silently diverged (`${capped}` vs `${capped!}`) because
  only Rust complained, and three LANGUAGE.md nullability examples
  interpolated `${person.surname}` after `person.surname is Str`,
  relying on field narrowing Salvo does not do — all now use the
  spec's own `is Str surname` binding idiom.
- **Doc comments and richer hover (user request 2026-09-03)** —
  `[doc-comment]`, `[doc-markdown]`, `[doc-symbol-ref]`,
  `[doc-struct-fields]`, `[doc-hover-narrowed]`. A declaration's docs are
  the run of own-line `//` comments *directly* above it — no `///`, no
  attribute, no separate syntax; one blank line ends the run. Decisions
  made along the way, all reversible:
  - **Comments are collected, not tokenized.** The lexer already knew
    exactly what a comment was and threw it away; it now records
    `LexResult::comments` (span, text, `own_line`) and the parser attaches
    the block above each declaration *by line number*. Keeping comments
    out of the token stream means the grammar is untouched — no skip logic
    in a hundred parse functions. `own_line` is what makes
    `a: Int, // note` document nothing.
  - **Docs live on the AST** (`docs: Vec<String>` on fn, struct, field,
    qualifier, effect, handler and type declarations), which is what the
    parser snapshots now show. The alternative — extracting them textually
    in the LSP — would misread a `//` inside a string literal on the
    preceding line.
  - **Hover is markdown throughout** (`MarkupContent`, not the deprecated
    `MarkedString`): a fenced `salvo` block with the signature or type,
    then doc sections separated by rules. The `ReadOnly` presentation
    moved into the same shape.
  - **`[symbol]` resolves locally first**, then against any declaration in
    the program, and renders as a *link* to the declaration; unresolved
    references are left verbatim so bracketed prose is never mangled.
    Deliberate simplification: the search is name-based over the AST
    rather than import-visibility-exact, which [mod-collision] makes
    almost always equivalent — `Resolution` borrows `Program`, so the
    analysis result cannot carry it.
  - **Hover now answers on declarations, not just uses.** Declared names
    (parameters, handler state, `let` patterns, `is`/`for` bindings) record
    their type in `expr_ty` at their own name span. Before this, hovering
    the `items` in `fn f(items: Mut List<Int>)` said nothing.
  - **The narrowed/declared pair needed no new table**: `Checked::repr_ty`
    already held the declared type for narrowed identifier uses (the
    emitters' re-wrapping fact), which is exactly the "declared as X"
    line. Hovering inside `if items is NonEmpty` shows
    `Mut NonEmpty List<Int>` over `Mut List<Int>`.
  - Struct hover lists *every* field with type and default-as-written, not
    only documented ones, so it shows the shape of the struct.
  - **Follow-up, same day: fields and members hover too.** The two gaps
    left above are closed. Hover (and go-to-definition) now reach *nested*
    declarations through one search over a module's items (`decl_at`,
    matching by name span): struct fields, handler state fields, qualifier
    field overrides, and the member fns of effects, handlers and
    qualifiers. Each renders its own declaration line, its docs, and what
    declares it ("Field of struct `Person`.", "Member of effect `Log`.").
    - Field *accesses* needed a new table: `Checked::field_refs` maps the
      field-name span in `base.field` to the field's declaration, built
      from the struct's `DefSite` (for the file) plus the `FieldDecl`'s own
      name span. It feeds go-to-definition as well, which fields never
      had. It points at the struct's field even under a qualifier field
      override — the override refines the field, it does not replace it.
    - `fn_signature` split into `fn_decl_signature(decl, inferred)` so
      members can render from the declaration; they have no `FnKey`, so
      they show their *declared* deduction list, which [decl-explicit]
      requires of them anyway.
    - A nested declaration's docs see its owner's names (`own_names`), so a
      handler member can write `[count]` for the handler's state
      [doc-symbol-ref].
    - Fallout worth noting: the feature immediately caught a real
      mis-attachment in `std/core/result.sv`, where an edit had turned the
      blank line between the module header and the qualifier's docs into a
      `//` line — so the whole header had become `qualifier Ok`'s
      documentation. Every std file's attachment was then checked.
- **Written names in type positions must resolve; `Ok`/`Err` move into
  std (user decisions 2026-09-03)** — `[name-resolve]`,
  `[qual-result-tags]`. Found through a TODO in
  `experiments/refinements.sv`: `if n is Ok` on `Ok Int | Err Str | None`
  reported "this check can never succeed", and the real problem was that
  `Ok` was never declared. Nothing checked qualifier or type *names*, so
  `Ok Int` in the return type quietly became a qualified type with an
  unheard-of qualifier, while `parse_check` — asking `is_qualifier`,
  getting no — read the same `Ok` as a *base type* that no arm matched.
  Then the empty match set narrowed the subject to `Nothing` and a bogus
  "consumed (moved)" error landed on the next use. Three errors' worth of
  noise, none of them the missing declaration.
  - The checker now rejects any written name in a type position that
    resolves to nothing, in either namespace (base types vs qualifiers),
    with import suggestions [diag-import-suggest] and a wording hint when
    the name exists in the *other* namespace. Reported from
    `validate_type` / `validate_quals` / `parse_check` at declaration
    sites, which is where [qual-of] already put applicability checking —
    lowering runs repeatedly and stays silent.
  - This narrowed [type-unknown-lenient] a first time: leniency is about
    types the checker cannot *infer*, not about names the author wrote.
    (It was narrowed again on 2026-09-03, when members stopped being
    lenient too — see the "Salvo assumes it can see everything" entry.
    At the time of this milestone, an interop type's *members* were still
    pass-through.)
  - An unresolved `is`/`when` check marks the pattern and suppresses every
    verdict that follows from the failed match — "can never succeed", "no
    remaining union arm", non-exhaustiveness, the `Nothing` cascade. One
    error per mistake.
  - Wiring the check up exposed genuinely missing declaration sites:
    struct fields, type-alias targets, effect-member signatures, handler
    ctor params and state fields were never validated at all (the
    [qual-of] rule *claimed* struct fields were). `canbe` clauses now
    reject user qualifiers, which is the `with` confusion [canbe-optin]
    already warns about.
  - Two real bugs fell out: `core.string`'s `char_at(index: Positive Int)`
    used an undeclared qualifier lifted from a LANGUAGE.md example — no
    caller could ever have satisfied it — now plain `Int`; and four
    std-less test preludes never declared `Int`/`Str`.
  - `Ok`/`Err`/`ok`/`err` now live in **`core.result`**, not
    `core.basic` (the user's initial suggestion; deviation raised and
    approved on the dead-code grounds): `core.basic` declares `Int`, so
    every program reaches it, and putting code there emits a dead
    `core/basic.{kt,rs}` into
    every output [mod-used-only]. Deliberately no `Result` alias — the
    union *is* the result. Every demo that hand-rolled the tags now uses
    std's, which is also what verifies the module end-to-end (the unions,
    qualifiers, and loops demos compile and run under `kotlinc`/`rustc`
    with `ok`/`err` imported from `salvo.core.result`).
    `crates/salvo-syntax/tests/corpus/qualifiers.sv` deliberately keeps
    its own `Ok`/`Err` and `type Result<S, T>` (user decision): it is a
    *parser* corpus — it also names an undeclared `Person` and `Pair` —
    so std's tags would buy it nothing.
- **Arrays get a std function surface (user decision 2026-09-02)** —
  `[type-array]`. Arrays already had literals, indexing, and native
  `for` iteration on both backends; only functions were missing, which
  is why LANGUAGE.md's `CyclicRandom` example (`values.size()` on a
  `T[]`) did not compile. New `core.array` module mirrors `core.list`
  minus construction (literals are the constructor) and mutation
  (fixed size): `size`, `get`, `first`, `iter`, with the same
  `<T canbe Linear>` opt-in pattern (measuring/iterating a linear array
  is fine; taking an element out is not). The example now compiles and
  runs verbatim on both backends.

- **Structured diagnostics [diag-structured] + `salvo analyze`
  [cli-analyze]**: errors are `FileDiagnostic` (file index, span,
  severity, message) rendered only at the consuming boundary; `analyze`
  runs the front half of the pipeline with text or JSON output. Analysis
  is *backend-neutral* (`--backend` only opts define files into parsing).
  Parse-broken files participate with recovered ASTs but contribute only
  their parse diagnostics — one broken file never suppresses diagnostics
  elsewhere.
- **`salvo lsp` [cli-lsp]**: LSP over stdio (`lsp-server`/`lsp-types`,
  sync); no incremental state — every document event re-runs
  whole-workspace analysis with open buffers as an overlay. Diagnostics
  (with clearing publishes), markdown hover [doc-markdown] — doc comments
  for fns and structs (with a per-field section) [doc-comment]
  [doc-struct-fields], `[symbol]` references linked to their declarations
  [doc-symbol-ref], fn signatures with effective (inferred) deductions
  [fn-ref-table], and a variable's flow-narrowed type
  [doc-hover-narrowed] — and import-fix code actions
  [diag-import-suggest].
- **VS Code extension + `salvo lang tm-grammar` [cli-lang]**: `vscode/`
  bundles a grammar *generated by the compiler* from the lexer's keyword
  table (tests fail if the checked-in grammar or keyword categories
  drift); `salvo.serverPath` points at a locally built binary.
- **Source discovery [mod-ignore]**: the walk skips hidden directories,
  `CACHEDIR.TAG` directories (Cargo's `target/`), and `.svignore`
  entries; the root itself is exempt.
- **Import suggestions [diag-import-suggest]**: unresolved
  handler/effect/import diagnostics carry `module.Item` suggestions from
  a whole-program declaration index; rendered as `help:` lines, JSON
  `imports`, and LSP quickfixes. std gained `random`
  (`DefaultRandom`), the first non-`core` module — and exposed
  [kt-handler-template-return]: value-returning handler-member templates
  emit `return run { … }`.
- **Numeric literal suffixes [lit-numeric]**: `1` Int, `1L` Long, `1.2`
  Double, `1.2f` Float; decision: `f` requires a decimal point (`1f` is
  a lex error). Kotlin renders native suffixes; Rust renders explicit
  types (`1i64`, `1.2f32`).
- **Missing-return [fn-must-return]**: non-`None` fns must return on
  every path (syntactic; loops never count; yield-fns exempt).
- **Predicate-qualifier constructors [qual-ctor-predicate]**: the
  constructive-only restriction was lifted; a predicate constructor
  asserts its predicate by construction.
- **Deduction entry forms [deduce-syntax]**: bare `[list]` keeps *all*
  declared qualifiers (semantics change from "keep none"),
  `[list: Mut]` keeps exactly the listed, `[list:]` strips all
  (`Deduction.explicit` flag).
- **Use-after-consume [deduce-consume]** — the largest post-M8 feature,
  built up across several user decisions:
  - Consumed values narrow to `Ty::Nothing` (decision: `Nothing` *is*
    the marker — "a value that no longer exists is an impossibility");
    referencing one is an error; assignment revives.
  - `check_program` runs *two rounds* so inferred deductions are
    enforced at call sites exactly like declared ones (round one checks
    + infers; round two re-checks with the inferred facts injected,
    then re-infers). Round one's diagnostics are discarded (checking is
    deterministic). Known non-convergence: round two's narrowing can
    change overload resolution, whose re-inferred deductions are not
    fed back again (no third round); acceptable at current scale.
  - Decision: consumption is *uniform across all types* (a rejected
    alternative exempted backend-copyable scalars; consistency of the
    abstract contract won). Made livable by a **branch-aware** analysis:
    per-branch snapshot/isolate/merge (`snapshot_narrows`/
    `restore_narrows`/`merge_fallthrough` + `block_always_exits`) —
    always-exiting branches contribute nothing, a value consumed on any
    fall-through path stays consumed (maybe-moved, as in Rust),
    disagreeing states keep only common qualifiers.
  - Kept parameters shed their removal set (declared − kept) from the
    argument's narrowed type, so a second `remove_first` after
    `[list: Mut]` stripped `NonEmpty` fails overload resolution.
  - Consumption survives `is`-narrowing restores (`Nothing` is skipped
    on restore), and loop bodies are re-checked once with their exit
    state as entry when the first pass changed anything
    (`check_loop_body`) — back-edge use-after-move surfaces like
    rustc's "moved in previous iteration" (second-pass duplicates
    deduplicated by file/span/message; value results discarded).
- **Union-arm arguments [type-union]**: fixed `unify`'s match-arm order
  so `describe(ok("x"))` resolves against `Ok Str | Err Str` (see
  Gotchas).

### S1 — shared fate, strict checker-only (completed 2026-09-01)

The first stage of the shared-fate roadmap (see the L1 section below for
the decided model). What landed:

- **`internal fn copy<T>(value: T) -> [value] T`** in `std/core/basic.sv`
  [internal-fn] [copy-fn]: parser already accepted `internal fn`
  (body-less like `external`); the declaration flows through
  Symbols/resolve/checker unchanged — the `[value]` deduction is the
  whole checker contract. Both emitters intercept
  `backing == Internal` before define-template lookup and lower the
  call from the checker's resolved argument type: Kotlin [kt-copy]
  identity for transitively immutable types, `.toMutableList()` /
  `.copy()` / `.copyOf()` for `Mut List` / `Mut` structs / arrays,
  codegen error for nested mutability and unknown/generic types; Rust
  [rs-copy] `.clone()` on the argument's place.
- **Fate links in the checker** [fate-link]: `LocalVar` gained
  `id`/`links`/`poison`; links are directed, flattened to roots at the
  binding, whole-variable. Creators: `let`/assignment from a bare
  identifier or projection chain, destructuring, `for` bindings,
  `is`/`when` bindings. Snapshot/restore/merge and the loop re-check
  carry the full `VarState`; links union across branch merges.
- **Poison rules** [fate-poison]: a `Mut`-kept call argument, projection
  assignment, or `++` on a root — and moves and whole-variable
  reassignment of it — poison its derived variables (`Nothing` + a
  recorded reason; the use-site error names the root, the event, and
  the `copy` remedy). Derived variables are read-only
  [fate-derived-readonly]: moving (consuming call, `return`, `break
  value`, `yield`) or mutating one errors at the site.
- **deduce.rs**: `let`/assignment of a bare parameter is no longer
  inferred as a move — it links; sound because every escape of the
  derived variable is a checker error until `copy` intervenes.
- **[struct-mut] is now enforced** at field-assignment sites (it was
  spec'd but unchecked, and became load-bearing: Kotlin's identity-copy
  is only correct if non-`Mut` values really are immutable). Arrays
  stay index-assignable without `Mut` (status quo; `copy` does a real
  array copy). Fixed a LANGUAGE.md spec bug the enforcement exposed:
  the `Mut` example mutated `person` instead of `mutable_person`.
- **Rust emission**: fate-linked bindings clone — `let`/assignment
  values and `for` iterables that are bare identifiers of owned
  non-Copy locals emit `.clone()` instead of moving (the checker keeps
  both sides readable). This also closed the old "`let a = b` moves
  local `b`" rustc-rejection leftover. Kotlin emission unchanged
  (aliasing is unobservable because mutation-after-link is rejected).
- Deliberately not tracked yet (later stages): non-identifier call
  arguments in *moved* positions (physically a clone today;
  kept-`Mut` positions *are* tracked since the 2026-09-02 parity fix —
  they mutate their provenance roots), lambda captures, and S2's
  move-mode relaxation. (Literal stores, spread, and `use`
  handler-constructor arguments landed in L2 — see the next section.)

### L2 — remaining consuming sites (completed 2026-09-02)

Every remaining move event now feeds the same consumption lattice as
call-site moves (mirroring the [deduce-infer] move list): storing a bare
identifier in a struct/array/tuple literal, spread `...n` (struct-literal
spreads and any `Expr::Spread`), `return n`, `break n`, `yield n`, and
`use Handler(n)` constructor arguments. One helper (`fate_move`) handles
all of them: a derived variable errors at the site
[fate-derived-readonly], a root is consumed (`Nothing`) and poisons its
derived variables [fate-poison], exactly like a call. What's worth
knowing:

- **Loop exits merge break-path states.** `LoopCtx` captures a
  `NarrowSnapshot` at every `break`; the `while`/`for` checking merges
  them with the fall-through exit state via `merge_fallthrough`. Without
  this, a `break s` inside an `if` was invisible after the loop (the
  always-exiting branch contributes nothing to the merge *inside* the
  body — correct there, but the loop exit is precisely where break-path
  state lands). Merge order puts the current (fall-through) snapshot
  first so the shorter frame stack drives the merge (break snapshots
  carry extra inner frames; for `for` loops the merge runs after the
  binding frame is popped).
- **Diagnostics name the event**: `LocalVar`/`VarState` carry
  `consumed_by: Option<&'static str>` ("a literal store", "a `...`
  spread", "a `break`", "a `yield`", "a `use` handler registration",
  "an earlier call"), threaded through snapshot/restore/merge like
  poison, cleared by reassignment revival.
- **`yield` + back edge works for free**: the existing loop re-check
  reports the second-iteration use at the `yield` itself.
- **L2a decided (user, 2026-09-02)**: string interpolation is a *read*
  — `"${n}"` never consumes. Spec'd under [type-str], cross-referenced
  from [deduce-consume]. Parity-sound because both emitters render the
  interpolated value as an owned copy purely for formatting.
- **Parity audit** (per the backend-parity principle): all new sites
  render through `emit_expr` = owned rendering in the Rust backend, and
  an owned non-Copy local emits as a bare place — a *physical move* —
  so bare-ident consumption is faithful-emission parity and closed real
  rustc-rejection gaps (`let t = (s, 1)` then `read(s)` was
  checker-clean but rustc-rejected before L2). Struct-literal spread was
  a latent *mutable-data* parity hole: Kotlin emits shallow `.copy()`
  (aliases `Mut` fields) while Rust deep-clones (`..base.clone()`) —
  `let p2 = Person {...p}; add(p.tags, 2)` would print different values
  per backend. Closed by restriction: the spread consumes `p`. The one
  finding left open — *projection* values in moved positions (literal
  stores `Box {item: h.tags}`, moved-position call arguments), where
  Rust cloned and Kotlin aliased, observable for mutable data and *not*
  caught by rustc — was accepted as a known live divergence (user
  decision 2026-09-02) and closed by S2 the same day [fate-move-mode].

### S2 — move-mode bindings (completed 2026-09-02)

The relaxation stage of shared fate: bindings have *modes* inferred
from downstream flow. What landed (rule [fate-move-mode]):

- **Mode inference rides the two-round architecture.** Round one is
  strict S1; `error_derived` — the single choke point for every derived
  move/mutation — records the *whole bind chain* as move-mode
  candidates (links now keep their original bind spans when flattened,
  so a variable's links describe the full derivation chain even after
  intermediate variables die) plus parameter *claims*. Round two
  applies the modes at every bind event (`declare_var` and assignment
  re-links): all live ancestors owned → consume them at the binding
  (poison names the binding, `FateEvent::BoundAway`), the binding
  carries no links, and the event is recorded in
  `Checked::binding_modes`.
- **Claims make the flagship work.** A move-mode binding reaching a
  parameter of an *inferable* fn claims it as moved;
  `deduce::infer(program, checked, claims)` seeds claims after every
  body inference (monotone). Written-kept parameters block claims: the
  binding stays borrow-mode and the S1 error stands at the move site.
- **Moved-position projections closed the accepted parity divergence**
  (the S2 obligation): a projection of *transitively mutable* data
  (`ty_transitively_mut`, following struct fields with a visited set)
  in a moved position consumes its owned roots
  (`Checked::moved_projections`; for-binding roots flip the loop to
  by-value) or errors for a written-kept parameter root ("cannot move
  mutable data out of `h`", remedy `copy`). The 2026-09-02 probe
  (`wrap(h.tags)` then `add(h.tags, 9)`) is now *rejected* — verified.
  Immutable projections stay untracked by design (unobservable).
- **Rust emission is faithful**: move-mode bind events emit raw places
  (real moves, partial for projections), move-mode loops iterate by
  value, tracked projections render without clones. The flagship
  pipeline (`longest_name` with inferred deductions) emits **zero
  clones**, rustc-compiles, and prints identically on both backends
  (verified end to end). Kotlin emission unchanged. `is`/`when`
  move-mode bindings still clone on Rust (restriction-valid).
- **Pre-existing false positive fixed**: a `for`-loop binding consumed
  in the body (`for s in xs { consume(s) }`) errored on the back-edge
  re-check — bindings now go through `check_loop_body`'s per-pass
  bindings channel (`pattern_bindings`), so each pass re-declares them
  fresh (an iteration binds a new element).

### L3 + L4 — convergence, same-call ordering, lambda captures (completed 2026-09-02)

- **L3 same-call ordering [deduce-same-call]:** within one call, a later
  argument may not mention a value an earlier argument consumed
  (`f(a, a)`, `f(a, size(a))`) — argument typing precedes contract
  enforcement, so the contract loop now tracks what this call consumed
  (`consumed_here`, fed by bare-ident moves and `projection_move`'s
  returned root names) and mention-checks every argument
  (`expr_mentions`, a full expression/block walk). Nested calls were
  already ordered (consumption applies during argument typing).
- **L3a decided (user, 2026-09-02): iterate to a capped fixpoint
  [deduce-fixpoint]** — option (iii): extra checking rounds run only
  when the driving facts changed (inferred deductions, move-mode
  candidates, claims), capped at four; stable programs stay at two
  rounds and identical cost; a program unstable at the cap gets a
  deterministic error naming the oscillating fns with the
  write-the-list remedy. Candidates/claims grow monotonically, so late
  discoveries converge — a move-mode candidate first seen under
  round-two narrowing is now *applied* in round three (the diagnostic
  moves from the raw derived-move error to the true site). The cap
  error is direct code but untested: constructing a genuine overload
  oscillator is an open exercise.
- **L4a decided (user, 2026-09-02): lambdas are ordinary values under
  shared fate [fate-lambda]** — superseding the recorded
  captures-copy recommendation after the emitter audit (plain borrow
  closures on Rust; Kotlin aliases; mutate-after-capture was a rustc
  E0502, not a silent divergence). Per-capture classification from the
  body, bound at creation: immutable reads free; mutable reads link
  the closure to the variable (root mutation poisons it — the E0502
  class becomes a Salvo diagnostic); mutated captures consumed at
  creation (kept-param → error, inferable param → claim); consuming a
  capture is always an error (multiplicity untracked). **No emitter
  changes**: borrow-captures alias on both backends, so parity is
  direct, and checker-legal programs pass NLL (verified end to end —
  identical stdout). Implementation rides the existing event
  machinery: a `lambda_ctx` boundary stack, capture recording in the
  Ident read arm, mutation marking in `fate_mutation`, consumption
  guards at the four consuming sites, and `finish_lambda_captures`
  applying the contract; lambda values get links via `links_for_value`
  and `Checked::lambda_captures` is exported for tooling/emitters.
  Discovered en route: PROGRESS previously overstated "captures are
  completely untracked" — body *consumption* already applied inline at
  creation; the new guard turns that into the multiplicity error.
- Known loud leftover [fate-lambda]: returning/storing a
  capture-carrying closure is a rustc lifetime error the checker does
  not reject; the recorded refinement is `move`-closure emission with
  hoisted clones, pending a treatment for captured effect-handler
  locals. Fn-type contracts (deductions/effects on `Ty::Fn`, the
  named-fn mode mismatch, closure double-use) remain deferred to the
  L7 parameterized-qualifier work.

### S3 — borrow emission (completed 2026-09-02)

The final shared-fate stage: borrow-mode bindings become real Rust
borrows [rs-borrow-locals]. What landed:

- **`&T` locals via `BindKind::Ref` reuse**: a borrow-mode `let` from a
  *pure place* (bare ident / field / index chain; no coercion,
  narrowing unwrap, or field cast; name never reassigned; bind event
  not move-mode) emits `let mut n = &person.name;` and registers as a
  reference binding — the entire existing kept-parameter rendering
  (owned reads clone, borrow positions pass bare, Copy derefs) then
  applies unchanged. Already-`&` roots pass the reference through;
  `&mut` roots reborrow (`&*x`).
- **By-reference loops**: a borrow-mode `for` over a pure-place
  iterable with a plain ident binding iterates without cloning the
  collection (`for person in persons` where `persons: &Vec<Person>`),
  the loop variable itself a reference binding. Guarded to concrete
  non-union element types — union/optional elements go through
  `matches!`/unwrap lowering that expects owned subjects and keep the
  clone path.
- **Decision S3a resolved as emission, not semantics**: a mixed join
  (linked on one path, independent on another) simply keeps today's
  owned/clone emission — since S1, links union across branches and
  poison covers every observation, so the clone is restriction-sound;
  forbidding would have added errors with no parity need, and `Cow`
  buys nothing. No program's legality changed anywhere in S3.
- **Borrowck alignment**: checker-legal programs pass NLL because
  poison forbids using a derived value after its root is mutated,
  moved, or reassigned — so every borrow's last use precedes the
  conflicting event. Known loud exception (documented, rare shape): a
  single call that passes a borrow-emitted local *and* moves its root
  (rustc E0505; the checker's left-to-right argument model accepts
  it). Loud, never wrong.
- **Exit criterion verified**: the moved-position parity probe is
  still rejected after the emission change, and the read-only pipeline
  demo compiles and prints identically on both backends with *zero*
  clones in the Rust output (`count_long`: borrowed param, borrowed
  loop, borrowed field binding).

### L6 — must-use linearity (completed 2026-09-02)

All five decisions (L6a–e) approved by the user as recommended; rules
[linear-canbe] [linear-obligation] [linear-discard] [linear-composite]
[linear-generics] [linear-lambda] [linear-static]. What landed:

- **`canbe Linear`** on type declarations (the auto-qualifier clause
  already parsed arbitrary auto-qualifiers; `has_auto_linear` mirrors
  `has_auto_mut`); `Linear` in a use-site type is an error — linearity
  is declared, not applied. `ty_transitively_linear` /
  `ast_type_linear` mirror the `Mut` transitive analysis (composites
  are contagious).
- **Obligation checks ride the existing flow machinery**:
  `owes_linear` (declared-linear + live + no links + owned-param rule
  via `own_contract`), scanned at frame pops (`check_linear_frame_drop`
  in `check_branch_block`/`check_fn`/`check_lambda`), at
  `return` (all frames) and `break`/`continue` (frames above the
  loop's `entry_depth`, new `LoopCtx` field) via `check_linear_exit`,
  at linear-typed expression statements, and at assignment over a live
  linear value. The all-paths rule lives in `merge_fallthrough` (which
  gained a `span` parameter): consumed on some fall-through paths but
  not all = error — the exact dual of maybe-moved. Reported variables
  are marked consumed (one error per obligation). Gated to round two+
  (`inferred.is_some()`), like the other contract-dependent checks.
- **`internal fn discard<T>(value: T) -> [] None`** in std; the `[]`
  deduction makes the discharge just another move. Rust lowers to
  `drop(value)`, Kotlin to `(value).let {}` [internal-fn]. Both
  verified end to end with identical stdout on the open/use/close
  resource demo.
- **Generic ban** in `resolve_named_call` on the resolved substitution:
  linear instantiation of an unconstrained `T` errors; `copy` refuses
  with its own message; `discard` (internal, by name) is blessed.
  Known leftover: effect members with their own generics are not
  covered by the ban.
- **Lambda guard**: capture-and-mutate of a linear value errors in
  `finish_lambda_captures` (the closure would swallow the obligation);
  read captures are aliases and fine.
- `LocalVar` gained `decl_span`, so obligation diagnostics point at the
  variable's declaration.
- Practical consequence (documented): a Salvo-bodied consumer
  (`fn close_file(h: FileHandle) -> []`) must itself end the chain with
  `discard(h)` — real resource release lives in external fns, which
  have no body to check. `List<FileHandle>` is expressible but not
  constructible until a generic opt-in exists (L7).

### L7a — generic linear opt-in `<T canbe Linear>` (completed 2026-09-02)

Syntax decision (user, 2026-09-02, option C of the explored set): the
opt-in reuses the `canbe Linear` phrase on *type parameters* — one
qualifier per `canbe`, comma separates parameters; struct-side syntax
deferred. (Both sites were spelled `with` until the 2026-09-03 rename
[canbe-optin].) What landed (rule [linear-generics] rewritten):

- **Parser**: `parse_generics_canbe` parses `<T canbe Q, U>` into
  `FnDecl.generic_canbe: Vec<(Ident, TypeRef)>` (fn declarations and
  define signatures); non-fn declarations report "`canbe` on a type
  parameter is only supported on functions". Uniform snapshot churn
  (new FnDecl field) accepted.
- **Checker**: `own_linear_generics` set per fn; `Ty::Var(name)` joined
  the transitive linearity analysis — so opted bodies are checked with
  `T` linear (a written-moved parameter the body drops is a leak), and
  forwarding an opted `T` to an unopted generic fails the ban
  compositionally. The ban lift replaced the discard-by-name blessing;
  `copy` keeps its dedicated refusal. Only `Linear` is accepted in the
  clause.
- **Variadic guard**: linear values are refused in variadic positions
  (untracked — the value would be physically moved but statically still
  owed); the audit found this while opting in `list`/`mutable_list`,
  whose variadic constructors would otherwise have leaked obligations.
  Empty construction + `add` is the supported pattern.
- **std audit**: `add`, `list`, `mutable_list`, `size` opted in;
  `discard` re-declared as
  `internal fn discard<T canbe Linear>(value: T) -> [] None`; `get`
  deliberately *not* opted (returns an alias of an element — a clone of
  a linear value would duplicate the obligation); `copy` refused.
- Verified end to end: the `List<FileHandle>` workflow (construct
  empty, `add` individually, `size`, `discard`) compiles and prints
  identically on both backends.

### L7b — `Once` fn types (completed 2026-09-02)

The call-multiplicity qualifier, landed as a *fn-type qualifier* rather
than the originally-sketched deduction-list surface (user decision
2026-09-02 — calling once is consuming, which contradicts a deduction
entry's kept-ness; the type-qualifier form rides the existing
machinery). Rule [once-fn]:

- **Enforcement is consumption**: calling a `Once` value consumes it —
  double calls, loop back-edge calls, and call-after-escape are the
  ordinary consumed-use errors; maybe-calls are conservative; zero
  calls fine. One fix en route: the Ident-callable path in `check_call`
  bypassed the standard consumed-read error (it never `check_expr`s the
  callee ident), so calling an already-consumed callable reported
  nothing — it now routes through the standard error.
- **Inverted subtyping, flagged for future review** (user request):
  plain fn <: `Once` fn, and `Once` may never be dropped — the
  opposite direction of every other qualifier, special-cased in
  `is_subtype` and `unify` with loud comments.
- **Consuming-capture lambdas legalized**: [fate-lambda]'s always-error
  became "legal but `Once`-typed" — the capture is consumed at
  creation, the closure fits only `Once` positions (boundary error
  otherwise), and every *enclosing* lambda is marked too (an outer
  closure re-creating an inner consuming one re-consumes per run).
  Linear captures still refuse (exactly-once closures are future
  work). The escape rule consumes a `Once` value passed as any
  argument (fn-value ownership is otherwise untracked).
- **Emission nearly free**: Rust emits `impl FnOnce(…)` for `Once`
  params (`QualifiedGroup` over `Fn` in `emit_type` + the
  fn-param-Owned mode extended to qualified groups); lambda emission
  unchanged (rustc's capture inference produces `FnOnce` closures
  itself). Kotlin erases `Once` entirely. Verified end to end with
  identical stdout.
- No inference in v1 (written `Once` only); the parser needed nothing —
  `Once () -> None` already parsed as a qualified group over a fn type.

### L7c — derived returns `ReadOnly[from: p]` (completed 2026-09-02)

The relaxation of S1a: zero-copy accessors across fn boundaries. Rule
[readonly-return]; surface decided by the user (square brackets — angle
reads as generics, round collides with qualified groups, square is
where annotations already name parameters). What landed:

- **Parser**: `parse_derived_return` recognizes
  `ReadOnly[from: param]` between the deduction list and the return
  type in both fn parse paths; stored as `FnDecl.derived_return`
  (never enters the type AST — mirroring the checker design, where the
  fact becomes links at the boundary and the result's *type* stays
  plain, so overloading is untouched).
- **Checker**: validation (parameter exists, *kept* — written or
  inferred), return-provenance validation (every returned value's link
  chain terminates at `p`, `None` free, forwarded derived calls
  validate through the same links), and returns in derived fns do not
  consume. Caller side: `Checked::derived_calls` (call span → argument
  index) makes `links_for_value` link results to arguments.
- **Borrowed links close the S2 interaction**: probing found move-mode
  would have "taken ownership" of a *physically borrowed* result
  (`take(h)` after narrowing `first(...)`'s result compiled to moving
  out of a `&`). `FateLink` gained a `borrowed` flag, set through
  derived calls and propagated through derivation chains;
  `apply_binding_mode` refuses move-mode over borrowed links, so the
  S1 error with the `copy` remedy stands.
- **Rust emission**: `&T` / `Option<&T>` returns; lifetime elision for
  a single reference parameter, mechanical `'a` generation onto the
  annotated parameter and return when there are more — the first
  deliberate exception to the no-lifetimes invariant. Return values
  render as borrows (`Some(&place)`, bare for `&` bindings,
  pass-through for forwarded derived calls). Caller-side narrowing
  works without emitter changes (`Option<&T>` is `Copy`; the
  `.clone()` on a `&&T` copies the reference — the
  `suspicious_double_ref_op` lint joined the generated allow list).
  std's `first` dropped its `.cloned()` — clone-free — and gained the
  annotation. Kotlin: zero changes.
- **v1 scope recorded**: plain `T` and `T?` shapes; fn declarations
  only; accumulator bodies (`best = person; …; return best`) rejected
  by the provenance validation — reassignable borrowed locals are the
  recorded refinement.

### L7d — fn-type contracts (completed 2026-09-02)

The user-directed redesign of the original "piece 3": instead of a
fixed all-moved convention, fn types carry *declared* contracts —
default keeps-everything, so nothing broke and the parity hole closed
by faithful emission. Rule [fn-contract]:

- **Surface**: fn-type parameters may be named and a standard deduction
  list may follow the arrow (`(v: List<Int>) -> [] Int`). `Type::Fn`
  gained `param_names`/`deductions`; `Ty::Fn` gained
  `contract: Option<Vec<FnParamContract>>` (a types.rs struct — kept
  out of `Display` to avoid message churn).
- **Checker**: `apply_fn_value_contract` mirrors the named-call
  contract loop (consumption, kept-`Mut` mutation events, qualifier
  shedding, same-call ordering, Once escape, capture/kept guards) at
  fn-value call sites; lambdas checked against a contract mark kept
  parameters `lambda_kept` (never consumable — guarded at all four
  consuming sites plus binding modes); named fns passed by value build
  their contract from written/inferred deductions; `contract_fits`
  (keeps <: consumes, inverted like [once-fn] and flagged with it)
  joined `is_subtype` and `unify`. One enabling fix: single-candidate
  named calls now type *lambda literal* arguments against the callee's
  parameter types upfront, so expected fn-type contracts actually
  reach `check_lambda` (multi-candidate calls keep the untyped probe).
- **deduce.rs**: calls through fn-typed *parameters* of the walking fn
  apply that parameter's written contract, so consuming contracts
  propagate interprocedurally (`caller_loses` sees its argument die).
- **Rust emission**: fn params render `&mut impl FnMut(…)` (closure
  double-use fixed by faithful emission; `FnMut` accepts
  handler-mutating closures; `Once` stays `impl FnOnce`), argument
  types per contract, call-site arguments per
  `Checked::fn_value_calls`, lambda bindings/annotations per
  `Checked::lambda_contracts`, and named fns wrap in mechanical
  adapter closures. **Kotlin**: contracts erase; named fns emit
  `::name` function references (that pass was silently broken on
  Kotlin too — `count` bare emitted a call-less identifier kotlinc
  rejects).
- Deferred, recorded: fn-type *effect* lists (parse, lexical env
  meanwhile), `Once` inference, per-parameter written contracts
  beyond kept/moved/quals.

### Current architectural facts worth knowing

- **`ReadOnly` presentation (landed 2026-09-02)**: reads of fate-linked
  variables record their links into `Checked::fate_reads` (root name +
  bind span per link [fate-link]); the LSP hover renders such a
  variable as `ReadOnly T` with the qualifier's parameters (roots,
  binding sites, `copy` remedy) as detail below the type line —
  progressive disclosure per user decision. Presentation-only:
  `ReadOnly` is not in the type system and cannot be written. It is
  phase 1 of the parameterized-compiler-qualifier design recorded
  under L7.
- **Resolution/checking pipeline**: `emit_program` runs
  `Symbols::collect` (flat, still used for define templates and arity
  fallbacks) → `salvo_core::resolve` (per-file scopes) →
  `salvo_core::check_program` (two rounds + deduction inference, see
  [deduce-consume]). Type errors are structured `FileDiagnostic`s
  [diag-structured]; they abort emission and are rendered at the backend
  boundary into `BackendError::Codegen` strings (the CLI `analyze`
  command consumes them structured instead).
- The checker assumes it can **see everything** (2026-09-03): calls,
  fields, subscripts and `for` subjects must be justified by declarations
  ([call-resolve], [field-resolve], [index-resolve], [iter-resolve]).
  What stays lenient is *inference*: a type it could not work out is
  `Ty::Unknown`, compatible with everything, so one mistake yields one
  diagnostic. Coercions/unwraps only fire where the tables say so — the
  emitter's syntactic paths remain the fallback everywhere else, which is
  what keeps a checker regression degraded rather than wrong.
- Wrapper-union arm identity is positional over the **declared** type's
  non-`None` arms; narrowing never re-wraps a variable in place (uses are
  unwrapped/re-wrapped at expression sites instead).
- Modules are emitted to `<module/path>.kt` / `<module/path>.rs`.
  `unions.kt` / `unions.rs` is emitted whenever any wrapper size is used
  by an *emitted* file.
- Only reachable modules that produce code are emitted [mod-used-only]
  (`reach.rs`: name-usage edges over `ModuleScope::name_origins`); each
  module gets its own Kotlin package `salvo.<module.path>` with generated
  imports [kt-package] [kt-imports]; companions copy verbatim
  [backend-companion].
- Deductions (`-> [list: Mut] T`) are inferred/validated by the
  `deduce.rs` post-pass and stored in `Checked::deductions`; the Kotlin
  backend ignores them, the Rust backend derives its parameter modes from
  them (kept = borrow, omitted = move [rs-borrows]); the checker enforces
  them flow-sensitively at call sites [deduce-consume].
- **Emitter effect-environment fallback (deliberate, revisit later)**:
  the emitters' effect environments are string-keyed; at each site they
  first consult the checker's `use_effects`/`effect_calls`/`call_effects`
  tables (rendered through `kotlin_ty`, which must agree with `emit_type`
  on the same source type) and fall back to string/base-name matching
  only when the table has no entry or the type contains `Unknown`. The
  fallback keeps the lenient-checker contract: a checker regression
  degrades to string matching rather than wrong code. Cost: double
  bookkeeping. When the emitters key their environments by checker `Ty`
  directly, the string env can be deleted.
- The two emitters deliberately share their architecture (side-table
  access, `emit_expr` = base + coercion, fallback paths, is-binding and
  loop lowering shape). When a lowering rule changes, check both crates —
  and the checker, which must agree with them on the ident-unwrap
  predicates (`maybe_coerce`'s "effective repr").

## Roadmap: toward full linear types

Where we are: an *affine* analysis ("use at most once") with solid
underpinnings: interprocedural contracts (inferred + validated
deductions), `Nothing`-narrowing with revival, and branch-/loop-aware
state merging. Since L2, the local flow analysis watches *every* move
event the deduction inference knows — call-site moves, literal stores,
spread, `return`/`break`/`yield`, `use` constructor arguments — for bare
identifiers; the remaining gaps are projection values in moved positions
(a parity question, see the L2 audit), lambda captures (L4), and the two
genuinely new mechanisms (places, must-use). The safety net holds:
holes surface as rustc errors on the generated code (loud, never
silently wrong); the one known exception — moved-position projections
of mutable data — was closed by S2 (see the backend-parity principle
note).

Each phase below is independently shippable, in rough dependency order.
Items marked **DECISION** need a language-design call before or during
implementation — everything else is analysis engineering under decisions
already made (uniform-across-types consumption, `Nothing` as the marker,
maybe-moved-is-unusable).

### Backend-parity principle (user decision 2026-09-01)

The operational semantics must be the *same* on both backends; divergent
representations are acceptable only where the difference is
unobservable. This principle governs every move/copy/borrow decision in
the stages below:

- **Immutable data**: clone vs shared reference is purely a performance
  question — free to address later, in any direction, at any time.
- **Mutable data**: a clone where Kotlin shares a reference is a
  *semantics* change, never just a cost. Parity must come from one of
  exactly two strategies:
  1. **Restriction** — the checker rejects every program that could
     observe the difference. This is what deductions and shared fate do
     today: S1's clone-emission is sound *because*
     mutate-after-link and mutate-through-derived are compile errors.
  2. **Faithful emission** — the Rust backend works harder, including
     emitting code shaped *differently from the source* when a
     mechanical equivalence justifies it. Recorded technique, **read
     redirection**: after `let name = person.name`, a later read of
     `person.name` with no intervening mutation is guaranteed equal to
     `name`, so Rust may emit the binding as a real (partial) move and
     redirect subsequent reads of the moved path to the surviving
     variable — no clone, no borrow, no lifetime; the fate-link table
     already knows the equivalence. The same idea generalizes to any
     place the checker can prove holds the same data as a live local.
  Silent clones of mutable data are *not* a valid parity strategy.
- **Parity bug closed (verified 2026-09-01, fixed 2026-09-02):** a
  *projection* passed directly to a kept `Mut` parameter is a mutation
  of its provenance roots — before the fix, `let t = h.tags;
  add(h.tags, 2); size(t)` compiled clean and printed 2 on Kotlin
  (alias) but 1 on Rust (clone). The call-site contract loop now runs
  `fate_mutation` on the provenance roots of non-identifier arguments
  in kept-`Mut` positions (mirroring projection assignment), so the
  derived variable is poisoned and the program is rejected; `copy` at
  the binding is the remedy. Audit the remaining untracked events
  (literal stores, `use` ctor args, interpolation, *moved*-position
  projection args) against this principle when L2 lands them.
- **Moved-position projection divergence — CLOSED by S2 (2026-09-02):**
  a *projection* of transitively-`Mut` data in a *moved* position (a
  consuming call argument, a literal store, a `use` ctor argument)
  cloned in Rust but aliased in Kotlin, and rustc did not catch it (the
  clone is valid Rust) — verified live with `wrap(h.tags)` then
  `add(h.tags, 9)`: Kotlin printed 2, Rust printed 1. S2 closed it
  [fate-move-mode]: such projections now consume their owned roots (the
  probe program is *rejected* — re-verified) or error for written-kept
  parameter roots with the `copy` remedy; tracked projections also emit
  as real partial moves in Rust. Projections of immutable data remain
  deliberately untracked (clone-vs-alias unobservable).

### L1 — Shared fate: links, poison, and `copy` (user decisions 2026-09-01)

Redesigned: replaces the original "aliasing bindings are moves" +
"reads are copies" plan. Shared fate is a lifetime-free borrow
discipline: a variable bound to the value or projection of another
*links* to it, reads flow freely through links, and ownership-requiring
operations (moves, `Mut` ops) consume the rest of the link group. The
motivating example: `longest = person.name` inside a loop over a *kept*
parameter `persons`, then `return longest` — invalid (moving a value
derived from a borrow); remedy `return copy(longest)` — one copy at the
escape instead of a clone per iteration.

The decided model:

- **L1a — `internal fn copy` (decided).** New `internal` item keyword
  for compiler-intrinsic fns:
  `internal fn copy<T>(value: T) -> [value] T` is declared in std (the
  signature + `[value]` deduction are all the checker needs:
  non-consuming, result independent), has *no define files*, and each
  emitter lowers calls to it type-directedly — Kotlin: identity for
  immutable data, real copies for `Mut`-capable types; Rust:
  `.clone()` (scalar identity as a later refinement). Unsupported
  types are codegen errors [backend-never-wrong]. Rationale: a define
  template is type-blind text per signature and cannot dispatch on the
  instantiated `T`; an intrinsic sees the checker's resolved argument
  type at each call site.
- **Shared fate (decided; supersedes L1b).** Links are *directed*
  (derived → root), *transitive* (`longest` → `person` → `persons`),
  and at *whole-variable granularity* (no place lattice). Link
  creators: `let`/assignment from a bare identifier or a projection,
  loop bindings, destructuring.
  - Reads never consume and never poison, on any member, any time.
  - Mutation events are already defined by the deduction system:
    Mut-kept call args and projection assignments (effect-handler
    capture deferred to L4).
  - A `Mut` op or move on the *root* poisons every derived member
    (narrows to `Nothing`, error at later *use*, revival by
    reassignment — the existing possibly-consumed machinery). The
    root stays usable after mutation. Poison is not retroactive:
    derivatives created after a mutation are fine. This is NLL-like
    precision without a liveness analysis (no later use → nothing
    fires).
  - **Bindings have modes**, decided by downstream flow:
    *borrow-mode* (derived value only ever read; ancestors stay
    usable) vs *move-mode* (derived value eventually moved/mutated;
    the *binding itself* consumes the ancestors — Rust partial-move
    semantics, emission-faithful, no hidden clones). Poison-at-move
    was rejected: it cannot be emitted faithfully without a hidden
    clone.
  - Moving or mutating a value derived from a *kept* parameter is a
    deduction-contract violation regardless of mode (you cannot move
    out of a borrow); remedy `copy`.
  - Uniform across all types (standing decision). On Kotlin the whole
    discipline is a purely static protocol (no runtime component); it
    rejects some JVM-fine programs by design, remedy `copy`.

Stages (each independently shippable):

- **S1 — strict, checker-only. ✅ Done 2026-09-01** (see the S1 section
  in the decision log above). Links + poison rules with derived
  members *read-only* (any move or `Mut` op on a derived member is an
  error at that site; remedy `copy`). Emission unchanged
  (clone-by-default), so nothing can physically break — the protocol
  lands before the performance. Includes `internal fn copy`
  end-to-end and diagnostics naming the link and the event.
  - **S1a — the function boundary (decided 2026-09-01):** call
    results never link to arguments — returned values are always
    independent; a fn returning a projection of a kept param must
    `copy` internally; links stay intraprocedural. Derived-return
    annotations are reconsidered in a dedicated late milestone (L7).
- **S2 — move-mode bindings. ✅ Done 2026-09-02** (see the S2 section
  in the decision log above; rule [fate-move-mode]). Binding modes
  inferred from downstream flow; move-mode bindings consume their
  owned ancestors at the binding and emit as real moves (zero-clone
  pipelines verified end to end); parameters are claimed into the
  deduction fixpoint; written-kept parameters keep the S1 errors.
  - **Obligation discharged**: the moved-position projection
    divergence is closed — projections of transitively-`Mut` data in
    moved positions consume owned roots / error for written-kept
    parameter roots (`copy` remedy); the parity probe is rejected.
  - Refinement (recorded, optional, still open): *read redirection*
    (see the backend-parity principle) can soften poison-at-binding —
    a read of exactly the moved path (`person.name` after a move-mode
    `let name = person.name`, before any mutation) is provably equal
    to the surviving variable, so the checker may allow it and the
    Rust emitter substitutes `name`. Keeps more source shapes legal
    without clones.
- **S3 — borrow emission (Rust). ✅ Done 2026-09-02** (see the S3
  section in the decision log above; rule [rs-borrow-locals]).
  Borrow-mode bindings from pure places emit `&T` locals; borrow-mode
  loops iterate by reference; Kotlin unchanged; clone fallback
  everywhere else (restriction-sound).
  - **DECISION S3a — resolved 2026-09-02 as emission, not semantics:**
    mixed joins keep the owned/clone emission — link-union + poison
    already reject every observation, so no forbid and no `Cow`; no
    program's legality changed. Read redirection remains a recorded
    future refinement.
  - Exit criterion verified: the parity probe is still rejected after
    the emission change, and the borrow demo prints identically on
    both backends with zero clones in the Rust pipeline.

### L2 — Remaining consuming sites. ✅ Done 2026-09-02

(See the L2 section in the decision log above.) All root-consuming move
events are tracked: literal stores, spread, `return`/`break`/`yield`
(with break-path states merged into loop exits, and the yield/back-edge
interaction handled by the existing two-pass loop analysis), and `use`
constructor arguments. The parity audit found bare-ident consumption is
faithful-emission parity (Rust already moved at these sites) and closed
the struct-spread shallow-copy-vs-deep-clone hole by restriction.
Remaining audit finding: projection values in moved positions cloned
in Rust and aliased in Kotlin — observable for mutable data only, and
*not* caught by rustc (the clone is valid Rust). Accepted as a known
live divergence (user decision 2026-09-02) and closed by S2 the same
day [fate-move-mode].

- **Priority item — done 2026-09-02**: the verified parity bug (see the
  backend-parity principle above) is fixed — non-identifier arguments
  in kept-`Mut` positions run `fate_mutation` on their provenance
  roots.
- **DECISION L2a — interpolation (decided 2026-09-02).** `"${n}"` is a
  *read*: reads never consume. Both emitters render interpolated values
  owned-by-clone purely for formatting (no reference retained), so the
  decision is parity-sound. Spec'd under [type-str] and cross-referenced
  from [deduce-consume].

### L3 — Same-call and convergence tightening. ✅ Done 2026-09-02

(See the L3+L4 section in the decision log above.) Same-call argument
ordering enforced [deduce-same-call]; checking iterates to a capped
fixpoint [deduce-fixpoint].

- **DECISION L3a — decided (user, 2026-09-02):** option (iii) — iterate
  only while facts changed, cap four rounds, deterministic instability
  error with the write-the-list remedy.

### L4 — Lambda captures. ✅ Done 2026-09-02

(See the L3+L4 section in the decision log above; rule [fate-lambda].)

- **DECISION L4a — decided (user, 2026-09-02):** lambdas are ordinary
  values under shared fate — per-capture classification from the body
  (read/mutate/move), contract bound at creation; immutable reads
  free, mutable reads fate-link the closure, mutated captures consumed
  at creation, consuming a capture always an error. The emitter audit
  superseded the recorded (b) recommendation: plain borrow-closures
  already alias on both backends, so no emitter change was needed.
  Option (c) — full capture/deduction contracts on fn types — remains
  the long-term design, folded into the L7 parameterized-qualifier
  work.

### L5 — Places and partial moves

Track paths (`x.field`, tuple/array elements), not just whole variables:
destructuring consumes its source; moving a field out leaves the struct
partially unusable. This is the largest analysis change (place lattice
instead of per-variable states).

- **L5a — answered by shared fate (2026-09-01):** partial moves exist
  as *move-mode bindings* at whole-variable granularity (L1/S2); the
  question left for L5 is only *field-disjoint precision* (using one
  field while another is moved/borrowed), a refinement with no current
  use case. Revisit only if whole-variable poison proves too coarse in
  practice.
- **Not L5: field smart-casting.** Place-based *type narrowing* (reads
  of `h.field` narrowed by `h.field is T`) is a separate feature from
  place-based ownership — it was roadmap phase **P1**, done 2026-09-03.
  L5 inherits its `Place` substrate: the projection type (fields *and*
  elements), the prefix/overlap relations, and per-root fact storage.

### L6 — Must-use: true linearity. ✅ Done 2026-09-02

(See the L6 section in the decision log above; rules [linear-*].) All
five decisions approved by the user as recommended on 2026-09-02:

- **L6a**: `canbe Linear` on the type declaration; `Linear` is not
  writable at use sites (every value of the type is linear, always).
- **L6b**: consumption = any move, as the deduction system defines it;
  moves transfer the obligation (compositional across calls, returns,
  stores, and move-mode bindings).
- **L6c**: `internal fn discard<T>(value: T) -> [] None` is the
  explicit escape hatch; early-exit paths are checked by the existing
  path machinery; panic/abort semantics out of scope until they exist.
- **L6d**: composites containing linear components are linear
  (transitive); unconstrained generics refuse linear instantiation
  (`copy` refused with a dedicated message, `discard` blessed);
  `where T: Linear`-style opt-in deferred to the L7 qualifier work.
- **L6e**: purely static protocol, identical on both backends, no
  runtime component.

### L7 — Reconsider derived-return annotations

Revisit S1a's "returned values are always independent" rule once
shared fate (S1–S3) and linearity (L6) have real usage. The question:
should deductions grow a derived-return dimension ("the result is
derived from parameter X" — lifetime elision by another name), so
zero-copy accessors (`first(persons)` returning a linked element)
work across call boundaries? Adding it later is purely a relaxation
(more programs expressible, nothing breaks). Evaluate against real
std/user code: if the intra-function `copy` costs never show up in
practice, independent returns may be the permanently right answer.

- **DECISION L7a** — whether to add it at all, and the annotation
  surface if so (e.g. `-> [persons] persons.T`-style vs a marker on
  the return type); every std external returning a projection would
  need auditing.
- **Leading design for L7a (user decision 2026-09-02): parameterized
  compiler qualifiers.** Supersedes the `-> [persons] persons.T`
  strawman. Compiler-inserted qualifiers form a distinct class — never
  affecting overload resolution/`unify`/mangling/erasure, not testable
  with `is`, not constructible, strippable only by blessed fns
  (`copy`, later `discard`) — and may carry *parameters* the checker
  uses for checking and diagnostics. `ReadOnly(from: root)` is the
  fate link as a type; `-> ReadOnly(from: param) T` in a signature is
  the derived-return annotation, giving exact root-naming (the caller
  links the result to that argument; Rust lifetime annotations are
  generated mechanically from the parameter — elision already covers
  the single-kept-param case). The same vehicle can later carry
  deduction contracts on `Ty::Fn` values (closing the
  named-fn-as-lambda mode-mismatch leftover) and L4 capture contracts.
  Representational discipline: parameters are var identities in flow
  state and param *names* in signatures, substituted at call
  boundaries (same shape as generic substitution). Adoption is phased
  so each step pays for itself:
  1. **Presentation (✅ done 2026-09-02)**: derived variables hover as
     bare `ReadOnly T`, with the parameters (roots, binding sites,
     `copy` remedy) as on-request detail — progressive disclosure per
     user decision; `Checked::fate_reads` carries the data. No
     type-system change.
  2. **Internal unification (when L7 starts)**: fold
     links/poison/consumed_by into parameterized qualifiers on the
     narrowed type with per-qualifier join direction (`ReadOnly`
     params union across branches; user qualifiers intersect);
     diagnostics gain LSP related-information spans from the
     parameters. Same programs accepted/rejected — done at L7 to avoid
     refactoring twice.
  3. **Signature transport (L7 proper)**: `ReadOnly(from: param)` in
     return position, call-site substitution, mechanical Rust lifetime
     generation; std externals returning projections audited then.
  Open decision points for when L7 starts: the writability boundary
  (readable everywhere; writable only in return position first?),
  per-qualifier join declarations, and whether `Nothing` gains
  parameters (recommended: yes — strictly more informative, revival
  unchanged).
- **L7a delivered 2026-09-02**: the generic linear opt-in
  `<T canbe Linear>` (see the decision-log section; syntax option C —
  `where` clauses and qualifier-prefix forms were explored and
  declined; body-inference recorded as a possible later complement,
  mirroring the deduction precedent: written validates, unwritten
  infers).
- **L7c delivered 2026-09-02**: derived returns `ReadOnly[from: p]`
  (see the decision-log section) — the L7a-recorded bare-`ReadOnly`
  idea matured into the parameterized square-bracket surface; the
  internal qualifier unification was *not* needed (links carry the
  fact across the one boundary it crosses).
- **L7b delivered 2026-09-02**: the `Once` call-multiplicity qualifier
  (see the decision-log section) — landed on *fn types* rather than in
  deduction lists (the surface originally sketched here; user approved
  the change): `fn run(f: Once () -> None)`, enforcement by
  consumption, inverted subtyping (flagged for review),
  consuming-capture lambdas legal and `Once`-typed, Rust `impl FnOnce`.
  Inference (a callee auto-promoting a once-called fn param) remains
  open for the fn-type-contracts sub-phase.

Sequencing note: L1 lands in stages (S1 strict checker-only → S2
move-mode bindings → S3 borrow emission); S1+L2 closed real
rustc-rejection gaps; L3 and L4 are done (2026-09-02);
L5 is largely subsumed by shared fate (field-disjoint precision only);
L6 landed 2026-09-02 with its own LANGUAGE.md section and the
[linear-*] rule family. Per AGENTS.md, each phase lands with
LANGUAGE_SPEC.md rules and tests at every affected layer.

## Roadmap: effects

### E1 — Effect-to-effect dependencies (✅ complete 2026-09-04)

A handler may depend on another effect. [effect-member-no-effects] still
forbids *members* declaring effects, because dispatch goes through the
handler instance and the call site has no way to thread extra handler
arguments; a handler's dependency is declared on the handler instead.

**Where the dependency is declared changed during design** (user decision
2026-09-04). The first sketch put it on the *effect* (every handler
mirroring it exactly, as a `define fn` mirrors an external). The user's
objection stands: dependencies are a property of *implementations* —
`ConsoleLogger` needs a Console, a null logger needs nothing — and an
effect declaring them forces every handler to pay for the union. So the
dependency is a handler **constructor parameter of effect type**:

```
handler ConsoleLogger(console: Console) of Logger {
    fn log(message: Str) -> [message] None { print(message) }
}
```

This already parses and type-checks (handler ctor params exist; only the
Rust emission of handler-typed values was broken — see the prerequisites in
E1a), so the declaration side of E1 needed almost no new syntax. What it
needed was: an effect-typed ctor param resolved from the ambient `use` set,
those effects placed in the member bodies' effect environment, and the
fusion emission ([rs-effect-fusion] / [kt-effect-fusion]) — all of which
landed 2026-09-04, on both backends, verified by running the same programs
under kotlinc and rustc to the same stdout.

- The mirroring principle still applies where the compiler cannot see an
  implementation: a handler must implement every member of its effect, and
  an `external handler`'s templates are trusted exactly like a `define
  fn`'s — declaration is the contract, no inference.
- Effect *member* deduction contracts (declared on the member, applied at
  call sites) landed with [decl-explicit], the deduce pass reads them
  ([call-resolve]), and handler bodies are validated against them
  ([deduce-infer], [effect-state-store]).
- Three cuts remain on the Rust side, all reported: a dependent handler
  using its own generic parameters in a member signature, a `use` whose
  effect instance is still generic, and a fn *value* that uses an effect
  passed to a callee that needs one ([rs-effect-fusion]). Kotlin accepts
  all three.

### E1a — Ownership strategy: explored options (✅ settled 2026-09-04)

Kept as the record of *why* B9 was chosen; skip to B9 for the plan.

E1's mechanics are all present (handlers already travel as leading
arguments, resolved per call site from `call_effects`); what is *not*
settled is how a dependent handler reaches its dependency in **Rust**,
where mutable state has exactly one usable path at a time and every
handler member is `&mut self` today. Kotlin has no problem here — objects
alias — so this is a one-backend constraint that nevertheless decides the
language rule, because the rule must hold on both.

Six strategies were explored, each verified by compiling the emitted shape
with `rustc` (and `kotlinc` where Kotlin was the constraint) rather than by
reasoning. **B9 (handler fusion) was chosen**; the others are kept because
their failure modes are the argument for it:

- **B1 — capture as a borrow** (`struct LoudLogger<'a> { console: &'a mut dyn Console }`;
  the lifetime stays inside generated Rust, invisible in Salvo).
  Compiles, but `&mut` exclusivity means registering the logger *locks*
  the console: a later direct `println` is `E0499`. Kotlin accepts the
  same program, so parity requires Salvo to adopt the restriction as a
  rule — expressible in existing vocabulary (capture = fate link, member
  call = mutation, so the existing poison rules produce Rust's answer),
  and block-scoped `use` is the remedy (`{ use LoudLogger(); … }`, then
  direct use after the block — verified).
- **B2 — shared ownership** (`Rc<RefCell<dyn Console>>`). Rejected: it
  turns a compile-time question into a **runtime panic**, which breaks
  parity by construction. Cycle detection is *not* sufficient, which was
  the surprise: `println("${bump()} ${bump()}")` — legal Salvo today —
  panics with "RefCell already borrowed" because Rust temporaries live to
  the end of the *statement*, so two guards coexist with no cycle
  anywhere. Reentrancy also arrives through ordinary functions, not just
  declared dependency edges. Three separate guarantees would be needed
  (cycle rejection, a handler-reachability rule, and a statement-hoisting
  invariant in the emitter), and a gap in any of them is a crash in a
  user's program instead of a diagnostic.
- **B3 — ownership at construction** (`use LoudLogger(StdOutConsole())`,
  handler owns a `Box<dyn Console>` or a monomorphized `C: Console`). No
  lifetimes, no runtime checks, and Kotlin already emits this shape
  correctly. Cost: the dependency comes from an explicit argument rather
  than the ambient environment, and a *stateful* dependency cannot be
  shared with the surrounding scope — you get two instances.
- **B7 — "B semantics, A mechanics"**: store nothing; thread the
  dependency closure through emitted signatures, checking the requirement
  at the `use` site instead of the call site. Verified with a three-level
  chain (`Audit` needing both `Logger` and `Console`, `Logger` needing
  `Console`, `main` interleaving direct use): one console, state intact,
  no exclusivity, no lifetimes. It works because a `&mut` passed as an
  argument is a *reborrow* whose duration is the call — the lender is
  suspended, so five frames can reach the value while only the innermost
  uses it. The failure of B1 is the same fact seen from the other side:
  a *held* borrow lasts as long as the holder, so it overlaps.
  **B7's constraint** is that the dependency closure must be a static
  property of the *effect type*, since a fn declaring `[Logger]` has one
  emitted signature. That forces dependencies to be declared on the
  effect, so every handler pays for the union (`NullLogger` receives a
  Console it ignores).
- **B8 — per-handler dependencies + specialization** (costed below).

**The user's objection to B7** (2026-09-03): dependencies are a property
of *handlers*, not effects — `LoudLogger` needs a Console, `FileLogger` a
filesystem, `NullLogger` nothing. Correct as interface design, and it
rules out B7's static closure. The tempting middle road (declare on
handlers, thread the per-effect *union*) collapses: if the union for
`Logger` includes Console, `use NullLogger()` would have to supply one,
which is exactly the unpredictable rule to avoid.

Framing that drove the exploration: **Rust appeared to demand one of
exclusivity (B1), duplication (B3 / B8), or a runtime check (B2)** — every
option a different concession. B9 escaped the trilemma by changing *what
holds the dependency*: a fusion that owns the handlers as **distinct
fields** can lend each one separately, so a shared dependency is threaded
per call instead of held for a scope. The lesson worth keeping is that the
binding constraint was never "who may reach this value" but **how long
each borrow lasts**.

#### B8 — per-handler dependencies with specialized consumers (superseded)

Costed and then superseded by B9, but two findings from the costing carry
over and are worth keeping:

- **Handler selection is lexically static.** `check_use` accepts only a
  handler *name* or a *constructor call*, resolved against
  `scope.handlers` rather than locals, so a variable holding a
  runtime-chosen handler cannot be registered. Every call site sits in a
  statically known set of `use` scopes. Any strategy that resolves
  handlers at compile time depends on this.
- **Nothing in *checking* depends on which handler serves an effect.**
  `[effect-disambiguation]` resolves by effect *instance type*, and a
  member call's contract is the *member's* declared deduction list
  ([decl-explicit]). So handler-aware work belongs to emission; type
  checking stays single-pass and handler-agnostic.

B8's own mechanism — one emitted copy of a *user function* per handler
binding it is reachable under — was rejected because it duplicates
arbitrarily large functions. B9 keeps per-handler dependencies without any
duplication of user code.

#### B9 — handler fusion ✅ chosen (user decisions 2026-09-03/04), shipped 2026-09-04

The strategy of record, now implemented on both backends. Full mechanism,
with the borrow reasoning and the
verified shapes, is in **BACKEND_SPEC.rust.md [rs-effect-fusion]** (the
constraint is Rust's) and **BACKEND_SPEC.kotlin.md [kt-effect-fusion]**.
In brief:

- Effect traits/interfaces stay **dependency-free**, so a user fn is
  emitted **once** no matter which handlers flow in.
- A handler's dependencies are its **constructor parameters of effect
  type** — `handler ConsoleLogger(console: Console) of Logger`, which
  already parses and type-checks today, so E1 needed almost no new
  declaration syntax. The dependency reaches the member body as an extra
  parameter — on Rust through a generated `__Impl_H` trait carrying the
  bodies, since `impl Logger for H` has no room for it.
- One **fusion** per `use` scope holds the registered handlers and exposes
  each member, hiding dependencies by passing them from its own fields.
  In Rust the forwarding impl destructures `&mut self` into **disjoint
  field borrows**, which is what makes a *shared* dependency work — every
  earlier option needed two `&mut` to the same place.
- A fn needing several effects takes **one fusion value**: a multi-bounded
  generic on both backends. Rust cannot use `dyn` here — a `dyn` fused value
  cannot be forwarded to a callee needing a *subset* of the effects — so the
  blanket-impl conjunction trait survives only as the type of the fusion's
  provider field ([rs-effect-fusion]).
- Nested `use` scopes **rebuild flat** (the inner fusion holds the outer
  scope's *handlers*, not the outer *fusion*): depth-independent
  threading, no forwarding hops, resolution explicit in the construction.
  **Revised at implementation time on Rust** (2026-09-04): flatness is not
  achievable there — effects inherited from a fn's fused *parameter* are not
  handler locals, and an inner fusion re-borrowing the outer scope's locals
  makes the outer fusion unusable after the inner block. Rust chains through
  one `__outer` field instead, which preserves the property this bullet was
  really about (threading is identical at every depth) at the cost of one
  forwarding hop per level; see [rs-effect-fusion]. Kotlin still rebuilds
  flat [kt-effect-fusion].
- **Facets are Kotlin-only**: erasure forbids one class implementing
  `Random<Int>` and `Random<Double>`, so Kotlin generates a non-generic
  facet interface per instance with the type argument in the member name.
  Rust needs none of it. Each backend leverages its own language rather
  than sharing one lowest-common-denominator shape.

Why this beats everything above it: dependencies live on handlers (the
user's interface-design objection to B7 is honoured), nothing is captured
for a scope so there is **no exclusivity**, no `Rc`/`RefCell` so **no
runtime failure mode**, no user-function duplication, no lifetimes visible
in Salvo — and, because a handler member can mutate a dependency it
receives as a parameter, **neither the immutable/mutable effect
distinction nor `Cell` is needed** to keep effects testable. It is the
only option whose supporting features turned out to be unnecessary rather
than merely deferred.

Prerequisites: **handler values** ✅ done 2026-09-04 (see the decision log
entry below); **effect dependency cycles** must still be rejected at
declaration time, which only becomes reachable once dependencies can be
declared.

Open sub-decision carried forward: **effectful fn values**. A named fn
passed by value needs its fusion baked in (a closure). Fn-type effect
lists parse but are unenforced today, so this is a pre-existing hole that
B9 turns into a decision point rather than creating.

### E2 — Heuristics for validating external functions (user decision 2026-09-03)

An `external fn` is a trust boundary: its declared contract (effects,
deductions, return type, qualifiers) is taken on faith, and a wrong
declaration introduces a hole *accidentally* rather than maliciously.
Worth building cheap checks that catch the common mistakes by inspecting
the backend define template:

- A template that mutates its argument (`.clear()`, `.push(...)`,
  assignment) under a parameter the external declares as kept-immutable,
  or that keeps qualifiers a mutation would invalidate.
- A template that moves an argument (Rust: passing by value into a
  container) while the external's deduction keeps it — the `add`
  ownership bug this arc found, in reverse.
- A template performing I/O (`println`, file APIs) while the external
  declares `[]`.
- Necessarily heuristic and backend-specific (pattern matching on native
  source), so findings should be *warnings* with an opt-out, never hard
  errors — a false positive must not block a legitimate define.

### E3 — Non-resumption: `defer`, `abort`, and an intrinsic `try` (user decisions 2026-09-04)

The first slice of *handler control* beyond "always resumes at the tail",
which is all E1 supports. The exploration ran through four rungs of handler
power — tail-resumptive (today, free), abort (resume zero or one time),
suspend (resume later), multi-shot (resume repeatedly) — and settled on
building the second, with the third deferred to an *explicit* async effect
and the fourth ruled out.

**Multi-shot is closed on principle, not for want of a mechanism**:
resuming twice duplicates a use obligation, so it cannot coexist with
`canbe Linear`. (It is also unavailable on both targets: a Kotlin
`Continuation` throws on a second resume, and a Rust `Future` cannot be
cloned.)

**No silent function colouring** (user decision 2026-09-04). Some colouring
is *inherent* to non-resumption — the code between the operation and the
delimiter must not run, so either the return shape changes, the stack
unwinds, or the function is split. The rule that keeps it honest: the
ability to not resume is **declared on the effect member**, never
discovered from the handler. That is forced anyway by the E1 principle that
a user fn is emitted once whatever handlers flow in — the shape of `work()`
cannot depend on which `Logger` is registered — and it means the colouring
is exactly the effect annotation the author already writes. The Rust
backend's `async`/`suspend` transform is *not* how this is built; async
arrives later as an explicit effect (likely a compiler intrinsic).

#### The design as decided

```
qualifier Aborted<M> of M            // mirrors `Err<T> of T` in core.result

effect Abort<M> {                    // intrinsic; message is moved, like `err`
    fn abort(message: M) [] -> [] Nothing
}

// `try` is a compiler intrinsic, not an effect:
try { body } : Ok T | Aborted M
```

- **`try` is an intrinsic, not an effect** (user decision 2026-09-04):
  "there's not much value in a function declaring the `Try` effect in its
  signature any more than there is in declaring that it uses loops or
  if-expressions". So no `Try` handler to register, no `[Try]` in
  signatures, and — see below — no collision with the fusion.
- **Both arms are qualified: `Ok T | Aborted M`** (user decision
  2026-09-04), reusing `Ok` from `core.result` so ordinary `is` checks and
  exhaustive `when` work on the outcome exactly as they do on a result.
  `Err` is deliberately *not* reused: an abort is not an error value.
- **`Aborted M` is parameterized by a message type** (user decision
  2026-09-04), mirroring `Err`. It follows that the outcome union is
  structurally an `Ok T | Err M`, so union arm identity, narrowing and
  exhaustiveness need no new rules.
- **`Aborted M` is forgeable, deliberately** (user decision 2026-09-04):
  the qualifier carries no *authority* — a hand-written `-> M as Aborted`
  produces a value in the aborted arm but transfers no control. The
  authority is `[Abort<M>]` availability alone, which is why the original
  sketch's "only `abort()` may construct it" rule turned out to be
  unnecessary. No provenance semantics, no intrinsic qualifier.
- **`abort` returns `Nothing`**, which is what keeps intermediate frames
  silent: a fn that may abort declares `[Abort<Str>]` and returns `Int`.
  It does *not* also return `Aborted` — that would be `Result` plumbing
  with extra steps and would defeat abort being an effect. `Aborted M`
  appears in exactly one place: the `try` outcome.

#### Lowering

Rust: the message type *is* `ControlFlow`'s `Break` type, so the
propagation falls out of the design rather than being imposed on it.
`abort(m)` is `return ControlFlow::Break(m)` — no handler, no dispatch, no
allocation — every call in a fn with `[Abort<M>]` is `f(..)?`, and the
intrinsic converts at the delimiter:

```rust
pub fn parse(line: &String) -> ControlFlow<String, i32> { … }

// try { … }
match (|| -> ControlFlow<String, i32> { … })() {
    ControlFlow::Continue(v) => Union2::U1(v),
    ControlFlow::Break(m)    => Union2::U2(m),
}
```

Kotlin: a private stack-trace-less signal, with the delimiter's identity as
an unforgeable token — nesting must not let an inner `try` swallow an outer
abort:

```kotlin
private class Abort_Signal(val message: Any?, val token: Any) :
    RuntimeException(null, null, false, false)
```

Mechanism divergence with behavioural parity, the same reasoning as facets
and unions: `?` returns through each Rust frame running `Drop`, the JVM
unwinds running `finally`, and nothing user-visible happens on the way out
either way — *provided* `defer` is what puts code on that path.

#### `defer` comes first (user decision 2026-09-04) — **done 2026-09-04**

Not tidiness; three reasons:

1. **It turns a prohibition into a pattern.** Without it, a linear value
   live across a may-abort call cannot discharge its obligation on the
   abort path, so the checker would have to forbid the combination. With
   `defer close(f)` the author discharges on every path and the existing
   flow analysis can count it.
2. **It proves both backends can run code on an abort path** before
   anything depends on that. The two lowerings are exactly the two abort
   mechanisms' unwind paths: a `Drop` guard in Rust (whose reverse
   declaration order gives LIFO for free) and nested `try/finally` in
   Kotlin.
3. **It exercises the capture machinery** the `try` body needs, on a
   smaller independently testable feature. Its own open question is
   whether a deferred body captures by move or by reference — the
   fn-boundary contract question again.

Doing abort first would mean revisiting its lowering to add guards and
`finally` afterwards: the interesting part, twice.

**Built 2026-09-04** — see the decision-log entry at the top for the four
user decisions and the shape it landed in. What the sketch above got wrong:
the `Drop` guard is not a usable Rust lowering (it must own the value from
the `defer` onward, killing the very pattern), and the capture question
does not arise at all under splice-at-exit. What it got right: it does turn
the linear-across-an-exit prohibition into a pattern, and both backends
demonstrably run code on the way out of a block — the guarantee `abort`
now builds on. Reason 3 (exercising the capture machinery the `try` body
needs) is *not* discharged: nothing was captured, so `try`'s body closure
is still unexercised ground.

#### Consequences to settle before building — **all settled 2026-09-04**

- **`M` inference collides with [call-type-args].** ✅ Settled by the
  generalization: `M` is the **union** of the message types the body
  performs (user decision), and a body that cannot abort at all is an
  *error* rather than `Aborted None` (user decision) — so nothing has to be
  inferred from an empty set. The union costs a wrap at each propagation
  site on Rust (`?` needs identical `Break` types) and a tag dispatch at the
  catch on Kotlin; both are in the backend specs.
- **A `Nothing`-typed expression statement must count as terminating.** ✅
  Done, and it went further than "small": both path analyses
  (`block_returns` for [fn-must-return], `block_exits` for branch merging)
  became *type-aware* Checker methods reading the recorded types, and a
  written `Nothing` now lowers to the bottom type rather than a nominal
  type spelled that way. Without the second half, `abort(n)` in a branch
  leaked its consumption of `n` to the fall-through path.
- **The intrinsic couples the compiler to two core qualifier names.** ✅
  `Ok` and `Aborted` are resolved by name from the implicitly imported core,
  with a diagnostic naming the missing one; the compiler knows three names
  in total (`Abort` the effect, `Ok` and `Aborted` the arms) and nothing
  else about them.
- **Nested qualification is the honest consequence of wrapping in `Ok`.**
  ✅ Confirmed, both shapes tested: `Ok None`, and `Ok (Ok Int | Err Str)`
  taken apart through a binding at the inner type. `when` does reject a
  qualified-group subject, but that is not a dead end — the droppable
  qualifier rule unwraps it; see "Nested qualification" below for the two
  alternatives that were rejected.
- **Abort targets the innermost `try`.** ✅ [try-innermost]. Rust needs no
  token at all (the block label decides where a `break` lands); Kotlin's
  catch-all is innermost by construction, and its `else -> throw` rethrow is
  what an escaped function value would hit.

#### Nested qualification: resolved without new surface (2026-09-04)

`try`'s outcome makes two qualified-union shapes reachable — a
result-returning body (`Ok (Ok Int | Err Str) | Aborted Str`) and a union
message (`Aborted (Str | Int)`) — and `when` rejects a qualified-group
subject. That looked like a dead end and was reported as one; it is not.
The **droppable-qualifier rule already unwraps**: `Qual T <: T`, so a
binding at the inner type takes the level off, and both backends emit it
correctly (Rust a plain unwrap, Kotlin a cast the narrowing makes
unfailable — verified end to end, same stdout).

```
when nested {
    is Ok {
        let inner: Ok Int | Err Str = nested     // outer claim dropped
        when inner { is Ok { … } is Err { … } }
    }
    is Aborted {
        let message: Str | Int = mixed           // union message, same idiom
        when message { is Str { … } is Int { … } }
    }
}
```

Two alternatives were considered and rejected (user discussion
2026-09-04):

- **Merging nested qualifiers** (`Ok Err Str`) collides with an existing
  meaning: a multi-qualifier type is a *set of claims* (`Mut NonEmpty
  List<T>`), so `Ok Err Str` reads as both applying — a contradiction, not
  nesting. It would also flatten the outcome union's arms, changing
  [union-arm-identity] and the wrapper representation, and cost `try` its
  uniform two-arm shape.
- **A dedicated "check and unwrap" keyword.** Two objections. It conflates
  a test with a *static* strip (inside `is Ok` the type is already known,
  so nothing needs checking), and — decisively — the exclusions it would
  need already exist: `Qual T <: T` is written "except `Once`", and
  `Linear` is never a use-site qualifier, so the annotation path gets the
  intrinsic-qualifier cases right for free. A keyword would re-implement
  that list and have to keep it in sync as intrinsics are added (`Cell` is
  next).

What *was* wrong is discoverability: the diagnostic dead-ended. It now
names the remedy — "`Ok (…)` is the claim `Ok` *about* a union: bind the
inner union to a local and match that" — and the binding keeps which level
is meant visible to a reader, which matters when the same qualifier name
appears twice. Sugar (a stripping form on `is`) stays open, but it must
reuse the subtype rule's exclusions rather than inventing its own.

#### Why the intrinsic dodges a gate the library form would need

An earlier sketch had `Try` as an ordinary effect whose member takes the
body as a lambda. That form needs work this one does not:

- Fn-type **effect lists are parsed and dropped** today, and lambda bodies
  check under the enclosing fn's effect environment (lexically). "The
  lambda gets an effect its enclosing scope does not have" — the core of an
  *effect transformer* — is therefore not expressible yet, and neither is
  the dual guarantee that a body carrying `[Abort<M>]` cannot outlive its
  delimiter. [effect-not-data] already stops the capability escaping as a
  *value*; the lambda case is what remains.
- It collides head-on with the fusion cut landed 2026-09-04: a realistic
  body performs other effects (`try { println("x"); abort("bad") }`), so
  the lambda would capture the fused value while the call to the `try`
  *member* borrows it too — exactly the reported `E0499` shape.

Both point at one piece of work: enforce fn-type effect lists, pass effects
*into* fn values instead of capturing them, and forbid their escape. That
gate is also what rung 3 needs and what would lift the fusion cut, so three
motivations converge on it — but the intrinsic `try` needs none of it,
because the compiler generates the body closure and can pass the fused
value in.

**Effect transformers** — an effect member that runs a fn-typed parameter
with *additional* effects available, its body having registered handlers
for them — remain the general prize (`Retry`, `Timeout`, and async itself
are the same shape). Worth building once the gate exists and there are two
customers rather than one; `try` could then be re-expressed as a library
transformer if that reads better.

#### Sequencing

1. ~~`defer` — standalone, testable, settles linear-on-abort first.~~
   **Done 2026-09-04** ([defer], [defer-no-escape], [kt-defer-finally],
   [rs-defer-splice]).
2. ~~`abort` returning `Nothing` + intrinsic `try`, with `Aborted<M>` in
   std.~~ **Done 2026-09-04** ([abort], [try], [try-innermost],
   [abort-not-main], [abort-linear], [rs-abort-controlflow],
   [rs-try-label], [kt-abort-signal]). What the design got right and wrong
   is in the decision-log entry at the top; the short version is that the
   `ControlFlow` propagation and the `defer`-first ordering both held, and
   the closure lowering for `try` did not.
3. ~~Enforce fn-type effect lists; pass effects into fn values; forbid
   escape. (Lifts the fusion cut, opens transformers.)~~ **Done
   2026-09-04** ([fn-effects], [rs-fn-effect-params],
   [kt-fn-effect-params]) — with "forbid escape" dropped as unnecessary (a
   fn value carries no capability) and the fusion cut duly lifted.
   Transformers are now unblocked.
4. Transformers, and async as the second one.

Before (2), hand-write and run the Rust shapes for the three compositions
that decide whether propagation stays clean: a may-abort call inside a
loop, one inside a nested `try`, and one with a live linear value plus a
`defer` across it. Loops are the specific reason CPS-splitting into
continuation legs was rejected for this rung — a `perform` inside a loop
becomes recursion through the continuation, and neither target guarantees
tail calls, so ten thousand iterations means ten thousand frames unless you
hand-build a trampoline, which is the async machinery under another name.
The legs idea is right for rung 3, where the continuation must be reified
anyway; there its cost (a boxed closure per call, answer-type erasure) buys
something.

## Roadmap: shared mutable state (`Cell`)

An idea developed 2026-09-03 while looking for a way to keep *immutable*
effects testable (a recording double needs state). It stands on its own
merits and is **not** tied to that use case — most of the patterns below
have nothing to do with effects. Open **DECISION**.

### The problem it addresses (and what it unlocks)

Salvo's mutation rule is about the **handle**: you may mutate through a
path only if that path is `Mut`, which is exclusive. Several ordinary
patterns need the opposite — mutation through a *shared* path:

- two lambdas appending to one accumulator (today the first one to mutate
  a capture *consumes* it, so the second is an error and the original is
  dead afterwards — verified: "`total` cannot be used here: it was
  consumed (moved) by a lambda that captures and mutates it");
- memoization / lazy initialization behind an immutable handle;
- counters, metrics, id generators shared by several holders;
- a stateful handler of an effect whose other handlers want to be shared.

### The proposal: a capability qualifier, not a container type

`Cell` joins the intrinsic capability qualifiers (`Mut`, `Linear`,
`Once`, `ReadOnly`) rather than arriving as a std generic type
`Cell<T>`. The family fits exactly — each intrinsic qualifier exists
because it needs "a representation choice, a flow rule, a subtyping
direction or a restricted position that no user declaration could
supply", and `Cell` needs the first three:

- `Mut T` — mutation permitted, only through *this* handle.
- `Cell T` — mutation permitted through *any* handle.

**Benefits over a container type:**

- **No wrapper noise.** `count = count + 1` and `if count > 3`, rather
  than `set(count, get(count) + 1)` and `if get(count) > 3`. Reads and
  assignments keep ordinary syntax; only the *permission* differs.
- **It inherits machinery instead of adding surface**: `canbe Cell`
  opt-in on declarations, qualifier erasure, overload selection, and
  D1's stripping rule — where `Cell`, being a capability rather than a
  claim about contents, is never stripped (like `Mut` and provenance).
- **Family membership is the documentation.** "Capability qualifiers say
  what you may do with a handle" already exists as a concept; a std
  container with its own API is a second thing to learn.
- Danger stays visible in the type either way: `Cell Int` at every use
  site, greppable, opt-in — unlike interior mutability hidden inside an
  ordinary type.

### Representation, and why it cannot panic

- **Copyable contents → Rust `Cell<T>`**: `get`/`set` only, no borrow
  guard exists, so no runtime check and no panic is *representable*
  (verified: a shared id generator, `ids: 1 2 3`).
- **Collections → Rust `RefCell<T>`**: a guard exists, but the only
  operations are std primitives whose define templates the compiler
  controls (`${list}.push(${value})`), so no Salvo code ever runs inside
  the borrow (verified: a recording double shared by a capturing logger
  *and* used directly, all three writes recorded).
- Kotlin: a plain mutable field. No parity gap — both backends accept the
  same programs.

**The rule that keeps this true:** a cell may be mutated by assignment
and by *standard-library* primitives, but never lent to a user-defined
`Mut` parameter. Handing `&mut` into user code is what puts a borrow
guard around a user call, which is precisely where B2's reentrancy panics
came from (see E1a). One sentence to teach: "you can mutate a cell; you
cannot hand its insides to a function you wrote."

### Rules it drags in (the real design work)

- **No state qualifiers on cell contents.** A claim like `NonEmpty` is
  about contents, and contents can change through a handle the compiler
  is not looking at — so `qualifier … of Cell …` must be rejected for
  *state* claims. Provenance claims are fine (they are about where the
  handle came from). This is the one genuine soundness rule, enforced at
  the declaration.
- **No shared-fate links.** Reads copy rather than lend, so
  `let v = count` is an independent value: no link, no poison. Simpler
  than the field case, and a direct consequence of "no handle into the
  contents".
- **Linear contents need `replace`.** Overwriting a cell holding a
  `canbe Linear` value would silently drop an obligation; `replace(cell,
  v) -> T` hands the old value back and transfers the obligation, while
  plain assignment over linear contents stays an error.
- **Deductions say nothing.** A fn taking a `Cell` and writing it needs
  no `Mut`, so its signature cannot report the write — acceptable only
  because the first rule leaves no claim worth preserving.

### The concession being accepted

`Cell` is a sanctioned hole in "no hidden shared mutable state": two
holders can surprise each other, and the ownership analysis stops helping
inside a cell. That is the price of shared mutable state in any language
with an ownership discipline; what makes it defensible is that it is
visible in the type rather than hidden behind an ordinary one.

### Relationship to E1

`Cell` is **not load-bearing** for effects. E1's chosen strategy (B9
handler fusion) keeps handler members able to mutate dependencies they
receive as parameters, so recording test doubles need no interior
mutability and effects need no immutable/mutable distinction. `Cell`
therefore stands on the lambda/memoization/counter cases, which are real
limitations today, and can land independently of the effects work if it is
wanted at all.

## Roadmap: place-based flow analysis

A second flow-analysis arc, independent of linearity. Flow facts are keyed
by *place* — a local root plus a projection path (`h`, `h.field`,
`h.a.b`) — rather than by variable name. The place facts live inside the
root's `LocalVar`, so `snapshot_narrows`/`restore_narrows`/
`merge_fallthrough` remain the single source of truth (the S1 gotcha) and
an event on a root reaches everything below it.

### P1 — Field smart-casting. ✅ Done 2026-09-03

`is` narrows places, so a checked field or tuple element reads at its
narrowed type ([flow-place]), and invalidation ([flow-place-invalidate])
rides on the event set the fate analysis already watches. Landed:
`place.rs` (the `Place`/`Proj` types and the prefix/overlap relations),
place-keyed narrowing through the branch machinery, narrowed-read
unwrapping in both emitters, tuple element access as new syntax
([expr-tuple-index]), and — from the Kotlin parity bug it surfaced —
[kt-narrow-field-assert]. Decisions P1a/P1b/P2 are in the decision log.

Deliberately out: array elements (an unknown index may alias any element,
so they are tested and bound but never narrowed) and `when` on field
subjects (P2, decided against — `when` stays variable-only).

**Tuple element access** ([expr-tuple-index]) was added in the same
session to close P1a: `t.0` is a `Proj::Index` place, so it narrows,
invalidates and merges like a field.

### L5 — field-disjoint ownership (rides on this substrate)

Place-based *ownership*: using one field while another is moved or
borrowed. Lives in the linear-types roadmap above; it now inherits P1's
`Place` type, prefix/overlap relations, and the per-root fact storage.
Sequencing worked out as recommended (P1 first): the substrate was
designed and validated under the monotone feature, and the element
projection variant is already in place for ownership's benefit.

## Roadmap: deductions and qualifier reasoning

Motivated by a confirmed unsoundness (see "Remaining leftovers"): the
removal-set rule (`removal = declared − kept`) assumes a function can only
invalidate qualifiers it *declares*, which is false for any function that
mutates. Fixing it needs a way to say "and nothing else survives", which a
delta-only model cannot express.

### D1 — Exhaustive and delta qualifier deductions. ✅ Done 2026-09-02

Keep the `:` syntax; give the qualifier list a *polarity* per entry:

| Form | Meaning |
|---|---|
| `[list]` | keep the parameter, all qualifiers preserved (unchanged) |
| `[list:]` | keep it, strip every qualifier (unchanged) |
| `[list: NonEmpty Mut]` | **exhaustive**: afterwards *only* these apply |
| `[list: -NonEmpty]` | **delta**: drop `NonEmpty`, everything else preserved |
| `[list: Nothing]` | moved (equivalently: omit the entry) |
| `[]` | no promises about any parameter — everything moved |
| *(absent)* | inferred [deduce-infer] |

`+Q` (add a qualifier) is **not part of the language** — it is an assert,
see D2 (user decision 2026-09-02). The parser recognizes the `+` only to
report "adding qualifiers in a deduction (`+Qual`) is not supported yet"
instead of a bare parse error.

The plain (exhaustive) form is what closes the hole: `clear`'s
`[list: Mut]` now drops a caller's `NonEmpty` because it was not listed.

- **Soundness rule (the load-bearing part).** For a parameter the body
  *mutates*, only the exhaustive form (or `[list:]`) may be written:
  keep-all `[list]` and `-` deltas both claim "everything else survives",
  which is exactly the unsound claim. Inference emits the declared set
  for mutated parameters and keep-all for read-only ones — so pure reads
  still never strip a caller's qualifiers, which is the property the
  original removal-set rule existed to protect.
- **Is mutation the only invalidating operation?** (user question
  2026-09-02) Within the current language, yes — and it follows from the
  model rather than being a stipulation. Everything a callee can do to a
  *kept* parameter is: read it (projections, interpolation, passing it on
  to other readers) — observably state-preserving, so no predicate can
  break; mutate it — the invalidating case; or move it — after which the
  caller has no access, so preservation is moot. There is no way for a
  kept value to escape observably (fate links do not outlive the call;
  `use` constructor arguments are moves). So "mutation forces the
  exhaustive form" is the complete rule for today's language, and a new
  invalidating operation could only arrive with a new capability
  (out-parameters, escaping references).
- **But the *granularity* is wrong, and that is worth marking on
  qualifiers.** Qualifiers split into two kinds:
  * **State predicates** (`NonEmpty`, `Sorted`, `Validated`): claims
    about content. Mutation may falsify them.
  * **Capabilities** (`Mut`, and the usage disciplines `Linear`,
    `Once`): claims about what the *holder* may do. Mutation cannot
    falsify "you may mutate this".
  Only state predicates need dropping when a body mutates; capabilities
  survive trivially. Today this distinction is **degenerate**: every
  capability qualifier is built into the language and every *user*
  qualifier is a state predicate (or provenance, which behaves like one —
  `Validated` is falsified by mutation as surely as `NonEmpty`). So D1
  hard-codes the built-ins and does **not** add a marker; revisit if a
  user ever needs to declare a capability qualifier.
  * Consequence for D1's syntax: `Mut` is written explicitly in
    exhaustive lists (`[list: Mut]`), and `[list:]` keeps stripping
    everything including `Mut`, exactly as today. The alternative
    (auto-preserving capabilities) would silently change `[list:]`'s
    meaning for no present benefit.
- **Accepted cost (user decision 2026-09-02): over-strictness.** A
  mutating function cannot promise to preserve a qualifier it does not
  declare, so `add(list: Mut List<T>, elem: T)` drops a caller's
  `NonEmpty` even though appending cannot empty a list. Remedy is a
  re-test (`if list is NonEmpty`); the general answer is D3.
- **Mixing polarities is an error.** An entry is either all-plain
  (exhaustive) or all-delta (`-`, and later `+`): `[list: Mut -NonEmpty]`
  is redundant under the exhaustive reading and contradictory otherwise.
- **`-Q` may name a qualifier the parameter does not declare** (a
  function that knows it invalidates a specific property). It is a
  convenience only — the exhaustive form remains the sound default, and
  inference never relies on `-`.

**Implementation notes (2026-09-02).** `QualEffect { KeepAll,
Exhaustive(Vec<String>), Remove(Vec<String>) }` lives in `types.rs` and is
carried by both `ParamDeduction` and `FnParamContract`; all three forms
reduce to one operation, `removal_set(have)`, computed against the
qualifiers the *argument* carries (`have`) — that is where the soundness
comes from. Mutation is tracked like move-mode claims: `fate_mutation`
records the enclosing fn's parameter in a new `param_mutations` map (for
*written* lists too — that is what the validation reads), threaded through
`check_once` and the fixpoint's convergence check. Inference gained
`restrict_to` for the contagion rule, and the lattice `KeepAll` →
`Remove` (growing) → `Exhaustive` (shrinking) is what keeps the fixpoint
terminating; `bodyless_mutations` supplies the `Mut`-parameter proxy for
externals. Exactly one test had to change meaning:
`undeclared_qualifiers_pass_through_calls` *encoded* the unsoundness, and
is now `exhaustive_lists_drop_undeclared_qualifiers`, with
`delta_lists_pass_undeclared_qualifiers_through` covering the opt-in.
- **Type in the entry.** An entry's items are qualifiers plus *at most
  one* type — structurally identical to an `is` check pattern
  (`CheckPat { quals, base }`), so the parser and resolver can share that
  path: uppercase idents parse as type refs, and qualifier-vs-type is
  settled by resolution.
  * `[list: Nothing]` is sound *because `Nothing` is uninhabited*: it
    asserts nothing about runtime content, it only withdraws use. That is
    why "moved" falls out of the same rule rather than being a special
    case.
  * **Type narrowing beyond `Nothing`: its own step (user decision
    2026-09-02, following the recommendation).** `[x: Str]` on a `Str?`
    parameter is a *content* claim, so the callee must make it true.
    Out-parameters would do it, but Salvo has none (reassigning a
    parameter is a rebind, invisible to the caller). The implementable
    reading is a **verified postcondition**: "if this call returns
    normally, the parameter is a `Str`" — checked for Salvo bodies by
    requiring the promised narrowed type at every normal exit (the
    machinery `check_linear_exit` already walks), trusted for externals.
    D1 lands with `Nothing` only; this follows as **D1b**.

### D3 — Refinements (and why polymorphism is probably a dead end)

D1's over-strictness has two candidate general answers. Analysis
2026-09-02 says they are *not* both needed, and the more obvious one does
not work:

- **Polymorphism — "preserves whatever predicates it received" — is
  unsound as a blanket promise.** Different predicates react differently
  to the *same* mutation: `add(list, elem)` preserves `NonEmpty` but can
  break `Sorted`. So a mutating function cannot honestly promise to
  preserve an unknown set. Restricting it to *non*-mutating functions
  makes it vacuous (keep-all `[list]` already preserves everything
  there). The only sound version is an explicit per-qualifier set, which
  is refinements by another name.
- **Refinements — a qualifier annotates existing functions with
  additional deductions, without changing their implementation.** This is
  the same information as "which operations invalidate me", stated from
  the qualifier's side, and it is per-qualifier-per-function, which is the
  granularity soundness actually requires. It also inverts the dependency
  in the useful direction: a user's `NonEmpty` can declare that std's
  `add` preserves it, without std knowing the qualifier exists.
  * Open questions: where a refinement may be declared (the qualifier's
    own file, mirroring the constructor-fn rule?); whether it is trusted
    or checked; how refinements from several qualifiers on one function
    compose; and whether a refinement can *strengthen* a std function's
    contract for callers who do not import the qualifier (it must not).

### D2 — Qualifier asserts (`+Q`)

Deferred (user decision 2026-09-02: not even in the grammar for now).
`+Q` asserts that the body *establishes* `Q`, which is what `-> T as Q`
does for return values — the parameter-position analogue. A predicate
qualifier cannot be proven statically (that means reasoning about the
algorithm), so establishment is trust (like `as Q`) or a runtime
`qualifies` check.

- **DECISION D2a** — whether `+Q` and `as Q` unify into one notion of
  "this function establishes a qualifier", and whether establishment is
  trusted, runtime-checked, or restricted to fns declared in the
  qualifier's own file (as `as Q` is today).

### D4 — Predicate `is` on union subjects (and qualifiers over unions)

Motivated by an analysis of the `is` keyword (2026-09-03): `is` has one
grammar and two evidence sources — union-arm identity, statically known
[is-narrowing], and a runtime `qualifies` call [is-qualifies]. There is
no parse ambiguity (one `Expr::Is` node; both forms are
`subject is Qual* [Type] [binding]`), but `is_info` picks between them
by the *subject's shape*: a `Ty::Union` subject **always** takes the
arm-matching path. Consequence: a predicate qualifier can never be
tested against a union-typed value. With `let x: Int | Str`,
`x is Positive` matches no arm and reports "this check can never
succeed"; the workaround is to narrow first (`x is Int && x is Positive`
works, because the second test sees a non-union subject). The predicate
form is shadowed by the union form, and the shadow is invisible in the
surface syntax.

Fixing it is not a checker patch — the narrowed type it should produce
is a union whose arms carry a qualifier, so it needs qualifiers over
unions in general (today [qual-union-arm] binds a qualifier to a single
arm, and only an explicitly parenthesized group can be qualified
[qual-group]).

- **DECISION D4a — semantics of `x is Q` on a union subject.** Which
  arms participate (recommendation: those whose qualifier-stripped type
  satisfies `Q`'s `of` type), and what the check *is*: a conjunction of
  the arm/tag test and the `qualifies` call (recommended — it is what
  the two-step workaround does today), or `qualifies` alone.
  Then-type: the participating arms with `Q` added.
- **DECISION D4b — the else branch.** A failed predicate proves nothing
  ([is-qualifies] already records "no else information"), so the
  remaining set must *keep* the participating arms — unlike a pure arm
  test, which subtracts them. That asymmetry is the load-bearing
  difference and it propagates: a `when` whose arms are predicate checks
  can never be exhaustive. Decide whether predicate checks are allowed
  in `when` arms at all (recommendation: allow only when the arms are
  exhaustive on tags alone, otherwise reject with a message pointing at
  `if`/`elif`).
- **D4c — qualifiers over unions.** Decide the shape of the narrowed
  type: per-arm `Q A | Q B` (recommended — preserves [qual-union-arm]
  and existing arm identity) versus a qualified group `Q (A | B)`
  ([qual-group], which changes wrapper identity). Per-arm keeps the
  positional-arm invariant that the checker and both emitters share.
- **D4d — mixed checks.** `x is Positive Int` on a union: base-type arm
  test *plus* the `qualifies` call, one lowering.
- **Backend work.** `is_tests` and `predicate_tests` are separate side
  tables and each emitter lowers one of them; a union subject with a
  predicate needs a *combined* lowering (tag test `&&` qualifies call)
  in both, and checker and emitters must agree exactly, per the
  invariant. Effects on the `qualifies` fn stay subject to
  [is-qualifies-effects] at every such site.
- Sequencing: independent of D1–D3, but it shares the "what does a
  qualifier mean over a composite type" question with D3's refinements;
  do D4c's decision before either.
- Not in scope: renaming `is`. The two readings are opposites in
  *feel* — "already attached" versus "may be attached" — but both are
  "test whether this holds now, and refine if it does"; conferring a
  qualifier is what `-> T as Q` does [qual-ctor-fn]. User decision
  2026-09-03: keep one `is`, revisit only if D4's rules prove confusing
  in practice.

### D5 — Qualifier subjects: state vs provenance ✅ Done 2026-09-03

The predicate/constructive split [qual-predicate] [qual-constructive] is
an *evidence* axis — how a value acquires a fact. It says nothing about
what the fact is *about*, which is why `NonEmpty` is legitimately both
(one state claim, two evidence routes [qual-ctor-predicate]). Three
independent axes were identified:

| Axis | Values | Status |
|---|---|---|
| Evidence | predicate (runtime `qualifies`) / constructive (mint-only) | in the spec |
| **Subject** | **state (contents) / provenance (the handle)** | **D5** |
| Authority | user-declarable / compiler intrinsic | implicit |

One combination is impossible and explains the shape of the intrinsics:
a **predicate capability cannot exist** — no inspection of the bits
reveals whether you are *permitted* to write, so `Mut` has no
`qualifies` and never could. Capability/provenance implies constructive;
constructive does not imply capability (`Sorted` minted by `sort()` is
mint-only *state*).

Decided (user, 2026-09-03):

1. **Qualifier declarations gain a subject axis**: *state* (default,
   today's semantics) and *provenance*. Option B of the explored set.
2. **Provenance is a content-independent claim about where the handle
   came from** — mint-only (no `qualifies` body; `is Q` on a non-union
   subject stays a compile error, as [qual-constructive] already says).
   The payoff: a provenance tag **survives an unlisted mutation**.
   D1's stripping rule is sound only for claims about contents —
   [deduce-syntax] says so in as many words ("mutation can invalidate a
   caller's *state* predicates") — so exempting provenance needs no new
   soundness argument. Without this, `Authenticated Request` loses its
   tag to any logger declaring `[req: Mut]`, and D3 refinements would be
   the only escape: per-qualifier-per-function boilerplate for a fact
   that is blanket-true.
3. **Provenance is droppable** (forgetting provenance is safe) and
   **survives being stored into another value**.
4. **Provenance composes without a `with` declaration**, mirroring the
   `Mut` auto-qualifier [type-canbe-mut]: it is orthogonal to every
   claim about contents. Tags stack freely, including several over one
   base (`Authenticated EnvironmentId Str`). `with` [qual-with] keeps
   its original job — two *state* claims co-applying, where explicit
   compatibility is the whole point. Without this rule
   `Ok EnvironmentId Str` would be inexpressible (per-tag
   `qualifier Ok<T> of T with EnvironmentId` does not scale, and
   `Ok (EnvironmentId Str)` is the nested-qualified case [qual-generic]
   calls discouraged) — a newtype that cannot be returned in a `Result`
   is half a newtype.
5. **Hard capabilities stay compiler intrinsics** (`Mut`, `Linear`,
   `Once`, `ReadOnly[from: p]`). Each needs something no user can write:
   a per-backend representation choice (`Mut` is *not* erased —
   `MutableList`, `&mut`), a rule in the flow analysis, a non-standard
   subtyping direction (`Once` inverts it), or a restricted syntactic
   position (`Linear` never at a use site; `ReadOnly` return-only).
   User-authored versions would need a metalanguage for checker rules —
   rejected as out of proportion to Simplicity/Verifiability.
6. **Vocabulary.** What users declare is *state* or *provenance*. What
   the compiler owns are *permissions* and *obligations*, and the
   sub-rule is **permissions are droppable, obligations are not**:
   `Mut Person <: Person` (`map` returns `List<T>` while knowing
   `Mut List<T>` internally), while `Once` may never be dropped and
   `Linear` cannot be written at all.

**Motivating example (record this one — it is more legible than
`Authenticated`): the wrapper/newtype pattern.** The Kotlin habit of
`data class Environment(val id: Id)` with a nested
`data class Id(val value: String)` exists so that an incomplete refactor
is a type error instead of a silently mis-wired `String`. As a Salvo
provenance qualifier (`qualifier EnvironmentId of Str` plus a
constructor), it is content-independent (any string can be an id),
mint-only, and not runtime-testable — and it delivers the protection,
because a bare `Str` does not subtype `EnvironmentId Str`: swapping two
tagged arguments fails to compile in both positions. Only widening is
open, which is the approved droppability, and is the same hole Kotlin
has whenever the target parameter is `String`.

Two properties of the erased design worth knowing before revisiting it:

- **In union position the tag is reified.** `Ok Str | Err Str` already
  works — positional wrappers give arms a physical identity even when
  they erase to the same type, and `is Err Str` matches precisely
  [is-precise]. So `EnvironmentId Str | DeploymentId Str` discriminates
  at runtime; erasure applies to a value in a parameter, not to a value
  in a union.
- **What erasure actually costs** is identity, not usability: two tags
  over `"prod"` compare equal and collide as keys in one map. Display,
  interpolation, base-type operations and (static) overloading all work
  for free — the `toString` override the Kotlin pattern needs exists
  only to undo the boxing, which Salvo never does.

7. **Provenance stays erased** (user decision 2026-09-03, D5a settled):
   uniform with [qual-erasure], so the backends are untouched by D5.
   Reification (a wrapper type per tag per backend) was declined: it
   needs wrap/unwrap insertion at every widening site, double wrapping
   inside unions, and messier interop for std fns taking `Str`, while
   the only benefit — distinct equality and map keys — is already
   reachable with a one-field struct, which *is* the nominal flavor of
   this pattern and needs no new feature. The decisive cost is two
   lowering models for one concept, doubling the checker/emitter
   agreement surface. Kotlin's own tool for this job
   (`@JvmInline value class`) is likewise unboxed except in
   generic/nullable/collection positions, so the erased design is the
   idiomatic trade, not an exotic one.
Namespacing (the former D5b) outgrew this section and is now its own
roadmap item, **N1** — dot-names apply to structs as well as qualifiers,
so it is a naming feature independent of the subject axis.

What landed (rule [qual-subject]):

- **Syntax** (user decision 2026-09-03, confirmed after implementation):
  `provenance qualifier Q of T`, a prefix modifier matching the existing
  `internal`/`external` shape. Plain `qualifier` stays state
  (`QualSubject::State` is the AST default), so nothing existing changed;
  there is deliberately no `state` keyword — one keyword for the
  non-default is enough, and the default is documented. Revisitable if it
  reads badly in practice (there is no compatibility to preserve).
- **Stripping exemption (the payoff)**: `QualEffect::removal_set` now
  takes a provenance predicate and filters the removal set, so a
  provenance tag survives any call. The *inference* side matches
  (`remove_quals` filters, `restrict_to` re-admits the parameter's
  declared provenance quals) so inferred signatures never claim a removal
  that cannot happen. Deduce works from a program-wide provenance name
  set — the deduction machinery is qualifier-name keyed throughout, and
  [mod-collision] already rejects one name declared by two visible
  modules; the checker uses the precise per-file scope.
- **Mint-only**: a `provenance` qualifier with a body is an error naming
  both halves (no `qualifies`, no field overrides). `is Q` on a non-union
  provenance value needed no new code — provenance is bodiless, so
  [qual-constructive]'s existing message fires.
- **Free composition**: the pairwise `with` check in `validate_quals`
  skips any pair where either side is provenance.
- **No backend work**: provenance erases like every qualifier
  [qual-erasure]. The only emitter-visible consequence is *which
  overload* the checker picked.
- Verified end to end on both backends with one program that makes the
  distinction observable in program output: a `Mut List<Int>` carrying a
  state tag and one carrying a provenance tag both go through `add`
  (`[list: Mut]`), then dispatch — `trusted 3` (provenance survived),
  `plain 3` (state stripped), `checked 2` (state intact without
  mutation), identical under `kotlinc` and `rustc`.

Implementation sketch (no backend work — provenance is erased, so
emitters are untouched under D5a-as-recommended):

- `QualifierDecl` gains the subject kind (parser + AST); the checker's
  `validate_quals` enforces the provenance legality rules (no
  `qualifies` body, no `with` needed, `is` on a non-union subject
  rejected — the last one is already [qual-constructive] behavior).
- The removal-set computation in the deduction machinery skips
  provenance tags when a call strips state qualifiers; the *contagious*
  exhaustiveness rule [deduce-syntax] narrows accordingly.
- Stacking validation stops requiring pairwise `with` when either side
  is provenance.

**Future intrinsic capabilities (watch list, in order of how soon they
force themselves on us).** `Sendable`/thread-safety the moment
concurrency lands — structurally inferred, trusted-by-audit for
externals (the `canbe Linear` pattern), and asymmetric between backends,
so it cannot be user-authored. Then `Uniq`/`Shared` if sharing arrives
(`Uniq` is the precondition for in-place mutation of a shared type),
`Local`/`Escaping` (the conservative refusal of escaping closures
becomes a contract), `Init` for two-phase initialization
(`MaybeUninit`/`lateinit`), and `Const` if compile-time evaluation
lands. Note that `Uniq` and `Local` are facts the fate analysis
*already computes*: they are the user-facing side of the recorded
"internal qualifier unification" leftover, which is the strongest reason
to set the vocabulary up deliberately now.

### Related, independent: `!is` in expressions

Negated checks (`if x !is Str { … }`) — sugar over `!(x is Str)` with the
same fact propagation, and no binding form (a failed test binds nothing:
`x !is T name` is a parse error). Lexing (user decision 2026-09-02): `!`
binds *adjacently* — `x!` (assert) requires no space before `!`, `!is`
requires no space between, `!x` (not) requires no space after, and a
floating `!` (space on both sides) is a syntax error.

The disambiguating clause, refining "not preceded *and* followed by
alphanumerics": the left neighbour must count `)` and `]` as
value-endings, or `names.first()!is Str` stays ambiguous — and
`first()!` is real, common code (the M2 demo interpolates
`${names.first()!}`). So:

> `!` may not be *directly* preceded by a value-ending token
> (identifier, literal, `)`, `]`) **and** directly followed by an
> operand-starting token (identifier, literal, `(`, `[`) or the keyword
> `is`. Whitespace on one side resolves it.

This keeps `x!.field`, `x!)`, `x!,` legal (the following token cannot
start an operand), rejects `x!is T` and `a!b`, and leaves `x! is T`
(assert then test) and `x !is T` (negated test) as the two spellings.

## Roadmap: names and namespacing

### N1 — Dot-names for structs and qualifiers ✅ Done 2026-09-03

Motivated by the wrapper/newtype pattern (see D5): the Kotlin habit of
nesting `Id` inside `Environment` so signatures can demand
`Environment.Id`, without Salvo adopting nested *declarations* — the
user prefers flat code, so the namespace is in the *name*, not the
layout.

Proposed surface: a struct or qualifier may be declared as
`<ns>.<name>`, where `<ns>` names a struct in the same file.
`<ns>` must be a struct in that file, and no name in that file may be
`<ns><name>` when a dot-name with that `<ns>` exists (the Rust
flattening would collide). Backends: Kotlin nests the type inside
`<ns>`'s class; Rust concatenates (`<ns><name>`), since Rust modules and
structs do not relate the way Kotlin classes do. Qualifiers are erased,
so for them the dot is compile-time only on both backends — which is why
qualifiers are the easy half.

Import disambiguation rests on a new casing rule: modules always start
lowercase; types, structs and qualifiers always start uppercase. That
turns `import a.b.Environment.Id` into a decidable split (leading
lowercase segments are the module path, trailing uppercase segments the
item name) where today the split is purely positional — `resolve_import`
takes the *last* segment as the item name and everything before it as
the module prefix.

Decided in review (user decisions 2026-09-03):

- **N1a — the casing rule covers values too.** An uppercase-initial
  *head* identifier starts a *type path*; variables, parameters, fields
  and fn names start lowercase. Needed because in expression position
  `Environment.Id { value: "x" }` is otherwise indistinguishable from a
  field access on a variable named `Environment` (or a dot-notation
  call). Side benefit: `parse_is_check` currently guesses "lowercase
  means a binding" by convention (the `is Str surname` form) — the rule
  makes that principled.
- **N1b — uppercase module segments are a compile-time error, reported
  at source discovery.** Module paths *are* file paths [mod-file], so
  the rule reaches into the filesystem: `src/Utils.sv` stops being a
  legal program, and the diagnostic must name the file rather than
  surfacing later as a baffling unresolved import.
- **N1c — importing `Ns` brings `Ns.X` into scope.** Precedent:
  importing an *effect* brings its members (`ModuleScope` maps members
  to the owning effect). Dot-names carry their namespace at every use
  site, so auto-importing them cannot introduce an ambiguity — and
  without it every newtype costs an import line.
- The **collision ban is module-wide**, not file-wide: a module is all
  of its files, including backend define files (`list.sv` +
  `list.kotlin.sv`) [mod-visibility].
- The **collision ban also covers mangled names and imported names.**
  Overload mangling embeds qualifier names (`full_name__Surname`
  [kt-qual-mangling] [rs-fn-mangling]), so a dot-name must canonicalize
  its dot, and the resulting suffix can collide with a plain qualifier
  of the concatenated name imported from another module — which a
  file-scoped ban never sees. No encoding escapes this (Rust identifiers
  are `[A-Za-z0-9_]`, so every encoding is also a legal name), which is
  why a ban is the right mechanism; it just has to be scope-wide and
  applied to mangled forms.
- **Kotlin emits a *nested* class, never `inner`.** An `inner class`
  captures an outer instance and cannot be constructed without one.
  Consequence: a nested class of a *generic* struct cannot reference the
  outer type parameters (only `inner` can), so either `<ns>` must be
  non-generic or the member may not mention the outer generics.
- **Rust concatenates (`<ns><name>`) — the nested-module alternative is
  rejected.** Rust puts modules and structs in one *type namespace*, so
  `pub mod Environment` collides with `pub struct Environment`
  (E0428, verified with rustc: "`Environment` must be defined only once
  in the type namespace of this module") — and since `<ns>` is required
  to be a struct in the same file, that collision is guaranteed, not
  incidental. A mangled module (`Environment__ns::Id`) compiles but
  reads worse than the flat name; a lowercase module (`environment::Id`)
  also compiles but impersonates a Salvo module and adds a collision
  surface against real ones; Rust allows no item declarations in `impl`
  blocks and inherent associated types are unstable. Flattening is
  therefore the only clean rendering, which is what makes the collision
  ban load-bearing rather than a nuisance.
- Minor, but pin them: names cap at two segments (no `A.B.C`, and a
  dot-named struct may not itself be an `<ns>`); and `ty_base_name` /
  `type_base_name` plus the string-keyed define-template environments
  must agree on the canonical spelling of a dot-name — the documented
  "touch one, touch all three" trap.

Independent of D5: dot-names work the same for state qualifiers, and
apply to structs too, so this is a naming feature rather than part of the
subject axis.

What landed (rules [name-casing] [name-dot] [name-dot-import]
[kt-nested-dot-name]):

- **Parser**: `ident_decl_dotted` for struct/qualifier declarations and
  `type_ref_name` for every type position (annotations, `of` types, `is`
  checks, `as Q`, `canbe`, deduction lists, struct literals). A dot is
  only taken when *both* segments are uppercase, so `person.name` and
  `list.size()` are untouched. The name is carried as one dotted string —
  `Ident { name: "Environment.Id" }` — which is what keeps scope keys,
  checker types and the emitters' `ty_base_name`/`type_base_name`
  conventions in agreement without a second representation.
- **Casing rule**: `ident_type` / `ident_value` at every declaration site
  (structs, qualifiers, types, effects, handlers, generic parameters;
  fns, parameters, fields, `let`/`for`/destructuring bindings, lambda
  parameters). No existing Salvo source violated it — std and the
  corpora were already consistent.
- **Resolve**: casing-aware import splitting (trailing uppercase segments
  are the item, so `import env.types.Environment.Id` works while
  `import core.list.size` keeps its old positional meaning); an unaliased
  namespace import brings dot-named members (`want` matches the `Ns.`
  prefix); `ModuleItems::name_ref` exchanges a synthesized dotted lookup
  key for the declaration's own `&'p str`; `check_dot_names` enforces the
  same-file non-generic namespace, the scope-wide concatenation ban, and
  module-path casing.
- **Kotlin**: `emit_struct_with_members` nests members inside the
  namespace class (declared under their member segment, referenced with
  the dotted name); `qual_suffix` flattens dots so mangled overloads stay
  single identifiers (`label__EnvironmentTag`).
- **Rust**: `rs_ident` flattens dots — one funnel covers declarations,
  references and mangling, and no other Salvo name can carry a dot
  because dots are invalid in Rust identifiers.
- Verified end to end on both backends with two programs — a namespace
  struct with two members plus a dot-named qualifier driving an overload,
  and dot-named types as *union arms* (`when` narrowing, a dot-named
  qualifier in a nullable union, `canbe Mut` on a dot-named struct):
  identical output under `kotlinc` and `rustc`. Union arm identity needed
  no special handling — the wrapper/enum machinery keys off arm position,
  and the flattened Rust names fall out of `rs_ident`.



Ownership-arc remainders (each recorded in its milestone section and/or
spec rule; consolidated here for findability):

- **Fn-type effect lists** (`(v: T) [Console] -> ...`) parse but are not
  enforced as contracts — lambda bodies use the lexical effect
  environment [fn-contract].
- **`Once` inference**: a callee calling its fn param at most once does
  not auto-promote to `Once`; written only [once-fn]. Same
  written-validates/unwritten-infers pattern as deductions when taken.
- **Contracted lambdas passed to *overloaded* fns**: expected fn-type
  contracts only reach lambda literals for single-candidate callees
  (multi-candidate arg typing stays unbiased) [fn-contract]. Related:
  passing an *overloaded* fn by name resolves to the first overload
  (`entries[0]`) with no signature-based selection.
- **Returning/storing capture-carrying closures**: rustc lifetime error
  the checker does not reject; the recorded refinement is
  `move`-closure emission with hoisted clones, pending a treatment for
  captured effect-handler locals [fate-lambda].
- **Exactly-once closures**: a `Once` lambda may not consume a *linear*
  capture (the closure would inherit the obligation) [linear-lambda];
  supporting it means linear fn values.
- **Reassignable borrowed locals** (accumulator bodies in derived-return
  fns: `best = person; …; return best`) are rejected by provenance
  validation; supporting them is the recorded [readonly-return]
  refinement.
- **Effect members with their own generics** are not covered by the
  linear instantiation ban [linear-generics].
- **Same-call borrow/move (E0505 shape)**: one call that passes a
  borrow-emitted local *and* moves its root is checker-legal
  (left-to-right model) but rustc-rejected — loud, rare
  [rs-borrow-locals].
- **Internal qualifier unification** (folding links/poison/consumed_by
  into parameterized qualifiers with per-qualifier joins) remains
  unforced — L7c/L7d landed on links alone; revisit only when a
  customer needs it. L5 field-disjoint precision likewise only if
  whole-variable granularity pinches.

- `salvo lsp`: go-to-definition [lsp-definition] and doc-comment hover
  [doc-comment] landed, including nested declarations — struct fields,
  handler state, effect/handler/qualifier members. Still open: `[symbol]`
  resolution is name-based over the AST rather than
  import-visibility-exact [doc-symbol-ref]. Also still open:
  incremental analysis if workspaces outgrow
  re-check-everything-per-keystroke, and a `positionEncoding` negotiation
  for UTF-8-native clients. Signature *hover* still covers fn decls only
  — effect members and define fns have no `FnKey` (go-to-definition does
  reach effect members, via `def_refs`).
- **Predicate `is` on a union subject** is now roadmap phase **D4**, not
  a leftover: a `Ty::Union` subject always takes the arm-matching path,
  so `x is Positive` on `Int | Str` errors ("this check can never
  succeed") instead of calling `qualifies` — narrow first
  (`x is Int && x is Positive`). Lifting it requires qualifiers over
  unions [is-qualifies] [qual-union-arm].
- Struct destructuring ignores predicate-qualifier field overrides
  (deliberate: bindings get the declared type; direct accesses get the
  override + cast).
- Constructing a *nested* qualified union group in one expression
  (`ok(ok("yes"))` into `Ok (Ok Str | Err Int) | …`) needs an annotated
  intermediate `let`; single-level coercion only (errors, never mis-emits).
- Deduction inference does not track bare-parameter value flow out of
  branch/loop tails as a move (documented leniency in [deduce-infer]).
- **Field smart-casting landed (P1, 2026-09-03)**: after `h.field is Str`
  a read of `h.field` *is* narrowed ([flow-place]), tuple positions
  included ([expr-tuple-index]). What remains coarse: array elements never
  narrow, since an unknown index may alias any element.
- `yield` inside a *value-position* loop of an iterator body is a kotlinc
  error ("restricted suspending functions…"): the `run {}` value lowering
  is not an inline suspension scope. Loud, never silently wrong; the fix
  is a lowering that keeps the loop inside the `iterator {}` builder.
- Module reachability is name-based and conservative: a local variable
  shadowing a std fn name still pulls that std module in (harmless
  extra output, never a missing module).
- **DECISION (open, for the user):**
  - Binary operators are typed only for `None` [op-no-none]: everything
    else is unchecked (`Str * Bool` passes, result typing is just the
    left operand's type). Decide the operator typing rules — legal
    operand types per operator, numeric promotion, `Bool` for `&&`/`||`.

## Test inventory (all green: 481)

- `salvo-core`: 184 - 13 unit tests (file classification; `types.rs` union
  normalization, subtyping, display, wrapper detection; `place.rs`
  [flow-place]: the prefix relation reflexive and downward-closed,
  different roots never relating, overlap symmetric, an unknown array
  index aliasing every element while constant indices stay distinct, and
  `narrowable` accepting field chains only) + 2 source
  discovery tests (`tests/source_tests.rs` [mod-ignore]: `.svignore`
  skips listed files/subtrees; hidden and `CACHEDIR.TAG` directories
  skipped with the root exempt) + 19 deduction
  tests (`tests/deduce_tests.rs`: exhaustive lists dropping *undeclared*
  qualifiers and delta lists passing them through, mutating bodies
  requiring the exhaustive form, `Nothing` meaning moved
  [deduce-syntax], move inference, call-graph fixpoint
  transitivity, effect-member contracts reaching inference [call-resolve]
  and a keeping member still borrowing, handler *state* stores counting as
  moves so a keeping member that stores its parameter is rejected while a
  moving one is accepted [effect-state-store], written-list body
  validation,
  written-list shape validation, stricter-than-body lists,
  `let`-bindings linking instead of moving — the parameter stays kept,
  reads through the alias are free, `copy` severs [fate-link] — and
  move-mode bindings *claiming* parameters as moved, through binding
  chains and propagated through the call graph [fate-move-mode], a
  lambda capture-mutation claiming its parameter while a read capture
  keeps it [fate-lambda], and late move-mode candidates converging in a
  third round with the refined diagnostic superseding the raw
  derived-move error [deduce-fixpoint]) + 2
  structured-diagnostic tests (`tests/diag_tests.rs`: checker errors
  carry file index/span/severity and render with file:line:col + caret;
  multi-file programs index the declaring file [diag-structured]) + 5
  name-collision tests ([mod-collision]: conflicting imports, an import
  shadowing an own-module declaration, `as` resolving the conflict, a
  duplicate declaration within one module, and same-name fns staying
  overloads) + 3 generic-binding tests ([fn-overload]: bindings widen to
  the more general type in either argument order; unrelated bindings
  still reject) + 4 optional-strictness tests ([op-no-none]
  [interp-no-none]: possibly-`None` operands rejected for arithmetic,
  ordering, and equality — and `None` itself — while narrowed/asserted
  operands pass; interpolating an optional or a struct field rejected,
  the `is T name` binding and `!` forms accepted) + 7 name tests
  (`tests/name_tests.rs` [name-dot] [name-dot-import] [name-casing]:
  dot-names resolve, the namespace must be a same-file non-generic
  struct, the concatenated-name ban fires for own-module *and* imported
  names, importing a namespace brings its members and a member imports
  directly, an uppercase module path is an error naming the file; and
  7 `[name-resolve]` tests: an undeclared qualifier in an `is` check
  reported as the unresolved name with no "can never succeed" and no
  consumed-value cascade, an unresolved `when` branch not reported as
  non-exhaustive, unknown base types and qualifiers reported at ten
  declaration sites (params, returns, struct fields, `let`, aliases,
  effect members, `of`, `with`), the wrong-namespace wording hint both
  ways, import suggestions on an unknown type [diag-import-suggest], the
  intrinsic qualifiers staying known names, and `canbe` refusing a user
  qualifier [canbe-optin]) + 6
  subject tests (`tests/subject_tests.rs` [qual-subject]: a mutating call
  strips a state qualifier but not a provenance one, provenance composes
  without `with` while two state claims still need it, a provenance body
  is rejected, `is` on a non-union provenance value is rejected,
  provenance is droppable and survives being stored in a struct field)
  + 19 place-narrowing tests (`tests/place_tests.rs` [flow-place]
  [flow-place-invalidate], using `[interp-no-none]` acceptance as the
  observable: a checked field narrows inside the branch but not after it,
  siblings stay independent, field *chains* narrow, `&&` accumulates place
  facts, the `else` branch carries the negative fact, array elements do
  not narrow (P1a); and for invalidation — assignment to the place, to a
  prefix (dropping the subtree) but *not* to a sibling, a `Mut`-keeping
  call, a `Mut` call through a projection hitting only that subtree, a
  kept-*immutable* call preserving the fact (P1b), and a fact only some
  paths agree on not surviving the join; and for tuple elements
  [expr-tuple-index] — a constant index narrowing, siblings staying
  independent, an out-of-range index and a non-tuple base erroring, and
  assignment to an element rejected)
  + 16 member-resolution tests (`tests/member_tests.rs` [call-resolve]
  [field-resolve] [index-resolve] [iter-resolve]: unresolved bare and
  dot-calls rejected — the latter naming `external fn` as the remedy, both
  carrying import suggestions — calling a non-fn value and a generic
  rejected, a *declared* external still callable by dot-notation, fields
  on a non-struct/opaque/generic base rejected, `[]` on a non-array
  rejected while arrays still work, `for` over a non-iterable rejected
  while arrays still iterate, and — the leniency that remains — a member
  read off an un-inferred value adding *no* second diagnostic
  [type-unknown-lenient]; and 5 effect/handler-as-data tests
  [effect-not-data] [handler-not-value]: an effect rejected in five data
  positions with the diagnostic naming `use`, effect lists and `of` clauses
  still accepting effects, a handler constructor rejected as a value while
  `use` still registers it, and a handler dependency — E1's future feature
  accepted with its effect available in the member body, a handler
  depending on its own effect rejected, and dependencies resolved from the
  `use` scope — absent one, an error at the registration; and a mutual
  dependency unregisterable in *either* order, so cycles need no check
  [effect-handler-deps]; and 2 handler-state tests [effect-handler]: a
  state field initializer checked against its declared type, a well-typed
  one accepted)
  + 10 type-argument tests (`tests/type_arg_tests.rs` [call-type-args]:
  an undetermined type argument reported with both remedies; determined by
  the arguments, by an explicit list, by a `let` annotation, by the
  enclosing return type, and by a *concrete* parameter of a nested call;
  a *generic* parameter determining nothing, so the nested call is still
  reported; a type argument confined to the parameters needing no context;
  an unknown-typed argument keeping the call lenient so one mistake yields
  one diagnostic [type-unknown-lenient]; and the resolved bindings recorded
  per call, which is what `${T}` interpolates
  [backend-define-generics])
  + 9 widening tests (`tests/widen_tests.rs` [qual-widen]: a `^` branch head
  opening a nested union, `^ Mut` stripping in an `if` with the mutation it
  then rejects, the intrinsic qualifiers refused with their reasons from the
  shared exclusion list, nothing-to-remove rejected, a *type* on the right
  rejected, more than one arm rejected, the no-binding parse error, and a
  `^` branch consuming its arms so exhaustiveness still reports the rest)
  + 11 fn-type-effect tests (`tests/fn_effect_tests.rs` [fn-effects]: a
  declared effect available in a lambda body while an undeclared one is
  rejected even with the effect in lexical scope; a fn *inheriting* its
  fn-typed parameters' effects, through a qualifier too, with its caller
  required to supply them; variance both ways (a pure fn and a pure lambda
  fitting an effectful position, an effectful fn rejected by a pure one); an
  un-annotated lambda's effects inferred from its body; each declared effect
  required at the call, and a fn value called where its effect is
  unavailable rejected — the reason "forbid escape" was unnecessary; and
  `use` in a fn type rejected)
  + 19 abort/`try` tests (`tests/abort_tests.rs` [abort] [try]: the
  outcome type read off an annotation mismatch (`Ok Int | Aborted Str`),
  several message types unioning (`Aborted (Str | Int)`), an always-leaving
  body still carrying `Ok None`, a `try` that cannot abort rejected, an
  abort with nowhere to land rejected while declaring the effect
  propagates, a message the target cannot carry rejected, `main` declaring
  `Abort` rejected [abort-not-main], a handler *for* `Abort` rejected, an
  aborting branch counting as returning [fn-must-return] and not leaking
  its consumption to the fall-through path [type-any-nothing], a linear
  value across a may-abort call rejected with the `defer` remedy accepted
  [abort-linear], `abort` *and* a may-abort call inside a deferred block
  rejected [defer-no-escape], an inner delimiter taking only its own
  aborts [try-innermost]; plus the two nested-qualification shapes the
  design asked to test rather than assume — a nested result outcome and a
  union message, both taken apart through a binding at the inner type — and
  the diagnostic that names that remedy when a qualified union is matched
  directly [when-union-subject])
  + 12 deferred-block tests (`tests/defer_tests.rs` [defer]
  [defer-no-escape]: a linear obligation discharged on both paths of a fn
  with an early `return` — with the no-`defer` control proving the
  acceptance means something — and per iteration on a loop body's
  `continue` path; a manual consume plus a deferred one reported as a
  use-after-move, exactly *once* even though the block is applied at two
  exits; a read after a deferred consume staying legal (the deferred code
  runs later); a narrowing the body relied on rejected when a `Mut` call
  invalidates it before the exit, with the surviving-fact control accepted;
  `return`/`break`/`continue` in a deferred body rejected while a loop
  written *inside* it keeps its own; and the body's own linear value owed
  inside the body).
  + 12 `when`-condition tests (`tests/when_tests.rs` [when-condition]
  [cond-bool]: the subject-less chain accepted; the mandatory `else`
  keeping `None` out of the value, with the `if`-without-`else` control
  showing the `Str?` it replaces [if-else-none]; a missing `else`, an
  `else`-only `when`, and a branch after the `else` rejected; an `else` in
  the *subject* form rejected with the diagnostic naming the other form;
  `is` heads narrowing their branch *and* the `else`; a chain returning
  from every branch counting as the fn's return [fn-must-return]; and the
  boolean rule on all four condition positions, on the offending *leaf* of
  a compound condition only, on `Bool?` (with `!` accepted), and staying
  quiet on an un-inferred type [type-unknown-lenient]).
- `salvo-cli`: 56 - 46 `analyze` integration tests running the built
  binary (`tests/analyze_tests.rs` [cli-analyze]: clean program exits 0,
  type errors render with location and exit 1, JSON diagnostics
  (populated + empty array), parse errors reported, a parse error in one
  file not suppressing checker diagnostics in others, `.svignore`
  exclusions [mod-ignore], import suggestions rendered as help lines +
  JSON `imports` for std and user modules [diag-import-suggest],
  missing-return analysis [fn-must-return], use-after-consume from
  declared *and inferred* deductions, uniform across types, incl.
  reassignment revival, consumption surviving `is`-narrowing restores,
  `while`/`for` loop back-edge re-checking with consume-then-revive
  staying clean, the `if`/`else` merge matrix (consumed on every
  fall-through path / on the only fall-through path / only on an
  always-exiting path), `when`-arm merging incl. subject consumption,
  partial qualifier removal joining conservatively, and call-site
  qualifier removal for kept params incl. `[list:]` [deduce-consume],
  bodyless-declaration explicitness ([decl-explicit]: an external missing
  all three parts, an effect member missing the two that apply to it and
  *not* asked for effects, an effect member's declared deductions enforced
  at the call site, and std's `add` consuming its element),
  the D1 deduction forms ([deduce-syntax]: the `clear`/`NonEmpty`
  unsoundness now rejected, a delta preserving an undeclared qualifier, a
  bodyless `Mut` parameter refusing the delta form, a mutating body
  refusing keep-all, mixed polarities, `+Qual`, and non-`Nothing` types),
  union-arm arguments resolving against union params [type-union],
  std's result tags working with no local declarations
  (`Ok Int | Err Str | None` narrowed by `when`, [qual-result-tags]) and
  an undeclared qualifier reporting the *name* rather than the arm
  mismatch it causes [name-resolve],
  the shared-fate matrix (derived variables read-only for
  moves/mutations/returns with the `copy` remedy [fate-derived-readonly],
  root mutation/move/reassignment poisoning derived variables with
  use-site errors naming the event — and unobserved poison staying
  silent [fate-poison], links flowing through projections, `for` and
  `is` bindings transitively to the root, reassignment revival, links
  unioning across branch merges [fate-link], `copy` producing
  independent values that make the whole matrix pass [copy-fn],
  and the backend-parity fix: a projection argument in a kept-`Mut`
  position mutating its provenance roots, poisoning derived variables
  and rejecting mutation through derived ones),
  `[struct-mut]` field assignment requiring a `Mut`-qualified struct
  value,
  the L2 move-site matrix (struct/array/tuple literal stores, spread,
  `break` inside an always-exiting branch consuming after the loop,
  `yield`-in-loop back-edge, `use` constructor arguments — each with the
  event named in the diagnostic, derived variables rejected at the new
  sites, and the positive side: `copy` at every site, reassignment
  revival incl. ahead of the loop back edge, `break n` consuming only
  its operand, and in-branch `return` poison not leaking to the
  fall-through path [deduce-consume] [fate-derived-readonly]),
  the S2 move-mode matrix ([fate-move-mode]: the zero-clone pipeline
  and mutation-driven move-mode staying clean, per-iteration loop
  bindings consumable, immutable projections in moved positions free,
  `copy` keeping sources usable; and the negative side: ancestors
  consumed at the binding with the use-site error naming the binding
  through whole chains, the moved-position parity probe rejected,
  kept-parameter projections erroring with the `copy` remedy, and
  written-kept bindings keeping the S1 move-site error),
  same-call argument ordering ([deduce-same-call]: double moves and
  move-then-read within one call rejected at the later argument, `copy`
  at the consuming argument and kept-position reads clean),
  the L4 capture matrix ([fate-lambda]: immutable captures free,
  closure poisoned by a mutable read-capture's root mutation, mutated
  captures consumed at creation with claims reaching callers,
  written-kept parameter capture-mutation rejected, capture consumption
  always rejected, `copy` remedies clean),
  the L6 linearity matrix ([linear-obligation]: scope-exit leak,
  consumed-on-some-paths-only, dropped expression result, overwrite of
  a live value, return-while-owing, `copy` refused, lambda swallow
  rejected — with pass/discard/return/kept-borrow/alias/composite all
  clean — and the generic instantiation ban [linear-generics]),
  the L7a opt-in matrix ([linear-generics]: opted bodies checked with
  `T` linear, unopted forwarding rejected, non-`Linear` clauses
  rejected, variadic positions refusing linear values with their
  follow-on leaks, and the opted-std `List<FileHandle>` workflow
  clean),
  the L7b `Once` matrix ([once-fn]: double call and loop back-edge
  call consumed, `Once` lambdas rejected at plain-fn boundaries,
  escape-then-call rejected, `Once` on non-fn types rejected, with
  maybe-call and both subtyping directions' positives clean),
  the L7c derived-return matrix ([readonly-return]: independent
  returns rejected, moved/unknown parameters rejected, argument
  mutation poisoning the result, borrowed results immovable with the
  `copy` remedy clean, direct element returns and forwarded derived
  calls clean),
  the L7d contract matrix ([fn-contract]: double use through a
  consuming contract rejected, a consuming named fn rejected at a
  keeping boundary, kept lambda parameters unconsumable, consuming
  contracts propagating to callers, keeping contracts clean),
  `--backend` opting
  define files into the analysis, unknown backend rejected) + 2 UTF-16
  position-mapping unit tests (`src/lsp.rs` [cli-lsp]: multi-byte and
  supplementary-plane round-trips, clamping) + 3 LSP integration tests
  (`tests/lsp_tests.rs` [cli-lsp]: speaks framed JSON-RPC to the binary —
  initialize, didOpen of an unsaved broken buffer -> publishDiagnostics
  with UTF-16 range, didChange fix -> clearing publish, hover -> checked
  type, fn-name hover -> full signature with inferred deductions at both
  the declaration and a call site [fn-ref-table], derived-variable
  hover -> bare `ReadOnly T` type line with root/binding-site detail
  below [fate-link],
  shutdown/exit -> clean process exit; codeAction import quickfix
  round-trip [diag-import-suggest]; go-to-definition for a call-site
  callee, a cross-file struct, an effect member, a handler in `use`, and
  an effect in `of`, plus a no-name position yielding nothing
  [lsp-definition]; and doc-comment hover [doc-comment]: fn docs after the
  signature block, markdown verbatim, a resolvable `[symbol]` becoming a
  link while an unresolvable one stays literal [doc-symbol-ref], struct
  docs with the blank-line-separated comment above them excluded and a
  **Fields** section listing typed/defaulted fields with their own docs
  [doc-struct-fields], the same docs at a *use* of the struct name, and a
  variable hovering as its narrowed type with the declared type named
  below — and as its plain declared type on the parameter itself
  [doc-hover-narrowed]; and nested-declaration hover: a struct field at its
  declaration and at an access (identical contents, via
  `Checked::field_refs`) with its default as written and its owner named,
  an effect member at its declaration and at a call, handler state, and a
  handler member whose `[symbol]` references reach the handler's own state
  — plus go-to-definition on a field access [lsp-definition])
  + 3 grammar tests
  (`src/lang.rs` [cli-lang]: highlighting categories exactly partition
  the lexer's keyword table, generated grammar is valid JSON containing
  every keyword, checked-in VS Code grammar matches the generated one).
- `salvo-syntax`: 51 (the corpus grew three LANGUAGE.md examples with E3: a
  `defer` in `control_flow.sv`, `abort`/`try` in `effects.sv`, an effectful
  fn type in `functions.sv`) - std +
  LANGUAGE.md-corpus parse-clean assertions with
  insta AST snapshots (`tests/corpus/*.sv`, plus `std/core/result.sv`),
  error-reporting tests,
  lexer unit tests for numeric literal suffixes [lit-numeric] (`1L`,
  `1.2f`, invalid suffix/juxtaposition errors, `1.size()` stays an int),
  5 tuple-index tests ([expr-tuple-index]: `t.0` parses as a projection,
  `t.0.1` as *two* projections rather than a float, floats still lexing as
  floats where an index cannot appear, a tuple element as a dot-call
  receiver, and numeric suffixes rejected),
  3 `canbe` opt-in tests ([canbe-optin]: `canbe Mut` on a struct and
  on an `external type`, `<T canbe Linear>` on a fn, and `canbe` on a
  non-fn type parameter rejected [linear-generics]), and 5 name tests
  ([name-dot] [name-casing]: dot-names in declarations and type
  positions, a dot-name struct literal distinguished from a field read
  and a dot-call, three-segment names rejected, the casing rule enforced
  across ten declaration forms, generic parameters uppercase), 5 doc-comment
  tests ([doc-comment] [doc-struct-fields]: the block directly above a
  declaration with a blank line ending it and a bare `//` kept, trailing
  comments documenting nothing, struct and field docs captured separately,
  docs surviving `external`/`internal`/`provenance` modifiers, and
  indentation kept after the marker), and 2
  subject tests ([qual-subject]: `provenance qualifier` parses with the
  provenance subject while a plain declaration defaults to state;
  `provenance` must precede `qualifier`), 2 `defer` tests ([defer]:
  `defer { ... }` parses into a block statement; a bodyless `defer` is a
  parse error naming the form), and 2 `try` tests ([try]: `try { ... }`
  parses as a block *expression*; a bodyless `try` is a parse error naming
  the form), and 6 subject-less `when` tests ([when-condition]: the
  condition chain parsing into `Expr::WhenCond` with its branches and
  `else`, a subject still parsing as the arm form, and the four parse
  errors — missing `else`, `else`-only, a branch after the `else`, and an
  `else` in the subject form).
- `salvo-backend-kotlin`: 109 - golden snapshots of the M2 demo, the M3
  unions demo, the M4 qualifiers demo, the M5 effects demo, and the M6
  loops demo;
  M7 assertions (only-used-modules + companion copying, per-module
  packages + generated imports, alias imports, effect-param collision
  avoidance, unique destructure temps);
  M8 `Mut` assertions (`Mut List<T>` maps through the `Mut inline:`
  template; `Mut` on a non-`canbe Mut` type is an error [type-canbe-mut]);
  wrapper/wrap/`is`-lowering assertions (`unions_emit_sealed_wrappers`),
  predicate/mangling/field-cast assertions
  (`qualifiers_lower_to_predicates_and_mangled_overloads`), checker-driven
  effect-resolution assertions (`effects_resolve_through_checker_tables`),
  loop-lowering assertions (`loops_lower_to_run_blocks`);
  union-arm argument wrapping at call sites [type-union]; numeric literal
  suffixes; handler-member template returns
  [kt-handler-template-return]; predicate-qualifier constructors
  [qual-ctor-predicate];
  negative tests (non-exhaustive `when`, non-union `when` subject,
  no-matching-arm wrap, missing effect handler at a fn call site and at an
  effect-member call site, `use` without the `use` effect, duplicate effect
  in an effect list, duplicate `use` registration, unknown effect,
  ambiguous generic effect call, handler-member effect deps, duplicate
  qualifier, incompatible qualifiers, `of`-type mismatch, constructor
  same-file rule, non-simple constructor
  return, `is` on constructive qualifiers, `qualifies` signature,
  constructive values only from constructors, `break` outside a loop,
  missing defines for used external fns/types, uncovered core externals,
  companion/generated-file collision); `copy` intrinsic lowering
  assertions (identity / `.toMutableList()` / `.copy()` / `.copyOf()`
  [kt-copy] [internal-fn]) and a negative test (`copy` of nested
  mutability is a codegen error);
  define/external pairing ([decl-explicit]: same-name defines dispatching
  by parameter base types, a define with no external rejected, two defines
  for one external rejected);
  general-sweep assertions (precedence-preserving binary rendering,
  iterator-body `return` retargeting through a value-position loop,
  alias imports of mangled qualified overloads keeping the `__Qual`
  suffix [kt-imports] [kt-qual-mangling], field-subject `is` lowering
  [is-narrowing] with `when` still rejecting field subjects
  [when-union-subject], place narrowing [flow-place] (a narrowed nullable
  field read asserting [kt-narrow-field-assert], a narrowed field *chain*
  read, a narrowed wrapper-union field read taking its arm payload — plus
  the kotlinc run, whose stdout matches the Rust backend's byte for
  byte); a narrowed `val` field *not* asserted (kotlinc smart-casts it, so
  the assert would only warn) and a narrowed field used as an operator
  operand [op-no-none], union coercion inside array/tuple literals and
  lambda tail returns, type-directed dispatch for unchecked define
  overloads plus the ambiguity error [backend-never-wrong], effect
  member generics rendered on the interface and bound per call
  [effect-member-generics], and an aliased effect type resolving to the
  handler registered under the canonical type
  [effect-disambiguation], and the std array functions with the
  LANGUAGE.md `CyclicRandom` handler [type-array]);
  handler-dependency assertions
  ([effect-handler-deps]: the dependency as a constructor field, the
  member signature still matching the interface, the `use` site supplying
  it, callers not mentioning it);
  define-template type-argument assertions ([backend-define-generics]:
  `${T}` interpolated from a `let` annotation and from an explicit type
  argument, and std's list constructors carrying their element type —
  `mutableListOf<Int>()`, which is the form kotlinc requires);
  and twenty-seven kotlinc compile+run tests
  with exact stdout assertions (including the M7 multi-module program
  with packages, generated imports, and a companion file, the S1
  copy demo, the S2/S3 move-mode and borrow demos, the L6 linear
  resource demo, and the L7a–L7d linear-generics, `Once`,
  derived-returns, and fn-contracts demos —
  emission aliases throughout, stdout identical to the Rust runs
  [fate-move-mode] [fate-link] [linear-static] [once-fn];
  the last four are the handler-dependency programs the Rust fusion runs —
  the same sources, the same asserted stdout, which is what parity means
  here [effect-handler-deps] [rs-effect-fusion]);
  and 2 fn-type-effect tests ([fn-effects] [kt-fn-effect-params]:
  `fn_type_effects_thread_into_lambdas` asserting the inherited effect in
  the signature, the effect as a leading *lambda parameter* rather than a
  capture, a matching named fn passing as `::name` and a pure one wrapped in
  an adapter; plus the kotlinc run of the program the Rust fusion used to
  reject, with the same stdout);
  and 2 abort tests ([abort] [try] [kt-abort-signal]:
  `abort_lowers_to_a_signal_and_try_to_a_catch` asserting the generated
  stack-trace-less signal, a tagged `throw`, a *plain* call in the
  propagating frame (no colouring), the delimiter's tag dispatch with its
  rethrow fallback, and no interface emitted for the effect; plus the
  kotlinc run of the abort demo, whose stdout matches the Rust run byte for
  byte);
  and 2 `defer` tests ([defer] [kt-defer-finally]:
  `defer_lowers_to_try_finally` asserting one `try` per `defer`, nested
  latest-first, and a single `finally` covering both `return`s of a
  two-exit fn; plus the kotlinc run of the defer demo — LIFO at a block
  end, an early `return`, `continue`/`break` out of a loop body, and a
  linear handle released on both paths — whose stdout matches the Rust
  run byte for byte)
  and 2 subject-less `when` tests ([when-condition] [kt-when-cond]:
  `a_subjectless_when_emits_a_subjectless_kotlin_when` asserting the
  Kotlin `when {` with `cond ->` arms, a plain `else ->` with no optional
  filler, and the `is` binding declared inside its arm; plus the kotlinc
  run of the demo — value and statement position, `is` heads, a chain
  returning from every branch, and one nested in a subject `when`'s arm —
  whose stdout matches the Rust run byte for byte).
- `salvo-backend-rust`: 81 - golden snapshots of the same five demos
  emitted as Rust; deduction-mode assertions
  (`deductions_drive_parameter_modes`: kept -> `&`, kept+Mut -> `&mut`,
  omitted -> move, matching call-site argument shapes [rs-borrows]);
  union-enum assertions (`unions_emit_enums`), predicate/mangling
  assertions (`qualifiers_lower_to_predicates_and_mangled_fns`),
  effect-trait assertions (`effects_lower_to_traits_and_mut_dyn_params`),
  loop-lowering assertions (`loops_lower_to_block_expressions`),
  crate-layout assertions (`crate_layout_mounts_only_used_modules`
  [rs-crate] [rs-imports]); numeric literal suffixes emit explicit types;
  predicate-qualifier constructors emit plain fns [qual-ctor-predicate];
  negative tests (missing defines for external
  fns/types, uncovered core externals, generic effect members
  [rs-effects]); `copy`-lowering assertions (`.clone()` on the
  argument's place, and fate-linked `let`s cloning instead of moving
  [rs-copy] [fate-link]); move-mode emission assertions
  (`move_mode_bindings_emit_real_moves` [fate-move-mode]: claimed
  parameter taken by value, loop by value, partial field move, move-mode
  `let` moving, pipeline clone-free); borrow emission assertions
  (`borrow_mode_bindings_emit_borrows` [rs-borrow-locals]: `&Vec`
  parameter iterated bare by reference, `&T` field binding, borrow
  alias of an owned local, read-only pipeline clone-free); `discard`
  lowering assertions (`drop(...)` [linear-discard]);
  general-sweep assertions (field-subject `is` lowering
  [is-narrowing], place narrowing [flow-place] (narrowed nullable field
  and field-chain reads unwrapping the `Option`, a narrowed wrapper-union
  field read using the arm accessor — plus the rustc run against the same
  expected stdout as Kotlin) and a narrowed field as an operator operand
  [op-no-none], union coercion inside array/tuple literals and lambda
  tail returns with fn-type `let` annotations dropped [fn-contract],
  type-directed dispatch for unchecked define overloads plus the
  ambiguity error [backend-never-wrong], and an aliased effect type
  resolving to the handler registered under the canonical type
  [effect-disambiguation], and the std array functions with the
  LANGUAGE.md `CyclicRandom` handler [type-array]); and twenty-five rustc
  compile+run tests with exact stdout assertions mirroring the kotlinc
  set (demo, unions, qualifiers, effects, loops, multi-module, copy,
  the S2 zero-clone move-mode demo, the S3 borrow demo, the L6 linear
  resource demo, the L7a linear-generics workflow demo, the L7b
  `Once` demo — with `once_fn_params_emit_fnonce` asserting the
  `impl FnOnce` rendering [once-fn] — the L7c derived-returns demo,
  with `derived_returns_emit_borrows` asserting the elided and
  generated-lifetime signatures and the borrow returns
  [readonly-return], and the L7d contracts demo, with
  `fn_type_contracts_emit_modes` asserting `&mut impl FnMut`
  signatures with contract-mode argument types and the named-fn
  adapter [fn-contract]);
  define-template type-argument assertions ([backend-define-generics]:
  `${T}` interpolated into `Vec::<i32>::new()` from an annotation and from
  an explicit type argument, with a rustc run);
  fusion assertions ([rs-effect-fusion]: the dependency absent from the
  struct and from `new`, member bodies in the generated `__Impl_H` trait,
  the fusion chaining through `__outer` and owning the handler, the
  disjoint-field-borrow destructuring, a generic fused parameter forwarded
  to a smaller callee, the two-dependency `__Deps_H` adapter, a
  conjunction trait with its blanket impl, UFCS dispatch for a generic
  effect, nested effect calls hoisted — plus
  `programs_without_handler_dependencies_do_not_fuse`, which pins the
  gate: no dependency anywhere means the per-effect `&mut dyn` parameters
  are untouched; and `generic_dependent_handler_is_a_codegen_error`
  [backend-never-wrong]); and five further rustc compile+run tests for
  the fusion (the handler-deps program Kotlin also runs, a stress program
  with handler state behind a dependency, a two-dependency handler,
  nested `use` scopes with the outer one reused, a `use` inside a fn that
  already has effects and inside a loop body, a dependency chain, a
  qualifier's `qualifies` effects, the `use` site in a different module
  from the handler, constructor parameters mixing a dependency with plain
  data, a dependent member calling a fn that does its own `use`, an
  effect-using lambda passed to an effect-free higher-order fn, three
  effects in one signature, a `use` in a `while` body, and an effect member
  taking a `Mut` parameter); and
  `effect_using_fn_value_at_an_effectful_call_is_a_codegen_error`
  [backend-never-wrong]; and 2 `defer` tests ([defer] [rs-defer-splice]:
  `defer_splices_at_every_exit` asserting LIFO order at a block end with no
  scaffolding, the hoisted `return` value, and the loop body's deferred
  code appearing at its `continue`, its `break` and the block end; plus the
  rustc run of the same demo Kotlin runs, with the same stdout); and 3
  abort tests ([abort] [try] [rs-abort-controlflow] [rs-try-label]:
  `abort_lowers_to_controlflow` asserting the `ControlFlow<M, T>` return
  shape, `abort` as a `Break` return, `Continue`-wrapped returns, no trait
  for the effect, and the deferred release on the abort path of a
  propagating call; `try_lowers_to_a_labelled_block` asserting the label,
  the *absence* of a closure, and the message wrapped into its union arm;
  plus the rustc run of the demo Kotlin also runs); and 3 fn-type-effect
  tests ([fn-effects] [rs-fn-effect-params]:
  `fn_type_effects_thread_into_closures` asserting the `&mut dyn` effect in
  the closure *type* and as a leading closure parameter, and the named-fn
  adapter forwarding or ignoring it;
  `rustc_compiles_and_runs_effect_using_fn_value`, which is the **lifted
  fusion cut** — the program this test used to assert *could not* be
  emitted; and the shared demo Kotlin also runs); and 2 subject-less
  `when` tests ([when-condition] [rs-when-cond]:
  `a_subjectless_when_emits_an_if_chain` asserting the
  `if`/`else if`/`else` chain, no `else { None }` filler in value position
  and no `unreachable!()` arm (the mandatory `else` makes it total), and
  the `is` binding declared inside its branch; plus the rustc run of the
  demo Kotlin also runs, with the same stdout).

When intentionally changing std, the parser AST, the checker's lowering, or
the emitter output, rerun with `INSTA_UPDATE=always` and review the
snapshot diffs.

## Gotchas / lessons learned

- **A rule can sit in the spec for eight milestones without being
  enforced.** `[if-bool]` ("`if`/`elif` conditions must be boolean
  expressions; there is no truthiness") was written in M0 and never
  checked: `analyze_cond`'s fallback arm typed the condition and threw the
  type away. Nothing caught it because nobody *wrote* a truthy condition —
  the spec was describing a convention the authors were already following.
  When adding enforcement to a stated-but-unchecked rule, expect the sweep
  to come back empty and do not read that as evidence the rule was
  redundant; the next contributor is who it is for. Worth auditing the
  other "must"s in LANGUAGE_SPEC.md the same way.
- **New AST variants are only half-caught by the compiler.** Adding
  `Expr::WhenCond` produced five `non-exhaustive patterns` errors (the two
  `check_expr` dispatches, `expr_defer_escape`, and each emitter's
  expression dispatch); the *dozen* other traversals that needed an arm end
  in `_ => {}` or `_ => false` and compiled silently. The ones that matter
  fail quietly: `reach.rs`'s `expr_names` (a module used only inside the
  new construct would have had its import pruned), `block_exits` /
  `block_returns` / `expr_terminates` (a total chain would not have counted
  as returning), `collect_assigned_expr` / `expr_mentions`,
  `deduce.rs`'s walk, `collect_mutated` / `collect_declared` in both
  emitters, and each emitter's `emit_operand` parenthesization list. Grep
  `Expr::If` and `Expr::While` — the two variants every traversal handles —
  and add an arm at each site rather than trusting the build.

- **"Erased at runtime" does not mean "no lowering".** `^` removes a
  qualifier, and qualifiers are erased, so it looked like a typing-only
  feature — but where the qualifier sat on a *union arm*, the value's
  physical view moved inside a wrapper, and reads had to peel it. The
  emitted code compiled and ran with plausible output while taking the wrong
  branch. Two lessons: a wrapper-arm change is a representation change even
  when the *type* change is pure, and a generated `Display` that prints
  "whichever arm I hold" can hide exactly this class of bug — so test a
  branch that distinguishes the arms (`err("zero")`, not just the happy
  path).
- **An expected type that only flows to *some* argument shapes will bite.**
  `resolve_named_call` passed the parameter type down for lambda and call
  arguments but not for a bare identifier — a deliberate old choice ("other
  expressions keep the historical untyped probe"). With [fn-effects] that
  silently broke a *named fn passed by value*: it never saw the fn type it
  had to adapt to, so the Kotlin emitter kept `::plain` where a
  `(Console, String) -> String` was wanted. When a feature depends on the
  expectation reaching an argument, check *which* argument shapes get it.
- **Innermost-first is the right search order for a scoped environment.**
  Both emitters looked up effect values from the front of `effect_env`, so a
  lambda's own effect *parameter* lost to the enclosing fn's value and the
  closure captured after all. The same reversal was needed for the
  base-name fallback, where "the same effect twice" (an inner scope
  shadowing an outer) had to stop counting as ambiguity while two genuinely
  different generic instances still do.
- **A new file under `std/` needs a touch to be seen.** `std/` is embedded
  into the CLI with `include_dir`, which has no rerun-if-changed trigger for
  *added* files: `cargo run -- analyze` kept reporting "7 std files" after
  `std/core/abort.sv` appeared. `touch crates/salvo-cli/src/main.rs` (or any
  edit to the crate) picks it up.
- **A hand-maintained copy of a keyword list will drift, and this one
  panicked the compiler.** `TokenKind::symbol()` matched every keyword
  explicitly and `unreachable!()`d otherwise, so *any diagnostic* mentioning
  the new `defer`/`try` tokens crashed instead of reporting — and the crash
  looked like a parser bug, not a diagnostic bug. It now resolves through
  `KEYWORDS`. When adding a token, grep for the enum name: a second match
  arm somewhere is a liability.
- **Write the target-language shape by hand before choosing a lowering.**
  The roadmap's `try` sketch used a closure returning `ControlFlow`; hand-
  writing it showed the closure has to capture the fn's effect parameters
  (the same exclusivity trap the fusion hits), while a *labelled block*
  captures nothing. Ten minutes of `rustc` saved a rewrite — and the same
  probe confirmed `?` on `ControlFlow` is stable and that a may-abort call
  in a loop stays a loop.
- **`?` is not usable where deferred code must run**: it returns without
  running the splice. That is why a may-abort call with pending `defer`s
  becomes an inline `match` — and it is the concrete reason `defer` had to
  be built before `abort` rather than after.
- **Where a union arm gets wrapped is a backend-shaped decision.** Rust
  wraps at the propagation site (it has one); the JVM does not have one, so
  Kotlin must choose the arm at the `catch` — which the throwing frame
  cannot know. The fix is a type-*name* tag on the signal, not an `is` test
  on the payload: erasure makes `List<Int>` and `List<Str>` the same class,
  and the wrapper encoding exists precisely to avoid such tests.
- **A lowering sketched in a roadmap can be unbuildable — check the
  motivating program against it.** E3's `defer` sketch called for a Rust
  `Drop` guard ("reverse declaration order gives LIFO for free"). It cannot
  work: the deferred call *consumes* the handle, so the guard has to own it
  from the `defer` statement onward, which makes the handle unusable for
  the rest of the block — the exact code `defer` exists to enable. Writing
  the three-line target program by hand before implementing would have
  shown it immediately (and did, once asked).
- **Splicing at exits means the block-end splice can be dead code — and
  rustc borrow-checks dead code.** A fn whose last statement is `return`
  got the deferred body twice: once before the `return`, once after it. The
  second copy used a value the first had moved. `unreachable_code` is only
  a lint, but a use-after-move there is an error, so blocks whose own
  statements always exit skip the trailing splice (`block_terminates`).
- **Kotlin's `try` is an *expression*, which is what makes the `finally`
  lowering fit everywhere.** Wrapping "the rest of the block" in
  `try { … } finally { … }` keeps working in value position — the block's
  value is the `try` block's tail — so `defer` needed no result-local
  machinery on that side, unlike the Rust splice, which has to hoist the
  tail into a temporary.
- **Check a deferred body once, replay its effect at each exit.** The
  tempting alternative (re-check the body at every exit) writes the
  checker's type/coercion side tables repeatedly, and if two exits disagree
  on a narrowing the emitters get whichever recording came last — a
  silently wrong `!!` or unwrap. Checking once (in the flow state at the
  `defer`, snapshot/restored) and replaying a *summary* — what it consumes,
  what it weakens, and the facts it relied on — keeps one recording and
  makes the disagreement a diagnostic instead of a miscompile.
- **When replaying flow effects, only ever lose facts.** The summary is
  computed in the state at the `defer`; at an exit the state may be
  *narrower* (a later `if x is None { return }`), so re-imposing the
  recorded narrowing would resurrect a fact that path does not have. The
  replay widens only when the body genuinely invalidated the narrowing, and
  never re-adds place facts.

- **A codegen symptom can be a checker hole.** "Kotlin emits
  `mutableListOf()`" looked like a one-line template fix
  (`mutableListOf<${T}>()`). It was not: the *checker* had never determined
  the element type either, so there was nothing to interpolate — and the
  same hole was silently accepting `add(xs, 1); add(xs, "two")` on one list.
  The template change only became possible *after* the checker learned to
  demand the type. When a backend cannot render something, ask what the
  checker knows about it before reaching for the emitter.
- **The absence of a diagnostic is evidence.** Both holes found in that
  session were found the same way: writing a program that *should* be an
  error and watching it pass. `let xs = mutable_list(); add(xs, "two")` and
  `held: Mut List<Int> = "definitely not a list"` were each accepted with no
  errors at all — the second because handler state initializers were never
  checked, only their declared types validated. A cheap habit with real
  yield: for any construct, write the obviously-wrong version and confirm it
  is rejected.

- **A design recorded before implementation is a hypothesis.** The fusion
  strategy was written up in detail, with rustc-verified snippets, *before*
  being built — and two of its load-bearing choices turned out to be wrong
  in ways the snippets could not show, because a snippet exercises one
  shape and a program exercises their composition. `dyn` fused parameters
  cannot forward to a callee needing a *subset* of the effects (trait
  upcasting reaches supertraits only), and flat rebuilding over the outer
  scope's handler locals makes the *outer* fusion unusable after an inner
  block. Both were found within an hour of writing the whole shape out as
  one compiling program. The write-up still paid for itself many times
  over: the reasoning it recorded is what made the fixes obvious. Record
  the design *and* expect to revise it — and prototype the composition,
  not the pieces.
- **Under a fusion, "one value with every capability" creates aliasing
  where there was none.** Two habits that were safe with one parameter per
  effect become `E0499`: an effect call inside another call's arguments
  (`fx.a(&fx.b())`), and a method call that is ambiguous because two
  effects in scope share a member name. The fixes are mechanical (hoist
  arguments into a temporary; dispatch by UFCS), but they are only
  *discoverable* by compiling a program that does both — a single-effect
  test program never trips either.
- **Environment entries that get rewritten cannot be restored by
  truncation.** The emitter's effect environment was scoped by
  `truncate(depth)`, which is correct only while entries are immutable. The
  fusion *rewrites* outer entries (every effect now threads through the
  inner fusion), so leaving a block had to restore a saved clone —
  otherwise the outer scope kept pointing at a fusion local that had gone
  out of scope. Whenever a scope-restore mechanism is a depth counter, ask
  whether anything mutates the entries below the mark.

- **"Loud" is not the same as "an error".** The interop leniency did not
  emit *wrong* code — kotlinc and rustc both rejected what it produced —
  which is why it survived so long. But the diagnostic pointed at
  generated code the author never wrote, and Kotlin's version
  (`unresolved reference 'n'`) named a symbol that plainly exists in the
  Salvo source. When judging a leniency against
  [backend-never-wrong], ask *where the error surfaces*, not just whether
  one does.
- **Leniency with no customer is pure risk.** Removing the interop
  pass-through that dated from M3 (unresolved calls, dot-calls, non-fn
  callees, fields on non-structs, non-array subscripts, non-iterable
  `for`) broke exactly **one** test — and that test's premise *was* the
  leniency. If a permissive path has no test that needs it, it is not a
  feature.
- **Lexer rules can block a syntax before the parser sees it.** `t.0.1`
  cannot be parsed as two tuple indices while the lexer still owns the
  decimal point: it hands over one `Float` token, and no parser trick
  recovers the digits (`.0.10` and `.0.1` are the same `f64`). Fixing it
  in the lexer — no fraction directly after `.`, the one position where
  the grammar cannot hold a numeric literal — made the parser side
  trivial. Where two layers disagree about who owns a character, the
  earlier layer is usually the cheaper place to fix it.
- **Kotlin does not smart-cast properties.** A narrowed nullable *field*
  read cannot rely on the smart cast a narrowed local gets: kotlinc
  rejects it ("smart cast to 'String' is impossible, because 'surname' is
  a mutable property that could be mutated concurrently"), and every
  `canbe Mut` struct field emits as `var`, so the hazard is the common
  case, not a corner. Narrowed nullable field reads emit `!!`
  ([kt-narrow-field-assert]). The lesson generalizes: when a checker fact
  is discharged by the *target language's* own analysis, check that the
  target's rules are at least as permissive — here Rust (explicit
  `unwrap`) was fine and Kotlin was not, which is exactly the asymmetry
  the backend-parity principle exists to catch. It only surfaced because
  the e2e test compiles the output; a golden-string test would have
  passed.
- **Place facts belong on the root, not in a new map.** Keying narrowing
  by `Place` looked like it needed a place-keyed frame map — a second
  structure for `snapshot_narrows`/`restore_narrows`/`merge_fallthrough`
  to carry, against the S1 gotcha below. Storing the projection facts
  *inside* the root's `LocalVar` instead kept those three functions the
  single source of truth, and made "an event on the root invalidates
  everything below it" fall out of clearing one list.
- **Restores must not resurrect invalidated facts.** `with_narrows`
  reinstates the pre-branch narrowing on exit, and the variable version
  already had to skip that when the branch *consumed* the value. Place
  facts need the same exception for invalidation (an assignment or a
  mutating call inside the branch): the dual rule is "only put the old
  fact back if the fact you applied is still standing".
- **A feature can be blocked by syntax that was never written.** P1a's
  decision to narrow constant tuple indices could not be implemented at
  first: Salvo had no tuple element access at all (`.` required an
  identifier, and `t[0]` on a tuple typed as `Unknown`). Worth checking
  that the construct a rule talks about is actually *expressible* before
  designing the rule around it — the gap here was one session wide, but
  it was invisible from the rule's wording.

- Trivia the lexer discards is expensive to get back. Comments were
  dropped outright, and the cheap-looking recovery — re-scan the text
  above a declaration in the LSP — would misread a `//` inside a string
  literal. Collecting them at the one place that already knows what a
  comment is (`LexResult::comments`) cost less than the workaround and is
  correct by construction. The trick that kept it small: comments stay
  *out* of the token stream, so no parse function had to learn to skip
  them; the parser matches them to declarations by line number.
- Look for the fact you need before adding a table. The
  narrowed-vs-declared hover line wanted "the declared type of a narrowed
  identifier use" — which `Checked::repr_ty` had been recording all along
  for the emitters' re-wrapping. A second table would have been a second
  thing to keep in sync.
- Declaration *names* are not expressions, and tooling does not care.
  Hover, which reads `expr_ty`, said nothing on the `items` in
  `fn f(items: List<Int>)` or on the `x` in `let x = …`. Recording a type
  at binding-name spans (in `declare_var`, the single funnel) fixed
  parameters, `let`, `is` and `for` bindings at once — with
  `entry().or_insert` so a more specific entry from an earlier pass wins.

- A misleading diagnostic is usually a *missing* one further up. `if n is
  Ok` reporting "this check can never succeed" was correct on its own
  terms — no arm matched — but the arm never matched because `Ok` was
  undeclared and nothing said so. When a diagnostic is technically true
  and practically useless, look for the check that should have fired
  first, rather than softening the message.
- Two namespaces sharing one syntax need explicit resolution rules.
  `Ok Int` is a qualifier and a base type side by side, and the checker
  had two independent guesses about which was which: `lower_quals` took
  any name as a qualifier, `parse_check` asked `is_qualifier` and fell
  back to *base type*. Same source token, opposite classification, no
  error either way. Any position where a name's meaning depends on which
  table it is in should reject "in neither".
- Cascade suppression needs a flag, not a heuristic. Once an `is` check
  names something unresolved, four downstream verdicts become garbage
  (can-never-succeed, no-remaining-arm, non-exhaustive, and the
  `Nothing`-narrowing that poisons the subject). Carrying an `unresolved`
  bit on the parsed pattern kills all four at once; trying to recognize
  the situation at each site would have missed some.
- Where a *lenient* checker draws the line matters more than how lenient
  it is. [type-unknown-lenient] read as "unknown names pass through",
  which is what let undeclared qualifiers survive; the useful reading is
  "types I cannot *infer* pass through". Interop needs the second, not
  the first — you reach a foreign type by declaring it.
- Validation that runs only at "declaration sites" needs its site list
  audited when it grows. `[qual-of]` documented struct fields as a
  declaration site; `validate_type` was never called on them, and the
  same held for type-alias targets, effect-member signatures, and handler
  state. Nothing failed, because the only check there was
  applicability — which silently skips undeclared qualifiers.
- std is checked by the same rules as user code, so an aspirational
  signature there rots quietly: `char_at(index: Positive Int)` had been
  copied out of a LANGUAGE.md example with no `Positive` qualifier
  anywhere, and no call could have satisfied it if there had been one.
- Where a std declaration *lives* is a code-size decision, not just
  taste. `ok`/`err` in `core.basic` would have emitted a dead
  `core/basic.{kt,rs}` into every program, because `core.basic` declares
  `Int` and [mod-used-only] reachability is name-based: every file
  reaches it. Put anything that *emits code* in a module whose names only
  its users mention.

- (sweep) Four "known leftovers" were already fixed by earlier work and
  only *looked* open because nothing tested them (field-subject `is`
  lowering, union coercion inside arrays/tuples/lambda returns, and both
  halves of the unchecked-mangling item). Probe before implementing: the
  cheapest step in closing a leftover is a program that demonstrates it.
  Two of those probes instead found *new* bugs (a stray `eprintln!`
  debug print left in `check.rs`, and Rust rendering fn-type `let`
  annotations as `impl FnMut(...)` — invalid Rust).
- (sweep) Emitter *state* beats threaded parameters for context that must
  survive nested lowerings: `StmtCtx` was passed down through
  `emit_stmt`/`emit_block_stmts`, so every value-position path
  (`emit_value_block`, loop lowering, `when` expressions) silently reset
  it to `Normal` and iterator-body `return`s stopped retargeting. As a
  field with explicit save/restore at the two real boundaries (fn bodies,
  lambda bodies) the default becomes "inherit", which is what the
  language means.
- (sweep) Kotlin and Rust do *not* share an operator precedence table:
  Kotlin gives comparison a tighter level than equality, Rust puts them
  on one non-associative level. A ported precedence table must be
  re-derived per backend, or `(a == b) < c` re-renders flat and
  re-associates.
- (sweep) Emitter dispatch in unchecked contexts must be *arity plus
  types*: `resolve_define_fn`'s arity-only fallback silently picked the
  first same-arity template. The fix pattern is
  `<candidates> → filter by checked argument base names → exactly one or
  error` ([backend-never-wrong]); `ty_base_name` (checker types) and
  `type_base_name` (AST types) must agree on conventions (`T?` compares
  as its value arm, arrays as `[]`).
- (sweep) `HashMap` iteration order leaked into user-visible behavior:
  implicit `core.*` modules were added to each scope in hash order, which
  ordered overload candidates. Anything that feeds resolution or
  diagnostics must iterate deterministically (`core_modules` is sorted
  now).
- (sweep) Generic bindings in `unify` must *widen*: first-binding-wins
  made `pick(1, maybe_int)` bind `T = Int` and hide the optional. There
  is deliberately no occurs check — `Ty::Var` identity is name-scoped per
  side, so a callee's `T` legitimately binds to `List<caller-T>`; a
  name-based occurs check would reject generic forwarding.
- (sweep) Handler state fields are declared bare (`i: Int = 0`) — there
  is no `state` keyword, despite the prose in some notes.
- (D1) A test can *encode* the bug. `undeclared_qualifiers_pass_through_
  calls` asserted the exact behavior that made the checker unsound, with a
  comment explaining why it was right. When a soundness fix makes a test
  fail, read the test as evidence about the old model before assuming the
  new code is wrong — and rewrite it to state the new rule rather than
  deleting it (the delta form now carries the old behavior as an opt-in).
- (D1) The unsoundness survived because the *inference* direction hid it:
  facts only shrink, so "preserve everything not mentioned" looked like a
  safe optimistic start. Optimism is only safe if the pessimistic end of
  the lattice is reachable for every fn that needs it — here mutating fns
  could never reach it, because no constraint knew about qualifiers the
  signature never named.
- (decl-explicit) The `add` bug is the cautionary tale for
  inference-from-nothing: a bodyless fn has no body to constrain
  inference, so the "optimistic start" of the deduction fixpoint *was* the
  answer — keep everything, including an element the list had taken
  ownership of. It survived because the two backends disagreed quietly:
  Kotlin aliased and printed, rustc rejected with E0382. When a rule's
  inputs are absent, ask what the optimistic default *claims*, not whether
  the algorithm terminates.
- (decl-explicit) Trust boundaries should be one-directional. Once
  externals must declare their contracts, the checker must *stop*
  second-guessing them — the `Mut`-parameter proxy added hours earlier had
  to come back out, because inferring mutation from a declaration and
  overriding the author is the opposite of "the declaration is the
  contract". Catching wrong declarations belongs in a separate, opt-out
  validation pass (roadmap E2), not in the semantics.
- (decl-explicit) Requiring declarations exposed a second silent hole for
  free: effect-member deductions were *parsed* and ignored, so a member
  taking ownership never consumed its argument. The fix was to extract the
  flow half of the fn-value contract loop (`apply_call_contract`) and
  reuse it — when two call paths are documented as "keep the two in
  sync", that is a sign they should share code instead.
- (D1) Deriving a capability from a *declaration* rather than a body is
  the only option for `external`s, and it is often the right call anyway:
  `Mut` on a bodyless parameter means "I may mutate this", which is
  exactly the fact the soundness rule needs. Applying the same proxy to
  bodied fns would have been strictly worse (a non-`Mut` parameter can
  still have `Mut` *contents* mutated through it — the `h.tags` case the
  fate analysis already tracks).
- (std) Analyzing `--src std` double-loads the standard library (the CLI
  always loads its embedded copy) and trips every [mod-collision] check.
  Expected artifact of pointing the tool at its own std — use an ordinary
  project directory to see std diagnostics.
- (arrays) The CLI embeds std with `include_dir` at *build* time: adding a
  file under `std/` does not invalidate the crate, so the new module is
  silently invisible to `cargo run -- compile` (the tell is the file count
  in "parsed N file(s)"). Touch a `salvo-cli` source file to force the
  rebuild. The backend test crates read `std/` from disk and see new
  files immediately, which makes the discrepancy easy to misread.
- (optionals) A new strictness rule is also a *documentation* audit: the
  interpolation ban immediately flagged three LANGUAGE.md examples that
  read `${person.surname}` after `person.surname is Str` — the spec was
  quietly assuming Kotlin-style field smart-casts. Fix the examples to
  the binding form (`is Str surname`), which the spec already used
  elsewhere. Also worth reading twice: when two backend demo *sources*
  differ (here `${capped}` vs `${capped!}`), that divergence is usually
  evidence of a missing checker rule, not a backend quirk.

- (L7d) Expected types only reach lambda arguments when the arg-typing
  pass supplies them: named calls typed all arguments with `None`
  before overload matching, so fn-type contracts silently never
  arrived at `check_lambda`. Single-candidate callees now pre-lower
  their parameter types as expecteds for lambda literals; overloaded
  callees still probe untyped (an expected could bias resolution) —
  contracted lambdas passed to *overloaded* fns are a known gap.
- (L7d) The named-fn-by-value pass was broken on BOTH backends in
  different ways (Rust: signature-mode mismatch; Kotlin: a bare
  identifier where `::name` is required) — when a feature is
  "loud-but-unsupported", verify each backend's failure mode
  separately; they rarely match.
- (L7c) A borrow crossing a fn boundary interacts with *every* later
  relaxation: S2's move-mode would happily have "taken ownership" of a
  physically borrowed call result (moving out of a `&` — rustc
  rejects, checker accepted). Flow facts that change physical
  representation must be carried on the links themselves
  (`FateLink.borrowed`) so every downstream rule can refuse; when
  adding a new emission regime, probe its interaction with move-mode
  before shipping.
- (L7b) The Ident-callable path in `check_call` never `check_expr`s the
  callee identifier, so consumed-value reads did not error there —
  calling an already-consumed callable was silently accepted until the
  path got an explicit `Ty::Nothing` branch. When adding a consumption
  rule, audit every place an identifier is *used* without going
  through the standard read path.
- (L7b) `Once` is the first qualifier with *inverted* subtyping (a
  restriction, not a refinement): plain fn <: `Once` fn and `Once` may
  never drop. Both `is_subtype` and `unify` carry special cases —
  flagged for review; if a second restricting qualifier ever appears,
  generalize direction into the qualifier model instead of a third
  special case.
- (L7a) Opting a variadic constructor into linearity is a trap:
  variadic positions are untracked by the flow analysis, so a linear
  argument would be physically moved while statically still owed —
  contradictory requirements. The variadic guard refuses linear values
  there outright; collection construction with linear elements is
  empty-then-`add`. When opting in an external, audit *every* position,
  not just the contract shape.
- (L6) A Salvo-bodied consuming fn must end the obligation chain
  itself: `fn close(h: FileHandle) -> [] None {}` leaks `h` by its own
  rules — the body owns the moved-in value and must `discard` it (real
  release lives in external fns with no body to check). Tests and
  examples that stub consumers with empty bodies will all fail the
  frame-drop check; stub with `discard`.
- (L6) The obligation checks mark reported variables as consumed so
  each obligation errors exactly once (a `return`-site error would
  otherwise repeat at the frame pop). Any new exit-shaped check should
  follow the same report-then-consume pattern.
- (S3) Decide the borrow path *before* emitting the value: `emit_let`
  computed `value_code` eagerly, and the discarded emission would have
  recorded coercions/union sizes for code that never lands. Emission
  helpers are not side-effect free — order decisions first, render
  second.
- (S3) By-reference loops hand `&T` to every lowering that touches the
  loop variable: `matches!`/unwrap lowering for union and optional
  elements expects owned subjects and breaks on references — hence the
  concrete-non-union element guard. If later refinements widen the
  guard, extend the union-test rendering to ref patterns first.
- (S3) Borrowck alignment is a consequence of poison, not a separate
  analysis: a checker-legal program never uses a derived value after
  its root's mutation/move, so NLL sees every borrow die before the
  conflict. The one mismatch is *within a single call* (pass a
  borrowed local and move its root in one argument list — rustc E0505,
  checker-legal): loud, documented, rare.
- (L4) "Lambda bodies are a barrier" was only ever true of
  `loop_stack`: bodies are checked *inline*, so flow events (consuming
  calls, mutations) always fired against outer variables — captures
  were never fully untracked, they were tracked with the wrong
  multiplicity (once, at creation). The L4 model rides those existing
  events (a boundary stack + guards at the consuming sites) instead of
  adding a separate capture walk; when auditing "untracked" claims,
  check what the inline checking already does.
- (L3) Sibling arguments of one call are the *only* place where a read
  can see a value after its move without the standard consumed-read
  error firing: argument typing runs before contract enforcement, and
  nested calls consume during typing. Hence the dedicated
  mention-scan (`expr_mentions`) rather than a flow-state fix.
- (L3a) The fixpoint converges because move-mode candidates and claims
  only grow; inferred deductions are the one non-monotone axis
  (overload resolution can flip with narrowing), which is why the
  round cap exists. The instability error path is untested — nobody
  has constructed a genuine oscillator yet; if you find one, turn it
  into a test.
- (S2) Round one must see *every* fate event, or mode inference goes
  blind: kept-`Mut` mutation events only fired when a call contract
  existed, and round one had no inferred facts — so mutation-driven
  move-mode candidates were recorded one round too late. The fix:
  round one falls back to the *optimistic* contract
  (`deduce::optimistic`) for unwritten callees — it enforces no moves
  and strips nothing, so it is behavior-neutral except for surfacing
  the `Mut` mutation events.
- (S2) Flattened fate links now keep their *original* bind spans
  (only the direct source link carries the current event's span). This
  makes a variable's links describe its whole derivation chain, which
  move-mode candidate recording needs — intermediate variables of a
  chain (loop bindings especially) are often dead by the time the move
  is seen. If links are ever restamped again, zero-clone chains break
  silently (one clone reappears per dead intermediate).
- (S2) A `for`-loop binding must be re-declared *fresh on every
  checking pass* of the body (it binds a new element each iteration).
  Declaring it outside `check_loop_body` let its consumed state leak
  into the back-edge re-check — a live false positive
  (`for s in xs { consume(s) }` errored) that predated S2 and only
  surfaced when move-mode made consuming loop bindings routine. Loop
  bindings go through the per-pass `bindings` channel
  (`pattern_bindings`); `is`-bindings always did.
- (L2) An always-exiting branch contributes nothing to the merge after
  the construct — which is correct *inside* a loop body but silently
  drops `break`-path consumption for the code *after the loop*. The
  loop exit is a join point of its own: `LoopCtx` captures a state
  snapshot at every `break` and the loop merges them with the
  fall-through exit state. When adding a new control-flow event, ask
  *where its state lands*, not just whether the branch exits.
- (L2) When merging snapshots with different frame depths
  (`merge_fallthrough`), put the shallowest (current) snapshot first —
  it drives the key iteration, and deeper break-time frames align as a
  prefix. For `for` loops, merge only after the loop-binding frame is
  popped.
- (L2) Kotlin's `.copy()` for struct spread is *shallow* while Rust's
  `..base.clone()` is *deep* — a mutable-data parity divergence that
  had been sitting unobserved in [struct-spread] since M8. Consuming
  the spread base closed it by restriction. Same audit lens as the S1
  lesson: for every emission difference, ask "could the two backends
  disagree observably?" — the remaining known case is projection values
  in moved positions (documented in [fate-poison]).
- (S1) `[struct-mut]` ("only `Mut Name` values may have fields
  assigned") was in both specs but *unenforced* — and became
  load-bearing: Kotlin's identity lowering of `copy` is only correct if
  non-`Mut` values really are immutable. When a backend decision leans
  on a checker rule, verify the rule is actually enforced, not just
  written down. (Enforcing it also exposed a LANGUAGE.md example bug:
  the `Mut` struct example mutated `person` instead of
  `mutable_person`.)
- (S1) Poison rides the existing consumed-state machinery: a poisoned
  variable is just `narrowed = Nothing` plus a `Poison` reason on the
  `LocalVar` — branch merging, revival-by-reassignment, `is`-restore
  survival, and the loop re-check all carry it with no new lattice.
  Keep new flow facts inside `VarState` (narrowed/links/poison) so
  `snapshot_narrows`/`restore_narrows`/`merge_fallthrough` stay the
  single source of flow-state truth.
- (S1) Variable *names* are not stable identities across sibling scopes
  even with [var-no-shadow] (two sequential `for person in …` loops);
  fate links use per-binding numeric ids (`Checker.next_var_id`), and a
  link may outlive its root (the loop binding dies, the flattened link
  to the collection survives) — flatten to all transitive roots at the
  binding, not lazily.
- (S1) `is`/`when`/`for` bindings alias their subject, so they must
  inherit its fate links — the binding tuples are `(Ident, Ty,
  Vec<FateLink>)` threaded through `CondInfo`/`IsInfo`/
  `check_branch_block`. A new binding form must decide its provenance.
- (S1) The checker now keeps *both* sides of `let a = b` readable, so
  the Rust emitter had to stop moving bare-ident sources
  (`emit_linked_value`: owned non-Copy locals clone in `let`/assignment
  values and `for` iterables). Checker-permissiveness changes and
  emission must move together, or the gap surfaces as rustc errors on
  generated code.
- (S1) `i++` and whole-variable reassignment are *rebinds* (revival:
  sever the variable's own links, poison its previous derivatives), not
  mutations — only projection assignment and `Mut`-kept call arguments
  are mutation events. Getting this wrong makes ordinary loop counters
  (`let i = start; while … { i++ }`) uncompilable.
- (S1) Kotlin's `copy` immutability analysis must be *transitive* (a
  non-`Mut` struct with a `Mut List` field is not immutable — identity
  would alias the mutable part and diverge from Rust's deep clone), and
  generic struct fields must be checked under the instantiation's
  substitution (`approx_ty` bridges written field types to checker
  `Ty`s just far enough for that).
- (S1) Leniency gaps in the fate analysis are not always mere
  strictness gaps — some are *backend-parity* holes. Untracked
  mutation events let a Kotlin alias observe a change a Rust clone
  does not (`let t = h.tags; add(h.tags, 2); size(t)` prints 2 vs 1).
  When auditing an unhandled event, always ask "could the two backends
  disagree?", not just "should this be an error?" — the parity
  principle in the roadmap is the checklist.
- `unify` (overload resolution) is *order-sensitive across its match
  arms*: the argument-qualifier-stripping arm
  (`(_, Ty::Qualified { .. })`) must come *after* the union-parameter
  arm, or a qualified argument (`Ok Str`) loses its qualifier before the
  union's qualified arms are tried and `describe(ok("x"))` against
  `Ok Str | Err Str` never resolves. Found passing a union-arm value in
  argument position; `is_subtype` had the arm→union rule all along, but
  candidates were rejected by `unify` before the subtype check ran.
- `SourceSet::classify` with a backend name that matches no define suffix
  (the CLI passes `""` for backend-neutral `analyze`) loads language
  files only — every `*.<something>.sv` is treated as another backend's
  define file and skipped. Cheap way to get a language-only load; don't
  name a real backend the empty string.
- CLI integration tests use `env!("CARGO_BIN_EXE_salvo")` +
  `env!("CARGO_TARGET_TMPDIR")` (both provided by Cargo for integration
  tests of a crate with a binary) — no `assert_cmd`/`tempfile`
  dependencies needed.
- (lsp) `lsp_server::Connection` must be *dropped before*
  `io_threads.join()`: the writer thread only exits when the
  connection's channel sender is dropped — joining first deadlocks the
  server on shutdown (symptom: clean shutdown/exit exchange, then the
  process never terminates).
- (lsp) `Path::canonicalize` fails for files that don't exist on disk
  (unsaved editor buffers): canonicalize the *parent* and re-append the
  file name, or symlinked roots (macOS `/tmp` -> `/private/tmp`) make
  overlay paths miss the workspace root and the buffer silently drops
  out of the analysis.
- (lsp) Publish diagnostics against the URI the client opened the
  document under, not one rebuilt from the canonicalized path — clients
  match URIs textually, and a `/var` vs `/private/var` rewrite makes
  them ignore the publish.
- Kotlin smart casts make some emitted `as` casts redundant (kotlinc warns
  "no cast needed") — harmless. The `(x.value as T)` unwrap casts are
  *required* though: narrowing may come from `elif` exclusion where Kotlin
  has no smart cast, and `value` is typed `Any?` on the sealed interface.
- `is UN_i` checks need star projections (`is U2_1<*, *>`) — kotlinc
  rejects bare generic classes in `is`. Sealed exhaustiveness still works.
- A subject-less Kotlin `when` used as an expression demands an `else`
  branch — the emitter converts the last branch of a `T?`-subject `when`
  to `else` (sound because the checker proved exhaustiveness).
- The checker and emitter must agree on the ident-unwrap rule
  (`repr is wrapper && logical is a single non-None arm`); `maybe_coerce`
  computes the "effective repr" with the same predicate the emitter uses.
- `define` templates that call something with the same name as the effect
  member they implement must qualify it (hence `kotlin.io.print`).
- Overload resolution now happens in the checker; `size(Str)` vs
  `size(List<T>)` style collisions are resolved by argument type. The
  emitter still falls back to arity in unchecked contexts.
- insta snapshot tests fail on first run by design; accept with
  `INSTA_UPDATE=always`.
- The `Number` type alias in std is a general union — fine: aliases expand
  on use (now with generic substitution in both checker and emitter), and
  nothing uses `Number` yet.
- Subtype-rule *order* matters for qualified union groups: the
  `(_, Union)` any-arm rule would otherwise compare `Ok (A | B)` against
  single arms and always fail; the group rule (exact-arm equality, then
  drop-quals) must come before it. Symmetrically, `maybe_coerce` must try
  wrapping the group as a whole arm *before* stripping its qualifiers.
- `substitute_vars` must re-normalize `Ty::Qualified` through `qualify()`:
  a generic constructor's `Ok T` with `T = Ok Str` would otherwise nest
  `Qualified` inside `Qualified` (breaking the type invariant).
- Qualifier validation happens at *declaration sites* (`validate_type` in
  check_fn / let / struct fields), not inside `lower_type` — lowering runs
  repeatedly (e.g. per overload candidate), which would duplicate errors.
- Overload mangling compares *emitted* Kotlin parameter strings, so the
  `Mut List` → `MutableList` mapping naturally avoids false collisions.
- Predicate `is` on a subject already narrowed out of a wrapper union works
  because `emit_expr_base` (the unwrap) is used for the `qualifies` call
  argument.
- All handlers — external ones included — emit as Kotlin *classes* and are
  instantiated at their `use` site (user decision: `object` was an artifact
  of StdOutConsole being stateless). Bare `use Handler` is sugar for
  `use Handler()`; external handlers support constructor params like any
  other handler.
- `kotlin_ty(Ty)` (checker type → Kotlin) must agree with `emit_type`
  (AST type → Kotlin) on the same source type: checker-resolved effect
  types are looked up in an effect environment keyed by `emit_type`
  renderings. Both expand type aliases and map internal names through
  `emit_named_parts`, which is what keeps them aligned. If they drift, the
  lookup degrades to base-name fallback matching (or a codegen error) —
  never silently wrong dispatch, but worth knowing when adding type forms.
- The checker types effect-member call *args* only when it must
  disambiguate between multiple instances (multi-candidate path types them
  with `None` expected, then records coercions afterwards); the
  single-candidate path checks args directly against the substituted param
  types, preserving expected-type-driven inference for lambda/struct-lit
  arguments. Keep the two paths in sync when touching `check_effect_call`.
- `Stmt::Use` no longer runs `check_expr` on the whole handler expression
  (ctor args are typed individually in `check_use`), so the handler `Call`
  expr itself has no `expr_ty` entry — the emitter doesn't need one, but
  don't add a table lookup keyed on it.
- Same-name defines for overloaded externals (`size(Str)` vs
  `size(List<T>)`) are disambiguated in `define_for_decl` by comparing
  parameter *base type names* against the checker-resolved declaration —
  the arity-only fallback (`resolve_define_fn`) can still pick the wrong
  one in unchecked contexts.
- The `use` duplicate-registration check compares checker `Ty`s, so it
  catches `Random<Int>` twice while allowing `Random<Int>` +
  `Random<Str>`; the effect-list duplicate check is separate
  (`check_effect_list`) because declared effects never pass through
  `check_use`.
- Kotlin loops are *statements*: any block whose value is a trailing
  `while`/`for` (if-expr branches, `when` branches) must route the loop
  through `emit_loop_value` — a plain emission would silently value the
  block as `Unit`. `emit_value_block` special-cases the trailing loop.
- The loop result local must be a *nullable* temp (`var __loopN: T? =
  null`) even for non-optional joins: Kotlin cannot prove definite
  assignment through a loop, so the block ends `__loopN!!` instead. Only
  loops with `else` produce non-optional joins, and then every path
  assigns — except the spec-silent corner where every iteration
  `continue`s before the tail; the checker closes it by joining `None`
  into the type whenever a bare `break`/`continue` exists.
- `break`/`continue` attribution needs matching stacks on both sides
  (checker `loop_stack`, emitter `loop_results`), and both must treat
  lambda bodies as barriers and *pop before checking/emitting the `else`
  block* (a `break` in `else` belongs to the outer loop).
- Deduction inference must interpret a callee's deduction list relative
  to the callee's *declared* parameter qualifiers (removal set =
  declared − kept, subtracted from the argument), not as the absolute
  set of remaining qualifiers — otherwise qualifiers the callee never
  declared would be wrongly stripped from the caller's argument.
- Deduction fixpoint direction matters: start optimistic (all kept, all
  quals) and only remove facts — starting pessimistic would not converge
  to the least-strict sound answer and recursive fns would infer
  everything as moved.
- Kotlin wildcard imports of *nonexistent* packages are compile errors:
  generated `import salvo.<mod>.*` lines must be filtered to modules that
  are actually emitted (reachable *and* produce code) — hence
  `emitted_modules` is computed before any file is emitted.
- A companion file with the same stem as a code-producing module would
  silently overwrite the generated `.kt` at write time (both map to the
  same path); `emit_program` errors on the collision instead. The
  LANGUAGE.md pattern keeps companion modules externals-only.
- Aliased Salvo imports need Kotlin alias imports only for items that
  exist as Kotlin symbols; alias-importing an *inlined* external (define
  template) would reference a nonexistent symbol and fail kotlinc.
  `emit_fn_call` must emit the source (alias) name when it differs from
  the declaration name, or the alias import is dead and the call
  ambiguous.
- `unions.kt` is now driven by the emitters' tracked sizes only (the
  checker's `union_sizes` may include wraps in unreachable modules);
  every emission path that renders a wrapper inserts its size
  (`emit_ty`, `emit_union_type`, `apply_coercion`, union tests) — keep
  that invariant when adding forms.
- The `--emit-ast` flag takes an optional `=MODULE` value
  (`num_args 0..=1` + `require_equals` in clap); a bare flag must not
  swallow the next positional argument.
- (M8) `matches!(subj, Pat)` and `match subj` on a place do not move it
  when the patterns bind nothing - union tests and `when` lowering rely
  on this to test borrowed subjects without clones.
- (M8) Rust implicitly reborrows `&mut` *place* expressions at call
  sites, and coerces `&mut T` to `&T` - which is why parameter bindings
  thread through calls as bare names while locals need explicit
  `&`/`&mut`. Unsized coercion turns `&mut ConcreteHandler` into
  `&mut dyn Effect` at the argument position for free.
- (M8) The Rust ident-unwrap rule is a superset of Kotlin's: it also
  unwraps `T?`-repr idents narrowed to their value arm (Kotlin smart
  casts those). `maybe_coerce`'s "effective repr" was extended to match
  both - if you touch one of the three places, touch all three.
- (M8) `WrapOption` must be recorded even though Kotlin ignores it:
  the checker cannot know which backend will consume the tables.
  Backends must treat unknown-to-them coercions as no-ops only when the
  representation really is transparent (Kotlin nullability), never by
  default.
- (M8) Everything the Rust emitter renders in a *moved* position must be
  owned; the easy mistake is a field read (`person.name`) - moving out
  of a borrow is illegal, hence the clone-by-default owned rendering.
  The place/owned/raw rendering split (`emit_place`/`emit_owned`/
  `emit_raw`) exists to keep assign targets and `matches!` subjects
  clone-free.
- (M8) Define templates receive raw *places* for non-variadic args so
  method-style templates (`${list}.push(..)`) borrow natively; variadic
  parts splice owned because they land inside constructors
  (`vec![${...elems}]`). Getting this backwards either double-clones or
  moves out of borrows.
- (M8) rustc needs `match` exhaustiveness over the *representation*:
  a `when` over a narrowed subject covers only the logical arms, so the
  emitter appends `_ => unreachable!()` when repr arms are left over
  (the checker proved them impossible).
- (M8) Enum-variant wraps need turbofish (`Union2::<A, B>::U1(x)`): the
  other type parameters are not inferable from one arm's payload.
- (M8) Eager iterators change side-effect *timing* vs Kotlin's lazy
  `Iterable` (documented divergence [rs-iter-vec]); values are equal for
  finite iterators, and infinite ones would hang - revisit if generators
  stabilize.
- (M8) The crate root must carry `#![allow(...)]` *before* any item, and
  `#[path]` mounts resolve relative to the file containing them - the
  root-file header is prepended after all files are emitted, when the
  full mount list (unions, companions) is known.
- (rename) A keyword rename touches more than the lexer: `KEYWORDS`
  feeds `salvo lang tm-grammar`, and a test compares the generated
  grammar against the checked-in `vscode/syntaxes/salvo.tmLanguage.json`
  byte for byte — add the word to a category list in `cli/src/lang.rs`
  (`keywords_are_fully_categorized` asserts the partition is exact) and
  regenerate the file in the same change. A longer keyword also shifts
  every span in the parser snapshots, so the insta diffs are large but
  should contain *only* span deltas and the renamed field.
- (rename) Sweeping a keyword with `sed` needs case sensitivity and a
  pass over the hits first: `with Linear` (syntax) and `with linear
  type` (an error message's prose) differ only in case, and one site
  that looked like the same clause — `qualifier NonEmpty<T> of List<T>
  with Mut<T>` in `experiments/` — was the *other* meaning of `with`
  ([qual-with]) and had to stay. Grep with context, exclude the
  exceptions explicitly, then re-grep for leftovers.
- (N1) **Rust puts modules and structs in one type namespace.** The
  attractive idea of emitting a dot-name as a real nested module
  (`pub mod Environment { pub struct Id }`) dies on E0428 the moment the
  namespace struct exists — which the rule requires. Verified with rustc
  before writing any emitter code; a lowercase module (`environment::Id`)
  does compile but impersonates a Salvo module. Flattening plus a
  language-level collision ban was the answer.
- (N1) **One name representation beats two.** Carrying a dot-name as a
  single dotted string in `Ident.name` meant scope keys, checker types,
  `ty_base_name`/`type_base_name` and define-template environments needed
  *no* changes — the "touch one, touch all three" trap never fired.
  Translation happens where a Salvo name becomes target syntax: Kotlin
  renders it verbatim (nested access is spelled the same), Rust flattens
  in `rs_ident`, which is the single funnel for all 56 identifier
  renderings. The safety argument for that: a dot is invalid in a Rust
  identifier, so any dotted string reaching `rs_ident` can only be a
  dot-name.
- (N1) **A casing rule pays for itself.** Making types-uppercase /
  values-lowercase normative was needed for dotted *expressions*
  (`Environment.Id { … }` vs `person.name`), but it also turned an
  existing heuristic into a consequence: `parse_is_check` had been
  guessing that a lowercase word after `is` was a binding. Rules that
  retire guesses are cheaper than they look — and no existing source
  violated it, so the sweep cost nothing.
- (N1) Import paths split *positionally* (last segment = item), so
  two-segment item names needed the split to become casing-aware:
  trailing uppercase segments are the item, and a lowercase last segment
  stays a value (`import core.list.size`). The synthesized dotted key is
  short-lived, so `ModuleItems::name_ref` exchanges it for the
  declaration's own `&'p str` before it enters the scope maps.
