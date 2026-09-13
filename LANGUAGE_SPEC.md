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
  decides; when they conflict, fix one and note it in COMPLETED.md.
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
  * Declared as `intrinsic type` in `std/core/basic.sv` /
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
* [inc-dec] `i++`, `++i`, `i--`, `--i`: a step of one on an `Int` place, in
  either fixity (user request 2026-09-11 added all but `i++`). The operand
  must be a place; all four forms do the same thing to it — which is why the
  checker treats them identically — and the fixity decides only the
  expression's *value*: postfix yields the value before the step, prefix the
  value after. In statement position the two are indistinguishable.
  * One AST node (`Expr::IncDec { down, prefix }`) rather than four
    variants, because every consumer but the emitters treats them alike.
  * Kotlin has both operators in both fixities and renders them directly
    [kt-inc-dec]; Rust has neither, so a value-position step becomes a block
    [rs-inc-dec].
* [interp-to-str] An interpolated value must have a **text form**, checked
  rather than left to the backend (user request 2026-09-11; before this a
  non-renderable value reached rustc as "doesn't implement `Display`"
  [backend-never-wrong]).
  * **Native**: the scalars (`Int`, `Long`, `Float`, `Double`, `Bool`,
    `Char`, `Byte`) and `Str`. A union is native when *every* arm is —
    both backends reach the payload (Kotlin through the wrapper's
    `.value`, Rust through the arm accessor). `Mut Str` never reaches the
    question: a builder is converted first [str-drop-mut].
  * **Otherwise a `to_str`**, resolved *at the interpolation site* like an
    implicit parameter (user decision 2026-09-11): a `to_str` in scope
    whose parameter accepts the type and which returns `Str`. The winner is
    recorded in `Checked::interp_to_str` and the emitters call it.
  * **std provides** `intrinsic fn to_str<T>(list: List<T>) -> Str`,
    rendering `[1, 2, 3]`. The format is the *language's*, implemented per
    backend, because the targets' own collection formatting disagrees
    (Rust `Debug` quotes strings, Kotlin's `joinToString` does not).
  * **`params ToStr<T>`** exists as a *convenience* only (user decision
    2026-09-11): declaring `: ToStr<self>` does not enable interpolation —
    a `to_str` in scope does that — it validates at the declaration that
    one exists, which is where the mistake is easier to see.
  * **Known limitation**: a generic `List<T>` cannot be interpolated, since
    an opaque `T` has no text form on either backend. Composing an element
    `?to_str` would fix it and is not built: `resolve_implicit_fn` skips
    candidates that themselves take implicits, and `implicit_args` is keyed
    by *call* spans, which an interpolation does not have.
* [interp-struct] A **struct** with no `to_str` of its own interpolates
  when every field is natively renderable (user request 2026-09-11),
  rendering `Person { name: ann, age: 3 }` — Salvo's struct-literal shape,
  identical on both backends. Deliberately *not* Rust's `Debug` or a
  Kotlin data class's `toString`, which disagree with each other.
  * An explicit `to_str` always wins: the derivation is the fallback.
  * A field that itself needs a `to_str` is **not** followed — the
    derivation is for the simple cases, and the diagnostic asks for a
    `to_str` instead. Generic structs are excluded (their field types would
    need substituting).
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
  * A *written* `Nothing` lowers to the bottom type, not to a nominal type
    that happens to be spelled that way: `throw`'s declared
    `-> Nothing => !message` [throw] means callers see a value that fits
    everywhere and ends the path.
  * **A `Nothing`-typed expression statement terminates its path**, which
    both path analyses read from the checker's recorded types rather than
    syntax: a branch ending in `throw(m)` satisfies [fn-must-return], and
    its consumption never reaches the code after the branch
    [deduce-consume]. Any diverging call qualifies, not just `throw`.
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
    means declaring it — an `intrinsic` in std [backend-intrinsic], or a
    `platform effect` member [platform-effect] in customer code.
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
  (`intrinsic type List<T> canbe Mut`), and applying `Mut` to a type whose
  declaration does not say `canbe Mut` is an error. `Mut` composes with
  every other qualifier (no `with` compatibility needed).
  * Validated at declaration sites (`validate_quals` in the checker)
    against struct and opaque-type `auto_qualifiers`.
  * Backends decide what `Mut` means, as an intrinsic lowering
    [backend-intrinsic]: Kotlin maps `Mut List<T>` to `MutableList<T>` and
    `Mut Str` to `StringBuilder` (through `mut_type_name`), while Rust
    erases `Mut` — mutability lives in the binding (`mut` bindings and
    `&mut` references) instead.
* [str-drop-mut] **Dropping `Mut` may be a conversion.** Every other
  qualifier erases [qual-erasure], so widening is free; `Mut` is the one a
  backend may render as a *different type* [type-canbe-mut], and where it
  does, `Mut T` used as `T` needs a real conversion (Kotlin's
  `StringBuilder` is not a `String` — `MutableList<T>` *is* a `List<T>`,
  which is why this never came up before `Str canbe Mut`).
  * A **checker-recorded coercion** (`Coercion::DropMut`), not an emitter
    guess: the two sides cannot disagree about where a conversion happens
    (the standing checker/emitter agreement invariant). The record carries
    the qualified type, so the backend decides from its base — Kotlin
    renders `.toString()` for `Str` and nothing for `List`, Rust nothing at
    all.
  * It fires at **every** drop site: call arguments (intrinsic lowerings
    included — `char_at(builder, 0)` must not reach a `String` method with
    a builder), returns, `let` annotations, struct fields, union arms,
    string interpolation, and **operator operands**. Operators are included
    rather than rejected: Kotlin's `StringBuilder == String` is `false` and
    `sb1 == sb2` is *reference* equality, against Rust's structural
    `String == String` — a live parity divergence — and rejecting `Mut` at
    operators the way [op-no-none] rejects optionals would surprise, since
    `Mut List` is accepted everywhere else.
  * A drop **keeps** whatever representation change the site would have
    recorded anyway (a union wrap, say) as the record's continuation: one
    expression has one coercion slot, and both have to happen — the drop
    first, since the wrap is about the plain type.
  * Nothing is dropped where the target *keeps* `Mut`: a `Mut T`
    parameter, an optional `Mut T?`, or a generic position (whose pattern
    substitutes to the argument's own type, which is what makes
    `copy(builder)` a builder).
  * `copy` has to know too: on a backend where `Mut T` is its own mutable
    type, the copy of a builder is a *new* builder ([kt-copy] renders
    `StringBuilder(sb)`), never the identity.

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
  `intrinsic type List<T> canbe Mut` [type-canbe-mut], `struct FileHandle
  canbe linear` on a type parameter [linear-generics]) and, for a
  declaration, the `linear struct` modifier [linear-group]; per-type-parameter opt-ins on fns
  (`fn hold<T canbe linear>` [linear-generics]).
  * `canbe` and `with` are unrelated clauses: `canbe` grants a qualifier
    to one declaration ("this may be Mut"), while `with` declares that
    two qualifiers may co-apply to one type ("Old may stack with
    Surname", [qual-with]). Separate keywords (`TokenKind::KwCanbe`),
    accepted at disjoint positions (user decision 2026-09-03).
  * Only the compiler's own qualifiers can be opted into: `Mut`, `Linear`
    and `once` on declarations, `Linear` on type parameters. A user
    qualifier in a `canbe` clause is the `with` confusion above, and is
    rejected as such.
  * `once` joined the list 2026-09-07 (user decision) so a hand-written
    **pass** can declare that driving it uses it up
    ([iter-protocol], [once-fn]) — the alternative, inferring it from the
    presence of a `next`, would attach an obligation to someone's type on
    the strength of a method name.
* [qual-with] Two qualifiers may stack on one type only if one declares
  `with` the other; the `Mut` auto-qualifier composes with everything
  [type-canbe-mut].
  * Pairwise `with` compatibility is validated at declaration sites.
  * One implementation, two callers: the declaration-site check and the
    refinement-conflict rule [qual-refn-conflict] share
    `refine::quals_compatible`, since a conflict is *precisely* "these two
    could not have been written together".
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
  * **A value may be built straight into a group arm** (fixed 2026-09-10):
    `emitted(ok("x"))` where `Emitted (Ok Str | Err Str)` is expected.
    Qualifier lists are flat (`Ty::Qualified`'s base is never itself
    `Qualified`), so applying a qualifier to an already-qualified value
    *appends* — `Qualified { quals: [Emitted, Ok], base: Str }`, which is
    shape-identical to "two qualifiers on a `Str`". Where the expected arm
    is `Q (A | B)`, the value is read the other way round: `Q` comes off
    the value's list and the remainder must fit exactly one arm of the
    inner union (`types::nested_group_arm`). Ambiguity is unresolvable
    from a flat list, so two fitting arms is a no-match, and the arm
    reported instead.
  * **Two wraps, inner first.** A group over a *wrapper* union has an
    inner physical wrapper of its own, so a value entering it is wrapped
    into the inner union's arm before the outer one — carried as
    `Coercion::WrapUnion`'s `inner`, which both emitters apply first.
    This holds whether or not a qualifier is left over: `Emitted (Str |
    Int)` needs it too, and before 2026-09-10 that case type-checked and
    emitted a single wrap, which both target compilers rejected.
  * **The one shape flatness cannot express** is the *same* qualifier
    twice: `quals` is deduplicated, so `ok(ok(x))` is exactly `Ok Str` and
    no rule can recover the nesting. It needs an annotated intermediate
    `let` binding the inner union, and the no-arm diagnostic says so.
* [qual-generic] Qualifiers can be generic, and as generic as their `of`
  type or less (`qualifier Ok<T> of T`, `qualifier Ints of Pair<Int,
  Int>`). Nested qualified types (`Ok (Ok Str | Err Int)`) are legal but
  discouraged.
  * `substitute_vars` re-normalizes `Ty::Qualified` through `qualify()` so
    `Ok T` with `T = Ok Str` never nests `Qualified` in `Qualified`.
  * Constructing one in a single expression works for *distinct*
    qualifiers and not for a repeat of the same one — see [qual-group].
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
    user-declarable: `Mut` [type-canbe-mut], `Linear` [linear-group],
    `once` [once-fn], `proj` [readonly-return] each need a
    representation choice, a flow rule, a non-standard subtyping
    direction, or a restricted position. Vocabulary: users declare
    *state* or *provenance*; the compiler owns *permissions* (droppable,
    like `Mut`) and *obligations* (never droppable, like `Linear` and
    `once`).
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
    over unions; see D4 in ROADMAP.md.
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
* [qual-refn] A **refinement** states what a function *someone else*
  declared does to a qualifier's claim (user design 2026-09-06, roadmap
  D3): `refn add(list: Mut List<T>, elem: T) => list: +NonEmpty, !elem `,
  written in the qualifier that owns the claim or as a top-level item.
  It exists because [deduce-syntax] is sound only by forbidding a
  mutating function from promising a qualifier it never declared — and
  the function is the wrong party to ask, since it has never heard of the
  qualifier.
  * **Narrower than a `fn` by construction**: no body, no effect list, no
    return type (a refinement changes what is *known* after a call, never
    what the call does), and its entries are only additions (`+Q`) and
    removals (`-Q`). A plain (exhaustive) name and `Nothing` are parse
    errors: whether a parameter is kept is the function's own deduction to
    make. The AST carries additions and removals as separate lists rather
    than reusing `Deduction`, so the restriction is structural.
  * **`+Q` is legal only here.** In a *function's* own deduction clause it
    stays rejected (roadmap D2): there it would be a claim about the
    body, which needs an establishment rule; in a refinement it is the
    qualifier author's claim about someone else's call.
  * **Applied after the callee's own list** at each call site
    [deduce-consume]: `add`'s exhaustive `=> list: Mut` drops `NonEmpty`,
    then the refinement puts it back. Only on the *kept* path — nothing is
    known about a moved parameter, and refining one is an error.
  * **Trusted**, like `-> T as Q` [qual-ctor-fn]: no `qualifies` call is
    emitted where a refinement applies, even for a predicate qualifier.
  * **State qualifiers only** [qual-subject]. Provenance cannot be
    invalidated, so there is nothing to re-establish; the compiler's own
    qualifiers carry representation choices and flow rules (`Mut` is not
    even erased), so a refinement may not hand one out. This is also what
    makes the whole feature invisible to the backends.
  * **A qualifier may only refine its own claim.** `NonEmpty` cannot say
    what a call does to `Sorted`. That is what makes [qual-refn-scope]'s
    opt-in honest, and it is why conflicts reduce to "two qualifiers that
    cannot co-apply" [qual-refn-conflict].
  * A refinement's qualifier must apply to the parameter's type
    [qual-of], and each entry must name a parameter of the resolved
    overload, once.
  * Effect members are **not** refinable (deferred, user decision
    2026-09-06): a member has no `FnKey` and naming one needs an
    effect-qualified form.
* [qual-refn-match] A refinement's parameter list picks **one** overload:
  it must repeat that overload's parameters exactly — same names, same
  variadic/implicit flags, same types, with type parameters matched by
  **position** (the refinement's own, preceded by the qualifier's). Zero
  or several matches is an error at the refinement, so a typo cannot
  become a refinement that silently never fires. A name in the parameter
  list that is neither a type parameter nor a visible type is reported as
  such, naming the type-parameter remedy — a top-level `refn` has no
  qualifier to borrow `T` from.
* [qual-refn-scope] A refinement declared **inside a qualifier** applies
  wherever that qualifier is in scope, and nowhere else: the user opts
  into the refinements by opting into the qualifier (user decision
  2026-09-06). A **top-level** `refn` is *module*-scoped and **not
  importable** — reconciling conflicting refinements is the consumer's
  call, and a library shipping its own reconciliation would move the
  conflict one level up.
* [qual-refn-conflict] When the refinements applying to one (callee,
  parameter) **disagree**, none of them apply (user decision
  2026-09-06). Two additions disagree when the qualifiers could not have
  been written together [qual-with]; an addition and a removal of the
  same qualifier disagree outright.
  * **Not an error**: the program compiles and the function is simply
    less useful. But a **warning** is reported at the call site, once per
    (callee, parameter) — silence would make an imported refinement's
    doing nothing undiagnosable. The remedies it names are testing the
    property with `is` (always available, since these are state claims)
    and [qual-refn-reconcile].
    * "Compiles" is enforced, not merely intended: each backend's
      emission gate aborts on *errors only*, so a warning cannot stop
      codegen.
    * And it is *reported* on that path, not only by `salvo analyze` and
      the language server: `Backend::emit` returns `Emitted { files,
      warnings }`, the driver prints the warnings and carries on, and each
      emitter has `emit_program_reporting` next to the warning-dropping
      `emit_program` the golden tests use. Tested per backend, since both
      the gate and the channel are duplicated in each.
    * `salvo platform generate` deliberately does not report them: it
      writes host stubs once, and the program's diagnostics belong to the
      compile path.
  * Granularity is per **parameter**: a disagreement about one parameter
    does not cost the refinements of another.
  * Judged in the *calling* file's scope, since that is where the
    refinements are visible. A qualifier that file cannot see is assumed
    compatible rather than suppressing on missing information.
  * A single addition is also skipped when it could not co-apply with a
    qualifier the call *preserved*: the value cannot carry both claims,
    and knowing less is the safe direction.
* [qual-refn-reconcile] A top-level `refn` **replaces** the qualifiers'
  own refinements for the parameters it names, rather than joining them —
  which is what makes reconciling a conflict possible at all. Same
  precedence own-module declarations have over imported ones
  [mod-collision].
* [qual-refn-infer] Refinements reach **inferred** deductions
  [deduce-infer], so the fact survives one frame outward: a fn whose
  parameter declares the qualifier and whose body makes a refined call
  may promise it back, and a *written* list promising it validates against
  the same body facts. Two limits keep this sound and keep D2 deferred:
  * An addition contributes only for a qualifier the parameter itself
    **declares** — it can cancel a removal, never invent a claim. Adding
    one a signature never made is `+Q` in a fn's own list, which is D2.
  * An addition is honored only when the refined call is **unconditional**
    in the body (not inside an `if`/`when` branch, a loop body, a lambda
    or a `try`). The deduction walk is a meet over all
    uses rather than a flow analysis, so a call that may not run cannot
    establish a fact the signature then promises. Removals are unaffected:
    applying one unconditionally is the conservative direction. The
    *call site* remains flow-sensitive and does narrow inside the branch.
