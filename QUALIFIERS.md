# Qualifier & type-construction redesign — working document

Status: **planning**. This tracks a seven-part redesign of how qualifiers and
type construction work. Per AGENTS.md, each part is a user language-design
decision; this file records the decisions, the plan, the open questions, and
progress. When a part lands, its record moves to COMPLETED.md's decision log
and this file shrinks; when the whole thing lands, this file is deleted and
its still-open tail (if any) moves to ROADMAP.md.

The user's framing: constructing a type should not require knowing the
function that builds it. Two offenders today — constructor functions
(`mut_list_of`) and literal syntaxes (`[…]`, `{…}`) — get replaced by one
consistent, user-extensible system: **functions named after the type they
build**.

---

## The seven changes (as stated by the user)

1. **Type-named functions.** Allow a `fn` whose name is a type — base type,
   qualifiers, generics, whitespace:
   ```
   fn Mut List<T>(first: T, ...elems: T[]) -> Mut List<T>
   fn List<T>(first: T, ...elems: T[]) -> List<T>
   fn Heap List<T>(first: T, ...elems: T[], ?Ordered<T>) -> Heap(?cmp) Mut List<T>
   ```
   Rules: (a) the base type must always be present; (b) the base type and
   every qualifier in the name must be in scope where the fn is defined;
   (c) the fn must return a value whose type has *at least* the base type and
   qualifier set named — it may have more (the `Heap` example returns a
   `Heap(?cmp) Mut List<T>`). **No dot-notation** (names have spaces).
   Does NOT replace `list_by` etc. (those are ordinary fns).

2. **Key-value literal sugar for pairs.** A `...pairs: (K, V)[]` variadic may
   be called with `Map("a": 1, "b": 2)` key-value syntax. General to any fn
   with a `(K,V)[]` variadic, not special to maps.

   → With 1 + 2, retire the type literals (`[…]`, `{…}`, `{k: v}`). Users write
   `Mut List(1, 2, 3)`, `Map("name": "x")`, `Heap List(5, 4, 3)`.

3. **Type-named functions for provenance qualifiers** (no base type):
   ```
   fn Ok<T canbe linear>(t: T) [] -> +Ok T => !value
   ```
   So a user writes `Ok("world")` not the lowercase `ok(...)`. No dot-calls.

4. **Infer the provenance-qualifier function at an unambiguous union arm.**
   ```
   fn long_name_length(name: Str) -> Ok Int | Err Str {
       if name.size() > 10 { return name.size() }   // implicitly Ok(name.size())
       return "Name not long enough"                // implicitly Err(...)
   }
   ```
   The named fn is the fallback whenever there is *any* ambiguity about which
   arm a value belongs to.

5. **Conditional deductions of state qualifiers:**
   ```
   fn non_empty<T>(list: List<T>) -> list is NonEmpty => list
   ```
   Backend: returns `Bool`; the deductions are conditional on the bool being
   true. Chainable with `&&`; `!is` can invalidate a qualifier. Mentioned
   parameters are kept. **Proof-only for now**: the `true` value must be
   returned from a branch that has *proven* all the named qualifiers hold.

6. **Retire the state-qualifier compatibility clause** (`with`). Compatibility
   becomes an emergent property of how qualifiers apply to their types:
   invalid combinations should be *constructively* impossible, not declared.

7. **Redesign `refn`s as pure matchers** that all combine. All applicable
   refns combine on a matching fn; the open question is how to interpret the
   combination. **Deferred — its own design session, done last.**

---

## Where the current machinery lives (from the 2026-10-12 survey)

- `FnDecl.name` is a single `Ident` (ast.rs:909). Type-named fns need the name
  to carry base + quals + generics, or to be parsed as a type and lowered to a
  canonical internal name.
- `FnDecl.constructs: Option<TypeRef>` holds the `+Qual` return marker
  (parser.rs ~2585); `ok`/`err` in std/core/result.sv are the model.
- Collection literals: `Expr::ArrayLit`/`SetLit`/`MapLit` (ast.rs ~1673),
  checked at check.rs ~22381, **already desugared to constructor calls** by
  `desugar_collection_literal` (check.rs:12123) — the backends see plain calls.
  Retiring literals = deleting these nodes once type-named ctors exist.
