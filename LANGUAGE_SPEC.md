# Salvo Language Spec — Labeled Rules

A companion to [LANGUAGE.md](LANGUAGE.md) (the narrative spec). This file
states every language feature as a short, labeled rule, with the compiler's
implementation decisions as sub-bullets. It assumes familiarity with the
language; its purpose is precision and greppability.

Conventions:

* Rule labels are stable identifiers: `[area-topic]`. Compiler code and
  tests reference them in comments (`grep -rn '\[qual-erasure\]'` finds the
  rule, its implementation, and its tests).
* Top-level bullets are *language rules* (what LANGUAGE.md means).
  Sub-bullets are *compiler decisions* (how the implementation realizes the
  rule, including deliberate cuts). When behavior is ambiguous, LANGUAGE.md
  decides; when they conflict, fix one and note it in PROGRESS.md.
* This file is backend-neutral. Each backend has its own
  `BACKEND_SPEC.<backend>.md` (e.g. [BACKEND_SPEC.kotlin.md](BACKEND_SPEC.kotlin.md)),
  loaded only when working on that backend. A backend spec may *repeat*
  rules from this file to add backend interpretation details, and may add
  new rules whose labels are prefixed with the backend's tag (`kt-` for
  Kotlin, `rs-` for Rust).
* "Checker" = `salvo-core/src/check.rs`, "resolver" =
  `salvo-core/src/resolve.rs`; each backend's emitter lives in its own
  crate (`salvo-backend-<backend>`).

## Types

* [type-basic] Basic types: `Byte`, `Int`, `Long`, `Float`, `Double`,
  `Bool`, `Char`, `None` (unit/no-value singleton), `Str`.
  * Declared as `internal type` in `std/core/basic.sv` /
    `std/core/string.sv`; each backend maps them natively.
* [lit-numeric] Numeric literals: `1` is `Int`; `1L` is `Long`; `1.2` is
  `Double`; `1.2f` is `Float`. Underscore separators are allowed
  (`1_000L`).
  * The `f` suffix requires a decimal point (`1f` is a lex error telling
    you to write `1.0f`); `L` forbids one (`1.2L` is a lex error); a
    literal running into identifier characters (`10x`, `1.2fx`) is a lex
    error. `1.size()` still lexes as an int followed by a method call.
  * There are no implicit numeric widenings: `let x: Long = 1` is a type
    error — write `1L`.
  * Backends: Kotlin renders the suffixes as its own (`1L`, `1.2f`);
    Rust renders explicit types (`1i64`, `1.2f32`) and leaves unsuffixed
    literals bare for inference.
* [type-str] Strings are immutable, with `${...}` interpolation in
  literals.
  * The lexer captures each `${...}` fragment as raw source + offset; the
    parser re-lexes fragments with spans shifted back into the file.
  * Interpolation is a *read* (user decision 2026-09-02, L2a): `"${n}"`
    never consumes `n` [deduce-consume]. Both backends render the
    interpolated value as an owned copy purely for formatting — no
    reference is retained — so reads-never-consume is parity-sound.
* [interp-no-none] Interpolating a possibly-`None` value is an error
  (user decision 2026-09-02): narrow it (`is` / `when`, taking the
  binding for a non-variable place) or assert it with `!`. Interpolating
  `None` itself is an error too — it has no text form. Motivation is
  parity: Kotlin would print `null` while Rust rejects the `Option`
  (`Display` is not implemented), so leniency made the backends disagree
  observably. `Unknown`/`Nothing` operands stay lenient
  [type-unknown-lenient].
  * A narrowable *place* is enough: `if p.surname is Str { "${p.surname}" }`
    is accepted, because field chains flow-narrow [flow-place]. Element
    reads (`arr[i]`) do not narrow, so those still need the
    `is Str name` binding form.
* [type-tuple] `(A, B, C)` is a tuple type; tuples can be destructured in
  `let` and indexed by position ([expr-tuple-index]).
  * Backends may support only small sizes; unsupported sizes are codegen
    errors ([backend-never-wrong]).
* [expr-tuple-index] `t.0` reads a tuple element by *constant* position,
  zero-based; chains nest left to right (`t.1.0` is element 0 of element 1).
  * Only tuples have elements, and the position must exist — both are
    errors, not leniency: the index is program text, so nothing about it
    can be interop-dependent. An `Unknown` base stays lenient
    ([type-unknown-lenient]).
  * Tuple elements are **read-only**: qualifiers cannot apply to a tuple
    ([qual-union-arm]), so no tuple value can be `Mut` and there is nothing
    to assign through ([struct-mut]). Assigning to one is an error naming
    the rebuild remedy.
  * No suffixes (`t.0L`, `t.0f` are parse errors).
  * Lexing: a digit sequence directly after `.` is an index, never a
    fraction, so `t.0.1` is two indices rather than `t` and `0.1`. Nothing
    else in the grammar places a numeric literal after a dot (paths and
    dot-calls take identifiers; spread is one `...` token), so the
    suspension of the decimal-point rule is unambiguous.
  * A constant index names one storage location, so it narrows
    ([flow-place]) and can be an `is` subject.
* [type-union] `A | B | C` is a union type. Duplicate arms collapse
  (`Str | Str` ≡ `Str`) — but *qualified* duplicates are distinct arms.
  * `Ty::Union` invariants: ≥ 2 arms, no nested unions (flattened), arms
    deduped, declaration order preserved.
* [union-arm-identity] Union arm identity is *positional over the declared
  type's non-`None` arms, in declaration order* (qualifiers erased).
  * The checker's side tables (`coerce`, `is_tests`) are expressed in
    these arm indices; every backend's runtime encoding must preserve
    them. Never reorder or dedupe in ways that change arm indices, and
    keep the checker and emitters agreeing on the ident-unwrap predicate.
* [type-nullable] There is no null value: `T?` is shorthand for
  `T | None`. `x!` asserts non-`None` (panics otherwise).
* [type-array] `T[]` is an array; literals `[1, 2, 3]`; generator form
  `Int[5] { i: Int -> 0 }`; `arr[i]` is 0-indexed; size via `size()`.
  * The generator form is an `ArrayInit` AST node (speculative parse).
  * std's `core.array` mirrors `core.list`'s function surface minus
    construction (literals are the constructor) and mutation (arrays are
    fixed-size): `size`, `get`, `first`, `iter` (user decision
    2026-09-02). `for` iterates arrays natively — `iter_elem_ty` handles
    `Ty::Array` before the implicit-`iter` lookup.
* [type-any-nothing] `Any` is the top type; `Nothing` is the bottom type
  (the type of `return`/`break`/`continue`), subtype of everything.
* [type-alias] `type Name<G> = ...` declares a type alias; aliases can be
  generic and must be imported like other declarations.
  * Aliases expand *structurally* at use sites (with generic
    substitution), in both the checker (`lower_base_ref`) and the
    emitters; no nominal identity.
* [type-unknown-lenient] A type the checker cannot *infer* is
  `Ty::Unknown`, compatible in both directions, and never an error by
  itself: one mistake yields one diagnostic instead of a cascade of
  follow-on complaints. Coercions/unwraps fire only where checker side
  tables say so.
  * Leniency is about **inference, not visibility** (user decision
    2026-09-03). It does not excuse anything the author *wrote*: names in
    type positions must resolve [name-resolve], and members must be
    justified by a declaration ([call-resolve], [field-resolve],
    [index-resolve], [iter-resolve]). Reaching a target-language feature
    means declaring it — `external type` for the type, `external fn`
    (+ `define`) for anything you do with it.
  * The emitters keep syntactic fallbacks where a checker table may
    legitimately have no entry (an `Unknown`-typed expression still has to
    render), but *not* where the checker now guarantees resolution: an
    unresolved dot-call is a codegen error naming the internal
    inconsistency ([backend-never-wrong]).

## Variables and scoping

* [var-no-shadow] Variable shadowing is not allowed: declaring a name that
  is already visible is an error.
* [var-no-widen] A variable's type can never widen: assignments must be
  subtypes of the *declared* type (qualifiers/narrowing may vary within
  it).
* [var-block-scope] Every block (`if`, `while`, `for`, `when` branches) is
  its own scope; declarations do not escape.
* [let-infer] `let` without an annotation takes the value's logical type
  as the declared type.
  * A narrowed union value is physically re-wrapped to its own logical
    type at the `let` boundary (coercion recorded by the checker).
* [let-destructure] `let (a, b) = ...` and `let {field, field: name} = ...`
  destructure tuples and structs.
  * Struct destructuring binds the *declared* field types, deliberately
    ignoring predicate-qualifier field overrides (see
    [qual-field-override], which applies to direct accesses only).
* [flow-place] Flow facts are keyed by *place*, not by variable name: a
  place is a local root plus a projection path (`h`, `h.field`, `h.a.b`,
  `h.pair.0`). Narrowing applies to every step that names one location
  statically — **fields and constant tuple indices**, in any combination
  (user decision P1a, 2026-09-03). An array element (`arr[i]`) is carried
  by the place type but never narrows: an unknown index may alias any
  element.
  * A read of a narrowed place has the narrowed type; its storage keeps
    the *declared* type, so both backends unwrap at the use site exactly
    as they do for a narrowed variable ([is-narrowing]). `is` tests, `is`
    bindings and `when` subjects read the storage, never the unwrapped
    payload.
  * Places relate by *prefix* (an event on `h.a` reaches `h.a.b`) and
    *overlap* (either is a prefix of the other). Siblings (`h.a`, `h.b`)
    are independent.
  * A fact survives a branch join only when every fall-through path
    agrees on it exactly; otherwise the place falls back to its declared
    type, which is always a supertype.
  * The same substrate is what place-based *ownership* (roadmap L5) will
    key on.