* [qual-refn-docs] A refinement's doc comment [doc-comment] is merged into
  the refined function's documentation, in a **Refinements** section
  listing the effective entry, where it came from, and — for a suppressed
  group — the conflict. A refinement is written somewhere else entirely,
  so this is the only place a reader of the call can find it. Rendered
  against the scope of the *file the cursor is in*, since that decides
  which refinements apply.

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
* [cond-bool] Conditions are boolean expressions: `if`/`elif`, `while`,
  and a subject-less `when`'s branch heads [when-condition] accept `Bool`
  and nothing else. There is no truthiness — no rule could turn an `Int`,
  a `Str` or a possibly-absent `Bool?` into a decision, and guessing one
  is how a backend divergence gets in (Kotlin has no truthiness either;
  Rust would reject the `Option`). `is`/`^` checks evaluate to `Bool`.
  * Checked per **leaf** of a `&&`/`||`/`!` condition, which is where the
    wrong type was written; `Unknown` and `Nothing` stay lenient
    [type-unknown-lenient].
  * Remedy named in the diagnostic: compare explicitly (`n != 0`,
    `list.size() > 0`), assert with `!`, or test the type with `is`.
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
* [is-narrow-guard] A narrowing **survives a guard** (user decision
  2026-09-09): when every branch of an `if` leaves the block, the code
  after it is on the else-path, so each condition's else-narrows hold
  there — the facts [is-narrowing] gives an `else` branch, carried past a
  statement whose branches cannot fall through.
  ```
  if e is None {
      return finished()
  }
  return emitted(e)        // `e` is the element type here
  ```
  * "Leaves the block" is `return`, `break`, `continue`, or a diverging
    call ([type-any-nothing]) — the same predicate [fn-must-return] uses,
    plus the loop exits. One branch exiting is not enough: if any branch
    can fall through, either path may have been taken and nothing is
    narrowed.
  * An `elif` chain accumulates, so `if v is Int { return } elif v is Str
    { return }` leaves the third arm.
  * [narrow-assign-reset] still applies on the *surviving* path only: an
    assignment inside a branch that exits cannot be observed after the
    `if`, so it resets nothing there (the same reason the fall-through
    merge ignores that branch).
  * Rationale: guarding the empty case and then using the value is how a
    `next` over a container is written, and requiring an `else` block or a
    two-armed `when` for it was a limitation of the analysis, not a rule
    anybody chose.
* [is-binding] `is Type name` binds the narrowed value to a fresh
  variable in the matched branch (and per-iteration in `while`).
  * Parse heuristic: uppercase idents in the check are type refs; a
    trailing lowercase ident is the binding.
* [is-precise] Checks may include qualifiers and generics:
  `is Err Str` matches only the `Err Str` arm; `is Err` matches every
  `Err`-qualified arm; overlapping matches infer the smaller union.
* [qual-widen] `expr ^ Qual...` is the **dual of `is`** (user decision
  2026-09-05): boolean-valued, same places, same runtime test — but where a
  successful `is` *narrows* the subject (a qualifier added, an arm picked), a
  successful `^` **generalizes** it, removing the listed qualifiers for the
  branch. `list ^ Mut` reads `list` without `Mut`; `outcome ^ Ok` without
  `Ok`.
  * A `when` **branch head** may be `^ Qual...` as well as `is ...`: the
    same arm test, with the subject reading widened inside the branch. That
    is the form the qualified-union case wants — `when o { ^ Ok { when o {
    … } } }` reaches the union inside `Ok (Ok Int | Err Str)` with no
    intermediate binding and no repeated type.
  * The qualifiers must be **present**: nothing to remove is an error, not a
    silently-false test (`^` is not a predicate test — that is `is`).
    Several may be removed at once (`v ^ Mut NonEmpty`), and the right side
    is qualifier names only: a *type* there is an `is` question.
  * **Droppability comes from one list** — `types::qual_drop_block`, which
    `Qual T <: T` also reads, so the two cannot drift as intrinsic
    qualifiers are added. `once` (restricts rather than refines), `Linear`
    (carries a use obligation) and `proj` (the value is derived from
    another) may never be dropped; every other qualifier may, since dropping
    a *claim* only loses knowledge and dropping a *permission* only loses
    permission.
  * No binding form: the subject itself reads widened, so a second spelling
    would be redundant.
  * Widening more than one arm at a time is an error: each arm would peel a
    different wrapper position, so one widened view cannot stand for all.
  * Backends: the runtime test is `is`'s (qualifiers are erased, so widening
    is a *typing* act), and where the check peels a wrapper arm the widened
    value is bound to a **shadowing local** for the branch — see
    [rs-widen-shadow] / [kt-widen-shadow]. Without that materialization a
    nested `when` would scrutinize the wrapper it came out of, which is
    exactly the wrong-code bug the feature's own demo caught while it was
    being built.
* [when-union-subject] `when` **with a subject** requires a union-typed
  *variable* subject, and takes no `else`: the arms are the cases and
  covering them is what is checked [when-exhaustive]. An `else` here is a
  parse error naming the subject-less form [when-condition]. Field
  subjects stay rejected even though
  they now narrow (user decision 2026-09-03): `if … is` covers them.
  * A **qualified union** (`Ok (A | B)`, a `Thrown (Str | Int)` message
    [try]) is a claim *about* a union, so its arms belong to the inner
    type. Reach them with a `^` branch head ([qual-widen]:
    `when v { ^ Ok { when v { … } } }`), or bind at the inner type
    (`let inner: A | B = value`). The droppable-qualifier rule does the
    unwrapping ([qual-erasure]: `Qual T <: T`), which is also why the
    intrinsic capability qualifiers need no special case — `once` is
    excluded from dropping and `Linear` is never written at a use site.
    The diagnostic names this remedy. Considered and rejected
    (2026-09-04): merging nested qualifiers (`Ok Err Str` collides with
    multi-qualifier types, which mean a *set* of claims) and a dedicated
    unwrap keyword (it would re-implement the exclusion list the subtype
    rule already has).
* [when-exhaustive] `when` must be exhaustive over the subject's arms;
  arms are consumed sequentially (each branch matches what previous
  branches left), and a branch that can match nothing is an error. A
  `None` arm is handled via the subject's nullability.
* [when-condition] `when` is **always exhaustive**, and a *subject-less*
  `when` is the second way it can be: `when { cond { … } … else { … } }`,
  a condition chain whose `else` is **mandatory** (user decision
  2026-09-05). `when {` is unambiguous — a subject is always a plain
  variable, so a brace after `when` cannot be one.
  * Semantics are `if`/`elif`/`else`'s, and the checker takes the same
    path: bare boolean branch heads [cond-bool], narrowing per branch with
    the earlier conditions accumulated as exclusions [is-narrowing],
    branch values unioning into the result [when-value], and
    definitely-returning when every branch returns [fn-must-return].
  * **The mandatory `else` is the whole point.** An `if` chain without one
    folds `None` into its value [if-else-none], so a total chain of
    conditions is a shape the reader has to verify; this form is the shape
    the grammar guarantees. Nothing else is new — which is why it reuses
    `check_if` rather than getting its own checking.
  * **Branch heads are ordinary conditions**, so `is`/`^` work in them and
    narrow their branch (user decision 2026-09-05). No arm-exhaustiveness
    follows: `is` heads that happen to cover a union still need the
    `else` — the subject form is how you ask for that check.
  * Parse errors, each naming the remedy: a missing `else` (the form is
    not exhaustive without it — use `if`/`elif` when there is no
    fall-back), an `else`-only `when` (nothing to decide — write the
    block), a branch after the `else` (it closes the chain), and an `else`
    in the *subject* form, which names this form instead of reporting a
    missing `is` [when-union-subject].
  * Backends: Kotlin has the same construct [kt-when-cond]; Rust has no
    subject-less `match` and lowers to `if`/`else if`/`else`
    [rs-when-cond].
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
* [for-iter] `for x in e` drives `e` when it is a pass [iter-protocol] and
  otherwise iterates `iter(e)` implicitly [iter-pass]; missing or ambiguous
  `iter` resolution is an error [iter-resolve].

## Functions

* [fn-syntax] `fn name<G>(params) [effects] -> ReturnType`; => deductions
  no implicit returns from functions (unlike blocks); omitted return type
  means `None`; omitted effect list means pure (`[]`).
* [fn-return-none] Functions returning `None` may `return` bare or not
  return at all.
  * Exception: a *bodyless* declaration must write `-> None` explicitly
    [decl-explicit].
* [decl-explicit] Nothing the compiler cannot see is inferred (user
  decision 2026-09-03). A fn with **no body** — an `intrinsic fn` — must
  declare its **effect list**, **deduction clause** (every parameter but
  Copy scalars [deduce-syntax]) and **return type**; an *effect member*
  must declare its deduction clause and return type (it may not declare
  effects at all
  [effect-member-no-effects]). Inference from an absent body is a guess,
  and always the most permissive one: it is how std's `add` came to be
  inferred as *keeping* the element the list had taken ownership of, so
  `add(xs, h)` then reading `h` compiled on Kotlin and was rejected by
  rustc.
  * The declaration is then a real contract: an effect member's deductions
    are applied at call sites exactly like a named call's, so a member
    that takes ownership consumes its argument. Validating each *handler*
    body against the member's contract is future work (roadmap E1).
  * An `intrinsic fn`'s declaration is **trusted**, not second-guessed:
    the checker does not infer mutation from a `Mut` parameter and
    override it — the written signature is the contract the backends lower
    against.
* [decl-body] A top-level `fn` with **no body**, and a `type` with neither
  an `= alias` nor the `intrinsic` modifier, are parse errors (in
  `parse_item`): both were the shape of the now-removed bodiless
  declaration form, and there is nothing left for such a declaration to
  mean. Each error names the surviving forms — write the body, or (if the
  target language provides it) declare a `platform effect` member
  [platform-effect]; give the type a definition (`type N = ...`), or
  declare it `intrinsic`. `intrinsic` declarations parse through their own
  modifier branch, so they are the only bodiless `fn`/`type` forms left,
  and only std may write them [intrinsic-std-only].
* [fn-must-return] A fn with a non-`None` return type must return on
  every path. Definitely-returning constructs: `return`, a **diverging
  expression** ([type-any-nothing]: a statement the checker typed
  `Nothing`, e.g. `throw(m)`), `if` with an `else` where every branch
  returns, `when` where every branch returns (exhaustiveness is enforced
  separately [when-exhaustive]).
  * Conservative by design: loops never count as returning (they may run
    zero times).
  * Yield-based iterator fns are exempt — their body produces elements,
    not a return value.
* [fn-overload] Functions overload by parameter types (including
  qualifiers: `full_name(Person)` vs `full_name(Surname Person)`). One rule
  decides every call, in three steps — `@module`, then the most specific
  *scope* [fn-overload-scope], then the most specific *signature*
  [fn-overload-rank] — and no single winner is an error
  [fn-overload-ambiguous]. Nothing else takes part: not the return type, not
  effects, not deductions, not implicit parameters.
  * The checker records the winner per call site (`call_fn`). In unchecked
    contexts (no recorded winner) emitters narrow same-arity candidates by
    the checked argument types' base names; an ambiguous dispatch is a
    codegen error ("annotate the argument types"), never a guess
    [backend-never-wrong]. That fallback is a *subset* of the real rule: it
    may only ever error where the checker would have chosen.
  * Generic bindings in `unify` widen: when arguments bind the same `T`
    to related types, the more general one wins regardless of order
    (`pick(1, maybe_int)` binds `T = Int?`). No occurs check,
    deliberately: `Ty::Var` identity is name-scoped per side, so a
    callee's `T` never appears inside argument types (a caller's
    same-named `T` is a different variable).
* [fn-overload-scope] **The most specific scope that fits wins** (user
  decision 2026-09-07). Functions arrive from ever more specific places —
  `core`, then this file's explicit imports, then this module, then the fn's
  own scope (fn-typed parameters, locals, implicit parameters, effect
  members), then inner scopes — and only the most specific rung with a
  candidate *fitting the arguments* competes.
  * So an own-module `size(List<T>)` means *this* module's for calls in it,
    while `size("text")` still reaches core's — the shadowing overload does
    not fit, so it never competes. Before this rule, fns merged into one
    flat overload set and declaration order handed the call to std, silently.
  * The rungs above `Own` are not overload sets: a fn-typed local, parameter
    or implicit *is* the function the caller chose and shadows the name
    outright, an effect member takes the name before any fn does
    [effect-member-unique], and a rename introduces a fresh name
    [fn-rename]. `FnEntry::rung` therefore has three values (`Core`,
    `Import`, `Own`).
  * **Scope beats signature**, deliberately: the alternative is a rule no
    reader can predict without knowing std's surface. When it discards a
    *more specific signature* from a lower rung the call gets a
    `Severity::Warning` naming both candidates and both `@module` forms —
    writing either silences it, since an explicit selector is the
    confirmation.
* [fn-overload-rank] **Then the most specific signature**, compared **per
  argument slot** (`types::spec_cmp`, `rank_cmp`):
  1. a **type variable** says the least, structurally (`List<Int>` beats
     `List<T>`);
  2. **union arms compare as sets**: fewer arms says more, so `Int` beats
     `Int | Str` beats `Int | Str | Bool`, and `Int` beats `Int?`. `Any` is
     the broadest type, so it is always least specific — which is why the
     written `Any` lowers to `Ty::Any` rather than a nominal type
     [type-any-nothing];
  3. **qualifier sets compare by inclusion**: more qualifiers says more, and
     the *kind* never ranks (`Mut List<T>` and `NonEmpty List<T>` are
     unrankable, deliberately — ranking them would ask the caller to know
     more than what is in front of them);
  4. with the slots otherwise equal, a **fixed** parameter list beats a
     variadic one, which is what lets `list()` pick a no-argument overload
     over `list(...elems)`.
  * A candidate wins only by being at least as specific in *every* slot and
    strictly more specific in one. A sum of per-slot scores was the previous
    rule and is deliberately gone: it let one argument's gain pay for
    another's loss, which is a guess.
  * Criteria pulling in opposite directions within one slot (a more specific
    base with a smaller qualifier set) are unrankable, for the same reason.
  * Specificity never exceeds what the caller knows: an `Int | Str` value
    does not *fit* `f(Int)`, and after narrowing it does — that is
    subtyping, not a ranking rule.
  * The ranking also decides which candidate **leads**: expected types flow
    into the arguments from the most specific candidate *still compatible
    with the arguments typed so far*, on the most specific rung. That is what
    gives a bare lambda an expected type when the name is overloaded
    (`map(xs, n -> n * 2)` with a `List` fast path beside the generic
    overload), and the per-argument re-narrowing is what keeps the subject
    deciding: `map(arr, n -> n + 1)` drops the `List` candidate when `arr`
    turns out to be an array, before the lambda is typed. With no dominant
    candidate the arguments keep the untyped probe, and the lead is only ever
    a *hint* — the selection re-derives everything from the argument types it
    ends up with.
* [fn-overload-ambiguous] **No single most specific candidate is an error**,
  never a pick. The diagnostic names the candidates and the remedies:
  narrowing an argument, or `rename fn <new> = f(...)`.
  * [type-unknown-lenient] An un-inferred argument fits every candidate, so
    it suppresses the ambiguity: one mistake, one diagnostic. The winner in
    that state is unspecified.
* [fn-overload-at] **`f@module(args)`** names the module whose overload is
  meant, overriding scope precedence: `size@core.list(xs)`,
  `size@main(xs)`, and `xs.size@core.list()` in dot form (the selector
  attaches to the *name*). Also valid as a value (`describe@main`).
  * A module path, not a rung keyword: `@mod`/`@import` would ask the reader
    to know which rung a name came in on (user decision 2026-09-07).
  * Naming a module with no *fitting* overload is an error listing the
    modules that have one — never a silent fallback.
  * It is the way out of a **shadowed** name: a local of the same name hides
    every function, and `@` is what reaches one anyway.
  * On a renamed name it is an error: a rename already names one
    declaration.
* [fn-rename] **`rename fn add2 = add(a: Int | Str, b: Int | Str)`** gives
  one overload a name of its own (user decision 2026-09-07), which is how an
  ambiguity the ranking cannot settle is settled.
  * **Not an alias**: from that point the overload answers *only* to the new
    name — it leaves the old name's candidate set, in calls, in fn values
    and in implicit resolution. That is what makes the old name unambiguous
    again.
  * The parameter list repeats one overload's parameters exactly (same
    names, same types, type parameters positional), matched by the same
    matcher `refn` uses [qual-refn-match] — so a std rename surfaces as a
    diagnostic rather than as a rename that quietly stops applying. Effects,
    a deduction clause, a return type and an implicit-group spread are parse
    errors naming the reason: none of them takes part in selection.
  * **Scoped**: at module level it applies to the whole module (in every
    file of it), order-independent like any module-level declaration; inside
    a fn or a block it applies from its line to the end of that scope, loops
    and lambdas included. Not importable — taking an overload out of a
    shared name is the consumer's decision [qual-refn-scope]'s reasoning.
  * The new name must be otherwise unused (a rename removes an ambiguity, so
    adding one would defeat it), and it is **erased**: `renamed_calls` tells
    the emitters to spell the declaration's own name, where an import alias
    keeps the alias.
* [fn-overload-duplicate] Two declarations of one name in one module with
  the same parameter **types** are a duplicate, not an overload set —
  reported at the second one. Parameter names and return types take no part
  in selection, so nothing at a call site could tell them apart.
* [fn-value-select] A function passed **by name** is selected by the
  **expected fn type**, through the same three steps a call uses (parameters
  contravariant, result covariant, contract included). With no expectation a
  single candidate is taken and an overloaded name is an error naming both
  remedies (annotate the position, or `rename`). When candidates exist but
  none fits, the diagnostic says so and explains why — a contract difference
  does not show in a printed type [fn-contract].
* [effect-member-call] An effect member call is checked against its declared
  parameters like any other call — arity and types. Members do not overload
  [effect-member-unique], so there is nothing to select and nothing to rank;
  these are plain mismatch diagnostics. (Until 2026-09-07 the member's
  parameter types only flowed in as *expected* types, so `log(true)` on
  `fn log(message: Str)` was accepted.)
