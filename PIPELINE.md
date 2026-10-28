# The compiler pipeline: phases, sub-phases, and caching (working document)

**Status:** Part I (§1–§8) built; Part II (per-phase lowering, §II.1–§II.12)
decided 2026-10-27 and being built. Rounds: R1–R8 (2026-10-20, Part I), R9 and R10
(2026-10-27, Part II). Part I's plan is §7 and its log §8; Part II's plan is
§II.9 and its log §II.12. §5 ends with what stays program-wide.

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

### R9 (2026-10-27): per-phase lowering (Part II)

Six rounds the same day, in full in §II.11. **Direction:** per-phase lowering
for structural unity (no surface spelling grows semantics of its own), the
representation secondary, origins kept through every lowering. **Decided:**
P1 (overloads in a declaration id disambiguated by the written parameter
types), P2 (facts keyed `(DeclId, NodeId)`, the span kept as origin data;
amends R1's relative-span key), P3 (call resolution is the reference; every
other spelling behaves like the call it lowers to), P4 case by case
(effect members compete with fns; interpolation and operators per
OPERATOR_OVERLOAD.md O1: an operator is the call it names, and each operator
shape on a type is declared in the type's file, `[unique-shape]`), P5 (the
round trip on the corpus and examples), P6 (representation decided at D1),
P7 (one `salvo dump --phase parse-ast|resolve|type-check|generate-ir`), P8
(comptime's signature/body split only in C3); `by` at a call is not
supported (a named fn is the workaround). Recommended, not decided: resolve
and type stay separate (§II.3.1).

### R10 (2026-10-27): the census decisions (B2)

The user's answers to §II.13's "Decisions needed beyond P3", recorded as
**B2**. The guiding rule, the user's words: *follow the one-rule principle in
all cases where we can.*

- **D3: effect members fill every position a fn does.** An effect introduces
  function names into the declaring function's scope, so its members are
  visible in all the ways a normal function is: calls, interpolation parts,
  operators, implicits (a member as a value carries its handler), fn values.
  Ambiguity is always an error, with disambiguation (`m@E`) as the remedy.
- **D4: a general union `to_str`, not a lowering to `when`.** Lowering each
  part to a `when` gets out of hand for several union parts in one string;
  the ideal is a general `to_str` over unions that a call reaches like any
  other. **Deferred** if not easy now (then `interp_union` stays a fact until
  it lands).
- **D5: follow the call.** No "operators compare the data" exception: `xs ==
  ys` is `eq(xs, ys)` with the qualified argument types.
- **D6: drop the same-type rule** for `==` and `<`; a mixed `eq(Meters, Feet)`
  is an operator, placed with its left type (OPERATOR_OVERLOAD.md D-2).
- **D7: [op-arm-eq] stays a lowering before the call, for now.** Recorded
  for later (not now): a general way for more complicated cases, perhaps
  generating helper functions over the unions.
- **D10: no `: Yield` / `: Iter` gate.** With `next()` and `iter()`
  operators (O1), `for x in c` is `next(c)` when a `next` fits, otherwise
  `next(iter(c))`, resolved as those calls are; the declarations are not
  needed.
- **D13: a dot call sees fn-typed locals.** `x.f(a)` is `f(x, a)`, literally.
- **D14: an `eff[E]` receiver's call is an ordinary dot call.** In Salvo a
  send member is understood as taking `self: eff[E]` as an implicit first
  parameter, and `e.member(args)` is a normal dot call `member(e, args)`,
  ranked with fns taking `eff[E]` first; the backends still spell it as a
  send. The user was unsure of the knock-on decisions; they are listed in
  §II.12 when B3 piece 7 reaches them, and asked before building.
- **D18: a mint ranks by its captures**, like a partial call, and refuses a
  tie naming `k@m`.
- **D21: implicits of one name are ranked** like overloads.
- **D1: no semantic fast path.** `"${n}"` and `==` on scalars are the calls
  they name (an enclosing `?to_str` or `?eq` applies); the backends emit the
  native spelling exactly when the call resolves to std's intrinsic.
  OPERATOR_OVERLOAD.md O5's text is to be corrected accordingly.

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
- **§7 DECISION (b), a kept fn parameter reads a borrow: done** (committed
  "Let a kept fn parameter read a borrowed argument"). [fn-kept-borrow]: a
  written fn value whose owned parameter `T` it keeps fits a `proj T` slot
  (`types::fn_param_reads_borrow` + `fn_contract_keeps` in `is_subtype`'s fn
  arm, the same disjunct in `fn_value_fits`, and `unify`'s fn fallback); a
  consuming parameter is refused with its reason; `Mut`, `ref` and type
  arguments stay invariant. Census: nothing previously accepted changes
  meaning. Kotlin needs nothing; Rust reuses the retag adapters, plus a
  written-`proj` slot passing the borrow as it is and an owned key lent twice
  at a `K = proj Str` slot. No new DECISION.


---

# Part II — Per-phase lowering (decided 2026-10-27)

Folded in from PIPELINE2.md (2026-10-27), the user's design with another
agent while Part I's leftovers were finished. Part I's §7 plan is built; Part
II is what comes after it. Its rounds are summarized in §3 as R9 and kept in
full in §II.11; its sequence is §II.9, and its work log is §II.12. Operator
overloading, which decides Part II's interpolation and operator rows (P4), is
designed in OPERATOR_OVERLOAD.md.

## II.1. Goal

**Structural unity: surface syntax must not grow semantics of its own.**
Several rules have been found applied inconsistently across spellings of one
thing. The latest: implicit resolution selected overloads differently from
call resolution (`resolve_implicit_fn_in` beside `select_overload` in
`check.rs`, sharing the fit test `fn_value_at` but not the selection). The
cause is structural: wherever the compiler treats two spellings as different
constructs, each grows its own code path, and a rule changed on one is not
changed on the other.

The remedy is **per-phase lowering**: each phase lowers the program to a
simpler form, every spelling of one concept becomes the *same* node before
any rule about that concept runs, and the next phase works only on the
simpler form. The IR is the last of these forms.

- Each phase's output is **complete** (nothing the phase is responsible for
  is left for later) and **dumpable** in one consistent way.
- **The representation is secondary.** Side tables or trees, what matters is
  that the lowered form exists, is the only input of the next phase, and
  has one node per concept. (User, 2026-10-27.)
- **Every lowered node keeps its origin in the user's code** (§II.4), so errors
  from any later phase still land where the user wrote the thing.

## II.2. Principles

- **Placement rule.** A feature is interpreted in the earliest phase whose
  output has every fact it reads. It may wait later only while its output
  feeds nothing earlier.
- **One-way boundaries.** A phase boundary is worth keeping where the
  dependency runs one way: the earlier phase never needs anything the later
  one computes. Where a fact must cross back (the fate links claims need,
  R8 option C), it stays in the earlier phase and is handed on.
- **Lower before judging.** A rule (overload selection, contract fit,
  claims, ownership) is written against the core node, and every surface
  form reaches it as that node.
- **Origins, not fake spans.** Synthesized nodes carry where they came from;
  nothing invents source positions.

## II.3. The phases

| phase | output | contents |
|---|---|---|
| 1 `parse-ast` | AST | tokens, AST, sugar that needs no names (namespace flattening, struct-fn hoist, tests) |
| 2 `resolve` | resolved tree | declaration ids, per-file scopes, interfaces (signature types, default deductions, effects with aliases and prerequisites, implicits, lends, `by` member signatures), then comptime (§II.5), then body name binding |
| 3 `type-check` | typed partial IR | every callee picked, type arguments, implicit arguments explicit, call-like forms as calls, narrowings as bindings, coercions as nodes, each expression's type, claim links |
| 4 `generate-ir` | valid IR | erasure (qualifiers dropped, effects as parameters, union wrapping), ownership flow, linearity, contract validation; then the program-wide link stage (actor glue, `ref` containers, throws, deadlock) |
| 5 emit (no dump) | target source | per backend |

### II.3.1 Resolve and type stay separate

Recommended (2026-10-27), on the one-way rule: resolution can be made to
never need a type, provided it stops short of every decision that does.
Resolution leaves these to typing, as a **candidate set** or an unresolved
name:

- which overload a call means (chosen by argument types and claims);
- a field read `x.f`, a dot call `x.f(a)`, an effect member (`println`), a
  leading-dot `.Variant` — each needs the receiver's or the expected type.

Everything else is lexical and resolved in phase 2: locals and shadowing,
imports, aliases, renames, module paths, names in signatures, effect
prerequisites, `by` paths.

What the boundary buys: interfaces computed without typed bodies (the rule
R1–R5 established, and what early cutoff rests on); name errors
reported whether or not a body type-checks; go-to-definition without typing;
a dump answering "what does this name refer to". Merging them would inline
lexical lookup into the type walk, which is how the program-wide plain-name
lookups (§5) appeared.

### II.3.2 The resolved tree

The program as if every name had been written fully qualified: every
reference an absolute declaration id, no imports or aliases left,
prerequisites and default deductions written into signatures, comptime
expanded. It does **not** write out what needs a type: `size(xs)` is still a
call to one of a set of `size`s.

### II.3.3 The typed partial IR

The program as if the user had also written everything the type checker
decides: callees as `name@module`, every type argument, every implicit
argument passed (`to_str = to_str@main`), every interpolation part, operator,
dot call and effect-member call an ordinary call, narrowings as explicit
bindings, coercions as explicit wrap nodes, each expression's type, and the
claim links.

**Partial** in two senses: a declaration with errors keeps holes (unknown
types, unpicked calls) so the language server can read it; and phase 4 still
adds ownership decisions (which reads consume, which `let`s move), verified
linearity, erasure, actor glue, routes and deadlock.

### II.3.4 Narrowing (type) and flow (generate)

Both are path-sensitive analyses over one body; they answer different
questions. Narrowing: *what is this value* (after `if x is Int`, `x` is an
`Int`); it changes types, types choose overloads, so it is typing's. Flow:
*is this value still there, and who may use it* — use after move, partial
moves, poison, move mode, linearity, the ownership half of lends
validation (`ir/flow.rs`, `ir/check.rs`).

The split is legal because of R5: ownership must not change
types or overloads, so typing runs ownership-blind. The exception is the
fate links claims need, which stay in typing (R8 option C). Flow running on
the IR also has a smaller language to cover: one `Loop`, two branch forms,
narrowing as bindings, every operator a call.

## II.4. Origins

Built for comptime copies in A2 (§II.12, [origin]): a copied node keeps
its template's span and carries an expansion id the stamped fn's `Stamp`
records. Before it, copies got synthetic spans past the end of the file, a
`StampRegion` map, and `comptime::redirect_stamped` to move their errors
back; the past-EOF spans are also what kept step 8 piece 4 from
partitioning cached rows by declaration.

The general form (as macro systems do it; rustc gives each node a span plus
an expansion id): every lowered node carries an **origin** — its span in the
code it was copied or lowered from, plus an optional link to the expansion
step that produced it:

- comptime: the `by` site, the root, the field;
- desugaring: which surface form the node came from (an interpolation part,
  an operator, a dot call, an implicit argument).

Errors render by walking the chain: today's messages ("in `cmp` from `auto`
for `Point.x`", "in this interpolation") with no fake spans. The same origin
is what lets the unified call node (§II.6) still name the spelling the user
wrote.

## II.5. Comptime

**Placement: after interfaces, before body typing** (phase 2, after the
interface sub-phase). Comptime reads only interface facts about its root —
fields, their declared types, qualifiers, docs, defaults, visible modules
for a `by` path — and produces bodies only, because a stamped member takes
the signature it is declared with (user decision 2026-10-14). Without that
rule stamping would create signatures and the phases would form a cycle.

After it, **phase 3 sees no comptime**: stamped members are the declared
signature filled with the template's body, `[for]`/`[if]`/`[when]` unrolled
and splices filled; in-place expansions are unrolled where written over the
root named there. One residue is not really comptime: `field.default`
(`Expr::FieldDefault`) is an ordinary node, "this field's declared default",
typed in the declaring file ([default-scope]).

**Expanding a resolved template.** Resolve the template once in its own
module, then copy it with substitutions. Hygiene then follows from the
representation instead of being kept by hand:

- no `__c<N>` / `h__p` renaming of locals;
- the `hidden_free_name` error ("parameter `cmp` hides the function `cmp`
  that `auto`'s `cmp` calls") cannot arise: the template's call already
  refers to a declaration or candidate set, and declared parameters bind to
  template parameters by position;
- no `pick_decl` fallback;
- `[if f.type is Str]` compares declaration ids, not names (today's flat
  `World` matches by plain name, a latent bug with same-named types in two
  modules).

What stays open until expansion is exactly what comptime supplies: a
`[field.type]` splice is a hole filled with a type id; `v.[field]` is a field
read, resolved by typing like every field read.

Today comptime runs in `expand`, before resolution, over a flat `World`
(§5, rated C). Moving it is the comptime "signature/body split"
Part I and ROADMAP §2c have waiting on stable ids and relative spans.

### II.5.1 `by` at a call: not supported (user, 2026-10-27)

A call-site stamp whose root came from the call's argument types would need
typing, so comptime would have to run on demand inside phase 3 and the
one-way boundary would break. Decided: not supported directly. The user
writes a named fn in the same module and passes that name:

```
fn my_hash(p: P) -> Long by hash@auto
… hash = my_hash …
```

(`fn my_hash(p: P) -> Long by auto` does not work: a `by` path names the
template fn after the declared fn, `my_hash`, so the template must be named
when the names differ.)

Recorded, not chosen: a call-site `by` that names its root explicitly
(something like `hash = hash by auto for P`), which would keep expansion an
interface-level fact. It is the named-fn workaround without the name, and
not needed while the workaround is acceptable.

## II.6. Call-like forms lower to one call

The first concrete target. `Checked` has about 15 tables that each record a
different kind of call: `call_fn`, `fn_refs`, `fn_value_calls`,
`member_calls`, `effect_member_calls`, `qual_member_calls`, `dot_calls`,
`local_calls`, `interp_to_str`, `interp_implicit`, `implicit_args`,
`fn_value_implicits`, `implicit_members`, `addr_calls`, `effect_calls`. Each
is a place where "a call" may be judged by its own code. The IR already
collapses them into one resolved call — after typing, when the rules have
already run N times.

Plan: lower interpolation parts, operators, dot calls, effect-member calls
and implicit arguments to ordinary call nodes (with origins, §II.4) *before*
typing, so selection, contracts and claims are written once. "Implicits are
automatically selected lambdas" (user principle) becomes a fact of the
representation. This can start on today's AST and tables: an AST-to-AST
rewrite, then the checker's special paths deleted one table at a time.

Before deciding its size: a **census** of the ~15 tables — for each, does it
go through `select_overload` and `fn_value_at`, or have its own selection or
fit code. That lists the divergences still in the code.

## II.7. Dumps

One printer per lowered form, a dump at every sub-phase boundary (extending
`salvo ir`).

**Dumps as Salvo, where possible.** Print anything that has surface syntax
in that syntax (`name@module`, explicit implicit arguments, explicit type
arguments). Then re-checking a dump must reproduce the same picks: a rule
that behaves differently by spelling fails the round trip. Not total —
narrowing bindings, coercions and stamped origins have no surface spelling,
so they print in a marked notation the round trip skips.

## II.8. Representation (open)

Three ways to build a form that fills in phase by phase:

- **A. A tree per phase** (nanopass; rustc's HIR → THIR → MIR). The type says
  what is known; an unfilled slot cannot be read. Cost: near-duplicate node
  definitions, conversions, several printers.
- **B. One tree, optional slots filled in place** (Swift's type checker).
  One printer, one walker; an unfilled read is a runtime bug, and "is this
  set yet" checks creep in.
- **C. One node shape parameterized by phase** ("trees that grow", GHC):
  `Expr<P>` holds a candidate set in phase 2, a declaration id in phase 3.
  One walker and printer, filling enforced by types. Cost: generic noise
  through a 34k-line checker.

Recommendation (not decided): C only for the slots that change across
phases (callee, type, narrowing), plain fields elsewhere, and the
post-erasure IR kept as its own tree, since erasure changes the shape
(effects become parameters).

## II.9. The plan

Five stages, in order. Each numbered piece is one piece of work (one
commit, AGENTS.md) with its spec, docs and records, and leaves the suite
green. The method that worked for every Part I piece applies
throughout: build the new side, run a temporary differential against the
old over every program the suite checks, then delete the old. Where a piece
needs a decision of yours, it names it (§II.10).

**Gates that apply to every piece:** IR goldens and both codegen corpora
unchanged unless the piece says otherwise; `salvo facts` snapshots reviewed;
the warm suite no slower (ROADMAP §0h's budget); language-server edit cost
(`tools/lsp-timing.py`, medians) no worse than step 8's numbers (~28–39ms
`examples/cluster`, ~40ms `modules/aws`).

### Stage A — Per-declaration identity (the queued work)

What every later stage stands on. No language change.

**What is there today.** Three identities, all positional or by name:
`FnKey { file, item }` (resolve.rs) for fns, `DeclId { module, item, sub }`
(ir/nodes.rs) in the IR, and plain or `Name§module` string keys for types
and effects (the module-qualified attempt was deferred with 58 sites that
assume key == written name, §8 step 5 piece 4). Facts are keyed
`Key = (file, Span)` (check.rs), with `Checker::key`/`Lower::key` the choke
points; the checker's variable ids are per file; stamps and route stubs
synthesize spans past the end of the file, so a declaration does not own
its rows by containment.

- **A1. One stable declaration id** for every declaration kind (fns,
  structs, unions, effects and their members, handlers, qualifiers and their
  fns, define fns, aliases): module path + name + a disambiguator for
  overloads: the written parameter types (P1). Replaces `FnKey`, the IR's `DeclId`, and the
  string keys; every `FnKey`-keyed table moves over. Includes the 58 sites
  that render or compare keys as written names: rendering goes through the
  id's display name, clashes decided when rendering (§5).
- **A2. Origins** (§II.4) replace synthetic spans: `Origin { span, via }`
  with an expansion record for comptime stamps and route stubs.
  `StampRegion` and `redirect_stamped` become renderings of the chain.
  Needed before A3: it is what makes a stamped body's facts belong to its
  declaration.
- **A3. Per-declaration node keys.** `Key = (file, Span)` becomes
  `(DeclId, NodeId)`, the span kept as origin data (P2); variable ids per declaration, not
  per file (header expressions can then leave the body walk). Diagnostics
  and the language server map back through origins.
- **A4. Facts stored per declaration.** `Checked`'s rows partitioned by the
  declaration that wrote them; the per-*file* re-walk (`rewalk_files`) and
  the language server's per-file check cache become per declaration where
  that is cheaper (step 8 measured the cache level not worth it
  alone; with A1–A3 it is a partition, not a project). `salvo facts` prints
  by declaration.
- **A5. Dropped (P8):** comptime's signature/body split is built once, in
  C3, on the resolved form.

### Stage B — Call unification

The largest unity win, and it does not wait on a new tree: an AST-to-AST
lowering before typing, then the checker's special paths deleted. Needs A2
(origins carry "in this interpolation" to the error).

- **B1. The census** (read-only, can run now): for each of the ~15
  call-like tables (§II.6), which code selects the callee, which tests fit and
  contracts, and how it differs from `select_overload` + `fn_value_at` on:
  the visibility ladder, specificity ranking, ambiguity, implicit filling,
  effect members vs fns of the same name, claims, generics. Output: a table
  *form → core call it should lower to → divergences found, each with a
  probe program run on both backends*.
- **B2. Decide the divergences** (decision **P3**): for each, which
  behaviour is the language's. The census also proposes what each form
  lowers to (decision **P4**).
- **B3. The core call node and its lowering**, one surface form per piece,
  each deleting its table(s) and checker path:
  1. implicit arguments (after the other agent's implicit-selection work
     lands, which already routes selection through call rules);
  2. interpolation parts (`interp_to_str`, `interp_implicit`; `interp_union`
     is per-arm rendering and may stay a fact). **Waits on the user's
     operator-overloading design** (P4): interpolation is an operator on
     that model, and the candidate lowering is a call to a std format fn
     that backends may spell as native interpolation;
  3. operators (with the same design);
  4. dot calls (`x.f(a)` → `f(x, a)`, `dot_calls`);
  5. effect-member and qualifier-member calls (`member_calls`,
     `effect_member_calls`, `qual_member_calls`). **A language change**
     (P4): effect members join the call's candidate set and compete with
     fns, instead of taking the name first;
  6. fn values, local calls and fn-value calls (`fn_refs`, `local_calls`,
     `local_call_names`, `fn_value_calls`, `fn_value_implicits`);
  7. `addr_calls`, `effect_calls`, whatever the census adds.

  Each piece may change behaviour only where P3 said so, with tests at the
  changed sites; elsewhere the differential must show no difference.
- **B4. Round-trip check** (§II.7), for calls: print the lowered calls as Salvo
  (`name@module`, explicit implicit arguments, explicit type arguments),
  re-check, and require the same picks. Where it runs is decision **P5**.

### Stage C — The resolve phase as a lowering

- **C1. The resolved form.** Every lexical name bound to a declaration id or
  a candidate set, per file; the program-wide plain-name lookups (~21
  checker sites, `deduce`'s tables, comptime's and the desugaring's
  first-match tables, the IR's fallback; §5) become reads of
  it. `Symbols::ref_key` stops re-running `resolve()`. Whether this is a
  tree or a table is decision **P6** (via §II.8).
- **C2. The resolve dump** (`salvo dump --phase …`, P7), as Salvo
  with every name qualified.
- **C3. Comptime on the resolved form** (§II.5): templates resolved once in
  their module and copied; delete the flat `World`, `__c<N>`/`h__p`
  renaming, `hidden_free_name`, `pick_decl`; `[if]` type patterns compare
  ids. The `by`-at-a-call workaround (§II.5.1) documented in the language pages.
- **C4. The remaining early sweeps to their phases**: what is left of
  `expand` that reads names moves after C1 (the `iter fn` expansion per
  file; effect aliases and prerequisites already in scope after Part I
  step 6 — confirm nothing still rewrites effect lists program-wide).

### Stage D — The typed partial IR

- **D1. Representation** (decision **P6**, §II.8) settled from what B and C
  taught.
- **D2. The checker writes the typed form** (§II.3.3) instead of `Checked`'s
  span tables, one table cluster per piece — the clusters the IR builder's
  view accessors already group (`call_info`, `use_info`, `spawn_info`,
  `pick_info`, `interp_info`, COMPLETED.md 2026-10-10), then the rest. The
  checker reads its own facts back from the typed form (`call_fn` alone has
  13 readers inside the checker; the hard part).
- **D3. The IR builder becomes the lowering of the typed form**: erasure as
  its own sub-phase (qualifiers, effects to parameters, union wrapping),
  then flow. The builder's ~87 table reads go.
- **D4. The typed dump and the full round trip**: B4 extended to types,
  type arguments and narrowings (marked notation where Salvo has no
  spelling).

### Stage E — The link stage, and retiring the documents

- **E1. Per-declaration summaries for deadlock's inputs** (handler
  statefulness, the mixed and monitor classification, `parking_handlers`,
  task mints), then deadlock onto 03-generate-03-link over the IR; the
  checker-table graph and its router synthesis go.
- **E2. Retire**: this document into COMPLETED.md's design
  record, open items to ROADMAP.md, rules (origins, the core call,
  phase dumps) to LANGUAGE_SPEC.md.

### Order, and what can overlap

A → B → C → D → E. B before C because it is the unity goal and needs only
A2; C before D because the typed form is smaller once B and C have removed
tables. B1 (the census) and the B2/P3 decisions can run during Stage A.
B3's interpolation and operator pieces wait on the operator-overloading
design (P4); the other B3 pieces do not.

## II.10. Decisions

Outcomes of the 2026-10-27 round first; what is still open says so.

- **P1. What disambiguates overloads in a declaration id** (A1).
  **Decided (user, 2026-10-27): the written parameter types** (text, before
  lowering): stable under reordering and body edits; changes when the
  signature does, which is an interface change anyway. Rejected: ordinal
  among same-named declarations (adding or reordering an overload shifts
  the later ones' ids). An ordinal remains the tie-break for textually
  identical signatures (whether those are already refused is to check).
- **P2. Node keys** (A3). **Decided (user, 2026-10-27): a `NodeId` per
  declaration**, assigned in walk order, as the key facts are stored under;
  uniform for written and synthesized nodes (the IR already has `NodeId`).
  The span is kept on every node as origin data (§II.4): language-server
  lookups and compile errors read it, it is just not the key. This amends
  R1's "span relative to the declaration" key. Rejected:
  relative spans as the key (a copy has no position of its own).
- **P3. Call divergences** (B2). **Decided (user, 2026-10-27): call
  resolution is the reference; every other spelling behaves like the call
  it lowers to.** The census lists each divergence with a probe and its
  consequence (programs newly refused or accepted); the user may name a
  divergence to keep as an exception. (The 2026-10-14 implicit decision,
  applied to every form.)
- **P4. What each form lowers to** (B2). **Case by case** (user,
  2026-10-27), from the census table. Two directions given:
  - **Interpolation and operators wait on the user's operator-overloading
    design** (OPERATOR_OVERLOAD.md; discussion and decisions O1–O6 recorded
    there; O1 decided 2026-10-27: an operator is the call it names, resolved by name like any call, and each operator on a type is declared once, in the type's module), its interaction with implicits and generics; interpolation is an operator on that model. Candidate direction (user):
    `"a ${x} b"` lowers to a call of a std **format fn**, which a backend may
    emit as its native interpolation ([core-layers]: core states the call,
    the backend spells it). Open inside it: how each part becomes text (the
    operator question proper: a per-part `to_str` pick, an implicit, or the
    format fn's own parameter), and what becomes of `interp_union`'s
    per-arm rendering.
  - **Effect members compete with fns** (user, 2026-10-27): an effect in a
    fn's effect list is a namespace of additional functions in scope, so its
    members join the call's candidate set and are ranked with fns
    ([fn-overload-rank]). **This changes the language:** today
    [effect-member-overload] says "an effect member takes the name before
    any fn does" (it shadows, it does not compete), and a bare member call
    across effects resolves through the *one available* effect, two
    available being an error naming [effect-at]. Under the new rule two
    same-named members of available effects are ranked by argument types
    like any overloads, and ambiguity is the ordinary overload ambiguity.
    Unchanged: fn-typed locals, parameters and implicits still shadow (they
    *are* the function the caller chose). The census must list what changes:
    programs where a fn now beats a member, or a member-vs-fn tie becomes an
    ambiguity error.
- **P5. Where the round-trip check runs** (B4, D4). **Decided (user,
  2026-10-27): (b), the corpus and the examples only.** The round-trip check: take a phase's dump
  printed as Salvo, with every choice the compiler made written out (callee
  as `name@module`, implicit arguments passed, type arguments given), feed
  it back to the compiler as a program, check it, and require the same
  choices. A rule that fires differently on the implicit spelling than on
  the explicit one fails it. Example (found 2026-10-14): with a module's
  catch-all `fn to_str<T>(x: T) -> Str`, `"${List(Foo{n:1})}"` printed `?`
  because implicit selection fell back past std's list overload; the dump
  would show the pick, the re-check of the explicit form would pick
  differently, and the check would report the mismatch. It also catches a
  dumper that prints wrong. Cost: every program it covers is checked
  twice. Options: (a) every program the suite checks — strongest, adds to
  ROADMAP §0h's warm-run excess; (b) the corpus and examples only;
  (c) (b) plus a debug flag for any project.
- **P6. Representation** (§II.8; C1, D1). **Decided (user, 2026-10-27):
  decide at D1.** B and C are built on tables keyed by A3's node keys.
- **P7. Dump command surface** (C2, D4). **Decided (user, 2026-10-27): one
  `salvo dump --phase <name>`**, absorbing `salvo facts`, `salvo ir` and
  `salvo compile --emit-ast` (the parse phase's AST; its `=MODULE` form,
  which prints one module including std's, becomes a module filter on the
  dump). No compatibility shims (AGENTS.md). Phase names (user, 2026-10-27): `parse-ast`, `resolve`, `type-check`,
  `generate-ir` (§II.3's table); the language docs and `salvo dump --help`
  explain what each phase's output contains. Each old command goes in the
  piece that first adds its phase to `dump`.
- **P8. A5's placement.** **Decided (user, 2026-10-27): only in C3.**
  Comptime's signature/body split is built once, on the resolved form. A5
  is dropped from Stage A.
- **P9. DECISION (open, B3.1, 2026-10-28): does an effect member fill a
  binder position?** R10 D3 puts members at every implicit position; a
  binder's fill is an identity a type carries [cmp-binder], and only a
  named, top-level, capture-free fn is one [cmp-carry], while a member's
  value is a closure over its handler. (a) **fns only at a binder** — built,
  the safe reading: no program newly refused, one exception to the one
  rule; (b) rank members there too and refuse a member that wins, as a
  lambda written there is refused. Recommendation (a). ROADMAP 0af.

## II.11. Rounds

### 2026-10-27, first

- **Direction:** per-phase lowering for structural unity; representation
  secondary; origins kept through every lowering (user).
- **`by` at a call:** not supported directly; a named fn in the same module
  is the workaround (user). Explicit-root form recorded, not chosen.
- **Plan drafted** (§II.9) with decisions P1–P8. Recommended, not yet
  decided: keep resolve and type separate (§II.3.1).

### 2026-10-27, second

- P1 (written parameter types), P2 (`NodeId` key, span kept as origin), P3
  (calls are the reference), P6 (at D1), P7 (one `dump` command, including
  the AST), P8 (comptime split in C3 only): decided as in §II.10.
- P4: case by case; interpolation and operators wait on the user's
  operator-overloading design (format-fn lowering the candidate); effect
  members compete with fns (a change to [effect-member-overload]).
- Open: P5 (explained in §II.10), P7's phase names, §II.3.1.

### 2026-10-27, third

- P5: (b), corpus and examples. P7's phase names: `parse-ast`, `resolve`,
  `type-check`, `generate-ir`, documented in the language docs.
- The user's operator-overloading design (OPERATOR_OVERLOAD.md, untracked)
  shared for P4's interpolation and operator rows; under discussion.

### 2026-10-27, fourth

- Operators (OPERATOR_OVERLOAD.md O1, user): **an operator is the call it
  names**, resolved by name exactly like that call (option B); no fixed
  operator fn, no `operator` keyword, no generic exclusion. So B3's
  operator piece lowers `a < b` to `cmp(a, b) < 0` and `a == b` to
  `eq(a, b)` as ordinary calls, and the interpolation piece lowers to a
  std format fn over `to_str(part)` calls. Still open inside the
  interpolation row: the format fn's shape.

### 2026-10-27, fifth

- Operators, revised (OPERATOR_OVERLOAD.md O1, user): B **plus D**: every
  fn matching an operator pattern on `T` (or a qualified `T`) is declared
  in `T`'s module, so a concrete type has one candidate per operator and
  the name-based call is that operator. Lowering unchanged from the fourth
  round; adds a per-declaration placement check (an interface fact, in
  `resolve`). D's details D-1 to D-7 open in OPERATOR_OVERLOAD.md.

### 2026-10-27, sixth

- D's details decided (OPERATOR_OVERLOAD.md): the class is **unique
  function shapes** (operators its first members); placement is the
  type's declaring *file*; "on `T`" is the first explicit parameter's base
  type; a shape is name + explicit arity, plus the return type for `eq`
  (`Bool`), `cmp` (`Int`) and `next` (`Emitted T | Finished`); `fns` group
  members and comptime templates exempt, effect members covered;
  structural types get a std home, a bare `T` none. The check is a
  `resolve`-phase interface fact.

## II.12. Work log

Resumable state for the build of §II.9 (the user's instruction, 2026-10-27:
fold Part II in and start on it; as for Part I, one commit per piece, keep
this log current so the work can resume after a context reset). Newest last.

- **Folded (2026-10-27).** PIPELINE2.md's decisions and sequence moved here
  unchanged except for section numbers (`§N` → `§II.N`, Part I's sections
  plain `§N`); PIPELINE2.md deleted. Started: Stage A, piece A1 (stable
  declaration ids), with B1 (the read-only call census) beside it.
- **A1, first part: done** (committed "Identify every fn by module, name and
  signature"). Built: `salvo_core::DeclId` (`decl_id.rs`, [decl-id]): module
  path, kind, name path, signature (written parameter types with type
  parameters positional, plus the return type, as [fn-overload-duplicate]
  compares them) and ordinal; interned (a pointer), ordered by content,
  displayed `size(Point) -> Int@main`. `DeclIndex` (in `Resolution` and
  `Checked`, an `Arc`) maps ids to (file, item, member) and back, memoized
  per module allocation (`MEMO`, by `Weak`), so the comptime resolution and
  the program's, and the language server's analyses, compute a module's ids
  once. `FnKey` and the IR's `DeclId { module, item, sub }` are gone; every
  fn-keyed table and every IR reference, generated declaration and static
  uses the id. The differential (`SALVO_XP_A1`, removed) over 2,900 programs:
  every table, diagnostic and IR dump identical up to the key change.
  **Deviations:** (1) the signature includes the return type, since
  [fn-overload-return] lets overloads differ in it alone (P1 said the
  parameter types); (2) the ordinal is unreachable for a module's top-level
  fns in an accepted program ([fn-overload-duplicate] refuses the second),
  reached only past that error; (3) a handler member in the IR still carries
  its handler's id, and the checker's member facts stay keyed by name span;
  (4) the type and effect keys (`Name§module`, `Symbols::decl_keys`,
  `typekey`) are not yet the id: the second part.
  **For A2/A3:** a stamped fn's id is its declared signature's, so a stamp
  and its declaration agree; route stubs and actor glue have
  `DeclKind::Generated` ids by generated name, the anchor an `Origin` can
  point at; `DeclIndex::loc` is how a node key `(DeclId, NodeId)` finds its
  file for rendering.
- **A1: done** (committed "Identify every fn by module, name and signature"
  and "Key every type by its module"). The second part: a type's key, the
  string `Ty::Named` carries, is the spelling of its id — std's first
  declaration of a name is the name, every other declaration
  `Name§module`, whatever else the program declares — so no key moves when
  a file is added; `Symbols::key_of_id` maps a type's id to its key. The 45
  sites that assumed key == written name (Part I §8 step 5 piece 4 counted
  58) are fixed at their cause: diagnostics render keys as written names
  where they are made (`FileDiagnostic`, `typekey::shown_text`); the
  deadlock graph, the platform ABI closure and `platform_effects` read
  written names through the file's scope (`Symbols::ref_key`); derived
  names (routes, factories) come from the written name. The differential
  (new keys mapped back to the old ones) found six latent plain-name bugs,
  each fixed (COMPLETED.md "Every type is keyed by its module"), among them
  a std protocol hash that depended on a user type named like its type
  parameter. **Deviations:** (1) `Symbols::decl_keys`, `type_ref_keys`,
  `visible_keys` and the key-keyed tables stay: a key is now a pure
  function of the id, so keying by it is keying by the id; carrying a
  `DeclId` in `Ty::Named` itself (347 sites, both emitters' name rendering)
  was not done — a later piece if a stage needs a type's id without
  `key_of_id`'s map; (2) std keeps first-declarer among std's own files (two
  std modules declare `Fault`, two `Found`): stable for every user program,
  re-keyable only by editing std itself; (3) diagnostics render every key
  plain, clash or not, as they did; emitters keep deciding clashes from
  `Symbols::clashing`.
  **What A2 needs from A1:** the stamped fns, route stubs and actor glue
  have ids to hang an `Origin` on (`DeclKind::Generated` by generated
  name); `DeclIndex::loc` maps a `(DeclId, NodeId)` key back to its file;
  handler members still carry their handler's id in the IR and are keyed by
  name span in the checker — A3 should key them by their `HandlerMember`
  ids (`DeclIndex::member`), which exist already.

- **B1: done (2026-10-27), read-only; the census is §II.13.** 27 call-like
  forms, 25 divergences (D1–D25), 28 probes run on both backends. Five are
  wrong today whatever P3 decides: D11 and D13 (silent Kotlin miscompiles: a
  bare top-level call shadowed by a same-named parameter or local, in a
  `for` driver and in a dot or `@` call), D13's wrong "`@` not needed"
  warning, D16 and D19 (checker-clean programs both target compilers
  refuse), D20 (a claim dropped through a fn-typed local). Ten need a user
  decision beyond P3 (§II.13 "Decisions needed beyond P3": D3, D4, D5, D6,
  D7, D10, D13, D14, D18, D21; D1 optional). Next: the five bugs, then A2;
  B2's decisions are the user's.
- **B1 bugs: done** (committed "Qualify every declared callee in Kotlin and check fn-value calls"). D11 and D13: the
  Kotlin emitter qualifies every declared call by its package
  ([kt-decl-call-qualified]; a handler member calling the top-level fn of
  its name recursed too, the same bug); D13's warning withheld where a
  local, a `send fn` or an effect member takes the bare name; D16: a lambda
  in a colliding call takes the candidates' common type at its position or
  is refused (P4's shared lead pool not built); the fn-only overload half of
  D16's probe had the same bug (the census misread it) and gets the same
  rule; D19: a computed callee checked like a local call, and the IR passes
  its effects; D20: claims checked through fn values, dependent ones tied to
  the call's arguments. No B2 decision was taken: dot calls still see no
  locals. COMPLETED.md has the record. Next: A2.

- **B2: decided (user, 2026-10-27; §3 R10).** Every census divergence
  follows the call, with these qualifications: D4 a general union `to_str`
  (deferred if not easy), D7 kept as a pre-call lowering for now, D14's
  knock-ons asked before B3 piece 7 is built. **Instruction (user):** proceed
  through the steps without stopping until a decision is needed; commit per
  AGENTS.md; keep this log current. **Order from here:** A2 (origins), A3
  (per-declaration node keys), A4 (facts per declaration), then B3's pieces
  in §II.13's "Order for B3" (implicits, interpolation, operators + `for`,
  dot calls, effect and qualifier members, fn values, sends), each applying
  R10.

- **A2: done** (committed "Give every copied node its origin"). Built
  ([origin]; COMPLETED.md "Every copied node keeps its origin"): `Span` is
  `{ start, end, expn }` — a copied node keeps the range of the code it was
  copied from and carries an expansion id, 0 for written code — and the
  stamped fn's `Stamp` holds the records (`ExpansionRecord { id, parent,
  kind }`: `Template`, `Copy { target, label, written }`, `Member`, `Made`),
  replacing `StampRegion`. Comptime retags (one id per copy, per file, in the
  order the old regions were allocated) instead of remapping into virtual
  space. `origin::render` is the one renderer (the checker's
  `render_origin`, the deduction pass, and now the IR analyses), and tags
  the diagnostic with the stamped fn (`FileDiagnostic::decl`), which is what
  `ir::gate` attributes it by (`DeclIndex::loc`), no prefix match.
  Positions compare within one expansion only (`Span::order_key`,
  `Span::holds`); the IR's fate tables are keyed by `Span`. The differential
  (`SALVO_XP_A2`, removed; 3,839 outputs: rendered diagnostics and IR dumps
  over every program the suite checks) found one difference, a fix: an
  unresolved name in an in-place copy, which bypassed the redirect and
  printed past the end of the file. Emitted code unchanged; timings flat
  except parse ~+1ms on `examples/cluster` (12-byte spans).
  **Deviations:** (1) `Origin { span, via }` is the span itself, `via` its
  `expn`: every node has exactly one span, and until A3 the link must be in
  the key; (2) expansion ids are per file, not per declaration, because two
  stamps of one template in one file share every position and `(file,
  Span)` keys must stay apart — A3 can renumber per declaration once keys
  carry the `DeclId`; (3) route stubs and actor glue needed nothing (the IR
  derives them with written spans and `Generated` ids); the `iter fn`
  desugaring's one-byte spans (`desugar::Spans`) and the `0..0` placeholders
  stay, since they only keep keys apart and A3 removes the need; (4) a
  location computed from a copied node outside a diagnostic (a trap's
  `module:line:col`, "moved at line N", `salvo facts`, an LSP range) is
  still the file's end, so no output changed (ROADMAP 0ac).
  **What A3 needs from A2:** the key becomes `(DeclId, NodeId)` and the
  `Span` (with `expn`) is the origin data: diagnostics render with
  `origin::render(stamp, span, …)` where the declaration's `Stamp` is at hand
  (`DeclIndex::loc` → the `FnDecl`); the language server matches written
  spans only (`Span::holds`). With node ids, `expn` no longer has to be
  unique per file (only within a declaration), the desugaring's one-byte
  spans can become the declaration's real span, and `Lower::fn_body`'s
  "written range plus the fn's run of expansion ids" becomes "the
  declaration's rows". Handler members are still keyed by name span in the
  checker (A1's note stands).

- **A3: done** (committed "Key every fact by its declaration and node").
  Built ([node-id]; COMPLETED.md "Every fact is keyed by its declaration and
  node"): `check::Key` is `(DeclId, NodeId)` with the node's file and span
  carried as origin data outside its equality. Node ids are numbered on the
  tree after its last rewrite (`salvo_syntax::node_id::number`: the parse,
  `expand_file`, each comptime-stamped file, the `salvo test` harness) and
  ride on the span (`Span::node`, the ordinal in the file; `Span` equality
  ignores it); `Module::node_ranges` gives each declaration's run, and
  `DeclIndex::key(file, span)` the key — one field read by checker, IR
  builder, deduction walk and readers alike, so no recomputation can drift.
  A file's own nodes are keyed by a new `DeclKind::File` id; handler members
  by their member ids (and their IR `FnDecl`s carry them); variable ids per
  declaration; `Lower::fn_body` takes the declaration's rows; the `iter fn`
  desugaring's nodes take their declaration's span; ROADMAP 0ac fixed
  (`origin::landing`) and deadlock reports rendered through the stamp. The
  differential (`SALVO_XP_A3`, removed; 8,088 outputs) found only latent
  collisions of the old keys, now separated (5,965 `fn_refs` at `by` sites,
  309 `def_refs` at test names, 30 `expr_ty` of the desugaring); IR, diagnostics
  and emitted code unchanged except `examples/cli`'s trap locations (0ac).
  **Deviations:** (1) the id rides on the span rather than in a field of each
  node kind: every node has one span, every keying path passes it, and
  `visit_mut` reaches them all; (2) expansion ids stay per file: a file's id
  finds its stamp from a span alone (`origin::stamp_of`), which the location
  rendering needs, and two stamps of one template in one file would share
  every span-keyed position the per-file walk maps use (`fate_log`, the IR's
  fate tables); (3) header expressions still ride the body walk (nothing
  needed them out); (4) the two made-up spans that were keys became
  synthetic keys (`Key::synthetic`), not nodes of the tree; (5) found, not
  fixed: ROADMAP 0ad (a stamp's copy target in another file).
  **What A4 needs:** a key is stable under edits above its declaration, but
  what a row *says* is not: the span and file in a `Key`, and the spans in
  row values (`DefSite`, `LocalDecl`, `handler_deps`, and the binding
  "stamps" — `BindFact::stamp`, `FlowLink::stamp`, `FateRead::bind_span` —
  which the IR flow pass and `Decision` use as binding identities) are
  positions; a declaration's cached rows reused after code above it moved
  must have them re-derived from the current tree (by node id: a
  per-declaration node → span table) or made relative. The check cache still
  partitions by file (`split_files` by `Key::file`); partitioning by
  `Key::decl` is the change. Expansion ids per file make a stamp's spans
  depend on the stamps before it in its file. The checker's per-walk maps
  keyed by `Span` (`fate_log`, `fate_meta`) are per file walk.

- **A4: done** (committed "Store the checker's facts by declaration").
  Built ([decl-rows]; COMPLETED.md "The checker's facts are stored by
  declaration"): `partition.rs`, one exhaustive router over `Checked` into a
  sink — `Checked::split_decls` (each declaration's part, plus the
  program-wide indexes) and `Checked::split_files` (the check cache's, now
  by `DeclIndex::loc` of each row's declaration, not `Key::file`);
  `insert_rows` puts either back; tables not keyed by `Key` are placed by
  their declaration id, the handler's or effect's symbol key
  (`partition::Owners`), their node's span, or the file's own id. `salvo
  facts` prints by declaration, under each id (snapshot reviewed: headers,
  and the bindings' and parameters' rows now shown). The binding stamps
  (`BindFact::stamp`/`origins`, `FlowLink::stamp`, `DerivedUse::own`,
  `VirtualCandidate::stamp`, `FateRead::bind`, and the IR flow pass's
  copies and sets) are `Key`s; `flow::decide`, `bind_links` and
  `drop_owned_fate_reads` no longer key spans. The differential
  (`SALVO_XP_A4`, removed; 1,919 IR dumps with their `Decision`s and
  analyses' diagnostics, both runs cold, base a worktree): identical, one
  program differing only by the worktree's tmp path in its text. Emitted
  code and diagnostics unchanged.
  **The cache level: per file stays** (ROADMAP 0ae). Measured (debug,
  `tools/lsp-timing.py`, medians of 3×4 edits; per-item walk times from
  temporary instrumentation): the walk per-declaration reuse could save is
  `examples/cluster` ≤ 4.4ms of bodies (28 items; the edited `main` ~1ms)
  of a ~43ms edit, `modules/aws` ~1ms of ~43ms, and a 1,654-line one-file
  project (`lib/std/args.sv` as `main.sv`) ≤ 16.2ms (74 items) of ~76ms,
  less the reused rows' insert (~1.4ms there) and their span moves. The gate
  cases gain less than their ±10ms run spread; what it needs (every span in
  a reused row re-derived, the file walk split per declaration with
  per-declaration step logs) is listed in 0ae. So the node → span
  re-derivation was not built: under per-file reuse a cached file's spans
  point into the very module it is checked from. On the large file the
  whole-program IR (build 11, analyze 13.5ms) costs more per edit than the
  walk reuse would save. Timings flat: cluster 52.6 → 51.0, aws 42.3 → 45.5
  (`cache record` 0.6 → 0.8ms, the per-row declaration lookup, memoized),
  the large file 89.2 → 87.3 (interleaved, loaded machine); `salvo analyze`
  cluster check ~180–194ms both.
  **Deviations:** (1) resolution's indexes (`type_ref_keys`,
  `type_ref_span_keys`, `visible_keys`, `visible_handler_keys`,
  `platform_closure_all`, `refinements`) are program-wide, not assigned;
  (2) `carried_identities` go to the file's own id (first writer per file,
  keyed by what is written); (3) a diagnostic or a row at a covering span
  no node holds goes to its file's own id.
- **Stage A complete** (A5 dropped, P8). **What B3 needs from it:** a core
  call a lowering makes at a surface node is keyed by that node's key, or
  by `Key::synthetic(n)` of it when the form makes several calls at one
  node (`Key::INTERP_TEXT` is the precedent; `n` < 64); a lowering that
  rewrites the tree must run before the file is numbered
  (`salvo_syntax::node_id::number`, called after the last rewrite in
  `expand_file`) or renumber after it, and a lowered node's span is the
  surface form's (its origin), with an `ExpansionRecord` where an error must
  say where it came from ("in this interpolation", A2); a table B3 deletes
  goes from `partition.rs`'s router and `insert_rows` (and from
  `check_cache::StepRows` if it is kept in order); `salvo facts`, now per
  declaration, is the per-declaration dump a B3 differential can compare.

- **B3.1: done** (committed "Select every implicit argument as a call
  selects"). Built ([implicit-infer], [cmp-carry], [effect-member-overload],
  [implicit-same-name]; COMPLETED.md "Every implicit argument is selected as
  a call selects"): **D22** inference selects (`select_fn_value` at the
  implicit's pattern, the callee's open variables read off each candidate,
  `Checker::infer_open`) and reads the variables off the winner
  (`infer_read_back`); a pattern parameter still unknown teaches only
  through a single fitting candidate (`strict_select`), and the
  [implicit-fit] probe inside it does not infer again. **D23**
  `written_declaration` has no rung fallback: an ambiguity is an error
  naming `by_len@m`, once per written type (`reported_identities`). **D3,
  the implicit half**: the available effects' members are candidates
  (`ValueCand::Member`, one per member and instance, ranked with the fns),
  filled as `ImplicitArg::Member`, which the IR builder lowers to a closure
  over the instance (`Lower::member_value`: `Lambda` → `MemberCall` through
  the captured instance local); no fn values of members existed, and neither
  backend needed a change. `m@E` is a value at a fn-typed position
  (`Checked::member_values`, new in `partition.rs`), the ambiguity's remedy.
  Not at a binder (P9, open). **D21** own implicits of one name ranked like
  overloads (`pick_own_implicit`) at a call through them, a forwarded fill,
  a forwarded binder, an interpolation's own `to_str` and a comparison's own
  `cmp`/`eq`. No table went (the implicit tables are the core call's); the
  matcher, the rung fallback and four first-fit lookups did. The
  differential (`SALVO_XP_B31`, removed; 3,135 outputs per side: rendered
  diagnostics and `salvo facts` per declaration of every checked program,
  every IR build's dump with std's modules; base a worktree of 957a2061,
  both cold): no behaviour changed in any program the suite checks; 4
  diagnostics reworded (two [implicit-same-name] ties, two `m@E` refusals).
  Emitted code unchanged. Timings flat (lsp-timing medians cluster ~30–32ms,
  aws ~34–35ms both; `analyze` cluster check ~190ms both). Tests 2,122.
  **Deviations:** (1) the members also rank across two available effects at
  an implicit, where a bare call still refuses (D15, piece 5); (2) a bare
  member name as a value (`apply(to_str, p)`) waits for piece 6, and members
  at interpolation and operators for pieces 2–3 (their `resolve_as_call`
  path calls `resolve_implicit_fn_at`, fns only); (3) D21's ranking also
  covers the interpolation's and the comparison's own-implicit steps, which
  pieces 2–3 replace.
  **What B3.2 needs from it:** `select_fn_value` takes `ValueCand`s, so a
  part's `to_str` can rank members with fns by passing
  `implicit_candidates(name, true)`; the IR's `member_value` is the
  closure for a member-valued position; `pick_own_implicit` is the
  own-implicit step every form should share.

## II.13. The call census (B1, 2026-10-27)

Kept here, not in `tmp/`, because B2 and B3 work from it. The probe programs
it names were run from `tmp/b1/probes/` (gitignored; each section states its
probe's program and output, so they can be rebuilt). Line numbers are
`check.rs` at `c33b86d3`, before A1; search by function name.

PIPELINE.md §II.9 B1. For each call-like table in `struct Checked`, and each
other place the checker picks a callee, this records which surface form
writes it, which code selects the callee and tests the fit, how that differs
from the reference (a plain call `f(x)`: `resolve_named_call` →
`select_overload`, with `fn_value_fit`/`fn_value_at` at fn-typed positions),
and what the form should lower to under P3 (call resolution is the
reference) and O1 (an operator is the call it names). The probes are in
`tmp/b1/probes/<name>/main.sv`. Each probe was run with `salvo
analyze` and with `salvo run` on the Rust and on the Kotlin backend, one at a
time so the outputs are labelled (`--backend '*'` runs both but does not say
which output is which). The outputs are in `<name>/out.txt` and gathered in
`tmp/b1/all-probes.txt`. Runner: `tmp/b1/run.sh [probe…]`.

The binary was built from a `git archive` of HEAD `c33b86d3` in
`tmp/b1/head`, with `CARGO_TARGET_DIR=tmp/b1_target_head`, so the other
agent's uncommitted A1 edits are not in it. File:line references are to
`crates/salvo-core/src/check.rs` at HEAD; that file is being edited in the
working tree, so search by function name if a line has moved.
The fixed divergences from b6436d24 (implicits resolved the way calls are)
and 550669ad (a fn passed by name chosen the way a call chooses) are not
repeated here.

**Totals:** 27 forms and 25 divergences (D1–D25), with 28 probe programs.
D1–D23 each have a probe of their own. D24 and D25 are read from the code,
next to the nearest probe (`addr-dot-fn`, and the `member-vs-fn` baseline).
Five of the divergences are wrong today whatever P3 decides:

- two silent Kotlin miscompiles (D11, D13), plus a wrong warning (D13);
- two programs the checker accepts and rustc and kotlinc then refuse (D16,
  D19);
- a claim dropped silently at run time (D20).

Ten need a decision from you beyond P3, and D1 optionally; they are listed
after the divergences.

### The reference

A plain call `f(args)` (`check_call` 30161 → `resolve_named_call` 30974):

1. **Shadowing, before any candidate set.** A fn-typed local or parameter
   named `f` takes the call (30432, `local_calls`). An enclosing implicit
   named `f` takes it when the arguments fit it, and otherwise the call falls
   through to the declared fns (30406, [implicit-resolve-body]). Two
   implicits of one name are told apart by fit (30357, `local_call_names`).
   Inside a mixed handler, a handler's own `send fn` comes next (`check_servant_send`).
2. **Effect members** (31004). An available effect's members, plus the
   members `effect_members_here` sees, take the name. Two owning effects
   that are both available is an error naming `@Effect`. Where the name is
   also a fn, the chosen effect's members and the fns are ranked as one set
   (`route_member_or_fn` 8625, `fitting_members` 32899, `fitting_fns`
   8810). With no instance available, the call goes to the fns.
3. **Fn overloads.** `overloads_of` (8104) applies renames. A lead
   candidate gives each argument its expected type, with type variables
   bound progressively. The fit is unify plus `arg_fits_param` (8560) under
   the callee's contract, together with dependent claims, literal adoption,
   numeric adoption and the variadic and default shapes. Then
   [implicit-fit]'s shape drop, then `select_overload` (8174): the `@`
   selector, dedupe by declaration, the template fallback, `most_specific`
   over the declared patterns, and `pick_by_return` within a class of
   identical parameter types. No single winner is an error. The winner's
   implicits are then filled (`fill_implicit_list`).

At a fn-typed position (an implicit, a fn passed by name) the same rule runs
with the wanted fn type in place of the argument list (`select_fn_value`
7529, fitting with `fn_value_fit` 6594: contravariant in parameters,
covariant in the result, contract included).

### The census

| # | form | table(s) | selection / fit code | lowers to | divergences |
|---|---|---|---|---|---|
| 1 | `f(x)` | `call_fn`, `fn_refs`, `call_type_args`, `implicit_args`, `call_effects` | `resolve_named_call` 30974 → `select_overload` 8174 | itself: `f@m(x, imp = …)` | none: this is the reference |
| 2 | `f@mod(x)`, `f@Type(x)` | as 1 | `check_call` 30324 (`Scoped`) / 30255 (`EffectScoped` naming a type) → `select_overload` step 1 | `f@m(x)` | D13 (a wrong "`@` not needed" warning, and a Kotlin miscompile) |
| 3 | `A.B.f(x)` (namespace) | as 1 | `check_call` 30202 → `resolve_named_call` on the flat name | `f@A.B(x)` | none |
| 4 | `x.f(a)` | `dot_calls` (+ 1's tables or the member tables) | `check_call` 30221: qualifier call, namespace, addr receiver, then `resolve_named_call(f, [x, a…])` | `f(x, a)` | D12, D13, D14 |
| 5 | `x?.f(a)` | `dot_calls`, `safe_navs` | `check_safe_nav` 28713 → `resolve_named_call` under the narrowing | `x is T ? f(x, a) : None` | as 4 (D12, D13); it does no addr check |
| 6 | bare effect member `m(x)` | `member_calls`, `effect_member_calls`, `effect_calls`, `implicit_args` | `resolve_named_call` step 1 → `route_member_or_fn` / `pick_effect_member` 32951 / `check_effect_call` 33053 | `m@E(x)` plus the handler argument (an effect parameter after erasure) | D15, D16, D17 |
| 7 | `m@E(x)`, `x.m@E()` | as 6 | `check_call` 30255 → `check_effect_call` | `m@E(x)` | D17; otherwise as designed (members only) |
| 8 | call through a fn-typed local or parameter `f(x)` | `local_calls`, `fn_value_calls`, `derived_calls`, `lending_calls` | `check_call` 30432: `arg_fits_fn_value` 30967 per argument, then `apply_fn_value_contract` 13430 | `f(x)` with `f` a local | D20 |
| 9 | call through the enclosing implicit `cmp(a, b)` | `local_calls`, `local_call_names` | `check_call` 30357–30431 | `cmp(a, b)` with `cmp` the parameter | D21 |
| 10 | computed callee `e(x)` (`adder()(1)`, `fs[0](x)`) | (none of its own) | `check_call` 30630: `check_expr(a, params.get(i))` only | `let t = e; t(x)` | D19 |
| 11 | fn passed by name `apply(f)`, `f@m` | `fn_refs`, `fn_value_implicits`, `lambda_effects` | `fn_value_by_name` 7758 → `choose_by_name` 7938 → `select_fn_value` | the value `f@m` with its implicits filled | fixed by 550669ad; none left |
| 12 | implicit argument `?name` at a call | `implicit_args` (`implicit_params`, and `implicit_members` for members) | `fill_one_implicit` 7062 → `resolve_implicit_fn_at` → `select_fn_value` | `name = name@m` written out | D3 (effect members); fixed by b6436d24 otherwise |
| 13 | type inference from an implicit ([implicit-infer]) | `call_type_args` | `extend_subst_from_implicits` 9044: its own matcher | (inference inside 1) | D22 |
| 14 | identity written in a type `SortedSet<Str>(by_len)` | `carried_identities`, `fn_refs` | `written_fn_id` → `written_declaration` 19588, then `resolve_identity` 14388 | `SortedSet<Str>(by_len@m)` | D23 |
| 15 | keyed slot filled by name (`Set<Person>` uses Person's `hash`) | (read: `keyed_canonically` 4352) | `resolve_implicit_fn_at` | as 12 | none found |
| 16 | interpolation part `"${x}"` | `interp_to_str`, `interp_implicit`, `implicit_args` (at the zero-width span after the part) | `check_interpolable` 12467 → `interp_native` 12326 / `interp_text_form` 12381 (own implicit by shape, then `resolve_implicit_fn_at("to_str")` at `(T) -> Str`) | a std format fn over `to_str(x)` parts (O1; the format fn's shape is open) | D1, D2, D3 |
| 17 | interpolated union `"${u}"` | `interp_union` | `check_interpolable`: one `interp_text_form` per arm | open (P4) | D4 |
| 18 | `a < b`, `a <= b`, `a == b`, `a != b` | `comparisons`, `promotions` | `check_comparison_operands` 13897 | `cmp(a, b) < 0`, `eq(a, b)`, `!eq(a, b)` (O1) | D1, D2, D5, D6 |
| 19 | `x == v` with `x` a union and `v` matching one arm | `arm_compares` | `try_arm_compare` 13820 | `x is A && eq(x, v)` | D7 (a rule to keep) |
| 20 | `a + b`, `- * /` | `promotions` | `check_arith` 14148: numeric operands only; nothing is resolved | `plus(a, b)` … (O1 adds these names; not a call today) | none yet: O1 adds the feature |
| 21 | `for x in it` (a pass), `for x in c` (a source) | `for_drivers` | `iter_elem_ty` 26708 → `pass_elem_ty` / `find_next_driver` 26184, `generic_pass_elem_ty` 26454, `iter_pass_for` 26370 | `loop { when next(it) … }`, with `it = iter(c)` (O1) | D8, D9, D10, D11 |
| 22 | `for x in next(p)` (a step call) | `for_drivers` (`step_call`), `call_fn` | `step_call_elem_ty` 26602: the call's own resolution | itself | none |
| 23 | `a.m(x)` with `a: eff[E]` (a send) | `addr_calls`, `effect_member_calls` | `addr_receiver` 15645 (the receiver must be a place) → `check_addr_call` 15689: members picked by arity; type arguments and named arguments ignored | `m@E(a, x)` as a send | D14, D24 |
| 24 | `m@self(x)`, and a servant's bare call to a sibling `send fn` | `self_sends`, `facade_sends` | `check_self_send` 15531, `check_servant_send` 16491: the first member of that name | `m@self(x)` | D24 |
| 25 | `replyto k(c…)`, `k` a free `send fn` | `replyto_tasks`, `fn_refs`, `task_mint_effects` | `check_replyto` 17003 → `free_send_target` 16761 (scope rungs) | a mint of `k@m` with captures `c…` | D18 |
| 26 | `replyto m(c…)`, `m` a member of the own handler | `replyto_members` | `check_replyto`: the first member of that name | a mint of `m@self` | D24 |
| 27 | `Q.f(x)` (a qualifier's own fn), and `x is Q` (→ `Q.qualifies(x, …)`) | `qual_member_calls`, `predicate_tests` | `check_qual_member_call` 21806 (the first fn of that name; implicits refused), `check_predicate_quals` 27728 | `f@Q(x, slots…)` | none found (qualifier fns are not overloaded) |

Places that pick a function without being a call (inputs to inference or to
diagnostics): `some_fn_accepts` 7040 (the [implicit-fit] shape probe, by
design); `has_member_for` 14245 (whether a type is hashable or orderable:
the raw scope table and the first parameter's base name, not a selection);
`infer_waitfor_ty` 17237 (scans declared `Reply<T>` positions, deliberately
not overload resolution); and the `: Yield`/`: Iter` obligation check
`check_obligation_list` 4971, which reads the raw `scope.fns` table, so a
renamed-away `next` still satisfies it (part of D9). Effect-instance
choice at a member call ([effect-disambiguation], `check_effect_call`)
chooses a handler, not a callee; it is the implicit-like effect argument
and outside P3.

### Divergences

Each section gives what differs, the probe and its output, and what
happens under P3: which programs would newly be refused, newly be accepted,
or behave differently. "Decision" says whether it needs a call from you
beyond P3.

#### D1. The native fast path runs before the enclosing implicit (interpolation, `==`, `<`)

`check_interpolable` returns at once for a scalar or `Str` (`interp_native`)
without looking for an enclosing `?to_str`. `check_comparison_operands`
returns before its implicit step for numerics (every operator) and for
`==`/`!=` on the intrinsic types. A call `to_str(n)` or `eq(a, b)` in the
same body goes through the implicit [implicit-resolve-body].

- `interp-native-implicit`: inside `show(n: Int, ?to_str: (x: Int) -> Str)`
  called with `to_str = hex`. Rust and Kotlin both print `interp: 7` and
  `call:   #7`.
- `op-eq-native-implicit`: inside `check(a: Str, b: Str, ?eq: …)` called
  with `eq = same_len`, on `"ab"` and `"cd"`. Rust and Kotlin both print
  `op:   false` and `call: true`.

**Under P3** `"${n}"` → `to_str(n)` → the implicit, and `a == b` →
`eq(a, b)` → the implicit. Programs whose fn has an implicit `to_str`, `eq`
or `cmp` covering a scalar or `Str`, and which interpolate or compare such
a value directly, would behave differently. None would newly be refused.
The backends can keep the native spelling exactly when the call resolves
to std's intrinsic. **Decision:** none beyond P3, but OPERATOR_OVERLOAD.md
O5 is wrong today. O5 says `"${n}"` inside `show(…, ?to_str: (x: Int) ->
Str)` "uses the enclosing implicit". That holds only for elements nested
inside a std container's `to_str` (the 2026-10-14 test); a direct `${n}`
does not use it. You may want the native fast path kept as an exception.

#### D2. The native fast path skips resolution altogether

The same early returns mean a module's own `eq(Str, Str)` or `to_str(Int)`
is never consulted by `==` or `${}`. The call meets std's overload with the
same signature and is ambiguous.

- `native-overload`: the analysis reports only the two calls, as ambiguous
  between `main` and `std.core.compare` (and `std.core.basic`). The operator
  and the interpolation pass.

**Under P3** the operator and the interpolation would be ambiguous too, so
such programs would newly be refused. **Under O1-D** these declarations are
refused where they are written (a shape on a type declared elsewhere), so D2
dissolves once D's placement check exists. **Decision:** none.

#### D3. Effect members never fill interpolation, operator or implicit positions

`interp_text_form`, `check_comparison_operands` and `fill_one_implicit` all
resolve through `overloads_of`, which holds fns only. A call `to_str(p)`
reaches an available member.

- `interp-effect-member`: `effect Fmt { fn to_str(p: Point) -> Str }` with
  a handler in scope. The call `to_str(p)` is accepted; `"${p}"` is refused
  with "`Point` has no text form"; `render(p)` with `?to_str: (p: Point) ->
  Str` is refused with "no `to_str` fits".

**Under P3 + P4** the interpolation lowers to `to_str(p)`, so it would reach
the member and newly be accepted. A member beside a fn of the same
signature would become an ambiguity at `${}` where today the fn wins
silently. **Decision: yes.** P4 decided that members compete at calls. It
did not decide whether a member can fill an implicit (passed as a value,
which captures the handler) or an operator position. P3 implies yes for
interpolation and operators, since they lower to calls. For implicits it is
new: a member as a fn value carries its effect.

Fixed, the implicit half (commit "Select every implicit argument as a call selects"): an available member fills an implicit position, passed as a closure over the handler, and `m@E` is the value that names it; `render(p)` in the probe prints `<1, 2>` on both backends. Interpolation and operators wait for pieces 2–3.

#### D4. A union interpolates arm by arm; the call has no overload

- `interp-union`: `u: Point | Int`, with `to_str(Point)` declared.
  `"${u}"` is accepted. `to_str(u)` is refused: "no matching overload for
  `to_str(Point | Int)`".

**Under P3** a plain `to_str(u)` lowering would refuse every interpolated
union that has no `to_str` of its own. **Decision: yes**, and it is already
open in P4 ("what becomes of `interp_union`'s per-arm rendering"). The
candidate answer is that the format fn's part lowering becomes `when u { is
Point → to_str(u) … }`: a lowering written before the call, as D7 is.

#### D5. Comparison operands lose their qualifiers before `eq`/`cmp` is resolved

`check_comparison_operands` builds the wanted type from `lb`/`rb` with the
qualifiers stripped ("a comparison is about the data"). The call ranks the
qualified argument types.

- `op-qualified-operands`: `fn eq(a: NonEmpty List<Int>, b: NonEmpty
  List<Int>) -> Bool { true }`, with `xs = List(1, 2)` and `ys = List(3,
  4)`. Rust and Kotlin both print `op:   false` (std's `eq(List<T>)`) and
  `call: true` (the more specific `NonEmpty` overload).

**Under P3** `xs == ys` → `eq(xs, ys)` → the qualified overload, so the
behaviour changes and nothing is refused. This survives O1-D: a qualified
overload in the type's own file is legal ("several overloads of one shape
in the type's own file stay legal"). **Decision: maybe.** The stripping is
deliberate, so you may want "operators compare the data" kept as an
exception. Then the lowering would be `eq(strip(xs), strip(ys))`, which is
not a plain call.

#### D6. `==` and `<` require the same base type on both sides

- `op-mixed-types`: with `fn eq(a: Meters, b: Feet) -> Bool` declared,
  `eq(m, f)` is accepted and `m == f` is refused: "the operands of a
  comparison must be the same type".

**Under P3** `m == f` would newly be accepted. The O1 D-3 shape for `eq`
(two parameters, `Bool`) does not require one type, and D-2 puts the fn
with `Meters`. **Decision: yes.** Either drop the same-type rule (P3), or
keep it as an operator-only exception, and if kept, add it to D-3's shape so
that a mixed `eq` is not an operator at all.

#### D7. An arm comparison lowers to a test and a comparison, not to the call

- `op-arm-compare`: `x: Long? = -1L`. `x == -1L` and `y == -1L` (with `y =
  None`) are accepted. `eq(x, -1L)` is refused: "no matching overload for
  `eq(Long?, Long)`".

**Under P3** this is a lowering written before the call: `x is Long &&
eq(x, -1L)`. Nothing would change. **Decision:** confirm that [op-arm-eq]
(user decision 2026-10-12) stays as such a pre-call lowering; it is not a
selection divergence.

#### D8. `for` drives the first `next` in scope order, with no ranking or ambiguity

`find_next_driver` and `iter_pass_for` walk `scope.fns.get("next")` (and
`"iter"`) and take the first entry whose state parameter unifies.

- `for-first-next`: `next<T>(c: Mut Ctr<T>)` is declared before `next(c:
  Mut Ctr<Str>)`, which yields ×100. Rust and Kotlin both print `for:  1`,
  `for:  2` (the generic `next`) and `call: 100` (the specific one).

**Under P3** the `for` would drive the specific `next` (100, 200), so the
behaviour changes. Two same-signature `next`s from different modules would
become an ambiguity error where today the earlier one wins silently. This
survives O1-D: both overloads are in `Ctr`'s file. `iter` has the same
first-match rule (not probed separately). **Decision:** none.

#### D9. `for` ignores `rename fn`

Both the driver scan and the `: Yield` obligation check read `scope.fns`
directly, not `overloads_of`.

- `for-renamed-next`: after `rename fn advance = next(c: Mut Ctr)`, Rust
  and Kotlin both print `for:  1`, `for:  2` and `advance: 1`.
- `for-renamed-next-call`: the same program with `next(d)` is refused with
  "no matching overload for `next(Mut Ctr)`", while its `for` is accepted.

**Under P3** such a `for` would newly be refused, and so would the `: Yield`
obligation, since it is no longer satisfied by a `next`. [fn-rename] already
says a renamed overload leaves the old name "in calls, in fn values and in
implicit resolution". **Decision:** none.

#### D10. `for` needs a `: Yield` declaration that `next(c)` does not

- `for-no-yield`: `Ctr` declares no `: Yield`. `next(d)` is accepted;
  `for x in c` is refused with "`Ctr` is not iterable … declare `: Yield<self,
  Int>`".

**Under a plain P3 lowering** (`for x in c` → `next(c)` when a `next` fits,
otherwise `next(iter(c))`) the declaration would stop being needed, and
such programs would newly be accepted. **Decision: yes.** O1 lowers `for`
to `next`/`iter`, but whether the obligation stays as the gate on which of
the two forms is meant ("a type declares `: Iter` or `: Yield`, never
both") is yours. Keeping the gate is an exception to P3 that reads a
declaration, not a resolution.

#### D11. At a concrete pass type, `for` ignores the enclosing `?next`, and Kotlin does not

`generic_pass_elem_ty` consults implicits only when the subject is a type
variable. At a concrete type the declared `next` is recorded
(`PassMember::Fn`). The call `next(d)` goes through the implicit.

- `for-own-implicit`: `walk(c, d, ?next: (c: Mut Ctr) -> …)` is called with
  `next = by_tens`.
  - **Rust:** `for:  1`, `for:  2`, `call: 10`.
  - **Kotlin:** `for:  10`, `for:  20`, `call: 10`.

  The Kotlin emitter writes the driving call as a bare `next(c)`, and the
  parameter named `next` shadows the top-level fun
  (`tmp/b1/out/for-own-implicit-ktsrc/main.kt`: `val __step_2 … = next(c)`).
  **This is silently wrong code today** [backend-never-wrong], whatever P3
  decides.

**Under P3** the loop drives the implicit (10, 20) on both backends, so the
behaviour changes on Rust. **Decision:** none for P3. The Kotlin bug needs
fixing now: either qualify every top-level call the checker resolved, or
let P3's core call node do it.

Fixed (commit "Qualify every declared callee in Kotlin and check fn-value calls"): every declared call is
package-qualified on Kotlin; both backends print 1, 2 and 10.

#### D12. A dot call ignores enclosing implicits

`check_call`'s dot path goes straight to `resolve_named_call`, which never
consults `own_implicits`. Only the bare-identifier path does.

- `dot-implicit`: inside `order_dot<T>(a: T, b: T, ?cmp: …)`, `a.cmp(b)` is
  refused with "no matching overload for `cmp(T, T)`". `cmp(a, b)` in the
  same shape (`order`) is accepted.

**Under P3** `a.cmp(b)` → `cmp(a, b)` → the implicit, so it would newly be
accepted. The operator-overloading brief itself writes `a.cmp(b) < 0` as
the generic spelling. **Decision:** none.

#### D13. A dot call ignores fn-typed locals; Kotlin miscompiles it, and a wrong `@` warning

`5.show()` resolves among the declared fns even when a local `show`
shadows them at `show(5)`.

- `dot-local`, with `fn show(n: Int)` declared and `let show = (n: Int) ->
  "local ${n}"` in scope:
  - **Rust:** `call: local 5`, `dot:  decl 5`, `at:   decl 5`.
  - **Kotlin:** `call: local 5`, `dot:  local 5`, `at:   local 5`.

  The Kotlin emitter writes `show(5)` for all three, and the local wins
  (`tmp/b1/out/dot-local-ktsrc/main.kt` lines 15–17). So both `5.show()`
  and the explicit `show@main(5)` silently call the local on Kotlin.
- The checker also warns at `show@main(5)` that "this `@main` is not needed:
  `show(Int)` resolves to the same function without it". That is wrong:
  without it the call takes the local. `unqualified_winner` (8517) does not
  know about locals. [fn-overload-at] calls `@` "the way out of a shadowed
  name".

**Under P3** `5.show()` → `show(5)` → the local, so the dot call changes
behaviour on Rust. **Decision: yes.** [fn-dot] says `base.f(args)` is
`f(base, args)`, which P3 makes literal. You may prefer dot calls to name
declared functions only, so that a local `show` cannot capture `x.show()`.
Either way, the Kotlin miscompile and the warning are bugs now.

Fixed, the bugs half (commit "Qualify every declared callee in Kotlin and check fn-value calls"): Kotlin names the
declaration (`salvo.main.show(5)`), and the warning is withheld where a
local, a `send fn` or an effect member takes the bare name. The decision is
still open: the dot call keeps ignoring locals.

#### D14. A dot call on an `eff[E]` place is always a send

`addr_receiver` peeks at the receiver. If it is a place of type `eff[E]`,
the call is `check_addr_call` with E's members only, and fns are never
candidates. A receiver that is not a place (`get(addrs, 0).bump(1)`) is not
treated as a send at all.

- `addr-dot-fn`: with `fn describe(c: eff[Counter]) -> Str` declared,
  `describe(counter)` is accepted. `counter.describe()` is refused: "actor
  `Counter` serves effect `Counter`, which has no member named `describe`".

**Under P3** `counter.describe()` → `describe(counter)` would newly be
accepted, and a fn beside a send member of the same name would become an
ambiguity. **Decision: yes.** Should an addr receiver's dot call rank E's
send members together with fns that take `eff[E]` first (P3/P4), or stay
"a dot call on an addr is a send" (an exception)? Whether a non-place
receiver can be a send is the same question.

#### D15. Two available effects with the same member name are refused, not ranked (P4)

- `member-two-effects`: `Ints.put(x: Int)` and `Strs.put(x: Str)`, both
  available. `put@Ints(1)` is accepted. `put(1)` is refused: "`put` is a
  member of `Ints`, `Strs` — more than one is in scope, so the call must
  pick its effect".

**Under P4 (decided)** the call ranks by argument types and `put(1)` →
`Ints.put`, so it would newly be accepted. A tie stays an ambiguity, now
the ordinary overload one. **Decision:** none; this is the change P4 already
names.

#### D16. A call where member and fn collide types its arguments without expected types

`route_member_or_fn` types every argument with no expected type, as its
recorded cut says. A bare lambda's parameter is then `Unknown`, and nothing
reports it.

- `member-collide-lambda`: an available member `apply(f: (x: Int) -> Int)`
  beside `fn apply(f: (x: Str) -> Str)`, called as `apply(x -> x + 1)`. The
  analysis is clean. Rust: "a value of type `?` reached rust code
  generation". kotlinc: "unresolved reference 'plus'". The fn-only overload
  set `twice(x -> x + 1)` in the same program types the lambda through the
  lead candidate.

**Under P3 + P4** members and fns form one candidate set with one lead
pool, so the lambda would be typed from the lead candidate and the call checked; it would newly be accepted, or refused with an ordinary diagnostic. **Today
it is a [backend-never-wrong] / "one mistake, one diagnostic" gap**: a
checker-clean program fails in both target compilers. **Decision:** none.

Fixed (commit "Qualify every declared callee in Kotlin and check fn-value calls"): a lambda with an unwritten
parameter type takes the type every candidate declares at its position when
they agree, and is refused otherwise ("cannot type this lambda … write the
lambda's parameter types"). **Correction:** the probe's `twice(x -> x + 1)`
did not type its lambda either (its output has two kotlinc errors, one per
lambda): with no dominant lead the fn path typed it `Unknown` and took the
first overload. The same rule covers it now.

#### D17. An effect member's parameter defaults are accepted but cannot be used

`check_effect_call` and `pick_effect_member` require `args.len() == fixed`.
A member call has no [fn-param-default] shape (`call_shape_fits`) and no
named fixed arguments.

- `member-default`: `effect Log { fn log(msg: Str, level: Int = 1) }`.
  `note("hi")`, a fn of the same shape, is accepted. `log("hi")` is
  refused: "`log` takes 2 argument(s), found 1".

**Under P3** member calls are ordinary calls, so `log("hi")` would newly be
accepted. The alternative is to refuse defaults on members at their
declaration. **Decision:** none beyond P3, unless you prefer refusing the
declaration.

#### D18. A task mint resolves its target by the old scope ladder

`free_send_target` keeps the most specific rung that has a `send fn` of
that name, and refuses only a tie within that rung. That is the rule calls
dropped on 2026-09-26. It also reads `scope.fns` raw (renames ignored) and
takes no `@` selector. [task-mint]'s text ("by the ordinary scope ladder
[fn-overload-scope]") describes the old rule.

- `mint-rung`: `import lib.formatted` beside this module's own `send fn
  formatted` with the same signature. The analysis is clean, and Rust and
  Kotlin both print `mint: main: sum = 5` (this module's).
- `mint-rung-call`: the same import-beside-own pair as plain fns (`label`)
  is refused at the call as ambiguous.

**Under P3** the mint would be ambiguous and so newly refused, with
`formatted@main` as the remedy. **Decision: yes.** A mint has only a
prefix of the arguments (its captures, without the answer). Should a mint
rank its candidates by the capture types and the `Reply` position like a
partial call, or refuse whenever more than one `send fn` of the name is
visible?

#### D19. A computed callee checks neither the arity nor the argument types

The computed-callee path (30630) checks each argument against the
parameter type as an *expected* type only. It reports no mismatch and no
arity error. It also applies no contract, unlike a call through a local.

- `computed-callee`: `adder()("x")` and `adder()(1, 2)` pass the analysis.
  rustc refuses them (E0308, E0057), and so does kotlinc ("argument type
  mismatch", "too many arguments").

**Under P3** it is one call node, checked like a local call: newly refused,
at the checker instead of the target compiler. **Today it is a
[backend-never-wrong] gap.** **Decision:** none.

Fixed (commit "Qualify every declared callee in Kotlin and check fn-value calls"): checked like a call through a
local (fit, claims, count, contract); the IR also passes a computed
callee's effects, which it did not (rustc E0057, kotlinc "no value passed").

#### D20. A call through a fn-typed local does not check claims

`arg_fits_fn_value` compares base shapes ("what a qualifier or a `Mut`
demands is the contract's business"), and `apply_fn_value_contract` checks
only kept, `Mut` and projection rules. So a claim the parameter demands is
dropped.

- `local-call-claims`: inside `through(f: (xs: NonEmpty List<Int>) -> Int,
  ys: List<Int>)`, `f(ys)` is accepted. The same argument to the declared
  fn (`head1(ys)` in `direct`) is refused: "no matching overload for
  `head1(List<Int>)`".
- `local-call-claims-run`: `through(head1, List())`. Rust and Kotlin both
  print `local: size of a NonEmpty list = 0`. The claim is violated
  silently.

**Under P3** such programs would newly be refused (`arg_fits_param`,
qualifiers included). **Decision:** none.

Fixed (commit "Qualify every declared callee in Kotlin and check fn-value calls"): `f(ys)` is refused ("does not
carry what the parameter claims"); a dependent claim (`Idx(xs) Int`) is
about this call's argument, as at a declared call.

#### D21. Two implicits of one name are told apart by fit, not ranked

- `implicit-same-name-rank`: `both(n: Int, ?tag: (x: Int) -> Str, ?tag: (x:
  Int | Str) -> Str)`. `tag(n)` inside it is refused: "could go through
  more than one implicit of that name". Two declared overloads with those
  parameter types are ranked: `tag(1)` → `int`.

**Under P3** they would be ranked as overloads and so newly accepted. The
comparison operator's own-implicit step has the same rule: it refuses
when more than one fits ([cmp-binder]). **Decision: maybe.** Implicits shadow as
locals do, and two locals of one name cannot coexist, so ranking among
shadowing implicits is a rule P3 does not settle by itself.

Fixed (commit "Select every implicit argument as a call selects"): ranked like overloads; the probe prints `implicit: int` on both backends, and a tie is still refused.

#### D22. Inference from an implicit uses its own unranked matcher

`extend_subst_from_implicits` learns the callee's type arguments from an
implicit's candidates only when exactly one candidate fits. It skips any
candidate with implicits or a variadic tail.

- `implicit-infer-unranked`: with `f(x: Int) -> Str` and `f<T>(x: T) ->
  Str`, and `conv<B>(x: Int, ?f: (x: Int) -> B)`. `f(1)` resolves.
  `let s: Str = conv(1)` is accepted. `conv(1)` alone is refused: "cannot
  infer type argument `B` of `conv`".

**Under P3** inference runs selection (`select_fn_value` at `(Int) -> B`
with `B` open, the most specific winner) and reads `B` off the winner, so
the call would newly be accepted. **Decision:** none.

Fixed (commit "Select every implicit argument as a call selects"): inference selects and reads `B` off the winner; the probe prints `inferred:  int` on both backends.

#### D23. A written identity falls back to the most specific rung

`written_declaration` resolves the name at the slot's fn type, discards
the diagnostics, and when resolution fails (for example by ambiguity) takes
`max_by_key(rung)`: the old ladder. `resolve_identity` then looks only in
that module.

- `carry-rung`: `import lib.by_len` beside this module's own `by_len`.
  `let l: Mut SortedSet<Str>(by_len) = … cmp = by_len@lib` is refused:
  "expected `Mut SortedSet<Str>(by_len@main)`, found …(by_len@lib)". So the
  written `by_len` silently means `by_len@main`, while the call
  `by_len(a, b)` is ambiguous (the `mint-rung-call` shape).

**Under P3** the written type would be ambiguous and newly refused, with
`(by_len@main)` as the remedy. **Decision:** none.

Fixed (commit "Select every implicit argument as a call selects"): the written `SortedSet<Str>(by_len)` is ambiguous, an error naming `by_len@lib` and `by_len@main`; `SortedSet<Str>(by_len@main)` settles it.

#### D24. Send-member and own-handler targets are picked by arity or by first name

`check_addr_call` picks among overloaded send members by arity only and
refuses a tie. It also ignores the call's type arguments and named
arguments, and drops implicit parameters from the member's list.
`check_self_send`, `check_servant_send` and the member mint in
`check_replyto` take the first member with the name. The arguments are
checked with `is_subtype`, not `arg_fits_param`.

- Probe: `addr-dot-fn` covers the addr path. The overload cases are not
  probed: std and the examples overload no send member, so nothing changes
  for existing programs.

**Under P3** these calls would rank by type, and an implicit on a send
member would be filled or refused explicitly. The probe for D14 is the
evidence here. **Decision:** none, but piece 7 of B3 should decide whether
send members may overload at all.

#### D25. Effect-member selection is a weaker copy of call selection (P4)

These are differences inside `pick_effect_member` and `fitting_members`,
read from the code. They change results only for overloaded members:

- arity is checked first, and a single member of that arity is taken
  untested;
- the fit is `is_subtype || unify`, so there is no projection leniency, no
  literal, numeric or dependent-claim handling, and no contract
  (`arg_fits_param` is used only later, for the error message);
- there is no [implicit-fit] drop and no [fn-overload-return] tiebreak;
- an ambiguity is reported, and the check then continues as the first
  member.

The member-versus-fn comparison uses each side's best only, through
`fitting_fns`, which sees no named arguments, no defaults beyond
positional ones, no dependent claims and no literal adoption.

- `member-vs-fn` (the P4 baseline, no divergence): an available `show(x:
  Int | Str)` member beside `fn show(x: Int)`. Rust and Kotlin both print
  `show(1):   fn` and `show("s"): member`. So with one available effect,
  members **already compete with fns** (since 2026-09-14,
  [effect-available]). The [fn-overload-scope] bullet "an effect member
  takes the name before any fn does" and P4's "today … it shadows, it does
  not compete" describe the multi-effect case (D15), not this one.

**Under P4** all of this becomes one candidate set and one selection.
**Decision:** none.

### What P4's "effect members compete with fns" changes

- **D15:** newly accepted. A bare call across two available effects is
  ranked by arguments.
- **D16:** newly accepted. One lead pool types lambdas in colliding calls;
  today this is a codegen failure.
- **D17:** newly accepted. Member defaults become usable.
- **D25:** behaviour changes only where an overloaded member set was
  arity-picked or recovered as the first member.
- **D3** (if members also fill interpolation, operator and implicit
  positions): newly accepted where only a member fits. **Newly refused**
  where an available member and a fn have the same signature at `${}` or
  at an operator: today the fn wins there silently, while the call is
  already an ambiguity.
- **A fn that now beats a member:** none, because the single-effect case
  already ranks the two (`member-vs-fn`). **A member-versus-fn tie that
  becomes an ambiguity:** none new at calls, since a tie is already refused.
  New only at the D3 positions.
- No std effect declares a member named `to_str`, `eq`, `cmp`, `hash`,
  `next` or `iter` (checked in `lib/std`), so std programs are unaffected
  by the D3 half.

### Decisions needed beyond P3

- **D3:** may an effect member fill an implicit, an interpolation part or an
  operator?
- **D4:** a union's per-arm rendering (already open in P4).
- **D5:** keep "operators compare the data" (qualifiers stripped) as an
  exception?
- **D6:** keep the same-type rule for `==` and `<`, and if so, add it to
  O1 D-3's shape?
- **D7:** confirm [op-arm-eq] stays as a lowering before the call.
- **D10:** does the `: Yield`/`: Iter` declaration stay as the `for` gate?
- **D13:** does a dot call see fn-typed locals?
- **D14:** does an addr receiver's dot call rank fns beside its send
  members?
- **D18:** how does a mint rank candidates when it has only a prefix of the
  arguments?
- **D21:** do implicits of one name rank among themselves?
- **D1:** optional; you may want to keep the native fast path as an
  exception. If not, O5's text needs correcting.

### Bugs to fix whatever P3 decides

All fixed (commit "Qualify every declared callee in Kotlin and check fn-value calls"); kept for B3's reference.

- **D11, D13 (Kotlin):** a bare top-level call is shadowed by a same-named
  local or parameter. This affects the `for` driver, the dot call and
  `f@m(…)`.
- **D13 (warning):** "`@` not needed" when a local shadows the name.
- **D16, D19:** checker-clean programs that rustc and kotlinc refuse.
- **D20:** a claim is dropped through a fn-typed local.

### Order for B3

This follows §II.9 B3's list, with the census's additions in place:

0. **Before B3:** the bugs above. The Kotlin shadowing (D11, D13) and the
   wrong `@` warning are small fixes in the emitter and in
   `unqualified_winner`. D19 and D20 become part of piece 6 if they wait.
   Done (2026-10-27, §II.12 "B1 bugs").
1. **Implicit arguments:** D22 (inference through selection) and D23 (no
   rung fallback for a written identity). Both are mechanical under P3.
   D3's implicit half waits on its decision.
2. **Interpolation** (waits on P4's format fn): D1, D2 (dissolved by O1-D's
   placement check, so build that check first or together), D3, D4.
3. **Operators,** with **`for` as their second half**. O1 names `iter` and
   `next` as operators, and `for_drivers` is the table the census adds to
   this piece. Operators: D1, D2, D5, D6, with D7 as a pre-call lowering.
   `for`: D8, D9, D10, D11. The arithmetic names (`plus` …) are new
   features that land here.
4. **Dot calls:** D12, D13 (after its decision). `?.` goes with them.
5. **Effect-member and qualifier-member calls** (P4): D15, D16, D17, D25,
   and D3's member half. The qualifier calls (`qual_member_calls`,
   `predicate_tests`) show no divergence and only move tables.
   D16's fix is a stopgap to replace here: a lambda the candidates do not
   type alike (different types, or one generic type) is refused, where one
   lead pool over members and fns would type it.
6. **Fn values, local calls, fn-value calls:** D21 (D19 and D20 are fixed;
   the calls share `check_fn_value_args`). `fn_refs` by name has nothing
   left (550669ad).
7. **`addr_calls`, `self_sends`, `facade_sends`, mints**
   (`replyto_tasks`, `replyto_members`), **`effect_calls`:** D14, D18, D24.
   `effect_calls` is handler choice, not callee choice; it moves with
   erasure (effects as parameters), not with P3.
