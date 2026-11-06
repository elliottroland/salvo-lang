# PROP_TEST.md — property tests, the third kind of test

Working document (begun 2026-11-05). Nothing here is built. It records:
- what was already thought about property tests;
- what the actor tests set as precedent;
- the user's decisions (round 1, 2026-11-05);
- what the probes found;
- the questions still the user's to answer.

Each open **DECISION** gives the options, the trade-offs and a
recommendation. When the work is done, the file is retired the usual way:
the record goes to COMPLETED.md, the rules to LANGUAGE_SPEC.md, and what is
left to ROADMAP.md.

## 1. What was already thought

- **TF-5 (ROADMAP §9).**
  - A library `check(runs, property)`, with the generator as the implicit
    `?generate`, so the qualifier on the parameter type picks it.
  - Randomness is a `Mut Rng` value, not an effect, so generators are pure
    and replayable.
  - Shrinking replays the generator over a shrunken draw stream.
  - Prerequisites: bit and wrapping operations ([op-bits], [op-wrap]) and
    recursive implicits ([implicit-recursive], 2026-10-06). Both are built.
    ROADMAP §6's heading now says so.
- **[test-kind] (2026-10-03).** `test property(ARGS) "…"` was the planned
  spelling: the kind and its arguments come before the name.
- **D11 (the runtime record).** A `proptest` "runs on [the virtual runtime]
  too and generates the seed as one more input". Making *every* test
  virtual was rejected because it "changes what today's plain tests
  exercise". A new kind has no such problem.
- **`Failure`** was made a struct so that it could later carry a property's
  seed and shrink count.

## 2. Precedent from actor tests

| Piece | `test actor` today | `test property` |
|---|---|---|
| Syntax | `TestKind::Actor { seed }`, `seed: N` checked at parse time | `TestKind::Property { runs, seed }`, plus the parameters |
| Desugar | a fn `[use, spawn, Throw<Failure>]` | the property fn, a generated *case* fn (§4.1), and a call to the engine |
| Runtime | one program per runtime; `begin_actor_test(seed)` | the virtual program; a `begin_property_test` |
| Report | `##salvo-test begin/ok/fail` | plus the counterexample, seeds, run and shrink counts |
| Traps | no `trapped_by`, since the body spawns and a lambda cannot | `trapped_by` around each run, so a trap shrinks like a failure (§4.5) |

## 3. Decisions so far (user, 2026-11-05)

1. **The surface: the kind, desugaring to the library engine.** The kind's
   arguments are written like named arguments, and the property's
   parameters follow the name:
   ```salvo
   test property(runs=200) "reverse is its own inverse" (xs: List<Int>) {
       expect_eq(reverse(reverse(xs)), xs)
   }
   ```
2. **Generators are values, `Gen<T>`, found by the type argument.**
   Overloading `generate` by return type is refused by R-2
   [fn-overload-return] (probed). With `Gen<T>`, the pair `fn gen() ->
   Gen<Int>` and `fn gen() -> Gen<Small Int>` is accepted, and resolution
   matches exactly (probed; it runs on Rust).
   - Invariance is assumed. If variance is ever built it must include a way
     to declare a parameter invariant; this is now recorded in ROADMAP §7.
3. **Generators for user types come only from the derived declaration form,
   written in the test annex**: `fn gen() -> Gen<Point> by …`.
   - Nothing is written on the type: no `: Arbitrary<self>` obligation.
   - The derive refuses a type whose construction is restricted (a
     constructive qualifier on a field, or a type built only through a
     qualifier constructor).
   - A recursive type gets a depth budget.
4. **Randomness: a pure `Rng` in Salvo**, test-only for now.
   - It has a live mode (splitmix64, recording each draw) and a replay
     mode.
   - The run seed is random and printed.
   - A property's seed is `hash(run seed, test id)`.
5. **Shrinking replays a shrunken draw stream**, with a step budget.
   Because every shrunk value is regenerated, it still satisfies every
   qualifier.