* [call-type-args] A generic call's type arguments must be **determined**.
  In order: an explicit list (`mutable_list<Int>()`) pins them; otherwise
  the arguments bind them by unification; otherwise the **expected type**
  at the call does — a `let` annotation, the enclosing fn's return type, or
  a parameter type the result flows into. A type argument that appears in
  the *result* type and that none of these determine is an error naming
  both remedies (user decision 2026-09-04).
  * *Why an error rather than leniency*: an undetermined argument used to
    leave `Ty::Unknown` in the result, which swallowed later mistakes
    (`add(xs, 1)` then `add(xs, "two")` on the same list, both accepted)
    and pushed the problem onto the backends — where one target language
    infers what the other cannot (`mutableListOf()` is not valid Kotlin;
    rustc infers `vec![]` from a *later* use). Salvo does not look forward:
    what the compiler knows must be visible at the call.
  * A type argument reaching only the *parameters* needs no context —
    nothing downstream observes it — and an argument of unknown type keeps
    the call lenient, so one mistake still yields one diagnostic
    [type-unknown-lenient].
  * A *concrete* parameter type flows into a nested call as its expected
    type; a pattern still mentioning the callee's own generics does not
    (it would coerce against an unsubstituted `T`).
  * The resolved bindings are recorded per call (`call_type_args`) in the
    callee's declaration order, which the backends' intrinsic lowerings
    consume — e.g. the element type in Kotlin's `mutableListOf<Int>()`
    [backend-intrinsic].
* [call-generic-progressive] A callee's type variables bind **progressively,
  left to right**: each argument's expected type is its parameter pattern
  with everything the earlier arguments — and any explicit type-argument
  list — already determined substituted in. That is what types an
  un-annotated lambda from its *siblings*: in `map(xs.iter(), n -> n * 2)`
  the first argument binds `T = Int`, so the lambda is checked against
  `(Int) -> U` and its body determines `U`. The same rule effect member
  generics already state [effect-member-generics].
  * Not merely convenience: checking the lambda against an *unsubstituted*
    pattern gives it a type that mentions the callee's own variable
    (`(T) -> T`), which is exactly what `unify`'s deliberate lack of an
    occurs check assumes cannot happen [fn-overload] — so the call failed
    to match itself, and even an explicit type-argument list did not help.
  * Left to right, and no further: a lambda written *before* the argument
    that would bind its parameter type is not inferred (annotate the
    parameter). Salvo does not look forward [call-type-args], and a
    fixed-point pass over arguments would reorder the fate events a call
    records [deduce-consume].
  * The bindings are a *hint* for expected types only; the candidate scoring
    below re-derives them from the argument types it ends up with.
* [fn-dot] Dot-notation: `x.f(a)` ≡ `f(x, a)` whenever `f` resolves to a
  declared fn or effect member. There is no method-call fallback: an
  undeclared name is an unresolved call ([call-resolve]), and the
  diagnostic names a `platform effect` member [platform-effect] as the way
  to reach a target-language method.
* [ident-resolve] Every **identifier reference** must resolve to something
  declared — a local, a parameter, a handler, or a fn passed by name.
  Nothing else is a value, so the reference is an error (user request
  2026-09-11; it used to be typed `Ty::Unknown` and reach the emitters,
  which spelled the name verbatim, so rustc reported `E0425` and kotlinc
  "unresolved reference" [backend-never-wrong]). Scopes are not hoisted:
  reading a variable above its declaration is the same error.
  * A name that *is* declared but is not a value says what it is instead —
    a struct type, an effect, a qualifier, a `params` group, a type — the
    same courtesy [effect-not-a-type] already extends.
  * The unresolved case carries import suggestions [diag-import-suggest].
* [unused-var] A local that is never **read** is a *warning* (user request
  2026-09-11). Assignment is not a read: a variable only ever written to has
  no reader, which is the mistake worth reporting. A leading `_` opts out
  (`_spare`). Parameters are exempt — a signature often dictates them, and
  an effect or handler member implementing a declared interface cannot drop
  one — as are handler state fields and everything in std.
  * A *warning*, not an error: the program is still well-defined. It is the
    first diagnostic Salvo emits routinely at that severity, so anything
    reading `Checked::errors` must filter by severity to mean "errors".
* [call-resolve] Every call must resolve to something declared: a fn, an
  effect member, a handler constructor, or a value
  of fn type. Otherwise it is an error (user decision 2026-09-03) —
  unresolved names carry import suggestions [diag-import-suggest].
  * Calling a value whose type is known and is not a fn type is an error
    too ("`n` is not callable: its type is `Int`"), generics included: a
    type parameter has no bounds, so nothing makes a `T` callable.
  * Only an *un-inferred* callee stays silent [type-unknown-lenient].
* [field-resolve] Only structs have fields, and only the ones they declare
  (predicate-qualifier overrides refine them [qual-field-override]).
  A field on any other known type — an `intrinsic type`, an array, a fn
  value, a generic `T` — is an error; a target-language member is reached
  through a declared accessor (an `intrinsic fn`, or a `platform effect`
  member in customer code).
* [index-resolve] `[]` subscripts arrays only. Other collections expose
  element access as declared functions (std's `get(list, index)`), and
  tuples use constant positions ([expr-tuple-index]).
* [iter-resolve] A `for` subject must be an array, a **pass** — a type
  declaring `: Yield<self, T>` [iter-protocol] [group-obligation] — or a value
  some declared `iter` overload accepts (the implicit `iter(subject)` call);
  anything else is an error. When the subject has a protocol-shaped `next` but
  no declaration, the error names the `: Yield<self, T>` remedy.
  * Resolution order: arrays natively, then the declared pass
    [iter-protocol], then `iter` [iter-pass]. The pass comes before `iter`
    because a type with both is *already* a position in a sequence, so minting
    a second pass from it would be wrong.
* [iter-generic-drive] A `for` over a **type parameter** drives it when the
  enclosing fn has a protocol-shaped `?Yield<It, T>` spread for it
  [implicit-group] (user decision 2026-09-09): the position *is* the declaration
  — it says "this call supplies a `next` for `It`" — so the loop calls that
  implicit parameter and takes the element type from its result. This is what
  lets std's own combinators be written with `for`, and any combinator of one's
  own with them.
  * **By name, not by shape**: the implicit must be called `next`, which is what
    `for` drives everywhere else [iter-protocol]; and it must take its state as
    `Mut It`, for the same reason a declared `next` does (the same diagnostic
    fires when it does not).
  * **The element is owned.** `next` hands the element over by value, so the
    loop binding is *not* a projection of the pass and may be moved on — which
    is what lets a combinator put each element in its output. (A native
    container loop is the other case: there the binding really is a projection
    and it links [fate-link].)
  * Without the spread there is nothing to drive with, and the subject reports
    the ordinary not-iterable error [iter-resolve]: a bare type parameter says
    nothing, which is [call-resolve]'s rule for generics.
* [iter-drive-in-place] Any **named place** is driven *where it lives*
  (user decision 2026-09-12, extending the 2026-09-09 kept-parameter rule
  to locals and owned parameters — and deleting the loop's implicit
  release entirely):
  * A pass named by a variable or parameter is advanced in place: the
    position the loop reaches is what the owner sees next, the loop counts
    as a **mutation** rather than a move, driving an exhausted pass again
    is legal (zero iterations), and the discharge stays the owner's —
    a linear pass is bound with `let`, driven, and explicitly discharged
    after the loop and before early exits (the ordinary all-paths
    analysis enforces it) [linear-group].
  * Only a **temporary** subject (a minted pass, a call result) is
    consumed by the loop — and a *linear* temporary is refused there
    ("a `for` cannot consume a linear pass"): the loop never discharges
    what it drives, so an owned linear pass has to be a named place.
  * The in-place rule exists because the two backends disagreed without
    it: Rust bound the subject into a local — a *clone*, for a kept
    parameter's `&mut` — so the caller never saw the position the loop
    reached, while Kotlin aliased it and did [backend-parity].
  * [linear-generics] A **generic** pass under `canbe linear` follows the
    same rules: drive it in place and let the owner discharge, or — for a
    fn that owns the pass — take a **consuming callback**
    (`end: (x: It) -> None` with `=>[end] !x`) and hand the pass to it
    after the loop; callers pass the type's own discharger for a linear
    pass and std's `drop` for a plain one [linear-discard]. (This
    replaces the deleted `?Linear<It>` spread.)
* [iter-pass] `iter` converts a **container** into a fresh pass
  (`fn iter<T>(list: List<T>) [] -> Mut ListYield<T>`), and that is the whole => !list
  of container iteration: std declares a pass struct plus a `next` per
  intrinsic container, so the language has no container protocol of its own
  (user decision 2026-09-08, roadmap R5).
  * The container is **borrowed by** the pass (a `proj` field
    [proj-field]; user decision 2026-09-11 — until then it was moved in):
    `iter(xs)` keeps `xs` usable and links the pass to it, so walking the
    same container twice is `iter(xs)` twice, and mutating `xs` while a pass
    over it lives is refused [proj-infer].
  * A `for` over a container is lowered as *mint then drive*: the checker
    records the `iter` to call beside the `next` to drive, and **both emitters
    call it** — until 2026-09-09 the record was read by nobody, so a `for` over a
    container of one's own emitted a drive of the container itself, which the
    target compiler rejected (found while building [iter-fn], whose generated
    `iter` walks the same path).
* [iter-for-native] A `for` over an **intrinsic container** — a list, an array,
  a `Str` — records no driver at all: the backends iterate their own data
  natively, which neither allocates a pass nor consumes the subject. The
  language gets no special case (the rule is "intrinsic container", not a name
  list); the fast path is the emitters'.
* [iter-protocol] The pull iteration protocol is declared in std
  (`std/core/iterator.sv`), not built into the compiler: a **pass** is a
  value some `next` accepts, and `next` reports
  `Emitted T | Finished` (user decision 2026-09-07; the names were
  `Next`/`Stopped` in the design). This is the manual half of the iterator
  story — `zip`, `merge`, anything reading two sources at once — which
  a step function expresses directly.
  * `Emitted` is a *qualifier* (`qualifier Emitted<T> of T`) so the element
    keeps its own type, which is also what keeps the end of a sequence of
    optionals distinguishable: `Emitted None | Finished` has two arms where
    `None | None` would have one. `Finished` is a fieldless struct — it has
    nothing to qualify, and `None` would say "absent" where the claim is
    "the sequence ended".
  * `params Yield<It, T> { fn next(it: Mut It) -> Emitted T | => it: Mut
    Finished }` [group-obligation] [group-self]: a type of your own becomes
    drivable by declaring `: Yield<self, T>` and supplying the `next` — checked at
    the struct, where a misspelled member is caught, rather than surfacing
    as "not iterable" at some loop (roadmap R2, user decisions 2026-09-08;
    this replaced `params Iterator<St, T>` and the `once` requirement). The
    state is `Mut` because advancing a pass mutates its position.
  * Only the exact `Emitted T | Finished` shape is a driver; a `next` of
    any other shape is an ordinary function.
  * **`for` reads the declaration** — the `: Yield<self, T>` clause is the one
    fact that makes a value a pass, and the element type is the clause's
    argument. The overload scan only resolves *which* `next` (and the arm
    identity); when the obligation is declared but unsatisfied, the error
    has already landed at the struct and the loop stays lenient, answering
    the declared element type [type-unknown-lenient]. A matching `next`
    without the clause is not a pass ([iter-resolve] names the remedy) —
    the tie is declared, never inferred from a method name.
  * Driving consumes the subject: it is moved into the loop, exactly as the
    `once` passes it replaced were. Drive-in-place (`Mut` borrow — "a
    second drive continues") is recorded as the eventual semantics and
    deferred with `once`-on-producers\' deletion (R5).
  * `next` takes its state as `Mut St`: advancing a pass mutates its
    position, and the backends pass a mutable place. A `next` with the right
    *result* shape and a non-`Mut` state is an error saying so, rather than
    a puzzling "not iterable".
  * **What the checker hands over**: `Checked::for_drivers`, keyed by the
    subject's span — the overload *and* the arm identity (`PassDriver`).
    The driving loop is synthesized, so there is no call node for the
    emitters to resolve, and having each of them re-derive the arm index
    from the declaration is exactly the checker/emitter disagreement the
    invariants forbid.
  * **Lowering** (both backends, phase I2c): the subject is moved into a
    local — driving consumes a pass, so nothing else is looking at
    it — and each turn calls `next` on a mutable place.
    * Rust: `while let Union2::U1(mut n) = next(&mut __loop1_pass) {`. A
      `while let` re-evaluates its condition per turn, so `Finished` needs
      no arm of its own.
    * Kotlin: `while (true)` plus `if (step !is U2_1<…>) { break }`, since
      Kotlin has no pattern-matching loop condition. The arm is spelled with
      its *real* type arguments when `next` is non-generic, which keeps the
      element read free of an unchecked cast (star projection leaves `value`
      at `Any?`); a generic `next` has type arguments the loop cannot see —
      no call node — and falls back to stars plus a cast, and the emitted
      function carries `@Suppress("UNCHECKED_CAST")` so generated code stays
      warning-free [kt-suppress-cast].
  * An **effectful** `next` is a codegen error for now: its handlers would
    have to be threaded into every turn of the loop, which is phase I4
    [backend-never-wrong].
* [seq-pass] std's sequence functions (`map`, `filter`, `reduce`) take their
  subject as a **pass** and reach its `next` through a `?Yield<It, T>` spread
  [implicit-group] — the group std declares for iteration — so any pass is a
  subject: the one an `iter` hands back for a `List<T>`, an array or a `Str`,
  the one an `iter fn` generates, or a pass type of one's own. A container is
  iterated by *writing* its `iter` (`map(iter(xs), f)`), which is what keeps
  the inference ordinary: `It` is bound by an argument, so nothing depends on
  feeding one implicit's resolution into another (user decision 2026-09-09,
  replacing `?Iterable`). There is no `Iterable` group and no iterator type.
  * **std relies on the pass and never on an `iter`** (user decision
    2026-09-10), and the reason is stronger than the inference one: a source is
    not guaranteed to *have* a container behind it. An `iter fn`'s pass, a
    composed pass someone wrote by hand, a hand-written `zip` — for each of those the
    pass is all there is, so a std function that asked for an `iter` would
    exclude them by construction. A *program* may still write a
    container-shaped combinator (`?iter` as an implicit, whose result determines
    the pass type [implicit-infer]); std may not.
    * The `List` fast paths are not an exception: they are overloads on a
      concrete intrinsic type, lowered to the target's own collection
      operations, and they ask for no `iter` [fn-overload-rank].
  * **Eager, with one named variant** (user decisions 2026-09-08, 2026-09-10).
    `map`/`filter`/`reduce` return `Mut List<U>`; the default is the one that
    surprises least, and chaining works because a list has an `iter`.
    * **Nothing in std is lazy.** `map_lazy`/`filter_lazy` — composed passes
      that computed as they were driven — were **removed 2026-09-10** (user
      decision): laziness as a data structure couples the data to the functions
      over it, and the direction to try instead is composing *functions*,
      `iter fn`s included, into pipelines that mint a pass from data supplied
      separately. Reconsidered after concurrency lands; see ROADMAP.md. A
      composed pass remains ordinary code for a program to write — a pass is
      only a struct with a `next` [iter-protocol] — and [iter-mut-param] is
      still why one carrying mutable state is refused.
    * [seq-into] `map_to`/`filter_to` put the results in a collection the
      caller provides, passed **first** because it is what the call is about.
      Appending goes through an `?add` implicit parameter
      (`(dest: Mut D, elem: U) -> None` with `=>[add] dest: Mut, !elem`), so the destination is
      anything with an `add` the call site can find rather than a `List` —
      the spread's move, applied to the output. The destination is **moved in
      and returned** (user decision 2026-09-08), which is what lets one nest
      inside another; keeping hold of one across the call means rebinding it.
  * Each also has an `intrinsic` **`List` fast path**, which
    [fn-overload-rank] selects when the subject really is a list; the
    generic Salvo body is what every other subject reaches.
* [fn-variadic] `...xs: T[]` collects remaining arguments as an array;
  a spread argument `...xs` forwards an array whole; variadics bind after
  fixed params.
  * A spread into a **variadic intrinsic** is passed on as the collection,
    not as one element: Kotlin uses its own spread (`listOf(*arr)`) and
    Rust takes the vector itself (cloned, since Salvo does not track a
    variadic position, so the array stays usable). Splicing it as one
    argument built a collection *of one array* — which kotlinc catches for
    `listOf` but not for `StringBuilder`, where `append(Any?)` accepts it
    and prints `[Ljava.lang.String;@…` [backend-never-wrong].
  * A variadic position is otherwise untracked by the flow analysis, so an
    intrinsic that merely *reads* its parts must borrow them in Rust: an
    owned splice would move a variable the checker still considers live.