* [flow-place-invalidate] A place narrowing falls on any event that could
  falsify it: assignment to an overlapping place, reassignment of the
  root (including `++`), a move out of the place, and a call that keeps
  the value **mutably** (a `Mut` parameter). A call that keeps a value
  *immutably* cannot mutate it, so narrowing survives it (user decision
  P1b, 2026-09-03). Consuming a variable drops every fact about its
  parts.
  * This is the event set the fate analysis already watches
    ([fate-poison]); narrowing invalidation rides on the same sites.

## Structs

* [struct-decl] `struct Name<G> { field: Type, ... }` — public data
  fields, no methods. Trailing commas allowed.
* [struct-defaults] Fields with `= expr` defaults are optional at
  construction; all other fields are required.
* [struct-spread] `Name {...base, field: v}` copies `base` and overrides
  listed fields.
  * Multiple spreads in one literal are a codegen error (deliberate cut).
* [struct-lit-infer] The struct literal's type annotation can be dropped
  when the expected type is known (`let p: Person = {name: ...}`).
  * A struct literal with no inferable type is a codegen error, never a
    guess.
* [struct-mut] `struct Name canbe Mut { ... }` opts a struct into the `Mut`
  auto-qualifier; only `Mut Name` values may have fields assigned.
  * `Mut` is the only auto-qualifier.
  * Enforced at field-assignment sites since S1: assigning to a field of
    a struct value whose type is not `Mut`-qualified is an error (the
    `copy` intrinsic's identity lowering on Kotlin relies on non-`Mut`
    values really being immutable [copy-fn]). Arrays remain
    index-assignable without `Mut` (status quo; `copy` performs a real
    array copy).
* [type-canbe-mut] `Mut` is a language-level qualifier, not a library
  declaration: any type declaration may opt into it with `canbe Mut`
  (`external type List<T> canbe Mut`), and applying `Mut` to a type whose
  declaration does not say `canbe Mut` is an error. `Mut` composes with
  every other qualifier (no `with` compatibility needed).
  * Validated at declaration sites (`validate_quals` in the checker)
    against struct and opaque-type `auto_qualifiers`.
  * Backends decide what `Mut` means. A `define type` block may provide
    a `Mut inline:` template used when the type is `Mut`-qualified
    (Kotlin maps `Mut List<T>` to `MutableList<T>`); without one, `Mut`
    erases for that backend (Rust expresses it as `mut` bindings and
    `&mut` references instead).

## Qualifiers

* [qual-of] `qualifier Q<G> of T` declares a qualifier applying to `T`
  (its `of` type). `Q T` behaves like a distinct type for overloading.
  * Applicability is checked by unification against the `of` type at
    *declaration sites* (fn signatures, `let` annotations, struct fields)
    — not during type lowering, which runs repeatedly.
* [qual-no-dup] The same qualifier cannot be applied twice to one type
  (`Old Old Person` is invalid).
* [canbe-optin] `canbe` declares an *opt-in*: the declaration it follows
  may carry the named qualifier. Two sites use it — auto-qualifiers on
  struct and type declarations (`struct Person canbe Mut` [struct-mut],
  `external type List<T> canbe Mut` [type-canbe-mut], `struct FileHandle
  canbe Linear` [linear-canbe]) and per-type-parameter opt-ins on fns
  (`fn hold<T canbe Linear>` [linear-generics]).
  * `canbe` and `with` are unrelated clauses: `canbe` grants a qualifier
    to one declaration ("this may be Mut"), while `with` declares that
    two qualifiers may co-apply to one type ("Old may stack with
    Surname", [qual-with]). Separate keywords (`TokenKind::KwCanbe`),
    accepted at disjoint positions (user decision 2026-09-03).
  * Only the compiler's own qualifiers can be opted into: `Mut` and
    `Linear` on declarations, `Linear` on type parameters. A user
    qualifier in a `canbe` clause is the `with` confusion above, and is
    rejected as such.
* [qual-with] Two qualifiers may stack on one type only if one declares
  `with` the other; the `Mut` auto-qualifier composes with everything
  [type-canbe-mut].
  * Pairwise `with` compatibility is validated at declaration sites.
  * `with` is only ever this compatibility clause; opting a declaration
    into a qualifier is `canbe` [canbe-optin].
  * Provenance qualifiers need no `with` at all [qual-subject].
* [qual-union-arm] In an un-parenthesized union, a qualifier binds to the
  single arm it is written on, never the whole union; qualifiers cannot
  apply to tuples.
* [qual-group] A qualifier *can* apply to an explicitly parenthesized
  union group: `Ok (Ok Str | Err Int)`.
  * Lowered as `Ty::Qualified { base: Union }`. A group wraps as a whole
    when it is itself an arm of the expected union, otherwise it coerces
    as the bare inner union (physically identical after erasure). The
    group subtype rule (exact-arm equality, then drop-quals) must be
    tried *before* the generic any-arm union rule; `maybe_coerce`
    mirrors that order.
* [qual-generic] Qualifiers can be generic, and as generic as their `of`
  type or less (`qualifier Ok<T> of T`, `qualifier Ints of Pair<Int,
  Int>`). Nested qualified types (`Ok (Ok Str | Err Int)`) are legal but
  discouraged.
  * `substitute_vars` re-normalizes `Ty::Qualified` through `qualify()` so
    `Ok T` with `T = Ok Str` never nests `Qualified` in `Qualified`.
  * Constructing a nested qualified union group in a single expression
    needs an annotated intermediate `let` (single-level coercion only;
    errors, never mis-emits).
* [qual-subject] A qualifier declares what its claim is *about* (D5, user
  decisions 2026-09-03). `qualifier Q of T` is a **state** claim — about
  the value's *contents*. `provenance qualifier Q of T` is a
  **provenance** claim — about where the *handle* came from
  (`Authenticated`, `EnvironmentId`), which is content-independent.
  * **Provenance survives stripping.** A mutating call strips state
    claims the callee did not keep [deduce-syntax]; that rule is sound
    only because mutation can invalidate a claim about contents, so
    provenance is exempt (`QualEffect::removal_set` takes a provenance
    predicate; the inference side keeps them out of inferred removals
    too).
  * **Mint-only**: a provenance qualifier has no body — no `qualifies`
    (nothing in the bits establishes an origin, so a *predicate*
    provenance qualifier cannot exist) and no field overrides (those are
    claims about contents). Values gain it from constructor functions
    [qual-ctor-fn], and `is Q` on a non-union value is the error
    [qual-constructive] already gives.
  * **Droppable**, and it survives being stored into another value:
    forgetting an origin is safe, and a field typed `Q T` keeps the tag.
  * **Composes without `with`** [qual-with]: an origin is orthogonal to
    every claim about contents and to other origins, so tags stack
    freely, including several over one base
    (`Authenticated EnvironmentId Str`). Two *state* claims still need an
    explicit `with`.
  * Erased like every qualifier [qual-erasure] — the subject axis is
    invisible to backends (user decision 2026-09-03, D5a: the nominal
    flavor of this pattern is a one-field struct, and two lowering models
    for one concept was the cost that settled it).
  * The compiler's own capability qualifiers stay intrinsic and are *not*
    user-declarable: `Mut` [type-canbe-mut], `Linear` [linear-canbe],
    `Once` [once-fn], `ReadOnly` [readonly-return] each need a
    representation choice, a flow rule, a non-standard subtyping
    direction, or a restricted position. Vocabulary: users declare
    *state* or *provenance*; the compiler owns *permissions* (droppable,
    like `Mut`) and *obligations* (never droppable, like `Linear` and
    `Once`).
* [qual-predicate] A qualifier with a body is a *predicate qualifier*: it
  declares `fn qualifies(x: OfType) -> Bool`.
  * The `qualifies` signature is validated: exactly one param accepting
    the `of` type, returns `Bool`, no deductions; effects are allowed
    (see [is-qualifies-effects]).
* [is-qualifies] The `is` keyword is backed by the `qualifies` function
  for predicate qualifiers: `x is Positive` on a non-union subject calls
  `Positive.qualifies(x)` at runtime and narrows the subject's type by
  adding the qualifier in the then-branch (no else information).
  * Recorded in the checker's `predicate_tests` table (qualifier names,
    conjunction); backends lower the check to `qualifies` calls.
  * *Non-union* is load-bearing: a `Ty::Union` subject always takes the
    arm-matching path [is-narrowing], so a predicate qualifier cannot be
    tested against a union-typed value (`let x: Int | Str` then
    `x is Positive` errors with "this check can never succeed"). Narrow
    first — `x is Int && x is Positive`. Lifting this needs qualifiers
    over unions; see roadmap D4 in PROGRESS.md.
* [is-qualifies-effects] `qualifies` may declare effects; at each
  predicate `is` site those effects must be available in the caller's
  scope like any call.
