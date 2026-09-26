# Salvo Compiler — Roadmap

What is **left** to build. Everything finished — the milestones, the four
completed sequences, the options explored and abandoned, and every closed defect
with its repro — is in [COMPLETED.md](COMPLETED.md); this document assumes it and
points into it rather than repeating it.

Consolidated 2026-09-26 (user request): the finished sections that used to fill
this file are gone, their reasoning already recorded in COMPLETED.md's decision
log, and what remains is **one sequence**.

Companion documents: [docs/language/](docs/language/) is the narrative spec
(source of truth); [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md) states every feature as a
labeled rule (`[qual-erasure]` style) with the compiler decisions under it;
`BACKEND_SPEC.<backend>.md` ([kotlin](BACKEND_SPEC.kotlin.md),
[rust](BACKEND_SPEC.rust.md)) repeats rules with backend interpretation and adds
backend-prefixed rules — load one only when working on that backend. Labels are
referenced from code and tests (`grep -rn '[rule-name]'`).

## How to read this

- **The sequence is the order of work.** Take the next unblocked item; nothing
  below step 1 is blocked on anything above it except where it says so.
- **A `DECISION` needs a language-design call before implementation**, and that
  call is the user's: state the options, the trade-offs and a recommendation,
  then wait (AGENTS.md's first invariant).
- **Standing constraints on every item.** Unsupported constructs are *errors*,
  never wrong output [backend-never-wrong]. The two backends must agree
  observably; a divergence is closed by restriction or by faithful emission.
  Backwards compatibility is not a requirement, so a rule change is a sweep of
  every example rather than a shim.
- **Before starting anything**: read COMPLETED.md's "Gotchas / lessons learned"
  for traps in the area you are touching, and its decision log for whether the
  question was already answered.

## Where we are

Everything the four sequences covered is built and running on **both backends
with identical output**: shared fate and borrow emission, linearity with a
designated `close`, effects through handler dependencies, `throw`/`try`, places
and field narrowing, deductions with refinements, the iterator reduction to
`next`, the collections, the filesystem, actors (spawn, send, park, watch,
bridge), time, free concurrency, shareable-by-default handlers, refinement types,
group borrowing, the testing framework, and the comparison/hashing capabilities.
Ten worked examples in `examples/` carry the checked-in generated code for both
targets and the output they print. 1539 tests green.

## The sequence

### 1 — The std reorganisation (started 2026-09-26; steps b–e open)

One theme: std's shape. Step (a) landed with the sitting that scheduled the rest,
and each remaining step is mechanical but wide — the sweeps are the cost, not the
design.

- **(a) ✅ `Checked<T>` in `core`, and a list write answers one** — built
  2026-09-26 [checked-type] [col-bounds] (COMPLETED.md's log). `take` is
  `detach`; the fallible `swap` hands back a `Checked<Bool>`.
- **(b) ✅ The filesystem on `Checked<T>`** — built 2026-09-26 [fs-surface]
  [checked-type] (COMPLETED.md's log). `Err Checked<FsError>` throughout; the
  bespoke wrapper, its `ignore`/`detach` and its `to_str` are gone.
- **(c) The filesystem out of `core`.** `std.fs` holds the effect and
  `DefaultFs`; `std.fs.restricted`, `std.fs.mem` and `std.fs.host` hold the
  rest — so `import std.fs.mem.MemFs`. Two things to work out on the way: this
  is the first time a **file and a module share a name** (`fs.sv` beside an
  `fs/` directory), which source discovery and module-path classification have
  never had to resolve; and `core.fs` is currently visible everywhere, so every
  user of the filesystem gains an import. Sequence after (b), so the rename
  sweep happens once.
- **(d) `core.nonempty` dissolves into the collections.** Its `NonEmpty`
  qualifiers move next to the containers they claim, as `core.list`'s already is.
  The blocker to solve first is [qual-ctor-same-file]: a constructor must be
  declared in its qualifier's file, which is why the claims were gathered in one
  module in the first place — so either the element-taking constructors move with
  them (delegating with `set_of@core.set(…)`, the pattern `min`/`max` already
  use) or the rule is relaxed for an `intrinsic`'s `+Q` return. This also closes
  the recorded "`NonEmpty` constructor convention — the siblings" item.
- **(e) `throw` out of `core`.** The module moves like `time` did, so a program
  that never throws never links it. Every user of `Throw`/`try` gains an import,
  which is the sweep.

### 2 — A task body's effects (recorded 2026-09-26; analysed, not built)

A free `send fn` may declare no effects [free-send-fn], and the user's decision
is that it should **inherit them from the function that minted the reply**: the
restriction was written when there were no thread-shareable handlers, and there
are now.

Lifting the refusal is one line in the checker and the program then type-checks;
the **emission** is the work. A mint becomes
`Box::new(move |v| report(__c0, *v))`, and an effectful target needs its handler
arguments *inside* that closure, which must be `Send + 'static`. The machinery
exists: spawn-inheritance built fused handle bundles (`__Hs_N`,
[rs-handle-bundle]) for exactly "carry this scope's shareable handlers into a
child". The slice: synthesise the target's effect arguments at the mint from the
minting scope's handles (the checker's `spawn_dep_items` is the precedent),
capture the bundle, and pass it per activation on both backends. A probe is in
`tmp/task1`.

### 3 — The two open defects

Both are findings from the 2026-09-25 defect round, and both are *reported* by a
target compiler rather than silently wrong.

- **An un-annotated lending `?iter` has no lifetime to tie.** A fn-typed implicit
  whose return is a **borrowing** pass needs `holds proj(c)` [proj-infer] for the
  emitter to tie the lifetime [rs-proj-lends]; without it rustc says "lifetime
  may not live long enough". Repro:

  ```
  fn total<C, It>(c: C, ?iter: (c: C) -> Mut It, ?Yield<It, Int>) -> Int => c {
      let t = 0
      for x in iter(c) { t = t + x }
      return t
  }

  fn main() [use] -> None {
      use StdOutConsole()
      println("${total([1, 2, 3])}")   // rustc: lifetime may not live long enough
  }
  ```

  Adding `holds proj(c)` fixes it, so the shape is writable — what is missing is
  the diagnostic. **A small language call**: *infer* the lend for a fn type whose
  return instantiates to a borrow-holding type (the [proj-infer] fallback already
  says "conservatively every kept parameter" for bodiless declarations, so the
  checker could synthesize it), or *refuse* the declaration naming
  `holds proj(c)`. Inference is the better default; the refusal is the cheap one.
  Kotlin runs both forms.

- **A bare generic *struct* literal does not determine a type parameter.** The
  collection-literal half is fixed [col-literal-arg]; a struct literal takes a
  different path. Repro:

  ```
  struct Box<T> { value: T }
  fn unwrap<T>(b: Box<T>) -> T => !b { return b.value }
  fn main() [use] -> None {
      use StdOutConsole()
      println("${unwrap(Box { value: 7 })}")   // no matching overload for `unwrap(Box)`
  }
  ```

  The fix is the struct-literal counterpart of the collection-literal one: infer
  the literal's own type arguments from its **field values** (unify each declared
  field type against the value's type) before the enclosing call's substitution
  is solved.

### 4 — Project manifest and LSP source-root discovery (DECISION, then build)

**The defect**: editing std with the *repository root* as the editor's workspace
folder produces ~750 lines of spurious diagnostics, because the LSP takes the
client's `rootUri` as the analysis root and the repo's independent trees (`std/`,
`examples/*/salvo/`, `demo/`, test corpora) are then analyzed as one program. The
reported symptom was at `std/core/list.sv:86` — a `preserve Idx` promise
apparently ignored — and it is collateral: under the repo root the on-disk file
classifies as module `std.core.list` while the embedded copy is `core.list`, so
[std-shadow] misses, both copies load, and the duplicated `swap` makes the
refinements refuse to attach. Repro without an editor:

```bash
cargo run -- analyze --src .      # from the repo root: errors in std
cargo run -- analyze --src std    # the correct root: clean
```

The **workaround** meanwhile: open `std/` as its own workspace folder.

**The decided direction** (user, 2026-09-24): per-document source-root discovery
in the LSP, anchored by a **project manifest** — option (b) of that round, chosen
over widening [std-shadow] to strip a leading `std/` (fixes only this case, and
silently re-classifies a user's own `std/` tree) and over documenting the
workaround alone.

**DECISION — the manifest's shape**: what the file is called, what it may state
(source root certainly; backend, main and target dir are the obvious candidates,
each a CLI flag today), whether `run`/`compile`/`test` read it too (they should,
or the LSP and the CLI disagree about what a project is), and what root discovery
does with no manifest in sight (fall back to `rootUri`, today's behaviour).

### 5 — Consistency passes the 2026-09-26 ambiguity round left

Three narrower questions, all downstream of "refuse to choose" (COMPLETED.md's
log for the round itself).

- **DECISION — identical-signature shadowing.** Two candidates with the *same*
  signature at different rungs are now an ambiguity, so a module declaring its own
  `size(List<T>)` must write `size@mymodule(xs)` at every call (or take the
  overload out of the shared name with `rename fn` / `import … as`, which is the
  intended shape). Nothing in std or the examples depended on the old silence —
  the churn was six tests that existed to encode it — so this is a question about
  *ergonomics*, not about breakage: is the strict reading what you want, or should
  an exactly-identical signature at a nearer rung still shadow?
- **Implicit resolution still resolves by rung**, deliberately: an implicit has no
  written call site to annotate, and two same-named types have no distinguishing
  selector at all. If that is to change it needs a spelling first.
- **The rest of the resolution-by-position rules.** The round covered calls and
  refinements. A pass over the spec should list every remaining
  resolution-by-position rule and decide each — separating *silent winners* from
  *designed shadowing* (a later `use` shadowing an earlier handler is interception
  semantics [effect-intercept], and a fn-typed local shadowing a name outright is
  the caller's explicit choice).

### 6 — Recursive implicit resolution, so a tuple can have a `cmp`

[col-hashed-ordered] says "a `List` or a tuple qualifies exactly when its elements
do, comparing lexicographically", and that is true of the two *backends* rather
than of the language: `core.compare` declares `cmp` for the intrinsic scalars
only. It cannot declare one for a tuple, because
`cmp<A, B>(a: (A, B), b: (A, B)) -> Int` needs `?Ordered<A>, ?Ordered<B>` and
[implicit-resolve] **skips a candidate that itself needs implicits**.

This is the accepted limitation behind the keyed containers: a keyed container over
a **tuple or list** is refused by name (user decision 2026-09-26 — "I'm ok with the
limitation today"). Lifting it has two halves, and the second is the larger:

1. **Resolution** — `resolve_implicit_fn_at`'s one-line skip becomes a recursive
   resolution with a depth cap and a cycle refusal. Contained.
2. **Emission** — a filled implicit that *itself* needs implicits has to be handed
   its own, so `ImplicitArg::Resolved` needs nested arguments and both backends'
   adapter closures have to pass them (`cmp((A, B))` calling `cmp(A)`/`cmp(B)`).
   `implicit_args` records no nesting today, so this is where the work is.

The cheaper alternative, with its cost stated: declare the tuple and `List<T>`
`cmp`/`eq`/`hash` as **intrinsics**, which is what the backends already do
structurally. No recursion needed, and it *documents* the status quo — but it
freezes it: an element type's own declared identity would be ignored inside a
tuple or list key. That is already true; declaring it makes it look intended.

Two recorded items wait on the same lift: `expect_eq` on a generic container
cannot resolve a `to_str` [interp-to-str], and property testing's `?generate`
(step 9) needs it.

### 7 — Qualifiers are droppable, then variance

- **Qualifiers are droppable on assignment** (user decision 2026-09-23, not
  built). A variable's type may never *widen*, but a qualifier is by definition
  something that optionally applies, so dropping one is always legal: assigning a
  plain `List<Int>` to a variable inferred as `NonEmpty List<Int>` must be
  accepted, and the variable simply stops being `NonEmpty` (the flow state already
  models exactly this — a mutating call drops claims the same way). Provenance
  qualifiers need thought: dropping one is harmless, re-*gaining* it must stay
  impossible. Where it shows up today: `analyze_tests`'
  `fate_links_merge_across_branches` and `inferred_moves_consume_arguments` had to
  annotate their `let`s when the constructors started claiming, and those
  annotations come back out.
- **An inferred type argument is not widened by the expected type** — found
  while building step 1(b), and the same invariance in a narrower place.
  `checked<T>(value: T)` binds `T` from the argument, so
  `checked(NotFound { … })` is a `Checked<NotFound>` and the expected
  `Checked<FsError>` does not widen it — not through a `let` annotation either
  (probed). std therefore names the argument at all 35 construction sites,
  `checked<FsError>(NotFound { … })`, which is explicit and costs a word. The
  declared-position widening that the old `FsError { kind: … }` relied on still
  works; what does not is *seeding a type variable* from the expectation. The
  keyed-container work seeds a callee's type parameters from the expected type
  already, so the machinery is there — the question is whether an expectation
  may pick a **supertype** for an inferred argument, which is the same question
  variance asks and should be answered with it. **A small DECISION**: widen
  (unify against the expectation's arms, so `checked(NotFound { … })` at a
  `Checked<FsError>` position infers `FsError`), or leave the explicit form as
  the one way and keep this recorded as intended.
- **DECISION — variance on generic parameters** (user direction 2026-09-23). A
  `List<NonEmpty List<Int>>` is not a `List<List<Int>>` today, so a constructor
  call cannot fill a plain type-argument position. The direction is `in`/`out`
  declaration-site variance as C# and Kotlin have it. What has to be decided with
  it: declaration-site or use-site (recommendation: declaration-site first,
  because std's containers are where it pays); what `Mut` does to it (a
  `Mut List<T>` cannot be covariant in `T` — the classic array-store hole — so the
  natural rule is that covariance holds only while the value is not `Mut`);
  whether qualifiers on a type argument are a **separate, narrower** rule worth
  pricing first (the case that raised this needs only `NonEmpty List<Int>` →
  `List<Int>` *inside* a type argument, which is qualifier-dropping at depth); and
  that the emitted Rust must not depend on it, since Rust has no variance.

### 8 — Mutating through a union arm (DECISION)

One shape is **refused on Rust and accepted on Kotlin**, which is a divergence
closed by restriction on one side and therefore a decision rather than a resting
place:

```
fn bump(o: Ok Mut List<Int> | Err Str) [] -> None {
    if o is ^Ok {
        add(o, 7)          // Kotlin: mutates the caller's list
    }                      // Rust: reported — `o` is read-only here
}
```

The parameter renders `&Union2<…>` because an arm's `Mut` is not a claim about
`o`, and **nothing can ask for the `&mut`**: a written `=> o: Mut` is refused
("a deduction may preserve or drop qualifiers, not add them"). So the question is
what a `Mut` arm means for the *parameter* that carries it.

- **(a) The arm's `Mut` makes the parameter mutable.** Cost: the Rust signature
  then disagrees with the checker's contract, which still says the parameter is a
  kept read — and two arguments naming the same place, which the checker permits
  as two reads, become two `&mut` borrows and an E0499 on a program Salvo
  accepted. Teaching the fate analysis about that is the real scope.
- **(b) Admit the deduction.** Relax "may not add qualifiers" so `=> o: Mut` is
  legal when `Mut` is present on an arm, and let inference write it from the body.
  Cost: a written deduction now means "through an arm", a new reading of the
  clause; benefit: the contract stays visible in the signature, which is what
  deductions are for.
- **(c) Keep the refusal, and refuse it in the *checker*** so both backends say
  the same thing. Cost: a Kotlin program stops compiling; benefit: one story, and
  the remedy (take the payload as its own `Mut List<T>` parameter) is one line.

**Recommendation: (b)**, with (c) as the fallback. (a) is the one to avoid: it
makes the two sides of the compiler disagree about what a signature means, which
is how the original defect happened.

### 9 — Testing, beyond the MVP (decided 2026-09-23, not built)

The framework's core is built and `salvo test` runs std's own suite. What was
deliberately cut, in the order the decisions put it:

- **`context` scopes** — decided in shape (TF-9): nesting plus per-test re-run
  initialization (`use`s and `let`s that run again for every test, so no
  cross-test state is expressible), with the test's id joining the context names.
  Three sub-calls ride the implementation: statements **preamble-only**
  (recommended, error otherwise), a preamble may do **anything a test body may**,
  and a **childless context warns**. The lowering is per-test inlining, which
  makes isolation true by construction.
- **Property testing** (TF-5): `check(runs, property)` with the generator arriving
  as the implicit **`?generate`**, so the *qualifier on the parameter type picks
  the generator* (`(text: ValidDate Str) -> …`); randomness threaded as a
  `Mut Rng` **value**, which the effect-free-resolution rule turns into a
  determinism guarantee; shrinking by replaying the generator over a shrunken draw
  stream. Two prerequisites: std needs wrapping/bit `Long` intrinsics for a
  pure-Salvo splitmix64 (parity by construction — a recommendation, not yet
  decided), and step 6's implicit lift.
- **Actor testing helpers** (TF-6): `settle(p)`/`expect_settled(p)` over
  [actor-on-idle], a recording `Probe<M>` handler, `expect_fault(target, body)`
  over [actor-watch]. Needs `spawn` added to [test-body]'s implicit powers, and
  the harness's default fault sink to record rather than print. No test scheduler
  (rejected as out of scale).
- **The blackbox tier** (TF-2's `tests/` tree): ordinary modules that `import`
  what they test and see only exports.
- **`--isolate`, `--timeout`, crash recovery** (TF-4): one process per test, a
  per-test wall-clock kill, automatic re-run of a crashed batch's remainder. The
  MVP runs everything in one process and reports a test that died (`DIED`).
- **The `std.` import prefix** (TF-8, decided 2026-09-19): `import std.time`
  rather than `import time`, `std` a reserved root, the bare spelling refused with
  the corrected path named, and the library tree moving to `lib/std`. Sequenced
  after the MVP deliberately; note that **step 1(c) overlaps it** — `std.fs` is
  the first module to want the prefix, so doing them together may be cheaper.
- **Smaller leftovers**: the annex's one-way visibility holds by construction but
  is not *checked*; the report has no `--format json` for editors; nothing
  migrates the compiler's own e2e suite onto `salvo test` (user decision: leave
  it).

### 10 — The assertion trap policy (A-6, decided 2026-09-23, not built)

Three failure classes still take the hosts' behaviour:

- **Subscript out of range** → trap with *our* message (index and length),
  replacing the hosts' two different texts. Needs care on the *place* path
  (`arr[i] = x`), where a block expression cannot stand.
- **Division by zero** → trap with our message; both hosts already trap, only the
  text differs.
- **Integer overflow** → **wrapping**, stated in the spec, with `checked_*` /
  `saturating_*` std functions for the cases that care. This is the JVM's
  behaviour today and the cheap one on Rust. **It is the only row that changes
  what existing programs compute**, so it wants its own slice and a parity test:
  today Kotlin wraps silently while Rust refuses a constant fold and panics in
  debug.

### 11 — One read, one mode: the rendering that reports a reference

Three slices landed 2026-09-23 [rs-read-mode]; what is left is the **refactor the
section is named after**. The slices work because their sites know the shape they
will get and can ask a predicate first (`owned_optional_local`,
`narrowed_borrow`). The general case cannot: a site that wants `&T` writes
`&{code}`, and if `{code}` is already a `&T` the result is `&&T`. The durable
answer is for the rendering to answer *what it produced* (`Rendered { code,
is_ref }`) so a site can decide whether to add the `&`.

The site waiting on it is **interpolation**, the most common read position in the
tree, which still clones every narrowed or `!`-ed value:

```rust
// std/test.sv's expect_trap_with, where the same `trap` two lines up borrows
format!("… got `{}`: {}", trap.as_ref().unwrap().clone(), label.clone())
```

`emit_interp_value` cannot simply switch to `emit_read`: its native branch would
take the reference happily (`format!("{}", &String)` displays), but the two
`to_str` branches in the same function write `to_str(&{arg})` and
`{place}.{field}` — three questions to the predicate in one function, which is
precisely the shape that wants the rendering to answer for itself.

Worth finishing because it is the last **systematic** copy nobody wrote,
`[copy-opt-in]` is a stated principle rather than an aspiration, and it is
measurable: `examples/*/rust/**` carries the clones, so the diff *is* the
benefit. Two recorded items are the same missing information in other clothes:
the temporary-subject `for` loop, and the copy an adapter closure makes of a
returned projection.

### 12 — Effect transformers (E3 step 4)

The last rung of the handler-control arc. A **transformer** is an effect member
that runs a fn-typed parameter with *additional* effects available — its body
having registered handlers for them. `Retry`, `Timeout` and async are all that
shape, and `try` could be re-expressed as a library transformer if it reads
better than the intrinsic.

- **The gate is already built**: step 3 made fn-type effect lists real and threads
  effects *into* a fn value instead of capturing them [fn-effects]. What remains is
  the surface.
- **No silent colouring** (user decision 2026-09-04): the ability to not resume is
  declared on the effect member, never discovered from the handler.
- **Async is explicitly not part of this arc**: it arrives later as an explicit
  effect, likely a compiler intrinsic, not as an `async`/`suspend` transform of
  the whole program.
- **Multi-shot resumption is closed on principle**: resuming twice duplicates a
  use obligation, so it cannot coexist with `Linear`. (Neither target offers it
  either.)

Two related recorded items: a **handler cannot dispatch to itself** — inside a
member the bare member name is already taken (it means the handler registered
*before* this one [effect-intercept]), so self-dispatch needs a different
spelling, and that spelling is a language call (`self.bump()`, `bump@self()`, …);
the workaround (a free fn both members call) is what std does and has cost
nothing. And **two cuts inside the effect fusion** stay *reported* rather than
mis-emitted [rs-effect-fusion]: a dependent handler using its own generic
parameters in a member signature, and a `use` whose effect instance is still
generic.

### 13 — The sugar pass (after the explicit surface, decided 2026-09-15)

Phase 5 delivered the **explicit** actor surface (tokens and reply parameters
written out); every layer of sugar above it is a later item with its own decision
surface. What is already decided, so the pass starts from a plan:

- **Per-kind `-> T`** (EU-6): plain effects unchanged forever; inside an
  `actor effect`, `fn m(a) -> T` means an implicit trailing `Reply<T>`,
  fulfil-at-every-return, and caller-side call syntax = auto-mint + gate. This is
  why `send fn` stayed explicit: the non-send forms are stated *against* it.
- **The generalized mint** (EU-7b): `replyto k(c)` resolves lexically against the
  enclosing handler, else through the effect list as a *remote* mint — a curried,
  capacity-reserved, one-shot send, with reservation at mint time so discharge
  never blocks, and mint sites contributing deadlock edges like sends.
- **Two stub readings appear here, and only here.** An answering member cannot
  have one implementation for both bindings: from an actor the call parks, from
  synchronous code it must block.
- Then the rest of the tower, each its own call: `then`/`then!`, `defer`,
  merge/join, the gate's member-set generalization.
- [fate-lambda] belongs here — no first-pass form crosses a closure. The recorded
  refinement is `move`-closure emission with hoisted clones, plus a treatment for
  captured effect-handler locals.

### 14 — Shared mutable state: `Cell` (DECISION)

Deferred until after actors deliberately, because the OTP answer is that actors
own their state and message-pass — which may remove the motivation. The full
design (a capability *qualifier* rather than a container type, the representation
per backend, why it cannot panic, the rules it drags in, and the producer case
that is its hardest customer) is recorded in COMPLETED.md's "Shared mutable state
(`Cell`)" section, moved there with this consolidation. The question to answer
first is whether shared mutable state joins the language at all.

When it is taken, it should be decided **on one table** with the rest of the
sharing story: shared-immutable versus shared-mutable, invalidation-checked
versus unchecked (today's fate links, frozen `Reg`, `canbe` groups, `Cell`) —
with the observation that `proj` is the degenerate group (a read-only member of a
singleton group under maximal invalidation sensitivity), so the two are points on
one dial rather than two features.

### 15 — Regions (designed 2026-09-10, unbuilt)

Fully designed and recorded: `effect Region` with an intrinsic handler, `Reg` as
an intrinsic provenance qualifier, regional-by-birth defaults, `reg`/`unreg`, the
freeze (dropping `Mut` makes handles duplicable, kills fate links and makes state
qualifiers permanent), and the escape rule. Kotlin erases it entirely; Rust stages
it — v1 `Rc<T>` [rs-region-rc], v2 a real arena with one mechanical lifetime per
delimiter [rs-region-arena]. The design, the R2 rejection (regions manage memory
and lifetime, never obligations) and what it retires for frozen values are in
COMPLETED.md's "Regions" section, moved there with this consolidation.

Still open when it is picked up: the exact freeze spelling (`^Mut` as an
expression, freeze-by-position, or both); cross-region operations (out of v1);
`unreg` of a deeply regional structure copying deeply; and folding D7's
`Local`/`Escaping` watch-list entry into the design.

### 16 — Laziness, after concurrency (direction decided 2026-09-10)

std's lazy pair was **removed** rather than carried along, and the question
reopens here. The direction is the user's: standard laziness *couples data to the
functions over it*, and the two should stay separate — so what is wanted is a good
way to **compose functions (`iter fn`s included) into pipeline functions**, which
then mint a fresh pass from data supplied independently. `map`-then-`filter` would
build a *function*, not a wrapped data structure, and the data arrives at the end.

What the existing implementation contributes: an `iter fn` is already "a function
that mints a pass" whose pass is unnameable by design; a pass is only a struct
with a `next` [iter-protocol]; effects on fn types already thread a callback's
effects to whoever calls the value [fn-effects].

Questions to answer with it: what composes and how it is spelled (two `(T) -> U`
steps compose obviously, an `iter fn` is a different arrow, and a filter changes
the *count* of elements); where per-run state lives (minted per drive, which is
the replay property `iter fn` already has); whether it dissolves the L8 casualty
or inherits it (a pipeline holding only *functions* stores no source, so
"lazily `map` over a file's lines?" may become yes without widening
[linear-composite] at all — the strongest argument for this direction, and the
thing to test first); and sendability.

### 17 — Recursive types (DECISION, end of the queue)

Investigated 2026-09-12; nothing needs it, and List-mediated recursion covers its
customers (trees, ASTs, JSON) meanwhile. Where it stands: nothing rejects a
recursive type, so `struct Node { value: Int, next: Node | None }` passes the
checker, runs on Kotlin and dies at rustc with E0072 — an accept/reject
divergence. Recursion **through `List<T>` already works end to end on both
backends**.

- **Step 1, the diagnostic** (a defect fix, independent of the feature): an SCC
  walk over the type graph (struct fields, union arms, alias expansions;
  `List`/array/fn-typed edges do **not** count — they indirect already) and an
  error at the declaration naming the field that closes the cycle, with the
  `List<T>` encoding as the named remedy. [type-no-cycle] landed the declaration
  refusal for the *direct* case 2026-09-25; the union-arm case is what is left.
- **Step 2, the feature**: a boxing rule for the Rust backend — where the box goes
  (minimal-edge boxing, the presumption), transparency at every use site (literals
  wrap, reads autoderef, matches need explicit derefs since box patterns are not
  stable, partial moves keep working, `Mut` paths get `&mut` via `DerefMut`,
  `copy` deep-clones), and **non-regular (polymorphic) recursion refused in
  Salvo** — a type may recurse only at its own instantiation, or Rust
  monomorphizes forever while Kotlin's erasure accepts it.
- **The semantic edges**, each a small language call: **constructibility**
  (`struct A { a: A }` has no base case — refuse cycles with no optional/union
  escape arm, recommended); **depth, not cycles** (values are acyclic, so
  `to_str`/equality/drop terminate, but each recurses per node and a 100k chain
  overflows generated `toString`/derived `Drop` — accept-and-document for v1);
  **linearity stays out** (refuse recursion + linearity in one declaration for
  v1); and an `iter fn` over a recursive subject snapshots per field, which is a
  deep copy.

## Recorded, not scheduled

Each was considered and deliberately parked. Nothing here is blocking, and
several are "revisit only if a customer appears".

- **Pick chains** (`^Ok?: Err?: err(_)`) — recorded rather than scheduled at the
  user's call: the same program is expressible today with one pick plus a `when`,
  so a chain buys brevity and the evidence that would settle it is *a real program
  in `examples/` or `std/` that reads worse without one*. Decided and still
  standing if built: picks consume arms **in order**, and the empty pick may appear
  anywhere in the chain. Cost: `Expr::Elvis` needs `picks: Vec<ElvisPick>`, the
  checker consumes arms pick by pick, both emitters emit an `if`/`else if` chain;
  the parser's lookahead already exists.
- **Value-level parity for hash and random.** Callable `hash` lowers to each
  backend's **native** hashing, so hash *values* diverge across backends —
  accepted deliberately, on the analogy of random numbers. The shape and the
  high-level guarantees are identical (`eq(a,b)` ⇒ `hash(a) == hash(b)` within one
  execution). The possible future reversal, for hash and random *together*:
  language-defined algorithms implemented identically in both runtimes (the
  rejected sketch: FNV-1a 64 over a canonical byte encoding). Until then a program
  must not print or persist a hash value and expect cross-backend identity.
- **Platform-handler thread-safety contract (DECISION).** A platform handler is
  *assumed* thread-safe (user decision 2026-09-20) and nothing validates it. The
  divergence to close: Kotlin binds the raw host instance while Rust shares it
  through the per-effect lock adapter, so a *non-conforming* host races on Kotlin
  and is accidentally serialized on Rust. Two emissions would restore parity —
  **(a)** drop the Rust lock for a handler that declares the contract (`&self`
  members, `Arc<H>`, which also makes rustc machine-check half the contract), or
  **(b)** serialize both by putting Kotlin's platform bindings behind
  `__Mon_E`. (a) as the declared path, (b) as the undeclared fallback. The
  DECISION is the contract's surface: where the declaration lives, what it
  asserts, and what the undeclared case means. `salvo platform generate` should
  print the chosen contract into the host file it writes.
- **`on_idle`'s predicate (DECISION).** The hook fires on the strict quiescence
  condition while the deadlock report fires on a weaker one, so a program stuck
  *with a parked frame* gets the report and exit 1 where the relaxed reading would
  let it react. One line in each runtime either way; the argument for relaxing is
  consistency, the argument against is that `on_idle`'s meaning drifts toward
  "nobody can move", which is the report's job.
- **`size(Str)` outside ASCII (DECISION).** Kotlin lowers it to `String.length`
  (UTF-16 code units), Rust to `chars().count()` (code points), so
  `println("${size("a😀b")}")` prints 3 on Rust and 4 on Kotlin. Needs a decision
  about what a `Str` index *means*, then one lowering per backend. `byte_size` is
  parity-safe by construction and is what the filesystem uses.
- **Intersection types (DECISION)** — whether `Addr<A & B>`-style types join the
  language; recorded 2026-09-17 when the tuple form shipped instead.
- **`platform type`** — deferred by decision; `platform effect` and
  `platform handler` are the whole interop surface until a need arises.
- **`const` bindings** — announced (user intent 2026-09-19), not designed. Its
  first customer is recorded: the shareable-handler taxonomy's rung 1 keys on "no
  mutable state", which today means "no fields, no `Mut` constructor parameters";
  `const` immutable fields would join the allowance.
- **`Deque<T>`** — the honest replacement for a linked list, and the next
  collection when a customer appears: one intrinsic type, six functions, no new
  concepts. (A representation qualifier `Linked List<T>` was examined and
  rejected: it would make the shared `List` surface worse, and Rust's
  `LinkedList` has no stable cursor API.)
- **`entries`/`values` passes over a Map** — deferred: an entries pass needs an
  owned `(K, V)` and Kotlin cannot copy a generic `V`, so identity-sharing would
  alias mutable values. The answer is probably the snapshot shape the key pass
  uses, over a `List<(K, V)>`.
- **Test-suite speed** — the stamp key stays **keyed on the generated sources**
  (user decision 2026-09-25): it cannot go stale, and that is worth more than the
  seconds a cheaper key would save. If it ever bites, the options in order:
  a source-keyed stamp plus an emitter-version token; caching the emission beside
  the verdict; emitting `std` once per source and running many; shrinking the
  registry. Measure first: how much of the Kotlin binary's ~15s is `std`
  re-emission versus per-case work.
- **Locators through opaque anchors, branded tokens, and the bounds-check
  mitigation ladder** — the group-borrowing ladder's recorded refinements, with
  GhostCell declined on the record (a brand is a scope-bound *lifetime* and actor
  state escapes every scope) and raw pointers / `RefCell` rejected. In
  COMPLETED.md's log for 2026-09-24/25.
- **In-place writes during iteration** — refused by the driven-origins rule
  [iter-fn]. The contents-versus-replacement distinction [deduce-field] is what
  would license it, and the locator model already makes the shape renderable.
- **Field-set inference for [deduce-field]**, and qualifiers on struct fields:
  v1/v2 are written-only, so an unannotated fn keeps the conservative whole-value
  event.
- **`once` inference**, **returning/storing capture-carrying closures**,
  **exactly-once closures consuming a linear capture**, **reassignable borrowed
  locals** (accumulator bodies under a projected return), **same-call borrow/move
  (E0505 shape)**, **the internal qualifier unification**, **L5 field-disjoint
  precision**, **`NotEq` over any `?eq`-capable subject**, **link parameters**
  (with the nested-`proj` source they would make real), and **type-mention tracing
  for instantiation links** — all unforced, each recorded with its shape where its
  rule lives.
- **The linear instantiation ban misses generic effect members** — a hole in what
  shipped: `[linear-generics]` is checked in `resolve_named_call`, and an effect
  member's own generics are not covered, so a linear value can be smuggled
  through one.
- **Nested patterns** (`let ((a, b), c) = …`) are refused in a `let` as in a
  loop, and **struct patterns bind by field name only** (no `..` rest, no nested
  field pattern, no binding of a projection).
- **A generic `List<T>` cannot be interpolated** [interp-to-str] — waits on
  step 6's lift plus `implicit_args` being keyed by something an interpolation
  has.
- **Emission marks everything public**, by decision: [mod-export] is a checker
  rule. Narrowing generated visibility would buy dead-code warnings the suite
  already tolerates and needs the reachability pass to agree.
- **No re-export**, so a facade module declares wrappers; the spelling would be
  `export import a.B`, currently a targeted parse error. And **no example shows
  `export`**, because every program in `examples/` is a single file — a two-file
  example would fix that and would be the tree's first multi-module one.
- **Where dot-notation is normalized** [fn-dot] — the receiver-as-argument-0
  rewrite happens twice and never in the AST, so anything inspecting the *written*
  expression must re-derive the shift (one defect came from exactly that). Two
  shapes when it is picked up: record the normalized argument list per call span
  in a side table (cheap), or a real normalization pass after types are known
  (removes the duplication). It cannot move into `salvo-syntax`: the name must
  resolve, and an `Addr<E>` receiver is not argument 0.
- **Actor-surface prose is owed in `docs/language/`.** Every actor rule is in
  LANGUAGE_SPEC.md, and docs/language/ has the "Where work runs" and "Time"
  chapters — both of which assume vocabulary the document never introduces
  (`actor effect`, `send fn`, `spawn`, `Addr`, `replyto`/`waitfor`, `watch`).
- **Actor leftovers, each with a named trigger**: a self-send into a full own
  mailbox wedges and `k@self` deliberately contributes no deadlock edge
  (**DECISION** when it matters); the deadlock graph is over actor *types*, not
  instances, so a chain of same-protocol workers reads as a self-loop
  (stratification and the timeout form wait for observed false positives —
  writable now that `Timer` exists); the report names actors by index rather than
  by handler and member; a one-thread pool whose occupant sends into a full
  mailbox on that same pool still hangs (only the `main` case is caught); a `use`
  site does not check the `[spawn]` capability; a main-pool task whose answer
  arrives after `main`'s last wait never runs, silently; an effectful discharger
  cannot `drain` a container; there is no positional list write; `on_idle`'s
  refinements (per-pool firing, naming who is parked, a many-shot form); FC-7 host
  bridging; and the `[waitfor]` spawn-placement diagnostic still states a hazard
  the pump rule removed.
- **Multi-effect and mixed-handler leftovers**: a same-named member across two
  faces still needs `@Effect` at the call even where the parameters distinguish it
  ([effect-at] keys on the name); a dependent multi-face handler is untested;
  mixed handlers with dependencies, overloaded servant members and several faces
  were first-slice cuts; one lock behind several faces has no backend
  representation; generic-instance dependencies stay fusion-pinned (the
  representation now exists, so lifting the `handler_handle_deps` exclusion is
  engineering); a `with` item may not have dependencies of its own, and `main`'s
  platform-effect parameters cannot be captured as handles.
- **`local` inference** — the checker writing `local` for you, lifting the
  virality down local-trafficking call chains (the deduction pattern: written
  validates, unwritten infers). **Note (user, 2026-09-26): the meaning of `local`
  is itself being revisited** — a round of questions about `println`'s
  `local Console` dependency was deferred with "I might have misunderstood what
  `local` means", so re-read [use-local] and [effect-local] with the user before
  building anything here.
- **`fn qualifies@Positive`** — migrating qualifier bodies' `fn qualifies` to the
  `@`-scoped shape canonicals used. Note that the shape it would migrate *to*
  changed on 2026-09-26 [fn-attached], so this is now "should a qualifier's
  `qualifies` be declared on its subject type?" and wants re-deciding rather than
  implementing. Handler members stay put: they interact with handler state.
- **Constants: literal establishment and constant subtyping** — the refinement
  round's two remainders. Literal establishment is `listen(8080)` proving itself
  (compile-time evaluation of `qualifies`); constant subtyping is
  `InRange(10, 20)` fitting an `InRange(0, 100)` position, which needs
  per-qualifier semantics for what the constants *mean* — **DECISION**-shaped
  when it is wanted.
- **`enumerate`'s claimed `index` field and a claimed `keys` pass** — left out of
  pass minting by design: a dependent claim on a struct field names a value the
  struct does not contain, and the map pass walks a key snapshot. The snapshot
  form (`keys -> List<KeyOf(map) K>`) waits for step 7's variance round.
- **The `Bytes` span twin** needs same-name-different-subject value slots.
- **Binding a view of a temporary** is refused for now (user, 2026-09-11); the
  possible automation is hoisting the temporary into a fresh local, which is what
  the user writes today — not free of judgement, since the temporary then lives to
  the end of the block and a hoist inside a loop changes how often it is built.
- **A runtime file name silently clobbers an emitted std module of the same
  name** — worked around by naming the runtime files `hosttime.{rs,kt}`, not
  fixed: extend the **companion** collision check [backend-companion] to the
  runtime files, or namespace them under `salvo_rt/`.
- **`to_str(Duration)` stops at seconds**, and there is no `to_str` for `Instant`
  or `Tick`: a wall-clock text form is a date (the calendar layer's), and a
  monotonic reading has no rendering beyond its number. **Cancellation is not in
  the timer surface** (recorded by decision), and **the wall-clock layer is
  designed but unbuilt** — `DateTime` as the calendar view of an `Instant` in a
  zone, `Period`, the two bridges, and a `WallClock` effect whose member must
  **not** be named `now`.
- **The unified test clock fakes `Ticker`; the `Clock` face is untested**, and a
  handler that waits on a *positive* deadline still wedges a `ManualTime` test —
  though it now says so, via the deadlock report. Every clock reading through the
  unified form is a round trip, which is the stance's remaining cost; the recorded
  upgrade is scheduler-owned virtual time.
- **The parked-obligation gap in the deadlock graph**: an actor gated on a token a
  *task* must discharge has a wait-for edge pointing at no effect node.
- **Array elements never narrow**, **deduction inference does not track
  bare-parameter value flow out of branch/loop tails as a move**, **module
  reachability is conservative for non-fn names**, **a private type in an
  exported signature is an opaque type and nothing checks the author meant it**,
  **struct destructuring ignores predicate-qualifier field overrides**,
  **repeating the *same* qualifier in a nested group needs an annotated
  intermediate `let`**, **`Byte` is not operator-numeric**, and **generic
  (`Ty::Var`) operands stay lenient** — each documented where its rule lives.
- **LSP**: `[symbol]` resolution is name-based over the AST rather than
  import-visibility-exact [doc-symbol-ref]; no incremental analysis; no
  `positionEncoding` negotiation for UTF-8-native clients; signature *hover*
  covers fn decls only (effect members and define fns have no `FnKey`); the
  def-site table is name-keyed, so go-to-definition on a shared effect-member
  name lands on one declaration; and hovering a fate *root* to see what derives
  from it is not built.
- **Fresh-suite speed, remaining steps toward ~10–15s**: the CLI suites (each
  spawns `salvo run`/`compile` and pays its own kotlinc), the rust codegen suite
  (a shared-runtime batch or precompiled `libcore`), and running the batched
  Kotlin programs in one JVM instead of one `kotlin` launch each. Past those the
  floor is the matrix size itself.
