# Salvo Compiler — Progress & Plan

**Overload resolution finalized 2026-09-07 (user decisions), and it is now
one rule with two escape hatches.** The agenda was drawn up from probing the
old implementation — twelve small programs, most of which answered in ways
nobody had chosen — and the user settled every item. The governing principle
was stated first and decides most of the details: *the rule for which
function is selected should be simple, we should refuse to guess but rather
raise an error, and we should give the user options for addressing the
error.*

**The rule.** A call is decided in three steps [fn-overload]:

1. `f@module(args)` names the module whose overload is meant, if written
   [fn-overload-at].
2. **The most specific scope that fits wins** [fn-overload-scope]: `core`,
   then this file's imports, then this module, then the fn's own scope
   (fn-typed locals, parameters, implicits, effect members), then inner
   scopes. Only the most specific rung with a candidate *fitting the
   arguments* competes.
3. **Then the most specific signature** [fn-overload-rank], per argument
   slot: a type variable says least; fewer union arms says more (`Int` >
   `Int | Str` > `Int | Str | Bool`, and `Any` is the broadest type there
   is); more qualifiers says more, with the *kind* never ranking; and a
   fixed parameter list beats a variadic one. No single winner is an error
   [fn-overload-ambiguous].

**What that fixed.** The open defect from yesterday — an own-module fn losing
to an identically shaped std one, *silently* — is closed by step 2: a
program's own `size(List<T>)` now means its own, while `size("text")` still
reaches core's, because the shadowing overload does not fit. Scope beats
signature deliberately (the alternative needs the reader to know std's whole
surface), and where it discards a more specific signature the call gets a
**warning** naming both candidates and both `@module` forms.

**The two escape hatches, both erased before emission:**

- **`f@module(...)`** — a module *path*, not a rung keyword (`@mod`/`@import`
  would ask the reader to know which rung a name came in on). It works in dot
  form (`xs.size@core.list()`), as a value (`describe@main`), and it is the
  only way to reach a function a **local of the same name** shadows.
- **`rename fn label_small = label(n: Small Int)`** — a scope-local name for
  one overload, which from that point answers *only* to it. Not an alias:
  that is what makes the old name unambiguous again, and it is the remedy the
  ambiguity diagnostic names. Module-scoped (whole module, not importable) or
  block-scoped from its line, matched by the same overload matcher `refn`
  uses [qual-refn-match], and it may not mention effects, deductions or a
  return type — none of them takes part in selection [fn-rename].

**Three defects came out with it**, all found by probing rather than by
report: an own-module fn losing to std's (above); **two identical parameter
lists** being declarable, with the second silently unreachable — now a
declaration error [fn-overload-duplicate]; and **effect member calls not
checking their arguments at all** (`log(true)` against
`fn log(message: Str)`) — now checked like any other call
[effect-member-call]. A fourth was already recorded and is fixed here too: an
overloaded function *passed by name* resolved to whichever overload was
declared first; it is now selected by the expected fn type
[fn-value-select].

**Two things the implementation forced:**

- **`Any` had to become the top type it always claimed to be.** Written as
  the `intrinsic type Any`, it lowered to a *nominal* `Named("Any")`, so
  `f(v: Any)` accepted nothing at all — unification compares names. The
  ranking rule "`Any` is the broadest thing a parameter can say" would have
  been theory. `Any` now lowers to `Ty::Any`, like `Nothing` already lowered
  to `Ty::Nothing` [type-any-nothing].
- **Rust needed a path for a shadowed call** [rs-shadowed-call]: functions
  and locals share one value namespace there, so `describe(7)` beside
  `let describe = "…"` is E0618 — the emitted call is
  `crate::describe(7)`. Kotlin needs nothing, since functions and properties
  are separate namespaces. This is the first mechanism whose *only* reason to
  exist is that `@` made a shadowed function reachable.

Deferred, and recorded rather than dropped: **`@Effect` for member
disambiguation** (`println@Console(...)`). Two effects declaring one member
name is currently a declaration error whose message already promises this
syntax; lifting it means a multimap plus every member path
(`check_effect_call`, deduction inference, refinements, LSP), so it is a
milestone of its own rather than an easy win.

**S-Str landed 2026-09-06 (user design): `Str canbe Mut`, and the string
function surface.** A string is still immutable; what `Mut Str` adds is a
string *under construction*, asked for explicitly (`mutable_str("he",
"llo")` — a literal is never a builder) and mutated by exactly three
functions (`append`, `set`, `clear`). Everything else in `core.string` takes
a plain `Str`, and a builder reaches all of it by dropping its `Mut`.

**That drop is the feature.** Every other qualifier erases, so widening only
forgets a claim — and `Mut List<T> <: List<T>` has always been free because
`MutableList<T>` *is* a Kotlin `List<T>`. A `StringBuilder` is not a
`String`, so this is the first `Mut` whose removal costs an instruction.
Rule [str-drop-mut]:

- **The checker records the drop** (`Coercion::DropMut`), rather than each
  emitter guessing from types — the standing checker/emitter agreement. The
  record carries the qualified type, so a backend decides from its base:
  Kotlin renders `.toString()` for `Str` and nothing for `List`, Rust
  nothing at all (there `Mut` erases, and `coercion_of` unwraps a `DropMut`
  so even the "is this argument a fresh temporary?" tests see that nothing
  happens).
- **It fires at every drop site**, which is the part worth testing rather
  than believing: call arguments (intrinsic lowerings included — a
  `StringBuilder` must not reach `String.getOrNull`), returns, `let`
  annotations, struct fields, union arms, interpolation, and operator
  operands. Operators are *coerced, not rejected*: Kotlin's `StringBuilder
  == String` is `false` and `sb1 == sb2` is reference equality, against
  Rust's structural `String == String` — a live parity divergence, and
  rejecting `Mut` there (the [op-no-none] treatment) would surprise, since
  `Mut List` is accepted everywhere else.
- **A drop carries the coercion it displaces.** One expression has one
  coercion slot, and a `Mut Str` flowing into a `Str | Int` needs both the
  conversion *and* the union wrap, so `DropMut` has a `then` field. Found by
  writing the union-arm test, not by design.
- **`copy` had to learn about it**: a `Mut Str`'s copy is
  `StringBuilder(sb)` on Kotlin [kt-copy]. Identity would alias the buffer —
  the S1 transitive-mutability trap, and `ty_immutable` already answered
  correctly (`Mut` anywhere means mutable), so this was one arm in the
  copy lowering rather than an analysis change.

**Two things fell out of implementation.** First, the surface is
*char-indexed* on both targets, which Rust needs conversions for (`find`
answers in bytes) — and Kotlin still counts UTF-16 code units, the
divergence `size`/`char_at` already had, now shared by `substr`/`index_of`/
`set`. Astral-plane text is where the two differ; a `Char`-exact `Str` is a
decision, not a patch. Second, **`set` needed a generated support module on
Rust** (`strings.rs`, mounted and imported like `iter.rs`): replacing a
character reads *and* writes the string, and an inline `let s: &mut String =
&mut place;` does not compile for a `&mut String` parameter (E0596 — the
binding is not `mut`). A trait method auto-refs an owned local, a `&mut`
parameter and a field projection alike, and mentions the receiver once, so a
call argument is never evaluated twice.

**And a pre-existing hazard surfaced: `...spread` into a variadic
intrinsic.** `mutable_str(...parts)` spliced the array as one argument,
which Kotlin's `StringBuilder.append(Any?)` cheerfully accepted — printing
`[Ljava.lang.String;@37bba400`, i.e. silently wrong output
[backend-never-wrong]. The same shape in `list(...arr)` was merely a
kotlinc/rustc error, which is why it had gone unnoticed. Fixed for all of
them [fn-variadic]: Kotlin uses its own spread operator (`listOf(*arr)`),
Rust takes the vector itself (cloned — Salvo does not track a variadic
position, so the array stays usable), and `mutable_str` *borrows* its parts,
since it reads them rather than storing them.

Verified end to end by compiling and running one 40-line program covering
the whole surface under both `kotlinc` and `rustc` with byte-identical
stdout, plus a second one for `set` through a parameter and a field.

**S-Seq landed the same day too — `map`/`filter`/`reduce` over anything with
an `iter`** — and it is the answer to "what did Salvo get instead of traits"
applied to collections: `params Iterable<It, T>` is a bundle of implicit
parameters, so a customer type becomes iterable by declaring one function.
The design was decided in advance and needed **four mechanisms that did not
exist** — implicit resolution feeding back into type inference
[implicit-infer], a *lead* candidate for expected types
[fn-overload-rank], intrinsics passed as adapter closures
[implicit-intrinsic], and parameter contravariance in implicit resolution —
plus generated Rust helpers, because Rust closure inference rules out every
inline shape. Details under "Roadmap: standard library surface".

**O1 (overload specificity) landed the same day**, and was superseded the
next: a concrete parameter type beating a type variable is now one rung of
the finalized ranking [fn-overload-rank] — see "Overload resolution" and the
entry at the top of this file. What O1 got right and the finalized rule kept:
comparing the *declared* patterns, the partial order with an error for "no
winner", and the leniency guard for un-inferred arguments.

**Qualifier refinements (`refn`) landed 2026-09-06 (user design), closing
roadmap D3.** D1 made deductions sound by forbidding a mutating function
from promising a qualifier it never declared, and accepted the
over-strictness that follows: `add` cannot promise `NonEmpty` back even
though appending to a list can never empty it. The insight that fixes it is
that **the function was never the party to ask** — it has never heard of
`NonEmpty`. The qualifier that owns the claim states it instead:

```
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool { return list.size() > 0 }

    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) -> [list: +NonEmpty]
}
```

The user's decisions (all 2026-09-06):

- **A qualifier may only refine its own claim**; a *top-level* `refn` may
  name any state qualifier in scope. This was the load-bearing choice: it
  collapses the conflict taxonomy to one case. The two failure modes the
  design memo listed — refinements removing each other, and a refinement
  declaring exhaustively — become unreachable, the second by grammar and
  the first because only `Q` can speak about `Q`. What remains is
  add/add: two qualifiers that cannot co-apply [qual-with].
- **A top-level `refn` is module-scoped and not importable.** Reconciling
  is the consumer's call; a library shipping its own reconciliation would
  move the conflict one level up.
- **Overload matching is by types *and* names** [qual-refn-match], with
  type parameters positional. Requiring names means a std rename surfaces
  as a diagnostic instead of a refinement that silently stops firing.
- **Conflicts are per (callee, parameter)**, not per call: a disagreement
  about one parameter must not cost the refinements of another.
- **A suppressed conflict warns.** No error — the program compiles and the
  function is simply less useful, exactly as designed — but silence would
  make an imported refinement's doing nothing undiagnosable, so a
  `Severity::Warning` lands at the call site, once per (callee, parameter).
- **Effect members are not refinable** (deferred): a member has no `FnKey`,
  and naming one needs an effect-qualified form.
- **Inference is included.** A refinement reaches the inferred contract, so
  the fact survives one frame outward.

**Two things fell out of implementation that the memo did not predict.**

First, *reconciliation only works if a top-level `refn` **replaces** the
qualifiers' refinements* for the parameters it names [qual-refn-reconcile].
Joining them would keep the disagreement — the user's stated remedy ("
redeclare the reconciled qualifier in their scope") is only a remedy under
replacement. It is the same precedence own-module declarations already have
over imported ones [mod-collision].

Second, *additions and the deduction fixpoint do not mix freely*. The walk
in `deduce.rs` is a meet over all uses, not a flow analysis: removals only
ever accumulate, which is what makes it order-insensitive and terminating.
An addition is the opposite direction, so `if c { add(list, x) }` would
have let a signature promise a fact that holds on one path. Two
restrictions keep it sound [qual-refn-infer], and both were worth having
for independent reasons:

- **Only at `cond_depth == 0`** — the addition must be unconditional in the
  body (a new depth counter bumped by `if`/`when`/loop/lambda/`defer`/`try`
  bodies). The *call site* stays flow-sensitive and still narrows inside
  the branch; it is only the exported *contract* that is conservative.
- **Only for a qualifier the parameter declares.** A refinement can cancel
  a removal, never invent a claim — inventing one is `+Q` in a function's
  own deduction list, which is D2 and stays deferred. This also bounds the
  lattice (the keep set stays a subset of the declared set), so the
  fixpoint still only shrinks and still terminates.

**`+Q` now has exactly one home, and D2 is untouched.** In a `refn` it is
the qualifier author's claim about someone else's call — trusted, like
`-> T as Q` [qual-ctor-fn]. In a `fn` it remains a parse error naming D2,
because there it would be a claim about your *own* body, which needs an
establishment rule.

**No backend work at all**, and that is a consequence of the "state
qualifiers only" rule rather than luck: `Mut` is the one qualifier that is
*not* erased, so admitting `+Mut` would have made this an emission feature.
Verified by compiling and running one refined program under both `kotlinc`
and `rustc` — `after add: 1` / `after refill: 2`, byte-identical — with the
emitted `main` asserted to contain no `qualifies` call, since a refinement
is trusted rather than checked.

**The warning had to be made non-fatal, and then given somewhere to go.**
Both backends' emission gates aborted on *any* diagnostic, so the first
conflicting program printed the warning and then refused to compile — the
exact opposite of "no compiler error, just a less useful function". The gate
is errors-only now; and since a warning nobody sees is the same as no
warning, `Backend::emit` gained a channel for it (user request 2026-09-06):
it returns `Emitted { files, warnings }`, and the CLI's one build path
prints the warnings unconditionally — not under `--verbose` — before
carrying on. Two things worth knowing about the shape:

- **The warning-dropping entry point stayed.** `emit_program` is what 108
  golden tests call, so it keeps its signature and delegates to the new
  `emit_program_reporting`, where the drop is one visible `map`. Changing
  the public shape would have churned every one of those call sites to say
  nothing new.
- **The failure path renders *every* diagnostic**, warnings included. Each
  carries its own severity prefix, so they read correctly next to the
  errors — and nothing is lost while the author fixes the errors, which is
  what the old all-diagnostics behaviour got right before it also aborted.

`salvo platform generate` deliberately reports nothing: it writes host stubs
once, and the program's diagnostics belong to the compile path.

**One rule, one implementation**: the refinement-conflict test and the
declaration-site [qual-with] check now share `refine::quals_compatible`. A
conflict *is* "these two could not have been written together", so letting
the two drift would have been a bug in waiting.

The feature is `crates/salvo-core/src/refine.rs` (~700 lines): resolve each
refinement to its overload, validate it, compute per-file visibility, merge
and detect conflicts. The checker applies groups after the removal set in
the named-call contract loop; `deduce.rs` applies the same table through
`apply_callee`; the LSP merges the docs [qual-refn-docs].

**`use Handler<T>()` now binds the handler's generics (fixed 2026-09-06).**
Found while recording it as a defect, which is the only reason it was found at
all: the written type arguments were *parsed* and then read by nobody, so the
checker left the effect instance generic and both emitters constructed the
handler with no type argument. Both backends emitted code their own compiler
rejected — Kotlin "cannot infer type for type parameter 'T'", Rust `E0283`
plus `E0392` — with `salvo analyze` reporting nothing, which is a
[backend-never-wrong] violation rather than a documented cut. A generic
handler could therefore only be instantiated when a *constructor argument*
happened to bind its parameter, which a stateless one never has.

The fix follows the data: `check_use` binds the handler's generics from the
written list first (a constructor argument that disagrees is now an error, and
the wrong number of arguments is reported), records them as
`use_handler_args`, and both emitters write them at the constructor —
`Plain<Int>()`, `Plain::<i32>::new()`. Rust additionally emits a
`PhantomData` field for a type parameter no field mentions, since a handler is
a *behaviour* and a generic one need hold nothing, where Rust insists every
parameter be used (`E0392`).

Two things this corrected in the specs, both under [rs-effect-fusion]'s cut
list. Its "Reported" promise held only on the *fusion* path, so a
single-effect program emitted invalid Rust instead; and its claim that
"Kotlin accepts these (erasure)" was false — erasure removes a type argument
from the JVM, not from the source, so Kotlin needs it written exactly as Rust
does. The type arguments are now written **even where the target could have
inferred them**: the emitter does not reason about the target's inference, and
the case that motivated the fix leaves nothing to infer from. That made two
golden snapshots and three assertions more explicit.

**Implicit parameters landed 2026-09-05 (user decisions), and they are what
Salvo got instead of traits.** `?cmp: (T, T) -> Int` is a parameter the caller
need not pass: the call site fills it by resolving the parameter's *name* at
the parameter's *type*. The whole feature rests on one observation — Salvo
already overloads by parameter type, so **the default for a type is just a
function**:

```
params Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn total<T>(xs: List<T>, ?Field<T>) -> [xs] T { ... add(acc, x) ... zero() ... }

