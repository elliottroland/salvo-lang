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
  `Bool`, `Char`, `None` (unit/no-value singleton), `Str`, `Bytes`.
  * Declared as `intrinsic type` in `std/core/basic.sv` /
    `std/core/string.sv` / `std/core/bytes.sv`; each backend maps them
    natively.
* [byte-value] A `Byte` is an **unsigned octet** (0..255) on every backend:
  `u8` on Rust, `UByte` on Kotlin ([kt-byte-unsigned]) — *not* the JVM's
  signed `Byte`, which would render the same octet as `-1` where Rust
  renders `255` [backend-parity].
  * It is **not operator-numeric** [op-arith]: `to_int(b)` widens into
    0..255 and `to_byte(n)` narrows keeping the low 8 bits (300 → 44,
    -1 → 255, identically on both backends), so byte arithmetic is written
    through `Int` and the wrapping is stated rather than implied.
  * It interpolates natively (as an unsigned number) and compares with
    `==` like any other primitive.
  * The text bridge is `to_bytes(str) -> Bytes` (UTF-8) and
    `str_of_bytes(data) -> Str?` (strict UTF-8: invalid bytes are `None`,
    never a replacement character) [bytes-type].
* [bytes-type] **`Bytes` is std's byte buffer** (`core.bytes`, user decision
  2026-09-15), and the payload type of every byte-shaped API — *not*
  `List<Byte>`, which boxed every octet on the JVM and carried a list's
  surface rather than a buffer's.
  * Two shapes, exactly `Str`/`Mut Str`: `Bytes` is read (`size`, `get`,
    `slice`, `index_of`, `to_str`, `to_hex`, `str_of_bytes`, `iter`) and
    `Mut Bytes` is built (`add`, `append`, `set`, `clear`), reaching the read
    surface by **dropping `Mut`** — free on both backends here, unlike
    `Mut Str` [str-drop-mut]. Constructors: `bytes_of(...)` (fixed) and
    `mut_bytes(...)` (a builder, empty or over given parts), mirroring
    `mut_str`/`mut_list_of` [col-literal].
  * `slice` **copies**; nothing in the surface borrows, since a borrow would
    have to be stated in a deduction [proj-field]. Out-of-range reads answer
    `None` and `set` out of range does nothing (growing there would make it
    an `add`).
  * `==` is **structural** on both backends, and `copy` is a real copy — the
    Kotlin buffer is one mutable object for both shapes [kt-bytes], so
    identity would alias it.
  * A buffer is **not hashable**: `Set<Bytes>`/`Map<Bytes, V>` are refused
    [col-hashed-ordered], because a key that can be mutated under its map is
    a bug no diagnostic would catch later.
  * `for b in data` iterates the bytes natively on both backends
    [iter-for-native]; `BytesYield` is the pass the combinators drive
    [iter-protocol].
* [lit-numeric] Numeric literals: `1` is `Int`; `1L` is `Long`; `1.2` is
  `Double`; `1.2f` is `Float`. Underscore separators are allowed anywhere
  inside the digits, before *and* after the decimal point (`1_000L`,
  `1_000.500_5`; the fractional half was added 2026-09-18 at the user's
  request, having been an "invalid numeric literal" until then). They are
  stripped before the value is parsed, so a backend never sees one.
  * The `f` suffix requires a decimal point (`1f` is a lex error telling
    you to write `1.0f`); `L` forbids one (`1.2L` is a lex error); a
    literal running into identifier characters (`10x`, `1.2fx`) is a lex
    error. `1.size()` still lexes as an int followed by a method call.
  * Backends: Kotlin renders the suffixes as its own (`1L`, `1.2f`);
    Rust renders explicit types (`1i64`, `1.2f32`) and leaves unsuffixed
    literals bare for inference.
* [lit-adopt] An **unsuffixed** numeric literal adopts the numeric type
  its position expects (user decision 2026-09-14, replacing the earlier
  "no implicit widenings — write `1L`" rule): `let x: Long = 1`,
  `let d: Double = 3`, a `Long` call argument, and `T?` positions through
  the sole non-`None` arm all adopt. A written suffix never adopts, an
  integer literal never adopts `Int`-ward (a float literal cannot become
  an integer), and *variables* never widen implicitly — only literals,
  and only where a numeric expectation exists (operator operands need
  none: `x + 1` widens per [op-promote]).
  * Backends render the adopted type explicitly (`1i64`/`1L`, `3f64`/
    `3.0`, `0.5f32`/`0.5f`), since neither target adopts everywhere Salvo
    does (Kotlin refuses a bare `1` for a `Long` *parameter*).
  * **A call argument adopts from the candidates, as a fallback** (fixed
    2026-09-18, when `millis(500)` was refused as `millis(Int)` — the rule
    named that case from the start and only annotated `let`s and struct
    fields honoured it). Adoption must never re-rank an overload set
    [fn-overload-rank], so a candidate whose parameter *is* the literal's own
    type (`Int` for an integer literal) blocks it, and the candidates must
    name exactly one numeric type between them for it to apply. Non-numeric
    and generic parameters neither block nor supply a target, so `f(9)` over
    `f(Long)`/`f(Str)` adopts `Long`.
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
  observably. `Unknown`/`Never` operands stay lenient
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
  * **Any arity** (2026-09-18). Rust's tuples are native; Kotlin has `Pair`
    and `Triple` and nothing past them, so the backend **generates** a tuple
    class per arity a program names [kt-tuple-class] — the same answer it
    already gives for union wrappers, rather than the codegen error the size
    used to be.
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
* [elvis] `subject ?: rhs` **picks the non-`None` arms** of its subject (user
  decision 2026-09-21, step 4 of the `?` family sequence): the expression is
  that value when the subject has one, and `rhs` otherwise. Reserved for `T?` —
  a *qualifier* is picked by naming it (step 5) — and it keys on the **presence
  of a `None` arm**, not on the `?` spelling, so `Str | None` written longhand
  works and `Str | Int | None` picks `Str | Int`.
  * **Precedence is Kotlin's**: tighter than `is`/comparison/equality, looser
    than additive, right-associative. `m[k] ?: 0 > 3` is `(m[k] ?: 0) > 3`;
    `a ?: b + 1` is `a ?: (b + 1)`; `a ?: b ?: c` is `a ?: (b ?: c)`. Borrowed
    with the spelling, so a reader who knows Kotlin's `?:` learns no new
    binding.
  * **The subject is evaluated once**, and the form needs no position
    restriction (unlike a call subject in an `is` [loop-while-is]): the
    temporary is internal to the lowering.
  * **The right side is an ordinary expression**, which since [expr-escape]
    includes `return`/`break`/`continue` — that is what step 2 was for. A
    diverging right side contributes nothing to the type, so
    `maybe_t() ?: return None` has the picked type exactly.
  * **The two sides must agree on a representation**, as an `if`'s branches do:
    the right side is coerced into the join, so a bare `Str` wraps into the
    subject's `Str | Int`. A right side that *widens* the join past the
    subject's own arms (`Int? ?: "none"`) is **refused for now**: the picked
    value would need wrapping too and has no span of its own to hang a
    coercion on. `when` says it today.
  * A subject with no `None` arm is an error (nothing can take the right side,
    the same refusal a throw-free `try` gets [try]); so is a subject that is
    only `None`.
  * Backends: Kotlin's own `?:`, since the `T?` representation *is* a Kotlin
    nullable [kt-elvis]; on Rust a `match` on the `Option` — a `match` rather
    than `unwrap_or_else` precisely because the right side may escape, and a
    `return` inside a closure would return from the closure [rs-elvis].
* [pick] `subject Qual?: rhs` is the **qualifier form** of `?:` (user decision
  2026-09-21, step 5): the named arm is *picked* — it becomes the expression's
  value — and everything the pick did not claim goes to the right-hand side,
  where `_` names it [placeholder].
  * **`^Qual` lifts, `Qual` keeps.** `r ^Ok?: …` yields `T`, `r Ok?: …` yields
    `Ok T`. The same `^Q` notation an `is` check uses [qual-lift], which is why
    step 3 came first: one mark, two places.
  * **`_` carries its tags** (user decision 2026-09-21): the unpicked arm reads
    as `Err Str`, not `Str`, so the right side can route on it. There is no way
    to strip a tag here — replacing one tag with another is an `if` or a `when`,
    and `err(_)` on a `Thrown Str` honestly gives `Err Thrown Str`.
  * **Only the last `?:` has a right-hand side**, so a chain of picks reads as
    "these arms are answers too". *Not built yet*: a chain of more than one pick
    (`r ^Ok?: Err?: err(_)`), which needs the multi-arm work below.
  * **The subject is evaluated once**, into a temporary registered exactly as a
    hoisted `is` subject is [is-bind-once] — so, unlike `?.`, the subject need
    not be a place.
  * **A pick that matches nothing** can never run; one that matches
    *everything* leaves the right side dead. Both are errors, the same "dead
    scaffolding" refusal a throw-free `try` gets [try]. A pick naming a *type*
    rather than a qualifier is an error too.
  * **First slice: one matched arm, one left.** A pick spanning several arms on
    either side needs the arm-mapping re-wrap [let-infer] — the same one step
    3's multi-arm lift waits on — so it is refused by name rather than emitted
    wrong [backend-never-wrong]. That single lift unblocks three deferred
    shapes at once: the multi-arm lift, the multi-arm pick, and pick chains.
* [safe-call] `receiver?.member` / `receiver?.member(args)` reaches a field or
  a dot-notation function on the **non-`None`** side of an optional (user
  decision 2026-09-21, step 4). The result is the member's own type **plus
  `None`**, so a chain re-tests at each link and composes with `?:`
  (`p.home?.city ?: "-"`). Reserved for `T?`, like `?:`.
  * **The member is typed by the ordinary path.** The node holds the equivalent
    plain access (`Field`, or a `Call` whose callee is one), so field overrides,
    overload resolution, effects and diagnostics are exactly those of
    `member(receiver, args)`. `?.` adds the condition and the `None` arm, and
    nothing else.
  * **The receiver must be a place** — a variable or a field of one. The form
    reads it twice, once to test and once to reach the member, which is the rule
    (and the remedy: bind it with `let`) a call subject in an `is` already has
    [loop-while-is].
  * **A member that is already optional is not wrapped twice**: `p?.zip` on a
    `Str?` field is `Str?`, because the result's `None` arms dedupe
    [union-arm-identity]. Which of the two cases applied is recorded for the
    emitters, since the inner access shares the operator's span.
  * A receiver with no `None` arm is an error naming the remedy (drop the `?`);
    so is one that is only `None`.
  * Backends: Kotlin cannot use its own `?.`, because Salvo's dot-notation is a
    *free function* call — `xs?.size()` is `size(xs)` guarded on `xs` — so it
    emits a conditional whose arms are the narrowed read and `null`
    [kt-safe-call]. Rust emits the same shape over the `Option`
    [rs-safe-call].
* [elvis-guard] A `?:` (plain or a pick) whose right side **leaves** narrows its
  subject on the path below (user decision 2026-09-21): the value was there, or
  the code would not be running. The same facts an `is` guard leaves
  [is-narrow-guard], reached from an expression rather than a condition, and
  installed only when the subject is a narrowable place [flow-place].
  * **Only when the right side diverges.** One that yields a value leaves the
    subject possibly-unpicked, since that is the path it took.
  * **A pick narrows to the matched *arm*, tag included** — not to the lifted
    type. `r ^Ok?: return` leaves `r` an `Ok Int`: the lift applies to the value
    the expression produced, while the place still holds the tagged arm. Naming
    the lifted type there would name a type matching no arm of the storage,
    which is exactly what the Rust backend reported when it did.
  * Consequence for the lowering: the picked value is **read** out of the
    subject, not moved out of it, since the subject is still live below. A place
    subject is therefore not hoisted into a temporary at all — reading one twice
    is free [loop-while-is] — while any other subject is, once.
* [placeholder] `_` reads as **the value the enclosing construct left
  unnamed**, and **no construct binds one yet** (user decision 2026-09-21): a
  plain `?:` leaves `None`, which the program can already write, so a
  placeholder there would name a value that has a name. It earns its keep where
  a *qualifier* is picked and the unpicked side carries a tag (step 5), which is
  where `x Ok?: err(_)` needs it.
  * It is not a name: it cannot be declared, shadowed or captured. The word is
    reserved, and appeared in no `.sv` source when it was.
  * Reading it is an **error** until a construct binds it, not a silent
    `Unknown`.
  * Considered and rejected for now (user decision 2026-09-21): a general rule
    covering other "single unnamed value" scopes. `waitfor` was the candidate
    and it wants ordinary **binder inference** instead — `waitfor out { … }`
    with the `Reply<T>` inferred from where `out` is used — so `_` stays one
    piece of the `?:` form rather than a rule about scopes. Single-parameter
    lambdas stay out too: `i -> f(i)` already says it, and Scala's placeholder
    scoping is the cautionary case.
* [type-none-unit] `None` is **one spelling for two things** — the absent
  arm of a `T?` and the sole value of the `None` type — and the targets
  spell them differently (Rust `None` vs `()`, Kotlin `null` vs `Unit`),
  so which one a `None` *expression* lowers to comes from its **slot**, not
  from the expression. The checker records `Coercion::NoneUnit` where a
  `None` literal fills a slot whose type *is* `None`, and the emitters
  render the target's unit value there.
  * The shape that needs it is a generic argument the call inferred as
    `None`: `ok(None)` building an `Ok None | Err E` (phase 4's
    `close`/`flush`/`delete` all return one), `emitted(None)` for a
    sequence of optionals. Found 2026-09-14 — before the rule both
    backends emitted an optional into a unit-typed union arm, which
    neither target compiles.
  * A `return None` from a `-> None` fn never needed it: the return
    statement renders no value at all.
* [type-array] `T[]` is an array; `arr[i]` is 0-indexed; size via
  `size()`. It has **no literal syntax and no generator syntax** since
  2026-09-13 — `[1, 2, 3]` is a `List` [col-literal] — so `array_of(...)`
  and `array_by(n, init)` [col-by] are how one is built, and an array's
  remaining reason to exist is the variadic boundary (`...elems: T[]`).
  * The `Int[5] { i: Int -> 0 }` generator *form* was deleted with its
    `ArrayInit` node (user decision 2026-09-13): it had been silently
    broken on the Rust backend since before the collections work — the
    closure yielded `()` and the enclosing effect handler was spliced into
    it — and `array_by` says the same thing through the ordinary intrinsic
    path. Deleting beat fixing because the form bought nothing the
    constructor does not.
  * std's `core.array` mirrors `core.list`'s function surface minus
    mutation (arrays are fixed-size): `array_of`, `size`, `get`, `first`,
    `iter` (user decision 2026-09-02). `for` iterates arrays natively —
    `iter_elem_ty` handles `Ty::Array` before the implicit-`iter` lookup.
* [col-by] Every collection has a **generated constructor**: `*_by(size,
  init)` builds `size` elements by calling `init` once per index, in order —
  `array_by`, `list_by`/`mut_list_by`, `set_by`/`mut_set_by` (duplicates
  collapse, so the result may be smaller), `map_by`/`mut_map_by` (the
  callback returns a `(K, V)` pair; a repeated key takes its last value).
  * On Kotlin the callback is handed to a builder (`Array(n, init)`,
    `MutableList(n, init)`) or to `.map(init)` rather than being invoked
    inline: an immediately applied lambda literal has no expected type, and
    kotlinc then demands an explicit parameter type.
* [col-convert] Converters between the collections: `to_list` (from a set or
  sorted set), `to_set` (from a list — duplicates collapse, first-appearance
  order), and **two `to_map` forms** (user decision 2026-09-12): from a list
  of pairs, and from a list of anything plus a rule
  (`to_map(words, w -> (w, size(w)))`). Duplicate keys are last-wins
  throughout [col-duplicate-keys].
  * **Known limitation**: a *bare inline literal* argument to one of these
    does not determine the callee's type parameter
    (`to_set([1, 2])` — "cannot infer type argument `T`"), because the
    literal's own element types are not propagated back into the
    instantiation. Binding it first (`let xs = [1, 2]; to_set(xs)`),
    annotating the result, or a nested call all work, and the diagnostic
    names the remedies. Recorded in ROADMAP.md.
* [col-nonempty] std declares `qualifier NonEmpty<T> of List<T>` in
  `core.list` (2026-09-13), with a `qualifies` of `size(list) > 0`, a
  by-construction constructor `non_empty_list(first, ...rest)`, and a
  `first(list: NonEmpty List<T>) -> proj[from: list] T` overload that drops
  the optional — ranked above the plain `first` by [fn-overload-rank].
  * A **refinement** `refn add(list: Mut List<T>, elem: T) => list: +NonEmpty`
    establishes the claim, because `add` itself may not [qual-refn].
    Consequence for user code: another qualifier refining `add` over a `List`
    now *disagrees* with std's, so neither applies and the call warns
    [qual-refn-conflict]. The remedy is one word — `with NonEmpty` on the
    user's qualifier — which the diagnostic names.
  * The overload delegates to `get(list, 0)!`, **not** to `first@core.list`:
    the scope selector names the module, and within it a `NonEmpty` argument
    re-picks this same overload, which recurses forever.
  * The constructor is an `intrinsic` only because mixing a plain argument
    with a `...spread` in one variadic call is unsupported, so
    `list_of(first, ...rest)` cannot be its body (recorded in ROADMAP.md).
  * **One name, one subject type.** A qualifier name is unique within a module
    *and* across the implicitly visible `core` modules, so there is no
    `NonEmpty` over `Set`/`Map`/`SortedSet`/`SortedMap`; that needs same-name
    different-subject qualifiers, a DECISION in ROADMAP.md.
* [col-sorted-list] std declares `qualifier Sorted<T> of List<T>` in
  `core.list` — a **state claim** over a list, and a different mechanic from
  the `SortedSet`/`SortedMap` types [col-sorted], which are a representation.
  A `Sorted List<T>` still reaches the whole list surface.
  * **No `qualifies`**, so no `is Sorted`: deciding whether a list happens to
    be sorted compares its elements, which nothing can do over an
    unconstrained `T`. It is minted by `sort` / `mut_sort` and nowhere else.
  * `add_sorted(list: Mut Sorted List<T>, elem: T) => list: Mut Sorted, !elem`
    inserts at the position that keeps the order, and names `Sorted` in its
    **own** exhaustive deduction list rather than needing a refinement — it is
    the one function that genuinely knows the claim survives. Its parameter
    *demands* the claim, since inserting in order into an unordered list would
    not make it ordered.
  * `binary_search(list: Sorted List<T>, elem: T) -> Int?` is honest only
    because of the parameter's claim. With equal elements both backends answer
    the **lowest** matching index: each lowers to an explicit lower bound
    (Rust `partition_point` plus an equality test — not `Vec::binary_search`,
    which may answer any index in an equal run; Kotlin an `indexOfFirst` over
    `__salvoCompare`).
  * The elements must be **orderable**, on the same terms a `SortedSet` key is
    [col-key-eligible] — checked where the claim is written, and where a call
    infers it (a constructor's `as Q` lives beside the return type rather than
    in it, so the inferred path re-applies it before checking).
* [col-distinct] std declares `qualifier Distinct<T> of List<T>` in
  **`core.set`**, not `core.list`: a constructor must sit beside its qualifier
  [qual-ctor-same-file], and a *set* is what can honestly promise the claim —
  `to_list(set)` returns `List<T> as Distinct`. Mint-only, like `Sorted`.
  `to_list` over a `SortedSet` lives in `core.sorted` and so cannot mint it.
* [col-insertion-order] `Set<T>` and `Map<K, V>` **iterate in insertion
  order, on every backend** (user decision 2026-09-12) — with
  `LinkedHashMap`'s exact semantics: writing a key that is already present
  keeps its original position, and removing one is O(1) and leaves the order
  of the rest intact. `to_list` on a set, `keys` on a map, a `for` over
  either, and `to_str` all agree on that order.
  * Kotlin gets it from `LinkedHashSet`/`LinkedHashMap`. Rust's standard
    library has no ordered hash container, so **the backend ships one**:
    `SalvoSet`/`SalvoMap` in `runtime/collections.rs` (a slot vector plus a
    hash index, compacted when the graveyard outgrows the live entries)
    [rs-collections].
  * Two alternatives were rejected. **Unspecified order** — what
    `HashMap`/`HashSet` give — would make a program's output depend on its
    backend, which [backend-parity] forbids. **Always sorted** would charge
    every collection an ordering it may not need, and would demand orderable
    keys where hashable ones suffice; that is what `SortedSet`/`SortedMap`
    are for [col-sorted].
* [col-to-str] `to_str` of a collection is **the language's format, not the
  target's**, and both backends emit the same string: `[1, 2, 3]` for a
  list, `{1, 2, 3}` for a set, `{a: 1, b: 2}` for a map — the shape of the
  literal that would build it [col-literal]. Elements appear in the
  collection's own order ([col-insertion-order], or key order for the sorted
  pair).
  * Neither backend's native rendering is used, because they disagree with
    each other and with Salvo: Rust's `Debug` for a map quotes string keys
    and writes `:`, Kotlin's `toString` writes `a=1`. The Rust runtime and
    the Kotlin lowerings each write the format out.
* [col-sorted] `SortedSet<T>` and `SortedMap<K, V>` are **separate types**
  from `Set`/`Map`, kept in the natural order of their keys (user decision
  2026-09-12). Not a qualifier on the unordered types: a qualifier is
  droppable by design, so a `Sorted Set` could be passed where a plain `Set`
  is wanted and quietly lose the property the callee relies on.
  * Their keys must be **orderable** rather than hashable — a different bar,
    checked by the same declaration-site and instantiation-site machinery
    [col-key-eligible]. A **union** is hashable but never orderable:
    comparing values of different types has no obvious meaning.
  * `min`/`max` on a set and `first_key`/`last_key` on a map are the
    cheap-at-either-end operations an ordered tree exists for; iteration and
    `to_str` are in key order.
  * Rust maps them to `BTreeSet`/`BTreeMap`, Kotlin to `TreeSet`/`TreeMap`
    built with Salvo's own comparator [kt-ordered] — natural ordering would
    not do, since a `List` and a tuple are not `Comparable` on the JVM and a
    `Str` would compare by UTF-16 code unit.
  * **Strings order by code point** on both backends. Rust's `String: Ord`
    is byte-wise UTF-8, which is code-point order; the JVM's
    `String.compareTo` is code-unit order, which puts an astral character
    (a surrogate pair, 0xD800–0xDFFF) below a BMP one at 0xE000–0xFFFF. The
    same call Salvo already made for string *indexing* — characters, not
    encoding units — so the Kotlin comparator compares code points.