* [implicit-param] `?cmp: (T, T) -> Int` declares an **implicit parameter**:
  one the caller need not pass (user decisions 2026-09-05). Its type must be
  a fn type — what fills it is a function — and implicit parameters trail the
  ordinary ones, since a positional parameter written after one could not be
  passed. They take no part in arity or overload scoring.
  * Inside the body an implicit parameter is an ordinary local of fn type,
    **kept**: it belongs to whoever supplied it. A call to its name goes
    through the parameter, not through overload resolution — it *is* one of
    those functions, chosen by the caller.
  * A fn type carries no effects here, so an implicitly resolved default is
    effect-free by construction: [fn-effects] variance already refuses an
    effectful function where a pure one is expected.
* [implicit-resolve] A call fills each implicit parameter, in this order:
  1. a `name = value` argument written at the call site
     [implicit-override];
  2. an implicit parameter of the **enclosing** fn with the same name and a
     matching type [implicit-forward];
  3. the *unique* visible fn of that name whose signature matches the
     parameter's type, after the call's type arguments are substituted in —
     the overload query the language already runs [fn-overload], only
     against a type instead of an argument list;
  4. otherwise an error naming both remedies (declare one, or pass one).
  * **So Salvo needs no qualified-name syntax for defaults.** Koka spells
    them `Str/cmp` because it does not overload on argument types; here the
    `cmp` whose parameters accept `Str` already *is* the default for `Str`,
    and nothing ties it to the type's declaration.
  * Parameters are contravariant and the result covariant, as for any fn
    value [fn-contract] — and the **contract** has to fit too: a fn that
    consumes an argument cannot fill a position that keeps it. A candidate
    that itself needs implicits is skipped.
  * **A near-miss is reported as one.** The contract is not part of how a
    type *prints*, so two types that differ only there render identically;
    a diagnostic that showed them would read "expects `(Int, Int) -> Int`,
    found `(Int, Int) -> Int`". So the checker explains the difference
    instead — which argument, in which direction, and the two ways to fix it
    — and prefers the most informative candidate: a same-shape,
    wrong-contract one over an unrelated overload of the same name.
  * Which default a call gets depends on what is **visible where the call
    is written** — no global coherence, and two call sites may legitimately
    resolve differently. That is what `use` and handlers already do; the
    locality is deliberate.
  * Ambiguity is an error, never a guess: two matching declarations of the
    name report both and name the override as the remedy.
* [implicit-forward] Inside a generic fn, an implicit parameter of the same
  name and type is passed on automatically. It is the only thing that *can*
  fill an inner call there: nothing about an opaque `T` is knowable
  [call-resolve], so there is no default to resolve. A generic fn that
  declares no implicit therefore cannot call one that needs it — the error
  says which to add. This is colouring, in the same shape effects have and
  for the same reason.
  * Matching is by name and type, **not** by how the parameters were
    declared: a `?Field<T>` spread here fills an individually declared
    `?add` there, and the other way round.