- `Param.variadic` + `(K,V)[]` element type is the target of change 2.
- Union-arm selection for wrapping: `coerce_repr` (check.rs ~26405). The
  0-match error path is where change 4 inserts provenance-ctor inference; the
  >1-match "disambiguate" error is the model for "unambiguous".
- `with` compatibility: `QualifierDecl.with` (ast.rs:723), enforced at
  check.rs:19213-19244 via `refine::quals_compatible`. Change 6 removes these.
- Deductions: `Deduction`/`DeductionKind` (ast.rs:1060-1130), resolved form
  `deduce::ParamDeduction`. No `is`-conditioned variant exists — change 5 adds
  one. Closest existing concept: refinement `requires` ([qual-refn-narrow]).
- Refinements: `RefnDecl` + refine.rs (`RefnGroup`, `quals_compatible`,
  `requires`, conflict logic). Change 7 replaces the conflict/compat logic.

---

## Decisions (user, 2026-10-12) — all confirmed

- **Q1 (call grammar):** a **capitalized callee is a type-fn lookup**. In call
  position a leading type (optional quals + capitalized base + optional
  `<...>`) names a type-fn: `Mut List(...)`, `List(...)`, `Heap List(...)`,
  `Map(...)`. Lowercase callees stay ordinary value-fns.
- **Q2 (rule c):** the fn must return a value whose type has the **named base
  AND at least the named qualifiers** (extra quals allowed). Not a general
  subtype check: base matches, named quals ⊆ return quals.
