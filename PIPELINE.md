# The compiler pipeline: phases, sub-phases, and caching (working document)

**Status:** decided; ready to build. Rounds, all 2026-10-20: R1 (D1, D3), R2
(lends, mutated fields, D4), R3 (`iter T` removed), R4 (state structs and
names), R5 (parameter holding, state field types). The plan of work is §7;
§5 ends with what stays program-wide.

**Goal.** Organize the compiler into named phases and sub-phases, each defined by
its inputs and outputs, so that:

- each phase's output can be dumped and tested on its own;
- results are cached per **file** or per **declaration**, and the language
  server recomputes only the declaration being edited (and whatever actually
  depends on it);
- the whole program is visited at one point only, as late as possible. The
  user's reading: deadlock analysis is the one analysis that needs it.

**Charter.** As with the other option-space documents: the calls are the
user's, recorded as dated rounds below. When the work lands, the reasoning goes
to COMPLETED.md's decision log, anything undecided to ROADMAP.md, the rules to
LANGUAGE_SPEC.md, and this file is deleted.

**Terms used here.**
- **Interface** of a declaration: what other declarations may know about it
  (a fn's signature, deduction contract, effects, implicits, lends; a struct's
  fields; an effect's members and prerequisites). The rule this design is
  built on: *an interface is computed from the declaration's own text and
  other interfaces, never by type-checking a body.*
- **Body**: everything else (a fn's statements). Checking a body reads
  interfaces only.
- **Index**: a lookup structure over every declaration's *header* (name →
  declaration id), rebuilt cheaply from per-file declaration lists. An index
  is program-wide but is not a **sweep** (a pass that walks bodies or does
  real work over every declaration).
- **Early cutoff**: when a declaration is edited and its re-derived interface
  is unchanged, nothing that depends on it is recomputed.

---

## 1. The pipeline as built (survey, 2026-10-20)

```
01 parse     per file. lexer::lex → Vec<Token> → Parser → AST
             (parser lowers some sugar: +=, !is, use…on, pair sugar,
             trailing blocks, leading-dot); then flatten_namespaces
(expand)     whole program, AST → AST, before resolution:
             hoist_struct_fns, hoist_iter_types, iter fn expansion, tests,
             comptime, route stubs, effect aliases, prerequisites
02 resolve   Symbols::collect + resolve(): one scope per file
   check     check_program: ≥ 2 rounds over every body (round 1 discarded);
             types, overloads, narrowing, ownership, linearity, effects in
             one walk. Output: Checked, ~110 tables keyed (file, byte span)
03 IR        build_program copies Checked into IR nodes (infers nothing);
             ir::analyze checks linear leaks (clean programs only)
04 emit
```

Facts the design rests on:

- **Narrowing feeds overload resolution**, so it belongs with type inference,
  not after it: `if s is Str { size(s) }`, `assert!(xs is NonEmpty)` making a
  `NonEmpty`-only overload match, a callee's `=> list: Mut` dropping `NonEmpty`
  so a second call fails. What looks separable from typing is **ownership**:
  moves, borrows and `proj` links, poison, linearity, contract validation
  (not yet audited in full).
- **Call resolution needs argument types** (overloads, generics, literal
  adoption, lambdas typed from the parameter), so "insert `let t: Int = f()`"
  and "resolve which `f`" are one sub-phase. Binding names to locals is
  lexical and can come first.
- **`Ty::Unknown` already poisons quietly** [type-unknown-lenient].
- **Identity is positional**: `FnKey`/`DeclId` are (file, item index), type
  declarations are memory addresses, and `Checked` keys are absolute byte
  spans. An edit near the top of a file changes every later key. A type's key
  (`Token§args.lex`) depends on which module declared the name *first*.
- **`Symbols<'p>` and `Resolution<'p>` borrow the `Program`**, so nothing can
  be cached across analyses as it stands.
- **The language server** keeps a parse cache (by content) and the std cache,
  and re-runs a full analysis for every request (hover, completion,
  go-to-definition), with no analysis kept between them. ~449ms full, ~61ms
  with the std cache (COMPLETED.md, 2026-10-10); no per-phase breakdown exists.

## 2. The target pipeline

| phase | scope | contents | output |
|---|---|---|---|
| 01-parse-01-lex | file | tokens, comments | `Vec<Token>` |
| 01-parse-02-ast | file | AST, syntactic sugar, namespace flattening, struct-fn hoist, tests | AST |
| 02-resolve-01-declare | file | each declaration's header and stable id | per-file declaration list |
| 02-resolve-02-scopes | file | imports, aliases, renames, rungs, against an index of headers | per-file scope |
| 02-resolve-03-interfaces | declaration | signature types lowered, default deductions, effects (aliases and prerequisites resolved here), implicits, refinements, lends, `iter fn` structs, `by` member signatures | interface |
| 02-resolve-04-comptime | `by` site | stamped bodies, after resolution (D3) | bodies |
| 02-resolve-05-bodies | declaration | local binding, type inference, overloads, narrowing, picks; reads interfaces only | per-declaration facts (the pre-IR, D1) |
| 03-generate-01-lower | declaration | pre-IR → IR: effects as parameters, pass modes, union wrapping, narrows as nodes | IR declaration |
| 03-generate-02-flow | declaration | ownership, borrows, linearity, contract validation | diagnostics, IR annotations |
| 03-generate-03-link | program | indexes for actor glue, `ref` containers, throws; **deadlock** | program IR |
| 04-emit | | per backend | target source |

Each declaration-scoped step records which interfaces it read; a step is
recomputed when its own text, or one of those interfaces, changed (early
cutoff, §6).

## 3. Decisions

### R1 (2026-10-20)

- **D1. The pre-IR is per-declaration facts plus a dumper (option a).** The
  AST stays; the checker's facts are stored per declaration, keyed by
  declaration id and a span *relative to the declaration*, and a dumper prints
  the AST with types and targets filled in. A real typed tree (option b) is
  not ruled out later. The relative key is what makes a body's facts survive
  edits elsewhere in its file, and what lets the language server find the
  declaration under the cursor first and search only its facts.
- **D3. Comptime moves later in the pipeline, and the early whole-program
  sweeps are removed, as part of this design** (not deferred). Everything
  stays file- or declaration-scoped as long as possible; §5 is the plan per
  sweep.

### R2 (2026-10-20)

- **D2, lends (4.2): written, with a sound default (option a).** An unwritten
  bodied fn lends every kept, non-Copy parameter; `holds proj(…)` narrows it;
  the body is validated, never consulted. The desugaring writes `holds` for
  `iter fn` minters.
- **D2, mutated fields (4.3): the inference is dropped (option a).** Unwritten
  means "mutated anywhere".
- **D2, move-mode and parking (4.4, 4.5)** need no language change and go ahead
  as recommended.
- **D4: hand-rolled caching first (option H)**, written so salsa stays
  possible (§6).

### R3 (2026-10-20)

- **4.1: option (f). `iter T` is removed from the language**; `iter fn` stays.
  The user's reading: `iter T` existed mostly so that `Iter<C, T>` need not
  write the iterator parameter, making `C` look like the iterator, and that
  is unnecessary.
- **Every `iter fn` names its struct** (the simpler rule).
- **An obligation writes the iterator type**: `struct Bag : Iter<self,
  BagIter, Int>`, not inferred from the `iter` that fulfils it, at least
  until a better way is found. Spreads write the generic too: `fn total<C,
  It>(c: C, ?Iter<C, It, Int>)`.

### R4 (2026-10-20)

- **The state struct is declared by its owner, in a `state` block**, which
  brings it into scope the way `use`, `spawn` and `try` bring theirs: not in
  the effect list, which says what a *caller* provides, and a minter's or a
  handler's caller provides no state. An `iter fn`'s block runs once, at the
  mint, where an ordinary body's `state` statement would run where written;
  the `iter` on the declaration says so.
- **An `iter fn` names its state struct**: `state Range { i: Int = start }`.
  The name is declared in the scope the fn itself is declared in (its module,
  or its namespace), so two `state Range` there collide, as two `struct Range`
  would, and so does a `state Range` beside a type `Range`.
- **A handler's state is named by the handler** (`[state Counting]`) when that
  is unambiguous, and must be named explicitly (`state Tally { … }`) when the
  handler shares its name with a type.
- **One namespace of bare names per declaration**: a handler's constructor
  parameters and state fields, and an `iter fn`'s parameters and state
  fields, all differ, and a member's (or the step's) parameters and locals
  reuse none of them. A clash is an error naming both; the remedy is a
  rename. It follows the language's refusal of shadowing.
- **The handler defects of ROADMAP §2d item 9d are fixed with the state
  work.**

### R5 (2026-10-20)

- **A named state holds every parameter** of its `iter fn`: Copy scalars
  copied, the rest borrowed whole. The struct's shape is then a function of
  the header; the per-field snapshot (`Hold::Fields`) and its cross-file
  struct lookup go.
- **Inferred `iter fn` state field types stay (4.6, option b)**: initializers
  are checked in 02-resolve-03-interfaces against interfaces only, as
  parameter defaults are.
- **The general `state` effect** (a `state` statement in any body, `[state
  S]` helpers) is a language feature of its own, not a pipeline question: a
  `[state S]` entry is part of a signature like an effect, resolved in
  02-resolve-03 and lowered to a leading parameter in 03-generate-01, so the
  plan does not wait on it. Recorded in ROADMAP §2d for the user's design.
  Step 4's forms (named states, the bare-name rule) are written so that it
  can extend them.
- **Ownership does not change a type or an overload** (the user's
  expectation); step 7 verifies it before moving ownership out of the body
  walk.

### R6 (2026-10-20)

- **Hover shows what the code is interpreted as**: an effect list (and
  anything else the resolve phase expands) is shown expanded and resolved —
  aliases flattened, prerequisites added — read from the resolved output,
  not the text as written.
- **The refusal of a mixed handler whose face has a prerequisite is
  dropped**, for a uniform rule: an implied dependency is a dependency like
  a written one, wherever it is allowed.

### R7 (2026-10-20)

Route stubs, derived per effect in the IR (step 6, piece 5):

- **The "no route stub for `X` in this module" refusal is dropped**: a
  `use` routes whatever protocol its group's resolved type names.
- **A protocol member marking more than one `Key` is refused at the effect
  declaration**, whether or not anything routes it.
- **One stub per routed protocol, in the protocol's module**, and only for
  protocols something routes.
- **`spawn` of a route stays unsupported** (the addr would bind a strong
  `X`; ROADMAP ⑥).
- **Respelled as a modifier on `use`**: `use any stock` (a group handle,
  `eff[ActorGroup<X>]`) replaces `use route_any(stock)`, so the statement
  reads like the availability it binds, `any X`; `use any` on anything but
  a group handle is an error.
- **The wait settings live with the policy (option A)**: `RouteSelector<E>`
  gains `fn config() -> RouteConfig`, and std's policies take `config:
  RouteConfig = default_route_config()` as a constructor parameter, so
  `use Sharded<Inventory>(config = RouteConfig { … })` overrides them. Needs
  defaults on handler constructor parameters ([fn-param-default] extended),
  built first as its own piece.

### R8 (2026-10-20)

- **Ownership leaves the body walk straight for the IR (route b).** Typing
  runs ownership-blind (a consumed value keeps its declared type), and
  03-generate-02-flow on the IR owns fate, poison, partial moves,
  move-mode, lends validation and linearity; the checker's re-walk loop
  goes. Ownership errors exist where the IR exists — per declaration that
  type-checks — so a body with a type error reports its ownership errors
  once the type error is fixed.
- **Amended (user, 2026-10-14): static move-site rules stay in the
  checker.** A rule that reads no flow state — a lambda moving or mutating a
  capture it may not, a value stored into handler state, a written lend list
  checked against what the body visibly lends — is judged where the checker
  already sees the lambda boundary, the call contract and the handler's
  declaration. What those rules consume reaches the flow pass as moves
  (`lambda_captures`, `moved_reads`, `flow_moves`), so the flow pass owns
  every *path-dependent* ownership question and nothing else.

## 4. Body-derived interface facts (D2)

Six facts a declaration's callers read today come from type-checking its
body. They are why the checker runs whole-program rounds (§1), and each
violates the interface rule. Findings and options per fact:

### 4.1 `-> iter T` return patterns [iter-type] (decided R3, R4)

**The problem.** An `iter fn` declares a minter and a step, and the desugaring
writes the iterator struct, which had no name a program could write
(`__Iter_range_Int_Int_Int`). A fn handing such an iterator out wrote `->
iter Int`, filled from its body:

```
fn range(end: Int) -> iter Int { return range(0, end) }
for i in range(5) { … }
```

`for` needs `range(5)`'s concrete struct to find its `next`, so every caller
depended on `range`'s body: the iter-return pre-pass, repeated until nothing
changed. Three such fns in the tree, all one-line delegations.

**The root cause** (R3): `iter T` was one spelling for three answers to *who
picks the struct*: the caller (a parameter, a hidden generic per occurrence),
the implementation once per spread or obligation (a group member, a hidden
group parameter), or the callee (a return, a state-field pattern, which a
caller can learn only from the body). The last needs a name; the first two
only hid a type parameter, so that `Iter<C, T>` read as if `C` were the
iterator. `iter T` appeared on 6 lines of the tree against about 100 lines of
compiler machinery, an early sweep (`hoist_iter_types`) and a body-derived
interface with its own fixpoint. A probe (`tmp/itergrp`) showed the group
itself was sound: one hidden generic per spread, and a mismatched pair is a
type error. It also found ROADMAP §2d item 9c.

Options weighed and set aside: reading the struct off the `return`s
syntactically (restricting `return` to a call decided by name and arity); an
opaque type at the interface, like Rust's `impl Iterator`; removing only the
delegating form; and placing the name in the effect list (`[state Range]`),
a deduction (`=> state Range`), as a curried return, or as a separate
top-level `state struct`.

**The decided surface.**

```
iter fn range(start: Int, end: Int, step: Int) -> Emitted Int | Finished {
    state Range {
        i: Int = start
    }
    …
}
fn range(start: Int, end: Int) -> Mut Range {
    let step = when { start < end { 1 } start > end { -1 } else { 0 } }
    return range(start, end, step)
}
fn range(end: Int) -> Mut Range { return range(0, end) }

params Iter<C, It, T> => iter with next {
    fn iter(collection: C) -> Mut It
    fn next(iterator: Mut It) -> Emitted T | Finished => iterator: Mut
}
struct Bag : Iter<self, BagIter, Int> { items: List<Int> }
iter fn iter(bag: Bag) -> Emitted Int | Finished {
    state BagIter { at: Int = 0 }
    …
}
fn total<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c { … }
fn first<It>(it: Mut It, ?Yield<It, Int>) -> Int => it: Mut { … }   // was `it: iter Int`