6. **Property tests run on the virtual runtime.** The scheduler seed is one
   draw: a failure replays exactly but does not shrink its interleaving.
   Making each scheduling choice a draw comes later.
7. **The failure report** carries:
   - the counterexample, *regenerated* from the shrunk stream and printed
     with `to_str` (a parameter type without `to_str` is a compile error);
   - the seed, and the run and shrink counts;
   - a re-run command.

   It travels as extra protocol lines. A shrunk case that passes on its
   final replay is `FLAKY`.
8. **Qualified types:**
   - There is no filtering fallback. A parameter type with no declared
     generator is a compile error naming the declaration to write.
   - `Double` generators include `NaN` and `±∞`.
   - The user asked how constants (`InRange(lo, hi)`) and dependent
     qualifiers (`Idx(list)`) get generators. Round 2 answered it:
     decisions 11 and 12, with dependent qualifiers in v1.
9. **Ownership, purity and parity** are as proposed:
   - generators are pure;
   - linear and `Mut` parameters are fine;
   - the same seed gives the same counterexample on both backends.
10. **CLI and manifest.** `--runs` wins, then the test's own `runs=`, then
    `[test] runs` in the manifest, then the default of 100. Not in v1: a
    regression file.

Round 2 (user, 2026-11-05):

11. **A generator takes the qualifier's slots as arguments, for both kinds
    of slot** (option (b) of round 1). A generator for `Q(a, b) T` is
    `gen(a, b)`:
    ```salvo
    fn gen(lo: Int, hi: Int) -> Gen<InRange(lo, hi) Int>
    fn gen<T>(list: NonEmpty List<T>) -> Gen<Idx(list) Int>
    ```
    - The desugaring passes a constant slot's constant, and a place slot's
      earlier parameter.
    - In a signature, a constant slot may be filled by a parameter.
    - Where the result type matters, that argument must be a constant.
12. **Dependent parameters are in v1, by this rule:**
    - Parameters are generated in order, and a slot may name only an
      earlier parameter.
    - Types must fit, so no run is discarded:
      `(xs: List<Int>, i: Idx(xs) Int)` is an error asking for `xs:
      NonEmpty List<Int>`.
    - When `xs` shrinks, `i` is regenerated for the new `xs`.
13. **Actor properties are postponed**, but the v1 design must be forward
    compatible with them, including the surface. §5 is that design. Its
    conclusion: the surface stands.
14. **One spelling for a kind's arguments.** `test actor(seed: 7)` becomes
    `test actor(seed=7)`. The old form simply stops parsing, and every use
    in the repo is rewritten.
15. **The derive is spelled `by random`.** §6.1 covers where it lives and
    what that implies for decision 4.
16. **The seed has the same precedence as `runs`:** `--seed`, then the
    test's `seed=`, then `[test] seed`, then random.
17. **std gives the foundation, and the user declares the rest.** std
    declares generators for its own types and qualifiers, plus the
    combinators to build more. Every user type, and every qualified type
    std doesn't cover, needs a declared or derived generator.
    - std covers `Int`, `Long`, `Double`, `Bool`, `Byte`, `Char`, `Str`,
      `Bytes`, `List`, `Set`, `Map`, `T?`, tuples and literal unions.
    - Its qualifiers covered are `NonEmpty`, `InRange`, `Idx` and `KeyOf`.

## 4. What the probes found (2026-11-05)

Each probe was a small program under `tmp/`, analysed and run on both
backends where noted. The probes are deleted.

### 4.1 Passing a property with `Throw<Failure>` to the engine: works, with the engine catching each run

```salvo
fn check<T>(runs: Int, property: (x: T) [Throw<Failure>] -> None, ?gen: () -> Gen<T>) [] -> Str? {
    …
    let r = try { property(g.run(rng)) }
    when r { is Ok { } is Thrown { … return the report } }
}
```

- This analyses cleanly. It runs on both backends with a named fn and with
  a lambda, and reports `run 2: expectation failed: small` on each.
