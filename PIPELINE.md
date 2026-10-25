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