* [qual-field-override] A predicate qualifier on a struct may re-declare
  fields with more specific types; these are *not* proven by the checker
  but cast-and-asserted at direct access sites (runtime exception risk is
  the user's).
  * Overrides must refine the real field's type (subtype check); access
    sites are recorded in the checker's `field_casts` table.
* [qual-constructive] A qualifier without a body is *constructive*: values
  gain it only through constructor functions. Plain values never subtype
  `Q T`, and `is Q` on a non-union value is a compile error.
* [qual-ctor-fn] `fn f(...) -> T as Q { ... }` marks a constructor: every
  return point returns plain `T`; the qualifier is applied *by
  construction*, and callers see `Q T`. This replaced value-level `as`
  expressions and the `as` effect (removed from the language).
  * Union tagging goes through generic constructors
    (`fn ok<T>(value: T) -> T as Ok`).
* [qual-ctor-same-file] Constructor functions must be declared in the same
  file as their qualifier.
* [qual-result-tags] std ships the result tags in `core.result`:
  `qualifier Ok<T> of T`, `qualifier Err<T> of T`, and their constructors
  `ok`/`err` (2026-09-03), so `-> Ok Int | Err Str` needs no local
  declarations. Nothing about them is intrinsic — they are ordinary
  constructive qualifiers, erased like all others [qual-erasure], and a
  file may still declare its own.
  * Own module rather than part of `core.basic`: `core.basic` declares
    `Int`, so every program reaches it, and `ok`/`err` living there would
    emit a dead `core/basic.{kt,rs}` into every output
    [mod-used-only]. `core.result` is reached only by programs that
    mention its names.
  * There is no `Result` type: the union *is* the result. A `type
    Result<T, E> = Ok T | Err E` alias would only hide the arms.
* [qual-ctor-simple] Constructor return types must be simple (no
  union/tuple); the constructed qualifier must satisfy the `of` type.
* [qual-ctor-predicate] Predicate qualifiers may have constructor
  functions too: the constructor asserts its predicate holds *by
  construction*, so callers get `Q T` without a runtime `is` check (no
  `qualifies` call is emitted for constructed values). All other
  constructor rules apply unchanged ([qual-ctor-same-file],
  [qual-ctor-simple]).
* [qual-erasure] Qualifiers are erased in generated code; only their
  compile-time consequences (overload choice, casts, predicate calls,
  union arm choice) survive.
  * Backends must resolve name collisions that erasure creates between
    overloads (see the backend specs).

## Control flow and expressions

* [expr-everything] Every control-flow construct is an expression; a
  branch's value/type is its last expression, and the construct's type is
  the union of branch types.
* [op-no-none] Arithmetic (`+ - * / %`) and comparison (`< > <= >=`,
  `== !=`) reject a possibly-`None` operand, and `None` itself, as an
  error (user decision 2026-09-02): nullability is tested with `is None`,
  so an optional reaching an operator is a missing narrowing. Remedy:
  narrow (`is` / `when`) or assert with `!`.
  * Same parity motivation as [interp-no-none]: Kotlin compares against
    `null` happily while Rust rejects the `Option`.
  * `Unknown`/`Nothing` operands stay lenient [type-unknown-lenient].
  * Arithmetic result typing is otherwise unchanged (the left operand's
    type, qualifiers stripped); operand typing *beyond* `None` — numeric
    towers, promotion, `Bool` for `&&`/`||` — is still open, so `&&`/`||`
    are deliberately not covered by this rule.
* [if-bool] `if`/`elif` conditions must be boolean expressions; there is
  no truthiness. `is` checks evaluate to `Bool`.
  * The parser disables struct-literal speculation in condition position
    (`no_struct`) so `if x is Person { ... }` parses.
* [if-else-none] A missing `else` contributes `None` to an
  if-expression's type (`Str` + no else → `Str?`).
* [is-narrowing] `is` checks flow-narrow their subject *place*
  [flow-place]: matched type in the then-branch, remaining arms in the
  else-branch; `elif` chains accumulate exclusions; `&&`/`||`/`!`
  propagate facts.
  * Union-test lowering and `is`-bindings work for *any* subject
    expression; narrowing needs a narrowable place, so an element read
    (`arr[i]`) is tested and bound but never narrowed.
  * Narrowing resets to the declared type for any variable assigned
    inside a branch ([narrow-assign-reset]); place facts fall on the
    events in [flow-place-invalidate].
* [is-binding] `is Type name` binds the narrowed value to a fresh
  variable in the matched branch (and per-iteration in `while`).
  * Parse heuristic: uppercase idents in the check are type refs; a
    trailing lowercase ident is the binding.
* [is-precise] Checks may include qualifiers and generics:
  `is Err Str` matches only the `Err Str` arm; `is Err` matches every
  `Err`-qualified arm; overlapping matches infer the smaller union.
* [when-union-subject] `when` requires a union-typed *variable* subject;
  there is no default branch. Field subjects stay rejected even though
  they now narrow (user decision 2026-09-03): `if … is` covers them.
* [when-exhaustive] `when` must be exhaustive over the subject's arms;
  arms are consumed sequentially (each branch matches what previous
  branches left), and a branch that can match nothing is an error. A
  `None` arm is handled via the subject's nullability.
* [when-value] `when` is an expression; branches ending in `Nothing`
  (e.g. `return`) drop out of the value type.
* [while-value] `while` evaluates to the last evaluated body expression,
  or a `break value`; `else` runs (and provides the value) only if the
  loop never ran. Same for `for`.
  * The value type joins: the body's tail type, every `break value` type,
    and the `else` tail type — or `None` when there is no `else` (the
    loop may never run). A bare `break` or a `continue` also joins `None`
    (an iteration may end without producing a value); `!` recovers the
    non-optional type when the user knows better.
  * Tails and break values coerce to the join like `if` branch values;
    `break`/`continue` outside a loop are errors; a lambda body is a
    loop barrier.
* [loop-while-is] `while x is T (name)?` re-tests in the loop condition
  and re-binds per iteration at the top of the body.
* [for-iter] `for x in e` iterates `iter(e)` implicitly when `e` is not
  already an `Iter<T>`; missing/ambiguous `iter` resolution is an error.

## Functions

* [fn-syntax] `fn name<G>(params) [effects] -> [deductions] ReturnType`;
  no implicit returns from functions (unlike blocks); omitted return type
  means `None`; omitted effect list means pure (`[]`).
* [fn-return-none] Functions returning `None` may `return` bare or not
  return at all.
  * Exception: a *bodyless* declaration must write `-> None` explicitly
    [decl-explicit].
* [decl-explicit] Nothing the compiler cannot see is inferred (user
  decision 2026-09-03). A fn with **no body** — `external fn`,
  `internal fn` — must declare its **effect list**, **deduction list**,
  and **return type**; an *effect member* must declare its deduction list
  and return type (it may not declare effects at all
  [effect-member-no-effects]). Inference from an absent body is a guess,
  and always the most permissive one: it is how std's `add` came to be
  inferred as *keeping* the element the list had taken ownership of, so
  `add(xs, h)` then reading `h` compiled on Kotlin and was rejected by
  rustc.
  * The declaration is then a real contract: an effect member's deductions
    are applied at call sites exactly like a named call's, so a member
    that takes ownership consumes its argument. Validating each *handler*
    body against the member's contract is future work (roadmap E1).
  * An external's declaration is **trusted**, not second-guessed: the
    checker does not infer mutation from a `Mut` parameter and override
    it. Catching a *wrong* external declaration by inspecting its define
    template is roadmap E2.
  * `define fn` declarations are exempt: they carry a native template, not
    a contract, and must pair one-to-one with an `external fn` (see
    [backend-define-inline]) whose declaration supplies the contract.
* [fn-must-return] A fn with a non-`None` return type must return on
  every path. Definitely-returning constructs: `return`, `if` with an
  `else` where every branch returns, `when` where every branch returns
  (exhaustiveness is enforced separately [when-exhaustive]).
  * Conservative by design: loops never count as returning (they may run
    zero times).
  * Yield-based iterator fns are exempt — their body produces elements,
    not a return value.
* [fn-overload] Functions overload by parameter types (including
  qualifiers: `full_name(Person)` vs `full_name(Surname Person)`).
  * The checker scores viable candidates (exact type match > subtype;
    qualified params more specific) and records the winner per call site
    (`call_fn`). In unchecked contexts (no recorded winner — e.g.
    define-only names) emitters narrow same-arity candidates by the
    checked argument types' base names; an ambiguous dispatch is a
    codegen error ("annotate the argument types"), never a guess
    [backend-never-wrong].
  * Generic bindings in `unify` widen: when arguments bind the same `T`
    to related types, the more general one wins regardless of order
    (`pick(1, maybe_int)` binds `T = Int?`). No occurs check,
    deliberately: `Ty::Var` identity is name-scoped per side, so a
    callee's `T` never appears inside argument types (a caller's
    same-named `T` is a different variable).
* [fn-dot] Dot-notation: `x.f(a)` ≡ `f(x, a)` whenever `f` resolves to a
  declared fn/define/effect member. There is no method-call fallback: an
  undeclared name is an unresolved call ([call-resolve]), and the
  diagnostic names `external fn` as the way to reach a target-language
  method.
* [call-resolve] Every call must resolve to something declared: a fn, a
  `define` signature, an effect member, a handler constructor, or a value
  of fn type. Otherwise it is an error (user decision 2026-09-03) —
  unresolved names carry import suggestions [diag-import-suggest].
  * Calling a value whose type is known and is not a fn type is an error
    too ("`n` is not callable: its type is `Int`"), generics included: a
    type parameter has no bounds, so nothing makes a `T` callable.
  * Only an *un-inferred* callee stays silent [type-unknown-lenient].
* [field-resolve] Only structs have fields, and only the ones they declare
  (predicate-qualifier overrides refine them [qual-field-override]).
  A field on any other known type — an `external type`, an array, a fn
  value, a generic `T` — is an error; a target-language member is reached
  through a declared accessor (`external fn`).