- **The written `[]` is required.** Without it, the engine inherits the
  property's `Throw` [fn-effects], so its own `try` sees nothing that
  throws and its callers are charged with the throw.
- **The `[E]` version** (`property: (x: T) [E] -> None`) is accepted by the
  checker and runs on Kotlin. **The Rust emitter refuses it**
  (`throw:outside-try`, since a throw through an effect variable has
  nowhere to go). This is recorded as a ROADMAP §9 defect. The engine
  doesn't need `[E]`: a property's only effect is `Throw<Failure>`, and
  anything else it uses it `use`s in its own body.
- **What this means for the desugaring, given dependent parameters
  (decision 12).** A single `check<T>` over one generated value can't
  express `(xs, i: Idx(xs) Int)`. The kind therefore generates a property
  fn and a case fn, and the harness drives them. §5 sets the final shape;
  it is a loop in the harness over a library driver, not a higher-order
  `run_property(case)`. `check<T>` over a single value stays as the library
  form for properties inside an ordinary test.

### 4.2 Plain bodies on the virtual runtime: they run unchanged, except for host threads and real time

Nothing in the virtual mode depends on the test kind. A body that uses
these runs the same on both runtimes:
- collections, assertions, `use`;
- the in-memory fakes (`MemFs`, `MemTransport`);
- synchronous host I/O (file reads and writes, a stream's
  `read`/`write`/`lines`);
- `Console`, `Env` and `Args`;
- `DefaultRandom`, which is seeded there.

What differs on the virtual runtime:
- **Host threads end the process.** This covers `receive`/`pipe` on host
  streams, the host network transport, and any platform handler answering
  through `SalvoReply::hosted` (an SDK client). Each goes through
  `external_begin`, which reports and exits. A property needs the
  in-memory fakes, as an actor test does.
- **A wait only a real thread can answer** is an immediate deadlock report.
- **Time is virtual.**
  - `tick()` and `monotonic_nanos` stay at 0 unless a timer deadline moves
    them, and timers fire at once.
  - `DefaultClock.now()` still reads the real wall clock, which is not
    deterministic.
- **No worker threads.** Pools start none, so a body that relies on
  parallel progress (spinning or parking while another pool works) hangs.
- **The virtual-only fatal reports** (deadlock, a full mailbox, a host
  thread) are process exits, which `trapped_by` doesn't catch. The runner
  reports `DIED` and runs the remaining tests in a fresh process.
- **Two tables survive between virtual tests**: the routing table and the
  stream table. This is already a ROADMAP follow-up.

Decision 6 stands. Section 6 below lists the remaining caveats, for
Testing.md.

### 4.3 Constants of a qualifier: today only comptime can read them

- `fn gen<lo, hi>()` is refused: "generic parameter names must start with
  an uppercase letter".
- `InRange(Lo, Hi)` doesn't parse, because a constant slot takes a literal.
- So no ordinary fn can be generic over a qualifier's constants. Today
  constants are reached two ways:
  - **in comptime**, by the constant binder (`[if field.type is <lo, hi>
    InRange(lo, hi) *]`, [comptime-qualifier]);
  - **at run time, by the qualifier's own `qualifies`**, which receives
    them as ordinary arguments (`InRange.qualifies(n, 0, 100)`).
- Decision 11 chose how a generator gets them.

### 4.4 Dependent generators: your form works in the checker today

```salvo
fn gen<T>(list: NonEmpty List<T>) -> Gen<Idx(list) Int> {
    return Gen<Idx(list) Int> { run: (rng: Rng) -> {
        let i = rng.seed % size(list)
        if i is Idx(list) {
            return i
        }
        return unreachable!()
    } }
}
…
let g = gen(xs)                       // Gen<Idx(xs) Int>
let i = g.run(rng)
let v: Int = get(xs, i)               // the total `get`: the claim arrived
```

- The checker accepts it: the claim is bound to the caller's `xs` through
  the type argument. Kotlin runs it and prints `2`.