handler Counting() of Counter {
    state {                       // named `Counting`
        n: Int = 0
    }
    …
}
```

- `iter T` is gone from every position, and with it the marker type, the
  hidden generics (`desugar::hoist_iter_types`), the iter-return pre-pass and
  its fixpoint, and the lends special case.
- Every `iter fn` names its struct; the struct's interface is a function of
  the declaration's header (its parameters, see below, and its state fields'
  written types or header-checked initializers, 4.6).
- `[state S]` stays available for helpers that need a caller's state (the
  general form, below).

**Holding the parameters (decided R5: every parameter).** Today an `iter fn`'s struct holds a
parameter only if the step body reads it (copied when a Copy scalar, else
borrowed, or borrowed per field read). With a named state, a helper `fn
skip_spaces() [state Lexer]` reading `src` needs `Lexer` to hold `src`
whatever `tokens`' own body reads, and the shape would otherwise depend on a
body. **Recommended: a named state holds every parameter**, Copy scalars
copied and the rest borrowed whole; the cost is an iterator borrowing a
parameter it never reads for as long as it lives. (The per-field snapshot of
a struct parameter, `Hold::Fields`, then goes too, unless kept for the
fields the declaration names.)

**The general form (a separate feature, ROADMAP §2d; R5).** The ways a state reaches a body:

| form | where | who builds the value | lifetime |
|---|---|---|---|
| an `iter fn`'s `state S { … }` | the declaration | the minter, once | the iterator's |
| a handler's `state { … }` | the handler body | construction, once | the handler's |
| a `state` statement, `state Cursor { src: text, at: 0 }` | any body | that statement | the rest of the block |
| `[state S]` on a fn | the signature | a caller with an `S` in scope | the call |

Uses, each standing on its own: helpers of an `iter fn`'s step (`fn
skip_spaces() [state Lexer]`, called from `tokens`'s step); helpers of a
handler's members (`fn record(k: Int) [state Tally]`); threading a cursor or
a machine in an ordinary body (a parser's `parse_expr() [state Cursor]`), the
classic state effect (Koka's `state<s>`, Haskell's `State s`). Only `iter fn`s
and handlers *declare* a state struct; the general form uses ordinary structs.
To decide: two states of one type in scope (refused, as [use-no-dup]); a
lambda reaching a state (its type carries `[state S]`, as with effects);
states and actors (not shareable); reading the final state after a `state`
statement's block.

### 4.2 Inferred result lends [proj-infer] (decided R2: option a)

**Today.** Which parameters a result holds borrows of (a struct with `proj`
fields, `List<proj T>`, `iter T`) is inferred from the body when no `holds
proj(…)` is written (`lends.rs`). The inference is syntactic, but it walks
**callee bodies** transitively, and is memoised per file (so a callee is
re-walked once per file). Consumers: the caller's fate links (`lending_calls`,
`temp_views`) and the Rust lifetimes (`FnDecl.borrows`).

**Uses.** 5 std container minters (`iter` over list, array, bytes, deque, str;
each `return Mut XYield { items: p, at: 0 }`), `window` in
`examples/borrowing`, the 16 `iter fn` minters (the desugaring writes no
`holds`), and the 3 pattern fns of 4.1 (the only cross-body walks).

**Options.**
- (a) **Written, with a sound default** (the shape [deduce-default] gave
  contracts on 2026-10-11): an unwritten bodied fn lends every kept,
  non-Copy parameter; a written `holds proj(…)` narrows it; the body is
  validated against the interface ("declared covers inferred", as today) but
  never changes it. The desugaring writes `holds` for `iter fn` minters (it
  knows what the struct holds); the 5 std minters and `window` already have
  exactly one non-Copy parameter, so the default is exact for them. A
  language change: [proj-infer]'s "infer what can be inferred" stops for
  interfaces.
- (b) **Keep inference, read callee interfaces instead of callee bodies**: a
  per-declaration summary computed from its own body plus callees' summaries,
  in call-graph order. No language change; a body edit can change its
  interface, so its callers are rechecked (early cutoff limits it).

**Recommendation:** (a). Note `kept_params` must skip Copy scalars for the
default to be right for `range(0, n)` (today it does not).

### 4.3 Inferred mutated field paths [deduce-field] (decided R2: option a)

**Today.** A callee with no written field entry (`=> h.tags: Mut`) has the
fields it mutates read off its body, and callers use them for
[fate-field-disjoint] and [qual-field-place] precision. This is the one fact
that genuinely needs the cross-declaration fixpoint (one more round per level
of call depth; a fact still changing at round 4 is silently dropped).

**Uses.** std writes no field entries, and under the std cache std callees
already get the whole-value default (their paths are never re-inferred), so a
cached and an uncached check can differ in precision today. The two field
entries in `examples/borrowing` are written. The inference has two tests in
`deduce_tests.rs`.

**Options.**
- (a) **Drop the inference**: unwritten means "mutated anywhere", the meaning a
  written `=> h: Mut` already has. Only precision is lost; no wrong program is
  accepted. A language change (the "Inferred where unwritten" bullet of
  [deduce-field] goes).
- (b) Call-graph-ordered body summaries (as 4.2 (b)).
- (c) Move field-precise fate to 03-generate-02-flow, after all bodies; the
  checker would have to defer the poison errors that precision can spare.

**Recommendation:** (a).

### 4.4 Move-mode candidates, claims, mutations [fate-move-mode]

**Findings.**
- **Candidates are intra-body**: produced and consumed by the same body (bind
  spans of its own locals), and every input they need is signature-only since
  2026-10-11. A per-body second walk replaces the program round, as the
  iter-return pre-pass already does with a scratch `Checked`.
- **Claims are dead**: `deduce::infer` ignores them (`let _ = (&claims,
  &mutations)`), nothing else reads them, and they can still force a useless
  third round through the stability test.
- **Mutations** are read only after checking, by `param_mode` (a mutated
  generic `c: C` passes `LentMut`), and through the std cache. They never feed
  back into checking.

**Recommendation** (no language change): a local two-pass per body for
candidates; delete claims; publish mutations from the final body check for
03-generate.

### 4.5 `parking_handlers` [actor-replyto]

**Today.** Handlers whose members mint a continuation to their own member
(`replyto k(…)`, `self@Face`) are found while checking those bodies, and read by
other bodies' `use` refusal, deadlock, and the IR (`stateful`). It relies on
round 2 having seen every handler.

**Finding.** Decidable from the handler declaration's own text by name lookup:
`replyto k` resolves lexically against the handler's own members, and
`self@Face` against its `of` list. The syntactic version over-approximates only
for programs that are in error anyway.

**Recommendation** (no language change): an interface fact of the handler,
computed syntactically. The spec already describes the gate as syntactic.

### 4.6 `iter fn` state field types [state-infer] (decided R5: option b)

**What is left.** A state field writes its type (`at: Int = from`) or has it
inferred from the initializer (`at = from`), as `let at = from` infers (user
decision 2026-10-20, for handlers and `iter fn`s alike). The pattern form
(`inner: iter Int = …`) went with `iter T` (R3). A handler's state types stay
inside the handler, so only an `iter fn`'s matter here: they are the fields of
a struct other declarations read (its lends, wire form, linearity, emitted
shape), and today they are found by checking the minter's body.

```
iter fn chunks(xs: List<Int>, n: Int) -> Emitted List<Int> | Finished {
    state Chunks {
        at = 0                       // inferred: Int
        buf = Mut List<Int>()        // inferred: Mut List<Int>
    }
    …
}
```

**Uses.** None in std, examples or modules: all 113 state fields write a
type. Only tests use the inferred form.

**Options.**
- (a) Require a written type on `iter fn` state. Nothing to rewrite; takes
  back part of the 2026-10-20 decision, and the two `state` blocks differ.
- (b) **Keep inference, and check initializers in 02-resolve-03-interfaces**
  against interfaces only, as a parameter default is checked
  (`check_default`): an initializer reads only the parameters and calls
  functions, with no effects, so it is a header expression, not body code. A
  cycle is an error.
- (c) As (a), for handlers too.

**Recommendation:** (b).

### 4.7 What removing them buys

With 4.1–4.6 settled as recommended, no interface depends on a checked body
(an `iter fn`'s state initializers are header expressions, 4.6),
the program rounds go away (`check_program_rounds`, `MAX_ROUNDS`,
[deduce-fixpoint]), and every body is checked once, plus a local second walk
where move-mode needs it. Today every body is checked at least twice, plus the
iter pre-pass. The `self.inferred.is_some()` gates (about 30) become "final
walk of this body".

## 5. Whole-program sweeps, and where each goes (D3)

Ratings: **(A)** already file/declaration-scoped; **(B)** becomes so with an
index; **(C)** needs a design change; **(D)** inherently whole-program.

| sweep | today | plan | rating |
|---|---|---|---|
| `hoist_struct_fns`, `expand_tests` | per file, in `expand` | move into 01-parse | A |
| `hoist_iter_types` | needs every `params` group (one bit: does a member mention `iter T`), by plain name | removed with `iter T` (R3) | — |
| `iter fn` expansion | needs the subject struct's fields from any file (plain name, first match), for per-field snapshots | expand per file; if a named state holds every parameter (4.1), the snapshot and its lookup go | B |
| comptime | flat `World` of every struct, type, compfn, group, qualifier; writes stamps into the site's module | the `by` clause declares the member signatures (group members with `self` substituted) at 02-resolve-01; bodies stamped per site in 02-resolve-04 from resolved declarations | C |
| route stubs | unscoped effect table, exported-name index, synthetic imports | `use route_any(g)` checked as "a handler of the resolved `X`"; the stub derived per effect in 03-generate (where `actors::generate` already derives stubs) | C → B |
| effect aliases | rewrites every effect list, synthetic imports | resolved in scope like type aliases; flattened in 02-resolve-03 | B |
| prerequisites | rewrites every effect list, synthetic imports | a per-effect closure over effect headers, applied in 02-resolve-03 | B |
| `Symbols::collect` keys | `Name§module` by first declarer | module-qualified ids always; clashes decided when rendering | B |
| `resolve` | pass 1 by module, pass 2 per file | header index + per-file scopes | B |
| `written_keys`, `_by_span` | walk every type reference, each round; by-span map has no file index | per file (part of interface/body resolution); drop the by-span map | A/B |
| protocol hashes | over all actor effects | per effect interface | B |
| `refine::collect` | once, program-wide | per file, through scopes | B |
| `platform_closure_all` | closure over declarations | closure over interfaces | B |
| implicit-signature pre-pass | every fn, before bodies | *is* 02-resolve-03 | A |
| `check_module_uses`, `check_runtime_layers`, intrinsic rule | per-file loops | per file | A |
| `deduce::infer` | plain-name tables (provenance, effects, proj fields); body validation reads checked facts | interfaces per declaration (signature-only); validation moves to 03-generate-02 | B |
| IR `compute_names`, `throws_map`, `refs::annotate` | per module / summary + index | unchanged | A |
| IR `actors::generate` | interfaces across modules, plain-name fallback | index by key; drop the fallback | B |
| deadlock | gates and sends of every handler | 03-generate-03-link | D |

Also: about 21 checker sites, the `deduce` tables, comptime's and the
desugaring's first-match tables, and the IR's fallback look names up
**program-wide by plain name**, ignoring scopes. They make one declaration's
result depend on unrelated same-named declarations, and some are latent bugs
(two modules each declaring `Token`). Each becomes a lookup through the
file's scope or by id.

### What stays program-wide

After the plan, **deadlock is the only whole-program *analysis***, with these
qualifications:

- **Indexes.** The header index name resolution reads, the module list, and
  the set of clashing names are program-wide *data*, built from per-file
  declaration lists and updated when a file's list changes; no body is read.
- **Combining steps over per-declaration summaries** in 03-generate-03-link:
  the throws map, `ref` container types, and the actor message enums (a
  handler implements faces declared in other modules). Each reads summaries
  the declarations produced, so a change re-runs only the cheap combine.
  Deadlock has this shape too: per-handler gates and sends, then one graph
  pass.
- **One interface closure in 02**: `abi::platform_closure_all`, the types
  reachable from platform declarations, read by the checker to plan factories
  for type aliases. It walks signatures, not bodies; it can become
  per-declaration by planning the factories at each platform declaration for
  the types its own signature reaches.
- **Emission (04)** is whole-program by nature, outside the language server's
  path: which modules are reached [mod-used-only], which union wrapper types
  and runtime features (scheduler, codecs) to write, the ABI closure, and the
  lock file's protocol hashes. All read interfaces or IR summaries.

## 6. Caching (D4, decided R2: H)

**What both options need first** (the prerequisites are the same):

1. Stable ids: module path + name + overload ordinal (or signature), not item
   index or address.
2. Spans relative to their declaration (D1).
3. Owned data: no `'p` borrows of the `Program` in what is cached.
4. Per-declaration outputs instead of one shared `&mut Checked`; the checker's
   per-fn state already resets per fn, but writes into global tables.
5. Phases as functions of their inputs (§2), with no hidden program walks
   (`Symbols::ref_key` re-runs `resolve()` today).

**Option H: hand-rolled.** Two cache levels, matching the existing
`ParseCache`/`StdCache` style: per file (tokens, AST, declaration list, scope)
by content hash; per declaration (interface, body facts, IR) keyed by the
hash of its text plus the hashes of the interfaces it read. Early cutoff by
comparing interface hashes.