* [implicit-group] `params Field<T> { fn add(a: T, b: T) -> T ... }` declares
  a named bundle, spread into a signature as `?Field<T>`. It is a
  *declaration-side* shorthand only: the members become implicit parameters
  in their own right, and the group has **no binder** (user decision
  2026-09-05 — the binder referenced nothing, prevented no collision, and
  made the caller name it to override one member).
  * A group is never a value, which is what keeps it free of any runtime
    representation: neither backend knows groups exist. Declaring one as a
    struct of fn-typed fields instead is possible since [rs-fn-field] (a
    field is an `Rc<dyn Fn…>` on Rust, 2026-09-08) but still says something
    different: a struct is a value with an identity, and reaching a member
    would need a local first, since `h.f(e)` is dot-notation for `f(h, e)`
    [fn-dot]. A group is resolved per call site instead
    [implicit-resolve].
  * A **member's deduction clause is part of the position** [fn-contract]: a
    member declared `fn next(it: Mut It) -> … => it: Mut` is filled only by an
    implementation that keeps and mutates its parameter, and one that
    consumes it does not fit. (Until 2026-09-09 the member's fn type was
    built without its contract, so every parameter read as
    kept-and-immutable and *no* mutating implementation could fill such a
    position — which is every pass's `next`.)
  * The expansion order — written implicits first, then each group's members
    in declaration order — is published by the checker as the one ordered
    list both the callee's parameters and the caller's arguments follow.
  * Two implicits of the same name in one signature are an error: with no
    binder nothing tells them apart, and [var-no-shadow] would refuse them
    in the body. The remedy is to write the clashing ones out individually.
* [group-obligation] A `params` group may be stated as an **obligation** on
  a struct declaration: `struct Lines : Linear, Yield<Str> canbe Mut { … }`
  — a `:` clause between the generics and `canbe`, comma-separated, each
  entry a group name with type arguments (user decisions 2026-09-08,
  roadmap R1). Where `?Group<T>` asks the *call site* to supply the members,
  `: Group<T>` promises they exist for this type, and the promise is checked
  **at the struct**:
  * the name must be a visible `params` group (a type or qualifier there is
    a position mistake with its own hint), with the group's arity;
  * the same group twice is an error;
  * every member must be satisfied by a **visible fn overload**: parameter
    types and return type equal, positionally, up to a *bijective* renaming
    of type variables — so a generic struct satisfies a group through its
    own type parameters, and `(A, A)` never matches `(A, B)`. The member's
    parameter *names* belong to the group and are not required of the
    implementation (unlike [qual-refn-match], which picks an overload
    someone already declared). Effects are deliberately not compared: the
    group declares none, each implementation declares its own. A failure is
    an error at the declaration naming the missing signature with `self`
    substituted.
  * The obligation adds no scope and no dispatch: the satisfying fn is an
    ordinary overload, and calls to it resolve as ever [fn-overload]. The
    clause moves the *check* to the declaration; nothing is designated in
    the mechanism itself (`Yield<T>`/`Linear` designation is roadmap
    R2/R4).
* [group-self] The declaring type is written **`self`, as a type argument at
  the obligation** — `struct Countdown : Step<self, Int>` — and not as a magic
  `Self` inside the group (user decision 2026-09-08). The group's members
  mention only their own parameters, which is the point:
  * **one declaration serves both uses.** The very same group spreads as
    `?Step<It, T>` implicit parameters, which is how a generic combinator
    reaches its source's member [implicit-group] — the rendering composition
    needs. A `Self` inside the group would have made that impossible, since
    nothing binds `Self` in a signature.
  * The shorthand is bare and unqualified: `Mut self` or `self<T>` is not it.
  * `self` anywhere else — including in a `?Group<self, …>` spread, whose
    arguments are validated like any other written type — is an unknown type.
  * A designated group therefore puts the **state first and the element
    second** (`params Yield<It, T>`), because the state is the parameter
    `self` fills.
* [group-not-a-value] **No value may have a group as its type** (user
  decision 2026-09-08). `items: Yield<Countdown, Int>` is an error in every type
  position — parameter, return, field, `let` annotation, type argument,
  union arm, array element. There is no `dyn`, no erasure, no interface
  value: a group constrains a *named* type and is resolved statically. This
  is the single restriction that keeps the mechanism a where-clause rather
  than a trait, so it is a rule in its own right and not a consequence of
  one.
  * One mistake, one diagnostic: the refusal is reported by validation
    (`reject_group_as_data`), the unknown-type error is suppressed for
    group names, and the *lowering* of a group-typed annotation is
    `Ty::Unknown` so no type-mismatch cascade follows
    [type-unknown-lenient].
* [implicit-infer] **What fills an implicit can determine the call's type
  arguments** (user design 2026-09-06, built with the sequence functions).
  Resolution feeds back into the substitution *between* the arguments, so a
  variable that appears only in an implicit's type is still inferred:
  `map<It, T, U>(it: Mut It, f: (T) -> U, ?Yield<It, T>)` binds `It` from its
  subject, then resolves `next` at `(Mut ListYield<Int>) -> Emitted T | Finished`
  and reads `T = Int` off the `next` that fits.
  * A designated group teaches even more directly: a `?Yield<It, T>` spread
    reads `T` off `It`'s own `: Yield<self, T>` clause, reaching through the
    pass an `iter fn` generates [iter-fn]. That is what types a *bare*
    lambda over an origin subject, where no declared `next` exists to resolve.
  * Two-sided unification: the *candidate's* generics bind from the known
    part of the pattern, and then the *caller's* variables bind from the
    instantiated candidate. A part that is still one of the caller's
    variables teaches nothing and must not match everything.
  * **Repeated until it stops learning, and once more after every argument is
    typed** (user decision 2026-09-10). That is what makes a *container*-shaped
    combinator work — `total<C, It>(c: C, ?iter: (c: C) -> Mut It,
    ?Yield<It, Int>) -> Int =>[iter] proj[from: c] => c`, where nothing but
    the chosen `iter` says what `It` is (and the group says the pass it
    returns holds a borrow of `c` [proj-infer]):
    `C` is only known after the arguments, so the sweeps between them cannot
    learn `It`, and one implicit determining another needs the sweep repeated.
    Declaration order is therefore not a constraint on the author.
    * The motivating case is a [iter-fn] subject, whose generated pass is
      **unnameable** — so a written type-argument list is not an available
      workaround and inferring it is the only way the shape can exist.
    * **Ambiguity is accepted as the price** (user decision 2026-09-10): with
      the container type itself undetermined, several `iter`s match and the
      choice would be a guess. The remedies are the ordinary ones — a `rename`
      that makes one of them answer to a different name [fn-rename], or passing
      the member by name (`iter = ...`) [implicit-override].
  * Silent when the resolution is ambiguous or absent — [implicit-resolve]
    reports that at the end of the call, so one mistake stays one
    diagnostic.
  * Without it the generic half of a sequence function would only work with
    a written type-argument list: the lambda would be checked against an
    unbound `T`, and `U` would then be undeterminable [call-type-args].
* [implicit-resolve]'s candidate test is **parameter-contravariant**: a
  visible `size(list: List<T>) -> Int` fills a position wanting
  `(Mut List<Int>) -> Int`, because reading a list that happens to be
  mutable is what it does. (`is_subtype` compares fn parameters invariantly
  — deliberately, since a backend renders a parameter's convention from its
  declared type — so this is a rule of implicit resolution, where the value
  is only ever called by the callee that declared the position.)
* [implicit-intrinsic] An `intrinsic fn` that fills an implicit is passed as
  an **adapter closure whose body is its lowering**: an intrinsic has no
  target-language function to reference (`::iter` does not exist in Kotlin,
  and `iter` names the generated *module* in Rust — E0423). The adapter's
  arguments follow the resolved declaration's own parameter modes, which on
  Rust means a kept struct parameter is borrowed.
* [implicit-override] `sort(xs, cmp = my_cmp)` supplies one implicit
  parameter by name. Named arguments exist for exactly this — Salvo has no
  general named-argument form — so a name matching no implicit parameter of
  the callee is an error, and a positional argument cannot follow a named
  one. The `=` is unambiguous because assignment is a statement in Salvo,
  never an expression.
* [implicit-fn-only] Implicit parameters are declared on a **fn or an effect
  member** — a member is an ordinary signature, so it may have them (user
  decision 2026-09-06). They belong to the member's signature: the generated
  interface takes them, every handler's implementation of the member takes
  them, and the *call* fills them, resolving with the effect instance's type
  arguments substituted in.
  * A **handler constructor** may have *fn-typed* implicits (`?copy: (v: T)
    -> T`; lifted 2026-09-11 [copy-implicit]): `use` is where the handler's
    type arguments are known, so it resolves them as a call resolves a fn's.
    A **lambda** may not: its type has no room to declare one (deliberate
    cut).
* [fn-lambda] Lambdas: `x -> expr`, `(a, b) -> expr`, and block bodies
  `{ x: T -> ... }` (blocks require `return`). Lambdas may declare
  effects/deductions.
  * Param types come from annotation or the expected fn type; early
    `return` inside expression-position lambdas is a codegen error
    (deliberate cut).
* [iter-fn] A **`iter fn`** is a hand-written `next` whose **pass struct is
  generated** (user decision 2026-09-09): the subject stays ordinary data, the
  `state { … }` block declares the pass's own fields, and the compiler writes the
  pass struct plus the `iter` that mints one. It is the third way to be
  iterable, beside a written-out pass [iter-protocol], and the one with the
  least to declare — the subject needs
  no `: Yield<self, T>` clause, because the `iter fn` *is* the declaration.

  ```
  struct Countdown { from: Int }

  iter fn next(c: Countdown) -> Emitted Int | Finished {
      state {
          at: Int = c.from
      }
      if at <= 0 { return finished() }
      at = at - 1
      return emitted(at + 1)
  }
  ```

  * **It is a desugaring, done in the syntax crate** (`desugar::expand_pass_fns`,
    inside `parse_module`), into a hidden `struct __Pass_<Subject> :
    Yield<self, T> canbe Mut { __subject: proj Subject, <state fields> }`, an
    `fn iter(s: Subject) [] -> Mut __Pass_<Subject>` whose body is the struct
    literal (its lend of `s` inferred [proj-infer]), and the author's body as
    `fn next(__p: Mut __Pass_<Subject>) -> Emitted T | Finished => __p: Mut`
    with the subject and the state fields written out as field reads (a
    `proj[from: s]` in the written return is redirected to `__p`
    [yield-proj]). Nothing downstream knows the form exists, which
    is why `for`, the combinators, `let p = iter(c)`, deductions, narrowing and
    both emitters need no new machinery.
  * **The `state` block is declarations only**, each with an annotation and an
    initializer, evaluated **once per pass, at the mint**. The initializers
    become the body of the generated `iter`, which is declared `[]` — so "no
    effects in an initializer" is not a rule of its own but an ordinary effect
    error at the offending call. (The annotations are required for the same
    reason a struct field's are; all three sites — struct fields, handler state,
    `state` fields — would gain inference together.)
  * **The pass holds as little of the subject as the body needs**, decided by a
    scan of the body in three tiers — all observationally identical, because the
    pass **borrows** what it holds [proj-field] and the subject cannot be
    written while a pass over it lives [proj-infer] (user decision 2026-09-11;
    until then the mint *copied*, the phase's last hidden copy):
    1. the body never reads the subject (a plain counter): the pass holds
       **nothing** of it, and the mint is `__Pass_C { at: c.from }`;
    2. the body only ever reads *plain fields* of it, their types are visible
       (the subject's struct is declared somewhere in the **program** — the
       expansion runs program-wide, local declarations winning a name) and the
       subject type is non-generic: one **`proj` field per field read**
       (a Copy scalar stays owned [copy-scalar-free]), initialized at the mint
       (`__Pass_Fibs { count: f.count, … }`), keeping the field's own name
       unless a `state` field already has it;
    3. otherwise — the subject handed on as a value, an assignment through it, a
       generic subject, a declaration the program cannot see: the **whole
       subject** is borrowed (`__subject: proj Subject`, minted as
       `__subject: s`).
  * **Borrowing rather than copying** is what keeps the backends in step
    without a copy: a write to the subject during a drive is *refused* rather
    than differently visible. An `iter fn` that wants a snapshot writes one
    (`state { rows: List<Int> = copy(c.rows) }`). A *generic* subject therefore
    needs no `copy` of a `T`, and the former Kotlin refusal [kt-copy] is gone.
  * **The subject is read-only in the body**: it is a field of the pass typed as
    the subject, so writing through it is the standing [struct-mut] refusal.
  * **The generated pass is not nameable.** `__Pass_…` is the compiler's
    namespace and a *written* type reference beginning with `_` is a parse error,
    so a pass a program must name is written out by hand — the boundary the form
    rests on.
  * **Refused, each at the declaration**: a name other than `next` (it is the
    obligation's member, and `for` reads a declaration), a parameter count other
    than one, a `Mut` subject (advancing never writes through it), a result other
    than `Emitted T | Finished` (the element type is read out of it), a
    structural subject (array, tuple, union — the pass is named after the
    subject), a `state` field without an initializer or an empty `state` block,
    and a binding in the body that would **shadow** the subject or a `state`
    field (it would silently mean something else).
  * **Effects are ordinary effects**: each `next` is a separate call, so
    `[Console]` on an `iter fn` needs no threading of handlers across a
    suspension — a `for` passes them per turn [fn-effects].

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
  extra handler args. Members must still declare their deductions and
  return type [decl-explicit].
  * Roadmap E1 lifts this for *handlers*, whose dependencies are declared
    as constructor parameters of effect type and supplied by the per-scope
    fusion (user decision 2026-09-04; mechanism in
    [rs-effect-fusion] / [kt-effect-fusion]). A *member* declaring its own
    effects stays an error: the dependency belongs to the implementation,
    not the interface.
* [effect-handler] `handler H<G>(ctor params) of E<G> { state fns }`
  implements every member of its effect; state fields have initializers
  and persist for the handler's lifetime.
  * A state field's initializer is **checked against its declared type**,
    like a struct field's default: it runs at construction with no locals
    in scope. (Until 2026-09-04 it was not checked at all, so
    `held: Mut List<Int> = "no"` was accepted and the emitters never saw
    the expression's types, which the backends' intrinsic lowerings rely
    on [backend-intrinsic].)
* [effect-state-store] Assigning a value into a handler **state** field is a
  *store*: the field outlives every member call, so the handler takes
  ownership, exactly as a struct literal does ([deduce-consume]). Ordinary
  locals only *link* ([fate-link]) — the difference matters because a link
  that outlives its call is representable on one backend and not the other.
  * Consequence: a member that promises a parameter back
  (`=> list: Mut`) may not store it; the honest contract for a storing
  member moves it (`=> !list`), and callers then give up ownership.
  * Handler member bodies are validated against their written lists like any
    fn's ([deduce-infer]), which is what makes the rule bite. Before both
    halves existed, `held = list` under a keeping contract diverged
    observably: Kotlin aliased the list into the state (later caller
    mutations visible) while Rust cloned it — the same program printed 2 and
    1.
* [effect-not-data] An effect names a *capability*, not a type of values.
  It may appear in a fn's effect list (`[Console]`) and in a handler's `of`
  clause; every data position — struct field, parameter, return type,
  `let` annotation, type alias, union or tuple component — is an error
  (user decision 2026-09-03). The value would have to be a handler
  instance, and those are reached through `use`.
  * The one exception is a handler *dependency*
    ([effect-handler-deps]).
* [handler-not-value] A handler instance is produced by `use` and lives in
  the effect environment; a handler constructor call in any other position
  is an error naming the `use` remedy. (Rust could not render one anyway:
  the emitted `Name()` is not a constructor, `E0423`.)
* [effect-handler-generics] A `use` may write its handler's type arguments
  (`use Plain<Int>()`), and they bind the handler's generics — which for a
  handler with no constructor argument is the only thing that can, since
  there is nothing else to infer them from.
  * The written list and what the constructor arguments imply must **agree**:
    a disagreement is an error naming both, rather than one silently winning.
    The wrong *number* of arguments is reported against the handler.
  * What they decide is the **effect instance** the `use` registers, which is
    what a member call resolves against [effect-disambiguation] — so this is
    a language-level rule, not a rendering detail. Both backends then have to
    construct the handler *at* that type, since neither target can infer a
    class's parameter from an empty argument list.
* [effect-handler-deps] A handler constructor parameter of **effect type**
  is a *dependency*: the one position where an effect names something a
  handler holds ([effect-not-data]). It is declared on the *handler*, not
  the effect — implementations differ in what they need (user decision
  2026-09-04).
  * The handler's member bodies may use that effect, exactly as if they had
    declared it — which they may not ([effect-member-no-effects]): the
    dependency belongs to the implementation, so it is stated once.
  * A dependency is **not written at the `use` site**: the compiler supplies
    it from the enclosing scope, so it does not count as a constructor
    argument, and a `use` whose dependency has no handler in scope is an
    error naming it ("register one before it").
  * A handler may not depend on the effect it implements — registering it
    would require itself.
  * Dependency **cycles need no separate check**: a dependency must already
    be registered when its dependent is, so a cycle cannot be constructed
    in any order (verified both ways). The availability rule *is* the
    acyclicity guarantee, which is what the fusion emission relies on.
  * Emission is the fusion strategy ([rs-effect-fusion],
    [kt-effect-fusion]). Kotlin injects the dependency at construction
    (objects alias, so nothing more is needed); Rust builds one *fusion*
    per `use` scope which owns the handler, implements every effect in
    scope, and hands the dependency to the member body from a disjoint
    borrow — because capturing it in the handler would lock it for the
    handler's lifetime where Kotlin shares it freely. Both backends run
    the same programs to the same output.
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

### Non-resumption: `throw` and `try`

* [throw] `throw(message)` leaves the enclosing delimiter instead of
  resuming. It is declared in std (`core.throw`) as the sole member of
  `effect Throw<M> { fn throw(message: M) -> Nothing }` and known to the => !message
  compiler by name; the message is *moved* into the outcome.
  * Its type is `Nothing`, the bottom type: nothing after it runs, so the
    intermediate frames stay silent. A fn that may throw declares
    `[Throw<M>]` and keeps its **own** return type — it never also returns
    an outcome union, which would be `Result` plumbing with extra steps
    and would defeat throw being an effect.
  * **The ability to not resume is declared, never discovered from a
    handler**, which is what keeps the colouring honest: the shape of a fn
    cannot depend on which handler flows in, so it is exactly the effect
    annotation the author already writes.
  * `Throw` has **no handlers**: `handler X of Throw` is an error, and it
    is never threaded as an effect parameter (both backends exclude it).
    What *provides* it is an enclosing `try`, or a caller that declares it
    in turn.
  * A site that may throw is every `throw` call **and** every call whose
    callee declares `[Throw<M>]`; each is recorded in `Checked::may_throw`,
    since taking the throw is the emitters' job.
  * The message type must fit the landing site's: a `Str` throw inside a
    fn declaring `[Throw<Int>]` is an error naming both.
* [throw-not-main] `main` may not declare `[Throw<M>]`: there is no caller
  to receive it, Rust cannot express a `main` returning `ControlFlow`, and
  Kotlin would die on an uncaught signal. The delimiter goes inside.
* [try] `try { block }` is the delimiter: a **compiler intrinsic**, not an
  effect (user decision 2026-09-04 — "there's not much value in a function
  declaring the `Try` effect in its signature any more than there is in
  declaring that it uses loops or if-expressions"). So no `Try` handler, no
  `[Try]` in signatures, and no collision with the fusion.
  * It is an **expression** of type `Ok T | Thrown M`, where `T` is the
    body's value type and `M` the message type. Both arms are qualified,
    reusing `Ok` from `core.result`, so `is`, `when` and exhaustiveness
    need no new rules — the outcome is structurally an `Ok T | Err M`.
    `Err` is deliberately *not* reused: a throw is not an error value.
  * **`M` is the union of the message types the body performs** (user
    decision 2026-09-04, chosen for consistency with `if`/`when` branch
    types): one type stays bare, several form a union, and the generated
    code only wraps when a union is present.
  * `Thrown M` is **forgeable**, deliberately: the qualifier carries no
    authority, so `core.throw`'s `thrown(message)` constructor produces a
    value in the thrown arm without transferring control. The authority is
    `[Throw<M>]` availability alone.
  * A body whose every path leaves still has an `Ok` arm — `Ok None`.
  * **A `try` whose body cannot throw is an error** (user decision
    2026-09-04): nothing can produce the thrown arm, so it is dead
    scaffolding. The alternative (`Thrown None`) would force callers to
    handle an arm nothing can make.
  * A `return` inside a `try` body returns from the enclosing *fn*: the
    delimiter catches throws, not returns.
* [try-innermost] A throw lands in the **innermost** enclosing `try`.
  There are no labelled throws; a nested delimiter takes its own body's
  throws and lets an outer one pass through.
* [throw-linear] Nothing linear may be live across a site that may throw
  [linear-obligation]: the code after
  the site does not run on the throw path. The diagnostic names the two
  remedies that exist: release before the call, or move the value onward so
  the obligation travels with it. (`defer` was the third — it discharged on
  a path the author does not write — and was removed 2026-09-10, since
  linearity is what makes the obligation checked in the first place.)
  * The frames that die are those inside the delimiter (a throw caught by
    an enclosing `try` does not leave the fn), so the check's floor is the
    `try` body's scope, or the fn's when the throw propagates out.

## Deductions

* [deduce-syntax] The **deduction clause** — `=> entry, entry, …` after the
  return type (or after the effect list when there is no return type), on
  the same line or the next; the body's `{` follows its last entry — states
  what a call does to each parameter and what the result holds of them
  (user design 2026-09-02 D1 for the entries' polarity; respelled from the
  bracket list by user decision 2026-09-11 — `=>` is implication, brackets
  after a parameter list now mean effects only).
  * Entry forms:

    | Form | Meaning |
    |---|---|
    | `=> list` | keep-all: kept, nothing stripped |
    | `=> !list` | consumed (caller loses access); `list: Nothing` says the same |
    | `=> list: A B` | **exhaustive**: afterwards *only* `A B` apply |
    | `=> list: None` | exhaustive and empty: every qualifier stripped |
    | `=> list: -A` | **delta**: drop `A`, everything else survives |
    | `=> .f: proj[from: a]` | the result's field `f` projects `a` [proj-infer] |
    | `=> v.f: proj[from: a]` | the parameter `v`'s field is re-pointed to project `a` [proj-infer] |
    | `=> proj[from: a, b]` | opaque: the result holds a borrow of `a` and `b` somewhere inside [proj-infer] |
    | `=>[f] entry, …` | a group: entries about the fn-typed parameter `f`, whose own parameters are named in its type [fn-contract] |

  * **Unmentioned means inferred.** A parameter the clause does not mention
    gets the entry the body implies [deduce-infer] — the clause is partial,
    and most fns write none. What is written is fixed and validated against
    the body. A declaration *without a body* (an effect member, a `platform
    effect` member, an `intrinsic fn`) must mention every parameter except
    Copy scalars [copy-scalar-free], implicits and variadics; a fn *type*
    keeps its default (kept) and is written only through `=>[f]`, never
    inline (`f: (v: T) -> [v] U` is a parse error: ambiguous with the next
    parameter).
  * The removal set is computed against the qualifiers the *argument*
    actually carries, not against the parameter's declared set. That is
    what makes the exhaustive form sound: it also drops qualifiers the
    callee never declared and therefore cannot have preserved.
  * **Mutation forces the exhaustive form.** A parameter the body
    mutates may use neither keep-all nor a delta: mutation can invalidate
    a caller's *state* predicates that the signature never mentions
    (provenance claims are exempt from removal entirely
    [qual-subject]; the
    unsoundness D1 fixed — `clear(list: Mut List<Int>) => list` would
    silently preserve a caller's `NonEmpty`). Mutation is the only
    invalidating operation on a kept value: reads preserve state and a
    move ends the caller's access.
    * A fn with *no body* (an `intrinsic fn`, an effect member) has
      nothing to inspect, so a parameter declared `Mut` counts as mutated —
      taking `Mut` is taking permission to invalidate. This is what makes
      std's own mutators (`add`) drop a caller's predicates. Residual, and
      deliberate: an intrinsic that mutates through the *contents* of a
      non-`Mut` parameter is trusted, like `as Qual`.
    * The resulting over-strictness (`add` cannot promise `NonEmpty` back
      even though appending can never empty a list) is recovered by
      **refinements** [qual-refn]: the function cannot state the fact, so
      the qualifier that owns the claim states it instead.
  * Exhaustiveness is **contagious** through the call graph: a fn that
    hands a parameter to an exhaustive callee can no longer promise its
    own caller's extras either, so its inferred entry becomes exhaustive
    too.
  * Written entries are shape-checked: a plain entry names a parameter
    (once); an exhaustive entry may only keep qualifiers declared on that
    parameter (a deduction preserves or drops, it never *adds* — `+Qual`
    is rejected in a *function's* clause, see D2; it is how a **refinement**
    states what a call establishes [qual-refn]); an entry is either
    exhaustive or a delta, never both; `Nothing` is the only type form
    (other type narrowings are D1b); a result path or a parameter's field
    path can only state a projection; a parameter both consumed and named
    as a projection source is an error.
  * A delta may name a qualifier the parameter does not declare (a fn that
    knows it invalidates a specific property). It is a convenience — the
    exhaustive form is the sound default, and inference never relies on a
    delta to be correct.
  * **Revisit (recorded 2026-09-11):** with unmentioned parameters inferred,
    editing a body to consume a parameter changes what callers may do with
    no signature change. Accepted (the hover shows the effective clause);
    the remedy, if it bites, is an opt-in exhaustive marker or a lint
    asking for `!p` to be written.
  * `rename fn` takes no clause; `refn` takes only `+`/`-` entries
    [qual-refn]. The LSP hover renders the *effective* clause — written and
    inferred entries alike, Copy scalars and keep-all entries omitted —
    and any `=>[f]` groups.
* [deduce-infer] A parameter the clause does not mention is inferred as the
  strictest deduction over all uses of it in the body;
  deductions never depend on the return value. If inference is impossible,
  they must be written.
  * Inference runs as a whole-program fixpoint after checking
    (`deduce.rs`), starting optimistic (keep-all for every parameter);
    facts only shrink along `KeepAll` → `Remove` (growing) →
    `Exhaustive` (shrinking), so it terminates. Results
    are stored in the typed IR (`Checked::deductions`); Kotlin ignores
    them — they are the Rust backend's ownership/borrow contract.
  * Moves are inferred when a bare parameter is: passed to a call whose
    contract consumes it, returned *owned* (a return under a wholesale
    `proj[from: p]` is a borrow, not a move [readonly-return]), `break`-ed,
    stored in a
    struct/array/tuple literal, or passed to a `use` handler
    constructor. Binding a bare parameter (or a projection of one) with
    `let`/assignment is *not* a move by itself — it fate-links the new
    variable to the parameter [fate-link] — but a binding that is later
    moved or mutated *claims* the parameter as moved (move-mode takes
    ownership through the chain [fate-move-mode]); the claims are
    seeded into the fixpoint between the checking rounds. Effect-member
    calls have no `FnKey` (the handler is chosen at run time) but their
    *declared* clause is the contract: inference reads it through the
    checker's recorded effect instance, so a member that takes ownership
    moves the argument here exactly as it does at the call site. Value flow
    out of a branch/loop tail is not tracked as a move yet.
  * A written entry is validated against the same body facts: it may be
    *stricter* than the body (drop qualifiers, move a parameter the body
    gives back), but promising a parameter back that the body moves, or
    a qualifier the body may remove, is an error. Unmentioned parameters
    are not validated (nothing is written to be wrong); every bodied fn
    joins the fixpoint, and its written entries are overlaid on each round.
    * **Handler members** are validated the same way. They carry no
      `FnKey` (not top-level items), so they are checked outside the
      fixpoint — sound because the dependency runs one way: a member's body
      facts depend on other fns' contracts, and a fn's contract depends on
      members' *declared* clauses, never their bodies.
* [deduce-consume] Deduction clauses are enforced flow-sensitively at call
  sites on bare identifier arguments — written entries directly, and
  unmentioned parameters through their *inferred* facts: checking and
  inference iterate to a fixpoint [deduce-fixpoint], so `return list` in a
  callee consumes the caller's argument exactly like an explicit `!list`.
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
    literal or any spread position), `return n`, `break n`,
    and passing it to a `use` handler constructor. The use-site
    diagnostic names the consuming event ("consumed (moved) by a
    literal store / a `...` spread / a `break` / a `use`
    handler registration / an earlier call"). Reads never consume —
    in particular string interpolation `"${n}"` is a read [type-str]
    (user decision 2026-09-02, L2a).
  * The code after a loop is reached from the fall-through exit *and*
    from every `break`: the loop exit merges the flow state captured at
    each `break` statement, so a value consumed on a break path stays
    consumed after the loop even when the `break` sits inside an
    always-exiting branch (which contributes nothing to the merge
    *inside* the body — the loop exit is where its state lands).
  * A *kept* parameter sheds its removal set: the argument's narrowed
    type loses whatever the entry's effect drops — everything unnamed for
    an exhaustive entry, the named ones for a delta — so a follow-up call
    whose overload requires a removed qualifier fails resolution (e.g. a
    second `remove_first` after `=> list: Mut` stripped `NonEmpty`).
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
  narrowing), with the remedy of writing the entry explicitly.
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
  binding), and carry **which projection of the root** the value came from
  ([fate-field-disjoint]). Reads never consume and
  never poison, on any member, at any time. Function results are
  independent — unless the fn declares a derived return
  (`proj[from: p]` [readonly-return]), in which case the result
  links to the argument; a fn returning a projection of a kept
  parameter *without* the annotation must `copy` internally.
  `copy(...)` produces an unlinked value [copy-fn].
  * Provenance is static: bare identifiers and field/index/`!` chains
    over one. Values built by calls, literals, operators, or branch
    expressions are independent.
  * Links are flow state: they union across branch merges (may-be-linked
    is linked) and survive loop back-edge re-checking.
  * Tooling presentation (user decision 2026-09-02): a derived variable
    is rendered with a compiler-inserted `proj` qualifier — bare on
    the type line, with its parameters (the fate roots and binding
    sites, recorded in `Checked::fate_reads`) shown only as on-request
    detail. Presentation-only today; `proj` is not part of the type
    system and cannot be written in source. Parameterized compiler
    qualifiers as *checked* signature vocabulary are the leading design
    for L7 (see COMPLETED.md, and ROADMAP.md for what is left of it).
* [fate-derived-readonly] A fate-linked (derived) variable in
  *borrow-mode* is read-only: moving it (a call that does not keep it,
  `return`, `break value`, a struct/array/tuple literal store,
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
    literal store, spread, `return`/`break`, `use` ctor
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
    their provenance roots at the projection they name (`add(h.tags, 2)`
    poisons values derived from `h.tags` and from `h` itself, but not one
    derived from `h.name` [fate-field-disjoint]; the projection-argument
    case is a backend-parity fix from 2026-09-02) — an assignment through
    a projection, and `++`. Whole-variable reassignment (`x = ...`,
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
* [fate-field-disjoint] **Fields are tracked apart** (L5, 2026-09-10). A
  link records the projection path out of its root (`let n = p.name` →
  `[.name]`, `let q = p` → `[]`), an event carries the path it hit, and
  poison fires only where the two **overlap** — `Place::overlaps`, the
  relation P1 built for narrowing [flow-place], reused unchanged. So
  reading `p.name` while `p.tags` is mutated is legal, and the four
  overlap cases still poison:
  * the **same** projection (`p.tags` mutated, value derived from
    `p.tags`);
  * a **prefix** either way — mutating `o.inner.tags` poisons a value
    derived from `o.inner` (it changed part of what that names), and
    mutating `o.inner` poisons one derived from `o.inner.tags`;
  * the **whole variable** (`[]`), which is a prefix of everything: a
    `Mut` argument of a bare identifier, a reassignment, `++`;
  * a **computed index**, since `proj::Element` may-aliases any element
    position — `xs[i]` and `xs[j]` are not distinguished.
  * A derivation that is not a plain projection chain (a `!` unwrap, a
    value the analysis cannot place) links with an **unknown** path,
    which overlaps every event: precision is never assumed where it has
    not been earned.
  * Transitive links compose paths: with `let p = q.inner` then
    `let n = p.name`, `n`'s link to `q` is `[.inner, .name]`.
  * **The move half is [fate-partial-move]**: a move of a projection
    records it as *moved out* of its root rather than consuming the whole
    variable.
  * It changes no emission: on the Rust backend a borrow-mode binding
    from a pure place already emits a real borrow, and rustc permits
    that borrow to live across a `&mut` of a disjoint field of the same
    local — Salvo's precision and rustc's coincide, so the newly legal
    programs compile clone-free [rs-borrow-locals]. Kotlin aliases
    regardless.
* [fate-partial-move] **Moving one part leaves the rest** (L5's move half,
  2026-09-10). A move of a projection (a consuming call, a move-mode
  binding, a store) records the path as *moved out* of its root; the root
  stays live, and its `moved_places` are flow state like any other:
  * a read of a **disjoint** projection passes — `eat(p.tags)` then
    `p.name` is legal;
  * a read **overlapping** a moved place is an error naming the field
    that left (same path, a prefix either way, or a computed index);
  * a use of the **whole value** is always refused: a struct missing a
    part cannot be passed, returned or stored. The diagnostic says so
    separately from the read case, because the remedy differs (move the
    remaining parts individually, or `copy` at the move site);
  * an **assignment** to the place puts it back, dropping every moved
    record the assigned place covers; whole-variable reassignment revives
    everything, exactly as it revives a consumed variable;
  * merging is **union** — moved on any path is moved — and survives loop
    back edges, so a read early in a body errors when a later statement
    moved the field in the previous iteration. (`place_narrows`, the
    narrowing dual, intersects instead.)
  * A **kept parameter still refuses the move outright**: the caller keeps
    the value, so nothing may be taken from it. This is Rust's rule for a
    borrowed parameter (E0507), and it is why partial moves need no
    signature notation — an *owned* parameter may be partially moved
    because the caller already surrendered the whole value and can never
    observe the partial state. Ownership stays all-or-nothing per
    parameter, and a place-parameterized deduction is deliberately not
    wanted (it would be viral, and passing the field instead of the
    struct says the same thing).
  * On the Rust backend a moved projection emits as a real **partial
    move** of the field (the existing `moved_projections` rendering), and
    a revival emits as rustc's reinitialization of a moved field; both are
    accepted by borrowck [rs-borrows].
* [fate-lambda] Lambdas are ordinary values under shared fate (decision
  L4a, 2026-09-02): a lambda's relationship to the variables it
  captures is classified from its body, and the contract binds at
  *creation* (a closure may run zero or more times, unlike a named fn
  whose deductions fire per call).
  * A capture that is only *read* makes the lambda a **view** of it
    [lambda-view]: the closure holds a borrow of every non-Copy read
    capture (a Copy scalar is the value itself [copy-scalar-free]) — the
    variable stays readable, mutating or moving it poisons the closure,
    and binding/moving the closure follows the ordinary view rules.
    (Until 2026-09-12 only *transitively-mutable* read captures linked,
    alias-style — which let a named lambda smuggle a capture-rooted
    projection past the discipline; see [lambda-view].)
  * A capture the body *mutates* is consumed at creation — the closure
    takes ownership (each call mutates it; an original observing those
    mutations on one backend but not the other would break parity). A
    written-kept parameter cannot be captured-and-mutated (error,
    remedy capture `copy(x)`); an inferable parameter is *claimed* as
    moved [deduce-infer].
  * A lambda that *consumes* a capture (a call that moves it, a store,
    spread, `return`/`break`) is `once`-typed [once-fn]: the
    capture is consumed at creation and the closure is callable at most
    once — the closure *owns* that value, so it is **not** a view of it
    [lambda-view]. Consuming a *linear* capture remains an error
    [linear-lambda].
  * Effects do not yet cross the lambda boundary as a contract: fn
    types parse an effect list (`(S) [E] -> T`) but the checker drops
    it, and lambda bodies check under the enclosing fn's effect
    environment (lexical). Fn-type contracts (deductions, effects, and
    a call-multiplicity qualifier enabling consuming captures) are the
    L7 parameterized-qualifier work.
* [fn-contract] Fn types carry *contracts* (L7d, 2026-09-02): parameters
  may be named, and the enclosing declaration may write a group for the
  parameter [deduce-syntax] — `f: (v: List<Int>) -> Int` with `=>[f] !v`
  consumes its argument, with `=>[f] v` (or no group) keeps it, and an unannotated fn type
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
  * Effect lists on fn types are enforced as of E3 step 3 — see
    [fn-effects]. `once` inference remains open.
* [fn-effects] A fn type may declare effects — `(s: Str) [Logger] -> Str` —
  and they mean **a requirement the caller of the value supplies**, not a
  capability the value carries (user decision 2026-09-04). The effects are
  *threaded into* the call on both backends; nothing is captured.
  * A lambda body performs the effects **its type declares**, not whatever
    its enclosing scope happens to have. Lexical leakage is what made a fn
    value's capabilities invisible in its type — the same objection as
    silent colouring elsewhere in E3.
  * **A fn inherits its fn-typed parameters' effects** (user decision
    2026-09-04): the only reason to take `f` is to call it, and calling it
    needs those effects *here*, so `fn run(f: (s: Str) [Logger] -> Str)`
    needs no list of its own — and its callers must supply `Logger`, since
    that is where the value comes from at run time. Inheritance reaches
    through qualifiers (`once () [Logger] -> None`), optionals and unions,
    and it is part of the callee's contract at call sites.
  * **Variance**: a value performing *fewer* effects fits where more are
    expected (a pure lambda passes to a `[Logger]` position and simply
    ignores what it is given); never the reverse, which would reach a call
    site that cannot supply it. Same direction as [fn-contract] and
    [once-fn].
  * **Inference**: an un-annotated lambda's effect set is inferred from its
    body (inference from a *visible* body is what [decl-explicit] permits),
    so it cannot silently fit a pure position. Where a fn type is written,
    its list is authoritative — including for emission, since a pure lambda
    in an effectful position must still *take* the parameters the caller
    passes.
  * **A fn value carries no capability, so it may be stored and passed
    freely** — only *calling* it needs its effects in scope. That is why
    the roadmap's third sub-item ("forbid escape") was dropped rather than
    built (user decision 2026-09-04): the error surfaces at the call, not
    at the storage.
  * `use` in a fn type's effect list is an error: registering a handler is
    local to a body, so a lambda may `use` exactly when the function
    containing it may.
  * A named fn used as a value performs exactly the effects it declares
    (inherited ones included), which is what the variance rule compares.
  * Backends: both **thread** the effects as leading parameters
    ([rs-effect-fusion]'s cut is lifted by this; [kt-effect-params]).
    Kotlin erases contracts (aliases throughout; named fns
    pass as `::name` function references, or an adapter lambda when the
    effect lists differ). Rust renders fn parameters
    as `&mut impl FnMut(…)` (accepting both plain and handler-mutating
    closures; `once` stays owned `impl FnOnce`), argument types per
    contract (kept non-Copy `&T`, kept `Mut` `&mut T`, moved/Copy
    owned), call-site arguments per the recorded contract, lambda
    parameter bindings/annotations per contract, and named fns wrap in
    mechanical adapter closures bridging the contract's calling
    convention to the declaration's actual modes.
* [once-fn] `once` is the language-level **use**-multiplicity qualifier
  (decision L7b, 2026-09-02; generalized from calls to uses by a user
  decision 2026-09-07): it means the value may be used **at most once**.
  Enforcement is consumption [deduce-consume], and what counts as a use
  depends on the type.
  * **Valid on any type** (D6, user decision 2026-09-12; until then
    fn-types plus `canbe once` opt-ins): the upper bound `[0,1]` is a
    restriction the holder imposes on itself, demanding nothing of the
    type's author — `once Ticket` means "use at most once" wherever it is
    written, and the position gate (`types::once_position`,
    `has_auto_once`) is gone.
    * On a **data** type, using is consuming, and `once T <: T`: a
      plain-typed holder can consume at most once anyway (a move is a
      move), so the bound survives the drop — `spend(t: once Ticket)`
      forwards `t` to a consuming `redeem(t: Ticket)` and the second
      `redeem(t)` is the ordinary consumed-value error.
    * On a **fn** type nothing changed: using is calling, a plain fn
      value is callable repeatedly, so `once` never drops there — passing
      a `once` fn where a plain one is expected stays refused.
  * **On a fn type**: calling a `once` value consumes it, so a second
    call, a call on the loop back edge, and a call after the value
    escapes are the ordinary consumed-use errors; a call on only some
    branches leaves it maybe-consumed (conservative), and zero calls is
    fine.
  * **On any other data type**: using means consuming — a move into a
    consuming call, a store, a `return`. (`canbe once` opt-ins are no
    longer required or meaningful; the D6 generalization covers every
    type. Driving a named pass advances it in place
    [iter-drive-in-place], which is a mutation, not a use.)
  * `once` erases like every other qualifier [qual-erasure].
  * **Inverted subtyping — flagged for future review** (user decision
    2026-09-02): `once` *restricts* instead of refining, so plain
    `(A) -> B` <: `once (A) -> B` (any fn may be treated as
    once-callable) and, generally, `T` <: `once T`; and `once` may **never**
    be dropped — the exact opposite of every other qualifier's direction. Special-cased in
    `is_subtype` and `unify`.

  * A lambda that *consumes* a capture is legal (superseding the
    always-error rule in [fate-lambda]) and is `once`-typed by
    construction: the capture is consumed at creation and the closure
    fits only `once` positions. Consuming a *linear* capture is still
    an error (the closure would inherit an exactly-once obligation —
    future work).
  * Escape rule (conservative, relaxable with fn-type contracts):
    passing a `once` value as an argument consumes it regardless of the
    callee's contract — fn-value ownership is otherwise untracked.
  * No inference in v1: a callee must *write* `once` to accept
    consuming lambdas (a body that calls its fn param once does not
    auto-promote); inference may come later (written validates,
    unwritten infers — the deduction precedent).
  * Backends: Rust emits `once` fn parameters as `impl FnOnce(…)`
    (rustc's capture inference already makes consuming closures
    `FnOnce`); Kotlin emits the ordinary function type — the
    multiplicity is protocol-only on the JVM.
* [proj-type] **`proj` is part of the type** (option A, user decision
  2026-09-12; until then it was erased in lowering and carried only on
  fate links). `proj Str`, `Mut List<proj Str>`, `Emitted (proj Str) |
  Finished` are the checker's types: they print in diagnostics and hover,
  and a generic binds through them (`Emitted T` against
  `Emitted (proj Str)` gives `T = proj Str`).
  * Subtyping: `X <: proj X` — an owned value satisfies a projected
    position (it can do strictly more). The reverse never holds, and
    `proj` is **never dropped implicitly**: it is a *never-drop* qualifier
    (like `Linear`; unlike `Ok`). Argument acceptance is **kept-aware**: a
    kept, non-`Mut` parameter only reads its argument, so a *top-level*
    projection fits it even where the parameter is written owned (the
    borrow of an owned value is a borrow — and on the Rust side a kept
    non-`Mut` parameter is `&T` whatever the spelling [rs-borrows]).
    A projection is refused, with an error naming the type and the
    remedies ("write the parameter's type with the `proj`, or pass
    `copy(...)`"), exactly where the representation would clash: the
    position **consumes** (`ProjBlock::Consumes` — by-value in Rust),
    **mutates** (`ProjBlock::Mutates` — `&mut T`; [proj-readonly]), or the
    projection sits **nested** inside the type (`ProjBlock::Nested` — a
    union arm or type argument, where `List<Str>` and `List<proj Str>`
    are different Rust types and must match exactly). The same on both
    backends, so a program's legality never depends on the target.
  * `proj` on a Copy scalar erases: `proj Int` *is* `Int` — the number is
    the value itself on both backends [copy-scalar-free].
  * Unification treats `proj` in a *pattern* as optional (like `once`): a
    parameter written `proj T` accepts an owned argument (binding `T` to
    it whole) and a projected one (binding `T` to the base minus the
    matched qualifiers — `proj T` against `Mut Str` gives `T = Mut Str`).
  * Overloading: a `proj` position **accepts more, so it says less** — an
    owned position beats a projected one (`X <: proj X`, so the inversion
    mirrors the union rung of [fn-overload-rank]); it is its own ranking
    dimension, so `proj Mut Str` vs `Str` is an ambiguity, not a guess.
    (This replaces the earlier "`proj` never affects overloading".)
  * At a *definition* over a bare generic, `proj T` constrains the value
    (a projection may arrive) but the body treats `T` uniformly; which
    instantiations actually borrow is the call sites' fact
    [rs-proj-arm].
* [readonly-return] `-> proj[from: p] T` marks a **projected return** (L7c,
  2026-09-02; `proj[from: p]` as a qualifier on the type, user decision
  2026-09-11): the fn returns a *borrow* of the kept parameter `p` instead
  of an independent value — the relaxation of S1a's "results are always
  independent" rule.
  * Callee: every source in `from` must be a parameter and *kept* (written
    or inferred — a return under the projection is a borrow, not a move
    [deduce-infer]); every returned value must derive from one of the
    sources (its link chain terminates there) or be `None`; returns do not
    consume. A forwarded projected call (`return first(persons)`) validates
    through the same links.
  * Caller: the result fate-links to the arguments in the sources'
    positions, *wholesale* — the ordinary discipline follows (mutating the
    argument poisons the result [fate-poison]; the links carry a
    *borrowed* flag, so move-mode can never take ownership through them
    [fate-move-mode] — moving the result or its narrowed binding is an
    error with the `copy` remedy). The result's *type* carries the
    projection ([proj-type]), which is also what overloading sees.
  * Backends: Kotlin unchanged (the result is the alias). Rust returns
    `&T` / `Option<&T>` / `Union2<&T, …>`: lifetime elision covers a single
    reference parameter; with more, a `'a` is generated onto every source
    parameter and the return — the first deliberate exception to the
    no-lifetimes invariant. std's `first`, `get` and `next` are clone-free.
* [proj-anywhere] `proj[from: a, b]` is a qualifier writable wherever a
  type is (user decisions 2026-09-11): the whole result, a union arm
  (`Emitted (proj[from: p] T) | Finished`), a nullable, a tuple element, a
  type argument (`List<proj T>`) and a struct field. Where it sits decides
  what it means:
  * On the value itself (result, arm, tuple element): a **wholesale**
    projection [readonly-return] — `[from: …]` is required and names kept
    parameters; several sources are one projection of all of them (a
    projection joined across branches is "from both").
  * Inside a type argument (`List<proj T>`) or on a struct field: a borrow
    the value **holds** [proj-field] [proj-infer] — `from` is *not*
    written (the value's, not the type's), and a `proj` type argument is
    allowed only on the intrinsic containers the backends render as such
    (`List`, arrays) [proj-type-arg]; on a user struct it would smuggle a
    borrow into a field through generics.
  * On a parameter, bare `proj T` names the kind of value expected (a
    projection) and behaves as a kept parameter.
  * A *view of a temporary* — a projecting or lending call whose borrowed
    argument is a call result or literal — may be used within its
    statement (`map(iter(list(1, 2)), f)`, `for x in iter(list(1, 2))`)
    but not bound, returned or stored (`let p = iter(list(1, 2))`: "cannot
    bind a view of a temporary"). Rust exposed it (E0716); the rule keeps
    the backends in agreement. A possible later automation (hoisting the
    temporary) is recorded in ROADMAP.
* [proj-readonly] A `proj` value is **read-only whatever its `Mut` says**
  (user decision 2026-09-11): `proj Mut X` is a legal type — the value came
  out of a mutable slot — but it never satisfies a `Mut` position (`Mut X <:
  proj Mut X`, not the reverse); passing it where `Mut` is required, or
  mutating it, reports the projection and the `copy` remedy. `copy` is the
  way out and yields an owned `Mut X`. Moving a wholesale projection is
  refused the same way — except a Copy scalar [copy-scalar-free].
  * A **parameter** written with a top-level `proj Mut` is an error at the
    *declaration* (user decision 2026-09-12): the `proj` promises to accept
    borrowed values, but the `Mut` makes the position one no projection can
    satisfy, and the body could never use the permission either. The
    diagnostic names both remedies (drop the `Mut` — a `proj` position
    accepts `proj Mut` arguments — or drop the `proj`). Nested occurrences
    (`Mut List<proj Mut Str>`) and non-parameter positions (locals, fields,
    returns) keep the type legal.
  * Implementation: the projection lives in the lowered type
    ([proj-type], 2026-09-12; before that it was erased and carried only
    on links) *and* on the fate link — `FateLink.borrowed && !held` is a
    wholesale projection, `held` a borrow an owned object carries
    [proj-infer]. A variable whose every link is held may be mutated (its
    own fields are its own); one with a wholesale or alias link may not.
* [proj-field] **Any struct may hold `proj` fields**, written without a
  source (`items: proj List<T>`): the struct declares *that* it projects,
  each literal decides *what* (user decision 2026-09-11; replaces the
  pass-only exemption that first landed). Such a struct is an owned object
  that *holds* borrows — a **view**: its `Mut` is real (a pass is advanced
  in place), its non-`proj` fields are its own, and it may be moved, stored
  or passed on; what it may not do is outlive its roots. A struct holding a
  view in an owned field is a view too. Writing `proj[from: x]` on a field
  is an error ("names no source").
  * Rust: the struct carries one lifetime, `View<'s>`, with `&'s` fields
    and `<'s>` on owned view-typed fields; every mention elides (`'_`)
    except where a lend ties it [rs-proj-lends]. Kotlin: unchanged.
  * Assigning a `proj` field re-points the borrow; a fn that does so writes
    `=> v.items: proj[from: other]` [proj-infer], and Rust renders the
    assignment as a borrow with the struct's lifetime tied to `other`.
* [proj-infer] **Which parameters a result holds borrows of is inferred**
  (user decision 2026-09-11: "infer what can be inferred; the user writes
  what inference cannot reach"):
  * With a body: read off every returned value — a `proj` field takes the
    roots of what is stored in it; an owned view-typed field, a forwarding
    call, a local, a `!`, a branch are followed (`lends.rs`, memoised per
    declaration, conservative on any shape it cannot follow: every kept
    parameter). Per field, exact.
  * Without a body — an effect member, an intrinsic, a fn type — the
    written entries decide (`=> proj[from: c]`, `=> .items: proj[from: c]`,
    `=>[iter] proj[from: c]`), else conservatively every kept parameter
    (exact for `iter(list)`; only ever over-links).
  * A written projection entry about the result must name every lend the
    body performs; it may name more (a generic body — `filter`'s
    `add(out, x)` with `x` an element of an opaque pass — lends through
    opacity the analysis cannot see, and the entry is how it says so).
    A written entry takes **precedence over the instantiation fallback**
    (2026-09-12): where a substituted return holds `proj` the written
    type does not show (`-> Mut List<T>` with `T = proj Str`), a call
    links the result to *every* kept argument unless the author's
    `=> proj[from: it]` names the lends — then only those link, still
    flowing through a temporary in the named position to its roots.
  * The caller links the result to the lent arguments, *held*
    [proj-readonly]; returning a view rooted in a local is an error ("a
    local that dies with this call"), and a returned view may be rooted
    only in the fn's own lent parameters. A re-pointing entry
    (`v.items: proj[from: other]`) gives the caller's variable at `v` a
    held link to `other`'s roots from the call on (the body is trusted for
    these — see ROADMAP).
  * Reserved, not built: struct-level **link parameters**
    (`struct Pair<T, U, a, b> { first: proj[from: a] T, … }`) as the
    per-field explicit form, should the conservative fallback ever bite;
    `[name-casing]` already makes it parse (ROADMAP).
* [lambda-view] **A capturing lambda is a view** (user decision
  2026-09-12): the closure holds a borrow of every non-Copy capture its
  body only reads, exactly as a struct holds its `proj` fields
  [proj-field] — because a body can return projections rooted in a
  capture (`i -> get(words, i)!` hands out elements of `words`), which no
  fn type can name (`proj[from: …]` sources are parameters; captures are
  unnameable). So the *value* carries the link: binding the lambda links
  it to the captured roots (held, borrowed); a call result linked to a
  lambda argument reaches those roots transitively (which is what makes
  `map(indices, i -> get(all, i)!)` both legal and correctly poisoned by
  a later move of `all`); a capture-free lambda holds nothing, so
  `map(p, w -> w)` binds with no ceremony (a lambda expression is never
  itself a "temporary" a view could dangle from). A capture the closure
  *consumes* is owned, not borrowed — the `once` rule [fate-lambda] —
  and a Copy scalar capture links nothing [copy-scalar-free]. Before
  this rule, naming the lambda (`let f = i -> get(words, i)!`) evaded
  the discipline entirely: `eat(words)` was accepted with the view live.
  * Rust: a capture-rooted projection in a lambda tail stays the borrow
    (`|i| all.get((*i) as usize).unwrap()` returning `&String` into a
    `Vec<&String>`) — no clone [copy-opt-in].
* [yield-proj] A pass that walks data declares `: Yield<self, proj T>` and
  its `next` returns `Emitted (proj[from: p] T) | Finished` — the element is
  a projection of the pass, which projects the source; a generator declares
  `: Yield<self, T>` and emits owned values (user decision 2026-09-11: one
  `Yield` group, the element type argument carrying `proj`). The
  obligation and the member must agree — `proj` on one and not the other
  is an error naming the fix. Reading combinators (`?Yield<It, T>`) accept
  both. std's `ListYield`/`ArrayYield`/`StrYield` borrow (`items: proj
  List<T>`; `StrYield` emits owned `Char`); an `iter fn`'s generated pass
  borrows its subject (`__subject: proj Subject`, `proj` snapshot fields,
  Copy scalars owned) and names the subject as the elements' source
  (`Emitted (proj[from: b] T)`), which the desugar redirects to the pass
  parameter. Rust: the element generic is retagged to `&T` at call sites
  whose filled `next` borrows [rs-proj-arm].
* [copy-opt-in] **A copy never happens without the program opting in**
  (user decision 2026-09-11, the principle behind phase 2b): `copy(x)` where
  a copy is wanted, a `_to` function that fills a destination the caller
  provides (with `?copy` for the element type [copy-implicit]), and nothing
  else. Everything std hands back either owns fresh data (`map`, `split`) or
  projects what it was given (`get`, `first`, `iter`, `filter`, `next`)
  [proj-anywhere]; an `iter fn`'s pass borrows its subject [iter-fn]. A
  backend that would need a hidden clone to be correct reports instead
  [backend-never-wrong] [rs-proj-arm].
* [copy-scalar-free] Moving a derived **Copy scalar** (`Int`, `Long`,
  `Float`, `Double`, `Bool`, `Char`, `Byte` — not `Str`) is a read: the
  number taken out of a borrow is the value itself on both backends, so no
  `copy` is owed (user decision 2026-09-11). Poison still applies. The
  same exemption lets a bodiless declaration leave a scalar parameter out
  of its clause [deduce-syntax], and the hover omits scalars.
* [copy-implicit] `?copy: (v: T) -> T` is an ordinary implicit parameter
  (user decision 2026-09-11): a generic body that must copy a `T` it
  cannot see through — `filter_to`'s `add(dest, copy(x))`, a handler
  holding a `List<T>` it hands out — takes it, and the call site (or the
  `use` site, for a handler constructor) resolves `copy` at the concrete
  type. Handler constructors may take fn-typed implicits (the former
  [implicit-fn-only] restriction is lifted; a non-fn implicit there is
  still an error): `use` fills them, recorded under the `use` span. The
  `_to` family names its copy twice — in `_to`, and in `?copy`.
  * Kotlin: the ctor implicit is a property; `copy(v)` in a member
    dispatches to it; the `use` site passes the type-directed copy
    (`{ __i0 -> __i0 }` for an immutable type) [kt-copy]. Rust: a `Box<dyn
    FnMut(&T) -> T>` field, members call `(self.copy)(&v)`, the `use` site
    passes a `move` adapter; `copy` at a retagged element position is the
    identity [rs-proj-arm].
* [copy-fn] `core.copy` — `intrinsic fn copy<T>(value: proj T) -> T =>
  value` — duplicates a value: the argument is kept untouched, and the
  result is a fresh owned value with no fate links. The parameter says
  `proj T` because a projection is exactly what `copy` is *for* — and
  `X <: proj X` [proj-type] means an owned value is accepted too. The
  binding un-projects one level and keeps the rest: `copy` of a
  `proj Mut Str` is a `Mut Str`. It is the one-word remedy in every fate
  diagnostic.
  * Lowered type-directedly by each backend [intrinsic-fn]; identity
    where no Salvo operation can mutate the value, a real copy where
    one can, and a codegen error where no correct copy exists yet
    [backend-never-wrong].

## Linear types

* [linear-group] A type declares linearity with the **`linear struct`
  modifier**: `linear struct Lines { … }` (user decision 2026-09-12,
  replacing the designated `: Linear<self>` group entry of 2026-09-08,
  which replaced `canbe linear`, L6a 2026-09-02 — the `params Linear`
  group is deleted). The obligation's **discharge set** is every fn
  declared in the *type's own file* whose contract consumes a parameter
  of the type — `close` for a file, `stop`/`join` for a thread,
  `remove(cache, entry)` for a pooled handle (context parameters are
  ordinary parameters; generic dischargers count). Declaring a `linear
  struct` with no discharger is an error at the **struct** ("no legal
  death"), and leak diagnostics enumerate the set.
  * **A discharger is not an exemption**: inside it, the consumed
    parameter still owes, and the obligation must terminate on every
    path — `discard(x)` as the terminal [linear-discard], or a forward
    into another consuming fn (`shutdown(t) { stop(t) }`).
  * **A `close` never implies linearity.** Only the modifier does; a bare
    consuming function of a non-linear type is an ordinary function.
    Attaching an obligation on the strength of a function name is what
    [qual-*] keeps the compiler from doing.
  * **`canbe` does not grant it**: `canbe` means only "may be qualified
    thus" (`canbe Mut`, `canbe once`), and `canbe linear` on a
    *declaration* is an error naming `linear struct`. On a **type
    parameter** it keeps its spelling [linear-generics] — permission on a
    parameter is a different thing from obligation on a declaration.
  * Every value of the type is linear — `linear` still cannot be written
    in a use-site type (a per-value spelling that could be forgotten
    would defeat the protection) [obligation-spelling]. Hover presents
    linearity from the declaration.
  * **Not yet covered**: `intrinsic type` carries no modifier, so a
    linear opaque type has no spelling until `TypeDecl` gains one (nothing
    declares one today).
* [linear-obligation] A linear value carries a *use obligation*: on
  every path it must be moved onward before it goes out of scope
  (decision L6b: consumption = any move, exactly as the deduction
  system defines it — consuming call, `return`, `break` value,
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
  `intrinsic fn discard<T canbe linear>(value: T) -> None` — deliberately => !value
  drops a value, consuming it. For a **linear** value it is the
  obligation's **terminal**, legal only inside a *discharger* of the
  value's type (a same-file consuming fn [linear-group]; user decision
  2026-09-12, revising 2026-09-08's blanket refusal): anywhere else it is
  an error naming the discharge set — dropping a handle elsewhere is
  exactly the leak the obligation exists to prevent. It remains the
  unrestricted escape hatch for non-linear values (decision L6c), and
  std's `fn drop<T>(value: T) -> None => !value` is the *passable* form
  for consuming-callback positions — deliberately without `canbe linear`,
  so a linear argument is refused by the instantiation ban
  [linear-generics]. Lowered per
  backend [intrinsic-fn]: Rust `drop(value)`; Kotlin evaluates and
  ignores (`(value).let {}`). Failure/panic paths are out of scope
  until Salvo has such semantics.
* [linear-union-arm] **A union arm may be linear** (O-C2, user decision
  2026-09-12): a union value *is* the value — one handle, not a box
  holding one — so `fn open(path: Str) -> Ok InputStream | Err Str` is
  legal, the un-narrowed union **owes**, and narrowing settles it:
  narrowed to the linear arm, the value owes as that arm (discharge as
  normal); narrowed to a **non-linear arm, the obligation is
  discharged** — an `Err Str` never held the handle. `T?` falls out
  (`None` owes nothing), which is the optional-handle and lookup shape.
  This is the phase-4 unblocking move (the fallible open).
  * Implementation: the discharge-by-narrowing is a **flow fact**
    (`linear_settled`), set where a narrow lands a linear value on a
    non-linear type, surviving the branch's narrow-restore exactly as
    consumption does, and joined across branches (settled on every path —
    by narrowing or by consuming — means settled after the join). The
    no-`else` fall-through path of an `if` applies the else-narrows'
    settling (`if h is Lines s { close(s) }`: on the untaken path `h`
    is `None`).
  * Backends: nothing — linearity is static and backend-identical
    [linear-static]; the union lowers as any union does.
* [linear-composite] A composite may not **hold** a linear value
  (interim rule, user decision 2026-09-08, roadmap R4 part 2 — replacing
  decision L6d's contagion, under which a composite containing a linear
  component became linear itself; **union arms carved out 2026-09-12**
  [linear-union-arm]). Its obligation would have to travel
  with the container, and composition plus conditional linearity are one
  design question (roadmap L8), so until that is answered a linear value
  lives only in a **local, a parameter or a return value** — or a union
  arm. Refused *at
  the store*, so the container is never built and there is no follow-on
  leak to report:
  * a **struct or handler-state field** whose declared type is linear —
    at the field, naming it (`Holder.handle`);
  * a **written composite** with a linear component, one level at a time
    so a nested one reports at its own node: a type argument
    (`List<Lines>`), an array element (`Lines[]`), a tuple component —
    but **not** a union arm [linear-union-arm];
  * an **array or tuple literal** whose element type is linear (nothing
    written to refuse);
  * a **struct literal** field taking a linear value where the field's
    declared type is generic (`struct Box<T> { item: T }`), which is the
    store the struct's own declaration cannot see;
  * a **call** that takes a bare `T` and puts a `T` inside a composite —
    `add(list: Mut List<T>, elem: T)` is the shape — refused whatever the
    callee's `<T canbe linear>` claims, since no container can carry the
    obligation yet. A signature that only *reads* a composite of `T`
    (`size(list: List<T>) -> Int`) stores nothing and stays legal.
  * Consequently a *union* is not a container: a value of `Lines | Int`
    **is** the linear value, so the obligation survives narrowing and
    branch merges (an inferred union arm keeps it, a written one is the
    refusal above).
  * **Known casualties**, both recorded rather than worked around: a
    linear pass cannot be composed (a wrapper pass over `open_lines("a")`
    stores its source), and the fallible-open shape `Ok InputStream | Err Str`
    is unavailable to S-IO until union arms get an exception or the result
    shape changes.
* [linear-generics] An unconstrained generic parameter cannot be
  instantiated with a linear type (decision L6d): unopted generic code
  does not honor the obligation. A fn opts in *per type parameter* with
  `<T canbe linear>` (decision L7a-syntax, 2026-09-02 — the same
  `canbe linear` phrase as on type declarations, one qualifier per
  `canbe`, the comma separates parameters; spelled `with` until the
  2026-09-03 rename [canbe-optin]). The ban reaches everywhere a generic
  binds (extended 2026-09-12): ordinary calls, **effect members' own
  generics** (`swallow<T>(x: T)` cannot bind a linear `T` — handlers
  never promised to honor it), and **fn values** (a generic fn passed by
  name instantiates from the position's expected fn type
  [fn-value-select], and that instantiation is checked too, which is what
  makes std's `drop` refuse a linear pass while filling the same
  consuming-callback slot for a plain one).
  **Structs opt in the same way** (user decision 2026-09-12):
  `struct Box<T canbe linear>` with `T` reaching a field is a
  **conditional container** — `Box<Lines>` is linear, `Box<Int>` is plain
  — whose discharge set is checked at its declaration like any linear
  struct's [linear-group], and whose obligation **settles by
  decomposition**: once every field through which linearity reaches the
  value has been moved out (`return box.item` in `unbox`), the shell owes
  nothing. A *concrete* linear field takes the `linear struct` marker
  instead [linear-composite].
  * inside the opted fn, `T`-typed values are treated as linear
    (`Ty::Var` counts as linear), so the body is
    checked under the worst case — including that forwarding an opted
    `T` to an unopted generic is an error (compositional);
  * for bodiless intrinsics the opt-in is a trusted audit claim; std's
    audit opts in `list`, `mutable_list`, `add`, `size`, and `discard`
    (whose declaration is now honestly
    `intrinsic fn discard<T canbe linear>(value: T) -> None` — no => !value
    blessed-by-name special case), while `get` stays out (returns an
    alias of an element) and `copy` refuses with a dedicated message
    (duplicating an obligation is meaningless). The opt-ins are
    **signatures, not permissions to store**: since R4 part 2 a call that
    would put a linear value into `List<T>` is refused by
    [linear-composite] regardless, so the audited surface now means only
    "these may be *called* with a linear `T`" — `size` of a list of them,
    never a list of them;
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
  destructors — a value's own `close` [linear-group] is the only way it
  legally dies without being passed on.

## Modules, imports, files

* [mod-file] `.sv` files are modules; the module path is the file path (no
  in-file module declaration).
* [mod-file-name] A `.sv` file name's stem may not contain a dot: the
  module path comes from the directory layout [mod-file], so `list.ext.sv`
  would be indistinguishable from `list/ext.sv`. `SourceSet::classify`
  rejects it, naming both spellings in full. This is the double-extension
  spelling that used to pick a backend's per-backend template file — it was
  skipped in silence then, and is reported now, so a leftover file beside
  the sources can no longer vanish from a build.
* [mod-ignore] Source discovery walks the source root recursively but
  skips: hidden directories (`.git`, ...), cache directories carrying a
  `CACHEDIR.TAG` marker (Cargo's `target/`), and anything listed in
  `<root>/.svignore` — one path per line, relative to the root, naming a
  file or a directory subtree; blank lines and `#` comments ignored.
  The root itself is exempt from the hidden/cache rules.
* [mod-visibility] Code sees: everything declared in its own module (all
  its `.sv` files), everything in `core.*` (implicit), and whatever it
  imports.
* [mod-import] `import path.Name` / `import path.Name as Alias`; aliasing
  resolves ambiguity. Unresolved/ambiguous imports are errors.
  * Import prefixes match module paths exactly or as a leading path
    (`import core.Str` finds `core.string`).
* [obligation-spelling] **Obligations are lowercase keywords** (user
  decision 2026-09-12): `proj`, `once`, `linear` — reserved words, written
  in qualifier position (`proj NonEmpty List<T>`, `once (A) -> B`,
  `proj[from: p]`) but visually distinct from user qualifiers, the way
  `provenance` marks its declaration form. The lowercase marks the closed
  set of compiler-owned behaviors: a qualifier *narrows* and may be
  dropped; an obligation *widens* (`T <: proj T`, `T <: once T`) and never
  drops — the reader should not have to learn the direction per name.
  Bounds follow (`T canbe linear`, and `q canbe once` stays available as
  the future multiplicity-variable spelling); `canbe Mut` and every other
  permission stay uppercase. `linear` is never written in a use-site type
  ([linear-group]'s rule, now enforced on the keyword); it appears in
  declarations and bounds only.
* [name-casing] Casing is a *rule*, not a convention (N1a, user decision
  2026-09-03): names of types — structs, qualifiers, type declarations
  and aliases, effects, handlers, generic parameters — start with an
  uppercase letter; names of values — fns, parameters, fields,
  variables, bindings, lambda parameters — do not. The three obligation
  *keywords* (`proj`, `once`, `linear` [obligation-spelling]) are the
  deliberate exception in type positions: reserved words, not names, so
  they collide with nothing. Module path segments
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
    aliases, `intrinsic type`s, effects (effect lists are
    written as type refs), plus generic parameters in scope and the
    language-level `None`, which has no declaration. Qualifiers:
    declared qualifiers plus the compiler's intrinsic `Mut`, `Linear`,
    `once` (`proj` is parsed as part of the return annotation, never
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
  `is` checks, `as Q` constructors, `canbe` clauses, deduction clauses,
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
    types, and `ty_base_name` / `type_base_name` agree by construction.
    Backends translate at the point
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
  * A module's backend companion files [backend-companion] travel with it;
    a reachable module is emitted only if it produces code.

## Backends

* [backend-intrinsic] `intrinsic` declarations (types) are
  mapped inside the compiler; every backend must handle all of them
  (`Str`, numeric types, `List<T>`, ...). Since 2026-09-05
  there are **no separate template files**: each backend carries its
  lowerings as code, in its own `intrinsics.rs` (`type_name`, `fn_call`,
  `handler_member`), and the `Mut` auto-qualifier maps through
  `mut_type_name` where the backend needs a different native type
  (Kotlin `MutableList<T>`; Rust erases it, since mutability lives in the
  binding) [type-canbe-mut].
  * `intrinsic` is the *compiler's* modifier and std-only
    [intrinsic-std-only]. An intrinsic the backend has no lowering for is
    a codegen error naming it, so the restriction is largely
    self-enforcing.
  * Dispatch is on the **checker-resolved declaration** — the declaration
    name plus the base type name of its first parameter — which is what
    separates std's overloads (`size(Str)`, `size(List<T>)`, `size(T[])`)
    without the arity guessing the older per-backend template scheme
    needed.
* [intrinsic-std-only] `intrinsic` is the compiler's own modifier, so only
  the standard library may write it (user decision 2026-09-05): a customer
  declaring `intrinsic` names an implementation the compiler does not have.
  Enforced in the checker (`check_intrinsic_is_std_only`), not the parser,
  which sees one file's tokens and cannot tell where the file came from —
  the checker has `SourceFile::is_std` at hand. Not cosmetic: the backends
  dispatch intrinsics from a table keyed by *name* [backend-intrinsic], so
  an intrinsic the compiler does not already know has no lowering anywhere.
  The diagnostic names the one interop path customer code *does* have — a
  member of a `platform effect` [platform-effect]. Applies to
  `intrinsic fn`, `intrinsic type`, `intrinsic handler`, and
  `intrinsic qualifier` alike.
* [intrinsic-fn] `intrinsic fn` declares a compiler-intrinsic function:
  the declaration carries the signature and deduction clause the checker
  uses (body-less), and each backend lowers calls to it directly, seeing
  the checker's resolved argument type at every call site — type-directed
  lowering a single generic template could not express. An intrinsic fn a
  backend does not implement, or an argument type it cannot lower, is a
  codegen error [backend-never-wrong].
  * Two kinds live here. `copy` [copy-fn] and `discard`
    [linear-discard] dispatch on the argument's *type or shape*, which is
    the whole reason they are intrinsics, and each backend lowers them
    itself. Everything else std declares — the `core.list`, `core.array`
    and `core.string` surface — is a table entry.
  * A call's type arguments reach the lowering ([call-type-args]), which
    is how Kotlin's list constructors spell out an element type kotlinc
    cannot infer from an empty argument list (`mutableListOf<Int>()`).
  * Rust keeps the place-vs-owned distinction at the argument boundary
    [rs-borrows]: a place splices raw so a method-style lowering borrows
    natively (`list.push(..)`), while a variadic tail splices owned
    because it lands inside a constructor (`vec![..]`).
* [platform-effect] `platform effect E { members }` declares an effect
  whose members the **host** implements, in the target language (user
  decisions 2026-09-05). The compiler generates the interface (Kotlin) or
  trait (Rust) exactly as it does for an ordinary effect; what differs is
  where the implementation comes from. This is the *only* interop path for
  customer code.
  * It is an ordinary effect in every other respect: a function that
    performs a member declares `[E]`, intermediate frames declare it and
    thread it, and the existing interface/trait emission, `&mut dyn`
    threading and handler fusion all apply unchanged. Grouping the
    functions under an effect — rather than declaring them one by one — is
    what makes the interop boundary the author's choice and gives the
    dependency wiring somewhere to live.
  * **The host owns the entry point.** A platform effect's instance cannot
    be `use`d, because it is not constructed in Salvo. Instead the effects
    `main` declares are its *parameters*: `main` is emitted as `salvoMain`
    / `salvo_main` taking them, and the host's own `main` constructs the
    implementations and calls it. A program with no platform effect is
    unaffected — `main` stays `main`.
  * A Salvo `handler H of E` for a platform effect is an error: the host's
    implementation *is* the handler, so a Salvo one would be a second,
    unreachable implementation. The diagnostic names the remedy (declare an
    ordinary `effect` if you meant to handle it in Salvo).
  * A handler may **depend** on a platform effect (a constructor parameter
    of effect type, [effect-handler-deps]): that is how a Salvo-written
    handler reaches the host.
  * Neither the effect nor its members may be generic. A generic member is
    already a loud codegen error on Rust [effect-member-generics], and a
    generic *effect* would need the host to implement one interface per
    instantiation — Kotlin's facets exist for that [kt-effect-fusion] and
    Rust has no equivalent, so it is refused at the declaration rather than
    at codegen [backend-never-wrong].
  * `platform` takes nothing but `effect`. A platform *type* and a platform
    *handler* are deferred (user decision 2026-09-05), and the parse error
    names the form rather than reporting a bare "expected item".
* [platform-tree] The host implementations live in the source root's
  **`platform/` tree**, mirroring the source layout: `platform/app/entry.kt`
  implements the platform effects of module `app.entry` in Kotlin,
  `platform/app/entry.rs` does it in Rust. `salvo platform generate` writes
  them [cli-platform]. They are ordinary companion files
  [backend-companion] — discovered by the active backend's native extension,
  copied verbatim, gated on their module being reachable — with these
  differences.
  * The leading `platform/` is **stripped** when the file is attributed to a
    module. Without that, the file's module would be `platform.app.entry`,
    which no Salvo module is ever called, so it would never be reachable and
    never be copied. Only the source root's `platform/` is special: a nested
    one is an ordinary directory, and a module *named* `platform` keeps its
    own companions.
  * The host file is where the program's entry point lives, so the backends
    single it out: Kotlin launches its facade class, Rust mounts it under
    `platform_<module>` and delegates the crate root's `fn main` to it.
  * A module whose `main` needs a platform effect **must** have a host file:
    otherwise the program has no entry point at all, and the error names
    `salvo platform generate` rather than leaving the target toolchain to
    report a missing `main` against generated code [backend-never-wrong].
  * Both backends' host files coexist in one tree, because discovery only
    ever picks up the active backend's extension — the same sources build
    for both targets.
* [effect-member-unique] A member name identifies its effect program-wide,
  so no two effects may declare the same member name, and no effect may
  declare one twice (user decision 2026-09-05). Before this, a collision
  resolved to whichever effect was collected last and surfaced downstream
  as a baffling "no handler for effect" — and there is no syntax to
  disambiguate, since `emit<Logger>(…)` parses as *member* type arguments.
  * Checked in **source order** and reported at the second declaration, so
    the diagnostic is deterministic and fires exactly once per collision.
    The `Symbols` maps cannot serve here: they are hash-ordered and
    last-wins.
* [backend-companion] A backend-native source file next to a module's
  sources (`complicated.kt` beside `complicated.sv`, using the backend's
  native extension) is a *companion*: it is copied verbatim into the
  output whenever its module is reachable. It is how hand-written native
  code joins the build — most importantly the host implementations of a
  `platform effect`, which live in companions under the source root's
  `platform/` tree [platform-tree]. A companion that collides with a
  generated file is an error.
* [backend-never-wrong] A backend must never emit silently wrong code:
  unsupported constructs are codegen/checker errors. Each backend spec
  lists its current deliberate cuts.

## Comments and documentation

* [lsp-fn-origin] Hover on a **fn** names the module its resolved overload
  came from, and what kind of place that is — the standard library, another
  module, or the file being edited (user request 2026-09-11). With overloading
  by scope ladder [fn-overload-scope] this is load-bearing rather than
  decoration: `size` may be std's, an import's or the module's own, and the
  signature alone does not say which won. The module path is what an
  `@module` selector would name [fn-overload-at], so it is directly
  actionable. Other declarations carry the same section, but only when they
  come from a *different* file — being told a local declaration is local is
  noise.
* [lsp-name-positions] Every *name position* resolves to its declaration for
  hover and go-to-definition, including the two that did not until
  2026-09-11: the group name in a struct's **obligation clause**
  (`: Yield<self, T>`), and the type or qualifier name in an **`is` check**
  (`i is Positive`). Before, hovering the latter fell through to the
  enclosing expression and reported only its `Bool`.
* [doc-qualifies-body] Hover on a **predicate qualifier** shows the
  *condition* it holds under, when its `qualifies` is a single
  `return <expression>` — the expression alone, inline (`Holds when
  `int > 0`.`). A one-line predicate *is* the rule, so showing it saves a
  jump; anything longer is an implementation the reader did not ask for and
  is hidden, leaving the qualifier's own doc comment to explain it (user
  decision 2026-09-11, narrowing an earlier five-line rule). Nothing extra
  is shown for a body that is not exactly one `return`, an expression that
  does not fit on one line, or a qualifier with no `qualifies` at all (a
  constructive or provenance one).
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
  * Backing modifiers do not interfere: `intrinsic` and
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
  * The severity is load-bearing, not decorative: a *warning* reports
    something the author probably did not intend without rejecting the
    program, which is what a suppressed refinement conflict needs
    [qual-refn-conflict]. `salvo analyze` counts warnings separately and
    exits 0 when there are no errors.
* [fn-ref-table] The checker records every fn-*name* reference —
  declaration names, call-site callees (incl. dot-notation), and
  fn-by-name uses — as `Checked::fn_refs: (file, name span) -> FnKey`.
  The LSP's hover renders the referenced declaration as a full
  source-like signature with an explicit return type (`None` when
  omitted) and the *effective* deduction clause in the `=>` spelling: the
  inferred/validated one (`Checked::deductions` [deduce-infer]) when
  available, else as declared. Consumed parameters render as `!p`; kept
  whole ones, and Copy scalars, are omitted; lends render as `proj[from:
  …]`; fn-type groups as `=>[f] …`. Effect-member calls
  have no `FnKey` and are not recorded.
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
* [cli-analyze] `salvo analyze --src DIR [--format text|json]`
  runs the front half of the pipeline — parse,
  resolve, type-check — and reports every diagnostic without generating
  code. Exit code is nonzero iff any diagnostic is an error.
  * Analysis is **unconditionally** backend-neutral: nothing
    backend-specific remains to load, so there is no `--backend` option.
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
* [cli-run] `salvo run --backend NAME (--src DIR | --main FILE)
  [--target DIR] [--clean-target before|both]` compiles and then runs the
  program with the backend's own toolchain (user decisions 2026-09-05).
  One command from `.sv` source to program output.
  * `--backend` is **required**: it decides which toolchain must be
    installed, so there is no sensible default (unlike `compile`, which
    defaults to `kotlin`).
  * **`--src` and `--main` are independent; at least one is required**, and
    each supplies a reasonable default for the other (user decision
    2026-09-05):
    * `--src DIR` alone compiles the directory and uses the unique `main`
      it declares (several is a warning naming them, and the first wins).
    * `--main FILE` alone additionally implies `--src $(dirname FILE)`.
    * **Both** is the only way to say "compile this tree, start at this
      file" when the entry sits in a *subdirectory* — which is also when it
      matters, since a module shared from above the entry's own directory
      is out of reach for `--main` alone. The file must be inside the
      source directory, at any depth; outside it is an error, not an
      ignored flag.
    * Either way `--main` is how you choose between several entry points; a
      named file that declares no `main` with a body is an error naming it.
    * The entry choice reaches the backend, because it can shape the
      *output* and not just the launch command: Rust gives the
      `main`-declaring module the crate root [rs-crate].
    * Considered and dropped as premature (user decision 2026-09-05):
      `--main` including only the modules its imports transitively reach.
      The whole directory is compiled either way; reachability already
      prunes the *output* to the modules actually used.
  * `--target` defaults to `.salvo_tmp_run` in the working directory.
    Dot-prefixed on purpose: source discovery skips hidden directories
    [mod-ignore], so the default works even though it normally lands
    inside the source tree.
  * **The target may not overlap the sources**, for two separate reasons,
    both errors before anything is built or deleted:
    * It may not *be* or *contain* the source directory — the target is
      deleted before the build, so that would delete the program.
    * It may not sit *visibly* inside the sources, where the next build
      would read the emitted files back: output carrying the backend's
      native extension is indistinguishable from a hand-written companion
      file [backend-companion]. Nesting is allowed under a dot-prefixed
      directory, which discovery skips.
    * Independently, clearing a target that holds any `.sv` file is
      refused — a guard on the deletion itself, not on the paths.
  * `--clean-target` (default `both`) says what survives. Both modes clear
    the target *before* the build, so a run never picks up the previous
    run's output; `before` leaves the generated sources for inspection,
    `both` deletes them after the run. A failed *build* leaves the target
    alone (there was no run to clean up after, and the output is what one
    would want to look at).
  * **The command's exit code is the program's**, so `salvo run` can stand
    in for running the binary. Program stdio is inherited rather than
    captured. A toolchain that is not installed, or that fails, is an
    error from the command itself.
* [cli-lsp] `salvo lsp` starts a language server speaking LSP over stdio.
  The workspace root comes from the client's
  `initialize` request. Like [cli-analyze] the server is unconditionally
  backend-neutral — there is no `--backend` option.
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
  * A fate-linked (derived) variable hovers as `proj T` — a bare
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
* [cli-platform] `salvo platform generate --backend NAME (--src DIR |
  --main FILE)` writes the host implementation skeleton for every
  `platform effect` into `<src>/platform/` [platform-tree]: a named class
  (Kotlin) or unit struct (Rust) per effect, implementing the generated
  interface with every member stubbed (`TODO` / `todo!`), plus — in the
  module whose `main` needs a platform effect — the `main` that constructs
  the implementations and calls the generated entry point. `--src` and
  `--main` behave as in `salvo run` [cli-run].
  * **Generated once, never overwritten.** An existing file is reported and
    left alone. This is what the interface framing bought: because Salvo and
    the host meet at a generated interface, every later divergence is a
    *target-language* compile error — a member added is "does not implement
    abstract member" / `E0046`, one removed is "overrides nothing" /
    `E0407`, a changed signature is an ordinary type error, a new platform
    effect breaks the entry-point call — so there is nothing to merge, no
    marker regions, and no canonical name to key them by.
  * The skeleton is rendered by the *same* code that emits the interface, on
    the same checked program, so a skeleton that does not match the
    interface it implements is impossible by construction.
  * A program with no platform effect generates nothing, and says so.