* [col-equality] **Every struct supports `==` and `!=`**, structurally
  (user decision 2026-09-12). Both operands must be the **same base type** —
  comparing two different struct types is an error, not a constant `false`
  — and qualifiers are ignored on both sides (`Surname Person == Person` is
  fine): equality is about the data at the moment of the check, not about
  what is claimed of the handle. State, provenance and `Mut` alike.
  * A **fn-typed field bars a struct from equality**: `Rc<dyn Fn>` has none
    on Rust and Kotlin would compare by reference, so no answer exists that
    both backends can give. Such a struct is therefore also barred from
    `canbe hashed` / `canbe ordered`.
  * **Ordering (`< <= > >=`) needs `canbe ordered`**; equality needs no
    opt-in. The axes are separate.
  * **Numeric widths mix** [op-promote]: `Long == Int` compares at `Long`.
    Everything else still demands the same base type, and the int↔float mix
    is refused with the conversions named.
  * **Salvo owns floating-point equality.** Rust's derived `PartialEq` is
    IEEE (`NaN` equals nothing, `+0.0 == -0.0`); Kotlin's data-class
    `equals` calls `Double.equals`, which is the *opposite* on both counts,
    so a float-bearing struct gets a generated `equals` of its own
    [kt-float-eq]. Chosen deliberately as the hook for a future
    precision-specified comparison. Verified: the same program reported
    `struct nan == nan` as `false` on Rust and `true` on Kotlin before the
    fix.
  * Generated union enums derive `PartialEq` on Rust, conditionally on
    their payloads, so a struct holding a union can derive its own.
* [col-hashed-ordered] A struct opts into being a **key** by declaring
  `canbe hashed` (a `Set` element, a `Map` key) or `canbe ordered` (a
  `SortedSet` element, a `SortedMap` key, and the ordering operators). Both
  are **validated where they are written**, so the error names the field
  rather than surfacing at a distant `Set<Point>`:
  * the struct may not be `canbe Mut` — a value that can change while a
    collection holds it corrupts the collection's lookup or order, which is
    the classic silent-corruption bug made a compile error;
  * every field must itself be hashable / orderable. `Int`, `Long`, `Str`,
    `Char` and `Bool` are both; `Double`/`Float` are **neither** (Rust's
    `f64` is not `Eq`, `Hash` or `Ord`) while equality on a struct holding
    one still works; a nested struct must carry the same claim; a `List` or
    a tuple qualifies exactly when its elements do (user decision
    2026-09-12), ordering **lexicographically**, with a shorter list that is
    a prefix comparing less.
  * Ordering of a struct is lexicographic **by field declaration order**,
    which makes field order semantically significant. Rust derives it;
    Kotlin generates a `Comparable` with a `compareTo` that goes through a
    runtime helper, since a `List` and a `Pair` are not `Comparable` on the
    JVM [kt-ordered].
  * A type variable is *not* checked at the declaration: like
    [linear-generics], the instantiation is where the key rule bites, which
    keeps generic code over keyed collections writable.
* [col-literal] Each everyday collection has a literal, and each is sugar
  for the matching constructor (user decisions 2026-09-12/13):
  `[1, 2]` is `list_of`, `{1, 2}` is `set_of`, `{"a": 1}` is `map_of`.
  * **Kinds are told apart inside the brace**: a brace lambda first
    (`{ i: Int -> 0 }`), then a bare *struct* literal when the first entry
    is `identifier:` — which is why a map key is an expression and
    `{x: 1}` stays a struct literal — then `:` after the first element
    means a map and its absence a set.
  * **`{}` is an empty collection**, never an empty struct literal (a
    fieldless struct is written with its name, `Finished {}`). Its kind
    comes from the expected type, and the AST node is an empty `SetLit`
    that the checker re-reads as a `Map`/`List` where the position says so
    — so the emitters follow the *checked type*, not the node.
  * **An empty literal needs a type from its position** — a `let`
    annotation or the parameter it is passed to — and is an error
    otherwise, naming both remedies. Overload probing therefore hands a
    concrete expected type to literals as it does to nested calls.
  * A literal **constructs**, so it adopts a `Mut` the position asks for
    (`let ys: Mut List<Int> = [4, 5]`); no other qualifier is adopted,
    since construction does not establish a claim [qual-constructive].
  * Elements **move** into the literal [deduce-consume], and a linear one
    is refused as it is in any composite [linear-composite].
  * A bracket literal still types as an **array** where the position
    expects one, which is what keeps a literal usable in a variadic
    argument.