* [index-resolve] `[]` subscripts arrays only. Other collections expose
  element access as declared functions (std's `get(list, index)`), and
  tuples use constant positions ([expr-tuple-index]).
* [iter-resolve] A `for` subject must be an array, an `Iter<T>`, or a value
  some declared `iter` overload accepts (the implicit `iter(subject)`
  call); anything else is an error.
* [fn-variadic] `...xs: T[]` collects remaining arguments as an array;
  a spread argument `...xs` forwards an array whole; variadics bind after
  fixed params.
* [fn-lambda] Lambdas: `x -> expr`, `(a, b) -> expr`, and block bodies
  `{ x: T -> ... }` (blocks require `return`). Lambdas may declare
  effects/deductions.
  * Param types come from annotation or the expected fn type; early
    `return` inside expression-position lambdas is a codegen error
    (deliberate cut).
* [fn-iterator] A function returning `Iter<T>` and using `yield` is an
  iterator function: `yield` produces elements; `return` only
  short-circuits (no value). `Iter<T>` is an `internal type` each backend
  maps to its native iterable.

## Effects

* [effect-decl] `effect E<G> { fn member(...) -> T }` declares an effect:
  a set of functions available to code that depends on `E`.
* [effect-member-generics] Effect member fns may declare their *own*
  generics (`fn pick<T>(a: T, b: T) -> T`); they bind per call from the
  argument types (progressively — later params and the return type see
  earlier bindings). Explicit type args at the call site keep their
  [effect-disambiguation] meaning (they pin the effect *instance*, not
  member generics). Kotlin renders them on the interface member
  (`fun <T> pick(...)`); the Rust backend rejects them (loudly): `dyn`
  traits cannot have generic methods [rs-effects].
* [effect-member-no-effects] Effect member fns (and handler member fns)
  cannot declare their own effect dependencies (compile error "…yet"):
  dispatch call sites go through the handler instance and cannot thread
  extra handler args. Effect-to-effect dependencies declared on the
  *effect* are roadmap E1. Members must still declare their deductions
  and return type [decl-explicit].
* [effect-handler] `handler H<G>(ctor params) of E<G> { state fns }`
  implements every member of its effect; state fields have initializers
  and persist for the handler's lifetime.
* [effect-fn-deps] A fn's `[E1, E2<T>]` list declares its effect
  dependencies. Calling a fn requires each of its effects to be available
  in the caller (declared or `use`d) — validated by the checker at every
  call site, recorded per-call (`call_effects`) in declaration order.
* [effect-no-dup] Two effects of the same type in one list are an error
  unless their generic arguments differ (`[Random<Int>, Random<Double>]`
  is fine, `[Console, Console]` is not).
* [effect-use] `use Handler(...)` registers a handler instance for the
  rest of the current scope; `use Handler` is sugar for `use Handler()`.
  * The checker infers the handler's generics from the constructor
    arguments and registers the *concrete* effect instance
    (`use CyclicRandom(list(1,2,3))` registers `Random<Int>`), recorded
    in `use_effects`.
* [use-requires-use] `use` is only legal in functions declaring the
  special `use` effect (`main() [use]` is the conventional entry point).
* [use-no-dup] Registering a second handler for an effect instance already
  in scope is an error.
* [effect-available] Calling an effect member requires an instance of its
  effect in scope; otherwise "no handler for effect" (a spanned checker
  error since M5).
* [effect-disambiguation] With multiple instances of a generic effect in
  scope, a member call disambiguates by (in order): explicit type args
  (`next_random<Int>()`), argument types, the expected type
  (`let i: Int = next_random()`). Still >1 → "ambiguous effect call"
  error; 0 → "no handler" error.
  * The resolved instance is recorded per call site (`effect_calls`).
  * Emitter effect environments are keyed by the *checker's* effect types
    (`fn_effects` for declared lists, `use_effects` for registrations,
    `effect_calls`/`call_effects` for lookups); the backend type
    rendering is a secondary key, used only where no checker type exists
    (unchecked contexts) and for the same-base-name fallback that matches
    a generic callee effect (`Random<T>`) against a concrete instance in
    scope.
* [effect-scope] `use` registrations are block-scoped: they expire at the
  end of the enclosing block.
  * Checker `effect_env` and emitter environments truncate at block
    boundaries identically.

## Deductions

* [deduce-syntax] The `-> [param: Quals, ...]` list states what a call
  does to each parameter: listed = returned to the caller (borrowed) with
  the stated qualifiers still known; omitted from a specified list =
  moved (caller loses access).
  * Entry forms, by polarity (user design 2026-09-02, D1):

    | Form | Meaning |
    |---|---|
    | `[list]` | keep-all: nothing is stripped |
    | `[list: A B]` | **exhaustive**: afterwards *only* `A B` apply |
    | `[list:]` | exhaustive and empty: every qualifier stripped |
    | `[list: -A]` | **delta**: drop `A`, everything else survives |
    | `[list: Nothing]` | moved (same as omitting the entry) |
    | `[]` | no promises about any parameter — all moved |

  * The removal set is computed against the qualifiers the *argument*
    actually carries, not against the parameter's declared set. That is
    what makes the exhaustive form sound: it also drops qualifiers the
    callee never declared and therefore cannot have preserved.
  * **Mutation forces the exhaustive form.** A parameter the body
    mutates may use neither keep-all nor a delta: mutation can invalidate
    a caller's *state* predicates that the signature never mentions
    (provenance claims are exempt from removal entirely
    [qual-subject]; the
    unsoundness D1 fixed — `clear(list: Mut List<Int>) -> [list]` would
    silently preserve a caller's `NonEmpty`). Mutation is the only
    invalidating operation on a kept value: reads preserve state and a
    move ends the caller's access.
    * A fn with *no body* (`external`, define signatures) has nothing to
      inspect, so a parameter declared `Mut` counts as mutated — taking
      `Mut` is taking permission to invalidate. This is what makes std's
      own mutators (`add`) drop a caller's predicates. Residual, and
      deliberate: an external that mutates through the *contents* of a
      non-`Mut` parameter is trusted, like `as Qual`.
  * Exhaustiveness is **contagious** through the call graph: a fn that
    hands a parameter to an exhaustive callee can no longer promise its
    own caller's extras either, so its inferred entry becomes exhaustive
    too.
  * Written lists are shape-checked: entries must name a parameter
    (once); an exhaustive entry may only keep qualifiers declared on that
    parameter (a deduction preserves or drops, it never *adds* — `+Qual`
    is rejected, see D2); an entry is either exhaustive or a delta, never
    both; and `Nothing` is the only type form (other type narrowings are
    D1b).
  * A delta may name a qualifier the parameter does not declare (a fn that
    knows it invalidates a specific property). It is a convenience — the
    exhaustive form is the sound default, and inference never relies on a
    delta to be correct.
* [deduce-infer] An unspecified deduction list is inferred as the
  strictest deduction over all uses of each parameter in the body;
  deductions never depend on the return value. If inference is impossible,
  they must be written.
  * Inference runs as a whole-program fixpoint after checking
    (`deduce.rs`), starting optimistic (keep-all for every parameter);
    facts only shrink along `KeepAll` → `Remove` (growing) →
    `Exhaustive` (shrinking), so it terminates. Results
    are stored in the typed IR (`Checked::deductions`); Kotlin ignores
    them — they are the Rust backend's ownership/borrow contract.
  * Moves are inferred when a bare parameter is: passed to a call whose
    deduction omits it, returned, `break`- or `yield`-ed, stored in a
    struct/array/tuple literal, or passed to a `use` handler
    constructor. Binding a bare parameter (or a projection of one) with
    `let`/assignment is *not* a move by itself — it fate-links the new
    variable to the parameter [fate-link] — but a binding that is later
    moved or mutated *claims* the parameter as moved (move-mode takes
    ownership through the chain [fate-move-mode]); the claims are
    seeded into the fixpoint between the checking rounds. Effect-member
    calls — the one callee kind with no declaration to resolve to, since
    the handler is chosen at run time — borrow leniently and preserve all
    qualifiers ([call-resolve] removed the other source, backend interop);
    value flow out of a branch/loop tail is not tracked as a move yet.
  * A written list is validated against the same body facts: it may be
    *stricter* than the body (drop qualifiers, move parameters the body
    gives back), but promising a parameter back that the body moves, or
    a qualifier the body may remove, is an error.
* [deduce-consume] Deduction lists are enforced flow-sensitively at call
  sites on bare identifier arguments — written lists directly, and
  unannotated fns through their *inferred* facts: checking and inference
  iterate to a fixpoint [deduce-fixpoint], so `return list` in a callee
  consumes the caller's argument exactly like an explicit `[]`.
  * A parameter *not kept* is consumed — the variable's type narrows to
    `Nothing`, and any later reference to it is a compile error (a
    `Nothing`-typed value represents an impossibility). Reassigning the
    variable revives it. Consumption is uniform across all types: for
    backend-copyable scalars the move never appears in generated code,
    but the Salvo-level contract is enforced the same (decision:
    consistency over target-level permissiveness).
  * Every other move event consumes a bare identifier the same way (L2,
    mirroring the [deduce-infer] move list): storing it in a
    struct/array/tuple literal, spreading it (`...n` — in a struct
    literal or any spread position), `return n`, `break n`, `yield n`,
    and passing it to a `use` handler constructor. The use-site
    diagnostic names the consuming event ("consumed (moved) by a
    literal store / a `...` spread / a `break` / a `yield` / a `use`
    handler registration / an earlier call"). Reads never consume —
    in particular string interpolation `"${n}"` is a read [type-str]
    (user decision 2026-09-02, L2a).
  * The code after a loop is reached from the fall-through exit *and*
    from every `break`: the loop exit merges the flow state captured at
    each `break` statement, so a value consumed on a break path stays
    consumed after the loop even when the `break` sits inside an
    always-exiting branch (which contributes nothing to the merge
    *inside* the body — the loop exit is where its state lands).
  * `yield n` inside a loop consumes anew every iteration; the loop
    back-edge re-check reports the second-iteration use at the `yield`
    itself. `return n` is terminal — the consumption is visible only to
    unreachable code and to derived-variable poison on that path
    [fate-poison].
  * A *kept* parameter sheds its removal set: the argument's narrowed
    type loses whatever the entry's effect drops — everything unnamed for
    an exhaustive entry, the named ones for a delta — so a follow-up call
    whose overload requires a removed qualifier fails resolution (e.g. a
    second `remove_first` after `[list: Mut]` stripped `NonEmpty`).
  * Branch-aware merging: each `if`/`when` branch body's consumption and
    qualifier-removal effects are isolated and joined at the construct's
    exit. A branch that always exits (`return`/`break`/`continue` on
    every path) contributes nothing to the code after the construct —
    so `if n == 2 { break ok(n) }` followed by `n++` is legal. Across
    fall-through paths: a value consumed on *any* path stays consumed
    (maybe-moved is unusable, as in Rust); disagreeing states keep only
    the qualifiers common to all paths.
  * Each round's diagnostics replace the previous round's (checking is
    deterministic, so the stable part re-derives identically); the final
    round's deductions are re-inferred against its call resolutions
    [deduce-fixpoint].
  * Consumption is preserved across narrowing scopes: consuming an
    `is`-narrowed variable (or `when` subject) inside its own narrowed
    branch survives the narrowing restore.
  * Loop bodies are checked twice when the first pass consumed or
    weakened any variable: the second pass runs with the body's exit
    state as its entry state, so back-edge flows surface — a use early
    in the body errors when a later statement consumed the value in the
    previous iteration (re-derived duplicate diagnostics are dropped;
    consume-then-reassign within the body stays clean).
  * Variadic parameters and non-identifier arguments are not tracked.

* [deduce-fixpoint] Checking and deduction inference iterate until the
  driving facts stabilize (decision L3a, 2026-09-02): after each
  checking round the inferred deductions, move-mode candidates, and
  parameter claims [fate-move-mode] are compared with the previous
  round's — another round runs only when something changed, capped at
  four rounds total. Stable programs finish in two rounds (the
  pre-existing behavior and cost); a late-discovered fact (e.g. a
  move-mode candidate first visible under round-two narrowing) gets one
  more round, so the diagnostic lands at the true site. A program still
  unstable at the cap gets a deterministic error naming each fn whose
  inferred contract oscillates (overload resolution can flip with
  narrowing), with the remedy of writing the deduction list explicitly.
* [deduce-same-call] Arguments are evaluated left to right; within one
  call, an argument may not *mention* (read, project, interpolate, or
  capture) a value that an earlier argument of the same call consumed —
  `f(a, a)` with two moving parameters, `f(a, size(a))`, and the like
  are errors at the later argument. The remedy is `copy` at the
  argument that consumes the value. (Argument typing precedes contract
  enforcement, so this sibling-argument scan is what closes the gap;
  nested calls were always ordered correctly.)

## Shared fate

* [fate-link] Binding a variable to the value or a projection of another
  variable — `let m = n`, `let m = person.name`, assignment,
  destructuring, `for` loop bindings, `is`/`when` bindings — *links* the
  new variable to its source: they share fate. Links are directed
  (derived → root), transitive (flattened to the ultimate roots at the
  binding), and at whole-variable granularity. Reads never consume and
  never poison, on any member, at any time. Function results are
  independent — unless the fn declares a derived return
  (`ReadOnly[from: p]` [readonly-return]), in which case the result
  links to the argument; a fn returning a projection of a kept
  parameter *without* the annotation must `copy` internally.
  `copy(...)` produces an unlinked value [copy-fn].
  * Provenance is static: bare identifiers and field/index/`!` chains
    over one. Values built by calls, literals, operators, or branch
    expressions are independent.
  * Links are flow state: they union across branch merges (may-be-linked
    is linked) and survive loop back-edge re-checking.
  * Tooling presentation (user decision 2026-09-02): a derived variable
    is rendered with a compiler-inserted `ReadOnly` qualifier — bare on
    the type line, with its parameters (the fate roots and binding
    sites, recorded in `Checked::fate_reads`) shown only as on-request
    detail. Presentation-only today; `ReadOnly` is not part of the type
    system and cannot be written in source. Parameterized compiler
    qualifiers as *checked* signature vocabulary are the leading design
    for L7 (see PROGRESS.md).
* [fate-derived-readonly] A fate-linked (derived) variable in
  *borrow-mode* is read-only: moving it (a call that does not keep it,
  `return`, `break value`, `yield`, a struct/array/tuple literal store,
  spread `...`, a `use` handler-constructor argument) or mutating it (a
  `Mut` call argument, projection assignment, `++`) is an error at that
  site; the remedy is `copy`. Since S2, this is the rule's *residual*
  scope: it applies when an ancestor is a written-kept parameter (you
  cannot move out of a borrow) — every other derived move/mutation makes
  the binding move-mode instead [fate-move-mode].
* [fate-move-mode] Bindings have modes, inferred from downstream flow
  (S2). A derived variable that is later *moved or mutated* makes its
  bind event (the `let`/assignment/`for`/`is`/`when`/destructure that
  created the links) **move-mode**: the binding takes ownership — every
  ancestor is consumed *at the binding* (a later use of an ancestor is
  an error naming the binding), and the variable is the value's
  independent owner from the binding on (no links). The whole derivation
  chain moves together (`persons` → `person` → `name` → `longest`), so
  consuming pipelines are zero-copy end to end.
  * Ownership requirement: every live ancestor must be owned by the fn —
    a local, or a parameter the fn's effective deduction contract moves.
    When the contract is *inferred*, a move-mode binding reaching a
    parameter **claims** it (the parameter becomes moved; callers hand
    over ownership — the claim is seeded into the deduction fixpoint
    [deduce-infer]). A *written* list that keeps the parameter blocks
    the claim: the binding stays borrow-mode and the S1 error stands at
    the move site [fate-derived-readonly].
  * Mode inference rides the checking rounds [deduce-fixpoint]: the
    first round is strict (every derived move/mutation records its bind
    chain as move-mode candidates and its parameter claims; the errors
    are discarded), later rounds apply the modes. A candidate first
    discovered in a later round triggers one more round, so it is
    applied rather than left as a strict error.
  * A **projection in a moved position** (consuming call argument,
    literal store, spread, `return`/`break`/`yield`, `use` ctor
    argument) moves data out of its provenance roots — but only
    projections of *transitively mutable* data are tracked (`Mut` at
    any depth, following struct fields): for immutable data the
    backends' clone-vs-alias difference is unobservable
    (backend-parity principle). Owned roots are consumed at the site;
    a written-kept parameter root errors ("cannot move mutable data
    out of ..."), remedy `copy`. This closed the 2026-09-02 moved-
    position parity divergence.
  * A `for`-loop binding is fresh each iteration (consuming it in the
    body is legal; the loop re-check re-declares it per pass). When a
    move-mode binding consumes a loop binding — or the loop binding is
    itself moved — the loop iterates *by value* and the iterable's
    roots are consumed.
  * Emission: the Rust backend emits move-mode bind events as real
    moves (partial moves for projections; by-value iteration for
    move-mode loops; tracked moved-position projections render as raw
    places) — `Checked::binding_modes` / `Checked::moved_projections`.
    Kotlin emission is unchanged: the consumption of the ancestors is
    what keeps alias-vs-move unobservable. `is`/`when` move-mode
    bindings still emit clones on Rust (restriction-valid; a faithful
    refinement can come with S3).
* [fate-poison] Mutating, moving, or reassigning a *root* variable
  poisons every variable derived from it: they narrow to `Nothing` with
  the fate recorded, a later use is an error naming the root and the
  event, and reassignment revives them [deduce-consume]. The root itself
  stays usable after a mutation or reassignment. Poison is not
  retroactive (a binding created after the event is unaffected), and a
  use-free poison never fires — NLL-like precision without a liveness
  analysis.
  * Mutation events are defined by the existing machinery: a call
    keeping a parameter whose declared type carries `Mut` — for bare
    identifier arguments *and* for projection arguments, which mutate
    their provenance roots (`add(h.tags, 2)` poisons variables derived
    from `h`; backend-parity fix 2026-09-02) — an assignment through a
    projection, and `++`. Whole-variable reassignment (`x = ...`,
    `x++`) is revival for `x` itself but poisons `x`'s previous
    derivatives (the old value is gone).
  * The discipline is uniform across all types and purely static: on
    Kotlin nothing physically prevents the rejected programs — it is the
    same protocol on both backends, and it is what makes clone-vs-alias
    emission differences unobservable (any program that could tell the
    difference is rejected).
  * Bare identifiers in every moved position are tracked since L2
    [deduce-consume]; projections of *mutable* data in moved positions
    are tracked since S2 [fate-move-mode]; lambda captures are tracked
    since L4 [fate-lambda]. Projections of immutable data in moved
    positions stay untracked by design: the difference is
    unobservable.
* [fate-lambda] Lambdas are ordinary values under shared fate (decision
  L4a, 2026-09-02): a lambda's relationship to the variables it
  captures is classified from its body, and the contract binds at
  *creation* (a closure may run zero or more times, unlike a named fn
  whose deductions fire per call).
  * A capture that is only *read*: free for transitively-immutable
    values (clone-vs-alias is unobservable — backend-parity principle);
    for transitively-mutable values the lambda *value* fate-links to
    the variable [fate-link] — the variable stays readable, mutating
    it poisons the closure, and binding/moving the closure follows the
    ordinary derived-value rules [fate-move-mode].
  * A capture the body *mutates* is consumed at creation — the closure
    takes ownership (each call mutates it; an original observing those
    mutations on one backend but not the other would break parity). A
    written-kept parameter cannot be captured-and-mutated (error,
    remedy capture `copy(x)`); an inferable parameter is *claimed* as
    moved [deduce-infer].
  * A lambda that *consumes* a capture (a call that moves it, a store,
    spread, `return`/`break`/`yield`) is `Once`-typed [once-fn]: the
    capture is consumed at creation and the closure is callable at most
    once. Consuming a *linear* capture remains an error
    [linear-lambda].
  * Effects do not yet cross the lambda boundary as a contract: fn
    types parse an effect list (`(S) [E] -> T`) but the checker drops
    it, and lambda bodies check under the enclosing fn's effect
    environment (lexical). Fn-type contracts (deductions, effects, and
    a call-multiplicity qualifier enabling consuming captures) are the
    L7 parameterized-qualifier work.
* [fn-contract] Fn types carry *contracts* (L7d, 2026-09-02): parameters
  may be named, and a standard deduction list may follow the arrow —
  `(v: List<Int>) -> [] Int` consumes its argument,
  `(v: List<Int>) -> [v] Int` keeps it, and an unannotated fn type
  keeps everything (the default, matching the previous lenient
  behavior — no programs changed legality by default).
  * Calling a fn-typed value applies its contract to the arguments
    exactly like a named call [deduce-consume] [deduce-same-call]:
    moved positions consume, kept `Mut` positions are mutation events
    [fate-poison], kept positions shed their entry's removal set.
    The contract propagates through deduction inference (a fn passing
    its own parameter into a consuming contract has that parameter
    inferred moved).
  * A lambda checked against a contracted fn type inherits it: kept
    parameters belong to the closure's caller and can never be
    consumed inside the body (error, remedy `copy`); `Mut`-kept
    parameters may be mutated (mutation is `Mut`-type-gated as usual).
  * A *named fn* passed by value carries its declaration's contract
    (written list, else inferred), so boundaries are checked with real
    modes.
  * Boundary variance is inverted like [once-fn], flagged for the same
    future review: a fn that *keeps* its argument fits where a
    *consuming* one is expected (the caller merely over-estimates the
    damage), never the reverse; mutation permission must be granted by
    the expectation (`contract_fits`).
  * Effect lists on fn types (`(v: T) [Console] -> ...`) parse but are
    not yet enforced as contracts (deferred; lambda bodies use the
    lexical effect environment). `Once` inference also remains open.
  * Backends: Kotlin erases contracts (aliases throughout; named fns
    pass as `::name` function references). Rust renders fn parameters
    as `&mut impl FnMut(…)` (accepting both plain and handler-mutating
    closures; `Once` stays owned `impl FnOnce`), argument types per
    contract (kept non-Copy `&T`, kept `Mut` `&mut T`, moved/Copy
    owned), call-site arguments per the recorded contract, lambda
    parameter bindings/annotations per contract, and named fns wrap in
    mechanical adapter closures bridging the contract's calling
    convention to the declaration's actual modes.
* [once-fn] `Once` is the language-level call-multiplicity qualifier
  (decision L7b, 2026-09-02): valid only on *function types*
  (`f: Once () -> None`), it means the value is callable **at most
  once**. Enforcement is consumption [deduce-consume]: calling a `Once`
  value consumes it, so a second call, a call on the loop back edge,
  and a call after the value escapes are the ordinary consumed-use
  errors; a call on only some branches leaves it maybe-consumed
  (conservative), and zero calls is fine.
  * **Inverted subtyping — flagged for future review** (user decision
    2026-09-02): `Once` *restricts* instead of refining, so plain
    `(A) -> B` <: `Once (A) -> B` (any fn may be treated as
    once-callable) and `Once` may **never** be dropped — the exact
    opposite of every other qualifier's direction. Special-cased in
    `is_subtype` and `unify`.
  * A lambda that *consumes* a capture is legal (superseding the
    always-error rule in [fate-lambda]) and is `Once`-typed by
    construction: the capture is consumed at creation and the closure
    fits only `Once` positions. Consuming a *linear* capture is still
    an error (the closure would inherit an exactly-once obligation —
    future work).
  * Escape rule (conservative, relaxable with fn-type contracts):
    passing a `Once` value as an argument consumes it regardless of the
    callee's contract — fn-value ownership is otherwise untracked.
  * No inference in v1: a callee must *write* `Once` to accept
    consuming lambdas (a body that calls its fn param once does not
    auto-promote); inference may come later (written validates,
    unwritten infers — the deduction precedent).
  * Backends: Rust emits `Once` fn parameters as `impl FnOnce(…)`
    (rustc's capture inference already makes consuming closures
    `FnOnce`); Kotlin emits the ordinary function type — the
    multiplicity is protocol-only on the JVM.
* [readonly-return] `-> [p] ReadOnly[from: p] T` marks a *derived
  return* (L7c, 2026-09-02; square-bracket surface — user decision:
  angle brackets read as generics, round brackets collide with
  qualified groups `Ok (A | B)`, and square brackets are already where
  annotations name parameters). The fn returns a *borrow* of the kept
  parameter `p` instead of an independent value — the relaxation of
  S1a's "results are always independent" rule.
  * Callee: `p` must be a parameter and *kept* (written or inferred);
    every returned value must be derived from `p` (its link chain
    terminates at `p`) or be `None`; returns do not consume. A
    forwarded derived-return call (`return first(persons)`) validates
    through the same links.
  * Caller: the result fate-links to the argument in `p`'s position —
    the ordinary discipline follows (mutating the argument poisons the
    result [fate-poison]; the links carry a *borrowed* flag, so
    move-mode can never take ownership through them
    [fate-move-mode] — moving the result or its narrowed binding stays
    an error with the `copy` remedy). The result's *type* is the plain
    written type: `ReadOnly` never affects overloading.
  * v1 scope: fn declarations (incl. externals) only; plain `T` and
    `T?` return shapes; not writable anywhere but return position.
    Accumulator bodies (`best = person; ...; return best`) are out of
    scope — reassignable borrowed locals are a recorded refinement.
  * Backends: Kotlin unchanged (the result is the alias). Rust returns
    `&T` / `Option<&T>`: lifetime elision covers a single reference
    parameter; with more, a `'a` is generated mechanically onto the
    annotated parameter and return — the first deliberate exception to
    the no-lifetimes invariant. Return values render as borrows; std's
    `first` is clone-free.
* [copy-fn] `core.copy` — `internal fn copy<T>(value: T) -> [value] T` —
  duplicates a value: the argument is kept untouched with all its
  qualifiers (`[value]`), and the result is a fresh value with no fate
  links. It is the one-word remedy in every fate diagnostic.
  * Lowered type-directedly by each backend [internal-fn]; identity
    where no Salvo operation can mutate the value, a real copy where
    one can, and a codegen error where no correct copy exists yet
    [backend-never-wrong].

## Linear types

* [linear-canbe] A type opts into linearity at its declaration with
  `canbe Linear` (structs and `internal`/`external` types; decision L6a,
  2026-09-02). Every value of the type is linear — `Linear` cannot be
  written in a use-site type (error): a per-value qualifier that could
  be forgotten would defeat the protection. Tooling may present
  linearity as a compiler-facing property.
* [linear-obligation] A linear value carries a *use obligation*: on
  every path it must be moved onward before it goes out of scope
  (decision L6b: consumption = any move, exactly as the deduction
  system defines it — consuming call, `return`, `break`/`yield` value,
  literal store, spread, `use` constructor argument, move-mode
  binding). Moves *transfer* the obligation: a callee that receives the
  value by move discharges it in turn; a kept parameter leaves the
  obligation with the caller; derived (fate-linked) variables are
  aliases and owe nothing. Violations are errors at:
  * scope exit — a frame pops while a variable still owns a live
    linear value (including moved-in fn parameters and lambda
    parameters);
  * `return` (any live obligation anywhere) and `break`/`continue`
    (obligations in frames inside the loop);
  * expression statements whose value is linear (dropped on the spot);
  * assignment over a variable still owning a live linear value;
  * branch merges where the value is consumed on some fall-through
    paths but not all — the dual of the affine maybe-moved rule.
  * Enforced from the second checking round onward (parameter
    ownership needs contracts [deduce-fixpoint]); reported variables
    are marked consumed so each obligation errors once.
* [linear-discard] `core.discard` —
  `internal fn discard<T>(value: T) -> [] None` — deliberately drops a
  value, consuming it: the escape hatch (decision L6c). Lowered per
  backend [internal-fn]: Rust `drop(value)`; Kotlin evaluates and
  ignores (`(value).let {}`). Failure/panic paths are out of scope
  until Salvo has such semantics.
* [linear-composite] A composite containing a linear component is
  itself linear (decision L6d): struct fields (followed recursively
  through declarations), type arguments, array/tuple/union components.
  Storing a linear value moves the obligation into the container.
* [linear-generics] An unconstrained generic parameter cannot be
  instantiated with a linear type (decision L6d): unopted generic code
  does not honor the obligation. A fn opts in *per type parameter* with
  `<T canbe Linear>` (decision L7a-syntax, 2026-09-02 — the same
  `canbe Linear` phrase as on type declarations, one qualifier per
  `canbe`, the comma separates parameters; spelled `with` until the
  2026-09-03 rename [canbe-optin]):
  * inside the opted fn, `T`-typed values are treated as linear
    (`Ty::Var` participates in the transitive analysis), so the body is
    checked under the worst case — including that forwarding an opted
    `T` to an unopted generic is an error (compositional);
  * for bodiless externals the opt-in is a trusted audit claim; std's
    audit opts in `list`, `mutable_list`, `add`, `size`, and `discard`
    (whose declaration is now honestly
    `internal fn discard<T canbe Linear>(value: T) -> [] None` — no
    blessed-by-name special case), while `get` stays out (returns an
    alias of an element) and `copy` refuses with a dedicated message
    (duplicating an obligation is meaningless);
  * a linear value cannot be passed in a *variadic* position (variadic
    arguments are untracked, so the obligation would be physically
    moved but statically unresolvable);
  * `canbe` on type parameters of non-fn declarations (structs,
    qualifiers, effects) is a parse error for now (struct-side deferred
    by user decision); only `Linear` is accepted in the clause.
  * Effect members with their own generics are not yet covered by the
    ban (known leftover).
* [linear-lambda] A lambda may read-capture a linear value (an alias,
  no obligation) but not capture-and-mutate one [fate-lambda]: the
  closure would swallow the obligation.
* [linear-static] Linearity is enforced purely statically and
  identically on both backends (decision L6e): no runtime component, no
  destructors — `discard` is the only way a value legally dies without
  being passed on.

## Modules, imports, files

* [mod-file] `.sv` files are modules; the module path is the file path (no
  in-file module declaration).
* [mod-ignore] Source discovery walks the source root recursively but
  skips: hidden directories (`.git`, ...), cache directories carrying a
  `CACHEDIR.TAG` marker (Cargo's `target/`), and anything listed in
  `<root>/.svignore` — one path per line, relative to the root, naming a
  file or a directory subtree; blank lines and `#` comments ignored.
  The root itself is exempt from the hidden/cache rules.
* [mod-visibility] Code sees: everything declared in its own module (all
  files of the module, including backend define files), everything in
  `core.*` (implicit), and whatever it imports.
* [mod-import] `import path.Name` / `import path.Name as Alias`; aliasing
  resolves ambiguity. Unresolved/ambiguous imports are errors.
  * Import prefixes match module paths exactly or as a leading path
    (`import core.Str` finds `core.string`).
* [name-casing] Casing is a *rule*, not a convention (N1a, user decision
  2026-09-03): names of types — structs, qualifiers, type declarations
  and aliases, effects, handlers, generic parameters — start with an
  uppercase letter; names of values — fns, parameters, fields,
  variables, bindings, lambda parameters — do not. Module path segments
  are lowercase, and since a module path *is* a file path [mod-file],
  that constrains file and directory names (`src/Utils.sv` is an error
  naming the file).
  * What the rule buys: an uppercase-initial head identifier always
    starts a *type path*, which is what makes `Environment.Id { … }` (a
    struct literal) decidable against `person.name` (a field read) and
    `list.size()` (a dot-call) [name-dot]. It also turns the parser's
    old "a lowercase word after `is` is a binding" heuristic
    (`is Str surname`) into a consequence of the rule.
  * Enforced at declaration sites in the parser (`ident_type` /
    `ident_value`); module paths in `resolve`.
* [name-resolve] Every name written in a *type position* must resolve to a
  declaration visible under [mod-visibility]; an unresolved name is an
  error naming it, with import suggestions [diag-import-suggest]
  (2026-09-03).
  * Two namespaces, checked separately. Base types: structs, type
    aliases, `internal`/`external type`s, effects (effect lists are
    written as type refs), plus generic parameters in scope and the
    language-level `None`, which has no declaration. Qualifiers:
    declared qualifiers plus the compiler's intrinsic `Mut`, `Linear`,
    `Once` (`ReadOnly` is parsed as part of the return annotation, never
    as a type ref).
  * A name found in the *other* namespace gets a wording hint instead of
    an import suggestion (`unknown type `Tag` (`Tag` is a qualifier, not
    a type)`) — no import fixes a position mistake.
  * Positions covered: fn params and return types, `let` annotations,
    struct fields, handler ctor params and state fields, effect-member
    signatures, type-alias targets, qualifier `of` types and field
    overrides, `with` [qual-with] and `canbe` [canbe-optin] clauses,
    array-init element types, and `is` / `when` checks. Reported from
    `validate_type` / `validate_quals` / `parse_check` at *declaration
    sites*, not during type lowering, which runs repeatedly (same
    discipline as [qual-of]).
  * In an `is` / `when` check either namespace is admissible in the last
    position, so the message is `unknown type or qualifier`. An
    unresolved check marks the pattern and suppresses the match-arm
    verdicts that follow from it — "this check can never succeed", "this
    `when` branch matches no remaining union arm", non-exhaustiveness,
    and the `Nothing`-narrowing cascade onto the subject. Motivation:
    with `Ok` undeclared, `if v is Ok` on `Ok Int | Err Str` used to
    report only the arm mismatch (the name was silently read as a base
    type) plus a bogus consumed-value error, and never the missing
    declaration.
* [name-dot] A struct or qualifier may be declared with a *dot-name*
  `Ns.Name` (N1, user decisions 2026-09-03), giving the Kotlin
  wrapper-type idiom (`Environment.Id`) without nested declarations.
  Dot-names are legal in every type position: annotations, `of` types,
  `is` checks, `as Q` constructors, `canbe` clauses, deduction lists,
  struct literals.
  * `Ns` must be a struct declared in the **same file**, and must not be
    generic — Kotlin emits the member as a *nested* class, which cannot
    reference the outer class's type parameters
    [kt-nested-dot-name].
  * Exactly two segments: `A.B.C` is an error, and a dot-name cannot
    itself be a namespace.
  * **Collision ban:** nothing visible in the file may carry the
    *concatenated* spelling (`EnvironmentId` beside `Environment.Id`).
    The Rust backend flattens dot-names, and overload mangling embeds the
    same spelling [rs-fn-mangling] [kt-qual-mangling], so the ban is
    checked against the whole scope — own module (all its files),
    `core.*`, and imports — not just the declaring file. No encoding
    escapes this: Rust identifiers are `[A-Za-z0-9_]`, so every encoding
    is also a legal name.
  * The name is carried as *one dotted string*, so scope keys, checker
    types, `ty_base_name` / `type_base_name` and define-template
    environments agree by construction. Backends translate at the point
    a Salvo name becomes target syntax: Kotlin renders it verbatim (a
    valid nested reference), Rust flattens it in `rs_ident`.
* [name-dot-import] `import path.Ns.Name` imports a dot-named item; the
  path splits by casing — trailing uppercase segments are the item name
  (two of them for a dot-name), everything before is the module prefix,
  and a lowercase last segment is a value (a fn), as before
  [mod-import].
  * Importing the namespace struct also brings its dot-named members
    (`import a.b.Environment` makes `Environment.Id` visible), following
    the precedent that importing an effect brings its members. Members
    ride along only on an *unaliased* import: an alias renames exactly
    one name, and a partial rename would have no sensible spelling.
* [mod-collision] Non-fn name collisions are errors, not last-win —
  same-name fns form overload sets and are exempt. Reported: a same-kind
  same-name duplicate within one module; the same name declared in two
  implicitly visible `core.*` modules; an import colliding with an
  own-module declaration or another import (`as` renames resolve it).
  Deliberate shadowing stays silent: own-module declarations and
  explicit imports override implicit `core.*` visibility.
  * Kinds are per-namespace (struct/effect/handler/qualifier/type
    alias/opaque type): same-name declarations of different kinds do
    not collide.
* [mod-used-only] Only modules used by the program are transpiled.
  * Roots are the user modules declaring `fn main` (all user modules for
    a library compile without one). Reachability follows *name usage*:
    every identifier/type name a file mentions is looked up in its
    resolved scope (`ModuleScope::name_origins`), and each declaring
    module becomes reachable (`reach.rs`). Deliberately conservative:
    shadowed and overloaded names pull in every declaring module.
  * A module's backend define files travel with it; a reachable module is
    emitted only if it produces code.

## Backends

* [backend-internal] `internal` declarations (types) are
  mapped inside the compiler; every backend must handle all of them
  (`Str`, numeric types, `Iter<T>`, ...). The `Mut` auto-qualifier is
  mapped per backend via `Mut inline:` define sections [type-canbe-mut].
* [internal-fn] `internal fn` declares a compiler-intrinsic function:
  the declaration carries the signature and deduction list the checker
  uses (body-less, like `external fn`), but there are *no* define files
  — each backend lowers calls to it directly, seeing the checker's
  resolved argument type at every call site (type-directed lowering a
  single generic define template cannot express). An internal fn a
  backend does not implement, or an argument type it cannot lower, is a
  codegen error [backend-never-wrong]. The only internal fn today is
  `core.copy` [copy-fn].
* [backend-external] `external` declarations (fns, types, handlers) carry
  only signatures; each backend that needs them provides `define`
  templates in a sibling `<module>.<backend>.sv` file. Coverage is
  checked at compile time: everything external in `core.*` must have a
  define (core is implicitly imported); outside core, a missing define
  is an error at every reference (call to an external fn, use of an
  external type, emission of an external handler) — never a silent
  pass-through.
  * `SourceSet::classify` filters define files per selected backend at
    load time.
* [backend-companion] A backend-native source file next to a module's
  sources (`complicated.kt` beside `complicated.sv`, using the backend's
  native extension) is a *companion*: it is copied verbatim into the
  output whenever its module is reachable, letting `define` templates
  delegate to hand-written native code. A companion module should
  declare only `external` items; a companion that collides with a
  generated file is an error.
* [backend-define-inline] `define fn` bodies hold an `inline:`
  \`\` template \`\` interpolated at each call site: `${param}` splices the
  argument's code, `${...variadic}` splices remaining arguments.
  Template output is written as-is (correctness not validated by Salvo).
  * Templates lex as raw dedented `Template` tokens. A template calling a
    same-named native fn must qualify it (`kotlin.io.print`) to avoid
    self-recursion.
  * **One-to-one with externals** [decl-explicit]: every `define fn` must
    implement exactly one `external fn` declaration, and no external may
    have two defines. Matching is by name, arity, then parameter *base
    type names*, so overloaded externals (`size(Str)` / `size(List<T>)`)
    pair with their own defines. The external carries the contract; the
    define's own signature is documentation.
    * Consequence: the emitters' arity/type-directed dispatch for
      *unchecked* define calls is unreachable for valid programs (every
      define has an external, which the checker resolves), so it remains
      only as a [backend-never-wrong] safety net.
  * Overloaded externals share a define name; templates are matched to
    the checker-resolved declaration by parameter base types, then
    arity/shape (`define_for_decl`; the arity-only fallback can still
    mis-pick in unchecked contexts — known leftover).
* [backend-define-imports] `imports:` sections list target-language
  imports, hoisted (deduped) to the top of any file whose code used the
  template.
* [backend-define-type] `define type` templates map external types
  (`${T}` interpolates generic args); `internal type`s map natively in
  the compiler.
* [backend-define-handler] `define handler H of E { define fn ... }`
  provides template bodies for an external handler's members; external
  handlers support constructor params like any other handler.
* [backend-companion] A `<name>.<ext>` companion source file next to a
  define file is copied into the output when used (not yet implemented,
  M7).
* [backend-never-wrong] A backend must never emit silently wrong code:
  unsupported constructs are codegen/checker errors. Each backend spec
  lists its current deliberate cuts.

## Comments and documentation

* [doc-comment] A declaration's *documentation* is the run of `//` comment
  lines directly above it (2026-09-03): the comment on the line
  immediately before the declaration, plus every consecutive comment line
  above that. One blank line ends the run, so an unrelated comment earlier
  in the file stays unrelated. There is no separate doc-comment syntax —
  a comment above a declaration *is* its documentation.
  * A comment sharing its line with code (`a: Int, // note`) documents
    nothing: it is neither the previous declaration's docs nor the next
    one's.
  * A bare `//` line inside the run is kept, as an empty line — that is
    the paragraph break of [doc-markdown], not a separator.
  * The `//` and one following space are stripped; further indentation
    survives (markdown nesting needs it).
  * Carried on the AST (`docs: Vec<String>`) for fns, structs, struct
    fields, qualifiers, effects, handlers, and type declarations.
    Comments are not tokens: the lexer collects them separately
    (`LexResult::comments`) and the parser attaches the block above each
    declaration by line number.
  * Backing modifiers do not interfere: `external`, `internal` and
    `provenance` sit on the declaration's own line.

## Tooling

* [diag-structured] The resolver, checker, and deduction pass report
  errors as structured diagnostics — `(file index, span, severity,
  message)` (`salvo_core::FileDiagnostic`) — never as pre-rendered
  strings. Human-readable rendering (file:line:col + caret) happens at
  the consuming boundary: the backends render before returning
  `BackendError`, the CLI renders for terminal output. Parser
  diagnostics stay per-file (`salvo_syntax::Diagnostic`); the CLI
  attributes them to files the same way. This is the contract a future
  language server builds on.
* [fn-ref-table] The checker records every fn-*name* reference —
  declaration names, call-site callees (incl. dot-notation), and
  fn-by-name uses — as `Checked::fn_refs: (file, name span) -> FnKey`.
  The LSP's hover renders the referenced declaration as a full
  source-like signature with an explicit return type (`None` when
  omitted) and the *effective* deduction list: the inferred/validated
  one (`Checked::deductions` [deduce-infer]) when available, else as
  declared. Moved parameters are omitted from the rendered list; an
  empty list renders as `[]` (moves everything). Effect-member calls
  and backend define fns have no `FnKey` and are not recorded.
* [diag-import-suggest] Diagnostics for unresolved names carry structured
  import suggestions: the modules elsewhere in the program that declare
  the name, as `module.Item` paths (`FileDiagnostic::suggested_imports`).
  * Computed from a whole-program declaration index
    (`Resolution::declared_in`); effect members map to their owning
    *effect* (importing the effect brings its members). `core.*` modules
    are never suggested — they are implicitly visible, so an unknown
    name cannot be fixed by importing from core.
  * Attached at: unknown handler in `use`, unknown effect in an effect
    list, and unresolved imports (which suggest the correct module path
    for the item, e.g. `import std.random.Random` -> `import
    random.Random`).
  * Rendering: text mode appends one ``help: add `import …` `` line per
    suggestion; `--format json` adds an `"imports"` array (only when
    non-empty); the LSP carries them on `Diagnostic.data` and serves
    `textDocument/codeAction` quickfixes ("Add `import …`") that insert
    the import line after the file's last import (or at the top).
* [cli-analyze] `salvo analyze --src DIR [--backend NAME]
  [--format text|json]` runs the front half of the pipeline — parse,
  resolve, type-check — and reports every diagnostic without generating
  code. Exit code is nonzero iff any diagnostic is an error.
  * Analysis is backend-neutral: checking never consults define files.
    `--backend` only opts that backend's `*.<backend>.sv` define files
    into loading (so they get parse checking); without it only language
    files are loaded.
  * Resolve/check always run, even with parse errors — a broken file
    must not suppress diagnostics elsewhere. Parse-broken files
    participate with their recovered ASTs (their parsed declarations
    still resolve for other files) but contribute only their parse
    diagnostics; their resolution/checker diagnostics are dropped
    (recovered ASTs cascade nonsense).
  * `--format json` prints a JSON array of
    `{file, line, col, start, end, severity, message}` objects to
    stdout (line/col 1-based, start/end byte offsets); text mode
    renders to stderr with a summary line.
* [cli-lsp] `salvo lsp [--backend NAME]` starts a language server
  speaking LSP over stdio. The workspace root comes from the client's
  `initialize` request; `--backend` selects define files exactly like
  `analyze --backend`.
  * No incremental state: every document event re-runs the [cli-analyze]
    pipeline over the whole workspace, with open-editor buffers as a
    content overlay (unsaved files under the root participate).
  * Diagnostics are pushed on open/change/close/save; every open
    document gets a publish (an empty list marks it clean), and files
    whose diagnostics disappeared get an explicit clearing publish.
    Diagnostics are published against the URI the client opened the
    document under (clients compare URIs exactly).
  * Hover contents are markdown [doc-markdown]: a fenced `salvo` code
    block with the declaration or type, then the doc sections below,
    separated by rules.
  * Hover answers, in order: a fn *name* — the full source-like signature
    ([fn-ref-table]) plus its docs; any other *declared name*, at its
    declaration or at a reference to it [lsp-definition] — its declaration
    line plus its docs; otherwise the checker's type
    (`Checked::expr_ty`) for the smallest expression under the cursor.
    `Unknown`-typed expressions yield no hover.
  * "Declared name" reaches *nested* declarations, not only top-level ones
    (2026-09-03): struct fields, handler state fields, qualifier field
    overrides, and the member fns of effects, handlers and qualifiers. One
    search over the module's items (`decl_at`) locates them by name span,
    so hover and go-to-definition agree.
    * A struct adds a **Fields** section [doc-struct-fields].
    * A field renders as `name: Type = default` and names what declares
      it ("Field of struct `Person`."). Reached at a *use* through
      `Checked::field_refs`.
    * A member fn renders its signature from the declaration and names its
      owner ("Member of effect `Log`."). Members have no `FnKey`, so no
      *inferred* deductions are shown — the declared list is
      [decl-explicit], which members must write anyway.
  * A fate-linked (derived) variable hovers as `ReadOnly T` — a bare
    compiler qualifier on the type line — with the qualifier's parameters
    (the roots it shares fate with and their binding sites, plus the
    `copy` remedy) as detail below (progressive disclosure, user decision
    2026-09-02) [fate-link].
  * `textDocument/codeAction` serves import quickfixes from the
    suggestions on published diagnostics [diag-import-suggest].
* [doc-markdown] Doc comments are markdown and pass through verbatim: the
  lines are already stripped of `//`, so emphasis, inline code, lists and
  code fences work as written, and a bare `//` line is a paragraph break
  [doc-comment].
* [doc-symbol-ref] `[symbol]` inside a doc comment references a name: the
  documented declaration's own names first (a fn's parameters and
  generics; a struct's fields and generics; a qualifier's, effect's or
  handler's members and state), then a type/qualifier/effect/handler/fn
  declared in the program, the declaring file first.
  * A *nested* declaration also sees its owner's names, so a handler
    member's docs can reference the handler's state and its sibling
    members.
  * A reference that resolves renders as a markdown link to the
    declaration (inline code when the declaration has no addressable
    location, as in the embedded std); one that does not resolve is left
    exactly as written, so ordinary prose in brackets is never mangled.
  * Markdown links (`[text](url)`) and inline code spans are skipped, so
    `` `[a, b]` `` stays literal.
* [doc-struct-fields] A struct's hover lists every field in declaration
  order with its type and its default as written, and appends each
  field's own doc comment [doc-comment]. Undocumented fields are listed
  too — the section shows the shape of the struct, not only its annotated
  part.
* [doc-hover-narrowed] A variable hovers as the type *known at that
  position* — the flow-narrowed type, qualifiers included — with the
  declared type named below it when narrowing changed it (`Checked::repr_ty`
  holds it) [is-narrowing].
  * Declared *names* are hovered like the variables they introduce:
    parameters, handler state fields, `let` patterns and `is`/`for`
    bindings all record their type in `Checked::expr_ty` at their own
    name span, so hover answers on the declaration and not only on uses.
* [lsp-definition] `textDocument/definition` jumps from a name to its
  declaration's *identifier*. Fn names resolve through
  `Checked::fn_refs` (overload-precise, so a call site lands on the
  overload the checker picked); every other name — struct, effect,
  handler, qualifier, type alias, opaque type, effect member — through
  `Checked::def_refs`, fed by `ModuleScope::def_sites` (visible name ->
  declaring file + identifier span, alias-aware); *field* accesses through
  `Checked::field_refs`, keyed by the field-name span in `base.field`
  (2026-09-03).
  * A field resolves to the struct's own declaration even when a predicate
    qualifier overrides its type [qual-field-override]: the override
    refines the field, it does not replace the declaration.
  * The smallest name span containing the cursor wins; a type-alias use
    jumps to the alias declaration, not through to its target.
  * Declarations in the embedded std have no on-disk URI and yield no
    location.
  * Positions convert between byte offsets (Salvo spans) and UTF-16
    line/character pairs (the LSP default encoding).
* [cli-lang] `salvo lang tm-grammar [--out PATH]` emits the TextMate
  grammar consumed by the VS Code extension (`vscode/syntaxes/`);
  without `--out` it prints to stdout.
  * Keyword alternations are derived from the lexer's keyword table
    (`salvo_syntax::token::KEYWORDS` — the same table
    `TokenKind::keyword` consults), partitioned into highlighting
    categories (control / declaration / other / boolean). A test
    asserts the partition covers the table exactly, so adding a keyword
    without categorizing it fails `cargo test`.
  * The checked-in extension grammar must byte-equal the generated one
    (`vscode_extension_grammar_is_up_to_date`); regenerate with
    `cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json`.