total(list(1, 2, 3))                          // 6, defaults resolved
total(list(2, 3, 4), add = times, zero = one)  // 24, overridden by name
```

Koka spells the defaults `Str/cmp` because it does not overload on argument
types; here the `cmp` whose parameters accept `Str` already *is* the ordering
for `Str`, so no qualified-name syntax was needed and nothing is tied to a
type's declaration. The decisions (all user, 2026-09-05):

- **Resolution is name + type, at the call site**, with no global coherence:
  which default a call gets depends on what is visible where the call is
  written, exactly as `use` and handlers already work. Ambiguity is an error
  naming the override as the remedy.
- **Generic code forwards its own implicits automatically** — matched by name
  and type. It is the only thing that *can* fill an inner call there, since
  nothing about an opaque `T` is knowable [call-resolve]. A generic fn that
  declares none cannot call one that needs one; the error says which to add.
  Approved as colouring in the same shape effects have.
- **A group has no binder** (the refinement that shaped the design):
  `?Field<T>`, not `?ops: Field<T>`. The binder referenced nothing (members
  are called unqualified), prevented no collision, and forced the caller to
  know it just to override one member. Without it the *individual* implicit
  parameter is the only unit in the system — resolution, forwarding and
  override all key on a member's own name — so an inner fn may declare `?add`
  directly, or reach the same parameter through a different grouping.
- **A group is declaration-side sugar, never a value.** That is what keeps it
  free of any runtime representation: neither backend knows groups exist.
  Verified before choosing: a struct of fn-typed fields runs on Kotlin but
  **does not compile on Rust** (`E0562: impl Trait is not allowed in field
  types`), so a bundle-as-value would have needed `Box<dyn Fn>` fields and a
  way to call a fn-typed field — which dot-notation already spells otherwise
  (`ops.add(a, b)` *is* `add(ops, a, b)` in Salvo, and even `(ops.add)(x)`
  routes there).
- **Overrides are named arguments**, `cmp = f`, scoped to implicit
  parameters — Salvo has no general named-argument form, and a general one
  stays a separate decision. Unambiguous because assignment is a statement
  here, never an expression.
- **Implicits trail**, and are declared on a **fn or an effect member** (the
  member half added 2026-09-06: "they're just normal functions"). A member's
  implicits belong to its signature — the interface takes them, every handler
  takes them, the call fills them. Handler *constructors* and lambdas still
  may not have them: `use` resolves nothing, and a lambda's type has no room
  to declare one.
  * On Rust an implicit parameter is `&mut dyn FnMut(..)` — **`dyn`
    everywhere**, decided by two failures in a row. First the trait and its
    impl disagreed (`E0053`), because a member's implicits have to be `dyn`
    for `&mut dyn E` to stay object-safe. Then a member *forwarding* to a
    plain fn handed a `dyn` value to an `impl` (`Sized`) parameter, which
    rustc refuses — so a per-position convention could not compose, and one
    convention everywhere is both simpler and the only sound choice. The cost
    is an indirect call, which every effect member call already pays.

`[name-casing]` pays off unexpectedly: `?cmp:` versus `?Field<T>` is decided
by the case of the first token after `?`, so the grammar needs no lookahead.

**No new backend machinery**, which is the sharpest contrast with the traits
design this replaced (that needed generated interfaces on Kotlin and real
traits plus impl blocks on Rust). An implicit is an ordinary trailing
parameter of fn type; the call site passes a function reference (`::add`) on
Kotlin, an adapter closure on Rust. Two Rust wrinkles were worth the trouble:
a resolved fn is a fn *item*, not a closure, so it is wrapped mechanically;
and an argument that reborrows an implicit the same call passes has to be
hoisted into a `let` first, or the borrows overlap (`E0499`) — the same rule
[effect-args-hoisted] already applies to threaded effect values.

**The contract is part of fitting, and it does not print** (2026-09-06). What
a call does to each argument decides whether a function fits a fn-typed
position, but `Ty`'s Display shows parameters, effects and the result — not
the contract. So a mismatch there used to read "expects `(Int, Int) -> Int`,
found `(Int, Int) -> Int`". The checker now explains it instead: which
argument, in which direction, and both fixes (give the candidate a deduction
list that returns the argument, or declare the position as consuming). Two
further touches came out of the same work: a *near-miss* is reported as one
("no `cmp` fits … the `cmp` in scope is …") rather than as "nothing of that
name", and candidates are **ranked**, so a same-shape wrong-contract
declaration is what gets explained rather than whichever unrelated overload
of the name came first — the first version dutifully reported std's
`add(Mut List<T>, T)` when the user's own `add(Int, Int)` was the near-miss.

Verified end to end on both backends with one shared program and one shared
expected stdout: group defaults resolved, one member overridden by name, a
lambda override, forwarding through an opaque `T`, an individually declared
`?add` reached by the same name, and (a second shared program) an effect
member's implicit resolved at the call and received by its handler.

**The struct-field-of-fn-type hole is closed as part of it**: that shape is
now a Rust codegen *error* naming the implicit-parameter remedy, rather than
invalid output rustc rejects [backend-never-wrong].

**The suite got its iteration cost back, 2026-09-05.** The toolchain tests
had grown to ~75 of the ~80 seconds a `cargo test` took, which was starting
to shape how often it got run. Three findings, in order of how much they
returned:

1. **The availability *probe* was nearly as expensive as the work.** Each of
   39 Kotlin tests ran `kotlinc -version` as its guard, and that starts a
   JVM: 1.4s, against 2.4s for the compile it guarded. Probing once per test
   binary took the Kotlin crate from 44s to 32s without touching a test.
2. **A verification is worth remembering.** Compiling and running generated
   code is a pure function of the code, the expected output and the compiler
   doing it, so `salvo-testkit` writes a stamp keyed by the content hash of
   exactly those inputs and skips the work when one already exists. A plain
   `cargo test` is therefore still complete but now costs **~8s warm** (80s
   cold). Touch the emitter and every affected stamp misses — verified by
   adding one comment line to the emitted output and watching the suite go
   straight back to 15s of real toolchain work. A stamp is written only after
   every assertion passes, so a failure is never remembered as a success.
   `SALVO_E2E_FRESH=1` ignores stamps: that is what **full** means, ~70s with
   nothing taken on trust.
3. **`cargo nextest` did not help, and is kept for diagnosis.** It runs each
   test in its own process, which is a loss when 531 of them are mostly
   microseconds: 18s warm against `cargo test`'s 8s, and a wash on a fresh
   run (74s vs 71s). Thread count does not change the picture (`-j6` 22s,
   `-j20` 18s). What it *is* good for is per-test timings, which stock
   libtest will not print on stable — that is how the CLI's five long poles
   were found. Config in `.config/nextest.toml`.

Two smaller things fell out. The Kotlin and Rust runners were writing their
scratch trees to the *system* temp dir, against the repository's own rule
that temporary files stay inside it; they now use `CARGO_TARGET_TMPDIR` like
everything else. And three Kotlin tests had no toolchain guard at all, so
they would have *failed* rather than skipped on a machine without `kotlinc`
— found by moving the gate inside the runner helpers, where a test cannot
forget it.

**`Iter<T>` is now lazy on both backends (user decision 2026-09-05, option 3
of the four costed below).** The divergence that prompted it: the same
program printed different things, because Kotlin's `Iterable { iterator { …
} }` produced elements on demand and re-ran its producer per pass while
Rust's `Vec` materialised everything at creation — so *when* a producer's
effects happened, *whether* they interleaved with the consumer, and
*whether* a second `for` re-ran them all differed. The user chose lazy on
both **with iterator functions restricted to being effect-free**, on the
grounds that "other functions can just use them with effects via
for-loops".

What that took:

- **[iter-effect-free]** An iterator fn declares no effects, `use`
  included. Checked once at the declaration, which is enough: performing an
  effect requires declaring it, and `use` is what would otherwise let a
  body register its own handler. The diagnostic names the remedy (perform
  it where the elements are consumed).
- **[rs-iter-lazy]** `Iter<T>` is a generated `SalvoIter<T>` — a *factory*
  of passes, `Rc<dyn Fn() -> Box<dyn Iterator<Item = T>>>`, which is
  exactly what Kotlin's `Iterable` already was. Keeping it a factory rather
  than a one-shot iterator is what avoided the second decision the costing
  flagged: `for` still does not consume its subject, and linearity is
  untouched.
- **Stable Rust has no generators, so the state machine came from
  `async`**: rustc builds one for an `async` block, and the generated
  `SalvoGen` drives it with `Waker::noop()`. `yield v` becomes
  `__slot.replace(Some(v)); SalvoYield::once().await;`. No `unsafe`, no
  crates, ~120 generated lines in `iter.rs` (mounted like `unions.rs`, only
  when the program touches `Iter<T>`). Hand-written and verified before
  being taught to the emitter, per the gotcha that says to do exactly that.
- **Parameters are captured once into the factory and cloned per pass**, so
  each pass starts from the beginning and the captured state is `'static`.
  Sound *because* of [iter-effect-free]: a handler arrives as `&mut dyn E`
  borrowed for the call and could not live that long. Generic parameters
  gained `'static` next to the blanket `Clone` bound for the same reason.
- **One convention exception**: an iterator fn's fn-typed parameter arrives
  owned as `impl Fn(…) + 'static` (shared across passes via `Rc`) rather
  than `&mut impl FnMut(…)`, because it is called in every pass rather than
  during the call. `Fn` rather than `FnMut` follows from repeatability —
  callback state would depend on how many times the iterator was consumed,
  which is the same divergence [iter-effect-free] rules out for effects.
  The checker does not reject a state-mutating callback up front; rustc
  does. Known gap, recorded in [fn-iterator].

Verified end to end on both backends with one shared program and one shared
expected stdout: an *unbounded* producer (`while true { yield … }`) that
terminates because the consumer `break`s — it would previously have hung
forever on Rust — a filter over it, and a factory consumed twice that
starts over each time.

**Three defects found by writing `map`/`filter`/`reduce` in Salvo, fixed
2026-09-05.** Testing the combinators — rather than reasoning about them —
turned up one checker bug and two silent-wrong-code bugs, all independent of
the traits question that prompted the exercise:

- **[call-generic-progressive]** A callee's type variables now bind
  *progressively*, left to right, so an un-annotated lambda argument is
  checked against the pattern its *siblings* already determined
  (`map(xs.iter(), n -> n * 2)`). Before, the lambda was checked against an
  unsubstituted `(T) -> U`, so its inferred type mentioned the callee's own
  variable (`(T) -> T`) — exactly what `unify`'s deliberate lack of an
  occurs check assumes cannot happen [fn-overload] — and the call failed to
  match itself. An explicit type-argument list did not help either; it is
  now what seeds the substitution. Left-to-right only: a lambda written
  before the argument that would bind its parameter type still needs an
  annotation, because Salvo does not look forward [call-type-args] and a
  fixed-point pass would reorder the fate events a call records.
- **[rs-fn-param-convention]** A lambda in a fn-typed parameter position
  now binds its parameters the way the *callee's declared* fn type renders
  them. The two sides used to test `Copy`-ness on different types — the
  declaration on `T` (never known to be `Copy`, hence `FnMut(&T)`), the
  lambda on its own annotation (`Int`, hence `|n: i32|`) — so rustc
  rejected the call with `E0631`. Invisible until a lambda was *annotated*:
  the un-annotated form compiled, because rustc inferred the parameter from
  the bound.