- The control holds: with `let i = 1`, `get(xs, i)` is the optional one
  ("expected `Int`, found `Int?`").
- **Rust refuses it**: the closure stored in `Gen`'s `Arc<dyn Fn + Send +
  Sync>` captures a `Vec<T>` with no `T: Send + Sync` bound (E0277). This
  is an emitter defect, recorded in ROADMAP §9. It blocks dependent
  generators on Rust, so it is the first thing to fix.

### 4.5 `spawn` in a property: not in v1 without a language change

- A lambda that spawns is refused (`actor-spawn-effect:in-lambda`), and a
  fn type cannot declare `spawn`.
- So `trapped_by`, which takes a lambda, can't wrap a spawning body. That is
  why actor tests run without it.
- A named `[spawn]` fn *does* pass as a value to a slot without `spawn`.
  That is a capability hole, recorded in ROADMAP §9, and the design must
  not rely on it.
- For properties this matters more than for actor tests: a trap that ends
  the process can't be shrunk.

### 4.6 The harness loop, with and without `spawn`: both work on both backends

This probe models the desugaring §5 proposes. A generated case fn takes the
attempt as plain data, builds its `Rng` inside, draws, and calls the
property. The harness loop calls the case *directly*:
- **Non-spawning:** inside `trapped_by(() -> { try { case(a) } … })`. The
  lambda captures `a`, a fresh `let` each iteration.
- **Spawning:** a `[spawn, Throw<Failure>]` case and property, called with
  a plain `try` from the `[use, spawn]` harness. No fn value is involved.

Both printed the same results on Rust and Kotlin. One thing it showed:
**a lambda moves the locals it captures**, so a `Mut Rng` local can't be
lent to a trap catch inside a loop (`flow-use-after-move`). That is why an
attempt is plain data, and why the draws are not read back out of the catch
(§5.2).

## 5. Forward compatibility with actor properties (decision 13)

An actor property is a property whose body spawns. It runs on the virtual
runtime, which decision 6 already chose, and its interleaving should
eventually shrink as its values do. What it rules out for the design now:

### 5.1 No fn value can carry the case, so the engine is a driver and the loop is generated

- A fn type cannot declare `spawn`, and a lambda cannot spawn. So a library
  fn that *takes* the case, like round 1's `run_property(runs, case,
  show)`, can never call a spawning case. The named-fn route works only
  through the §4.5 hole.
- Instead, the library provides a **driver**: a value the harness asks for
  attempts and reports verdicts to. The generated loop calls the case
  directly (§4.6):
  ```salvo
  let run: Mut PropertyRun = property_run(runs, seed)
  while run.next() is Attempt a {
      begin_property_attempt(a)         // a fresh virtual runtime, seeded from the attempt
      let failure = trapped_by(() -> { …try { __case_N(a) }… })   // v1
      run.record(a, failure, __draws_N(a))
  }
  report(run, __show_N)                 // show is pure, so a fn value is fine
  ```
  - For an actor property, the line becomes `try { __case_N(a) }`, with no
    trap catch.
  - Everything else is unchanged: the driver, the case and show fns, and
    the report.
- **So decision 1 holds.** The kind still desugars to library code: the
  driver, `Gen`, `Rng` and the generators are all std. Only the loop is
  generated, as the harness already generates one per test.
- `check<T>`, the higher-order form for a property inside an ordinary test,
  is built on the same driver. Like every fn that takes a fn value, it is
  for non-spawning properties only.

### 5.2 An attempt is plain data, and its draws are re-derived

- `Attempt` is a struct of plain data: the live seed, or the stream to
  replay. It is copied into a trap catch, and it survives a trap outside
  the catch (§4.6).
- The driver needs an attempt's draws to shrink it. Every draw happens
  *before* the property is called, because the case draws the parameters
  and then calls it. So the driver gets them by re-running the draw-only
  `__draws_N(a)`, with no state carried out of a catch. A trap mid-property
  loses nothing.
- **Linear parameter types.** A draw-only or show fn makes a value it must
  then end. For a linear type, that needs the type's discharger, or the
  property's parameter refused. This is engineering, to settle when
  building. It is recorded here because it touches decision 9.

### 5.3 What actor properties add later, and why nothing above changes

- **Spawn in the body.** The property fn's powers grow from `[use,
  Throw<Failure>]` to `[use, spawn, Throw<Failure>]`, as an actor test's
  are. Adding a power breaks no v1 property.
- **Spelling.** None is new. `test property` already runs on the virtual
  runtime, and the only things `test actor` added over `test` were that
  runtime and `spawn`. A spawning body is simply allowed; there is no
  `test actor property`.
- **Seed.** `seed=` already seeds the whole attempt. The scheduler's seed
  is a draw from it (decision 6), so one seed reproduces the values, the
  interleaving and `DefaultRandom` together.
- **Traps in a spawning body.** These can't be inside `trapped_by`. Three
  later answers, none of which changes v1:
  - (a) a trap boundary that is a block rather than a lambda, as `try` is,
    so nothing is a fn value;
  - (b) shrinking across processes: an attempt is plain data, so the
    runner can resume shrinking in a fresh process after a death;
  - (c) accept that a trap in an actor property reports unshrunk, with its
    seed.
- **Shrinking interleavings.** The scheduler would draw each choice from a
  log the runtime keeps in its own module state. That log survives a trap,
  so the driver reads it back and shrinks it as a second stream.
  `Attempt` grows a field for it, and nothing else changes.

### 5.4 What v1 must therefore do

- Build the driver plus the generated loop, not a higher-order runner.
- Make `Attempt` a plain-data struct that can grow fields.
- Draw every parameter before calling the property.
- Start a fresh virtual runtime per attempt (`begin_property_attempt`),
  not per test. A body's `DefaultRandom` then replays with the attempt, and
  actor properties inherit the reset.
- Refuse `spawn` in a property body, with a message saying it is not yet
  supported. Keep the §4.5 hole out of the design, and fix it on its own.

## 6. Open DECISIONs

### 6.1 DECISION — where `by random` lives

[obligation-by] reads `X` in `by X` as a module, named by an unambiguous
suffix among the modules the file can see. `std.random` already exists,
holding `effect Random` and `DefaultRandom`. So:

- **(a) The derive goes in `std.random`, and with it `Rng`, `Gen<T>`, std's
  generators and the combinators.**
  - `by random` then means exactly that module, with no ambiguity.
  - "Random values of a type" is a natural reading of `std.random`.
  - Test annexes gain `std.random` as an implicit whole-module import
    beside `std.test` [test-implicit-import]. `Failure`, the driver and
    `check` stay in `std.test`.
  - The cost: `Rng` and `Gen` become ordinary std API usable by programs.
    That revises decision 4's "test-only for now". A seedable,
    reproducible `Rng` is useful outside tests (simulations, sampling),
    and nothing about it is test-specific.
- **(b) The derive goes in a new `std.test.random`**, and test annexes see
  it implicitly.
  - Keeps decision 4 as it is.
  - But `random` is then a suffix of two modules. Any test file that also
    imports `std.random`, which is likely for code that uses randomness,
    gets "`random` is ambiguous" at every `by random`, and has to write
    `by test.random`.
- **(c) Keep `by random`, and special-case it in [mod-suffix]** to prefer
  std.test's module. This is the special case [mod-suffix] was chosen to
  avoid.

**Recommendation: (a).**

**User decision, round 3 (2026-11-05): (b).** These are test-only features.
If they are ever needed more generally, they can move into `std.random`
then, and decision 4 stands. How to resolve the `random` ambiguity is open
(§9).

### 6.2 DECISION (small) — the type `by` stamps at, for a fn with no parameters

[fn-by] stamps "at the **first parameter's** type". `fn gen() -> Gen<Point>
by random` has no parameter, so as specified it has nothing to stamp at.

- **(a) Unify the whole signature with the template's.** For example,
  `comptime fn gen<T is Struct>() -> Gen<T>` against the declaration
  `() -> Gen<Point>` gives `T = Point`.
  - This subsumes the first-parameter rule: the first parameter is just
    where `T` appeared so far.
- **(b) A narrow extension.** With no parameters, stamp at the return
  type's single type argument.

**Recommendation: (a)**, since it is the rule (b) is a case of, and it
covers a dependent derive's signature if one is ever wanted. **User
decision, round 3: (a).**

## 7. Caveats to document (Testing.md, when built)

- A property may not open host threads (host stream `receive`/`pipe`, the
  network transport, SDK clients). Use the in-memory fakes.
- Monotonic time is virtual; `DefaultClock.now()` is real and therefore
  not reproducible.
- A body that hits a virtual-only fatal report dies, with no shrinking.
- `FLAKY` means the body reached something non-deterministic.
- In v1, a property body may not spawn.

## 8. Recommended first slice

1. **Fix the Rust `Send + Sync` defect (§4.4)**, since dependent generators
   need it.
2. **The engine in Salvo** (in `std.random` and `std.test`, per §6.1):
   - `Rng` (live and replay, splitmix64);
   - `Gen<T>` and its combinators (`map`, `filter_map`, `one_of`,
     `list_of`, `weighted`);
   - std's generators (decision 17), with edge-case bias, `NaN` and `±∞`
     included;
   - `Attempt` and the `PropertyRun` driver, with replay shrinking;
   - `check<T>` on top of it.

   Tested in `*.test.sv` on both backends, including one counterexample
   that must be the same on Rust and Kotlin.
3. **The language pieces of decisions 11 and 12**: a constant slot filled
   by a parameter in a signature, and the constant-argument check.
4. **`fn gen() -> Gen<T> by random`**:
   - [fn-by]'s unification (§6.2);
   - structs, unions and literal unions;
   - the construction guard and the depth budget.
5. **`test property(runs=, seed=) "…" (params) { … }`**:
   - `test actor(seed=)` rewritten to the same spelling (decision 14);
   - the parser and `TestKind::Property`;
   - the property, case, draws and show fns;
   - the harness loop (§5.1), with `begin_property_attempt`;
   - the protocol lines and report;
   - `--runs`, `--seed` and the `[test]` keys;
   - [test-kind]'s "planned" becoming a rule, and a Testing.md section.
6. **Later**: actor properties (§5.3), scheduling choices as draws, a
   regression file.

## 9. Before the unattended run: the last calls (round 4)

**User answers, round 4 (2026-11-05):**
- **9.1: (a).** Keep the rule; `by test.random` is the explicit remedy.
- **9.2: the user discharges the obligations in the test.** The design
  changes to make that possible, so linear parameter types are allowed:
  - Values are generated **once per attempt, outside the trap catch**, and
    *moved* into the property. The body then ends a linear value by the
    ordinary rules.
  - There are no regenerating draws or show fns. The attempt fn keeps the
    `Rng` outside the catch, so its log is read after the attempt.
  - On the final attempt, the fn prints each parameter's `to_str`, a borrow,
    before the move.
  - A trap inside a *generator* is a process death (`DIED`): it is a bug in
    the generator, not a counterexample.
  - §5.2 is superseded by this.
- **9.3–9.7: as recommended.**
- **Unexpected design calls:** as proposed. The work takes the
  recommendation, marks it "agent's call, to confirm", and lists it at the
  end.

The original round-4 list follows, for the record.

Each item has a recommendation. An item the user answers "as recommended"
becomes a plan.

### 9.1 DECISION — `by random` when `std.random` is also visible

`sees_module` counts every imported module (comptime.rs). So a test file
that imports anything from `std.random` sees two modules with the suffix
`random`, and `by random` is refused as ambiguous.

- **(a) Keep the rule.** The remedy is `by test.random`, and the error says
  so.
- **(b) Narrow the candidates by content.** A `by` module that declares no
  comptime fn of the stamped member's name and kind drops out before the
  ambiguity check, the same way [implicit-fit] drops a candidate nothing
  can fill. `std.random` declares no comptime `gen`, so `by random` is
  unambiguous. This is a general rule, not a special case for `random`.
- **Recommendation: (b).**

### 9.2 DECISION — linear parameter types in v1

The draws and show fns make values they then have to end (§5.2).

- **(a) v1 refuses a linear parameter type** in a property, with a message.
  Lift it later, with dischargers.
- **(b) v1 requires the type's discharger** and calls it in the draws and
  show fns.
- **Recommendation: (a).** `Mut` parameters are fine: the generator is
  found for the type without `Mut`, and the value is made `Mut`.

### 9.3 DECISION — how general decision 11's rule is

The rule is "a constant slot may be filled by a parameter in a signature;
at a call, the argument must be a constant".

- **(a) Any fn.** For example, `fn clamp(n: Int, lo: Int, hi: Int) ->
  InRange(lo, hi) Int`, called as `clamp(x, 0, 10)`, is an `InRange(0, 10)
  Int`. In the body, `n is InRange(lo, hi)` tests with the parameters.
- **(b) Generators only** (fns returning `Gen<…>`).
- **Recommendation: (a).** It is one rule with nothing special about
  testing, and the body side is needed by generators anyway.
- **A non-constant argument at a call.** Recommendation: refuse it,
  because the result type depends on the value: "`InRange`'s slots take
  constants: `lo` here is a variable". The alternative would drop the
  qualifier from the result.
- **Included as a small fix:** negative constants (`InRange(-5, 5)`)
  parse.

### 9.4 DECISION — std API names (std API shape is the user's)

**`std.test.random`**, visible in every test annex as `std.test` is:
- `struct Rng`, test-only and opaque:
  - `long(rng)`, `int_in(rng, lo, hi)`, `bool(rng)`, `double(rng)`,
    `choose(rng, n)`;
  - its live and replay modes are internal.
- `struct Gen<T> { run: (rng: Mut Rng) -> T }`.
- Combinators:
  - `map(g, f)`, `filter_map(g, f)`;
  - `one_of(gs...)`, `weighted((w, g)...)`;
  - `list_of(g)` (sizes through a "continue?" draw per element);
  - `constant(v)`.
- std's generators, all named `gen` (decision 17).
- The comptime derive, `gen` for `Struct` and `Union` kinds:
  `fn gen() -> Gen<Point> by random`.

**`std.test`:**
- `struct Attempt`, plain data.
- `struct PropertyRun`, the driver, made by `property_run(runs, seed)`,
  with `next`, `record` and `report`.
- `check(runs, property)` for a property inside an ordinary test.
- `begin_property_attempt(a)` in `std.test.property`, beside
  `std.test.actor`.

**Recommendation:** these names. `Attempt`, `PropertyRun` and
`begin_property_attempt` are harness-facing and could stay unexported from
the user's point of view, as `begin_actor_test` is documented.

### 9.5 DECISION (small) — defaults

- 100 runs.
- 1,000 shrink steps.
- A run seed drawn from OS entropy.
- About 1 draw in 8 is an edge case for numbers (`0`, `±1`, min, max;
  `NaN`, `±∞`, `-0.0` for `Double`).
- `Str` is 90% ASCII, with the rest spanning the BMP and beyond.
- Collections continue with probability 0.8 per element, up to 64
  elements.
- Recursive derived types: the weight on the recursive arms halves per
  level, with a depth cap of 8.

**Recommendation:** these, changed freely later.

### 9.6 DECISION (small) — `FLAKY` in the summary and the exit code

**Recommendation:** count `FLAKY` as a failure (exit status 1), listed
separately in the summary line. A flaky property hides a real problem.

### 9.7 Scope of the run — what I would do, in order

1. Fix the **Rust `Send + Sync` defect** (§4.4), which is needed.
2. Fix the **named-`[spawn]`-fn hole** (§4.5): it is small, it is a safety
   hole, and the design keeps clear of it.
3. Leave the **Rust `[E]` + `Throw` defect** recorded. Nothing here needs
   it.
4. The slice in §8, steps 2–5. Actor properties, scheduling choices as
   draws, and a regression file go to ROADMAP.
5. **Commits: one per piece of work**, as AGENTS says, so five commits:
   1. the two defect fixes;
   2. constant slots as parameters (9.3) and [fn-by]'s unification (6.2);
   3. `std.test.random` with its generators and derive;
   4. the driver, `check` and `test property` with the harness and CLI,
      plus the `seed=` rewrite;
   5. the retirement of this file (record → COMPLETED.md, rules →
      LANGUAGE_SPEC.md, open items → ROADMAP.md, Testing.md written).

   If one commit for the whole feature is wanted instead, say so.
6. **Anything that turns out to be a language-design call** I didn't
   foresee is recorded as a **DECISION** in ROADMAP. I take the
   recommended option where the work cannot wait, mark it "agent's call,
   to confirm", and list it in the final report.

## 10. Implementation log (the unattended run, begun 2026-11-05)

This is the working state, for picking up after a context reset. Update it
at each step. Commits follow §9.7.5.

**The attempt fn the desugaring generates** (supersedes §5.1's sketch). For
`test property(…) "name" (xs: NonEmpty List<Int>, i: Idx(xs) Int) { body }`:
```salvo
fn __prop_N(xs: NonEmpty List<Int>, i: Idx(xs) Int) [use, Throw<Failure>] -> None { body }
fn __prop_N_attempt(run: Mut PropertyRun, a: Attempt) [use, Console] -> None => run: Mut {
    let rng = rng_of(a)                       // live or replay
    let xs = gen<…>().run(rng)                // parameters in order; slot fillers as arguments
    let i = gen(xs).run(rng)
    if is_final(a) { print each parameter's to_str }
    let failure = trapped_by(() -> { …try { __prop_N(xs, i) }… })
    record(run, a, failure, log_of(rng))
}
```
The harness loop:
```salvo
let run = property_run(runs, seed)
while next(run) is Attempt a {
    begin_property_attempt(a)
    __prop_N_attempt(run, a)
}
report(run)
```
An actor property (later) drops `trapped_by` and adds `spawn`.

**Status:**
- [x] C1: Rust `Send + Sync` defect; named-`[spawn]`-fn hole
- [x] C2: constant slots as parameters (9.3), negative constants, [fn-by]
  unification (6.2)
- [ ] C3: `std.test.random` (Rng, Gen, combinators, std's gens, derive)
- [ ] C4: driver, `check`, `test property`, harness, report, CLI, `seed=`
  rewrite
- [ ] C5: retire this file

**Notes as they arise:**
- C1 is done. Two Kotlin defects found along the way are recorded in
  ROADMAP §9. **Agent's call, to confirm:** the `constant(v)` combinator is
  cut from 9.4, because Kotlin can't `copy` a generic `T`. The
  substitute is `map(g, (x) -> v_expr)` with a literal, or a declared
  generator.
- Kotlin refuses `let v: Str = get(xs, i)` for a `proj Str` element. Avoid
  annotating owned locals from `get` in std's generators.
- C2 is done. **Agent's call, to confirm:** a place argument for a
  parameter-filled value slot stays a dependent claim about the place,
  which it already was, instead of being refused as 9.3 had it. Only a
  literal makes a constant.
- Useful for C3: a comptime fn with `T` only in its return type can also be
  *called* at the expected type (`let r: Ok S | … = from_found(found)` in
  std/args.sv). `[for arm in X.arms]` walks a union's arms, with
  `arm.kind`, `arm.type` and `[arm.type](v)` / `[arm.type] { … }` to build
  one.
