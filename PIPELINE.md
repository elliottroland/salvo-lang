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
   03-generate-02-flow, after the audit (§3 "Open"). **Do not forget**: the
   deadlock graph reads the AST, so the IR-derived `__Route_X` (step 6,
   piece 5) has no router node; when deadlock moves to 03-generate-03-link
   over the IR (here or in step 8), give every routed protocol its router
   servant node back, with its selector dependency's edges, and add a test
   of a cycle through a route (ROADMAP "A derived route has no deadlock
   node").
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
