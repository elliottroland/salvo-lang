# Testing — the built-in-framework option space (working document)

Status: DECIDED in core (round 3, 2026-09-19) — see "Round 3 — the decided
list" before the pending table. Superseded option sections are kept with
pointers for the argument trail; what remains open is marked in the table.

Written 2026-09-19 by an agent session, from the user's sketch of a built-in
testing framework for Salvo programs, while the shareable-handler build (SH-1…7,
`defer`) was landing in a parallel session. No sequence is in progress, so this
opens a new phase rather than folding into one. The document lays out the
design as options, trade-offs and recommendations, **and the calls are the
user's** (AGENTS.md's first invariant). **Round 2, same day:** after reading
the first draft, the user added two directions — std-prefixed imports with
`test` as a sibling library root (§0 point 10, §TF-8), and the question of
qualifier-driven generation (§0 point 11, §TF-5b). **Round 3, same day:**
the user took the core decisions — companion-only whitebox testing, the
implicit std.test import (moving the surface back under std), `?generate`
as the generator mechanism, test-local qualifiers, and `context` scopes
(§TF-9) — and asked for the qualifier-generator idea fleshed out on a
concrete date-parsing example (§TF-5b's worked example). Sections touched
by a later round say so inline.

**Propagation owed.** This session is read-only except for this file, so
nothing here has propagated: the ROADMAP.md **DECISION** rows this phase
needs, the COMPLETED.md decision-log entries once calls are made, the new
LANGUAGE.md chapter, the fresh LANGUAGE_SPEC.md rules (working labels used
below: `[test-decl]`, `[test-file]`, `[test-visibility]`, `[test-fail]`,
`[test-run]`, `[test-gen]`, `[test-actor]`, after round 2 `[test-root]`
and `[test-gen-qual]`, after round 3 `[test-context]` and
`[test-implicit-import]` — none exists yet) and their
backend-spec counterparts, the std module, and the sweep of examples are all
*owed, not done*. Per the working-document charter, **this file is deleted
once the phase lands**, its outcomes recorded in COMPLETED.md's decision log.

Sources: LANGUAGE.md "Modules and files", "Backends", "Where work runs",
"Time" (incl. "Time in a test"), "Qualifiers continued"; LANGUAGE_SPEC.md
rules [mod-file],
[mod-export], [mod-visibility], [mod-import], [mod-import-module],
[mod-used-only], [name-casing], [fn-overload-scope], [throw], [try],
[qual-result-tags], [qual-ctor-fn], [intrinsic-std-only], [platform-effect],
[backend-never-wrong], [actor-kind], [actor-effect-kind], [actor-mailbox],
[actor-spawn-effect], [actor-waitfor], [waitfor-pump], [actor-on-idle],
[actor-watch], [actor-deadlock-cycle], [actor-replyto], [actor-sendable],
[main-pool], [task-mint], [linear-obligation], [time-coupling], and for
round 2 [qual-predicate], [is-qualifies], [is-qualifies-effects],
[qual-subject], [qual-constructive], [qual-ctor-same-file],
[qual-field-override], [qual-refn], [diag-import-suggest] (all verified
present); ROADMAP.md "Decisions waiting on the user" and "Consolidated
leftovers" (the std naming conventions, the `using` rename); COMPLETED.md's
decision log (module visibility 2026-09-18, the coupling stance 2026-09-18);
`std/core/throw.sv`, `std/core/result.sv`, `std/random.sv`; and the user's
stated intent of 2026-09-19, including two mid-session additions: the
permitted-file-name rule may be extended for test files with extra dots, and
the standard library may be extended with whatever operations the framework
needs. Round 2 added three more statements: imports should mention `std`
(`import std.time`) with `test` a sibling library root (`lib/std`,
`lib/test` in this repository); the question whether qualifiers should
control property-test generation; and that **qualifier rules may themselves
be changed** to make that easier — "they're still a fluid concept in the
language".

## §0. The stated intent

The user's sketch, numbered so the decisions below can be judged against it.
It is a direction, not a specification — used here to guide the options and
their cost, not to fix answers:

1. **A built-in testing framework** for Salvo programs — not more compiler
   test infrastructure (that exists: `salvo-testkit`, the corpus, the golden
   and e2e suites test the *compiler*; this phase is for testing *Salvo
   code*, run by Salvo's own toolchain).
2. **Suspected home: a separate crate**, managing the framework — the
   compiler workspace grows a crate rather than the feature living diffused
   through `salvo-cli`.
3. **Assertions** as part of the testing toolkit.
4. **Property testing** as part of the toolkit.
5. **Testing for actors** as part of the toolkit.
6. **Tests both inside module files and outside them** — inside because
   declarations are module-private by default and a private function is
   otherwise untestable; outside because a very big test case should not
   have to live in the file it tests.
7. **Test names are strings**, so a test reads naturally
   (`test "an empty cart totals to zero"`), not as an identifier with
   underscores standing in for spaces.
8. *(Steering, 2026-09-19)* The user is **happy to extend the permitted
   file names** to include test files with extra dots in them — which
   unblocks the `list.test.sv` companion-file shape that the current
   "a file name may not contain a dot" rule [name-casing] forbids.
9. *(Steering, 2026-09-19)* The **standard library can be extended** with
   operations the framework needs — assertion helpers, generator
   combinators, PRNG support, probe handlers are all allowed to become std
   surface rather than having to be conjured from what exists.
10. *(Round 2, 2026-09-19)* **Imports should mention `std`** —
    `import std.time`, not `import time` — and the test surface becomes a
    **top-level module root beside it**: `import test`, with the two
    library roots living at `lib/std` and `lib/test` in this repository.
    Stated as a want, not a question, but it supersedes recorded behavior
    (§TF-8) and its sub-questions need answers before the sweep.
11. *(Round 2, 2026-09-19)* **Should qualifiers control property-test
    generation?** — asked as a question, and paired with a steering note
    that qualifier rules are still fluid and may be changed to make this
    easier. Taken up as §TF-5b.
12. *(Round 3, 2026-09-19)* **`context` scopes** — nesting for tests *and*
    common initialization (`use`s, `let` bindings) that re-runs for every
    test rather than persisting across them. Arrived as a decision, not a
    question; recorded as §TF-9 with its remaining sub-calls.

What fits the recorded direction outright: (1) and (3) collide with nothing —
`try`/`throw` [throw] [try] and result tags [qual-result-tags] are natural
raw material, and std already ships a test double (`ManualTime`) and a test
posture ([time-coupling]). (5) builds on machinery that exists and was built
partly *for* tests: [actor-on-idle] is documented as "the sequencing tool"
for exactly the ManualTime race. What meets a rule and therefore needs a
decision section: (6) collides with [mod-file]/[mod-export] (privacy is per
file, so an out-of-file test cannot see private names — §TF-2); (7) collides
with nothing syntactically but has no precedent in the language (no
declaration today is named by a string — §TF-1); (8) amends [name-casing]'s
file-name clause (§TF-2); (4) collides with backend parity (a property test
that generates different cases per backend violates the identical-output
principle — §TF-5); (2) is an architecture call with a load-bearing
consequence for how the harness is emitted (§TF-7). (10) collides with a
*recorded decision*: today's diagnostics literally correct
`import std.random.Random` to `import random.Random` [diag-import-suggest],
and the whole-module import rule was written over rootless paths
[mod-import-module] — so the respelling gets its own section, §TF-8. (11)
meets the qualifier rules where they are settled ([qual-predicate] /
[qual-constructive] / [qual-subject]) carrying a license to unsettle them —
§TF-5b. How tests fail and what
a test body may do is forced by (3) and (5) together (§TF-3), and how tests
are compiled and run is forced by all of them (§TF-4).

## §1. Fixed points — already decided, inherited here

- **[mod-export] / [mod-visibility] / [mod-file]** — declarations are
  module-private by default, privacy is per file, and there is no re-export.
  This is *why* in-module tests must exist (§0 point 6), and it is the rule
  an out-of-module whitebox test collides with. The decision log records the
  rule as deliberately simple; a testing feature should extend it at one
  seam, not weaken it.
- **[name-casing]** — module paths are lowercase, and **a file name may not
  contain a dot**, because a dot would read as a path separator; the same
  rule deliberately killed per-backend companion files (`string.kotlin.sv`).
  §0 point 8 is the user pre-authorizing an amendment for test files. The
  amendment must stay narrow enough not to reopen the companion-file door.
- **[mod-used-only]** — only modules the program reaches are emitted. This is
  the mechanism that already keeps a test-only module out of a production
  binary *if nothing imports it*; what it does not cover is a `test`
  declaration sitting inside a production module (§TF-4 decides stripping).
- **[throw] / [try]** — non-resumption exists: `throw(m)` returns `Never`,
  the delimiter is `try { … }`, its value is `Ok T | Thrown M`, and `Thrown`
  is deliberately forgeable because the authority is `[Throw<M>]`
  availability alone. An assertion that aborts a test and a harness that
  catches it can be built from exactly this (§TF-3).
- **[backend-never-wrong] and backend parity** — the two backends must agree
  observably; identical output is the standing bar (every example checks one
  `expected.txt` against both). A test framework whose *reports* or whose
  *generated cases* differ per backend breaks the principle where users will
  see it most (§TF-4, §TF-5).
- **[intrinsic-std-only] / [platform-effect]** — `intrinsic` is std's alone
  and every backend must lower every intrinsic. A deterministic PRNG or a
  wall-clock watchdog that cannot be pure Salvo lands as std intrinsics
  (§0 point 9 authorizes the growth), never as customer-visible magic.
- **The actor test surface already built**: `ManualTime` (two faces, least
  authority), [actor-on-idle] quiescence (`on_idle(p, notify)`, edge-triggered,
  token consumed by registration), [actor-watch] monitors, fault sinks
  (`pool(n, sink)`), the static deadlock report [actor-deadlock-cycle], and
  the posture [time-coupling] "pass time rather than read it". §TF-6 adds
  conveniences over these; it does not redesign them.
- **[main-pool] / [waitfor-pump] / [actor-waitfor]** — `main` is the single
  worker of its own pool and a wait serves its pool. A generated harness
  `main` inherits all of this; a test that spawns and waits behaves exactly
  as `main` does today, which is a feature (§TF-3, §TF-6).
- **[linear-obligation]** — linearity holds inside tests like anywhere else:
  a test that mints a `Reply<T>` and drops it is a compile error, not a
  flaky hang. No test-mode exemption is proposed anywhere below.
- **The std naming conventions** (`*_of` / `*_by` / `to_*`, recorded in
  ROADMAP.md's S-Col entry) bind the assertion and generator surfaces.
- **The qualifier taxonomy** (D5, user decisions 2026-09-03 →
  [qual-subject]): users declare *state* claims (about contents, possibly
  with a `qualifies` predicate [qual-predicate], tested by `is`
  [is-qualifies]) and *provenance* claims (mint-only, no predicate by
  construction); qualifiers without a body are *constructive* — values gain
  them only through constructor functions [qual-ctor-fn], declared in the
  qualifier's own file [qual-ctor-same-file]. §TF-5b builds on exactly this
  split — and, per the round-2 steering, is *allowed to propose amendments
  to it*: qualifiers are still fluid, so a collision with these rules is a
  candidate change rather than automatically a cost. Amendments remain the
  user's call like everything else.
- **The 2026-09-18 import decisions** ([mod-import-module],
  [fn-overload-at], [diag-import-suggest]) — the whole-module import, the
  ladder rung it enters at, and diagnostics that today treat `std.` as a
  wrong spelling. §0 point 10 deliberately supersedes the *path shape*
  these were written over; §TF-8 works out what survives.
- **Open and adjacent, not reopened here**: the `use`-clause → `using`
  rename (ROADMAP, unscheduled) — snippets below write today's syntax; and
  `salvo-testkit`'s cache design (ROADMAP "Fresh-suite speed") — the new
  crate is a sibling concern, not a refactor of it.

## §2. What other languages teach

### Zig and D — the test *block*, in-file

Zig's `test "description" { … }` is the closest existing thing to §0 points
6+7 in one construct: a top-level block with a **string name**, sitting in
the same file as the code it tests (so it sees file-private declarations),
compiled only by `zig test`, discovered without registration, run by a
generated harness with a default reporter. D's `unittest { … }` blocks are
the same shape minus the name (compiled under `-unittest`). Two lessons:
the string-named block is proven syntax with no identifier to invent, and
"compiled only under test" is a *compilation-model* rule, not a visibility
rule — the block simply does not exist in a production build. Informs
§TF-1 (the declaration form) and §TF-4 (stripping).

### Rust — the two-tier layout

Rust's convention is exactly §0 point 6: unit tests live in the module
(`#[cfg(test)] mod tests`, seeing private items because a child module is
*inside* the privacy boundary), and integration tests live in `tests/`,
compiled as separate crates that see only the public API. The two tiers are
not redundant — one tests the implementation, the other tests the contract.
Rust also shows the cost of name-as-identifier (`fn
empty_cart_totals_to_zero`) that §0 point 7 rejects. `cargo test` compiles a
harness `main` (libtest) per test binary; custom harnesses are still
unstable years in, a warning about coupling to a backend's native framework
(§TF-4). Informs §TF-2 (two tiers), §TF-4.

### Kotlin/JVM — what the Kotlin backend could lower onto

kotlin.test / JUnit 5: annotation-discovered methods, `@DisplayName("string")`
for readable names, backticked method names with spaces as the idiom.
Powerful (IDE runners, parallelism, reports) — and heavy: lowering Salvo
tests onto JUnit means a jar dependency `salvo run` never needed, a
per-backend report format, and JVM-flavored failure output. The same survey
for Rust: `#[test]` costs nothing extra but its output format
(`test foo ... ok`) is libtest's, not ours. Together they say: *both*
backends have native harnesses, and using them buys integration at the
price of the identical-output principle. Informs §TF-4's central choice.

### QuickCheck, Hypothesis, proptest — property testing

Three generations of the same idea. QuickCheck: type-directed generators
(a typeclass picks the generator from the parameter type) and type-based
shrinking — minimal ceremony, but shrinking can violate the generator's
invariants (shrink a "valid date" into an invalid one). Hypothesis and
proptest: **integrated shrinking** — the generator and its shrink travel
together (proptest: a strategy produces a value *tree*; Hypothesis: shrink
the underlying byte stream and re-run the generator), so shrunken values
always satisfy the generator. All three print the seed on failure and
accept it back for replay; Hypothesis additionally persists failing
examples. The lesson for Salvo is sharpened by backend parity: the
generator chain must be *deterministic per seed and identical on both
backends*, which no host RNG gives. Informs §TF-5.

### Erlang/Elixir and Akka — testing actors

ExUnit is a second precedent for string test names (`test "description" do`).
The actor-testing canon: Akka's `TestProbe` (an actor the test controls,
handed to the code under test as a collaborator; the test asserts on what it
received), `expectMsg`-with-timeout, virtual-time schedulers, and
`watch`-based termination assertions. Elixir adds: tests default to
*synchronous unless declared async*, because shared runtime state makes
parallel actor tests flaky. Salvo already has the virtual clock
(`ManualTime`) and the monitor (`watch`); what it lacks is the probe and the
settle-then-assert idiom packaged (§TF-6), and the isolation question
(§TF-4's process model) is exactly Elixir's async default seen from the
other side.

### Go — the runner's ergonomics

`go test`: file-suffix discovery (`_test.go`, same package → private access;
`package foo_test` in the same directory → blackbox), a plain `-run` filter,
subtests named by strings at runtime (`t.Run("name", …)`). Confirms that
file-suffix + same-module-privacy is a proven alternative to Rust's child
module, and that string names and filtering compose naturally. Informs
§TF-2 and §TF-4's CLI surface.

## §TF-1. The test declaration form

**The question.** What is the syntax for a test, and what are its naming
rules? Folds §0 points 6 and 7. Load-bearing: every other section writes
this form.

**Option A — a `test` block declaration: `test "name" { … }`.**

```
test "trim removes both edges" {
    expect_eq(trim("  padded  "), "padded")
}
```

(No import line: round 3 made the test surface implicitly available in
test files — §TF-8's note.)

A new top-level declaration: contextual keyword `test` (like `iter`, `send`,
`actor` — a variable may still be called `test`), a **string-literal** name,
a block body. No parameters, no return type, no `export` (it is not callable,
so there is nothing to export — `export test` is an error naming why). The
name must be a *literal* (no interpolation), so the harness can enumerate
and filter without running anything; uniqueness is per module, checked like
any duplicate declaration; the test's full identity is `module.path :: name`.

- Cost: a new declaration kind through parser → AST → checker → both
  emitters; snapshot churn; an LSP touch (tests should appear in document
  symbols). The string name needs a lowering rule per backend (§TF-4 decides
  whether that is a generated-`main` registry — trivial — or a native test
  function name — mangling).
- Trade-offs: reads exactly as §0 point 7 wants; matches Zig/ExUnit
  precedent; the block is statement-position Salvo with no signature to
  bikeshed. Being a non-function means it cannot be called from code, which
  is a feature (no test calling a test) but also means shared setup is an
  ordinary function the test calls, not a fixture mechanism — Salvo's
  effects/handlers already are the fixture story (`use` a fake in the body).

**Option B — a modifier on a function: `test fn …` with the string
attached.** E.g. `test "name" fn t1() { … }` or `test fn t1() "name" { … }`.

- Cost: same pipeline touch as A, plus a redundant identifier per test that
  exists only to be unique and never referenced — the worst of both naming
  worlds.
- Trade-offs: a test becomes a real fn (callable in-module, so composable),
  and overload machinery [fn-overload-scope] applies for free. But two names
  per test violates orthogonality for no gain A doesn't give: A's body can
  call any fn it likes for composition.

**Option C (shape-changing) — no syntax at all: tests as data.** A module
declares a value the harness looks for — `let tests = suite_of(case("name",
() -> { … }), …)` — pure library, zero parser work.

- Cost: none in the compiler front end; all of it in ergonomics. Static
  enumeration becomes evaluation; filtering and reporting need the list
  built before anything runs; a test is a lambda, so its inferred effects
  are invisible at a glance; in-module privacy works but the registry value
  itself needs a naming convention the checker doesn't enforce; duplicate
  names become a runtime report. Lambdas also cannot be `use`-scoped
  entry points the way `main` is without extra rules.
- Trade-offs: this is what a language without top-level blocks would do.
  Salvo *has* declarations with contextual modifiers as its idiom; buying
  zero syntax at the price of a stringly registry runs against
  "everything must be declared".

**Recommendation (the user's call):** Option A. It is the sketch's own
shape, it has two languages' precedent, and its one real cost (a new
declaration kind) is paid once and bounded. Sub-choices to confirm with it:
literal-only names, per-module uniqueness, no `export`, body rules per
§TF-3.

> **Round 3 — decided.** Option A taken: `test "name" { … }` with the
> sub-choices as recommended, plus one change against the first draft:
> the block lives **only in test files** (companions and the blackbox
> tree), never in a production `.sv` file — §TF-2's round-3 note. Nesting
> and shared setup gained a form of their own, `context` (§TF-9). Name
> uniqueness is per *sibling scope* (module level, or one `context`
> level) rather than flatly per module, since context names join the id.

## §TF-2. Where tests live and what they may see

**The question.** How do in-module, companion-file, and separate-tree tests
work against [mod-export]/[mod-file], and what exactly does the
[name-casing] amendment (§0 point 8) permit? Folds §0 points 6 and 8.

Three placements are on the table; they are not mutually exclusive (Rust and
Go ship the first and last as a pair):

**Option A — in-module tests only.** `test` declarations sit in any `.sv`
file and see that module's private names, because they are *in* the module.
Nothing else changes; big tests must live in the big file.

- Cost: none beyond §TF-1. Trade-off: fails §0 point 6's second half — a
  500-line test of a private function bloats the production source file it
  tests, and [mod-file] means splitting the file changes visibility.

**Option B — companion test files: `list.test.sv` joins module `list`.**
The [name-casing] amendment, kept narrow: a file name may contain a dot
**only** as the suffix `.test` (`<name>.test.sv`), such a file is **part of
the module `<name>`** — same privacy boundary, exactly like Rust's
`#[cfg(test)]` child module or Go's same-package `_test.go` — and it is
loaded **only by `salvo test`** ([test-file]). A companion whose `<name>.sv`
does not exist is an error naming the orphan. Only `test` declarations and
the private helpers they use may live there — it is a test annex, not a
second file for the module's production code; concretely: declarations in a
`.test.sv` file are invisible to production files even of the same module,
so production code cannot come to depend on its annex.

- Cost: the file-walker and module-path rules learn one suffix; the
  "no companion files" precedent gets a carve-out that must be worded so
  `string.kotlin.sv` stays dead (only `.test`, and only under `salvo test`);
  the one-way visibility (annex sees module, module never sees annex) is a
  new wrinkle in [mod-visibility] that needs its own rule and tests.
- Trade-offs: this is the big-whitebox-test answer, and the user has
  pre-authorized the file-name change. It keeps [mod-export] intact — no
  new visibility *grant* exists; the annex simply is the module.

**Option C — a separate test tree, blackbox.** Tests under `tests/` at the
source root (or a `--tests DIR`), each file an ordinary module that
**imports** what it tests and sees only exports. Loaded only by
`salvo test`; excluded from `compile`/`run` walks like `platform/` already
is conceptually (today the walker skips hidden/cache dirs; `tests/` joins
the skip list for production builds rather than needing `.svignore`).

- Cost: a walker rule and a CLI flag; nothing touches visibility at all.
- Trade-offs: this is the contract-testing tier — it exercises the module
  exactly as a consumer would, which whitebox tests structurally cannot.
  Alone it fails §0 point 6's first half (private functions unreachable).

**Option D (shape-changing) — a visibility grant instead of placements:**
`test import list.digits` — an import form, legal only in test contexts,
that reaches private names explicitly.

- Cost: [mod-export] stops being the whole visibility story; the diagnostic
  "declared in `m` but not exported" gains an exception; opacity reasoning
  (private type in exported signature) gains a hole; every future
  visibility feature must consider the test door. The precedent it sets —
  privacy with a bypass — is the one the 2026-09-18 module-visibility
  decision deliberately did not set.
- Trade-offs: maximum flexibility (any test file reaches any module's
  internals, no companion naming), and honest about what it does. But
  "honest bypass" is still a bypass, and B gets the same tests written with
  zero new visibility semantics.

**Recommendation (the user's call):** B + C together, D rejected, A
subsumed by B (an in-file `test` block stays legal — it is the zero-setup
case and Zig's whole model; B is where it goes when it outgrows the file).
Two tiers with distinct meanings, both proven elsewhere: `.test.sv` for the
implementation, `tests/` for the contract.

> **Round 3 — decided, against one part of the recommendation.**
> **Whitebox testing is always the `*.test.sv` companion file** — Option B
> taken, Option A (in-file blocks) *dropped*: a production `.sv` file never
> contains a test, which keeps production sources clean of test vocabulary,
> makes stripping trivial (nothing to strip — `compile`/`run` simply do not
> load `.test.sv`), and gives every test file the implicit std.test import
> (§TF-8's round-3 note) with no rule needed for production files. The
> companion has private access as designed (same module, one-way
> visibility), and — decided — it may declare things of its own: **test-only
> qualifiers, structs, and helper fns live in the companion**, invisible to
> production code, which is what §TF-5b's worked example leans on (a
> `ValidDate` qualifier the shipped program never hears of). One wrinkle
> that decision surfaces for the build: [qual-ctor-same-file] says
> constructors live in the qualifier's *file* — companion-declared
> qualifiers keep their constructors in the companion (consistent), but a
> companion cannot add a constructor to a *production* qualifier; if a
> test wants one, the remedy is a test-only wrapper qualifier or an
> exported production constructor. D stays rejected; C (`tests/`,
> blackbox) stays as recommended, not explicitly re-confirmed.

## §TF-3. What a test body is — effects, failure, and the harness contract

**The question.** What may a test body do, and how does a failing assertion
travel to the report? Folds §0 points 3 and 5's prerequisite. Load-bearing
with §TF-1: this is the semantic half of the declaration.

**The body's powers.** A test is an entry point the harness calls, so the
natural rule is: a test body has `main`'s powers — `use` and `spawn`
available implicitly (a test that registers no handlers simply uses
neither), `waitfor` available as everywhere. It runs as the single worker
of its own pool exactly as `main` does [main-pool], so everything LANGUAGE.md
says about `main` and tasks transfers verbatim. A test may not declare an
effect list of its own: whatever capabilities it needs beyond the implicit
ones, it `use`s or `spawn`s fakes for — which is the framework working *with*
the effect system (a test that needs `Fs` registers `MemFs()`; one that
needs time spawns `ManualTime()`), not around it.

**Option A — failure is `Throw`, the harness is a `try`.** Assertions
declare `[Throw<Str>]` (or a dedicated message struct, below); the harness
wraps each test body in the moral equivalent of `try { body }` and reads
`Ok … | Thrown …` off it [try]. A test passes if it completes, fails if it
throws. `expect`/`expect_eq` are ordinary library functions (in the test
root, per §TF-8):

```
export fn expect(condition: Bool, label: Str) [Throw<Str>] -> None {
    if !condition { throw("expectation failed: ${label}") }
}
```

- Cost: the implicit `[Throw<Str>]` on test bodies must be part of the
  `test` declaration's rule ([test-fail]); a failure message is a `Str`, so
  structured failure data (expected vs actual, a seed, a shrink count) is
  formatted at the assertion site rather than carried as data to the
  reporter.
- Trade-offs: zero new machinery — [throw]/[try] exist end to end on both
  backends today. Non-resumption is exactly assertion semantics ("stop this
  test, not the program"). The `Str` message is a real limitation for §TF-5
  (a property failure wants to carry its seed and shrunken value to the
  reporter as data), though a conventional prefix format can carry it in
  band at the cost of elegance.

**Option B — a dedicated `Test` effect the harness handles.**
`effect Test { fn fail(report: Failure) -> Never }` with
`struct Failure { label: Str, details: Str, … }`; test bodies implicitly
declare `[Test]`; the harness registers its reporter handler around each
test. Assertions perform `fail`.

- Cost: `fail` must be non-resuming, which today only `throw` is — a
  handler member returning `Never` whose *handler* must abort the test
  needs either to be implemented *by throwing* (making B a structured skin
  over A: `handler HarnessReporter of Test [Throw<Str>]`… with the details
  flattened to `Str` again at the boundary) or a second non-resumption
  channel, which is real language surface for one customer.
- Trade-offs: structured failure data reaches the reporter as data; a user
  could intercept `Test` (a handler that counts soft failures, an
  expect-all-then-report style). But the honest implementation *is* A plus
  a struct, so B's real content is "should `Failure` be a struct rather
  than a formatted `Str`" — worth deciding, cheaper than it looks, and
  possible to retrofit onto A later since `Throw<M>` is already generic:
  **A with `Throw<Failure>`** is a coherent middle point.
- One consequence either way: because the authority to fail is effect/throw
  availability, a *helper function* that asserts declares it
  (`[Throw<Failure>]`) and composes — custom assertion vocabularies are
  ordinary functions, no framework registration.

**Option C (shape-changing) — assertions as outcome values.** No implicit
effect at all: a test body is an expression of type `Ok None | Err Failure`
[qual-result-tags], assertions are combinators, the last expression is the
verdict.

- Cost: every multi-assertion test threads outcomes through `when` chains
  or early returns; the ergonomic tax lands on every test ever written.
  Rejected languages-wide for a reason (even Rust's `#[test] fn … ->
  Result` is the alternate form, not the default).
- Trade-offs: totally transparent, zero magic — and maximally verbose.

**Panics and faults.** An assertion is not the only way a test dies: `!` on
a `None`, an intrinsic's own panic, or an actor fault reaching a sink are
all real outcomes. The rule to decide with this section: a panic in the
test's own frames fails the test (how it is *contained* is §TF-4's process
model); an actor fault is **not** automatically a failure — actors die as a
matter of course in supervision tests — but an unwatched fault whose sink is
the default stderr one fails the test that spawned it, because a test that
did not mean to leak a fault should hear about it (the harness installs a
recording sink as each test's default; `expect_fault`-style helpers in
§TF-6 make the deliberate case explicit).

**Recommendation (the user's call):** A with the generic message —
`[Throw<Failure>]`, `Failure` a small std struct with a `to_str` — which is
Option A's machinery carrying Option B's data. Implicit body powers as
stated (`use`, `spawn`, the implicit throw); the fault-sink default as
stated.

## §TF-4. The runner — CLI, compilation model, isolation, report

**The question.** What does `salvo test` do, end to end? Folds §0 points 1,
6, 7; constrained hard by backend parity ([backend-never-wrong]'s sibling
principle: the two backends agree observably).

**Common ground (all options).** A new CLI subcommand beside
`compile`/`run`/`analyze`/`platform`/`lang`:

```
salvo test --backend rust --src ./my_project              # run everything
salvo test --backend kotlin --src . "totals"              # substring filter
salvo test --src . --list                                 # enumerate, run nothing
```

Discovery is static: walk sources (including `.test.sv` companions and the
`tests/` tree per §TF-2), collect `test` declarations, honor a positional
substring filter against the full joined id (`module.path :: "context" ::
"name"` — Go's `-run`, simplified).
`compile` and `run` **strip tests**: with round 3's companion-only
placement this is placement, not surgery — a production build simply does
not walk `.test.sv` files or the `tests/` tree, and a `test` (or `context`)
declaration in a production `.sv` file is a parse-time error naming the
companion as the fix. `analyze` still checks test files, so a broken test
fails analysis. [mod-used-only] then keeps test-only
*helper* modules out of production binaries automatically, since only tests
import them.

**Option A — a generated Salvo harness, one report format.** The runner
synthesizes an entry module: for each discovered test, a registry entry
(name string → the test body) and a `main` that filters, runs each test
under its delimiter (§TF-3), and prints a fixed-format report — Salvo's
own, byte-identical on both backends:

```
test module list :: "an empty list has size zero" ... ok
test module cart.test :: "a stale entry is dropped" ... FAILED
    expectation failed: totals differ: expected 0, got 3

2 tests: 1 passed, 1 failed (seed 8241)
```

Then the existing pipeline compiles and runs it exactly as `salvo run`
does — the harness is *a Salvo program*, so both backends need **zero new
emission machinery** for the running itself (the `test` declaration lowers
to an ordinary function with a mangled name; the registry calls it).
One wrinkle: the generated `main` lives in a synthesized module, and a test
in module `m` is module-private to `m` — the lowering gives each module's
tests one generated, exported trampoline table ([test-run] would pin this),
which is a checker-level exemption of exactly one generated name per module
rather than a visibility change users can reach.

- Cost: the registry/trampoline synthesis; a report format to design once;
  no IDE-native integration (a later LSP/`--format json` bridge is the
  path, and `analyze --format json` is precedent).
- Trade-offs: identical output *is* the house principle, and it makes the
  framework's own e2e tests cheap (one `expected.txt` per fixture, both
  backends — the existing example convention). Test parallelism inside the
  process is Salvo-level (tests could be scheduled on a pool) but
  sequential-by-default is the right start (Elixir's lesson, §2).

**Option B — lower onto the backends' native frameworks.** Kotlin tests
become kotlin.test/JUnit methods (`@DisplayName` carrying the string name),
Rust tests become `#[test]` fns (name mangled from the string).

- Cost: two lowerings, two report formats, a JUnit dependency `salvo`
  never had, string names mangled on Rust (losing §0 point 7 in the output),
  and the parity principle broken at the user-facing surface. Filtering,
  ordering, and failure output all diverge per backend.
- Trade-offs: buys mature runners (parallelism, IDE integration, JUnit XML
  for CI) that A must re-earn. But Salvo positions the backend as a target,
  not a home — `salvo run` already owns the run experience, and a Salvo
  user asking "why does my test output depend on the backend" has no good
  answer available.

**Option C (shape-changing) — interpret, don't compile.** Run tests in a
tree-walking interpreter inside the toolchain; no backend involved.

- Cost: an interpreter for the whole language (effects, actors, linearity)
  that exists nowhere today — a compiler-sized project — and it would test
  programs under semantics *neither backend executes*, which for a
  transpiler is testing the wrong thing.
- Trade-offs: instant startup and perfect isolation would be nice; the
  price is disqualifying. Recorded as considered.

**Isolation — a sub-decision under A.** One process for the whole run is
fastest but a panic or a livelocked test takes the run down with it. The
shape that gives both: the harness `main` accepts an argv of test ids and
the runner chooses — default, one process running all (fast; a panic aborts
the batch, and the report names the in-flight test because each test prints
its line *before* running... or the runner infers the culprit from the last
completed marker); `--isolate`, one process per test (slow on the JVM —
each `kotlin` launch costs ~0.4s per ROADMAP's own measurements — but
survivable); and the runner automatically re-runs a crashed batch's
remainder isolated, so a panic costs one extra process, not a policy. A
per-test wall-clock timeout (`--timeout`, generous default) is the
runner's, not the harness's — the runner kills the process, which is the
only reliable answer to a livelock and needs no in-language watchdog.

**Recommendation (the user's call):** A, with the argv-dispatch harness and
the crash-recovery isolation described; B recorded as rejected for parity,
C as out of scale. Sub-calls riding along: the report format above (or the
user's preferred shape), sequential default, `--list`, substring filter.

## §TF-5. Property testing

**The question.** What are generators, where does randomness come from, and
how does shrinking work — under the constraint that a property test must
behave **identically on both backends**. Folds §0 points 4 and 9.

**The determinism problem first, because it shapes everything.** std's
`Random` effect is handled by `DefaultRandom`, an intrinsic over the host's
RNG — two backends, two sequences. A property test seeded with 42 must
generate the same cases and find the same counterexamples on both backends,
or the house principle breaks exactly where reproducibility matters most.
And a pure-Salvo PRNG is not free either: splitmix64-style generators live
on wrapping `Long` arithmetic and bit operations, which Salvo does not
surface — and whose Rust lowering of plain `+`/`*` panics on overflow in
debug builds, so "just write it in Salvo" is a latent
backend-divergence bug, not a shortcut. Two viable shapes, both authorized
by §0 point 9:

- **(i) std grows the integer operations** (`wrapping_add`/`wrapping_mul`/
  `xor`/`shift_left`/… as `intrinsic fn`s on `Long`, each trivially lowered
  on both backends), and the PRNG (splitmix64) is **pure Salvo** in the
  `test` module — the algorithm is Salvo source, so agreement is by
  construction and the new intrinsics are generally useful std surface
  beyond testing.
- **(ii) an intrinsic seeded handler** — `intrinsic handler SeededRandom(
  seed: Long) of Random` with the algorithm pinned in the backend spec and
  cross-checked by an e2e test asserting both backends print the same first
  N draws. Smaller std surface, but the algorithm lives twice (once per
  backend runtime) and agreement is by test rather than by construction.

**Option A — library-only, inside an ordinary test.** `Gen<T>` is a std
struct (a generation function plus a shrink function — fn-typed fields
exist); combinators build them; `check` drives:

```
test "reversal is an involution" {
    check(lists_of(ints()), xs -> {
        expect_eq(reverse(copy(xs)), reverse_twice_reversed_somehow(xs))
    })
}
```

`check(gen, property)` runs N cases from the run's seed, and on failure
shrinks, then throws a `Failure` (§TF-3) carrying the seed, the case count,
and the shrunken counterexample's rendering — so replay is
`salvo test --seed 8241 "involution"`.

- Cost: entirely library + harness plumbing (the seed must flow from the runner
  into `check`, which the harness can do by registering a `Random`-adjacent
  handler or a `TestSeed` effect around each test). Rendering an arbitrary
  `T` in the report hits the known `[interp-to-str]` limit for generic
  containers (ROADMAP leftover) — `check` may need an explicit `show`
  parameter for generic payloads until that lifts, or accept degraded
  rendering.
- Trade-offs: no syntax, ships incrementally, and the combinator library is
  where all the real design content is anyway (integrated shrinking per §2:
  a `Gen<T>` carries its own shrinker, so shrunken values respect the
  generator).

**Option B — parameterized tests as syntax.** `test "name" (xs: List<Int>)
{ … }` — a test with parameters *is* a property test; the checker resolves
a generator per parameter type (a `params`-style implicit, the same
machinery `Yield<It, T>` uses), custom generators by annotation.

- Cost: syntax + an implicit-resolution rule now, before the library
  underneath even exists; the annotation spelling for "this Int should be
  drawn from `ints_up_to(10)`" is a fresh design question with no Salvo
  precedent; and it can be added *later* as pure sugar over A with nothing
  wasted.
- Trade-offs: the prettiest form (QuickCheck's legacy), and Salvo's
  type-directed implicits genuinely fit it. Sequencing is the argument, not
  desirability.

**Recommendation (the user's call):** A now with determinism shape (i) —
the std intrinsics are the smaller long-term liability and by-construction
parity is worth more than a smaller std diff — and B recorded in ROADMAP as
the sugar pass once A exists. Shrinking integrated (the `Gen` carries it),
seed printed on every property failure, `--seed` on the runner. How a
generator is *chosen* for a qualified type — the round-2 question — is
§TF-5b, and B's attractiveness rises with it: the parameterized form
`test "…" (n: Positive Int)` becomes meaningful exactly when
`Gen<Positive Int>` is derivable.

> **Round 3 — partly superseded.** The selection mechanism was decided in
> §TF-5b (`?generate`), and with it the `Gen<T>` *struct* shape above is
> likely unnecessary — generators are plain overloaded functions over a
> threaded `Mut Rng`, and shrinking moves to the draw stream. The
> determinism decision (shape (i): std wrapping/bit intrinsics, PRNG in
> pure Salvo) and the seed/replay surface remain open as recommended. See
> "Round 3 — the decided list".

## §TF-5b. Qualifier-driven generation (round 2)

**The question.** Should qualifiers control what a property test generates —
a parameter typed `Positive Int` or `NonEmpty List<Int>` drawing only values
that satisfy the claim — and does making that ergonomic justify amending the
qualifier rules themselves? Folds §0 point 11, under the steering license
that qualifiers are still fluid.

**Why the idea fits.** A qualifier is precisely a machine-checkable claim
about a value, and property testing is precisely "produce values under a
claim, check a property over them" — QuickCheck-family libraries reinvent a
claim vocabulary (`Positive`, `NonEmptyList` *newtypes* in Haskell's
QuickCheck) that Salvo already has in its type language. Better: `is` is
already backed by `qualifies` at runtime [is-qualifies], so a framework can
produce a value and *honestly narrow it* — a generated `Positive Int` is not
trusted, it is checked, and the type on the test parameter is earned the
same way it would be in production code.

**The kind split does most of the design.** The three qualifier kinds
[qual-subject] map to three generation stories, and the split is a feature:

- **Predicate qualifiers** [qual-predicate] carry `qualifies`, so a
  *derived* generator exists mechanically with no declaration anywhere:
  filter the base type's generator through `qualifies`, shrink by re-checking
  every shrink step (integrated shrinking keeps the claim through
  shrinking, which is the classic QuickCheck failure mode §2 noted). Two
  caveats. Sparse predicates make rejection sampling hopeless (`Sorted`
  almost never falls out of random lists) — those need a *constructive
  override*: generate, then transform, then re-check (`sort` the list, and
  `is Sorted` passes by construction). And `qualifies` may declare effects
  [is-qualifies-effects], so deriving from an effectful predicate makes the
  generator effectful — legal in a test body, but worth a diagnostic
  nudge toward a pure predicate. A struct predicate's field overrides
  [qual-field-override] compose here for free: generate the overridden field
  types, re-check the whole.
- **Constructive qualifiers** [qual-constructive] have no predicate by
  definition — values exist only through constructors [qual-ctor-fn] — so
  nothing can be derived: a generator must be *stated*, built over the
  constructors (`mapped_by(ints(), i -> ok(i))` for a result arm; union-typed
  parameters generally generate by picking arms via their constructors, no
  D4 dependency).
- **Provenance qualifiers** are content-*independent* — nothing about bits
  can establish an origin — so auto-generation is a category error, not a
  missing feature. Same answer as constructive (state a generator over a
  constructor), and the framework's "no generator known" diagnostic should
  say *why* for this kind.

So the real decision is not "can qualifiers drive generation" (yes, and the
predicate case needs nothing new) but **where the hand-written generator for
the non-derivable cases lives**:

**Option A — nowhere special: wire by hand at the use site.** `check`
takes the generator explicitly, always; a qualified type just documents what
the generator was built to satisfy. No resolution, no new rules.

- Cost: the claim is stated twice (once in the type, once in the wiring),
  and the parameterized-test sugar (§TF-5 Option B) can never work for
  qualified types, since nothing associates `Positive Int` with a generator.
- Trade-offs: zero design risk; where TF-5's library-first recommendation
  starts anyway, so A is the fallback, not a destination.

**Option B — a `generate` member on the qualifier itself** (a qualifier
amendment, licensed by the steering):

```
qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool { return int > 0 }
    fn generate(source: Gen<Int>) -> Gen<Int> {
        return mapped_by(source, i -> abs_or_one(i))
    }
}
```

The framework derives `Gen<Positive Int>` by running the member over the
base generator and re-checking with `qualifies` (trust but verify: a value
that fails the recheck is a test-infrastructure failure naming the
qualifier, so the type stays honest even against a buggy generator). For
constructive and provenance qualifiers the member is the *only* mechanical
path, and it is opt-in.

- Cost: the member's signature mentions `Gen<T>`, a *test-library* type —
  so either every qualifier-declaring module may import the test root
  (layering: production declarations referencing test vocabulary, and std's
  own `NonEmpty` declaring one would make `lib/std` depend on `lib/test`,
  the wrong direction under §TF-8), or `Gen<T>` moves into std, which drags
  the generator combinators with it. This is B's real problem and it is
  structural, not syntactic.
- Trade-offs: best locality (the claim and its generator in one place, the
  same argument [qual-ctor-same-file] makes for constructors), best
  discoverability, and the qualifier body already holds `qualifies`, so a
  second well-known member is idiomatic. A milder spelling with the same
  locality: a *specially-shaped fn in the qualifier's file* (the
  constructor precedent exactly — `fn positives(source: Gen<Int>) -> Gen<Int>
  gen Positive`), which keeps the qualifier body itself untouched.

**Option C (shape-changing) — no association syntax at all: type-directed
resolution over ordinary functions.** A generator for `Q T` is an exported
fn returning `Gen<Q T>`, anywhere in scope; `check` (and the future
parameterized sugar) resolves it implicitly by the parameter's type, the way
interpolation already resolves an element `?to_str` (`resolve_implicit_fn`
exists) and overloads already pick by scope [fn-overload-scope] — your
module's generator beats an imported one beats the derived-from-`qualifies`
default.

- Cost: implicit resolution rules to specify (ambiguity = error naming the
  candidates, per the house style); discoverability is grep-shaped rather
  than declared-in-one-place; the `resolve_implicit_fn` limitations ROADMAP
  records ([interp-to-str]'s two lifts) get a second customer and must be
  lifted for real.
- Trade-offs: zero new syntax, no layering problem (generators live in test
  code, importing test vocabulary where it is legal anyway per §TF-8), the
  override story is just scope, and std's qualifiers need declare nothing —
  a test that wants `Gen<NonEmpty List<Int>>` exports one next to its tests.
  This is Haskell's typeclass-instance model with Salvo's own scope ladder
  standing in for coherence.

**Recommendation (the user's call):** layered — (1) the *derived* filter
generator from `qualifies` for predicate qualifiers, free and automatic,
with re-check-on-shrink; (2) Option C for overrides and for the
constructive/provenance cases, since it adds no syntax and dodges B's
layering trap; (3) record B's milder spelling (the `gen`-marked fn in the
qualifier's file, constructor-style) as the locality upgrade to revisit
once §TF-8 settles who may depend on the test root — it can be added atop C
as a resolution rung rather than instead of it. Working label
[test-gen-qual].

> **Round 3 — decided: Option C, spelled as an implicit parameter.** The
> generator arrives through **`?generate`** — the existing implicit-parameter
> mechanism (`?cmp` on `sort` is the model: filled by looking for a function
> *named* `generate` whose type fits, resolved locally on the scope ladder,
> overridable by name at the call site). `check` declares it and the
> qualifier does the picking *through the type*:
>
> ```
> fn check<T>(runs: Int, ?generate: (Mut Rng) -> T,
>             property: (T) [Throw<Failure>] -> None)
> ```
>
> At `check(200, (text: ValidDate Str) -> { … })`, `T` binds to
> `ValidDate Str`, so resolution wants a `generate: (Mut Rng) -> ValidDate
> Str` — and the overload returning the qualified type is more specific
> than one returning bare `Str`, so the qualifier selects the generator
> exactly as the user asked. Consequences the mechanism buys for free:
> ambiguity is the standard implicit-resolution error naming the override
> (`generate = my_gen`); "no generator known" is the standard "nothing to
> resolve" error; and since an implicitly resolved function is
> **effect-free** by rule, randomness must arrive as a *value* — the
> seeded `Mut Rng` `check` threads — which is §TF-5's determinism decision
> enforced by an existing rule rather than by discipline. Shrinking moves
> to the Hypothesis model, which fits plain generator *functions*: shrink
> the recorded draw sequence inside `Rng` and re-run `generate` over it,
> so a shrunken value is always one the generator itself produced and the
> qualifier's claim survives shrinking by construction. The derived
> filter-from-`qualifies` generator (recommendation (1)) is subsumed
> rather than decided: a rejection-sampling `generate` is four lines to
> write, and compiler-synthesized ones are recorded as a later
> convenience. Test-only generators, and the test-only qualifiers they
> return, live in the companion file (§TF-2's round-3 note).

### Worked example — parsing dates, valid and invalid (round 3)

The production module ships a parser; nothing test-flavoured appears in it:

```
// dates.sv
export struct Date { year: Int, month: Int, day: Int }

// Accepts "YYYY-MM-DD". Rejects bad shapes, bad months, bad days —
// including 2023-02-29 but not 2024-02-29.
export fn parse_date(text: Str) -> Ok Date | Err Str { … }

fn days_in(year: Int, month: Int) -> Int { … }   // private helper
```

The companion `dates.test.sv` is the same module (private access: it may
call `days_in` directly), implicitly imports std.test, and declares the
test-only vocabulary — two qualifiers the shipped program never hears of,
both **constructive** [qual-constructive]: validity here is established *by
construction*, not by predicate, precisely so the claim does not secretly
call `parse_date` and make the property circular. The generators encode the
date *spec*; the parser is then checked against them:

```
// dates.test.sv — same module as dates.sv, loaded only by `salvo test`

qualifier ValidDate of Str      // constructive: minted below, by construction
qualifier InvalidDate of Str

// Constructors live with their qualifier [qual-ctor-same-file] — here,
// in the companion.
fn valid(year: Int, month: Int, day: Int) -> Str as ValidDate {
    return "${pad4(year)}-${pad2(month)}-${pad2(day)}"
}

fn corrupt(text: Str) -> Str as InvalidDate {
    return text
}

// The ValidDate generator: draw a real date, format it. Uses the
// *private* days_in, so February 29 is generated exactly when legal.
fn generate(rng: Mut Rng) -> ValidDate Str => rng: Mut {
    let year  = int_between(rng, 1600, 2400)
    let month = int_between(rng, 1, 12)
    let day   = int_between(rng, 1, days_in(year, month))
    return valid(year, month, day)
}

// The InvalidDate generator: start from a valid shape, break exactly one
// thing — so failures are near-misses, not noise the parser rejects for
// boring reasons.
fn generate(rng: Mut Rng) -> InvalidDate Str => rng: Mut {
    let year  = int_between(rng, 1600, 2400)
    let month = int_between(rng, 1, 12)
    let pick  = int_between(rng, 0, 3)
    return when {
        pick == 0 { corrupt("${pad4(year)}-00-${pad2(int_between(rng, 1, 28))}") }
        pick == 1 { corrupt("${pad4(year)}-13-${pad2(int_between(rng, 1, 28))}") }
        pick == 2 { corrupt("${pad4(year)}-${pad2(month)}-${pad2(days_in(year, month) + 1)}") }
        else      { corrupt("${pad4(year)}/${pad2(month)}/01") }   // wrong separator
    }
}

context "parse_date" {
    test "accepts every constructible date" {
        check(200, (text: ValidDate Str) -> {
            let outcome = parse_date(text)
            expect(outcome is Ok, "should parse: ${text}")
        })
    }

    test "rejects every near-miss" {
        check(200, (text: InvalidDate Str) -> {
            let outcome = parse_date(text)
            expect(outcome is Err, "should reject: ${text}")
        })
    }

    test "round-trips what it accepts" {
        check(200, (text: ValidDate Str) -> {
            let outcome = parse_date(text)
            if outcome is Ok {
                expect_eq(valid(outcome.year, outcome.month, outcome.day), text)
            }
        })
    }
}
```

What the example demonstrates, piece by piece: the two `generate` overloads
differ only in their **return qualifier**, and `?generate` picks between
them from the lambda's parameter type — the qualifier is doing the work the
user asked about. The qualifiers are companion-declared (§TF-2 round 3), so
the production surface is untouched. `days_in` being reachable is the
whitebox privilege paying off — the generator and the parser share one leap-year
truth *without exporting it*, and if they drift, the property fails.
`Mut Rng` is a value, so both generators state their mutation in an
ordinary deduction (`=> rng: Mut`) and determinism is [time-coupling]-style:
data in, no ambient source. On failure the report prints the seed and the
shrunken counterexample — shrinking replays the generators over a shrunken
draw sequence, so a shrunken "invalid" string is still one `corrupt` built.
(One grammar point for the build to settle: `expect(outcome is Ok, …)`
assumes `is` is legal as an ordinary `Bool` expression in argument
position — conditions are its documented home. If the grammar keeps it
condition-only, the surface answer is arm-shaped assertions —
`expect_ok(outcome)` / `expect_err(outcome)` — which read better anyway.)

## §TF-6. Actor testing

**The question.** What does the toolkit add for §0 point 5, given the
surface that exists ([actor-on-idle], [actor-watch], `ManualTime`, fault
sinks, [time-coupling])? This section is deliberately additive: the
concurrency design already made its testing decisions, and this phase
packages them.

**Option A — helpers only, no new semantics.** Four library pieces in the
test root (§TF-8):

1. **`settle(p)`** — the quiescence idiom as one call:
   `waitfor i: Reply<Idle> { on_idle(p, i) }` returning the `Idle`, plus
   `expect_settled(p)` asserting both counts zero. This is the
   ManualTime middle line ("not optional in spirit") made unforgettable.
2. **A probe** — Akka's TestProbe in Salvo terms: a generic recording
   handler (`spawn Probe<M>() on p` answering an addr the test hands to
   the code under test as its collaborator; wait-based
   `expect_received(probe, m)` / `received_so_far(probe)` reads via
   `waitfor`). Needs [actor-sendable] payloads and generic handler support
   only; nothing new. Where a protocol has several members, today's answer
   is one probe handler per effect written by hand (a handler must name the
   effect it implements) — a generated probe-per-effect is recorded as a
   possible later nicety, not first-slice.
3. **`expect_fault(target, body)`** — a `watch`-based helper: watches,
   runs, waits for the down notification, fails the test if the target
   survived. The complement of §TF-3's default recording sink (which fails
   tests on *unexpected* faults).
4. **The docs posture** — the testing chapter teaches [time-coupling]'s
   ladder: pass time in (no fake needed) → `ManualTime` → `TestTicker` over
   the timer, in that order, and `settle` between registration and advance.

- Cost: library code plus e2e coverage; the probe's ergonomics depend on
  rendering (`expect_received`'s failure message wants to show messages —
  same `[interp-to-str]` caveat as §TF-5).
- Trade-offs: everything a first release needs, nothing speculative; every
  piece is deletable independently if unused.

**Option B — a test scheduler mode.** A harness-controlled deterministic
scheduler (virtualized pools, controlled interleavings — Loom-style model
checking lite), so a test can *enumerate* orderings rather than settle
around them.

- Cost: a second scheduler implementation per backend runtime, a
  determinism contract far beyond what the language promises today
  (arrival order per queue is pinned; cross-pool interleavings are not),
  and a research-grade surface. Out of scale for this phase.
- Trade-offs: it is the only way to test race *absence* rather than race
  *avoidance*. Salvo's design already leans the other way (settle-then-
  assert, state-not-interleavings, deadlock reported statically
  [actor-deadlock-cycle]) — record B as future work with a customer
  required first.

**Isolation note (rides on §TF-4's decision).** In run-all-in-one-process
mode, an actor a test leaks keeps its pool's threads alive into the next
test. Pools are values with no shutdown surface today, deliberately. The
cheap discipline: the harness warns when a test ends unsettled
(`on_idle`-check after the body — its counts are exactly "what you leaked"),
and `--isolate` is the escape hatch. A pool-shutdown surface is *not*
proposed — it would be new language surface with exactly one customer.

**Recommendation (the user's call):** A entire, B recorded as out of scope,
the leak warning as described.

## §TF-7. Where it all lives — crates and std layout

**The question.** §0 point 2's architecture call: what goes in a new crate,
what must go elsewhere, and what is the new crate's seam?

**Forced placements first (no option changes these).** The `test`
declaration is parsed in `salvo-syntax`, checked in `salvo-core` (implicit
effects, name uniqueness, companion-file module rules), and its lowering
touches both backend crates however §TF-4 falls. The assertion/generator/
probe surface is library Salvo — the first draft placed it *inside* std
(`std/test.sv` or `std/test/`); **superseded in round 2** by §0 point 10:
it becomes its own library root, `lib/test` beside `lib/std`, so
`import test` reaches it as a whole-module import of a sibling namespace
[mod-import-module] rather than a corner of std, and §TF-8 owns the root
semantics (including whether the import is legal outside `salvo test`).
Either way [mod-used-only] keeps untested programs from emitting any of it,
and the PRNG-support intrinsics from §TF-5(i) stay in `lib/std` — generally
useful surface, and `lib/test` depends on `lib/std`, never the reverse.
None of this can live in a separate crate; the
"framework crate" question is about the *runner*.

**Option A — a runner crate: `salvo-harness`.** New workspace crate owning:
discovery over the checked `Program` (enumerate `test` declarations),
harness synthesis (§TF-4's generated registry + `main`), the report
format and its parser (the runner reads the harness's stdout), seed and
filter plumbing, the process model (batching, `--isolate`, timeouts,
crash recovery). `salvo-cli` adds the thin `test` subcommand that calls
into it, exactly as `run` composes compile+execute today. Name chosen to
collide with nothing: `salvo-testkit` (the *compiler's* dev-dependency
toolbox) keeps its name and purpose; the two never depend on each other.

- Cost: a crate boundary to design (input: a checked `Program` + config;
  output: synthesized source + a run plan); some duplication risk with
  `run`'s toolchain-invocation code, which wants extracting into a shared
  spot rather than copying.
- Trade-offs: matches §0 point 2; keeps `salvo-cli` the thin shell it is;
  gives the framework its own test suite and its own README.

**Option B — no new crate: the runner inside `salvo-cli`.** The subcommand
owns everything, as `platform generate` does today.

- Cost/trade-offs: fewer moving parts and one less seam to design — and
  `main.rs` is already 35k with the CLI's concerns; the harness synthesis +
  report + process model is the biggest subcommand yet and would double
  down on that. Fine for a spike; the crate is where it ends up anyway.

**Option C (shape-changing) — the harness as a Backend-trait client.** Make
the runner a *fourth kind of backend consumer*: a crate that implements
test-emission per backend (`salvo-harness-kotlin`, `salvo-harness-rust`),
mirroring the backend registry.

- Cost: only worth it under §TF-4 Option B (native-framework lowering),
  where per-backend emission genuinely differs. Under §TF-4 A, the harness
  is backend-*independent* by construction (it synthesizes Salvo), and this
  split would be structure without content.
- Trade-offs: listed because it falls out if §TF-4 goes the other way —
  the §TF-4 and §TF-7 decisions are coupled and should be taken together.

**Recommendation (the user's call):** A, taken together with §TF-4 A. Library
layout per §TF-8 (`lib/test/` with `assert`, `gen`, `probe` modules — names
to taste); a fixture example under `examples/` once running, with
`expected.txt` proving report parity.

## §TF-8. The library roots — `import std.*`, `test` beside it (round 2)

**The question.** §0 point 10 is a stated direction: imports mention `std`
(`import std.time`), the test surface is a sibling root (`import test`),
and the repository holds them at `lib/std` and `lib/test`. What needs
deciding is the rule it becomes and the sub-questions the respelling opens —
because it supersedes recorded behavior: today `std` is *not* a path
segment (the import-suggestion machinery corrects `import std.random.Random`
to `import random.Random` [diag-import-suggest]), and the 2026-09-18 import
decisions [mod-import] [mod-import-module] were written over rootless paths.

**Option A — two reserved library roots, clean break.** `std` and `test`
become reserved first segments naming the two shipped trees. `import
std.time` / `import std.random.Random` / `import test` are the spellings;
the bare forms (`import time`) become ordinary unknown-module errors whose
diagnostic names the `std.`-prefixed fix (the no-backcompat invariant:
the old spelling simply stops being the language, and [diag-import-suggest]
already has the machinery to point at the right path). A user source tree
containing a top-level `std/` or `test/` directory is an error at discovery
(the module path would collide with a reserved root). Core's *implicit
visibility* is untouched — nothing about reaching `println` changes — and
the redundant-import warning transfers to the new spelling (`import
std.core` warns as `import core` does today).

- Cost: the sweep — every `import` in `std/` itself (internal cross-module
  imports), every example, every corpus `.sv`, every inline test source,
  every spec snippet, plus the resolver's module-path table, the
  import-suggestion paths, [mod-import-module]'s prefix matching, the LSP
  quickfixes, and the `include_dir` embedding moving from `std/` to
  `lib/std` with `lib/test` joining it. Mechanical but wide; it is most of
  this decision's price and the reason to take TF-8 *early* — the
  framework's own tests and examples should be written in the final
  spelling, not swept twice.
- Trade-offs: reading an import line now says whose code arrives — the
  library's or the program's — which is §0 point 10's point; and the test
  root gets a natural extra rule with nowhere to live in the rootless
  scheme: **`import test` is legal only in test compilation** (`salvo
  test`; a production module importing the test root is an error naming
  why), which makes §TF-4's stripping story airtight — production binaries
  cannot reach harness vocabulary even by accident. One wrinkle to accept
  deliberately: the `tests/` *directory* (§TF-2's blackbox tier) and the
  `test` *import root* are near-homonyms; Go lives with `testing`/`_test.go`,
  and the alternative (renaming the directory tier `itests/` or the root
  `testing`) is available if the user finds it grating.

**Option B — `std.` optional, both spellings resolve.** Prefix matching
accepts `import time` and `import std.time` alike.

- Cost/trade-offs: this is a dual-accepting grammar, which the standing
  invariant refuses outright (no deprecation periods, no dual spellings) —
  listed only to record it was considered. It also makes the reserved-root
  collision *silent* (a user `time` module would shadow-or-race std's)
  instead of an error.

**Option C (shape-changing) — a general package-root concept.** `lib/<root>`
for any number of roots with a small manifest; `std` and `test` are merely
the first two, and a future vendored library is `lib/mylib` + `import
mylib.thing` with no new design.

- Cost: designing a package system's front door now — root registration,
  collision rules, possibly versioning pressure — for a phase that needs
  exactly two roots, both shipped inside the compiler binary.
- Trade-offs: it is where this ends up if Salvo ever has third-party
  libraries, and Option A is forward-compatible with it (two hardcoded
  roots become two entries in a table). Record the direction; build A.

**Recommendation (the user's call):** A — reserved roots `std` and `test`,
bare spellings refused with the corrected path named, `import test` legal
only under test compilation, reserved-root collision an error at discovery,
core implicit visibility unchanged — with C's table shape kept in mind so
the hardcoding stays shallow. Take TF-8 **before or with the first
implementation slice**, so every test the phase writes is already in the
final spelling. Working label [test-root] for the test-root availability
rule; the respelling itself amends [mod-import] / [mod-import-module] /
[diag-import-suggest].

> **Round 3 — decided, and simplified.** The `std.` prefix stands, and the
> sibling `test` root **falls**: because every test file **implicitly
> imports the standard test module** ([test-implicit-import]), nobody ever
> writes the import line, so the surface does not need a root of its own —
> it lives at **`std.test`**, one reserved root (`std`), one library tree
> (`lib/std`, with `lib/std/test/` inside it). What survives of the
> section: the clean break on bare spellings (`import time` refused naming
> `std.time`), the reserved-root collision error (now just `std/`), the
> unchanged core implicitness, and take-it-early. The availability rule
> transfers to the module: an **explicit** `import std.test` from a
> production file is refused, naming why — test files are the only place
> the surface exists, and they get it without asking. The implicit import
> enters the scope ladder at the bulk-import rung [mod-import-module], so
> a test file declaring its own `expect` wins silently, as any file
> beats a whole-module import today. The near-homonym wrinkle shrinks:
> `tests/` the directory and `std.test` the module no longer share a bare
> name.

## §TF-9. `context` scopes (round 3 — decided in shape)

**The decision.** The user decided test files support **`context` scopes**,
doing two jobs at once: **nesting tests** under readable names, and holding
**common initialization** — `use` registrations, `let` bindings — that
**re-runs for every test** rather than persisting across them
([test-context]):

```
context "with an in-memory filesystem" {
    use MemFs()
    let root = "/data"

    test "writes then reads back" { … }        // MemFs fresh, root bound
    test "missing file reports its path" { … } // a *different* fresh MemFs

    context "under a restricted root" {
        use RestrictedFs(root)                 // sees the outer bindings
        test "escaping the root is refused" { … }
    }
}
```

**The semantics the decision fixes.** A context body holds statements and
nested `test`/`context` declarations. For each test, the enclosing
contexts' statements run **outer-to-inner, in order, before the body** —
and they run *per test*, so there is nothing that could be shared: the
natural lowering is per-test inlining (each test's synthesized function is
its ancestors' preambles concatenated with its body), which makes
isolation true **by construction** rather than by a reset protocol. This is
RSpec/Kotest's `describe` + `beforeEach` collapsed into one construct with
the persistent-state variant simply not offered — the footgun ExUnit warns
about (shared mutable fixtures) cannot be written. Everything else follows
from existing rules: a `use` in a context scopes into the nested tests
exactly as it scopes into the rest of a function body today; bindings are
visible downward; a **linear** value bound in a context becomes each test's
own obligation, so "teardown" of tokens is [linear-obligation] doing its
job (each test must `close`/discharge what the preamble opened, or refuse
to compile) — no `afterEach` construct is needed for the things Salvo
tracks, and the things it does not track are what `MemFs`/`ManualTime`
fakes are for. A test's full id joins the names —
`fs_utils :: "with an in-memory filesystem" :: "writes then reads back"` —
which is what the report prints and the filter matches (§TF-4); name
uniqueness is per sibling scope.

**Sub-calls still open under the decision**, each with a recommendation:

- **Statement placement.** Preamble-only (every statement precedes the
  first nested `test`/`context`) versus interleaved (statements between
  tests, executing in declaration order for the tests below them...
  or for all?). Interleaving makes textual position semantically loaded in
  a construct that *re-runs*, which is exactly where readers guess wrong.
  *Recommendation: preamble-only, error naming the rule.*
- **What a preamble may do.** Full test-body powers (including `spawn` —
  a context-spawned actor is then per-test-fresh, usually what a fixture
  wants) versus a restricted statement set. The restriction has no
  principled line to draw; the powers are §TF-3's either way.
  *Recommendation: anything a test body may do.*
- **Empty and childless contexts.** A `context` containing no test runs
  nothing; silently legal or a warning? *Recommendation: warn, like the
  unused-variable warning — it is almost certainly a mistake.*

## Round 3 — the decided list (2026-09-19)

The user's calls, recorded here as the section-by-section outcome; each
superseded discussion above carries a pointer down to this state. All
propagation still owed (see the header note).

1. **The declaration** (§TF-1): `test "string name" { … }`, literal names,
   uniqueness per sibling scope, not exportable — in **test files only**.
2. **Whitebox = companion, always** (§TF-2): private-access testing happens
   in `*.test.sv` and nowhere else; in-file test blocks are dropped.
   Companions are the same module with one-way visibility, and **may
   declare test-only qualifiers, structs, and helpers** of their own.
   (The blackbox `tests/` tier stands as recommended — not explicitly
   re-confirmed.)
3. **The surface lives at `std.test` and arrives implicitly** (§TF-8):
   every test file behaves as if it wrote the whole-module import
   [test-implicit-import]; the separate `test` root is dropped; the `std.`
   import prefix (round 2) stands; explicit `import std.test` from a
   production file is refused.
4. **Generation is `?generate`** (§TF-5b): an implicit parameter filled by
   name-and-type over the scope ladder; the return type's **qualifier
   picks the generator**; randomness is a threaded `Mut Rng` value (so
   resolution's effect-free rule enforces determinism); shrinking replays
   generators over a shrunken draw stream. Worked example: the date
   parser, with constructive `ValidDate`/`InvalidDate` companion
   qualifiers.
5. **`context` scopes** (§TF-9): nesting plus per-test re-run
   initialization, no persistent cross-test state expressible.

One knock-on worth stating: §TF-5's recommended `Gen<T>` *struct* (a
generate/shrink pair as data) is likely **unnecessary** under decision 4 —
generators are plain overloaded functions and shrinking lives at the `Rng`
draw-stream level. The combinator vocabulary (`int_between`, choosing,
sizing) survives as ordinary functions over `Mut Rng`. §TF-5's determinism
shape (i) (std wrapping/bit intrinsics, PRNG in pure Salvo) is
*recommended, still undecided*.

## Decisions pending

Round 3 decided TF-1, TF-2, TF-5b, TF-8 (revised) and added TF-9 — the
decided list above is authoritative for those. **Still open**, with the
load-bearing order among them: **TF-3 first** (the failure channel — every
assertion and the worked example write it), then **TF-4 with TF-7**
(coupled: the compilation model decides the crate seam), then TF-5's
remaining determinism/replay calls and TF-6 (each independent once the
core stands). TF-9's three sub-calls are small and can ride the
implementation review.

Still open:

| Label | Question | Folds | Recommendation (one line) |
|---|---|---|---|
| TF-3 | Body powers and the failure channel | §0.3, §0.5 | `main`-like implicit powers; failure = `[Throw<Failure>]`, harness delimits with `try`; recording fault sink by default |
| TF-4 | The runner and compilation model | §0.1, §0.7, parity | `salvo test` synthesizes a Salvo harness `main`; one report format, byte-identical per backend; argv dispatch, crash-recovery isolation, `--timeout` |
| TF-5 (rest) | Determinism source and replay surface | §0.4, §0.9 | Pure-Salvo splitmix64 over new std wrapping/bit `Long` intrinsics; seed printed on failure, `--seed` replay |
| TF-6 | Actor-testing toolkit | §0.5 | Helpers only: `settle`/`expect_settled`, a probe handler, `expect_fault`, the [time-coupling] docs ladder; leak warning; no test scheduler |
| TF-7 | Crate layout | §0.2 | New `salvo-harness` crate for the runner; declaration in syntax/core; thin CLI subcommand |
| TF-9 (subs) | `context` details | round 3 | Preamble-only statements; full test-body powers; warn on a childless context |
| TF-2 (rest) | Blackbox `tests/` tier confirmation | §0.6 | Keep as designed: ordinary modules seeing exports only, loaded only by `salvo test` |

Decided (round 3; see "Round 3 — the decided list" for the full statements):

| Label | Outcome |
|---|---|
| TF-1 | `test "name" { … }` blocks, in test files only; literal names, sibling-scope uniqueness, no `export` |
| TF-2 | Whitebox always via `*.test.sv` companions (same module, one-way visibility); companions may declare test-only qualifiers and data; in-file blocks dropped |
| TF-5b | `?generate` implicit parameter, qualifier-directed by return type; `Mut Rng` value threading; draw-stream shrinking |
| TF-8 | `std.` import prefix; surface at `std.test`, implicitly imported by every test file; explicit production import refused; single `lib/std` tree |
| TF-9 | `context` scopes: nesting + per-test re-run initialization; no persistent cross-test state |