- **[kt-fn-mangling]** Overload dispatch is the checker's, and Kotlin no
  longer gets a second opinion: every emitted overload of a name gets a
  unique Kotlin name, by the same rule the Rust backend already used
  [rs-fn-mangling]. Sharing the name let Kotlin resolve by *Kotlin's*
  lattice, and two Salvo types with no subtype relation can map onto Kotlin
  types that have one (`Iter<T>` → `Iterable<T>`, `List<T>` → `List<T>`,
  and Kotlin's `List` *is* an `Iterable`). A `List` overload delegating to
  its `Iter` sibling emitted a call that re-resolved to itself: infinite
  recursion, no diagnostic anywhere [backend-never-wrong].

**The `Iter<T>` laziness divergence is closed** — see the lead entry: option
3 was chosen and built on 2026-09-05. The divergence as measured (Kotlin's
`Iterable { iterator { … } }` lazy and re-running its producer per pass,
Rust's `Vec` materialising at creation) is gone, and the obstacle that
shaped the decision — a lazy Rust iterator would have to hold the fn's
effect handlers *across* the suspension, where emitted Rust borrows them
only for the call — is what [iter-effect-free] removes rather than works
around.

**Non-resumption was renamed 2026-09-05 (user decision): `Abort` → `Throw`,
`abort` → `throw`, `Aborted` → `Thrown`.** A pure rename of the E3 step-2
feature, taken under the no-backwards-compatibility invariant: the old
spelling simply stopped being the language, with no transitional
diagnostic. What moved: the std module `std/core/abort.sv` →
`std/core/throw.sv` (module `core.abort` → `core.throw`), the compiler's
known names (`ABORT_EFFECT`/`ABORTED_QUALIFIER` → `THROW_EFFECT`/
`THROWN_QUALIFIER`, with `OK_QUALIFIER` untouched), `Checked::may_abort` →
`may_throw` with `AbortSite` → `ThrowSite`, the Kotlin signal
`salvo.AbortSignal` → `salvo.ThrowSignal` in `abort.kt` → `throw.kt`, and
the rule labels `[abort]` → `[throw]`,
`[abort-not-main]` → `[throw-not-main]`, `[abort-linear]` →
`[throw-linear]`, `[kt-abort-signal]` → `[kt-throw-signal]`,
`[rs-abort-controlflow]` → `[rs-throw-controlflow]`. `try` is untouched:
it was never named after the operation. Nothing about the *semantics*
changed — `throw` is still an ordinary effect operation returning
`Nothing`, still handler-less, still delimited by the intrinsic `try`
yielding `Ok T | Thrown M`; in particular it is **not** a keyword, so
`throw(message)` is a plain call. The test suite kept its 514 tests
(`crates/salvo-core/tests/abort_tests.rs` → `throw_tests.rs`), and both
backends still compile and run the demo byte-identically under `kotlinc`
and `rustc`. Two sites were deliberately left alone because their `abort`
is the English verb, not the feature: `salvo-cli`'s "does not abort the
analysis" / "aborting due to N parse errors", and the roadmap's
"panic/abort semantics out of scope" (Rust's process abort).

**Interop was redesigned 2026-09-05 (user decisions): string-template
interop was replaced by `platform effect`, and the redesign is complete.**
The old model — `external` declarations resolved by `define` templates that
interpolate `${param}` into target syntax — was, in the user's words,
"difficult to validate, and only really there so that certain std types and
functions (like those related to the list) don't get unnecessarily wrapped in
backend functions". The replacement follows Roc's platform idea: the compiler
*generates an interface* and the host implements it, so the target
language's own compiler checks the two against each other. There is now
exactly one interop path for customer code (`platform effect`) and one for
std (`intrinsic`, which only std may declare).

The decisions (all user, 2026-09-05):

- **`internal` → `intrinsic`**, and it is the *compiler's* modifier:
  declared by std, never by customer code, because "I don't see why a
  customer would be able to declare `intrinsic` if the compiler doesn't
  already declare it".
- **`external` → `platform`, and `define` goes away entirely** — "one
  interop path is good". So every std lowering moved into the backends as
  code, and the unvalidatable template text is gone from the language.
- **A platform declaration is an `effect`**, not a free function
  (agreed after the alternative — top-level `platform fn` collected into
  one synthetic effect per module — was costed): the author picks the
  interop boundary, the name is theirs, and the implementation has
  somewhere to keep state and dependencies.
- **The host owns `main`.** A platform effect's instance is constructed
  outside Salvo, so it cannot be `use`d; it is a *parameter*. A `main`
  declaring one is emitted as `salvoMain`/`salvo_main` and the host's
  `main` calls it. The user accepted this consequence explicitly, noting
  it can be skipped when `main` needs no platform effect — which is how it
  works.
- **Deferred**: `platform handler` (a host handler of an ordinary Salvo
  effect, constructed by `use`) and `platform type` ("let's leave platform
  type until a need arises" — the type-aliasing gap the user was willing
  to accept, with platform structs sketched as a future answer).
- **Rejected**: generic platform effects and generic platform members.
  **Made errors**: member-name collisions.
- The CLI command is **`salvo platform generate`** (the `-api` suffix was
  dropped).

**The interface framing removed work rather than adding it, in two places
worth recording.** First, a platform effect *is* an effect, so the existing
interface/trait emission, `&mut dyn` threading and handler fusion all
applied unchanged — the only emitter surgery was the `if !is_main` guard on
effect-parameter emission. Second, it made the whole
"don't overwrite the customer's implementation" problem disappear: the
first design needed marker-delimited regions keyed by a canonical mangled
name, with signature-change detection, because generation would run
repeatedly over a file the customer edits. With an interface, the
implementation file is generated **once** and never touched again, and every
kind of drift is a target-language compile error — a member added is
"does not implement abstract member" / `E0046`, one removed is "overrides
nothing" / `E0407`, a changed signature is an ordinary type error, and a new
platform effect breaks the entry-point call. No markers, no keys, no merge.

Landed so far (each step left the tree green):

1. **`internal` → `intrinsic` everywhere.** `TokenKind::KwIntrinsic`,
   `BackingMod::Intrinsic`, `emit_intrinsic_call`,
   `Symbols::intrinsic_types`, rules `[backend-intrinsic]` and
   `[intrinsic-fn]`. Incidentally deleted the dead duplicate keyword list
   in `TokenKind::symbol()` — the `KEYWORDS` lookup precedes it, so those
   arms were unreachable, and they are exactly the drift hazard the gotcha
   below records as having once panicked the compiler.
2. **std migrated off templates.** New `intrinsics.rs` per backend
   (`type_name`, `mut_type_name`, `fn_call`, `handler_member`); all ten
   `std/**/*.{kotlin,rust}.sv` files deleted; `emit_define_handler` became
   `emit_intrinsic_handler`, taking member signatures from the *effect*
   declaration. Dispatch is on the checker-resolved declaration (name +
   first parameter's base type name), which is what separates
   `size(Str)`/`size(List)`/`size([])` without the define era's arity
   guessing. **No backend golden snapshot changed at all** — the emitted
   output is byte-identical to the template era, which is the strongest
   evidence the migration is faithful.
3. **`platform effect`.** Parser (the modifier takes nothing but `effect`),
   five checker rules, both emitters, entry-point restructuring. Verified
   end to end: the same source, with a hand-written host implementation,
   prints `[telemetry] work=41` / `result=42` under both `kotlinc` and
   `rustc`.
4. **`salvo platform generate` + the `platform/` tree wiring**, which
   together close the loop: the compiler now writes the host skeleton and
   builds it back in, so a platform program goes from `.sv` to program
   output with two commands and no hand-written glue.

   *The tree* is the source root's `platform/`, mirroring the source
   layout (`platform/app/entry.kt` for `app/entry.sv`). It reuses the
   companion mechanism [backend-companion] wholesale — `CompanionFile`
   gained a `platform: bool`, and `classify_companion` strips a leading
   `platform/` segment so the file is attributed to the module it
   implements for. The strip is load-bearing, not cosmetic: a companion is
   only copied when its *module* is reachable, and no Salvo module is ever
   called `platform.app.entry`, so without it the host file would be
   discovered and then silently dropped. Only the root's `platform/` is
   special, so a module named `platform` keeps its own companions. Both
   backends' hosts coexist in one tree, since discovery filters by the
   active backend's extension.

   *Kotlin* puts the host in package `salvo.platform.<module>`, not
   `salvo.<module>`. That was forced: Kotlin names a facade class after the
   **file**, so `platform/main.kt` sharing `salvo.main` would put a second
   `MainKt` on the classpath. The consequence is that the launch class
   changes when the host owns `main`, which is why `Backend::entry_hint`
   now takes the emitted file list — the presence of the host file is the
   evidence, and it is exactly the right one, since a platform `main`
   cannot be emitted without it.

   *Rust* mounts the host as `platform_<module>` (prefixed so it can never
   collide with the module it implements for) and appends
   `fn main() { crate::platform_<module>::main() }` to the crate root,
   because `fn main` must live there. The `rustc` invocation is unchanged.
   `module_mod_names` was extracted so `emit_program` and the skeleton
   renderer cannot disagree about what a module is called — a skeleton
   naming `crate::core_console::…` where the crate calls it something else
   would be generated code that does not compile.

   *The skeletons* are rendered by the same `Emitter` and the same
   signature renderers the interface emission uses (`emit_param_list` /
   `emit_member_param_list`, `emit_return_type`), on the same checked
   program, and the entry-point arguments come from the same
   `checked.fn_effects` table the entry's *parameters* come from. That is
   the point: a skeleton that does not match the interface it implements is
   impossible by construction rather than by test coverage. `TODO(…)` /
   `todo!(…)` bodies typecheck in return position, so a value-returning
   member stubs without a cast.

   *The command* never overwrites — an existing file is reported and left
   alone — which is the whole dividend of the interface framing recorded
   above. It shares the front end with `compile`/`run`: `build` was split
   into `assemble` (load, parse, resolve the entry) + emission, so
   `platform generate` resolves `--src`/`--main` identically.

   *The missing-host gap is closed by an error, not by hope*: any reachable
   module whose `main` needs a platform effect and has no host file is a
   codegen error naming `salvo platform generate` and the exact path. The
   rule is per module and identical on both backends, so a two-entry
   directory cannot compile on one backend and fail on the other. The old
   symptoms (`'main' method not found in class salvo.main.MainKt`, rustc
   `E0601`) are unreachable now.

   Both backends' `platform` tests were rewritten around the *generated*
   skeleton with only its stub body replaced, so what they prove is that
   the skeleton is complete and correct everywhere else — package, imports,
   trait path, member signature, entry-point call. Same stdout on both:
   `[telemetry] work=41` / `result=42`.
5. **`external`/`define` deleted, and `intrinsic` enforced as std-only** —
   the last item, and the one that makes the redesign real: there is now
   exactly one interop path.

   *Deleted from the language*: the `external` and `define` keywords, the
   `define fn`/`define type`/`define handler` items, the double-backtick
   `Template` token and its lexing, the `imports:`/`inline:`/`Mut inline:`
   sections, and the `<module>.<backend>.sv` define file as a concept. With
   them went `Symbols::{define_fns, define_types, define_handlers,
   external_types}`, `check_define_pairing`, both backends'
   `check_core_define_coverage`, `define_for_decl`, `emit_define_call`,
   `define_type_args` and the whole template-expansion machinery —
   about 900 lines net.

   *Two enums collapsed into flags.* `BackingMod` had one variant left, so
   `backing: Option<BackingMod>` became `intrinsic: bool` on `FnDecl`,
   `TypeDecl`, `HandlerDecl` and `QualifierDecl` — the same shape
   `EffectDecl::platform` already had, and for the same reason the earlier
   gotcha records: a one-variant enum is an invariant expressed as a
   runtime check. `SourceKind` went the same way: with no define files
   there is only one kind of source, so the field is gone rather than
   always `Language`, and every `if file.kind != Language { continue }` in
   the checker, both emitters and reachability went with it.

   *Three new rules fell out of the deletion, each replacing something that
   used to be silent or meaningless.*
   - **[decl-body]**: a bodiless top-level `fn`, and a `type` with neither
     `= alias` nor `intrinsic`, are parse errors. Those were `external`'s
     shape; without it there is nothing for them to mean, so the parser
     says so and names the surviving forms rather than reporting a bare
     "expected `{`".
   - **[intrinsic-std-only]**: `intrinsic` is the compiler's, so only std
     may write it. It is checked in `check_intrinsic_is_std_only` rather
     than the parser, which sees one file's tokens and cannot know where
     the file came from; the checker has `is_std` at hand. Not cosmetic —
     the backends dispatch intrinsics from a table keyed by *name*, so a
     customer `intrinsic fn` has no lowering anywhere, and the diagnostic
     names the `platform effect` path instead.
   - **[mod-file-name]**: a `.sv` file name may not contain a dot. This is
     exactly the spelling that used to select a backend's define file, and
     `classify` used to *skip it in silence* — so a leftover
     `main.kotlin.sv` would have vanished from the build. It now errors,
     naming both the file and the nested path it is ambiguous with. That
     is why `SourceSet::classify` returns `Result` and `add_dir` returns
     rendered messages rather than `(PathBuf, io::Error)` pairs.

   *CLI surface shrank.* `salvo analyze --backend` and `salvo lsp
   --backend` are **gone**: their only documented purpose was loading a
   backend's define files, and an option that does nothing is worse than
   no option. Both commands are unconditionally backend-neutral now.

   *The test sweep was the bulk of it* — 108 `external` declarations across
   14 files. Most became ordinary fns with the *same* signature and a
   minimal body, which is what keeps the tests testing what they tested: a
   written deduction list stays authoritative over anything inferred from a
   body, so the contract under test is unchanged and the call stays on the
   named-call path rather than moving to `check_effect_call`. The
   `intrinsic` type stubs those tests relied on moved into a std-loaded
   `core/prelude.sv` per harness (module `core.prelude`, implicitly
   imported), which is precisely the `is_std` split this document predicted
   would be needed. Tests whose *subject* was the define machinery were
   deleted outright — missing-define errors, core coverage, the
   define/external pairing, template generics — along with the
   `defines.kotlin.sv` corpus file and its snapshot. **No golden snapshot
   of emitted code changed**, again: the second time in this redesign that
   byte-identical output has been the evidence a mechanism swap was
   faithful.

   *Two things the suite caught that a human sweep would have missed.* The
   TextMate grammar still listed `external` and `define` as keywords and
   still defined a double-backtick template rule — caught by
   `keywords_are_fully_categorized`, which asserts the grammar's categories
   partition the lexer's keyword table exactly. And `run_tests` still
   asserted the old "backend define file" message for a dotted `--main`,
   which is now the [mod-file-name] error.

Still to do: nothing from the interop redesign. Its deferred pieces remain
deferred by decision — `platform handler` (a host handler of an ordinary
Salvo effect, constructed by `use`) and `platform type` — and are recorded
under the decisions above rather than as leftovers.

**One cut taken beyond the decisions, flagged to the user**: generic
platform *effects* are rejected as well as generic members. A generic
platform effect would need the host to implement one interface per
instantiation — Kotlin's facets exist for that and Rust has no equivalent,
and the fusion already refuses generic effect instances there.

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
a std intrinsic), where the Rust splice does not. Salvo has no `catch`, so
side effects during a crash are not part of a program's meaning; tightening
it means catching the throw signal specifically once `throw` exists.

Probed by hand on both backends beyond the test demo (identical output
except where noted): a `defer` in a value-position block, a `defer` nested
inside a deferred body, a deferred call through an *effect* member with the
handler registered by `use`, a `defer` inside a lambda block body passed to
a fn-typed parameter — and a `defer` in an *iterator* body, which differed
at the time: Rust's eager collection ran the deferred prints before the
consumer saw any element while Kotlin's lazy sequence interleaved them.
That cut predated `defer` (a plain `println` after a `yield` diverged the
same way) and is **closed** as of 2026-09-05, when `Iter<T>` was made lazy
on both backends [rs-iter-lazy].

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

**`salvo run` landed 2026-09-05 (user design).** One command from `.sv`
source to program output: compile with a backend, then build and run the
result with that backend's own toolchain [cli-run].

```bash
salvo run --backend kotlin --src ./my_project
salvo run --backend rust --main ./my_project/main.sv
salvo run --backend rust --src ./my_project --main ./my_project/bin/tool.sv
```

`--backend` is required (it decides which toolchain must be installed, so
there is no defensible default). **`--src` and `--main` are independent, and
each supplies a reasonable default for the other** (user decision
2026-09-05): `--src` alone uses the unique `main` the directory declares,
`--main` alone additionally implies `--src $(dirname FILE)`, and **both** is
the only way to say "compile this tree, start at this file" when the entry
sits in a subdirectory. That last case is the one that earns the
combination: `--main bin/tool.sv` alone would take `bin/` as the source
directory, leaving a module shared from above it outside the compilation.
Either way `--main` is how you pick between several entry points.

The first sketch had `--main` walk imports and include only the modules it
transitively reached; the user cut that as premature (2026-09-05), and
rightly — the whole directory is compiled either way, and reachability
already prunes the *output* to the modules actually used.

`--target` defaults to `.salvo_tmp_run` in the working directory, and
`--clean-target` (`before` | `both`, default `both`) says what survives:
both modes clear the target *before* the build, so a run never picks up the
previous run's output. **The command's exit code is the program's** and its
stdio is inherited, so `salvo run` can stand in for running the binary
(verified: a Rust panic propagates 101, a JVM exception 1).

Two design points worth keeping:

- **The target may not overlap the sources**, for two *separate* reasons.
  It may not be or contain the source directory, because the target is
  deleted before the build. And it may not sit *visibly* inside the sources,
  because the next build would read the emitted files back — output carrying
  the backend's native extension is indistinguishable from a hand-written
  companion file [backend-companion]. Nesting under a dot-prefixed
  directory is fine, since discovery skips those [mod-ignore]; that is
  exactly why the default target is `.salvo_tmp_run` and why the literal
  reading of "no overlap" had to be rejected — it would have made the
  default illegal for the most natural invocation (`cd project && salvo run
  --main main.sv`). Independently, clearing a target that holds any `.sv`
  file is refused: a guard on the deletion itself, not on the paths.
- **The entry choice had to reach the backend**, not just the launch
  command. Running is not purely a post-processing step: Rust gives the
  `main`-declaring module the crate root [rs-crate], so with two entry
  points the emitter picked the first it found and `--main other.sv` built a
  module with no `mod` declarations — rustc reported unresolved imports for
  every module. `Backend::emit` now takes the selected entry; Kotlin ignores
  it (a `MainKt`-style facade per module means the choice only picks the
  launch class). This also moved the `entry point:` hint out of the CLI's
  `match backend.name()` and into the trait, where that knowledge belongs —
  and immediately exposed the hint's own bug (see the gotchas: the Kotlin
  facade class is named after the *file*, so it is not always `MainKt`).

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

**E3 step 2 landed 2026-09-04: `throw` + the intrinsic `try` on both
backends.** Non-resumption works end to end: a fn that may throw declares
`[Throw<M>]` and keeps its own return type, `throw(m)` returns `Nothing` so
the frames in between stay silent, and `try { ... }` — a compiler
intrinsic, not an effect — yields `Ok T | Thrown M`. Four user decisions
shaped the open questions (all 2026-09-04): **`M` is the union** of the
body's message types (chosen for consistency with `if`/`when` branch types,
with generated code wrapping only when a union is present), a **`try` whose
body cannot throw is an error** rather than `Thrown None`, **`main` may not
declare `[Throw<M>]`**, and `Throw`/`Thrown` are **declared in std**
(`std/core/throw.sv`) with the compiler knowing only their names.

The roadmap said to hand-write and run the three deciding Rust shapes first;
that paid for itself twice. It confirmed `?` on `ControlFlow` is stable and
that a may-throw call inside a loop stays a loop (no trampoline — the reason
CPS-splitting was rejected for this rung). And it showed the sketch's
closure lowering for `try` is the wrong shape: a closure would capture the
fn's effect parameters, so **`try` is a labelled block** instead
([rs-try-label]) — no captures at all. The price is that `?` cannot be used
inside a `try` body, so a may-throw call there becomes an inline `match`
that breaks the label. The same `match` form is what lets a deferred release
run on the throw path, since `?` returns without running the splice — which
is the third shape, and the concrete payoff of building `defer` first.

Kotlin diverges in mechanism, as expected: the JVM's unwinding *is* the
propagation, so `throw` throws a generated stack-trace-less
`salvo.ThrowSignal` and an intermediate frame does nothing at all. One
consequence was not obvious and is worth remembering: **the thrown arm has
to be chosen at the `catch`, not at the throw.** Rust wraps a message into
the delimiter's union arm at the propagation site; the JVM has no such site,
and a throwing frame cannot know which `try` will catch it. So the signal
carries the Salvo type name of the message as a `tag` and the delimiter
dispatches on it, with an `else -> throw __signal` rethrow for a signal from
outside its set ([kt-throw-signal]). Comparing Salvo type *names* rather
than JVM classes keeps it erasure-proof.

Verified by compiling and running the same program on both: the value path,
the throw path with a linear handle released by `defer` *on it*, two message
types meeting at one delimiter (`Thrown (Str | Int)`), a may-throw call
inside a loop, and a nested delimiter that does not swallow the outer throw
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
the arguments, a backend can spell an element type out where its own
compiler cannot infer it [backend-intrinsic], which is how Kotlin's list
constructors get theirs.

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

**Also landed with it: the era's `define fn` templates gained type-argument
interpolation** (user request "I want the templating to be consistent").
`${T}` resolved against the define's own type parameters using the checker's
new `call_type_args` table, exactly as `define type` templates already did.
The templates are gone (2026-09-05), but `call_type_args` outlived them: it
is what the backends' intrinsic lowerings read. std's Kotlin list defines
used it
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
`defer`, then `throw` returning `Nothing` with a compiler-intrinsic `try`
yielding `Ok T | Thrown M`, then effects on fn types threaded into the
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
Target-language features are reached by *declaring* them — as of
2026-09-05 that means a member of a `platform effect` [platform-effect]
(then `external type` for the type and `external fn` + `define` for
anything you did with it) — and dot-notation still reads like a method call
because it *is* a call to a declared function. Generics fall under the same rule: with no bounds,
nothing about a `T` is knowable, so `value.name` inside `fn f<T>(value: T)`
is an error rather than a promise about future call sites.

Six holes closed, all of which the compiler used to accept silently and
hand to the target compiler: an unresolved bare call (`nowhere()`), an
unresolved dot-call (`text.shout()`), calling a value of known non-fn type
(`n()` on an `Int`), a field on a non-struct (an opaque type, a generic, an
`Int`), `[]` on a non-array, and `for` over a non-iterable.
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
standard the new diagnostics are held to: each names the remedy (a
declared fn — since 2026-09-05 a `platform effect` member — for a missing
member, `get(collection, index)` for a subscript, "rebuild the tuple" for an
element write) — and where no remedy
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
inference for anything the compiler cannot see: a bodyless fn must declare
effects, deductions, *and* return type; effect members must declare return
type and deductions [decl-explicit]. (At the time that covered
`external`/`intrinsic` fns and required every `define fn` to match an
external one-to-one; since 2026-09-05 `intrinsic fn` and effect members are
the only bodyless forms, and a bodyless `fn` anywhere else is a parse error
[decl-body].) This removed the last inference-from-nothing guess (and
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
cargo test                  # 602 tests, complete: the toolchain tests are
                            # content-cached, so an unchanged one is not
                            # recompiled — ~8s warm, ~80s cold
SALVO_E2E_FRESH=1 cargo test # FULL: every test, nothing taken from the cache (~70s)
SALVO_SKIP_E2E=1 cargo test # inner loop: ~4s, by skipping every test that shells
                            # out to kotlinc/rustc. Those tests still report as
                            # *passing*, so this is never the pre-submit check.
cargo nextest run           # the same tests with per-test timings (diagnosis only;
                            # measured slower here — a process per test)
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

# Generate the host side of `platform effect` declarations [cli-platform]:
cargo run -- platform generate --backend kotlin --src ./some_dir
# Writes ./some_dir/platform/<module>.kt once per module that declares
# platform effects; never overwrites, so implement the stubs and re-run
# `salvo run`.

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
├── salvo-backend-rust/   # Rust emitter (emit.rs) + golden/rustc tests
└── salvo-testkit/        # dev-dependency for the test crates: toolchain probing
                          #   (once per binary) + the e2e content-hash cache
std/                      # stdlib: core/ (basic, string, list, console) + random.sv
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
  packages + generated imports [kt-package] [kt-imports]; interop coverage
  checked upfront for `core.*` and at reference sites (the `define` era's
  rule, since deleted); backend-native companion files copy verbatim
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
  `&mut dyn` threading; iterators are lazy on both backends (`Iter<T>` is
  a generated factory type [rs-iter-lazy]; it was `Vec<T>` and eager
  until 2026-09-05); `WrapOption` coercion added
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

- **`intrinsic fn copy<T>(value: T) -> [value] T`** in `std/core/basic.sv`
  [intrinsic-fn] [copy-fn]: parser already accepted `intrinsic fn`
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
- **`intrinsic fn discard<T>(value: T) -> [] None`** in std; the `[]`
  deduction makes the discharge just another move. Rust lowers to
  `drop(value)`, Kotlin to `(value).let {}` [intrinsic-fn]. Both
  verified end to end with identical stdout on the open/use/close
  resource demo.
- **Generic ban** in `resolve_named_call` on the resolved substitution:
  linear instantiation of an unconstrained `T` errors; `copy` refuses
  with its own message; `discard` (intrinsic, by name) is blessed.
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
  `intrinsic fn discard<T canbe Linear>(value: T) -> [] None`; `get`
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

## Open defects

Bugs found and reproduced, not yet fixed. Each carries a repro small enough to
paste and a root cause, so picking one up needs no re-investigation.

*(None open. The last two — an own-module fn losing to an identically shaped
std one, and effect member calls not checking their arguments — were fixed
2026-09-07 by the overload-resolution work; see the entry at the top.)*

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

- **L1a — `intrinsic fn copy` (decided).** New `intrinsic` item keyword
  for compiler-intrinsic fns:
  `intrinsic fn copy<T>(value: T) -> [value] T` is declared in std (the
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
  lands before the performance. Includes `intrinsic fn copy`
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
- **L6c**: `intrinsic fn discard<T>(value: T) -> [] None` is the
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

### E3 — Non-resumption: `defer`, `throw`, and an intrinsic `try` (user decisions 2026-09-04)

The first slice of *handler control* beyond "always resumes at the tail",
which is all E1 supports. The exploration ran through four rungs of handler
power — tail-resumptive (today, free), throw (resume zero or one time),
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
qualifier Thrown<M> of M            // mirrors `Err<T> of T` in core.result

effect Throw<M> {                    // intrinsic; message is moved, like `err`
    fn throw(message: M) [] -> [] Nothing
}

// `try` is a compiler intrinsic, not an effect:
try { body } : Ok T | Thrown M
```

- **`try` is an intrinsic, not an effect** (user decision 2026-09-04):
  "there's not much value in a function declaring the `Try` effect in its
  signature any more than there is in declaring that it uses loops or
  if-expressions". So no `Try` handler to register, no `[Try]` in
  signatures, and — see below — no collision with the fusion.
- **Both arms are qualified: `Ok T | Thrown M`** (user decision
  2026-09-04), reusing `Ok` from `core.result` so ordinary `is` checks and
  exhaustive `when` work on the outcome exactly as they do on a result.
  `Err` is deliberately *not* reused: a throw is not an error value.
- **`Thrown M` is parameterized by a message type** (user decision
  2026-09-04), mirroring `Err`. It follows that the outcome union is
  structurally an `Ok T | Err M`, so union arm identity, narrowing and
  exhaustiveness need no new rules.
- **`Thrown M` is forgeable, deliberately** (user decision 2026-09-04):
  the qualifier carries no *authority* — a hand-written `-> M as Thrown`
  produces a value in the thrown arm but transfers no control. The
  authority is `[Throw<M>]` availability alone, which is why the original
  sketch's "only `throw()` may construct it" rule turned out to be
  unnecessary. No provenance semantics, no intrinsic qualifier.
- **`throw` returns `Nothing`**, which is what keeps intermediate frames
  silent: a fn that may throw declares `[Throw<Str>]` and returns `Int`.
  It does *not* also return `Thrown` — that would be `Result` plumbing
  with extra steps and would defeat throw being an effect. `Thrown M`
  appears in exactly one place: the `try` outcome.

#### Lowering

Rust: the message type *is* `ControlFlow`'s `Break` type, so the
propagation falls out of the design rather than being imposed on it.
`throw(m)` is `return ControlFlow::Break(m)` — no handler, no dispatch, no
allocation — every call in a fn with `[Throw<M>]` is `f(..)?`, and the
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
throw:

```kotlin
private class Throw_Signal(val message: Any?, val token: Any) :
    RuntimeException(null, null, false, false)
```

Mechanism divergence with behavioural parity, the same reasoning as facets
and unions: `?` returns through each Rust frame running `Drop`, the JVM
unwinds running `finally`, and nothing user-visible happens on the way out
either way — *provided* `defer` is what puts code on that path.

#### `defer` comes first (user decision 2026-09-04) — **done 2026-09-04**

Not tidiness; three reasons:

1. **It turns a prohibition into a pattern.** Without it, a linear value
   live across a may-throw call cannot discharge its obligation on the
   throw path, so the checker would have to forbid the combination. With
   `defer close(f)` the author discharges on every path and the existing
   flow analysis can count it.
2. **It proves both backends can run code on a throw path** before
   anything depends on that. The two lowerings are exactly the two throw
   mechanisms' unwind paths: a `Drop` guard in Rust (whose reverse
   declaration order gives LIFO for free) and nested `try/finally` in
   Kotlin.
3. **It exercises the capture machinery** the `try` body needs, on a
   smaller independently testable feature. Its own open question is
   whether a deferred body captures by move or by reference — the
   fn-boundary contract question again.

Doing throw first would mean revisiting its lowering to add guards and
`finally` afterwards: the interesting part, twice.

**Built 2026-09-04** — see the decision-log entry at the top for the four
user decisions and the shape it landed in. What the sketch above got wrong:
the `Drop` guard is not a usable Rust lowering (it must own the value from
the `defer` onward, killing the very pattern), and the capture question
does not arise at all under splice-at-exit. What it got right: it does turn
the linear-across-an-exit prohibition into a pattern, and both backends
demonstrably run code on the way out of a block — the guarantee `throw`
now builds on. Reason 3 (exercising the capture machinery the `try` body
needs) is *not* discharged: nothing was captured, so `try`'s body closure
is still unexercised ground.

#### Consequences to settle before building — **all settled 2026-09-04**

- **`M` inference collides with [call-type-args].** ✅ Settled by the
  generalization: `M` is the **union** of the message types the body
  performs (user decision), and a body that cannot throw at all is an
  *error* rather than `Thrown None` (user decision) — so nothing has to be
  inferred from an empty set. The union costs a wrap at each propagation
  site on Rust (`?` needs identical `Break` types) and a tag dispatch at the
  catch on Kotlin; both are in the backend specs.
- **A `Nothing`-typed expression statement must count as terminating.** ✅
  Done, and it went further than "small": both path analyses
  (`block_returns` for [fn-must-return], `block_exits` for branch merging)
  became *type-aware* Checker methods reading the recorded types, and a
  written `Nothing` now lowers to the bottom type rather than a nominal
  type spelled that way. Without the second half, `throw(n)` in a branch
  leaked its consumption of `n` to the fall-through path.
- **The intrinsic couples the compiler to two core qualifier names.** ✅
  `Ok` and `Thrown` are resolved by name from the implicitly imported core,
  with a diagnostic naming the missing one; the compiler knows three names
  in total (`Throw` the effect, `Ok` and `Thrown` the arms) and nothing
  else about them.
- **Nested qualification is the honest consequence of wrapping in `Ok`.**
  ✅ Confirmed, both shapes tested: `Ok None`, and `Ok (Ok Int | Err Str)`
  taken apart through a binding at the inner type. `when` does reject a
  qualified-group subject, but that is not a dead end — the droppable
  qualifier rule unwraps it; see "Nested qualification" below for the two
  alternatives that were rejected.
- **Throw targets the innermost `try`.** ✅ [try-innermost]. Rust needs no
  token at all (the block label decides where a `break` lands); Kotlin's
  catch-all is innermost by construction, and its `else -> throw` rethrow is
  what an escaped function value would hit.

#### Nested qualification: resolved without new surface (2026-09-04)

`try`'s outcome makes two qualified-union shapes reachable — a
result-returning body (`Ok (Ok Int | Err Str) | Thrown Str`) and a union
message (`Thrown (Str | Int)`) — and `when` rejects a qualified-group
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
    is Thrown {
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
  the dual guarantee that a body carrying `[Throw<M>]` cannot outlive its
  delimiter. [effect-not-data] already stops the capability escaping as a
  *value*; the lambda case is what remains.
- It collides head-on with the fusion cut landed 2026-09-04: a realistic
  body performs other effects (`try { println("x"); throw("bad") }`), so
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

1. ~~`defer` — standalone, testable, settles linear-on-throw first.~~
   **Done 2026-09-04** ([defer], [defer-no-escape], [kt-defer-finally],
   [rs-defer-splice]).
2. ~~`throw` returning `Nothing` + intrinsic `try`, with `Thrown<M>` in
   std.~~ **Done 2026-09-04** ([throw], [try], [try-innermost],
   [throw-not-main], [throw-linear], [rs-throw-controlflow],
   [rs-try-label], [kt-throw-signal]). What the design got right and wrong
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
that decide whether propagation stays clean: a may-throw call inside a
loop, one inside a nested `try`, and one with a live linear value plus a
`defer` across it. Loops are the specific reason CPS-splitting into
continuation legs was rejected for this rung — a `perform` inside a loop
becomes recursion through the continuation, and neither target guarantees
tail calls, so ten thousand iterations means ten thousand frames unless you
hand-build a trampoline, which is the async machinery under another name.
The legs idea is right for rung 3, where the continuation must be reified
anyway; there its cost (a boxed closure per call, answer-type erasure) buys
something.

## Roadmap: iterators — `Iter<T>` laziness (**DECIDED and built 2026-09-05**)

`Iter<T>` mapped to Kotlin `Iterable<T>` (lazy, re-iterable) and Rust
`Vec<T>` (eager, materialised). That was a [backend-parity] violation, not a
representation detail: the same program printed different things. Measured
2026-09-05 with a `yield` fn that prints per element —

| | Kotlin | Rust |
|---|---|---|
| created, never consumed | nothing | producer runs |
| consumed once | producer/consumer interleave | producer fully, then consumer |
| consumed twice | producer re-runs | second pass replays the buffer |

The obstacle was never the `Iter` mapping, which is easy on both sides, but
the body of a Salvo `yield` fn: on the JVM the emitted `iterator { … }`
builder suspends and resumes for free, while stable Rust has no generators,
and — the part that decided the design — a suspended iterator must hold the
fn's *effect handlers* across the suspension. Emitted Rust takes them as
`&mut dyn E` borrowed for the call, which a returned iterator cannot
outlive.

The four options as costed, with the user's choice marked:

1. **Eager on both** — Kotlin materialises. Parity by restriction, small
   change, no lifetimes. Loses lazy chains *and* infinite generators.
2. **Lazy where free, eager where not** — `Iter<T>` a repeatable factory,
   std combinators as intrinsics over each target's lazy adapters; a
   user-written `yield` fn stays eager on both. Parity exact, no new rule,
   moderate cost. Loses user-written lazy generators. (This was the
   recommendation.)
3. ✅ **Lazy on both, effects forbidden in `yield` fns** — a checker
   restriction buys `'static` generators, so no lifetimes; Rust lowers the
   body to a state machine. Infinite and lazy generators work; effectful
   ones are rejected on *both* backends. **Chosen** (user, 2026-09-05): "I'm
   happy with iterators needing to be effect-free, since other functions can
   just use them with effects via for-loops."
4. **Lazy on both, lifetimes in the Rust output** — nothing lost, nothing
   restricted, but lifetimes propagate through returns, locals, struct
   fields and unions. Largest change; not taken.

Built as [iter-effect-free] + [rs-iter-lazy]; the lead entry records what it
took. The second decision the costing flagged — whether a lazy `Iter` is
**one-shot**, which would make `for` consume its subject and interact with
linear types — did **not** have to be taken: keeping `SalvoIter` a factory
preserves Kotlin's repeatable semantics exactly. It is back on the table
under the next section, which supersedes this one's *implementation*
(the semantics it chose — lazy, both backends — are kept).

## Roadmap: iterators — Salvo-level pull iterators (decided 2026-09-07, **not started**)

The design above works, and its cost is concentrated in one place: the
Rust body is lowered through `async` because stable Rust has no
generators, and *that* is what forces [iter-effect-free]. An `async`
closure captures its environment `'static`, while a handler arrives as
`&mut dyn E` borrowed for one call, so a suspended producer cannot hold
one. Every other compromise follows from the same root: the `Rc<dyn Fn()
-> Box<dyn Iterator>>` factory, the `T: Clone + 'static` bounds, the
clone-params-per-pass, the `Fn`-not-`FnMut` convention exception, and
eager `map`/`filter` in `seq.sv`.

The exit is to stop borrowing the target language's coroutine transform
and materialise a pull iterator as a **state struct the compiler writes**,
whose `next` takes the effect handlers as parameters like any other Salvo
function. Then effects thread in per resume, nothing is captured, and
[iter-effect-free] dissolves — without going push, so `zip`, `merge` and
lookahead stay expressible.

### How this was decided

A push design was costed first (`yield` as a tail-resumptive effect, à la
Koka: producer runs inside the consumer's dynamic scope). It was
**rejected as the iteration model** (user, 2026-09-07) once it became
clear that its sole real advantage — effectful producers — is a property
of *owning the suspension*, not of pushing, and that push can never zip or
merge two lazy sequences (inverting a push producer needs the `ctl`-clause
continuation capture Koka has and the Rust backend cannot). Pull → push is
free, so a yielding *consumer* can be layered on later if some feature
turns out to need it; nothing in this design forecloses it.

### Decisions taken (user, 2026-09-07)

1. **Pull stays the model.** Push revisitable as an addition, never as a
   replacement.
2. **The protocol is Salvo-level, not intrinsic**: `fn next(st: Mut St)
   -> [st: Mut] Emitted T | Finished`, gathered in a `params Iterator<St, T>`
   group beside the existing `params Iterable<It, T>`. `has_next`/`next`
   retire. Consequences: a user can write an iterator *by hand* (which is
   how `zip`/`merge` become ordinary structs — no materialising one side),
   and the state type binds per call site through the `params` group, so
   it never has to be named in a signature.
   * Tagged `Emitted T | Finished`, **not** `T?`: Salvo's unions are flat, so
     `Iter<Int?>` would collapse `Int??` and lose the end signal. It
     lowers to the generated `Union2`; the emitter may special-case
     `Option` where the element type is provably non-nullable.
3. **Effects thread into `next`** like any other function, so an iterator
   function may perform them and [iter-effect-free] goes away. An
   effectful iterator is consequently not a `std::iter::Iterator` (its
   `next` takes extra parameters), which costs nothing — the `for` lowering
   is ours. Kotlin *could* instead capture the handler at creation (a JVM
   handler is just a reference), and must not: creation-time vs
   consumption-time binding is observable when an iterator outlives the
   `use` scope that made it, i.e. exactly the class of [backend-parity]
   divergence the laziness episode was about.
4. **`close` is mandatory and compiler-injected.** See "Abandonment"
   below.
5. **Boxing at the meeting point.** Each iterator function has its own
   state type, so a position holding either of two producers gets one
   boxed value there and nowhere else. Recorded with an example in
   LANGUAGE.md ("Planned change: Salvo-level pull iterators") for further
   thought. The alternative — rejecting such positions
   [backend-never-wrong] — would have kept Rust output free of `dyn`
   entirely at the price of banning an `Iter<T>` struct field.
6. **One uniform lowering on both backends**, hidden behind hand-written
   runtime modules so user-facing generated code stays concise. Those
   modules move out of the emitters' string literals into real source
   files under each backend crate (`runtime/iter.rs`, `runtime/iter.kt`,
   pulled in with `include_str!`) — which makes them reviewable as code
   and, more importantly, lets a test hand them straight to `rustc` /
   `kotlinc`, turning the "hand-verify the generated shape first" gotcha
   into a standing test. `[mod-used-only]` still applies, and a test must
   compile *emitter output against the checked-in runtime*, since the two
   can now drift in a way a string literal made impossible.
   * The same treatment is wanted for `defer` on Kotlin: an
     `inline fun <R> deferScope(f: DeferScope.() -> R): R` in the runtime
     module instead of the current try/finally splices. **`inline` is
     mandatory** — Salvo's `defer` bans control flow out of the *deferred*
     body, but the enclosing block's `return`/`break`/`continue` must
     still cross the scope function, which Kotlin permits only through an
     inline lambda. It trades a zero-allocation splice for one scope
     object per block.
7. **Two tiers over one protocol**: `yield` functions (compiler-generated
   state machine) and hand-written struct + `next`. Both satisfy
   `params Iterator<St, T>`, so `for`/`map`/`filter` accept either — which
   lets tier 1 stay deliberately incomplete without blocking anyone.
   * **Simple-generator fast path**: when every `yield` sits in the tail
     of a single loop nest, the state is the loop's locals plus a resume
     bit and `next` emits as readable structured code. `range`, `chars`,
     lazy `map`/`filter` land here; `rangeIncl` needs two states but still
     no CFG. The general flat machine is the correctness backstop, not the
     common path, so it can land later than the feature.
8. **Consumer side lowers to `while let`**; a `for` over an obvious
   backend iterable (list, array, `Str`) keeps emitting a native loop, so
   the common case pays nothing.
9. **Factory and pass are distinguished at the type level, by `Once`** —
   see the next subsection. This retired the open DECISION rather than
   answering it: both exist, and the author of an iterator function picks.

### Abandonment, and why `close` is mandatory

A consumer that `break`s stops driving an iterator before it reports
`Finished`, leaving the body suspended at a `yield` forever. Today that is
harmless *only* because of [iter-effect-free]: a producer that cannot
perform effects holds nothing worth releasing, and its pending `defer`
blocks are skipped invisibly (dropping the future / abandoning the
coroutine never runs the emitted exit code). Threading effects in creates
the problem — `let f = open(path); defer { close(f) }` inside a producer
leaks the handle on every `break`.

Because the machine is ours, each state knows which defers are pending, so
it gets a **close path** that jumps to the unwind states and runs them
latest-first. The compiler injects the call on every exit out of the `for`
— exhaustion, `break`, `return`, a `Throw` transfer. Deterministic on both
backends, and it needs no destructors: Kotlin has none, and a Rust `Drop`
could not take the effect parameters a deferred block may need.

**On the linear-types angle** (user asked whether the injection could
later be removed): must-use linearity is already *built* — L6, done
2026-09-02, rules [linear-obligation] / [linear-discard] / [linear-canbe].
So the obligation is expressible today: declare the pass type
`canbe Linear` and draining-or-closing becomes the ordinary all-paths
obligation check, with `close` as its `discard`. The injection is
therefore a *convenience* (the `for` lowering is the one place the
compiler always knows every exit path) rather than a workaround for a
missing feature, and it can be relaxed to a plain obligation whenever we
want the user to see it. The umbrella roadmap already exists — "Roadmap:
toward full linear types" above, remaining phases L5 (places and partial
moves) and L7 (derived-return annotations) — so no new roadmap item was
added; what is *not* yet decided is whether a hand-written iterator's
state must be `canbe Linear` by rule.

### Factory and pass: `Iter<T>` vs `Once Iter<T>` (user decision 2026-09-07)

The 2026-09-05 costing deferred a second question — whether a lazy `Iter`
is **one-shot** — and a state-struct materialisation brings it back, now
sharper: with producers allowed to perform effects, replaying a pass
replays its I/O. Both branches were costed (factory: two loops over one
value both replay, silently re-reading a file; pass: the second use is a
consumption error, and re-reading means calling the producer again, where
the cost is visible).

Neither branch was taken. **The distinction moves into the type**, and
`Once` already means exactly what a pass needs — with no new rule to
write:

```
Iter<T>        // factory: replayable; a fresh pass is minted per use
Once Iter<T>   // pass: a position in a sequence, consumed by driving it
```

Why `Once` fits with nothing invented:

- It is already specified as **never droppable** ("it restricts rather
  than refines", LANGUAGE.md; [qual-*]: the compiler owns permissions,
  which drop, and obligations, which do not). A pass must never be
  forgettable into a replayable recipe, and it cannot be.
- Its **variance is already inverted and already the direction needed**:
  "any ordinary function value can be used where a `Once` one is
  expected — never the reverse" generalizes to "a factory fits where a
  pass is wanted, never the reverse".
- The **conversion in the permitted direction is already a call**: the
  implicit `iter()` that `[iter-resolve]` inserts *is* factory → pass.
- **Enforcement is the existing consumption machinery**
  [deduce-consume] / [once-fn]; no new analysis.
- The backend mapping follows the existing pattern (`Once` fn params
  already compile to `FnOnce`): `Once Iter<T>` is the owned state struct,
  plain `Iter<T>` the argument bundle it re-mints from. Boxing at the
  meeting point (decision 5) applies to each form independently.

Rejected alternative: a second nominal type (`Pass<T>` / `Cursor<T>`). It
would need its own variance rule, its own never-drop rule and its own
name in every signature — three things `Once` already has written down.
The one objection to `Once` is that a qualifier would gate the *operation
set* (`iter` versus `next`), and `Mut` is the precedent for exactly that
(it gates the mutators, overloads select on qualifiers, and a backend may
render `Mut T` as a different type with the compiler inserting the
conversion).

Consequences recorded with it:

- **`Once` generalizes from call-multiplicity to use-multiplicity**
  (user, 2026-09-07), with the fn case as the instance where using means
  calling. Valid positions stay explicit — fn types and `Iter<T>` — and
  whether it applies to *any* type is roadmap **D6**.
- **An effectful producer may return a factory** (user, 2026-09-07).
  Mechanically fine, since handlers arrive per `next`; semantically,
  replay re-does the I/O, and `Iter<T>` visibly means replayable, so the
  type carries the warning. Forbidding it would outlaw the legitimate
  read-a-file-twice case. Each minted pass is closed at its own loop
  exit, so nothing leaks either way.
- **The close obligation stays injected, not declared.** `Once` covers
  no-replay; the injected `close` covers no-leak. Making the obligation
  visible in the language needs linearity conditioned on a use-site
  qualifier — roadmap **D7**, which records the relation to this change.

### Still open

- **D6** — `Once` on any type (and see the I2b collision below, which forces
  the question in a narrower form).
- **D7** — qualifier-conditional linearity.

### I2b as built (2026-09-07): the protocol, and the collision it found

Built and verified so far:

- **`std/core/iterator.sv`** declares the protocol in Salvo rather than in
  the compiler: `qualifier Emitted<T> of T` with its constructor, a fieldless
  `struct Finished {}`, and
  `params Iterator<St, T> { fn next(st: Mut St) -> [st: Mut] Emitted T | Finished }`.
  Names are the user's (2026-09-07), renamed from `Next`/`Stopped` before
  any of it was written.
  * `Emitted` is a *qualifier* so the element keeps its own type and a
    sequence of optionals still has a distinguishable end (`Emitted None |
    Finished` has two arms where `None | None` would have one). `Finished`
    is a *struct* because it has nothing to qualify, and reusing `None`
    would say "absent" where the claim is "the sequence ended".
- **`Checked::for_drivers`**, a side table keyed by the subject's span,
  holding the `next` overload a `for` drives. The driving loop is
  synthesized, so there is no call node for the emitters to resolve — the
  choice has to be handed over.
- **`pass_elem_ty`** resolves `next` *before* `iter` in `iter_elem_ty` (a
  type with both is already a position in a sequence, so minting a second
  pass from it would be wrong), matching on the bare types since a
  subject's own `Once`/`Mut` say nothing about which `next` fits, and
  accepting only the exact `Emitted T | Finished` shape — anything else is
  some other `next`, not a driver.
- **The "not a pass" error** fires and reads well:
  ``` `Countdown` has a `next` but is not a pass: driving it uses it up, so
  it has to be declared `Once Countdown` — annotate the return type of the
  function that builds it ```

**The collision, and how it was resolved.** The error's remedy was at first
impossible: I2a restricted `Once` to function types and `Iter<T>`
deliberately, to keep a general affine qualifier as roadmap D6 — but a
hand-written pass is a *user type*, so "hand-written iterators must say
`Once`" requires `Once` on user types. Three ways out were costed (`canbe
Once`; take D6 now; or make `Once` valid on any type that has a `next`,
which would make a *type*'s legality depend on which functions are in
scope). **User decision 2026-09-07: `canbe Once`**, with the note that D6
and D7 should be designed together rather than piecemeal.

- **`canbe Once`** joins `canbe Mut` and `canbe Linear` [canbe-optin]: a
  type opts into being a pass the way it opts into mutability and
  linearity, and `Once` stays inapplicable to types that never asked. The
  author declaring it is the same argument the "not a pass" error rests on
  — an obligation should not attach on the strength of a method name.
- The position rule is now in **two halves, on purpose**:
  `types::once_position` (fn types, `Iter<T>`) and the checker's
  `has_auto_once` (the opt-in, which needs the declaration).
  `is_subtype`'s inverted `Once` rule checks *neither* — where a qualifier
  may be **written** is a different question from what it means once
  present, and an unauthorized `Once` has already been reported, so
  accepting it in the weakening direction keeps one mistake to one
  diagnostic. That is a deliberate reversal of I2a's "one predicate, no
  drift" arrangement, which stopped being possible once the answer depended
  on a declaration.

Tests: `once_tests.rs` grew to 15 — the opt-in as a valid position, the
`canbe` allowlist rejection, a hand-written pass driving a `for`, driving it
twice, and the not-a-pass error.

### I2c first half as built (2026-09-07): hand-written passes run

**`zip` works.** The driving-loop emission landed on both backends, which is
the point at which the manual half of the iterator story stops being a
declaration and becomes a feature: a struct, a `next`, and `for` drives it —
no state machine, no compiler support beyond the loop.

- **What the checker hands over** grew from a `FnKey` to a `PassDriver`: the
  overload *and* the arm identity (`emitted_arm`, `arms`). Deriving the arm
  index in each emitter instead would be precisely the checker/emitter
  disagreement the invariants forbid, and the checker already has the
  lowered return type in `pass_elem_ty`.
- **Rust**: `let mut __loop1_pass = countdown(3);` then
  `while let Union2::U1(mut n) = next(&mut __loop1_pass) {`. A `while let`
  re-evaluates its condition per turn, so `Finished` needs no arm.
- **Kotlin**: `while (true)` plus `if (step !is U2_1<Int, Finished>) { break }`,
  since Kotlin has no pattern-matching loop condition. The arm is spelled
  with its **real type arguments** when `next` is non-generic, which is what
  keeps the element read cast-free: a star-projected arm leaves `value` at
  `Any?`, and casting back warns ("unchecked cast") in code the user cannot
  edit. A generic `next` has type arguments the loop cannot see — there is no
  call node — and falls back to stars plus a cast.
- **`next` must take its state as `Mut`**, checked: the right result shape
  with a non-`Mut` state gets a diagnostic saying so, rather than falling
  through to a puzzling "not iterable".
- **An effectful `next` is a codegen error** on both backends: threading
  handlers into every turn is phase I4 [backend-never-wrong].

**Defect found and fixed: an empty struct emitted invalid Kotlin.**
`struct Finished {}` became `data class Finished()`, which kotlinc rejects
("data class must have at least one primary constructor parameter"). A
fieldless struct now emits a plain `class` [kt-struct-empty] — nothing is
lost, since with no fields there is no state for `equals`/`copy` to compare
or clone. Latent since structs were implemented; nothing had ever declared
an empty one until std's iterator protocol did.

Verified end to end with one shared program and one shared expected stdout on
both backends: a `Countdown` pass, and a `Zip` reading two lists at once
(`n 3 / n 2 / n 1 / ada is 36 / grace is 45 / done`), plus emission-shape
tests on each side and the `kt-struct-empty` regression test.

**What is left of I2c**: the representation split — a pass becoming the
generated state struct, a factory keeping the arguments it re-mints from, and
the async machinery going away. That is the `yield` half; the manual half is
done.

### Hand-written iterators must say `Once` (user decision 2026-09-07)

A hand-written state type — the point of decision 2, since `zip`/`merge`
read two sources and `yield` cannot express them — has to be drivable by
`for` like a generated one. It is **not** inferred: a value whose type has
a `next` but no `Once` qualifier is an **error** at the driving site, and
the diagnostic names the remedy — annotate the return type `Once X`.

```
struct Zip<A, B> { ... }
fn next<A, B>(z: Mut Zip<A, B>) -> [z: Mut] Emitted (A, B) | Finished { ... }

fn zip<A, B>(xs: Once Iter<A>, ys: Once Iter<B>) -> Zip<A, B> { ... }
for pair in zip(as, bs) { ... }   // ERROR: `Zip<A, B>` has a `next` but is
                                  // not a pass — return `Once Zip<A, B>`
```

Why an error rather than an inference: a struct with a `next` is not
self-evidently single-use — `next` says it can be advanced, `Once` says
advancing it uses it up, and only the author knows whether the second is
true. Inferring `Once` from the presence of `next` would attach an
obligation to someone's type on the strength of a name, and attaching
obligations silently is what [qual-*] keeps the compiler from doing. The
error is also the cheap half of the feature: it is a check at the driving
site plus a diagnostic, with the LSP surfacing the same message.

### Phases

- **I1** — ✅ Done 2026-09-07. Runtime modules extracted to real source files
  and the motivating program hand-verified on both backends; see "I1 as
  built" below.
- **I2** — Salvo-level protocol (`Emitted T | Finished`, `params Iterator`),
  `for` lowering to `while let`, simple-generator lowering. Effect-free
  producers only: pure simplification, `SalvoGen`/`SalvoYield`/`SalvoIter`
  deleted.
  - **I2a** ✅ Done 2026-09-07 — `Once Iter<T>` is a real type and the
    factory/pass distinction is checked. See "I2a as built" below.
  - **I2b** ✅ Done 2026-09-07 — the protocol in std (`Emitted`/`Finished`,
    `params Iterator<St, T>`, `next`), `for` resolving `next` before `iter`,
    the "has a `next` but is not `Once`" error, and `canbe Once`. Emission
    is not part of it: both backends reject a `for` over a pass for now.
    See "I2b as built" below.
  - **I2c** — ✅ *first half* done 2026-09-07: the driving-loop emission, so
    hand-written passes (`zip`, `merge`) run on both backends. Remaining: the
    representation split — a pass becomes the generated state struct, a
    factory keeps the arguments it re-mints from, and the async machinery is
    deleted.
- **I3** — The general flat state machine (CFG-ified body) as the
  backstop for bodies the fast path rejects.
- **I4** — Effects threaded into `next`; [iter-effect-free] removed;
  injected `close`.
- **I5** — Lazy `map`/`filter` in `seq.sv` (the eager-because-of-callbacks
  justification disappears); std surface sweep.
- **I6** — Sweep: ~48 test fns and 10 of 23 insta snapshots mention
  `Iter`/`yield`; `[fn-iterator]`, `[iter-effect-free]`, `[rs-iter-lazy]`,
  `[seq-iterable]` rewritten; the LANGUAGE.md planned-change subsection
  folded into the section proper.

### I1 as built (2026-09-07)

**I1a — the runtime modules are source files now.** The four *static*
generated modules moved out of Rust string literals in the emitters into
`crates/salvo-backend-rust/runtime/{iter.rs,strings.rs,seq.rs}` and
`crates/salvo-backend-kotlin/runtime/throw.kt`, pulled in with
`include_str!`. The parameterized generators stay generated — `unions.rs` /
`unions.kt` are a function of the arities a program needs, so there is no
static text to extract.

- **Extracted byte-for-byte, deliberately**: the files were captured from
  the compiler's own output (one scratch program touching all four),
  so emitted bytes did not change, no insta snapshot moved, and no e2e
  content stamp missed. Verified by compiling that program before and
  after and diffing the whole output tree — identical on both backends.
- **New tests** `crates/salvo-backend-{rust,kotlin}/tests/runtime_tests.rs`
  compile each module *on its own* (`rustc --crate-type lib`, `kotlinc`)
  and assert the toolchain said nothing at all: this code is spliced into
  user output, where a warning is noise the user cannot fix. They skip
  without the toolchain like every other e2e test, and they are
  content-cached the same way.
- **Negative-tested**, since a test that cannot fail is decoration: an
  unused local added to `seq.rs` made
  `every_runtime_module_compiles_warning_free` fail with the warning
  quoted, and reverting restored green.
- The list of modules is a `const` in each test, so a new runtime module
  that is not registered is a visible omission rather than an untested
  file.

**I1b — the design is hand-verified end to end.** The prototype lives in
`experiments/pull-iterators/` (with a README recording what it proves and
how to run it), since `tmp/` is scratch and these are evidence.
`lines.sv` is the motivating program, written in the *planned* language: an
effectful iterator function (`[FileSystem, Console]`) holding a resource,
releasing it with a `defer` **that itself performs effects**, with two
distinct resume points (a header `yield`, then a `yield` inside
`while true`), consumed by a `for` that `break`s after three elements.
`lines.rs` and `lines.kt` are what the emitters would produce. Both compile
warning-free and print **byte-identical stdout**:

```
opening data.txt
line -- data.txt --
line alpha
line beta
closing data.txt
done
```

What that establishes, in order of how much it was in doubt:

- **`close` can be idempotent, and that collapses the injection.** Guard
  the unwind path on the per-site `defer` flags and *one* call after the
  loop covers `break` and exhaustion alike, because both land there — no
  per-exit-path duplication, and no double-run. Verified with a second
  variant that drains instead of breaking: exactly one `closing` line,
  identical on both backends. Only `return`/`throw` out of the loop body
  need their own splice, which is machinery `defer` already has.
- **Effects thread in cleanly, and that is the whole design.** `next(&mut
  self, fs: &mut dyn FileSystem, console: &mut dyn Console)` holds no
  handler, so there is no lifetime, no `'static` bound, and nothing
  captured — which is exactly what [iter-effect-free] existed to avoid.
  A deferred block performing effects on the close path works for the same
  reason, and it is why a Rust `Drop` impl could never have been the
  mechanism (it takes no parameters).
- **The Rust output contains no `Pin`, `Future`, `Waker`,
  `Box<dyn Iterator>`, `Rc`, or `async`.** The state machine is a struct
  with a `u32` state, the body's locals as fields, and one `bool` per
  `defer` site.
- **The consumer lowering is a `while let`** on Rust
  (`while let SalvoStep::Next(l) = pass.next(fs, console)`) and the
  obvious `while (true) { … if (step !is Next) break }` on Kotlin.
- **The two backends can share one lowering** (decision 6) with the JVM
  losing nothing: Kotlin's `iterator { … }` builder could only ever have
  *captured* the handlers, so the uniform machine is not a concession on
  that side, it is the only shape that threads them.

Not yet prototyped, and still the riskiest thing in the plan: a body where
`defer`, `when`, labelled loops and a `Throw` transfer all have to be
resumable at once. I1's program has one `defer` site and one loop; **I3
should open with the gnarly shape** (`rangeIncl` with a `defer` inside a
nested `for`) before the general lowering is written.

### I2a as built (2026-09-07): the factory/pass distinction ships first

`Once Iter<T>` is now a type the compiler accepts, and the one-shot rule is
enforced — *before* the representation splits. That order is deliberate:
the semantics are the part the user decided, they are checkable today, and
landing them first makes I2c's representation change a non-event
semantically (nothing legal before it becomes illegal after).

What it took was small, because `Once` was already the right shape:

- **`types::once_position`** is the single predicate for where `Once` may
  be written — `Ty::Fn` or `Iter<T>` — read by *both* the checker's
  position check and `is_subtype`'s inverted rule, so the two cannot drift
  as D6 widens the list.
- **The position check** (check.rs, `Once` branch of the qualifier
  validation) now consults it, and its message names both positions:
  ``` `Once` applies to function types and `Iter<T>`, not `Int` ```.
- **The inverted subtyping rule** lost its `Ty::Fn` gate: `(_, Qualified
  { Once, base })` with `once_position(base)`, so plain `Iter<T>` <:
  `Once Iter<T>` exactly as plain `(A) -> B` <: `Once (A) -> B`. The
  never-drop direction needed nothing — `qual_drop_block` was already
  type-agnostic.
- **The one new rule**: a `for` whose subject type carries `Once` calls
  `fate_move(iterable, "iterate", "a `for` loop", …)` and gives the loop
  binding *no* links — the elements are owned by the loop rather than
  derived from a subject that is still alive [fate-link]. A factory keeps
  the existing behaviour (links, no consumption). The diagnostic falls out
  of the existing machinery: "`p` cannot be used here: it was consumed
  (moved) by a `for` loop".
- **The emitters needed no change at all**: `Once` erases
  [qual-erasure], and `iter_elem_ty` already went through `strip_quals`.
  Both backends compile and run a pass-returning producer, verified with
  one shared program and one shared expected stdout
  (`{rustc,kotlinc}_compiles_and_runs_a_once_iterator`,
  `pass 6 / factory 6 6 / total 6 / total 6`).

Tests: `crates/salvo-core/tests/once_tests.rs` (10) covers the position
list, both variance directions, driving a pass once, driving it twice
(consumed-use error), driving a factory twice (fine), and two calls giving
two passes; plus the two e2e tests above.

**Two diagnostic leftovers**, both quality rather than correctness:

- A pass in a factory position reports "no matching overload for
  `twice(Once Iter<Int>)`" rather than saying `Once` never drops. The
  `qual_drop_block` message exists and is used for `^` widening; the
  overload-failure path does not reach for it. This will be a common
  mistake, so it is worth a near-miss hint.
- An invalid `Once` position cascades: `let bad: Once Int = 3` reports the
  position error *and* "expected `Once Int`, found `Int`", because the
  rejected qualifier stays on the lowered type. One mistake should be one
  diagnostic — the fix is to fall back to the base type when the position
  check fails.

### Costs recorded up front

- **Recursive producers need boxing on Rust.** A nested `for` keeps the
  inner pass alive across the outer's suspensions, so it becomes a
  *field*; for a recursive producer that field has the struct's own type
  (`E0072`), hence `Option<Box<…>>`. One allocation per level per pass and
  O(depth) per element — the cost profile of chained `flatten`. Kotlin is
  unaffected. This is the one durable advantage push kept.
- **The flat machine is the one lowering whose output stops resembling its
  input**, and the interaction to distrust is `defer` + `when` + labelled
  loops + `Throw` transfers all having to be resumable in the same body.
  Hand-prototype the gnarly shape (`rangeIncl` with a `defer` inside a
  nested `for`) before committing.
- **`yield x` stops being a move.** Today it consumes ([deduce-consume],
  "a yield in a loop consumes anew every iteration"); under a pass the
  element is handed over per `next`, so the loop binding becomes a
  kept-or-moved decision the deduction engine has to make. A class of
  current errors disappears; sizing the analysis that replaces it is the
  one item that could not be bounded from reading the code.

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
  re-test (`if list is NonEmpty`); the general answer is D3, **built
  2026-09-06** as `refn` [qual-refn] — the claim's owner states what the
  call does to it, since the function cannot.
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

### D3 — Refinements ✅ Done 2026-09-06

D1's over-strictness had two candidate general answers. Analysis
2026-09-02 said they are *not* both needed, and the more obvious one does
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

**Built 2026-09-06 as `refn`** — see the decision-log entry at the top for
the six user decisions and the two things implementation revised. Every
open question this section recorded is answered:

- *Where a refinement may be declared*: in the qualifier that owns the
  claim (in scope wherever the qualifier is), or as a **top-level `refn`**,
  module-scoped and not importable [qual-refn-scope].
- *Trusted or checked*: **trusted**, like `-> T as Q` — no runtime check is
  emitted where it applies [qual-refn].
- *How refinements from several qualifiers on one function compose*: they
  merge; when they **disagree none of them apply**, with a warning
  [qual-refn-conflict], and a top-level `refn` **replaces** them for the
  parameters it names [qual-refn-reconcile].
- *Whether a refinement can strengthen a contract for callers who do not
  import the qualifier*: **no.** Visibility is per file, and a fn can only
  re-promise a qualifier its own signature names [qual-refn-infer].

Rules: [qual-refn], [qual-refn-match], [qual-refn-scope],
[qual-refn-conflict], [qual-refn-reconcile], [qual-refn-infer],
[qual-refn-docs].

### D2 — Qualifier asserts (`+Q`)

Still deferred for a *function's own* deduction list (user decision
2026-09-02: not even in the grammar there — the parser reports "adding
qualifiers in a deduction (`+Qual`) is not supported yet" and names this
item). `+Q` asserts that the body *establishes* `Q`, which is what
`-> T as Q` does for return values — the parameter-position analogue. A
predicate qualifier cannot be proven statically (that means reasoning
about the algorithm), so establishment is trust (like `as Q`) or a runtime
`qualifies` check.

**D3 gave `+Q` exactly one home, and it is not this one** (2026-09-06):
inside a `refn` [qual-refn], where it is the *qualifier author's* claim
about someone else's call rather than a claim about your own body. That
distinction is what kept D2 deferred through D3's implementation, and it
is also why an inferred deduction may only re-establish a qualifier the
parameter *declares* [qual-refn-infer] — letting inference add a new one
would have implemented D2 by the back door, without ever deciding its
establishment rule.

- **DECISION D2a** — whether `+Q` and `as Q` unify into one notion of
  "this function establishes a qualifier", and whether establishment is
  trusted, runtime-checked, or restricted to fns declared in the
  qualifier's own file (as `as Q` is today). D3's answer for refinements
  was *trusted*, which is the precedent but not the decision: a
  refinement is written by the party that owns the claim's meaning, and a
  function asserting `+Q` about its own body is not.

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

### D6 — `Once` on any type (opened 2026-09-07)

`Once` is specified as *fn-type only* ([once-fn], "the language-level
call-multiplicity qualifier"). The iterator rework generalizes it to a
**use-multiplicity** qualifier and applies it to `Iter<T>`, with the fn
case as the instance where using means calling (user decision
2026-09-07; see "Roadmap: iterators — Salvo-level pull iterators"). The
generalization was accepted; the *scope* was deliberately left narrow.

- **Open question: is `Once` valid on any type?** Nothing in its
  semantics is iterator- or fn-specific — it is an obligation-side
  qualifier the compiler owns, never droppable, with inverted variance
  ([qual-*]: permissions drop, obligations do not), and enforcement is
  the existing consumption machinery [deduce-consume]. So `Once
  FileHandle` or `Once Ticket` would already mean something coherent:
  "use this at most once".
- **Why it was not opened up in the same step** (user decision
  2026-09-07): shipping a general affine qualifier as a side effect of
  an iterator change is how a language surface grows by accident. The
  position list stays explicit — fn types and `Iter<T>` — and widens on
  demand.
- **What to weigh when it comes up**: `Once T` (at most once) sits next
  to `canbe Linear` (exactly once) and `Mut`/`ReadOnly`; a general
  `Once` makes the affine/linear pair complete and user-reachable,
  which is a bigger vocabulary decision than it looks. Also note
  `Once` is *applied* at use sites while `Linear` is *declared*
  ([linear-canbe], "linearity is declared, not applied") — a general
  `Once` would be the first obligation a user can attach to someone
  else's type.

### D7 — Qualifier-conditional linearity (opened 2026-09-07)

Today linearity is a property of a *declaration*: `canbe Linear` opts a
type in, and every value of it carries the obligation [linear-canbe]
[linear-obligation]. There is no way to say **"the qualified form carries
the obligation, the plain form does not."**

The iterator rework is the first concrete need. `Once Iter<T>` (a pass)
holds a position and may hold a resource, so it must be drained or
closed; plain `Iter<T>` (a factory) holds nothing and needs no disposal.
`canbe Linear` on the `Iter` declaration cannot express that split, since
it would burden the factory too.

- **Not blocking**: the mandatory `close` is *compiler-injected* on every
  exit out of a `for` (decision 4 of the iterator roadmap), so the
  no-leak half is covered without linearity. `Once` covers the no-replay
  half. This item is about making the obligation *visible and checked in
  the language* rather than injected.
- **What it would take**: linearity conditioned on a use-site qualifier —
  i.e. the obligation set becomes a function of the qualified type, not
  of the declaration. The existing all-paths obligation machinery
  (`owes_linear`, `check_linear_exit`, `merge_fallthrough`) would not
  change shape; what changes is which values enter it.
- **Relation to D6**: if `Once` generalizes to any type, this is the
  natural companion — the pair "at most once" (applied) and "exactly
  once" (currently declared) would want the same application mechanism.
  **Decide them together** (user, 2026-09-07: "I think we'll need to design
  the 'Once on any type' (D6) and the 'optional Linear' (D7) together").
  The 2026-09-07 iterator work took the narrow road instead — `canbe Once`,
  an opt-in on the declaration — precisely so that this pair stays a single
  deliberate design rather than an accumulation of special cases.
- **Payoff beyond iterators**: it is the general shape of
  "borrowed handle versus owned resource" without lifetimes — the same
  question `ReadOnly[from: p]` answers for derived returns [readonly-return].


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
  `intrinsic`/`external` shape. Plain `qualifier` stays state
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

## Overload resolution — **FINALIZED 2026-09-07**

Salvo overloads by argument type, and that decision reaches further than any
other single rule: `size(Str)`/`size(List)`/`size([])` are three
declarations, qualifiers make `full_name(Surname Person)` a distinct
overload, mangling exists to keep the target from re-resolving them
[kt-fn-mangling], and the checker's choice is authoritative everywhere
downstream. What had never been *designed* was the ranking; it is now, along
with the scope ladder and the two ways a caller overrides both. The rule is
at the top of this file and in LANGUAGE.md; the labelled rules are
[fn-overload] [fn-overload-scope] [fn-overload-rank]
[fn-overload-ambiguous] [fn-overload-at] [fn-rename]
[fn-overload-duplicate] [fn-value-select] [effect-member-call].

**Where the decisions came from.** Twelve probe programs against the old
implementation, each answering a question nobody had chosen: `describe(3)`
resolving to `describe<T>` (fixed as O1 on 2026-09-06); an own-module `size`
losing to std's *silently*; two identical parameter lists both declarable;
fixed-vs-variadic decided by declaration order; an unavailable-effect
candidate winning and then erroring; an overloaded fn passed by name
resolving to `entries[0]`; and effect member calls checking nothing about
their arguments. The user's answers, in the order asked:

- **Scope first, then signature** — with a *warning* where scope discarded
  the more specific signature, silenced by an explicit `@`.
- **`@` takes a module path**, not a rung keyword: `@mod` vs `@import` asks
  the reader to know which rung a name came in on.
- **The fn rung is everything a function scope holds**: fn-typed parameters,
  locals, implicit parameters, effect members.
- **Qualifier sets rank by inclusion**, kind ignored; unrankable pairs are
  settled by renaming.
- **Unions rank by arm inclusion**; broader is less specific.
- **Fixed beats variadic**, and an *empty* parameter list counts as fixed —
  so `list()` picks the no-argument overload over `list(...elems)`, which is
  what lets an "empty" case be an overload rather than a special form.
- **Identical parameter types are a declaration error**, whatever the names
  or return type say.
- **Effect availability does not filter candidates**: selection is by types,
  and a missing handler is its own diagnostic.
- **Renames apply everywhere a name resolves**, including fn values and
  implicit parameters — documented in LANGUAGE.md with examples, as asked.
- **Implicit parameters take no part in ranking**; an unresolvable one is a
  failure at the winner, not a demotion.
- **`@Effect` for members is deferred** and recorded (see the top of this
  file).

**Implementation notes worth keeping:**

- The old *sum of per-slot scores* is gone. A sum lets one argument's gain
  pay for another's loss, which is the definition of a guess; the ranking is
  a per-slot partial order (`types::spec_cmp` / `rank_cmp` /
  `most_specific`), and "no winner" is the ambiguity error.
- `FnEntry` carries its **rung** (`Core`/`Import`/`Own`) and its declaring
  module, filled where the scope is built — the rungs above `Own` are not
  overload sets, so they need no entry (a local shadows outright, a member
  takes the name first, a rename introduces a new one).
- The **lead candidate** (the source of expected types for arguments) uses
  the same rung filter and the same ranking, re-narrowed per argument. That
  is what keeps `map(arr, n -> n + 1)` working with a `List` fast path in
  scope: the `List` candidate leads until `arr` turns out to be an array.
- `Ty::Any` is now produced by lowering the written `Any` — see the top of
  this file.
- Verified end to end on both backends with one program that overrides
  resolution both ways (`@core.list`, `@main`, a dot-form `@`, a rename, and
  a call past a shadowing local): byte-identical stdout under `kotlinc` and
  `rustc`, with the emitted sources asserted to contain no `@` and no renamed
  name.

## Roadmap: standard library surface

Decisions taken 2026-09-06 (user). **S-Str and S-Seq are built** (see their
entries below and the narrative at the top of this file); **S-IO** is the one
still open, deferred by the user until IO streams are designed properly. Each
item was independently shippable, in the order given, because each fed the
next.

### S-Str — a mutable string, and the string function surface — **LANDED 2026-09-06**

See the entry at the top of this file for what the drop coercion turned out
to need. The surface as built, all in `std/core/string.sv` with lowerings in
each backend's `intrinsics.rs`:

- `intrinsic type Str canbe Mut`; `intrinsic fn mutable_str(...parts: Str[])
  [] -> [parts] Mut Str` (the parts are *kept*, since they are read rather
  than stored — which is what makes the Rust lowering borrow them).
- On `Str`: `size`, `char_at`, `iter` (→ `Iter<Char>`, which is what makes
  the S-Seq functions work over strings for nothing), `split`, `index_of`
  (`Int?`, no `-1` sentinel), `contains`, `starts_with`, `ends_with`,
  `trim`, `trim_prefix`, `trim_suffix` (unchanged when the affix is absent),
  `substr` (`Str?`), `to_upper`, `to_lower`, `join(List<Str>, Str)`,
  `parse_int` (`Int?`).
- On `Mut Str`: `append`, `set` (out of range does nothing — growing here
  would make a `set` an `append`), `clear`.
- Rule [str-drop-mut] in LANGUAGE_SPEC.md, with [kt-mut-str] and
  [rs-mut-str] in the backend specs.

Deferred deliberately, because nothing needs them yet: a `Char` → `Str`
conversion (so `set` is the only way to place a character), `replace`,
`repeat`, `pad`, and a `Char`-exact `Str` on Kotlin (see the UTF-16 note
above).

### S-Seq — `map`, `filter`, `reduce` over anything iterable — **LANDED 2026-09-06**

Built as decided (`std/core/iterable.sv` + `std/core/seq.sv`), rule
[seq-iterable]:

```
params Iterable<It, T> { fn iter(it: It) -> Iter<T> }

fn map<It, T, U>(xs: It, f: (T) -> U, ?Iterable<It, T>) -> [xs, f] Mut List<U>
fn filter<It, T>(xs: It, keep: (T) -> Bool, ?Iterable<It, T>) -> [xs, keep] Mut List<T>
fn reduce<It, T, A>(xs: It, init: A, f: (A, T) -> A, ?Iterable<It, T>) -> [xs, f] A

intrinsic fn map<T, U>(list: List<T>, f: (T) -> U) [] -> [list, f] Mut List<U>   // + filter, reduce
```

Verified on both backends with one program covering a `List` (the fast
path), an array, a `Str`, an `Iter` from an iterator function, a chain, a
named fn as the callback, non-`Copy` elements, and a customer struct made
iterable by declaring `fn iter` — byte-identical stdout under `kotlinc` and
`rustc`.

**The decided design needed four mechanisms that did not exist**, and the
memo's claim that "the generic half needs no new mechanism" was wrong on
every one of them:

- **[implicit-infer] — implicit resolution has to feed back into the call's
  type arguments.** `T` appears *only* in the implicit's type, so nothing
  bound it: the lambda was typed against an unbound `T` and `U` came out
  undeterminable [call-type-args]. Resolution now runs *between* the
  arguments, two-sided: the candidate's generics bind from the known part of
  the pattern, then the caller's variables bind from the instantiated
  candidate. This is also what fixed the recorded array gap — the `List`
  case only ever worked because `List<T>`'s own `T` did the binding.
- **[fn-overload-rank] — a *lead* candidate, re-narrowed per argument.**
  With a `List` fast path beside the generic overload, a bare lambda had no
  expected type at all (multiple candidates kept the untyped probe), which
  is the second gap the memo recorded. Expected types now come from the most
  specific candidate *still compatible with the arguments typed so far* —
  and the per-argument re-narrowing is what makes `map(arr, …)` work: the
  `List` candidate leads until `arr` turns out to be an array, and it has to
  be dropped before the lambda is typed. The lead is a hint only; the
  scoring still re-derives everything.
- **[implicit-intrinsic] — an intrinsic cannot be passed by name.** std's
  `iter` overloads *are* lowerings, so `::iter` (Kotlin) and `iter(__i0)`
  (Rust, where `iter` is the generated *module* — E0423) were both
  nonsense. The adapter closure's body is now the intrinsic's own lowering.
  While there: a resolved *declared* fn's adapter forwards each argument in
  that fn's parameter mode, or a kept struct parameter is passed by value
  (E0308) — reachable as soon as a customer type declares `fn iter`.
- **Parameter contravariance in implicit resolution.** `filter(map(xs, …),
  …)` wants `(Mut List<Int>) -> Iter<Int>` and std has `(List<Int>) ->
  Iter<Int>`; `is_subtype` compares fn parameters *invariantly*, so it did
  not fit. [implicit-resolve] already documented contravariance — the
  implementation just did not do it. Fixed there rather than in `is_subtype`,
  deliberately: a backend renders a parameter's convention from its declared
  type, so general fn-value contravariance would let a `&Vec` callback reach
  a `&mut Vec` position.

**And the Rust `List` fast path could not be an inline expression.** Every
shape that splices the callback into an expression hits Rust closure
inference: a closure bound to a `let` cannot infer its parameter types, and
neither can one nested inside another closure's argument
(`filter(|__x| (|n| *n > 1)(*__x))` — E0282). The fast paths therefore lower
to generated helpers in `seq.rs` ([rs-seq], gated and mounted exactly like
`iter.rs` and `strings.rs`): a generic parameter *is* an expected type, and
it also pins the callback convention `FnMut(&T)` that a declared `(T) -> U`
renders as. `salvo_reduce` is a loop rather than `Iterator::fold`, because
that convention borrows the accumulator and `fold` passes it by value.

Everything else landed as decided: eager `Mut List` results, the group in
its own implicitly visible `core.iterable`, and the identity
`intrinsic fn iter<T>(it: Iter<T>)` that makes an `Iter` iterable and chains
compose.

Deferred (nothing needs them yet): `any`/`all`/`find`/`count`/`zip`/`flat_map`
— the same shape, one more overload pair each — and a lazy `map` for the
effect-free case, which would need a way to say "this callback performs
nothing" at the type level rather than by convention.

### S-IO — streams, then the filesystem (deferred by decision)

An `Fs` effect was designed in outline (effect + `intrinsic handler
DefaultFs`, a `File` struct, linear `InputStream`/`OutputStream` as
`intrinsic type … canbe Linear`, errors in the return type because
[effect-member-no-effects] forbids a member from declaring `[Throw<M>]`).
**Deferred by the user 2026-09-06**: IO *streams* should be designed
properly first, with the filesystem as their first customer, rather than
the other way round. Two findings from the outline worth keeping for when
it resumes:

- Errors cannot use `Throw` at all — an effect member may not declare
  effects — so every fallible member returns `Ok T | Err Str`. That makes
  `Ok InputStream | Err Str` the normal shape, and a **linear value inside
  a union arm** the interaction to verify first ([linear-composite] says
  composites are contagious, but nothing exercises it).
- Whether stream operations are *members* of the effect or free
  `intrinsic fn`s is a testability question, not a plumbing one: only
  members can be faked by a double.

## Test inventory (all green: 672)

The kotlinc/rustc tests are **content-cached** (`salvo-testkit`): a plain
`cargo test` still runs every one of them, but only recompiles the ones whose
generated code, expected output or toolchain actually changed. Use
`SALVO_E2E_FRESH=1 cargo test` for a run that takes nothing from the cache,
and `cargo nextest run` when you want to see which tests cost what.

- `salvo-core`: 266 - 18 unit tests (file classification, including the
  `platform/` strip [platform-tree]; `types.rs` union
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
  + 15 type-argument tests (`tests/type_arg_tests.rs` [call-type-args]:
  an undetermined type argument reported with both remedies; determined by
  the arguments, by an explicit list, by a `let` annotation, by the
  enclosing return type, and by a *concrete* parameter of a nested call;
  a *generic* parameter determining nothing, so the nested call is still
  reported; a type argument confined to the parameters needing no context;
  an unknown-typed argument keeping the call lenient so one mistake yields
  one diagnostic [type-unknown-lenient]; and the resolved bindings recorded
  per call, which is what a backend's intrinsic lowering renders
  [backend-intrinsic]; plus 5 progressive-binding tests
  [call-generic-progressive]: an earlier argument typing a later
  un-annotated lambda, the lambda's *body* determining the result type
  argument, an explicit type-argument list doing the same, an annotated
  parameter needing no binding in any position, and the deliberate
  left-to-right limit — a lambda before its binding argument is not
  inferred)
  + 9 widening tests (`tests/widen_tests.rs` [qual-widen]: a `^` branch head
  opening a nested union, `^ Mut` stripping in an `if` with the mutation it
  then rejects, the intrinsic qualifiers refused with their reasons from the
  shared exclusion list, nothing-to-remove rejected, a *type* on the right
  rejected, more than one arm rejected, the no-binding parse error, and a
  `^` branch consuming its arms so exhaustiveness still reports the rest)
  + 25 implicit-parameter tests (`tests/implicit_tests.rs` [implicit-param]
  [implicit-group] [implicit-resolve] [implicit-forward] [implicit-override]
  [implicit-fn-only]: a group's members and a written `?cmp` resolved from the
  visible overloads; an unresolvable one reporting *both* remedies; one member
  overridden by name and by lambda; a named argument matching nothing, and one
  of the wrong type, rejected; forwarding through a generic fn, and forwarding
  *across groupings* — a `?Field<T>` spread filling an individually declared
  `?add`; a generic fn without the implicit unable to call one that needs it,
  which is the colouring cost; and the declaration-site rules — non-fn type,
  unknown group, wrong group arity, two implicits of one name, and a group
  member with a body; an effect member *may* declare them, and its call may
  override one, while a handler constructor may not [implicit-fn-only]; and
  the three diagnostics a printed type cannot carry — a contract mismatch
  explained for a resolved default and for a call-site override, and an
  ambiguity that says how many matched); plus 4 handler-generics tests
  [effect-handler-generics]: a `use` writing its handler's type arguments, a
  constructor argument still binding them, the two disagreeing reported, and
  the wrong number of them rejected)
  + 14 fn-type-effect tests (`tests/fn_effect_tests.rs` [fn-effects]: a
  declared effect available in a lambda body while an undeclared one is
  rejected even with the effect in lexical scope; a fn *inheriting* its
  fn-typed parameters' effects, through a qualifier too, with its caller
  required to supply them; variance both ways (a pure fn and a pure lambda
  fitting an effectful position, an effectful fn rejected by a pure one); an
  un-annotated lambda's effects inferred from its body; each declared effect
  required at the call, and a fn value called where its effect is
  unavailable rejected — the reason "forbid escape" was unnecessary; and
  `use` in a fn type rejected; plus 3 iterator tests [iter-effect-free]: an
  iterator fn declaring an effect rejected with its remedy, the same for
  `use` (the hole that would let the body register its own handler), and a
  `for` loop over an iterator performing effects freely — the restriction is
  on producing, not consuming)
  + 20 throw/`try` tests (`tests/throw_tests.rs` [throw] [try]: the
  outcome type read off an annotation mismatch (`Ok Int | Thrown Str`),
  several message types unioning (`Thrown (Str | Int)`), an always-leaving
  body still carrying `Ok None`, a `try` that cannot throw rejected, an
  throw with nowhere to land rejected while declaring the effect
  propagates, a message the target cannot carry rejected, `main` declaring
  `Throw` rejected [throw-not-main], a handler *for* `Throw` rejected, an
  throwing branch counting as returning [fn-must-return] and not leaking
  its consumption to the fall-through path [type-any-nothing], a linear
  value across a may-throw call rejected with the `defer` remedy accepted
  [throw-linear], `throw` *and* a may-throw call inside a deferred block
  rejected [defer-no-escape], an inner delimiter taking only its own
  throws [try-innermost]; plus the two nested-qualification shapes the
  design asked to test rather than assume — a nested result outcome and a
  union message, both taken apart through a binding at the inner type — and
  the diagnostic that names that remedy when a qualified union is matched
  directly [when-union-subject]; and an assignment inside a `try` body
  resetting an earlier `is` narrowing, with the no-assignment control —
  behavioural, since flow-sensitive checking is what achieves it rather
  than the syntactic assigned-name scan)
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
  + 9 platform-effect tests (`tests/platform_tests.rs` [platform-effect]
  [effect-member-unique]: a platform effect performed like any other effect
  with no handler anywhere; a Salvo handler *depending* on one (which is how
  Salvo-written handlers reach the host); and the restrictions — a Salvo
  `handler ... of` a platform effect rejected with the ordinary-`effect`
  remedy, a generic platform effect and a generic member each rejected, two
  members of one effect sharing a name, the same name across two effects
  (reported *once*, at the second declaration), and distinct names across
  effects staying legal).
  + 17 refinement tests (`tests/refine_tests.rs` [qual-refn]
  [qual-refn-match] [qual-refn-scope] [qual-refn-conflict]
  [qual-refn-reconcile] [qual-refn-infer], with *overload resolution* as
  the observable — a `NonEmpty` overload resolves only while the checker
  still believes the claim): a refinement re-establishing what a mutating
  call's exhaustive list dropped, with the no-refinement control proving
  the acceptance is the refinement's doing; `-Q` invalidating a claim a
  call would otherwise have kept; conflicting refinements all standing
  down *with a warning* while compatible ones (`with`-declared) both
  apply; a top-level `refn` reconciling the conflict by replacing them; a
  refinement in scope only with its qualifier, and a top-level one not
  leaving its module (same module, second file: applies; another module
  importing everything it can: does not); and the declaration rules —
  four ways to miss an overload (wrong parameter name, wrong type, no such
  function, no such parameter), an unbound type parameter reported as
  itself with the `refn add<T>` remedy, a qualifier refining someone
  else's claim, provenance and intrinsic qualifiers refused, a *moved*
  parameter having nothing to refine, and a qualifier that does not apply
  to the parameter's type; plus the three inference facts — a refinement
  reaching an inferred deduction, a written list allowed to promise the
  refined qualifier (with the no-refinement control rejected by
  [deduce-infer]), and a *conditional* refined call **not** reaching the
  contract).
- **22 overload-resolution tests** (`tests/overload_tests.rs` [fn-overload]
  [fn-overload-scope] [fn-overload-rank] [fn-overload-ambiguous]
  [fn-overload-at] [fn-rename] [fn-overload-duplicate] [fn-value-select]:
  the ranking rung by rung — concrete over a type variable in *both*
  declaration orders, a narrower union (and the caller-knowledge limit: an
  `Int | Str` value not fitting `f(Int)` until it is narrowed), `T` over
  `T?`, `Any` last *and* accepting everything, qualifier sets by inclusion
  with the unrankable pair reported, fixed over variadic including the
  no-argument case, and per-slot dominance so one slot never pays for
  another; the ladder — this module over core, core→import→module in order,
  and the scope-override *warning* naming both `@` forms; `@module` picking
  a module's overload, erroring when that module has none, reaching past a
  local of the same name, and working in dot form; fn values selected by the
  expected type and reported as ambiguous without one; renames settling an
  ambiguity, scoped to their block, refusing a taken name, a mismatched
  parameter list and a `@module` on top; and a duplicate parameter list
  reported as a duplicate while differing types stay an overload set).
- **12 `Mut Str` tests** (`tests/str_tests.rs` [str-drop-mut]
  [type-canbe-mut]: `Str canbe Mut` while `Mut Int` is still an error, and a
  literal is not a builder; a recorded drop at every site — call argument
  (a declared fn and an intrinsic), `let` annotation, `return`, struct
  field, union arm (with the displaced `WrapUnion` asserted to survive as
  the drop's continuation), interpolation, `==` and `+`; and no drop where
  the target keeps `Mut` — a `Mut Str` parameter, an optional `Mut Str?`, a
  generic position (which is what makes `copy(builder)` a builder) — nor
  for a plain `Str`).
- **9 sequence-function tests** (`tests/seq_tests.rs` [seq-iterable]
  [implicit-infer] [fn-overload-rank]: everything inferred for a `List`,
  an **array** (the recorded gap) and a `Str` subject — with a `Char`
  element proved by rejecting a `Str` operation on it; iterators and chains
  composing through the identity `iter`; a customer struct made iterable by
  declaring `fn iter`; a subject with no `iter` reported as the missing
  implicit; and the *selection* facts — a `List` subject resolving to the
  intrinsic fast path, every other subject to the generic body, and the lead
  candidate narrowing before the lambda is typed).
- `salvo-cli`: 80 - 47 `analyze` integration tests running the built
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
  bodyless-declaration explicitness ([decl-explicit]: an effect member
  missing the two parts that apply to it and *not* asked for effects, an
  effect member's declared deductions enforced at the call site, and std's
  `add` consuming its element),
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
  define files into the analysis, unknown backend rejected;
  and refinements over the *real* std [qual-refn]: a refinement recovering
  the `NonEmpty` std's `add` necessarily strips, with the no-refinement
  control failing overload resolution, and a conflict warning that leaves
  the exit code 0 and renders as `"severity": "warning"` in the JSON
  [qual-refn-conflict]) + 2 UTF-16
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
  a refinement's docs merged into the refined fn's hover as a
  **Refinements** section naming the effective entry and the qualifier it
  came from [qual-refn-docs],
  an effect member at its declaration and at a call, handler state, and a
  handler member whose `[symbol]` references reach the handler's own state
  — plus go-to-definition on a field access [lsp-definition])
  + 3 grammar tests
  (`src/lang.rs` [cli-lang]: highlighting categories exactly partition
  the lexer's keyword table, generated grammar is valid JSON containing
  every keyword, checked-in VS Code grammar matches the generated one)
  + 17 `run` tests (`tests/run_tests.rs` [cli-run], each in its own
  working directory so the default `--target` lands in the sandbox): the
  same program compiled and run on *both* backends with identical asserted
  stdout; `--main` alone implying its directory as the source root;
  `--src` and `--main` *together* reaching a nested entry point on both
  backends, with the `--main`-alone control failing on the unresolved
  import that proves the combination is not redundant; an entry outside
  `--src` rejected; `--main` choosing between two entry points in one
  directory on both backends —
  which is the [rs-crate] regression, since the emitter used to pick the
  first `main` it found and the other choice built a module without the
  `mod` declarations; the program's exit code and stderr reaching the
  caller (a Rust panic, so nonzero rather than a fixed code); the two
  `--clean-target` modes and the fact that both clear the target first;
  and the validation that needs no toolchain — a target that is, contains,
  or sits visibly inside the sources (each with the sources asserted
  intact afterwards), at least one of `--src`/`--main` required, a
  required and validated `--backend`, a missing `main` from either
  direction, a define file rejected as an entry point, a check error
  stopping the run with a single un-double-prefixed diagnostic, and the
  deletion guard refusing a target that holds a `.sv` file;
  and — per backend — a *warning* reaching the builder without failing the
  run [qual-refn-conflict] [diag-structured]: the program's stdout asserted
  (so it really ran), the exit code 0, and the rendered warning with its
  location asserted on stderr. Either half alone would be a bug — an abort
  rejects a legal program, and silence leaves the diagnostic visible only in
  `salvo analyze`).
  + 5 `platform generate` tests (`tests/platform_tests.rs` [cli-platform]
  [platform-tree]): the whole arc per backend — the run failing with an
  error that names the command and the path, the command writing
  `platform/main.<ext>`, the stub implemented, and the *same* `salvo run`
  then printing identical stdout on both backends; a second generate
  leaving an edited host byte-identical and saying so; both backends'
  hosts coexisting in one tree; a program without platform effects
  generating nothing; and a nested layout where the effect's module gets
  the implementation and the entry's module (chosen with `--main`) gets the
  `main`, each mirroring its own source path, with the cross-module
  reference qualified as `crate::platform_telemetry::TelemetryHost`.
- `salvo-syntax`: 70 (three parser tests for the scope selector and
  `rename` [fn-overload-at] [fn-rename]: `@` on a name, a dot call and a
  value, the placement error, module- and statement-level renames, and the
  four things a rename may not repeat; two std snapshots for `core.iterable`
  and
  `core.seq` [implicit-group]; three refinement parser tests [qual-refn]:
  a refinement in a qualifier body with its docs and its `+`/`-` entries, a
  top-level `refn` as an item of its own, and the four things a refinement
  may not say — effects, a return type, an unsigned qualifier, a missing
  deduction list — each a diagnostic naming the reason. Six
  implicit-parameter parser tests were added
  [implicit-param] [implicit-group] [implicit-override]: `?cmp:` and
  `?Field<T>` parsing side by side — the group spread is *not* a parameter —
  a `params` group of bodiless members, a `name = value` argument recognised
  by the `=` that follows an identifier, and the two ordering rejections, an
  ordinary parameter after an implicit and a positional argument after a
  named one) (the std *define-file* snapshot tests were deleted
  with the define files themselves, as were the `defines.kotlin.sv` corpus
  file and its snapshot, and `imports_externals.sv` became `imports.sv`;
  six declaration-form tests were added — [decl-body]: a bodiless top-level
  `fn` erroring with `platform effect` named, a bodiless non-alias `type`,
  a `canbe` clause not counting as a definition, and the two defining forms
  (`= alias`, `intrinsic`) *not* erroring; plus `external` and `define`
  asserted to be ordinary identifiers now, so each of their three old forms
  fails with a plain "expected item"; three `platform effect` parser tests
  were added [platform-effect]: the flag is set by the modifier, a plain
  `effect` leaves it clear so nothing existing changed meaning, and
  `platform type` / `platform fn` are parse errors naming the form) (the
  corpus grew three LANGUAGE.md examples with E3: a
  `defer` in `control_flow.sv`, `throw`/`try` in `effects.sv`, an effectful
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
  docs surviving `external`/`intrinsic`/`provenance` modifiers, and
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
- `salvo-backend-kotlin`: 128 - golden snapshots of the M2 demo, the M3
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
  [kt-copy] [intrinsic-fn]) and a negative test (`copy` of nested
  mutability is a codegen error);
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
  type-argument assertions ([call-type-args] [backend-intrinsic]: std's
  list constructors carrying their element type from a `let` annotation and
  from an explicit type argument — `mutableListOf<Int>()`, which is the form
  kotlinc requires);
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
  and 2 throw tests ([throw] [try] [kt-throw-signal]:
  `throw_lowers_to_a_signal_and_try_to_a_catch` asserting the generated
  stack-trace-less signal, a tagged `throw`, a *plain* call in the
  propagating frame (no colouring), the delimiter's tag dispatch with its
  rethrow fallback, and no interface emitted for the effect; plus the
  kotlinc run of the throw demo, whose stdout matches the Rust run byte for
  byte);
  and 2 `defer` tests ([defer] [kt-defer-finally]:
  `defer_lowers_to_try_finally` asserting one `try` per `defer`, nested
  latest-first, and a single `finally` covering both `return`s of a
  two-exit fn; plus the kotlinc run of the defer demo — LIFO at a block
  end, an early `return`, `continue`/`break` out of a loop body, and a
  linear handle released on both paths — whose stdout matches the Rust
  run byte for byte);
  and 2 overload-dispatch tests ([kt-fn-mangling] [fn-overload]:
  `every_emitted_overload_gets_its_own_kotlin_name` asserting the second
  overload is renamed and the delegation reaches its sibling; plus the
  kotlinc run of a `List`→`Iter` delegation, which before the rule
  recursed until the stack ran out);
  and 2 implicit-parameter tests ([implicit-param] [implicit-group]:
  `implicit_parameters_lower_to_trailing_fn_parameters` asserting the trailing
  fn-typed parameters, `::add`/`::times` references at the call sites and *no*
  class for the group; plus the kotlinc run of the same source and stdout the
  Rust backend asserts);
  and 2 effect-member-implicit tests ([implicit-param]: the interface method,
  every handler's `override` and the member call all carrying the member's
  implicits; plus the kotlinc run of the shared demo);
  and 2 lazy-iterator tests ([fn-iterator]:
  `an_iterator_fn_lowers_to_a_lazy_iterable` pinning the builder that was
  already lazy; plus the kotlinc run of the *same source* the Rust backend
  runs, asserting the *same stdout* — which is the parity claim itself, not
  a Kotlin property)
  and 2 subject-less `when` tests ([when-condition] [kt-when-cond]:
  `a_subjectless_when_emits_a_subjectless_kotlin_when` asserting the
  Kotlin `when {` with `cond ->` arms, a plain `else ->` with no optional
  filler, and the `is` binding declared inside its arm; plus the kotlinc
  run of the demo — value and statement position, `is` heads, a chain
  returning from every branch, and one nested in a subject `when`'s arm —
  whose stdout matches the Rust run byte for byte)
  and 2 `try`-body tests ([try]: a variable assigned *only* inside a `try`
  body declared `var` — the traversal gap that emitted `val` and had
  kotlinc reject the output — plus the kotlinc run of the same program).
  and 2 refinement tests ([qual-refn] [qual-erasure]
  [qual-refn-conflict]: `kotlinc_compiles_and_runs_a_refined_program` — the
  refined program runs, and the emitted `main` contains no `qualifies` call,
  since a refinement is trusted rather than checked; same source and same
  asserted stdout as the Rust backend, which is the parity claim itself; and
  a suppressed conflict still *emitting*, since a warning may not stop
  codegen);
  and 4 platform tests ([platform-effect] [kt-platform-entry]
  [platform-tree] [kt-platform-host]:
  `platform_effect_emits_an_interface_and_a_host_entry` asserting the
  generated `interface`, the *absence* of a handler class, `salvoMain`
  taking the instance, no generated `fun main(`, and the effect threaded
  into an intermediate frame; `platform_generate_renders_a_host_skeleton`
  asserting the skeleton's path, its own `salvo.platform.main` package, the
  import, the `TelemetryHost : Telemetry` class, the stubbed `override` and
  the host `main`; `a_missing_host_file_names_the_command` asserting that a
  platform program without a host does not emit and that the error carries
  both the path and the command; plus a kotlinc compile+run of the
  *generated* skeleton with only its `TODO` body replaced — so the test
  proves the skeleton is right everywhere else — whose stdout matches the
  Rust run byte for byte. The `run_kotlin_entry` helper exists because
  `run_kotlin_files` hardcodes `salvo.main.MainKt`, and the entry here is
  the host's `salvo.platform.main.MainKt`); and 1 overload-specificity test
  ([fn-overload-rank]: `kotlinc_runs_the_most_specific_overload` —
  `concrete` then `generic`, with the generic overload declared first); and
  4 string tests ([kt-mut-str] [str-drop-mut] [fn-variadic]:
  `mut_str_lowers_to_a_string_builder` asserting the `StringBuilder`
  construction, `.toString()` at a call argument, in interpolation and at
  `==`, `StringBuilder(b)` for `copy`, and `set`'s guarded `setCharAt`;
  `a_spread_into_a_variadic_intrinsic_spreads` asserting Kotlin's own spread
  operator — the case that printed `[Ljava.lang.String;@…` before; plus
  kotlinc runs of the whole string surface and of `set` through a parameter
  and a field, both asserting the stdout the Rust backend asserts); and 2
  overload-override tests ([fn-overload-at] [fn-rename]:
  `scope_selectors_and_renames_are_erased` asserting that no `@` and no
  renamed name reaches Kotlin, that `@core.list` emits std's lowering, that
  the renamed overload is called by its declaration's mangled name, and that
  a call past a shadowing local needs nothing here (separate namespaces);
  plus the kotlinc run of that program); and 2 sequence tests ([kt-seq] [implicit-group] [implicit-intrinsic]:
  `sequence_functions_lower_to_collection_operations` asserting
  `.map{}.toMutableList()`, `.filter{}.toMutableList()`, `.fold(init, op)`,
  the adapter lambda an intrinsic `iter` becomes, and that nothing *declares*
  `Iterable`; plus the kotlinc run of the seven-subject demo).
- `salvo-backend-rust`: 102 - golden snapshots of the same five demos
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
  type-argument assertions ([call-type-args] [backend-intrinsic]:
  `Vec::<i32>::new()` from an annotation and from an explicit type argument,
  with a rustc run);
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
  throw tests ([throw] [try] [rs-throw-controlflow] [rs-try-label]:
  `throw_lowers_to_controlflow` asserting the `ControlFlow<M, T>` return
  shape, `throw` as a `Break` return, `Continue`-wrapped returns, no trait
  for the effect, and the deferred release on the throw path of a
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
  demo Kotlin also runs, with the same stdout); and 1 `try`-body test
  ([try]: the rustc run of the program whose `try` body assigns an outer
  variable and declares a local — the Rust half of the same traversal
  gap); and 2 generic-higher-order tests ([rs-fn-param-convention]
  [fn-contract]: `a_lambda_binds_a_generic_fn_parameter_by_reference`
  asserting the declared `FnMut(&T)` convention and the `|n: &i32|` /
  `|s: &String|` annotations that now follow it; plus the rustc run of a
  generic `map` in every argument form — bare lambda, annotated lambda and
  named fn, over a `Copy` and a non-`Copy` element type — which before the
  fix failed with `E0631` for the annotated forms only); and 3
  implicit-parameter tests ([implicit-param] [implicit-group]
  [backend-never-wrong]: `implicit_parameters_lower_to_trailing_fn_arguments`
  asserting the expanded trailing parameters, the adapter closure a resolved
  default is wrapped in, the `&mut *add` reborrow forwarding uses and no
  struct for the group; the rustc run of the same source and stdout Kotlin
  asserts; and a function in a struct field reported as a codegen error naming
  the implicit-parameter remedy rather than emitted as `impl Trait` in a field
  position); and 2 effect-member-implicit tests ([implicit-param]: the trait
  method, its implementation and the call site rendered from one helper, with
  `dyn` for object safety; plus the rustc run of the shared demo); and 3
  lazy-iterator
  tests ([rs-iter-lazy] [fn-iterator] [iter-effect-free]:
  `an_iterator_fn_lowers_to_a_lazy_factory` asserting the factory, the
  `async` body, the slot-and-suspend `yield`, the absence of the old
  `__yielded` collection and the generated-and-mounted `iter.rs`;
  `a_for_loop_borrows_an_iter_subject`, the pairing that makes a second pass
  possible; plus the rustc run of an *unbounded* producer that terminates
  because the consumer `break`s and a factory consumed twice — the same
  source and stdout the Kotlin backend asserts); and 2 refinement tests
  ([qual-refn] [qual-erasure] [qual-refn-conflict]:
  `rustc_compiles_and_runs_a_refined_program`, the same source and stdout the
  Kotlin backend asserts, with no `qualifies` call in the emitted `main`; and
  a suppressed conflict still emitting and returning its rendered warning
  through `emit_program_reporting`); and 4 platform tests
  ([platform-effect] [rs-platform-entry] [platform-tree]
  [rs-platform-host]:
  `platform_effect_emits_a_trait_and_a_host_entry` asserting the generated
  `trait`, the *absence* of a handler struct, `salvo_main` taking
  `&mut dyn`, the effect threaded into an intermediate frame, and the
  crate-root wiring — the `#[path]` mount of `platform/main.rs` as
  `platform_main` plus the `fn main()` that delegates to it;
  `platform_generate_renders_a_host_skeleton` asserting the skeleton's
  path, the unit struct, `impl crate::Telemetry`, the stubbed member and
  the `main` that calls `crate::salvo_main`;
  `a_missing_host_file_names_the_command` asserting that a platform program
  without a host does not emit and that the error carries both the path and
  the command; plus the rustc compile+run of the *generated* skeleton with
  only its `todo!` body replaced, asserting the same stdout Kotlin does);
  and 1 overload-specificity test ([fn-overload-rank]:
  `rustc_runs_the_most_specific_overload`, the generic overload declared
  first and the concrete one still chosen — same source and stdout as the
  Kotlin backend's `kotlinc_runs_the_most_specific_overload`, since the
  winner is the *checker's* choice and the two targets must agree on it);
  and 4 string tests ([rs-mut-str] [str-drop-mut] [fn-variadic]:
  `mut_str_is_a_plain_string` asserting that a drop renders *nothing*, that
  `mutable_str` borrows its parts rather than moving them, the
  byte-to-character correction in `index_of`, `set` reaching the generated
  trait, the `strings.rs` mount and import, and that the support file is
  absent when nothing needs it;
  `a_spread_into_a_variadic_intrinsic_is_the_collection`; plus rustc runs of
  the whole string surface and of `set` through a `&mut String` parameter
  and a field projection — the case the trait exists for — each asserting
  the stdout Kotlin asserts); and 2 overload-override tests
  ([fn-overload-at] [fn-rename] [rs-shadowed-call]: the same program as
  Kotlin's, asserting the erasure, the std lowering behind `@core.list`, the
  renamed overload's mangled name, and `crate::describe(7)` for the call
  past a shadowing local — E0618 without it; plus the rustc run asserting
  the stdout Kotlin asserts); and 2 sequence tests ([rs-seq]
  [implicit-intrinsic]: `sequence_functions_lower_to_helpers` asserting the
  `salvo_map`/`salvo_filter`/`salvo_reduce` calls with their `&place[..]`
  receivers, the adapter a *named fn* callback wraps in, the intrinsic
  lowering inside the implicit's adapter closure, and `seq.rs` present only
  when something needs it; plus the rustc run of the same seven-subject demo,
  asserting the stdout Kotlin asserts).

When intentionally changing std, the parser AST, the checker's lowering, or
the emitter output, rerun with `INSTA_UPDATE=always` and review the
snapshot diffs.

## Gotchas / lessons learned

- **Adding a file to `std/` does not always reach the CLI.** The embedded
  standard library is `include_dir!("$CARGO_MANIFEST_DIR/../../std")` in
  `salvo-cli/src/analysis.rs`, and there is no `build.rs` emitting
  `rerun-if-changed` for that tree — so a *new* `.sv` file can leave the
  binary compiled against the old set. The tell is the file count in the
  CLI's own output ("analyzed 12 file(s) (1 user, 11 std)"): if it did not
  go up, touch `analysis.rs` and rebuild. Found while adding
  `std/core/iterator.sv` (2026-09-07); editing an *existing* std file is
  fine, since its content is part of the macro's input.

- **Probe the *current* behaviour before designing the rule.** The overload
  agenda was worth more than the design discussion that followed it: twelve
  five-line programs answered questions nobody had chosen an answer to —
  fixed-vs-variadic by declaration order, an own-module fn losing to std's
  *silently*, effect member calls checking nothing. Writing the options down
  from the code would have missed all three, because the code looked
  reasonable; only running it showed what it did.
- **A ranking that sums per-argument scores is a guess in disguise.** The old
  scoring added +2/+1/+4 per argument and picked the maximum, so a candidate
  could win by being much better in one argument and worse in another. Nobody
  noticed until the rule was written down as "at least as specific in every
  argument", at which point the sum was obviously the wrong shape. When a
  comparison is a *lattice*, implement the lattice, not a scalar projection
  of it.
- **A type declared in std is not the same as a type the compiler knows.**
  `Any` was `intrinsic type Any` and lowered to a nominal `Named("Any")`, so
  `f(v: Any)` accepted *nothing*: unification compares names, and no argument
  is named `Any`. It had been that way for as long as `Any` existed, hidden
  because nobody wrote an `Any` parameter. If a type has language-level
  meaning ([type-any-nothing]), the lowering has to say so — the std
  declaration only gives it a name.
- **Making something reachable creates new emission cases.** `@module` let a
  call reach a function shadowed by a local, which had been *unreachable*
  before — and immediately produced E0618 on Rust, where functions and locals
  share a namespace. A feature that removes a restriction should be followed
  by the question "what did the restriction make impossible in the output?"

- **A "free" widening can stop being free when a backend disagrees.**
  `Mut T <: T` had been one line in `is_subtype` because the only
  `canbe Mut` type mapped to a Kotlin *subtype*. `Str canbe Mut` broke that
  in the direction that shows least: equality. `sb1 == sb2` compiles, runs,
  and answers `false` where Rust answers `true`. When adding a `canbe Mut`
  type, ask what the *drop* costs on each backend before asking what the
  mutators cost [str-drop-mut].
- **One expression, one coercion slot — so a new coercion has to say what
  it displaces.** `DropMut` was first recorded by *removing* whatever was
  already at that span, which quietly discarded the union wrap a `Mut Str`
  needs on its way into a `Str | Int`. The `then` field is not extra
  generality: without it the emitters would have to re-derive the wrap.
- **Rust closure inference decides the shape of a lowering.** A closure
  bound to a `let` cannot infer its parameter types, and neither can one
  nested inside another closure's argument — so of all the ways to splice a
  callback into an expression, *none* works without an annotation the
  emitter does not have. A generic function parameter is an expected type,
  which is why the sequence fast paths are generated helpers [rs-seq]. The
  general rule: when a lowering has to *call* a value the program supplied,
  give it a typed home rather than an inline one.
- **"No new mechanism needed" is a hypothesis, not a finding.** The S-Seq
  memo said the generic half worked with today's machinery, having probed
  the shape by hand; building it needed four new mechanisms (see the
  roadmap entry). What the probe had actually shown was that the *syntax*
  parsed and one hand-written case checked. Probe with the case you intend
  to ship — here, a bare lambda over a non-`List` subject with the fast
  path present.
- **A generated support module beats a clever inline lowering.** Rust's
  `set` wants the string read and written; every inline form either
  splices the receiver twice (evaluating a call argument twice) or fails
  for one place shape — `&mut place` is E0596 when the place is a `&mut
  String` parameter, since that binding is not `mut`. A trait method in a
  generated file (`strings.rs`, gated like `iter.rs`) auto-refs every
  shape and mentions the receiver once. The precedent is worth reusing:
  when a lowering needs a *statement*, generate a helper instead of
  building an expression that pretends otherwise.
- **A new tie-break needs a leniency audit before it needs tests.**
  Overload specificity [fn-overload-rank] was correct on the case it
  was written for and immediately wrong on `size(xs)` where `xs` had *no
  inferred type*: an un-inferred argument fits every candidate, so a
  perfectly ordinary fate error grew a second, bogus "ambiguous call"
  beside it. Any rule that turns "several candidates match" into an error
  has to ask first whether they match *because the checker knows nothing*
  [type-unknown-lenient]. The suite caught it — the two CLI `analyze`
  tests that assert whole stderr text — which is the argument for keeping
  full-output assertions somewhere.
- **Rank the declared patterns, not the substituted ones.** The reason
  `describe<T>(T)` beat `describe(Int)` for `describe(3)` is that by the
  time the candidates are scored, `T` has been substituted to `Int` and
  the two candidates look *identical* (both "exact match", both zero
  qualifiers). Specificity is a property of the signature as written, so
  it must be read from `patterns`, before `substitute_vars`.

- **A remedy the user names has to be *checked* against the mechanism.**
  Refinements were designed with "reconcile it in a top-level `refn`" as
  the escape from a conflict, and the first implementation *merged* every
  refinement in scope — under which the reconciliation joins the
  disagreement it was supposed to settle, and changes nothing. The remedy
  only works because a top-level `refn` **replaces** the qualifiers'
  refinements [qual-refn-reconcile]. When a design records an escape
  hatch, write the program that uses it before believing the mechanism
  provides it.
- **A fact-*adding* rule cannot be dropped into a meet-shaped analysis.**
  `deduce.rs` is order-insensitive and terminating *because* removals only
  accumulate. A refinement's `+Q` is the opposite direction, so
  `if c { add(list, x) }` would have let a signature promise a fact that
  holds on one path — sound-looking, because the *call site* really does
  narrow there, and the call site is flow-sensitive while the walk is not.
  Two restrictions fixed it [qual-refn-infer], and the second (additions
  only for qualifiers the parameter declares) also keeps the lattice
  bounded, so the fixpoint still terminates. Before adding a monotone
  fact, ask which direction the existing analysis is monotone *in*.
- **A restriction can be what makes a feature backend-free.** "State
  qualifiers only" reads like a scoping decision about which claims a
  refinement may make. It is the reason the whole feature needed *no*
  emitter work: `Mut` is the one qualifier that is not erased, so `+Mut`
  would have turned a compile-time statement into an emission feature. The
  test that pins it asserts the emitted `main` contains no `qualifies`
  call — a refinement is trusted, so nothing may be emitted for it.
- **"None of them apply" needs a granularity, and per-call is the wrong
  one.** Suppressing every refinement of a *call* because two qualifiers
  disagree about one parameter costs refinements that never disagreed. Per
  (callee, parameter) is the unit, which is also the unit the warning
  deduplicates on.

- **A silent skip is a bug waiting for a deletion.** `SourceSet::classify`
  returned `None` for a file belonging to another backend, and `add_dir`
  dropped it. That was right while define files existed. The moment they
  were deleted, the same code silently ignored any `x.y.sv` — so a leftover
  `main.kotlin.sv` in a source tree would have contributed nothing, with no
  message. Deleting a feature means auditing the *tolerances* it justified,
  not just its code: every "skip this, it isn't for us" branch is a claim
  that something else handles the file, and the deletion may have removed
  the something else.
- **Delete the enum when it loses its second variant.** `BackingMod` was
  down to `Intrinsic` and `SourceKind` to `Language`. Keeping either would
  have left an invariant as a runtime check — `if file.kind != Language`
  in a dozen places, all of them dead — so both became a flag or nothing at
  all (`intrinsic: bool`, no `kind` field). This is the same lesson the
  `platform: bool` decision recorded, arriving from the opposite direction:
  there, an enum was refused before it existed; here, one was removed after
  it emptied out.
- **Keep the signature, add the body.** The 108 `external fn` declarations
  in the tests were not obstacles to route around: each became an ordinary
  fn with the *same* signature and a minimal body. That preserved what each
  test tested, because a written deduction list stays authoritative over
  anything inferred from a body — so the contract under test was unchanged
  and calls stayed on the named-call path. Rewriting them as `platform
  effect` members would have compiled just as well and quietly moved the
  tests onto `check_effect_call`, testing something else.
- **A partition test earns its keep during a deletion.** The TextMate
  grammar's keyword list is asserted to partition the lexer's keyword table
  *exactly*, and that assertion is what caught `external` and `define` still
  being highlighted after they stopped being keywords. Nothing else in the
  suite would have noticed. When a generated artifact mirrors a table, assert
  the mirroring rather than the artifact's contents.

- **Reuse the mechanism, and check what the mechanism keys on.** Mounting
  the `platform/` tree looked like "companions already do this", and it
  was — but companions are gated on their *module* being reachable, and
  `platform/main.kt` classifies as module `platform.main`, which no Salvo
  program declares. Discovery would have found the file and then dropped it
  silently. Stripping the leading segment is one line and the whole reason
  the reuse works; when adopting an existing mechanism, find the key it
  filters on before assuming the fit.
- **A generated file's name can collide in the target's namespace, not
  just the filesystem's.** `platform/main.kt` and the generated `main.kt`
  are different paths, so nothing looked wrong — but Kotlin names a facade
  class after the *file*, so both would have produced `salvo.main.MainKt`
  and the classpath would have carried two. The fix (host files live in
  `salvo.platform.<module>`) then propagated: the launch class differs when
  the host owns `main`, which is why `Backend::entry_hint` had to learn
  what was emitted. Check the target language's naming rules, not just the
  output paths.
- **Two backends' checks should fire on the same condition.** The
  missing-host error was first written per module for Kotlin (no crate
  root, so any module's `main` can be launched) and only for the crate root
  on Rust (only its `main` is reachable) — each locally correct, and
  together a program that compiles on one backend and fails on the other.
  Made uniform: any reachable module whose `main` needs a platform effect
  must have a host. When a rule's natural scope differs per backend, pick
  the stricter one rather than shipping the divergence.
- **Render generated glue with the emitter that generates what it glues
  to.** The host skeleton must match the interface member for member. It
  does, because `host_impl` calls the same `emit_param_list` /
  `emit_member_param_list` and `emit_return_type` that `emit_effect` does,
  on the same checked program, and takes the entry's arguments from the
  same `checked.fn_effects` table its parameters come from. Likewise
  `module_mod_names` is shared, so a skeleton cannot name a Rust module
  something the crate root calls otherwise. None of that needed a test to
  hold — which is the point, since drift here would emit code that does not
  compile, and a test only tells you afterwards.

- **An unchanged golden snapshot is the best evidence a refactor is
  faithful.** Moving std's interop out of `define` templates and into
  backend code could have changed emitted output in a hundred small ways;
  the check that settled it was that *no* backend snapshot moved at all,
  with both toolchain suites passing unmodified. When replacing a
  mechanism rather than a behaviour, make "the output is byte-identical"
  the acceptance criterion and you get a regression test for free.
- **Let the type system refuse a bad shape.** Adding
  `BackingMod::Platform` compiled the parser fine and immediately produced
  three `non-exhaustive patterns` errors at sites that classify *type* and
  *fn* backings — where `Platform` can never occur. That was the signal the
  shape was wrong: `platform` applies only to effects and
  `intrinsic`/`external` never do, so the flag belongs on `EffectDecl`
  (`platform: bool`), which makes the invariant structural instead of three
  unreachable arms someone would later have to reason about. A compiler
  error asking you to handle an impossible case is usually asking you to
  restructure, not to write the arm.
- **A feature framing can delete a subsystem.** The template design needed
  marker regions, canonical keys and signature-change detection so
  regeneration would not clobber hand-written code. Choosing an interface
  the host implements made all of it unnecessary — generate once, and let
  the target compiler catch every kind of drift. Before building
  machinery to protect user edits, check whether a different boundary makes
  the edits unreachable.
- **Reuse the existing injection mechanism instead of adding a second
  one.** A platform effect needed no new emission path *because* effects
  already lower to interfaces/traits threaded as parameters — the only
  change was which effects `main` receives. The alternative (a
  registration/wiring mechanism for host objects) would have duplicated
  what `use` and handler dependencies already do.
- **A program-wide uniqueness check must not read the `Symbols` maps.**
  They are hash-ordered and last-wins, which is how the
  "two effects cannot share a member name" bug hid for so long: the
  collision *was* the map overwriting an entry. The check walks files and
  items in source order and reports at the second declaration, so it is
  deterministic and fires exactly once — the same lesson as the
  `core_modules` sorting fix below.
- **When a test fails, read the test source before the compiler.** The
  first platform-effect positive test failed with "deduction promises `n`
  back to the caller, but the body moves it" — and the compiler was right:
  `return n` moves `n`, so `-> [n] Int` was a contradiction I had written.
  The checker's diagnostics have been load-bearing enough for long enough
  that a fresh test source is the more likely culprit.

- **`try` was invisible to four traversals, and one of them emitted wrong
  code.** `Expr::Try`'s body is ordinary code, but the wildcard arms of the
  scanning traversals never looked into it. The mutability census was the
  fatal one: a variable assigned *only* inside a `try` body was declared
  `val`, and kotlinc rejected the output — a [backend-never-wrong] miss that
  nothing but the toolchain would have caught. (Proven by reverting the arm:
  `'val' cannot be reassigned`.) Also missing: `collect_declared` (generated
  locals could collide with a name declared in a `try` body),
  `expr_mentions` (a use-after-move mentioned only inside a `try` would go
  unreported), and the deduction walk (a consuming call inside a `try` did
  not reach the inferred contract). Rust's mutability census was missing
  `Expr::Widen` too.
- **New AST variants are only half-caught by the compiler.** Adding
  `Expr::WhenCond` produced five `non-exhaustive patterns` errors (the two
  `check_expr` dispatches, `expr_defer_escape`, and each emitter's
  expression dispatch); the *dozen* other traversals that needed an arm
  ended in `_ => {}` or `_ => false` and compiled silently. All of them are
  now exhaustive, so the next variant is a compile error at every site —
  which is how the `try` gaps above were found, since making the match
  exhaustive forces every variant to be *classified* rather than defaulted.
  The ones that matter fail quietly: `reach.rs`'s `expr_names` (a module
  used only inside the new construct has its import pruned), `block_exits` /
  `block_returns` / `expr_terminates` (a total chain not counting as
  returning), `collect_assigned_expr` / `expr_mentions`, `deduce.rs`'s walk,
  `collect_mutated` / `collect_declared` in both emitters, and each
  emitter's `emit_operand` parenthesization list.
- **A rule can sit in the spec for eight milestones without being
  enforced.** `[if-bool]` (now `[cond-bool]`, renamed when the rule grew to
  cover `while` and subject-less `when` heads) said "`if`/`elif` conditions
  must be boolean expressions; there is no truthiness" from M0 and was never
  checked: `analyze_cond`'s fallback arm typed the condition and threw the
  type away. Nothing caught it because nobody *wrote* a truthy condition —
  the spec was describing a convention the authors were already following.
  When adding enforcement to a stated-but-unchecked rule, expect the sweep
  to come back empty and do not read that as evidence the rule was
  redundant; the next contributor is who it is for. Worth auditing the
  other "must"s in LANGUAGE_SPEC.md the same way.
- **The Kotlin entry class is named after the file, not the function.** The
  `entry point:` hint `salvo compile` prints read `salvo.<module>.MainKt`
  unconditionally — right only because every entry file so far was
  `main.sv`. Kotlin puts a file's top-level declarations in a facade class
  named for the *file*, so `other.sv` gives `salvo.other.OtherKt`. Latent
  as long as the hint was only ever read by a human who then typed the
  right thing; `salvo run` executes it, so it had to be correct
  [kt-run].


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
- **Check what the *other* backend does before designing a representation.**
  A bundle of functions as a struct value read perfectly and ran on Kotlin;
  on Rust it does not compile at all (`impl Trait` is illegal in a field
  type). Writing the five-line program first turned a plausible design into
  a rejected one, and made the group declaration-side sugar — which is why
  neither backend needs to know groups exist.
- **A feature that adds no lowering is worth looking for.** Implicit
  parameters gave `sort(?cmp)`, numeric abstractions and overridable defaults
  with *no* new backend machinery, because they reduce to something both
  emitters already do: pass a function. The traits design that preceded them
  needed generated interfaces on Kotlin and generated traits plus impl blocks
  on Rust. When a feature seems to need new runtime shapes, check whether an
  existing one already carries it.
- **A cache is only safe if its key is the whole input.** The e2e stamps hash
  the generated files, the expected output and the toolchain version — so
  changing the emitter invalidates exactly the tests whose output changed,
  and nothing else. Where the thing under test is a *subprocess* (the CLI
  tests), the key has to include the binary instead, which is why those
  stamps miss on every rebuild: correct, if less rewarding. Fingerprint a
  binary by length and mtime, not by content — hashing tens of megabytes in
  a debug build cost more than the tests it saved (+12s, measured).
- **The availability *probe* was one of the most expensive things in the
  suite.** Each of 39 Kotlin tests ran `kotlinc -version` as its guard, and
  that starts a JVM: 1.4s a time, against 2.4s for the compile it was
  guarding. Probing once per test binary (`OnceLock`) took the Kotlin crate
  from 44s to 32s without touching a single test. When a test suite is slow,
  measure the scaffolding before the work.
- **A gate belongs at the point of use, not in every caller.** Three tests
  called the Kotlin runner with no toolchain guard at all, so they would
  have *failed* rather than skipped on a machine without `kotlinc` — found
  only by putting the check inside `run_kotlin_files`/`run_kotlin_entry`/
  `run_rust_files`, where it cannot be forgotten.
- **An `async` block is a state machine you are allowed to borrow.** Rust
  has no stable generators, which is why `Iter<T>` was eager for eight
  milestones — but rustc *will* build a resumable state machine for an
  `async` block, and driving one by hand needs nothing but `Box::pin` and
  `Waker::noop()`. No `unsafe`, no crates, no CPS transformation of the
  body. The lesson generalises: when the target lacks a feature, check
  whether it lacks a *mechanism* or only the syntax.
- **Restricting the feature was cheaper than plumbing lifetimes, and it
  came from the same place the divergence did.** A lazy iterator has to
  hold whatever its body needs across the suspension; effects are the only
  thing emitted Rust holds as a borrow. Forbidding them
  ([iter-effect-free]) bought `'static` captures — which is why the whole
  change needed no lifetimes anywhere in the Rust output. Both parity
  strategies were available; the restriction was an order of magnitude
  less work than faithful emission would have been.
- **Write the feature in the language before designing around it.** The
  traits discussion assumed `map`/`filter`/`reduce` needed an `Iterable`
  bound. Actually writing them found the opposite: monomorphic combinators
  over `Iter<T>` already worked on both backends, and the three things in
  the way were a checker inference gap and two emitter bugs
  ([call-generic-progressive], [rs-fn-param-convention],
  [kt-fn-mangling]) — none of which a trait would have fixed, and two of
  which emitted silently wrong code. The trait question survived the
  exercise, but its *justification* changed from "needed for combinators"
  to "needed for `for` and for one body over many collections".
- **An emitted-name collision is only safe if the target agrees with the
  checker about which overload it is.** Kotlin's overload resolution
  follows Kotlin's type lattice, and Salvo types that are unrelated can map
  onto Kotlin types that are not (`Iter`/`List` both reach
  `Iterable`). Mangling every overload [kt-fn-mangling] is cheaper than
  reasoning about the target's subtyping — and the Rust backend had it
  right for a different reason all along.
- **Two renderers of the same contract must share one function.** The
  fn-type declaration and the lambda that fills it decided `Copy`-ness
  independently, on different types, and agreed everywhere except generics
  — where the failure was an `E0631` in *generated* code and only for
  *annotated* lambdas. The fix that sticks is one function computing the
  convention, called by both sides [rs-fn-param-convention].
- **A new file under `std/` needs a touch to be seen.** `std/` is embedded
  into the CLI with `include_dir`, which has no rerun-if-changed trigger for
  *added* files: `cargo run -- analyze` kept reporting "7 std files" after
  `std/core/throw.sv` appeared. `touch crates/salvo-cli/src/main.rs` (or any
  edit to the crate) picks it up.
- **A std module may be named after a target-language keyword, but only
  because the emitters already escape identifiers.** `core.throw` emits
  ``package salvo.core.`throw` `` on Kotlin — backticks are legal in a
  package declaration *and* in the matching import, verified with `kotlinc`
  before committing to the name — and a plain `mod`/`#[path]` pair on Rust,
  where `throw` is not a keyword at all. The escape comes free from
  `kt_ident`/the Rust equivalent; a module name that needed escaping in a
  position those functions do not cover would not have worked.
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
  probe confirmed `?` on `ControlFlow` is stable and that a may-throw call
  in a loop stays a loop.
- **`?` is not usable where deferred code must run**: it returns without
  running the splice. That is why a may-throw call with pending `defer`s
  becomes an inline `match` — and it is the concrete reason `defer` had to
  be built before `throw` rather than after.
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
- (M8) Eager iterators changed side-effect *timing* vs Kotlin's lazy
  `Iterable`, and infinite ones hung. **Closed 2026-09-05** without waiting
  for generators to stabilize: an `async` block *is* a state machine rustc
  will build, so the lowering drives one with a no-op waker
  [rs-iter-lazy].
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