* [type-any-never] `Any` is the top type; `Never` is the bottom type
  (the type of `return`/`break`/`continue`), subtype of everything.
  * A *written* `Never` lowers to the bottom type, not to a nominal type
    that happens to be spelled that way: `throw`'s declared
    `-> Never => !message` [throw] means callers see a value that fits
    everywhere and ends the path.
  * **A `Never`-typed expression statement terminates its path**, which
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
    `platform effect` member [platform-effect] / a `platform handler`
    [platform-handler] in customer code.
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
  * **A tuple pattern needs a tuple of that arity behind it** (2026-09-18):
    anything else is an error naming what it found — `let (a, b) = 7` and
    `let (a, b) = (1, 2, 3)` both report. Until then a mismatch bound
    `Unknown`s *silently* and the emitters wrote target code that could not
    build, so a Salvo mistake was reported, at best, by rustc or kotlinc. An
    `Unknown` subject stays lenient ([type-unknown-lenient]).
  * **A loop element destructures too** (2026-09-18): `for (k, v) in pairs` and
    `for {name, score} in rows` bind exactly what the same pattern binds in a
    `let`, and are checked the same way — the element type is the subject.
    * **One lowering, both backends**: the loop header binds the element to a
      temporary and the body opens with the pattern's bindings, read off it.
      A native pattern in the header would have had to differ per backend and
      per loop shape — Kotlin cannot destructure a pass's cast payload or a
      struct at all, and Rust's `mut` bindings in a pattern cannot move out of
      a projection ([rs-borrow-locals]) — so one shape serves every loop and
      both pattern kinds.
    * A binding the body **assigns to** is the local's own: the element is not
      written through it (a loop binding is a `let`, and a collection is not
      mutated by rebinding one), so it takes an owned copy where the others
      borrow.
    * **Nested patterns are still refused**, in a loop as in a `let`.
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
* [str-byte-size] `byte_size(str) -> Long` is the **UTF-8 byte count**, and
  the unit every byte offset in the filesystem surface is in: `write` answers
  one, `position` reports one, `open_read_at` takes one [fs-token]. A separate
  name from `size` (which counts characters) on purpose — the two differ the
  moment a string leaves ASCII, and confusing them silently corrupts an
  offset. Lowered per backend (`String::len`, `toByteArray(UTF_8).size`),
  which is also the one length that cannot drift between them.
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
* [qual-overload] **A qualifier name may be declared over several subject
  types**, and which one a use means is decided by the subject — the way a
  function overload is decided by its arguments (user decision 2026-09-13).
  std declares `NonEmpty` five times over: `of List<T>` in `core.list`, and
  `of Set<T>`, `of Map<K, V>`, `of SortedSet<T>`, `of SortedMap<K, V>` in
  `core.nonempty`.
  * The subject is keyed **syntactically, by the `of` type's base name**
    (`List`, `Set`, …). A generic `of` (`qualifier Ok<T> of T`) has no base
    name, accepts every subject, and so cannot be told apart from another
    declaration of its name — which therefore stays a duplicate. Syntactic on
    purpose: resolution, the [mod-collision] checks, the refinement matcher
    and both backends need the same answer, and only the checker can unify.
  * **Same name *and* same subject is not an overload but a replacement.** A
    module declaring its own `NonEmpty of List<T>` shadows std's, exactly as
    before; two in *one* module are a duplicate ("nothing at a use site could
    tell them apart"), as are two in different implicitly visible `core`
    modules.
  * A use whose subject **no** declaration of that name accepts is the same
    error as a single inapplicable declaration ("does not apply to `Int`"),
    not silence.
  * Where the question is about the *name* rather than a subject — does one
    exist, does it have a body, is it provenance, is it `with`-compatible with
    another — any declaration of the name answers, since `with` names a
    qualifier and same-named declarations are the same claim over different
    containers.
  * A **refinement** picks its qualifier the same way, from the refined
    parameter's own type: `refn add(set: Mut Set<T>, elem: T) => set:
    +NonEmpty` inside the `of Set<T>` declaration refines the `add` that takes
    a set. Two container claims therefore do *not* conflict with each other
    [qual-refn-conflict] — they are about different subjects.
  * Backends: erasure [qual-erasure] makes two same-named `qualifies`
    functions collide where the target has no overloading, so **Rust mangles
    the emitted name with the subject** (`Filled__List_qualifies`) and only
    when the name is actually overloaded, following [rs-fn-mangling]. Kotlin
    needs nothing — the JVM overloads on the parameter type.
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
    removals (`-Q`). A plain (exhaustive) name and `Never` are parse
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
* [expr-escape] **`return`, `break` and `continue` are expressions of type
  `Never`** (user decision 2026-09-21), not statements — step 2 of the `?`
  family sequence, and the reason `maybe_t() ?: return _` needs no grammar
  exception. A bare escape on its own line is an ordinary expression
  statement.
  * **A value follows only on the same line**, and only when a token that can
    begin one does: `return` before a `}` or a newline is still the
    value-less form. So the shape of existing code is unchanged.
  * **The rules are the ones they always had**, only reached through the
    expression checker: the return type check, the bare-`return`-in-a-
    value-returning-fn error, `break`'s contribution to a loop's value
    [while-value], "`break`/`continue` outside a loop", the linear exit
    checks [linear-obligation] and the state-whole check, and the move of a
    returned or broken value [deduce-consume].
  * **What it deletes**: the syntactic special cases in the two path
    analyses. `block_exits` and `block_returns` now read divergence off the
    checker's recorded types alone ([type-any-never]'s predicate), which is
    what they already did for a `throw(m)` call. One rule, one mechanism.
  * **`break`/`continue` diverge but do not *return***: they leave a loop,
    not the function, so [fn-must-return] excludes them explicitly. The
    distinction used to be free, because only a `return` *statement* counted.
  * **Nested positions work**: `twice(return 0)` is grammatical, as
    `twice(throw("no"))` already was. Both backends have the three as
    expressions natively, so each renders directly. The one refusal is an
    escape nested in an expression where the enclosing block owes cleanup
    ([rs-exit-splice] runs at statement position), reported rather than
    emitted wrong [backend-never-wrong].
* [op-no-none] Arithmetic (`+ - * / %`) and comparison (`< > <= >=`,
  `== !=`) reject a possibly-`None` operand, and `None` itself, as an
  error (user decision 2026-09-02): nullability is tested with `is None`,
  so an optional reaching an operator is a missing narrowing. Remedy:
  narrow (`is` / `when`) or assert with `!`.
  * Same parity motivation as [interp-no-none]: Kotlin compares against
    `null` happily while Rust rejects the `Option`.
  * `Unknown`/`Never` operands stay lenient [type-unknown-lenient].
* [op-arith] Arithmetic (`+ - * / %`, unary `-`) works on **numeric
  operands only** — `Int`, `Long`, `Float`, `Double` (user decision
  2026-09-14, closing the operand-typing DECISION). The result is the
  (promoted) operand type, qualifiers stripped. Refusals name their
  remedy: `Str + Str` points at `${}` interpolation, mixed classes at the
  conversions [op-convert].
  * `Byte` is deliberately **not** operator-numeric [byte-value]: it is an
    octet, not a number, and its arithmetic goes through `to_int`/`to_byte`
    — one call each way, which also states the wrapping instead of implying
    it.
  * `Int / Int` is integer division on both backends.
  * An unconstrained generic operand stays lenient like `Unknown` — a
    documented leftover matching the equality slice, not a rule.
* [op-promote] Mixed widths widen implicitly **within** a class:
  `Int + Long` computes at `Long`, `Float`/`Double` at `Double`, for
  arithmetic, ordering **and equality** alike (equality joined 2026-09-18,
  user decision: `n < 0` had always been fine for a `Long` `n` while `n == 0`
  was an error, and no reading justified the split — a widening comparison is
  exact in both directions). On Kotlin, equality is the one operator whose
  mixed form the target refuses, so the promotion is rendered as an explicit
  `.toLong()`/`.toDouble()` there [kt-op-promote]; the narrower operand's span is recorded
  in `Checked::promotions`. Backends render it their own way: Rust casts
  (`((n) as i64)` — it has no mixed-width operators), Kotlin's operator
  set already covers the mixes. Mixing the integer and float classes is
  an error naming the explicit conversions — never implicit, so float
  surprises stay opt-in.
* [op-convert] `core.basic` declares the explicit conversions the
  operators point at: `to_int`, `to_long`, `to_float`, `to_double`, one
  overload per source width [intrinsic-fn]. Truncating conversions
  truncate toward zero and **saturate** at the target's bounds
  identically on both backends; `to_int(Long)` keeps the low 32 bits
  (Kotlin `toX()` ≡ Rust `as`, verified pairwise).
* [op-order] Ordering (`< <= > >=`) works on numeric operands (widened
  per [op-promote]) and on structs declaring `canbe ordered`
  [col-equality]; everything else — `Str`, `Char`, `Bool`, containers,
  tuples, fn values — is refused with the rule spelled out (user decision
  2026-09-14). Note the deliberate difference from sorted-collection
  keys [col-hashed-ordered]: `Double` **is** orderable at the operator
  (both backends agree on IEEE partial comparison, `NaN` answering
  `false`), while a sorted container of them stays refused (no total
  order).
* [op-bool] `&&`, `||` and unary `!` take `Bool` operands only — the
  value-position twin of [cond-bool], with the same no-truthiness
  reasoning and remedy text. Condition position reports once, through
  [cond-bool].
* [cond-bool] Conditions are boolean expressions: `if`/`elif`, `while`,
  and a subject-less `when`'s branch heads [when-condition] accept `Bool`
  and nothing else. There is no truthiness — no rule could turn an `Int`,
  a `Str` or a possibly-absent `Bool?` into a decision, and guessing one
  is how a backend divergence gets in (Kotlin has no truthiness either;
  Rust would reject the `Option`). `is`/`^` checks evaluate to `Bool`.
  * Checked per **leaf** of a `&&`/`||`/`!` condition, which is where the
    wrong type was written; `Unknown` and `Never` stay lenient
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
    call ([type-any-never]) — the same predicate [fn-must-return] uses,
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
* [qual-lift] `expr is ^Qual...` **lifts** a qualifier: the same arm test an
  ordinary `is` performs, after which the listed qualifiers are *removed* for
  the branch. `list is ^Mut` reads `list` without `Mut`; `outcome is ^Ok`
  without `Ok`. Spelled as a mark on the qualifier since 2026-09-21 (user
  decision, step 3 of the `?` family sequence); from 2026-09-05 until then it
  was a standalone operator `expr ^ Qual...` with its own precedence tier, and
  the change buys one rule instead of two — `^Q` is "Q, lifted", wherever a
  qualifier may be written in a check.
  * **All or none.** Every qualifier in a check is marked or none is; a mix
    (`is ^Ok NonEmpty`) is a *parse* error, because some-marked-some-not is a
    syntactic property and the parser cannot tell a qualifier from a base type
    (both are uppercase). One consequence: lifting a qualifier while *naming*
    the arm's base type (`is ^Ok Int`) is unexpressible for now. Nothing was
    lost — the old operator refused a base type outright.
  * **A binding is allowed** (user decision 2026-09-21, superseding the
    no-binding rule of 2026-09-05): `is ^Ok value` names the lifted value.
    Two emissions, because they genuinely differ: a lift that peels a
    **wrapper arm** reads the payload out of the storage, while a lift of a
    **qualifier only** binds the value itself, qualifiers being erased.
  * **Several arms at once** is what the binding is for: the lifted value is
    *produced*, so it can span two arms where a re-read of the subject cannot,
    and with a binding nothing narrows — the subject keeps its declared type
    and the lifted union lives in the name. The **checker** implements this;
    the **emission is deferred** (the bound value spans fewer arms than its
    storage, so it needs the arm-mapping re-wrap [let-infer]), so a multi-arm
    lift with a binding is refused by name rather than emitted wrong
    [backend-never-wrong]. Without a binding it stays refused for the original
    reason: one re-read cannot stand for two wrapper positions.
  * A `when` **branch head** takes the same form: `is ^Qual...` alongside
    `is ...`, the same arm test with the subject reading lifted inside the
    branch. That
    is the form the qualified-union case wants — `when o { is ^Ok { when o {
    … } } }` reaches the union inside `Ok (Ok Int | Err Str)` with no
    intermediate binding and no repeated type.
  * The qualifiers must be **present**: nothing to remove is an error, not a
    silently-false test (`^` is not a predicate test — that is `is`).
    Several may be removed at once (`v is ^Mut ^NonEmpty`), and the right side
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
    type. Reach them with a `^` branch head ([qual-lift]:
    `when v { is ^Ok { when v { … } } }`), or bind at the inner type
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
* [when-value] `when` is an expression; branches ending in `Never`
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
* [is-bind-once] **An `is` binding evaluates its subject exactly once.** The
  test and the payload are two reads of the same subject, so a subject that is
  not a **place** (a call, above all) is evaluated into one temporary that both
  read — per *iteration* for a `while`. This was a defect until 2026-09-16:
  both backends emitted the subject twice, so
  `while remove_first(q) is Ticket t` called `remove_first` twice per turn and
  silently dropped every other element — with a linear element, its obligation
  with it.
  * **Where a non-place subject may go**: as the whole condition of a `while`,
    or of an `if`'s **first** branch. Those are the positions with somewhere to
    put the single evaluation, and they are the shapes programs want — the
    take-until-empty loop above all, which is how an *effectful* discharger
    drains a container of obligations [linear-container].
  * **Everywhere else it is refused**, naming the `let` remedy: inside a
    `&&`/`||` chain, hoisting would evaluate a subject that short-circuiting
    says must not run, and *not* hoisting is the defect; in an `elif`
    condition, there is no statement position for the temporary. A **place**
    subject is unrestricted, since reading one twice is free, and a subject
    without a binding is read once by the test and so needs nothing.
  * `when` needs no rule of its own: its subject must already be a plain
    variable [when-union-subject].
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
  expression** ([type-any-never]: a statement the checker typed
  `Never`, e.g. `throw(m)`), `if` with an `else` where every branch
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
    (resolving across same-named effects by availability
    [effect-member-overload]), and a rename introduces a fresh name
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
     [type-any-never];
  3. **qualifier sets compare by inclusion**: more qualifiers says more, and
     the *kind* never ranks (`Mut List<T>` and `NonEmpty List<T>` are
     unrankable, deliberately — ranking them would ask the caller to know
     more than what is in front of them);
  4. with the slots otherwise equal, a **fixed** parameter list beats a
     variadic one, which is what lets `list_of()` pick a no-argument overload
     over `list_of(...elems)`.
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
  attaches to the *name*). Also valid as a value (`describe@main`). A
  **capitalized** name after `@` is an effect selector instead
  [effect-at].
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
  parameters like any other call — arity and types. Both selections happen
  *before* this: which **effect**, by availability or `@Effect`, and which
  **overload** within it, by the argument types
  [effect-member-overload] [effect-at] — so by here the signature is fixed and
  these are plain mismatch diagnostics. (Until 2026-09-07 the member's
  parameter types only flowed in as *expected* types, so `log(true)` on
  `fn log(message: Str)` was accepted.)
* [call-type-args] A generic call's type arguments must be **determined**.
  In order: an explicit list (`mut_list_of<Int>()`) pins them; otherwise
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
    * For the same reason a spread of a **place** into an *owning* variadic
      parameter is **cloned**, on the ordinary call path as well as the
      intrinsic one: the parameter is owned in the emitted Rust (it is built
      from the arguments) while the caller's array stays live, so moving it
      made `f(...rest)` followed by any further use of `rest` a raw rustc
      E0382 (fixed on the intrinsic path 2026-09-13 with the sorted
      collections, and on the ordinary path later the same day).
  * **A tail may mix plain arguments with a spread** — `list_of(first,
    ...rest)` — since 2026-09-13. Kotlin always could, its spread being an
    operator on an argument (`listOf(first, *rest)`); Rust needs the tail as
    one `Vec<T>`, so the emitter assembles it in written order (pushing each
    plain element, extending from each spread), which also allows a spread
    anywhere in the tail rather than only last. Nothing about the targets had
    prevented it: the ordinary path refused the mixture outright and the
    intrinsic path had no guard at all, so it read the spread as the *whole*
    tail and silently dropped the leading elements [backend-never-wrong].
    * A constructor lowering is told which it got (`Spread::Borrowed` vs
      `Spread::Owned`), because an assembled vector is fresh and must not be
      cloned again while a borrowed forward must be. Ownership is the
      caller's fact, not something a lowering can infer from the text.
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
    as an effect list on the handler and supplied by the per-scope fusion
    (user decisions 2026-09-04 and 2026-09-14; mechanism in
    [rs-effect-fusion] / [kt-effect-fusion]). A *member* declaring its own
    effects stays an error: the dependency belongs to the implementation,
    not the interface.
  * (The `[waitfor]` exception of 2026-09-17 went with the capability's
    deletion [waitfor-effect]: members refuse every effect ref again.)
  * A state field's initializer is **checked against its declared type**,
    like a struct field's default: it runs at construction with no locals
    in scope. (Until 2026-09-04 it was not checked at all, so
    `held: Mut List<Int> = "no"` was accepted and the emitters never saw
    the expression's types, which the backends' intrinsic lowerings rely
    on [backend-intrinsic].)
* [effect-handler-multi] **A handler may implement several effects**, one face
  per effect: `handler ManualTime() of Timer, TimerCtl` (T-4(a), user decision
  2026-09-17; built 2026-09-18). One handler, one piece of state, one mailbox
  when it is an actor's — and one *typed face* per protocol.
  * **What it replaces**: the forwarding split. Two protocols over one state
    used to need a state-owning handler plus a second handler forwarding into
    it — the same runtime shape, written twice. The pattern generalizes past
    the test-control case it was found in: a **public face and an admin face**
    (health, draining, stats) is the everyday form.
  * **A `spawn` answers one addr per face**, in declaration order: a bare
    `Addr<E>` for one face (unchanged), a **tuple** for several —
    `let (timer, ctl) = spawn ManualTime() on pool(1)`. Least authority falls
    out of the types with no new type machinery: production code holding
    `timer` cannot name `advance`, because `Addr<Timer>` is typed by `Timer`.
    Intersection-typed addrs (`Addr<Timer & TimerCtl>`) were the alternative
    and are recorded in ROADMAP.md as an unscheduled future consideration —
    the tuple gets the value without importing intersection types.
  * **A `use` binds every face**, so the synchronous form of the pattern is
    one registration and two effect lists that reach it; per-effect shadowing
    is unchanged ([use-no-dup]), since each face is registered on its own.
  * **Conformance is checked here, face by face**: every member of every face
    needs an implementation, named as
    ``handler `H` does not implement `E.m(T)` `` — the diagnostic the target
    compilers' missing-trait-method errors used to stand in for.
  * **Same-named members across faces** are legal in exactly two shapes:
    *overloading distinguishes them* (different written parameter types, so
    they are two members here and the handler implements both), or *one method
    implements both* (identical signatures — same return type, same deduction
    clause, since those are the two halves of the contract a caller relies on).
    Refused where overloading cannot see the difference: same parameters,
    different return type or different deductions. A caller still names the
    face with `m@E(…)` where two effects in scope declare the name, which is
    [effect-at] unchanged — it keys on the name, not on the parameters.
  * **The faces are all of one kind.** An `actor effect` beside a plain one is
    refused: a handler is bound one way or the other (`spawn` or `use`), and a
    handler whose plain members run on the caller's thread while its send
    members run on the actor's is the mixed handler [mixed-handler] — a
    different construct, classified by shape.
  * **Each face is named once** — a repeated face would make a `spawn` answer
    the same addr twice and say nothing new.
  * **A bodyless handler wears one face**: an `intrinsic handler`'s members are
    the backend's and a `platform handler`'s are the host's, and each writes
    one implementation of one generated interface.
  * **The deadlock graph keeps its effect-keyed nodes** [actor-deadlock-cycle]:
    every face of a handler contributes the handler's edges, so two nodes that
    happen to be one actor is the same conservative approximation the
    type-level graph already makes.
  * **A multi-face handler may not be *constructed* in a spawn's `use`
    clause**: the child owns what a clause builds, and one instance cannot be
    two of the child's dependencies. Two addrs of the same actor are the shape
    that works, and they are two clause items.
  * **One mailbox serves every face** [actor-mailbox], so arrival order across
    faces is arrival order — the same rule that already holds across a single
    protocol's members.
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
  * **The read direction, same rule** (2026-09-15): a member may not move a
    value **out** of the handler's storage either — a `state` field or a
    *constructor parameter*, both of which the handler still owns when the
    member returns. So consuming one (a `=> !p` parameter, a `send`, an `add`,
    a struct literal) or `return`ing it is an error naming `copy`, and
    `copy(held)` is the whole remedy. The evidence is the store rule's,
    mirrored: Rust silently `.clone()`d, so the handler kept a *copy* and
    mutable data diverged (`eat(items)` then `size(items)` printed 2 on Rust
    and 3 on Kotlin) — and a **linear** value had its obligation *duplicated*
    — while Kotlin shared the reference; where the clone was missing (the
    consuming *intrinsics*) rustc reported a bare `E0507` with no Salvo
    diagnostic at all.
    * A **linear** stored value has no `copy`, so it is refused outright:
      taking one out of a composite is the interim refusal
      ([linear-composite]), and here the composite is the handler.
    * A **Copy scalar** is exempt ([copy-scalar-free]): its copy is free and
      indistinguishable from a move, both backends agree, and `copy` around
      every `Int` a member answers with would be noise. Which is exactly why
      the rule went unnoticed — the first actor handler's state was
      `sum: Int`.
    * This is a *tightening*: `return held` and `eat(held)` used to compile.
      Under "copies only by opt-in" they were a copy nobody wrote.
* [effect-not-data] An effect names a *capability*, not a type of values.
  It may appear in a fn's effect list (`[Console]`), in a **handler's** effect
  list and its `of` clause; every data position — struct field, parameter,
  return type, `let` annotation, type alias, union or tuple component — is an
  error (user decision 2026-09-03). The value would have to be a handler
  instance, and those are reached through `use`.
  * **No exceptions.** A handler dependency used to be one — a constructor
    parameter of effect type — and since 2026-09-14 it is an effect list on
    the declaration ([effect-handler-deps]), so an effect in a data position
    is always this error.
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
* [effect-handler-deps] A handler declares the effects it *depends on* as an
  **effect list on the declaration**, exactly as a fn does:
  `handler Stamped [Logger, Clock] of Logger` (user decision 2026-09-14,
  replacing constructor parameters of effect type — the names could not be
  used for anything, and an effect in a data position is now always
  [effect-not-data]). Declared on the *handler*, not the effect —
  implementations differ in what they need (user decision 2026-09-04).
  * The handler's member bodies may use those effects, exactly as if they had
    declared them — which they may not ([effect-member-no-effects]): the
    dependency belongs to the implementation, so it is stated once.
  * The entries are **unnamed**, because nothing could refer to one: a member
    body reaches an effect by calling its members, like all Salvo code. So
    the list says only what the handler needs in order to run.
  * `use` and `Throw` are refused in the list (a handler registers no
    handlers; a throw wants a delimiter, not a handler [throw]), as is the
    same effect twice.
  * A dependency is **not written at the `use` site**: the compiler supplies
    it from the enclosing scope, so it is not a constructor argument, and a
    `use` whose dependency has no handler in scope is an error naming it
    ("register one before it").
  * A handler **may** depend on the effect it implements — that is
    *interception* — and the dependency binds **strictly outward**
    ([effect-intercept]).
  * A handler member may **not** call another member of its *own* effect:
    there is no self-dispatch, and declaring the effect as a dependency means
    the handler registered before this one. Diagnosed as such (a member can
    use neither remedy the general "no handler" message names), and recorded
    as a gap in ROADMAP.md.
  * Dependency **cycles need no separate check**: a dependency must already
    be registered when its dependent is, so a cycle cannot be constructed
    in any order (verified both ways). The availability rule *is* the
    acyclicity guarantee, which is what the fusion emission relies on.
  * Emission is **one of two forms**, decided per handler declaration by the
    shared predicate `handler_handle_deps` (`salvo-core`, used verbatim by
    the checker and both emitters so the shapes cannot disagree):
    * **Owned handles** (user decision 2026-09-20, [use-local]): a
      plain-face, non-mixed, dep-bearing handler whose declared effects are
      all the shareable default (`[E]` — no `local E`, no `use`, no
      `spawn`, no actor-effect dep, no generic-effect-instance dep)
      captures its dependencies as handles **at construction** — fields and
      trailing constructor arguments, one per dep in declaration order
      (`Checked::handle_captures` records the resolved instances per
      construction; the emitters read only this). Such a handler may be
      bound shareable — the interceptor case: a production `Logger` is
      stateless-with-deps and shares bare, no `local` anywhere. A
      **generic** handler works in this form (its dep is a handle field, so
      no generated trait names the handler's generics — the cut that still
      stands for the fusion form).
      * The handle is minted off a **binding in the same function**
        ([use-local]'s eager handle) **or threaded in by the caller** when
        the effect arrives through the enclosing *signature*
        ([spawn-inherit]'s lift, 2026-09-20: a recorded handle requirement,
        propagated up the call graph, lowered as [rs-handle-bundle]). A
        `use local` dep binding is still refused — a local binding cannot
        yield an owned handle.
    * **Fusion** ([rs-effect-fusion], [kt-effect-fusion]) for every other
      dep-bearing handler — which therefore binds `use local` only. Since
      2026-09-14 in the **Has-accessor** shape on both backends (user
      decision, adopted from FILE_SYSTEM.md §5.8.1): one fused value per
      scope carries the registered handlers behind generated per-effect
      accessor traits/interfaces, so effect members can never collide on it
      and instances of a generic effect stay apart. Kotlin additionally
      injects the dependency at construction (objects alias); Rust hands it
      to the member body from a disjoint borrow of the fusion. Both
      backends gate the fusion on the same program-wide predicate and run
      the same programs to the same output. At the fusion layer a `local E`
      dep threads like any other — dep locality is the checker's business.
* [effect-fn-deps] A fn's `[E1, E2<T>]` list declares its effect
  dependencies. Calling a fn requires each of its effects to be available
  in the caller (declared or `use`d) — validated by the checker at every
  call site, recorded per-call (`call_effects`) in declaration order.
* [with-clause] **`with` supplies specific dependency instances** to a `use`
  or a `spawn` (user decision 2026-09-20, superseding the never-built `using`
  rename of 2026-09-19 — and the clause's original `use` spelling, which
  shared a word with the statement and let a bare spawn swallow a following
  `use` line): `use H(args) with D(), addr` and
  `spawn H(args) with D(), addr on POOL`.
  * Each item is a handler **construction** or an `Addr`. A construction is a
    **private instance**: built at the clause, owned by the handler (or the
    child) being registered, unshared with the scope — which is what makes
    "give *this* one its own" writable without a second scope.
  * **Partial clauses are the rule**: an item wins for the dependency it
    matches, and every dependency the clause does not cover resolves from the
    scope ([spawn-inherit] for a spawn, the ordinary availability rule for a
    `use`). Matching is by effect instance, exact before unifying, so the
    *written* order and the handler's *declaration* order may differ freely
    (recorded in `use_with_items` / `spawn_dep_items` for the emitters).
  * **The self-dependency may be supplied**, which is [effect-intercept]'s one
    written exception: `use Loud with Formal()` wraps the clause's private
    instance instead of the registration already in scope. Acyclicity is
    untouched — a fresh construction cannot point back at the handler being
    registered.
  * **`use local H with …` is legal**: the clause chooses *which instance*,
    which is orthogonal to the binding's locality.
  * Refused, each by name: an item the handler does not depend on (supplied
    for nothing); a **multi-face** handler as an item (the clause builds one
    instance per item, so the second face has nowhere to go — bind it with
    its own `use` and name the addr); an item with **dependencies of its
    own** (a clause item has no scope to resolve them from — register it
    before this one instead); a `with` on a `use` of an **existing handle**
    (its dependencies were settled where it was built); and a `with` on a
    handler whose dependencies take the **fusion** form ([use-local]'s
    `local`/actor/generic-instance deps), which threads per call from the
    scope's fused value and has no slot for a private instance — both
    remaining cuts are recorded in ROADMAP.
  * `with` is a keyword already — the qualifier-compatibility clause spells
    it (`qualifier Q of T with A`) — and the two positions cannot be
    confused: one follows a qualifier's `of` type, the other a `use`/`spawn`
    handler expression. Same-line, like every trailing clause.
* [spawn-inherit] **A spawned handler inherits the spawning scope's
  dependencies** (user decision 2026-09-20; the arc the shareable-by-default
  round was built for). A declared dependency the `with` clause does not
  cover is resolved from the scope's availabilities and captured as an owned
  handle that travels with the child — which is sound *modularly*, with no
  whole-program analysis and no runtime check, exactly because a bare `[E]`
  now guarantees shareability [effect-local]. The old rule ("dependencies
  come from the clause, never from the spawning scope") is gone; the clause
  is for overriding [with-clause].
  * **Only a shareable availability can be inherited**: a `use local`
    binding exists precisely so that it does not cross a seam, and the
    refusal names the remedy — bind it shareable, or supply the child its
    own with `with`. For an **actor** effect the local binding is an inline
    `use H()`, so the remedy is instead "spawn it and bind the addr".
  * **Ambiguity is an error, not a guess**: two instances in scope that
    could both satisfy the dependency (a generic effect with an unpinned
    instantiation) is reported, naming `with` as how the program says which.
    An exactly matching instance always wins, so this only fires where
    unification is doing the work.
  * **Nothing in scope** stays an error, now naming both remedies (bind one
    before the spawn — it is then inherited — or supply it in the clause).
  * **A `local E` dependency cannot be inherited**: it accepts only a
    scope-local binding, which a child cannot hold. Declare the dependency
    shareable instead.
  * **The v1 lexical cut is lifted with it.** A capture used to require the
    dependency to come from a `use` *in the same function*; an effect
    arriving through the enclosing **signature** now works too, with the
    handle threaded in by the caller: the checker records a **handle
    requirement** per fn (`handle_requirements`) and propagates it up the
    call graph to a fixpoint, gated on the caller supplying that effect from
    its own signature in turn. Only a fn whose effect list carries `use` or
    `spawn` can have one (user decision: the visible capability is what
    admits the hidden parameter), and the lowering is [rs-handle-bundle] —
    one hidden fused parameter on Rust, nothing at all on Kotlin, where an
    object reference already *is* a handle [kt-monitor].
  * Two shapes cannot answer and are errors: a **platform effect** (the host
    owns that instance and hands it to `main` as a borrow — there is nothing
    to mint; the remedy is a Salvo handler over it, the `DefaultFs [RawFs]`
    shape, or a `local` dependency — that surface gets its own design round,
    ROADMAP), and a **lambda** body (a fn value's effects are call-only, so
    no caller could supply a handle).
  * **The deadlock graph is unchanged**: it prices a handler's dependencies
    from its *declaration*, so an inherited dependency carries exactly the
    edges a written one did [actor-deadlock-cycle].
* [effect-local] **`local E` in an effect list accepts a scope-local
  binding of `E` and disclaims seam rights** (user decision 2026-09-20);
  the bare `[E]` default now means *shareable `E`* — the fn may pass it
  across seams. Contextual: `local` followed by an effect name; an effect
  *called* `local` (unwise) still parses.
  * **The call-site rule**: a `use local` binding satisfies only
    `[local E]` requirements — a bare requirement over it is an error
    naming both remedies (bind `E` shareable, or declare `[local E]` if
    the fn only calls it). A shareable binding satisfies both forms,
    `local` being the weaker claim.
  * The annotation is **viral down call chains** that traffic in local
    bindings — an accepted cost, to be lifted later by inference (the
    deduction pattern: written validates, unwritten infers). std's own
    effect-forwarding fns (`println`, the whole `[Fs]` surface, `elapsed`)
    declare `[local E]`: a pure forwarder makes the weakest demand.
  * **A fn type's effects are always call-only** — a function value cannot
    spawn, so its requirement grants no seam rights either way. Writing
    `local` on a fn type is refused as redundant, and the availabilities a
    fn value's declared effects grant its body are local — so a fn called
    from inside a *lambda* declares `[local E]`. A fn-typed parameter's
    inherited effects [fn-effects] enter the enclosing fn's environment as
    local for the same reason.
  * On a **handler's** list, `local E` declares a dependency that accepts a
    scope-local binding — which cannot be captured into a shared instance,
    so it pins the handler itself to `use local` ([use-local]) and keeps
    the handler on the fusion emission ([effect-handler-deps]).
* [effect-no-dup] Two effects of the same type in one list are an error
  unless their generic arguments differ (`[Random<Int>, Random<Double>]`
  is fine, `[Console, Console]` is not).
* [effect-use] `use Handler(...)` registers a handler instance for the
  rest of the current scope; `use Handler` is sugar for `use Handler()`.
  * The checker infers the handler's generics from the constructor
    arguments and registers the *concrete* effect instance
    (`use CyclicRandom(list_of(1,2,3))` registers `Random<Int>`), recorded
    in `use_effects`.
* [use-local] **A bare `use H(args)` binds shareable by default** (user
  decision 2026-09-20; the motivating goal is spawn-inheritance — a bare
  `[E]` in a signature has to *guarantee* shareability for a `spawn` to
  synthesize its dep clause from scope). The checker classifies every
  handler-construction `use` (recorded in `use_kinds`; the emitters wrap, or
  don't, off this and never off their own re-derivation):
  * **Bare** — a stateless handler: shareable without a lock, nothing
    changes at the binding. On Rust the struct derives `Clone` so a seam
    can box a clone; a stateless clone is observationally the instance.
    A **platform handler classifies bare too** (user decision 2026-09-20):
    it is *assumed thread-safe by its design* — its state is the host's and
    invisible here, so this is an assumption, not a proof, and a way to
    validate/specify it is future work (ROADMAP). The point is that nothing
    depending on a platform-backed effect ever writes `local`
    (`DefaultFs [RawFs]` stays annotation-free). Kotlin shares the raw host
    instance; Rust shares it through the lock adapter as a mechanical
    consequence of `&mut self` members ([rs-platform-handler]), not as a
    semantic monitor.
  * **Monitor** — a stateful handler: lock-shaped from birth, effectively
    `let h = spawn H(args); use h` ([monitor-handler]'s form, minus the
    words). Intrinsic handlers are the emitters' own structs, so their
    declared (stateless) shape is trusted.
  * **Local** — `use local H(args)`: the scope-local, lock-free binding
    (the pre-2026-09-20 meaning of a bare `use`). *Required*, by an error
    naming it, for a handler that cannot be shared: unsendable constructor
    parameters or state [actor-sendable], the `use` capability (bindings
    would not be fixed at construction), the `spawn` capability, a
    `local E` dependency (a scope-local binding cannot be captured into a
    shared instance), an actor-effect or generic-effect-instance dependency
    (both pin the fusion form, [effect-handler-deps]), or — stateful only —
    several faces (one lock behind several effect types has no backend
    representation yet). A `use` of a handler with an **actor-effect face**
    classifies Local silently: binding one inline is the historical escape
    hatch, and its shareable handle is the addr a `spawn` answers.
  * `use local … on POOL` is a parse error — the `on` clause spawns a
    shared servant, which contradicts `local`. `local` is contextual:
    `use local H` binds `H` locally, and a *binding named* `local` is still
    reachable as `use (local)`.
  * A stateful handler of a **generic effect instance** shares like any
    other (user decision 2026-09-20: `handler CyclicRandom of Random<Int>`
    is shareable): the per-effect wrappers are generic exactly as the
    effect is ([rs-monitor], [kt-monitor]).
  * Both emitters mint an **eager handle** beside a shareable binding when
    a later construction in the same file captures the effect
    ([effect-handler-deps]): the binding value itself moves into the local
    dispatch machinery, so the handle is minted where the value is whole.
    A monitor's handle-clone is a lock-handle clone (same instance); a
    stateless clone is indistinguishable from the instance.
* [use-requires-use] `use` is only legal in functions declaring the
  special `use` effect (`main() [use]` is the conventional entry point).
* [use-no-dup] A `use` may **shadow** an earlier registration for the same
  effect instance: the innermost wins for the rest of the scope, and the
  shadowed handler comes back when the shadowing `use`'s block ends
  [effect-scope]. Later-in-block over earlier-in-block is shadowing too
  (`use DefaultFs(); use RestrictedFs(root)` in one statement list is
  legal). What stays an error is registering the same handler *instance*
  twice — and under [handler-not-value] an instance exists only at its
  `use`, so that clause is future-proofing for nameable handler values
  rather than a check today. (Until 2026-09-14 the rule was the opposite:
  a second registration for an instance already in scope was an error,
  which made interception unwritable — user decision, FILE_SYSTEM.md §5.1.)
  * Consequence for every lookup: the effect environment is a **scope**, not
    a set. The checker's `effect_env` and both emitters' environments
    resolve innermost-first with shadowed entries hidden; reading them as
    sets made an outer handler answer a call the inner one owned, which is
    a silent divergence rather than a diagnostic (found exactly that way in
    the Kotlin emitter, 2026-09-14).
* [effect-intercept] A handler constructor parameter of the **same** effect
  the handler implements is *interception*: `handler RestrictedFs(root: Str) [Fs] of Fs`. The dependency binds **strictly outward** — to the
  instance in scope *before* this handler's own `use` — so an interceptor
  wraps the handler it shadows, and interceptors stack (an interceptor may
  wrap an interceptor). User decision 2026-09-14 (FILE_SYSTEM.md §5.1,
  option O-R2), where `RestrictedFs of Fs` over a `MemFs`/`DefaultFs` is
  the customer that made it load-bearing.
  * **Acyclicity survives unchanged.** Every dependency edge still points
    at a registration that precedes this one, and "precedes" is
    well-founded, so no cycle can be constructed in any order — the
    argument [effect-handler-deps] already rests on, with the outward
    binding as its self-dependency clause.
  * With **nothing** registered for that effect before the `use`, and no
    `with` item supplying one, there is nothing to intercept: the
    registration is an error in interception's own words ("handler `H`
    intercepts `E` … an intercepting handler wraps the instance already in
    scope, or the one you name with `with`"), which is the self-dependency
    case of [effect-handler-deps]'s "register one before it".
  * **`with` may aim the interception** [with-clause]: `use Loud with
    Formal()` wraps that private instance instead of the scope's
    registration — the one written exception to "binds strictly outward",
    and acyclicity is untouched since a fresh construction cannot point
    back.
  * Inside the handler's members the effect resolves to the **dependency**,
    never to the handler itself — a member call is an outward call, so
    nothing recurses. This is the ordinary [effect-handler-deps] rule; it
    reads as a special case only because the effect names match.
  * Interception is per **instance** of a generic effect: intercepting
    `Store<Int>` leaves `Store<Str>` with the handler it had.
  * Emission: the fusion of the shadowing `use` carries exactly *one*
    accessor per effect — the new handler's — while the shadowed instance
    stays reachable through the provider the intercepting handler is handed
    ([rs-effect-fusion], [kt-effect-fusion]).
* [effect-available] Calling an effect member requires an instance of its
  effect in scope; otherwise "no handler for effect" (a spanned checker
  error since M5).
  * **Availability decides a bare call first** (user-visible consequence,
    2026-09-14): a name that is both an effect member and an ordinary fn
    resolves to the **fn** wherever no instance of the owning effect is in
    scope, without `@` anywhere. The multi-owner case always read
    availability this way; this is the single-owner case of the same rule.
    Without it, std declaring `Fs` claimed `close`, `write`, `read_line`
    and `position` program-wide, and a program with a `close` of its own
    stopped compiling whether or not it touched a file.
    * Recorded for the emitters as `fn_over_member_calls`, since their own
      "is this a member?" question is asked of a program-wide, scope-blind
      map (`Symbols::effect_of_fn`) — the same reason `local_calls` exists.
      **The record is made wherever the fn path wins, not only where the
      member lost a contest** (fixed 2026-09-15): a member of an effect that
      is not in scope *at all* never reaches the availability rule, and a
      written `@module` skips it deliberately, so both used to leave the
      emitters guessing. Two bugs came of it, and only one was loud —
      declaring `effect Tally { fn add(n: Int) }` made *std's* own
      `core/seq.sv` emit a member dispatch for its `add(out, x)` ("no handler
      for effect `Tally`", from inside std, for any program with such a
      member whether or not it called the name), while
      `to_upper@core.string("hi")` **ran the member** and printed `hi!` where
      `HI` was asked for, silently and on both backends.
  * **One overload set where the effect *is* available** (user decision
    2026-09-14): the chosen effect's members and the fn overloads of the
    name are ranked **together**, by the same specificity order two fn
    overloads use [fn-overload-rank], and the more specific signature wins —
    a *generic* member loses to a concrete fn, and `close(InStream)`,
    `close(OutStream)` and `close(Lines)` are three overloads of one name
    across two kinds of declaration. std needs exactly that: `close` is an
    `Fs` member per stream token *and* the `Lines` pass's discharger
    [fs-surface].
    * A **tie** — both sides fitting, neither more specific — is an error
      naming both remedies (`close@Fs(…)` for the member,
      `close@module(…)` for the fn), never a silent preference. A call
      fitting *neither* side is one diagnostic listing both sides'
      signatures, since to the caller they are one name.
    * `@module` skips the member set whole: it names a module's overloads,
      which no member is. That is how a fn shadowed by an equally specific
      member is called by hand.
    * Where a name has candidates on **one** side only, that side resolves
      exactly as it always did — the ranking step exists for the colliding
      case alone, so nothing else could change meaning.
    * The arguments are typed **once**, while the side is decided, and
      handed to whichever path runs. They are typed *without* expected
      types, because the lead-candidate machinery belongs to the fn path
      [fn-overload-rank]: in a colliding call a lambda argument that needs
      its parameter type from the position falls back to
      [type-unknown-lenient] instead of being checked against it. Sharing
      the lead pool across both kinds is the improvement; nothing in std or
      the tests depends on it.
* [effect-disambiguation] With multiple instances of a generic effect in
  scope, a member call disambiguates by (in order): explicit type args
  (`next_random<Int>()`), argument types, the expected type
  (`let i: Int = next_random()`). Still >1 → "ambiguous effect call"
  error; 0 → "no handler" error.
  * Repeats of *one* instance are not ambiguity: they are shadowing, and
    the innermost answers [use-no-dup]. Only *different* instances of a
    generic effect can be ambiguous.
  * The resolved instance is recorded per call site (`effect_calls`).
  * Emitter effect environments are keyed by the *checker's* effect types
    (`fn_effects` for declared lists, `use_effects` for registrations,
    `effect_calls`/`call_effects` for lookups); the backend type
    rendering is a secondary key, used only where no checker type exists
    (unchecked contexts) and for the same-base-name fallback that matches
    a generic callee effect (`Random<T>`) against a concrete instance in
    scope.
* [effect-scope] `use` registrations are block-scoped: they expire at the
  end of the enclosing block — a shadowing one included, so the handler it
  shadowed answers again afterwards [use-no-dup].
  * Checker `effect_env` and emitter environments end the block with the
    same entries they began it with. The checker truncates (a `use` only
    pushes); an emitter whose `use` *rewrites* the entries already in scope
    saves and restores the whole environment instead.

### The filesystem (std, `core.fs` + `core.hostfs`)

* [fs-surface] `core.fs` declares the **surface**: `linear struct FsError`
  over a droppable `FsErrorKind` union (8 arms), the linear stream tokens
  `InStream`/`OutStream`, `FileInfo`, the `Fs` effect (path operations
  *and* stream operations as members [effect-member-overload]), the `Lines`
  pass, the `Chunks` pass, and the one-shots (`read_to_str`, `read_lines`,
  `write_str`, `open_lines`, `read_to_bytes`, `write_bytes_to`, `copy_stream`,
  `copy_file`). Application code declares `[Fs]` (or `[local Fs]` where a
  local binding suffices) and nothing else; the std forwarders themselves
  declare `[local Fs]` — the weakest form [effect-local].
  * Fallible members return `Ok T | Err FsError`: an effect member may
    declare no effects, so there is no `[Throw]` here
    [effect-member-no-effects]. The `Err` arm is linear, so a result that
    is never looked at is a compile error, and narrowing to `Ok` discharges
    it [linear-union-arm]. `ignore(e)` acknowledges, `detach(e)` hands back
    the droppable kind — which is what an aggregation collects, since a list
    of *errors* wants no obligations in it even now that a container could
    hold them [linear-container].
* [fs-token] A stream token is **opaque and never `Mut`** (user decision
  2026-09-14): its only field is the handle, and every byte of mutable
  state — position, buffer, the resource — lives in *handler* state, where
  `Mut` on the token would have claimed a mutation that does not happen.
  Stream members therefore keep their token (`read_line(s: InStream) ->
  Str | None => s`) and only `close` consumes it (`=> !s`), which is what
  makes `close` the discharger [linear-group].
  * Consequences: `Mut` appears nowhere in the fs surface, the tokens need
    no `canbe Mut`, and a token in a field (`Lines`) is read without
    projecting a `Mut` out of it.
  * A token that outlives its minting handler's `use` scope is a clean
    `Err StaleHandle`, not undefined behavior: handler id namespaces are
    per handler.
* [fs-errors-at-close] Read and write errors are **recorded** by the
  handler and surface at `close` (and `flush`): `read_line` reports the end
  of the stream either way, and `write` returns only the byte count, so a
  loop never narrows a result per line. `close` on both token types returns
  `Ok None | Err FsError`, so dropping it on the floor does not compile.
* [fs-bytes] Bytes are part of the v1 surface: `read_bytes(s, max) ->
  Ok List<Byte> | Err FsError` answers **up to** `max` bytes (fewer means
  the stream ended, none means it had ended already) and
  `write_bytes(s, data) -> Long` writes them, neither encoding nor decoding
  anything — a file that is not text is read and written by the same effect.
  * **One stream, one position, counted in bytes**: text and byte
    operations interleave on a stream, so a `read_all` continues exactly
    where a `read_bytes` stopped. This is why the host handlers buffer
    *bytes* below the decoder (`std/platform/core/hostfs.{kt,rs}`) rather
    than reusing a character-counting reader.
  * A ranged open that lands mid-codepoint is a **legal seek** — an offset
    is bytes, and bytes have no characters. The strict decode afterwards is
    what fails (`Err InvalidUtf8`), and the failure is recorded, so `close`
    reports it a second time [fs-errors-at-close].
  * The payload type is **`Bytes`** [bytes-type] — `Vec<u8>` on Rust, the
    shipped buffer class on Kotlin [kt-bytes].
* [fs-read-to] Every read has a **fill-a-buffer** form, for the loop where a
  payload per step is the cost (user decision 2026-09-15): `read_to(s, buf:
  Mut Bytes, max) -> Ok Int | Err FsError`, `read_to(s, buf: Mut Str) ->
  Ok Long | Err FsError` (the `read_all` parallel), and `read_line_to(s, buf:
  Mut Str) -> Bool` (the `read_line` parallel — `false` for end-of-stream or a
  recorded failure, exactly as `read_line` answers `None`). One name for the
  two `read_to`s: the **buffer's type** picks the overload
  [effect-member-overload].
  * They **append**, never overwrite, so `size(buf)` is the data and no
    "only the first n are meaningful" convention exists; `clear(buf)` between
    steps is what makes one buffer serve a loop. It is also what lets `MemFs`
    implement them with `append` alone [fs-double].
  * `RawFs`'s mirror splits the names (`raw_read_to_bytes`,
    `raw_read_to_str`, `raw_read_line_to_str`) rather than overloading: the
    host file is *hand-written*, and an overload set would make it implement
    mangled names [fs-host-split].
  * Riding along: `chunks(s, size)`/`open_chunks` — a pass over a stream's
    bytes as `Lines` is over its lines, **a fresh buffer per step** (a pass
    recycling its own would overwrite what the caller holds) — and the
    one-shots that keep the buffer inside std: `copy_stream(s, w)`,
    `copy_file(from, to)`, `read_to_bytes(path)`,
    `write_bytes_to(path, data)`, `fill_from(s, buf)`.
* [fs-host-split] The host-backed filesystem is a **separate module**,
  `core.hostfs`: `effect RawFs` (plain `Long` handles, droppable kinds),
  `platform handler HostRawFs of RawFs` (std ships
  `std/platform/core/hostfs.{kt,rs}` [platform-handler]) and
  `handler DefaultFs [RawFs] of Fs`, which mints the tokens, maps kinds
  into `FsError` and discharges in Salvo — the host never holds an
  obligation.
  * The split is load-bearing, not cosmetic: `core.fs` declares a `next`
    and a `to_str`, so name-based reachability drags it into ordinary
    programs [mod-used-only], while a *dependent handler* switches the
    whole program to the fused effect emission
    ([rs-effect-fusion]/[kt-effect-fusion]). Keeping `DefaultFs` out of
    `core.fs` is what keeps a program that never opens a file unfused.
  * `[RawFs]` stays greppable as the audit: nothing but a composition root
    (`use HostRawFs()`) and `DefaultFs` reaches raw handles.
* [fs-double] `core.memfs` ships **`MemFs of Fs`**: an in-memory filesystem
  in pure Salvo, with no dependency and no host anywhere, so a test that
  registers it touches no disk. It fakes the *whole* surface — streams
  included — which is what putting the stream operations on the effect buys
  [fs-surface].
  * A fresh `MemFs` is **empty**; write into it with the ordinary surface.
    (Seeding it from a constructor argument would need an immutable `Map` to
    become a `Mut Map`, which std has no route for [type-canbe-mut].)
  * Directories are **implicit**: a path is a key, and a directory exists
    exactly while something under it does. `create_dirs` therefore succeeds
    without doing anything, `list_dir` answers the immediate child names, and
    deleting a non-empty directory is the `IoError` the host reports.
  * **A file is bytes, and every offset counts bytes**, as on the host:
    `MemFs` stores `List<Byte>` and its read cursor *is* a byte offset, so
    `position` and `open_read_at` agree with a real filesystem by
    construction rather than by conversion. Text reads decode strictly off
    those bytes, a decode failure is recorded and reported by `close`, and a
    ranged open landing mid-codepoint succeeds exactly as the host's seek
    does [fs-bytes]. This is the hazard the type exists to get right: a fake
    that stored text and counted characters would let unit tests pass while
    production broke.
* [fs-restricted] `core.restrictedfs` ships **`RestrictedFs(root: Str) [Fs]
  of Fs`**: an *interceptor* [effect-intercept], so it wraps whichever
  filesystem is already registered — the host's in production, a `MemFs` in a
  test of the restriction itself.
  * Paths are **rebased**: the code under it writes `"notes/a.txt"` and never
    learns where it really runs. A path that resolves outside the root is
    refused **distinguishably**, as `Err PathEscapes` — this is a
    least-authority tool for honest code, not a boundary against an adversary
    inside the actor, so debuggability wins. `exists` answers `false` there,
    having nowhere to put a reason.
  * Resolution is **lexical**: `..` segments are resolved right to left, so
    `a/../b` stays inside while `../b` does not, and an absolute path is
    refused outright rather than rebased. It is therefore **not
    symlink-safe** — a symlink inside the root pointing out of it escapes.
    Closing that needs the host (`openat2(RESOLVE_BENEATH)`, cap-std), which
    under this layering belongs to `RawFs`; recorded as the hardening path.
  * The policy lives entirely in the **opens**: the stream members are
    pass-throughs, and a token it forwarded was minted by the handler it
    wraps, which is where the token goes back to.
* [fs-v1-cuts] Not in v1, and each an error rather than a surprise: seek
  (a ranged `open_read_at` replaces it, so streams stay forward-only),
  recursive walk or delete, temp files, watching, permissions, symlink
  creation, and stdin (Console's, not Fs's). `rename_path` is spelled with
  the suffix because `rename` is a keyword [fn-rename].

### Non-resumption: `throw` and `try`

* [throw] `throw(message)` leaves the enclosing delimiter instead of
  resuming. It is declared in std (`core.throw`) as the sole member of
  `effect Throw<M> { fn throw(message: M) -> Never }` and known to the => !message
  compiler by name; the message is *moved* into the outcome.
  * Its type is `Never`, the bottom type: nothing after it runs, so the
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

## Actors — effect handlers bound asynchronously (phase 5 — being built)

The concurrency surface: **an actor is an effect handler bound
asynchronously**. Designed across 2026-09-14/15 (user decisions; the argument
trail and the decided summary are in COMPLETED.md's decision log), and
being built in slices — so each rule below states what already holds and what
does not exist yet. Nothing here is in LANGUAGE.md until the feature runs;
LANGUAGE.md remains the source of truth for everything that does.

* [actor-kind] An **actor** is a handler whose members run one at a time,
  on a scheduler, in the order their invocations arrived: a state struct plus
  one function per member, exactly the handler that `use` binds
  synchronously. The same handler is bindable both ways — the binding
  changes where the body runs and nothing about the code that calls it.
  * The substrate is a **library in each backend's runtime files**
    (`runtime/scheduler.rs`, `runtime/scheduler.kt`), not a runtime baked
    into emitted code: run-to-completion activations on pools, one
    arrival-order queue per actor with an explicit bound, replies with
    reserved capacity, the gate, death as a faulted activation, and the
    idle-with-parked-gates report. Built 2026-09-15, with identical
    behaviour asserted on both backends.
  * An actor **is a region**, so sendability and region-escape are one
    check (user, 2026-09-10, confirmed 2026-09-15). **Handlers never cross
    into a spawn — construction does** [actor-spawn-expr].
* [actor-effect-kind] **`actor effect E { … }` declares an actor protocol**
  (user decision 2026-09-15, EU-5 of the effect-unification design). The kind
  is *declared*, not diagnosed at a binding, because it is a design-time
  choice: sync effects can keep arguments and actor ones cannot, and that is
  something an author reckons with when deciding which effect to write.
  * A **plain effect is never actor-backed** — which is how the design's
    carried named question ("may an ordinary member be actor-backed?")
    closes: *forbid*, with the actor kind as the sanctioned spelling. Since
    SH-3 (user decision 2026-09-19) `spawn`, `use addr` and naming `Addr<E>`
    *accept* a plain effect — as the **monitor** kind [monitor-handler], one
    shared instance behind a lock — which does not reopen the question: a
    monitor has no mailbox and no messages, its members run synchronously on
    the callers' threads, and an ordinary member still cannot be answered by
    an actor.
  * `send fn` requires the actor kind, and is refused in a plain effect.
  * **Mixed kinds are refused outright**, not per member: the same handler
    state would be reachable from two threads, and a synchronous reader can
    observe state mid-activation — precisely what scheduler serialization
    exists to prevent.
  * Inside an `actor effect`, a member may **not**: keep a parameter (a send
    payload always crosses, so it is always consumed — the clause must say
    `=> !p`), take a **`Mut`** parameter (mutating across a boundary would
    share what an actor owns alone), return a **`proj` view** (a borrow of
    state the actor keeps mutating), or carry a **non-sendable** payload
    [actor-sendable]. Each is checked at the declaration and names the law
    rather than the symptom.
  * A handler of an actor effect is still **bindable both ways** — `use H()`
    runs its member bodies inline, `spawn H(…)` runs them as activations. That
    is Example 6's binding swap, and the reason the kind sits on the *effect*
    and not on the handler.
  * `actor` is **contextual** (`actor` followed by `effect`), so nothing is
    reserved and `actor` stays an ordinary name. There is no `actor fn` and no
    `async` anywhere in the language: the phase decided against colouring, and
    the word was renamed from `async` on 2026-09-16 (user decision) partly
    because the old spelling kept implying it. `async effect` is a parse error
    naming this form — worth its own diagnostic, since `async` is what an
    author arriving from another language will reach for.
* [monitor-handler] **A monitor is a plain-effect handler shared behind a
  lock** (SH-3, user decision 2026-09-19 — the first piece of the
  shareable-handler design; the decision round is in COMPLETED.md's log).
  `spawn H(args)` where every face of `H` is a **plain** effect answers the
  same `Addr<E>` an actor spawn answers; the handle is freely copyable and
  sendable, `use addr` binds the effect to it, and every member call locks,
  runs the member on the caller's thread, and unlocks. Since 2026-09-20
  ([use-local]) **a bare `use H(args)` of a stateful plain handler is also a
  sharing site** — the same lock shape, minted at the binding — so the
  spawn spelling and the bare `use` answer one form and `use local` is the
  scope-local, lock-free binding.
  * **The dependency restriction is lifted** (user decision 2026-09-20;
    it was "a monitor-spawned handler declares no dependencies"). A monitor
    may declare dependencies when every one is the shareable default —
    captured as owned handles at construction ([effect-handler-deps]),
    resolved from the enclosing scope exactly as a `use` resolves them. The
    availability rule keeps the capture acyclic: a dep was bound before
    this handler, so no lock order can cycle and no path routes back —
    which is what keeps the JVM-reentrant/Rust-non-reentrant parity trap
    closed [backend-never-wrong], *provided the door stays shut*: no `use`,
    no `spawn`, no `local` deps on a shareable handler (bindings are fixed
    at construction; each is a named blocker under [use-local]).
  * **Waits under the lock are priced, not refused** — option (b), user
    decision 2026-09-20, chosen over a wait-free restriction since the
    deadlock graph already exists: a dep-bearing or waiting shareable plain
    handler gets a graph node (`H's lock`), occupancy-style edges in (from
    handlers depending on its effect) and out (to mixed servants and priced
    monitors of its dep effects; `waitfor` in members as blocks), so a
    cycle through a held lock is reported before it runs, naming the locks.
  * **Refused with it, each by name**: an `on` clause (members run on the
    callers' threads — there is nothing to place); more than one plain
    face *when stateful* (one lock behind several effect types has no
    backend representation yet); unsendable constructor parameters or state
    fields [actor-sendable] (the instance crosses to every thread that
    binds the handle); and a `mailbox` slot, refused at the declaration
    already (a plain-face handler has no queue to bound [actor-mailbox]).
  * **Serialization without a servant**: members are mutually excluded by the
    lock, not serialized by a mailbox — two handle holders' calls interleave
    per member, and there is no arrival order, no gate, no death, nothing to
    `watch`. A member calling a sibling member runs within one acquisition on
    both backends (a direct self-call underneath).
  * **Both binding forms remain**: the same handler `use local`-bound is the
    inline handler (single-threaded, no lock).
  * Calling a plain member *through the addr* (`rng.next()` without a `use`)
    stays refused for now, with the send-member diagnostic; `use` the handle.
  * Lowering: [rs-monitor] and [kt-monitor] — a per-effect lock wrapper
    (`__Mon_E`) implementing the effect's trait/interface by
    lock-and-delegate, so every downstream binding and dispatch path is
    unchanged; the checker and both emitters key the `Addr<E>` representation
    on the effect's declared kind. The wrapper is **generic exactly as the
    effect is** (`__Mon_Random<T>` for `Random<T>`, 2026-09-20), so a
    stateful handler of a generic effect instance shares like any other.

* [mixed-handler] **A mixed handler is a servant and a façade in one
  declaration** (SH-1, user decision 2026-09-19; built the same day). Every
  face a plain effect, plus `send fn` members: the send members and the state
  form the **servant** — an ordinary actor over a handler-local protocol,
  with a `mailbox` (required, [actor-mailbox] keys on "has send members" as
  much as the faces' kind) — and the sync members form the **façade**,
  running on the caller's thread. `spawn H(args)` answers the same `Addr<E>`
  a monitor spawn answers: the façade value — the servant's addr plus the
  constructor parameters plus dispatch to the sync bodies — copyable and
  sendable by construction, which is [actor-use-addr]'s addr *generalized*.
  * **Confinement**: state fields are reachable only from `send fn` members.
    A sync member's environment is its own parameters, the constructor
    parameters (immutable after construction, so freely copied into the
    façade), and sends to its own servant; touching a state field is refused
    by name ("`cursor` is the servant's state…"), and the handler's
    dependency list is invisible to sync members. Even an immutable read is
    refused: it would race an activation that assigns.
  * **The servant send**: inside *either* member kind, a bare call naming
    one of the handler's own `send fn` members resolves as a send to the
    servant — after locals, before the general ladder; every argument is
    consumed (the payload crosses), and the call answers nothing. From a
    sync member it enqueues through the façade's addr; from a send member it
    is a **self-enqueue** on the servant's own mailbox [actor-self-send]
    (user decision 2026-09-19 — sync-only at SH-1, extended the same day),
    and a reply riding the payload must be declared `defer`
    [defer-deduction], exactly as on an addr send. This is the one place
    sibling members resolve — `send fn` siblings only; sync siblings stay
    unresolved as they always were.
    * **Ambiguity is refused, not resolved**: a name that is *also* a
      member of an effect available in the body (a declared dependency,
      once mixed handlers may have them) must say which it means —
      `k@self(…)` for the servant, `k@E(…)` for the effect [effect-at]. The
      arm is dormant until the dependency cut lifts: today a mixed handler
      declares no effects and its members may declare none, so nothing can
      collide.
  * **No `[waitfor]` anywhere** (the user's stated intent, SH-5(d)'s down
    payment): a sync member may `waitfor` with no capability declared — on
    the plain effect's member, on the handler, or at the callers. Occupancy
    is a fact the deadlock graph infers (SH-4): a dependency on a plain
    effect with mixed handlers is an occupancy edge onto each servant node.
  * **Spawn-only**: a `use` of a mixed handler is refused where the binding
    is chosen — its sync members would send toward an instance with no
    mailbox and no dispatcher (the parking-handler reasoning, structural).
    The `on` clause stays, optional as at any actor spawn: the servant is an
    ordinary actor and runs somewhere.
  * **Constructor parameters are copied into the façade**, so they must be
    sendable and must not be `Mut` — a mutable parameter would be *shared*
    between servant and façade on the JVM and *cloned apart* on Rust, an
    observable divergence refused rather than emitted [backend-never-wrong].
  * **A servant `send fn` carries the free send fn's obligations**
    [free-send-fn], because no effect declaration mirrors it: an explicit
    all-consumed clause when it has parameters, no `Mut`/unsendable/generic
    parameters — and no overloading (dispatch is by member name).
  * **A servant parks like an actor** [defer-deduction] [actor-replyto]:
    `replyto k(captures)` inside a `send fn` member targets another of the
    handler's own send members, parks the continuation in the servant's
    table, and resumes on its own mailbox — the received reply it captures
    must be declared `defer`. A gated mint (`replyto!`) makes the servant's
    sends Wait-kind in the deadlock graph, mirroring the actor rule. A mixed
    handler is **direct-answer (rung 3) unless a member declares `defer`** —
    SH-9's default, carried by the declared opt-in.
  * **First-slice cuts, each a diagnostic naming its remedy**: no
    dependencies (take an `Addr` constructor parameter and send to it), one
    face only.
  * Lowering: [rs-mixed] and [kt-mixed] — a handler-keyed message enum/class
    (`__Msg_H`) and continuation enum/class (`__Cont_H`, one variant per
    parked-on send member) and actor body for the servant, a `__Fac_H` value
    for the façade, and the spawn answering the shared handle over it.
    Constructor arguments are evaluated once and shared between handler and
    façade.

* [defer-deduction] **`defer p` is the escaped disposition of a linear
  parameter** (SH-10, user decisions 2026-09-19; built the same day, first
  half). A linear parameter's obligation has three fates relative to a call:
  discharged in-frame, returned, or **escaped** — parked, stored, forwarded,
  captured — and `defer` is the third arm made declarable, riding the
  deduction syntax (`send fn after(d: Duration, out: Reply<Fired>) => !d,
  defer out`). To the *caller* a deferral is a move (`!p` and `defer p` are
  the same contract shape); what differs is what the body may do with the
  obligation.
  * **The load-bearing consequence is the mixed handler's rung-4 opt-in**
    [mixed-handler]: inside a mixed handler's `send fn` member, a `Reply`
    parameter that leaves the activation any way but the discharge (std's
    `send`, its first argument) **must be declared `defer`** — inference
    alone is not accepted at the contract point, so an edit that starts
    deferring errors at the member rather than detonating in a consumer's
    build. Checked at the sites an escape has: passing it to a function,
    forwarding it to another actor, storing it in state, capturing it in a
    continuation.
  * **An upper bound, both ways** (SH-10(b)): a declared `defer` the body
    never exercises is legal — the member reserves the right, and pays with
    graph precision, a self-inflicted price. (The effect-member level of the
    bound has no customer until mixed handlers wear actor faces, and arrives
    with T-4's intersection.)
  * **Nowhere else is the declaration required**: ordinary functions, free
    `send fn`s and actor handlers keep the inferred regime — their escapes
    are already the graph's business through gates, sends and task tracing.
    `defer` on them is checked documentation.
  * **What deferral buys**: **forwarding** — the servant hands the reply to
    another actor, whose discharge ends the caller's wait one activation
    later — and **parking** (2026-09-19, the design's last piece): `replyto`
    inside a mixed handler parks a continuation on another of the servant's
    own send members, handler-keyed (`__Cont_H`), resumed on the servant's
    mailbox. Both run on both backends.

* [actor-sendable] **What may cross a seam** (user decision 2026-09-15, C-4's
  structural rule (a)): a value may not transitively hold
  * a **function value** — a callback is shared rather than owned, and the
    Rust backend holds one in an `Rc`, which is not `Send` [rs-fn-field];
  * a **`proj` view** — a borrow of a value the sender still owns.
  Checked structurally, through struct fields, type arguments, arrays, tuples
  and unions, with a depth guard. The check's *site* is the actor effect's
  declaration for member payloads — where the author is choosing — and the
  crossing site for `replyto` captures and spawn arguments.
  * `Arc`-where-sent inference is the recorded growth point (C-4(c)), for when
    sent closures and pipeline functions become real; until then the answer is
    a diagnostic naming the field.
* [actor-send-fn] An **asynchronous member** is declared `send fn`, in an
  effect and in the handlers implementing it — and, since 2026-09-17, on a
  **free function** too [free-send-fn], where the kind means the same thing
  without an effect to belong to. Sending it *enqueues* an
  invocation, so it **answers nothing**: a written return type is an error
  naming the shape that does carry an answer — an `out: Reply<T>` parameter
  minted with `replyto` [actor-replyto]. In the first pass every member of a
  actor protocol is one; the unmarked `fn` member spelling stays reserved
  for the later call-member sugar.
  * `send` is **contextual**, not reserved (the `iter fn` precedent): a
    state field named `send`, a fn named `send`, and `r.send(v)` — the
    discharge of a reply token — all keep working. The form is recognised
    from `send` immediately followed by `fn` inside a member list.
  * A send member is **exempt from [decl-explicit]**'s "an effect member
    must declare its return type": it has none to declare. The *deduction*
    half still applies — a bodiless `send fn go(c: Config)` writes `=> !c`
    (user decision 2026-09-15: kept for now, revisited when the surface is
    real) — because there the member does have something true to say, even
    though a send payload always crosses the seam and so is always consumed.
* [actor-spawn-effect] `[spawn]` in an effect list is the **capability to
  create an actor** — lowercase and compiler-owned, like `use`, and
  contextual for the same reason `send` is. Accepted on a function and on a
  handler's dependency list (a supervisor spawns its children); refused on a
  fn *type*, exactly as `use` is, because the capability belongs to the body
  that spawns rather than to a value's type.
  * It **propagates through calls** like any effect: a function calling one
    that declares `[spawn]` — std's `pool` and `watch`, or a helper of your
    own — must declare it too. Until 2026-09-16 the gate sat on the `spawn`
    *expression* only, which made the declaration on `pool` decorative: a
    caller reached it through an undeclared helper. Fixed with the `watch`
    slice, since `watch` is the second `[spawn]` function there has ever been.
* [actor-mailbox] **A handler of an actor effect declares its mailbox** (user
  decision 2026-09-16, replacing the frozen `capacity N` spawn clause):

  ```
  handler Desking(room: Int) of Desk {
      mailbox { capacity: room }
      …
  }
  ```

  * **`mailbox` names a compiler-known slot**, and the braces are a **struct
    literal with the type elided** — the slot's type is std's
    `struct Mailbox { capacity: Int }`, so field names, types, a missing field
    and an unknown one are the ordinary struct diagnostics, and a *future*
    setting (an overflow policy, say) is a **field** on that struct rather than
    new grammar. That is what keeps `mailbox` one word of grammar instead of a
    special case.
  * **Required** on a handler of an `actor effect`, with no default: a queue
    bound the compiler chose would be a performance cliff nobody wrote. What
    moved is only *where* it is said — once, by the author who knows the
    protocol's traffic, instead of at every spawn.
  * **Refused** on a handler of a plain effect: its members run on the
    caller's thread, so there is no queue to bound. Under `use`, an actor
    handler's mailbox is simply **inert** — the same handler binds both ways
    [actor-kind], and only one of the two has a queue.
  * Its field expressions see **constructor parameters only** — the bound is
    wanted before the actor exists, ahead of every state initialiser — so a
    state field is not in scope and no effect can be performed to compute one.
    Consuming a parameter there is refused by [effect-state-store] already, a
    `Copy` scalar excepted [copy-scalar-free], which is why
    `mailbox { capacity: room }` is the ordinary shape.
  * `mailbox` is **contextual**: a state field may still be called `mailbox`
    (`mailbox: Int = 3`), and only a brace after the word makes the slot.
  * **No spawn-site override**, deliberately: exposing the bound as a
    constructor parameter *is* the override, and it needs no grammar.
* [actor-spawn-expr] `spawn H(args) with D1(...), addr on POOL` is the
  asynchronous binding, and its value is the child's `Addr`. Read left to
  right: what to run, what it depends on, where it runs — the queue depth is
  the handler's [actor-mailbox].
  * The **`with` clause is optional** and supplies the child's declared
    dependencies [with-clause]; each item is a handler *construction* (a
    private instance, built on the child) or an `Addr` (the same effect
    backed by an actor or a shared handler), which is what lets an actor and
    a local handler swap without touching the consuming code. Arguments
    evaluate in the parent and cross the seam; construction happens on the
    child. What the clause does *not* cover is **inherited from the
    spawning scope** [spawn-inherit], so the clause is for overriding.
  * **The mailbox is not a clause here.** It was `capacity N` until
    2026-09-16, on the argument that a bound is a property of *this instance*;
    the counter-argument won (user decision): the author who knows the
    protocol's traffic is the handler's, the bound is then written once instead
    of per spawn, and a per-instance bound stays expressible by taking it as a
    constructor parameter. What the old rationale got right survives as the
    plain-effect refusal [actor-mailbox].
  * **`on POOL` is optional**, and takes an ordinary expression: `pool(n)` is
    a function declared `[spawn]`, not syntax. Omitted, the child runs on the
    pool **current where the spawn was written** [main-pool] (user decision
    2026-09-17, FC-3's inheritance rule extended to `spawn`) — which is how a
    spawn site names `main`'s own pool without new vocabulary, and what makes
    "run where the work that created you runs" true of a member's spawns too.
    It was required until then (user decision 2026-09-15), on the argument
    that every spawn should say where it runs; what changed is that there is
    now an ambient pool *everywhere* the language owns a thread, so the
    default has a meaning instead of a hole.
  * `spawn` and its clause words are **contextual**; the form is
    recognised from `spawn` followed by a name, so a function named `spawn`
    is still callable. A handler construction is **not a value** — only
    `use` and `spawn` may write one — which is why this is a form with
    clause keywords rather than a call with named arguments.
  * **A spawn is a `use` that runs its handler elsewhere.** The construction
    is checked identically (arguments typed and *stored*, so a bare name
    moves; generics inferred from the arguments and any written type list;
    the constructor's implicits filled), and since 2026-09-20 its
    dependencies resolve the same way too — from the clause where one is
    written, from the scope otherwise [spawn-inherit]. What still differs is
    that the instance runs on the other side of a seam, which is what
    "handlers never cross, construction does" means in practice.
    Consequences, each an error: supplying a dependency the handler does not
    declare, and constructing a clause item that has dependencies *of its
    own*, since there is no scope on the child to resolve those from — an
    `Addr` of an actor already serving the effect is the remedy.
  * **Two orders, and they differ routinely**: the *handler's declaration*
    order is what the child's dependency slots are in, and the *written*
    clause order is what the program says. The checker matches the two while
    resolving the clause and records the matching, so both backends build
    the child's environment in declaration order without re-deriving it
    (and without disagreeing about it) — `with tally, Recording()` and
    `with Recording(), tally` are one program.
  * `on` must be a `Pool`; the mailbox is typed where it is declared. A
    `Dedicated Pool` placement is additionally **consumed** by the clause
    [waitfor-dedicated].
* [actor-waitfor] `waitfor out: Reply<T> { ... }` is the explicit bridge into
  the asynchronous world and `main`'s only token source (`replyto` targets a
  member of the enclosing handler, and `main` has none): it mints a token,
  requires the block to consume it, occupies the thread until it is sent to,
  and yields what was sent. Legal **anywhere** but inside a lambda (SH-5(d),
  user decision 2026-09-19 — a function value's call sites cannot be
  enumerated, so the graph could not place the occupancy): it was `main`-only
  until 2026-09-17, capability-gated for the two days after, and is now what
  a wait always was underneath — an ordinary expression whose occupancy the
  graph infers and the runtime reports [waitfor-effect].
  * "The block must consume it" is **not a rule of its own**: the binding is
    an ordinary linear local, so [linear-obligation] reports a token the
    block never sent to, naming `send` as the discharge. The expression's
    value is the token's *payload* — `waitfor out: Reply<Int>` is an `Int` —
    and a binding whose written type is not a `Reply<T>` is an error about
    what the form does.
  * Paired rule: **the program ends when `main` returns.** Anything still
    running dies with it; a program that means to serve says so by waiting
    on a shutdown token. There is no run-to-quiescence semantics.
* [waitfor-infer] **The binder's type is optional** (user decision 2026-09-21):
  `waitfor out { counter.total(out) }` reads it off the send the block makes, and
  `waitfor out: Reply<Int> { … }` writes it out. The binder is always named —
  there is no placeholder here (the `_` this started as was withdrawn; see
  [placeholder]).
  * **Read from the declared parameter**, not from overload resolution: the
    binder's type is what resolution would need as an *input*, so inference takes
    the declared type at the position the binder occupies, across every overload
    and effect member of that name. One distinct `Reply<T>` is the answer.
  * **Several is an error**, naming the written form — the user's call. **None**
    is an error too, naming both remedies: write the type, or send the token.
  * A `receiver.member(args)` call is tried at **two** positions, because the
    syntax cannot say which it is: dot-notation on a value makes the receiver the
    first declared parameter [fn-dot], while the same shape on an `Addr` is a
    send whose member declares only the message's parameters. Only a `Reply<T>`
    position is accepted, so the wrong guess contributes nothing, and two that
    both hit are the ambiguity above.
  * The walk covers the forms that can contain a call; a shape it does not reach
    produces the "cannot tell" diagnostic rather than a wrong answer, and the
    remedy is written there.
* [waitfor-effect] **The `[waitfor]` capability is deleted** (SH-5(d), user
  decision 2026-09-19; it existed for two days — introduced 2026-09-17 with
  T-5(c), whose argument trail is in COMPLETED.md's log beside the
  deletion's). A wait needs no declaration anywhere: not on the function that
  waits, not on a handler whose member waits, not on an effect member, not at
  any caller. The reasoning, §6 of the retired shareable-handler document:
  after [waitfor-pump] a wait *serves its pool*, so the placement gate priced
  a hazard that no longer exists; the deadlock net is the graph's **inferred**
  occupancy (a handler waits when a member contains a `waitfor`, or when it
  binds a handler that waits — the propagation the capability used to spell,
  computed instead) plus the runtime's named report; and an optional checked
  annotation was examined and rejected (non-propagating it has no consequence,
  propagating-when-declared is incoherent). What survives:
  * **`waitfor` in an effect list is a parse error naming the deletion**, not
    an unknown name.
  * **The lambda refusal, on its own reason**: `waitfor` inside a function
    value would occupy call sites nothing can enumerate, so the graph could
    not place the edge.
  * **The word at the host boundary** (FC-7, unbuilt): a synchronous export
    bridge blocks a host thread that serves nothing — there the hazard is
    real and cannot be inferred into, and the mandatory placement gate
    returns with it.
  * The graph's block edge, **site-inferred** [actor-deadlock-cycle]: from
    `Checked`'s recorded `waitfor` sites and handler constructions, with the
    same severity it always had.
* [waitfor-dedicated] **A dedicated thread is placement one may want** —
  no longer a grant anything requires (SH-5(d), 2026-09-19). `thread()` (std)
  answers a **`Dedicated Pool`** — one fresh thread, owned by whatever is
  placed on it; `pool(n)` answers a plain `Pool`.
  * **The `on` clause consumes a `Dedicated Pool`** (user refinement
    2026-09-17, unchanged): reusing a thread is impossible *by linearity*
    rather than by convention, so a second spawn onto the same `thread()` is
    the ordinary use-after-move diagnostic and needs no rule. `Dedicated` is a
    **provenance qualifier** — a claim about where the handle came from, not
    about the pool's contents — so it survives stores and calls
    [qual-subject], and it is erased in the generated code like every
    qualifier [qual-erasure].
  * The fit: work that genuinely wants a thread of its own — blocking-IO
    wrappers around host APIs, and the FC-7 bridges when they arrive.
  * An actor on its own thread **may** block: it wedges only itself, which is
    its own business, like a gate. What placement does *not* remove is a wait
    whose fulfilment routes back through the waiter's own stalled mailbox —
    that is the edge the graph prices [actor-deadlock-cycle].
* [main-pool] **`main` is the single worker of its own pool** (user decision
  2026-09-17, FC-4(a); the argument trail is in COMPLETED.md's log). The pool
  exists from the start
  and is never given a thread of its own: `main`'s thread is its worker.
  * So **an ambient placement exists on every thread the language owns**, and
    a spawn or (from the task kernel on) a mint with no `on` clause has
    somewhere to land wherever it is written. That is the uniformity the rule
    is for: a function called from `main` and the same function called from an
    actor behave identically.
  * **Actors may be spawned onto it**, which yields genuinely single-threaded
    cooperatively-scheduled programs.
  * Two consequences, each the existing rule seen from a new angle: work
    placed there runs **only while `main` waits** [waitfor-pump], and it dies
    when `main` returns [actor-waitfor].
  * One hazard the statics do not cover: `main` filling a main-pool actor's
    mailbox past its bound blocks the only thread that could drain it. Both
    runtimes report that by name and exit non-zero rather than hanging — the
    sibling of the idle-with-parked-gates report.
* [waitfor-pump] **A wait serves its own pool.** One semantics everywhere
  (user requirement 2026-09-17: no blocking/pumping fork): *a `waitfor` serves
  the work of the pool it is running on, except activations of the actor doing
  the waiting.* Blocking is the degenerate case of an empty queue, so
  "blocking versus pumping" is not a fork at all — one rule whose behaviour
  depends on what is queued, identical on both backends.
  * **For `main`**, which has no mailbox, that is everything on its pool: this
    is what runs main-pool work at all, since nothing else serves it.
  * **For an actor on a dedicated thread**, its mailbox stays stalled while it
    waits, so serialization is preserved and the behaviour is observably
    identical to blocking for *messages*. Its own **tasks still progress**,
    which is what dissolves the default path's trap: [task-pool-inherit] places
    a task on the waiter's own thread, and waiting for that task's answer would
    be a guaranteed self-deadlock under pure blocking.
  * **A task belongs to no actor**, so a wait *inside a task body* excludes
    nothing and may serve every activation on the pool. It cannot re-enter an
    actor mid-activation regardless: an actor running an activation has no
    deliverable entry. And since a dedicated pool has
    exactly one occupant [waitfor-dedicated], "except its own" and "never
    activations" coincide there.
  * **Costs, stated**: pumped work runs nested on the waiter's stack
    (recursion — a pumped item that itself waits nests further, with stack
    depth the budget, the nested-`runBlocking` precedent), and side effects
    are observable *during* a wait. Only the waiting actor's mailbox is quiet.
  * **A wait that cannot end is a named error, not a hang** (defect fixed
    2026-09-18, the prerequisite of the shareable-handler work). A frame parked
    in a wait is **not progress**: it still holds its actor's `running` flag, so
    nothing of that actor's mailbox is delivered by anybody until its token
    arrives. The runtime therefore books parked frames separately from running
    ones, and reports the deadlock when *every* frame it knows about is parked,
    nothing is queued anywhere (no deliverable entry, no task, no deadline, no
    answer already sitting in a waiter's slot) and `main`'s own thread is
    waiting too. The report names the waiting frame, the actors **parked in a
    wait** — whose whole mailbox is stalled — and the **gated** ones, which
    serve only the reply they are waiting for.
    * `main`'s liveness is the load-bearing clause: `main`'s thread runs
      program code without being a frame the scheduler counts, so while it is
      not waiting, it may yet fulfil the token anybody is parked on, and
      nothing may be declared stuck. An actor parked in a wait while `main`
      works is an ordinary program, not a deadlock.
    * The blind spot is the one [actor-on-idle] states: a platform handler with
      a thread of its own can inject work the scheduler never saw.
    * Until the fix the condition required *no* frame to be running, so any
      wait nested inside an activation hid the report and the program hung
      silently — which was tolerable only while `main` was the one thing that
      could wait.
  * The rule is stated in terms of *work* because the task kernel is what
    fills a pool with things that are not activations; until it lands, what a
    wait can serve is main-pool actors.
* [free-send-fn] **`send fn` on a free function** — the send kind extended
  beyond an effect's members (user decision 2026-09-17,
  FC-1(a)). It is a unit of work that runs by being
  **scheduled**, never called:

  ```
  send fn parse_row(out: Reply<User>, row: Row) => !out, !row {
      out.send(user_from(row))
  }
  ```

  * **Why it exists**: in a concurrent program the logic concentrated in
    actors because only a handler member could be a continuation target, so a
    free function was a limited citizen. Extending the *kind* rather than
    making everything asynchronous is the resolution — ordinary functions keep
    the whole synchronous feature set, and the boundary stays where
    [actor-effect-kind] put it.
  * **The refusal list is the actor member's, inherited rather than
    invented**: no return type (it answers nothing — a reply travels as a
    `Reply<T>` parameter), no **kept** parameter (a scheduled body outlives
    the frame that minted it, so what it is given is always consumed: the
    clause says `=> !p`), no **`Mut`** parameter, no `proj` return, and every
    parameter **sendable** [actor-sendable]. Checked at the declaration, where
    the author is deciding.
  * **Not a value and not callable**: a call would run it in the caller's
    frame on the caller's thread, which is the callback anti-pattern the kind
    refuses, and there is no position a fn *value* of this kind could fill.
    Both are errors naming the mint.
  * **No mailbox, no addr, no identity** — so no gate (a gate is a mailbox
    policy), no capacity to reserve, and no death to watch: what a task's
    fault reaches instead is the pool's sink [pool-fault-sink].
  * `send` stays **contextual** at item level for the reasons it is contextual
    in a member list: `send` is an ordinary name and `r.send(v)` is how a token
    is discharged.
  * **First-pass cuts**, each a diagnostic: **no effects but `[waitfor]`** — a
    scheduled body runs detached from the frame that minted it, so there is no
    scope to supply a handler from, and capturing the minting scope's handlers
    would send values that scope still owns; the remedy the diagnostic names is
    an `Addr` capture, which needs no effect declaration [actor-use-addr].
    Lifting it for *actor-backed* effects (whose provider is an addr stub, and
    so sendable) is the recorded growth point. And **no generics**: a mint
    carries captures and no type arguments, so there would be nothing to choose
    an instantiation by.
* [task-mint] **`replyto` targets any send-kind function** (user decision
  2026-09-17, FC-2), which is what makes the mint legal in **any** function:

  ```
  fn fetch_user(id: Int, out: Reply<User>) [Db] -> None => !out {
      db.query(id, replyto parse_row(out))     // wires work, then returns
  }
  ```

  * **Resolution is lexical-member-first**: the enclosing handler's members are
    tried first (the existing rule, unchanged), then free `send fn`s by the
    ordinary scope ladder [fn-overload-scope]. A tie *within* one rung is
    refused rather than guessed — a mint carries only captures, which is not
    enough to choose an overload by.
  * **Targeting a plain `fn` stays an error**: a normal function runs by being
    called, and a fulfilled token must never run arbitrary synchronous code on
    the fulfiller's thread inside its activation budget.
  * The **trailing-token convention is unchanged**: the target's last
    parameter is what the token carries, the ones before it are the captures,
    and a capture is *stored*, so a bare name moves [deduce-consume] and is
    checked sendable at the crossing site [actor-sendable].
  * **`replyto!` cannot target a task**: the gate holds back the minting
    actor's mailbox, and a task has none.
  * **No deadlock edge at the mint** [actor-deadlock-cycle]: no mailbox to
    fill, no gate to cycle. A strict simplification relative to a member mint,
    whose capacity is reserved in a bounded queue.
  * **What the graph does see** is the task's *body* (FC-6): its sends are
    traced whole-program and attributed to every actor whose mints reach it,
    following task-to-task mints — conservative, and the same coarseness the
    type-level graph already accepts. The recorded gap: an obligation parked
    *in* a task is a wait-for edge pointing at no effect node, netted by
    linearity [linear-obligation] and the runtime's idle report.
  * **Direct invocation** (`parse_row(out, row) on p` as a statement) is
    *derivable* — a mint plus an immediate self-discharge — so the kernel ships
    targets-only and the spelling stays a separate decision.
* [task-pool-inherit] **A task's placement is inherited unless written**
  (user decision 2026-09-17, FC-3): `on POOL` is optional at a mint, and
  omitted means the pool **current at the mint site**, resolved there and
  captured into the token. The fulfiller's pool is irrelevant — *whoever
  creates work pays for it*, which is also what keeps an actor's continuations
  on the pool its author budgeted.
  * The rejected alternatives, recorded: **required-`on`-always** (either
    threads a pool through every signature or reads the ambient pool through
    an accessor — the same ambient read with ceremony), the **callee's pool**
    (ill-defined, and it lets clients spend a shared service's budget), and a
    **dedicated task pool** (a global noisy neighbour that breaks CPU/IO
    budgeting). Swift's `Task { }` and Kotlin's coroutine builders both
    converged on inherit-by-default.
  * A **member** mint takes no `on` clause at all: the answer arrives on the
    actor's own mailbox, so there is no placement to choose. Writing one is an
    error saying where a placement belongs.
  * **A task inherits its pool's serving rules, and the main pool's are
    unusual** [main-pool] [waitfor-pump]: work placed there runs only while
    `main` waits. So a mint from `main` (or from anything `main` calls) whose
    answer arrives after `main`'s last `waitfor` **never runs**, and dies with
    `main`'s return — the same fate as an actor message still queued there, and
    the paired rule [actor-waitfor] already states it. It is worth knowing
    anyway, because "wire it and forget it" is the shape that hits it: from
    `main`, a task is only *reached* by a subsequent wait. On any other pool a
    worker picks it up as soon as it is queued, and a **task before an
    activation** is the order both backends take.
  * **The typed exception** [waitfor-dedicated]: a target that declares
    `[waitfor]` needs dedicated placement — explicit `on thread()`, or
    inherited from a frame that itself declares `[waitfor]`, whose ambient pool
    is thereby provably a `Dedicated Pool`. The proof travels in the effect
    lists, so this stays a binding-site check rather than an ambient one.
* [pool-fault-sink] **A pool may name an actor to report its faults to**
  (user decision 2026-09-17, FC-5(a)): `pool(n, sink)` where `sink` is an
  `Addr<Faults>`, and every uncaught fault on that pool arrives as an ordinary
  message.
  * **Why an addr and not a `watch`**: the two shapes differ. A death watch is
    a one-shot linear token for the death of an *identity*; a pool has no
    lifecycle and emits a recurring *stream*. So the sink is an ordinary actor
    of an ordinary protocol — `actor effect Faults { send fn faulted(fault:
    Fault) }` in `core.actor` — and needs no new mechanism at all.
  * **What reaches it**: a faulted **task**, which has no addr to watch, and a
    faulted **actor nobody was watching**. Per-addr `watch` is untouched — the
    sink is the net *beneath* supervision, not its replacement, which is the
    arrangement OTP also has.
  * **No sink is the named runtime report** on stderr, the sibling of the
    idle-with-parked-gates report. A program does not die of a task's fault.
  * The report is enqueued **past the sink's bound** deliberately: a fault
    report must not block the faulting thread, and dropping it would lose the
    one thing the sink exists for.
  * **Minter attribution** (structured concurrency's answer — every task chain
    bottoms out at an owning activation) is the recorded refinement if the
    per-pool sink proves too coarse for *recovery* rather than diagnosis.
  * The spelling is **positional**, `pool(n, sink)`: the design sketch wrote
    `pool(4, faults: sink)`, but a name before a colon at a call site is the
    implicit-override syntax [implicit-override], not a named argument — so an
    ordinary overload of `pool` is what the surface actually needs.
* [actor-use-addr] `use addr` binds an effect in the current scope to a
  generated forwarding stub over an `Addr` — first-pass surface, not sugar, and
  no new syntax: the `use` statement already takes an expression, and whether
  a bare name is a handler construction or an `Addr` is a checker question. Its
  value is unqualified calls and, above all, passing the capability *down*
  through ordinary effect lists (`fn drive() [Roll]`).
  * Binding **does not consume** the addr: an addr is freely copyable, so the
    holder keeps it and may send through it directly as well.
  * **Dot-call through an addr** (`counter.total(out)`) is the inline form of
    the same binding: the receiver names *where* the message goes rather than
    being the member's first argument, the member is resolved against the
    effect the actor serves, and the payload is *consumed* — which is how a
    reply token's linearity is discharged by sending it onward. Only a
    `send fn` is reachable this way in the first pass: a member that answers
    would have to park its caller, which is the call-sugar pass (and the
    named question of whether an ordinary member may be actor-backed at
    all).
  * The receiver may be any **place** whose type is an addr — a variable, a
    field chain (`registry.child`), a tuple element, an array element — since
    a place has a type the checker can read without checking the expression
    twice. A receiver that is a *call* (`get(addrs, 0).bump(1)`) needs a `let`
    first; nothing is lost but a line.
  * An overloaded send member is picked by **argument count**
    [effect-member-overload]. Typing the arguments to choose the overload and
    again against the winner's parameters would report every mistake in them
    twice; arity settles every overload the first pass can express, and a
    same-arity tie is refused rather than guessed.  * **`use H(args) on POOL` is the spawn-and-bind sugar** (SH-7, user
    decision 2026-09-19; built the same day): one shared instance serving an
    effect in a scope, in one line — parsed as a `use` whose handler is a
    spawn expression, so the spawn machinery and the addr binding each do
    their own half and the emitters need nothing new. The `on` clause is the
    marker (without it, `use H(args)` keeps its scope-local meaning); a
    multi-face handler is refused by name (one binding cannot split the
    tuple); and a dependency clause does not fit the sugar yet — it arrives
    with the `using` rename, which unambiguates the two `use`s.

* [actor-self-send] `k@self(args)` — send a message to **the actor the
  enclosing member belongs to** (user decision 2026-09-15, option (a) of
  three). The one thing an unqualified call cannot say: that would be
  self-dispatch, which a handler has no way to perform, and it would run `k`
  *now* rather than as its own later activation. So the form's point is
  **ordering** — "finish this activation, then continue with `k`" — which is
  otherwise unwritable, since extracting a function runs the work
  immediately.
  * Defined for **both bindings**, like everything else on this surface: an
    enqueue on the actor's own mailbox when the handler was spawned, and the
    ordinary inline member call when it was `use`d (which is what a local
    binding of a `send` protocol already does).
    * A handler is compiled **once**, so which reading applies is a property
      of the *instance*, not of the source: both emitters discriminate at run
      time on the same generated field the mint reads — the actor's own
      address, absent exactly when the instance was bound synchronously
      ([rs-actor], [kt-actor]). One field, two rules, no second
      compilation.
  * **Inside a mixed handler** [mixed-handler] the form works in both member
    kinds (user decision 2026-09-19): in a send member it enqueues on the
    servant's own mailbox unconditionally (a mixed handler is spawn-only, so
    there is no inline reading to discriminate), and in a sync member it is
    the explicit spelling of the façade send. It is also the
    **disambiguator** where a bare call would be ambiguous — see the servant
    send rule under [mixed-handler].
  * `k` must be a member of the enclosing handler and a **`send fn`**: a
    member that answers would have to wait for itself. Arguments are typed
    against its parameters and *consumed* — the message outlives this
    activation even though it never leaves the actor.
  * **A selector, not a receiver** (user decision 2026-09-15, revising the
    `self.k(…)` form that shipped for a few hours): `k@self` joins the family
    `k@E` (an effect's member [effect-at]) and `k@module` (a module's overload
    [fn-overload-at]) — one rule for "the call says which it means" instead of
    a second, dot-shaped mechanism beside it. It is also the spelling the sugar
    tower's merge/join form already assumes
    (`k@self(c, reply e1, reply e2)`), and the disambiguator the generalized
    mint will need when a bare `k` could name either an enclosing member or an
    in-scope async one.
  * `self` is **contextual, not reserved**: it means the enclosing handler only
    immediately after `@`, so it stays an ordinary name elsewhere and no local
    can shadow the form — which the receiver spelling could not promise (it
    needed a rule refusing a variable named `self` inside a member; the
    selector deletes that rule). Since [effect-at] reads a lowercase name after
    `@` as a module path, a module named `self` is unreachable this way, which
    costs nothing.
  * The **old spelling is a plain parse error** naming the new one — no
    transitional accept, per the no-backwards-compatibility invariant.
  * The self-dispatch diagnostic names this form as the remedy when the member
    it refused was a `send fn`.
* [actor-no-closure] **No first-pass form crosses a closure**: `spawn`,
  `waitfor` and `replyto` are all errors inside a lambda body, and so is
  `k@self(…)` — `self` names nothing there. A function
  value's body runs wherever it is *called*, and none of the three can travel
  with it — a fn type cannot declare `spawn` [actor-spawn-effect], a `waitfor`
  inside one would occupy call sites nothing can enumerate (so the graph could
  not place the edge [waitfor-effect]), and a continuation belongs to the
  handler that minted it. Each diagnostic says so
  in the closure's terms, since "add the capability to the effect list" is not
  available for a lambda. ([fate-lambda], the escaping-closure work, is
  deferred to the call-sugar pass for exactly this reason: nothing in the
  first pass needs a form to cross one.)
* [actor-replyto] `replyto k(captures)` mints a parked one-shot continuation
  targeting member `k` of the **enclosing handler** and yields its `Reply<T>`
  token, which is **linear** [linear-obligation] and discharged by sending to
  it: `r.send(v)`. The arguments are the continuation's *captures* — what the
  member needs besides the answer — and are positional; the parentheses are
  part of the form even when empty.
  * **`k`'s parameters are the captures, then the answer**: the *trailing*
    parameter is what the token carries, so `send fn arrived(id: Int, sum:
    Int)` minted as `replyto arrived(7)` is a `Reply<Int>` (the trailing-token
    convention the sugar tower's `-> T` also follows). Writing the wrong
    number of captures is an error stating how many the member wants.
  * `k` must be a member of the handler the `replyto` is written in, and a
    **`send fn`**: an answer arrives as a message. Outside a handler the form
    is an error naming `main`'s alternative, `waitfor` — `main` has no members
    for a continuation to target.
  * **A parking handler may only be `spawn`ed** (user decision 2026-09-15):
    bound with `use`, its member bodies run inline on the caller's thread, so
    the mint would target a member of a *local* instance — which has no
    mailbox for the answer to arrive on and no dispatcher to run it, leaving
    the continuation silently dead. Refused at the `use` site, naming `spawn`.
    * The gate is syntactic per **handler**, not per member, because effects
      propagate: a fn declaring `[E]` may call any member, so a `use` site
      cannot know which ones its scope will reach. The over-refusal that
      buys — a `use` of a handler whose *other* members are the only ones
      called — was judged theoretical: parking is the one thing only a
      actor can do, so a handler that parks is an actor.
    * It does not touch Example 6's binding swap, which swaps a *dependency*
      between a construction and an `Addr` in a spawn clause, never a `use`.
  * **The target is resolved lexically**, against the enclosing handler's
    members — and then, since 2026-09-17, against free `send fn`s
    [task-mint], which is the one case where the mint needs no enclosing
    handler at all. Naming a member of another `actor effect` in scope is a
    *remote mint* — the generalized form (decided; ROADMAP.md's sugar pass) — and
    is refused for now with the workaround that needs nothing new: a token is
    an ordinary linear value, so the handler that owns `k` mints it and passes
    it. The generalization is a later slice because it makes the mint itself
    send-like: capacity has to be reserved in the *target's* bounded queue, so
    a remote mint can block and contributes its own wait-for edge.
  * A capture is *stored* in the continuation, so passing a bare name moves
    it ([deduce-consume]), like a `use` constructor argument. A caller's own
    reply token travelling as a capture is the ordinary way to answer later;
    since 2026-09-16 a handler may also park tokens in **state**, in a
    `Mut List<Reply<T>>` whose terminal is `drain` [linear-container]
    [linear-state].
  * `replyto!` is the same mint **plus the gate**: bounded selective
    receive, at most one outstanding per actor, so the actor serves
    nothing else until the answer arrives. Self-only by nature, so it stays
    lexical with everything above.
  * A token is one-shot *statically*, which is what linearity buys over the
    dynamic enforcement the effects literature settles for.
* [actor-types] The three types the forms produce and consume live in
  **`core.actor`**, all `intrinsic` because each is a handle into the
  scheduler its backend ships:
  * **`Addr<E>`** — what a `spawn` hands back, and the whole of what one
    actor knows about another. Named for what the design's own prose calls
    it: a **many-shot address typed by a protocol**, with `Reply<T>` the
    one-shot address typed by a single value. (It was `Pid<E>` for a few hours;
    renamed by user decision 2026-09-15, because `Pid` is the operating
    system's word and a `platform effect` wrapping process management will want
    it. `Addr` also keeps signature-heavy code short — `List<Addr<ShardApi>>` —
    and rarely collides with a domain noun the way `Address` would.) Its argument is the **effect** the actor
    serves, the one sanctioned effect-in-a-type-position [effect-not-data]
    (user decision 2026-09-15): what a holder may *do* with an addr is exactly
    that effect, and parameterizing by it is what lets an actor and a
    locally `use`d handler stand behind one name. The exception is one
    argument of one type — `List<E>` and every other position stay refused,
    and an addr is still not a handler instance.
  * **`Reply<T>`** — the one-shot answer channel, `linear intrinsic type`
    [linear-opaque], discharged by `send(r, v)` (`r.send(v)` in dot form).
    Linearity is what makes "answered exactly once, on every path" a
    *static* guarantee.
    * **Capacity is reserved in the token's target when the token is
      minted**, which is why a discharge never blocks and never counts
      against the mailbox bound: the room for the answer was taken when the
      request was made. The rule generalizes unchanged when the mint does
      (the sugar pass's remote mint reserves in *another* actor's queue, so
      minting becomes send-like and can block; the first pass's lexical mint
      reserves in the minting actor's own).
    * Sending to a token whose target has died is a silent no-op, as every
      send is [actor-watch].
  * **`Pool`**, with `intrinsic fn pool(size: Int) [spawn] -> Pool` — an
    ordinary value, so one pool can be shared by many spawns; `on pool(2)`
    is a call, and the `[spawn]` on the function is what makes creating one
    a capability. **`intrinsic fn thread() [spawn] -> Dedicated Pool`** is the
    other one: a pool of exactly one thread, and the placement a
    `[waitfor]`-carrying handler needs [waitfor-dedicated]. `Dedicated` is a
    `provenance qualifier` on `Pool`, declared in `core.actor` like any other
    — the compiler owns no new vocabulary for it — and the `on` clause
    consumes a value carrying it.
  * An addr is **never linear and freely copied**: a send to a dead actor is
    a silent no-op, so a stale addr is safe to hold and death is *observed*
    with `watch` rather than tripped over.
  * **`Exit`**, with `intrinsic fn watch<E>(target: Addr<E>, on_exit:
    Reply<Exit>) [spawn]` — the monitor surface [actor-watch]. The only
    `<E>` in std that stands for an *effect*, which is what makes one
    function serve every protocol.
  * **`Idle`**, with `intrinsic fn on_idle(p: Pool, notify: Reply<Idle>)
    [spawn]` — the quiescence hook [actor-on-idle], `watch`'s shape applied to
    an event that belongs to no identity.
* [actor-watch] **`watch(target, on_exit)` is the whole monitor surface**
  (user decisions 2026-09-15 S-1…S-4, spelled 2026-09-16): one function, one
  struct, and no new syntax — because a death notification *is* an answer, so
  the request/response machinery already carries it.
  * **Death is a faulted activation**, and nothing else. There is no `kill`,
    and a Salvo-level `throw` cannot cross a member boundary
    ([effect-member-no-effects] means a `send fn` can never carry
    `[Throw<M>]`), so a program with no platform handlers and no backend
    faults cannot experience death at all. Each backend catches at its
    dispatch boundary (`catch_unwind` / `try`), marks the actor dead, and
    records the reason.
  * **The registration is the obligation**: `on_exit` is consumed, so a
    minted-and-unregistered token is the ordinary leak [linear-obligation] —
    "you cannot silently forget you were watching" needs no rule of its own.
    `main` mints with `waitfor`, a handler with `replyto`, so watching needs
    no special context.
  * **Watching an already-dead actor answers immediately**, with the reason
    that death recorded — so a watch that loses the race to a fast fault is
    not a watch that never answers, and a spawn-then-watch pair needs no
    ordering care.
  * **`[spawn]`-gated**, like `pool`: a monitor is part of running actors.
  * **The corpse**: its queue is dropped, its obligations are lost (statically
    checked linearity cannot survive a crash [linear-static]), and sends and
    fulfils to it are **silent no-ops** — the only composable rule, since a
    send that could fail on a dead target would make *every* send fallible.
    The monitor is the recovery mechanism; a requester gated on a dead callee
    is caught by the runtime's **idle-with-parked-gates report**, which names
    the parked actors and exits non-zero instead of hanging.
  * **`Exit`'s `reason` is the host's text** — a panic message on the Rust
    backend, an exception's on the Kotlin one. It is the one thing on this
    surface that is *not* identical across backends (the same hole
    `IoError { message: Str }` already accepts): print it in a diagnostic, do
    not branch on it. The struct rather than a bare `Str` is the growth point
    for a `kind` when a non-fault death becomes expressible.
  * **The runtime cannot build it**, so the *watch site* hands the scheduler a
    builder along with the token ([rs-actor], [kt-actor]) — which keeps a
    watcher's payload identical to an ordinary `r.send(Exit{…})` instead of
    special-casing the delivery.
  * **Supervision is a pattern, not a construct**: an interceptor (a handler
    of `E` depending on `E`) that holds its child's addr privately, watches
    it, and respawns on `Exit` gives clients a stable addr and never lets them
    observe the death. Restart strategies, intensity budgets and escalation are
    handler logic; a std `Supervisor` waits for real usage to shape it.
* [actor-on-idle] **`on_idle(p, notify)` is the quiescence hook** (T-3(a),
  user decision 2026-09-17; built 2026-09-18). The runtime already knew when
  nothing could run — that is what the idle-with-parked-gates report reads —
  and this exposes the same detection through [actor-watch]'s shape: a linear
  one-shot token, answered with an `Idle { parked_gates: Int, parked_tokens:
  Int }`.
  * **When it fires: when nothing anywhere can run.** No activation running, no
    mailbox with a deliverable entry, no queue of scheduled work — the
    scheduler's existing predicate, unchanged. Firing on the *named pool's*
    quiescence alone is the recorded refinement and deliberately not the rule:
    a pool can be idle while another pool holds work that will send into it, so
    the narrow reading answers "settled" to a program that is not.
  * **What `p` decides is the answer, not the timing**: `parked_gates` counts
    the actors placed on `p` whose mailbox is gated on a reply that has not
    arrived [actor-replyto], and `parked_tokens` the reply tokens aimed at work
    on `p` — a continuation parked on an actor there, a task waiting for its
    answer, a frame of that pool parked in a `waitfor` — that nobody has
    discharged. Both zero is *done*; either non-zero is *idle and still owed
    something*, which is the difference a test needs and a diagnosis wants.
  * **A registration the scheduler holds is not an outstanding token.** A
    `watch` and an `on_idle` both hand their token to the runtime, which will
    answer it when the event happens — so counting them would make every
    steady-state program look stuck.
  * **Edge-triggered and one-shot**, for the reason the answer itself is work:
    delivering it ends the idleness that produced it. Hearing about the next
    one means registering again, and a program that wants a stream of them
    re-registers from the member the answer wakes.
  * **The obligation is the registration** [linear-obligation], as with a
    watch: the token is minted the ordinary way — `waitfor` in `main`,
    `replyto` in a handler — and consumed here, so a forgotten hook is the
    ordinary leak diagnostic rather than a silently dropped request. It is
    `[spawn]`-gated for `pool`'s reason: asking about the scheduler is part of
    running actors.
  * **It composes with the deadlock report rather than competing**: a pending
    hook is *progress*, so a waiter fires it and looks again, and the report is
    what firing nothing leaves. The two predicates are no longer the same one,
    though, since the report's hole was fixed (2026-09-18, [waitfor-pump]): the
    **report** counts a frame parked in a wait out of the running frames, where
    the **hook** still counts it as running — so idleness does not fire while
    any wait is in flight, and a program that is stuck *with* a parked frame
    gets the report rather than an `Idle` answer. Whether the hook should read
    the report's weaker condition (firing `Idle { parked_gates, parked_tokens }`
    where the report would kill the program) is an open call, recorded in
    ROADMAP.md: it is observable behaviour, so it is the user's.
  * **Meaningful only while nothing outside injects work** — a platform handler
    with a thread of its own can stale the answer. The same caveat the report
    has always had, stated where a program can now read the answer.
* [actor-deadlock-cycle] **The static deadlock baseline: a cycle check over
  the effect graph** (design decision 2026-09-14, built 2026-09-16). Nodes are
  `actor effect`s — that is what an `Addr` is typed by — and a cycle means
  actors that can wait for each other. Two severities, because there are
  two kinds of edge:
  * A **gated mint** (`replyto!`) is a *wait-for* edge: while the
    continuation is outstanding the actor serves nothing but its answer, so
    a cycle of gates deadlocks whenever the requests cross, whatever the load.
    An **error**, naming the cycle as a path, the handlers on it, and the two
    ways out (make one side's continuation ungated, or have the answer come
    from a third actor). A bare `replyto` leaves the mailbox open and
    contributes **no** edge — which is why one ungated side is enough.
  * An **occupancy** edge [mixed-handler] (SH-4, user decision 2026-09-19;
    built the same day) is *inferred, never written*: an actor handler that
    declares a dependency on a plain effect with **mixed handlers** in the
    program may call a façade member, which parks its activation until the
    servant answers — and a parked activation serves nothing of its own
    mailbox. One edge per mixed handler of the effect, from every served
    protocol to the servant's own node (`H's servant` — a mixed handler
    serves no actor effect, so it is a node in its own right, whose outgoing
    edges are its servant's sends, classified as an actor's are: a gated
    park makes them wait-for edges, an inferred wait makes them blocks,
    otherwise back-pressure — task sends counted the same way). A cycle containing an occupancy edge is
    an **error with no downgrade**, even when back-pressure closes it: the
    ungated-side argument (the other actor keeps serving) is exactly what an
    occupied activation's `running` flag removes. The report is anchored at
    the dependency declaration — the seam where the binding is chosen — and
    names the three ways out: answer without reaching the peer, respell the
    consulting call as a send plus a continuation, or bind a handler of the
    effect that does not wait (a monitor, or a scope-local `use`). Over
    types, not instances, like every edge here: a program that binds a
    non-mixed handler everywhere still gets the edge if a mixed one exists.
  * An ordinary **send** is a *back-pressure* edge: it blocks while the
    target's bounded mailbox is full, so a cycle of sends deadlocks only when
    the mailboxes fill together. A **warning** (user decision 2026-09-16):
    the program is legal and usually fine, and refusing every pair of
    actors that send to each other would refuse most useful topologies.
    The remedies it names are a larger `capacity` or routing one direction
    through a reply token, whose capacity is reserved at park time.
  * An **inferred wait** is a *wait-for* edge too, and the third edge kind
    [waitfor-effect]: a handler whose member contains a `waitfor` — or which
    binds a handler that waits, the propagation the deleted capability used
    to spell, computed over recorded sites and constructions since SH-5(d)
    (2026-09-19) — serves no message while it waits, so a cycle through it
    deadlocks exactly as a gate cycle does, and a thread of its own does not
    save it: the fulfilment has to route back through the stalled mailbox.
    An **error**, anchored at the wait site (or at the construction that
    brought the wait in).
  * A `fulfil` (`r.send(v)`) contributes nothing: the capacity is already
    reserved, so it never blocks. Neither does a `k@self(…)` — a self-send
    waits for no *other* actor (the wedge a full own mailbox could cause is
    a recorded gap, in ROADMAP.md).
  * **Interception is exempt.** A handler of `E` declaring `[E]` — a policy
    wrapper around the actor already serving `E` — is an `E → E` edge by
    construction, and it can never deadlock: the dependency binds strictly
    *outward* [effect-intercept], so the chain ends at the innermost instance.
    An edge from a handler's **own-effect dependency** is therefore dropped,
    while an `Addr` of its own protocol (a genuine peer mesh) still counts.
  * **Sound, not precise, and stated so.** The graph is over actor *types*,
    not instances, so a chain of same-protocol workers is a self-loop here and
    acyclic in the running program; and a handler's *declared dependencies*
    stand in for what its members can reach, so a gate in one member and a
    send in an unrelated one make an edge. What that costs is a diagnostic on
    a legal program. Stratification (a tier qualifier on an addr) and the
    fallbacks (a timeout form, a per-edge reentrant opt-in) are the recorded
    answers, deliberately unbuilt until the false positives are *observed*
    rather than predicted.
* **Built so far, and what is refused meanwhile.** The surface **runs**, and as
  of 2026-09-15 it runs **without `main` in the loop**: a program can spawn a
  handler — a **dependent** one included, its dependencies supplied by its own
  `use` clause as constructions, as addrs, or a mix — send to it through its
  `Addr`, park a continuation with `replyto` / `replyto!` for an answer that
  arrives *at another actor*, continue an activation with `k@self(…)`, and
  bridge with `waitfor`. Since 2026-09-16 it can also **watch an actor die**
  [actor-watch], and a topology that could deadlock is reported before it runs
  [actor-deadlock-cycle]; since 2026-09-18 it can **hear when the work runs
  out** [actor-on-idle]. All of it on **both backends, with identical output**
  ([rs-actor], [kt-actor]). `use addr` **runs**: it binds a generated
  forwarding stub, so a function declaring `[Log]` never learns that its
  capability is an actor — and the same stub is what a spawn clause's addr
  becomes, which is how a child's dependency swaps between a local handler and
  an actor without the child changing at all. What is still refused, each with
  a diagnostic naming it: spawning a **generic** handler, a **generic effect**
  as a protocol, a **remote mint** (a `replyto` target reached through the
  effect list rather than lexically), and `use` of a handler that parks. The
  sugar tower — member `-> T` with call syntax, `then`/`then!`, `defer`,
  merge/join, the gate's member-set generalization, and the generalized mint —
  is later passes, each with its own decision surface.

## Time (std, module `time`)

Built 2026-09-18 (step 5 of the second sequence), on the user's decisions of
the same day. **Not part of `core`**: the surface is imported, and one
`import time` [mod-import-module] brings all of it.

* [time-types] **Three types, one representation**: `Duration` (a span),
  `Instant` (a point on the wall clock, nanoseconds since the Unix epoch) and
  `Tick` (a point on the monotonic clock, from an arbitrary origin). Each is a
  plain std struct with a single `nanos: Long` field, `canbe hashed, ordered`.
  * **One field, deliberately.** Struct equality is structural
    [col-equality] and fields are public, so a `{secs, nanos}` pair would make
    non-canonical values constructible — `{secs: 1, nanos: 0}` and `{secs: 0,
    nanos: 1000000000}` are one span and would compare unequal. With
    nanoseconds in a `Long` every value is canonical by construction.
  * **Two point types, not one.** The wall clock jumps (NTP steps and slews
    it; a suspend advances it while the monotonic clock stops), so a deadline
    measured against it would move under the program; the monotonic clock
    never jumps but has no epoch. Keeping them apart makes the mistake a type
    error — `between` takes two of one timeline, and comparing an `Instant`
    with a `Tick` is refused where it is written [col-equality]. A single type
    with `Monotonic`/`Wall` provenance qualifiers was considered and rejected:
    `==` ignores qualifiers, so a cross-timeline comparison would compile and
    answer `true`.
  * **Signed** durations, which is what makes `between` total: arguments the
    other way answer a negative span rather than trapping or clamping.
  * **Ranges**, stated rather than hidden: a `Duration` spans ±292 years and
    an `Instant` covers 1678–2262 — the right window for machine events.
    Historical dates and month arithmetic belong to the later calendar layer
    (`DateTime` as a *view* of an `Instant` in a zone, with a `Period`-shaped
    span), which is additive over this.
  * **The surface is named functions**, since operators are numeric-only
    [op-arith]: `nanos`/`micros`/`millis`/`seconds`/`minutes`/`hours` build a
    span, `to_nanos`/`to_micros`/`to_millis`/`to_seconds` read one back
    (truncating), `plus`/`minus`/`times`/`abs` compute, `to_str` renders
    (integer and the largest unit that divides exactly, seconds at the top:
    `1500ms`, `120s`, `37ns`). Points: `epoch_nano`/`epoch_milli`/
    `epoch_second` in, `to_epoch_*` out, `plus`/`minus` with a span, and
    **`between(start, end)`** overloaded for both timelines — named for how it
    reads at the call site, the argument order being the direction of the
    answer.
* [time-ticker] **`effect Ticker { fn tick() -> Tick }`** is the monotonic
  clock, and reading it is a *capability*: a function whose answer depends on
  when it was called has a dependency, and Salvo's dependencies live in
  signatures. `DefaultTicker` is the machine's, `elapsed(since)` is
  `between(since, tick())`.
* [time-clock] **`effect Clock`** is the wall clock: `now() -> Instant`, plus
  the bridge between timelines — `to_instant(at: Tick)` and
  `to_tick(at: Instant)`, named so dot-notation reads (`t.to_instant()`).
  * The conversions are **members rather than free functions** because they
    are an *estimate*: nothing exposes the monotonic origin, so relating the
    two timelines means reading both clocks at nearly the same moment and
    keeping the difference — which then drifts (slew, steps, suspend). A
    handler owns that correlation, which is what makes the conversion
    available at all, and exact in a test.
  * `DefaultClock` takes the correlation **once, at construction**, so the
    conversion is a fixed affine map and therefore order-preserving: earlier
    ticks convert to earlier instants. Re-reading per call would track
    adjustments better and could answer out of order, which is the worse
    surprise. `now()` reads live.
  * Both defaults are **ordinary Salvo** over two intrinsics —
    `monotonic_nanos()` and `epoch_nanos()` — so the arithmetic is the same on
    both backends and nothing about the drift model lives in a backend
    ([rs-time], [kt-time]).
* [time-timer] **`actor effect Timer { send fn after(wait: Duration, done:
  Reply<Fired>) => !wait, !done }`**, with `struct Fired { at: Tick }`.
  * An **actor** effect because run-to-completion leaves nothing else
    [actor-kind]: a handler cannot block mid-body, so "wait two seconds" can
    only mean parking a continuation. `after` therefore takes the
    continuation and answers nothing, and the caller mints it with `replyto`
    or `waitfor`.
  * The payload carries a **`Tick`**, not an `Instant`: a deadline that moved
    when the wall clock was adjusted would not be a deadline.
  * **No cancellation** in this cut: a timer nobody wants fires into a
    continuation that finds its work done — one no-op activation, the shape of
    a lost race.
  * `DefaultTimer` is an **ordinary Salvo handler** (`mailbox { capacity: 64
    }`) over one `intrinsic fn fire_after(wait, done)`, which dissolved the
    flagged "first intrinsic handler for an actor effect" case: no new emitter
    capability was needed. Its mailbox bounds *registrations*, not deadlines.
  * The runtime keeps **one deadline structure and one thread** for the whole
    program, started by the first registration ([rs-time], [kt-time]) — no
    thread per timer, no polling. A pending deadline counts as work on its
    way, so it holds off both the quiescence hook [actor-on-idle] and the
    deadlock report: a program waiting for a fire is waiting for time to pass.
* [time-manual] **`handler ManualTime() of Timer, TimerCtl`** is the
  pure-Salvo fake — `MemFs`'s answer applied to time [effect-handler-multi].
  Virtual time starts at zero and moves only through
  `TimerCtl.advance(by)`, firing every deadline it passes **in deadline
  order**, with virtual `now` standing *at* each deadline as it fires.
  * The two faces are the point: a spawn answers an addr per face, so the code
    under test holds the `Timer` and cannot reach `advance` — least authority
    out of the types.
  * Its state is a `Mut List<Long>` of deadlines **beside** a `Mut
    List<Reply<Fired>>` of tokens, index-aligned, because a list holding
    obligations cannot be *read* positionally — only `remove_at` reaches one
    [linear-container]. Keeping the deadlines in a plain list is what lets the
    handler ask which is earliest.
  * A deadline that is **already due fires at registration**, inside `after`,
    rather than waiting for the next `advance` — which is what the real timer
    does ("as soon as the scheduler looks") and what makes `after(nanos(0),
    done)` a *reading* of virtual time rather than a park that never ends. Added
    2026-09-18 with [time-coupling], whose unified test clock hangs without it.
  * `advance` races the `after` registrations of the code under test, which is
    what `on_idle` is for: settle, then advance ([actor-on-idle], and the
    worked test in both backends' `time-manual` case).
  * One name is exposed that a module system would hide: `fire_after`, the
    default timer's plumbing. Calling it needs a `Reply<Fired>` in hand, so it
    cannot manufacture time from nothing, but Salvo has no module-private
    declarations — recorded in ROADMAP.
* [time-coupling] **Time is data.** Where a value can be *passed*, std's posture
  is to pass it rather than to read it: a `Fired` carries the `at` it came due
  at, and a function that takes its times as parameters declares no effect,
  needs no handler and is tested by being called. Decided 2026-09-17 (T-2
  option 1) and made good 2026-09-18; the worked arc is `examples/time/`.
  * The argument is not just testability. A function that reads an ambient clock
    mid-body has an answer that depends on when the scheduler ran it — inside an
    actor, a race with its own mailbox — so passing the time in *removes* the
    dependency instead of mocking it. What the stance costs is stated too: code
    that wants ambient `now()` in the middle of a computation must be
    restructured to be handed it.
  * Three test postures follow, in the order to reach for them, and the language
    supports all three today:
    1. **Time as data** — no effect at all. Nothing to bind.
    2. **Scripted readings** — a `Ticker`/`Clock` handler of your own whose
       answers come from its constructor or its state. For code that reads a
       clock but shares no time with anything else in the test; it does *not*
       agree with a `Timer`'s virtual time, and is not meant to.
    3. **`ManualTime`** [time-manual] — virtual time for the deadlines
       themselves, sequenced with `on_idle` before each advance.
    4. **`ManualTime` plus a unified test clock** — for a measurement that must
       agree with a deadline.
  * The **unified test clock** is the principled endpoint, and it is written in
    Salvo rather than provided: a `Ticker` (or `Clock`) handler whose reading is
    a zero-length deadline on the timer the test advances, so one virtual clock
    is behind both.

    ```
    handler TestTicker(timer: Addr<Timer>) [waitfor] of Ticker {
        fn tick() -> Tick {
            let fired = waitfor answer: Reply<Fired> { timer.after(nanos(0), answer) }
            return fired.at
        }
    }
    ```

    Two rules already in force shape it, and neither is negotiable here. The
    timer arrives as an **`Addr<Timer>` constructor parameter**, not as a
    handler dependency, because a handler that itself depends on an effect
    cannot be *constructed* in a spawn's `with` clause — there is no scope on the
    child to resolve that dependency from, so an addr is what crosses
    [actor-spawn-expr]. And the wait
    makes the handler carry `[waitfor]`, which propagates to whatever binds it,
    so the compiler requires that actor to run on a `Dedicated Pool`
    [waitfor-dedicated] — `on thread()`, consumed by the clause. A wait can
    therefore occupy only its own thread. std ships no such handler: it is six
    lines, and which effect it fakes (`Ticker`, `Clock`, or both) is the test's
    business.
  * **Scheduler-owned virtual time is the recorded, un-built upgrade path**
    (T-2 option 4; kotlinx-coroutines' `TestCoroutineScheduler` and Tokio's
    `pause()` are the precedents). A pool would own a clock that the *production*
    `DefaultTicker`/`DefaultClock`/`DefaultTimer` read, so a test would rebind
    nothing and clock/timer agreement would be automatic. It is not built
    because it moves time into both runtimes and forfeits the pure-Salvo fake;
    what it uniquely buys is now only that agreement, since [actor-on-idle]
    already supplies the sequencing. Revisit if the dedicated thread the unified
    clock costs, or the advance ergonomics, start to bite.

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
    | `=> !list` | consumed (caller loses access); `list: Never` says the same |
    | `=> list: A B` | **exhaustive**: afterwards *only* `A B` apply |
    | `=> list: None` | exhaustive and empty: every qualifier stripped |
    | `=> list: -A` | **delta**: drop `A`, everything else survives |
    | `=> .f: proj[from: a]` | the result's field `f` projects `a` [proj-infer] |
    | `=> v.f: proj[from: a]` | the parameter `v`'s field is re-pointed to project `a` [proj-infer] |
    | `=> proj[from: a, b]` | opaque: the result holds a borrow of `a` and `b` somewhere inside [proj-infer] |
    | `=>[f] entry, …` | a group: entries about the fn-typed parameter `f`, whose own parameters are named in its type [fn-contract] |

  * A `=>[f]` group reaches a fn type **through its qualifiers**: a
    `once (t: T) -> None` parameter is a qualifier group wrapping the fn type,
    and until 2026-09-13 the scoping refused it — which had locked the
    contract out of exactly the parameter that most wants one, since a
    callback that *consumes* what it is given can only be called once
    ([once-fn]; `hand_over` in `examples/linearity/` is the shape).

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
    exhaustive or a delta, never both; `Never` is the only type form
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
    `Never`, and any later reference to it is a compile error (a
    `Never`-typed value represents an impossibility). Reassigning the
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
  poisons every variable derived from it: they narrow to `Never` with
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
    statement (`map(iter(list_of(1, 2)), f)`, `for x in iter(list_of(1, 2))`)
    but not bound, returned or stored (`let p = iter(list_of(1, 2))`: "cannot
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
  **and every effect member** declared in the *type's own file* whose
  contract consumes a parameter of the type — `close` for a file,
  `stop`/`join` for a thread, `remove(cache, entry)` for a pooled handle
  (context parameters are ordinary parameters; generic dischargers count).
  Declaring a `linear struct` with no discharger is an error at the
  **struct** ("no legal death"), and leak diagnostics enumerate the set.
  * **Members join the set** (user decision 2026-09-14, entailed by phase
    4's stream ops being effect members — FILE_SYSTEM.md §5.8): a member's
    written clause is its contract ([decl-explicit] makes it complete), and
    the same-file rule keys on the *effect's* file, so a module cannot
    declare a member that disposes of another module's linear type.
  * Discharger status attaches to the **member declaration**, so **every
    handler's implementation** of a consuming member is a discharge context
    [linear-discard] — the real handler, a test double in another module, an
    interceptor discharging by forwarding into the handler it wraps. An
    *overloaded* member gives each body its own contract: the
    `close(InStream)` implementation may discard an `InStream`, not an
    `OutStream` [effect-member-overload]. A **keeping** member's body may
    not discard at all.
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
  * **Not yet covered**: nothing — the opaque half arrived 2026-09-15 (below).
* [linear-opaque] **`linear intrinsic type Reply<T>`** — the obligation
  modifier on an *opaque* type (user decision 2026-09-15, taken for phase 5's
  reply token: a token is a scheduler handle, so its representation belongs
  to the backend and there is nothing to make a `linear struct` out of). Every
  rule of [linear-group] applies unchanged — the declaring file must contain a
  discharger, every value owes, `linear` is unwritable in a use-site type —
  and the only difference is that an opaque type has no fields for linearity
  to reach *through*.
  * The discharger for one is an `intrinsic fn` beside it, since a bodiless
    declaration's written clause is its whole contract [decl-explicit]:
    `intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply,
    !value` is std's, and it is why "a `Reply` is a one-shot `Addr` with a
    single send member" is true in the type system rather than only in prose.
  * Only an `intrinsic type` may carry it, never an **alias**: an alias is a
    second name for a type that has already decided whether it owes.
    `intrinsic` is std-only [intrinsic-std-only], so a linear opaque type is
    std's to declare — which is the point, the user's reason being that
    unifying the synchronous and asynchronous effect surfaces will want this
    control in the compiler's hands.
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
* [linear-composite] A composite may not **hold** a linear value **unless it
  opts in** (the interim refusal of 2026-09-08 — roadmap R4 part 2, replacing
  decision L6d's contagion — narrowed by [linear-union-arm] on 2026-09-12 and
  by [linear-container] on 2026-09-16, which is L8's answer). What remains
  refused, always *at the store*, so the container is never built and there is
  no follow-on leak to report:
  * a **struct field** whose declared type is linear — including a container
    of obligations (`handles: Mut List<Lines>`), which makes the struct a
    resource too: contagion stays **spelled**, so the diagnostic asks for
    `linear struct` [linear-group];
  * a **written composite** with a linear component and no opt-in, one level
    at a time so a nested one reports at its own node: an array element
    (`Lines[]`), a tuple component — but **not** a union arm
    [linear-union-arm], and not an opted container position
    [linear-container];
  * an **array or tuple literal** whose element type is linear (nothing
    written to refuse);
  * a **struct literal** field taking a linear value where the field's
    declared type is generic and unopted (`struct Box<T> { item: T }`), which
    is the store the struct's own declaration cannot see;
  * a **call** that takes a bare `T` and puts a `T` inside an *unopted*
    composite. `add(list: Mut List<T>, elem: T)` was this refusal's shape
    until `List` opted in; a signature that only *reads* a composite of `T`
    (`size(list: List<T>) -> Int`) stores nothing and was always legal.
  * Consequently a *union* is not a container: a value of `Lines | Int`
    **is** the linear value, so the obligation survives narrowing and
    branch merges (an inferred union arm keeps it, a written one is legal).
    Neither is `T?`, which is a union — and that is what makes every
    take-by-move signature (`remove_first(list) -> T?`) express "the
    obligation comes back out" rather than "the obligation is stored".
  * **Arrays and tuples stay out** of the opt-in for now (LC-3): an array
    sits next to the untracked variadic boundary, and a tuple has the
    union-arm alternative for the two-things case. Neither is a customer
    shape; extend if one appears.
  * **The casualty that remains** is the composed linear pass (a wrapper pass
    over `open_lines("a")` stores its source in an unopted field). The
    fallible-open shape `Ok InStream | Err FsError` was the other one and
    shipped with phase 4 [linear-union-arm].
* [linear-container] **A container is linear exactly when its element type
  is** (LC-1/LC-5, user decisions 2026-09-15/16 — the answer to roadmap L8's
  "composition plus conditional linearity are one design question"). This is
  Linear Haskell's model, and it needs no new spelling: the *element's*
  declaration is the source of the linearity, the container's `canbe linear`
  is a conditional carrier, and nothing at a use site ever says which.
  * **The opt-in** is per type parameter, on the declaration:
    `struct Box<T canbe linear>` for a user type (2026-09-12) and
    `intrinsic type List<T canbe linear>` for an opaque one (2026-09-16). A
    struct's parameter must **reach a field** to count — a parameter the
    fields never mention stores nothing — while an opaque type has no fields
    to read, so an opted parameter holds by definition, which is exactly what
    `List` claims about its elements.
  * **The judgment is structural and transitive**: an instantiation is linear
    iff a type argument in an opted position is, so `List<Reply<Str>>` and
    `List<Mut List<Token>>` owe while `List<Int>` is an ordinary list.
    Depth-guarded, and mutually recursive containers terminate
    conservatively.
  * **Which containers**: `List` and `Map` **values** (LC-3). `Set`,
    `SortedSet` and map **keys** are refused, and the diagnostic says *why* —
    insertion deduplicates, so an equal element or a repeated key drops one of
    the two values, and a silent drop is what linearity exists to prevent. It
    is semantics, not an implementation fence: no API reshaping fixes it,
    because returning the displaced element would make set-insert
    order-dependent in a way `==`-based dedup cannot honestly express.
  * **The operations** (LC-2), audited into std under `<T canbe linear>`:
    * **in**: `add(list, v)` moves the value in; the obligation joins the
      container's. `put(map, k, v)` stays closed to obligations — it answers
      nothing, so what it overwrote would be dropped — and the diagnostic
      names `replace(map, k, v) -> V?`, which hands the displaced value back.
    * **out, one at a time**: `remove_first(list) -> T?`,
      `remove_at(list, i) -> T?`, `remove(map, k) -> V?`. The `T?` shape is
      the whole absence story: the `None` arm owes nothing, so the emptiness
      check *is* the union narrow [linear-union-arm]. `get`/`first` stay
      closed — they answer a borrow, and an alias would let one obligation be
      discharged twice.
      * The **binding takes the obligation out**: `remove_at(pending, i) is
        Reply<Fired> token` moves the payload rather than copying it, recorded
        by the checker at the binding and honoured by the emitters
        [rs-linear-move]. Before 2026-09-18 the Rust backend read a narrowed
        binding as `.as_ref().unwrap().clone()`, which duplicates an
        obligation for a `Clone` handle and does not compile at all for a
        reply token — found while writing [time-manual]'s deadline queue.
    * **the terminal**: `drain(list, each)` / `drain(map, each)` consumes the
      container and hands every element to a consuming callback
      (`=>[each] !x`). A container that is neither drained nor moved onward is
      an ordinary leak whose diagnostic names **`drain`**, which is the whole
      point of the model: forgetting a queue of obligations is a compile
      error.
    * **no `clear`**, which would be a mass drop, and **no positional write**
      for a list: `replace(list, i, v)` would have to answer `None` for an
      out-of-range index and drop the value it was handed. A map's `replace`
      has no such hole (a fresh key simply stores it).
  * **The terminal is a callback rather than a `for`** (D7-a, user decision
    2026-09-16): a `for` cannot consume a linear temporary
    [iter-drive-in-place] and the language has no implicit discharge site, so
    `for x in drain(list)` — the shape the design document sketched —
    would have needed one of the two rules to change. A callback also has no
    half-drained state to account for: a drain either happened or did not.
    The cost is that a *lambda* performs only the effects its type declares
    [fn-effects], so an **effectful** discharger cannot fill the position
    today; the recorded shape for it is a `for`-driven form, in ROADMAP.md.
* [linear-state] **Handler state may hold obligations, and an activation must
  leave it whole** (LC-4, user decision 2026-09-15). This is where the
  concurrency surface actually lives — a queue of parked reply tokens, a map
  of gathers — and it is the one place the strong guarantee weakens, so the
  weakening is stated rather than discovered: **the actor owes until it
  ends**, and what end-of-life does with parked obligations is `watch`'s
  answer [actor-watch].
  * Within an activation the discipline is unchanged: take an obligation out
    of a field, and either discharge it or put something back. A member that
    *returns* with a state field moved out is an error — it would leave the
    actor with a hole a later activation would read, and no analysis can
    know what an actor holds at an arbitrary future point.
  * **A container is the form.** A *bare* obligation in a field
    (`held: Token`) is refused, naming the container: taking it out would
    leave the hole above and nothing could be put back, so its obligation
    would have no reachable discharge at all. `Mut List<T>` and
    `Mut Map<K, V>` take one element at a time and leave the storage intact.
  * A `Copy` scalar field is unaffected: handing one over copies it, which
    leaves no hole — the exemption [effect-state-store] already grants.
  * Drain paths stay *forced* wherever the code path exists: a `Shutdown`
    member that does not answer its parked tokens is a leak, so the drain
    loop is compulsory rather than stylistic.
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
    audit opts in `list_of`, `mut_list_of`, `add`, `size`, `discard`
    (whose declaration is honestly
    `intrinsic fn discard<T canbe linear>(value: T) -> None`) and — since
    2026-09-16 — the take-by-move and terminal surface (`remove_first`,
    `remove_at`, `drain`; `remove`, `replace`, `drain` on a map), while
    `get`/`first` stay out (they answer an alias of an element), `put` stays
    out (it drops what it overwrites) and `copy` refuses with a dedicated
    message (duplicating an obligation is meaningless). Since
    [linear-container] the opt-ins are permissions to **store** as well: the
    container carries the obligation, and its terminal is where it dies;
  * a linear value cannot be passed in a *variadic* position (variadic
    arguments are untracked, so the obligation would be physically
    moved but statically unresolvable);
  * `canbe` on a type parameter is accepted on **fns**, **structs** and
    **type declarations** (the last since 2026-09-16, which is what makes an
    opaque container conditional [linear-container]); on a qualifier's or an
    effect's parameters it stays a parse error. Only `linear` is accepted in
    the clause.
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
  its `.sv` files), and everything **exported** [mod-export] by `core.*`
  (implicit) or by whatever it imports.
* [mod-export] **Declarations are module-private by default; `export` lets one
  out** (user decision 2026-09-18). A declaration without it can be used only
  inside the file that declares it [mod-file]; with it, it is part of the
  module's public surface and nothing else is.
  * The modifier comes **first**, before `intrinsic` / `linear` / `actor` /
    `platform` / `provenance` / `iter` / `send`, so a declaration reads
    "who can see it, what kind it is, what it is called": `export fn size(…)`,
    `export linear struct Ticket { … }`, `export actor effect Timer { … }`,
    `export intrinsic fn epoch_nanos()`.
  * **Contextual, not reserved**, like `iter`, `send` and `actor`: a variable,
    parameter or field may still be called `export`. There is nothing to
    disambiguate at item level, where a bare identifier is a parse error
    anyway.
  * **Declarations only.** `export` before an `import` is refused (there is no
    re-export: a module states what *it* declares), before a `refn` (a
    refinement travels with the qualifier whose claim it is about
    [qual-refn-scope]) and before a `rename` (a name for the writing file's own
    use [fn-rename]).
  * **It applies to the whole declaration, not its parts.** An exported struct
    exports its fields, an exported effect its members, an exported handler its
    state. There is no member- or field-level visibility, and the fields being
    public is load-bearing elsewhere ([time-types] rests on it). A type nobody
    exported is simply unreachable.
  * **An `iter fn`'s generated pass type inherits its visibility** [iter-fn]: a
    `for` over the pass needs the *type* in scope, so exporting the function
    while hiding its pass would make the iterator undrivable from another
    module.
  * **A private type may appear in an exported signature**, and that is an
    *opaque type* rather than an error: the value flows, and only its name is
    unavailable. Nothing is unsound — there is no separate compilation — and
    Salvo has no other spelling for opacity. Revisit if it proves to be a
    mistake more often than a tool.
  * **Diagnostics name the reason, not just the absence.** A use of a private
    name says "declared in module `m` but not exported", names the fix, and is
    *not* offered as an import suggestion [diag-import-suggest], which could
    not work. An `import` of a private name is refused **at the import**, where
    the reader is looking. And a name that is in scope under another *kind* —
    a qualifier written where a type belongs — keeps its own diagnostic: it is
    not an export problem.
  * **Emission is unchanged**: the rule is enforced in the checker, and both
    backends emit exactly what they emitted before (Rust `pub`, Kotlin
    top-level). Narrowing generated visibility would buy only dead-code
    warnings that are already tolerated, and cross-module emission is a known
    fragile seam (see COMPLETED.md's `__mailbox_capacity` gotcha). Recorded as
    a later refinement.
  * **A hover does not show it** (user decision 2026-09-18): the hover already
    names the module a declaration comes from [lsp-fn-origin] — "the standard
    library", "another file of the program", "this file" — which is what a
    reader wants from it, so the modifier would say the same thing twice. The
    editor colours the word itself as a keyword instead, through the TextMate
    grammar, which is the only thing that highlights anything (the language
    server has no semantic tokens).
  * std obeys the rule like anything else, which is what it was built for: its
    internal helpers (`fire_after`, `earliest_due`, `mem_*`, `fs_resolve`,
    `MemRead`/`MemWrite`) are now genuinely unreachable instead of merely
    undocumented.
* [mod-import] `import path.Name` / `import path.Name as Alias`; aliasing
  resolves ambiguity. Unresolved/ambiguous imports are errors.
  * Import prefixes match module paths exactly or as a leading path
    (`import core.Str` finds `core.string`).
* [mod-import-module] `import time` imports a whole **module** — every name
  in it, and in every module under it (`time.clock`, `time.timer`), by the
  same prefix match the name form uses (user decision 2026-09-18, with
  `core.time` moved out to module `time`: a std surface that is not
  implicitly visible needs one line to reach, not one line per name).
  * **The reading is decided by the path**, not by new syntax: every
    segment lowercase *and* at least one module matching means the module
    form, since a type is uppercase [name-casing] and a fn import still
    has a module prefix in front of it. A single-segment path can only be
    a module, so an unknown one says so and lists the importable modules
    rather than reporting the `module.item` shape.
  * **A bulk import never fights anything.** It enters at its own ladder
    rung — above implicit `core.*`, below a named import and below this
    module's own declarations [fn-overload-scope] — and loses *silently*
    both ways, because a convenience import must not break a file that
    declares its own `Span`.
  * **Functions need no diagnostic**: two modules' overloads of one name
    coexist on the ladder and `f@time(x)` names either [fn-overload-at]
    (user decision 2026-09-18). Only where two *bulk* imports carry one
    **type** name is there nothing to disambiguate with, so the first in
    module order wins and the second **warns**, naming the named-import
    remedy.
  * **No alias**: `import time as t` is an error. An alias renames one
    imported name, and there are no module-qualified type references for a
    module alias to qualify.
  * `import core` is redundant — reported as a warning rather than adding
    every core name at a second rung, which would put each core overload
    into the set twice (the shape [qual-refn-match] is sensitive to).
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
    and the `Never`-narrowing cascade onto the subject. Motivation:
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
  where the implementation comes from. With [platform-handler] it is one of
  the two interop paths customer code has, and the one for a capability that
  is the *host's* rather than the language's.
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
  * `platform` takes `effect` or `handler` [platform-handler]; the parse
    error names both forms rather than reporting a bare "expected item". A
    platform *type* remains deferred (user decision 2026-09-05).
* [platform-handler] `platform handler H of E` declares a handler of an
  **ordinary** Salvo effect whose implementation is a class the *build*
  supplies, in the target language (FS-1 resolved as O-M2, user decision
  2026-09-14; the deferred 2026-09-05 proposal, un-deferred because
  phase 4's `HostRawFs` needs it). Where a `platform effect` hands the whole
  effect to the host, this hands it one *handler*: the effect stays Salvo's,
  with as many other handlers as it likes.
  * It is registered with `use` like any handler — that is the point of the
    form — so **the entry point does not move**: the instance is
    constructed inside the program, not handed to it. Constructor
    parameters are passed through to the host class
    (`use HostS3("bucket")`).
  * Under [use-local] a platform handler is **assumed thread-safe by its
    design** (user decision 2026-09-20) and classifies bare/stateless, so
    handlers depending on its effect share with no `local E` anywhere. The
    assumption is unvalidated for now — a declaration-level contract (and
    what the compiler could check of it) is a follow-up (ROADMAP). Sharing
    mechanics differ per backend ([rs-platform-handler],
    [kt-platform-handler]): Kotlin shares the raw host instance, Rust
    shares it behind the per-effect lock adapter because its members take
    `&mut self` — for a host that honors the assumption the two are
    observationally equivalent, and the residual divergence is recorded
    with the follow-up.
  * Nothing is emitted for the declaration itself: the effect's
    interface/trait is emitted as any effect's, and the `use` site
    constructs the *host's* class by name — `salvo.platform.<module>.H`
    (Kotlin) / `crate::platform_<module>::H::new(…)` (Rust). The class is
    named after the **handler**, since the `use` site names it.
  * The class lives in the `platform/` companion of the module that
    *declared* the handler [platform-tree], and `salvo platform generate`
    writes its skeleton. std ships its own, under `std`'s `platform/` tree,
    one file per backend — the same mechanism, a different author.
  * A `use` with no host companion for that module is an error naming
    `salvo platform generate` [backend-never-wrong]: the alternative is
    generated code referencing a class nobody wrote.
  * **Three restrictions**, each because the implementation is not Salvo's:
    no body (no members, no state — the host class holds both); no effect
    dependencies (a dependency is supplied to *members*, and these members
    are host code, which performs no Salvo effect — put an ordinary Salvo
    handler in between, `handler DefaultFs [RawFs] of Fs`); not generic (the
    host writes one concrete class, as for a platform effect).
  * A `platform handler` of a *platform effect* is the ordinary
    "platform effects have no Salvo handler" error [platform-effect].
* [platform-tree] The host implementations live in the source root's
  **`platform/` tree**, mirroring the source layout: `platform/app/entry.kt`
  implements the platform effects *and platform handlers*
  [platform-handler] of module `app.entry` in Kotlin,
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
    The same is required of the module declaring a platform handler the
    program `use`s [platform-handler] — that one does not move the entry
    point, so the trigger is the `use`, not `main`.
  * The tree is not the *customer's* alone: std ships its host classes
    there too (`std/platform/core/fs.kt`), which is how a std
    `platform handler` is implemented. The embedded std is loaded with the
    active backend's extension, exactly as a source directory is.
  * Both backends' host files coexist in one tree, because discovery only
    ever picks up the active backend's extension — the same sources build
    for both targets.
* [effect-member-unique] Within one effect a member **signature** is
  unique: two members with the same name *and* the same parameter types are
  an error at the second declaration (source order, so the diagnostic is
  deterministic and fires once). Signatures are compared as *lowered* types,
  so two spellings of one type are the duplicate they are.
* [effect-member-overload] A member name may recur, **within one effect and
  across effects** (user decisions 2026-09-14: the cross-effect ban lifted
  in the fifth round — `close` on `Fs` and on `Net` is the natural spelling
  — and within-effect overloading decided in the sixth, §5.10.2
  sub-question A, because phase 4's `Fs` declares `close` and `position`
  once per stream token).
  * **Across effects**: `Symbols::effect_of_fn` and the scope's member table
    are multimaps, and a bare call resolves through the one candidate effect
    that is *available* (an instance in the effect environment). None
    available, or more than one, is an error naming the selector form
    [effect-at]. The original ban existed because there was no such syntax.
    Several same-named members of *one* effect are one candidate here, not an
    ambiguity between effects.
  * **Within one effect**: the overload is picked by arity, then by the
    argument types ranked exactly as a function overload's are
    [fn-overload-rank] — arguments are typed once and reused, so nothing is
    checked twice. No overload fitting is an error naming the member and the
    argument types; several fitting with none most specific is an ambiguity
    error. Both are errors, never a guess [backend-never-wrong].
  * The chosen overload is recorded per call
    (`Checked::effect_member_calls`, the member's index in declaration
    order) because the emitters cannot re-derive it from a name.
  * **Emitted names**: every overload after the first is suffixed
    (`close`, `close__2`, … — `salvo_core::effect_member_name`, shared so the
    backends cannot disagree, and so an interface, a handler's override, a
    fusion's forwarding impl, a `platform generate` skeleton and a call site
    all say the same thing). Rust has no trait-method overloading at all;
    Kotlin would resolve by *Kotlin's* type lattice, the [kt-fn-mangling]
    hazard. Positional suffixes need no qualifier pass, since every overload
    but the first is renamed regardless.
  * A handler's implementing member is matched to its effect member by name
    and written parameter types (`salvo_core::effect_member_index`), which is
    what tells two overloads apart.
  * The emitters read the checker's per-call resolution
    (`Checked::effect_calls`); the name-keyed fallback only answers when
    the name has a sole owner.
  * Known leftover: the LSP def-site table is name-keyed, so
    go-to-definition on a *shared* member name lands on one declaration
    (last collected).
* [effect-at] `member@Effect(args)` selects the effect a member call goes
  through — `close@Fs(h)`, dot form `h.close@Net()` — parsed by case: a
  **capitalized** name after `@` is an effect, a lowercase one starts a
  module path [fn-overload-at]. The named effect must be in scope, must
  declare the member, and must still have a handler available (the
  selector picks the effect, not a handler out of thin air). The call's
  type arguments keep their [effect-disambiguation] meaning — they pin a
  generic effect's *instance*: `next_random@Random<Int>()`. A selected
  member is a call form, not a value.
  * The selector is a checker mechanism: it narrows what
    `Checked::effect_calls` records, and emission is the ordinary member
    call.
* [backend-companion] A backend-native source file next to a module's
  sources (`complicated.kt` beside `complicated.sv`, using the backend's
  native extension) is a *companion*: it is copied verbatim into the
  output whenever its module is reachable. It is how hand-written native
  code joins the build — most importantly the host implementations of a
  `platform effect` or `platform handler`, which live in companions under
  the source root's `platform/` tree [platform-tree]. A companion that collides with a
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
  * **Keywords by position** are highlighted by *shape*, not by name (user
    requests 2026-09-18). Every contextual word the parser recognises has an
    entry in `CONTEXTUAL_PATTERNS`, each shape mirroring the parser's own
    `at_word(w) && peek_at(1)…` test:
    * modifiers before the declaration they modify — `actor effect`,
      `send fn`, `iter fn`, and `mailbox {`;
    * the asynchronous expression forms — `spawn H(…)` (a name follows),
      `waitfor out:` (a name and a colon follow), `replyto k(…)` and
      `replyto! k(…)`;
    * the lowercase capability effects inside an effect list
      (`[use, spawn, waitfor]`), matched by the comma or bracket on each side —
      `use` needs no entry, being a real keyword;
    * the placement clause `on POOL`, the projection source in
      `proj[from: list]`, the `hashed`/`ordered` claims of a `canbe` clause,
      and `self` immediately after `@` (`k@self(…)`), which renders as a
      language variable rather than a keyword.
    A variable named `on`, a field named `from` and a local named `ordered`
    all stay plain, which a name list could not have managed. `watch`,
    `on_idle`, `pool` and `thread` get no entry at all: they are ordinary std
    *functions*, and highlighting them as syntax would misdescribe them.
    The `canbe` patterns come *first* in the keyword list, since TextMate
    takes the first pattern that matches at a position and the plain
    alternation would otherwise consume `canbe` and leave `hashed` unmatched.
    A test asserts the table covers every contextual word the parser names.
  * **`[symbol]` doc references highlight inside comments** [doc-symbol-ref]
    (user request 2026-09-18): the comment rule is a `begin`/`end` pair with
    one inner pattern, so a reference — and a `[rule-label]` in the compiler's
    own comments — reads as a link rather than as more comment text.
  * The checked-in extension grammar must byte-equal the generated one
    (`vscode_extension_grammar_is_up_to_date`); regenerate with
    `cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json`.
* [lsp-hover-linear] A hover shows **obligations and bounds**, because they are
  what a reader cannot infer from the name (user requests 2026-09-18):
  `linear struct Ticket` and `linear intrinsic type Reply<T>` lead with their
  modifier exactly as the source does, and a generic that may be handed an
  obligation shows its bound — `fn hold<T canbe linear>(value: T)`. Bounds
  render for structs and opaque types too, which is what says a container is
  *conditionally* linear [linear-container].
* [lsp-hover-overloads] A hover lists **every other declaration visible under
  the name**, under "Also visible under this name" (user request 2026-09-18).
  A name can carry several declarations at once — overloads by argument type
  [fn-overload-rank], same-named effect members [effect-member-overload], and
  the two mixed, since a call site cannot tell a member from a fn — so showing
  only the resolved one hid the fact that a choice was made. `read_to` in
  `core.fs` is the case that prompted it.
  * Each entry renders from its own declaration (a member says which effect it
    belongs to, a fn shows its signature) with the module it comes from, which
    is the same information the scope ladder decides on [fn-overload-scope].
  * The index is built by the LSP's analysis pass while the resolver's scopes
    are alive and kept as owned data (`Analysis::overloads`), since a
    `Resolution` borrows the program. Only names with more than one
    declaration are stored.
* [cli-platform] `salvo platform generate --backend NAME (--src DIR |
  --main FILE)` writes the host implementation skeleton for every
  `platform effect` and `platform handler` into `<src>/platform/`
  [platform-tree]: a named class (Kotlin) or unit struct (Rust) per effect,
  a class/struct named after each platform handler [platform-handler] —
  with that handler's constructor parameters, and a `new` on Rust, since the
  `use` site constructs it — each implementing the generated interface with
  every member stubbed (`TODO` / `todo!`), plus — in the module whose `main`
  needs a platform effect — the `main` that constructs the implementations
  and calls the generated entry point. `--src` and `--main` behave as in
  `salvo run` [cli-run].
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
  * A program with no platform declaration generates nothing, and says so.
  * std's own host files are **not** generated: they are shipped in
    `std/platform/` [platform-tree], so the command only ever writes into
    the customer's tree.