**Option S: salsa** ([salsa-rs/salsa](https://github.com/salsa-rs/salsa),
the incremental framework rust-analyzer is built on). Phases become tracked
queries over inputs; salsa records every dependency as a query runs, reuses
results whose inputs are unchanged, and cuts off early when a recomputed
result is equal to the old one. rust-analyzer moved to the rewritten salsa in
2025, citing future parallel evaluation and persistence
([changelog #277](https://rust-analyzer.github.io/thisweek/2025/03/17/changelog-277.html)).

| | H | S |
|---|---|---|
| dependency tracking | by hand: each step lists what it read | automatic, exact, per query |
| early cutoff | interface hashes we compare | built in (output equality) |
| granularity | the two levels we choose | whatever the queries are |
| migration | per phase, behind today's API | the data model must be salsa's (interned ids, `'db` lifetimes, outputs `Eq + Hash`) before anything is incremental |
| extra | nothing | parallel queries and cancellation are available later |
| risk | a missed dependency is a stale result | pre-1.0 API, proc macros through a 33k-line checker, build time |

**Decided (R2): H first, written so S stays possible.** The
prerequisites above are most of the work either way, and salsa cannot be
adopted until they are done; two levels are enough for the language server's
need (one declaration edited at a time); and H keeps the change incremental
behind today's API. Switch to S if hand-kept dependency lists grow past two
or three levels or produce stale-result bugs.

## 7. The plan

In order; each step lands as one piece of work (one commit) with its spec,
docs and records, and leaves the suite green. Steps 1–2 need no open decision.

1. **Measure and keep.** Per-phase timings for `salvo analyze` (expand,
   resolve, each check round, IR); the language server keeps its last
   analysis per project until the overlay changes.
2. **The rounds' no-language-change facts** (4.4, 4.5): delete claims;
   move-mode candidates as a per-body second walk; parking as a syntactic
   handler fact; mutations published from the final check.
3. **Lends and mutated fields** (4.2, 4.3, decided): the written-with-default
   lends (`kept_params` skipping Copy scalars, the desugaring writing `holds`
   for `iter fn` minters) and the end of field-path inference.
4. **The iterator and state redesign** (4.1, 4.6, R3–R5): `iter T` removed; named `iter fn` state structs and
   handler-named state; `Iter<C, It, T>` with obligations and spreads writing
   `It`; the bare-name rule; ROADMAP §2d items 9c and 9d. With 2–4 the program
   rounds and the iter-return pre-pass go, and every body is checked once.
5. **Identity and storage**: stable ids (module path, name, overload
   ordinal), spans relative to their declaration, the checker's facts stored
   per declaration, and the pre-IR dumper (D1); module-qualified type keys;
   scoped lookups for the plain-name sites (§5).
6. **The sweeps, in pipeline order** (§5, D3): the struct-fn hoist and tests
   into parse; effect aliases and prerequisites resolved in scope; the `iter
   fn` expansion per file; comptime after interfaces, with `by` declaring its
   member signatures; route stubs derived per effect in the IR.
7. **The phase split**: 02-resolve-03-interfaces as its own pass; bodies
   reading interfaces only; ownership out of the body walk into
   03-generate-02-flow, after the audit (§3 "Open"). ~~**Do not forget**: the
   deadlock graph reads the AST, so the IR-derived `__Route_X` (step 6,
   piece 5) has no router node~~ — **done** (R8 piece 6, §8): the graph,
   still on the checker's tables, builds every routed protocol's router
   servant node from `route_uses`, with its selector dependency's edges,
   and `route_deadlock_tests` has a cycle through a route. When deadlock
   moves to 03-generate-03-link over the IR (step 8), the route is an
   ordinary `face_any` impl there and the synthesis goes.
8. **Caching (H)**: per file, then per declaration with early cutoff; the
   language server recomputes the edited declaration and what read its
   interface.

The general `state` effect (4.1) is a separate feature (ROADMAP §2d); the
plan does not wait on it.

## 8. Work log

Resumable state for the build of §7 (the user's instruction, 2026-10-20:
proceed through the steps without stopping, commit per AGENTS.md, keep this
log current so the work can resume after a context reset). Newest last.

- **Step 1 (done, committed "Time the pipeline's phases, and reuse an
  analysis between edits").** `SALVO_TIMINGS=1` [phase-timings]
  (`salvo_core::timing::phase`, wired in `analysis.rs::analyze_project` and
  `check.rs` rounds/`check_once`); the server's analysis cache
  [lsp-analysis-reuse] (`lsp.rs` `analysis_with`, cleared in
  `handle_notification`). Numbers (debug, `demo/`): parse 22ms, expand 10ms,
  resolve 17ms, check 500ms = **three** rounds of ~160ms (body walk ~145ms
  each) + deduce ~5ms each. A third round runs on ordinary programs: suspect
  claims (step 2 removes them).
- **Steps 2 and 3 (done, one commit "Check with signature contracts, and read
  no interface off a body").**
  `deduce::contracts` before the first walk; `check_program_rounds` loops
  while candidates or `mut_fields` change (no claims, no mutations in the
  test); `syntactic_parking_handlers` seeds parking. Three rounds → two on
  every program measured; the second is `mut_fields` (step 3). **Deferred to
  step 5:** the per-body second walk for move-mode candidates (needs facts
  stored per declaration to replace one body's rows); until then a new
  candidate re-walks the program.
  Step 3: `lends::lends_of` is declared-or-default (kept non-Copy params;
  `kept_params` skips Copy scalars), `infer_from_body` only validates written
  lists; field-path inference removed (`mut_fields` and `MutFieldFacts`
  gone). Still two walks on every program measured: std itself has move-mode
  candidates, so the second walk is for those (step 5's per-body re-walk).
- **Step 4 (done, committed "Name iterator state structs, and remove iter
  T").** `state Name { … }` required in an `iter fn` (`FnDecl.iter_state_name`,
  also on the generated minter and `next`, which the LSP keys on); every
  parameter held (`Hold::Owned`/`Whole`); `iter T`, `hoist_iter_types`, the
  iter-return pre-pass and `Checked.iter_returns` gone; `Iter<C, It, T>`
  through std, examples, tests and docs. Handlers: `HandlerDecl.state_name`,
  `check_handler_names` ([bare-names], [state-name]); 9d's initializer fix in
  `ir::build::decls::fields`; 9c in `learn_from_implicits_once` (a written
  implicit teaches through the fn written). Still two walks per program
  (move-mode candidates; step 5). Full nextest after the std change took
  8 minutes (every stamp missed).
- **Step 5, pieces 1–2 (done, committed "Re-walk only the files with new
  candidates, and look types up by key").** Per-file re-walk
  (`rewalk_files`, `Checked::drop_files`, `ErrorMarks`); keyed lookups in
  deduce, features, platform checks, IR actors, both emitters. **Decided
  while building:** per-*file* re-walk now, per-declaration storage and
  relative spans with step 8's caching (stamps and route stubs synthesize
  spans past the end of the file, so a declaration does not own its rows by
  containment; the writer will have to name it). Qualifier lookups stay
  program-wide by name (overloads chosen by subject).
- **Step 5, piece 3 (done, committed "Add salvo facts, the pre-IR dump").**
  `crates/salvo-core/src/facts.rs`, `salvo facts`, snapshot
  `golden_tests__facts_iteration`.
- **Step 5, piece 4: module-qualified keys tried, deferred to step 8.** The
  rule tried: a std declaration keeps its plain key (std is fixed text, so
  first-declarer among std is stable), every other declaration is
  `Name§module`. 124 tests failed; keying the linear discharge sets fixed
  66 (`check_fn_item`'s `own_discharges`, `member_discharges_in`: written
  names passed to the keyed `linear_capable`, a latent bug for a clashing
  linear struct; kept, committed "Key linear discharge sets by type key").
  The 58 left, by cause: route stubs looked up by plain effect name
  (`route.rs`, "no route stub for `Ping§main`"); the platform host-project
  check ("`HostGreet` implements `Greet§lib`, which is not a declared
  effect"); messages printing keys (`P§main`, `Counter§main`); the deadlock
  graph's handler and effect names (cycles no longer found); the throw
  tests' test-local `effect Throw` against `THROW_EFFECT`; the
  `a_next_without_a_yield…`/`driving_a_hand_written_pass_twice` pass lookups;
  a handler-dependency message; `protocol_hashes` read by plain name in a
  test. Each is a site assuming key == written name for an unclashing user
  type. Deferred because what the keys are for is step 8's cache: until
  then first-declarer keys are a program-wide index (which modules declare
  which type names), and the cache can treat a change to it as invalidating
  every declaration that names one of the affected types. **Stable ids**
  likewise move to step 8, where they are first read (cache keys, the std
  cache's transfer): `FnKey`/`DeclId` stay (file, item) until then.

- **Step 6, piece 1 (done, committed "Cache a file's own expansions with
  its parse").** `expand::expand_file` / `expand_program`; the language
  server's `ParseCache` keyed by (text, module, annex).
- **Step 6, piece 5a (defaults on handler constructor parameters): done**
  ("Let a handler's constructor parameters have defaults") — [fn-param-default]
  for `use`, `spawn` and `with` constructions, the prerequisite of R7's
  option A.
- **Step 6, piece 5 (route stubs derived per effect in the IR): done,
  committed "Spell routes as use any, and derive them in the IR".** As
  planned under R7: `Stmt::Use.any`, `check_use_any`, `route_uses` and
  `route_keys`, Phase 1b in `actors::generate`, `generated_impl` in both
  emitters, `RouteSelector.config()` with std's policies taking `config`;
  `route.rs` and `desugar::walk_module` deleted. One loss recorded in
  ROADMAP: the deadlock router node (the derived route is not an AST
  handler). **Step 6 is complete.**
- **Step 6, piece 4 (comptime lookups through scopes): done, committed
  "Look comptime names up through each file's scope".** Step A of the
  comptime row: `resolve_sources` over the unstamped program, `FileNames`
  per file, clause targets from the item in hand, groups and qualifiers
  scoped, a `type` outranking a struct. **Not done, and why:** step B (the
  `by` clause declaring member signatures at interfaces, bodies stamped
  after resolution) needs step 8's stable ids and relative spans (stamped
  fns are found by address and their spans sit past EOF), and a DECISION
  recorded in ROADMAP §2c (template's signature vs the group member's).
- **Step 6, piece 3 (mixed handlers with dependencies, R6): done,
  committed "Let a mixed handler declare dependencies".** The refusal was
  the only real gate: the spawn already fell through to the actor path's
  dep matching, the IR built `__depN`, both emitters routed them to the
  servant only, and façade members already had them hidden. Added: a named
  façade diagnostic, the servant node's dependency edges in the deadlock
  graph, the ambiguity arm over available effects; e2e `MIXED_DEPS` on both
  backends.
- **Step 6, piece 2 (effect aliases and prerequisites in scope): done,
  committed "Resolve effect aliases and prerequisites where effect lists are
  lowered".** Built as plan (b) below collapsed into one step: aliases are
  module items in `ModuleScope.effect_aliases`; `expand_effect_refs` (with a
  quiet variant for call sites) expands each list with origins, resolving
  every entry in its declaring file (`scope_of`); new tables `decl_effects`,
  `handler_deps`, `effect_prereqs`; IR/deadlock/reach/hover read them;
  `alias.rs` and `prereq.rs` deleted. The one gap found: an effect reached
  only through an alias had no callable members, since `effect_members` is
  the file's import scope — `effect_members_here` adds the members of every
  effect available in the body (and LSP completion offers every program
  member behind its availability gate). The prerequisite name clash became
  a non-error. The mixed-handler refusal still fires, on written plus
  implied deps; next piece lifts it (R6). Survey, as written before: both passes rewrite AST effect lists before resolution
  (zero-span implied entries, synthetic imports), and about 17 consumers
  read those lists off the AST: `check_effect_list`,
  `handler_dep_effects_here`, `lower_fn_effects`, the call-site wants
  (`check.rs` ~32151, which lowers the *callee's* list), task mints
  (`resolve_task_effects`), the one zero-span reader (`check_use`'s
  "implied by" message), the mixed-handler-with-deps refusal, the IR's
  `interface_decl`/`impl_decl`/`fn_decl` fallback, two IR body sites,
  `approx_keyed`, deadlock (×3), reach, and the hover signature. Staged plan:
  (1) one checker helper `expand_effect_list(list, scope)` returning each
  effect with its origin (written / alias / prerequisite-of), and tables
  `handler_deps` and a complete `fn_effects`, the AST rewrite kept; (2) every
  consumer onto them; (3) the rewrite deleted, aliases resolved in scope.
  **Two calls are the user's before (3)**: whether hover shows an effect
  list as written or expanded, and whether a mixed handler whose face has a
  prerequisite is still refused (`check.rs` ~15780 sees the implied
  dependency today, so the refusal fires; with the rewrite gone it would
  not). Comptime after interfaces and route stubs in the IR follow it.

- **Step 7, the plan (surveyed 2026-10-20, after step 6).** Today:
  `walk_signatures` is only `collect_implicit_signatures` (implicits and
  `+Q` established claims; diagnostics dropped); `check_fn`'s first ~450
  lines are signature work done *during* the body walk, in walk order —
  `check_effect_list` → `decl_effects`/`fn_effects`, `handler_deps`,
  `effect_prereqs`, `infer_lends` → `fn_lends`. **Order bug suspected**:
  `fn_effects` is read by other bodies (check.rs ~6250 building a fn key's
  `Ty::Fn`, ~7421 `fn_value_by_name`) with `unwrap_or_default`, so a fn
  value naming a later-declared effectful fn reads no effects. The re-walk
  (`check_program_rounds`, MAX_ROUNDS 4) exists only for move-mode
  candidates and re-derives whole files. `WalkCtx.inferred` is always
  `Some`, so the ~21 `inferred.is_none()` gates are dead. Ownership
  feedback into typing, by audit (reading): signature-level (a callee's
  contract dropping `NonEmpty`, `invalidate_place_narrows`,
  `strip_dependent_claims`) — stays in typing; body-flow (a consumed value
  reads `Ty::Unknown`, `narrowed = Never` doubling as "consumed", a partial
  move reading Unknown) — error paths only, so it changes which later
  diagnostics appear, not what an accepted program means. Pieces, each one
  commit:
  1. signature facts into the pre-pass (`fn_effects`, `decl_effects`,
     `handler_deps`, `effect_prereqs`, `fn_lends`), with a test of a fn
     value naming a later-declared effectful fn; dead `inferred` gates
     removed;
  2. the audit made evidence: an ownership-blind walk whose typing tables
     must equal the normal walk's on every clean corpus/example program;
  3. 02-resolve-03 as its own pass with its own diagnostics (the signature
     half of `check_fn`, handler/effect headers), `check_fn` keeping only
     the body;
  4. bodies read interface tables only;
  5. typing state split from ownership state (`fate_mutation_root` split;
     a consumed marker apart from `narrowed`);
  6. ownership as a per-body flow pass replaying recorded events, move-mode
     as a local fixpoint, the re-walk loop deleted — **user call needed**:
     keep the Unknown cascade suppression after an ownership error, or not;
  7. the flow pass onto the IR (03-generate-02-flow) — **user call
     needed**: ownership errors only on type-clean programs — plus the
     deadlock router node (§7 note).
- **Step 7, piece 2 (done, committed "Check that ownership never changes an
  accepted program's typing").** [ownership-audit]:
  `check_program_ownership_blind` (thread-local switch; `mark_consumed` at
  the 8 `narrowed = Never` sites; `check_moved_place` off) and
  `ownership_audit_tests.rs` over every example and std: no difference in
  `expr_ty`/`repr_ty`/`call_fn`/`coerce`/`is_tests` once dependent-qualifier
  root numbers are normalized (std `heap.sv` numbers variables 3 apart
  between the walks; unexplained, identity only). Next: piece 3.
- **Step 7, piece 4 surveyed (bodies read interfaces only): already nearly
  true.** The `self.out.*` reads in check.rs that another body could have
  written: `fn_effects` (fixed, piece 1), `implicit_params` and
  `established_quals` (pre-pass), `written_types` (an output keyed by AST
  node, read only to skip re-lowering the same node — harmless), and
  `carried_identities` (keyed by `(FnId, subject)`, first writer wins
  program-wide: two files resolving the same written identity name to
  different fns would share the first one's answer — latent, unreproduced;
  fix by keying with the file, or resolving at the read). Everything else
  read is the body's own rows.
- **Step 7, pieces 5–7: rethought, waiting on the user (R8 questions).**
  The audit (piece 2) shows typing does not need ownership's answers. Two
  routes to "ownership out of the body walk":
  (a) as planned — a checker-side event log replayed by a flow pass (piece
  6), then that pass moved onto the IR (piece 7): two rewrites of the same
  machinery (~thousands of lines: `fate_move`, `apply_call_contract`,
  `fate_mutation*`, `poison_derived`, `apply_binding_mode`, partial moves,
  lambda captures, linear obligations, state stores);
  (b) straight to the IR: typing runs ownership-blind (the cascade question
  answers itself — a consumed value keeps its declared type, the flow pass
  reports the use once, and typing reports only genuine type errors, so
  [type-unknown-lenient]'s one-mistake-one-diagnostic holds), and
  03-generate-02-flow on the IR owns fate, poison, partial moves, move-mode
  (a local fixpoint), lends validation and linearity; the re-walk loop goes.
  **The user's call for (b)**: ownership errors then exist only where the
  IR exists — per *declaration* that type-checks clean (the IR builder
  needs typing facts), so a body with a type error reports no ownership
  errors until the type error is fixed; today both appear together.
  Recommendation: (b), per declaration. Piece 5 (splitting `narrowed`'s
  consumed marker) is only needed by (a). Piece 3 (the interface pass with
  its own diagnostics) is independent and is what step 8's per-declaration
  cache needs; it can go before or after.
- **R8 (route b) chosen by the user.** Pieces, each one commit, each with a
  differential test against the checker before the checker's version is
  switched off:
  1. `ir/flow.rs`, the use-after-move pass, run by tests only: per path a
     set of wholly moved locals (a `Read { consume: true }` of a whole
     non-Copy local, a consumed lambda `Capture`); a later read of one is the
     error. Joins and loops mirror the checker's rules exactly (whatever they
     are — the differential decides), findings recorded. Differential test:
     a corpus of use-after-move programs; the checker's consumed-read error
     spans (normal walk) must equal the flow pass's spans over the IR built
     from the ownership-blind walk's tables.
  2. Gates: "clean" for the IR becomes "no typing errors" — typing runs
     ownership-blind in the drivers (`analysis.rs`, `driver.rs`,
     `ir::dump_program`), the flow pass's diagnostics join the IR's, and the
     checker's consumed-read diagnostic is switched off. Per declaration if
     the builder allows (a type error in one fn must not hide another fn's
     ownership errors), else per program at first.
  3. The same-call left-to-right rule; partial moves (`moved_places`,
     projection consumes); poison/BoundAway (fate links); lambda capture
     moves; state stores; lends validation; linearity's checker half; then
     move-mode and the re-walk loop's deletion — each its own piece.
  4. Deadlock onto the IR (link stage), with the route's router node (§7).
- **R8 piece 1: done** (committed "Add the IR use-after-move pass, checked
  against the checker"). `ir/flow.rs` (`check`, `check_one`)
  [flow-use-after-move], run by tests only. `tests/flow_tests.rs`: 36
  programs; the checker's "it was consumed (moved) by …" spans (normal walk,
  the same-call variant included: on a bare later argument it is the read
  the pass reports) equal the pass's spans over the IR built from the
  ownership-blind tables, compared by start offset (the checker marks `b` in
  `b.s`, the IR the whole read), except for a documented `KNOWN` list. A
  second test pins every case's reads; a third finds nothing on any example
  or on std.
  - **What differs (piece 2 must carry these moves):** a literal store, a
    `...` spread, a `use` constructor argument and a `break` value are moves
    to the checker (`fate_move`) but plain reads in the IR, so the pass does
    not see them. The builder was not changed: `consume: true` is the Rust
    emitter's move-versus-`.clone()` switch (`ir_emit/body.rs`, the `Read`
    arm and `f.consumed`), so marking them changes the output. Options: a
    second flag on `Read` (moved, as opposed to consumed-for-emission), or
    accept the clone→move change in Rust and review the goldens. Linear
    values already come through, via `linear_moves`.
  - **The checker's rules, mirrored as they are.** Joins are union (moved
    on any fall-through path is moved; all arms leaving keeps the pre-arm
    set). After the join, `reset_assigned` revives every variable assigned
    in a fall-through `if` arm, or in *any* `when` arm. A loop body is
    walked again from its end state when that changed; the end state is the
    sequential state even when the body always leaves. The set after the
    loop is that end state joined with every `break`, then the body's
    assignments are revived. `continue` states are dropped. A narrowing
    binding of a whole local is the local; one re-made at a test *of that
    local* starts alive, any other inherits (a `Cond` justification is
    reused for persisting narrows, so the pass reads the condition's `Test`
    nodes).
  - **Findings.** Unsound, each failing rustc with E0382 (ROADMAP 0o, with
    repros): an `if` arm that assigns then moves; a `when` whose leaving
    arm assigns; a move before `continue` read in the next iteration.
    Over-strict: a body that always `break`s is still re-walked
    (`while c { eat(s); break }` reports `eat(s)`, and a top-level
    `break s` is refused), and a narrowed Copy scalar is consumed because
    the checker judges by the declared type. A surprise: the ownership-blind
    walk reports ~20 false linear errors in std, because its linear join
    reads `narrowed == Never` as settled. The audit compared tables only.
    Piece 2 must not gate on the blind walk's linear errors; the IR's
    linear check (`ir::check`) already owns that question.
  - **Piece 2 must:** make the IR carry the four missing moves; run typing
    ownership-blind in the drivers and drop or move the linear half of the
    checker's join that depends on `Never`; join the flow diagnostics to the
    IR's in `ir::analyze` with a renderer that names the move
    (`FlowDiagnostic::moved_at`); then switch the checker's consumed-read
    off. Fix the unsound rules (ROADMAP 0o) in the pass once the checker's
    version is off, with `flow_tests`' pinned expectations changing in the
    same commit.
- **R8 piece 2: done** (committed "Report use-after-move from the IR").
  Design as adjusted from piece 1's note: the drivers do **not** run typing
  ownership-blind (its linear join reads `Never` as settled — ~20 false
  errors in std). The checker keeps its internal `narrowed = Never` marking,
  but a consumed read is no longer its diagnostic: it answers the type the
  value had before it was consumed (`LocalVar::consumed_ty`, set by
  `mark_consumed(var)` and kept up to date through `install_narrows`,
  `with_narrows`' restore, snapshots and `merge_fallthrough`'s join, so it
  types like the blind walk), so typing is unaffected and the IR gets built.
  `ir::analyze` runs `ir::flow::check_one` per decl next to `ir::check`
  (test files included now, for the flow pass only) and renders "`s` cannot
  be used here: it was consumed (moved) at line N; reassign it before use, or
  `copy` it where it is moved" (`FlowDiagnostic::render`); `ir::dump_program`
  puts the same inline. The four moves the IR missed come through
  `Checked::moved_reads` (every bare identifier `fate_move` consumes wholly),
  read `consume: true`: the Rust output moves where it cloned (std's
  `partition`/`both`/`split_once`, `args.sv`'s `scan`, `replyto` captures, …;
  five Rust and eight IR goldens, 79 example files and std's and aws's
  host projects regenerated; rustc and kotlinc green). The same-call rule's bare-name half is off in both copies.
  - **Found building it:** the IR evaluated a literal's components in
    order, but the checker moves what a literal stores *after* reading every
    component — `(w, size(w))` (examples/collections) and std's
    `Spec { ...base, name: "${base.name}" }` were E0382s waiting for the
    move. The builder now hoists the reading components into `__part`
    temporaries ahead of the moves when they conflict
    (`hoist_past_stores`, tuples and struct literals). And the flow pass
    took a written `if true { break s }` for the builder's block expression;
    a block expression's condition spans the whole branch now.
  - **Left in the checker (piece 3 takes them):** a read after a `once` call
    (the IR has no read at a call of a local), a root a projection move took
    wholly (the IR marks the projection, not the root), and **every fn
    value** (the IR lends fn-typed parameters whatever their contract, so
    `take(g)` of a `=> !g` parameter is a plain read; the pass skips
    fn-typed locals so nothing reports twice), plus poison/BoundAway and
    partial moves as planned. The same-call scan keeps its projection-root
    half for the same reason.
  - **The differential, reworked:** `flow_tests` compares the pass with the
    checker run through `check_program_reporting_consumed_reads` (a
    thread-local switch putting the old diagnostic back, test-only), builds
    the IR from the normal walk (which must report nothing on the corpus),
    and checks the gate (`ir::analyze`) reports what the pass finds; `KNOWN`
    is empty. 39 cases (three new: `tuple_reads_before_store`,
    `spread_reads_base`, `break_in_always_arm`).
  - **Gating is per program:** a checker error anywhere hides every flow
    diagnostic (analyze tests that mixed both were split in two). Per
    declaration would need the builder to run over a program with errors.
  - **Piece 3 next**, in order: the remaining checker-reported moves above
    (a second flag on `Read` — moved, as opposed to consumed-for-emission —
    is the likely shape for fn values and projection roots; a `once` call
    needs the call of a local to read it); partial moves (`moved_places`,
    `check_moved_place`); poison/BoundAway (fate links); lambda capture
    moves; state stores; lends validation; the 0o unsound join/loop rules
    (in the pass, with a `KNOWN` entry per fixed rule); then move-mode and
    the re-walk loop's deletion.
- **R8 piece 3a: done** (committed "Carry every move the checker made into
  the IR"). The IR's `Read` and `Call` carry a second, flow-only flag,
  `moved` (false at every builder and emitter construction site; no emitter
  reads it, the dump does not show it), set from a new
  `Checked::flow_moves`: an identifier a call's contract consumed (both
  contract copies: `apply_call_contract` and the named call — a fn value,
  which `param_mode` lends whatever the contract, or a `once` value passed
  on), a projection whose move took its root wholly (`projection_move`,
  keyed by the projection; the builder's `mark_root_move` marks the first
  read of the root inside the lowered expression, since an unwrap lowers
  to a block, not a place read), and a `once` local's call (keyed by the
  call, on the IR's `Call`). The flow pass treats `consume || moved` as the
  move — `moved` whatever the type, of the place's root through
  narrowings and projection bindings — makes a call of a local (and a fn
  value of one) a use, and no longer skips fn-typed locals or captures.
  Then the checker's consumed-read diagnostic went entirely: the read
  path's no-poison variant, the callable path's, the `consumed_by` field
  (and `fate_move`'s `moved_by` argument), `CONSUMED_BY_ONCE_CALL` /
  `CONSUMED_BY_PROJECTION`, both copies of the same-call scan
  (`consumed_here`; `projection_move` returns nothing now), and the
  test-only switch `check_program_reporting_consumed_reads`. Poison/BoundAway
  and `check_moved_place` stay.
  - **The differential, retired:** 23 new cases (fn values moved then
    called, passed, lent, stored, in a loop, in one call twice; `once` called
    twice, in a loop, after being passed on and before; roots an unwrap
    took, in an arm and in one call; near misses) agreed with the
    switched-back checker before it was removed; the pinned expectations
    are those reads, and `the_gate_reports_what_the_flow_pass_finds` keeps
    "the checker reports nothing, the gate reports the pass". 62 cases.
    Emitted code did not change.
  - **Found:** an index projection (`xs[0]`) is a *partial* move (path
    `[?]`), not a root taken wholly; only an unwrap (`o ?: unreachable!()`)
    or a derivation through one reaches `CONSUMED_BY_PROJECTION`'s case. The
    `l7b_once_fns` analyze test mixed a checker error with the `once`
    errors and was split in two (gating is per program).
  - **Piece 3 next**, in order: partial moves (`moved_places`,
    `check_moved_place`); poison/BoundAway (fate links); lambda capture
    moves; state stores; lends validation; the 0o unsound join/loop rules
    (in the pass, each fixed rule changing its case's pinned expectation);
    then move-mode and the re-walk loop's deletion.
- **R8 piece 3b: done** (committed "Report partial moves from the IR").
  `ir::flow` keeps, per path beside the wholly moved locals, the places moved
  out of each local (by storage, in `crate::place::Step` terms, composed
  through narrowings of projections and, for the move, through any binding
  of one); a move is a consuming projection read (`consume`, not `moved` —
  `moved_projections` and move-mode bindings already made them, so the
  builder and emitted code did not change). The checker's rules, mirrored: a
  whole read is a use of every moved place, a projection read a use when it
  or a shorter projection of it overlaps one; assigning a projection puts
  back what it covers before its value is read, the whole local everything;
  joins union, `reset_assigned` leaves the places alone, a changed set
  re-walks a loop. Messages: the checker's two shapes with the move's line
  (`FlowDiagnostic::part`, `PartUse`). Then `check_moved_place` went with
  `assign_target`, `projection_base`, `MovedPlace::{blocks, describe,
  span}`; `moved_places` stays (now a path only) for the linear
  decomposition rule and `check_state_whole`.
  - **The differential:** 33 `part_…` cases; the checker's messages, dumped
    before removal, equal the pass's read for read and word for word but the
    line, except `part_same_call` (`eatl2(p.tags, p.tags)`): the checker
    moved a call's projection arguments after reading them all and accepted
    it, an E0382 in Rust; the pass reports it [deduce-same-call], a fix. 95
    cases.
  - **Found (ROADMAP 0o):** unsound — `p.tags = relist(p.tags)` after
    moving `p.tags` passes the read (the target is revived first) and leaves
    the field moved; over-strict — `n.a.name` after moving `n.a.tags` is
    refused (the base `n.a` overlaps). Reassigning the root in one arm does
    not put parts back after the join (correct: the other path has them
    moved). `s2_move_mode_ancestors_are_consumed` (analyze_tests) was split
    (gating is per program); deduce_tests' partial-move tests go through
    `ir::analyze`.
  - **Piece 3 next**, in order: poison/BoundAway (fate links); lambda
    capture moves; state stores; lends validation; the 0o unsound rules (in
    the pass, each fixed rule changing its case's pinned expectation); then
    move-mode and the re-walk loop's deletion.
- **R8 piece 3c: done** (committed "Report poisoned and bound-away reads
  from the IR"). The checker records what the IR cannot show:
  `Checked::fate_binds` (each borrow-mode binding's flattened fate links —
  root, projection path, `crosses` — by the bound name or an assignment's
  target), `Checked::fate_bound` (the links a move-mode binding took
  ownership through) and `Checked::fate_mutations` (every mutation event
  `poison_derived_full` saw, by its span, with the acting handle, the
  contents-mutation spare and the `NotSame`-proven handles it spares). The
  builder carries them on a flow-only `FnDecl::fate` (`FateFacts`, by the
  binding's `NodeId`, an assignment's value span, or the mutated-at
  expression's span); emitted code did not change. `ir::flow` keeps the links
  per path (joined by union, re-walking a loop when they change, dropped by
  a fresh binding of the root's name — the checker's links are by id) and
  poisons at every event on a root: the moves it already sees (whole at `[]`;
  a projection at its path, on every root along the way), a whole
  reassignment (after the value), a mutation (after its call; before an
  assignment's value), and a move-mode binding (which also marks a root
  taken whole as bound away, named by the binding). A poison replaces a move
  as the reason; a join keeps the first path's poison. Messages: the
  checker's two shapes with the event's line (`PoisonUse`). Then the
  checker's two messages went, with all its `poison` state (`LocalVar::poison`,
  `VarState::poison`, `Poison`, `FateEvent::BoundAway`/`describe`): nothing
  else read it. The poisoning itself stays (it marks the values consumed,
  which the checker's joins and linear rules read), and so do the links
  (move-mode inference, [fate-derived-readonly]).
  - **The differential:** 42 `poison_…`/`bound_away…` cases; the checker's
    messages, dumped by a temporary test before removal, equal the pass's
    read for read and word for word but the line, except
    `poison_same_call` (`eatlook(s, t)` with `let t = s`): the checker applied
    a call's moves after reading every argument and accepted it, an E0505 in
    Rust; the pass reports it [deduce-same-call], a fix. `bound_away_unwrap`
    (a move-mode `let m = o ?: unreachable!()`, which the IR does not read as
    a move) came from reading `apply_binding_mode` afterwards and is what
    `fate_bound` is for. 138 cases; the test count is unchanged.
  - **Found:** the `if` revival (0o's `reset_assigned` rule) revives a
    *poisoned* local too — `t = xs; add(xs, "a")` in an arm, then
    `size(t)`, prints 0 on Rust and 1 on Kotlin (ROADMAP 0o, mirrored); a
    statically true `is` test with a binding loses the binding in the IR
    (`UNSUPPORTED(ident h)`, ROADMAP 0p), so no read is left to report a
    poison at; the checker's `reset_assigned` left a stale poison that would
    have misnamed a later plain move (gone with the field). Nine test
    programs mixed a checker error with a poison and were split or made
    clean (a checker error hides the flow pass).
  - **Piece 3 next**, in order: lambda capture moves; state stores; lends
    validation; the 0o unsound rules (in the pass, each fixed rule changing
    its case's pinned expectation — the poison revival with the arm rule);
    then move-mode and the re-walk loop's deletion. The links are still the
    checker's (`fate_binds`): computing them in the pass is part of
    move-mode's piece, which needs them first.
- **R8 piece 4 (move-mode and the re-walk): done** (committed "Decide
  move-mode at the first moving use, and check every body once"). Taken
  before the remaining static move-site rules (lambda capture moves,
  handler-storage moves, lends validation), which need no flow and can stay
  in the checker for now: move-mode is what the re-walk existed for.
  - **First attempt interrupted** (no commit; its experiment hook in
    check.rs reverted). What it measured, `tmp/xp_rewalk.txt`: with the
    re-walk forced off (one walk), an example's program (std included) has
    1 move-mode binding (`binding_modes`) where the normal two walks have
    ~104, and ~120 S1 errors per example (1445 in all, mostly in std:
    "cannot store/return/move `x`: it was bound from `y` and shares its
    fate, so it can only be read") — exactly the derived bindings the
    second walk turns move-mode. So the single walk's S1 errors *are* the
    candidate set.
  - **What the second walk changes, measured** (every `Checked` table, one
    walk against two, on every example): `binding_modes`, `fate_bound` /
    `fate_binds`, `fate_reads`, and at the derived variable's later move
    sites `moved_reads`, `flow_moves`, `linear_moves` and `fate_mutations`
    (the move is made instead of refused) — and, besides the S1 errors,
    **nine checker linear diagnostics in std** ("`e` owns a linear value
    that is consumed on some paths but not others", "assigning to `r` drops
    the linear value it still owns"): a move-mode binding consumes its
    ancestors, and the checker's linear joins read that consumption.
  - **Design chosen: (C), the decision at the first moving use, in the one
    walk.** (A), the IR deciding, is blocked by the checker's linear half:
    whatever decides move-mode after the walk, the walk's own linear joins
    have already run on borrow-mode state and reported those nine errors;
    the decision can move to the IR together with the linear joins, not
    before. (B), a per-body re-walk, still walks those bodies twice and
    needs rows dropped per declaration (step 5's deferral). (C) needs
    neither: a derived binding's mode is known exactly when its first move
    or mutation is reached — it may take ownership (the `apply_binding_mode`
    conditions, judged at the binding and logged: `may_take_ownership`,
    `bind_log`) and it is now a candidate — so `error_derived` takes
    ownership there instead of refusing (`take_ownership_at_use`): every
    binding of the chain behind the use (`record_move_candidates`' set), in
    binding order, gets what the second walk did at its binding —
    `binding_modes` (and a `for` origin's), the ancestors consumed or partly
    moved and their other derivatives poisoned on the current path, the
    bound variable's links dropped and its descendants' copies of them — and
    the use then proceeds as the move of an owned local, recording what the
    second walk recorded there. A candidate that may not take ownership
    reports the S1 error at the use, as before. What the bindings wrote
    before the decision is rewritten when the file's walk ends
    (`finish_fate`): `fate_binds` / `fate_bound` from a log of every
    binding's links by stamp (no link stamped by a move-mode binding
    survives), and `fate_reads` likewise. A binding met again after the
    decision (a loop's second pass) takes move-mode at the binding, as the
    second walk did.
  - **Found by the differential, each fixed**: a destructure binds several
    variables at one span, each linked to the whole root, so the first one's
    poison of the root killed the next (`bind_log` holds one record per
    variable; `poison_floor` spares every variable made after the binding);
    a root assigned between the binding and the use keeps its new value and
    the poison that reached the bound variable is undone
    (`LocalVar::assigned`, flow state unioned at joins but outside
    `VarState`'s equality; path-insensitive first, which broke std's
    `send_or_back`: an arm that assigns and `continue`s); the paths that went
    through the binding but not the use (std's `let {…, body} = e` then `if
    body is Body b { put_task(…b…) }`) get it at the join
    (`take_ownership_on_paths` in `merge_fallthrough`); a linear "assigning
    drops" error raised while the binding was undecided is withdrawn when it
    is decided (`pending_drops`); and a `for` binding is stamped with its
    iterable's span, which `binding_modes` also holds for a by-value loop,
    so the decided set is its own (`decided`).
  - **The differential, retired**: `check_program` behind an environment
    switch ran both checkers and wrote every difference (each table line by
    line, the rendered diagnostics) to `tmp/xp_diff/`. Over every program
    the whole suite checks, e2e included, nothing differed but the
    ownership-blind audit walk, which reports three fewer of its known false
    linear errors in std. Then the loop, `rewalk_files`, `MAX_ROUNDS`,
    `ErrorMarks`, `Checked::drop_files` (no LSP or std-cache user), the
    shared `move_candidates` plumbing (a per-file `HashSet<Span>` on the
    `Checker` now) and the per-file split of body diagnostics went.
    Emitted code did not change. Tests: four `mode_…` cases in
    `flow_tests` (142), `a_linear_binding_takes_ownership_where_it_is_made`
    in `ir_linear_tests` (2039 in all).
  - **Timings** (`SALVO_TIMINGS=1 target/debug/salvo analyze --src demo`,
    debug, three runs each; demo/ analyzes clean with the user's edits):
    check ~290ms before (walk ~177ms, re-walk of 10 files ~99ms, two
    deduction passes ~5.5ms each), ~186ms after (walk ~175ms, one deduction
    pass ~5.4ms). The walk itself costs what it did.
  - **What is left of R8**: lambda capture moves, handler-storage moves and
    lends validation (static move-site rules, still the checker's); the
    ROADMAP 0o unsound join/loop rules (in the flow pass, each fixed rule
    changing its case's pinned expectation, the poison revival with the arm
    rule); linearity's checker half (its joins, the assigning-drops rule,
    decomposition), after which move-mode itself can move to the IR flow
    pass with the links (`fate_binds` computed there rather than by the
    checker); and deadlock onto the IR (the link stage), with the derived
    route's router node per §7 and a test of a cycle through a route.
- **R8 piece 5 (ROADMAP 0o in the flow pass): done** (committed "Track
  moves path by path in the IR flow pass"). The pass's joins and loops are
  dataflow over paths, its own rules now: an assignment revives on its own
  path (the `reset_assigned` mirror went, and with it the poison revival:
  `t = xs; add(xs, …)` in an arm leaves `t` poisoned at the join); a loop's
  top state is the entry joined with the body's end (when it falls through)
  and every `continue`'s, walked to a fixpoint (cap 32); after it, the
  `break`s (and a `for`'s top state); a `Loop` with no `break` never falls
  through; a projection is put back after its value is read. Over-strict
  rules dropped, each checked with `salvo run --backend '*'`: a body that
  always leaves has no back edge (`while c { eat(s); break }`, a top-level
  `break s` — the pass's refusal, not the checker's typing); a whole read
  whose own type is a Copy scalar is a copy whatever the `moved` flag says
  (the checker sets it by the declared union); a projection read is checked
  at its own place only (`n.a.name` after `n.a.tags`). The checker's joins
  were not touched (its linear rules and move-mode read them). Tests: 148
  `flow_tests` cases (eight expectations changed, six new), a gate test in
  salvo-cli's analyze_tests for the unsound repros, a KotlinCase and a
  `run_rust_files` test for the over-strict ones; std and every example
  stay clean. Found: a move out of an indexed element fails rustc (E0507),
  older than this, ROADMAP 0q.
  - **What is left of R8**: lambda capture moves, handler-storage moves and
    lends validation (static move-site rules, still the checker's);
    linearity's checker half (its joins, the assigning-drops rule,
    decomposition), after which move-mode can move to the flow pass with
    the links (`fate_binds` computed there), and the checker's own
    `reset_assigned`/`check_loop_body` joins can go with it; and deadlock
    onto the IR (the link stage), with the derived route's router node per
    §7 and a test of a cycle through a route.
- **R8 piece 6 (the derived route's deadlock node): done** (committed "Give
  a derived route its node in the deadlock graph"). **Choice: (a), the
  graph stays on the checker's tables and synthesizes the router node from
  `Checked::route_uses`**, not (b), the graph reading the IR. Why:
  - (b) changes when deadlock reports: the IR exists only for a program
    that type-checks (gating is per program), so a cycle would vanish
    whenever an unrelated error is present, and the graph's AST inputs
    (`handler_is_stateful`, the mixed and monitor classification,
    `parking_handlers`, task mints) have no IR summary yet. Every existing
    result would have to be re-proved for a node that, as measured below,
    reports nothing.
  - (a) is one more servant node built by the code an `of any X` handler
    already goes through: from `X`'s first `use any` site, a forward to `X`
    (back-pressure: the route neither gates nor waits) and its
    `RouteSelector<X>` dependency's occupancy edges (to the mixed servants
    and priced shareable handlers of `RouteSelector`); its own face skipped.
  - **What the old node did, measured** (the compiler at `1130e3cd^` built
    in `tmp/oldwt`, its graph dumped): `__Route_X's servant` had exactly
    those two kinds of out-edge and **no in-edge** — callers' sends land on
    `X`, and nothing else targets a servant of an actor face — so it never
    sat on a cycle. Its job was the near miss: keeping the forward off `X`,
    which would otherwise read as `X → X`. Every route program tried
    reports the same before and after the move to the IR. Deadlock onto
    the IR stays the 03-generate-03-link plan, for when the graph's inputs
    are per-declaration summaries.
  - **Built:** the three servant kinds (mixed, `of any X`, route) share
    `deadlock::Servant`; the route's node is a back-pressure forward to `X`
    plus its `RouteSelector<X>` dependency (the `use_deps` type at `X`'s
    first `use any`) as occupancy edges; `deadlock::edges` lists the graph
    for tests. `route_deadlock_tests` (3, real std): a gate cycle through a
    routed protocol, the near miss (no `Ping → Ping`), and a program's own
    stateful selector with an actor dependency — each diagnostic and each
    route edge equal to the old compiler's. The old compiler's extra stubs
    (every protocol a routing module spelled in `ActorGroup<…>`, e.g.
    cluster's `Lookup` and `Search`) had nodes too; the IR derives only
    routed ones, which reports nothing different since such nodes have no
    in-edge.
  - **Found:** a cycle closing *through* a router is never seen
    (`Pong → route → Watching's lock → Pong` is silent), as old as `of any
    E`; ROADMAP "A router's node has no in-edges", with the repro.
  - **What is left of R8**: lambda capture moves, handler-storage moves and
    lends validation (static move-site rules, still the checker's);
    linearity's checker half (its joins, the assigning-drops rule,
    decomposition); then move-mode into the IR flow pass with the links
    (`fate_binds` computed there), after which the checker's own
    `reset_assigned`/`check_loop_body` joins can go. Deadlock onto the IR is
    step 8's link stage now, not an R8 piece.
- **R8 piece 7 (linearity's checker half onto the IR): done** (committed
  "Report linear flow errors from the IR"). The checker reports no linear
  rule that depends on paths or moves; the IR does.
  - **The classification** (every linear diagnostic in check.rs):

    | rule | kind | owner now |
    |---|---|---|
    | consumed on some paths but not others (`merge_fallthrough`'s linear join, `linear_settled`, the `if` skip-path and `when` all-arms-left settling) | flow | `ir::check`, at each join (`join_paths`) |
    | assigning drops a value that still owes (`owes_linear`, `pending_drops`) | flow | `ir::check` (a state field: `ir::flow`) |
    | decomposition settles a container (`owes_linear`'s `moved_places` test) | flow | `ir::check` (it had it: `pending_fields`) |
    | a member leaves a state field with a hole (`check_state_whole`) | flow | `ir::flow` |
    | path-exit leaks (scope exit, `return`, `break`/`continue`, throw) | flow | `ir::check` (since IR_CHECKER) |
    | a linear value in statement position | typing: the expression's type alone | checker |
    | `check_linear_field`, `check_linear_opaque`, `check_linear_components`, no discharger, instantiation ban, variadic positions, `copy`, `discard` outside a discharger, store refusals, `for` over a linear iterator, a lambda consuming or mutating a linear capture, a ctor parameter moved out of state | declaration shape / typing / static move site | checker |

  - **Built:** `Stmt::Return` carries its span; `FnDecl::fate` carries the
    written assignments' statement spans, the written loops' spans, the
    name span and a member's obligation-holding state fields. The ledger
    joins a loop's exits (a `for`'s top, every `break`), counts a narrowing
    of a value as owing it at a join, discharges a failed positive arm test
    on later arms and the `else`, gains a written assignment of an owed
    value (the local's declared type), and gains nothing for a borrow-mode
    binding with fate links. A test file gets the flow rules, not the
    leaks. Gone from the checker: `owes_linear`, `check_state_whole`,
    `linear_settled`, `pending_drops`, `undecided_binds_of`,
    `linear_fields_of`, and `moved_places` (`MovedPlace`,
    `revive_moved_places`), which nothing read any more.
  - **The differential, retired:** a temporary hook in `check_program`
    compared the checker's three diagnostics with the IR's on every
    program the suite checks (2,465, nothing differed; only 10 had any,
    and 2 more mixed them with other errors) and on a 71-case corpus.
    Equal but for 8 cases where the IR is right: 3 checker false errors
    (a consumption before a `while true`'s only `break`; decomposed on one
    path and consumed on the other; `linear_settled` surviving a
    reassignment), 4 checker misses the IR's leak already caught (the
    checker's join ignored decomposition, and after it took a struct whose
    two halves left on different paths for settled; a move-mode alias
    consumed in one arm), and an assignment over a full state queue (the
    checker declares a state field as a parameter, which never owes). The
    corpus is `linear_flow_tests`, each difference noted on its case.
  - **Found, older than this piece, fixed:** the ledger gained nothing at
    an assignment (a reassigned linear value could leak unreported), and a
    borrow-mode alias owed an obligation of its own (a false leak a checker
    error had hidden).
  - **The ownership-blind walk's false linear errors in std are gone**: it
    reports no errors at all now (they came from the linear joins reading
    `narrowed = Never` as settled).
  - **Move-mode is not unblocked yet.** The checker's consumption marking
    (`narrowed = Never`, `consumed_ty`, `poison_derived`'s marking) has
    three readers besides move-mode (`take_ownership_at_use` undoing a
    poison that came through links the binding drops):
    1. `fate_mutation_kind`: a write through a `Mut` handle already
       poisoned returns early, so the mutation is not recorded on the
       handle's roots (`record_param_mutation`, `fate_mutations`) — a
       contract input;
    2. `fate_mutation_fields_preserving`: a consumed derived variable is
       left out of `virtual_place_binds` (the Rust virtual-place rendering)
       — an emission input;
    3. `check_loop_body`: a loop is re-walked when the state changed, which
       consumption and poisoning change (move-mode needs the re-walk too).
    The read and call paths' `consumed_ty` typing and the join's dropping
    of a consumed variable's place narrowings are neutral by the audit
    (nothing reads a consumed value in an accepted program). So the
    next piece first gives 1 and 2 a flow-pass answer (or proves them
    unreachable on an accepted program), then moves move-mode into the
    flow pass with the links, and deletes the marking with
    `reset_assigned`/`check_loop_body`'s ownership use.
  - **What is left of R8**: those readers, then move-mode into the IR flow
    pass with the links (`fate_binds` computed there); lambda capture
    moves, handler-storage moves and lends validation (static move-site
    rules, still the checker's).
- **R8 piece 8 (move-mode into the flow pass): done** (committed "Decide
  move-mode in the IR flow pass"). The IR flow pass decides which
  borrow-mode bindings take ownership and reports the S1 error
  [fate-derived-readonly] for the rest; the checker's walk keeps every
  binding borrow-mode, marks nothing consumed and poisons nothing.
  - **The three readers of the marking.** (1) `fate_mutation_kind`'s early
    return for a write through a poisoned `Mut` handle was *reachable* on
    an accepted program: `a.hp = 1; d.hp = 5` (two `at` handles, no
    `NotSame`) passed, and so did `eat(p); p.hp = 5`, which fails rustc
    (E0382). The IR's `Assign` holds a place, not a read of its root, so
    the flow pass never saw the target; the checker had seen it through
    its read of the target's base until piece 2. Fixed in the pass (an
    assignment through a projection is a use of its root, and of a moved
    place strictly containing the target; `FateFacts::assign_targets`
    gives the span), which makes the early return unreachable on an
    accepted program, and it went. (2) The virtual places'
    consumed-exclusion is the IR's (below). (3) `check_loop_body` re-walks
    on a narrowing or a fate-link change only: the pass has its own loop
    fixpoint, and the links still need the second walk.
  - **Design: decide over a borrow-mode build, then build again.** The
    checker records facts: `Checked::bind_facts` (each binding's links,
    each stamped by the binding that made it; whether it may take
    ownership, `may_take_ownership` at the binding; the `for` loops whose
    binding is a root), `derived_uses` (every move or mutation of a derived
    variable, with the S1 message and the stamps of its own links; the use
    then goes on as an owned value's move), `virtual_candidates` and
    `by_value_loops`. `ir::build_program` builds the bodies with every
    binding borrow-mode, runs `flow::decide` per declaration (at each
    derived use, every binding behind it that may take ownership does: the
    variable's own, then back along the chain as the IR's links stand on
    that path; and the virtual places whose binding has not taken
    ownership on that path and, for a survivor, whose local is still
    live), and builds the modules with a decision again from it
    (`ir::Decision`, on `ir::Program::decision`). The rebuild is what keeps
    emitted code identical without teaching an IR rewrite every place a
    mode reaches (a by-value loop changes the element's type, which flows
    into the binding locals and test subjects). The builder's `Ctx::mode`,
    `virtual_bind` and `bind_links` replace `binding_modes`,
    `virtual_place_binds` and `fate_binds`/`fate_bound` (with the walk's
    `finish_fate` filter: no link copied from a decided ancestor). The
    check walk reports a derived use none of whose own bindings may take
    ownership as the S1 error and does not make that move (no second error).
  - **The links stay the checker's facts**, as allowed: an `is` binding is
    the same storage as its subject in the IR (a `Narrow`, with no
    statement of its own), and a derived return, a held view and an
    element identity are typing facts. The IR computes the chains and the
    per-path state from them.
  - **The differential** (temporary, `SALVO_XP_P8`): the old checker behind
    a thread-local switch next to the new one in `check_program`, over
    every program the suite checks, e2e included; it compared the gates'
    diagnostics, the IR dump of every module (std included) and twelve
    tables. It found two bugs, both fixed: the flow pass's `forget` dropped
    a link to a rebound name with its stamp, so the pipeline's
    `let name = person.name` lost its decision at the loop's exit (now a
    `GONE` link: poisons nothing, keeps the stamp); and `is` bindings were
    never eligible, so std's `fs/path.sv` had 30 false S1 errors (facts are
    now noted for every binding in a body's span). After that the gates and
    the IR were equal everywhere. Tables differed only on rejected
    programs. Then the old checker, the hook, the ownership-blind switch
    and its audit test (two identical walks now) went. Emitted code did
    not change.
  - **Timings** (`SALVO_TIMINGS=1 target/debug/salvo analyze --src demo`,
    debug, three runs): check 178–197ms before (walk 169–188ms), 170–181ms
    after (walk 162–173ms). A whole-program IR build (std included, for
    emission) is 35ms: 22ms of bodies, 2ms deciding, 9.5ms building again
    the modules with a decision (most of std's). Rebuilding per
    declaration rather than per module would cut the last part, if it
    matters.
  - **What is left of R8.** Lambda capture moves, handler-storage moves and
    lends validation are static move-site rules, still the checker's. None
    of them *needs* moving now: they read the lambda boundaries, the
    contract and the handler's declaration, and no flow state (what they
    marked consumed, the IR moves through `lambda_captures` and the
    moves they record). Moving them is only for R8's "the flow pass owns
    lends validation" (or that sentence is amended, the user's call). Also
    left: the fate links themselves in the IR (needs an `is` binding with
    links of its own in the IR, and the typing facts as IR facts), after
    which `check_loop_body`'s link-triggered re-walk can go; and gating per
    declaration (a checker error anywhere still hides every flow
    diagnostic, the S1 error now included).
- **R8 piece 9 (per-declaration gating): done** (committed "Check
  ownership per declaration, beside type errors elsewhere"). The IR is
  built, and the flow pass and the linear check run, for every declaration
  whose own checking produced no error [ir-analyze]; emission still refuses
  any program with an error.
  - **The skip rule, as built** (`ir::gate`). (1) Attribution, from the
    errors' spans: an error at a stamp's `by` site or region target with the
    stamp's "in `f` from `x` for `…`" prefix breaks that stamped fn's body
    only; otherwise every item containing it breaks — its body when inside a
    fn's (or a handler member's) body, its interface elsewhere; an import,
    no item, past the file's end, or any parse/expansion error breaks the
    file. (2) `build_program_gated` (a `Gate` on `Ctx`) builds a broken
    declaration's interface only and a broken file not at all, and answers
    the items whose build reported an error. (3) `gate::Checkable`, in a
    program with any error, skips a broken declaration, one whose build
    failed or holds `Unsupported`, and one that calls or names a broken
    interface (`FnRef::Decl`, `ReplyTarget::Task`, `MemberRef.interface`, a
    type key anywhere in its IR). (4) `analyze_full` takes the parse
    diagnostics; `analysis.rs` calls it always and lists its diagnostics
    after the checker's; `check_for_emission` and `dump_program` list the
    checker's errors, then the IR's, and still refuse.
  - **Changed from the plan:** no `Ty::Unknown` rule — a census found it in
    the IR of 84 clean programs of the suite. The checker keeps no
    per-declaration error bookkeeping; spans plus the stamp prefix sufficed.
  - **The audit** (temporary `SALVO_XP_P9` hook, every program the suite
    checks, toolchain tests skipped): no builder panic on any errored
    program; six had IR diagnostics or builder failures, the diagnostics all
    real (the leaks `l6_linear_obligations` and `l7a_generic_linear_opt_in`
    now assert). Differential: each clean program again with an unrelated
    broken module — the IR diagnostics were identical in all 104 that had
    any.
  - **Tests:** `ownership_errors_are_reported_beside_type_errors_elsewhere`
    (analyze, both compiles: a type error beside another fn's use after a
    move; one fn with both; a broken-signature callee's caller; a stamped
    `hash` with an error; a parse-broken file) and
    `ownership_errors_show_beside_a_type_error` (LSP). The l6/l7a tests
    split for per-program gating were merged back; the test helpers that ran
    `ir::analyze` only on error-free programs run it always.
  - **Timings** (`SALVO_TIMINGS=1 target/debug/salvo analyze --src demo`,
    debug, three runs): check 182–185ms before, 176–183ms after; IR
    analysis 1.0ms both (demo/ is clean). With an unrelated broken file, the
    IR analysis now runs: 1.0–1.1ms.
  - **What is left of R8.** Lambda capture moves, handler-storage moves and
    lends validation (static move-site rules, still the checker's; moving
    them is only for R8's "the flow pass owns lends validation", or that
    sentence is amended — the user's call). The fate links themselves in the
    IR (an `is` binding with links of its own, the typing facts as IR
    facts), after which `check_loop_body`'s link-triggered re-walk can go.
    The gate is span-based: an error the checker reports at a span inside
    another declaration than the one it was checking breaks the wrong one
    and leaves the right one checked; rule 3 is the safety net, and the
    audit found no false report from it.
- **Step 7, piece 3 (02-resolve-03-interfaces as its own pass, with its own
  diagnostics): done** (committed "Check every declaration's interface
  before any body"). [interface-pass]: `check_once` runs `walk_interfaces`
  (step one `check_signature_facts` over every file, then `check_module` in
  `Pass::Interface` and the platform boundary over every file), then
  `walk_bodies` (`check_module` in `Pass::Body`). `walk_signatures` and its
  dropped diagnostics are gone.
  - **Design: records, not a silent re-run.** The interface walks keep a
    per-file `FileIface`: a `SigFacts` and a `FnSig` per fn and a
    `HandlerSig` per handler (by declaration address), the module's
    renames, and the once-per-file diagnostic memories. The body walk
    installs them (`install_sig`) and re-derives nothing. `check_fn` =
    `fn_interface` + `check_fn_body`. Signature facts are a step of their
    own because other headers read them from tables (a written `Set<T>`
    resolves `T`'s `hash` through `implicit_params`, and the platform
    boundary does the same).
  - **The classification** (`check_module`'s and `check_fn`'s per-item work):

    | work | half |
    |---|---|
    | module: return-type overloads, Kotlin name clashes, doc tags, scalar `!v` warnings, renames | interface |
    | module: `use` bindings [mod-use] | body: `use` statements, read only by this file's bodies |
    | fn: implicits, `+Q` claims, effect list (step one); parameter and return types, `proj Mut`, type-named fn, constructor signature, conditional responses, `preserve`, lends (`fn_lends`), `proj`/`ref` sources, `proj(p)` kept, `canbe linear`, handler deps merged, `main` and `Throw`, the `+Q` trusted check and the return type the body is checked against (both moved up from after the body gate); platform-fn rules; free send fn; the declaration's own `fn_refs` row | interface |
    | fn: a written lend list against what the body visibly lends | body: it reads the body |
    | fn: parameter defaults [fn-param-default], the parameter frame, root claims, the walk, declared mutation paths, missing return, `fn_body_effects`; a stamped fn's comptime roots; `own_discharges`, `own_task` | body |
    | handler: platform rules, implicit constructor parameter types, faces (duplicates, kinds, bodyless, `Throw`), constructor parameter types, conformance, dependencies (`handler_deps`), `spawn`, mixed, mailbox presence, names, state field types (`canbe Mut`, `proj`, linear), `init` clashes, member signatures (no effects, send member, private send member) | interface (`HandlerSig`) |
    | handler: constructor defaults, the mailbox slot's expression, state initializers, member bodies | body: expressions checked in a frame of locals the body walk numbers |
    | effect alias; effect (prerequisites, route keys, member signatures and implicits, kind) | interface |
    | qualifier, struct and `type` headers; `params` groups | interface (a struct field's default stays in the body walk; a qualifier's fns split as fns) |
    | platform boundary plans (after the body walk until now) | interface |
    | the minter of an `iter fn` whose state infers a field type | interface, body and all (below) |

  - **Diagnostics: the same set, each once.** The order changes (every
    file's interface diagnostics before any body's). `salvo analyze` and
    the LSP sort by file and position, and no test depended on order. No
    span moved, so `ir::gate` blames the same declarations. Emitted code
    did not change.
  - **The differential, retired.** `SALVO_XP_P3` ran the old staging (a
    `Pass::Both` walk after the dropped pre-pass) beside the new one in
    `check_program` and `check_project`. It compared every `Checked` table
    as a sorted multiset, diagnostics included, over every program the
    suite checks, e2e included (2,005 distinct programs). It found two
    latent last-writer bugs, both fixed: `foreign_ref` (a callee's signature
    lowered at a use recorded its type names' definition sites under this
    file: `lib/std/fs/mem.sv`'s `Str` went to `Int`), and effect members
    lowered without the effect's generics (`written_types` read `M` as a
    named type). After the fixes, nothing differed.
  - **An interface read off a body, fixed:** an `iter fn`'s inferred state
    field type came from the minter's body walk, so a fn declared above it
    read `Unknown` (`return c.at` from a `-> Str` passed). Such a minter
    (`body_pass_of`) is checked in the header walk now: its body is the
    initializers (R5 option b). Not fixed: inferring minters among
    themselves still go in declaration order (ROADMAP 0r).
  - **Does any interface still read a body?** No checked body. Two read a
    body's *text*: `parking_handlers` (syntactic, 4.5) and an inferring
    minter (its body is the declaration's state block). Outside checking,
    one body fact crosses declarations: `param_mutations` (a generic
    container mutated through a minted handle makes the parameter
    `LentMut`, which callers' pass modes read in `param_mode`). Step 8's
    IR cache must treat it as a callee-body input of a caller's IR, or the
    fact becomes a signature one.
  - **Interface outputs for step 8's per-declaration cache.** The checker
    tables written by the interface walks, and their keys:
    - step one: `implicit_params` (`FnKey`), `implicit_members` (file,
      member name span), `established_quals` (`FnKey`, parameter),
      `decl_effects` (file, name span), `fn_effects` (`FnKey`);
    - headers: `fn_lends` (`FnKey`), `handler_deps` (handler key),
      `effect_prereqs` and `route_keys` (effect key), `boundary_checks` and
      `factories` (file, span), the declaration's own `fn_refs` row (file,
      name span), the `expr_ty` rows of inferred `iter fn` state fields
      (file, field name span);
    - mixed (header rows from the interface walks, body rows from the body
      walk): `written_types` (AST address), `def_refs`/`field_refs`/
      `module_refs` (file, span), `expr_ty`, and `carried_identities`
      ((`FnId`, `Ty`), first writer program-wide: piece 4's latent issue);
      a per-declaration cache needs the header rows split from the body rows;
    - before the walks (already interfaces): the contracts
      (`deduce::contracts`, `FnKey`), `refinements`, `protocol_hashes`,
      `parking_handlers`, and the indexes (`type_ref_keys`, `visible_keys`,
      …);
    - outside `Checked`: `FileIface` (`SigFacts`, `FnSig`, `HandlerSig`,
      renames), keyed by declaration address today. That is the
      per-declaration interface record to cache, once stable ids replace
      addresses.
  - **Timings** (`SALVO_TIMINGS=1 target/debug/salvo analyze --src demo`,
    debug, three runs each): check 186–188ms before, 179–187ms after; the
    walk 177–180ms (pre-pass 5.3–5.6ms, bodies 155–158ms) against 171–179ms
    (`check: interfaces` 27.6–29.2ms = `interfaces: facts` 5.5–5.8ms +
    `interfaces: headers` 22.0–22.7ms; `check: bodies` 128–135ms).
  - **Left:** ROADMAP 0r. Header expressions (defaults, initializers, the
    mailbox slot) are checked by the body walk: moving them would renumber
    the body walk's locals. R8's leftovers (the three static move-site
    rules; fate links computed in the IR) wait: the first is the user's
    call, and the second needs IR-side `is` bindings.

- **Step 8, measured first (2026-10-20).** `tools/lsp-timing.py` drives
  `salvo lsp` through body edits with `SALVO_TIMINGS=1`. One edit of
  `examples/cluster` (373 lines, one project file, the std cache in force;
  debug at opt-level 1) cost **~85ms**: parse 11 (every `ParseCache` hit
  clones its AST, std's ~65 included), expand 17 (comptime resolves the whole
  program for its scoped names: 13), resolve 28 (scopes 11; the language
  server's overload and completion indexes 17, built for every std file),
  check 28 (the std seed's clone 4, written keys 4, protocol hashes **10.6**:
  the first `Symbols::ref_key` of an undeclared name resolved the whole
  program a second time, §6 prerequisite 5; interfaces 0.6, bodies 4,
  deduce 3), IR 2.5. **What the step names — the edited declaration's body
  re-walked and the rest reused — would save at most the walk's ~5ms of
  85.** The cost is the program-wide front half over std, recomputed on
  every edit. So step 8 is reordered, H's per-file level first, std as the
  unit that never changes:
  1. the indexes for viewable files only, `ref_key` seeded with the written
     keys the check computes anyway, finer phase timings, the driver
     (piece 1);
  2. std's front half shared across analyses: its parsed and expanded
     modules kept by the server and shared by every analysis's `Program`
     (no clone; stable addresses, which also ends the `StdCache`'s
     `written_types` rows keyed by the addresses of a dropped program),
     comptime run for the project's files only, std's written keys from the
     cache (ROADMAP §5 item 4's first half);
  3. the per-file checker rows (generalizing `StdCache` to every unchanged
     file) for multi-file projects, with its prerequisites: `written_types`
     keyed by something stable, `carried_identities` by file, `task_mints`
     keyed, `DefSite` targets remapped after an edit shifts them;
  4. per declaration, only if the numbers after 2–3 call for it.
- **Step 8, piece 1: done** (committed "Build less of every language-server
  analysis"). [lsp-index-viewable]: the overload, completion and
  type-completion indexes skip embedded std files (a project analysis's
  views never show one; `analyze_std` builds them for its own), and the
  completion index's in-scope test is a set, not a scan. `Symbols::
  seed_ref_keys`: `check_once` hands `ref_key` the written keys it has just
  computed. New phases: `expand: comptime scopes` (`comptime: resolve`,
  `comptime: names`), `expand: comptime stamps`, `resolve: symbols /
  scopes / lsp indexes`, `check: contracts`, `check: seed`, `written keys`,
  `visible keys`, `protocol hashes`, `platform closure`. The same edit:
  **~85ms → ~62ms** (indexes 17 → 0.5, protocol hashes 10.6 → 0.5). Left,
  per edit: parse 11, comptime's resolve 13, resolve 11, seed 4.6, written
  keys 4.5, bodies 4.5, deduce 3, IR 2.6. `salvo analyze` gains the
  protocol-hash part too. Next: piece 2.
- **Step 8, piece 2: done** (committed "Share the standard library's parsed
  modules across language-server analyses"). [lsp-std-front].
  - **Built.** `Program::modules` is `Vec<Arc<Module>>`; an expansion
    rewrites a module through `expand::ModuleSlot` (`Arc::make_mut`, no copy
    when the program owns it alone), and comptime leaves a module with
    nothing to stamp untouched (`stamps_anything`), so a `ParseCache` hit (an
    `Arc` now, not a clone) stays shared. `analysis::StdFront` replaces
    `build_std_cache`: std parsed and expanded once, its modules kept, the
    `StdCache` checked over a program sharing them; the cache holds the
    modules too (`StdCache::modules`), so its address-keyed rows can no
    longer outlive the nodes they are keyed by. `comptime::
    expand_comptime_prefix` records std's `StdComptime`: its tables as they
    were before stamping (the world's `base`), every name std's stamping
    looked a declaration up by, and every `by` path it matched module paths
    against. `analyze_project` takes std's modules from the front when
    `std_front_applies` (the leading files are the front's, by name, module
    and content; unshadowed; nothing after std but a shadow) and
    `StdComptime::admits` (no later file declares a struct, `type`, group,
    qualifier or comptime fn under an asked name, and no later module path
    matches a std `by` path); parses the rest; and runs
    `expand_program_after` → `comptime::expand_comptime_after`, which stamps
    the later files only. Else everything runs as before (std reparsed).
  - **Item 4 done.** `check_once`, when `StdCache::shares_modules`: std's
    `type_ref_keys` are the seed's, the project's are computed
    (`resolve::written_keys_from`), and the by-span keys are merged with
    std's gathered ones (`resolve::SpanKeys`, kept in the cache) by the rule
    gathering them together applies — an entry agrees only if every
    reference at that span agrees, so the parts merge exactly. Asserted by
    the soundness test against a full recomputation.
  - **Deviations.** (1) The gate needed its comptime half: std's expansion
    does depend on the project. A project module `auto` makes std's `by
    auto` sites name two modules (ROADMAP 0s, a defect of its own, found by
    the test). (2) Comptime resolves with `resolve::resolve_scopes_where`,
    which builds scopes for the stamped files only *and* skips the
    program-wide diagnostics and reference tables (duplicates, overload
    duplicates, the attached-export rule, `type_ref_files`, stamp scopes):
    filtering pass 2 alone left comptime's resolve at 7.4ms, most of it
    those checks over std. Comptime reads only the scopes, so `salvo
    analyze` and every compile save it too. (3) The soundness tests compare
    through `analyze_project_with(…, front_half)`, the same analysis with
    the front's cache but without its front half, so they isolate this
    piece from the `StdCache`.
  - **Numbers** (`tools/lsp-timing.py … examples/cluster …`, debug, five
    runs of six edits each way; medians): per edit **~62ms → ~45ms**
    (run medians 58–66 before, 41–48 after). parse 10.9 →
    1.8; expand 15.6 → 1.5 (comptime's resolve 11.9 → 1.0, stamps 1.2 → 0);
    written keys 4.2 → 0.2. The phases after expand measure 10–35% *slower*
    (resolve 11.1 → ~15, seed 4.4 → ~5.1, bodies 4.6 → ~5.6), the seed's
    clone included, which reads no AST: a 25ms busy loop before each
    analysis (no memory touched; an experiment, not kept) brings them back to the before numbers
    (resolve 11.3, seed 4.6, bodies 4.8; ~36ms per edit). It is the core's
    clock ramping up from idle, now that an edit's first ~25ms of work is
    gone, not added work; `salvo analyze` times as before.
  - **Left per edit:** resolve's scopes ~11 (std's ~65 files' scopes, every
    edit), the seed's clone of std's `Checked` 4.5, bodies 4.5, deduce 3,
    IR 5. **Next: piece 3** (the per-file checker rows, with the
    prerequisites listed in "Step 8, measured first"); std's scopes per
    analysis are the largest single cost left, and sharing them is part of
    ROADMAP §5 item 4 (Option B).
- **Step 8, piece 3 (the per-file checker rows): planned (2026-10-20).**
  Measured on the one multi-file project in the repository, `modules/aws`
  (7 files, 2,775 lines; `tools/lsp-timing.py target/debug/salvo
  modules/aws/salvo modules/aws/salvo/aws/s3.sv "export fn close(value:
  PutObjectInput) [Streams] -> Ok None | Err Checked<StreamError> => !value
  {"`), one body edit of `s3.sv`: ~56ms — resolve 17.7 (scopes 13.8),
  `check: interfaces` 14.8 (of it `ddb.sv`'s platform boundary 10.6, the
  `boundary_plan` recursion through DynamoDB's nested types for every
  platform fn), bodies 3.0, seed 4.4, deduce 2.9, IR 4.8. Reusing the six
  unchanged files' rows would save ~16ms of it. Design, from a survey of
  every `Checked` field:
  - **Validity of a file's rows**: its module is the *same `Arc`* as when
    they were recorded (a `ParseCache` hit that comptime did not stamp into;
    a stamped or reparsed module is a new one, so its file is walked), and
    the program's **interface fingerprint** is unchanged: the file list
    (names, modules, flags), each file's text with its plain fn bodies cut
    out (bodies of top-level fns, handler members and qualifier fns; kept:
    comptime fns, an inferring `iter fn` minter, header expressions), and the
    syntactic parking set (PIPELINE 4.5). A body-only edit changes only its
    own file's module.
  - **The rows**: everything the file's two walks write, snapshotted after
    `walk_bodies` and before `program_checks`, `deduce::infer`,
    `literal::lower_tables` and `unmangle_scoped_locals` (which run over the
    whole `Checked` every time); partitioned by key file (`Checked::
    take_file`, an exhaustive destructure as `drop_files` was in 94ef85dc).
    Prerequisites: `written_types` rows a walk writes for *another* file's
    nodes are kept only while that file's module is the same `Arc`;
    `carried_identities` made order-independent (keyed with the file, or
    resolved at the read); `task_mints` and the walk's `parking_handlers`
    inserts carried per file; the walk's diagnostics separated from the
    resolution's, the refinements' and the program checks'.
  - **What another file's edit shifts**: a clean file's `def_refs` /
    `field_refs` targets in an edited file move by the edits in the bodies
    before them; remapped from the old and new bodies' byte ranges (the
    same count and order when the fingerprint matched). Anything else
    pointing into another file's text (a message with its line, a span in a
    `Ty`) is found by the differential.
  - **Proof**: a temporary differential over every program the suite checks
    and over generated edits (a body edit of each file of each multi-file
    program, shifting later declarations): the cached check equals a full
    check, every table as a sorted multiset, diagnostics included; then a
    permanent test over `modules/aws` and a few multi-file programs.
- **Step 8, piece 3: done** (committed "Reuse unchanged files' checks across
  language-server edits"). [lsp-check-cache].
  - **Built.** `check_cache::CheckCache`, kept by the server beside its
    `ParseCache`, and `check_project_cached`, which the language server's
    check through the std cache now is (`salvo analyze` and the compile paths
    are unchanged). Per project file: the module `Arc`, the hash of its text
    with its plain fn bodies cut out, the bodies' byte ranges, its keyed rows
    (`Checked::split_files`, an exhaustive destructure; `insert_rows`) with
    its `param_mutations`, and per walk step (signature facts, headers,
    bodies; `StepRows`) what the step appended: diagnostics, the ordered
    tables (`waitfor_sites`, `handler_constructs`, `actor_gates`,
    `actor_sends`, `task_sends`, `task_mints`, `fn_body_effects`), the
    `written_types` rows it wrote (in order, with the row, whether only if
    absent, and the file whose node it is, from an address → file map kept
    over every module, `walk_module`), and the handlers it marked parking.
    `CheckCache::plan` keeps a file's rows when its module is the same `Arc`
    and the program's interfaces are as they were (same files, std's same
    modules, same syntactic parking set, every new module's text the same
    outside its plain bodies); `check_once`'s walks insert a cached file's
    keyed rows at its signature-facts step and each step's appends at that
    step, so the order of everything is the full check's, and walk the rest;
    `record` takes the walked files' rows before the program checks. The
    program-wide steps run as before. `CheckCache::walked` names the files
    the last check walked.
  - **Prerequisites (behavior-neutral; the suite passed with them alone,
    2048/2048):** `carried_identities` keyed by file as well (the first file
    to write an identity was the only one whose `fn_refs` and errors recorded
    it); `task_mints` carry the mint's span; the walk logs (`record`,
    `written_log`, `parking_log`); per-step marks for the ordered tables and
    the diagnostics.
  - **Deviations.** (1) `written_types`: neither "kept only while that
    file's module is the same `Arc`" nor "each file lowers its own types":
    dropping a cached file's rows for an edited file's nodes lost rows only
    another file's walk writes (a signature's nested type arguments, lowered
    in declared shape at a substituted use, by callers only), 63 tables in the
    first differential. They now follow the node: outside its plain bodies a
    body-edited file's old and new modules hold the same nodes in the same
    order (`node_map`, checked against `remap`), so each row moves to the new
    node. (2) The log keeps each step's own row and its if-absent flag rather
    than the final table's, so a writer that differs from a later one is
    replayed as it was. (3) The keyed tables go in with a file's first step,
    not split per step: no walk reads another file's keyed rows except
    signature facts (all written before any header), `handler_deps` (read
    for the walking handler's own), `written_types` (per step) and
    `carried_identities` (now per file). (4) A program with one project file
    skips the cache: that file is walked anyway, and plan plus record cost
    ~1ms (`examples/cluster`). (5) A file comptime stamps is a new module on
    every analysis, so it is walked every time (`modules/aws`'s `aws.sv`).
  - **The differential** (temporary, `SALVO_XP_P8`, in `check_program` and
    `check_project`): beside each check, a second full check (the control),
    a recording check, an all-cached check, and for every non-std, unstamped
    file with a plain body an edit adding a statement to its first body
    (every later declaration moves), cached against full, then reverted;
    every table as a sorted multiset (diagnostics, then their order), and the
    IR's diagnostics. 2,004 distinct programs (106 through the std cache),
    3,858 edits, 293 with more than one file. The control showed that a few
    `written_types` rows are keyed by nodes the checker builds and drops,
    which differ between two full checks; rows at no module's node were left
    out of the comparison. First run: 21 programs differed, every one a
    latent bug of the checker's or the first design's, fixed at its cause:
    1. `type_ref_files` (the file every written type reference is in, which
       `foreign_ref` and `scope_for_ref` read) missed the references in a
       qualifier's slots, a rename's parameters, parameter defaults and
       more: a use of `Heap<T>(?cmp: (T, T) -> Int)` from `main.sv`
       recorded `Int`'s definition site under `main.sv` at `heap.sv`'s
       span, and resolved the slot's names in `main.sv`'s scope. It is
       built by the exhaustive `walk_module` now.
    2. A predicate `is` test lowered *copies* of the qualifier's value slots
       (`value_slots.clone()`): `map.test.sv`'s `k is KeyOf(m)` recorded
       `Map`'s site at `map.sv`'s span, and rows at dropped nodes. It lowers
       the declaration's own slots.
    3. A module-level `rename fn` was checked and recorded in every file of
       its module: the annex `sorted.test.sv` held `fn_refs` at
       `sorted.sv`'s positions, and a mistake in a rename was reported in
       every file at the declaring file's span. It is checked in its own
       file; the others take the binding.
    4. The `written_types` rows of deviation (1).
    5. An obligation's `self` type was lowered from a temporary node,
       leaving a row at its address in every program with an obligation
       (found by counting rows at no module's node).
    Second run: nothing differed anywhere (one LSP test timed out under the
    hook's cost, `hover_works_in_a_std_shadowing_file`, and passes without
    it). The hook is gone; `check_cache_matches_an_uncached_analysis_across_edits`
    (`analysis.rs`) stays: over `modules/aws` and two written programs
    (qualifier slots, a module rename with an annex, a generic struct read
    through a substitution, an effect and handler, an error in a body), a
    body edit of each file in turn, accumulating, then all reverted, gives
    the same diagnostics, IR diagnostics and the rows the views read
    (`expr_ty`, `fn_refs`, `def_refs`, `field_refs`, `call_fn`,
    `local_decls`, errors, overloads, completions) as an analysis without
    the cache, walking only the edited file and the stamped ones.
  - **Numbers** (`tools/lsp-timing.py`, debug, medians of three edits, two
    runs after): `modules/aws`, one body edit of `s3.sv`: **~56.7ms →
    ~44–47ms**; `check` 29.7 → 17.7–18.2 (`interfaces` 14.5 → 2.6–2.8,
    bodies 3.0 → 1.2–1.3; new: `cache plan` 0.3–0.4, `cache insert`
    1.3–1.4, `cache record` 0.5–0.6). `examples/cluster` (one project file,
    the skip): `check` 18.8 before, 17.6–21.5 after, totals within the
    clock-ramp noise (33–54ms; see the gotcha).
  - **Left per edit** (`modules/aws`): resolve ~17 (std's scopes ~14), the
    std seed's clone ~5, `deduce` ~3.4 (`deduce::infer` clones the whole
    `expr_ty`, std's rows included, each check), IR ~5, `s3.sv`'s own headers
    ~2.3 (its platform boundary), the stamped `aws.sv` walked every time, and
    the insert cloning every cached row.
  - **Next: piece 4**: measure whether per-declaration caching (the edited
    file's other declarations) is worth building, and look at the remaining
    costs: std's scopes in resolve, the std seed's clone, `ddb.sv`'s
    boundary recursion (`boundary_plan` through DynamoDB's nested types for
    every platform fn), and a stamped module that comes out equal to the
    last one.
- **Step 8, piece 4: measured; per declaration not built** (committed
  "Stop copying the checker's tables to validate deductions"). After pieces
  1–3, one body edit costs (debug, `tools/lsp-timing.py`, medians of runs
  of four edits): `examples/cluster` **~28–39ms** (from ~85 before step 8):
  parse 1–2, expand 1–1.5, resolve 10–15 (std's scopes), check 13–17 (the
  std seed's clone ~5, interfaces 0.8, bodies ~5.5), IR ~3;
  `modules/aws` **~40ms** (from ~57): parse 2, expand 3.5, resolve 17
  (scopes 13), check 15–17 (seed 5, interfaces 3, bodies 1.3, cache insert
  1.6), IR 3. A per-declaration level (the edited file's other declarations'
  rows reused) can save at most the edited file's own walk: ~6ms on
  cluster, ~4ms on aws, while needing what the per-file level did not:
  rows partitioned by declaration inside a file (stamps and route stubs sit
  past the file's end, §8 step 5), spans relative to their declaration, and
  stable declaration ids. **Not built**; recorded in ROADMAP with these
  numbers, for when files grow or the larger costs are gone.
  - **Built instead, three costs found while measuring:** `deduce::infer`
    cloned the whole `expr_ty` and `effect_calls` (std's rows included) to
    hold them across loops that never write them (3.5ms → 0.4); it also
    re-validated every std handler member's contract under the std cache,
    which the cache's own check had done (now skipped like std's fns); and
    pass 2 of resolution built every file's `std.core.*` part from scratch —
    it depends only on which core module the file is in, so it is built
    once per such module and cloned (exact by construction: adding at
    `Level::Core` into an empty scope reports nothing; should it ever,
    every file builds its own). Pass 2 on cluster: ~7.4ms → ~4.6.
  - **Left, largest first** (ROADMAP): std's scopes rebuilt every analysis
    (~10–13ms: 24 core bases plus pass 1 and the address maps over std;
    sharing them across analyses needs the scopes to outlive the program
    that borrows them, ROADMAP §5 item 4); the std seed's clone of std's
    `Checked` (~5ms, needs a layered `Checked` or a cache that hands rows
    out without copying); `ddb.sv`'s platform-boundary recursion (10ms when
    walked, `boundary_plan` per platform fn through DynamoDB's nested types,
    unmemoized); a stamped module is walked every time even when it comes
    out the same; the cache's insert clones every cached row (1.6ms).
  - **Step 8 is complete as re-scoped.** Every step of §7 is built. Open:
    R8's leftovers (the user's call, §8 piece 9), comptime step B (waits on
    ROADMAP §2c's DECISION), ROADMAP 0r, and the costs above.
- **After step 8 (user, 2026-10-14): the doc stays open** as the working
  doc while ROADMAP 0s and R8's leftovers are dealt with. 0s first, decided:
  a `by` path names only modules the stamping file can see; std's
  diagnostics are hidden unless `std = true`; then the `std.` prefix (TF-8),
  so that "std sees only std" is a rule about the `std` namespace and the
  std front's comptime gate can go entirely. Piece 0s-1 (visibility-limited
  `by`, hidden std diagnostics) is next; the prefix needs the open TF-8
  calls first (whether `core` becomes `std.core`, the tree's layout).
- **0s, piece 1: done** (committed "Let a by clause name only the modules
  its file can see"). Built: the resolver records each file's visible
  modules (`ModuleScope::modules`); a `by` path's module and comptime-fn
  readings consider only those [obligation-by]; a std file's name fallback
  considers std's declarations only; `StdComptime::admits` refuses only a
  project module under `std.core` and is asked before parsing (the queried-name
  and `by`-path bookkeeping and the std reparse are gone); diagnostics
  located in the embedded std are hidden from analyze, the language server
  and every build path, with one unattributed line when hidden errors remain
  [diag-std-hidden]. The repro analyzes clean and passes the front's gate.
  Left: the `std.` prefix (TF-8), after which the gate can go; it needs the
  open TF-8 calls first.
- **0s, piece 2 (the `std.` prefix, TF-8): decided (user, 2026-10-14)** —
  `core` becomes `std.core` (implicitly visible as before) and the tree
  moves to `lib/std`, module paths still derived from the layout. Being
  built; afterwards no project module can be implicitly visible to std, and
  `StdComptime::admits` goes.
- **0s, piece 2: done** (committed "Put the standard library under std").
  Built: `git mv std lib/std`, the tree's manifest at `lib/salvo.toml`
  (source root `lib/`, so `lib/std/core/list.sv` is `std.core.list`; the
  embed root is `lib/`, and embedded file names stay `std/…`), std's
  platform root at `lib/platform` with its host files under
  `lib/platform/std/`; `ModulePath::std`/`is_std_core`/`is_std_internal`
  replacing every module-path string match; `std` reserved [std-root] (a
  non-std tree's module under it is an error at the file, and not implicitly
  visible) and the bare spelling refused with the corrected path named;
  `StdComptime::admits`, its call and comptime's `shadow_copy` removed — the
  front stands in for every program `std_front_applies` matches. Every
  program, example tree, `modules/aws`, the docs and both runtimes swept.
  Deviations: the misplaced module is also kept out of `std.core.*`
  visibility (one error of its own instead of a page of hidden std errors);
  [diag-std-hidden]'s test now builds its std errors from a `std = true`
  dependency; the redundant-`@` warning echoes the selector as written.
  Left: ROADMAP 0t (`--src lib/std`), `demo/demo.sv`'s `import heap` (the
  user's). Next: R8's leftovers, awaiting the user (§8 piece 9).
- **R8's leftovers settled (user, 2026-10-14): option (b), R8 amended.**
  Lambda capture moves, handler-storage moves and lends validation stay
  checker rules: they read no flow state, and what they consume reaches the
  flow pass as recorded moves (§3 R8, "Amended"). Nothing to build. Still
  open, engineering only: the fate links computed in the IR (needs an `is`
  binding with links of its own), after which `check_loop_body`'s
  link-triggered re-walk can go. `demo/demo.sv` now spells `import
  std.heap` (the user's file; not committed by the agent).
- **Comptime step B's DECISION made (user, 2026-10-14):** a stamped member
  has the signature it is declared with — the written one for `fn … by X`,
  the group member's with `self` substituted for an obligation clause, which
  behaves as one `fn … by X` per member — and the template supplies only the
  body (ROADMAP §2c). Built first in today's expansion; the
  signature/body split itself still waits on stable ids and relative spans.
- **comptime signatures: done (committed "Give a stamped member the
  signature it is declared with").** One stamping path in `comptime.rs`: a
  clause builds each member's declaration from the group member
  (`Stamper::member_decl`: `self` substituted, qualifiers kept, the
  [comptime-generic] additions keyed by the member's name) and fills it with
  the same `Stamper::stamp_into` a written `fn … by X` uses, which now copies
  only the body. The template's parameters bind positionally (renamed after
  the expansion; a capturing local renamed apart to `name__p`); its effect
  list and deductions no longer reach the stamped fn; the positional
  type/return check is kept and covers clauses too. The deduction pass's
  contract errors at a clause member's synthetic spans are redirected to the
  `by` site (`comptime::redirect_stamped`). No program's behavior or emitted
  code changed (every std and test template already spelled its group's
  signature and the default contract); 7 new tests
  (`stamp_signature_tests.rs`). Deviations: a written `fn … by X` at a
  generic type is still refused. Left: the signature/body split itself (§5)
  on step 8's stable ids and relative spans; a group member with type
  parameters of its own loses them in `member_decl` (none exists yet).
- **router in-edges: done (committed "Report deadlock cycles that close
  through a router").** A dependency on `any E` lands on `E` and on every
  router of `E` (written `of any E` and `__Route_E`): occupancy into one that
  locks or waits (the route always), the caller's own kind into a bare one;
  a router's forward to `E` is no edge. The R8 piece 6 repro reports;
  no example, `modules/aws` or std test reports anything new.
- **Fate links stay the checker's (user, 2026-10-14: option C).** Moving
  them into the IR was weighed and rejected. Besides feeding the IR, about
  20 checker rules read the flow-tracked links during type-checking, two of
  them typing rules: which value a dependent claim (`KeyOf`, `Idx`,
  `SpanOf`, `NotSame`) is about, which decides claim-dependent overloads
  (the total `get(m, k)` returns `V`, the partial one `V?`), and whether an
  iterator variable's elements are borrowed or handed over. Option A (a
  monotone per-variable derivation in the checker, precise links in the IR)
  would have refused correct programs that re-point a borrowed variable, for
  tidiness and ~5 of 38 loop re-walks on `examples/cluster`; B kept two link
  computations nothing keeps equal; deferring the overload choice to the IR
  would interleave typing and flow again, since the chosen overload's result
  type types the rest of the body. **The lesson for later evolutions:**
  Salvo's claims are types, so the flow facts a claim's identity needs
  reach typing; a pass split that moves flow out of the checker has to
  leave those facts (or redesign claims to name the variable written rather
  than its ultimate root). Found on the way: ROADMAP 0v (the Rust emitter
  types a re-pointed borrow from its first binding). R8 is now closed:
  nothing of it is left to build.
- **0t: done** (committed "Refuse a --src inside a project's source root").
  `--src` is the source root [manifest-src]: one strictly inside the found
  project's root, or around it, is refused naming the root; `--src lib/std`
  no longer loads std twice. The implicit-resolution decision it left open was
  made the next entry.
- **Implicit resolution follows call resolution (user decision 2026-10-14):
  done** (committed "Resolve implicits the way calls are resolved"). One
  resolver (`resolve_implicit_fn_in`): fit every visible overload by the
  wanted fn type without filling its own implicits (`fn_value_fit`), the
  template fallback [comptime-hygiene], [implicit-fit]'s shape drop, the most
  specific signature, then fill only the winner (`ImplicitMiss::Unfilled`
  on failure, no fallback). By-name fitting uses `fn_value_fit` too, and the
  interpolation's first-declared `to_str` fallback is gone. **Census**
  (temporary `SALVO_IMPLICIT_CENSUS` switch, both rules side by side, now
  deleted): `cargo test` logged 6 differences in 5 tests, every example,
  `modules/aws`, `bench/*` and `demo/` none; no program rewritten, no
  generated code changed; classes (a) 1 test, (b) none, (c) 3 tests, (d)
  none. Doubled std (pre-0t `--src lib/std` on a tree copy): >120 s killed →
  0.67 s. IMPLICIT_RECURSE.md retired into COMPLETED.md. Found: ROADMAP 0w
  (by-name selection still by rung) and 0x (a written override is checked
  invariantly — a DECISION, since every by-name fn value is; not fixed).
- **0w: a function passed by name is chosen as a call chooses (user
  decision 2026-10-14): done** (committed "Choose a function passed by name
  the way a call chooses"). `Checker::select_fn_value` is the shared
  selection (steps 1–3 of `resolve_implicit_fn_in`, moved); `fn_value_by_name`
  keeps its `@`, its single-candidate shortcut and its no-expectation error,
  and calls it (`choose_by_name`). **Census** (temporary
  `SALVO_BYNAME_CENSUS`, deleted): `cargo test` one line (the census's own
  message at an error both ways), examples, `modules/aws`, `bench/*`,
  `demo/`, `--src lib` none; nothing rewritten, no DECISION. Fixed on the
  way: an `@`-written fn value as a call argument now takes the parameter's
  expected type (`apply(tag@main, 1)` with a generic `tag<T>`).
- **0x: done** (committed "Let a written fn value fit across erased
  qualifiers"; user decision 2026-10-14, option (b)). A written fn value (an
  override, a by-name argument, a fn-typed local) fits a fn type
  contravariantly in erased qualifiers [fn-erased-variance]: `is_subtype`'s fn
  arm compares parameters with `types::fn_param_fits`, `unify` binds through
  the stripped bases; `Mut`, `proj`, `ref`, `once`, `linear` and effects stay
  invariant, results stay covariant. No adapter, no emitted-code change.
  Options (a) (adapters where the rendering differs) and (c) not chosen.
- **0v: done** (committed "Give a re-pointed view a Rust variable per live
  range"). Two causes, both Rust: the `List` fast path of `filter` answered
  owned clones under a view's type (a nested `proj T` over a variable a
  source parameter holds now renders `&'a T` [rs-proj-generic], the host
  borrows, a Copy element is copied out at the call); and one variable holds
  every borrow ever stored in it, so [rs-repoint] (`ir_emit/repoint.rs`)
  splits a view local into a variable per live range (a shadowing `let`, or
  `let mut v;` before an `if` whose arms all re-point it). Found: ROADMAP 0y
  (a fn parameter forwarded to a retagged slot, E0308; not fixed).
- **The overlong implicit-chain error: done** (committed "Shorten the error
  for an implicit that cannot be filled"). `ImplicitMiss::Unfilled` carries
  a structured `Unfilled { links, why }` (a nested position's failure is
  recorded in `nested_miss`, not worded per level); `unfilled_text` names the
  position written and the one that failed, the compact chain between
  (elided past four links) and the remedy. The 11-deep depth-cap case:
  3,115 → 764 bytes. No DECISION.
- **0y: done** (committed "Adapt a forwarded fn value to a borrowed slot").
  A fn *value* (a parameter, a local, a field read, a forwarded implicit, a
  call's result) at a slot the call retags to a borrow [rs-proj-arm] is
  wrapped in a closure that peels the extra reference and converts the other
  arguments' pass modes (`fn_adapter` in the Rust emitter,
  `&mut |__a0: &&String| keep(*__a0)`). Fixed on the way: a fn value a fn
  returns is now stored as its `Arc<dyn Fn>` return type says, and an `Arc`
  fn value (field, call result, a local bound from either) passed to a
  fn-typed parameter is called through a closure typed by the instantiated
  slot (a plain generic slot used to get the uninstantiated `&T`). Rust
  backend only; no core fact, no emitted-code change in the examples. Found:
  ROADMAP 0z (an element of a view list read through a generic `proj(list) T`
  return, E0308) and a §7 DECISION (a fn value over owned elements at a
  view-element slot is refused by [fn-erased-variance]).
- **0z: done** (committed "Flatten a borrow of a borrowed element on Rust").
  A callee whose result is `proj T` rendered `&'a T`, instantiated at
  `T = proj Str`, answers `&&String`; the call now copies the outer reference
  out [rs-proj-flat] (`.copied()` on `get`/`last`, `*` on `first`, arm by arm
  on a `next`), with the instantiation read off the arguments' types where
  the IR states no type arguments (a `for` loop's `next`, a `?Yield`
  adapter). Rust backend only; no core fact, no emitted-code change in the
  examples.