- **Q3 (representation):** parse the name **as a type**, derive a canonical
  internal key, keep the base+qual set for rule (c). Overloads key on the
  canonical name (like `list_of`'s two shapes).
- **Q4 (transition):** keep the lowercase ctors (`list_of`, …) working in
  parallel; remove them in a later cleanup commit.
- **Q5 (pair grammar):** parse a `k: v` argument into a pair-argument node,
  checked to fill a `(K, V)[]` variadic.
- **Q6 (mixing):** **all-pairs** — a pair-variadic call does not mix positional
  and `k: v` args.
- **Q7 (provenance fns):** `fn Ok<T>(t: T) -> +Ok T` is change 1 with the
  base-type requirement relaxed for provenance qualifiers. No dot-calls.
- **Q8 (arm inference ambiguity):** infer the provenance constructor at a
  return/arm position when **exactly one** arm is a provenance-tagged arm
  whose qualifier has **exactly one** constructor fn accepting the value. More
  than one qualifying arm → no inference; the user writes `Ok(x)` explicitly
  (works, not an error).
- **Q9 (arm inference scope):** only provenance-tagged arms. Plain untagged
  unions (`Int | Str`) already resolve by subtyping, untouched.
- **Q10 (conditional-deduction return):** `-> list is NonEmpty => list` IS the
  return-type sugar; it lowers to `Bool`. The user does not write `-> Bool`.
- **Q11 (proof-only):** `return true` is legal only from a branch where every
  named qualifier was proven for the parameter via existing narrowing
  (`is`/`assert!`/`when`). Checked against the flow-narrows already tracked.
- **Q12 (call-site narrowing):** `if non_empty(xs)` installs `NonEmpty` on `xs`
  in the then-branch; `!is` in the condition invalidates in the corresponding
  branch. A new narrowing source like `assert!(xs is NonEmpty)`.
- **Q13 (retire `with`):** **Option A** — delete the compatibility clause and
  its check entirely. Incompatibility is *implicit*: expressed by which
  functions can mint a qualifier, plus the qualifier's own `qualifies` fn. No
  declarative compatibility rule replaces `with`.
- **Q14 (refns):** deferred to its own design session, done last.

## (resolved — see Decisions above) Open design questions

### Change 1 (type-named fns)
- **Q1a. Internal representation.** Keep `FnDecl.name: Ident` and parse the
  type-name into a canonical string key (e.g. the type's `Display`), or add a
  structured `FnName` (base + quals + generics)? Recommendation: parse the
  name *as a type* (`parse_type`) and derive a canonical internal name +
  record the base/qual set for rule (c) checking. This reuses the type grammar
  (which already handles `Mut List<T>`, `Heap(?cmp) Mut List<T>`).
- **Q1b. How does a *call* `Mut List(1,2,3)` resolve?** The call `callee` is
  currently an `Expr::Ident`/path. `Mut List(...)` parses as... what? `Mut`
  then `List(...)`? The callee is a *type expression in call position*. Need a
  grammar rule: in call position, a leading type (quals + base + optional
  generics) names the type-fn. Does `List(1,2,3)` (no quals) collide with an
  ordinary fn named `List`? (Capitalization already distinguishes types from
  values, so `List` in call position is unambiguous-ish — but `Map("a": 1)`
  vs a user fn `Map`?) **This is the biggest grammar question.**
- **Q1c. Rule (c) checking** ("returns at least that base+quals"): is this
  `is_subtype(return_ty, named_ty)` with the named type as a lower bound, or a
  structural "base matches and every named qual is present"? The `Heap`
  example returns `Heap(?cmp) Mut List<T>` for a name `Heap List<T>` — so the
  check is "base == base AND named quals ⊆ return quals", not subtype.
- **Q1d. Overloading & ranking.** `fn List<T>(...)` and `fn Mut List<T>(...)`
  are different names (different qual sets) → no overload collision. But
  `List()` empty vs `List(first, ...rest)` — same name, two overloads, as
  today. Confirm the existing overload machinery keys on the *canonical type
  name* so these behave like `list_of`'s two shapes.

### Change 2 (pair literal sugar)
- **Q2a. Grammar.** `Map("a": 1, "b": 2)` — the `:` inside call args. Today
  `:` in an arg list appears nowhere. Parse each `k: v` arg as a tuple `(k,
  v)` when the callee's matching variadic param is `(K,V)[]`? Or always parse
  `k: v` args as pairs and let the checker reject if the param isn't a pair
  variadic? Recommendation: parse `k: v` in an arg list into a pair-arg AST
  node, check it fills a `(K,V)[]` variadic.
- **Q2b. Mixing.** Can positional and `k: v` args mix? (`f(1, "a": 2)`?)
  Probably not — a `(K,V)[]` variadic takes all-pairs. Confirm.

### Change 4 (provenance arm inference)
- **Q4a. "Unambiguous" definition.** Exactly one arm whose provenance
  qualifier has a constructor fn that accepts the value's type. If two arms
  could (`Ok Int | AlsoOk Int`), fall back to requiring the explicit `Ok(...)`.
  Confirm: ambiguity = ">1 arm's provenance ctor accepts the value", and the
  fallback is a *required explicit call*, not an error — i.e. the user writes
  `Ok(x)` and it works, they just can't omit it.
- **Q4b. Which fn is inferred?** Change 3 makes `Ok` a fn. Does inference
  insert a call to the *type-named* `Ok` fn, or to the provenance qualifier's
  constructor generally? If a provenance qual has *several* constructor fns,
  "only infer when there's exactly one" (the user said "only one function for
  a provenance qualifier" is the common case). Confirm: infer only when the
  arm's provenance qual has exactly one constructor fn AND exactly one arm
  qualifies.
- **Q4c. Non-provenance arms.** `Int | Str` (no provenance tags) already works
  by subtype arm-matching. Change 4 is specifically about *tagged* arms where
  the plain value isn't a subtype. Confirm scope: only provenance-tagged arms.

### Change 5 (conditional deductions)
- **Q5a. Return type.** `-> list is NonEmpty => list` — the fn's actual return
  type is `Bool` (backend), but the signature writes the *condition*. Is the
  surface return type `Bool` (implicit) or must the user write it? The example
  omits it. Recommendation: the `X is Q` return *is* the return type sugar,
  lowering to `Bool`.
- **Q5b. "Proof-only."** The `true` must be returned from a branch that proved
  all named quals. How is "proved" established — via the existing narrowing
  machinery (an `assert!`/`is` that installed the qual in that branch)? So the
  body must have narrowed `list` to `NonEmpty` on the path that returns `true`.
  Confirm this is checked against the flow narrows already tracked.
- **Q5c. Call-site effect.** At `if non_empty(xs) { ... }`, the then-branch
  installs `NonEmpty` on `xs` (and the else-branch via `!is`?). This is a new
  narrowing source — like `assert!(xs is NonEmpty)` but as a reusable fn.
  Confirm the then/else narrowing semantics.

### Change 6 (retire `with`)
- **Q6a.** "Constructively incompatible" — what makes two state quals
  incompatible once `with` is gone? Today `with` is a *declared allowlist*
  (pairs must opt in). Removing it means *all* pairs are allowed unless
  something else forbids them. What forbids them? If nothing does, then every
  state-qual pair composes freely — is that the intent (drop the check
  entirely), or is there a replacement rule (e.g. two quals over the same
  `qualifies` predicate that can't both hold)? The user says "emergent from how
  they apply to their types" — I need the concrete rule. **This is the key
  open question for 6.**

### Change 7 — deferred, design session later.

---

## Proposed sequencing

The changes are coupled; order matters so we don't build on soon-to-change
ground:

1. **Change 1 (type-named fns)** first — it's the foundation 2/3/4 build on.
   Big grammar change. Land it with the collection ctors *re-expressed* as
   type-named fns (keeping the lowercase ones working in parallel initially).
2. **Change 3 (provenance type-named fns)** next — small, extends 1 to the
   no-base-type case. `Ok`/`Err`/`Emitted` get `Ok`/`Err`/`Emitted` fns.
3. **Change 2 (pair sugar)** — enables `Map("a": 1)`, then retire the literals.
4. **Change 4 (arm inference)** — depends on 3 (needs the provenance fns).
5. **Change 6 (retire `with`)** — independent, but do before 7 (7 reuses the
   compat logic). Needs Q6a answered.
6. **Change 5 (conditional deductions)** — fairly independent; can slot in.
7. **Change 7 (refn redesign)** — last, own design session.

Each lands as its own commit (or small series), warm nextest green before
each, with its own spec rules and a decision-log entry.

---

## Progress

Status: **core landed** (parser + resolution + both backends run type-fn
decls and calls). Remaining for change 1: the checker's rule-(c) validation
(enforce "returns the named base + ⊇ named quals"), and migrating std
collection constructors to type-fn form (deferred to the cleanup after 3).

**Implemented so far:**
- `FnDecl.type_name: Option<Type>` (ast.rs). Canonical key joins quals+base
  with `_` (`Mut List<T>` → `Mut_List`), generics from the base's bare args.
  `type_fn_key()` free fn in parser.rs.
- `parse_fn_flavored`: capitalized name → `parse_type_fn_name()` (a new ref
  parser that stops before `(` and suppresses value-args via
  `parse_type_ref_opts(false)`), derives key+generics, sets `type_name`, skips
  the usual `<…>` generics parse.
- Call site (`parse_ident_expr`): capitalized callee → `parse_type_fn_name`,
  then if `(` follows AND no `{` follows the parens AND `parse_call_args`
  succeeds → `Expr::Call` with the canonical-key ident callee + base type-args.
  Rolls back to struct-lit/value-args otherwise (this handles
  `Ranked<T>(?cmp) { … }` struct literals and `(?cmp)` value-args, which must
  NOT be read as calls).
- `parse_type_ref_opts(allow_value_args)`: new, `parse_type_ref` delegates
  `true`. Type-fn names use `false` so `Pair(1,2)` isn't read as `Pair` with
  value-args.

**Parser gotchas discovered (for a resumer):**
- `parse_type_atom` reads a trailing `(` as a *qualified group*
  (`Ok (A|B)`) and the value-args block reads `(?cmp)`/`(1,2)` as type value
  args — both would swallow a type-fn's param/arg parens. Hence the dedicated
  `parse_type_fn_name` + `allow_value_args=false`.
- A struct literal `Ranked<T>(?cmp) { … }` is a type + value-args + `{}`; the
  call-site branch must NOT treat its `(?cmp)` as call args — the "`{` after
  the parens → not a call" + "`parse_call_args` fails on `?` → not a call"
  checks cover it.

**Tests:** `type_named_fn_parses` (parser_tests.rs). `casing_rule_...` lost its
`fn Foo()`-is-error case (change 1 overturns it). Parser snapshots re-accepted
(the new `type_name: None` field).

### Still TODO for change 1
- Checker rule-(c) validation. ✓ DONE (`check_type_named_fn`, carry_tests
  `type_named_fn_return_is_checked`). Handles the provenance base-less form too.
- `[fn-type-named]` spec rule added to LANGUAGE_SPEC.md. ✓

### KNOWN GAP (blocks change 3's `canbe`): generics with `canbe` on a type-fn name
FIXED 2026-10-12. `parse_type_fn_name` now parses each ref as name +
canbe-aware `<…>` (via `parse_generics_canbe`), returning `(Type, generics,
generic_canbe)`; the base's `canbe` opt-ins thread into `FnDecl.generic_canbe`.
`fn Mut List<T canbe linear>(…)` parses. `parse_fn_rest` gained a
`type_canbe` param. Test: `type_named_fn_parses` extended.

### Change 3 — provenance type-named fns — NOT STARTED
### Change 3 — provenance type-named fns — DONE (2026-10-12)
Landed. Key finding: a `-> +Ok T` establishment **parses into `f.constructs`**
(the parser lifts the `+Q` out of the return type, leaving `T`), so a
provenance type-fn is already a `constructs` constructor and the
`[qual-ctor-same-file]` rule applies automatically via `f.constructs` — no new
constructor-recognition machinery was needed. The only checker change was in
`check_type_named_fn`: its provenance branch now treats the named qualifier as
carried when it appears either on the lowered return `ret.quals()` **or** in
the lifted `f.constructs` clause (`ctor_qual`), because the lowered return no
longer shows the `+Q`.

std migration: `std/core/result.sv` now declares `fn Ok<T canbe linear>(value:
T) [] -> +Ok T => !value` and `fn Err<…>` alongside the lowercase `ok`/`err`
(kept in parallel per Q4). Verified on both backends: `Ok(7)` / `Err("neg")`
mint the correct union arms, `when` discriminates, identical stdout.

Tests: `provenance_type_named_fn_is_checked` in carry_tests (accepts the
matching tag, refuses a `-> +Err T` return under `fn Ok`). Spec: the
`[fn-type-named]` rule gained a "Provenance form is a constructor" bullet.

Still open for change 4: arm inference (`return name.size()` → implicit
`Ok(...)`) — the "which provenance ctor builds this arm" machinery. Not part
of change 3.


### Change 1 — type-named functions — IN PROGRESS (started 2026-10-12)

Concrete implementation design (locked):

- **AST** (`crates/salvo-syntax/src/ast.rs`): `FnDecl` gains
  `type_name: Option<Type>`. When `Some`, `name` is a **canonical key** derived
  from the type — qualifiers in sorted order then the base, generics stripped
  to nothing in the key (e.g. type `Mut List<T>` → key `"Mut List"`, type
  `Ok<T>` → key `"Ok"`, type `Heap List<T>` → key `"Heap List"`). The type's
  generic params populate `FnDecl.generics`. All downstream name-keying keeps
  working on the string key.
- **Parser decl** (`parse_fn_rest`, parser.rs ~2516): after `fn`, if the next
  token begins a type name (`at_type_name()` — capitalized — or a qualifier
  word), parse a **type** (reusing `parse_type` / a type-atom parse) as the
  name; derive canonical key + generics into `FnDecl`. Else the existing
  lowercase `ident_value` path. A type-named fn does NOT then call
  `parse_generics_canbe` (the `<T>` was consumed as part of the type).
- **Parser call** (`parse_ident_expr`, parser.rs ~5850): after the speculative
  `parse_type_atom`, if `(` follows (same line), it is a **type-fn call**: the
  callee is `Expr::Ident(canonical_key)` and we parse call args. Covers
  `Mut List(...)`, `List(...)`, `Heap List(...)`. (Capitalized callee = type-fn,
  Q1.)
- **Resolver/overload**: keys on `name.name` string already → the canonical key
  participates in the normal overload set. `fn List()` and `fn List(first,
  ...rest)` are two overloads of key `"List"`, ranked by the existing
  arity-shape tiebreak.
- **Rule (c) check** (checker, where fns are validated): for a type-named fn,
  the return type's base must equal the named base AND every named qualifier
  must be present in the return type's qualifier set (extra quals allowed).
  Base-type required EXCEPT when the name is purely provenance qualifiers
  (change 3 relaxation — a type-named fn whose "type" is `Ok<T>` has base `T`,
  a bare var, which is the no-base case).
- **No dot-notation**: a type-named fn is not callable as `x.Mut List(...)` —
  the resolver's dot-call path must skip type-fn keys (they contain spaces /
  capitals, so a dot-call lookup naturally won't find them, but assert it).

Status: design locked, starting AST + parser edits.

